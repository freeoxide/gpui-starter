/*! gpui-starter storage worker — SQLite on OPFS for the wasm build.
 *
 * Classic script on purpose (no import maps, no ES-module worker): the
 * harness serves COOP/COEP on every response and same-origin classic
 * scripts dodge the cross-origin fetch quirks module loading can hit under
 * COEP. It hosts the vendored @sqlite.org/sqlite-wasm classic build
 * (sqlite3.js + sqlite3.wasm, see LICENSE-SQLITE-WASM.md) with the OPFS
 * VFS, which requires the cross-origin isolation the harness provides.
 *
 * Protocol: JSON strings both ways, defined by
 * src/services/storage/protocol.rs (whose tests lock the exact op names —
 * keep both sides in sync):
 *   app → worker:  {"op":"schema-version","id":1}
 *                  {"op":"persist-error-record","id":2,"record":{...}}
 *                  {"op":"mark-crash-report-uploaded","id":3,
 *                   "reportId":"...","uploadedAt":"..."}  …
 *   worker → app:  {"op":"schema-version","id":1,"version":3}
 *                  {"op":"error","id":2,"message":"…"}     …
 * Every request is answered exactly once, correlated by `id`.
 *
 * Requests that arrive before the engine is ready are queued and flushed
 * after init; if init fails (no OPFS, engine load error) every queued and
 * future request answers {"op":"error"} with the init failure text — the
 * app side then degrades to its unavailable-storage behavior.
 */

"use strict";

// ---------------------------------------------------------------------------
// Schema parity: mirrors init_db DDL in src/services/storage/runtime.rs,
// i.e. migrations 1-3 of src/persistence/sqlite/db_migrations.rs
// (kv_store, error_log, crash_reports). journal_mode stays at the OPFS VFS
// default — WAL is a desktop-throughput knob, not part of the schema.
// ---------------------------------------------------------------------------

const MIGRATIONS = [
  {
    version: 1,
    name: "kv_store",
    sql: `
      CREATE TABLE IF NOT EXISTS kv_store (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL,
          updated_at TEXT NOT NULL
      );`,
  },
  {
    version: 2,
    name: "error_log",
    sql: `
      CREATE TABLE IF NOT EXISTS error_log (
          id TEXT PRIMARY KEY,
          occurred_at TEXT NOT NULL,
          severity TEXT NOT NULL,
          category TEXT NOT NULL,
          message TEXT NOT NULL,
          actions TEXT NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_error_log_occurred_at
          ON error_log (occurred_at DESC);`,
  },
  {
    version: 3,
    name: "crash_reports",
    sql: `
      CREATE TABLE IF NOT EXISTS crash_reports (
          id TEXT PRIMARY KEY,
          panic_message TEXT NOT NULL,
          backtrace TEXT NOT NULL,
          app_version TEXT NOT NULL,
          os TEXT NOT NULL,
          arch TEXT NOT NULL,
          timestamp TEXT NOT NULL,
          render_path BOOLEAN NOT NULL DEFAULT 0,
          recent_errors TEXT NOT NULL DEFAULT "[]",
          uploaded BOOLEAN NOT NULL DEFAULT 0,
          uploaded_at TEXT,
          upload_error TEXT
      );
      CREATE INDEX IF NOT EXISTS idx_crash_reports_timestamp
          ON crash_reports (timestamp DESC);
      CREATE INDEX IF NOT EXISTS idx_crash_reports_uploaded
          ON crash_reports (uploaded);`,
  },
];

const SCHEMA_MIGRATIONS_TABLE = `
  CREATE TABLE IF NOT EXISTS schema_migrations (
      version INTEGER PRIMARY KEY,
      applied_at TEXT NOT NULL
  );`;

// OPFS path of the database. The OPFS namespace is per-origin; a flat name
// avoids depending on VFS parent-directory creation.
const DB_PATH = "/gpui-starter-app.db";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let db = null; // sqlite3.oo1.OpfsDb once ready
let ready = false;
let initError = null;
const queue = []; // requests received before the engine settled

// ---------------------------------------------------------------------------
// Engine boot
// ---------------------------------------------------------------------------

importScripts("./sqlite3.js");

self.sqlite3InitModule({
  // Route engine diagnostics somewhere visible without spamming the
  // console in the happy path.
  print: () => {},
  printErr: (text) => console.warn("[storage-worker:sqlite]", text),
}).then(onEngineReady, (err) => {
  initError = `sqlite3 engine failed to initialize: ${err}`;
  console.warn("[storage-worker]", initError);
  flushQueue();
});

function onEngineReady(sqlite3) {
  let database;
  try {
    if (!("opfs" in sqlite3)) {
      throw new Error(
        "OPFS VFS unavailable (requires a secure context with OPFS support)"
      );
    }
    database = new sqlite3.oo1.OpfsDb(DB_PATH);
  } catch (err) {
    initError = `failed to open OPFS database: ${err}`;
    console.warn("[storage-worker]", initError);
    flushQueue();
    return;
  }

  db = database;
  try {
    db.exec(SCHEMA_MIGRATIONS_TABLE);
    // Mirrors run_migrations in db_migrations.rs (skip applied, one tx
    // per migration); OR IGNORE, unlike native, tolerates a two-tab race.
    let currentVersion =
      db.selectValue("SELECT COALESCE(MAX(version), 0) FROM schema_migrations");
    for (const migration of MIGRATIONS) {
      if (migration.version <= currentVersion) {
        continue;
      }
      db.exec("BEGIN");
      try {
        db.exec({ sql: migration.sql });
        db.exec({
          sql: "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?, datetime('now'))",
          bind: [migration.version],
        });
        db.exec("COMMIT");
      } catch (err) {
        try {
          db.exec("ROLLBACK");
        } catch {
          // Best-effort rollback; the migration error is what propagates.
        }
        throw err;
      }
      currentVersion = migration.version;
    }
    ready = true;
    const schemaVersion = currentVersion;
    console.log(
      "[storage-worker] ready: sqlite",
      sqlite3.version.libVersion,
      "on OPFS, schema version",
      schemaVersion
    );
  } catch (err) {
    initError = `schema init failed: ${err}`;
    console.warn("[storage-worker]", initError);
    try {
      db.close();
    } catch {
      // Best-effort close on a failed init.
    }
    db = null;
  }
  flushQueue();
}

// ---------------------------------------------------------------------------
// Message loop
// ---------------------------------------------------------------------------

self.onmessage = (event) => {
  let message;
  try {
    message = JSON.parse(event.data);
  } catch {
    return; // Not our protocol; nothing to correlate a reply to.
  }
  if (!message || typeof message !== "object" || typeof message.id !== "number") {
    return;
  }
  if (!ready && !initError) {
    queue.push(message);
    return;
  }
  handle(message);
};

function flushQueue() {
  const pending = queue.splice(0, queue.length);
  for (const message of pending) {
    handle(message);
  }
}

function handle(message) {
  if (initError || !db) {
    replyError(message.id, initError || "storage worker not ready");
    return;
  }
  try {
    handleOp(message);
  } catch (err) {
    replyError(message.id, String((err && err.message) || err));
  }
}

function reply(payload) {
  self.postMessage(JSON.stringify(payload));
}

function replyError(id, message) {
  reply({ op: "error", id, message });
}

// ---------------------------------------------------------------------------
// Ops — one per StorageBackend trait method
// (src/services/storage/mod.rs)
// ---------------------------------------------------------------------------

function handleOp(message) {
  switch (message.op) {
    case "schema-version": {
      const version =
        db.selectValue("SELECT COALESCE(MAX(version), 0) FROM schema_migrations");
      reply({ op: "schema-version", id: message.id, version: version || 0 });
      return;
    }

    case "health-check": {
      db.exec("SELECT 1");
      reply({ op: "health-check", id: message.id });
      return;
    }

    case "maintenance": {
      db.exec("PRAGMA optimize;");
      reply({ op: "maintenance", id: message.id });
      return;
    }

    case "persist-error-record": {
      const r = message.record;
      db.exec({
        sql: `INSERT OR IGNORE INTO error_log
                (id, occurred_at, severity, category, message, actions)
              VALUES (?, ?, ?, ?, ?, ?)`,
        bind: [
          r.id,
          r.occurred_at,
          r.severity,
          r.category,
          r.message,
          JSON.stringify(r.actions || []),
        ],
      });
      reply({ op: "persist-error-record", id: message.id });
      return;
    }

    case "load-error-history": {
      const rows = db.exec({
        sql: `SELECT id, occurred_at, severity, category, message, actions
              FROM error_log
              ORDER BY occurred_at DESC
              LIMIT ?`,
        bind: [message.limit],
        rowMode: "object",
        returnValue: "result_rows",
      });
      const records = rows.map((row) => ({
        id: row.id,
        occurred_at: row.occurred_at,
        severity: row.severity,
        category: row.category,
        message: row.message,
        actions: JSON.parse(row.actions || "[]"),
      }));
      reply({ op: "load-error-history", id: message.id, records });
      return;
    }

    case "persist-crash-report": {
      const r = message.report;
      db.exec({
        sql: `INSERT OR IGNORE INTO crash_reports
                (id, panic_message, backtrace, app_version, os, arch,
                 timestamp, render_path, recent_errors)
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
        bind: [
          r.id,
          r.panic_message,
          r.backtrace,
          r.app_version,
          r.os,
          r.arch,
          r.timestamp,
          r.render_path ? 1 : 0,
          JSON.stringify(r.recent_errors || []),
        ],
      });
      reply({ op: "persist-crash-report", id: message.id });
      return;
    }

    case "load-pending-crash-reports": {
      const rows = db.exec({
        sql: `SELECT id, panic_message, backtrace, app_version, os, arch,
                     timestamp, render_path, recent_errors
              FROM crash_reports
              WHERE uploaded = 0
              ORDER BY timestamp DESC
              LIMIT ?`,
        bind: [message.limit],
        rowMode: "object",
        returnValue: "result_rows",
      });
      const reports = rows.map((row) => ({
        id: row.id,
        panic_message: row.panic_message,
        backtrace: row.backtrace,
        app_version: row.app_version,
        os: row.os,
        arch: row.arch,
        timestamp: row.timestamp,
        render_path: !!row.render_path,
        recent_errors: JSON.parse(row.recent_errors || "[]"),
      }));
      reply({ op: "load-pending-crash-reports", id: message.id, reports });
      return;
    }

    case "mark-crash-report-uploaded": {
      db.exec({
        sql: "UPDATE crash_reports SET uploaded = 1, uploaded_at = ? WHERE id = ?",
        bind: [message.uploadedAt, message.reportId],
      });
      reply({ op: "mark-crash-report-uploaded", id: message.id });
      return;
    }

    default:
      replyError(message.id, `unknown storage op: ${message.op}`);
  }
}

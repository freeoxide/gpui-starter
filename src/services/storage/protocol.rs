//! JSON wire protocol between the wasm storage backend
//! ([`super::web::WebSqliteStorage`]) and the sqlite worker
//! (`wasm/sqlite/worker.js`).
//!
//! Target-neutral on purpose: the types are plain serde so the
//! encode/decode round-trips (and the exact op strings the classic worker
//! script switches on) are unit-testable on native builds, while the wasm
//! backend serializes the very same values onto `Worker.postMessage`.
//!
//! Both directions are JSON strings — the Rust side sends
//! `WorkerRequest`-shaped JSON, the worker answers with
//! `WorkerResponse`-shaped JSON, and every message is correlated by its
//! `id` (allocated by the Rust side).

use serde::{Deserialize, Serialize};

use crate::error_surface::ErrorRecord;
use crate::services::crash_report::CrashReport;

/// Op names serialize kebab-case (`"schema-version"`, …) and payload fields
/// camelCase (`reportId`, `uploadedAt`, …) — the exact strings the worker's
/// `switch (op)` matches on. The tests below lock them in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum WorkerRequest {
    #[serde(rename_all = "camelCase")]
    SchemaVersion {
        id: u64,
    },
    HealthCheck {
        id: u64,
    },
    Maintenance {
        id: u64,
    },
    PersistErrorRecord {
        id: u64,
        record: ErrorRecord,
    },
    #[serde(rename_all = "camelCase")]
    LoadErrorHistory {
        id: u64,
        limit: usize,
    },
    PersistCrashReport {
        id: u64,
        report: CrashReport,
    },
    #[serde(rename_all = "camelCase")]
    LoadPendingCrashReports {
        id: u64,
        limit: usize,
    },
    #[serde(rename_all = "camelCase")]
    MarkCrashReportUploaded {
        id: u64,
        report_id: String,
        uploaded_at: String,
    },
}

/// [`WorkerResponse::Error`] is the single failure shape for every op,
/// worker-side SQL errors included.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum WorkerResponse {
    #[serde(rename_all = "camelCase")]
    SchemaVersion {
        id: u64,
        version: i64,
    },
    HealthCheck {
        id: u64,
    },
    Maintenance {
        id: u64,
    },
    PersistErrorRecord {
        id: u64,
    },
    LoadErrorHistory {
        id: u64,
        records: Vec<ErrorRecord>,
    },
    PersistCrashReport {
        id: u64,
    },
    #[serde(rename_all = "camelCase")]
    LoadPendingCrashReports {
        id: u64,
        reports: Vec<CrashReport>,
    },
    #[serde(rename_all = "camelCase")]
    MarkCrashReportUploaded {
        id: u64,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        id: u64,
        message: String,
    },
}

impl WorkerResponse {
    pub fn id(&self) -> u64 {
        match self {
            Self::SchemaVersion { id, .. }
            | Self::HealthCheck { id }
            | Self::Maintenance { id }
            | Self::PersistErrorRecord { id }
            | Self::LoadErrorHistory { id, .. }
            | Self::PersistCrashReport { id }
            | Self::LoadPendingCrashReports { id, .. }
            | Self::MarkCrashReportUploaded { id }
            | Self::Error { id, .. } => *id,
        }
    }

    pub fn into_result(self) -> Result<WorkerResult, String> {
        match self {
            Self::SchemaVersion { version, .. } => Ok(WorkerResult::SchemaVersion(version)),
            Self::HealthCheck { .. }
            | Self::Maintenance { .. }
            | Self::PersistErrorRecord { .. }
            | Self::PersistCrashReport { .. }
            | Self::MarkCrashReportUploaded { .. } => Ok(WorkerResult::Unit),
            Self::LoadErrorHistory { records, .. } => Ok(WorkerResult::ErrorRecords(records)),
            Self::LoadPendingCrashReports { reports, .. } => {
                Ok(WorkerResult::CrashReports(reports))
            }
            Self::Error { message, .. } => Err(message),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkerResult {
    SchemaVersion(i64),
    Unit,
    ErrorRecords(Vec<ErrorRecord>),
    CrashReports(Vec<CrashReport>),
}

impl WorkerResult {
    /// Op name this payload answers — read only by the wasm backend's
    /// protocol-mismatch path (gated so native test builds don't see it dead).
    #[cfg(target_family = "wasm")]
    pub fn op_name(&self) -> &'static str {
        match self {
            Self::SchemaVersion(_) => "schema-version",
            Self::Unit => "unit",
            Self::ErrorRecords(_) => "load-error-history",
            Self::CrashReports(_) => "load-pending-crash-reports",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_record() -> ErrorRecord {
        ErrorRecord {
            id: crate::ids::EventId::new(),
            occurred_at: crate::time::AppTimestamp::now(),
            severity: crate::errors::AppErrorSeverity::Warning,
            category: crate::error_surface::ErrorCategory::Storage,
            message: "disk full".into(),
            actions: vec![
                crate::error_surface::ErrorAction::Retry,
                crate::error_surface::ErrorAction::Dismiss,
            ],
        }
    }

    fn example_report() -> CrashReport {
        CrashReport::new(
            "test panic".into(),
            "backtrace".into(),
            true,
            vec!["error1".into()],
        )
    }

    /// Every request variant must survive a JSON string round-trip
    /// unchanged — that string is exactly what crosses the worker boundary.
    #[test]
    fn requests_round_trip_through_json() {
        let requests = vec![
            WorkerRequest::SchemaVersion { id: 1 },
            WorkerRequest::HealthCheck { id: 2 },
            WorkerRequest::Maintenance { id: 3 },
            WorkerRequest::PersistErrorRecord {
                id: 4,
                record: example_record(),
            },
            WorkerRequest::LoadErrorHistory { id: 5, limit: 25 },
            WorkerRequest::PersistCrashReport {
                id: 6,
                report: example_report(),
            },
            WorkerRequest::LoadPendingCrashReports { id: 7, limit: 50 },
            WorkerRequest::MarkCrashReportUploaded {
                id: 8,
                report_id: "report-1".into(),
                uploaded_at: "2025-01-01T00:00:00Z".into(),
            },
        ];
        for request in requests {
            let json = serde_json::to_string(&request).expect("encode request");
            let decoded: WorkerRequest = serde_json::from_str(&json).expect("decode request");
            assert_eq!(decoded, request);
        }
    }

    #[test]
    fn responses_round_trip_through_json_with_ids() {
        let responses = vec![
            (WorkerResponse::SchemaVersion { id: 11, version: 3 }, 11),
            (WorkerResponse::HealthCheck { id: 12 }, 12),
            (WorkerResponse::Maintenance { id: 13 }, 13),
            (WorkerResponse::PersistErrorRecord { id: 14 }, 14),
            (
                WorkerResponse::LoadErrorHistory {
                    id: 15,
                    records: vec![example_record()],
                },
                15,
            ),
            (WorkerResponse::PersistCrashReport { id: 16 }, 16),
            (
                WorkerResponse::LoadPendingCrashReports {
                    id: 17,
                    reports: vec![example_report()],
                },
                17,
            ),
            (WorkerResponse::MarkCrashReportUploaded { id: 18 }, 18),
            (
                WorkerResponse::Error {
                    id: 19,
                    message: "opfs unavailable".into(),
                },
                19,
            ),
        ];
        for (response, expected_id) in responses {
            let json = serde_json::to_string(&response).expect("encode response");
            let decoded: WorkerResponse = serde_json::from_str(&json).expect("decode response");
            assert_eq!(decoded, response);
            assert_eq!(decoded.id(), expected_id);
        }
    }

    /// The op strings and camelCase field names are the cross-language
    /// contract with `wasm/sqlite/worker.js` — lock the wire format so a
    /// rename on either side fails here instead of at runtime in the
    /// browser.
    #[test]
    fn wire_format_op_and_field_names_are_locked() {
        let json = serde_json::to_string(&WorkerRequest::SchemaVersion { id: 1 }).unwrap();
        assert_eq!(json, r#"{"op":"schema-version","id":1}"#);

        let json =
            serde_json::to_string(&WorkerRequest::LoadErrorHistory { id: 2, limit: 5 }).unwrap();
        assert_eq!(json, r#"{"op":"load-error-history","id":2,"limit":5}"#);

        let json = serde_json::to_string(&WorkerRequest::MarkCrashReportUploaded {
            id: 3,
            report_id: "r".into(),
            uploaded_at: "t".into(),
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"op":"mark-crash-report-uploaded","id":3,"reportId":"r","uploadedAt":"t"}"#
        );

        let json = serde_json::to_string(&WorkerResponse::LoadPendingCrashReports {
            id: 4,
            reports: vec![],
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"op":"load-pending-crash-reports","id":4,"reports":[]}"#
        );

        let json = serde_json::to_string(&WorkerResponse::Error {
            id: 5,
            message: "boom".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"op":"error","id":5,"message":"boom"}"#);
    }

    /// `into_result` maps success variants to typed payloads and the error
    /// shape to `Err(message)` — the exact split the correlation table
    /// forwards to the awaiting trait call.
    #[test]
    fn responses_convert_to_trait_results() {
        assert_eq!(
            WorkerResponse::SchemaVersion { id: 1, version: 3 }.into_result(),
            Ok(WorkerResult::SchemaVersion(3))
        );
        assert_eq!(
            WorkerResponse::HealthCheck { id: 2 }.into_result(),
            Ok(WorkerResult::Unit)
        );
        let records = vec![example_record()];
        assert_eq!(
            WorkerResponse::LoadErrorHistory {
                id: 3,
                records: records.clone(),
            }
            .into_result(),
            Ok(WorkerResult::ErrorRecords(records))
        );
        assert_eq!(
            WorkerResponse::Error {
                id: 4,
                message: "opfs unavailable".into(),
            }
            .into_result(),
            Err("opfs unavailable".to_string())
        );
    }

    /// An unknown op tag must be rejected outright (not silently dropped) —
    /// the worker and this module upgrade together or not at all.
    #[test]
    fn unknown_op_tags_are_rejected() {
        let bad_request = serde_json::from_str::<WorkerRequest>(r#"{"op":"vacuum","id":1}"#);
        assert!(bad_request.is_err());
        let bad_response = serde_json::from_str::<WorkerResponse>(r#"{"op":"vacuum","id":1}"#);
        assert!(bad_response.is_err());
    }
}

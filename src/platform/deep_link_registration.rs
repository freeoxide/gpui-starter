//! Per-user URL-scheme registration for deep links. Windows only launches
//! `gpui-starter://` URLs when a protocol handler exists under
//! `Software\Classes`, and unpackaged installs have none — browsers and the
//! shell silently do nothing until this module writes one. Packaged (MSIX or
//! installer) builds register the scheme themselves; this covers the rest.
//! [`ensure_deep_link_registered`] is idempotent and best-effort, mirroring
//! `toast_identity::ensure_toast_identity`.

use std::path::Path;

use crate::platform::process::single_instance::SCHEME;
use crate::platform::toast_identity::same_path;

const LOG: &str = "gpui_starter::deep_link_registration";

/// Register the `gpui-starter://` scheme for the current exe under HKCU (no
/// admin rights): the `URL Protocol` marker plus a `shell\open\command`
/// pointing at the running exe. `Err(reason)` never blocks startup — callers
/// log and continue.
pub fn ensure_deep_link_registered() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| format!("current_exe: {err}"))?;
    let command = format!("\"{}\" \"%1\"", exe.display());

    let scheme = windows_registry::CURRENT_USER
        .create(format!("Software\\Classes\\{}", scheme_key()))
        .map_err(|err| format!("create scheme key: {err}"))?;
    // RegCreateKeyExW creates one level per call, so walk down to the command key.
    let command_key = scheme
        .create("shell")
        .and_then(|shell| shell.create("open"))
        .and_then(|open| open.create("command"))
        .map_err(|err| format!("create command key: {err}"))?;

    // Self-healing fast path: a stored command already pointing at this exe is
    // left untouched; anything else (first run, exe moved between dev target
    // dirs, garbage) is rewritten.
    if let Ok(existing) = command_key.get_string("")
        && command_matches(&existing, &exe)
    {
        tracing::debug!(target: LOG, "deep-link registration already current");
        return Ok(());
    }

    // An empty value name is the key default; `URL Protocol` is what marks the
    // key a protocol handler.
    scheme
        .set_string("", "URL:GPUI Starter")
        .and_then(|()| scheme.set_string("URL Protocol", ""))
        .and_then(|()| command_key.set_string("", &command))
        .map_err(|err| format!("write scheme values: {err}"))?;
    tracing::info!(target: LOG, command = %command, "deep-link scheme registered for the current exe");
    Ok(())
}

/// Registry key name for the scheme: `gpui-starter://` minus `://`. Derived,
/// never a second copy of the scheme string.
fn scheme_key() -> String {
    SCHEME.strip_suffix("://").unwrap_or(SCHEME).to_string()
}

/// A stored `shell\open\command` value matches when its quoted path is the
/// current exe (case-insensitive, like registry paths) and the `%1`
/// placeholder the OS substitutes the URL into is intact.
fn command_matches(command: &str, exe: &Path) -> bool {
    let Some((quoted_path, tail)) = command
        .strip_prefix('"')
        .and_then(|rest| rest.split_once('"'))
    else {
        return false;
    };
    tail == " \"%1\"" && same_path(Path::new(quoted_path), exe)
}

#[cfg(test)]
#[path = "deep_link_registration.test.rs"]
mod deep_link_registration_test;

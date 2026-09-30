use gpui_kit::{App, BorrowAppContext as _, Global, SharedString};

// Native: variants carry the underlying `keyring::Error`. Wasm: keyring does
// not exist, so the same variant names carry the error message string —
// callers only ever `Display` the error.
#[cfg(not(target_family = "wasm"))]
#[derive(Debug, thiserror::Error)]
pub enum SecureStorageError {
    #[error("entry creation failed for '{service}/{key}': {source}")]
    EntryCreation {
        service: String,
        key: String,
        #[source]
        source: keyring::Error,
    },
    #[error("failed to set secret for '{service}/{key}': {source}")]
    SetFailed {
        service: String,
        key: String,
        #[source]
        source: keyring::Error,
    },
    #[error("failed to get secret for '{service}/{key}': {source}")]
    GetFailed {
        service: String,
        key: String,
        #[source]
        source: keyring::Error,
    },
    #[error("failed to delete secret for '{service}/{key}': {source}")]
    DeleteFailed {
        service: String,
        key: String,
        #[source]
        source: keyring::Error,
    },
}

#[cfg(target_family = "wasm")]
#[derive(Debug, thiserror::Error)]
pub enum SecureStorageError {
    #[error("entry creation failed for '{service}/{key}': {reason}")]
    EntryCreation {
        service: String,
        key: String,
        reason: String,
    },
    #[error("failed to set secret for '{service}/{key}': {reason}")]
    SetFailed {
        service: String,
        key: String,
        reason: String,
    },
    #[error("failed to get secret for '{service}/{key}': {reason}")]
    GetFailed {
        service: String,
        key: String,
        reason: String,
    },
    #[error("failed to delete secret for '{service}/{key}': {reason}")]
    DeleteFailed {
        service: String,
        key: String,
        reason: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct SecureStorageSnapshot {
    pub available: bool,
    pub last_error: Option<String>,
}

impl Global for SecureStorageSnapshot {}

#[cfg(not(target_family = "wasm"))]
pub fn initialize(cx: &mut App) {
    let available = keyring::Entry::new("gpui-starter", "availability-check").is_ok();
    let snapshot = SecureStorageSnapshot {
        available,
        last_error: if available {
            None
        } else {
            Some("keyring entry initialization unavailable".to_string())
        },
    };
    cx.set_global(snapshot.clone());
    let status = if snapshot.available {
        crate::capabilities::CapabilityStatus::supported_enabled()
    } else {
        crate::capabilities::CapabilityStatus::error(
            snapshot
                .last_error
                .as_deref()
                .unwrap_or("secure storage unavailable"),
        )
    };
    crate::capabilities::set("secure_storage", status, cx);
}

/// Wasm: no OS keyring exists — report the backend as unavailable so the
/// settings/diagnostics UI degrades gracefully.
#[cfg(target_family = "wasm")]
pub fn initialize(cx: &mut App) {
    let snapshot = SecureStorageSnapshot {
        available: false,
        last_error: Some(WASM_UNAVAILABLE.to_string()),
    };
    cx.set_global(snapshot.clone());
    crate::capabilities::set(
        "secure_storage",
        crate::capabilities::CapabilityStatus::error(WASM_UNAVAILABLE),
        cx,
    );
}

pub fn snapshot(cx: &App) -> SecureStorageSnapshot {
    cx.try_global::<SecureStorageSnapshot>()
        .cloned()
        .unwrap_or_default()
}

#[cfg(target_family = "wasm")]
const WASM_UNAVAILABLE: &str = "secure storage unavailable on wasm";

#[cfg(not(target_family = "wasm"))]
fn entry(service: &str, key: &str) -> Result<keyring::Entry, SecureStorageError> {
    keyring::Entry::new(service, key).map_err(|err| {
        tracing::error!(target: "gpui_starter::secure_storage", "entry creation failed: {err}");
        SecureStorageError::EntryCreation {
            service: service.to_string(),
            key: key.to_string(),
            source: err,
        }
    })
}

#[cfg(not(target_family = "wasm"))]
pub fn set_secret(
    service: &str,
    key: &str,
    value: &str,
    cx: &mut App,
) -> Result<(), SecureStorageError> {
    let entry = entry(service, key)?;
    entry.set_password(value).map_err(|err| {
        tracing::error!(target: "gpui_starter::secure_storage", service, key, "set_password failed: {err}");
        update_last_error(Some(err.to_string()), cx);
        SecureStorageError::SetFailed { service: service.to_string(), key: key.to_string(), source: err }
    })?;
    tracing::info!(target: "gpui_starter::secure_storage", service, key, "secret written");
    update_last_error(None, cx);
    Ok(())
}

#[cfg(not(target_family = "wasm"))]
pub fn get_secret(
    service: &str,
    key: &str,
    cx: &mut App,
) -> Result<Option<SharedString>, SecureStorageError> {
    let entry = entry(service, key)?;
    match entry.get_password() {
        Ok(value) => {
            tracing::info!(target: "gpui_starter::secure_storage", service, key, "secret read");
            update_last_error(None, cx);
            Ok(Some(value.into()))
        }
        Err(keyring::Error::NoEntry) => {
            tracing::warn!(target: "gpui_starter::secure_storage", service, key, "no entry found");
            Ok(None)
        }
        Err(err) => {
            tracing::error!(target: "gpui_starter::secure_storage", service, key, "get_password failed: {err}");
            Err(SecureStorageError::GetFailed {
                service: service.to_string(),
                key: key.to_string(),
                source: err,
            })
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn delete_secret(service: &str, key: &str, cx: &mut App) -> Result<(), SecureStorageError> {
    let entry = entry(service, key)?;
    entry.delete_credential().map_err(|err| {
        tracing::error!(target: "gpui_starter::secure_storage", service, key, "delete failed: {err}");
        update_last_error(Some(err.to_string()), cx);
        SecureStorageError::DeleteFailed { service: service.to_string(), key: key.to_string(), source: err }
    })?;
    tracing::info!(target: "gpui_starter::secure_storage", service, key, "secret deleted");
    update_last_error(None, cx);
    Ok(())
}

#[cfg(target_family = "wasm")]
pub fn set_secret(
    service: &str,
    key: &str,
    _value: &str,
    cx: &mut App,
) -> Result<(), SecureStorageError> {
    update_last_error(Some(WASM_UNAVAILABLE.to_string()), cx);
    Err(SecureStorageError::SetFailed {
        service: service.to_string(),
        key: key.to_string(),
        reason: WASM_UNAVAILABLE.to_string(),
    })
}

#[cfg(target_family = "wasm")]
pub fn get_secret(
    service: &str,
    key: &str,
    cx: &mut App,
) -> Result<Option<SharedString>, SecureStorageError> {
    update_last_error(Some(WASM_UNAVAILABLE.to_string()), cx);
    Err(SecureStorageError::GetFailed {
        service: service.to_string(),
        key: key.to_string(),
        reason: WASM_UNAVAILABLE.to_string(),
    })
}

#[cfg(target_family = "wasm")]
pub fn delete_secret(service: &str, key: &str, cx: &mut App) -> Result<(), SecureStorageError> {
    update_last_error(Some(WASM_UNAVAILABLE.to_string()), cx);
    Err(SecureStorageError::DeleteFailed {
        service: service.to_string(),
        key: key.to_string(),
        reason: WASM_UNAVAILABLE.to_string(),
    })
}

fn update_last_error(last_error: Option<String>, cx: &mut App) {
    cx.update_global::<SecureStorageSnapshot, _>(|state, _cx| {
        state.last_error = last_error;
    });
}

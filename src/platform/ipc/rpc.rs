//! Line adapters between serde types and the newline-JSON IPC framing used by
//! `crate::single_instance`.

use serde::de::DeserializeOwned;

use super::command::ForwardedRequest;

const LOG: &str = "gpui_starter::ipc::rpc";

/// Encode a `Serialize`-able value as one compact JSON line with trailing
/// `\n`, the wire shape the single-instance forwarder expects.
pub fn encode_line<T: serde::Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    Ok(line)
}

/// Decode one JSON value from a line; blank lines yield `Ok(None)` so
/// keepalives can be skipped without a protocol error.
pub(crate) fn decode_line<T: DeserializeOwned>(line: &str) -> Result<Option<T>, serde_json::Error> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    serde_json::from_str(trimmed).map(Some)
}

/// Decode a line as a [`ForwardedRequest`]; `None` on blank or malformed
/// input (the latter is logged).
pub fn decode_request(line: &str) -> Option<ForwardedRequest> {
    match decode_line::<ForwardedRequest>(line) {
        Ok(Some(req)) => Some(req),
        Ok(None) => None,
        Err(err) => {
            tracing::warn!(
                target: LOG,
                error = %err,
                line = line.trim(),
                "failed to decode ipc request line"
            );
            None
        }
    }
}

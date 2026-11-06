//! Typed command/response layer for the newline-delimited local-socket IPC
//! used by `crate::single_instance`; the socket plumbing lives there.

pub mod command;
pub mod rpc;

pub use command::{ForwardedCommand, ForwardedRequest, ForwardedResponse};

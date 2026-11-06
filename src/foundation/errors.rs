use std::{io, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppErrorSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorCategory {
    Network,
    Storage,
    Rendering,
    Config,
    System,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to initialize app paths")]
    PathInitialization,
    #[error("failed to read app state from {path}: {details}")]
    StateRead { path: PathBuf, details: String },
    #[error("failed to parse app state from {path}: {details}")]
    StateParse { path: PathBuf, details: String },
    #[error("failed to write app state to {path}: {details}")]
    StateWrite { path: PathBuf, details: String },
    #[error("invalid deep link `{input}`: {reason}")]
    InvalidDeepLink { input: String, reason: String },
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

impl AppError {
    /// Construct an `InvalidDeepLink` error from any string-like input/reason.
    pub fn invalid_deep_link(input: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidDeepLink {
            input: input.into(),
            reason: reason.into(),
        }
    }

    pub fn severity(&self) -> AppErrorSeverity {
        match self {
            Self::InvalidDeepLink { .. } | Self::StateParse { .. } => AppErrorSeverity::Warning,
            Self::PathInitialization
            | Self::StateRead { .. }
            | Self::StateWrite { .. }
            | Self::Io(_) => AppErrorSeverity::Error,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::StateRead { .. } | Self::StateWrite { .. } => ErrorCategory::Storage,
            Self::StateParse { .. } => ErrorCategory::Config,
            Self::PathInitialization | Self::InvalidDeepLink { .. } | Self::Io(_) => {
                ErrorCategory::System
            }
        }
    }
}

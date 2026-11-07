use crate::state::config_store::{APP_STATE_VERSION, AppConfig};

/// Bump a config from any older schema version to the current one. Every
/// post-v0 field carries a serde default, so only the version moves;
/// `load_config` quarantines newer-than-build files before this runs.
pub fn migrate(mut config: AppConfig) -> AppConfig {
    if config.version < APP_STATE_VERSION {
        config.version = APP_STATE_VERSION;
    }
    config
}

#[cfg(test)]
#[path = "migrations.test.rs"]
mod migrations_test;

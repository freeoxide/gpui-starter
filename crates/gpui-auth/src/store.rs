use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::AuthError;
use crate::token::TokenSet;

/// Persisted-token storage keyed by provider id. Methods run on whatever
/// thread awaits them (callers keep OS-keyring I/O off the UI thread), so
/// implementations must not touch main-thread-only state.
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait TokenStore: Send + Sync {
    async fn save(&self, provider_id: &str, tokens: &TokenSet) -> Result<(), AuthError>;
    async fn load(&self, provider_id: &str) -> Result<Option<TokenSet>, AuthError>;
    async fn delete(&self, provider_id: &str) -> Result<(), AuthError>;
}

/// Process-lifetime store: tokens vanish when the process exits. The default
/// on targets without an OS credential store.
#[derive(Default)]
pub struct InMemoryStore {
    entries: Mutex<HashMap<String, TokenSet>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn put(&self, provider_id: &str, tokens: TokenSet) {
        self.entries
            .lock()
            .expect("in-memory token store lock poisoned")
            .insert(provider_id.to_string(), tokens);
    }

    fn get(&self, provider_id: &str) -> Option<TokenSet> {
        self.entries
            .lock()
            .expect("in-memory token store lock poisoned")
            .get(provider_id)
            .cloned()
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl TokenStore for InMemoryStore {
    async fn save(&self, provider_id: &str, tokens: &TokenSet) -> Result<(), AuthError> {
        self.put(provider_id, tokens.clone());
        Ok(())
    }

    async fn load(&self, provider_id: &str) -> Result<Option<TokenSet>, AuthError> {
        Ok(self.get(provider_id))
    }

    async fn delete(&self, provider_id: &str) -> Result<(), AuthError> {
        self.entries
            .lock()
            .expect("in-memory token store lock poisoned")
            .remove(provider_id);
        Ok(())
    }
}

#[cfg(test)]
#[path = "store.test.rs"]
mod store_test;

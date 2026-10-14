use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::AuthError;
use crate::token::TokenSet;

/// Persisted-token storage keyed by provider id. Every method runs on the
/// caller's App context so app-side implementations can update their own
/// globals.
pub trait TokenStore: Send + Sync {
    fn save(
        &self,
        provider_id: &str,
        tokens: &TokenSet,
        cx: &mut gpui::App,
    ) -> Result<(), AuthError>;
    fn load(&self, provider_id: &str, cx: &mut gpui::App) -> Result<Option<TokenSet>, AuthError>;
    fn delete(&self, provider_id: &str, cx: &mut gpui::App) -> Result<(), AuthError>;
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

impl TokenStore for InMemoryStore {
    fn save(
        &self,
        provider_id: &str,
        tokens: &TokenSet,
        _cx: &mut gpui::App,
    ) -> Result<(), AuthError> {
        self.put(provider_id, tokens.clone());
        Ok(())
    }

    fn load(&self, provider_id: &str, _cx: &mut gpui::App) -> Result<Option<TokenSet>, AuthError> {
        Ok(self.get(provider_id))
    }

    fn delete(&self, provider_id: &str, _cx: &mut gpui::App) -> Result<(), AuthError> {
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

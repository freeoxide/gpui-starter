//! Auth foundation for GPUI apps: token model, PKCE, provider trait, and a
//! controller entity. Security rules: secrets live only in the token store,
//! `SecretString` never prints, and every browser flow carries PKCE + a fresh
//! random state.
pub mod controller;
#[cfg(not(target_family = "wasm"))]
pub mod demo_backend;
pub mod email;
pub mod error;
#[cfg(feature = "mock-provider")]
pub mod mock;
pub mod pkce;
pub mod provider;
pub mod store;
pub mod token;

pub use controller::{AuthController, AuthEvent, AuthState};
#[cfg(not(target_family = "wasm"))]
pub use demo_backend::DemoBackend;
pub use email::{
    AuthBackend, AuthenticatedSession, EMAIL_PROVIDER_ID, EmailAccountStatus,
    EmailPasswordProvider, PasswordAuth, SignInOutcome, TotpChallenge, TotpEnrollment,
};
pub use error::AuthError;
#[cfg(feature = "mock-provider")]
pub use mock::MockProvider;
pub use pkce::{PkcePair, authorize_challenge, random_state};
pub use provider::{AuthIo, AuthProvider, FlowLaunch, RedirectParams, RefreshPolicy};
pub use store::{InMemoryStore, TokenStore};
pub use token::{AccountInfo, SecretString, TokenSet};

use crate::util::init_once::{AsyncInitOnce, AsyncInitializer};
use async_trait::async_trait;
use gcp_auth::{AuthenticationManager, GCPAuthError};
use once_cell::sync::Lazy;

pub(crate) struct AuthenticationManagerInitializer {}
#[async_trait]
impl AsyncInitializer for AuthenticationManagerInitializer {
    type T = AuthenticationManager;
    type Error = GCPAuthError;
    async fn create(&self) -> Result<AuthenticationManager, GCPAuthError> {
        gcp_auth::init().await
    }
}

pub(crate) static AUTHENTICATION_MANAGER: Lazy<AsyncInitOnce<AuthenticationManagerInitializer>> =
    Lazy::new(|| AsyncInitOnce::new(AuthenticationManagerInitializer {}));

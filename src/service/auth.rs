use crate::util::init_once::{AsyncInitOnce, AsyncInitializer};
use async_trait::async_trait;
use gcp_auth::AuthenticationManager;
use once_cell::sync::Lazy;

pub(crate) struct AuthenticationManagerInitializer {}
#[async_trait]
impl AsyncInitializer<AuthenticationManager> for AuthenticationManagerInitializer {
    async fn create(&self) -> Result<AuthenticationManager, Box<dyn std::error::Error>> {
        Ok(gcp_auth::init().await?)
    }
}

pub(crate) static AUTHENTICATION_MANAGER: Lazy<
    AsyncInitOnce<AuthenticationManager, AuthenticationManagerInitializer>,
> = Lazy::new(|| AsyncInitOnce::new(AuthenticationManagerInitializer {}));

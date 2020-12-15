use crate::util::init_once::{AsyncCreator, InitOnce};
use async_trait::async_trait;
use gcp_auth::AuthenticationManager;
use once_cell::sync::Lazy;

pub(crate) struct AuthenticationManagerCreator {}
#[async_trait]
impl AsyncCreator<AuthenticationManager> for AuthenticationManagerCreator {
    async fn create(&self) -> Result<AuthenticationManager, Box<dyn std::error::Error>> {
        Ok(gcp_auth::init().await?)
    }
}

pub(crate) static AUTHENTICATION_MANAGER: Lazy<
    InitOnce<AuthenticationManager, AuthenticationManagerCreator>,
> = Lazy::new(|| InitOnce::new(AuthenticationManagerCreator {}));

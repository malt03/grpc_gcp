use crate::init_once::InitOnce;
use once_cell::sync::Lazy;

pub(crate) static AUTHENTICATION_MANAGER: Lazy<InitOnce<gcp_auth::AuthenticationManager>> =
    Lazy::new(|| InitOnce::new());

pub(crate) async fn init() -> Result<(), gcp_auth::GCPAuthError> {
    AUTHENTICATION_MANAGER.init(gcp_auth::init).await?;
    Ok(())
}

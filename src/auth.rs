use once_cell::sync::{Lazy, OnceCell};
use tokio::sync::Mutex;

static AUTHENTICATION_MANAGER: OnceCell<gcp_auth::AuthenticationManager> = OnceCell::new();
static AUTHENTICATION_MANAGER_INITIALIZED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub(crate) async fn get_authentication_manager(
) -> Result<&'static gcp_auth::AuthenticationManager, Box<dyn std::error::Error>> {
    if let Some(manager) = AUTHENTICATION_MANAGER.get() {
        return Ok(manager);
    }
    let mut initialized = AUTHENTICATION_MANAGER_INITIALIZED.lock().await;
    if !*initialized {
        let manager = gcp_auth::init().await?;
        if AUTHENTICATION_MANAGER.set(manager).is_err() {
            panic!("unexpected");
        }
        *initialized = true;
    }
    return Ok(AUTHENTICATION_MANAGER.get().unwrap());
}

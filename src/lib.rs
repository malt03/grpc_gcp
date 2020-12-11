mod auth;
mod config;
pub mod firestore;
mod init_once;
mod proto;

pub async fn init(project_id: impl Into<String>) -> Result<(), Box<dyn std::error::Error>> {
    config::init(project_id);
    let (a, b) = futures::future::join(auth::init(), firestore::v1::init()).await;
    a?;
    b?;
    Ok(())
}

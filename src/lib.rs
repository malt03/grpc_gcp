mod config;
mod proto;
mod service;
mod util;

pub use service::google::{firestore, pubsub};

pub async fn init(project_id: impl Into<String>) -> Result<(), Box<dyn std::error::Error>> {
    config::init(project_id);
    service::init().await?;
    Ok(())
}

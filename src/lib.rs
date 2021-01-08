#[macro_use]
mod service;
mod config;
mod proto;
mod serde_properties;
mod util;

pub use service::google::{datastore, firestore, pubsub};

pub fn init(project_id: impl Into<String>) {
    config::init(project_id);
}

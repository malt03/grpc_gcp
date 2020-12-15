#[macro_use]
mod service;

mod config;
mod proto;
mod util;

pub use service::google::{firestore, pubsub};

pub fn init(project_id: impl Into<String>) {
    config::init(project_id);
}

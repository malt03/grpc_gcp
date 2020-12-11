mod auth;
mod config;
pub mod firestore;
mod proto;

pub fn init(project_id: impl Into<String>) {
    config::init(project_id);
}

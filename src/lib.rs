mod auth;
mod config;
mod proto;
mod service;
mod util;

use futures::future::BoxFuture;
pub use service::google::firestore::v1 as firestore;

pub async fn init(project_id: impl Into<String>) -> Result<(), Box<dyn std::error::Error>> {
    config::init(project_id);
    let all_futures: Vec<BoxFuture<Result<(), Box<dyn std::error::Error>>>> =
        vec![Box::pin(auth::init()), Box::pin(firestore::init())];
    for result in futures::future::join_all(all_futures).await {
        result.unwrap();
    }
    Ok(())
}

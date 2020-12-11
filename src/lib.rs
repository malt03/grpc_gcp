use futures::future::BoxFuture;

mod auth;
mod config;
pub mod firestore;
mod init_once;
mod proto;

pub async fn init(project_id: impl Into<String>) -> Result<(), Box<dyn std::error::Error>> {
    config::init(project_id);
    let all_futures: Vec<BoxFuture<Result<(), Box<dyn std::error::Error>>>> =
        vec![Box::pin(auth::init()), Box::pin(firestore::v1::init())];
    for result in futures::future::join_all(all_futures).await {
        result.unwrap();
    }
    Ok(())
}

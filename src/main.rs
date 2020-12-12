use grpc_gcp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let auth = gcp_auth::init().await?;

    // let token = auth
    //     .get_token(&["https://www.googleapis.com/auth/cloud-platform"])
    //     .await?;
    // println!("{}", token.as_str());
    grpc_gcp::init("malt03").await?;

    // let paths = vec!["/test/JM6W5nAExLohWQir079S", "/test/njavllVh8IctGwjyit2n"];

    // let mut all = Vec::new();
    // for path in paths.clone() {
    //     let future = grpc_gcp::firestore::v1::get_document(path);
    //     all.push(future);
    // }

    // let results = futures::future::join_all(all).await;
    // for result in results {
    //     let response = result.unwrap();
    //     println!("{:?}", response.get_ref().fields);
    // }

    let response = grpc_gcp::pubsub::v1::publish("test", "hoge".into()).await?;
    println!("{:?}", response.get_ref().message_ids);

    Ok(())
}

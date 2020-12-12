use grpc_gcp;
// use std::io::{self, BufRead};

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     grpc_gcp::init("babyfood").await?;

//     let paths = vec![
//         "/families/04vT4jWP1GhmqdAlfaD1",
//         "/familyExpires/0965H9XdH66Vcs6W7c2X",
//     ];

//     // loop {
//     //     let stdin = io::stdin();
//     //     for _ in stdin.lock().lines() {
//     let mut all = Vec::new();
//     for path in paths.clone() {
//         let future = grpc_gcp::firestore::v1::get_document(path);
//         all.push(future);
//     }

//     let results = futures::future::join_all(all).await;
//     for result in results {
//         let response = result.unwrap();
//         println!("{:?}", response.get_ref().fields);
//     }
//     //     }
//     // }

//     Ok(())
// }

use futures::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    grpc_gcp::init("babyfood").await?;

    let mut response = grpc_gcp::pubsub::v1::subscribe("test").await?;
    let streaming = response.get_mut();

    while let Some(result) = streaming.next().await {
        let response = result.unwrap();
        println!("{:?}", response);
    }

    Ok(())
}

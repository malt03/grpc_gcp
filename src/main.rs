// use std::convert::TryFrom;

// use grpc_gcp::firestore::v1::serde_document;
// use serde::Deserialize;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     grpc_gcp::init("malt03");

//     let paths = vec!["/test/JM6W5nAExLohWQir079S", "/test/njavllVh8IctGwjyit2n"];

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

//     Ok(())
// }

fn main() {
    println!("Ok");
}

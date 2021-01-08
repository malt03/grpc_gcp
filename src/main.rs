// use grpc_gcp::firestore::v1 as firestore;
// use serde::Deserialize;

// #[derive(Debug, Deserialize)]
// struct Document {
//     foo: String,
// }

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     grpc_gcp::init("malt03");

//     let a = firestore::collection("test").doc("JM6W5nAExLohWQir079S");
//     let b = firestore::collection("test").doc("njavllVh8IctGwjyit2n");

//     let mut all = Vec::new();
//     all.push(a.get());
//     all.push(b.get());

//     let results = futures::future::join_all(all).await;
//     for result in results {
//         let doc: Document = result.unwrap();
//         println!("{:?}", doc);
//     }

//     Ok(())
// }

use datastore::Datastore;
use grpc_gcp::datastore::v1 as datastore;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Document {
    foo: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    grpc_gcp::init("seconds-299513");

    let datastore = Datastore::new(None);
    let key = datastore.id("Test", 5634161670881280, None);

    let doc: Document = Datastore::get(key).await?;
    println!("{:?}", doc);

    Ok(())
}

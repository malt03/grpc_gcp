use std::convert::TryFrom;

use grpc_gcp::firestore::v1::serde_document;
use serde::Deserialize;

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

#[derive(Deserialize, Debug)]
struct Person {
    age: String,
    name: String,
}

fn main() {
    let i: i64 = 2147483647;
    let u: u8 = i as u8;
    // let u: u32 = TryFrom::try_from(i);
    println!("{}, {}", i, u);
    // let data = r#"{"name":"John Doe","age":"29"}"#;

    // let result: serde_document::Result<Person> = serde_document::from_fields(data);
    // match result {
    //     Err(e) => println!("{:?}", e),
    //     Ok(p) => println!("{:?}", p),
    // }
}

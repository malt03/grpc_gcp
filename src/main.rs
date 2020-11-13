use grpc_gcp;
use std::io::{self, BufRead};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    grpc_gcp::initialize("projectmap-develop");

    loop {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let path = line.unwrap();
            grpc_gcp::firestore::v1::get_document(path).await?;
        }
    }
}

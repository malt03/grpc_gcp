use grpc_gcp;
use std::io::{self, BufRead};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    grpc_gcp::initialize("projectmap-develop");

    let paths = vec![
        "/AllChatRoomUnreads/ExBBU90TPTR5DBg1Og0UFu4eCeZ2",
        "/Areas/0055a16bd49468b6df2cb8ea384c23ff",
    ];

    loop {
        let stdin = io::stdin();
        for _ in stdin.lock().lines() {
            let mut all = Vec::new();
            for path in paths.clone() {
                let future = grpc_gcp::firestore::v1::get_document(path);
                all.push(future);
            }

            let results = futures::future::join_all(all).await;
            for result in results {
                let response = result.unwrap();
                println!("{:?}", response.get_ref().fields);
            }
        }
    }
}

use grpc_gcp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    grpc_gcp::initialize("projectmap-develop");

    let paths = vec![
        "/AllChatRoomUnreads/ExBBU90TPTR5DBg1Og0UFu4eCeZ2",
        "/Areas/0055a16bd49468b6df2cb8ea384c23ff",
        "/ChatRooms/9VDq7RjLKhAiuRDbSeeT",
        "/Pins/04NkBxn906NOb7LpSzh9",
        "/Users/3CRKoFAOygcl7xK60WamxvcrjYY2",
    ];

    let mut all = Vec::new();
    for path in paths {
        let future = grpc_gcp::firestore::v1::get_document(path);
        all.push(future);
    }

    let results = futures::future::join_all(all).await;
    for result in results {
        let response = result.unwrap();
        println!("{:?}", response.get_ref().fields);
    }

    Ok(())
}

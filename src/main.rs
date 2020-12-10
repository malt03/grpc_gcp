use grpc_gcp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    grpc_gcp::initialize("babyfood");

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

// fn main() {
//     let vec = vec![1, 2, 3, 4, 5];

//     let mut rng = rand::thread_rng();
//     let mut handles = Vec::new();

//     for value in vec {
//         let wait: u64 = rng.gen_range(100, 200);
//         let handle = std::thread::spawn(move || {
//             std::thread::sleep(std::time::Duration::from_millis(wait));
//             println!("{}", value);
//             value * 2
//         });
//         handles.push(handle);
//     }

//     std::thread::sleep(std::time::Duration::from_secs(5));

//     let results: Vec<_> = handles
//         .into_iter()
//         .map(|handle| handle.join().unwrap())
//         .collect();
//     println!("{:?}", results);
// }

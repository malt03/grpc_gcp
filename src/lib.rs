pub mod firestore;
mod proto;
#[macro_use]
mod singleton;

pub async fn hoge() {
    // tonic::Request::new(message)
    // let request = proto::google::firestore::v1::GetDocumentRequest{
    //     name: "".to_string(),
    //     ..Default::default()
    // };
    // print!("{:?}", request);
}
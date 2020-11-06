pub mod firestore;
pub mod rpc {
    tonic::include_proto!("google.rpc");
}
pub mod r#type {
    tonic::include_proto!("google.r#type");
}

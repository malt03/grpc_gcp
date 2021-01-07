pub(crate) mod datastore;
pub(crate) mod firestore;
pub(crate) mod pubsub;
pub(crate) mod rpc {
    tonic::include_proto!("google.rpc");
}
pub(crate) mod r#type {
    tonic::include_proto!("google.r#type");
}

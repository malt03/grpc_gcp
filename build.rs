fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(false).compile(
        &[
            "proto/googleapis/google/datastore/v1/datastore.proto",
            "proto/googleapis/google/firestore/v1/firestore.proto",
            "proto/googleapis/google/pubsub/v1/pubsub.proto",
        ],
        &["proto/googleapis"],
    )?;
    Ok(())
}

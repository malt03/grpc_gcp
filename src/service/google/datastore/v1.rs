use crate::{
    config::project_id,
    proto::google::datastore::v1::{
        datastore_client::DatastoreClient, key::path_element::IdType, key::PathElement,
        run_query_request::QueryType, Key, LookupRequest, Query, RunQueryRequest,
    },
};
use serde::Deserialize;

const DOMAIN: &str = "datastore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";
define_client!(DatastoreClient);

#[derive(Deserialize, Debug)]
struct T {}

pub async fn lookup() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DatastoreClient::get().await?;
    let key = Key {
        path: vec![PathElement {
            kind: "Test".into(),
            id_type: Some(IdType::Id(5634161670881280)),
        }],
        ..Default::default()
    };
    let request = tonic::Request::new(LookupRequest {
        project_id: project_id().to_string(),
        keys: vec![key],
        ..Default::default()
    });
    let response = client.lookup(request).await?;
    // for found in response.into_inner().found {
    //     let properties = found.entity.unwrap().properties;
    //     let t: T = crate::serde_entity::deserializer::from_fields(properties).unwrap();
    //     println!("{:?}", properties);
    // }
    Ok(())
}

#[tokio::test]
async fn test() {
    crate::init("seconds-299513");
    lookup().await.unwrap();
}

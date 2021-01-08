mod error;
mod serde_properties;

use crate::{
    config::project_id,
    proto::google::datastore::v1::{
        self as datastore, datastore_client::DatastoreClient, key::path_element::IdType,
        key::PathElement, LookupRequest, PartitionId,
    },
};
pub use error::Error;

const DOMAIN: &str = "datastore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";
define_client!(DatastoreClient);

pub struct Key(datastore::Key);

impl Key {
    fn partition_id() -> PartitionId {
        PartitionId {
            project_id: project_id().into(),
            namespace_id: "".into(),
        }
    }

    fn path(kind: impl Into<String>, id_type: IdType, parent: Option<Key>) -> Vec<PathElement> {
        let mut path = match parent {
            None => Vec::new(),
            Some(key) => key.0.path,
        };
        path.push(datastore::key::PathElement {
            kind: kind.into(),
            id_type: Some(id_type),
        });
        path
    }

    pub fn name(kind: impl Into<String>, name: impl Into<String>, parent: Option<Key>) -> Key {
        let path = Self::path(kind, IdType::Name(name.into()), parent);
        Key(datastore::Key {
            partition_id: Some(Self::partition_id()),
            path: path,
        })
    }

    pub fn id(kind: impl Into<String>, id: i64, parent: Option<Key>) -> Key {
        let path = Self::path(kind, IdType::Id(id), parent);
        Key(datastore::Key {
            partition_id: Some(Self::partition_id()),
            path: path,
        })
    }
}

pub async fn get<'de, T>() -> Result<(), Box<dyn std::error::Error>>
where
    T: serde::Deserialize<'de>,
{
    let mut client = DatastoreClient::get().await?;
    let key = Key::id("Test", 5634161670881280, None);
    let request = tonic::Request::new(LookupRequest {
        project_id: project_id().to_string(),
        keys: vec![key.0],
        ..Default::default()
    });
    let response = client.lookup(request).await?;
    for found in response.into_inner().found {
        let properties = found.entity.unwrap().properties;
        let t: T = crate::serde_properties::deserializer::deserialize(properties).unwrap();
    }
    Ok(())
}

#[tokio::test]
async fn test() {
    crate::init("seconds-299513");
    // get().await.unwrap();
}

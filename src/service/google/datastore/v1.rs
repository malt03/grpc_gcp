mod error;
mod serde_properties;

use crate::{
    config::project_id,
    proto::google::datastore::v1::{
        self as datastore, datastore_client::DatastoreClient, key::path_element::IdType,
        key::PathElement, LookupRequest, PartitionId,
    },
    serde_properties::deserializer,
};
pub use error::Error;

const DOMAIN: &str = "datastore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";
define_client!(DatastoreClient);

pub struct Datastore {
    namespace_id: String,
}

impl Datastore {
    pub fn new(namespace: Option<String>) -> Datastore {
        Datastore {
            namespace_id: namespace.map(|s| s.into()).unwrap_or("".into()),
        }
    }

    pub fn name_key(
        &self,
        kind: impl Into<String>,
        name: impl Into<String>,
        parent: Option<Key>,
    ) -> Key {
        let path = path(kind, IdType::Name(name.into()), parent);
        Key(datastore::Key {
            partition_id: Some(partition_id(self.namespace_id.clone())),
            path: path,
        })
    }

    pub fn id_key(&self, kind: impl Into<String>, id: i64, parent: Option<Key>) -> Key {
        let path = path(kind, IdType::Id(id), parent);
        Key(datastore::Key {
            partition_id: Some(partition_id(self.namespace_id.clone())),
            path: path,
        })
    }

    pub async fn get<'de, T>(&self, key: Key) -> Result<T, Error>
    where
        T: serde::Deserialize<'de>,
    {
        let mut client = DatastoreClient::get().await?;
        let request = tonic::Request::new(LookupRequest {
            project_id: project_id().to_string(),
            keys: vec![key.0.clone()],
            ..Default::default()
        });
        let response = client.lookup(request).await?;
        let mut found = response.into_inner().found;
        if found.is_empty() {
            Err(Error::NotFound(key))
        } else {
            Ok(deserializer::deserialize(
                found.remove(0).entity.unwrap().properties,
            )?)
        }
    }
}

#[derive(Debug)]
pub struct Key(datastore::Key);

fn partition_id(namespace_id: String) -> PartitionId {
    PartitionId {
        project_id: project_id().into(),
        namespace_id: namespace_id,
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

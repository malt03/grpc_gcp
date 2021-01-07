use super::collection_reference::CollectionReference;
use crate::{
    config::project_id,
    firestore::v1::serde_document::from_fields,
    proto::google::firestore::v1::{firestore_client::FirestoreClient, GetDocumentRequest},
};
use serde::Deserialize;

#[derive(Clone)]
pub struct DocumentReference {
    id: String,
    parent: Box<CollectionReference>,
}

impl DocumentReference {
    pub(crate) fn new(id: impl Into<String>, parent: &CollectionReference) -> Self {
        DocumentReference {
            id: id.into(),
            parent: Box::new(parent.clone()),
        }
    }

    pub fn collection(&self, id: impl Into<String>) -> CollectionReference {
        CollectionReference::new(id.into(), Some(self))
    }

    pub fn path(&self) -> String {
        format!("{}/{}", self.parent.path(), self.id)
    }

    pub async fn get<'de, T>(&self) -> Result<T, Box<dyn std::error::Error>>
    where
        T: Deserialize<'de>,
    {
        let mut client = FirestoreClient::get().await?;

        let request = tonic::Request::new(GetDocumentRequest {
            name: format!(
                "projects/{}/databases/(default)/documents/{}",
                project_id(),
                self.path()
            )
            .to_string(),
            ..Default::default()
        });
        let response = client.get_document(request).await?;
        let result = from_fields(response.into_inner().fields)?;
        Ok(result)
    }
}

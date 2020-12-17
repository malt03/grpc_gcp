pub mod serde_document;

use crate::config::project_id;
use crate::proto::google::firestore::v1::{
    firestore_client::FirestoreClient, Document, GetDocumentRequest,
};

const DOMAIN: &str = "firestore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";
define_client!(FirestoreClient);

pub async fn get_document(
    path: impl Into<String>,
) -> Result<tonic::Response<Document>, Box<dyn std::error::Error>> {
    let mut client = FirestoreClient::get().await?;

    let request = tonic::Request::new(GetDocumentRequest {
        name: format!(
            "projects/{}/databases/(default)/documents{}",
            project_id(),
            path.into()
        )
        .to_string(),
        ..Default::default()
    });
    let response = client.get_document(request).await?;
    Ok(response)
}

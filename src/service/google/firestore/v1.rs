use once_cell::sync::Lazy;

use crate::{
    proto::google::firestore::v1::{
        firestore_client::FirestoreClient, Document, GetDocumentRequest,
    },
    util::init_once::InitOnce,
};

const DOMAIN: &str = "firestore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";

type Client = FirestoreClient<tonic::transport::Channel>;
static CHANNEL: Lazy<InitOnce<tonic::transport::Channel>> = Lazy::new(|| InitOnce::new());

pub(crate) async fn init() -> Result<(), Box<dyn std::error::Error>> {
    CHANNEL
        .init(|| async {
            Ok(crate::service::create_channel(DOMAIN).await?)
                as Result<tonic::transport::Channel, tonic::transport::Error>
        })
        .await?;
    Ok(())
}

async fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
    let channel = CHANNEL.get().await.clone();
    let token = crate::service::get_token(&[SCOPE]).await?;

    let bearer = format!("Bearer {}", token.as_str());
    let header_value = tonic::metadata::MetadataValue::from_str(&bearer)?;
    let client = FirestoreClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
        req.metadata_mut()
            .insert("authorization", header_value.clone());
        Ok(req)
    });
    Ok(client)
}

pub async fn get_document(
    path: impl Into<String>,
) -> Result<tonic::Response<Document>, Box<dyn std::error::Error>> {
    let mut client = get_client().await?;

    let request = tonic::Request::new(GetDocumentRequest {
        name: format!(
            "projects/{}/databases/(default)/documents{}",
            crate::config::project_id(),
            path.into()
        )
        .to_string(),
        ..Default::default()
    });
    let response = client.get_document(request).await?;
    Ok(response)
}

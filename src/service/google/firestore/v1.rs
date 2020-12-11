use once_cell::sync::Lazy;

use crate::{
    proto::google::firestore::v1::{
        firestore_client::FirestoreClient, Document, GetDocumentRequest,
    },
    util::init_once::InitOnce,
};

const URL: &str = "https://firestore.googleapis.com";
const DOMAIN: &str = "firestore.googleapis.com";

type Client = FirestoreClient<tonic::transport::Channel>;

static CHANNEL: Lazy<InitOnce<tonic::transport::Channel>> = Lazy::new(|| InitOnce::new());

pub(crate) async fn init() -> Result<(), Box<dyn std::error::Error>> {
    CHANNEL.init(create_channel).await?;
    Ok(())
}

async fn create_channel() -> Result<tonic::transport::Channel, tonic::transport::Error> {
    let tls = tonic::transport::ClientTlsConfig::new().domain_name(DOMAIN);
    let channel = tonic::transport::Channel::from_static(URL)
        .tls_config(tls)?
        .connect()
        .await?;
    Ok(channel)
}

async fn get_token() -> Result<gcp_auth::Token, Box<dyn std::error::Error>> {
    let authentication_manager = crate::auth::AUTHENTICATION_MANAGER.get().await;
    let token = authentication_manager
        .get_token(&["https://www.googleapis.com/auth/datastore"])
        .await?;
    Ok(token)
}

async fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
    let channel = CHANNEL.get().await.clone();
    let token = get_token().await?;

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

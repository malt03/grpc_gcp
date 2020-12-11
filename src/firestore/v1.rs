use once_cell::sync::{Lazy, OnceCell};
use tokio::sync::Mutex;

use crate::proto::google::firestore::v1::{
    firestore_client::FirestoreClient, Document, GetDocumentRequest,
};

const URL: &str = "https://firestore.googleapis.com";
const DOMAIN: &str = "firestore.googleapis.com";

type Client = FirestoreClient<tonic::transport::Channel>;

static CHANNEL: OnceCell<tonic::transport::Channel> = OnceCell::new();
static CHANNEL_INITIALIZED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

async fn get_token() -> Result<gcp_auth::Token, Box<dyn std::error::Error>> {
    let authentication_manager = crate::auth::get_authentication_manager().await?;
    let token = authentication_manager
        .get_token(&["https://www.googleapis.com/auth/datastore"])
        .await?;
    Ok(token)
}

async fn create_channel() -> Result<tonic::transport::Channel, Box<dyn std::error::Error>> {
    let tls = tonic::transport::ClientTlsConfig::new().domain_name(DOMAIN);
    let channel = tonic::transport::Channel::from_static(URL)
        .tls_config(tls)?
        .connect()
        .await?;
    Ok(channel)
}

async fn get_channel() -> Result<tonic::transport::Channel, Box<dyn std::error::Error>> {
    if let Some(channel) = CHANNEL.get() {
        return Ok(channel.clone());
    }
    let mut initialized = CHANNEL_INITIALIZED.lock().await;
    if !*initialized {
        let channel = create_channel().await?;
        CHANNEL.set(channel).unwrap();
        *initialized = true;
    }
    return Ok(CHANNEL.get().unwrap().clone());
}

async fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
    let channel = get_channel().await?;
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

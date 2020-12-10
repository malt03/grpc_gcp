use once_cell::sync::OnceCell;

use crate::proto::google::firestore::v1::{
    firestore_client::FirestoreClient, Document, GetDocumentRequest,
};

const URL: &str = "https://firestore.googleapis.com";
const DOMAIN: &str = "firestore.googleapis.com";

type Client = FirestoreClient<tonic::transport::Channel>;

static CHANNEL: OnceCell<tonic::transport::Channel> = OnceCell::new();
static AUTHENTICATION_MANAGER: OnceCell<gcp_auth::AuthenticationManager> = OnceCell::new();

async fn get_token() -> Result<gcp_auth::Token, Box<dyn std::error::Error>> {
    let authentication_manager = get_authentication_manager().await?;
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

async fn get_authentication_manager(
) -> Result<&'static gcp_auth::AuthenticationManager, Box<dyn std::error::Error>> {
    let manager = match AUTHENTICATION_MANAGER.get() {
        Some(manager) => manager,
        None => {
            let manager = gcp_auth::init().await?;
            if AUTHENTICATION_MANAGER.set(manager).is_err() {
                panic!("unexpected");
            }
            AUTHENTICATION_MANAGER.get().unwrap()
        }
    };
    Ok(manager)
}

async fn get_channel() -> Result<tonic::transport::Channel, Box<dyn std::error::Error>> {
    let channel = match CHANNEL.get() {
        Some(channel) => channel,
        None => {
            let channel = create_channel().await?;
            CHANNEL.set(channel).unwrap();
            CHANNEL.get().unwrap()
        }
    };
    Ok(channel.clone())
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
            crate::Config::shared().lock().unwrap().get_project_id(),
            path.into()
        )
        .to_string(),
        ..Default::default()
    });
    let response = client.get_document(request).await?;
    Ok(response)
}

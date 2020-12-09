use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};

use crate::proto::google::firestore::v1::{firestore_client::FirestoreClient, GetDocumentRequest};

const URL: &str = "https://firestore.googleapis.com";
const DOMAIN: &str = "firestore.googleapis.com";

type Client = FirestoreClient<tonic::transport::Channel>;

static CLIENT: OnceCell<Arc<Mutex<Client>>> = OnceCell::new();

async fn get_token() -> Result<gcp_auth::Token, Box<dyn std::error::Error>> {
    let authentication_manager = gcp_auth::init().await?;
    let token = authentication_manager
        .get_token(&["https://www.googleapis.com/auth/datastore"])
        .await?;
    Ok(token)
}

async fn create_client() -> Result<Client, Box<dyn std::error::Error>> {
    let tls = tonic::transport::ClientTlsConfig::new().domain_name(DOMAIN);
    let channel = tonic::transport::Channel::from_static(URL)
        .tls_config(tls)?
        .connect()
        .await?;
    let token = get_token().await?;
    let bearer = format!("Bearer {}", token.as_str());
    let header_value = tonic::metadata::MetadataValue::from_str(&bearer)?;
    let c = FirestoreClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
        req.metadata_mut()
            .insert("authorization", header_value.clone());
        Ok(req)
    });
    Ok(c)
}

async fn get_client() -> Result<&'static Arc<Mutex<Client>>, Box<dyn std::error::Error>> {
    let c = match CLIENT.get() {
        Some(client) => client,
        None => {
            let client = create_client().await?;
            CLIENT.set(Arc::new(Mutex::new(client))).unwrap();
            CLIENT.get().unwrap()
        }
    };
    Ok(c)
}

pub async fn get_document(path: impl Into<String>) -> Result<(), Box<dyn std::error::Error>> {
    let c = get_client().await?;
    let mut client = c.lock().unwrap();

    let request = tonic::Request::new(GetDocumentRequest {
        name: format!(
            "projects/{}/databases/(default)/documents/{}",
            crate::Config::shared().lock().unwrap().get_project_id(),
            path.into()
        )
        .to_string(),
        ..Default::default()
    });
    let response = client.get_document(request).await?;
    println!("{:?}", response.get_ref().fields);

    Ok(())
}

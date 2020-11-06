use crate::proto::google::firestore::v1::{firestore_client::FirestoreClient, GetDocumentRequest};

const URL: &str = "https://firestore.googleapis.com";
const DOMAIN: &str = "firestore.googleapis.com";

pub async fn get_document(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let authentication_manager = gcp_auth::init().await?;
    let token = authentication_manager
        .get_token(&["https://www.googleapis.com/auth/datastore"])
        .await?;

    let bearer = format!("Bearer {}", token.as_str());
    let header_value = tonic::metadata::MetadataValue::from_str(&bearer)?;
    let tls = tonic::transport::ClientTlsConfig::new().domain_name(DOMAIN);
    let channel = tonic::transport::Channel::from_static(URL)
        .tls_config(tls)?
        .connect()
        .await?;
    let mut client =
        FirestoreClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
            req.metadata_mut()
                .insert("authorization", header_value.clone());
            Ok(req)
        });

    let request = tonic::Request::new(GetDocumentRequest {
        name: format!(
            "projects/projectmap-develop/databases/(default)/documents/{}",
            path
        )
        .to_string(),
        ..Default::default()
    });
    let response = client.get_document(request).await?;
    println!("{:?}", response.get_ref());

    Ok(())
}

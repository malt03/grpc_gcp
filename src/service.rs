#[macro_use]
mod macros;

pub mod auth;
pub mod google;

pub(crate) async fn create_channel(
    domain: &str,
) -> Result<tonic::transport::Channel, tonic::transport::Error> {
    let tls = tonic::transport::ClientTlsConfig::new().domain_name(domain);
    let uri: http::uri::Uri = ["https://", domain].concat().parse().unwrap();
    let channel = tonic::transport::Channel::builder(uri)
        .tls_config(tls)?
        .connect()
        .await?;
    Ok(channel)
}

pub(crate) async fn get_token(
    scopes: &[&str],
) -> Result<gcp_auth::Token, Box<dyn std::error::Error>> {
    let authentication_manager = auth::AUTHENTICATION_MANAGER.get().await?;
    let token = authentication_manager.get_token(scopes).await?;
    Ok(token)
}

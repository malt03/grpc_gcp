use crate::proto::google::firestore::v1::{firestore_client::FirestoreClient, GetDocumentRequest};

const URL: &str = "https://firestore.googleapis.com";
const DOMAIN: &str = "firestore.googleapis.com";

type Client = FirestoreClient<tonic::transport::Channel>;

#[derive(Debug)]
struct ConnectionPool {
    clients: Vec<Client>,
}

impl ConnectionPool {
    crate::singleton!(ConnectionPool);
    fn new() -> ConnectionPool {
        ConnectionPool {
            clients: Vec::new(),
        }
    }

    async fn get_token(&mut self) -> Result<gcp_auth::Token, Box<dyn std::error::Error>> {
        let authentication_manager = gcp_auth::init().await?;
        let token = authentication_manager
            .get_token(&["https://www.googleapis.com/auth/datastore"])
            .await?;
        Ok(token)
    }

    async fn borrow_client(&mut self) -> Result<Client, Box<dyn std::error::Error>> {
        if let Some(client) = self.clients.pop() {
            println!("reuse client");
            return Ok(client);
        }
        println!("create client");
        let token = self.get_token().await?;
        let bearer = format!("Bearer {}", token.as_str());
        let header_value = tonic::metadata::MetadataValue::from_str(&bearer)?;
        let tls = tonic::transport::ClientTlsConfig::new().domain_name(DOMAIN);
        let channel = tonic::transport::Channel::from_static(URL)
            .tls_config(tls)?
            .connect()
            .await?;
        let client =
            FirestoreClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
                req.metadata_mut()
                    .insert("authorization", header_value.clone());
                Ok(req)
            });
        Ok(client)
    }

    fn return_client(&mut self, client: Client) {
        self.clients.push(client);
    }
}

pub async fn get_document(path: impl Into<String>) -> Result<(), Box<dyn std::error::Error>> {
    let pool = ConnectionPool::shared();

    let mut client = pool.lock().unwrap().borrow_client().await?;

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

    pool.lock().unwrap().return_client(client);
    Ok(())
}

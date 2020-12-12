mod models;

use crate::config::project_id;
use models::Message;
use tonic::{Request, Response};

use crate::proto::google::pubsub::v1::{
    publisher_client::PublisherClient, PublishRequest, PublishResponse,
};

const DOMAIN: &str = "pubsub.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/pubsub";

define_client!(PublisherClient);

pub async fn publish(
    topic: impl Into<String>,
    message: Message,
) -> Result<Response<PublishResponse>, Box<dyn std::error::Error>> {
    let mut client = PublisherClient::get().await?;

    let message = PublishRequest {
        topic: format!("projects/{}/topics/{}", project_id(), topic.into()),
        messages: vec![message.to_tonic()],
    };

    let request = Request::new(message);
    let response = client.publish(request).await?;
    Ok(response)
}

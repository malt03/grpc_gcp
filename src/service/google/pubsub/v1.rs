use std::collections::HashMap;

use crate::proto::google::pubsub::v1::{
    publisher_client::PublisherClient, PublishRequest, PublishResponse, PubsubMessage,
};

const DOMAIN: &str = "firestore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";

define_client!(PublisherClient);

pub async fn publish(
    topic: impl Into<String>,
    data: Vec<u8>,
) -> Result<tonic::Response<PublishResponse>, Box<dyn std::error::Error>> {
    let mut client = PublisherClient::get().await?;

    let message = PubsubMessage {
        data: data,
        attributes: HashMap::new(),
        message_id: "".into(),
        publish_time: None,
        ordering_key: "".into(),
    };
    let message = PublishRequest {
        topic: topic.into(),
        messages: vec![message],
    };

    let request = tonic::Request::new(message);
    let response = client.publish(request).await?;
    Ok(response)
}

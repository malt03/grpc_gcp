use crate::config::project_id;
use futures::stream;
use std::collections::HashMap;
use tonic::{Request, Response, Streaming};

use crate::proto::google::pubsub::v1::{
    publisher_client::PublisherClient, subscriber_client::SubscriberClient, PublishRequest,
    PublishResponse, PubsubMessage, StreamingPullRequest, StreamingPullResponse,
};

const DOMAIN: &str = "firestore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";

define_client!(PublisherClient, SubscriberClient);

pub async fn publish(
    topic: impl Into<String>,
    data: Vec<u8>,
) -> Result<Response<PublishResponse>, Box<dyn std::error::Error>> {
    let mut client = PublisherClient::get().await?;

    let message = PubsubMessage {
        data: data,
        attributes: HashMap::new(),
        message_id: "".into(),
        publish_time: None,
        ordering_key: "".into(),
    };
    let message = PublishRequest {
        topic: format!("projects/{}/topics/{}", project_id(), topic.into()),
        messages: vec![message],
    };

    let request = Request::new(message);
    let response = client.publish(request).await?;
    Ok(response)
}

pub async fn subscribe(
    subscription: impl Into<String>,
) -> Result<Response<Streaming<StreamingPullResponse>>, Box<dyn std::error::Error>> {
    let mut client = SubscriberClient::get().await?;
    let message = StreamingPullRequest {
        subscription: format!(
            "projects/{}/subscriptions/{}",
            project_id(),
            subscription.into()
        ),
        ..Default::default()
    };
    let request = stream::once(async { message });
    let response = client.streaming_pull(request).await?;
    Ok(response)
}

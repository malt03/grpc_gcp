use crate::proto::google::pubsub::v1::{
    publisher_client::PublisherClient, subscriber_client::SubscriberClient,
};

const DOMAIN: &str = "firestore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";

define_client!(PublisherClient, SubscriberClient);

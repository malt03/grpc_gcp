use std::collections::HashMap;

use crate::proto::google::pubsub::v1::PubsubMessage;

pub struct Message {
    data: Vec<u8>,
    attributes: HashMap<String, String>,
}

impl From<&str> for Message {
    fn from(s: &str) -> Self {
        Message {
            data: s.to_string().into_bytes(),
            attributes: HashMap::new(),
        }
    }
}

impl Message {
    pub(crate) fn to_tonic(self) -> PubsubMessage {
        PubsubMessage {
            data: self.data,
            attributes: self.attributes,
            ..Default::default()
        }
    }
}

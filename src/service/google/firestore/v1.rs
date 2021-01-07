mod extensions;
mod models;
pub mod serde_document;
pub use models::{CollectionReference, DocumentReference};

use crate::proto::google::firestore::v1::firestore_client::FirestoreClient;

const DOMAIN: &str = "firestore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";
define_client!(FirestoreClient);

pub fn collection(id: impl Into<String>) -> CollectionReference {
    CollectionReference::new(id, None)
}

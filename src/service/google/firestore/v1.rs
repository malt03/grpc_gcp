pub mod error;
mod models;
mod serde_fields;

pub use error::Error;
use models::CollectionReference;

use crate::proto::google::firestore::v1::firestore_client::FirestoreClient;

const DOMAIN: &str = "firestore.googleapis.com";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";
define_client!(FirestoreClient);

pub fn collection(id: impl Into<String>) -> CollectionReference {
    CollectionReference::new(id, None)
}

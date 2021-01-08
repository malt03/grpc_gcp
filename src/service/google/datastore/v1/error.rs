use crate::serde_properties;
use gcp_auth::GCPAuthError;

#[derive(Debug)]
pub enum Error {
    Auth(GCPAuthError),
    Transport(tonic::transport::Error),
    Status(tonic::Status),
    Deserialize(serde_properties::deserializer::Error),
    NotFound(super::Key),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Auth(e) => e.fmt(f),
            Error::Transport(e) => e.fmt(f),
            Error::Status(e) => e.fmt(f),
            Error::Deserialize(e) => e.fmt(f),
            Error::NotFound(key) => write!(f, "Not Found: {:?}", key),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<GCPAuthError> for Error {
    fn from(err: GCPAuthError) -> Self {
        Error::Auth(err)
    }
}

impl From<tonic::transport::Error> for Error {
    fn from(err: tonic::transport::Error) -> Self {
        Error::Transport(err)
    }
}

impl From<tonic::Status> for Error {
    fn from(err: tonic::Status) -> Self {
        Error::Status(err)
    }
}

impl From<serde_properties::deserializer::Error> for Error {
    fn from(err: serde_properties::deserializer::Error) -> Self {
        Error::Deserialize(err)
    }
}

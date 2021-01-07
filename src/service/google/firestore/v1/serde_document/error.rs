use super::TraceKey;
use fmt::Debug;
use serde::{de, ser};
use std;
use std::fmt::{self, Display};
pub type Result<T> = std::result::Result<T, Error>;
use crate::proto::google::firestore::v1::Value;

#[derive(Clone, PartialEq)]
pub enum Error {
    Message(String),

    Eof,

    ExpectedBoolean(TraceKey, Value),
    ExpectedInteger(TraceKey, Value),
    ExpectedDouble(TraceKey, Value),
    ExpectedString(TraceKey, Value),
    ExpectedBytes(TraceKey, Value),
    ExpectedNull(TraceKey, Value),
    ExpectedArray(TraceKey, Value),
    ExpectedMap(TraceKey, Value),
    ExpectedEnum(TraceKey, Value),
    CouldNotConvertNumber(TraceKey, Value),
    ExpectedArrayEnd(TraceKey),
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Error {
    fn to_string(&self) -> String {
        match self {
            Error::Message(msg) => msg.to_string(),
            Error::Eof => "unexpected end of input".into(),

            Error::ExpectedBoolean(key, value) => format!(
                "A boolean value was expected for {}, but it was {}",
                key, value
            ),
            Error::ExpectedInteger(key, value) => format!(
                "A integer value was expected for {}, but it was {}",
                key, value
            ),
            Error::ExpectedDouble(key, value) => format!(
                "A double value was expected for {}, but it was {}",
                key, value
            ),
            Error::ExpectedString(key, value) => format!(
                "A string value was expected for {}, but it was {}",
                key, value
            ),
            Error::ExpectedBytes(key, value) => format!(
                "A bytes value was expected for {}, but it was {}",
                key, value
            ),
            Error::ExpectedNull(key, value) => format!(
                "A null value was expected for {}, but it was {}",
                key, value
            ),
            Error::ExpectedArray(key, value) => format!(
                "A array value was expected for {}, but it was {}",
                key, value
            ),
            Error::ExpectedMap(key, value) => {
                format!("A map value was expected for {}, but it was {}", key, value)
            }
            Error::ExpectedEnum(key, value) => format!(
                "A enum value was expected for {}, but it was {}",
                key, value
            ),
            Error::CouldNotConvertNumber(key, value) => format!(
                "Could not convert {}, the value of {}, to the expected type.",
                value, key
            ),
            Error::ExpectedArrayEnd(key) => {
                format!("The length of the array is invalid. key: {}", key)
            }
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl std::error::Error for Error {}

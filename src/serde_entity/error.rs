use fmt::Debug;
use serde::{de, ser};
use std;
use std::fmt::{self, Display};
pub(crate) type Result<T, Value> = std::result::Result<T, Error<Value>>;

use super::common::TraceKey;

#[derive(Clone, PartialEq)]
pub(crate) enum Error<Value: Display> {
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

impl<Value: Display> ser::Error for Error<Value> {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl<Value: Display> de::Error for Error<Value> {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl<Value: Display> Error<Value> {
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

impl<Value: Display> Debug for Error<Value> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl<Value: Display> Display for Error<Value> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl<Value: Display> std::error::Error for Error<Value> {}

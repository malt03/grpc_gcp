use super::value::ValueTrait;
use std::fmt::Display;

#[derive(Clone, PartialEq)]
pub enum TraceKey {
    Root,
    Map(String, Box<TraceKey>),
    Array(Box<TraceKey>),
}

impl Display for TraceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TraceKey::Root => write!(f, ""),
            TraceKey::Map(key, parent) => write!(f, "{}/{}", parent, key),
            TraceKey::Array(parent) => write!(f, "{}[]", parent),
        }
    }
}

pub(crate) struct KeyValueSet<Value: ValueTrait>(pub(crate) TraceKey, pub(crate) Value);

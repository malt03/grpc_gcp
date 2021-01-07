use crate::proto::google::firestore::v1::{value::ValueType, ArrayValue, MapValue, Value};
use std::{collections::HashMap, fmt::Display, iter::FromIterator};

impl Value {
    pub(crate) fn from_fields(input: HashMap<String, Value>) -> Self {
        Value {
            value_type: Some(ValueType::MapValue(MapValue { fields: input })),
        }
    }

    pub(crate) fn new(value_type: ValueType) -> Self {
        Value {
            value_type: Some(value_type),
        }
    }

    pub(crate) fn integer(value: i64) -> Self {
        Value::new(ValueType::IntegerValue(value))
    }

    pub(crate) fn double(value: f64) -> Value {
        Value::new(ValueType::DoubleValue(value))
    }

    pub(crate) fn integer_value(&self) -> Option<i64> {
        match self.value_type.as_ref().unwrap() {
            ValueType::IntegerValue(i) => Some(*i),
            ValueType::TimestampValue(t) => Some((*t).seconds),
            _ => None,
        }
    }

    pub(crate) fn byte_value(self) -> Option<Vec<u8>> {
        match self.value_type.unwrap() {
            ValueType::BytesValue(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn array_value(self) -> Option<ArrayValue> {
        match self.value_type.unwrap() {
            ValueType::ArrayValue(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn has_map_value(&self) -> bool {
        match self.value_type.as_ref().unwrap() {
            ValueType::MapValue(_) | ValueType::GeoPointValue(_) | ValueType::TimestampValue(_) => {
                true
            }
            _ => false,
        }
    }

    pub(crate) fn map_value(self) -> Option<HashMap<String, Value>> {
        match self.value_type.unwrap() {
            ValueType::MapValue(value) => Some(value.fields),
            ValueType::GeoPointValue(value) => {
                let map = HashMap::from_iter(vec![
                    ("latitude".into(), Value::double(value.latitude)),
                    ("longitude".into(), Value::double(value.longitude)),
                ]);
                Some(map)
            }
            ValueType::TimestampValue(value) => {
                let map = HashMap::from_iter(vec![
                    ("seconds".into(), Value::integer(value.seconds)),
                    ("nanos".into(), Value::integer(value.nanos.into())),
                ]);
                Some(map)
            }
            _ => None,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.value_type.as_ref().unwrap() {
            ValueType::NullValue(value) => write!(f, "Null {:?}", value),
            ValueType::BooleanValue(value) => write!(f, "Boolean {:?}", value),
            ValueType::IntegerValue(value) => write!(f, "Integer {:?}", value),
            ValueType::DoubleValue(value) => write!(f, "Double {:?}", value),
            ValueType::TimestampValue(value) => write!(f, "Timestamp {:?}", value),
            ValueType::StringValue(value) => write!(f, "String {:?}", value),
            ValueType::BytesValue(value) => write!(f, "Bytes {:?}", value),
            ValueType::ReferenceValue(value) => write!(f, "Reference {:?}", value),
            ValueType::GeoPointValue(value) => write!(f, "GeoPoint {:?}", value),
            ValueType::ArrayValue(value) => write!(f, "Array {:?}", value),
            ValueType::MapValue(value) => write!(f, "Map {:?}", value),
        }
    }
}

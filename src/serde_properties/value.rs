use crate::proto::google::datastore::v1::{key::path_element::IdType, key::PathElement, Key};
use prost_types::Timestamp;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    iter::FromIterator,
};

pub(crate) enum ValueType<Value: ValueTrait> {
    NullValue(i32),
    BooleanValue(bool),
    IntegerValue(i64),
    DoubleValue(f64),
    TimestampValue(::prost_types::Timestamp),
    StringValue(std::string::String),
    BytesValue(std::vec::Vec<u8>),
    ReferenceValue(std::string::String),
    GeoPointValue(Value::LatLng),
    ArrayValue(Value::ArrayValue),
    MapValue(Value::MapValue),
    KeyValue(Key),
}

pub(crate) enum ValueTypeRef<'a, Value: ValueTrait> {
    NullValue(&'a i32),
    BooleanValue(&'a bool),
    IntegerValue(&'a i64),
    DoubleValue(&'a f64),
    TimestampValue(&'a ::prost_types::Timestamp),
    StringValue(&'a std::string::String),
    BytesValue(&'a std::vec::Vec<u8>),
    ReferenceValue(&'a std::string::String),
    GeoPointValue(&'a Value::LatLng),
    ArrayValue(&'a Value::ArrayValue),
    MapValue(&'a Value::MapValue),
    KeyValue(&'a Key),
}

impl<'a, Value: ValueTrait> ValueTypeRef<'a, Value> {
    pub fn is_some_value(&self) -> bool {
        if let ValueTypeRef::NullValue(_) = self {
            false
        } else {
            true
        }
    }
}

pub(crate) trait MapValueTrait<Value: ValueTrait> {
    fn get_fields(self) -> HashMap<String, Value>;
    fn new(fields: HashMap<String, Value>) -> Self;
}

pub(crate) trait ArrayValueTrait<Value: ValueTrait> {
    fn get_values(self) -> Vec<Value>;
    fn new(values: Vec<Value>) -> Self;
}

pub(crate) trait LatLngTrait {
    fn get_latitude(&self) -> f64;
    fn get_longitude(&self) -> f64;

    fn map_value<Value: ValueTrait>(&self) -> HashMap<String, Value> {
        HashMap::from_iter(vec![
            (
                "latitude".into(),
                Value::new(ValueType::DoubleValue(self.get_latitude())),
            ),
            (
                "longitude".into(),
                Value::new(ValueType::DoubleValue(self.get_longitude())),
            ),
        ])
    }
}

trait MapValue {
    fn map_value<Value: ValueTrait>(&self) -> HashMap<String, Value>;
}

impl MapValue for Timestamp {
    fn map_value<Value: ValueTrait>(&self) -> HashMap<String, Value> {
        HashMap::from_iter(vec![
            (
                "seconds".into(),
                Value::new(ValueType::IntegerValue(self.seconds)),
            ),
            (
                "nanos".into(),
                Value::new(ValueType::IntegerValue(self.nanos.into())),
            ),
        ])
    }
}

impl IdType {
    fn map_value<Value: ValueTrait>(&self) -> HashMap<String, Value> {
        match self {
            IdType::Id(id) => HashMap::from_iter(vec![(
                "Id".into(),
                Value::new(ValueType::IntegerValue(*id)),
            )]),
            IdType::Name(name) => HashMap::from_iter(vec![(
                "Name".into(),
                Value::new(ValueType::StringValue(name.clone())),
            )]),
        }
    }
}

impl PathElement {
    fn map_value<Value: ValueTrait>(&self) -> HashMap<String, Value> {
        let id_type = match self.id_type.as_ref() {
            Some(id_type) => ValueType::MapValue(Value::MapValue::new(id_type.map_value())),
            None => ValueType::NullValue(0),
        };
        HashMap::from_iter(vec![
            (
                "kind".into(),
                Value::new(ValueType::StringValue(self.kind.clone())),
            ),
            ("id_type".into(), Value::new(id_type)),
        ])
    }
}

impl Key {
    fn map_value<Value: ValueTrait>(&self) -> HashMap<String, Value> {
        let partition_id = match self.partition_id.as_ref() {
            Some(partition_id) => {
                let map: HashMap<String, Value> = HashMap::from_iter(vec![
                    (
                        "project_id".into(),
                        Value::new(ValueType::StringValue(partition_id.project_id.clone())),
                    ),
                    (
                        "namespace_id".into(),
                        Value::new(ValueType::StringValue(partition_id.namespace_id.clone())),
                    ),
                ]);
                ValueType::MapValue(Value::MapValue::new(map))
            }
            None => ValueType::NullValue(0),
        };
        let path: Vec<_> = self
            .path
            .iter()
            .map(|element| {
                Value::new(ValueType::MapValue(Value::MapValue::new(
                    element.map_value(),
                )))
            })
            .collect();
        HashMap::from_iter(vec![
            ("partition_id".into(), Value::new(partition_id)),
            (
                "path".into(),
                Value::new(ValueType::ArrayValue(Value::ArrayValue::new(path))),
            ),
        ])
    }
}

pub(crate) trait ValueTrait: Sized + Display + 'static {
    type LatLng: LatLngTrait + Debug;
    type ArrayValue: ArrayValueTrait<Self> + Debug;
    type MapValue: MapValueTrait<Self> + Debug;

    fn from(input: HashMap<String, Self>) -> Self;
    fn new(value_type: ValueType<Self>) -> Self;

    fn get_value_type<'s>(&'s self) -> Option<ValueTypeRef<Self>>;
    fn into_value_type(self) -> Option<ValueType<Self>>;

    fn integer_value(&self) -> Option<i64> {
        match self.get_value_type().unwrap() {
            ValueTypeRef::IntegerValue(i) => Some(*i),
            ValueTypeRef::TimestampValue(t) => Some((*t).seconds),
            _ => None,
        }
    }

    fn byte_value(self) -> Option<Vec<u8>> {
        match self.into_value_type().unwrap() {
            ValueType::BytesValue(value) => Some(value),
            _ => None,
        }
    }

    fn array_value(self) -> Option<Self::ArrayValue> {
        match self.into_value_type().unwrap() {
            ValueType::ArrayValue(value) => Some(value),
            _ => None,
        }
    }

    fn has_map_value(&self) -> bool {
        match self.get_value_type().unwrap() {
            ValueTypeRef::MapValue(_)
            | ValueTypeRef::GeoPointValue(_)
            | ValueTypeRef::TimestampValue(_) => true,
            _ => false,
        }
    }

    fn map_value(self) -> Option<HashMap<String, Self>> {
        match self.into_value_type().unwrap() {
            ValueType::MapValue(value) => Some(value.get_fields()),
            ValueType::GeoPointValue(value) => Some(value.map_value()),
            ValueType::TimestampValue(value) => Some(value.map_value()),
            ValueType::KeyValue(value) => Some(value.map_value()),
            _ => None,
        }
    }

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.get_value_type().unwrap() {
            ValueTypeRef::NullValue(value) => write!(f, "Null {:?}", value),
            ValueTypeRef::BooleanValue(value) => write!(f, "Boolean {:?}", value),
            ValueTypeRef::IntegerValue(value) => write!(f, "Integer {:?}", value),
            ValueTypeRef::DoubleValue(value) => write!(f, "Double {:?}", value),
            ValueTypeRef::TimestampValue(value) => write!(f, "Timestamp {:?}", value),
            ValueTypeRef::StringValue(value) => write!(f, "String {:?}", value),
            ValueTypeRef::BytesValue(value) => write!(f, "Bytes {:?}", value),
            ValueTypeRef::ReferenceValue(value) => write!(f, "Reference {:?}", value),
            ValueTypeRef::GeoPointValue(value) => write!(f, "GeoPoint {:?}", value),
            ValueTypeRef::ArrayValue(value) => write!(f, "Array {:?}", value),
            ValueTypeRef::MapValue(value) => write!(f, "Map {:?}", value),
            ValueTypeRef::KeyValue(value) => write!(f, "Key {:?}", value),
        }
    }
}

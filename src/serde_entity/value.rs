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
}

pub(crate) trait ArrayValueTrait<Value: ValueTrait> {
    fn get_values(self) -> Vec<Value>;
}

pub(crate) trait LatLngTrait {
    fn get_latitude(&self) -> f64;
    fn get_longitude(&self) -> f64;
}

pub(crate) trait ValueTrait: Sized + Display + 'static {
    type LatLng: LatLngTrait + Debug;
    type ArrayValue: ArrayValueTrait<Self> + Debug;
    type MapValue: MapValueTrait<Self> + Debug;

    fn from_fields(input: HashMap<String, Self>) -> Self;
    fn new(value_type: ValueType<Self>) -> Self;

    fn get_value_type<'s>(&'s self) -> Option<ValueTypeRef<Self>>;
    fn into_value_type(self) -> Option<ValueType<Self>>;

    fn integer(value: i64) -> Self {
        Self::new(ValueType::IntegerValue(value))
    }

    fn double(value: f64) -> Self {
        Self::new(ValueType::DoubleValue(value))
    }

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
            ValueType::GeoPointValue(value) => {
                let map = HashMap::from_iter(vec![
                    ("latitude".into(), Self::double(value.get_latitude())),
                    ("longitude".into(), Self::double(value.get_longitude())),
                ]);
                Some(map)
            }
            ValueType::TimestampValue(value) => {
                let map = HashMap::from_iter(vec![
                    ("seconds".into(), Self::integer(value.seconds)),
                    ("nanos".into(), Self::integer(value.nanos.into())),
                ]);
                Some(map)
            }
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
        }
    }
}

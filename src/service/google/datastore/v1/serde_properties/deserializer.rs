use crate::{
    proto::google::datastore::v1::{value::ValueType, ArrayValue, Entity, Value},
    serde_properties,
};
use std::collections::HashMap;

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.value_type.as_ref().unwrap() {
            ValueType::NullValue(v) => write!(f, "Null {:?}", v),
            ValueType::BooleanValue(v) => write!(f, "Boolean {:?}", v),
            ValueType::IntegerValue(v) => write!(f, "Integer {:?}", v),
            ValueType::DoubleValue(v) => write!(f, "Double {:?}", v),
            ValueType::TimestampValue(v) => write!(f, "Timestamp {:?}", v),
            ValueType::StringValue(v) => write!(f, "String {:?}", v),
            ValueType::BlobValue(v) => write!(f, "Bytes {:?}", v),
            ValueType::KeyValue(v) => write!(f, "Key {:?}", v),
            ValueType::GeoPointValue(v) => write!(f, "GeoPoint {:?}", v),
            ValueType::ArrayValue(v) => write!(f, "Array {:?}", v),
            ValueType::EntityValue(v) => write!(f, "Map {:?}", v),
        }
    }
}

impl serde_properties::ArrayValueTrait<Value> for ArrayValue {
    fn get_values(self) -> Vec<Value> {
        self.values
    }

    fn new(values: Vec<Value>) -> Self {
        ArrayValue { values }
    }
}

impl serde_properties::MapValueTrait<Value> for Entity {
    fn get_fields(self) -> HashMap<String, Value> {
        self.properties
    }

    fn new(fields: HashMap<String, Value>) -> Self {
        Entity {
            properties: fields,
            ..Default::default()
        }
    }
}

impl serde_properties::ValueTrait for Value {
    type LatLng = crate::proto::google::r#type::LatLng;
    type ArrayValue = ArrayValue;
    type MapValue = Entity;

    fn from(input: HashMap<String, Self>) -> Self {
        Value {
            value_type: Some(ValueType::EntityValue(Entity {
                properties: input,
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    fn new(value_type: serde_properties::ValueType<Self>) -> Self {
        let value_type = match value_type {
            serde_properties::ValueType::NullValue(value) => ValueType::NullValue(value),
            serde_properties::ValueType::BooleanValue(value) => ValueType::BooleanValue(value),
            serde_properties::ValueType::IntegerValue(value) => ValueType::IntegerValue(value),
            serde_properties::ValueType::DoubleValue(value) => ValueType::DoubleValue(value),
            serde_properties::ValueType::TimestampValue(value) => ValueType::TimestampValue(value),
            serde_properties::ValueType::StringValue(value) => ValueType::StringValue(value),
            serde_properties::ValueType::BytesValue(value) => ValueType::BlobValue(value),
            serde_properties::ValueType::ReferenceValue(_) => common_panic!(),
            serde_properties::ValueType::GeoPointValue(value) => ValueType::GeoPointValue(value),
            serde_properties::ValueType::ArrayValue(value) => ValueType::ArrayValue(value),
            serde_properties::ValueType::MapValue(value) => ValueType::EntityValue(value),
            serde_properties::ValueType::KeyValue(value) => ValueType::KeyValue(value),
        };
        Value {
            value_type: Some(value_type),
            ..Default::default()
        }
    }

    fn into_value_type(self) -> Option<serde_properties::ValueType<Self>> {
        self.value_type.map(|value_type| match value_type {
            ValueType::NullValue(value) => serde_properties::ValueType::NullValue(value),
            ValueType::BooleanValue(value) => serde_properties::ValueType::BooleanValue(value),
            ValueType::IntegerValue(value) => serde_properties::ValueType::IntegerValue(value),
            ValueType::DoubleValue(value) => serde_properties::ValueType::DoubleValue(value),
            ValueType::TimestampValue(value) => serde_properties::ValueType::TimestampValue(value),
            ValueType::StringValue(value) => serde_properties::ValueType::StringValue(value),
            ValueType::BlobValue(value) => serde_properties::ValueType::BytesValue(value),
            ValueType::KeyValue(value) => serde_properties::ValueType::KeyValue(value),
            ValueType::GeoPointValue(value) => serde_properties::ValueType::GeoPointValue(value),
            ValueType::ArrayValue(value) => serde_properties::ValueType::ArrayValue(value),
            ValueType::EntityValue(value) => serde_properties::ValueType::MapValue(value),
        })
    }

    fn get_value_type<'s>(&'s self) -> Option<serde_properties::ValueTypeRef<Self>> {
        self.value_type.as_ref().map(|value_type| match value_type {
            ValueType::NullValue(value) => serde_properties::ValueTypeRef::NullValue(value),
            ValueType::BooleanValue(value) => serde_properties::ValueTypeRef::BooleanValue(value),
            ValueType::IntegerValue(value) => serde_properties::ValueTypeRef::IntegerValue(value),
            ValueType::DoubleValue(value) => serde_properties::ValueTypeRef::DoubleValue(value),
            ValueType::TimestampValue(value) => {
                serde_properties::ValueTypeRef::TimestampValue(value)
            }
            ValueType::StringValue(value) => serde_properties::ValueTypeRef::StringValue(value),
            ValueType::BlobValue(value) => serde_properties::ValueTypeRef::BytesValue(value),
            ValueType::KeyValue(value) => serde_properties::ValueTypeRef::KeyValue(value),
            ValueType::GeoPointValue(value) => serde_properties::ValueTypeRef::GeoPointValue(value),
            ValueType::ArrayValue(value) => serde_properties::ValueTypeRef::ArrayValue(value),
            ValueType::EntityValue(value) => serde_properties::ValueTypeRef::MapValue(value),
        })
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::{
        proto::google::datastore::v1::{value::ValueType, ArrayValue, Entity, Value},
        serde_properties::{deserializer::deserialize, deserializer::Error, TraceKey},
    };
    use prost_types::Timestamp;
    use serde::Deserialize;
    use std::{collections::HashMap, iter::FromIterator};

    impl Value {
        fn new(value_type: ValueType) -> Self {
            Value {
                value_type: Some(value_type),
                ..Default::default()
            }
        }

        fn map(hashmap: HashMap<String, Value>) -> Self {
            Value::new(ValueType::EntityValue(Entity {
                properties: hashmap,
                ..Default::default()
            }))
        }

        fn geopoint(latitude: f64, longitude: f64) -> Self {
            Value::new(ValueType::GeoPointValue(
                crate::proto::google::r#type::LatLng {
                    latitude: latitude,
                    longitude: longitude,
                },
            ))
        }

        fn timestamp(seconds: i64, nanos: i32) -> Self {
            Value::new(ValueType::TimestampValue(Timestamp {
                seconds: seconds,
                nanos: nanos,
            }))
        }

        fn integer(value: i64) -> Self {
            Self::new(ValueType::IntegerValue(value))
        }

        fn double(value: f64) -> Self {
            Self::new(ValueType::DoubleValue(value))
        }

        fn child1(value: i64) -> Value {
            Value::map(HashMap::from_iter(vec![(
                "value".into(),
                Value::integer(value),
            )]))
        }

        fn child2(value: impl Into<String>) -> Value {
            Value::map(HashMap::from_iter(vec![(
                "value".into(),
                Value::string(value),
            )]))
        }

        fn string(value: impl Into<String>) -> Value {
            Value::new(ValueType::StringValue(value.into()))
        }

        fn array(values: Vec<Value>) -> Value {
            Value::new(ValueType::ArrayValue(ArrayValue { values: values }))
        }
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct ValueHolder<T> {
        value: T,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct NewType(ValueHolder<i32>);

    #[derive(Deserialize, PartialEq, Debug)]
    struct Tuple(String, ValueHolder<i32>);

    #[derive(Deserialize, PartialEq, Debug)]
    enum E {
        Unit,
        NewType(ValueHolder<i32>),
        Tuple(String, ValueHolder<i32>),
        Struct { value: f64 },
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Unit;

    #[test]
    fn test_fields() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            s: String,
            u_8: u8,
            u_16: u16,
            u_32: u32,
            u_64: u64,
            i_8: i8,
            i_16: i16,
            i_32: i32,
            i_64: i64,
            b: bool,
            f_32: f32,
            f_64: f64,
            #[serde(with = "serde_bytes")]
            bytes: Vec<u8>,
            option_some: Option<i64>,
            option_none: Option<i64>,
            option_empty: Option<i64>,
            unit: (),
            unit_struct: Unit,
            newtype: NewType,
            tuple: Tuple,
            child: ValueHolder<i32>,
            map: HashMap<String, i32>,
            geo: HashMap<String, f64>,
            time: HashMap<String, i64>,
            i_time: i64,
            u_time: u64,
            int_vec: Vec<i64>,
            child_array: [ValueHolder<i32>; 3],
            child_tuple: (ValueHolder<i32>, ValueHolder<String>),
            enum_unit: E,
            enum_newtype: E,
            enum_tuple: E,
            enum_struct: E,
        }

        let bytes: Vec<u8> = vec![0, 1, 2];
        let tuple = vec![Value::string("aaa"), Value::child1(9)];
        let map = HashMap::from_iter(vec![
            ("x".into(), Value::integer(8)),
            ("y".into(), Value::integer(9)),
        ]);
        let int_vec: Vec<_> = (1..=3).map(|i| Value::integer(i)).collect();
        let child_vec: Vec<_> = (2..=4)
            .map(|i| {
                Value::map(HashMap::from_iter(vec![(
                    "value".into(),
                    Value::integer(i),
                )]))
            })
            .collect();
        let child_tuple: Vec<_> = vec![Value::child1(5), Value::child2("piyo")];
        let enum_newtype: HashMap<String, Value> =
            HashMap::from_iter(vec![("NewType".into(), Value::child1(6))]);
        let enum_tuple: HashMap<String, Value> = HashMap::from_iter(vec![(
            "Tuple".into(),
            Value::array(vec![Value::string("fuga"), Value::child1(7)]),
        )]);
        let enum_struct: HashMap<String, Value> = HashMap::from_iter(vec![(
            "Struct".into(),
            Value::map(HashMap::from_iter(vec![(
                "value".into(),
                Value::double(0.3),
            )])),
        )]);
        let fields: HashMap<String, Value> = HashMap::from_iter(vec![
            ("s".into(), Value::string("hoge")),
            ("u_8".into(), Value::integer(8)),
            ("u_16".into(), Value::integer(16)),
            ("u_32".into(), Value::integer(32)),
            ("u_64".into(), Value::integer(64)),
            ("i_8".into(), Value::integer(-8)),
            ("i_16".into(), Value::integer(-16)),
            ("i_32".into(), Value::integer(-32)),
            ("i_64".into(), Value::integer(-64)),
            ("b".into(), Value::new(ValueType::BooleanValue(true))),
            ("f_32".into(), Value::double(0.1)),
            ("f_64".into(), Value::double(0.2)),
            ("bytes".into(), Value::new(ValueType::BlobValue(bytes))),
            ("option_some".into(), Value::integer(10)),
            ("option_none".into(), Value::new(ValueType::NullValue(0))),
            ("unit".into(), Value::new(ValueType::NullValue(0))),
            ("unit_struct".into(), Value::new(ValueType::NullValue(0))),
            ("newtype".into(), Value::child1(8)),
            ("tuple".into(), Value::array(tuple)),
            ("child".into(), Value::child1(2)),
            ("geo".into(), Value::geopoint(35.6, 139.7)),
            ("map".into(), Value::map(map)),
            ("time".into(), Value::timestamp(1609200000, 100000000)),
            ("i_time".into(), Value::timestamp(1609200001, 100000001)),
            ("u_time".into(), Value::timestamp(1609200002, 100000002)),
            ("int_vec".into(), Value::array(int_vec)),
            ("child_array".into(), Value::array(child_vec)),
            ("child_tuple".into(), Value::array(child_tuple)),
            ("enum_unit".into(), Value::string("Unit")),
            ("enum_newtype".into(), Value::map(enum_newtype)),
            ("enum_tuple".into(), Value::map(enum_tuple)),
            ("enum_struct".into(), Value::map(enum_struct)),
        ]);

        let test: Test = deserialize(fields).unwrap();
        let expected = Test {
            s: "hoge".into(),
            u_8: 8,
            u_16: 16,
            u_32: 32,
            u_64: 64,
            i_8: -8,
            i_16: -16,
            i_32: -32,
            i_64: -64,
            b: true,
            f_32: 0.1,
            f_64: 0.2,
            bytes: vec![0, 1, 2],
            option_some: Some(10),
            option_none: None,
            option_empty: None,
            unit: (),
            unit_struct: Unit,
            newtype: NewType(ValueHolder { value: 8 }),
            tuple: Tuple("aaa".into(), ValueHolder { value: 9 }),
            child: ValueHolder { value: 2 },
            geo: HashMap::from_iter(vec![("latitude".into(), 35.6), ("longitude".into(), 139.7)]),
            map: HashMap::from_iter(vec![("x".into(), 8), ("y".into(), 9)]),
            time: HashMap::from_iter(vec![
                ("seconds".into(), 1609200000),
                ("nanos".into(), 100000000),
            ]),
            i_time: 1609200001,
            u_time: 1609200002,
            int_vec: vec![1, 2, 3],
            child_array: [
                ValueHolder { value: 2 },
                ValueHolder { value: 3 },
                ValueHolder { value: 4 },
            ],
            child_tuple: (
                ValueHolder { value: 5 },
                ValueHolder {
                    value: "piyo".into(),
                },
            ),
            enum_unit: E::Unit,
            enum_newtype: E::NewType(ValueHolder { value: 6 }),
            enum_tuple: E::Tuple("fuga".into(), ValueHolder { value: 7 }),
            enum_struct: E::Struct { value: 0.3 },
        };
        assert_eq!(expected, test);
    }

    #[test]
    fn test_ignore_field() {
        let fields = HashMap::from_iter(vec![
            ("value".into(), Value::integer(1)),
            ("b".into(), Value::integer(2)),
        ]);
        assert_eq!(ValueHolder { value: 1 }, deserialize(fields).unwrap());
    }

    #[test]
    fn test_nested_map_error() {
        #[derive(Deserialize, Debug)]
        struct A {
            b: B,
        }
        #[derive(Deserialize, Debug)]
        struct B {
            c: C,
        }
        #[derive(Deserialize, Debug)]
        struct C {
            value: i64,
        }

        let c = HashMap::from_iter(vec![("value".into(), Value::string("a"))]);
        let b = HashMap::from_iter(vec![("c".into(), Value::map(c))]);
        let a: HashMap<String, Value> = HashMap::from_iter(vec![("b".into(), Value::map(b))]);
        let error = deserialize::<A, Value>(a).unwrap_err();
        assert_eq!(
            "A integer value was expected for /b/c/value, but it was String \"a\"",
            error.to_string()
        );
    }

    #[test]
    fn test_array_error() {
        #[derive(Deserialize, Debug)]
        struct A {
            b: B,
        }
        #[derive(Deserialize, Debug)]
        struct B {
            v: Vec<i64>,
        }

        let v = vec![Value::integer(1), Value::string("hoge")];
        let b = HashMap::from_iter(vec![("v".into(), Value::array(v))]);
        let a: HashMap<String, Value> = HashMap::from_iter(vec![("b".into(), Value::map(b))]);
        let error = deserialize::<A, Value>(a).unwrap_err();
        assert_eq!(
            "A integer value was expected for /b/v[], but it was String \"hoge\"",
            error.to_string()
        );
    }

    #[test]
    fn test_expected_value_error() {
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::string("hoge"))]);
        let key = TraceKey::Map("value".into(), Box::new(TraceKey::Root));
        let error_value = "String \"hoge\"";
        assert_eq!(
            Error::ExpectedMap(key.clone(), error_value.into()),
            deserialize::<ValueHolder<HashMap<String, i64>>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedBoolean(key.clone(), error_value.into()),
            deserialize::<ValueHolder<bool>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedInteger(key.clone(), error_value.into()),
            deserialize::<ValueHolder<u64>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedInteger(key.clone(), error_value.into()),
            deserialize::<ValueHolder<i64>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedDouble(key.clone(), error_value.into()),
            deserialize::<ValueHolder<f32>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedDouble(key.clone(), error_value.into()),
            deserialize::<ValueHolder<f64>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedNull(key.clone(), error_value.into()),
            deserialize::<ValueHolder<()>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedArray(key.clone(), error_value.into()),
            deserialize::<ValueHolder<Vec<i64>>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedArray(key.clone(), error_value.into()),
            deserialize::<ValueHolder<Vec<i64>>, Value>(fields.clone()).unwrap_err()
        );

        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::integer(0))]);
        let error_value = "Integer 0";
        assert_eq!(
            Error::ExpectedString(key.clone(), error_value.into()),
            deserialize::<ValueHolder<String>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedEnum(key.clone(), error_value.into()),
            deserialize::<ValueHolder<E>, Value>(fields.clone()).unwrap_err()
        );
    }

    #[test]
    fn test_convert_error() {
        let key = TraceKey::Map("value".into(), Box::new(TraceKey::Root));
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::integer(-1))]);
        assert_eq!(
            Error::CouldNotConvertNumber(key.clone(), "Integer -1".into()),
            deserialize::<ValueHolder<u64>, Value>(fields).unwrap_err()
        );
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::integer(256))]);
        assert_eq!(
            Error::CouldNotConvertNumber(key.clone(), "Integer 256".into()),
            deserialize::<ValueHolder<u8>, Value>(fields).unwrap_err()
        );
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::double(-3.40282348E+38))]);
        assert_eq!(
            Error::CouldNotConvertNumber(
                key.clone(),
                "Double -340282348000000000000000000000000000000.0".into()
            ),
            deserialize::<ValueHolder<f32>, Value>(fields).unwrap_err()
        );
    }

    #[test]
    fn test_end_error() {
        let key = TraceKey::Map("value".into(), Box::new(TraceKey::Root));
        let value = Value::integer(1);
        let array = Value::array(vec![value.clone(), value.clone(), value.clone()]);
        let fields: HashMap<String, Value> = HashMap::from_iter(vec![("value".into(), array)]);
        assert_eq!(
            Error::ExpectedArrayEnd(key.clone()),
            deserialize::<ValueHolder<(i32, i32)>, Value>(fields).unwrap_err()
        );
    }
}

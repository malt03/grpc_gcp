mod error;

use crate::proto::google::firestore::v1::{value::ValueType, MapValue, Value};
use de::SeqAccess;
pub use error::{Error, Result};
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor},
    Deserialize,
};
use std::{collections::HashMap, convert::TryFrom, iter::FromIterator, iter::Peekable, mem};

impl ValueType {
    fn is_some_value(&self) -> bool {
        if let ValueType::NullValue(_) = self {
            false
        } else {
            true
        }
    }
}

pub struct Deserializer {
    processing_bundle: DeserializerBundle,
    bundle_stack: Vec<DeserializerBundle>,
}

impl Deserializer {
    pub fn from_fields(input: HashMap<String, Value>) -> Self {
        Deserializer {
            processing_bundle: DeserializerBundle::root(input),
            bundle_stack: Vec::new(),
        }
    }
}

enum DeserializerBundle {
    Map(MapDeserializerBundle),
    Array(Peekable<Box<dyn Iterator<Item = Value>>>),
}

struct MapDeserializerBundle {
    entries: Peekable<Box<dyn Iterator<Item = (String, Value)>>>,
    poped_value: Option<Value>,
}

impl Value {
    fn from_fields(input: HashMap<String, Value>) -> Self {
        Value {
            value_type: Some(ValueType::MapValue(MapValue { fields: input })),
        }
    }
}

impl DeserializerBundle {
    fn map(input: HashMap<String, Value>) -> Self {
        DeserializerBundle::Map(MapDeserializerBundle {
            entries: (Box::new(input.into_iter()) as Box<dyn Iterator<Item = _>>).peekable(),
            poped_value: None,
        })
    }

    fn root(input: HashMap<String, Value>) -> Self {
        DeserializerBundle::Map(MapDeserializerBundle {
            entries: (Box::new(std::iter::empty()) as Box<dyn Iterator<Item = _>>).peekable(),
            poped_value: Some(Value::from_fields(input)),
        })
    }
}

pub fn from_fields<'a, T>(s: HashMap<String, Value>) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_fields(s);
    Ok(T::deserialize(&mut deserializer)?)
}

enum BundleElement {
    Key(String),
    Value(Value),
    EndOfBundle,
}

enum PeekedFieldElement<'a> {
    Key(&'a String),
    Value(&'a Value),
    EndOfBundle,
}

impl BundleElement {
    fn value(self) -> Result<Value> {
        if let BundleElement::Value(value) = self {
            Ok(value)
        } else {
            Err(Error::ExpectedValue)
        }
    }
}

impl Value {
    fn new(value_type: ValueType) -> Self {
        Value {
            value_type: Some(value_type),
        }
    }

    fn integer(value: i64) -> Self {
        Value::new(ValueType::IntegerValue(value))
    }

    fn double(value: f64) -> Value {
        Value::new(ValueType::DoubleValue(value))
    }

    fn map_value(self) -> Result<HashMap<String, Value>> {
        match self.value_type.unwrap() {
            ValueType::MapValue(value) => Ok(value.fields),
            ValueType::GeoPointValue(value) => {
                let map = HashMap::from_iter(vec![
                    ("latitude".into(), Value::double(value.latitude)),
                    ("longitude".into(), Value::double(value.longitude)),
                ]);
                Ok(map)
            }
            ValueType::TimestampValue(value) => {
                let map = HashMap::from_iter(vec![
                    ("seconds".into(), Value::integer(value.seconds)),
                    ("nanos".into(), Value::integer(value.nanos.into())),
                ]);
                Ok(map)
            }
            _ => Err(Error::ExpectedMap),
        }
    }
}

impl Deserializer {
    fn pop(&mut self) -> Result<BundleElement> {
        fn pop_bundle_stack(de: &mut Deserializer) -> Result<BundleElement> {
            match de.bundle_stack.pop() {
                None => Err(Error::Eof),
                Some(bundle) => {
                    de.processing_bundle = bundle;
                    Ok(BundleElement::EndOfBundle)
                }
            }
        }
        match self.processing_bundle {
            DeserializerBundle::Map(ref mut bundle) => {
                match mem::replace(&mut bundle.poped_value, None) {
                    Some(value) => {
                        bundle.poped_value = None;
                        Ok(BundleElement::Value(value))
                    }
                    None => match bundle.entries.next() {
                        None => pop_bundle_stack(self),
                        Some(entriy) => {
                            bundle.poped_value = Some(entriy.1);
                            Ok(BundleElement::Key(entriy.0))
                        }
                    },
                }
            }
            DeserializerBundle::Array(ref mut bundle) => match bundle.next() {
                None => pop_bundle_stack(self),
                Some(value) => Ok(BundleElement::Value(value)),
            },
        }
    }

    fn peek(&mut self) -> Result<PeekedFieldElement> {
        fn peek_bundle_stack(bundle_stack: &Vec<DeserializerBundle>) -> Result<PeekedFieldElement> {
            match bundle_stack.last() {
                None => Err(Error::Eof),
                Some(_) => Ok(PeekedFieldElement::EndOfBundle),
            }
        }
        match self.processing_bundle {
            DeserializerBundle::Map(ref mut bundle) => match bundle.poped_value {
                Some(ref value) => Ok(PeekedFieldElement::Value(value)),
                None => match bundle.entries.peek() {
                    None => peek_bundle_stack(&self.bundle_stack),
                    Some(entriy) => Ok(PeekedFieldElement::Key(&entriy.0)),
                },
            },
            DeserializerBundle::Array(ref mut bundle) => match bundle.peek() {
                None => peek_bundle_stack(&self.bundle_stack),
                Some(value) => Ok(PeekedFieldElement::Value(value)),
            },
        }
    }

    fn get_bool(&mut self) -> Result<bool> {
        if let BundleElement::Value(value) = self.pop()? {
            if let ValueType::BooleanValue(value) = value.value_type.unwrap() {
                Ok(value)
            } else {
                Err(Error::ExpectedBoolean)
            }
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn get_string(&mut self) -> Result<String> {
        match self.pop()? {
            BundleElement::Key(key) => Ok(key.clone()),
            BundleElement::Value(value) => {
                if let ValueType::StringValue(value) = value.value_type.unwrap() {
                    Ok(value.clone())
                } else {
                    Err(Error::ExpectedString)
                }
            }
            BundleElement::EndOfBundle => {
                return Err(Error::ExpectedString);
            }
        }
    }

    fn get_unsigned<T>(&mut self) -> Result<T>
    where
        T: TryFrom<u64>,
    {
        if let BundleElement::Value(value) = self.pop()? {
            let value = match value.value_type.unwrap() {
                ValueType::IntegerValue(value) => Ok(value),
                ValueType::TimestampValue(value) => Ok(value.seconds),
                _ => Err(Error::ExpectedInteger),
            }?;
            let min = u64::min_value() as i64;
            if value < min {
                Err(Error::CouldNotConvertNumber)
            } else {
                T::try_from(value as u64).or(Err(Error::CouldNotConvertNumber))
            }
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn get_signed<T>(&mut self) -> Result<T>
    where
        T: TryFrom<i64>,
    {
        if let BundleElement::Value(value) = self.pop()? {
            let value = match value.value_type.unwrap() {
                ValueType::IntegerValue(value) => Ok(value),
                ValueType::TimestampValue(value) => Ok(value.seconds),
                _ => Err(Error::ExpectedInteger),
            }?;
            T::try_from(value).or(Err(Error::CouldNotConvertNumber))
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn get_f64(&mut self) -> Result<f64> {
        if let BundleElement::Value(value) = self.pop()? {
            if let ValueType::DoubleValue(value) = value.value_type.unwrap() {
                Ok(value)
            } else {
                Err(Error::ExpectedDouble)
            }
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn get_f32(&mut self) -> Result<f32> {
        let value = self.get_f64()?;
        if value > f32::MAX as f64 && value < f32::MIN as f64 {
            Err(Error::CouldNotConvertNumber)
        } else {
            Ok(value as f32)
        }
    }

    fn get_char(&mut self) -> Result<char> {
        match self.get_string() {
            Err(err) => {
                return Err(if err == Error::ExpectedString {
                    Error::ExpectedChar
                } else {
                    err
                });
            }
            Ok(str) => {
                if str.len() != 1 {
                    Err(Error::ExpectedChar)
                } else {
                    Ok(str.chars().next().unwrap())
                }
            }
        }
    }

    fn get_bytes(&mut self) -> Result<Vec<u8>> {
        if let BundleElement::Value(value) = self.pop()? {
            match value.value_type.unwrap() {
                ValueType::BytesValue(value) => Ok(value.clone()),
                _ => Err(Error::ExpectedBytes),
            }
        } else {
            Err(Error::ExpectedValue)
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // let hoge = match self.peek()? {
        //     PeekedFieldElement::Key(_) => self.deserialize_str(visitor),
        //     PeekedFieldElement::Value(value) => match value.value_type.as_ref().unwrap() {
        //         ValueType::NullValue(_) => self.deserialize_unit(visitor),
        //         ValueType::BooleanValue(_) => self.deserialize_bool(visitor),
        //         ValueType::IntegerValue(_) => self.deserialize_i64(visitor),
        //         ValueType::DoubleValue(_) => Err(Error::ExpectedValue),
        //         ValueType::TimestampValue(_) => Err(Error::ExpectedValue),
        //         ValueType::StringValue(_) => Err(Error::ExpectedValue),
        //         ValueType::BytesValue(_) => Err(Error::ExpectedValue),
        //         ValueType::ReferenceValue(_) => Err(Error::ExpectedValue),
        //         ValueType::GeoPointValue(_) => Err(Error::ExpectedValue),
        //         ValueType::ArrayValue(_) => Err(Error::ExpectedValue),
        //         ValueType::MapValue(_) => Err(Error::ExpectedValue),
        //     },
        //     PeekedFieldElement::EndOfBundle => Err(Error::ExpectedValue),
        // };
        // hoge
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.get_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.get_signed()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.get_signed()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.get_signed()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.get_signed()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.get_unsigned()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.get_unsigned()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.get_unsigned()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.get_unsigned()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.get_f32()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.get_f64()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(self.get_char()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(&self.get_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(&self.get_bytes()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        {
            if let PeekedFieldElement::Value(value) = self.peek()? {
                if value.value_type.as_ref().unwrap().is_some_value() {
                    return visitor.visit_some(self);
                }
            } else {
                return Err(Error::ExpectedValue);
            }
        }

        self.pop().unwrap();
        visitor.visit_none()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let BundleElement::Value(value) = self.pop()? {
            if let ValueType::NullValue(_) = value.value_type.unwrap() {
                visitor.visit_unit()
            } else {
                Err(Error::ExpectedNull)
            }
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let BundleElement::Value(value) = self.pop()? {
            if let ValueType::ArrayValue(array_value) = value.value_type.unwrap() {
                let iter: Box<dyn Iterator<Item = Value>> =
                    Box::new(array_value.values.into_iter());
                let bundle = DeserializerBundle::Array(iter.peekable());
                let replaced = mem::replace(&mut self.processing_bundle, bundle);
                self.bundle_stack.push(replaced);
                let result = visitor.visit_seq(Entries::new(&mut self))?;
                if let BundleElement::EndOfBundle = self.pop()? {
                    Ok(result)
                } else {
                    Err(Error::ExpectedArrayEnd)
                }
            } else {
                Err(Error::ExpectedArray)
            }
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let BundleElement::Value(value) = self.pop()? {
            let bundle = DeserializerBundle::map(value.map_value()?);
            let replaced = mem::replace(&mut self.processing_bundle, bundle);
            self.bundle_stack.push(replaced);
            let result = visitor.visit_map(Entries::new(&mut self))?;
            if let BundleElement::EndOfBundle = self.pop()? {
                Ok(result)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let PeekedFieldElement::Value(value) = self.peek()? {
            match value.value_type.as_ref().unwrap() {
                ValueType::StringValue(_) => visitor.visit_enum(Enum::new(self)),
                ValueType::MapValue(_) => {
                    let map = self.pop()?.value()?.map_value()?;
                    let bundle = DeserializerBundle::map(map);
                    let replaced = mem::replace(&mut self.processing_bundle, bundle);
                    self.bundle_stack.push(replaced);
                    let result = visitor.visit_enum(Enum::new(self))?;
                    if let BundleElement::EndOfBundle = self.pop()? {
                        Ok(result)
                    } else {
                        Err(Error::ExpectedMapEnd)
                    }
                }
                _ => Err(Error::ExpectedEnum),
            }
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct Entries<'a> {
    de: &'a mut Deserializer,
}

impl<'a> Entries<'a> {
    fn new(de: &'a mut Deserializer) -> Self {
        Entries { de }
    }
}

impl<'a, 'de> SeqAccess<'de> for Entries<'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if let PeekedFieldElement::EndOfBundle = self.de.peek()? {
            Ok(None)
        } else {
            Ok(Some(seed.deserialize(&mut *self.de)?))
        }
    }
}

impl<'a, 'de> MapAccess<'de> for Entries<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if let PeekedFieldElement::EndOfBundle = self.de.peek()? {
            Ok(None)
        } else {
            Ok(Some(seed.deserialize(&mut *self.de)?))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

struct Enum<'a> {
    de: &'a mut Deserializer,
}

impl<'a> Enum<'a> {
    fn new(de: &'a mut Deserializer) -> Self {
        Enum { de }
    }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        Ok((seed.deserialize(&mut *self.de)?, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::{from_fields, Error};
    use crate::proto::google::firestore::v1::{value::ValueType, ArrayValue, MapValue, Value};
    use prost_types::Timestamp;
    use serde::Deserialize;
    use std::{collections::HashMap, iter::FromIterator};

    impl Value {
        fn map(hashmap: HashMap<String, Value>) -> Self {
            Value::new(ValueType::MapValue(MapValue { fields: hashmap }))
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
            c: char,
            #[serde(with = "serde_bytes")]
            bytes: Vec<u8>,
            option_some: Option<i64>,
            option_none: Option<i64>,
            option_empty: Option<i64>,
            unit: (),
            unit_struct: Unit,
            newtype: NewType,
            tuple: Tuple,
            child: Child1,
            map: HashMap<String, i32>,
            geo: HashMap<String, f64>,
            time: HashMap<String, i64>,
            i_time: i64,
            u_time: u64,
            int_vec: Vec<i64>,
            child_array: [Child1; 3],
            child_tuple: (Child1, Child2),
            enum_unit: E,
            enum_newtype: E,
            enum_tuple: E,
            enum_struct: E,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Child1 {
            value: i32,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Child2 {
            value: String,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct NewType(Child1);

        #[derive(Deserialize, PartialEq, Debug)]
        struct Tuple(String, Child1);

        #[derive(Deserialize, PartialEq, Debug)]
        enum E {
            Unit,
            NewType(Child1),
            Tuple(String, Child1),
            Struct { value: f64 },
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Unit;

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
            ("c".into(), Value::string("x")),
            ("bytes".into(), Value::new(ValueType::BytesValue(bytes))),
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

        let test: Test = from_fields(fields).unwrap();
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
            c: 'x',
            bytes: vec![0, 1, 2],
            option_some: Some(10),
            option_none: None,
            option_empty: None,
            unit: (),
            unit_struct: Unit,
            newtype: NewType(Child1 { value: 8 }),
            tuple: Tuple("aaa".into(), Child1 { value: 9 }),
            child: Child1 { value: 2 },
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
                Child1 { value: 2 },
                Child1 { value: 3 },
                Child1 { value: 4 },
            ],
            child_tuple: (
                Child1 { value: 5 },
                Child2 {
                    value: "piyo".into(),
                },
            ),
            enum_unit: E::Unit,
            enum_newtype: E::NewType(Child1 { value: 6 }),
            enum_tuple: E::Tuple("fuga".into(), Child1 { value: 7 }),
            enum_struct: E::Struct { value: 0.3 },
        };
        assert_eq!(expected, test);
    }

    #[derive(Deserialize, Debug)]
    struct ErrorTest<T> {
        value: T,
    }

    #[test]
    fn test_expected_map() {
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::string("hoge"))]);
        let error = from_fields::<ErrorTest<HashMap<String, i64>>>(fields).unwrap_err();
        assert_eq!(Error::ExpectedMap, error);
    }

    #[test]
    fn test_expected_bool() {
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::string("hoge"))]);
        let error = from_fields::<ErrorTest<bool>>(fields).unwrap_err();
        assert_eq!(Error::ExpectedBoolean, error);
    }
}

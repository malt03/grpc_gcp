mod error;

use crate::proto::google::firestore::v1::{value::ValueType, MapValue, Value};
use de::SeqAccess;
pub use error::{Error, Result};
use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor};
use serde::Deserialize;
use std::{collections::HashMap, convert::TryFrom, iter::Peekable, mem};

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
    fn mapValue(self) -> Result<MapValue> {
        if let ValueType::MapValue(value_type) = self.value_type.unwrap() {
            Ok(value_type)
        } else {
            Err(Error::ExpectedMap)
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
            if let Some(ref value_type) = value.value_type {
                if let ValueType::BooleanValue(value) = value_type {
                    return Ok(*value);
                }
            }
            Err(Error::ExpectedBoolean)
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn get_string(&mut self) -> Result<String> {
        match self.pop()? {
            BundleElement::Key(key) => Ok(key.clone()),
            BundleElement::Value(value) => {
                if let Some(ref value_type) = value.value_type {
                    if let ValueType::StringValue(value) = value_type {
                        return Ok(value.clone());
                    }
                }
                Err(Error::ExpectedString)
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
            if let Some(ref value_type) = value.value_type {
                if let ValueType::IntegerValue(value) = value_type {
                    let min = u64::min_value() as i64;
                    if *value < min {
                        return Err(Error::CouldNotConvertNumber);
                    }
                    return T::try_from(*value as u64).or(Err(Error::CouldNotConvertNumber));
                }
            }
            Err(Error::ExpectedInteger)
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn get_signed<T>(&mut self) -> Result<T>
    where
        T: TryFrom<i64>,
    {
        if let BundleElement::Value(value) = self.pop()? {
            if let Some(ref value_type) = value.value_type {
                if let ValueType::IntegerValue(value) = value_type {
                    return T::try_from(*value).or(Err(Error::CouldNotConvertNumber));
                }
            }
            Err(Error::ExpectedInteger)
        } else {
            Err(Error::ExpectedValue)
        }
    }

    fn get_f64(&mut self) -> Result<f64> {
        if let BundleElement::Value(value) = self.pop()? {
            if let Some(ref value_type) = value.value_type {
                if let ValueType::DoubleValue(value) = value_type {
                    return Ok(*value);
                }
            }
            Err(Error::ExpectedInteger)
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
            if let Some(ref value_type) = value.value_type {
                return match value_type {
                    ValueType::BytesValue(value) => Ok(value.clone()),
                    ValueType::StringValue(value) => Ok(value.clone().into_bytes()),
                    _ => Err(Error::ExpectedBytes),
                };
            }
            Err(Error::ExpectedBytes)
        } else {
            Err(Error::ExpectedValue)
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
        // match self.peek_char()? {
        //     'n' => self.deserialize_unit(visitor),
        //     't' | 'f' => self.deserialize_bool(visitor),
        //     '"' => self.deserialize_str(visitor),
        //     '0'..='9' => self.deserialize_u64(visitor),
        //     '-' => self.deserialize_i64(visitor),
        //     '[' => self.deserialize_seq(visitor),
        //     '{' => self.deserialize_map(visitor),
        //     _ => Err(Error::Syntax),
        // }
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
                if let Some(ref value_type) = value.value_type {
                    if value_type.is_some_value() {
                        return visitor.visit_some(self);
                    }
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
            if let Some(ref value_type) = value.value_type {
                if let ValueType::NullValue(_) = value_type {
                    return visitor.visit_unit();
                }
            }
            return Err(Error::ExpectedNull);
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
            if let Some(value_type) = value.value_type {
                if let ValueType::ArrayValue(array_value) = value_type {
                    let iter: Box<dyn Iterator<Item = Value>> =
                        Box::new(array_value.values.into_iter());
                    let bundle = DeserializerBundle::Array(iter.peekable());
                    let replaced = mem::replace(&mut self.processing_bundle, bundle);
                    self.bundle_stack.push(replaced);
                    let result = visitor.visit_seq(Entries::new(&mut self))?;
                    if let BundleElement::EndOfBundle = self.pop()? {
                        return Ok(result);
                    } else {
                        return Err(Error::ExpectedArrayEnd);
                    }
                }
            }
            Err(Error::ExpectedArray)
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
            if let Some(value_type) = value.value_type {
                if let ValueType::MapValue(map_value) = value_type {
                    let bundle = DeserializerBundle::map(map_value.fields);
                    let replaced = mem::replace(&mut self.processing_bundle, bundle);
                    self.bundle_stack.push(replaced);
                    let result = visitor.visit_map(Entries::new(&mut self))?;
                    if let BundleElement::EndOfBundle = self.pop()? {
                        return Ok(result);
                    } else {
                        return Err(Error::ExpectedMapEnd);
                    }
                }
            }
            Err(Error::ExpectedMap)
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
            if let Some(ref value_type) = value.value_type {
                return match value_type {
                    ValueType::StringValue(_) => visitor.visit_enum(Enum::new(self)),
                    ValueType::MapValue(_) => {
                        let map_value = self.pop()?.value()?.mapValue()?;
                        let bundle = DeserializerBundle::map(map_value.fields);
                        let replaced = mem::replace(&mut self.processing_bundle, bundle);
                        self.bundle_stack.push(replaced);
                        let result = visitor.visit_enum(Enum::new(self))?;
                        if let BundleElement::EndOfBundle = self.pop()? {
                            return Ok(result);
                        } else {
                            return Err(Error::ExpectedMapEnd);
                        }
                    }
                    _ => Err(Error::ExpectedEnum),
                };
            }
        }
        return Err(Error::ExpectedValue);
        // if self.peek_char()? == '"' {
        //     // Visit a unit variant.
        //     visitor.visit_enum(self.parse_string()?.into_deserializer())
        // } else if self.next_char()? == '{' {
        //     // Visit a newtype variant, tuple variant, or struct variant.
        //     let value = visitor.visit_enum(Enum::new(self))?;
        //     // Parse the matching close brace.
        //     if self.next_char()? == '}' {
        //         Ok(value)
        //     } else {
        //         Err(Error::ExpectedMapEnd)
        //     }
        // } else {
        //     Err(Error::ExpectedEnum)
        // }
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In JSON, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
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

// // `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// // which variant of the enum is supposed to be deserialized.
// //
// // Note that all enum deserialization methods in Serde refer exclusively to the
// // "externally tagged" enum representation.
impl<'de, 'a> EnumAccess<'de> for Enum<'a> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // The `deserialize_enum` method parsed a `{` character so we are
        // currently inside of a map. The seed will be deserializing itself from
        // the key of the map.
        Ok((seed.deserialize(&mut *self.de)?, self))
        // if let PeekedFieldElement::Value(_) = self.de.peek()? {
        //     Ok((val, self))
        // } else {
        //     Err(Error::ExpectedValue)
        // }
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, 'a> VariantAccess<'de> for Enum<'a> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
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
    use super::from_fields;
    use crate::proto::google::firestore::v1::{value::ValueType, ArrayValue, MapValue, Value};
    use maplit::hashmap;
    use serde::Deserialize;
    use std::collections::HashMap;

    impl Value {
        fn new(value_type: ValueType) -> Self {
            Value {
                value_type: Some(value_type),
            }
        }
    }
    impl Value {
        fn map(hashmap: HashMap<String, Value>) -> Self {
            Value::new(ValueType::MapValue(MapValue { fields: hashmap }))
        }

        fn child1(value: i64) -> Value {
            Value::map(hashmap! {
                "value".into() => Value::new(ValueType::IntegerValue(value)),
            })
        }

        fn string(value: impl Into<String>) -> Value {
            Value::new(ValueType::StringValue(value.into()))
        }

        fn vec(values: Vec<Value>) -> Value {
            Value::new(ValueType::ArrayValue(ArrayValue { values: values }))
        }
    }

    #[test]
    fn test_fields() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            s: String,
            uint: u64,
            int: i64,
            b: bool,
            float: f32,
            c: char,
            #[serde(with = "serde_bytes")]
            bytes: Vec<u8>,
            option_some: Option<i64>,
            option_none: Option<i64>,
            option_empty: Option<i64>,
            unit: (),
            child: Child1,
            int_vec: Vec<i64>,
            child_array: [Child1; 3],
            child_tuple: (Child1, Child2),
            enum_unit: E,
            enum_struct: E,
            enum_tuple: E,
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
        enum E {
            Unit,
            Struct(Child1),
            Tuple(String, Child1),
        }

        let bytes: Vec<u8> = vec![0, 1, 2];
        let child = Value::child1(2);
        let int_vec: Vec<_> = (1..=3)
            .map(|i| Value::new(ValueType::IntegerValue(i)))
            .collect();
        let child_vec: Vec<_> = (2..=4)
            .map(|i| {
                Value::map(hashmap! {
                    "value".into() => Value::new(ValueType::IntegerValue(i))
                })
            })
            .collect();
        let child_tuple: Vec<_> = vec![
            Value::map(hashmap! {
                "value".into() => Value::new(ValueType::IntegerValue(5))
            }),
            Value::map(hashmap! {
                "value".into() => Value::new(ValueType::StringValue("piyo".into()))
            }),
        ];
        let enum_struct: HashMap<String, Value> = hashmap! {
            "Struct".into() => Value::child1(6),
        };
        let enum_tuple_value = Value::vec(vec![Value::string("fuga"), Value::child1(7)]);
        let enum_tuple: HashMap<String, Value> = hashmap! {
            "Tuple".into() => enum_tuple_value,
        };
        let fields: HashMap<String, Value> = hashmap! {
            "s".into() => Value::new(ValueType::StringValue("hoge".into())),
            "uint".into() => Value::new(ValueType::IntegerValue(24)),
            "int".into() => Value::new(ValueType::IntegerValue(-24)),
            "b".into() => Value::new(ValueType::BooleanValue(true)),
            "float".into() => Value::new(ValueType::DoubleValue(0.1)),
            "c".into() => Value::new(ValueType::StringValue("x".into())),
            "bytes".into() => Value::new(ValueType::BytesValue(bytes)),
            "option_some".into() => Value::new(ValueType::IntegerValue(10)),
            "option_none".into() => Value::new(ValueType::NullValue(0)),
            "unit".into() => Value::new(ValueType::NullValue(0)),
            "child".into() => child,
            "int_vec".into() => Value::new(ValueType::ArrayValue(ArrayValue { values: int_vec })),
            "child_array".into() => Value::new(ValueType::ArrayValue(ArrayValue { values: child_vec })),
            "child_tuple".into() => Value::new(ValueType::ArrayValue(ArrayValue { values: child_tuple })),
            "enum_unit".into() => Value::string("Unit"),
            "enum_struct".into() => Value::map(enum_struct),
            "enum_tuple".into() => Value::map(enum_tuple),
        };

        let test: Test = from_fields(fields).unwrap();
        let expected = Test {
            s: "hoge".into(),
            uint: 24,
            int: -24,
            b: true,
            float: 0.1,
            c: 'x',
            bytes: vec![0, 1, 2],
            option_some: Some(10),
            option_none: None,
            option_empty: None,
            unit: (),
            child: Child1 { value: 2 },
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
            enum_struct: E::Struct(Child1 { value: 6 }),
            enum_tuple: E::Tuple("fuga".into(), Child1 { value: 7 }),
        };
        assert_eq!(expected, test);
    }
}

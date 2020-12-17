mod error;

use crate::{
    firestore,
    proto::google::firestore::v1::{value::ValueType, Value},
};
pub use error::{Error, Result};
use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;
use std::{
    collections::{hash_map::Iter, HashMap},
    convert::TryFrom,
    ops::{AddAssign, MulAssign, Neg},
};

pub struct Deserializer<'de> {
    entries: Iter<'de, String, Value>,
    poped_value: Option<&'de Value>,
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    pub fn from_fields(input: &'de HashMap<String, Value>) -> Self {
        Deserializer {
            entries: input.iter(),
            poped_value: None,
        }
    }
}

pub fn from_fields<'a, T>(s: &'a HashMap<String, Value>) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_fields(s);
    Ok(T::deserialize(&mut deserializer)?)
}

enum FieldElement<'de> {
    Key(&'de String),
    Value(&'de Value),
}

impl<'de> Deserializer<'de> {
    fn pop(&mut self) -> Result<FieldElement> {
        if let Some(value) = self.poped_value {
            self.poped_value = None;
            return Ok(FieldElement::Value(value));
        }
        match self.entries.next() {
            None => Err(Error::Eof),
            Some(entriy) => {
                self.poped_value = Some(entriy.1);
                Ok(FieldElement::Key(entriy.0))
            }
        }
    }

    fn get_bool(&mut self) -> Result<bool> {
        match self.pop()? {
            FieldElement::Key(_) => Err(Error::ExpectedBoolean),
            FieldElement::Value(value) => {
                if let Some(ref value_type) = value.value_type {
                    if let ValueType::BooleanValue(value) = value_type {
                        return Ok(*value);
                    }
                }
                Err(Error::ExpectedBoolean)
            }
        }
    }

    fn get_string(&mut self) -> Result<String> {
        match self.pop()? {
            FieldElement::Key(key) => Ok(key.clone()),
            FieldElement::Value(value) => {
                if let Some(ref value_type) = value.value_type {
                    if let ValueType::StringValue(value) = value_type {
                        return Ok(value.clone());
                    }
                }
                Err(Error::ExpectedString)
            }
        }
    }

    fn get_unsigned<T>(&mut self) -> Result<T>
    where
        T: TryFrom<u64>,
    {
        match self.pop()? {
            FieldElement::Key(_) => Err(Error::ExpectedInteger),
            FieldElement::Value(value) => {
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
            }
        }
    }

    fn get_signed<T>(&mut self) -> Result<T>
    where
        T: TryFrom<i64>,
    {
        match self.pop()? {
            FieldElement::Key(_) => Err(Error::ExpectedInteger),
            FieldElement::Value(value) => {
                if let Some(ref value_type) = value.value_type {
                    if let ValueType::IntegerValue(value) = value_type {
                        return T::try_from(*value).or(Err(Error::CouldNotConvertNumber));
                    }
                }
                Err(Error::ExpectedInteger)
            }
        }
    }

    fn get_f64(&mut self) -> Result<f64> {
        match self.pop()? {
            FieldElement::Key(_) => Err(Error::ExpectedInteger),
            FieldElement::Value(value) => {
                if let Some(ref value_type) = value.value_type {
                    if let ValueType::DoubleValue(value) = value_type {
                        return Ok(*value);
                    }
                }
                Err(Error::ExpectedInteger)
            }
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

    // // Look at the first character in the input without consuming it.
    // fn peek_char(&mut self) -> Result<char> {
    //     self.input.chars().next().ok_or(Error::Eof)
    // }

    // // Consume the first character in the input.
    // fn next_char(&mut self) -> Result<char> {
    //     let ch = self.peek_char()?;
    //     self.input = &self.input[ch.len_utf8()..];
    //     Ok(ch)
    // }

    // // Parse the JSON identifier `true` or `false`.
    // fn parse_bool(&mut self) -> Result<bool> {
    //     if self.input.starts_with("true") {
    //         self.input = &self.input["true".len()..];
    //         Ok(true)
    //     } else if self.input.starts_with("false") {
    //         self.input = &self.input["false".len()..];
    //         Ok(false)
    //     } else {
    //         Err(Error::ExpectedBoolean)
    //     }
    // }

    // // Parse a group of decimal digits as an unsigned integer of type T.
    // //
    // // This implementation is a bit too lenient, for example `001` is not
    // // allowed in JSON. Also the various arithmetic operations can overflow and
    // // panic or return bogus data. But it is good enough for example code!
    // fn parse_unsigned<T>(&mut self) -> Result<T>
    // where
    //     T: AddAssign<T> + MulAssign<T> + From<u8>,
    // {
    //     let mut int = match self.next_char()? {
    //         ch @ '0'..='9' => T::from(ch as u8 - b'0'),
    //         _ => {
    //             return Err(Error::ExpectedInteger);
    //         }
    //     };
    //     loop {
    //         match self.input.chars().next() {
    //             Some(ch @ '0'..='9') => {
    //                 self.input = &self.input[1..];
    //                 int *= T::from(10);
    //                 int += T::from(ch as u8 - b'0');
    //             }
    //             _ => {
    //                 return Ok(int);
    //             }
    //         }
    //     }
    // }

    // // Parse a possible minus sign followed by a group of decimal digits as a
    // // signed integer of type T.
    // fn parse_signed<T>(&mut self) -> Result<T>
    // where
    //     T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8>,
    // {
    //     // Optional minus sign, delegate to `parse_unsigned`, negate if negative.
    //     unimplemented!()
    // }

    // // Parse a string until the next '"' character.
    // //
    // // Makes no attempt to handle escape sequences. What did you expect? This is
    // // example code!
    // fn parse_string(&mut self) -> Result<&'de str> {
    //     if self.next_char()? != '"' {
    //         return Err(Error::ExpectedString);
    //     }
    //     match self.input.find('"') {
    //         Some(len) => {
    //             let s = &self.input[..len];
    //             self.input = &self.input[len + 1..];
    //             Ok(s)
    //         }
    //         None => Err(Error::Eof),
    //     }
    // }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
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

    // The `Serializer` implementation on the previous page serialized chars as
    // single-character strings so handle that representation here.
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(self.get_char()?)
    }

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let str = self.get_string()?;
        visitor.visit_str(&str)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    // An absent optional is represented as the JSON `null` and a present
    // optional is represented as just the contained value.
    //
    // As commented in `Serializer` implementation, this is a lossy
    // representation. For example the values `Some(())` and `None` both
    // serialize as just `null`. Unfortunately this is typically what people
    // expect when working with JSON. Other formats are encouraged to behave
    // more intelligently if possible.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
        // if self.input.starts_with("null") {
        //     self.input = &self.input["null".len()..];
        //     visitor.visit_none()
        // } else {
        //     visitor.visit_some(self)
        // }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
        // if self.input.starts_with("null") {
        //     self.input = &self.input["null".len()..];
        //     visitor.visit_unit()
        // } else {
        //     Err(Error::ExpectedNull)
        // }
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
        // Parse the opening bracket of the sequence.
        // if self.next_char()? == '[' {
        //     // Give the visitor access to each element of the sequence.
        //     let value = visitor.visit_seq(CommaSeparated::new(&mut self))?;
        //     // Parse the closing bracket of the sequence.
        //     if self.next_char()? == ']' {
        //         Ok(value)
        //     } else {
        //         Err(Error::ExpectedArrayEnd)
        //     }
        // } else {
        //     Err(Error::ExpectedArray)
        // }
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
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

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(Entries::new(&mut self))
        // Parse the opening brace of the map.
        // if self.next_char()? == '{' {
        //     // Give the visitor access to each entry of the map.
        //     let value = visitor.visit_map(CommaSeparated::new(&mut self))?;
        //     // Parse the closing brace of the map.
        //     if self.next_char()? == '}' {
        //         Ok(value)
        //     } else {
        //         Err(Error::ExpectedMapEnd)
        //     }
        // } else {
        //     Err(Error::ExpectedMap)
        // }
    }

    // Structs look just like maps in JSON.
    //
    // Notice the `fields` parameter - a "struct" in the Serde data model means
    // that the `Deserialize` implementation is required to know what the fields
    // are before even looking at the input data. Any key-value pairing in which
    // the fields cannot be known ahead of time is probably a map.
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
        todo!()
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

// // In order to handle commas correctly when deserializing a JSON array or map,
// // we need to track whether we are on the first element or past the first
// // element.
// struct CommaSeparated<'a, 'de: 'a> {
//     de: &'a mut Deserializer<'de>,
//     first: bool,
// }

// impl<'a, 'de> CommaSeparated<'a, 'de> {
//     fn new(de: &'a mut Deserializer<'de>) -> Self {
//         CommaSeparated { de, first: true }
//     }
// }

// // `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// // through elements of the sequence.
// impl<'de, 'a> SeqAccess<'de> for CommaSeparated<'a, 'de> {
//     type Error = Error;

//     fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
//     where
//         T: DeserializeSeed<'de>,
//     {
//         // Check if there are no more elements.
//         if self.de.peek_char()? == ']' {
//             return Ok(None);
//         }
//         // Comma is required before every element except the first.
//         if !self.first && self.de.next_char()? != ',' {
//             return Err(Error::ExpectedArrayComma);
//         }
//         self.first = false;
//         // Deserialize an array element.
//         seed.deserialize(&mut *self.de).map(Some)
//     }
// }

// // `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// // through entries of the map.
// impl<'de, 'a> MapAccess<'de> for CommaSeparated<'a, 'de> {
//     type Error = Error;

//     fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
//     where
//         K: DeserializeSeed<'de>,
//     {
//         // Check if there are no more entries.
//         if self.de.peek_char()? == '}' {
//             return Ok(None);
//         }
//         // Comma is required before every entry except the first.
//         if !self.first && self.de.next_char()? != ',' {
//             return Err(Error::ExpectedMapComma);
//         }
//         self.first = false;
//         // Deserialize a map key.
//         seed.deserialize(&mut *self.de).map(Some)
//     }

//     fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
//     where
//         V: DeserializeSeed<'de>,
//     {
//         // It doesn't make a difference whether the colon is parsed at the end
//         // of `next_key_seed` or at the beginning of `next_value_seed`. In this
//         // case the code is a bit simpler having it here.
//         if self.de.next_char()? != ':' {
//             return Err(Error::ExpectedMapColon);
//         }
//         // Deserialize a map value.
//         seed.deserialize(&mut *self.de)
//     }
// }

struct Entries<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de> Entries<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Entries { de, first: true }
    }
}

impl<'de, 'a> MapAccess<'de> for Entries<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match seed.deserialize(&mut *self.de) {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                if err == Error::Eof {
                    return Ok(None);
                }
                return Err(err);
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

// struct Enum<'a, 'de: 'a> {
//     de: &'a mut Deserializer<'de>,
// }

// impl<'a, 'de> Enum<'a, 'de> {
//     fn new(de: &'a mut Deserializer<'de>) -> Self {
//         Enum { de }
//     }
// }

// // `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// // which variant of the enum is supposed to be deserialized.
// //
// // Note that all enum deserialization methods in Serde refer exclusively to the
// // "externally tagged" enum representation.
// impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
//     type Error = Error;
//     type Variant = Self;

//     fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
//     where
//         V: DeserializeSeed<'de>,
//     {
//         // The `deserialize_enum` method parsed a `{` character so we are
//         // currently inside of a map. The seed will be deserializing itself from
//         // the key of the map.
//         let val = seed.deserialize(&mut *self.de)?;
//         // Parse the colon separating map key from value.
//         if self.de.next_char()? == ':' {
//             Ok((val, self))
//         } else {
//             Err(Error::ExpectedMapColon)
//         }
//     }
// }

// // `VariantAccess` is provided to the `Visitor` to give it the ability to see
// // the content of the single variant that it decided to deserialize.
// impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
//     type Error = Error;

//     // If the `Visitor` expected this variant to be a unit variant, the input
//     // should have been the plain string case handled in `deserialize_enum`.
//     fn unit_variant(self) -> Result<()> {
//         Err(Error::ExpectedString)
//     }

//     // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
//     // deserialize the value here.
//     fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
//     where
//         T: DeserializeSeed<'de>,
//     {
//         seed.deserialize(self.de)
//     }

//     // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
//     // deserialize the sequence of data here.
//     fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
//     where
//         V: Visitor<'de>,
//     {
//         de::Deserializer::deserialize_seq(self.de, visitor)
//     }

//     // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
//     // deserialize the inner map here.
//     fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
//     where
//         V: Visitor<'de>,
//     {
//         de::Deserializer::deserialize_map(self.de, visitor)
//     }
// }

////////////////////////////////////////////////////////////////////////////////

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
    }

    let mut fields = HashMap::new();
    fields.insert(
        "s".to_string(),
        Value {
            value_type: Some(ValueType::StringValue("hoge".into())),
        },
    );
    fields.insert(
        "uint".to_string(),
        Value {
            value_type: Some(ValueType::IntegerValue(24)),
        },
    );
    fields.insert(
        "int".to_string(),
        Value {
            value_type: Some(ValueType::IntegerValue(-24)),
        },
    );
    fields.insert(
        "b".to_string(),
        Value {
            value_type: Some(ValueType::BooleanValue(true)),
        },
    );
    fields.insert(
        "float".to_string(),
        Value {
            value_type: Some(ValueType::DoubleValue(0.1)),
        },
    );
    fields.insert(
        "c".to_string(),
        Value {
            value_type: Some(ValueType::StringValue("x".into())),
        },
    );

    let test: Test = from_fields(&fields).unwrap();
    let expected = Test {
        s: "hoge".into(),
        uint: 24,
        int: -24,
        b: true,
        float: 0.1,
        c: 'x',
    };
    assert_eq!(expected, test);
}
// fn test_struct() {
//     #[derive(Deserialize, PartialEq, Debug)]
//     struct Test {
//         int: u32,
//         seq: Vec<String>,
//     }

//     let j = r#"{"int":1,"seq":["a","b"]}"#;
//     let expected = Test {
//         int: 1,
//         seq: vec!["a".to_owned(), "b".to_owned()],
//     };
//     assert_eq!(expected, from_str(j).unwrap());
// }

// #[test]
// fn test_enum() {
//     #[derive(Deserialize, PartialEq, Debug)]
//     enum E {
//         Unit,
//         Newtype(u32),
//         Tuple(u32, u32),
//         Struct { a: u32 },
//     }

//     let j = r#""Unit""#;
//     let expected = E::Unit;
//     assert_eq!(expected, from_str(j).unwrap());

//     let j = r#"{"Newtype":1}"#;
//     let expected = E::Newtype(1);
//     assert_eq!(expected, from_str(j).unwrap());

//     let j = r#"{"Tuple":[1,2]}"#;
//     let expected = E::Tuple(1, 2);
//     assert_eq!(expected, from_str(j).unwrap());

//     let j = r#"{"Struct":{"a":1}}"#;
//     let expected = E::Struct { a: 1 };
//     assert_eq!(expected, from_str(j).unwrap());
// }

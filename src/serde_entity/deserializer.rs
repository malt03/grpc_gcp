use super::{
    common::{KeyValueSet, TraceKey},
    error::{Error, Result},
    value::{ArrayValueTrait, ValueTrait, ValueType, ValueTypeRef},
};
use core::panic;
use de::SeqAccess;
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor},
    Deserialize,
};
use std::{collections::HashMap, convert::TryFrom, iter::Peekable, mem};

struct Deserializer<Value: ValueTrait> {
    processing_bundle: DeserializerBundle<Value>,
    bundle_stack: Vec<DeserializerBundle<Value>>,
}

impl<Value: ValueTrait> Deserializer<Value> {
    fn from_fields(input: HashMap<String, Value>) -> Self {
        Deserializer {
            processing_bundle: DeserializerBundle::root(input),
            bundle_stack: Vec::new(),
        }
    }
}

enum DeserializerBundle<Value: ValueTrait> {
    Map(MapDeserializerBundle<Value>),
    Array(ArrayDeserializerBundle<Value>),
}

struct MapDeserializerBundle<Value: ValueTrait> {
    key: TraceKey,
    entries: Peekable<Box<dyn Iterator<Item = (String, Value)>>>,
    poped_value: Option<KeyValueSet<Value>>,
}

struct ArrayDeserializerBundle<Value: ValueTrait> {
    key: TraceKey,
    values: Peekable<Box<dyn Iterator<Item = Value>>>,
}

impl<Value: ValueTrait> DeserializerBundle<Value> {
    fn map(key: &TraceKey, input: HashMap<String, Value>) -> Self {
        DeserializerBundle::Map(MapDeserializerBundle::<Value> {
            key: key.clone(),
            entries: (Box::new(input.into_iter()) as Box<dyn Iterator<Item = _>>).peekable(),
            poped_value: None,
        })
    }

    fn array(key: &TraceKey, input: Peekable<Box<dyn Iterator<Item = Value>>>) -> Self {
        DeserializerBundle::Array(ArrayDeserializerBundle {
            key: key.clone(),
            values: input,
        })
    }

    fn root(input: HashMap<String, Value>) -> Self {
        DeserializerBundle::Map(MapDeserializerBundle::<Value> {
            key: TraceKey::Root,
            entries: (Box::new(std::iter::empty()) as Box<dyn Iterator<Item = _>>).peekable(),
            poped_value: Some(KeyValueSet(TraceKey::Root, Value::from_fields(input))),
        })
    }
}

pub(crate) fn from_fields<'a, T, Value: ValueTrait>(s: HashMap<String, Value>) -> Result<T, Value>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_fields(s);
    Ok(T::deserialize(&mut deserializer)?)
}

enum BundleElement<Value: ValueTrait> {
    Key(String),
    Value(KeyValueSet<Value>),
    EndOfBundle,
}

impl<Value: ValueTrait> BundleElement<Value> {
    fn key_value_set(self) -> KeyValueSet<Value> {
        if let BundleElement::Value(key_value_set) = self {
            key_value_set
        } else {
            common_panic!()
        }
    }
}

enum PeekedBundleElement<'a, Value: ValueTrait> {
    Key(&'a String),
    Value(&'a Value),
    EndOfBundle,
}

impl<'a, Value: ValueTrait> PeekedBundleElement<'a, Value> {
    fn value(self) -> &'a Value {
        if let PeekedBundleElement::Value(value) = self {
            value
        } else {
            common_panic!()
        }
    }
}

impl<Value: ValueTrait> Deserializer<Value> {
    fn pop(&mut self) -> Result<BundleElement<Value>, Value> {
        fn pop_bundle_stack<Value: ValueTrait>(
            de: &mut Deserializer<Value>,
        ) -> Result<BundleElement<Value>, Value> {
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
                        Some((key, value)) => {
                            bundle.poped_value = Some(KeyValueSet(
                                TraceKey::Map(key.clone(), Box::new(bundle.key.clone())),
                                value,
                            ));
                            Ok(BundleElement::Key(key))
                        }
                    },
                }
            }
            DeserializerBundle::Array(ref mut bundle) => match bundle.values.next() {
                None => pop_bundle_stack(self),
                Some(value) => {
                    let set = KeyValueSet(TraceKey::Array(Box::new(bundle.key.clone())), value);
                    Ok(BundleElement::Value(set))
                }
            },
        }
    }

    fn peek(&mut self) -> Result<PeekedBundleElement<Value>, Value> {
        fn peek_bundle_stack<Value: ValueTrait>(
            bundle_stack: &Vec<DeserializerBundle<Value>>,
        ) -> Result<PeekedBundleElement<Value>, Value> {
            match bundle_stack.last() {
                None => Err(Error::Eof),
                Some(_) => Ok(PeekedBundleElement::EndOfBundle),
            }
        }
        match self.processing_bundle {
            DeserializerBundle::Map(ref mut bundle) => match bundle.poped_value {
                Some(KeyValueSet(_, ref value)) => Ok(PeekedBundleElement::Value(value)),
                None => match bundle.entries.peek() {
                    None => peek_bundle_stack(&self.bundle_stack),
                    Some(entriy) => Ok(PeekedBundleElement::Key(&entriy.0)),
                },
            },
            DeserializerBundle::Array(ref mut bundle) => match bundle.values.peek() {
                None => peek_bundle_stack(&self.bundle_stack),
                Some(value) => Ok(PeekedBundleElement::Value(value)),
            },
        }
    }

    fn get_bool(&mut self) -> Result<bool, Value> {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        if let ValueTypeRef::BooleanValue(value) = value.get_value_type().unwrap() {
            Ok(*value)
        } else {
            Err(Error::ExpectedBoolean(key, value))
        }
    }

    fn get_string(&mut self) -> Result<String, Value> {
        match self.pop()? {
            BundleElement::Key(key) => Ok(key.clone()),
            BundleElement::Value(KeyValueSet(key, value)) => {
                if let ValueTypeRef::StringValue(value) = value.get_value_type().unwrap() {
                    Ok(value.clone())
                } else {
                    Err(Error::ExpectedString(key, value))
                }
            }
            BundleElement::EndOfBundle => common_panic!(),
        }
    }

    fn get_unsigned<T>(&mut self) -> Result<T, Value>
    where
        T: TryFrom<u64>,
    {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        match value.integer_value() {
            Some(i) => {
                let min = u64::min_value() as i64;
                if i >= min {
                    T::try_from(i as u64).or(Err(Error::CouldNotConvertNumber(key, value)))
                } else {
                    Err(Error::CouldNotConvertNumber(key, value))
                }
            }
            None => Err(Error::ExpectedInteger(key.clone(), value)),
        }
    }

    fn get_signed<T>(&mut self) -> Result<T, Value>
    where
        T: TryFrom<i64>,
    {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        match value.integer_value() {
            Some(i) => T::try_from(i).or(Err(Error::CouldNotConvertNumber(key, value))),
            None => Err(Error::ExpectedInteger(key.clone(), value)),
        }
    }

    fn get_f64(&mut self) -> Result<f64, Value> {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        if let ValueTypeRef::DoubleValue(value) = value.get_value_type().unwrap() {
            Ok(*value)
        } else {
            Err(Error::ExpectedDouble(key, value))
        }
    }

    fn get_f32(&mut self) -> Result<f32, Value> {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        if let ValueTypeRef::DoubleValue(f) = value.get_value_type().unwrap() {
            if *f > f32::MIN as f64 && *f < f32::MAX as f64 {
                Ok(*f as f32)
            } else {
                Err(Error::CouldNotConvertNumber(key, value))
            }
        } else {
            Err(Error::ExpectedDouble(key, value))
        }
    }

    fn get_bytes(&mut self) -> Result<Vec<u8>, Value> {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        match value.get_value_type().as_ref().unwrap() {
            ValueTypeRef::BytesValue(_) => Ok(value.byte_value().unwrap()),
            _ => Err(Error::ExpectedBytes(key, value)),
        }
    }
}

impl<'de, 'a, Value: ValueTrait> de::Deserializer<'de> for &'a mut Deserializer<Value> {
    type Error = Error<Value>;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.get_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.get_signed()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.get_signed()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.get_signed()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.get_signed()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.get_unsigned()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.get_unsigned()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.get_unsigned()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.get_unsigned()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.get_f32()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.get_f64()?)
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!(
            "Deserialization of char is not supported, please define it as string instead of char."
        )
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(&self.get_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(&self.get_bytes()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        {
            let value = self.peek()?.value();
            if value.get_value_type().as_ref().unwrap().is_some_value() {
                return visitor.visit_some(self);
            }
        }

        self.pop().unwrap();
        visitor.visit_none()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        if let ValueTypeRef::NullValue(_) = value.get_value_type().unwrap() {
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNull(key, value))
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        match value.get_value_type().unwrap() {
            ValueTypeRef::ArrayValue(_) => {
                let array_value = value.array_value().unwrap();
                let iter: Box<dyn Iterator<Item = Value>> =
                    Box::new(array_value.get_values().into_iter());
                let bundle = DeserializerBundle::array(&key, iter.peekable());
                let replaced = mem::replace(&mut self.processing_bundle, bundle);
                self.bundle_stack.push(replaced);
                let result = visitor.visit_seq(Entries::new(&mut self))?;
                if let BundleElement::EndOfBundle = self.pop()? {
                    Ok(result)
                } else {
                    Err(Error::ExpectedArrayEnd(key))
                }
            }
            _ => Err(Error::ExpectedArray(key, value)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Value>
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
    ) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        let KeyValueSet(key, value) = self.pop()?.key_value_set();
        if value.has_map_value() {
            let map = value.map_value().unwrap();
            let bundle = DeserializerBundle::map(&key, map);
            let replaced = mem::replace(&mut self.processing_bundle, bundle);
            self.bundle_stack.push(replaced);
            let result = visitor.visit_map(Entries::new(&mut self))?;
            if let BundleElement::EndOfBundle = self.pop()? {
                Ok(result)
            } else {
                common_panic!()
            }
        } else {
            Err(Error::ExpectedMap(key.clone(), value))
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Value>
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
    ) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        match self.peek()?.value().get_value_type().unwrap() {
            ValueTypeRef::StringValue(_) => visitor.visit_enum(Enum::new(self)),
            ValueTypeRef::MapValue(_) => {
                let KeyValueSet(key, value) = self.pop()?.key_value_set();
                let map = value.map_value().unwrap();
                let bundle = DeserializerBundle::map(&key, map);
                let replaced = mem::replace(&mut self.processing_bundle, bundle);
                self.bundle_stack.push(replaced);
                let result = visitor.visit_enum(Enum::new(self))?;
                if let BundleElement::EndOfBundle = self.pop()? {
                    Ok(result)
                } else {
                    common_panic!()
                }
            }
            _ => {
                let KeyValueSet(key, value) = self.pop()?.key_value_set();
                Err(Error::ExpectedEnum(key, value))
            }
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        self.pop()?;
        visitor.visit_unit()
    }
}

struct Entries<'a, Value: ValueTrait> {
    de: &'a mut Deserializer<Value>,
}

impl<'a, Value: ValueTrait> Entries<'a, Value> {
    fn new(de: &'a mut Deserializer<Value>) -> Self {
        Entries { de }
    }
}

impl<'a, 'de, Value: ValueTrait> SeqAccess<'de> for Entries<'a, Value> {
    type Error = Error<Value>;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Value>
    where
        T: DeserializeSeed<'de>,
    {
        if let PeekedBundleElement::EndOfBundle = self.de.peek()? {
            Ok(None)
        } else {
            Ok(Some(seed.deserialize(&mut *self.de)?))
        }
    }
}

impl<'a, 'de, Value: ValueTrait> MapAccess<'de> for Entries<'a, Value> {
    type Error = Error<Value>;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Value>
    where
        K: DeserializeSeed<'de>,
    {
        if let PeekedBundleElement::EndOfBundle = self.de.peek()? {
            Ok(None)
        } else {
            Ok(Some(seed.deserialize(&mut *self.de)?))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

struct Enum<'a, Value: ValueTrait> {
    de: &'a mut Deserializer<Value>,
}

impl<'a, Value: ValueTrait> Enum<'a, Value> {
    fn new(de: &'a mut Deserializer<Value>) -> Self {
        Enum { de }
    }
}

impl<'de, 'a, Value: ValueTrait> EnumAccess<'de> for Enum<'a, Value> {
    type Error = Error<Value>;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Value>
    where
        V: DeserializeSeed<'de>,
    {
        Ok((seed.deserialize(&mut *self.de)?, self))
    }
}

impl<'de, 'a, Value: ValueTrait> VariantAccess<'de> for Enum<'a, Value> {
    type Error = Error<Value>;

    fn unit_variant(self) -> Result<(), Value> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::{from_fields, Error, TraceKey, ValueTrait};
    use crate::{
        proto::google::firestore::v1::{value::ValueType, ArrayValue, MapValue, Value},
        serde_entity::value::{ArrayValueTrait, LatLngTrait, MapValueTrait, ValueTypeRef},
    };
    use prost_types::Timestamp;
    use serde::Deserialize;
    use std::{collections::HashMap, iter::FromIterator};

    impl LatLngTrait for crate::proto::google::r#type::LatLng {
        fn get_latitude(&self) -> f64 {
            self.latitude
        }

        fn get_longitude(&self) -> f64 {
            self.longitude
        }
    }

    impl ArrayValueTrait<Value> for ArrayValue {
        fn get_values(self) -> Vec<Value> {
            self.values
        }
    }

    impl MapValueTrait<Value> for MapValue {
        fn get_fields(self) -> HashMap<String, Value> {
            self.fields
        }
    }

    impl ValueTrait for Value {
        type LatLng = crate::proto::google::r#type::LatLng;
        type ArrayValue = ArrayValue;
        type MapValue = MapValue;

        fn from_fields(input: HashMap<String, Self>) -> Self {
            Value {
                value_type: Some(ValueType::MapValue(MapValue { fields: input })),
            }
        }

        fn new(value_type: crate::serde_entity::value::ValueType<Self>) -> Self {
            let value_type = match value_type {
                crate::serde_entity::value::ValueType::NullValue(value) => {
                    ValueType::NullValue(value)
                }
                crate::serde_entity::value::ValueType::BooleanValue(value) => {
                    ValueType::BooleanValue(value)
                }
                crate::serde_entity::value::ValueType::IntegerValue(value) => {
                    ValueType::IntegerValue(value)
                }
                crate::serde_entity::value::ValueType::DoubleValue(value) => {
                    ValueType::DoubleValue(value)
                }
                crate::serde_entity::value::ValueType::TimestampValue(value) => {
                    ValueType::TimestampValue(value)
                }
                crate::serde_entity::value::ValueType::StringValue(value) => {
                    ValueType::StringValue(value)
                }
                crate::serde_entity::value::ValueType::BytesValue(value) => {
                    ValueType::BytesValue(value)
                }
                crate::serde_entity::value::ValueType::ReferenceValue(value) => {
                    ValueType::ReferenceValue(value)
                }
                crate::serde_entity::value::ValueType::GeoPointValue(value) => {
                    ValueType::GeoPointValue(value)
                }
                crate::serde_entity::value::ValueType::ArrayValue(value) => {
                    ValueType::ArrayValue(value)
                }
                crate::serde_entity::value::ValueType::MapValue(value) => {
                    ValueType::MapValue(value)
                }
            };
            Value {
                value_type: Some(value_type),
            }
        }

        fn into_value_type(self) -> Option<crate::serde_entity::value::ValueType<Self>> {
            self.value_type.map(|value_type| match value_type {
                ValueType::NullValue(value) => {
                    crate::serde_entity::value::ValueType::NullValue(value)
                }
                ValueType::BooleanValue(value) => {
                    crate::serde_entity::value::ValueType::BooleanValue(value)
                }
                ValueType::IntegerValue(value) => {
                    crate::serde_entity::value::ValueType::IntegerValue(value)
                }
                ValueType::DoubleValue(value) => {
                    crate::serde_entity::value::ValueType::DoubleValue(value)
                }
                ValueType::TimestampValue(value) => {
                    crate::serde_entity::value::ValueType::TimestampValue(value)
                }
                ValueType::StringValue(value) => {
                    crate::serde_entity::value::ValueType::StringValue(value)
                }
                ValueType::BytesValue(value) => {
                    crate::serde_entity::value::ValueType::BytesValue(value)
                }
                ValueType::ReferenceValue(value) => {
                    crate::serde_entity::value::ValueType::ReferenceValue(value)
                }
                ValueType::GeoPointValue(value) => {
                    crate::serde_entity::value::ValueType::GeoPointValue(value)
                }
                ValueType::ArrayValue(value) => {
                    crate::serde_entity::value::ValueType::ArrayValue(value)
                }
                ValueType::MapValue(value) => {
                    crate::serde_entity::value::ValueType::MapValue(value)
                }
            })
        }

        fn get_value_type<'s>(&'s self) -> Option<crate::serde_entity::value::ValueTypeRef<Self>> {
            self.value_type.as_ref().map(|value_type| match value_type {
                ValueType::NullValue(value) => ValueTypeRef::NullValue(value),
                ValueType::BooleanValue(value) => ValueTypeRef::BooleanValue(value),
                ValueType::IntegerValue(value) => ValueTypeRef::IntegerValue(value),
                ValueType::DoubleValue(value) => ValueTypeRef::DoubleValue(value),
                ValueType::TimestampValue(value) => ValueTypeRef::TimestampValue(value),
                ValueType::StringValue(value) => ValueTypeRef::StringValue(value),
                ValueType::BytesValue(value) => ValueTypeRef::BytesValue(value),
                ValueType::ReferenceValue(value) => ValueTypeRef::ReferenceValue(value),
                ValueType::GeoPointValue(value) => ValueTypeRef::GeoPointValue(value),
                ValueType::ArrayValue(value) => ValueTypeRef::ArrayValue(value),
                ValueType::MapValue(value) => ValueTypeRef::MapValue(value),
            })
        }

        // fn new(value_type: ValueType) -> Self {
        //     Value {
        //         value_type: Some(value_type),
        //     }
        // }

        // fn get_value_type(&self) -> &Option<ValueType> {
        //     &self.value_type
        // }

        // fn into_value_type(self) -> Option<ValueType> {
        //     self.value_type
        // }
    }

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
        assert_eq!(ValueHolder { value: 1 }, from_fields(fields).unwrap());
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
        let error = from_fields::<A, Value>(a).unwrap_err();
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
        let error = from_fields::<A, Value>(a).unwrap_err();
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
        assert_eq!(
            Error::ExpectedMap(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<HashMap<String, i64>>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedBoolean(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<bool>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedInteger(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<u64>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedInteger(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<i64>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedDouble(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<f32>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedDouble(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<f64>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedNull(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<()>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedArray(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<Vec<i64>>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedArray(key.clone(), Value::string("hoge")),
            from_fields::<ValueHolder<Vec<i64>>, Value>(fields.clone()).unwrap_err()
        );

        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::integer(0))]);
        assert_eq!(
            Error::ExpectedString(key.clone(), Value::integer(0)),
            from_fields::<ValueHolder<String>, Value>(fields.clone()).unwrap_err()
        );
        assert_eq!(
            Error::ExpectedEnum(key.clone(), Value::integer(0)),
            from_fields::<ValueHolder<E>, Value>(fields.clone()).unwrap_err()
        );
    }

    #[test]
    fn test_convert_error() {
        let key = TraceKey::Map("value".into(), Box::new(TraceKey::Root));
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::integer(-1))]);
        assert_eq!(
            Error::CouldNotConvertNumber(key.clone(), Value::integer(-1)),
            from_fields::<ValueHolder<u64>, Value>(fields).unwrap_err()
        );
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::integer(256))]);
        assert_eq!(
            Error::CouldNotConvertNumber(key.clone(), Value::integer(256)),
            from_fields::<ValueHolder<u8>, Value>(fields).unwrap_err()
        );
        let fields: HashMap<String, Value> =
            HashMap::from_iter(vec![("value".into(), Value::double(-3.40282348E+38))]);
        assert_eq!(
            Error::CouldNotConvertNumber(key.clone(), Value::double(-3.40282348E+38)),
            from_fields::<ValueHolder<f32>, Value>(fields).unwrap_err()
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
            from_fields::<ValueHolder<(i32, i32)>, Value>(fields).unwrap_err()
        );
    }
}
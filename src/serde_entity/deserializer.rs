pub(crate) use super::{
    error::{Error, Result},
    ArrayValueTrait, KeyValueSet, TraceKey, ValueTrait, ValueTypeRef,
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
                match value.get_value_type().unwrap() {
                    ValueTypeRef::StringValue(value) => Ok(value.clone()),
                    ValueTypeRef::ReferenceValue(value) => Ok(value.clone()),
                    _ => Err(Error::ExpectedString(key, value)),
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

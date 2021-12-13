use super::Context;
use serde::{de::Visitor, ser, Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

pub struct DuktapeSerializer<'ctx> {
    ctx: &'ctx mut Context,
    objects: Vec<duktape_sys::duk_idx_t>,
}

pub struct DuktapeSeqSerializer<'a, 'ctx> {
    obj_id: duktape_sys::duk_idx_t,
    inner: &'a mut DuktapeSerializer<'ctx>,
    array_idx: u32,
}

impl<'a> DuktapeSerializer<'a> {
    pub fn from_ctx(ctx: &'a mut Context) -> Self {
        DuktapeSerializer {
            ctx,
            objects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum Error {
    #[error("{}", .0)]
    Message(String),
}

impl Error {
    fn unsupported() -> Self {
        Error::Message("not implemented".to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl<'a, 'ctx> Serializer for &'a mut DuktapeSerializer<'ctx> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = DuktapeSeqSerializer<'a, 'ctx>;
    type SerializeTuple = DuktapeSeqSerializer<'a, 'ctx>;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.ctx.push_bool(v);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.ctx.push_int(v.into());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.ctx.push_uint(v.into());
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.ctx.push_int(v.into());
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.ctx.push_uint(v.into());
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.ctx.push_int(v.into());
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.ctx.push_uint(v.into());
        Ok(())
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok> {
        Err(Error::unsupported())
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        Err(Error::unsupported())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.ctx.push_double(v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.ctx.push_double(v.into());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.ctx.push_string(v);
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok> {
        Err(Error::unsupported())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.ctx.push_null();
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        //self.ctx.push_null();
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(Error::unsupported())
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::unsupported())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        let obj_id = self.ctx.push_array();
        self.objects.push(obj_id);
        Ok(DuktapeSeqSerializer {
            inner: self,
            obj_id,
            array_idx: 0,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::unsupported())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::unsupported())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::unsupported())
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.objects.push(self.ctx.push_object());
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::unsupported())
    }
}

impl<'a, 'ctx> ser::SerializeSeq for DuktapeSeqSerializer<'a, 'ctx> {
    // Must match the `Ok` type of the serializer.
    type Ok = ();
    // Must match the `Error` type of the serializer.
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.inner)?;
        self.inner.ctx.put_prop_index(self.obj_id, self.array_idx);
        self.array_idx += 1;
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.inner.objects.pop();
        Ok(())
    }
}

impl<'a, 'ctx> ser::SerializeTuple for DuktapeSeqSerializer<'a, 'ctx> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeSeq>::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        <Self as ser::SerializeSeq>::end(self)
    }
}

// Same thing but for tuple structs.
impl<'a, 'ctx> ser::SerializeTupleStruct for &'a mut DuktapeSerializer<'ctx> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Tuple variants are a little different. Refer back to the
// `serialize_tuple_variant` method above:
//
//    self.output += "{";
//    variant.serialize(&mut *self)?;
//    self.output += ":[";
//
// So the `end` method in this impl is responsible for closing both the `]` and
// the `}`.
impl<'a, 'ctx> ser::SerializeTupleVariant for &'a mut DuktapeSerializer<'ctx> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Some `Serialize` types are not able to hold a key and value in memory at the
// same time so `SerializeMap` implementations are required to support
// `serialize_key` and `serialize_value` individually.
//
// There is a third optional method on the `SerializeMap` trait. The
// `serialize_entry` method allows serializers to optimize for the case where
// key and value are both available simultaneously. In JSON it doesn't make a
// difference so the default behavior for `serialize_entry` is fine.
impl<'a, 'ctx> ser::SerializeMap for &'a mut DuktapeSerializer<'ctx> {
    type Ok = ();
    type Error = Error;

    // The Serde data model allows map keys to be any serializable type. JSON
    // only allows string keys so the implementation below will produce invalid
    // JSON if the key serializes as something other than a string.
    //
    // A real JSON serializer would need to validate that map keys are strings.
    // This can be done by using a different Serializer to serialize the key
    // (instead of `&mut **self`) and having that other serializer only
    // implement `serialize_str` and return an error on any other data type.
    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::unsupported())
    }

    // It doesn't make a difference whether the colon is printed at the end of
    // `serialize_key` or at the beginning of `serialize_value`. In this case
    // the code is a bit simpler having it here.
    fn serialize_value<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::unsupported())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a, 'ctx> ser::SerializeStruct for &'a mut DuktapeSerializer<'ctx> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        let obj_id = self.objects.last().unwrap();
        self.ctx.put_prop_string(*obj_id, key);
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.objects.pop();
        Ok(())
    }
}

// Similar to `SerializeTupleVariant`, here the `end` method is responsible for
// closing both of the curly braces opened by `serialize_struct_variant`.
impl<'a, 'ctx> ser::SerializeStructVariant for &'a mut DuktapeSerializer<'ctx> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        let obj_id = self.objects.last().unwrap();
        self.ctx.put_prop_string(*obj_id, key);
        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

pub struct DuktapeDeserializer<'ctx> {
    inner: &'ctx mut Context,
    stack_idx: i32,
}

impl<'ctx> DuktapeDeserializer<'ctx> {
    pub fn from_ctx(ctx: &'ctx mut Context, stack_idx: i32) -> Self {
        DuktapeDeserializer {
            inner: ctx,
            stack_idx,
        }
    }
}

impl<'a, 'de, 'ctx> Deserializer<'de> for &'a mut DuktapeDeserializer<'ctx> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_bool(self.stack_idx);
        visitor.visit_bool(val)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_int(self.stack_idx);
        visitor.visit_i8(val as i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_int(self.stack_idx);
        visitor.visit_i16(val as i16)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_int(self.stack_idx);
        visitor.visit_i32(val)
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_uint(self.stack_idx);
        visitor.visit_u8(val as u8)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_uint(self.stack_idx);
        visitor.visit_u16(val as u16)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_uint(self.stack_idx);
        visitor.visit_u32(val)
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_number(self.stack_idx);
        visitor.visit_f32(val as f32)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_number(self.stack_idx);
        visitor.visit_f64(val)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_string(self.stack_idx);
        if val.len() == 1 {
            visitor.visit_char(val.chars().next().unwrap())
        } else {
            todo!()
        }
    }

    // we can't have borrowed strings
    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.inner.get_string(self.stack_idx);
        visitor.visit_string(val)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("figure out how to check for null")
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        //let _val = self.inner.get_null(self.stack_idx);
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let _val = self.inner.get_null(self.stack_idx);
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
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

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.inner.get_object(self.stack_idx);
        let des = DuktapeStructDeserializer {
            ctx: self.inner,
            fields,
            idx: 0,
            obj_idx: self.stack_idx,
        };
        let res = visitor.visit_seq(des)?;
        self.inner.pop();
        Ok(res)
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
        Err(Error::unsupported())
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
    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::unsupported())
    }
}

struct DuktapeStructDeserializer<'ctx> {
    ctx: &'ctx mut Context,
    fields: &'static [&'static str],
    idx: usize,
    obj_idx: i32,
}

impl<'de, 'ctx> serde::de::SeqAccess<'de> for DuktapeStructDeserializer<'ctx> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        if let Some(prop_name) = self.fields.get(self.idx) {
            if !self.ctx.get_prop(prop_name, 0) {
                return Err(Error::Message("incorrect field".to_string()));
            }
            let mut deserializer = DuktapeDeserializer::from_ctx(&mut *self.ctx, -1);
            let val = seed.deserialize(&mut deserializer)?;
            self.idx += 1;
            self.ctx.pop();
            return Ok(Some(val));
        }
        Ok(None)
    }
}

#[test]
fn deserialize_num() {
    let mut ctx = super::Context::default();
    ctx.push(&42.0f64);
    assert_eq!(ctx.peek::<f64>(-1), 42.0f64);
}

#[test]
fn deserialize_obj() {
    let mut ctx = super::Context::default();
    #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
    struct T {
        hello: String,
        num: u32,
    }
    let t1 = T {
        hello: "world".to_string(),
        num: 42,
    };
    ctx.push(&t1);
    let t2 = ctx.peek::<T>(0);
    assert_eq!(t1, t2);
}

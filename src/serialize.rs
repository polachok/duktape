use super::Context;
use serde::{ser, Serialize, Serializer};
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
        self.ctx.push_null();
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

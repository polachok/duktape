use crate::serialize;
use crate::Context;

use serde::{Deserialize, Serialize};

pub trait PushValue {
    fn push_to(&self, ctx: &mut Context) -> i32;
}

pub trait PeekValue {
    fn peek_at(ctx: &mut Context, idx: i32) -> Self;
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerdeValue<T: ?Sized>(pub T);

impl<'a, T: ?Sized> PushValue for SerdeValue<&'a T>
where
    T: Serialize,
{
    fn push_to(&self, ctx: &mut Context) -> i32 {
        let mut serializer = serialize::DuktapeSerializer::from_ctx(ctx);
        self.serialize(&mut serializer).unwrap(); // TODO
        ctx.stack_len() - 1
    }
}

impl<'de, T> PeekValue for SerdeValue<T>
where
    T: Deserialize<'de>,
{
    fn peek_at(ctx: &mut Context, idx: i32) -> Self {
        let mut deserializer = serialize::DuktapeDeserializer::from_ctx(ctx, idx);
        Self::deserialize(&mut deserializer).unwrap() // TODO
    }
}

macro_rules! via_serde {
    ($t: ty) => {
        impl PushValue for $t {
            fn push_to(&self, ctx: &mut Context) -> i32 {
                let v = SerdeValue(self);
                v.push_to(ctx)
            }
        }

        impl PeekValue for $t {
            fn peek_at(ctx: &mut Context, idx: i32) -> Self {
                let v: SerdeValue<Self> = SerdeValue::peek_at(ctx, idx);
                v.0
            }
        }
    };
}

via_serde!(());
via_serde!(bool);
via_serde!(u8);
via_serde!(u16);
via_serde!(u32);
via_serde!(i8);
via_serde!(i16);
via_serde!(i32);
via_serde!(f32);
via_serde!(f64);
via_serde!(String);

impl<'de, T> PeekValue for Vec<T>
where
    T: Deserialize<'de>,
{
    fn peek_at(ctx: &mut Context, idx: i32) -> Self {
        let v: SerdeValue<Self> = SerdeValue::peek_at(ctx, idx);
        v.0
    }
}

impl<T> PushValue for [T]
where
    T: Serialize,
{
    fn push_to(&self, ctx: &mut Context) -> i32 {
        let v = SerdeValue(self);
        v.push_to(ctx)
    }
}

impl<T> PushValue for &T
where
    T: Serialize,
{
    fn push_to(&self, ctx: &mut Context) -> i32 {
        let v = SerdeValue(*self);
        v.push_to(ctx)
    }
}

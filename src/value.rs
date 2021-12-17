use crate::serialize;
use crate::Context;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

pub trait PushValue {
    fn push_to(self, ctx: &mut Context) -> u32;
}

pub trait PeekValue: Sized {
    fn peek_at(ctx: &mut Context, idx: i32) -> Option<Self>;

    fn pop(ctx: &mut Context) -> Option<Self> {
        let this = Self::peek_at(ctx, -1);
        if this.is_some() {
            ctx.pop_it();
        }
        this
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerdeValue<T: ?Sized>(pub T);

impl<'a, T: ?Sized> PushValue for SerdeValue<&'a T>
where
    T: Serialize,
{
    fn push_to(self, ctx: &mut Context) -> u32 {
        let mut serializer = serialize::DuktapeSerializer::from_ctx(ctx);
        self.serialize(&mut serializer).unwrap(); // TODO
        ctx.stack_top()
    }
}

impl<'de, T> PeekValue for SerdeValue<T>
where
    T: Deserialize<'de>,
{
    fn peek_at(ctx: &mut Context, idx: i32) -> Option<Self> {
        let mut deserializer = serialize::DuktapeDeserializer::from_ctx(ctx, idx);
        Self::deserialize(&mut deserializer).ok() // TODO
    }
}

macro_rules! via_serde {
    ($t: ty) => {
        impl PushValue for $t {
            fn push_to(self, ctx: &mut Context) -> u32 {
                let v = SerdeValue(self);
                v.push_to(ctx)
            }
        }

        impl PeekValue for $t {
            fn peek_at(ctx: &mut Context, idx: i32) -> Option<Self> {
                let v: Option<SerdeValue<Self>> = SerdeValue::peek_at(ctx, idx);
                v.map(|v| v.0)
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

impl<T> PushValue for Rc<T> {
    fn push_to(self, ctx: &mut Context) -> u32 {
        let idx = ctx.push_object();

        let ptr = Rc::into_raw(self);

        ctx.push_pointer(ptr as _);
        ctx.put_prop_string(idx.try_into().unwrap(), "__rc");

        ctx.push_string(std::any::type_name::<T>());
        ctx.put_prop_string(idx.try_into().unwrap(), "__type");
        idx
    }
}

fn peek_rc<T>(ctx: &mut Context, idx: i32, copy: bool) -> Option<Rc<T>> {
    ctx.get_object(idx);

    if !ctx.get_prop(idx, "__type") {
        return None;
    }
    let typ = ctx.get_string(-1);
    ctx.pop();
    if typ != std::any::type_name::<T>() {
        return None;
    }

    if !ctx.get_prop(idx, "__rc") {
        return None;
    }
    let ptr = ctx.get_pointer(-1);
    ctx.pop();
    if copy {
        // increment because we just produced a new Rc and 1 rc is left in stack
        unsafe { Rc::increment_strong_count(ptr) };
    }
    let rc = unsafe { Rc::from_raw(ptr as *const T) };
    Some(rc)
}

impl<T> PeekValue for Rc<T> {
    fn peek_at(ctx: &mut Context, idx: i32) -> Option<Self> {
        peek_rc(ctx, idx, true)
    }

    fn pop(ctx: &mut Context) -> Option<Self> {
        let val = peek_rc(ctx, -1, false)?;
        ctx.pop();
        Some(val)
    }
}

#[test]
fn test_rc() {
    let vec = Rc::new(vec![1u32, 2, 3]);
    let mut ctx = Context::default();
    let idx = ctx.push(vec);
    let same_vec = <Rc<Vec<u32>>>::peek_at(&mut ctx, idx.try_into().unwrap()).unwrap();
    assert_eq!(Rc::strong_count(&same_vec), 2);
    let same_vec_2 = ctx.pop_value::<Rc<Vec<u32>>>();
    assert_eq!(Rc::strong_count(&same_vec), 2);
    drop(same_vec_2);
    assert_eq!(Rc::strong_count(&same_vec), 1);
}

impl<T: PushValue> PushValue for Option<T> {
    fn push_to(self, ctx: &mut Context) -> u32 {
        let idx = match self {
            Some(v) => v.push_to(ctx),
            None => {
                ctx.push_undefined();
                ctx.stack_top()
            }
        };
        idx
    }
}

impl<T: PeekValue> PeekValue for Option<T> {
    fn peek_at(ctx: &mut Context, idx: i32) -> Option<Self> {
        Some(T::peek_at(ctx, idx))
    }
}

impl<'de, T> PeekValue for Vec<T>
where
    T: Deserialize<'de>,
{
    fn peek_at(ctx: &mut Context, idx: i32) -> Option<Self> {
        let v: Option<_> = SerdeValue::peek_at(ctx, idx);
        v.map(|v| v.0)
    }
}

impl<'a, T> PushValue for &'a [T]
where
    T: Serialize,
{
    fn push_to(self, ctx: &mut Context) -> u32 {
        let v = SerdeValue(&self);
        v.push_to(ctx)
    }
}

impl<T> PushValue for &T
where
    T: Serialize,
{
    fn push_to(self, ctx: &mut Context) -> u32 {
        let v = SerdeValue(self);
        v.push_to(ctx)
    }
}

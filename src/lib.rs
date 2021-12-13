use std::ffi::CStr;
use thiserror::Error;

use duktape_macros::duktape;

mod serialize;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{}", .0)]
    Message(String),
}

type CFunction = unsafe extern "C" fn(*mut duktape_sys::duk_context) -> i32;

pub trait Function {
    const ARGS: i32;

    fn ptr(&self) -> CFunction;
}

macro_rules! push_function(
    ($ctx: ident, $name: ident, $args: literal) => {
        unsafe {
            let idx = duktape_sys::duk_push_c_function($ctx.inner, Some($name), $args);
            let fname = stringify!($name);
            duktape_sys::duk_put_global_lstring($ctx.inner, fname.as_ptr() as *const i8, fname.len() as u64);
            idx
        }
    }
);

#[repr(transparent)]
pub struct Context {
    inner: *mut duktape_sys::duk_context,
}

impl Context {
    pub unsafe fn from_raw(ctx: *mut duktape_sys::duk_context) -> Self {
        Context { inner: ctx }
    }

    pub fn as_raw(&mut self) -> *mut duktape_sys::duk_context {
        self.inner
    }

    pub fn stack_len(&self) -> i32 {
        unsafe { duktape_sys::duk_get_top(self.inner) }
    }

    pub fn push<T: serde::Serialize>(&mut self, value: &T) {
        let mut serializer = serialize::DuktapeSerializer::from_ctx(self);
        value.serialize(&mut serializer).unwrap();
    }

    pub fn push_function<F: Function>(&mut self, f: F) {
        unsafe { duktape_sys::duk_push_c_function(self.inner, Some(f.ptr()), F::ARGS) };
    }

    pub fn register_function<F: Function>(&mut self, name: &str, f: F) {
        self.push_function(f);
        unsafe {
            duktape_sys::duk_put_global_lstring(
                self.inner,
                name.as_ptr() as *const i8,
                name.len() as u64,
            );
        }
    }

    pub fn call_function<F: Function>(&mut self, f: F) -> Result<(), Error> {
        let rv = unsafe { f.ptr()(self.inner) };
        if rv < 0 {
            return Err(Error::Message("function failed".to_string()));
        }
        Ok(())
    }

    pub fn peek<T: serde::de::Deserialize<'static>>(&mut self, idx: i32) -> T {
        let mut deserializer = serialize::DuktapeDeserializer::from_ctx(self, idx);
        T::deserialize(&mut deserializer).unwrap()
    }

    pub fn put_global_string(&mut self, value: &str) {
        unsafe {
            duktape_sys::duk_put_global_lstring(
                self.inner,
                value.as_ptr() as *const i8,
                value.len() as u64,
            );
        }
    }

    pub fn put_prop_index(
        &mut self,
        obj_id: duktape_sys::duk_idx_t,
        idx: duktape_sys::duk_uarridx_t,
    ) {
        unsafe {
            duktape_sys::duk_put_prop_index(self.inner, obj_id, idx);
        }
    }

    pub fn put_prop_string(&mut self, obj_id: duktape_sys::duk_idx_t, val: &str) {
        unsafe {
            duktape_sys::duk_put_prop_lstring(
                self.inner,
                obj_id,
                val.as_ptr() as *const i8,
                val.len() as u64,
            )
        };
    }

    pub fn push_object(&mut self) -> duktape_sys::duk_idx_t {
        unsafe { duktape_sys::duk_push_object(self.inner) }
    }

    pub fn push_array(&mut self) -> duktape_sys::duk_idx_t {
        unsafe { duktape_sys::duk_push_array(self.inner) }
    }

    pub fn push_null(&mut self) {
        unsafe { duktape_sys::duk_push_null(self.inner) }
    }

    pub fn push_double(&mut self, value: f64) {
        unsafe { duktape_sys::duk_push_number(self.inner, value) };
    }

    pub fn push_bool(&mut self, value: bool) {
        let value = if value { 1 } else { 0 };
        unsafe { duktape_sys::duk_push_boolean(self.inner, value) };
    }

    pub fn push_int(&mut self, value: i32) {
        unsafe { duktape_sys::duk_push_int(self.inner, value) };
    }

    pub fn push_uint(&mut self, value: u32) {
        unsafe { duktape_sys::duk_push_uint(self.inner, value) };
    }

    pub fn push_string(&mut self, value: &str) {
        unsafe {
            let _ = duktape_sys::duk_push_lstring(
                self.inner,
                value.as_ptr() as *const i8,
                value.len() as u64,
            );
        }
    }

    pub fn eval<T: serde::Deserialize<'static>>(&mut self, value: &str) -> Result<T, Error> {
        const DUK_COMPILE_EVAL: u32 = 1 << 3;
        const DUK_COMPILE_SAFE: u32 = 1 << 7;
        const DUK_COMPILE_NOSOURCE: u32 = 1 << 9;
        const DUK_COMPILE_NOFILENAME: u32 = 1 << 11;

        let rv = unsafe {
            duktape_sys::duk_eval_raw(
                self.inner,
                value.as_ptr() as *const i8,
                value.len() as u64,
                DUK_COMPILE_EVAL | DUK_COMPILE_NOSOURCE | DUK_COMPILE_NOFILENAME | DUK_COMPILE_SAFE,
            )
        };
        if rv != 0 {
            let mut len = 0;
            let ptr = unsafe { duktape_sys::duk_safe_to_lstring(self.inner, -1, &mut len) };
            let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
            let str = std::str::from_utf8(slice).unwrap();
            return Err(Error::Message(str.to_owned()));
        } else {
            Ok(self.peek(-1))
        }
    }

    pub fn pop(&mut self) {
        unsafe {
            duktape_sys::duk_pop(self.inner);
        }
    }

    pub fn pop_value<T: serde::de::Deserialize<'static>>(&mut self) -> T {
        let value = self.peek(-1);
        self.pop();
        value
    }

    pub fn pop_n(&mut self, n: i32) {
        unsafe {
            duktape_sys::duk_pop_n(self.inner, n);
        }
    }

    pub fn dup(&mut self, idx: duktape_sys::duk_idx_t) {
        unsafe { duktape_sys::duk_dup(self.inner, idx) }
    }

    pub fn call(&mut self, n_args: duktape_sys::duk_idx_t) {
        unsafe { duktape_sys::duk_call(self.inner, n_args) }
    }

    pub fn get_global_str(&mut self, value: &str) -> bool {
        let val = unsafe {
            duktape_sys::duk_get_global_lstring(
                self.inner,
                value.as_ptr() as *const i8,
                value.len() as u64,
            )
        };
        val > 0
    }

    pub fn get_bool(&mut self, idx: duktape_sys::duk_idx_t) -> bool {
        unsafe { duktape_sys::duk_require_boolean(self.inner, idx) > 0 }
    }

    pub fn get_uint(&mut self, idx: duktape_sys::duk_idx_t) -> u32 {
        unsafe { duktape_sys::duk_require_uint(self.inner, idx) }
    }

    pub fn get_int(&mut self, idx: duktape_sys::duk_idx_t) -> i32 {
        unsafe { duktape_sys::duk_require_int(self.inner, idx) }
    }

    pub fn get_number(&mut self, idx: duktape_sys::duk_idx_t) -> f64 {
        unsafe { duktape_sys::duk_require_number(self.inner, idx) }
    }

    pub fn get_null(&mut self, idx: duktape_sys::duk_idx_t) {
        unsafe { duktape_sys::duk_require_null(self.inner, idx) }
    }

    pub fn get_string(&mut self, idx: duktape_sys::duk_idx_t) -> String {
        let mut len = 0;
        let ptr =
            unsafe { duktape_sys::duk_require_lstring(self.inner, idx, &mut len) } as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
        let s = std::str::from_utf8(slice).unwrap();
        s.to_owned()
    }

    pub fn get_object(&mut self, idx: duktape_sys::duk_idx_t) {
        unsafe { duktape_sys::duk_require_object(self.inner, idx) }
    }

    pub fn get_prop(&mut self, name: &str, idx: duktape_sys::duk_idx_t) -> bool {
        unsafe {
            duktape_sys::duk_get_prop_lstring(
                self.inner,
                idx,
                name.as_ptr() as *const i8,
                name.len() as u64,
            ) > 0
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        extern "C" fn fatal(_udata: *mut std::ffi::c_void, msg: *const i8) {
            let msg = unsafe { CStr::from_ptr(msg) };
            panic!("{:?}", msg.to_str());
        }
        Context {
            inner: unsafe {
                duktape_sys::duk_create_heap(None, None, None, std::ptr::null_mut(), Some(fatal))
            },
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        //unsafe { duktape_sys::duk_destroy_heap(self.inner) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_stuff() {
        extern "C" fn fatal(_udata: *mut std::ffi::c_void, msg: *const i8) {
            let msg = unsafe { CStr::from_ptr(msg) };
            panic!("{:?}", msg.to_str());
        }
        unsafe {
            const DUK_COMPILE_EVAL: u32 = 1 << 3;
            const DUK_COMPILE_NOSOURCE: u32 = 1 << 9;
            const DUK_COMPILE_NOFILENAME: u32 = 1 << 11;

            extern "C" fn print(ctx: *mut duktape_sys::duk_context) -> i32 {
                let value = " ";
                unsafe {
                    let _ = duktape_sys::duk_push_lstring(
                        ctx,
                        value.as_ptr() as *const i8,
                        value.len() as u64,
                    );
                    duktape_sys::duk_insert(ctx, 0);
                    duktape_sys::duk_join(ctx, duktape_sys::duk_get_top(ctx) - 1);
                    let mut len = 0;
                    let s = duktape_sys::duk_safe_to_lstring(ctx, -1, &mut len);
                    let slice: &[i8] = std::slice::from_raw_parts(s, len as usize);
                    let s = std::str::from_utf8(std::mem::transmute(slice));
                    println!("{:?}", s);
                }
                0
            }

            let ctx =
                duktape_sys::duk_create_heap(None, None, None, std::ptr::null_mut(), Some(fatal));
            duktape_sys::duk_push_c_function(ctx, Some(print), -1);
            let fname = "print";
            duktape_sys::duk_put_global_lstring(
                ctx,
                fname.as_ptr() as *const i8,
                fname.len() as u64,
            );
            let call = "print('hello world');";
            duktape_sys::duk_eval_raw(
                ctx,
                call.as_ptr() as *const i8,
                call.len() as u64,
                DUK_COMPILE_EVAL | DUK_COMPILE_NOSOURCE | DUK_COMPILE_NOFILENAME,
            );
            duktape_sys::duk_pop(ctx);
        }
    }

    #[test]
    fn it_works() {
        use crate as duktape;
        let mut ctx = Context::default();

        #[duktape]
        fn print(ctx: &mut Context) -> String {
            ctx.push_string(",");
            unsafe {
                duktape_sys::duk_insert(ctx.as_raw(), 0);
                duktape_sys::duk_join(ctx.as_raw(), duktape_sys::duk_get_top(ctx.as_raw()) - 1);
            };
            let v = ctx.peek(-1);
            println!("{}", v);
            v
        }

        ctx.register_function("print", Print);
        let s = ctx.eval::<String>("print('hello', 1, 2);").unwrap();
        assert_eq!(s, "hello,1,2");
        ctx.pop();
    }

    #[test]
    fn serialized() {
        use crate as duktape;

        #[derive(serde::Serialize, Debug)]
        struct T {
            hello: String,
        }
        let t = T {
            hello: "world".to_string(),
        };

        #[duktape]
        fn print(ctx: &mut Context) -> String {
            ctx.push_string(",");
            unsafe {
                duktape_sys::duk_insert(ctx.as_raw(), 0);
                duktape_sys::duk_join(ctx.as_raw(), duktape_sys::duk_get_top(ctx.as_raw()) - 1);
            };
            let v = ctx.peek(-1);
            println!("RES: {}", v);
            v
        }

        let mut ctx = Context::default();
        ctx.push_function(Print);
        ctx.push(&t);
        ctx.call(1);

        //ctx.eval("print('hello', 1);");
        //ctx.pop();
    }
}

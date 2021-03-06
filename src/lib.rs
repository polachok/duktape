//! bindings for Duktape javascript engine
//!
//! # Initialize a context
//! ```
//!     use duktape::Context;
//!
//!     let mut ctx = Context::default();
//!     let x: u32 = ctx.eval("1+2").unwrap();
//!     assert_eq!(x, 3);
//! ```
//!
//! # Add rust function binding
//! ```
//!     use duktape::Context;
//!     use duktape_macros::duktape;
//!
//!     let mut ctx = Context::default();
//!     #[duktape]
//!     fn native_adder(ctx: &mut Context) -> u32 {
//!         let n = ctx.stack_len();
//!         let mut res = 0;
//!
//!         for i in 0..n {
//!             res += ctx.get_uint(i);
//!         }
//!         res
//!     }
//!
//!     ctx.register_function("adder", NativeAdder);
//!     let res: u32 = ctx.eval("adder(2,3)").unwrap();
//!     assert_eq!(res, 5);
//! ```
//!
//! # Push an object
//! ```
//!     use duktape::{PushValue, PeekValue, Context};
//!     use duktape_macros::{duktape, Value};
//!
//!     let mut ctx = Context::default();
//!     #[derive(Value)]
//!     #[duktape(Peek, Push, Methods("sumFields"))]
//!     struct Test {
//!         a: u32,
//!         b: u32,
//!     }
//!
//!     impl Test {
//!         #[duktape(this = "Test")]
//!         fn sum_fields(&self) -> u32 {
//!             self.a + self.b
//!         }
//!     }
//!
//!     let t = Test { a: 4, b: 2 };
//!     let obj_id = t.push_to(&mut ctx);
//!     ctx.push_string("sumFields");
//!     ctx.call_prop(obj_id as i32, 0);
//!     let sum = u32::peek_at(&mut ctx, -1).unwrap();
//!     assert_eq!(sum, 6);
//! ```

use std::ffi::CStr;
use thiserror::Error;

pub use duktape_macros::{duktape, Value};
#[doc(hidden)]
pub use duktape_sys as sys;
pub use value::{PeekValue, PushValue};

pub mod serialize;
pub mod value;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{}", .0)]
    Message(String),
    #[error("{}", .0)]
    Peek(#[source] value::PeekError),
}

type CFunction = unsafe extern "C" fn(*mut duktape_sys::duk_context) -> i32;

pub trait Function {
    const ARGS: i32;

    fn ptr(&self) -> CFunction;
}

#[repr(transparent)]
pub struct Context {
    inner: *mut duktape_sys::duk_context,
}

impl Context {
    /// # Safety
    ///
    /// This function should only be called with pointer received
    /// by calling .as_raw() on a Context object
    pub unsafe fn from_raw(ctx: *mut duktape_sys::duk_context) -> Self {
        Context { inner: ctx }
    }

    pub fn as_raw(&mut self) -> *mut duktape_sys::duk_context {
        self.inner
    }

    pub fn stack_len(&self) -> i32 {
        unsafe { duktape_sys::duk_get_top(self.inner) }
    }

    pub fn stack_top(&self) -> u32 {
        unsafe {
            duktape_sys::duk_get_top_index(self.inner)
                .try_into()
                .unwrap()
        }
    }

    pub fn is_null_or_undefined(&mut self, idx: i32) -> bool {
        use duktape_sys::{DUK_TYPE_MASK_NULL, DUK_TYPE_MASK_UNDEFINED};

        unsafe {
            duktape_sys::duk_get_type_mask(self.inner, idx)
                & (DUK_TYPE_MASK_NULL | DUK_TYPE_MASK_UNDEFINED)
                != 0
        }
    }

    pub fn push<T: PushValue>(&mut self, value: T) -> u32 {
        value.push_to(self)
    }

    pub fn push_fixed_buffer(&mut self, value: &[u8]) {
        let buf = unsafe { duktape_sys::duk_push_buffer_raw(self.inner, value.len() as u64, 0) };
        let buf = buf as *mut u8;
        unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), buf, value.len()) }
    }

    pub fn push_this(&mut self) {
        unsafe { duktape_sys::duk_push_this(self.inner) }
    }

    pub fn push_function<F: Function>(&mut self, f: F) {
        unsafe { duktape_sys::duk_push_c_function(self.inner, Some(f.ptr()), F::ARGS) };
    }

    // Push p into the stack as a pointer value. Duktape won't interpret the pointer in any manner.
    pub fn push_pointer(&mut self, p: *const std::ffi::c_void) {
        unsafe { duktape_sys::duk_push_pointer(self.inner, p as *mut _) };
    }

    pub fn get_pointer(&mut self, idx: i32) -> *const std::ffi::c_void {
        unsafe { duktape_sys::duk_require_pointer(self.inner, idx) }
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

    pub fn peek<T: PeekValue>(&mut self, idx: i32) -> Result<T, value::PeekError> {
        T::peek_at(self, idx)
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

    pub fn put_prop_bytes(&mut self, obj_id: duktape_sys::duk_idx_t, val: &[u8]) {
        unsafe {
            duktape_sys::duk_put_prop_lstring(
                self.inner,
                obj_id,
                val.as_ptr() as *const i8,
                val.len() as u64,
            )
        };
    }

    pub fn push_object(&mut self) -> u32 {
        unsafe { duktape_sys::duk_push_object(self.inner).try_into().unwrap() }
    }

    pub fn push_array(&mut self) -> u32 {
        unsafe { duktape_sys::duk_push_array(self.inner).try_into().unwrap() }
    }

    pub fn push_null(&mut self) {
        unsafe { duktape_sys::duk_push_null(self.inner) }
    }

    pub fn push_undefined(&mut self) {
        unsafe { duktape_sys::duk_push_undefined(self.inner) }
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

    pub fn eval<T: PeekValue>(&mut self, value: &str) -> Result<T, Error> {
        use duktape_sys::{
            DUK_COMPILE_EVAL, DUK_COMPILE_NOFILENAME, DUK_COMPILE_NOSOURCE, DUK_COMPILE_SAFE,
        };

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
            Err(Error::Message(str.to_owned()))
        } else {
            self.peek(-1).map_err(Error::Peek)
        }
    }

    fn pop_it(&mut self) {
        unsafe {
            duktape_sys::duk_pop(self.inner);
        }
    }

    pub fn pop(&mut self) -> Result<(), Error> {
        self.pop_value::<()>().map_err(Error::Peek)
    }

    pub fn pop_value<T: PeekValue>(&mut self) -> Result<T, value::PeekError> {
        T::pop(self)
    }

    pub fn pop_n(&mut self, n: i32) {
        unsafe {
            duktape_sys::duk_pop_n(self.inner, n);
        }
    }

    pub fn swap(&mut self, a: i32, b: i32) {
        unsafe { duktape_sys::duk_swap(self.inner, a, b) }
    }

    pub fn dup(&mut self, idx: duktape_sys::duk_idx_t) {
        unsafe { duktape_sys::duk_dup(self.inner, idx) }
    }

    pub fn call(&mut self, n_args: duktape_sys::duk_idx_t) -> Result<(), Error> {
        let rc = unsafe { duktape_sys::duk_pcall(self.inner, n_args) };
        if rc == 0 {
            Ok(())
        } else {
            let mut len = 0;
            let err =
                unsafe { duktape_sys::duk_safe_to_lstring(self.inner, -1, &mut len) } as *const u8;
            let msg = unsafe { std::slice::from_raw_parts(err, len as usize) };
            let str = std::str::from_utf8(msg).unwrap().to_owned();
            Err(Error::Message(str))
        }
    }

    pub fn call_prop(
        &mut self,
        obj_id: duktape_sys::duk_idx_t,
        n_args: duktape_sys::duk_idx_t,
    ) -> Result<(), Error> {
        let rc = unsafe { duktape_sys::duk_pcall_prop(self.inner, obj_id, n_args) };
        if rc == 0 {
            Ok(())
        } else {
            let mut len = 0;
            let err =
                unsafe { duktape_sys::duk_safe_to_lstring(self.inner, -1, &mut len) } as *const u8;
            let msg = unsafe { std::slice::from_raw_parts(err, len as usize) };
            let str = std::str::from_utf8(msg).unwrap().to_owned();
            Err(Error::Message(str))
        }
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

    pub fn get_length(&mut self, idx: duktape_sys::duk_idx_t) -> usize {
        unsafe { duktape_sys::duk_get_length(self.inner, idx) as usize }
    }

    pub fn get_prop(&mut self, idx: duktape_sys::duk_idx_t, name: &str) -> bool {
        unsafe {
            duktape_sys::duk_get_prop_lstring(
                self.inner,
                idx,
                name.as_ptr() as *const i8,
                name.len() as u64,
            ) > 0
        }
    }

    pub fn get_prop_bytes(&mut self, idx: duktape_sys::duk_idx_t, name: &[u8]) -> bool {
        unsafe {
            duktape_sys::duk_get_prop_lstring(
                self.inner,
                idx,
                name.as_ptr() as *const i8,
                name.len() as u64,
            ) > 0
        }
    }

    pub fn get_prop_index(&mut self, prop_idx: u32, idx: duktape_sys::duk_idx_t) -> bool {
        unsafe { duktape_sys::duk_get_prop_index(self.inner, idx, prop_idx) > 0 }
    }

    pub fn get_buffer(&mut self, idx: duktape_sys::duk_idx_t) -> Vec<u8> {
        let mut buf_len = 0;
        let buf_ptr = unsafe { duktape_sys::duk_require_buffer_data(self.inner, idx, &mut buf_len) }
            as *const u8;
        if buf_len == 0 {
            Vec::new()
        } else {
            let slice = unsafe { std::slice::from_raw_parts(buf_ptr, buf_len as usize) };
            slice.to_vec()
        }
    }

    pub fn get_buffer_opt(&mut self, idx: duktape_sys::duk_idx_t) -> Option<Vec<u8>> {
        let mut buf_len = 0;
        let buf_ptr =
            unsafe { duktape_sys::duk_get_buffer_data(self.inner, idx, &mut buf_len) } as *const u8;
        if buf_len == 0 || buf_ptr.is_null() {
            None
        } else {
            let slice = unsafe { std::slice::from_raw_parts(buf_ptr, buf_len as usize) };
            Some(slice.to_vec())
        }
    }

    pub fn is_array(&mut self, idx: duktape_sys::duk_idx_t) -> bool {
        unsafe { duktape_sys::duk_is_array(self.inner, idx) > 0 }
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
        unsafe { duktape_sys::duk_destroy_heap(self.inner) }
        self.inner = std::ptr::null_mut();
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
            use duktape_sys::{DUK_COMPILE_EVAL, DUK_COMPILE_NOFILENAME, DUK_COMPILE_NOSOURCE};

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
            let v = ctx.peek(-1).unwrap();
            println!("{}", v);
            v
        }

        ctx.register_function("print", Print);
        let s = ctx.eval::<String>("print('hello', 1, 2);").unwrap();
        assert_eq!(s, "hello,1,2");
        ctx.pop_it();
    }

    #[test]
    fn serialized() {
        use crate as duktape;

        #[derive(serde::Serialize, serde::Deserialize, Value, Debug)]
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
            let v = ctx.peek(-1).unwrap();
            println!("RES: {}", v);
            v
        }

        let mut ctx = Context::default();
        ctx.push_function(Print);
        ctx.push(&t);
        ctx.call(1).unwrap();

        //ctx.eval("print('hello', 1);");
        //ctx.pop();
    }
}

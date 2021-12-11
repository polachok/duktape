use std::ffi::CStr;

//pub type Function = fn(&mut Context) -> i32;
//

macro_rules! declare_function(
    ($name: ident, $args: literal, $e: expr) => {
        unsafe extern "C" fn $name(ctx: *mut duktape_sys::duk_context) -> i32 {
            let mut ctx = Context { inner: ctx };
            let val = $e(&mut ctx);
            std::mem::forget(ctx);
            val
        }
    }
);

macro_rules! push_function(
    ($ctx: ident, $name: ident, $args: literal) => {
        unsafe {
            duktape_sys::duk_push_c_function($ctx.inner, Some($name), $args);
            let fname = stringify!($name);
            duktape_sys::duk_put_global_lstring($ctx.inner, fname.as_ptr() as *const i8, fname.len() as u64);
        }
    }
);

pub enum ArgCount {
    Exact(i32),
    Variable,
}

pub trait Function: Sized + 'static {
    const ARGS: ArgCount;

    fn call(ctx: &mut Context) -> i32;
}

#[repr(transparent)]
pub struct Context {
    inner: *mut duktape_sys::duk_context,
}

impl Context {
    pub fn push_number(&mut self, value: f64) {
        unsafe { duktape_sys::duk_push_number(self.inner, value) };
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

    pub fn eval(&mut self, value: &str) {
        const DUK_COMPILE_EVAL: u32 = 1 << 3;
        const DUK_COMPILE_NOSOURCE: u32 = 1 << 9;
        const DUK_COMPILE_NOFILENAME: u32 = 1 << 11;
        unsafe {
            duktape_sys::duk_eval_raw(
                self.inner,
                value.as_ptr() as *const i8,
                value.len() as u64,
                DUK_COMPILE_EVAL | DUK_COMPILE_NOSOURCE | DUK_COMPILE_NOFILENAME,
            )
        };
    }

    pub fn pop(&mut self) {
        unsafe {
            duktape_sys::duk_pop(self.inner);
        }
    }
    /*
        pub fn push_function<F: Function>(&mut self, f: F) {
            unsafe {
                duktape_sys::duk_push_c_function(self.inner, Some(func), args);
            }
        }
    */
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
        let mut ctx = Context::default();

        struct Print {}

        impl Function for Print {
            const ARGS: ArgCount = ArgCount::Variable;

            fn call(ctx: &mut Context) -> i32 {
                let mut len: u64 = 0;
                let cstr = unsafe {
                    ctx.push_string(",");
                    duktape_sys::duk_insert(ctx.inner, 0);
                    duktape_sys::duk_join(ctx.inner, duktape_sys::duk_get_top(ctx.inner) - 1);
                    CStr::from_ptr(duktape_sys::duk_safe_to_lstring(ctx.inner, -1, &mut len))
                };
                eprintln!("{:?}", cstr.to_str());
                0
            }
        }

        declare_function!(print, -1, Print::call);
        //ctx.push_number(1.0);
        push_function!(ctx, print, -1);
        ctx.eval("print('hello', 1);");
        ctx.pop();
    }
}

use std::ffi::CStr;

//pub type Function = fn(&mut Context) -> i32;
//

macro_rules! declare_function(
    ($name: ident, $args: literal, $e: expr) => {
        unsafe extern "C" fn $name(ctx: *mut duktape_sys::duk_context) -> i32 {
            let ctx = &mut *(ctx as *mut Context);
            $e(ctx)
        }
    }
);

macro_rules! push_function(
    ($ctx: ident, $name: ident, $args: literal) => {
        unsafe {
            duktape_sys::duk_push_c_function($ctx.inner, Some($name), $args);
            let fname = concat!(stringify!($name), "\0");
            duktape_sys::duk_put_global_string($ctx.inner, fname.as_ptr() as *const i8);
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
    fn it_works() {
        let mut ctx = Context::default();

        struct Print {}

        impl Function for Print {
            const ARGS: ArgCount = ArgCount::Variable;

            fn call(ctx: &mut Context) -> i32 {
                println!("HELLO");
                let mut len: u64 = 0;
                let cstr = unsafe {
                    CStr::from_ptr(duktape_sys::duk_safe_to_lstring(ctx.inner, -1, &mut len))
                };
                eprintln!("{:?}", cstr.to_str());
                0
            }
        }
        declare_function!(print, -1, Print::call);
        ctx.push_number(1.0);
        push_function!(ctx, print, -1);
        ctx.eval("print('hello');");
        ctx.pop();
    }
}

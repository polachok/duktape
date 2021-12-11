use duktape_macros::*;
use duktape_sys;

#[repr(transparent)]
pub struct Context {
    inner: *mut duktape_sys::duk_context,
}

impl Default for Context {
    fn default() -> Self {
        extern "C" fn fatal(_udata: *mut std::ffi::c_void, msg: *const i8) {
            let msg = unsafe { std::ffi::CStr::from_ptr(msg) };
            panic!("{:?}", msg.to_str());
        }
        Context {
            inner: unsafe {
                duktape_sys::duk_create_heap(None, None, None, std::ptr::null_mut(), Some(fatal))
            },
        }
    }
}

#[test]
fn it_works() {
    #[duktape]
    fn bla(_ctx: &mut Context, a: u32, b: u32) -> u32 {
        a + b
    }

    let ctx = Context::default();
    unsafe { duktape_sys::duk_push_uint(ctx.inner, 1) }
    unsafe { duktape_sys::duk_push_uint(ctx.inner, 2) }
    unsafe { bla(ctx.inner) };
    let rv = unsafe { duktape_sys::duk_opt_uint(ctx.inner, -1, 42) };
    println!("{:?}", rv);
}

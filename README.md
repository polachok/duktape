# duktape

Duktape is an embeddable Javascript engine, with a focus on portability and compact footprint.

## Add to build
Cargo.toml:

  duktape = { git = "https://github.com/polachok/duktape", rev = "45968b5be7240c6adaa5232b32085b7ac94b21f7" }
  duktape-sys = { git = "https://github.com/polachok/duktape", rev = "45968b5be7240c6adaa5232b32085b7ac94b21f7" }

 bindings for Duktape javascript engine

# Initialize a context
```
     use duktape::Context;

     let mut ctx = Context::default();
     let x: u32 = ctx.eval("1+2").unwrap();
     assert_eq!(x, 3);
```

## Add rust function binding
 ```
     use duktape::Context;
     use duktape_macros::duktape;

     let mut ctx = Context::default();
     #[duktape]
     fn native_adder(ctx: &mut Context) -> u32 {
         let n = ctx.stack_len();
         let mut res = 0;

         for i in 0..n {
             res += ctx.get_uint(i);
         }
         res
     }

     ctx.register_function("adder", NativeAdder);
     let res: u32 = ctx.eval("adder(2,3)").unwrap();
     assert_eq!(res, 5);
 ```

## Push an object
 ```
     use duktape::{PushValue, PeekValue, Context};
     use duktape_macros::{duktape, Value};

     let mut ctx = Context::default();
     #[derive(Value)]
     #[duktape(Peek, Push, Methods("sumFields"))]
     struct Test {
         a: u32,
         b: u32,
     }

     impl Test {
         #[duktape(this = "Test")]
         fn sum_fields(&self) -> u32 {
             self.a + self.b
         }
     }

     let t = Test { a: 4, b: 2 };
     let obj_id = t.push_to(&mut ctx);
     ctx.push_string("sumFields");
     ctx.call_prop(obj_id as i32, 0);
     let sum = u32::peek_at(&mut ctx, -1).unwrap();
     assert_eq!(sum, 6);
 ```

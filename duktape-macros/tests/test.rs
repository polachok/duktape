use duktape::Context;
use duktape_macros::*;

#[test]
fn method() {
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Obj {
        data: String,
        counter: u32,
    }

    impl Obj {
        #[duktape(Obj)]
        fn return_data(&self) -> String {
            self.data.clone()
        }

        fn push(&self, ctx: &mut Context) {
            let idx = ctx.push(self);
            Self::register_return_data(ctx, idx, "getData");
        }
    }
    let mut ctx = Context::default();
    let data = Obj {
        data: "hello".to_string(),
        counter: 5,
    };
    ctx.eval::<()>("var getData = function(obj) { return obj.getData() }")
        .unwrap();
    ctx.get_global_str("getData");
    data.push(&mut ctx);
    ctx.call(1);
    let res = ctx.peek::<String>(-1);
    println!("method output: {}", res);
    assert_eq!(res, "hello");
}

#[test]
fn object() {
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Pdo {
        data: String,
        counter: u32,
    }

    #[duktape]
    fn do_it(_ctx: &mut Context, obj: Pdo) -> u32 {
        println!("{:?}", obj);
        obj.counter
    }

    let mut ctx = Context::default();
    let data = Pdo {
        data: "hello".to_string(),
        counter: 3,
    };
    ctx.push(&data);
    ctx.call_function(DoIt).unwrap();
}

#[test]
fn adder() {
    #[duktape]
    fn bla(_ctx: &mut Context, a: u32, b: u32) -> u32 {
        a + b
    }

    let mut ctx = Context::default();
    ctx.push(&1u32);
    ctx.push(&2u32);
    let a = ctx.peek::<u32>(0);
    assert_eq!(a, 1u32);
    let b = ctx.peek::<u32>(1);
    assert_eq!(b, 2u32);
    ctx.call_function(Bla).unwrap();
    let rv = ctx.peek::<u32>(-1);
    assert_eq!(3, rv);
}

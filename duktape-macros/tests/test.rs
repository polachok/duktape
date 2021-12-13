use duktape::Context;
use duktape_macros::*;

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

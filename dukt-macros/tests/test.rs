use duktape::Context;
use duktape_macros::*;

#[test]
fn test_newtype() {
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Value)]
    #[duktape(Peek, Push, Methods("toString", "toInt"))]
    pub struct Obj {
        x: u32,
    }

    impl Obj {
        #[duktape(this = "Obj")]
        fn to_int(&self) -> u32 {
            self.x
        }

        #[duktape(this = "Obj")]
        fn to_string(&self) -> String {
            self.x.to_string()
        }
    }

    #[derive(Value)]
    struct ObjWrapper(Rc<RefCell<Obj>>);
}

#[test]
fn test_methods() {
    use duktape::{PeekValue, PushValue};
    use std::rc::Rc;

    #[derive(Value)]
    #[duktape(Peek, Push, Methods("toString", "toInt"))]
    pub struct Obj {
        t: Rc<Vec<u8>>,
    }

    impl Obj {
        #[duktape(this = "Obj")]
        fn to_int(&self) -> u32 {
            0
        }

        #[duktape(this = "Obj")]
        fn to_string(&self) -> String {
            String::new()
        }
    }
    let mut ctx = Context::default();
    let obj = Obj {
        t: Rc::new(Vec::new()),
    };
    let idx = obj.push_to(&mut ctx);
    let _obj = Obj::peek_at(&mut ctx, idx as i32).unwrap();
}

#[test]
fn test_hidden() {
    #[derive(Value)]
    pub struct Obj {
        #[hidden]
        data: Vec<u8>,
    }
}

#[test]
fn ret_ref_array() {
    #[derive(Debug, serde::Deserialize, serde::Serialize, Value)]
    #[duktape(Peek, Push, Serialize)]
    pub struct Obj {
        data: Vec<u8>,
    }

    impl Obj {
        #[duktape(this = "Obj")]
        fn get_data(&self) -> &[u8] {
            &self.data[..]
        }

        fn push(&self, ctx: &mut Context) {
            let idx = ctx.push(self.push_value());
            Self::register_get_data(ctx, idx, "getData");
        }
    }

    let obj = Obj {
        data: [0, 1, 2, 3].to_vec(),
    };
    let mut ctx = Context::default();
    ctx.eval::<()>("var getData = function(obj) { return obj.getData() }")
        .unwrap();
    ctx.get_global_str("getData");
    obj.push(&mut ctx);
    ctx.call(1).unwrap();
    let res = ctx.peek::<Vec<u8>>(-1).unwrap();
    assert_eq!(res, &[0, 1, 2, 3]);
}

#[test]
fn ret_ref_buf() {
    #[derive(Debug, serde::Deserialize, serde::Serialize, Value)]
    pub struct Obj {
        #[data]
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }

    impl Obj {
        #[duktape(this = "Obj")]
        fn get_data(&self) -> &[u8] {
            &self.data[..]
        }

        fn push(&self, ctx: &mut Context) {
            let idx = ctx.push(self);
            Self::register_get_data(ctx, idx, "getData");
        }
    }

    let obj = Obj {
        data: [0, 1, 2, 3].to_vec(),
    };
    let mut ctx = Context::default();
    ctx.eval::<()>("var getData = function(obj) { return obj.getData() }")
        .unwrap();
    ctx.get_global_str("getData");
    obj.push(&mut ctx);
    ctx.call(1).unwrap();
    let res = ctx.peek::<Vec<u8>>(-1).unwrap();
    assert_eq!(res, &[0, 1, 2, 3]);
}

#[test]
fn method() {
    #[derive(Debug, serde::Deserialize, serde::Serialize, Value)]
    pub struct Obj {
        data: String,
        counter: u32,
    }

    impl Obj {
        #[duktape(this = "Obj")]
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
    ctx.call(1).unwrap();
    let res = ctx.peek::<String>(-1).unwrap();
    println!("method output: {}", res);
    assert_eq!(res, "hello");
}

#[test]
fn object() {
    #[derive(Debug, serde::Deserialize, serde::Serialize, Value)]
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
fn generics() {
    #[duktape]
    fn bla(_ctx: &mut Context, data: Vec<u8>) -> Vec<u8> {
        data
    }
}

#[test]
fn arrays() {
    #[duktape]
    fn bla(_ctx: &mut Context, _data: Vec<u8>) -> [u8; 20] {
        panic!()
    }
}

#[test]
fn adder() {
    #[duktape]
    fn bla(_ctx: &mut Context, a: u32, b: u32) -> u32 {
        assert_eq!(a, 1);
        assert_eq!(b, 2);
        a + b
    }

    let mut ctx = Context::default();
    ctx.push(&1u32);
    ctx.push(&2u32);
    let a = ctx.peek::<u32>(0).unwrap();
    assert_eq!(a, 1u32);
    let b = ctx.peek::<u32>(1).unwrap();
    assert_eq!(b, 2u32);
    ctx.call_function(Bla).unwrap();
    let rv = ctx.peek::<u32>(-1).unwrap();
    assert_eq!(3, rv);
}

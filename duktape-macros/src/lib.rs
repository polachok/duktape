use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemFn};

#[proc_macro_attribute]
pub fn duktape(attr: TokenStream, input: TokenStream) -> TokenStream {
    let parsed: ItemFn = syn::parse_macro_input!(input);
    let mut args = Vec::new();
    let fn_name = parsed.sig.ident.clone();
    let (return_count, return_type) = match &parsed.sig.output {
        syn::ReturnType::Default => (0, None),
        syn::ReturnType::Type(_, typ) => {
            let ident = match &**typ {
                syn::Type::Path(path) => path.path.get_ident().unwrap().clone(),
                _ => panic!("unsupported return type"),
            };
            (1, Some(ident))
        }
    };
    for (i, param) in parsed.sig.inputs.iter().enumerate() {
        match param {
            syn::FnArg::Receiver(_) => panic!("self not supported"),
            syn::FnArg::Typed(pat_typ) => match &*pat_typ.ty {
                syn::Type::Path(path) => {
                    args.push(path.path.get_ident().unwrap().clone());
                }
                syn::Type::Reference(re) => {
                    if i > 0 {
                        panic!("unsupported reference");
                    }
                }
                _ => panic!("unsupported"),
            },
        }
    }
    let args_count = args.len() as i32;

    let args_names: Vec<_> = args
        .iter()
        .enumerate()
        .map(|(i, typ)| Ident::new(&format!("arg_{}", i), Span::call_site()))
        .collect();

    let args_getters: Vec<_> = args
        .iter()
        .zip(args_names.iter())
        .enumerate()
        .map(|(i, (typ, name))| match typ.to_string().as_str() {
            "u32" => {
                quote!(
                    let #name = duktape_sys::duk_get_int(ctx,  -(1 + #i as i32)) as #typ;
                )
            }
            _ => todo!(),
        })
        .collect();
    let push_result = match return_type {
        Some(typ) => match typ.to_string().as_str() {
            "u32" => {
                quote!(
                    duktape_sys::duk_push_uint(ctx, result);
                )
            }
            _ => todo!(),
        },
        None => quote!(),
    };

    let res = quote!(
        unsafe extern "C" fn #fn_name(ctx: *mut duktape_sys::duk_context) -> i32 {
            #parsed
            let n = duktape_sys::duk_get_top(ctx);
            if n < #args_count {
                return -1;
            }
            #(#args_getters)*
            let rctx = &mut Context { inner: ctx };
            let result = #fn_name(rctx, #(#args_names),*);
            #push_result
            #return_count
        }
    );

    println!("{}", res);
    res.into()
}

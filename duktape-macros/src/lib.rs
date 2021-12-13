use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, ItemFn};

#[proc_macro_attribute]
pub fn duktape(attr: TokenStream, input: TokenStream) -> TokenStream {
    let parsed: ItemFn = syn::parse_macro_input!(input);
    let mut args = Vec::new();
    let fn_name = parsed.sig.ident.clone();
    let struct_name = Ident::new(
        &inflections::case::to_pascal_case(&fn_name.to_string()),
        Span::call_site(),
    );
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
                syn::Type::Reference(_re) => {
                    if i > 0 {
                        panic!("unsupported reference");
                    }
                }
                _ => panic!("unsupported"),
            },
        }
    }
    let args_count = args.len() as i32;
    let raw_args_count = (args_count - 1).max(0);

    let args_names: Vec<_> = args
        .iter()
        .enumerate()
        .map(|(i, typ)| Ident::new(&format!("arg_{}", i), Span::call_site()))
        .collect();

    let args_getters: Vec<_> = args
        .iter()
        .zip(args_names.iter())
        .enumerate()
        .map(|(i, (typ, name))| {
            quote!(
                let #name = ctx.peek::<#typ>(-(1 + #i as i32));
            )
        })
        .collect();
    let push_result = match return_type {
        Some(_) => {
            quote!(
                ctx.push(&result);
            )
        }
        None => quote!(),
    };

    let res = quote!(
        struct #struct_name;

        impl duktape::Function for #struct_name {
            const ARGS: i32 = #raw_args_count;

            fn ptr(&self) -> unsafe extern "C" fn(*mut duktape_sys::duk_context) -> i32 {
                Self::#fn_name
            }
        }

        impl #struct_name {
            pub unsafe extern "C" fn #fn_name(raw: *mut duktape_sys::duk_context) -> i32 {
                #parsed

                let ctx = &mut duktape::Context::from_raw(raw);
                let n = ctx.stack_len();
                if n < #args_count {
                    return -1;
                }
                #(#args_getters)*
                ctx.pop_n(#raw_args_count);
                let result = #fn_name(ctx, #(#args_names),*);
                #push_result
                #return_count
            }
        }
    );

    //println!("{}", res);
    res.into()
}

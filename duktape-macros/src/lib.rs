use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, ItemFn};

#[proc_macro_derive(Value, attributes(duktape))]
pub fn value(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let ident = input.ident.clone();
    let options = input
        .attrs
        .iter()
        .filter(|attr| {
            if let Some(ident) = attr.path.get_ident() {
                ident.to_string() == "duktape"
            } else {
                false
            }
        })
        .filter_map(|attr| attr.parse_meta().ok())
        .filter_map(|meta| match meta {
            syn::Meta::List(list) => Some(list),
            _ => None,
        })
        .map(|list| {
            let mut idents = vec![];
            for val in list.nested {
                match val {
                    syn::NestedMeta::Meta(meta) => match meta {
                        syn::Meta::Path(path) => idents.push(path.get_ident().unwrap().clone()),
                        _ => {}
                    },
                    syn::NestedMeta::Lit(_) => {}
                }
            }
            idents
        })
        .flatten()
        .collect::<Vec<Ident>>();

    const GENERATE_PEEK: u8 = 1;
    const GENERATE_PUSH: u8 = 2;
    const GENERATE_AS_SERIALIZE: u8 = 4;
    const DEFAULT: u8 = GENERATE_PEEK | GENERATE_PUSH | GENERATE_AS_SERIALIZE;
    let flags = if options.is_empty() {
        DEFAULT
    } else {
        let mut flags = 0;
        for option in &options {
            flags |= match option.to_string().as_str() {
                "Peek" => GENERATE_PEEK,
                "Push" => GENERATE_PUSH,
                "Serialize" => GENERATE_AS_SERIALIZE,
                val => panic!(
                    "unknown attribute value: {}, expected Peek, Push, Serialize",
                    val
                ),
            }
        }
        flags
    };

    let ser = if flags & GENERATE_AS_SERIALIZE != 0 {
        quote! {
            impl #ident {
                fn push_value<'a>(&'a self) -> impl duktape::value::PushValue + 'a {
                    use duktape::value::SerdeValue;
                    SerdeValue(self)
                }
            }
        }
    } else {
        quote!()
    };

    let push = if flags & GENERATE_PUSH != 0 {
        quote! {
            impl duktape::PushValue for #ident {
                fn push_to(&self, ctx: &mut duktape::Context) -> i32 {
                    use ::serde::Serialize;
                    let mut serializer = duktape::serialize::DuktapeSerializer::from_ctx(ctx);
                    self.serialize(&mut serializer).unwrap(); // TODO
                    ctx.stack_len() - 1
                }
            }
        }
    } else {
        quote!()
    };
    let peek = if flags & GENERATE_PEEK != 0 {
        quote! {
            impl duktape::PeekValue for #ident {
                fn peek_at(ctx: &mut Context, idx: i32) -> Self {
                    use ::serde::Deserialize;
                    let mut deserializer = duktape::serialize::DuktapeDeserializer::from_ctx(ctx, idx);
                    Self::deserialize(&mut deserializer).unwrap() // TODO
                }
            }
        }
    } else {
        quote!()
    };
    let res = quote!( #peek #push #ser );
    //println!("{}", res);
    res.into()
}

#[proc_macro_attribute]
pub fn duktape(attr: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_attr: Option<Ident> = syn::parse_macro_input!(attr);
    //println!("attrs: {:?}", parsed_attr);
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
                syn::Type::Path(path) => {
                    let ident = path.path.get_ident().unwrap().clone();
                    quote!(#ident)
                }
                syn::Type::Reference(type_ref) => quote!(#type_ref),
                _ => panic!("unsupported return type"),
            };
            (1, Some(ident))
        }
    };
    let mut is_method = false;
    for (i, param) in parsed.sig.inputs.iter().enumerate() {
        match param {
            syn::FnArg::Receiver(receiver) => {
                if receiver.reference.is_none() {
                    panic!("self not supported")
                }
                is_method = true;
                continue;
            }
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
    let raw_args_count = args_count - 1;

    let args_names: Vec<_> = args
        .iter()
        .enumerate()
        .map(|(i, _typ)| Ident::new(&format!("arg_{}", i), Span::call_site()))
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
                use duktape::value::PushValue;
                result.push_to(ctx);
                //ctx.push(&result);
            )
        }
        None => quote!(),
    };

    let bare_func = {
        quote!(
            struct #struct_name;

            impl duktape::Function for #struct_name {
                const ARGS: i32 = #raw_args_count;

                fn ptr(&self) -> unsafe extern "C" fn(*mut ::duktape_sys::duk_context) -> i32 {
                    Self::#fn_name
                }
            }

            impl #struct_name {
                pub unsafe extern "C" fn #fn_name(raw: *mut ::duktape_sys::duk_context) -> i32 {
                    #parsed

                    let ctx = &mut duktape::Context::from_raw(raw);
                    let n = ctx.stack_len();
                    if n < #raw_args_count {
                        return -1;
                    }
                    #(#args_getters)*
                    if #raw_args_count > 0 {
                        ctx.pop_n(#raw_args_count);
                    }
                    let result = #fn_name(ctx, #(#args_names),*);
                    #push_result
                    #return_count
                }
            }
        )
    };
    let res = if !is_method {
        bare_func
    } else {
        let register_fn = Ident::new(
            &format!("register_{}", fn_name.to_string()),
            Span::call_site(),
        );
        let outer_type = parsed_attr.unwrap();
        quote!(

        #parsed

        pub fn #register_fn(ctx: &mut duktape::Context, idx: i32, name: &str) {
            struct #struct_name;

            impl duktape::Function for #struct_name {
                const ARGS: i32 = #raw_args_count;

                fn ptr(&self) -> unsafe extern "C" fn(*mut ::duktape_sys::duk_context) -> i32 {
                    Self::#fn_name
                }
            }

            impl #struct_name {
                pub unsafe extern "C" fn #fn_name(raw: *mut ::duktape_sys::duk_context) -> i32 {
                    let ctx = &mut duktape::Context::from_raw(raw);
                    let n = ctx.stack_len();
                    if n < #raw_args_count {
                        return -1;
                    }
                    #(#args_getters)*
                    ctx.push_this();
                    let this: #outer_type = ctx.peek(-1);
                    if #raw_args_count > 0 {
                        ctx.pop_n(#raw_args_count);
                    }
                    let result = this.#fn_name(#(#args_names),*);
                    #push_result
                    #return_count
                }
            }
            ctx.push_function(#struct_name);
            ctx.put_prop_string(idx, name);
            }
        )
    };
    //println!("{}", res);
    res.into()
}

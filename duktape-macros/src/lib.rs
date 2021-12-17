use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parse;
use syn::{Ident, ItemFn};

struct FieldMeta {
    name: Ident,
    ty: syn::Type,
    is_data: bool,
    serde_attrs: Vec<syn::Attribute>,
}

struct PushField<'a>(&'a FieldMeta);
struct PeekField<'a>(&'a FieldMeta);

impl<'a> quote::ToTokens for PushField<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.0.name;
        let q = if self.0.is_data {
            let wrapper_name = Ident::new(
                &format!("{}Wrapper", self.0.name.to_string()),
                Span::call_site(),
            );
            let serde_attrs = &self.0.serde_attrs;
            let ty = &self.0.ty;

            quote! {
                {
                #[derive(serde::Serialize, serde::Deserialize)]
                struct #wrapper_name(#( #serde_attrs )* #ty);

                impl duktape::PushValue for #wrapper_name {
                    fn push_to(self, ctx: &mut duktape::Context) -> u32 {
                        use ::serde::Serialize;
                        let mut serializer = duktape::serialize::DuktapeSerializer::from_ctx(ctx);
                        self.serialize(&mut serializer).unwrap();
                        ctx.stack_top()
                    }
                }

                #wrapper_name(self.#name).push_to(ctx)
                }
            }
        } else {
            quote! {
                self.#name.push_to(ctx)
            }
        };
        tokens.extend(q);
    }
}

impl<'a> quote::ToTokens for PeekField<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty = &self.0.ty;
        let q = if self.0.is_data {
            let wrapper_name = Ident::new(
                &format!("{}Wrapper", self.0.name.to_string()),
                Span::call_site(),
            );
            let serde_attrs = &self.0.serde_attrs;

            quote! {
                {
                    #[derive(serde::Serialize, serde::Deserialize)]
                    struct #wrapper_name(#( #serde_attrs )* #ty);

                    impl duktape::PeekValue for #wrapper_name {
                        fn peek_at(ctx: &mut duktape::Context, idx: i32) -> Option<Self> {
                            use ::serde::Deserialize;
                            let mut serializer = duktape::serialize::DuktapeDeserializer::from_ctx(ctx, idx);
                            Self::deserialize(&mut serializer).ok()
                        }
                    }

                    <#wrapper_name>::peek_at(ctx, idx).map(|w| w.0)
                }
            }
        } else {
            quote! {
                <#ty>::peek_at(ctx, idx)
            }
        };
        tokens.extend(q);
    }
}

#[proc_macro_derive(Value, attributes(duktape, data))]
pub fn value(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let ident = input.ident.clone();
    let fields = match input.data {
        syn::Data::Struct(data) => data.fields,
        _ => todo!("not (yet) supported"),
    };
    let mut fields_meta = Vec::new();
    match fields {
        syn::Fields::Named(named_fields) => {
            for field in named_fields.named {
                let mut serde_attrs = Vec::new();
                let mut is_data = false;
                for attr in field.attrs {
                    if let Ok(meta) = attr.parse_meta() {
                        if let Some(ident) = meta.path().get_ident() {
                            match ident.to_string().as_str() {
                                "serde" => {
                                    serde_attrs.push(attr);
                                }
                                "data" => {
                                    is_data = true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                fields_meta.push(FieldMeta {
                    name: field.ident.expect("named field").clone(),
                    ty: field.ty.clone(),
                    is_data,
                    serde_attrs,
                });
            }
        }
        _ => todo!("not (yet) supported"),
    }

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

    let field_names: Vec<_> = fields_meta.iter().map(|meta| meta.name.clone()).collect();
    let field_names_str: Vec<_> = fields_meta
        .iter()
        .map(|meta| meta.name.to_string())
        .collect();
    let fields_push: Vec<_> = fields_meta.iter().map(|meta| PushField(meta)).collect();
    let fields_peek: Vec<_> = fields_meta.iter().map(|meta| PeekField(meta)).collect();

    let push = if flags & GENERATE_PUSH != 0 {
        quote! {
            impl duktape::PushValue for #ident {
                fn push_to(self, ctx: &mut duktape::Context) -> u32 {
                    use std::convert::TryInto;
                    let idx = ctx.push_object();
                    #(
                        #fields_push;
                        ctx.put_prop_string(idx.try_into().unwrap(), #field_names_str);
                    )*
                    idx
                }
            }
        }
    } else {
        quote!()
    };
    let peek = if flags & GENERATE_PEEK != 0 {
        quote! {
            impl duktape::PeekValue for #ident {
                fn peek_at(ctx: &mut Context, idx: i32) -> Option<Self> {
                    ctx.get_object(idx);
                    #(
                        if !ctx.get_prop(idx, #field_names_str) {
                            return None;
                        }
                        let #field_names = #fields_peek?;
                        ctx.pop();

                    )*
                    Some(Self {
                        #( # field_names ),*
                    })
                }
            }
        }
    } else {
        quote!()
    };
    let res = quote!( #peek #push #ser );
    //println!(">>> {}", res);
    res.into()
}

struct Args {
    this: Option<Ident>,
    vararg: bool,
}

struct KV {
    name: Ident,
    value: Option<String>,
}

impl Parse for KV {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let name = Ident::parse(input)?;
        let value = if let Ok(_) = syn::token::Eq::parse(input) {
            let lit = syn::Lit::parse(input)?;
            match lit {
                syn::Lit::Str(str) => Some(str.value()),
                _ => panic!(),
            }
        } else {
            None
        };
        Ok(KV { name, value })
    }
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let vars = syn::punctuated::Punctuated::<KV, syn::Token![,]>::parse_terminated(input)?;
        let mut this = None;
        let mut vararg = false;
        for var in vars {
            match var.name.to_string().as_str() {
                "this" => this = Some(Ident::new(&var.value.unwrap(), Span::call_site())),
                "vararg" => {
                    vararg = true;
                }
                attr => {
                    panic!("unknown attribute {}", attr);
                }
            }
        }
        Ok(Args { this, vararg })
    }
}

#[proc_macro_attribute]
pub fn duktape(attr: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_attr = syn::parse_macro_input!(attr as Args);
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
                    quote!(#path)
                }
                syn::Type::Array(arr) => {
                    quote!(#arr)
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
                    args.push(path);
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
            let name_str = name.to_string();
            quote!(
                let #name = ctx.peek::<#typ>(-(1 + #i as i32)).expect(concat!("failed to peek ", #name_str));
            )
        })
        .collect();
    let push_result = match return_type {
        Some(_) => {
            quote!(
                use duktape::value::PushValue;
                result.push_to(ctx);
            )
        }
        None => quote!(),
    };

    let bare_func = {
        let func_args_count = if parsed_attr.vararg {
            -1
        } else {
            raw_args_count
        };
        quote!(
            struct #struct_name;

            impl duktape::Function for #struct_name {
                const ARGS: i32 = #func_args_count;

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
        let method_args_count = if parsed_attr.vararg {
            -1
        } else {
            raw_args_count + 1 /* self */
        };
        let register_fn = Ident::new(
            &format!("register_{}", fn_name.to_string()),
            Span::call_site(),
        );
        let outer_type = parsed_attr.this.unwrap();
        quote!(

        #parsed

        pub fn #register_fn(ctx: &mut duktape::Context, idx: u32, name: &str) {
            use ::std::convert::TryInto;
            struct #struct_name;

            impl duktape::Function for #struct_name {
                const ARGS: i32 = #method_args_count;

                fn ptr(&self) -> unsafe extern "C" fn(*mut ::duktape_sys::duk_context) -> i32 {
                    Self::#fn_name
                }
            }

            impl #struct_name {
                pub unsafe extern "C" fn #fn_name(raw: *mut ::duktape_sys::duk_context) -> i32 {
                    let ctx = &mut duktape::Context::from_raw(raw);
                    let n = ctx.stack_len();
                    if n < #method_args_count {
                        return -1;
                    }
                    #(#args_getters)*
                    ctx.push_this();
                    let this: #outer_type = ctx.peek(-1).expect("failed to peek this");;
                    if #method_args_count > 0 {
                        ctx.pop_n(#method_args_count);
                    }
                    let result = this.#fn_name(#(#args_names),*);
                    #push_result
                    #return_count
                }
            }
            //println!("registering method `{}` of {} args", name, #method_args_count);
            ctx.push_function(#struct_name);
            ctx.put_prop_string(idx.try_into().unwrap(), name);
            }
        )
    };
    //println!("{}", res);
    res.into()
}

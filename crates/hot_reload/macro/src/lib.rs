use quote::{format_ident, quote};
use syn::{parse_macro_input, Ident};

#[proc_macro_attribute]
pub fn hot_reload(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let original_item: proc_macro2::TokenStream = item.clone().into();

    let f = parse_macro_input!(item as syn::ItemFn);
    let name = &f.sig.ident;
    let internal_name = format_ident!("__internal_{}", name);
    let internal_name_as_str = &internal_name.to_string();
    let args = &f.sig.inputs;
    let arg_idents: Vec<Ident> = f
        .sig
        .inputs
        .pairs()
        .map(|p| match p.value() {
            syn::FnArg::Receiver(_) => panic!(),
            syn::FnArg::Typed(t) => match *t.pat.clone() {
                syn::Pat::Ident(i) => i.ident.clone(),
                _ => panic!(),
            },
        })
        .collect();
    let arg_types: Vec<_> = f
        .sig
        .inputs
        .pairs()
        .map(|p| match p.value() {
            syn::FnArg::Receiver(_) => panic!(),
            syn::FnArg::Typed(t) => *t.ty.clone(),
        })
        .collect();
    let body = &f.block;

    let reloaded_f = quote! {
        #[no_mangle]
        pub fn #internal_name (#args) {
            #body
        }
    };

    let loader_f = quote! {
        use ::winny::prelude::*;

        #[cfg(not(target_arch = "wasm32"))]
        pub fn #name (__internal_lib: Res<::winny::hot_reload::LinkedLib>, #args) {
            let f = unsafe {
                __internal_lib
                    .linked_lib
                    .lib
                    .as_ref()
                    .unwrap()
                    .get::<fn(#(#arg_types,)*)>(#internal_name_as_str.as_bytes()).unwrap()
            };
            f(#(#arg_idents,)*);
        }
    };

    quote! {
        #[cfg(target_arch = "wasm32")]
        #original_item

        #[cfg(not(target_arch = "wasm32"))]
        #reloaded_f

        #[cfg(not(target_arch = "wasm32"))]
        #loader_f
    }
    .into()
}

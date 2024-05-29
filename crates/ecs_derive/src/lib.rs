use core::panic;
use proc_macro2::{Ident, Span};
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::{Parse, ParseStream}, parse_macro_input, token::Comma, DeriveInput, Fields, LitInt, LitStr, Result};

enum StorageType {
    Table,
    SparseSet,
}

const TABLE: &str = "Table";
const SPARSE_SET: &str = "SparceSet";

#[proc_macro_derive(Component, attributes(component))]
pub fn component_impl(input: TokenStream) -> TokenStream {
    parse_component(input, quote! { winny::ecs }.into())
}

#[proc_macro_derive(WinnyComponent, attributes(component))]
pub fn winny_component_impl(input: TokenStream) -> TokenStream {
    parse_component(input, quote! { ::ecs }.into())
}

#[proc_macro_derive(InternalComponent, attributes(component))]
pub fn internal_component_impl(input: TokenStream) -> TokenStream {
    parse_component(input, quote! { crate }.into())
}

fn parse_component(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let mut storage = StorageType::Table;

    for meta in input.attrs.iter().filter(|a| a.path().is_ident("component")) {
        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident("storage") {
                storage = match nested.value()?.parse::<LitStr>()?.value() {
                    s if s == TABLE => StorageType::Table,
                    s if s == SPARSE_SET => StorageType::SparseSet,
                    _ => {
                        return Err(nested.error("Invalid storage type"));
                    }
                };              
                Ok(())
            } else {
                panic!("Invalid component attribute. Use \n\"component(storage = SparseSet)\"\nfor sparse set.");
            }
        }).expect("Invalid attribute(s)");
    }

    let storage = match storage {
        StorageType::Table => 
            Ident::new("Table", Span::call_site()),
        StorageType::SparseSet => 
            Ident::new("SparseSet", Span::call_site()),
    };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let derive = quote! {
        impl #impl_generics #path_to_ecs::storage::Storage for #name #ty_generics #where_clause {
            fn storage_type() -> #path_to_ecs::storage::StorageType {
                #path_to_ecs::storage::StorageType::#storage   
            }
        }

        impl #impl_generics #path_to_ecs::storage::Component for #name #ty_generics #where_clause {}
    }.into();

    append_type_getter(input, derive, path_to_ecs)
}

#[proc_macro_derive(Resource)]
pub fn resource_impl(input: TokenStream) -> TokenStream {
    parse_resource(input, quote! { winny::ecs })
}

#[proc_macro_derive(WinnyResource)]
pub fn winny_resource_impl(input: TokenStream) -> TokenStream {
    parse_resource(input, quote! { ::ecs })
}

#[proc_macro_derive(InternalResource)]
pub fn internal_resource_impl(input: TokenStream) -> TokenStream {
    parse_resource(input, quote! { crate })
}

fn parse_resource(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let derive = quote! {
        impl #impl_generics #path_to_ecs::storage::Resource for #name #ty_generics #where_clause {}
    }.into();

    append_type_getter(input, derive, path_to_ecs)
}

#[proc_macro_derive(Event)]
pub fn event_impl(input: TokenStream) -> TokenStream {
    parse_event(input, quote! { winny::ecs })
}

#[proc_macro_derive(WinnyEvent)]
pub fn winny_event_impl(input: TokenStream) -> TokenStream {
    parse_event(input, quote! { ::ecs })
}

#[proc_macro_derive(InternalEvent)]
pub fn internal_event_impl(input: TokenStream) -> TokenStream {
    parse_event(input, quote! { crate })
}

fn parse_event(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let derive = quote! {
        impl #impl_generics #path_to_ecs::Event for #name #ty_generics #where_clause {}
    }.into();

    append_type_getter(input, derive, path_to_ecs)
}

// #[proc_macro_derive(TypeGetter)]
// pub fn type_getter_impl(input: TokenStream) -> TokenStream {
//     parse_type_getter(input, quote! { winny::prelude::ecs }.into())
// }
// 
// #[proc_macro_derive(WinnyTypeGetter)]
// pub fn winny_type_getter_impl(input: TokenStream) -> TokenStream {
//     parse_type_getter(input, quote! { ecs }.into())
// }
// 
// #[proc_macro_derive(InternalTypeGetter)]
// pub fn internal_type_getter_impl(input: TokenStream) -> TokenStream {
//     parse_type_getter(input, quote! { crate }.into())
// }

fn append_type_getter(input: DeriveInput, derive: proc_macro2::TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let data = &input.data;
    let data_decon = match data {
        syn::Data::Enum(data) => {
            let d = &data.variants;
            quote! { #d }.to_string()
        }
        syn::Data::Union(_data) => {
            panic!()
        }
        syn::Data::Struct(data) => {
            let d = &data.fields;
            quote! { #d }.to_string()
        }
    };

    let names = name.to_string() + &data_decon;
    let name_str = name.to_string();

    let mut hasher = DefaultHasher::default();
    hasher.write(names.as_bytes());
    let id = hasher.finish();

    quote! {
        #derive

        impl #impl_generics #path_to_ecs::any::TypeGetter for #name #ty_generics #where_clause {
            fn type_id() -> #path_to_ecs::any::TypeId {
                #path_to_ecs::any::TypeId::new(#id)
            }

            fn type_name() -> #path_to_ecs::any::TypeName {
                #path_to_ecs::any::TypeName::new(#name_str)
            }
        }
    }.into()
}

#[proc_macro_derive(Bundle)]
pub fn bundle_impl(input: TokenStream) -> TokenStream {
    parse_bundle(input, quote! { winny::ecs }.into())
}

#[proc_macro_derive(WinnyBundle)]
pub fn winny_bundle_impl(input: TokenStream) -> TokenStream {
    parse_bundle(input, quote! { ::ecs }.into())
}

#[proc_macro_derive(InternalBundle)]
pub fn internal_bundle_impl(input: TokenStream) -> TokenStream {
    parse_bundle(input, quote! { crate }.into())
}

fn parse_bundle(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let data = &input.data;

    match data {
        syn::Data::Enum(_) => {
            panic!("{} bundle must have fields", name.to_string());
        }
        syn::Data::Union(_) => {
            panic!("{} bundle must have fields", name.to_string());
        }
        syn::Data::Struct(data) => {
            let mut fields = Vec::new();

            match &data.fields {
                Fields::Named(data) => {
                    for field in data.named.iter() {
                        fields.push(field.ident.as_ref().unwrap());
                    }
                }
                _ => panic!("helo"),
            }

            quote! {
                use #path_to_ecs::storage::components::ComponentStorageType;
                impl #path_to_ecs::storage::Bundle for #name {
                    fn push_storage<'w>(self, world: #path_to_ecs::world::UnsafeWorldCell<'w>, table: &mut #path_to_ecs::storage::Table) -> Result<(), #path_to_ecs::storage::IntoStorageError> {
                        (#(self.#fields),*).push_storage(world, table)
                    }

                    fn new_storages<'w>(&self, world: #path_to_ecs::world::UnsafeWorldCell<'w>) -> Vec<(#path_to_ecs::components::ComponentId, #path_to_ecs::storage::DumbVec)> {
                        (#(self.#fields.clone()),*).new_storages(world)
                    }

                    fn type_ids(&self) -> Vec<#path_to_ecs::any::TypeId>  {
                        (#(self.#fields.clone()),*).type_ids()
                    }

                    fn component_ids<'w>(&self, world: #path_to_ecs::world::UnsafeWorldCell<'w>) -> Vec<#path_to_ecs::components::ComponentId>  {
                        (#(self.#fields.clone()),*).component_ids(world)
                    }

                    fn storage_locations(&self) -> Vec<#path_to_ecs::storage::StorageType> {
                        (#(self.#fields.clone()),*).storage_locations()
                    }
                }
            }
            .into()
        }
    }
}

struct AllTuples {
    macro_ident: Ident,
    start: usize,
    end: usize,
    idents: Vec<Ident>,
}

impl Parse for AllTuples {
    fn parse(input: ParseStream) -> Result<Self> {
        let macro_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;
        let start = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let end = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let mut idents = vec![input.parse::<Ident>()?];
        while input.parse::<Comma>().is_ok() {
            idents.push(input.parse::<Ident>()?);
        }

        Ok(AllTuples {
            macro_ident,
            start,
            end,
            idents,
        })
    }
}

#[proc_macro]
pub fn all_tuples(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AllTuples);
    let len = 1 + input.end - input.start;
    let mut ident_tuples = Vec::with_capacity(len);
    for i in 0..=len {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));
        if input.idents.len() < 2 {
            ident_tuples.push(quote! {
                #(#idents)*
            });
        } else {
            ident_tuples.push(quote! {
                (#(#idents),*)
            });
        }
    }

    let macro_ident = &input.macro_ident;
    let invocations = (input.start..=input.end).map(|i| {
        let ident_tuples = &ident_tuples[..i];
        quote! {
            #macro_ident!(#(#ident_tuples),*);
        }
    });
    TokenStream::from(quote! {
        #(
            #invocations
        )*
    })
}

#[proc_macro]
pub fn all_tuples_with_size(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AllTuples);
    let len = 1 + input.end - input.start;
    let mut ident_tuples = Vec::with_capacity(len);
    for i in 0..=len {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));
        if input.idents.len() < 2 {
            ident_tuples.push(quote! {
                #(#idents)*
            });
        } else {
            ident_tuples.push(quote! {
                (#(#idents),*)
            });
        }
    }

    let macro_ident = &input.macro_ident;
    let invocations = (input.start..=input.end).map(|i| {
        let ident_tuples = &ident_tuples[..i];
        quote! {
            #macro_ident!(#i, #(#ident_tuples),*);
        }
    });
    TokenStream::from(quote! {
        #(
            #invocations
        )*
    })
}

use core::panic;
use proc_macro2::{Ident, Span};
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;

use proc_macro::{TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse::{Parse, ParseStream}, parse_macro_input, token::Comma, DeriveInput, Fields, LitInt, LitStr, Result};

enum StorageType {
    Table,
    SparseSet,
}

const TABLE: &str = "Table";
const SPARSE_SET: &str = "SparceSet";

#[proc_macro_derive(Component, attributes(component))]
pub fn component_impl(input: TokenStream) -> TokenStream {
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

    quote! {
        impl winny::prelude::ecs::storage::Storage for #name {
            fn storage_type() -> winny::prelude::ecs::storage::StorageType {
                winny::prelude::ecs::storage::StorageType::#storage   
            }
        }

        impl winny::prelude::ecs::storage::Component for #name {}
    }
    .into()
}

#[proc_macro_derive(WinnyComponent, attributes(component))]
pub fn winny_component_impl(input: TokenStream) -> TokenStream {
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

    quote! {
        impl crate::prelude::ecs::storage::Storage for #name {
            fn storage_type() -> crate::prelude::ecs::storage::StorageType {
                crate::prelude::ecs::storage::StorageType::#storage   
            }
        }

        impl crate::prelude::ecs::storage::Component for #name {}
    }
    .into()
}

#[proc_macro_derive(InternalComponent, attributes(component))]
pub fn internal_component_impl(input: TokenStream) -> TokenStream {
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

    quote! {
        impl ::ecs::storage::Storage for #name {
            fn storage_type() -> ::ecs::storage::StorageType {
                ::ecs::storage::StorageType::#storage   
            }
        }

        impl ::ecs::storage::Component for #name {}
    }
    .into()
}

#[proc_macro_derive(ComponentTest, attributes(component))]
pub fn component_impl_test(input: TokenStream) -> TokenStream {
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

    quote! {
        impl crate::Storage for #name {
            fn storage_type() -> crate::storage::StorageType {
                crate::storage::StorageType::#storage   
            }
        }

        impl crate::storage::Component for #name {}
    }
    .into()
}

#[proc_macro_derive(Resource)]
pub fn resource_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics winny::prelude::ecs::storage::Resource for #name #ty_generics #where_clause {}
    }.into()
}

#[proc_macro_derive(InternalResource)]
pub fn internal_resource_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics crate::storage::Resource for #name #ty_generics #where_clause {}
    }.into()
}

#[proc_macro_derive(WinnyResource)]
pub fn winny_resource_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ::ecs::storage::Resource for #name #ty_generics #where_clause {}
    }.into()
}

#[proc_macro_derive(Event)]
pub fn event_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics Event for #name #ty_generics #where_clause {}
    }.into()
}

#[proc_macro_derive(TypeGetter)]
pub fn type_getter_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
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
            quote! {  #d }.to_string()
        }
    };
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let names = name.to_string() + &data_decon;
    let name_str = name.to_string();

    let mut hasher = DefaultHasher::default();
    hasher.write(names.as_bytes());
    let id = hasher.finish();

    quote! {
        impl #impl_generics ecs::any::TypeGetter for #name #ty_generics #where_clause {
            fn type_id() -> ecs::any::TypeId {
                ecs::any::TypeId::new(#id)
            }

            fn type_name() -> ecs::any::TypeName {
                ecs::any::TypeName::new(#name_str)
            }
        }
    }
    .into()
}

#[proc_macro_derive(WinnyTypeGetter)]
pub fn winny_type_getter_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
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
            quote! {  #d }.to_string()
        }
    };

    let names = name.to_string() + &data_decon;
    let name_str = name.to_string();

    let mut hasher = DefaultHasher::default();
    hasher.write(names.as_bytes());
    let id = hasher.finish();

    quote! {
        use crate::prelude::ecs::any::*;
        impl #impl_generics TypeGetter for #name #ty_generics #where_clause {
            fn type_id() -> crate::prelude::ecs::any::TypeId {
                crate::prelude::ecs::any::TypeId::new(#id)
            }

            fn type_name() -> crate::prelude::ecs::any::TypeName {
                crate::prelude::ecs::any::TypeName::new(#name_str)
            }
        }
    }
    .into()
}

#[proc_macro_derive(InternalTypeGetter)]
pub fn internal_type_getter_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
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
            quote! {  #d }.to_string()
        }
    };

    let names = name.to_string() + &data_decon;
    let name_str = name.to_string();

    let mut hasher = DefaultHasher::default();
    hasher.write(names.as_bytes());
    let id = hasher.finish();

    quote! {
        impl #impl_generics crate::any::TypeGetter for #name #ty_generics #where_clause {
            fn type_id() -> crate::any::TypeId {
                crate::any::TypeId::new(#id)
            }

            fn type_name() -> crate::any::TypeName {
                crate::any::TypeName::new(#name_str)
            }
        }
    }
    .into()
}

#[proc_macro_derive(Bundle)]
pub fn bundle_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let data = &input.data;

    match data {
        syn::Data::Enum(_data) => {
            panic!("bunlde must have fields: {}", name.to_string());
        }
        syn::Data::Union(_data) => {
            panic!("bunlde must have fields: {}", name.to_string())
        }
        syn::Data::Struct(data) => {
            let mut push = quote!();
            let mut component_ids = quote!();
            let mut component_storage = quote!();
            let mut desc = quote!();

            match &data.fields {
                Fields::Named(data) => {
                    for field in data.named.iter() {
                        let name = field.ident.as_ref().unwrap();

                        push.extend(quote!(
                            table.push_column(self.#name)?;
                        ));

                        let ty = &field.ty.to_token_stream();

                        desc.extend(quote!(
                            winny::ecs::storage::ComponentDescription {
                                type_id: winny::ecs::any::TypeId::of::<#ty>(),
                                layout: std::alloc::Layout::new::<#ty>(),
                                drop: winny::ecs::storage::new_dumb_drop::<#ty>()
                            },
                        ));

                        component_ids.extend(quote!(
                            self.#name.type_id(),
                        ));

                        component_storage.extend(quote!(
                            self.#name.storage_type(),
                        ));
                    }
                }
                _ => panic!("helo"),
            }

            quote! {
                impl winny::ecs::storage::Bundle for #name {
                    fn push_storage(self, table: &mut winny::ecs::storage::Table) -> Result<(), ()> {
                        #push

                        Ok(())
                    }

                    fn descriptions(&self) -> Vec<winny::ecs::storage::ComponentDescription> {
                        vec![
                            #desc
                        ]
                    }

                    fn ids(&self) -> Vec<winny::ecs::any::TypeId>  {
                        vec![
                            #component_ids
                        ]
                    }

                    fn storage_locations(&self) -> Vec<winny::ecs::storage::StorageType> {
                        vec![
                            #component_storage
                        ]
                    }
                }
            }
            .into()
        }
    }
}

#[proc_macro_derive(InternalBundle)]
pub fn internal_bundle_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let data = &input.data;

    match data {
        syn::Data::Enum(_data) => {
            panic!("bunlde must have fields: {}", name.to_string());
        }
        syn::Data::Union(_data) => {
            panic!("bunlde must have fields: {}", name.to_string())
        }
        syn::Data::Struct(data) => {
            let mut push = quote!();
            let mut component_ids = quote!();
            let mut component_storage = quote!();
            let mut desc = quote!();

            match &data.fields {
                Fields::Named(data) => {
                    for field in data.named.iter() {
                        let name = field.ident.as_ref().unwrap();

                        push.extend(quote!(
                            table.push_column(self.#name)?;
                        ));

                        let ty = &field.ty.to_token_stream();

                        desc.extend(quote!(
                            ::ecs::storage::ComponentDescription {
                                type_id: #ty::type_id(),
                                layout: std::alloc::Layout::new::<#ty>(),
                                drop: ::ecs::storage::new_dumb_drop::<#ty>()
                            },
                        ));

                        component_ids.extend(quote!(
                            #ty::type_id(),
                        ));

                        component_storage.extend(quote!(
                            self.#name.storage_type(),
                        ));
                    }
                }
                _ => panic!("helo"),
            }

            quote! {
                use ::ecs::storage::components::ComponentStorageType;
                impl ::ecs::storage::Bundle for #name {
                    fn push_storage(self, table: &mut ::ecs::storage::Table) -> Result<(), ::ecs::storage::IntoStorageError> {
                        #push

                        Ok(())
                    }

                    fn descriptions(&self) -> Vec<::ecs::storage::ComponentDescription> {
                        vec![
                            #desc
                        ]
                    }

                    fn ids(&self) -> Vec<::ecs::any::TypeId>  {
                        vec![
                            #component_ids
                        ]
                    }

                    fn storage_locations(&self) -> Vec<::ecs::storage::StorageType> {
                        vec![
                            #component_storage
                        ]
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

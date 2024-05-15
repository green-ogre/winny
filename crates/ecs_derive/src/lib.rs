use core::panic;
use proc_macro2::{Ident, Span};
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;

use proc_macro::{TokenStream};
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
        impl winny::ecs::storage::Storage for #name {
            fn storage_type() -> winny::ecs::storage::StorageType {
                winny::ecs::storage::StorageType::#storage   
            }
        }

        impl winny::ecs::storage::Component for #name {}
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
        impl crate::prelude::ecs::storage::Storage for #name {
            fn storage_type() -> crate::prelude::ecs::storage::StorageType {
                crate::prelude::ecs::storage::StorageType::#storage   
            }
        }

        impl crate::prelude::ecs::storage::Component for #name {}
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

    let mut gen = proc_macro2::TokenStream::new();

    quote! {
        impl ecs::storage::Resource for #name {}
    }.into()
}

#[proc_macro_derive(InternalResource)]
pub fn internal_resource_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut gen = proc_macro2::TokenStream::new();

    let impl_block = quote! {
        impl #impl_generics crate::storage::Resource for #name #ty_generics #where_clause {}
    };

    gen.extend(impl_block);

    gen.into()
}

#[proc_macro_derive(Event)]
pub fn event_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let mut gen = proc_macro2::TokenStream::new();

    let impl_block = quote! {
        impl Event for #name {}
    };

    gen.extend(impl_block);

    gen.into()
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
        syn::Data::Union(data) => {
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
        impl ecs::any::TypeGetter for #name {
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
            fn type_id() -> TypeId {
                TypeId::new(#id)
            }

            fn type_name() -> TypeName {
                TypeName::new(#name_str)
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
        syn::Data::Enum(data) => {
            panic!("bunlde must have fields: {}", name.to_string());
        }
        syn::Data::Union(data) => {
            panic!("bunlde must have fields: {}", name.to_string())
        }
        syn::Data::Struct(data) => {
            let mut push = quote!();
            let mut into_storage = quote!();
            let mut component_ids = quote!();
            let mut component_storage = quote!();

            match &data.fields {
                Fields::Named(data) => {
                    for field in data.named.iter() {
                        let name = field.ident.as_ref().unwrap();

                        push.extend(quote!(
                        {
                            let id = self.#name.type_id();
                            let index = self
                            .ids()
                            .iter()
                            .enumerate()
                            .find(|(_, other)| **other == id)
                            .ok_or(winny::ecs::storage::IntoStorageError::MismatchedShape)?.0;
                            table.storage[index].try_push(&self.#name as &dyn Any).expect("sad");
                        }
                        ));

                        into_storage
                            .extend(quote!(Box::new(std::cell::RefCell::new(vec![self.#name])),));

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
                    fn push_storage(
                        self,
                        table: &mut winny::ecs::storage::Table
                        ) -> Result<(), winny::ecs::storage::IntoStorageError> {
                        #push
                        Ok(())
                    }

                    fn push_storage_box(
                        self: Box<Self>,
                        table: &mut winny::ecs::storage::Table
                        ) -> Result<(), winny::ecs::storage::IntoStorageError> {
                        self.push_storage(table)
                    }

                    fn into_storage(self) -> Vec<Box<dyn winny::ecs::storage::ComponentVec>> {
                        vec![
                            #into_storage
                        ]
                    }

                    fn into_storage_box(self: Box<Self>) -> Vec<Box<dyn winny::ecs::storage::ComponentVec>> {
                        self.into_storage_box()
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


#[proc_macro_derive(BundleTest)]
pub fn bundle_impl_test(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let data = &input.data;

    match data {
        syn::Data::Enum(data) => {
            panic!("bunlde must have fields: {}", name.to_string());
        }
        syn::Data::Union(data) => {
            panic!("bunlde must have fields: {}", name.to_string())
        }
        syn::Data::Struct(data) => {
            let mut push = quote!();
            let mut into_storage = quote!();
            let mut component_ids = quote!();
            let mut component_storage = quote!();

            match &data.fields {
                Fields::Named(data) => {
                    for field in data.named.iter() {
                        let name = field.ident.as_ref().unwrap();

                        push.extend(quote!(
                        {
                            let id = self.#name.type_id();
                            let index = self
                            .ids()
                            .iter()
                            .enumerate()
                            .find(|(_, other)| **other == id)
                            .ok_or(crate::storage::IntoStorageError::MismatchedShape)?.0;
                            table.storage[index].try_push(&self.#name as &dyn Any).expect("sad");
                        }
                        ));

                        into_storage
                            .extend(quote!(Box::new(std::cell::RefCell::new(vec![self.#name])),));

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
                impl crate::storage::Bundle for #name {
                    fn push_storage(
                        self,
                        table: &mut crate::storage::Table
                        ) -> Result<(), crate::storage::IntoStorageError> {
                        #push
                        Ok(())
                    }

                    fn push_storage_box(
                        self: Box<Self>,
                        table: &mut crate::storage::Table
                        ) -> Result<(), crate::storage::IntoStorageError> {
                        self.push_storage(table)
                    }

                    fn into_storage(self) -> Vec<Box<dyn crate::storage::ComponentVec>> {
                        vec![
                            #into_storage
                        ]
                    }

                    fn into_storage_box(self: Box<Self>) -> Vec<Box<dyn crate::storage::ComponentVec>> {
                        self.into_storage()
                    }

                    fn ids(&self) -> Vec<crate::any::TypeId>  {
                        vec![
                            #component_ids
                        ]
                    }

                    fn storage_locations(&self) -> Vec<crate::storage::StorageType> {
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

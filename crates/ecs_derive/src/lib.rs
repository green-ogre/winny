use core::panic;
use proc_macro2::{Ident, Span};
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;

use proc_macro::{TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields, LitStr};

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
        impl winny::ecs::Storage for #name {
            fn storage_type() -> winny::ecs::storage::Storage {
                #storage   
            }
        }
    }
    .into()
}

#[proc_macro_derive(Resource)]
pub fn resource_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let mut gen = proc_macro2::TokenStream::new();

    let impl_block = quote! {
        impl Resource for #name {}
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
        impl winny::ecs::any::TypeGetter for #name {
            fn type_id() -> winny::ecs::any::TypeId {
                winny::ecs::any::TypeId::new(#id)
            }

            fn type_name() -> winny::ecs::any::TypeName {
                winny::ecs::any::TypeName::new(#name_str)
            }
        }
    }
    .into()
}

#[proc_macro_derive(TestTypeGetter)]
pub fn test_type_getter_impl(input: TokenStream) -> TokenStream {
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
        impl TypeGetter for #name {
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

            match &data.fields {
                Fields::Named(data) => {
                    for field in data.named.iter() {
                        let name = field.ident.as_ref().unwrap();

                        push.extend(quote!(
                        {
                            let id = self.#name.type_id();
                            let index = component_ids
                            .ids
                            .iter()
                            .enumerate()
                            .find(|(_, other)| **other == id)
                            .ok_or(winny::ecs::storage::IntoStorageError::MismatchedShape)?.0;
                            storage[index].try_push(&self.#name as &dyn Any).expect("sad");
                        }
                        ));

                        into_storage
                            .extend(quote!(Box::new(std::cell::RefCell::new(vec![self.#name])),));

                        component_ids.extend(quote!(
                            self.#name.type_id(),
                        ));
                    }
                }
                _ => panic!("helo"),
            }

            quote! {
                impl winny::ecs::storage::IntoComponentStorage for #name {
                    fn push(
                        self,
                        storage: &mut Vec<Box<dyn winny::ecs::storage::ComponentVec>>,
                        component_ids: &winny::ecs::storage::ComponentSet,
                        ) -> Result<(), winny::ecs::storage::IntoStorageError> {
                        #push
                        Ok(())
                    }

                    fn into_storage(self) -> Vec<Box<dyn winny::ecs::storage::ComponentVec>> {
                         vec![
                             #into_storage
                         ]
                    }
                }

                impl winny::ecs::storage::GetComponentIds for #name {
                    fn ids(&self) -> Vec<TypeId>  {
                        vec![
                             #component_ids
                        ]
                    }
                }
            }
            .into()
        }
    }
}

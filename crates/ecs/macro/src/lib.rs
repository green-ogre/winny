use core::panic;
use proc_macro2::Ident;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Comma,
    DeriveInput, Fields, LitInt, Result,
};

// enum StorageType {
//     Table,
//     SparseSet,
// }

// const TABLE: &str = "Table";
// const SPARSE_SET: &str = "SparceSet";

#[proc_macro_derive(Component, attributes(component))]
pub fn component_impl(input: TokenStream) -> TokenStream {
    parse_component(input, quote! { winny::ecs })
}

#[proc_macro_derive(WinnyComponent, attributes(component))]
pub fn winny_component_impl(input: TokenStream) -> TokenStream {
    parse_component(input, quote! { ::ecs })
}

#[proc_macro_derive(InternalComponent, attributes(component))]
pub fn internal_component_impl(input: TokenStream) -> TokenStream {
    parse_component(input, quote! { crate })
}

fn parse_component(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // let mut storage = StorageType::Table;

    // for meta in input.attrs.iter().filter(|a| a.path().is_ident("component")) {
    //     meta.parse_nested_meta(|nested| {
    //         if nested.path.is_ident("storage") {
    //             storage = match nested.value()?.parse::<LitStr>()?.value() {
    //                 s if s == TABLE => StorageType::Table,
    //                 s if s == SPARSE_SET => StorageType::SparseSet,
    //                 _ => {
    //                     return Err(nested.error("Invalid storage type"));
    //                 }
    //             };
    //             Ok(())
    //         } else {
    //             panic!("Invalid component attribute. Use \n\"component(storage = SparseSet)\"\nfor sparse set.");
    //         }
    //     }).expect("Invalid attribute(s)");
    // }

    // let storage = match storage {
    //     StorageType::Table =>
    //         Ident::new("Table", Span::call_site()),
    //     StorageType::SparseSet =>
    //         Ident::new("SparseSet", Span::call_site()),
    // };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #path_to_ecs::storage::Component for #name #ty_generics #where_clause {}
    }
    .into()
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

    quote! {
        impl #impl_generics #path_to_ecs::storage::Resource for #name #ty_generics #where_clause {}
    }
    .into()
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

    quote! {
        impl #impl_generics #path_to_ecs::Event for #name #ty_generics #where_clause {}
    }
    .into()
}

#[proc_macro_derive(Bundle)]
pub fn bundle_impl(input: TokenStream) -> TokenStream {
    parse_bundle(input, quote! { winny::ecs })
}

#[proc_macro_derive(WinnyBundle)]
pub fn winny_bundle_impl(input: TokenStream) -> TokenStream {
    parse_bundle(input, quote! { ::ecs })
}

#[proc_macro_derive(InternalBundle)]
pub fn internal_bundle_impl(input: TokenStream) -> TokenStream {
    parse_bundle(input, quote! { crate })
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
            let mut tys = Vec::new();

            match &data.fields {
                Fields::Named(data) => {
                    for field in data.named.iter() {
                        fields.push(field.ident.as_ref().unwrap());
                        tys.push(&field.ty);
                    }
                }
                _ => panic!("helo"),
            }

            let generics = &input.generics;
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            quote! {
                impl #impl_generics #path_to_ecs::storage::Bundle for #name #ty_generics #where_clause {
                    fn push_storage(self, world: #path_to_ecs::world::UnsafeWorldCell<'_>, table_id: #path_to_ecs::storage::TableId) {
                        (#(self.#fields),*).push_storage(world, table_id)
                    }

                    fn new_table(self, world: &mut #path_to_ecs::world::World) -> #path_to_ecs::storage::Table {
                        let mut table = #path_to_ecs::storage::Table::new();
                        unsafe {
                            #(
                            let component_id = world.get_component_id(&std::any::TypeId::of::<#tys>());
                            let mut column = #path_to_ecs::any_vec::AnyVec::new::<#tys>();
                            {
                                let mut vec = column.downcast_mut_unchecked::<#tys>();
                                vec.push(self.#fields);
                            }

                            table.insert_column(column, component_id);
                            )*
                        }

                        table
                    }

                    fn type_ids(&self) -> Vec<std::any::TypeId> {
                        vec![#(std::any::TypeId::of::<#tys>()),*]
                    }

                    fn register_components(&self, world: &mut #path_to_ecs::world::World) {
                        #(world.register_component::<#tys>();)*
                    }
                }
            }.into()
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

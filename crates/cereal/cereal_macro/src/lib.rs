use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Serialize)]
pub fn impl_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let serialize_fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|field| {
                    let field_name = &field.ident;
                    quote! {
                        self.#field_name.serialize(serializer);
                    }
                })
                .collect::<Vec<_>>(),
            Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _field)| {
                    let field_name = syn::Index::from(i);
                    quote! {
                        self.#field_name.serialize(serializer);
                    }
                })
                .collect::<Vec<_>>(),
            Fields::Unit => Vec::new(),
        },
        Data::Enum(data) => data
            .variants
            .iter()
            .map(|field| {
                // let field_name = &field.ident;
                unimplemented!();
                quote! {}
            })
            .collect::<Vec<_>>(),
        _ => panic!("Components may only be structs of enums"),
    };

    quote! {
        impl #impl_generics ::cereal::Serialize for #name #ty_generics #where_clause {
            fn serialize(&self, serializer: &mut Serializer<'_>) {
                #(#serialize_fields)*
            }
        }
    }
    .into()
}

#[proc_macro_derive(Deserialize)]
pub fn impl_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let deserialize = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let mut fields = fields
                    .named
                    .iter()
                    .map(|field| {
                        let field_name = &field.ident;
                        let ty = &field.ty;
                        quote! {
                            #field_name: <#ty>::deserialize(deserializer),
                        }
                    })
                    .collect::<Vec<_>>();
                fields.reverse();

                quote! {
                    #name {
                        #(#fields)*
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let mut fields = fields
                    .unnamed
                    .iter()
                    .map(|field| {
                        let ty = &field.ty;
                        quote! {
                            <#ty>::deserialize(deserializer),
                        }
                    })
                    .collect::<Vec<_>>();
                fields.reverse();

                quote! {
                    #name(#(#fields)*)
                }
            }
            Fields::Unit => quote! { #name },
        },
        Data::Enum(data) => {
            unimplemented!();
            quote! {}
        }
        _ => panic!("Components may only be structs of enums"),
    };

    quote! {
        impl #impl_generics ::cereal::Deserialize for #name #ty_generics #where_clause {
            fn deserialize(deserializer: &mut Deserializer<'_>) -> Self {
                #deserialize
            }
        }
    }
    .into()
}

// impl Serialize for SomeData {
//     fn serialize(&self, serializer: &mut Serializer<'_>) {
//         self.x.serialize(serializer);
//         self.y.serialize(serializer);
//     }
// }

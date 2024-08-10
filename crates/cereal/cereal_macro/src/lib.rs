use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_attribute]
pub fn skip(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_derive(Serialize, attributes(skip))]
pub fn serialize(input: TokenStream) -> TokenStream {
    parse_s(input, quote! { winny::cereal })
}

#[proc_macro_derive(WinnySerialize, attributes(skip))]
pub fn winny_serialize(input: TokenStream) -> TokenStream {
    parse_s(input, quote! { ::cereal })
}

fn parse_s(input: TokenStream, path_to_cereal: proc_macro2::TokenStream) -> TokenStream {
    let hash_str = input.to_string();
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let version_hash = fxhash::hash32(&hash_str);

    let serialize_fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|field| {
                    let field_name = &field.ident;
                    let attrs = &field.attrs;
                    let skip = attrs.iter().any(|attr| attr.path().is_ident("skip"));

                    if !skip {
                        quote! {
                            self.#field_name.serialize(serializer);
                        }
                    } else {
                        quote! {}
                    }
                })
                .collect::<Vec<_>>(),
            Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let field_name = syn::Index::from(i);
                    let attrs = &field.attrs;
                    let skip = attrs.iter().any(|attr| attr.path().is_ident("skip"));

                    if !skip {
                        quote! {
                            self.#field_name.serialize(serializer);
                        }
                    } else {
                        quote! {}
                    }
                })
                .collect::<Vec<_>>(),
            Fields::Unit => Vec::new(),
        },
        Data::Enum(data) => data
            .variants
            .iter()
            .map(|_field| {
                panic!("enums are not supported");
            })
            .collect::<Vec<_>>(),
        _ => panic!("Components may only be structs of enums"),
    };

    quote! {
        impl #impl_generics #path_to_cereal::Serialize for #name #ty_generics #where_clause {
            fn serialize(&self, serializer: &mut #path_to_cereal::Serializer<'_>) {
                #(#serialize_fields)*
                #version_hash.serialize(serializer);
            }
        }
    }
    .into()
}

#[proc_macro_derive(Deserialize)]
pub fn deserialize(input: TokenStream) -> TokenStream {
    parse_d(input, quote! { winny::cereal })
}

#[proc_macro_derive(WinnyDeserialize)]
pub fn winny_deserialize(input: TokenStream) -> TokenStream {
    parse_d(input, quote! { ::cereal })
}

fn parse_d(input: TokenStream, path_to_cereal: proc_macro2::TokenStream) -> TokenStream {
    let hash_str = input.to_string();
    let version_hash = fxhash::hash32(&hash_str);

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
                        let attrs = &field.attrs;
                        let skip = attrs.iter().any(|attr| attr.path().is_ident("skip"));

                        if !skip {
                            quote! {
                                #field_name: <#ty>::deserialize(deserializer).unwrap(),
                            }
                        } else {
                            quote! {}
                        }
                    })
                    .collect::<Vec<_>>();
                fields.reverse();

                quote! {
                    #name {
                        #(#fields)*
                        ..Default::default()
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let mut fields = fields
                    .unnamed
                    .iter()
                    .map(|field| {
                        let ty = &field.ty;
                        let attrs = &field.attrs;
                        let skip = attrs.iter().any(|attr| attr.path().is_ident("skip"));

                        if !skip {
                            quote! {
                                <#ty>::deserialize(deserializer).unwrap(),
                            }
                        } else {
                            quote! {}
                        }
                    })
                    .collect::<Vec<_>>();
                fields.reverse();

                quote! {
                    #name(#(#fields)*, ..Default::default())
                }
            }
            Fields::Unit => quote! { #name },
        },
        Data::Enum(_) => {
            panic!("enums are not supported");
        }
        _ => panic!("Components may only be structs of enums"),
    };

    quote! {
        impl #impl_generics #path_to_cereal::Deserialize for #name #ty_generics #where_clause {
            fn deserialize(deserializer: &mut #path_to_cereal::Deserializer<'_>) -> Option<Self> {
                let hash = u32::deserialize(deserializer).unwrap();
                if hash == #version_hash {
                    Some(#deserialize)
                } else {
                    None
                }
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

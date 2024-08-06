use core::panic;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote, ToTokens};
use std::hash::{DefaultHasher, Hash, Hasher};
#[cfg(feature = "widgets")]
use syn::Data;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Comma,
    DeriveInput, Fields, LitInt, Result,
};

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

#[cfg(not(feature = "widgets"))]
fn parse_component(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #path_to_ecs::storage::Component for #name #ty_generics #where_clause {}
    }
    .into()
}

#[cfg(feature = "widgets")]
fn parse_component(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_as_str = &input.ident.to_string();

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let component_object = format_ident!("{}ComponentEguiObject", name);

    let display_widgets = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|field| {
                    let field_name = &field.ident;

                    quote! {
                        #path_to_ecs::egui_widget::Widget::display(&mut component_mut.#field_name, ui);
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
                        #path_to_ecs::egui_widget::Widget::display(&mut component_mut.#field_name, ui);
                    }
                })
                .collect::<Vec<_>>(),
            Fields::Unit => Vec::new(),
        },
        Data::Enum(data) => data
            .variants
            .iter()
            .map(|field| {
                let field_name = &field.ident;

                quote! {}
            })
            .collect::<Vec<_>>(),
        _ => panic!("Components may only be structs of enums"),
    };

    let widgets = if display_widgets.is_empty() {
        quote! {
            ui.label(#name_as_str);
        }
    } else {
        quote! {
            #path_to_ecs::prelude::egui::CollapsingHeader::new(#name_as_str)
                .show(ui, |ui| {
                    #(#display_widgets)*
                });
        }
    };

    let display = if generics.params.is_empty() {
        quote! {
            let component_mut = unsafe { component.cast::<#name>().as_mut() };
            #widgets
        }
    } else {
        quote! {}
    };

    quote! {
        impl #impl_generics #path_to_ecs::storage::Component for #name #ty_generics #where_clause {
            type Dispatch = #component_object;
        }

        #[derive(Default)]
        pub struct #component_object;

        impl #path_to_ecs::egui_widget::ComponentEgui for #component_object {
            fn display_component(&self, component: std::ptr::NonNull<u8>, ui: &mut #path_to_ecs::prelude::egui::Ui) {
                #display
            }
        }
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
        syn::Data::Enum(_) | syn::Data::Union(_) => {
            panic!("{} bundle must have fields", name.to_string());
        }
        syn::Data::Struct(data) => {
            let mut fields = Vec::new();
            let mut component_metas = Vec::new();
            let tys = &data.fields.iter().map(|f| &f.ty).collect::<Vec<_>>();
            match &data.fields {
                Fields::Named(data) => {
                    for field in data.named.iter() {
                        let field_name = field.ident.as_ref().unwrap();
                        fields.push(field_name);
                        let ty = &field.ty;

                        component_metas.push(quote! {
                            <#ty as #path_to_ecs::storage::Bundle>::component_meta(components, ids);
                        });
                    }
                }
                _ => panic!("Invalid Bundle"),
            }
            let generics = &input.generics;
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            quote! {
                impl #impl_generics #path_to_ecs::storage::Bundle for #name #ty_generics #where_clause {
                    fn component_meta<F: FnMut(&#path_to_ecs::components::ComponentMeta)>(components: &mut #path_to_ecs::components::Components, ids: &mut F) {
                        #(#component_metas)*
                    }
                    fn insert_components<F: FnMut(#path_to_ecs::storage::OwnedPtr)>(self, f: &mut F) {
                        #(
                            <#tys as #path_to_ecs::storage::Bundle>::insert_components(self.#fields, f);
                        )*
                    }
                }
            }.into()
        }
    }
}

#[proc_macro_derive(ScheduleLabel)]
pub fn schedule_label_impl(input: TokenStream) -> TokenStream {
    parse_schedule_label(input, quote! { winny::ecs })
}

#[proc_macro_derive(WinnyScheduleLabel)]
pub fn winny_schedule_label_impl(input: TokenStream) -> TokenStream {
    parse_schedule_label(input, quote! { ::ecs })
}

#[proc_macro_derive(InternalScheduleLabel)]
pub fn internal_schedule_label_impl(input: TokenStream) -> TokenStream {
    parse_schedule_label(input, quote! { crate })
}

fn parse_schedule_label(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let data = &input.data;

    match data {
        syn::Data::Enum(data) => {
            let variants = &data
                .variants
                .iter()
                .map(|v| format!("{}{}", name.to_string(), v.ident.to_string()))
                .collect::<Vec<_>>();
            let generics = &input.generics;
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            let mut variant_hashes = Vec::new();
            for variant in variants.iter() {
                let mut s = DefaultHasher::new();
                variant.hash(&mut s);
                variant_hashes.push(s.finish().to_token_stream());
            }

            let variants = &data.variants.iter().map(|v| &v.ident).collect::<Vec<_>>();

            quote! {
                impl #impl_generics #path_to_ecs::schedule::ScheduleLabel for #name #ty_generics #where_clause {}

                impl #impl_generics #path_to_ecs::sets::LabelId for #name #ty_generics #where_clause {
                    fn id(&self) -> usize {
                        match self {
                            #(Self::#variants => #variant_hashes as usize),*
                        }
                    }
                }
            }.into()
        }
        syn::Data::Union(_) => {
            panic!("ScheduleLabel must be an Enum: {}", name.to_string());
        }
        syn::Data::Struct(_) => {
            panic!("ScheduleLabel must be an Enum: {}", name.to_string());
        }
    }
}

#[proc_macro_derive(Widget)]
pub fn widget_impl(input: TokenStream) -> TokenStream {
    parse_widget(input, quote! { winny::ecs })
}

#[proc_macro_derive(WinnyWidget)]
pub fn winny_widget_impl(input: TokenStream) -> TokenStream {
    parse_widget(input, quote! { ::ecs })
}

#[proc_macro_derive(InternalWidget)]
pub fn internal_widget_impl(input: TokenStream) -> TokenStream {
    parse_widget(input, quote! { crate })
}

#[cfg(not(feature = "widgets"))]
fn parse_widget(_input: TokenStream, _path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    quote! {}.into()
}

#[cfg(feature = "widgets")]
fn parse_widget(input: TokenStream, path_to_ecs: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_as_str = &input.ident.to_string();

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let display_widgets = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|field| {
                    let field_name = &field.ident;

                    quote! {
                        #path_to_ecs::egui_widget::Widget::display(&mut self.#field_name, ui);
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
                        #path_to_ecs::egui_widget::Widget::display(&mut self.#field_name, ui);
                    }
                })
                .collect::<Vec<_>>(),
            Fields::Unit => Vec::new(),
        },
        Data::Enum(data) => data
            .variants
            .iter()
            .map(|field| {
                let _field_name = &field.ident;

                quote! {}
            })
            .collect::<Vec<_>>(),
        _ => panic!("Components may only be structs of enums"),
    };

    let widgets = if display_widgets.is_empty() {
        quote! {
            ui.label(#name_as_str);
        }
    } else {
        quote! {
            #path_to_ecs::prelude::egui::CollapsingHeader::new(#name_as_str)
                .show(ui, |ui| {
                    #(#display_widgets)*
                });
        }
    };

    quote! {
        impl #impl_generics #path_to_ecs::egui_widget::Widget for #name #ty_generics #where_clause {
            fn display(&mut self, ui: &mut #path_to_ecs::prelude::egui::Ui) {
                #widgets
            }
        }
    }
    .into()
}

#[cfg(feature = "widgets")]
macro_rules! impl_label_widget {
    ($t:ty, $l:expr) => {
        impl Widget for $t {
            fn display(&mut self, ui: &mut egui::Ui) {
                ui.label($l);
            }
        }
    };
}

struct LabelWidget {
    type_name: Ident,
}

impl Parse for LabelWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let type_name = input.parse::<Ident>()?;

        Ok(LabelWidget { type_name })
    }
}

#[proc_macro]
pub fn impl_label_widget(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LabelWidget);
    let ty = &input.type_name;
    let ty_name = &input.type_name.to_token_stream().to_string();

    TokenStream::from(quote! {
        impl ecs::egui_widget::Widget for #ty {
            fn display(&mut self, ui: &mut ecs::prelude::egui::Ui) {
                ui.label(#ty_name);
            }
        }
    })
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

#[proc_macro_derive(EnumIter)]
pub fn enum_iter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let data = &input.data;

    let variants = match data {
        syn::Data::Enum(data) => &data.variants.iter().collect::<Vec<_>>(),
        syn::Data::Union(_) => {
            panic!("enum_iter not supported on union");
        }
        syn::Data::Struct(_) => {
            panic!("enum_iter not supported on struct");
        }
    };

    quote! {
        impl #name {
            pub fn VALUES() -> impl Iterator<Item = Self> {
                [
                    #(Self::#variants,)*
                ].into_iter()
            }
        }
    }
    .into()
}

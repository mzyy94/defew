#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, MacroDelimiter, MetaList};

#[proc_macro_derive(Defew, attributes(new))]
pub fn defew(input: TokenStream) -> TokenStream {
    let input = &parse_macro_input!(input as DeriveInput);
    let DataStruct { fields, .. } = match &input.data {
        Data::Struct(v) => v,
        _ => panic!("Defew only supports structs"),
    };

    let mut default_values = Vec::new();
    for field in fields {
        if field.attrs.len() > 1 {
            return syn::Error::new_spanned(&field.attrs.last(), "Defew accepts one attribute")
                .to_compile_error()
                .into();
        }
        if let Some(attr) = field.attrs.first() {
            let ident = field.ident.as_ref().unwrap();
            let MetaList {
                tokens,
                delimiter: MacroDelimiter::Paren(_),
                ..
            } = attr.meta.require_list().unwrap()
            else {
                return syn::Error::new_spanned(&attr, "Defew supports #[new(value)] syntax")
                    .to_compile_error()
                    .into();
            };

            default_values.push(quote! {
                #ident: #tokens,
            });
        }
    }

    let struct_name = &input.ident;
    let (impl_generics, _, where_clause) = &input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics #struct_name #where_clause {
            pub fn new() -> Self {
                Self {
                    #(#default_values)*
                    ..Default::default()
                }
            }
        }
    };

    TokenStream::from(expanded)
}

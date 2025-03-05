#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::{format_ident, quote, TokenStreamExt};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, MacroDelimiter, MetaList};

#[proc_macro_derive(Defew, attributes(new))]
pub fn defew(input: TokenStream) -> TokenStream {
    let input = &parse_macro_input!(input as DeriveInput);
    let Data::Struct(DataStruct { fields, .. }) = &input.data else {
        panic!("Defew only supports structs")
    };

    let mut default_values = Vec::new();
    let mut params = Vec::new();
    for (i, field) in fields.into_iter().enumerate() {
        if field.attrs.len() > 1 {
            return syn::Error::new_spanned(field.attrs.last(), "Defew accepts one attribute")
                .to_compile_error()
                .into();
        }
        let ident = field.ident.as_ref();
        let punct = ident.map(|_| quote!(:));
        let Some(attr) = field.attrs.first() else {
            default_values.push(quote! {
                #ident #punct Default::default(),
            });
            continue;
        };

        let MetaList {
            tokens,
            delimiter: MacroDelimiter::Paren(_),
            ..
        } = attr.meta.require_list().unwrap()
        else {
            return syn::Error::new_spanned(attr, "Defew supports #[new(value)] syntax")
                .to_compile_error()
                .into();
        };

        if syn::parse2(tokens.clone()).map_or(false, |ident: syn::Ident| ident == "param") {
            let param = format_ident!("param{i}");
            let ident = ident.map_or_else(|| quote!(#param), |ident| quote!(#ident));
            let ty = &field.ty;
            params.push(quote! { #ident: #ty, });
            default_values.push(quote! {
                #ident,
            });
            continue;
        }

        default_values.push(quote! {
            #ident #punct #tokens,
        });
    }

    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();

    let mut values = quote! { Self };

    match fields {
        Fields::Named(f) => f
            .brace_token
            .surround(&mut values, |v| v.append_all(default_values)),
        Fields::Unnamed(f) => f
            .paren_token
            .surround(&mut values, |v| v.append_all(default_values)),
        Fields::Unit => panic!("Defew does not support unit structs"),
    };

    let expanded = quote! {
        impl #impl_generics #struct_name #ty_generics #where_clause {
            pub fn new(#(#params)*) -> Self {
                #values
            }
        }
    };

    TokenStream::from(expanded)
}

#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Index};

/// Creates a `new()` constructor with specified default values for a struct.
///
/// # Examples
///
/// ## Basic usage
///
/// `#[new(value)]` attribute can be used to specify the default value for a field.
///
/// ```rust
/// use defew::Defew;
///
/// #[derive(Defew)]
/// struct Data {
///     a: i32,
///     #[new("ABC".into())]
///     b: String,
///     #[new(Some(42))]
///     c: Option<u64>,
/// }
///
/// let value = Data::new();
/// assert_eq!(value.a, 0);
/// assert_eq!(value.b, "ABC");
/// assert_eq!(value.c, Some(42));
/// ```
///
/// ## Default values
///
/// If no `#[new(..)]` attribute is provided, the default value is used for all fields.
///
/// ```rust
/// # use defew::Defew;
/// #
/// #[derive(Default, PartialEq, Debug, Defew)]
/// struct Data {
///     a: i32,
///     b: u64,
/// }
///
/// assert_eq!(Data::new(), Default::default());
/// ```
///
/// ## Constructor parameters
///
/// `#[new]` attribute can be used to ask for the value as a parameter of the `new()` constructor.
///
/// ```rust
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// struct Data(#[new] u64, #[new(123)] i32);
///
/// let model = Data::new(42);
/// assert_eq!(model.0, 42);
/// assert_eq!(model.1, 123);
/// ```
///
/// ## With Generics
///
/// ```rust
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// struct Data<T: From<u8>> {
///     #[new]
///     a: T,
///     #[new(98.into())]
///     b: T,
/// }
///
/// let value = Data::new('a');
/// assert_eq!(value.a, 'a');
/// assert_eq!(value.b, 'b');
/// ```
///
/// ## With `PhantomData`
///
/// ```ignore
/// # use defew::Defew;
/// # use std::marker::PhantomData;
/// #
/// #[derive(PartialEq, Defew)]
/// struct Data<T>(#[new] i32, PhantomData<T>);
///
/// let _42 = Data::<i32>::new(42) == Data::<isize>::new(42); // compile error
/// ```
///
/// ## With Trait
///
/// ```rust
/// # use defew::Defew;
/// #
/// trait DataTrait: Sized {
///     fn new(a: i32) -> Self;
///     fn init_with_42() -> Self {
///         Self::new(42)
///     }
/// }
///
/// #[derive(Defew)]
/// #[defew(DataTrait)]
/// struct Data {
///     #[new]
///     a: i32,
/// }
///
/// let value = Data::init_with_42();
/// assert_eq!(value.a, 42);
/// ```
///
/// ## Using other fields
///
/// ```rust
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// struct Data {
///     #[new]
///     price: f32,
///     #[new(price * 1.25)]
///     total: f32,
/// }
///
/// let value = Data::new(100.0);
/// assert_eq!(value.total, 125.0);
///
/// #[derive(Defew)]
/// struct Values(#[new] f32, #[new(0.18)] f32, #[new(_0 + _0 * _1)] f32);
///
/// let value = Values::new(100.0);
/// assert_eq!(value.2, 118.0);
/// ```
///
/// # Errors
///
/// compile fails if #[derive(Defew)] is used on anything other than a struct.
///
/// ```compile_fail
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// enum Data {
///     Foo,
///     Bar,
/// }
/// ```
///
/// compile fails if #[derive(Defew)] is used on a unit struct.
///
/// ```compile_fail
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// struct Data;
/// ```
///
/// compile fails if #[new(..)] is used with invalid value.
///
/// ```compile_fail
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// struct Data {
///     #[new(42, 11, 'a')]
///     a: i32,
/// }
/// ```
///
/// compile fails if #[new(..)] is used more than once.
///
/// ```compile_fail
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// struct Data {
///     #[new(42)]
///     #[new(11)]
///     a: i32,
/// }
/// ```
///
#[proc_macro_derive(Defew, attributes(new, defew))]
pub fn defew(input: TokenStream) -> TokenStream {
    let input = &parse_macro_input!(input as DeriveInput);
    let Data::Struct(DataStruct { fields, .. }) = &input.data else {
        return quote! ( compile_error!("Defew only supports structs"); ).into();
    };
    if matches!(fields, Fields::Unit) {
        return quote! ( compile_error!("Defew does not support unit structs"); ).into();
    }

    let (trait_for, visibility) = match get_token_result(&input.attrs, "defew") {
        // If the attribute is #[defew(trait)], we will implement the trait
        TokenResult::List(tokens) => (quote! { #tokens for }, quote!()), // => `impl Trait for Struct`, `fn new(..)`
        TokenResult::Err(e) => return e.to_compile_error().into(),
        // If the attribute is not present, we will not implement any trait
        _ => (quote!(), quote!(pub)), // => `impl Struct`, `pub fn new(..)`
    };

    let mut field_values = Vec::new();
    let mut params = Vec::new(); // params for the `::new(..)` constructor
    let mut variables = Vec::new();
    for (i, field) in fields.iter().enumerate().map(|(i, f)| (Index::from(i), f)) {
        let ty = &field.ty;
        #[allow(clippy::option_if_let_else)]
        let (param, arg) = match &field.ident {
            Some(ident) => (quote!(#ident), ident),
            None => (quote!(#i), &format_ident!("_{}", i)), // for unnamed fields: e.g. _0, _1, _2
        };
        field_values.push(quote! { #param: #arg, });

        #[cfg(feature = "std")]
        let default = quote! { <#ty as ::std::default::Default>::default() };
        #[cfg(not(feature = "std"))]
        let default = quote! { <#ty as ::core::default::Default>::default() };

        match get_token_result(&field.attrs, "new") {
            // If the attribute is #[new], we will ask for the value at runtime
            TokenResult::Path => params.push(quote! ( #arg: #ty )),
            // If the attribute is #[new(value)], we will use the provided value
            TokenResult::List(value) => variables.push(quote! { let #arg = #value; }),
            // If the attribute is not present, we will use the default value
            TokenResult::NoAttr => variables.push(quote! { let #arg = #default; }),
            TokenResult::Err(e) => return e.to_compile_error().into(),
        }
    }

    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();

    let expanded = quote! {
        #[automatically_derived]
        impl #impl_generics #trait_for #struct_name #ty_generics #where_clause {
            #visibility fn new(#(#params),*) -> Self {
                #(#variables)*
                Self { #(#field_values)* }
            }
        }
    };

    TokenStream::from(expanded)
}

enum TokenResult<'a> {
    Path,
    List(&'a proc_macro2::TokenStream),
    NoAttr,
    Err(syn::Error),
}

fn get_token_result<'a>(attrs: &'a [syn::Attribute], name: &'static str) -> TokenResult<'a> {
    use syn::{Error, MacroDelimiter, Meta, MetaList};
    use TokenResult::{Err, List, NoAttr, Path};

    let another = match name {
        "new" => "defew",
        "defew" => "new",
        _ => unreachable!(),
    };
    if let Some(attr) = attrs.iter().find(|attr| attr.path().is_ident(another)) {
        return Err(Error::new_spanned(
            attr,
            format!("Defew only supports #[{name}] here"),
        ));
    }

    let attrs: Vec<_> = attrs.iter().filter(|a| a.path().is_ident(name)).collect();
    if attrs.len() > 1 {
        return Err(Error::new_spanned(
            attrs.last(),
            "Defew accepts one attribute",
        ));
    }
    match &attrs.first().map(|attr| &attr.meta) {
        Some(Meta::Path(_)) => Path,
        Some(Meta::List(MetaList {
            tokens,
            delimiter: MacroDelimiter::Paren(_),
            ..
        })) if !tokens.is_empty() => List(tokens),
        Some(meta) => Err(Error::new_spanned(
            meta,
            format!("Defew supports #[{name}(..)] syntax"),
        )),
        None => NoAttr,
    }
}

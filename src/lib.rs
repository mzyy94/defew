#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::{format_ident, quote, TokenStreamExt};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

/// Creates a `new()` constructor with specified default values for a struct.
///
/// # Examples
///
/// ## Basic usage
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
/// ## Require parameters
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
/// # Panics
///
/// panic if #[derive(Defew)] is used on anything other than a struct
///
/// ```compile_fail
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// enum Data {
///   Foo,
///   Bar,
/// }
/// ```
///
/// panic if #[derive(Defew)] is used on a unit struct
///
/// ```compile_fail
/// # use defew::Defew;
/// #
/// #[derive(Defew)]
/// struct Data;
/// ```
///
#[proc_macro_derive(Defew, attributes(new, defew))]
pub fn defew(input: TokenStream) -> TokenStream {
    let input = &parse_macro_input!(input as DeriveInput);
    let Data::Struct(DataStruct { fields, .. }) = &input.data else {
        panic!("Defew only supports structs")
    };

    let (trait_for, visibility) = match get_token_result(&input.attrs, "defew") {
        // If the attribute is #[defew(trait)], we will implement the trait
        Ok(Some(tokens)) if !tokens.is_empty() => (quote! { #tokens for }, quote!()),
        Err(e) => return e.to_compile_error().into(),
        // If the attribute is not present, we will not implement any trait
        _ => (quote!(), quote!(pub)),
    };

    let mut default_values = Vec::new();
    let mut params = Vec::new();
    for (i, field) in fields.into_iter().enumerate() {
        let ty = &field.ty;
        let ident = field.ident.as_ref();
        let punct = ident.map(|_| quote!(:));

        default_values.push(match get_token_result(&field.attrs, "new") {
            // If the attribute is #[new], we will ask for the value at runtime
            Ok(Some(tokens)) if tokens.is_empty() => {
                let param = format_ident!("param{i}");
                let param = ident.unwrap_or(&param);
                params.push(quote! { #param: #ty, });
                quote! { #param, }
            }
            // If the attribute is #[new(value)], we will use the provided value
            Ok(Some(tokens)) => quote! { #ident #punct #tokens, },
            // If the attribute is not present, we will use the default value
            Ok(None) => quote! { #ident #punct Default::default(), },
            Err(e) => return e.to_compile_error().into(),
        });
    }

    let mut values = quote! { Self };
    let append = |v: &mut proc_macro2::TokenStream| v.append_all(default_values);

    match fields {
        Fields::Named(f) => f.brace_token.surround(&mut values, append),
        Fields::Unnamed(f) => f.paren_token.surround(&mut values, append),
        Fields::Unit => panic!("Defew does not support unit structs"),
    };

    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics #trait_for #struct_name #ty_generics #where_clause {
            #visibility fn new(#(#params)*) -> Self {
                #values
            }
        }
    };

    TokenStream::from(expanded)
}

fn get_token_result<'a>(
    attrs: &'a [syn::Attribute],
    name: &'static str,
) -> Result<Option<&'a proc_macro2::TokenStream>, syn::Error> {
    use syn::{Error, MacroDelimiter, Meta, MetaList};
    if attrs.len() > 1 {
        return Err(Error::new_spanned(
            attrs.last(),
            "Defew accepts one attribute",
        ));
    }
    let Some(attr) = attrs.first() else {
        return Ok(None);
    };
    if !attr.path().is_ident(name) {
        return Err(Error::new_spanned(
            attr,
            format!("Defew only supports #[{name}] here"),
        ));
    }
    match &attr.meta {
        Meta::Path(_) => Ok(Some(Box::leak(Box::new(quote!())))),
        Meta::List(MetaList {
            tokens,
            delimiter: MacroDelimiter::Paren(_),
            ..
        }) => Ok(Some(tokens)),
        _ => Err(Error::new_spanned(
            attr,
            format!("Defew supports #[{name}(..)] syntax"),
        )),
    }
}

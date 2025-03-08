#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

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
/// # Panics
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
        panic!("Defew only supports structs")
    };

    let (trait_for, visibility) = match get_token_result(&input.attrs, "defew") {
        // If the attribute is #[defew(trait)], we will implement the trait
        TokenResult::List(tokens) => (quote! { #tokens for }, quote!()), // => `impl Trait for Struct`, `fn new(..)`
        TokenResult::Err(e) => return e.to_compile_error().into(),
        // If the attribute is not present, we will not implement any trait
        _ => (quote!(), quote!(pub)), // => `impl Struct`, `pub fn new(..)`
    };

    let mut default_values = Vec::new();
    let mut params = Vec::new(); // params for the `::new(..)` constructor
    for (i, field) in fields.into_iter().enumerate() {
        let ty = &field.ty;
        let ident = field.ident.as_ref();
        let punct = ident.map(|_| quote!(:)); // for named struct fields

        default_values.push(match get_token_result(&field.attrs, "new") {
            // If the attribute is #[new], we will ask for the value at runtime
            TokenResult::Path => {
                let param = format_ident!("param{i}"); // for unnamed fields
                let param = ident.unwrap_or(&param);
                params.push(quote! { #param: #ty, });
                quote! { #param, }
            }
            // If the attribute is #[new(value)], we will use the provided value
            TokenResult::List(tokens) => quote! { #ident #punct #tokens, },
            // If the attribute is not present, we will use the default value
            TokenResult::NoAttr => quote! { #ident #punct Default::default(), },
            TokenResult::Err(e) => return e.to_compile_error().into(),
        });
    }

    let values = match fields {
        Fields::Named(_) => quote!( Self { #(#default_values)* } ),
        Fields::Unnamed(_) => quote!( Self ( #(#default_values)* ) ),
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

enum TokenResult<'a> {
    Path,
    List(&'a proc_macro2::TokenStream),
    NoAttr,
    Err(syn::Error),
}

fn get_token_result<'a>(attrs: &'a [syn::Attribute], name: &'static str) -> TokenResult<'a> {
    use syn::{Error, MacroDelimiter, Meta, MetaList};
    use TokenResult::{Err, List, NoAttr, Path};
    if attrs.len() > 1 {
        return Err(Error::new_spanned(
            attrs.last(),
            "Defew accepts one attribute",
        ));
    }
    let Some(attr) = attrs.first() else {
        return NoAttr;
    };
    if !attr.path().is_ident(name) {
        return Err(Error::new_spanned(
            attr,
            format!("Defew only supports #[{name}] here"),
        ));
    }
    match &attr.meta {
        Meta::Path(_) => Path,
        Meta::List(MetaList {
            tokens,
            delimiter: MacroDelimiter::Paren(_),
            ..
        }) if !tokens.is_empty() => List(tokens),
        _ => Err(Error::new_spanned(
            attr,
            format!("Defew supports #[{name}(..)] syntax"),
        )),
    }
}

#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields, Lit, Member, Result};

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
    defew_internal(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn defew_internal(input: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    let Data::Struct(DataStruct { fields, .. }) = &input.data else {
        return Ok(quote! ( compile_error!("Defew only supports structs"); ));
    };
    if matches!(fields, Fields::Unit) {
        return Ok(quote! ( compile_error!("Defew does not support unit structs"); ));
    }

    let (trait_for, visibility) = match get_token_result(&input.attrs, "defew")? {
        // If the attribute is #[defew(trait)], we will implement the trait
        Some(MetaToken::List(tokens)) => (quote! { #tokens for }, quote!()), // => `impl Trait for Struct`, `fn new(..)`
        // If the attribute is #[defew], we will implement the new() constructor with private visibility
        Some(MetaToken::Path) => (quote!(), quote!()), // => `impl Struct`, `fn new(..)`
        // If the attribute is #[defew = "crate"], we will implement the new() constructor with specified visibility
        Some(MetaToken::NameValue(Lit::Str(s))) => {
            let restriction: Option<proc_macro2::TokenStream> = s.parse().ok();
            (quote!(), quote!(pub(#restriction))) // => `impl Struct`, `pub(crate) fn new(..)`
        }
        // If the attribute is not present, we will not implement any trait
        _ => (quote!(), quote!(pub)), // => `impl Struct`, `pub fn new(..)`
    };

    let field_values: Vec<_> = fields
        .members()
        .map(|member| match member {
            Member::Named(ident) => (quote!(#ident), ident),
            Member::Unnamed(i) => (quote!(#i), format_ident!("_{}", i)),
        })
        .collect();

    let default = quote! { ::core::default::Default::default() };
    let mut params = Vec::new(); // params for the `::new(..)` constructor
    let mut variables = Vec::new();
    for (Field { ty, attrs, .. }, (_, arg)) in fields.iter().zip(&field_values) {
        match get_token_result(attrs, "new")? {
            // If the attribute is #[new], we will ask for the value at runtime
            Some(MetaToken::Path) => params.push(quote! ( #arg: #ty )),
            // If the attribute is #[new(value)], we will use the provided value
            Some(MetaToken::List(value)) => variables.push(quote! { let #arg: #ty = #value; }),
            // If the attribute is #[new = value], we will use the provided value as const
            Some(MetaToken::NameValue(v)) => variables.push(quote! { const #arg: #ty = #v; }),
            // If the attribute is not present, we will use the default value
            None => variables.push(quote! { let #arg: #ty = #default; }),
        }
    }

    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();
    let field_values = field_values.iter().map(|(f, v)| quote! { #f: #v });

    let expanded = quote! {
        #[automatically_derived]
        impl #impl_generics #trait_for #struct_name #ty_generics #where_clause {
            #[doc = "Creates a new instance of the struct with default values"]
            #[allow(non_upper_case_globals)]
            #visibility fn new(#(#params),*) -> Self {
                #(#variables)*
                Self { #(#field_values),* }
            }
        }
    };
    Ok(expanded)
}

enum MetaToken<'a> {
    Path,
    List(&'a proc_macro2::TokenStream),
    NameValue(&'a syn::Lit),
}

type TokenResult<'a> = Result<Option<MetaToken<'a>>>;

fn get_token_result<'a>(attrs: &'a [syn::Attribute], name: &'static str) -> TokenResult<'a> {
    use syn::{Error, Expr, ExprLit, MacroDelimiter, Meta, MetaList, MetaNameValue};
    use MetaToken::{List, NameValue, Path};

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
        Some(Meta::Path(_)) => Ok(Some(Path)),
        Some(Meta::List(MetaList {
            tokens,
            delimiter: MacroDelimiter::Paren(_),
            ..
        })) if !tokens.is_empty() => Ok(Some(List(tokens))),
        Some(Meta::NameValue(MetaNameValue {
            value: Expr::Lit(ExprLit { lit, .. }),
            ..
        })) => Ok(Some(NameValue(lit))),
        Some(meta) => Err(Error::new_spanned(
            meta,
            "Defew does not support this syntax",
        )),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use crate::{defew_internal, get_token_result};
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn test_get_token_result() {
        use crate::MetaToken::{List, NameValue, Path};
        use syn::parse_quote as pq;

        macro_rules! am {
            ($left:expr, $right:pat) => {
                assert!(matches!($left, $right));
            };
            ($left:expr, $right:pat,?) => {
                assert!(matches!($left, Ok(Some($right))));
            };
        }

        am!(get_token_result(&[pq!(#[new])], "new"), Path,?);
        am!(get_token_result(&[pq!(#[new(42)])], "new"), List(_),?);
        am!(get_token_result(&[pq!(#[new = 42])], "new"), NameValue(_),?);
        am!(get_token_result(&[pq!(#[serde])], "defew"), Ok(None));
        am!(get_token_result(&[pq!(#[defew])], "new"), Err(_));
        am!(get_token_result(&[pq!(#[new]), pq!(#[new])], "new"), Err(_));
        am!(get_token_result(&[pq!(#[new[1]])], "new"), Err(_));
    }

    #[test]
    fn test_defew_internal_basic() {
        let input = parse_quote! {
            struct Data {
                a: i32,
                #[new("ABC".into())]
                b: String,
                #[new(Some(42))]
                c: Option<u64>,
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl Data {
                #[doc = "Creates a new instance of the struct with default values"]
                #[allow(non_upper_case_globals)]
                pub fn new() -> Self {
                    let a: i32 = ::core::default::Default::default();
                    let b: String = "ABC".into();
                    let c: Option<u64> = Some(42);
                    Self { a: a, b: b, c: c }
                }
            }
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_basic_unnamed() {
        let input = parse_quote! {
            struct Data(#[new(42)] u64, i32);
        };

        let output = quote! {
            #[automatically_derived]
            impl Data {
                #[doc = "Creates a new instance of the struct with default values"]
                #[allow(non_upper_case_globals)]
                pub fn new() -> Self {
                    let _0: u64 = 42;
                    let _1: i32 = ::core::default::Default::default();
                    Self { 0: _0, 1: _1 }
                }
            }
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_with_visibility_and_const() {
        let input = parse_quote! {
            #[defew = "crate"]
            struct Data {
                #[new = 42]
                a: i32,
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl Data {
                #[doc = "Creates a new instance of the struct with default values"]
                #[allow(non_upper_case_globals)]
                pub(crate) fn new() -> Self {
                    const a: i32 = 42;
                    Self { a: a }
                }
            }
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_with_trait_generics() {
        let input = parse_quote! {
            #[defew(DataTrait<T>)]
            struct Data<T: From<u8>> {
                #[new]
                a: T,
                #[new(98.into())]
                b: T,
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl<T: From<u8> > DataTrait<T> for Data<T> {
                #[doc = "Creates a new instance of the struct with default values"]
                #[allow(non_upper_case_globals)]
                fn new(a: T) -> Self {
                    let b: T = 98.into();
                    Self { a: a, b: b }
                }
            }
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_reference_other_field() {
        let input = parse_quote! {
            struct Data {
                #[new]
                a: i32,
                #[new = 42]
                b: i32,
                #[new(a * b + 4)]
                c: i32,
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl Data {
                #[doc = "Creates a new instance of the struct with default values"]
                #[allow(non_upper_case_globals)]
                pub fn new(a: i32) -> Self {
                    const b: i32 = 42;
                    let c: i32 = a * b + 4;
                    Self { a: a, b: b, c: c }
                }
            }
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_reference_other_field_unnamed() {
        let input = parse_quote! {
            struct Data(#[new] i32, #[new(_0 * 2)] i32);
        };

        let output = quote! {
            #[automatically_derived]
            impl Data {
                #[doc = "Creates a new instance of the struct with default values"]
                #[allow(non_upper_case_globals)]
                pub fn new(_0: i32) -> Self {
                    let _1: i32 = _0 * 2;
                    Self { 0: _0, 1: _1 }
                }
            }
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_with_unit_struct() {
        let input = parse_quote! {
            struct Data;
        };

        let output = quote! {
            compile_error!("Defew does not support unit structs");
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_with_enum() {
        let input = parse_quote! {
            enum Data {
                Foo,
                Bar,
            }
        };

        let output = quote! {
            compile_error!("Defew only supports structs");
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_with_multiple_attributes() {
        let input = parse_quote! {
            struct Data {
                #[new(42)]
                #[new(11)]
                a: i32,
            }
        };

        let output = "Defew accepts one attribute";

        assert_eq!(
            defew_internal(&input).unwrap_err().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_with_invalid_syntax() {
        let input = parse_quote! {
            struct Data {
                #[new[1]]
                a: i32,
            }
        };

        let output = "Defew does not support this syntax";

        assert_eq!(
            defew_internal(&input).unwrap_err().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_with_invalid_attribute() {
        let input = parse_quote! {
            struct Data {
                #[defew]
                a: i32,
            }
        };

        let output = "Defew only supports #[new] here";

        assert_eq!(
            defew_internal(&input).unwrap_err().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_invalid_defew_attribute() {
        let input = parse_quote! {
            #[defew[1]]
            struct Data {
                a: i32,
            }
        };

        let output = "Defew does not support this syntax";

        assert_eq!(
            defew_internal(&input).unwrap_err().to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_defew_internal_with_no_visibility() {
        let input = parse_quote! {
            #[defew]
            struct Data {
                a: i32,
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl Data {
                #[doc = "Creates a new instance of the struct with default values"]
                #[allow(non_upper_case_globals)]
                fn new() -> Self {
                    let a: i32 = ::core::default::Default::default();
                    Self { a: a }
                }
            }
        };

        assert_eq!(
            defew_internal(&input).unwrap().to_string(),
            output.to_string()
        );
    }
}

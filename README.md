# Defew = Default + `new()`

[![CI](https://github.com/mzyy94/defew/actions/workflows/ci.yml/badge.svg)](https://github.com/mzyy94/defew/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/defew.svg)](http://crates.io/crates/defew)
[![docs.rs](https://img.shields.io/docsrs/defew.svg)](https://docs.rs/defew/)

Creates a `new()` constructor with specified default values for a struct.

```rust
use defew::Defew;

#[derive(Defew)]
struct Data {
    a: i32,
    #[new("ABC")]
    b: &'static str,
    #[new(Some(42))]
    c: Option<u64>,
}

let value = Data::new();
assert_eq!(value.a, 0);
assert_eq!(value.b, "ABC");
assert_eq!(value.c, Some(42));

#[derive(Defew)]
struct Values(#[new] i32, #[new(0.5)] f64);

let value = Values::new(42);
assert_eq!(value.0, 42);
assert_eq!(value.1, 0.5);
```

## Changelog

See [releases page](https://github.com/mzyy94/defew/releases)

## License

[MIT](LICENSE.MIT) OR [Apache-2.0](LICENSE.APACHE)

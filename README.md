# Defew = Default + new()

Creates a `new()` constructor with specified default values for a struct.

## Examples

```rust
use defew::Defew;

#[derive(Defew)]
pub struct X {
    a: i32,
    #[new("ABC")]
    b: &'static str,
    #[new(Some(42))]
    c: Option<u64>,
}

let x = X::new();
assert_eq!(x.a, 0);
assert_eq!(x.b, "ABC");
assert_eq!(x.c, Some(42));
```

## License

[MIT](LICENSE.MIT) OR [Apache-2.0](LICENSE.APACHE)

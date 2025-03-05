# Defew = Default + new()

Creates a `new()` constructor with specified default values for a struct.

## Examples

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
struct Values(i32, #[new(1.5 - 1f64)] f64);

let value = Values::new();
assert_eq!(value.0, 0);
assert_eq!(value.1, 0.5);

#[derive(Defew)]
struct Generic<T: From<u8>>(#[new(0x61.into())] T);

let value: Generic<char> = Generic::new();
assert_eq!(value.0, 'a');
let value: Generic<i32> = Generic::new();
assert_eq!(value.0, 97);
```

## License

[MIT](LICENSE.MIT) OR [Apache-2.0](LICENSE.APACHE)

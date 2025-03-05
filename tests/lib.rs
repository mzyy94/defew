#[cfg(test)]
mod tests {
    use defew::Defew;

    #[derive(Clone, Debug, PartialEq, Default, Defew)]
    pub struct Data {
        foo: i32,
        #[new("bar".to_string())]
        bar: String,
        #[new(42)]
        baz: u64,
    }

    #[test]
    fn test_defew() {
        let model = Data::new();
        assert_eq!(model.foo, 0);
        assert_eq!(model.bar, "bar".to_string());
        assert_eq!(model.baz, 42);
    }
}

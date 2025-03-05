#[cfg(test)]
mod tests {
    use defew::Defew;

    const BAR: &str = "bar";

    #[derive(Clone, Debug, PartialEq, Default, Defew)]
    pub struct Data {
        foo: i32,
        #[new(BAR.into())]
        bar: String,
        #[new(42)]
        baz: u64,
        #[new("123".parse().ok())]
        abc: Option<i32>,
    }

    #[test]
    fn test_defew() {
        let model = Data::new();
        assert_eq!(model.foo, 0);
        assert_eq!(model.bar, "bar".to_string());
        assert_eq!(model.baz, 42);
        assert_eq!(model.abc, Some(123));
    }
}

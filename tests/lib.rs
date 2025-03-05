#[cfg(test)]
mod tests {
    use defew::Defew;

    #[test]
    fn test_defew_basic() {
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

        let model = Data::new();
        assert_eq!(model.foo, 0);
        assert_eq!(model.bar, "bar".to_string());
        assert_eq!(model.baz, 42);
        assert_eq!(model.abc, Some(123));
    }

    #[test]
    fn test_defew_without_default() {
        #[derive(Defew)]
        pub struct Data {
            foo: i32,
            #[new(42i32 as u64)]
            baz: u64,
        }

        let model = Data::new();
        assert_eq!(model.foo, 0);
        assert_eq!(model.baz, 42);
    }

    #[test]
    fn test_defew_struct() {
        #[derive(Defew)]
        pub struct Data(#[new(42)] u64, i32);

        let model = Data::new();
        assert_eq!(model.0, 42);
        assert_eq!(model.1, 0);
    }
}

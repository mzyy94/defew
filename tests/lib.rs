#[cfg(test)]
mod tests {
    use defew::Defew;

    #[test]
    fn test_defew_basic() {
        const BAR: &str = "bar";

        #[derive(Clone, Debug, PartialEq, Default, Defew)]
        struct Data {
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
        struct Data {
            foo: i32,
            #[new(42i32 as u64)]
            baz: u64,
        }

        let model = Data::new();
        assert_eq!(model.foo, 0);
        assert_eq!(model.baz, 42);
    }

    #[test]
    fn test_defew_struct_unnamed() {
        #[derive(Defew)]
        struct Data(#[new(42)] u64, i32);

        let model = Data::new();
        assert_eq!(model.0, 42);
        assert_eq!(model.1, 0);
    }

    #[test]
    fn test_defew_struct_default_generic() {
        #[derive(Defew)]
        struct Data<T: Default> {
            #[new(42)]
            a: u64,
            b: T,
        }

        let model = Data::<i32>::new();
        assert_eq!(model.a, 42);
        assert_eq!(model.b, 0);
    }

    #[test]
    fn test_defew_struct_convert_generic() {
        #[derive(Defew)]
        struct Data<T: From<u8>> {
            #[new(80u8.into())]
            a: T,
        }

        let model = Data::<i64>::new();
        assert_eq!(model.a, 80i64);

        let model = Data::<char>::new();
        assert_eq!(model.a, 'P');
    }

    #[test]
    fn test_defew_param() {
        #[derive(Defew)]
        struct Data {
            a: String,
            #[new(param)]
            b: i32,
            #[new(42i32 as u64)]
            c: u64,
        }

        let model = Data::new(1);
        assert_eq!(model.a, "".to_string());
        assert_eq!(model.b, 1);
        assert_eq!(model.c, 42);
    }

    #[test]
    fn test_defew_param_unnamed() {
        #[derive(Defew)]
        struct Data(#[new(42)] u64, #[new(param)] i32);

        let model = Data::new(5);
        assert_eq!(model.0, 42);
        assert_eq!(model.1, 5);
    }

    #[test]
    fn test_defew_param_generic() {
        trait Fruit {
            type Output;
            fn tax() -> Self::Output;
        }
        struct Banana();
        impl Fruit for Banana {
            type Output = i32;
            fn tax() -> Self::Output {
                15
            }
        }
        struct Apple();
        impl Fruit for Apple {
            type Output = i32;
            fn tax() -> Self::Output {
                50
            }
        }

        #[derive(Defew)]
        struct Data<T: Fruit> {
            #[new(param)]
            _input: T,
            #[new(T::tax())]
            output: <T as Fruit>::Output,
        }

        let model = Data::new(Banana());
        assert_eq!(model.output, 15);
        let model = Data::new(Apple());
        assert_eq!(model.output, 50);
    }
}

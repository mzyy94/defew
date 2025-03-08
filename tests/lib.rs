#[cfg(test)]
mod tests {
    use defew::Defew;

    #[test]
    fn test_defew_basic() {
        const BAR: &str = "bar";

        #[derive(Defew)]
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
    fn test_defew_struct_unnamed() {
        #[derive(Defew)]
        struct Data(#[new(42)] u64, i32);

        let model = Data::new();
        assert_eq!(model.0, 42);
        assert_eq!(model.1, 0);
    }

    #[test]
    fn test_defew_struct_default_generics() {
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
    fn test_defew_struct_from_generics() {
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
    fn test_defew_required() {
        #[derive(Defew)]
        struct Data {
            a: String,
            #[new]
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
    fn test_defew_required_unnamed() {
        #[derive(Defew)]
        struct Data(#[new(42)] u64, #[new] i32);

        let model = Data::new(5);
        assert_eq!(model.0, 42);
        assert_eq!(model.1, 5);
    }

    #[test]
    fn test_defew_required_generics() {
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
            #[new]
            _input: T,
            #[new(T::tax())]
            output: <T as Fruit>::Output,
        }

        let model = Data::new(Banana());
        assert_eq!(model.output, 15);
        let model = Data::new(Apple());
        assert_eq!(model.output, 50);
    }

    #[test]
    fn test_defew_child_new() {
        #[derive(Default, Defew)]
        struct DataA {
            #[new(42)]
            value: i32,
        }

        #[derive(Defew)]
        struct DataB {
            a1: DataA,
            #[new(DataA::new())]
            a2: DataA,
        }

        let model = DataB::new();
        assert_eq!(model.a1.value, 0);
        assert_eq!(model.a2.value, 42);
    }

    #[test]
    fn test_defew_with_trait() {
        trait NewTrait {
            fn new(a: i32) -> Self;
            fn get123(&self) -> i32 {
                123
            }
        }

        #[derive(Defew)]
        #[defew(NewTrait)]
        struct DataA {
            #[new]
            a: i32,
            #[new(42)]
            b: u64,
        }

        #[derive(Defew)]
        struct DataB<T: NewTrait> {
            #[new(T::new(12))]
            a: T,
        }

        let data = DataB::<DataA>::new();
        assert_eq!(data.a.a, 12);
        assert_eq!(data.a.b, 42);
        assert_eq!(data.a.get123(), 123);
    }
}

#[test]
fn test_func() {
    #[fire::fire]
    #[allow(dead_code)]
    fn hello(name: String, age: i32) {
        println!("hello, {name}, age: {age}");
    }

    std::env::set_var("__IN__RUST_FIRE_TEST", "hello");
    fire::run!(test);
    std::env::remove_var("__IN__RUST_FIRE_TEST");
}

use serial_test::serial;

#[test]
#[serial]
fn test_func() {
    fire::__clear_fires!();

    #[fire::fire]
    #[allow(dead_code)]
    fn hello(name: String, age: i32) {
        println!("hello, {name}, age: {age}");
    }

    std::env::set_var("__IN__RUST_FIRE_TEST", "hello");
    fire::run!();
    std::env::remove_var("__IN__RUST_FIRE_TEST");
}

#[test]
#[serial]
fn test_no_args() {
    fire::__clear_fires!();

    #[fire::fire]
    fn noargs() {}

    std::env::set_var("__IN__RUST_FIRE_TEST", "noargs");
    fire::run!();
    std::env::remove_var("__IN__RUST_FIRE_TEST");
}

#[test]
#[serial]
fn test_mod() {
    fire::__clear_fires!();

    #[allow(dead_code)]
    #[fire::fire]
    mod command {
        pub fn hello(name: &str, age: i32) {
            println!("hello, {name}, age: {age}");
        }

        pub fn bye() {
            println!("bye");
        }
    }

    std::env::set_var("__IN__RUST_FIRE_TEST", "hello_mod");
    fire::run!();
    std::env::remove_var("__IN__RUST_FIRE_TEST");
}

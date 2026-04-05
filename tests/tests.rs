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

    fire::run_with_args!(vec!["--name=JohnSmith", "--age=22"]);
}

#[test]
#[serial]
fn test_option_args() {
    fire::__clear_fires!();

    #[fire::fire]
    #[allow(dead_code)]
    fn hello(name: String, age: i32, nickname: Option<&str>) {
        if let Some(nn) = nickname {
            println!("hello, {nn}, age: {age}");
        } else {
            println!("hello, {name}, age: {age}");
        }
    }

    fire::run_with_args!(vec!["--name=JohnSmith", "--age=22"]);
}

#[test]
#[serial]
fn test_no_args() {
    fire::__clear_fires!();

    #[fire::fire]
    fn noargs() {}

    fire::run_with_args!(Vec::<String>::new());
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

    fire::run_with_args!(vec!["hello", "--name=JohnSmith", "--age=22"]);
}

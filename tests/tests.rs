use std::sync::Mutex;

static CALLS: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[allow(dead_code)]
mod single_command {
    #[fire::main]
    fn hello(name: String, age: u32, nickname: Option<String>, verbose: bool) {
        super::CALLS
            .lock()
            .unwrap()
            .push(format!("{name}:{age}:{nickname:?}:{verbose}"));
    }

    pub(crate) fn run<I, S>(args: I) -> Result<(), String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        __fire_run_hello(args)
    }
}

#[allow(dead_code)]
mod command_group {
    #[fire::main]
    mod cli {
        pub fn say_hello(name: &str) {
            super::super::CALLS
                .lock()
                .unwrap()
                .push(format!("hello:{name}"));
        }

        pub fn bye() {
            super::super::CALLS.lock().unwrap().push("bye".to_string());
        }
    }

    pub(crate) fn run<I, S>(args: I) -> Result<(), String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        cli::__fire_run(args)
    }
}

fn take_call() -> String {
    CALLS.lock().unwrap().pop().unwrap()
}

#[test]
fn function_becomes_cli() {
    single_command::run(["--name", "John", "--age=22", "--verbose"]).unwrap();
    assert_eq!(take_call(), "John:22:None:true");
}

#[test]
fn module_functions_become_kebab_case_subcommands() {
    command_group::run(["say-hello", "--name", "John"]).unwrap();
    assert_eq!(take_call(), "hello:John");

    command_group::run(["bye"]).unwrap();
    assert_eq!(take_call(), "bye");
}

#[test]
fn errors_are_descriptive() {
    assert_eq!(
        single_command::run(["--age", "22"]).unwrap_err(),
        "missing required option '--name'"
    );
    assert_eq!(
        command_group::run(["missing"]).unwrap_err(),
        "unknown command 'missing'"
    );
}

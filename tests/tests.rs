use std::sync::Mutex;

static CALLS: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[allow(dead_code)]
mod single_command {
    /// Greet a person.
    #[fire::main]
    fn hello(
        /// Person to greet.
        name: String,
        /// Person's age.
        age: u32,
        nickname: Option<String>,
        /// Enable verbose output.
        verbose: bool,
    ) {
        super::CALLS
            .lock()
            .unwrap()
            .push(format!("{name}:{age}:{nickname:?}:{verbose}"));
    }

    pub(crate) fn run<I, S>(args: I) -> Result<Option<String>, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        __fire_run_hello(args)
    }
}

#[allow(dead_code)]
mod command_group {
    /// Greeting commands.
    #[fire::main]
    mod cli {
        /// Say hello.
        pub fn say_hello(name: &str) {
            super::super::CALLS
                .lock()
                .unwrap()
                .push(format!("hello:{name}"));
        }

        /// Say goodbye.
        pub fn bye() {
            super::super::CALLS.lock().unwrap().push("bye".to_string());
        }
    }

    pub(crate) fn run<I, S>(args: I) -> Result<Option<String>, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        cli::__fire_run(args)
    }
}

#[allow(dead_code)]
mod async_single_command {
    /// Greet a person asynchronously.
    #[fire::main(tokio)]
    async fn hello(name: String) {
        tokio::task::yield_now().await;
        super::CALLS
            .lock()
            .unwrap()
            .push(format!("async-hello:{name}"));
    }

    pub(crate) fn run<I, S>(args: I) -> Result<Option<String>, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        __fire_run_hello(args)
    }
}

#[allow(dead_code)]
mod async_command_group {
    /// Network commands.
    #[fire::main(tokio)]
    mod cli {
        /// Ping a host.
        pub async fn ping(host: String) -> Result<(), String> {
            if host == "unreachable" {
                return Err("host is unreachable".to_string());
            }
            tokio::task::yield_now().await;
            super::super::CALLS.lock().unwrap().push(format!("ping:{host}"));
            Ok(())
        }

        /// Show the version.
        pub fn version() {
            super::super::CALLS.lock().unwrap().push("version".to_string());
        }
    }

    pub(crate) fn run<I, S>(args: I) -> Result<Option<String>, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        cli::__fire_run(args)
    }
}

fn assert_called(expected: &str) {
    let mut calls = CALLS.lock().unwrap();
    let index = calls
        .iter()
        .position(|call| call == expected)
        .unwrap_or_else(|| panic!("no call recorded matching '{expected}'"));
    calls.remove(index);
}

#[test]
fn function_becomes_cli() {
    single_command::run(["--name", "John", "--age=22", "--verbose"]).unwrap();
    assert_called("John:22:None:true");
}

#[test]
fn module_functions_become_kebab_case_subcommands() {
    command_group::run(["say-hello", "--name", "John"]).unwrap();
    assert_called("hello:John");

    command_group::run(["bye"]).unwrap();
    assert_called("bye");
}

#[test]
fn async_function_becomes_cli() {
    async_single_command::run(["--name", "John"]).unwrap();
    assert_called("async-hello:John");
}

#[test]
fn async_module_mixes_async_and_sync_commands() {
    async_command_group::run(["ping", "--host", "localhost"]).unwrap();
    assert_called("ping:localhost");

    async_command_group::run(["version"]).unwrap();
    assert_called("version");

    let error = async_command_group::run(["ping", "--host", "unreachable"]).unwrap_err();
    assert_eq!(error, "host is unreachable");
}

#[test]
fn errors_are_descriptive() {
    let argument_error = single_command::run(["--age", "22"]).unwrap_err();
    assert!(argument_error.starts_with("missing required option '--name'"));
    assert!(argument_error.contains("Usage:"));
    assert!(argument_error.contains("For more information, try '--help'."));

    let command_error = command_group::run(["missing"]).unwrap_err();
    assert!(command_error.starts_with("unknown command 'missing'"));
    assert!(command_error.contains("Usage:"));
    assert!(command_error.contains("For more information, try '--help'."));
}

#[test]
fn option_is_not_consumed_as_another_options_value() {
    let error = single_command::run(["--name", "--verbose", "--age", "22"]).unwrap_err();
    assert!(error.starts_with("option '--name' requires a value"));
    assert!(error.contains("Usage:"));
}

#[test]
fn function_help_uses_signature_and_documentation() {
    let help = single_command::run(["--help"]).unwrap().unwrap();
    assert!(help.contains("Greet a person."));
    assert!(help.contains("Usage:"));
    assert!(help.contains("--name <NAME>"));
    assert!(help.contains("Person to greet."));
    assert!(help.contains("[--nickname <NICKNAME>]"));
    assert!(help.contains("[--verbose]"));
    assert!(help.contains("-h, --help"));
}

#[test]
fn command_group_has_root_and_command_help() {
    let root = command_group::run(["-h"]).unwrap().unwrap();
    assert!(root.contains("Greeting commands."));
    assert!(root.contains("say-hello"));
    assert!(root.contains("Say hello."));

    let command = command_group::run(["say-hello", "--help"])
        .unwrap()
        .unwrap();
    assert!(command.contains("Usage:"));
    assert!(command.contains("say-hello --name <NAME>"));
}

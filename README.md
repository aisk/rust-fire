# Rust Fire

Turn a Rust function or module into a command-line application with one attribute.

## Installation

```sh
cargo add fire
```

## One function

```rust
/// Welcome a person.
#[fire::main]
fn welcome(name: String, excited: bool) {
    let suffix = if excited { "!" } else { "." };
    println!("Welcome, {name}{suffix}");
}
```

```console
$ app --name "John Smith" --excited
Welcome, John Smith!
```

`#[fire::main]` generates the program entry point, so no separate `fn main()` or
registration call is needed.

## Help

`-h` and `--help` are generated automatically from the function signature and
documentation comments:

```rust
/// Start the HTTP server.
#[fire::main]
fn serve(
    /// Address to listen on.
    host: String,
    /// Enable verbose logging.
    verbose: bool,
) {}
```

```console
$ app --help
Start the HTTP server.

Usage: app --host <HOST> [--verbose]

Options:
    --host <HOST>    Address to listen on.
    --verbose        Enable verbose logging.
    -h, --help       Print help
```

For module applications, `app --help` lists the subcommands and
`app <COMMAND> --help` describes one subcommand.

## Subcommands

Place `#[fire::main]` on an inline module to turn its functions into subcommands:

```rust
#[fire::main]
mod cli {
    pub fn hello(name: String, times: Option<u32>) {
        for _ in 0..times.unwrap_or(1) {
            println!("Hello, {name}!");
        }
    }

    pub fn bye() {
        println!("Bye!");
    }
}
```

```console
$ app hello --name John --times 2
Hello, John!
Hello, John!

$ app bye
Bye!
```

Rust `snake_case` function and parameter names are exposed as CLI `kebab-case`
names.

## Parameters

The function signature defines the CLI:

| Rust type | CLI behavior |
| --- | --- |
| `T` | required option parsed with `FromStr` |
| `Option<T>` | optional option |
| `bool` | value-less flag, defaulting to `false` |
| `&str` | borrowed string option |

Both common option formats are accepted:

```console
$ app --name John
$ app --name=John
```

A command may return `Result`. Errors are printed to stderr and the application
exits with status 2.

```rust
#[fire::main]
fn deploy(target: String) -> Result<(), DeployError> {
    do_deploy(&target)?;
    Ok(())
}
```

## License

BSD-2-Clause.

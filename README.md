# Rust Fire

![logo](https://orion-uploads.openroadmedia.com/lg_805c44212fcc-rengoku-2.jpg)

Turn your function(s) to a command line app. Inspires by Google's [Python Fire](github.com/google/python-fire).

## Installation

```sh
cargo add fire
```

## Usage

```rust
// Turn a single function to CLI app.
#[fire::fire]
fn welcome() {
    println!("Welcome!");
}

// Turn mutilple functions to CLI app.
#[fire::fire]
mod some_functions {
    pub fn hello(name: String, times: i32) {
        for _ in 0..times {
            println!("Hello, {name}!");
        }
    }

    pub fn bye() {
        println!("Bye!");
    }
}

fn main() {
    // 'Fire' the functions with command line arguments.
    fire::run!();
}
```

Now you can run your CLI app. By default the single function `welcome` should be called:

```sh
$ cargo build
$ ./target/debug/app
Welcome!
```

The funtions in the mod should be called as sub-command with it's function name:

```sh
$ cargo build
$ ./target/debug/app bye
Bye!
```

Funtions with arguments will receive the arguments from CLI app arguments, with format like `--argname=argvalue`:

```sh
$ cargo build
$ ./target/debug/app hello --name='John Smith' --times=3
Hello, John Smith!
Hello, John Smith!
Hello, John Smith!
```

Fire will call `.parse()` on every argument (except `&str`), so all types which implements [FromStr](https://doc.rust-lang.org/std/str/trait.FromStr.html) plus `&str` is supported. For optional argument, you can use the `Option` generic type, like `Option<String>` or `Option<i32>`;

## License

Licensed under the [BSD](https://github.com/aisk/rust-fire/blob/master/LICENSE) License.

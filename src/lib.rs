use std::collections::BTreeMap;
use std::sync::Mutex;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};

#[derive(Clone, Debug)]
struct Args {
    name: String,
    ty: String,
}

#[derive(Clone, Debug)]
struct SubCommand {
    func: String,
    args: Vec<Args>,
}

type Command = BTreeMap<String, SubCommand>;

// dumps to this format:
// cmd1-func2-arg1:type1-arg2:type2
// cmd2-func2-arg1:type1-arg2:type2
fn dumps(f: Command) -> String {
    let mut result = "".to_string();
    for (i, (name, cmd)) in f.iter().enumerate() {
        result += &name;
        result += "-";
        result += &cmd.func;
        result += "-";
        for (j, argdef) in cmd.args.iter().enumerate() {
            result += &argdef.name;
            result += ":";
            result += &argdef.ty;
            if j != cmd.args.len() - 1 {
                result += "-";
            }
        }
        if i != f.len() - 1 {
            result += "\n";
        }
    }

    return result;
}

lazy_static! {
    static ref FIRES: Mutex<Command> = Mutex::new(BTreeMap::new());
}

#[proc_macro]
pub fn __clear_fires(_: TokenStream) -> TokenStream {
    let mut f = FIRES.lock().unwrap();
    f.clear();
    return "".parse().unwrap();
}

#[proc_macro_attribute]
pub fn fire(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let item: syn::Item = syn::parse(input.clone()).unwrap();
    match item {
        syn::Item::Fn(f) => {
            fire_func("".to_string(), "".to_string(), f);
        }
        syn::Item::Mod(m) => {
            fire_mod(m);
        }
        _ => {
            return quote! (
                compile_error!("fire only support `fn` or `mod`");
            )
            .into();
        }
    };

    return input;
}

fn fire_func(key: String, modname: String, ast: syn::ItemFn) {
    let funcname = ast.sig.ident.to_string();
    let mut args: Vec<Args> = Vec::new();

    for x in ast.sig.inputs {
        match x {
            syn::FnArg::Typed(a) => {
                args.push(Args {
                    name: a.pat.to_token_stream().to_string(),
                    ty: a.ty.to_token_stream().to_string(),
                });
            }
            _ => panic!("not supported"),
        };
    }

    let mut m = FIRES.lock().unwrap();
    let full_func_name = if modname == "" {
        funcname
    } else {
        format!("{modname}::{funcname}")
    };
    m.insert(
        key,
        SubCommand {
            func: full_func_name,
            args,
        },
    );
}

fn fire_mod(ast: syn::ItemMod) {
    for item in ast.content.unwrap().1 {
        if let syn::Item::Fn(f) = item {
            let func = f.sig.ident.to_string();
            fire_func(func, ast.ident.to_string(), f);
        }
    }
}

#[proc_macro]
pub fn run(_: TokenStream) -> TokenStream {
    let defs = quote! {
        use std::collections::BTreeMap;

        #[derive(Debug, Clone)]
        struct Args {
            name: String,
            ty: String,
        }

        #[derive(Debug, Clone)]
        struct SubCommand {
            func: String,
            args: Vec<Args>,
        }

        #[derive(Debug)]
        struct FireError{
            reason: String,
        };

        impl FireError {
            fn new(reason: String) -> Self {
                FireError{reason}
            }
        }

        impl std::fmt::Display for FireError {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f,"{}", self.reason)
            }
        }

        impl std::error::Error for FireError {
            fn description(&self) -> &str {
                &self.reason
            }
        }

        type FireResult<T> = std::result::Result<T, FireError>;

        type Command = BTreeMap<String, SubCommand>;

        fn parse_arg(arg: String) -> FireResult<(String, String)> {
            if !arg.starts_with("--") {
                return Err(FireError::new(format!("invalid parameters: '{}'", arg)));
            }
            let arg = &arg[2..];

            let parts: Vec<&str> = arg.split("=").collect();
            if parts.len() != 2 {
                return Err(FireError::new(format!("invalid parameters: '{}'", arg)));
            }

            return Ok((parts[0].to_string(), parts[1].to_string()));
        }

        fn parse_args(args: Vec<String>) -> FireResult<Vec<(String, String)>> {
            let mut result = vec![];

            for arg in args {
                result.push(parse_arg(arg)?);
            }

            return Ok(result);
        }

        // returns `("xxx", ["--args=1"])` when input is `["xxx", "--args=1"]`,
        // returns `("", ["--args=1"])` when input is `["--args=1"]`.
        fn parse_command(args: Vec<String>) -> (String, Vec<String>) {
            if args.len() == 0 {
                return ("".to_string(), args);
            }
            if !args[0].starts_with("-") {
                return (args[0].clone(), args[1..].to_vec());
            }
            return ("".to_string(), args);
        }

        fn loads(input: String) -> Command {
            let mut result = Command::new();

            for line in input.split('\n') {
                let parts: Vec<&str> = line.splitn(3, '-').collect();
                let name = parts[0];
                let func = parts[1];

                let mut args: Vec<Args> = Vec::new();
                if parts.len() > 2 && parts[2] != "" {
                    for s in parts[2].split('-') {
                        let parts: Vec<&str> = s.splitn(2, ':').collect();
                        args.push(Args {
                            name: parts[0].to_string(),
                            ty: parts[1].to_string(),
                        });
                    }
                }

                result.insert(name.to_string(), SubCommand { func: func.to_string(), args });
            }

            return result;
        }
    };

    let m = FIRES.lock().unwrap();
    let data: String = dumps(m.clone());

    let parses = quote! {
        let m = loads(#data.to_string());

        if m.len() == 0 {
            panic!("no function registered");
        }

        let mut args = if std::env::var("__IN__RUST_FIRE_TEST") == Ok("hello".to_string()) {
            // Hack for test.
            vec!["--name=JohnSmith".to_string(), "--age=22".to_string()]
        } else if std::env::var("__IN__RUST_FIRE_TEST") == Ok("hello_mod".to_string()) {
            // Hack for test.
            vec!["hello".to_string(), "--name=JohnSmith".to_string(), "--age=22".to_string()]
        } else if std::env::var("__IN__RUST_FIRE_TEST") == Ok("noargs".to_string()) {
            // Hack for test.
            vec![]
        } else {
            std::env::args().skip(1).collect()
        };

        let (cmd, args) = parse_command(args);
        let mut pargs = parse_args(args).unwrap();

        let subcommand = m.get(cmd.as_str()).expect("func not found");
        let argdefs = &subcommand.args;

        let mut args: Vec<String> = vec![];

        for def in argdefs {
            let mut found = false;
            for (i, arg) in pargs.to_owned().iter().enumerate() {
                if arg.0 != def.name {
                    continue;
                }
                found = true;
                args.push(pargs[i].1.clone());
                pargs.remove(i);
            }
            if !found {
                args.push("__FIRE_THIS_IS_NONE".to_owned());
            }
        }

        if !pargs.is_empty() {
            panic!("unexpected args: {}", pargs[0].0);
        }
    };

    let mut calls = quote! {};

    for (k, v) in m.iter() {
        let fullname = &v.func;
        let e = syn::parse_str::<syn::Expr>(fullname).unwrap();
        let mut args = quote! {};
        for i in 0..v.args.len() {
            let ty = &v.args[i].ty;
            let name = &v.args[i].name;
            if ty == "Option < & str >" {
                args.extend(quote! {
                    if args[#i] == "__FIRE_THIS_IS_NONE" {
                        None
                    } else {
                        Some(args[#i].as_str())
                    },
                });
            } else if ty.starts_with("Option <") && ty.ends_with(">") {
                args.extend(quote! {
                    if args[#i] == "__FIRE_THIS_IS_NONE" {
                        None
                    } else {
                        Some(args[#i].parse().expect(format!("parse {} failed", args[#i]).as_str()))
                    },
                });
            } else if ty == "& str" {
                args.extend(quote! {
                    if args[#i] == "__FIRE_THIS_IS_NONE" {
                        panic!("arg '{}' not specified!", #name)
                    } else {
                        args[#i].as_str()
                    },
                });
            } else {
                args.extend(quote! {
                    if args[#i] == "__FIRE_THIS_IS_NONE" {
                        panic!("arg '{}' not specified!", #name)
                    } else {
                        args[#i].parse().expect(format!("parse {} failed", args[#i]).as_str())
                    },
                });
            }
        }
        let xxx = quote! {
            if (cmd == #k) {
                #e(#args);
            }
        };

        calls.extend(xxx);
    }

    return quote! {
        #defs
        #parses
        #calls
    }
    .into();
}

#[test]
fn test_fire_dumps() {
    let mut f = Command::new();
    f.insert(
        "command1".to_string(),
        SubCommand {
            func: "func1".to_string(),
            args: vec![
                Args {
                    name: "arg1".to_string(),
                    ty: "u32".to_string(),
                },
                Args {
                    name: "arg2".to_string(),
                    ty: "u32".to_string(),
                },
            ],
        },
    );
    f.insert(
        "command2".to_string(),
        SubCommand {
            func: "func2".to_string(),
            args: vec![Args {
                name: "arg1".to_string(),
                ty: "u32".to_string(),
            }],
        },
    );
    let s = dumps(f);
    assert!(s == "command1-func1-arg1:u32-arg2:u32\ncommand2-func2-arg1:u32".to_string());
}

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

        fn parse_command(args: Vec<String>) -> (String, Vec<String>) {
            if args.len() == 0 {
                return ("".to_string(), args);
            }
            if !args[0].starts_with("-") {
                return (args[0].clone(), args[1..].to_vec());
            }
            return ("".to_string(), args);
        }
    };

    let m = FIRES.lock().unwrap();

    let mut command_builder = quote! {
        let mut m = Command::new();
    };

    for (key, subcommand) in m.iter() {
        let func_name = &subcommand.func;

        let args_vec = if subcommand.args.is_empty() {
            quote! { vec![] }
        } else {
            let mut args_tokens = quote! {};
            for arg in &subcommand.args {
                let arg_name = &arg.name;
                let arg_type = &arg.ty;
                args_tokens.extend(quote! {
                    Args {
                        name: #arg_name.to_string(),
                        ty: #arg_type.to_string(),
                    },
                });
            }
            quote! { vec![ #args_tokens ] }
        };

        command_builder.extend(quote! {
            m.insert(
                #key.to_string(),
                SubCommand {
                    func: #func_name.to_string(),
                    args: #args_vec,
                },
            );
        });
    }

    let parses = quote! {
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
        #command_builder
        #defs
        #parses
        #calls
    }
    .into();
}

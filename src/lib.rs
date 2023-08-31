use std::collections::BTreeMap;
use std::sync::Mutex;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};

type Command = BTreeMap<String, (String, Vec<(String, String)>)>;

// dumps to this format:
// cmd1-func2-arg1:type1-arg2:type2
// cmd2-func2-arg1:type1-arg2:type2
fn dumps(f: Command) -> String {
    let mut result = "".to_string();
    for (i, (name, cmd)) in f.iter().enumerate() {
        result += &name;
        result += "-";
        result += &cmd.0;
        result += "-";
        for (j, argdef) in cmd.1.iter().enumerate() {
            result += &argdef.0;
            result += ":";
            result += &argdef.1;
            if j != cmd.1.len() - 1 {
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

#[proc_macro_attribute]
pub fn fire(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let ast: syn::ItemFn = syn::parse(input.clone()).unwrap();
    let func = ast.sig.ident.to_string();
    let mut args: Vec<(String, String)> = Vec::new();

    for x in ast.sig.inputs {
        match x {
            syn::FnArg::Typed(a) => {
                args.push((
                    a.pat.to_token_stream().to_string(),
                    a.ty.to_token_stream().to_string(),
                ));
            }
            _ => panic!("not supported"),
        };
    }

    let mut m = FIRES.lock().unwrap();
    m.insert("".to_string(), (func, args));

    return input;
}

#[proc_macro]
pub fn run(_: TokenStream) -> TokenStream {
    let defs = quote! {
        use std::collections::BTreeMap;

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

        type Command = BTreeMap<String, (String, Vec<(String, String)>)>;

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

        fn loads(input: String) -> Command {
            let mut result = Command::new();

            for line in input.split('\n') {
                let parts: Vec<&str> = line.splitn(3, '-').collect();
                let name = parts[0];
                let func = parts[1];

                let mut args: Vec<(String, String)> = Vec::new();
                for s in parts[2].split('-') {
                    let parts: Vec<&str> = s.splitn(2, ':').collect();
                    args.push((parts[0].to_string(), parts[1].to_string()));
                }

                result.insert(name.to_string(), (func.to_string(), args));
            }

            return result;
        }
    };

    let m = FIRES.lock().unwrap();
    let data: String = dumps(m.clone());

    let parses = quote! {
        let m = loads(#data.to_string());

        if m.len() != 1 {
            panic!("multiple or zero command not implemented");
        }

        let mut pargs = if std::env::var("__IN__RUST_FIRE_TEST") == Ok("hello".to_string()) {
            parse_args(vec!["--name=JohnSmith".to_string(), "--age=22".to_string()]).unwrap()
        } else {
            let osargs: Vec<String> = std::env::args().skip(1).collect();
            parse_args(osargs).unwrap()
        };

        let (func, argdefs) = m.get("").expect("not implemented");

        let mut args: Vec<String> = vec![];

        for def in argdefs {
            for (i, arg) in pargs.to_owned().iter().enumerate() {
                if arg.0 != def.0 {
                    continue;
                }
                args.push(pargs[i].1.clone());
                pargs.remove(i);
            }
        }

        if !pargs.is_empty() {
            panic!("unexpected args: {}", pargs[0].0);
        }
    };

    let (func, _) = m.get("").expect("not implemented");

    let mut calls = quote! {};

    for (_k, v) in m.iter() {
        let funcname = format_ident!("{}", func);
        let mut args = quote! {};
        for i in 0..v.1.len() {
            args.extend(quote! {
                args[#i].parse().expect(format!("parse {} failed", args[#i]).as_str()),
            });
        }
        let xxx = quote! {
            #funcname(#args);
        };

        calls.extend(xxx);

        break; // TODO: support non "" subcommands.
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
        (
            "func1".to_string(),
            vec![
                ("arg1".to_string(), "u32".to_string()),
                ("arg2".to_string(), "u32".to_string()),
            ],
        ),
    );
    f.insert(
        "command2".to_string(),
        (
            "func2".to_string(),
            vec![("arg1".to_string(), "u32".to_string())],
        ),
    );
    let s = dumps(f);
    assert!(
        s == "command1-func1-arg1:u32-arg2:u32
command2-func2-arg1:u32"
            .to_string()
    );
}

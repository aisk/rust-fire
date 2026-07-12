use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, Item, ItemFn, ItemMod, Pat, ReturnType, Type};

struct Argument {
    ident: Ident,
    ty: Type,
    cli_name: String,
    kind: ArgumentKind,
}

#[derive(Clone, Copy)]
enum ArgumentKind {
    Required,
    Optional,
    Flag,
}

fn kebab_case(name: &str) -> String {
    name.replace('_', "-")
}

fn inner_type<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != wrapper {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    match arguments.args.first()? {
        syn::GenericArgument::Type(ty) => Some(ty),
        _ => None,
    }
}

fn is_bool(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.path.is_ident("bool"))
}

fn is_str_reference(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference)
        if matches!(&*reference.elem, Type::Path(path) if path.path.is_ident("str")))
}

fn arguments(function: &ItemFn) -> syn::Result<Vec<Argument>> {
    function
        .sig
        .inputs
        .iter()
        .map(|input| {
            let FnArg::Typed(input) = input else {
                return Err(syn::Error::new_spanned(
                    input,
                    "methods cannot be CLI commands",
                ));
            };
            let Pat::Ident(pattern) = &*input.pat else {
                return Err(syn::Error::new_spanned(
                    &input.pat,
                    "command parameters must be identifiers",
                ));
            };
            if !input.attrs.is_empty() {
                return Err(syn::Error::new_spanned(
                    &input.attrs[0],
                    "parameter attributes are not supported",
                ));
            }

            let kind = if is_bool(&input.ty) {
                ArgumentKind::Flag
            } else if inner_type(&input.ty, "Option").is_some() {
                ArgumentKind::Optional
            } else {
                ArgumentKind::Required
            };

            Ok(Argument {
                ident: pattern.ident.clone(),
                ty: (*input.ty).clone(),
                cli_name: kebab_case(&pattern.ident.to_string()),
                kind,
            })
        })
        .collect()
}

fn parsed_value(value: TokenStream2, ty: &Type, cli_name: &str) -> TokenStream2 {
    if is_str_reference(ty) {
        quote! { #value.as_str() }
    } else {
        quote! {
            #value.parse::<#ty>().map_err(|_| {
                format!("invalid value for '--{}': '{}'", #cli_name, #value)
            })?
        }
    }
}

fn command_runner(
    function: &ItemFn,
    runner_name: &Ident,
    visibility: TokenStream2,
) -> syn::Result<TokenStream2> {
    if function.sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            function.sig.asyncness,
            "async commands are not supported",
        ));
    }
    if !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &function.sig.generics,
            "generic commands are not supported",
        ));
    }

    let arguments = arguments(function)?;
    let function_name = &function.sig.ident;

    let storage = arguments.iter().map(|argument| {
        let storage_name = format_ident!("__fire_value_{}", argument.ident);
        quote! { let mut #storage_name: Option<String> = None; }
    });

    let option_matches = arguments.iter().map(|argument| {
        let storage_name = format_ident!("__fire_value_{}", argument.ident);
        let cli_name = &argument.cli_name;
        match argument.kind {
            ArgumentKind::Flag => quote! {
                if __fire_key == concat!("--", #cli_name) {
                    if __fire_inline_value.is_some() {
                        return Err(format!("flag '--{}' does not take a value", #cli_name));
                    }
                    #storage_name = Some("true".to_string());
                    __fire_matched = true;
                }
            },
            ArgumentKind::Required | ArgumentKind::Optional => quote! {
                if __fire_key == concat!("--", #cli_name) {
                    let value = match __fire_inline_value {
                        Some(value) => value.to_string(),
                        None => {
                            __fire_index += 1;
                            __fire_args.get(__fire_index).cloned().ok_or_else(|| {
                                format!("option '--{}' requires a value", #cli_name)
                            })?
                        }
                    };
                    #storage_name = Some(value);
                    __fire_matched = true;
                }
            },
        }
    });

    let conversions = arguments.iter().map(|argument| {
        let ident = &argument.ident;
        let ty = &argument.ty;
        let storage_name = format_ident!("__fire_value_{}", ident);
        let cli_name = &argument.cli_name;
        match argument.kind {
            ArgumentKind::Flag => quote! {
                let #ident: bool = #storage_name.is_some();
            },
            ArgumentKind::Optional => {
                let inner = inner_type(ty, "Option").expect("optional type checked above");
                if is_str_reference(inner) {
                    quote! { let #ident: #ty = #storage_name.as_deref(); }
                } else {
                    let parsed = parsed_value(quote! { value }, inner, cli_name);
                    quote! {
                        let #ident: #ty = match #storage_name.as_ref() {
                            Some(value) => Some(#parsed),
                            None => None,
                        };
                    }
                }
            }
            ArgumentKind::Required => {
                if is_str_reference(ty) {
                    quote! {
                        let #ident: #ty = #storage_name.as_deref().ok_or_else(|| {
                            format!("missing required option '--{}'", #cli_name)
                        })?;
                    }
                } else {
                    let parsed = parsed_value(quote! { value }, ty, cli_name);
                    quote! {
                        let #ident: #ty = {
                            let value = #storage_name.as_ref().ok_or_else(|| {
                                format!("missing required option '--{}'", #cli_name)
                            })?;
                            #parsed
                        };
                    }
                }
            }
        }
    });

    let call_arguments = arguments.iter().map(|argument| &argument.ident);
    let call = match &function.sig.output {
        ReturnType::Type(_, ty) if inner_type(ty, "Result").is_some() => quote! {
            #function_name(#(#call_arguments),*)
                .map(|_| ())
                .map_err(|error| error.to_string())
        },
        _ => quote! {
            #function_name(#(#call_arguments),*);
            Ok(())
        },
    };

    Ok(quote! {
        #[doc(hidden)]
        #visibility fn #runner_name<I, S>(input: I) -> Result<(), String>
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            let __fire_args: Vec<String> = input.into_iter().map(Into::into).collect();
            #(#storage)*

            let mut __fire_index = 0usize;
            while __fire_index < __fire_args.len() {
                let __fire_raw = &__fire_args[__fire_index];
                let (__fire_key, __fire_inline_value) = match __fire_raw.split_once('=') {
                    Some((key, value)) => (key, Some(value)),
                    None => (__fire_raw.as_str(), None),
                };
                let mut __fire_matched = false;
                #(#option_matches)*
                if !__fire_matched {
                    return Err(format!("unexpected argument '{}'", __fire_raw));
                }
                __fire_index += 1;
            }

            #(#conversions)*
            #call
        }
    })
}

fn entrypoint(call: TokenStream2) -> TokenStream2 {
    quote! {
        fn main() {
            if let Err(error) = #call {
                eprintln!("error: {}", error);
                std::process::exit(2);
            }
        }
    }
}

fn expand_function(function: ItemFn) -> syn::Result<TokenStream2> {
    if function.sig.ident == "main" {
        return Err(syn::Error::new_spanned(
            &function.sig.ident,
            "put #[fire::main] on the command function, not on a function named `main`",
        ));
    }
    let runner_name = format_ident!("__fire_run_{}", function.sig.ident);
    let runner = command_runner(&function, &runner_name, quote! { pub(crate) })?;
    let main = entrypoint(quote! { #runner_name(std::env::args().skip(1)) });
    Ok(quote! { #function #runner #main })
}

fn expand_module(mut module: ItemMod) -> syn::Result<TokenStream2> {
    let module_name = module.ident.clone();
    let Some((_, items)) = &mut module.content else {
        return Err(syn::Error::new_spanned(
            &module,
            "#[fire::main] requires an inline module",
        ));
    };

    let mut commands = Vec::new();
    let mut runners = Vec::new();
    for item in items.iter() {
        let Item::Fn(function) = item else {
            continue;
        };
        let command_name = kebab_case(&function.sig.ident.to_string());
        let runner_name = format_ident!("__fire_run_{}", function.sig.ident);
        runners.push(command_runner(function, &runner_name, quote! {})?);
        commands.push((command_name, runner_name));
    }

    for runner in runners {
        items.push(syn::parse2(runner).expect("generated command runner"));
    }
    let dispatch = commands.iter().map(|(command_name, runner_name)| {
        quote! { #command_name => #runner_name(arguments), }
    });
    items.push(
        syn::parse2(quote! {
            #[doc(hidden)]
            pub(crate) fn __fire_run<I, S>(input: I) -> Result<(), String>
            where
                I: IntoIterator<Item = S>,
                S: Into<String>,
            {
                let mut input = input.into_iter().map(Into::into);
                let command = input.next().ok_or_else(|| "missing command".to_string())?;
                let arguments: Vec<String> = input.collect();
                match command.as_str() {
                    #(#dispatch)*
                    _ => Err(format!("unknown command '{}'", command)),
                }
            }
        })
        .expect("generated command dispatcher"),
    );

    let main = entrypoint(quote! { #module_name::__fire_run(std::env::args().skip(1)) });
    Ok(quote! { #module #main })
}

/// Turns a function or inline module into a complete command-line application.
#[proc_macro_attribute]
pub fn main(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    let expanded = match item {
        Item::Fn(function) => expand_function(function),
        Item::Mod(module) => expand_module(module),
        other => Err(syn::Error::new_spanned(
            other,
            "#[fire::main] only supports functions and inline modules",
        )),
    };
    expanded
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

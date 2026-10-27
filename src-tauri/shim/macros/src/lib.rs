//! TIQR in a phone's browser: `#[tauri::command]` for the WebAssembly build.
//!
//! The desktop app's commands are ordinary functions marked
//! `#[tauri::command]`; Tauri's own macro turns each into something its IPC
//! can call. The phone build has no Tauri, so this macro does the same job:
//! it leaves the function exactly as written and adds a sibling,
//! `__tiqr_wasm__<name>`, that takes the call's JSON arguments, hands each
//! parameter what it asks for - the app's state, the app handle, or its
//! argument by its camelCase name, as Tauri does - and turns what the command
//! returns into the answer the screen expects.
//!
//! What a parameter is decided by its type's name, as written: `State<..>` is
//! the app's state, `AppHandle` the app handle, anything else an argument.
//! The same for the answer: `Result`/`AppResult` is a success or an error,
//! `Response` raw bytes, anything else a value.
use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Attribute, FnArg, GenericArgument, ItemFn, Pat, Path, PathArguments, ReturnType, Token, Type};

#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let keep_names = attr.to_string().replace(' ', "").contains("rename_all=\"snake_case\"");
    let func = parse_macro_input!(item as ItemFn);
    match expand(&func, keep_names) {
        Ok(wrapper) => quote! { #func #wrapper }.into(),
        Err(e) => {
            let e = e.to_compile_error();
            quote! { #func #e }.into()
        }
    }
}

enum Param {
    State,
    App,
    Arg,
}

fn last_ident(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        Type::Reference(r) => last_ident(&r.elem),
        Type::Paren(p) => last_ident(&p.elem),
        Type::Group(g) => last_ident(&g.elem),
        _ => None,
    }
}

fn kind(ty: &Type) -> Param {
    match last_ident(ty).as_deref() {
        Some("State") => Param::State,
        Some("AppHandle") => Param::App,
        _ => Param::Arg,
    }
}

/// `event_id` -> `eventId`, as Tauri names a command's arguments.
fn camel(name: &str) -> String {
    let mut out = String::new();
    for (i, part) in name.split('_').filter(|p| !p.is_empty()).enumerate() {
        if i == 0 {
            out.push_str(part);
        } else {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
        }
    }
    out
}

/// The `T` of `Result<T, E>` / `AppResult<T>`, or `None` when the type is not
/// a result.
fn ok_type(ty: &Type) -> Option<&Type> {
    let Type::Path(p) = ty else { return None };
    let last = p.path.segments.last()?;
    if !last.ident.to_string().ends_with("Result") {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else { return None };
    args.args.iter().find_map(|a| match a {
        GenericArgument::Type(t) => Some(t),
        _ => None,
    })
}

fn is_bytes(ty: &Type) -> bool {
    last_ident(ty).as_deref() == Some("Response")
}

fn expand(func: &ItemFn, keep_names: bool) -> syn::Result<Tokens> {
    let name = &func.sig.ident;
    let cmd = name.to_string();
    let wrapper = format_ident!("__tiqr_wasm__{}", name);
    let cfgs: Vec<_> = func.attrs.iter().filter(|a| a.path().is_ident("cfg")).collect();

    let mut lets = Vec::new();
    let mut vars = Vec::new();
    for (i, input) in func.sig.inputs.iter().enumerate() {
        let FnArg::Typed(pt) = input else {
            return Err(syn::Error::new_spanned(input, "a command cannot take self"));
        };
        let var = format_ident!("__p{}", i);
        let ty = &pt.ty;
        match kind(ty) {
            Param::State => lets.push(quote! { let #var: #ty = ::tauri::Manager::state(__app); }),
            Param::App => lets.push(quote! { let #var: #ty = ::core::clone::Clone::clone(__app); }),
            Param::Arg => {
                let Pat::Ident(pi) = &*pt.pat else {
                    return Err(syn::Error::new_spanned(&pt.pat, "a command argument needs a plain name"));
                };
                let raw = pi.ident.to_string();
                let key = if keep_names { raw.trim_start_matches("r#").to_string() } else { camel(raw.trim_start_matches("r#")) };
                lets.push(quote! { let #var: #ty = ::tauri::ipc::arg(__args, #key, #cmd)?; });
            }
        }
        vars.push(var);
    }

    let reply = match &func.sig.output {
        ReturnType::Default => quote! { { let _ = __ret; ::core::result::Result::Ok(::tauri::ipc::Reply::Json(::serde_json::Value::Null)) } },
        ReturnType::Type(_, ty) => match ok_type(ty) {
            Some(ok) => {
                let good = if is_bytes(ok) { quote! { ::tauri::ipc::Reply::bytes(__v) } } else { quote! { ::tauri::ipc::Reply::json(&__v)? } };
                quote! {
                    match __ret {
                        ::core::result::Result::Ok(__v) => ::core::result::Result::Ok(#good),
                        ::core::result::Result::Err(__e) => ::core::result::Result::Err(::tauri::ipc::error_value(&__e)),
                    }
                }
            }
            None if is_bytes(ty) => quote! { ::core::result::Result::Ok(::tauri::ipc::Reply::bytes(__ret)) },
            None => quote! { ::core::result::Result::Ok(::tauri::ipc::Reply::json(&__ret)?) },
        },
    };

    let body = if func.sig.asyncness.is_some() {
        quote! {
            let __prep = (|| -> ::core::result::Result<_, ::serde_json::Value> {
                #(#lets)*
                ::core::result::Result::Ok((#(#vars,)*))
            })();
            match __prep {
                ::core::result::Result::Err(__e) => ::tauri::ipc::Pending::ready(::core::result::Result::Err(__e)),
                ::core::result::Result::Ok((#(#vars,)*)) => ::tauri::ipc::Pending::later(async move {
                    let __ret = #name(#(#vars),*).await;
                    (move || -> ::core::result::Result<::tauri::ipc::Reply, ::serde_json::Value> { #reply })()
                }),
            }
        }
    } else {
        quote! {
            let __out = (|| -> ::core::result::Result<::tauri::ipc::Reply, ::serde_json::Value> {
                #(#lets)*
                let __ret = #name(#(#vars),*);
                #reply
            })();
            ::tauri::ipc::Pending::ready(__out)
        }
    };

    Ok(quote! {
        #(#cfgs)*
        #[doc(hidden)]
        #[allow(non_snake_case, unused_variables, clippy::all)]
        pub fn #wrapper(__app: &::tauri::AppHandle, __args: &::serde_json::Value) -> ::tauri::ipc::Pending {
            #body
        }
    })
}

/// One entry of the command list: `#[cfg(..)] module::command`.
struct Entry {
    attrs: Vec<Attribute>,
    path: Path,
}

impl Parse for Entry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Entry { attrs: input.call(Attribute::parse_outer)?, path: input.parse()? })
    }
}

/// `generate_handler![..]` for the WebAssembly build: the same list Tauri's
/// takes (lib.rs's `tiqr_commands!`), turned into one function that calls a
/// command by its name - `None` for a name the list does not have here. An
/// entry's `#[cfg(..)]` stays on its line, so a command the phone build
/// leaves out is simply not there.
#[proc_macro]
pub fn generate_handler(input: TokenStream) -> TokenStream {
    let entries = parse_macro_input!(input with Punctuated::<Entry, Token![,]>::parse_terminated);
    let arms = entries.iter().map(|entry| {
        let attrs = &entry.attrs;
        let mut wrapper = entry.path.clone();
        let last = wrapper.segments.last_mut().expect("a command has a name");
        let name = last.ident.to_string();
        last.ident = format_ident!("__tiqr_wasm__{}", last.ident);
        quote! { #(#attrs)* #name => ::core::option::Option::Some(#wrapper(__app, __args)), }
    });
    quote! {
        |__app: &::tauri::AppHandle, __cmd: &str, __args: &::serde_json::Value| -> ::core::option::Option<::tauri::ipc::Pending> {
            match __cmd {
                #(#arms)*
                _ => ::core::option::Option::None,
            }
        }
    }
    .into()
}

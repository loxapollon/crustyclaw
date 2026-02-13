//! Implementation of `#[action_hook(event = "...", priority = N)]`.
//!
//! Transforms a function into a registered action hook by wrapping it
//! with metadata and generating a static registration entry.
//!
//! # Example
//!
//! ```ignore
//! #[action_hook(event = "on_message", priority = 10)]
//! fn handle_greeting(msg: &str) -> String {
//!     format!("Hello, {msg}!")
//! }
//! ```
//!
//! Expands to:
//! - The original function (unchanged)
//! - A `HookRegistration` const with metadata

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, LitInt, LitStr, Result};

struct HookAttrs {
    event: String,
    priority: u32,
}

impl HookAttrs {
    fn parse(attr: proc_macro::TokenStream) -> Result<Self> {
        let attr_ts: proc_macro2::TokenStream = attr.into();
        let parsed = syn::parse2::<HookAttrArgs>(quote! { (#attr_ts) })?;

        let mut event = String::new();
        let mut priority = 0u32;

        for meta in parsed.metas {
            match meta {
                HookMeta::Event(s) => event = s,
                HookMeta::Priority(p) => priority = p,
            }
        }

        if event.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "action_hook requires `event = \"...\"`",
            ));
        }

        Ok(HookAttrs { event, priority })
    }
}

enum HookMeta {
    Event(String),
    Priority(u32),
}

struct HookAttrArgs {
    metas: Vec<HookMeta>,
}

impl syn::parse::Parse for HookAttrArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        let mut metas = Vec::new();
        while !content.is_empty() {
            let ident: syn::Ident = content.parse()?;
            content.parse::<syn::Token![=]>()?;

            if ident == "event" {
                let lit: LitStr = content.parse()?;
                metas.push(HookMeta::Event(lit.value()));
            } else if ident == "priority" {
                let lit: LitInt = content.parse()?;
                metas.push(HookMeta::Priority(lit.base10_parse()?));
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "expected `event` or `priority`",
                ));
            }

            if content.peek(syn::Token![,]) {
                content.parse::<syn::Token![,]>()?;
            }
        }

        Ok(HookAttrArgs { metas })
    }
}

pub fn expand(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attrs = match HookAttrs::parse(attr) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let func = match syn::parse::<ItemFn>(item.clone()) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error().into(),
    };

    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let event = &attrs.event;
    let priority = attrs.priority;

    let registration_name = syn::Ident::new(
        &format!("__HOOK_REG_{}", fn_name_str.to_uppercase()),
        fn_name.span(),
    );

    let original: TokenStream = item.into();

    let expanded = quote! {
        #original

        /// Auto-generated hook registration metadata.
        const #registration_name: (&str, &str, u32) = (#fn_name_str, #event, #priority);
    };

    expanded.into()
}

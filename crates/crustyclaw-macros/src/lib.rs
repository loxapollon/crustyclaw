#![deny(unsafe_code)]

//! Procedural macros for CrustyClaw.
//!
//! This crate provides derive and attribute macros used across the CrustyClaw
//! workspace. Planned macros include:
//!
//! - `#[derive(Validate)]` — compile-time input validation from struct annotations
//! - `#[derive(Redact)]` — auto-redact sensitive fields in Debug/Display/logs
//! - `#[derive(SecureZeroize)]` — zeroize sensitive memory on Drop
//! - `#[derive(ActionPlugin)]` — Forgejo Action plugin scaffolding
//! - `#[action_hook(event, priority)]` — hook registration attribute

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for redacting sensitive fields in Debug output.
///
/// Fields annotated with `#[redact]` will display as `[REDACTED]` in the
/// generated `Debug` implementation.
///
/// # Example
///
/// ```ignore
/// use crustyclaw_macros::Redact;
///
/// #[derive(Redact)]
/// struct Credentials {
///     pub username: String,
///     #[redact]
///     pub password: String,
/// }
/// ```
#[proc_macro_derive(Redact, attributes(redact))]
pub fn derive_redact(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(name, "Redact only supports named fields")
                    .to_compile_error()
                    .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Redact can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    let field_debug: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let is_redacted = f.attrs.iter().any(|a| a.path().is_ident("redact"));

            if is_redacted {
                quote! {
                    .field(#field_name_str, &"[REDACTED]")
                }
            } else {
                quote! {
                    .field(#field_name_str, &self.#field_name)
                }
            }
        })
        .collect();

    let name_str = name.to_string();

    let expanded = quote! {
        impl ::std::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(#name_str)
                    #(#field_debug)*
                    .finish()
            }
        }
    };

    expanded.into()
}

//! Implementation of the `security_policy!{}` function-like proc macro.
//!
//! Parses a DSL for defining security policies at compile time:
//!
//! ```text
//! security_policy! {
//!     allow admin * *;
//!     allow user read config;
//!     deny user write secrets [priority = 100];
//!     deny * * * [priority = 0];
//! }
//! ```
//!
//! Expands to a `PolicyEngine` construction with compile-time validated rules.

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, Result, Token};

/// A parsed security policy block.
struct PolicyBlock {
    rules: Vec<RuleDef>,
}

/// A single rule definition from the DSL.
struct RuleDef {
    effect: Ident,    // "allow" or "deny"
    role: String,     // e.g. "admin", "*"
    action: String,   // e.g. "read", "*"
    resource: String, // e.g. "config", "*"
    priority: u32,
}

impl Parse for PolicyBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut rules = Vec::new();
        while !input.is_empty() {
            rules.push(input.parse::<RuleDef>()?);
        }
        Ok(PolicyBlock { rules })
    }
}

impl Parse for RuleDef {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse: allow|deny role action resource [priority = N];
        let effect: Ident = input.parse()?;
        if effect != "allow" && effect != "deny" {
            return Err(syn::Error::new_spanned(
                &effect,
                "expected `allow` or `deny`",
            ));
        }

        let role = parse_policy_token(input)?;
        let action = parse_policy_token(input)?;
        let resource = parse_policy_token(input)?;

        // Optional [priority = N]
        let priority = if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            let key: Ident = content.parse()?;
            if key != "priority" {
                return Err(syn::Error::new_spanned(key, "expected `priority`"));
            }
            content.parse::<Token![=]>()?;
            let lit: LitInt = content.parse()?;
            lit.base10_parse()?
        } else {
            0
        };

        // Consume trailing semicolon
        input.parse::<Token![;]>()?;

        Ok(RuleDef {
            effect,
            role,
            action,
            resource,
            priority,
        })
    }
}

/// Parse a single policy token — either an identifier or `*`.
fn parse_policy_token(input: ParseStream) -> Result<String> {
    if input.peek(Token![*]) {
        input.parse::<Token![*]>()?;
        Ok("*".to_string())
    } else {
        let ident: Ident = input.parse()?;
        Ok(ident.to_string())
    }
}

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let block = syn::parse_macro_input!(input as PolicyBlock);

    let rule_exprs: Vec<TokenStream> = block
        .rules
        .iter()
        .map(|rule| {
            let role = &rule.role;
            let action = &rule.action;
            let resource = &rule.resource;
            let priority = rule.priority;

            let constructor = if rule.effect == "allow" {
                quote! { ::crustyclaw_config::policy::PolicyRule::allow(#role, #action, #resource) }
            } else {
                quote! { ::crustyclaw_config::policy::PolicyRule::deny(#role, #action, #resource) }
            };

            quote! { #constructor.with_priority(#priority) }
        })
        .collect();

    let expanded = quote! {
        ::crustyclaw_config::policy::build_policy(
            ::std::vec![#(#rule_exprs),*]
        )
    };

    expanded.into()
}

//! Implementation of `#[derive(Redact)]`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

pub fn expand(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Redact only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Redact can only be derived for structs",
            ));
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

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::std::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(#name_str)
                    #(#field_debug)*
                    .finish()
            }
        }
    })
}

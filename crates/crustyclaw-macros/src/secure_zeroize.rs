//! Implementation of `#[derive(SecureZeroize)]`.
//!
//! Generates a `Drop` implementation that calls `zeroize::Zeroize::zeroize()`
//! on all fields except those annotated with `#[no_zeroize]`.
//!
//! The consuming crate must have `zeroize` as a dependency.

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
                    "SecureZeroize only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "SecureZeroize can only be derived for structs",
            ));
        }
    };

    let zeroize_calls: Vec<_> = fields
        .iter()
        .filter_map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let skip = f.attrs.iter().any(|a| a.path().is_ident("no_zeroize"));
            if skip {
                None
            } else {
                Some(quote! {
                    ::zeroize::Zeroize::zeroize(&mut self.#field_name);
                })
            }
        })
        .collect();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::std::ops::Drop for #name #ty_generics #where_clause {
            fn drop(&mut self) {
                #(#zeroize_calls)*
            }
        }
    })
}

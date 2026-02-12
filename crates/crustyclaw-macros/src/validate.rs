//! Implementation of `#[derive(Validate)]`.
//!
//! Parses `#[validate(...)]` attributes on struct fields and generates a
//! `validate(&self) -> Result<(), Vec<String>>` method.

use proc_macro2::TokenStream;
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::{DeriveInput, LitInt, Result};

/// Parsed validation rules for a single field.
struct FieldRules {
    field_name: syn::Ident,
    non_empty: bool,
    range_min: Option<i64>,
    range_max: Option<i64>,
    min_len: Option<usize>,
    max_len: Option<usize>,
}

impl FieldRules {
    fn parse(field: &syn::Field) -> Result<Option<Self>> {
        let field_name = field.ident.clone().unwrap();

        let mut rules = FieldRules {
            field_name,
            non_empty: false,
            range_min: None,
            range_max: None,
            min_len: None,
            max_len: None,
        };

        let mut has_validate = false;
        for attr in &field.attrs {
            if !attr.path().is_ident("validate") {
                continue;
            }
            has_validate = true;
            attr.parse_nested_meta(|meta| rules.parse_rule(meta))?;
        }

        if has_validate {
            Ok(Some(rules))
        } else {
            Ok(None)
        }
    }

    fn parse_rule(&mut self, meta: ParseNestedMeta) -> Result<()> {
        if meta.path.is_ident("non_empty") {
            self.non_empty = true;
            return Ok(());
        }

        if meta.path.is_ident("min_len") {
            let value = meta.value()?;
            let lit: LitInt = value.parse()?;
            self.min_len = Some(lit.base10_parse()?);
            return Ok(());
        }

        if meta.path.is_ident("max_len") {
            let value = meta.value()?;
            let lit: LitInt = value.parse()?;
            self.max_len = Some(lit.base10_parse()?);
            return Ok(());
        }

        if meta.path.is_ident("range") {
            meta.parse_nested_meta(|nested| {
                if nested.path.is_ident("min") {
                    let value = nested.value()?;
                    let lit: LitInt = value.parse()?;
                    self.range_min = Some(lit.base10_parse()?);
                    Ok(())
                } else if nested.path.is_ident("max") {
                    let value = nested.value()?;
                    let lit: LitInt = value.parse()?;
                    self.range_max = Some(lit.base10_parse()?);
                    Ok(())
                } else {
                    Err(nested.error("expected `min` or `max`"))
                }
            })?;
            return Ok(());
        }

        Err(meta.error("unknown validate rule; expected non_empty, range, min_len, or max_len"))
    }

    fn generate_checks(&self) -> TokenStream {
        let field_name = &self.field_name;
        let field_str = field_name.to_string();
        let mut checks = Vec::new();

        if self.non_empty {
            checks.push(quote! {
                if self.#field_name.is_empty() {
                    errors.push(format!("{}: must not be empty", #field_str));
                }
            });
        }

        if let Some(min) = self.min_len {
            checks.push(quote! {
                if self.#field_name.len() < #min {
                    errors.push(format!("{}: length must be at least {}", #field_str, #min));
                }
            });
        }

        if let Some(max) = self.max_len {
            checks.push(quote! {
                if self.#field_name.len() > #max {
                    errors.push(format!("{}: length must be at most {}", #field_str, #max));
                }
            });
        }

        if let Some(min) = self.range_min {
            checks.push(quote! {
                if (self.#field_name as i64) < #min {
                    errors.push(format!("{}: must be at least {}", #field_str, #min));
                }
            });
        }

        if let Some(max) = self.range_max {
            checks.push(quote! {
                if (self.#field_name as i64) > #max {
                    errors.push(format!("{}: must be at most {}", #field_str, #max));
                }
            });
        }

        quote! { #(#checks)* }
    }
}

pub fn expand(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Validate only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Validate can only be derived for structs",
            ));
        }
    };

    let mut all_checks = Vec::new();
    for field in fields {
        if let Some(rules) = FieldRules::parse(field)? {
            all_checks.push(rules.generate_checks());
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Validate this struct according to its field-level constraints.
            ///
            /// Returns `Ok(())` if all constraints pass, or `Err(Vec<String>)`
            /// with a list of human-readable validation error messages.
            pub fn validate(&self) -> ::std::result::Result<(), ::std::vec::Vec<::std::string::String>> {
                let mut errors = ::std::vec::Vec::new();
                #(#all_checks)*
                if errors.is_empty() {
                    ::std::result::Result::Ok(())
                } else {
                    ::std::result::Result::Err(errors)
                }
            }
        }
    })
}

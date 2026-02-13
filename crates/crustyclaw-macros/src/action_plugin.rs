//! Implementation of `#[derive(ActionPlugin)]`.
//!
//! Generates the boilerplate for a Forgejo Action plugin:
//! - Implements the `ActionPlugin` trait
//! - Parses input from environment variables or TOML
//! - Generates metadata (name, version, description) from struct-level attributes
//!
//! # Example
//!
//! ```ignore
//! #[derive(ActionPlugin)]
//! #[action(name = "greeting", version = "1.0.0", description = "Says hello")]
//! struct GreetAction {
//!     #[action_input(required)]
//!     pub name: String,
//!     #[action_input(default = "Hello")]
//!     pub greeting: String,
//! }
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::{DeriveInput, LitStr, Result};

struct PluginMeta {
    name: String,
    version: String,
    description: String,
}

impl PluginMeta {
    fn parse(input: &DeriveInput) -> Result<Self> {
        let mut meta = PluginMeta {
            name: input.ident.to_string().to_lowercase(),
            version: "0.1.0".to_string(),
            description: String::new(),
        };

        for attr in &input.attrs {
            if attr.path().is_ident("action") {
                attr.parse_nested_meta(|nested| {
                    if nested.path.is_ident("name") {
                        let value = nested.value()?;
                        let lit: LitStr = value.parse()?;
                        meta.name = lit.value();
                        Ok(())
                    } else if nested.path.is_ident("version") {
                        let value = nested.value()?;
                        let lit: LitStr = value.parse()?;
                        meta.version = lit.value();
                        Ok(())
                    } else if nested.path.is_ident("description") {
                        let value = nested.value()?;
                        let lit: LitStr = value.parse()?;
                        meta.description = lit.value();
                        Ok(())
                    } else {
                        Err(nested.error("expected `name`, `version`, or `description`"))
                    }
                })?;
            }
        }

        Ok(meta)
    }
}

struct InputField {
    field_name: syn::Ident,
    required: bool,
    default_value: Option<String>,
}

impl InputField {
    fn parse(field: &syn::Field) -> Result<Option<Self>> {
        let field_name = field.ident.clone().unwrap();
        let mut is_action_input = false;
        let mut required = false;
        let mut default_value = None;

        for attr in &field.attrs {
            if !attr.path().is_ident("action_input") {
                continue;
            }
            is_action_input = true;
            attr.parse_nested_meta(|meta: ParseNestedMeta| {
                if meta.path.is_ident("required") {
                    required = true;
                    Ok(())
                } else if meta.path.is_ident("default") {
                    let value = meta.value()?;
                    let lit: LitStr = value.parse()?;
                    default_value = Some(lit.value());
                    Ok(())
                } else {
                    Err(meta.error("expected `required` or `default`"))
                }
            })?;
        }

        if is_action_input {
            Ok(Some(InputField {
                field_name,
                required,
                default_value,
            }))
        } else {
            // Treat all fields as inputs (optional with empty default)
            Ok(Some(InputField {
                field_name,
                required: false,
                default_value: None,
            }))
        }
    }
}

pub fn expand(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let meta = PluginMeta::parse(&input)?;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "ActionPlugin only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "ActionPlugin can only be derived for structs",
            ));
        }
    };

    let mut input_fields = Vec::new();
    for field in fields {
        if let Some(input_field) = InputField::parse(field)? {
            input_fields.push(input_field);
        }
    }

    let plugin_name = &meta.name;
    let plugin_version = &meta.version;
    let plugin_description = &meta.description;

    // Generate input_names() items
    let input_name_strs: Vec<String> = input_fields
        .iter()
        .map(|f| f.field_name.to_string())
        .collect();

    // Generate from_env() field initializers
    let field_inits: Vec<TokenStream> = input_fields
        .iter()
        .map(|f| {
            let field_name = &f.field_name;
            let env_key = format!("INPUT_{}", f.field_name.to_string().to_uppercase());

            if f.required {
                quote! {
                    #field_name: ::std::env::var(#env_key)
                        .unwrap_or_else(|_| panic!("required input {} not set", #env_key))
                }
            } else if let Some(default) = &f.default_value {
                quote! {
                    #field_name: ::std::env::var(#env_key).unwrap_or_else(|_| #default.to_string())
                }
            } else {
                quote! {
                    #field_name: ::std::env::var(#env_key).unwrap_or_default()
                }
            }
        })
        .collect();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Plugin name.
            pub fn plugin_name() -> &'static str {
                #plugin_name
            }

            /// Plugin version.
            pub fn plugin_version() -> &'static str {
                #plugin_version
            }

            /// Plugin description.
            pub fn plugin_description() -> &'static str {
                #plugin_description
            }

            /// List of input parameter names.
            pub fn input_names() -> &'static [&'static str] {
                &[#(#input_name_strs),*]
            }

            /// Construct this plugin from environment variables.
            ///
            /// Environment variables are named `INPUT_<FIELD_NAME>` (uppercase).
            pub fn from_env() -> Self {
                Self {
                    #(#field_inits),*
                }
            }
        }
    })
}

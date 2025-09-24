//! # Stigmergy Derive Macros
//!
//! This crate provides derive macros for the stigmergy library, specifically for
//! automatic JSON schema generation from Rust types.
//!
//! ## JsonSchema Derive Macro
//!
//! The `JsonSchema` derive macro automatically implements the `JsonSchema` trait
//! for structs and enums, generating appropriate JSON schemas at compile time.
//!
//! ### Supported Types
//!
//! - **Structs**: Generates object schemas with properties for each field
//! - **Unit Enums**: Generates string enum schemas with all variant names
//! - **Complex Enums**: Generates `oneOf` schemas for mixed enum types
//! - **Struct Variant Enums**: Generates object schemas with named properties
//! - **Tuple Variant Enums**: Generates array schemas with typed elements
//!
//! ### Examples
//!
//! ```rust
//! use stigmergy::JsonSchema;
//! use serde_json::json;
//!
//! // Simple struct
//! #[derive(stigmergy_derive::JsonSchema)]
//! struct Point {
//!     x: f64,
//!     y: f64,
//! }
//!
//! let schema = Point::json_schema();
//! assert_eq!(schema["type"], "object");
//!
//! // Unit enum
//! #[derive(stigmergy_derive::JsonSchema)]
//! enum Status {
//!     Active,
//!     Inactive,
//!     Pending,
//! }
//!
//! let schema = Status::json_schema();
//! assert_eq!(schema["type"], "string");
//! assert_eq!(schema["enum"].as_array().unwrap().len(), 3);
//!
//! // Complex enum with mixed variants
//! #[derive(stigmergy_derive::JsonSchema)]
//! enum Shape {
//!     Circle { radius: f64 },
//!     Square { side: f64 },
//!     Point(f64, f64),
//! }
//!
//! let schema = Shape::json_schema();
//! assert!(schema["oneOf"].is_array());
//! ```
//!
//! ## Implementation Details
//!
//! The derive macro uses the `derive_util` crate to traverse Rust type structures
//! and generates appropriate JSON schema structures. For enums, it categorizes
//! variants into unit variants (simple string enums) and complex variants
//! (requiring `oneOf` patterns).

#![recursion_limit = "128"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro2::TokenStream;
use syn::{DeriveInput, parse_macro_input};

use derive_util::{EnumVisitor, StructVisitor};

/// Derive the JsonSchema trait for structs and enums.
#[proc_macro_derive(JsonSchema, attributes())]
pub fn derive_json_schema(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty_name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let schema_gen = match input.data {
        syn::Data::Struct(ref ds) => {
            let mut jsv = JsonSchemaStructVisitor;
            let (value, required) = jsv.visit_struct(&ty_name, ds);
            quote! {
                let mut result = serde_json::json!({});
                let mut properties = serde_json::json!({});
                #value
                result["required"] = serde_json::Value::Array(vec![].into());
                #required
                result["type"] = "object".into();
                result["properties"] = properties;
                result
            }
        }
        syn::Data::Enum(ref de) => {
            let mut jsv = JsonSchemaEnumVisitor::new();
            jsv.visit_enum(&ty_name, de)
        }
        syn::Data::Union(_) => {
            panic!("unions are not supported");
        }
    };

    let generated = quote! {
        impl #impl_generics stigmergy::JsonSchema for #ty_name #ty_generics #where_clause {
            fn json_schema() -> serde_json::Value {
                #schema_gen
            }
        }
    };
    generated.into()
}

///////////////////////////////////////// JsonSchemaStructVisitor ////////////////////////////////////////

struct JsonSchemaStructVisitor;

impl StructVisitor for JsonSchemaStructVisitor {
    type Output = (TokenStream, TokenStream);

    fn visit_struct_named_fields(
        &mut self,
        _ty_name: &syn::Ident,
        _ds: &syn::DataStruct,
        fields: &syn::FieldsNamed,
    ) -> Self::Output {
        let mut result = quote! {};
        let mut required = quote! {};
        for field in fields.named.iter() {
            if let Some(field_ident) = &field.ident {
                let field_ident = field_ident.to_string();
                let field_ident = if let Some(field_ident) = field_ident.strip_prefix("r#") {
                    field_ident.to_string()
                } else {
                    field_ident.clone()
                };
                let field_type = field.ty.clone();
                result = quote! {
                    #result
                    properties[#field_ident] = <#field_type as stigmergy::JsonSchema>::json_schema();
                };
                required = quote! {
                    #required
                    if let Some(serde_json::Value::Array(arr)) = result.get_mut("required") {
                        arr.push(#field_ident.into())
                    }
                };
            }
        }
        (result, required)
    }
}

///////////////////////////////////////// JsonSchemaEnumVisitor ////////////////////////////////////////

struct JsonSchemaEnumVisitor {
    unit_variants: Vec<String>,
    complex_variants: Vec<TokenStream>,
}

impl JsonSchemaEnumVisitor {
    fn new() -> Self {
        Self {
            unit_variants: Vec::new(),
            complex_variants: Vec::new(),
        }
    }

    fn generate_final_schema(
        unit_variants: &[String],
        complex_variants: &[TokenStream],
    ) -> TokenStream {
        if complex_variants.is_empty() {
            // Only unit variants - simple enum schema
            quote! {
                serde_json::json!({
                    "type": "string",
                    "enum": [#(#unit_variants),*]
                })
            }
        } else if unit_variants.is_empty() {
            // Only complex variants - oneOf with objects
            quote! {
                {
                    let mut schemas = vec![];
                    #(schemas.push(#complex_variants);)*
                    if schemas.len() == 1 {
                        schemas.into_iter().next().unwrap()
                    } else {
                        serde_json::json!({
                            "oneOf": schemas
                        })
                    }
                }
            }
        } else {
            // Mixed variants - oneOf with enum + objects
            quote! {
                {
                    let mut schemas = vec![];
                    // Add unit variants as enum schema
                    schemas.push(serde_json::json!({
                        "type": "string",
                        "enum": [#(#unit_variants),*]
                    }));
                    // Add complex variants
                    #(schemas.push(#complex_variants);)*
                    serde_json::json!({
                        "oneOf": schemas
                    })
                }
            }
        }
    }
}

impl EnumVisitor for JsonSchemaEnumVisitor {
    type Output = TokenStream;
    type VariantOutput = TokenStream;

    fn combine_variants(
        &mut self,
        _ty_name: &syn::Ident,
        _data_enum: &syn::DataEnum,
        _variants: &[Self::VariantOutput],
    ) -> Self::Output {
        // The variants slice contains individual variant schemas
        // We need to categorize them and combine into final schema
        Self::generate_final_schema(&self.unit_variants, &self.complex_variants)
    }

    fn visit_enum_variant_unit(
        &mut self,
        _ty_name: &syn::Ident,
        _data_enum: &syn::DataEnum,
        variant: &syn::Variant,
    ) -> Self::VariantOutput {
        let variant_name = variant.ident.to_string();
        self.unit_variants.push(variant_name);

        // Return empty token stream since we accumulate in self.unit_variants
        quote! {}
    }

    fn visit_enum_variant_named_field(
        &mut self,
        _ty_name: &syn::Ident,
        _data_enum: &syn::DataEnum,
        variant: &syn::Variant,
        fields: &syn::FieldsNamed,
    ) -> Self::VariantOutput {
        let variant_name = variant.ident.to_string();

        // Generate properties for struct variant
        let mut properties = quote! {};
        let mut required = quote! {};

        for field in fields.named.iter() {
            if let Some(field_ident) = &field.ident {
                let field_name = field_ident.to_string();
                let field_type = &field.ty;
                properties = quote! {
                    #properties
                    properties[#field_name] = <#field_type as stigmergy::JsonSchema>::json_schema();
                };
                required = quote! {
                    #required
                    required.push(#field_name.into());
                };
            }
        }

        let variant_schema = quote! {
            {
                let mut result = serde_json::json!({});
                let mut properties = serde_json::json!({});
                let mut required: Vec<serde_json::Value> = vec![];
                #properties
                #required
                result["type"] = "object".into();
                result["properties"] = serde_json::json!({
                    #variant_name: serde_json::json!({
                        "type": "object",
                        "properties": properties,
                        "required": required
                    })
                });
                result["required"] = serde_json::Value::Array(vec![#variant_name.into()]);
                result
            }
        };

        self.complex_variants.push(variant_schema.clone());
        variant_schema
    }

    fn visit_enum_variant_unnamed_field(
        &mut self,
        _ty_name: &syn::Ident,
        _data_enum: &syn::DataEnum,
        variant: &syn::Variant,
        fields: &syn::FieldsUnnamed,
    ) -> Self::VariantOutput {
        let variant_name = variant.ident.to_string();

        // Generate items for tuple variant
        let mut items = quote! {};

        for field in fields.unnamed.iter() {
            let field_type = &field.ty;
            items = quote! {
                #items
                items.push(<#field_type as stigmergy::JsonSchema>::json_schema());
            };
        }

        let variant_schema = quote! {
            {
                let mut items: Vec<serde_json::Value> = vec![];
                #items
                let items_schema = if items.len() == 1 {
                    items.into_iter().next().unwrap()
                } else {
                    serde_json::Value::Array(items)
                };
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        #variant_name: {
                            "type": "array",
                            "items": items_schema
                        }
                    },
                    "required": [#variant_name]
                })
            }
        };

        self.complex_variants.push(variant_schema.clone());
        variant_schema
    }
}

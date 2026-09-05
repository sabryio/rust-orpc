//! Implementation of `#[derive(OrpcErrors)]` proc macro.
//!
//! Generates an `inventory::submit!` call that registers error variant metadata
//! for TypeScript contract generation.
//!
//! # Example
//!
//! ```rust,ignore
//! #[derive(OrpcErrors)]
//! pub enum AppError {
//!     NotFound,
//!     Conflict { reason: String },
//!     DatabaseError(String),
//! }
//! ```
//!
//! Expands to:
//!
//! ```rust,ignore
//! pub enum AppError {
//!     NotFound,
//!     Conflict { reason: String },
//!     DatabaseError(String),
//! }
//!
//! inventory::submit! {
//!     ::orpc::ErrorRegistration {
//!         type_name: "AppError",
//!         variants: &[
//!             ::orpc::ErrorVariant {
//!                 name: "NOT_FOUND",
//!                 data_schema: None,
//!             },
//!             ::orpc::ErrorVariant {
//!                 name: "CONFLICT",
//!                 data_schema: Some("z.object({ reason: z.string() })"),
//!             },
//!             ::orpc::ErrorVariant {
//!                 name: "DATABASE_ERROR",
//!                 data_schema: Some("z.string()"),
//!             },
//!         ],
//!     }
//! }
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result, Type};

/// Generate the `#[derive(OrpcErrors)]` expansion.
pub fn expand_orpc_errors(input: DeriveInput) -> Result<TokenStream> {
    let enum_name = &input.ident;
    let enum_name_str = enum_name.to_string();

    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(OrpcErrors)] can only be used on enums",
            ))
        }
    };

    let mut variant_registrations = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;
        let variant_name_screaming = to_screaming_snake_case(&variant_name.to_string());

        let data_schema = match &variant.fields {
            Fields::Unit => {
                // Unit variant: NO_DATA: {}
                quote! { None }
            }
            Fields::Unnamed(fields) => {
                // Tuple variant
                if fields.unnamed.len() == 1 {
                    // Single field tuple: DatabaseError(String) → z.string()
                    let field_ty = &fields.unnamed[0].ty;
                    let schema = generate_zod_schema_for_type(field_ty);
                    quote! { Some(#schema) }
                } else {
                    // Multiple field tuple: wrap in z.tuple([...])
                    let schemas: Vec<_> = fields
                        .unnamed
                        .iter()
                        .map(|f| generate_zod_schema_for_type(&f.ty))
                        .collect();
                    let tuple_schema = format!("z.tuple([{}])", schemas.join(", "));
                    quote! { Some(#tuple_schema) }
                }
            }
            Fields::Named(fields) => {
                // Struct variant: Conflict { reason: String } → z.object({ reason: z.string() })
                let field_schemas: Vec<String> = fields
                    .named
                    .iter()
                    .map(|f| {
                        let field_name = f.ident.as_ref().unwrap().to_string();
                        let field_ty = &f.ty;
                        let schema = generate_zod_schema_for_type(field_ty);
                        format!("{}: {}", field_name, schema)
                    })
                    .collect();
                let object_schema = format!("z.object({{ {} }})", field_schemas.join(", "));
                quote! { Some(#object_schema) }
            }
        };

        variant_registrations.push(quote! {
            ::orpc::ErrorVariant {
                name: #variant_name_screaming,
                data_schema: #data_schema,
            }
        });
    }

    Ok(quote! {
        // Original enum is unchanged
        const _: () = {
            ::orpc::inventory::submit! {
                ::orpc::ErrorRegistration {
                    type_name: #enum_name_str,
                    variants: &[
                        #(#variant_registrations),*
                    ],
                }
            }
        };
    })
}

/// Convert a name to SCREAMING_SNAKE_CASE.
///
/// Examples:
/// - "NotFound" → "NOT_FOUND"
/// - "DatabaseError" → "DATABASE_ERROR"
/// - "RateLimitExceeded" → "RATE_LIMIT_EXCEEDED"
fn to_screaming_snake_case(name: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lower = false;

    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && prev_is_lower {
                result.push('_');
            }
            result.push(ch);
            prev_is_lower = false;
        } else {
            result.push(ch.to_ascii_uppercase());
            prev_is_lower = true;
        }
    }

    result
}

/// Generate a Zod schema string for a Rust type.
///
/// Maps common Rust types to their Zod equivalents:
/// - String → "z.string()"
/// - i32, u32, etc. → "z.number()"
/// - bool → "z.boolean()"
/// - Option<T> → "<T_schema>.optional()"
/// - Vec<T> → "z.array(<T_schema>)"
/// - Custom types → "<TypeName>Schema" (assumes ZodTs derive)
fn generate_zod_schema_for_type(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => {
            let last_segment = type_path.path.segments.last().unwrap();
            let type_name = last_segment.ident.to_string();

            match type_name.as_str() {
                "String" | "str" => "z.string()".to_string(),
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64"
                | "isize" | "usize" => "z.number()".to_string(),
                "bool" => "z.boolean()".to_string(),
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            let inner_schema = generate_zod_schema_for_type(inner_ty);
                            return format!("{}.optional()", inner_schema);
                        }
                    }
                    "z.unknown()".to_string()
                }
                "Vec" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            let inner_schema = generate_zod_schema_for_type(inner_ty);
                            return format!("z.array({})", inner_schema);
                        }
                    }
                    "z.array(z.unknown())".to_string()
                }
                _ => {
                    // Custom type — assume it has a ZodTs schema
                    format!("{}Schema", type_name)
                }
            }
        }
        _ => "z.unknown()".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_screaming_snake_case() {
        assert_eq!(to_screaming_snake_case("NotFound"), "NOT_FOUND");
        assert_eq!(to_screaming_snake_case("DatabaseError"), "DATABASE_ERROR");
        assert_eq!(
            to_screaming_snake_case("RateLimitExceeded"),
            "RATE_LIMIT_EXCEEDED"
        );
        assert_eq!(to_screaming_snake_case("OK"), "OK");
    }

    #[test]
    fn test_generate_zod_schema_primitives() {
        let string_ty: Type = syn::parse_str("String").unwrap();
        assert_eq!(generate_zod_schema_for_type(&string_ty), "z.string()");

        let i32_ty: Type = syn::parse_str("i32").unwrap();
        assert_eq!(generate_zod_schema_for_type(&i32_ty), "z.number()");

        let bool_ty: Type = syn::parse_str("bool").unwrap();
        assert_eq!(generate_zod_schema_for_type(&bool_ty), "z.boolean()");
    }

    #[test]
    fn test_generate_zod_schema_option() {
        let option_ty: Type = syn::parse_str("Option<String>").unwrap();
        assert_eq!(
            generate_zod_schema_for_type(&option_ty),
            "z.string().optional()"
        );
    }

    #[test]
    fn test_generate_zod_schema_vec() {
        let vec_ty: Type = syn::parse_str("Vec<String>").unwrap();
        assert_eq!(
            generate_zod_schema_for_type(&vec_ty),
            "z.array(z.string())"
        );
    }

    #[test]
    fn test_generate_zod_schema_custom() {
        let custom_ty: Type = syn::parse_str("Planet").unwrap();
        assert_eq!(generate_zod_schema_for_type(&custom_ty), "PlanetSchema");
    }
}

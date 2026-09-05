//! Code generation for `#[derive(ZodTs)]`.
//!
//! Generates a `fn zod_ts() -> String` method that returns a complete
//! TypeScript block with a Zod schema and a `z.infer` type alias,
//! plus an `inventory::submit!` for `SchemaRegistration`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::{
    attributes::{ZodAttrs, apply_rename_rule, parse_serde_attrs, parse_zod_attrs},
    errors::Result,
    types::{OPTION, VEC, is_primitive, try_extract_wrapper},
};

const ZOD_IMPORT: &str = "import * as z from \"zod\";";

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generate the `#[derive(ZodTs)]` expansion.
pub fn derive_zod_ts(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let name_str = name.to_string();

    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => expand_named_struct(name, &name_str, fields, &input),
            Fields::Unnamed(_) | Fields::Unit => Err(syn::Error::new_spanned(
                name,
                "ZodTs: only structs with named fields are supported",
            )
            .into()),
        },
        Data::Enum(data) => expand_enum(name, &name_str, data, &input),
        Data::Union(_) => {
            Err(syn::Error::new_spanned(name, "ZodTs cannot be derived for unions").into())
        }
    }
}

// ---------------------------------------------------------------------------
// Struct expansion
// ---------------------------------------------------------------------------

fn expand_named_struct(
    name: &syn::Ident,
    name_str: &str,
    fields: &syn::FieldsNamed,
    _input: &DeriveInput,
) -> Result<TokenStream> {
    let mut field_lines: Vec<String> = Vec::new();
    let mut dep_type_names: Vec<String> = Vec::new();

    for field in &fields.named {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let serde = parse_serde_attrs(&field.attrs)?;

        if serde.skip {
            continue;
        }

        let ts_key = serde.rename.as_deref().unwrap_or(&field_name);
        let zod = parse_zod_attrs(&field.attrs)?;
        let is_opt = is_option_type(&field.ty);

        let base_ty = if is_opt {
            option_inner(&field.ty).unwrap_or(&field.ty)
        } else {
            &field.ty
        };

        let zod_expr = rust_type_to_zod(base_ty, &zod);

        let final_expr = if is_opt {
            format!("{}.optional()", zod_expr)
        } else {
            zod_expr
        };

        field_lines.push(format!("  {}: {}", ts_key, final_expr));

        // Collect non-primitive custom types for dependency tracking
        if let Some(custom) = innermost_custom_name(base_ty) {
            dep_type_names.push(custom);
        }
    }

    let schema_name = format!("{}Schema", name_str);
    let ts_code = format!(
        "{}\n\nexport const {} = z.object({{\n{}\n}});\n\nexport type {} = z.infer<typeof {}>;",
        ZOD_IMPORT,
        schema_name,
        field_lines.join(",\n"),
        name_str,
        schema_name,
    );

    Ok(emit_registration(name, name_str, &ts_code, &dep_type_names))
}

// ---------------------------------------------------------------------------
// Enum expansion
// ---------------------------------------------------------------------------

fn expand_enum(
    name: &syn::Ident,
    name_str: &str,
    data: &syn::DataEnum,
    input: &DeriveInput,
) -> Result<TokenStream> {
    let serde_container = parse_serde_attrs(&input.attrs)?;
    let rename_all = serde_container.rename_all.as_deref();

    let mut variant_schemas: Vec<String> = Vec::new();

    for variant in &data.variants {
        let serde_variant = parse_serde_attrs(&variant.attrs)?;
        if serde_variant.skip {
            continue;
        }

        let raw_name = variant.ident.to_string();
        let variant_name = serde_variant
            .rename
            .as_deref()
            .map(str::to_string)
            .unwrap_or_else(|| {
                rename_all
                    .map(|rule| apply_rename_rule(rule, &raw_name))
                    .unwrap_or(raw_name)
            });

        variant_schemas.push(generate_variant_ts(&variant_name, &variant.fields)?);
    }

    let schema_name = format!("{}Schema", name_str);
    let variants_str = variant_schemas.join(",\n  ");
    let ts_code = format!(
        "{}\n\nexport const {} = z.union([\n  {}\n]);\n\nexport type {} = z.infer<typeof {}>;",
        ZOD_IMPORT, schema_name, variants_str, name_str, schema_name,
    );

    Ok(emit_registration(name, name_str, &ts_code, &[]))
}

// ---------------------------------------------------------------------------
// Variant code generation
// ---------------------------------------------------------------------------

fn generate_variant_ts(variant_name: &str, fields: &Fields) -> Result<String> {
    match fields {
        Fields::Unit => Ok(format!("z.literal(\"{}\")", escape_str(variant_name))),

        Fields::Unnamed(fields_unnamed) => {
            let count = fields_unnamed.unnamed.len();
            if count == 1 {
                let field = fields_unnamed.unnamed.first().unwrap();
                let zod = parse_zod_attrs(&field.attrs)?;
                let schema = rust_type_to_zod(&field.ty, &zod);
                Ok(format!(
                    "z.object({{ {}: {} }})",
                    ts_object_key(variant_name),
                    schema
                ))
            } else {
                let elements: Vec<String> = fields_unnamed
                    .unnamed
                    .iter()
                    .map(|f| {
                        let zod = parse_zod_attrs(&f.attrs)?;
                        Ok(rust_type_to_zod(&f.ty, &zod))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(format!(
                    "z.object({{ {}: z.tuple([{}]) }})",
                    ts_object_key(variant_name),
                    elements.join(", ")
                ))
            }
        }

        Fields::Named(fields_named) => {
            let field_schemas: Vec<String> = fields_named
                .named
                .iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    let serde = parse_serde_attrs(&field.attrs)?;
                    if serde.skip {
                        return Ok(String::new());
                    }
                    let ts_key = serde.rename.as_deref().unwrap_or(&field_name);
                    let zod_attrs = parse_zod_attrs(&field.attrs)?;
                    let is_opt = is_option_type(&field.ty);
                    let base_ty = if is_opt {
                        option_inner(&field.ty).unwrap_or(&field.ty)
                    } else {
                        &field.ty
                    };
                    let schema = rust_type_to_zod(base_ty, &zod_attrs);
                    let final_schema = if is_opt {
                        format!("{}.optional()", schema)
                    } else {
                        schema
                    };
                    Ok(format!("{}: {}", ts_key, final_schema))
                })
                .filter(|r| r.as_deref().map(|s| !s.is_empty()).unwrap_or(true))
                .collect::<Result<Vec<_>>>()?;

            Ok(format!(
                "z.object({{ {}: z.object({{ {} }}) }})",
                ts_object_key(variant_name),
                field_schemas.join(", ")
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// inventory::submit! emission
// ---------------------------------------------------------------------------

fn emit_registration(
    name: &syn::Ident,
    name_str: &str,
    ts_code: &str,
    dep_type_names: &[String],
) -> TokenStream {
    let dep_strs: Vec<&str> = dep_type_names.iter().map(String::as_str).collect();

    quote! {
        impl #name {
            pub fn zod_ts() -> String {
                #ts_code.to_string()
            }

            pub fn dependent_types() -> Vec<&'static str> {
                vec![#(#dep_strs),*]
            }
        }

        const _: () = {
            ::orpc::inventory::submit! {
                ::orpc::SchemaRegistration {
                    type_name: #name_str,
                    zod_ts: #name::zod_ts,
                    dependent_types: #name::dependent_types,
                }
            }
        };
    }
}

// ---------------------------------------------------------------------------
// Type → Zod expression
// ---------------------------------------------------------------------------

/// Map a `syn::Type` to a Zod schema expression string.
///
/// Uses AST-based wrapper detection for `Option<T>` and `Vec<T>` —
/// never string prefix matching.
pub fn rust_type_to_zod(ty: &syn::Type, attrs: &ZodAttrs) -> String {
    // Option<T> — recurse on inner, then .optional()
    if is_option_type(ty)
        && let Some(inner) = option_inner(ty)
    {
        let inner_schema = rust_type_to_zod(inner, &ZodAttrs::default());
        return format!("{}.optional()", inner_schema);
    }

    // Vec<T>
    if let Some(m) = try_extract_wrapper(ty, VEC)
        && let Some(inner) = m.first_type()
    {
        let inner_schema = rust_type_to_zod(inner, &ZodAttrs::default());
        let mut chain = format!("z.array({})", inner_schema);
        if let Some(n) = attrs.length {
            chain.push_str(&format!(".length({})", n));
        }
        if let Some(n) = attrs.min_length {
            chain.push_str(&format!(".min({})", n));
        }
        if let Some(n) = attrs.max_length {
            chain.push_str(&format!(".max({})", n));
        }
        return chain;
    }

    // Primitives — match on the final path segment ident
    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        let name = seg.ident.to_string();
        return match name.as_str() {
            "String" | "str" => build_string_schema(attrs),
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" => build_integer_schema(attrs),
            "f32" | "f64" => build_float_schema(attrs),
            "bool" => "z.boolean()".to_string(),
            // serde_json::Value → z.any()
            "Value" => "z.any()".to_string(),
            // Custom type — reference its schema by name
            other => format!("{}Schema", other),
        };
    }

    // Unit type ()
    if let syn::Type::Tuple(t) = ty
        && t.elems.is_empty()
    {
        return "z.void()".to_string();
    }

    "z.unknown()".to_string()
}

// ---------------------------------------------------------------------------
// Schema builders
// ---------------------------------------------------------------------------

fn build_string_schema(attrs: &ZodAttrs) -> String {
    let mut chain = String::from("z.string()");
    if let Some(n) = attrs.length {
        chain.push_str(&format!(".length({})", n));
    }
    if let Some(n) = attrs.min_length {
        chain.push_str(&format!(".min({})", n));
    }
    if let Some(n) = attrs.max_length {
        chain.push_str(&format!(".max({})", n));
    }
    if attrs.email {
        chain.push_str(".email()");
    }
    if attrs.url {
        chain.push_str(".url()");
    }
    if let Some(ref p) = attrs.regex {
        chain.push_str(&format!(".regex(/{}/)", p));
    }
    if let Some(ref p) = attrs.starts_with {
        chain.push_str(&format!(".startsWith(\"{}\")", p));
    }
    if let Some(ref p) = attrs.ends_with {
        chain.push_str(&format!(".endsWith(\"{}\")", p));
    }
    if let Some(ref p) = attrs.includes {
        chain.push_str(&format!(".includes(\"{}\")", p));
    }
    chain
}

fn build_integer_schema(attrs: &ZodAttrs) -> String {
    let mut chain = String::from("z.number().int()");
    append_number_validators(&mut chain, attrs);
    chain
}

fn build_float_schema(attrs: &ZodAttrs) -> String {
    let mut chain = String::from("z.number()");
    if attrs.int {
        chain.push_str(".int()");
    }
    append_number_validators(&mut chain, attrs);
    chain
}

fn append_number_validators(chain: &mut String, attrs: &ZodAttrs) {
    if let Some(n) = attrs.min {
        chain.push_str(&format!(".min({})", n));
    }
    if let Some(n) = attrs.max {
        chain.push_str(&format!(".max({})", n));
    }
    if attrs.positive {
        chain.push_str(".positive()");
    }
    if attrs.negative {
        chain.push_str(".negative()");
    }
    if attrs.nonnegative {
        chain.push_str(".nonnegative()");
    }
    if attrs.nonpositive {
        chain.push_str(".nonpositive()");
    }
    if attrs.finite {
        chain.push_str(".finite()");
    }
}

// ---------------------------------------------------------------------------
// Type helpers — all AST-based, no string matching on type names
// ---------------------------------------------------------------------------

fn is_option_type(ty: &syn::Type) -> bool {
    try_extract_wrapper(ty, OPTION).is_some()
}

fn option_inner(ty: &syn::Type) -> Option<&syn::Type> {
    try_extract_wrapper(ty, OPTION)?.first_type()
}

/// Return the simple name of the innermost non-primitive, non-wrapper type,
/// for dependency tracking in `dependent_types()`.
fn innermost_custom_name(ty: &syn::Type) -> Option<String> {
    // Strip Vec<T>
    if let Some(m) = try_extract_wrapper(ty, VEC) {
        return m.first_type().and_then(innermost_custom_name);
    }
    if is_primitive(ty) {
        return None;
    }
    if let syn::Type::Path(tp) = ty
        && let Some(seg) = tp.path.segments.last()
    {
        let name = seg.ident.to_string();
        // Exclude Value (serde_json) from dependency tracking
        if name == "Value" {
            return None;
        }
        return Some(name);
    }
    None
}

fn ts_object_key(name: &str) -> String {
    let valid = !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if valid {
        name.to_string()
    } else {
        format!("\"{}\"", escape_str(name))
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

//! `#[derive(ZodTs)]` — generates a `fn zod_ts() -> String` method on structs and enums.
//!
//! The generated method returns a complete TypeScript block:
//!
//! ```typescript
//! import * as z from "zod";
//!
//! export const PlanetSchema = z.object({
//!   id: z.number().int(),
//!   name: z.string(),
//!   description: z.string().optional(),
//! });
//!
//! export type Planet = z.infer<typeof PlanetSchema>;
//! ```
//!
//! Usage in orpc handlers — no manual registration needed:
//!
//! ```rust,ignore
//! use orpc::ZodTs;
//!
//! #[derive(Serialize, Deserialize, ZodTs)]
//! pub struct Planet {
//!     pub id: i32,
//!     pub name: String,
//!     pub description: Option<String>,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Meta};

const ZOD_IMPORT: &str = "import * as z from \"zod\";";

pub fn derive_zod_ts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => {
                let field_schemas: Vec<String> = fields
                    .named
                    .iter()
                    .map(|field| {
                        let field_name = field.ident.as_ref().unwrap().to_string();
                        let field_type = &field.ty;
                        let attrs = parse_zod_attributes(&field.attrs);
                        let is_optional = is_option_type(field_type);
                        let base_type = if is_optional {
                            get_option_inner_type_str(field_type)
                        } else {
                            type_to_string(field_type)
                        };
                        let zod_type = rust_type_to_zod(&base_type, &attrs);
                        let final_type = if is_optional {
                            format!("{}.optional()", zod_type)
                        } else {
                            zod_type
                        };
                        format!("  {}: {}", field_name, final_type)
                    })
                    .collect();

                let fields_str = field_schemas.join(",\n");
                let schema_name = format!("{}Schema", name_str);
                let ts_code = format!(
                    "{}\n\nexport const {} = z.object({{\n{}\n}});\n\nexport type {} = z.infer<typeof {}>;",
                    ZOD_IMPORT, schema_name, fields_str, name_str, schema_name
                );

                // Collect names of nested custom types (non-primitives)
                // so generate_contract() can emit their schemas too
                let dep_types: Vec<String> = fields
                    .named
                    .iter()
                    .filter_map(|field| {
                        let is_optional = is_option_type(&field.ty);
                        let base = if is_optional {
                            get_option_inner_type_str(&field.ty)
                        } else {
                            type_to_string(&field.ty)
                        };
                        // Strip Vec<T> wrapper
                        let base = if base.starts_with("Vec<") && base.ends_with('>') {
                            base[4..base.len() - 1].to_string()
                        } else {
                            base
                        };
                        // Keep only non-primitives
                        match base.as_str() {
                            "String" | "str" | "bool" | "i8" | "i16" | "i32" | "i64" | "i128"
                            | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "f32"
                            | "f64" => None,
                            t => Some(t.to_string()),
                        }
                    })
                    .collect();

                let expanded = quote! {
                    impl #name {
                        pub fn zod_ts() -> String {
                            #ts_code.to_string()
                        }

                        /// Returns the names of nested custom types this struct depends on.
                        /// Used by `generate_contract()` to ensure all referenced schemas
                        /// are emitted in the TypeScript output.
                        pub fn dependent_types() -> Vec<&'static str> {
                            vec![#(#dep_types),*]
                        }
                    }
                };
                TokenStream::from(expanded)
            }
            _ => TokenStream::from(
                syn::Error::new_spanned(&input, "ZodTs: only structs with named fields supported")
                    .to_compile_error(),
            ),
        },

        Data::Enum(data_enum) => {
            let rename_all = parse_serde_name(&input.attrs, "rename_all");
            let variant_schemas: Vec<String> = data_enum
                .variants
                .iter()
                .map(|variant| {
                    let variant_name = effective_variant_name(
                        &variant.ident,
                        &variant.attrs,
                        rename_all.as_deref(),
                    );
                    generate_variant_ts(&variant_name, &variant.fields)
                })
                .collect();

            let variants_str = variant_schemas.join(",\n  ");
            let schema_name = format!("{}Schema", name_str);
            let ts_code = format!(
                "{}\n\nexport const {} = z.union([\n  {}\n]);\n\nexport type {} = z.infer<typeof {}>;",
                ZOD_IMPORT, schema_name, variants_str, name_str, schema_name
            );

            let expanded = quote! {
                impl #name {
                    pub fn zod_ts() -> String {
                        #ts_code.to_string()
                    }

                    pub fn dependent_types() -> Vec<&'static str> {
                        vec![]
                    }
                }
            };
            TokenStream::from(expanded)
        }

        Data::Union(_) => TokenStream::from(
            syn::Error::new_spanned(&input, "ZodTs cannot be derived for Rust unions")
                .to_compile_error(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Variant helpers
// ---------------------------------------------------------------------------

fn effective_variant_name(
    ident: &syn::Ident,
    attrs: &[Attribute],
    rename_all: Option<&str>,
) -> String {
    if let Some(renamed) = parse_serde_name(attrs, "rename") {
        return renamed;
    }
    let name = ident.to_string();
    match rename_all {
        Some(rule) => apply_rename_rule(rule, &name),
        None => name,
    }
}

fn generate_variant_ts(variant_name: &str, fields: &Fields) -> String {
    match fields {
        Fields::Unit => format!(
            "z.literal(\"{}\")",
            variant_name.replace('\\', "\\\\").replace('"', "\\\"")
        ),

        Fields::Unnamed(fields_unnamed) => {
            let count = fields_unnamed.unnamed.len();
            if count == 1 {
                let field = fields_unnamed.unnamed.first().unwrap();
                let zod = rust_type_to_zod(
                    &type_to_string(&field.ty),
                    &parse_zod_attributes(&field.attrs),
                );
                format!("z.object({{ {}: {} }})", ts_object_key(variant_name), zod)
            } else {
                let elements: Vec<String> = fields_unnamed
                    .unnamed
                    .iter()
                    .map(|f| {
                        rust_type_to_zod(&type_to_string(&f.ty), &parse_zod_attributes(&f.attrs))
                    })
                    .collect();
                format!(
                    "z.object({{ {}: z.tuple([{}]) }})",
                    ts_object_key(variant_name),
                    elements.join(", ")
                )
            }
        }

        Fields::Named(fields_named) => {
            let field_schemas: Vec<String> = fields_named
                .named
                .iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    let is_optional = is_option_type(&field.ty);
                    let base = if is_optional {
                        get_option_inner_type_str(&field.ty)
                    } else {
                        type_to_string(&field.ty)
                    };
                    let zod = rust_type_to_zod(&base, &parse_zod_attributes(&field.attrs));
                    let final_zod = if is_optional {
                        format!("{}.optional()", zod)
                    } else {
                        zod
                    };
                    format!("{}: {}", field_name, final_zod)
                })
                .collect();
            format!(
                "z.object({{ {}: z.object({{ {} }}) }})",
                ts_object_key(variant_name),
                field_schemas.join(", ")
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Serde attribute parsing
// ---------------------------------------------------------------------------

fn parse_serde_name(attrs: &[Attribute], key: &str) -> Option<String> {
    let mut result = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(key) {
                if meta.input.peek(syn::Token![=]) {
                    let lit: syn::LitStr = meta.value()?.parse()?;
                    result = Some(lit.value());
                } else if meta.input.peek(syn::token::Paren) {
                    let mut serialize = None;
                    let mut deserialize = None;
                    meta.parse_nested_meta(|inner| {
                        let lit: syn::LitStr = inner.value()?.parse()?;
                        if inner.path.is_ident("deserialize") {
                            deserialize = Some(lit.value());
                        } else if inner.path.is_ident("serialize") {
                            serialize = Some(lit.value());
                        }
                        Ok(())
                    })?;
                    if let Some(name) = deserialize.or(serialize) {
                        result = Some(name);
                    }
                }
            } else {
                skip_serde_meta_value(&meta)?;
            }
            Ok(())
        });
    }
    result
}

fn skip_serde_meta_value(meta: &syn::meta::ParseNestedMeta) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        let _: syn::Expr = meta.value()?.parse()?;
    } else if meta.input.peek(syn::token::Paren) {
        meta.parse_nested_meta(|inner| skip_serde_meta_value(&inner))?;
    }
    Ok(())
}

fn apply_rename_rule(rule: &str, variant: &str) -> String {
    match rule {
        "lowercase" => variant.to_ascii_lowercase(),
        "UPPERCASE" => variant.to_ascii_uppercase(),
        "camelCase" => {
            let mut chars = variant.chars();
            match chars.next() {
                Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        }
        "snake_case" => {
            let mut snake = String::new();
            for (i, ch) in variant.char_indices() {
                if i > 0 && ch.is_uppercase() {
                    snake.push('_');
                }
                snake.push(ch.to_ascii_lowercase());
            }
            snake
        }
        "SCREAMING_SNAKE_CASE" => apply_rename_rule("snake_case", variant).to_ascii_uppercase(),
        "kebab-case" => apply_rename_rule("snake_case", variant).replace('_', "-"),
        "SCREAMING-KEBAB-CASE" => {
            apply_rename_rule("SCREAMING_SNAKE_CASE", variant).replace('_', "-")
        }
        _ => variant.to_string(),
    }
}

fn ts_object_key(name: &str) -> String {
    let is_valid = !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if is_valid {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

// ---------------------------------------------------------------------------
// Zod attribute parsing
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ZodAttributes {
    min: Option<f64>,
    max: Option<f64>,
    length: Option<usize>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    starts_with: Option<String>,
    ends_with: Option<String>,
    includes: Option<String>,
    email: bool,
    url: bool,
    regex: Option<String>,
    positive: bool,
    negative: bool,
    nonnegative: bool,
    nonpositive: bool,
    int: bool,
    finite: bool,
}

fn parse_zod_attributes(attrs: &[Attribute]) -> ZodAttributes {
    let mut zod = ZodAttributes::default();
    for attr in attrs {
        if attr.path().is_ident("zod") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens: Vec<_> = meta_list.tokens.clone().into_iter().collect();
                let mut i = 0;
                while i < tokens.len() {
                    let t = tokens[i].to_string();
                    match t.as_str() {
                        "min_length" => {
                            if let Some(v) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_number_from_parens(&t.to_string()))
                            {
                                zod.min_length = Some(v);
                                i += 1;
                            }
                        }
                        "max_length" => {
                            if let Some(v) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_number_from_parens(&t.to_string()))
                            {
                                zod.max_length = Some(v);
                                i += 1;
                            }
                        }
                        "length" => {
                            if let Some(v) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_number_from_parens(&t.to_string()))
                            {
                                zod.length = Some(v);
                                i += 1;
                            }
                        }
                        "min" => {
                            if let Some(s) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_string_from_parens(&t.to_string()))
                            {
                                if let Ok(v) = s.parse::<f64>() {
                                    zod.min = Some(v);
                                }
                                i += 1;
                            }
                        }
                        "max" => {
                            if let Some(s) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_string_from_parens(&t.to_string()))
                            {
                                if let Ok(v) = s.parse::<f64>() {
                                    zod.max = Some(v);
                                }
                                i += 1;
                            }
                        }
                        "starts_with" => {
                            if let Some(v) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_string_from_parens(&t.to_string()))
                            {
                                zod.starts_with = Some(strip_quotes(&v));
                                i += 1;
                            }
                        }
                        "ends_with" => {
                            if let Some(v) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_string_from_parens(&t.to_string()))
                            {
                                zod.ends_with = Some(strip_quotes(&v));
                                i += 1;
                            }
                        }
                        "includes" => {
                            if let Some(v) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_string_from_parens(&t.to_string()))
                            {
                                zod.includes = Some(strip_quotes(&v));
                                i += 1;
                            }
                        }
                        "regex" => {
                            if let Some(v) = tokens
                                .get(i + 1)
                                .and_then(|t| extract_string_from_parens(&t.to_string()))
                            {
                                zod.regex = Some(strip_quotes(&v));
                                i += 1;
                            }
                        }
                        "email" => zod.email = true,
                        "url" => zod.url = true,
                        "positive" => zod.positive = true,
                        "negative" => zod.negative = true,
                        "nonnegative" => zod.nonnegative = true,
                        "nonpositive" => zod.nonpositive = true,
                        "int" => zod.int = true,
                        "finite" => zod.finite = true,
                        _ => {}
                    }
                    i += 1;
                }
            }
        }
    }
    zod
}

// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

fn rust_type_to_zod(rust_type: &str, attrs: &ZodAttributes) -> String {
    match rust_type {
        "String" | "&str" | "str" => {
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
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "usize" => {
            let mut chain = String::from("z.number().int()");
            append_number_validators(&mut chain, attrs);
            chain
        }
        "f32" | "f64" => {
            let mut chain = String::from("z.number()");
            if attrs.int {
                chain.push_str(".int()");
            }
            append_number_validators(&mut chain, attrs);
            chain
        }
        "bool" => String::from("z.boolean()"),
        other => {
            if other.starts_with("Vec<") {
                let inner = other
                    .strip_prefix("Vec<")
                    .and_then(|s| s.strip_suffix('>'))
                    .unwrap_or("unknown");
                let inner_zod = rust_type_to_zod(inner, &ZodAttributes::default());
                let mut chain = format!("z.array({})", inner_zod);
                if let Some(n) = attrs.length {
                    chain.push_str(&format!(".length({})", n));
                }
                if let Some(n) = attrs.min_length {
                    chain.push_str(&format!(".min({})", n));
                }
                if let Some(n) = attrs.max_length {
                    chain.push_str(&format!(".max({})", n));
                }
                chain
            } else {
                format!("{}Schema", other)
            }
        }
    }
}

fn append_number_validators(chain: &mut String, attrs: &ZodAttributes) {
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

fn type_to_string(ty: &syn::Type) -> String {
    if let syn::Type::Path(type_path) = ty {
        let segments: Vec<String> = type_path
            .path
            .segments
            .iter()
            .map(|seg| {
                let ident = seg.ident.to_string();
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    let args_str: Vec<String> = args
                        .args
                        .iter()
                        .filter_map(|arg| {
                            if let syn::GenericArgument::Type(t) = arg {
                                Some(type_to_string(t))
                            } else {
                                None
                            }
                        })
                        .collect();
                    if args_str.is_empty() {
                        ident
                    } else {
                        format!("{}<{}>", ident, args_str.join(", "))
                    }
                } else {
                    ident
                }
            })
            .collect();
        segments.join("::")
    } else {
        "unknown".to_string()
    }
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn get_option_inner_type_str(ty: &syn::Type) -> String {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return type_to_string(inner);
                    }
                }
            }
        }
    }
    "unknown".to_string()
}

fn extract_number_from_parens(token: &str) -> Option<usize> {
    token
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .and_then(|inner| inner.parse::<usize>().ok())
}

fn extract_string_from_parens(token: &str) -> Option<String> {
    token
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .map(|s| s.to_string())
}

fn strip_quotes(value: &str) -> String {
    if let Some(inner) = value.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return inner.to_string();
    }
    if let Some(inner) = value.strip_prefix("r\"").and_then(|s| s.strip_suffix('"')) {
        return inner.to_string();
    }
    value.to_string()
}

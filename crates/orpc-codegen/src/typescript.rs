//! TypeScript import and Zod schema generation.

use crate::{HandlerInfo, SchemaEntry};
use std::collections::BTreeSet;

/// Generate standard TypeScript import block.
pub fn generate_imports() -> String {
    [
        r#"import { z } from "zod";"#,
        r#"import { oc } from "@orpc/contract";"#,
        r#"import { openapi } from "@orpc/openapi";"#,
        r#"import { asyncIteratorObject } from "@orpc/contract";"#,
    ]
    .join("\n")
}

/// Generate TypeScript from real `ZodTs::zod_ts()` output.
///
/// Each `SchemaEntry.zod_ts` starts with `import * as z from "zod";` — we strip
/// that line from each block since `generate_imports()` already emits it once.
pub fn generate_real_schemas(schemas: &[SchemaEntry]) -> String {
    schemas
        .iter()
        .map(|s| {
            // Strip the redundant import line that ZodTs emits per-type
            s.zod_ts
                .lines()
                .filter(|line| !line.trim_start().starts_with("import "))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Fallback: generate placeholder schemas from handler type names.
///
/// Used when no `SchemaRegistration` entries are present (types don't implement
/// `ZodTs`). Emits `z.unknown()` placeholders with a TODO comment.
pub fn generate_placeholder_schemas(handlers: &[HandlerInfo]) -> String {
    let mut lines = Vec::new();

    let unique_types: BTreeSet<&str> = handlers
        .iter()
        .flat_map(|h| [h.input_type_name, h.output_type_name])
        .filter(|t| !is_primitive(t))
        .collect();

    if unique_types.is_empty() {
        return String::new();
    }

    lines.push(
        "// ⚠️  Placeholder schemas — add #[derive(ZodTs)] to your types for real schemas"
            .to_string(),
    );
    lines.push(String::new());

    for type_name in unique_types {
        let schema_name = to_schema_name(type_name);
        let base_name = base_type_name(type_name);
        lines.push(format!(
            "// TODO: add #[derive(ZodTs)] to {base_name} in Rust"
        ));
        lines.push(format!(
            "export const {schema_name} = z.unknown(); // {type_name}"
        ));
        lines.push(format!(
            "export type {base_name} = z.infer<typeof {schema_name}>;"
        ));
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Map a raw Rust return/input type name to a TypeScript Zod schema reference.
///
/// Strips Axum wrappers (`Json<T>`, `Result<Json<T>, E>`) and maps primitives
/// to inline Zod expressions, custom types to `TSchema` references.
///
/// Examples:
/// - `"Json<Planet>"` → `"PlanetSchema"`
/// - `"Json<Vec<Planet>>"` → `"z.array(PlanetSchema)"`
/// - `"Result<Json<Planet>,StatusCode>"` → `"PlanetSchema"`
/// - `"String"` → `"z.string()"`
/// - `"()"` → `""` (no schema)
pub fn rust_type_to_ts_schema(raw: &str) -> String {
    // Strip whitespace
    let raw = raw.replace(' ', "");

    // Unwrap Result<T, E> → take T
    let inner = if raw.starts_with("Result<") {
        raw.strip_prefix("Result<")
            .and_then(|s| {
                // Find the matching closing > for the first type arg
                let mut depth = 0usize;
                let mut split_at = None;
                for (i, c) in s.char_indices() {
                    match c {
                        '<' => depth += 1,
                        '>' if depth == 0 => {
                            split_at = Some(i);
                            break;
                        }
                        '>' => depth -= 1,
                        ',' if depth == 0 => {
                            split_at = Some(i);
                            break;
                        }
                        _ => {}
                    }
                }
                split_at.map(|i| s[..i].to_string())
            })
            .unwrap_or(raw.clone())
    } else {
        raw.clone()
    };

    // Unwrap Json<T> → T
    let inner = if inner.starts_with("Json<") && inner.ends_with('>') {
        inner[5..inner.len() - 1].to_string()
    } else {
        inner
    };

    // Map to Zod schema
    type_name_to_zod_ref(&inner)
}

/// Map a clean inner type name to a Zod expression or schema reference.
fn type_name_to_zod_ref(type_name: &str) -> String {
    match type_name {
        "()" | "" => String::new(),
        "String" | "str" => "z.string()".to_string(),
        "bool" => "z.boolean()".to_string(),
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "usize" => "z.number().int()".to_string(),
        "f32" | "f64" => "z.number()".to_string(),
        _ if type_name.starts_with("Vec<") && type_name.ends_with('>') => {
            let inner = &type_name[4..type_name.len() - 1];
            let inner_schema = type_name_to_zod_ref(inner);
            format!("z.array({})", inner_schema)
        }
        _ if type_name.starts_with("Option<") && type_name.ends_with('>') => {
            let inner = &type_name[7..type_name.len() - 1];
            let inner_schema = type_name_to_zod_ref(inner);
            format!("{}.optional()", inner_schema)
        }
        _ => {
            // Custom type — reference its schema constant
            let base = type_name.rsplit("::").next().unwrap_or(type_name);
            format!("{}Schema", base)
        }
    }
}

/// Convert a Rust type name to a TypeScript Zod schema constant name.
///
/// `"Planet"` → `"PlanetSchema"`
/// `"Vec<Planet>"` → `"PlanetSchema"`
pub fn to_schema_name(rust_type: &str) -> String {
    format!("{}Schema", base_type_name(rust_type))
}

/// Extract the base type name, stripping wrapper generics.
///
/// `"Vec<Planet>"` → `"Planet"`
/// `"Option<Planet>"` → `"Planet"`
/// `"models::Planet"` → `"Planet"`
pub fn base_type_name(rust_type: &str) -> String {
    let base = rust_type
        .trim_start_matches("Vec<")
        .trim_end_matches('>')
        .trim_start_matches("Option<")
        .trim_end_matches('>')
        .trim();

    base.rsplit("::").next().unwrap_or(base).to_string()
}

/// Returns true for Rust primitive/std types that don't need a Zod schema.
pub fn is_primitive(type_name: &str) -> bool {
    matches!(
        type_name,
        "()" | "String"
            | "str"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "f32"
            | "f64"
            | "bool"
            | "usize"
            | "isize"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_name_simple() {
        assert_eq!(to_schema_name("Planet"), "PlanetSchema");
    }

    #[test]
    fn schema_name_vec() {
        assert_eq!(to_schema_name("Vec<Planet>"), "PlanetSchema");
    }

    #[test]
    fn schema_name_module_path() {
        assert_eq!(to_schema_name("models::Planet"), "PlanetSchema");
    }

    #[test]
    fn primitive_string_is_primitive() {
        assert!(is_primitive("String"));
        assert!(is_primitive("()"));
        assert!(!is_primitive("Planet"));
    }

    #[test]
    fn real_schemas_joined() {
        let schemas = vec![
            SchemaEntry {
                type_name: "Planet",
                zod_ts: "export const PlanetSchema = z.object({ id: z.number() });".to_string(),
            },
            SchemaEntry {
                type_name: "User",
                zod_ts: "export const UserSchema = z.object({ name: z.string() });".to_string(),
            },
        ];
        let output = generate_real_schemas(&schemas);
        assert!(output.contains("PlanetSchema"));
        assert!(output.contains("UserSchema"));
    }
}

#[cfg(test)]
mod schema_ref_tests {
    use super::*;

    #[test]
    fn json_planet() {
        assert_eq!(rust_type_to_ts_schema("Json<Planet>"), "PlanetSchema");
    }

    #[test]
    fn json_vec_planet() {
        assert_eq!(
            rust_type_to_ts_schema("Json<Vec<Planet>>"),
            "z.array(PlanetSchema)"
        );
    }

    #[test]
    fn result_json_planet() {
        assert_eq!(
            rust_type_to_ts_schema("Result<Json<Planet>,StatusCode>"),
            "PlanetSchema"
        );
    }

    #[test]
    fn json_string() {
        assert_eq!(rust_type_to_ts_schema("Json<String>"), "z.string()");
    }

    #[test]
    fn unit_type() {
        assert_eq!(rust_type_to_ts_schema("()"), "");
    }

    #[test]
    fn plain_string() {
        assert_eq!(rust_type_to_ts_schema("String"), "z.string()");
    }
}

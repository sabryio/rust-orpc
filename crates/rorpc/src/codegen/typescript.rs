//! TypeScript import and Zod schema generation.
//!
//! Re-exports runtime string-based type conversion utilities from `orpc_parse`.
//! The actual implementations live in `orpc_parse::codegen::zod_ts` to avoid
//! duplication and keep all type-to-Zod logic in one place.

use super::{HandlerInfo, SchemaEntry};
use std::collections::BTreeSet;

// Re-export runtime conversion utilities from rorpc-parse
pub use rorpc_parse::codegen::{base_type_name, rust_type_to_ts_schema, to_schema_name};
pub use rorpc_parse::types::is_primitive_type_name;

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

/// Generate TypeScript from real `ZodTs::zod_ts()` output, skipping fallbacks.
pub fn generate_real_schemas(schemas: &[SchemaEntry]) -> String {
    schemas
        .iter()
        .filter(|s| !s.zod_ts.contains("z.unknown()"))
        .map(|s| {
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

/// Parse a Zod object schema string and extract field definitions.
///
/// Returns a map of field names to their Zod type definitions, or `None` if
/// the input is not a `z.object()` schema.
///
/// # Examples
///
/// ```
/// # use std::collections::HashMap;
/// # use rorpc::codegen::typescript::parse_zod_object_fields;
/// let schema = "z.object({ id: z.number().int(), name: z.string() })";
/// let fields = parse_zod_object_fields(schema).unwrap();
/// assert_eq!(fields.get("id"), Some(&"z.number().int()".to_string()));
/// assert_eq!(fields.get("name"), Some(&"z.string()".to_string()));
///
/// // Non-object schemas return None
/// assert!(parse_zod_object_fields("z.string()").is_none());
/// ```
pub fn parse_zod_object_fields(schema: &str) -> Option<std::collections::HashMap<String, String>> {
    let trimmed = schema.trim();

    if !trimmed.starts_with("z.object({") || !trimmed.ends_with("})") {
        return None;
    }

    // Extract inner content between { and }
    let start = trimmed.find('{')? + 1;
    let end = trimmed.rfind('}')?;
    let inner = &trimmed[start..end];

    let mut fields = std::collections::HashMap::new();
    let mut depth = 0;
    let mut current_field = String::new();
    let mut current_name = String::new();
    let mut in_field_name = true;

    for ch in inner.chars() {
        match ch {
            '{' | '(' => depth += 1,
            '}' | ')' => depth -= 1,
            ':' if depth == 0 && in_field_name => {
                current_name = current_field.trim().to_string();
                current_field.clear();
                in_field_name = false;
                continue;
            }
            ',' if depth == 0 => {
                if !current_name.is_empty() {
                    fields.insert(current_name.clone(), current_field.trim().to_string());
                }
                current_field.clear();
                current_name.clear();
                in_field_name = true;
                continue;
            }
            _ => {}
        }
        current_field.push(ch);
    }

    // Handle last field
    if !current_name.is_empty() {
        fields.insert(current_name, current_field.trim().to_string());
    }

    Some(fields)
}

/// Fallback: generate placeholder schemas from handler type names.
pub fn generate_placeholder_schemas(handlers: &[HandlerInfo]) -> String {
    let mut lines = Vec::new();

    let unique_types: BTreeSet<&str> = handlers
        .iter()
        .flat_map(|h| [h.input_type_name, h.output_type_name])
        .filter(|t| !is_primitive_type_name(t))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_check() {
        assert!(is_primitive_type_name("String"));
        assert!(is_primitive_type_name("()"));
        assert!(!is_primitive_type_name("Planet"));
    }

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
    fn serde_json_value() {
        assert_eq!(rust_type_to_ts_schema("Json<serde_json::Value>"), "z.any()");
    }

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
    fn parse_simple_zod_object() {
        let schema = "z.object({ id: z.number().int(), name: z.string() })";
        let fields = parse_zod_object_fields(schema).unwrap();
        assert_eq!(fields.get("id"), Some(&"z.number().int()".to_string()));
        assert_eq!(fields.get("name"), Some(&"z.string()".to_string()));
    }

    #[test]
    fn parse_zod_object_with_optional() {
        let schema = "z.object({ id: z.number().int(), q: z.string().optional() })";
        let fields = parse_zod_object_fields(schema).unwrap();
        assert_eq!(fields.get("id"), Some(&"z.number().int()".to_string()));
        assert_eq!(fields.get("q"), Some(&"z.string().optional()".to_string()));
    }

    #[test]
    fn non_object_schema_returns_none() {
        assert!(parse_zod_object_fields("z.string()").is_none());
        assert!(parse_zod_object_fields("z.array(z.number())").is_none());
    }
}

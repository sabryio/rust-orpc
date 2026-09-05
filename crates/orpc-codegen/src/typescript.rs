//! Zod schema generation from Rust type names.
//!
//! Converts type names captured by the `#[orpc]` macro into TypeScript
//! Zod schema declarations.
//!
//! NOTE: Full implementation will use `zod_rs_ts` derive output when types
//! implement `ZodTs`. This module provides the string-assembly layer.

use crate::HandlerInfo;
use std::collections::BTreeSet;

/// Generate Zod schema imports and declarations for all types seen across handlers.
pub fn generate_schemas(handlers: &[HandlerInfo]) -> String {
    let mut lines = vec![
        r#"import { z } from "zod";"#.to_string(),
        r#"import { oc } from "@orpc/contract";"#.to_string(),
        r#"import { openapi } from "@orpc/openapi";"#.to_string(),
        r#"import { asyncIteratorObject } from "@orpc/contract";"#.to_string(),
        String::new(),
    ];

    // Collect unique non-primitive types
    let unique_types: BTreeSet<&str> = handlers
        .iter()
        .flat_map(|h| [h.input_type_name, h.output_type_name])
        .filter(|t| !is_primitive(t))
        .collect();

    for type_name in unique_types {
        let schema_name = to_schema_name(type_name);
        // Placeholder: full implementation calls ZodTs::zod_ts()
        // via a registry populated by #[derive(ZodTs)]
        lines.push(format!(
            "// TODO: replace with #[derive(ZodTs)] output for {type_name}"
        ));
        lines.push(format!(
            "export const {schema_name} = z.unknown(); // {type_name}"
        ));
        lines.push(format!(
            "export type {type_name} = z.infer<typeof {schema_name}>;"
        ));
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Convert a Rust type name to a TypeScript Zod schema constant name.
///
/// `"Planet"` → `"PlanetSchema"`
/// `"Vec<Planet>"` → `"PlanetListSchema"`
pub fn to_schema_name(rust_type: &str) -> String {
    // Strip generic wrappers
    let base = rust_type
        .trim_start_matches("Vec<")
        .trim_end_matches('>')
        .trim_start_matches("Option<")
        .trim_end_matches('>')
        .trim();

    // Extract just the last segment of a module path
    let base = base.rsplit("::").next().unwrap_or(base);

    format!("{base}Schema")
}

/// Returns true for Rust primitive/std types that don't need a Zod schema declaration.
fn is_primitive(type_name: &str) -> bool {
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
}

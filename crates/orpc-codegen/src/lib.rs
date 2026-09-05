//! TypeScript contract and Zod schema generation for orpc.
//!
//! Produces a TypeScript file containing:
//! - Zod schema constants for each input/output type (from `ZodTs::zod_ts()`)
//! - An oRPC `contract` object matching the Rust handler structure
//! - Type exports (`export type X = z.infer<typeof XSchema>`)
//!
//! This crate is pure functions — no Axum, no proc-macros, fully testable.

pub mod contract;
pub mod typescript;

use std::path::Path;

/// Metadata for a single handler passed from `orpc::generate_contract()`.
#[derive(Debug, Clone)]
pub struct HandlerInfo {
    pub name: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub input_type_name: &'static str,
    pub output_type_name: &'static str,
    pub module_path: &'static str,
    pub error_type_name: Option<&'static str>,
    pub stream_event_type_name: Option<&'static str>,
}

/// A collected Zod schema string for a single Rust type.
///
/// Produced by calling `T::zod_ts()` via the registered factory fn.
#[derive(Debug, Clone)]
pub struct SchemaEntry {
    /// Rust type name — used for deduplication and matching against handler types
    pub type_name: &'static str,
    /// Full TypeScript schema string produced by `ZodTs::zod_ts()`
    ///
    /// Example:
    /// ```typescript
    /// export const PlanetSchema = z.object({
    ///   id: z.number().int(),
    ///   name: z.string(),
    ///   description: z.string().optional(),
    /// });
    /// export type Planet = z.infer<typeof PlanetSchema>;
    /// ```
    pub zod_ts: String,
}

/// A collected error registration for TypeScript `.errors({...})` generation.
#[derive(Debug, Clone)]
pub struct ErrorVariantInfo {
    /// Variant name in SCREAMING_SNAKE_CASE (e.g. "NOT_FOUND")
    pub name: &'static str,
    /// Optional Zod schema for variant data (None for unit variants)
    pub data_schema: Option<&'static str>,
}

/// Registration entry for an error enum type.
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// Error type name (e.g. "AppError")
    pub type_name: &'static str,
    /// All variants of this error enum
    pub variants: Vec<ErrorVariantInfo>,
}

/// Builder for TypeScript contract generation.
///
/// # Example
///
/// ```rust,ignore
/// ContractBuilder::new(handlers, schemas)
///     .output("../client/src/rpc/index.ts")
///     .unwrap();
/// ```
pub struct ContractBuilder {
    handlers: Vec<HandlerInfo>,
    /// Real Zod schema strings from `ZodTs::zod_ts()` — empty if no types registered
    schemas: Vec<SchemaEntry>,
    /// Error registrations for `.errors({...})` generation
    errors: Vec<ErrorInfo>,
}

impl ContractBuilder {
    pub fn new(handlers: Vec<HandlerInfo>, schemas: Vec<SchemaEntry>) -> Self {
        Self {
            handlers,
            schemas,
            errors: Vec::new(),
        }
    }

    /// Add error registrations to the builder.
    ///
    /// Called by `orpc::generate_contract()` with collected `ErrorRegistration` entries.
    pub fn with_errors(mut self, errors: Vec<ErrorInfo>) -> Self {
        self.errors = errors;
        self
    }

    /// Generate the TypeScript contract and write it to `path`.
    ///
    /// Rejects paths containing `..` components to prevent path traversal.
    pub fn output(self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();

        // Security: reject path traversal (A03)
        for component in path.components() {
            if component.as_os_str() == ".." {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "output path must not contain '..' components",
                ));
            }
        }

        let content = self.generate();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)
    }

    /// Generate the full TypeScript content as a string.
    pub fn generate(self) -> String {
        let imports = typescript::generate_imports();

        // Collect type names that have real schemas (not fallbacks)
        let real_schema_types: std::collections::HashSet<&str> = self
            .schemas
            .iter()
            .filter(|s| !s.zod_ts.contains("z.unknown()"))
            .map(|s| s.type_name)
            .collect();

        // Generate real schemas (excluding fallbacks)
        let real_schemas = typescript::generate_real_schemas(&self.schemas);

        // Generate placeholder schemas for types referenced by handlers but not in real_schema_types
        let placeholder_schemas = generate_missing_placeholders(&self.handlers, &real_schema_types);

        let contract = contract::generate_contract(&self.handlers, &self.errors);

        // Combine: real schemas first, then placeholders, then contract
        let schema_block = if real_schemas.is_empty() && placeholder_schemas.is_empty() {
            String::new()
        } else if real_schemas.is_empty() {
            placeholder_schemas
        } else if placeholder_schemas.is_empty() {
            real_schemas
        } else {
            format!("{}\n\n{}", real_schemas, placeholder_schemas)
        };

        format!(
            "// AUTO-GENERATED by orpc — do not edit manually.\n// Re-generate: orpc::generate_contract().output(path)\n\n{imports}\n\n{schema_block}\n\n{contract}"
        )
    }
}

/// Generate placeholder schemas only for types that don't have real schemas.
fn generate_missing_placeholders(
    handlers: &[HandlerInfo],
    real_schema_types: &std::collections::HashSet<&str>,
) -> String {
    use std::collections::BTreeSet;

    let mut lines = Vec::new();

    // Collect unique types from handlers, extracting inner types from wrappers
    let mut unique_types: BTreeSet<String> = BTreeSet::new();

    for handler in handlers {
        // Extract types from input_type_name
        if !typescript::is_primitive(handler.input_type_name) {
            let base = extract_base_types(handler.input_type_name);
            for t in base {
                if !real_schema_types.contains(t.as_str()) {
                    unique_types.insert(t);
                }
            }
        }

        // Extract types from output_type_name
        if !typescript::is_primitive(handler.output_type_name) {
            let base = extract_base_types(handler.output_type_name);
            for t in base {
                if !real_schema_types.contains(t.as_str()) {
                    unique_types.insert(t);
                }
            }
        }
    }

    if unique_types.is_empty() {
        return String::new();
    }

    lines.push(
        "// ⚠️  Placeholder schemas — add #[derive(ZodTs)] to your types for real schemas"
            .to_string(),
    );
    lines.push(String::new());

    for type_name in unique_types {
        let schema_name = typescript::to_schema_name(&type_name);
        let base_name = typescript::base_type_name(&type_name);
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

/// Extract all base type names from a potentially complex type signature.
/// Example: "Result<Json<Vec<Planet>>, E>" → ["Planet"]
///
/// Skips streaming types like "Sse<...>" since they're handled specially.
fn extract_base_types(type_str: &str) -> Vec<String> {
    let mut result = Vec::new();

    // Use the rust_type_to_ts_schema logic but extract the inner type
    let cleaned = type_str.replace(' ', "");

    // Skip Sse streaming types - they're handled specially
    if cleaned.starts_with("Sse<") {
        return result;
    }

    // Unwrap Result<T, E> → T
    let inner = if cleaned.starts_with("Result<") {
        extract_first_type_arg(&cleaned).unwrap_or(&cleaned)
    } else {
        &cleaned
    };

    // Unwrap Json<T> → T
    let inner = if inner.starts_with("Json<") && inner.ends_with('>') {
        &inner[5..inner.len() - 1]
    } else {
        inner
    };

    // Now handle Vec, Option, etc.
    if inner.starts_with("Vec<") && inner.ends_with('>') {
        let element_type = &inner[4..inner.len() - 1];
        if !typescript::is_primitive(element_type) {
            result.push(element_type.to_string());
        }
    } else if inner.starts_with("Option<") && inner.ends_with('>') {
        let element_type = &inner[7..inner.len() - 1];
        if !typescript::is_primitive(element_type) {
            result.push(element_type.to_string());
        }
    } else if !typescript::is_primitive(inner) && !inner.is_empty() && inner != "()" {
        result.push(inner.to_string());
    }

    result
}

/// Extract the first type argument from "Wrapper<T, ...>".
fn extract_first_type_arg(type_str: &str) -> Option<&str> {
    let start = type_str.find('<')? + 1;
    let mut depth = 0;
    let mut end = start;

    for (i, ch) in type_str[start..].char_indices() {
        match ch {
            '<' => depth += 1,
            '>' if depth == 0 => {
                end = start + i;
                break;
            }
            '>' => depth -= 1,
            ',' if depth == 0 => {
                end = start + i;
                break;
            }
            _ => {}
        }
    }

    if end > start {
        Some(type_str[start..end].trim())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler(name: &'static str, method: &'static str, path: &'static str) -> HandlerInfo {
        HandlerInfo {
            name,
            method,
            path,
            input_type_name: "()",
            output_type_name: "String",
            module_path: "test::handlers",
            error_type_name: None,
            stream_event_type_name: None,
        }
    }

    #[test]
    fn rejects_path_traversal() {
        let builder = ContractBuilder::new(vec![make_handler("ping", "GET", "/ping")], vec![]);
        let result = builder.output("../../../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().kind() == std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn generates_with_no_schemas() {
        let builder = ContractBuilder::new(vec![make_handler("ping", "GET", "/ping")], vec![]);
        let output = builder.generate();
        assert!(output.contains("AUTO-GENERATED"));
        assert!(output.contains("ping"));
    }

    #[test]
    fn generates_with_real_schemas() {
        let builder = ContractBuilder::new(
            vec![make_handler("list_planets", "POST", "/planet/list")],
            vec![SchemaEntry {
                type_name: "Planet",
                zod_ts: "export const PlanetSchema = z.object({ id: z.number().int(), name: z.string() });\nexport type Planet = z.infer<typeof PlanetSchema>;".to_string(),
            }],
        );
        let output = builder.generate();
        assert!(output.contains("PlanetSchema"));
        assert!(output.contains("z.object"));
        assert!(output.contains("listPlanets"));
    }
}

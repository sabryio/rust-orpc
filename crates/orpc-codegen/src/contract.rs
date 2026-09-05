//! oRPC contract object generation.
//!
//! Produces the `export const contract = { ... } as const` TypeScript object
//! from collected handler metadata, grouped by path prefix (namespace).

use crate::HandlerInfo;
use std::collections::BTreeMap;

/// Generate the `export const contract = { ... } as const` TypeScript block.
pub fn generate_contract(handlers: &[HandlerInfo], errors: &[crate::ErrorInfo]) -> String {
    // Build error lookup: type_name → ErrorInfo
    let error_map: std::collections::HashMap<&str, &crate::ErrorInfo> =
        errors.iter().map(|e| (e.type_name, e)).collect();

    // Group handlers by their namespace (first path segment)
    let mut namespaces: BTreeMap<String, Vec<&HandlerInfo>> = BTreeMap::new();

    for handler in handlers {
        let namespace = extract_namespace(handler.path);
        namespaces.entry(namespace).or_default().push(handler);
    }

    let mut lines = vec!["export const contract = {".to_string()];

    for (namespace, handlers) in &namespaces {
        if namespace.is_empty() {
            // Root-level procedures
            for h in handlers {
                lines.push(format!("  {},", generate_procedure_entry(h, 1, &error_map)));
            }
        } else {
            // Namespaced procedures
            lines.push(format!("  {}: {{", namespace));
            for h in handlers {
                lines.push(format!(
                    "    {},",
                    generate_procedure_entry(h, 2, &error_map)
                ));
            }
            lines.push("  },".to_string());
        }
    }

    lines.push("} as const;".to_string());
    lines.push(String::new());
    lines.push("export type Contract = typeof contract;".to_string());

    lines.join("\n")
}

/// Generate a single procedure entry in the contract object.
fn generate_procedure_entry(
    handler: &HandlerInfo,
    _indent: usize,
    error_map: &std::collections::HashMap<&str, &crate::ErrorInfo>,
) -> String {
    let key = handler_key(handler.name);
    let method = handler.method;
    let path = handler.path;
    let input_schema = {
        let schema = crate::typescript::rust_type_to_ts_schema(handler.input_type_name);
        if schema.is_empty() {
            String::new()
        } else {
            format!("\n      .input({})", schema)
        }
    };
    let output_schema = {
        // Use stream_event_type_name if available for Sse types
        let schema = if handler.output_type_name.starts_with("Sse<") {
            if let Some(stream_event_type) = handler.stream_event_type_name {
                // Use the real stream event schema
                format!("asyncIteratorObject({}Schema)", stream_event_type)
            } else {
                // Fallback to z.unknown()
                crate::typescript::rust_type_to_ts_schema(handler.output_type_name)
            }
        } else {
            crate::typescript::rust_type_to_ts_schema(handler.output_type_name)
        };

        if schema.is_empty() {
            String::new()
        } else {
            format!("\n      .output({})", schema)
        }
    };

    // Generate .errors({...}) if handler has an error type
    let errors_block = if let Some(error_type_name) = handler.error_type_name {
        if let Some(error_info) = error_map.get(error_type_name) {
            let mut error_entries = Vec::new();
            for variant in &error_info.variants {
                let entry = if let Some(data_schema) = variant.data_schema {
                    format!(
                        "        {}: {{\n          data: {}\n        }}",
                        variant.name, data_schema
                    )
                } else {
                    format!("        {}: {{}}", variant.name)
                };
                error_entries.push(entry);
            }
            if !error_entries.is_empty() {
                format!(
                    "\n      .errors({{\n{}\n      }})",
                    error_entries.join(",\n")
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    format!(
        r#"{key}: oc
      .meta(openapi({{ method: "{method}", path: "{path}" }})){input_schema}{output_schema}{errors_block}"#
    )
}

/// Extract the first path segment as namespace.
///
/// `"/planet/list"` → `"planet"`
/// `"/ping"` → `""` (root level)
fn extract_namespace(path: &str) -> String {
    let segments: Vec<&str> = path.trim_start_matches('/').splitn(3, '/').collect();
    if segments.len() >= 2 {
        segments[0].to_string()
    } else {
        String::new()
    }
}

/// Convert a snake_case handler name to camelCase TypeScript key.
///
/// `"list_planets"` → `"listPlanets"`
fn handler_key(name: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for ch in name.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_key_camel_case() {
        assert_eq!(handler_key("list_planets"), "listPlanets");
        assert_eq!(handler_key("get_profile"), "getProfile");
        assert_eq!(handler_key("ping"), "ping");
    }

    #[test]
    fn namespace_extraction() {
        assert_eq!(extract_namespace("/planet/list"), "planet");
        assert_eq!(extract_namespace("/ping"), "");
        assert_eq!(extract_namespace("/user/profile"), "user");
    }

    #[test]
    fn contract_contains_handlers() {
        use crate::HandlerInfo;

        let handlers = vec![
            HandlerInfo {
                name: "list_planets",
                method: "POST",
                path: "/planet/list",
                input_type_name: "()",
                output_type_name: "Vec<Planet>",
                module_path: "handlers::planet",
                error_type_name: None,
                stream_event_type_name: None,
            },
            HandlerInfo {
                name: "ping",
                method: "GET",
                path: "/ping",
                input_type_name: "()",
                output_type_name: "String",
                module_path: "handlers",
                error_type_name: None,
                stream_event_type_name: None,
            },
        ];

        let output = generate_contract(&handlers, &[]);
        assert!(output.contains("listPlanets"));
        assert!(output.contains("ping"));
        assert!(output.contains("/planet/list"));
        assert!(output.contains("as const"));
    }
}

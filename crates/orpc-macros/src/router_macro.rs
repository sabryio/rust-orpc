use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, Expr, ExprArray, LitStr, Token};

/// Arguments for the router!() macro.
/// Supports 0-2 arguments in any order:
/// - No args: all handlers, no state
/// - 1 arg: either state (Expr) or pattern (LitStr/ExprArray)
/// - 2 args: state + pattern in any order
pub struct RouterMacroArgs {
    pub state: Option<Expr>,
    pub pattern: Option<Pattern>,
}

/// Pattern for filtering handlers by module path.
#[derive(Clone)]
pub enum Pattern {
    /// Single pattern string, e.g., "handlers::planet"
    Single(String),
    /// Array of patterns, e.g., ["handlers::planet", "handlers::user"]
    Multiple(Vec<String>),
}

impl Parse for RouterMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut state: Option<Expr> = None;
        let mut pattern: Option<Pattern> = None;

        // Parse first argument if present
        if !input.is_empty() {
            let first = parse_arg(input)?;
            match first {
                Arg::Pattern(p) => pattern = Some(p),
                Arg::State(s) => state = Some(s),
            }

            // Parse second argument if present
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
                if !input.is_empty() {
                    let second = parse_arg(input)?;
                    match second {
                        Arg::Pattern(p) => {
                            if pattern.is_some() {
                                return Err(syn::Error::new_spanned(
                                    input.cursor().token_stream(),
                                    "cannot specify pattern twice",
                                ));
                            }
                            pattern = Some(p);
                        }
                        Arg::State(s) => {
                            if state.is_some() {
                                return Err(syn::Error::new_spanned(
                                    input.cursor().token_stream(),
                                    "cannot specify state twice",
                                ));
                            }
                            state = Some(s);
                        }
                    }
                }
            }
        }

        Ok(RouterMacroArgs { state, pattern })
    }
}

/// Internal enum to distinguish argument types during parsing.
enum Arg {
    Pattern(Pattern),
    State(Expr),
}

/// Parse a single argument - either a pattern (string/array) or state (expression).
fn parse_arg(input: syn::parse::ParseStream) -> syn::Result<Arg> {
    // Try to parse as string literal (single pattern)
    if let Ok(lit) = input.parse::<LitStr>() {
        return Ok(Arg::Pattern(Pattern::Single(lit.value())));
    }

    // Try to parse as array (multiple patterns)
    if let Ok(array) = input.parse::<ExprArray>() {
        let patterns: Result<Vec<String>, syn::Error> = array
            .elems
            .iter()
            .map(|elem| {
                if let Expr::Lit(expr_lit) = elem {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        return Ok(lit_str.value());
                    }
                }
                Err(syn::Error::new_spanned(
                    elem,
                    "array elements must be string literals",
                ))
            })
            .collect();

        return Ok(Arg::Pattern(Pattern::Multiple(patterns?)));
    }

    // Otherwise, parse as state expression
    let expr = input.parse::<Expr>()?;
    Ok(Arg::State(expr))
}

/// Expand a pattern into individual module path patterns.
/// Handles brace expansion: "handlers::{planet,user}" -> ["handlers::planet", "handlers::user"]
/// Handles wildcards: "handlers::*" -> "handlers::"
fn expand_pattern(pattern: &str) -> Vec<String> {
    // Check for brace expansion: "prefix::{a,b,c}"
    if let Some(start) = pattern.find('{') {
        if let Some(end) = pattern.find('}') {
            let prefix = &pattern[..start];
            let group = &pattern[start + 1..end];
            let suffix = &pattern[end + 1..];

            return group
                .split(',')
                .map(|segment| {
                    let segment = segment.trim();
                    let expanded = format!("{}{}{}", prefix, segment, suffix);
                    // Recursively expand in case there are wildcards in the result
                    expand_single_pattern(&expanded)
                })
                .collect();
        }
    }

    // No brace expansion, just expand single pattern
    vec![expand_single_pattern(pattern)]
}

/// Expand a single pattern (no braces).
/// Wildcards ("handlers::*") are converted to prefix form ("handlers::").
fn expand_single_pattern(pattern: &str) -> String {
    let trimmed = pattern.trim();

    // Handle wildcard: "handlers::*" -> "handlers::"
    if trimmed.ends_with("::*") {
        trimmed[..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Main macro expansion logic.
pub fn expand_router(args: RouterMacroArgs) -> TokenStream {
    // Generate filter predicate based on pattern
    let filter_predicate = generate_filter_predicate(&args.pattern);
    
    // Generate state handling code
    let state_expr = generate_state_expr(&args.state);
    
    // Generate the full router construction code
    quote! {
        {
            // Build state as Arc<dyn Any + Send + Sync>
            let state: ::std::sync::Arc<dyn ::std::any::Any + Send + Sync> = #state_expr;
            
            // Start with empty router
            let mut app: ::axum::Router = ::axum::Router::new();
            
            // Iterate over all registered handlers
            for reg in ::orpc::inventory::iter::<::orpc::HandlerRegistration> {
                // Apply filter if pattern was provided
                let matches = #filter_predicate;
                
                if matches {
                    let route = (reg.factory)(::std::sync::Arc::clone(&state));
                    app = app.merge(route);
                }
            }
            
            app
        }
    }
}

/// Generate the filter predicate expression.
/// Returns a boolean expression that checks if a handler should be included.
fn generate_filter_predicate(pattern: &Option<Pattern>) -> TokenStream {
    match pattern {
        None => {
            // No pattern = include all handlers
            quote! { true }
        }
        Some(Pattern::Single(pat)) => {
            let expanded = expand_pattern(pat);
            generate_pattern_match(&expanded)
        }
        Some(Pattern::Multiple(patterns)) => {
            let all_expanded: Vec<String> = patterns
                .iter()
                .flat_map(|p| expand_pattern(p))
                .collect();
            generate_pattern_match(&all_expanded)
        }
    }
}

/// Generate code that matches module paths against expanded patterns.
/// For each pattern, we check: exact match OR starts with "pattern::"
fn generate_pattern_match(patterns: &[String]) -> TokenStream {
    if patterns.is_empty() {
        return quote! { true };
    }

    // Find corresponding HandlerMetadata for this registration
    // We need to match by path+method since that's unique
    let conditions: Vec<TokenStream> = patterns
        .iter()
        .map(|pattern| {
            // For each pattern, generate: module_path == "pattern" || module_path.starts_with("pattern::")
            let pattern_str = pattern.as_str();
            let pattern_with_sep = format!("{}::", pattern);
            
            quote! {
                (metadata.module_path == #pattern_str || metadata.module_path.starts_with(#pattern_with_sep))
            }
        })
        .collect();

    // Combine with OR
    quote! {
        {
            // Find the metadata entry for this registration
            let mut matches = false;
            for metadata in ::orpc::inventory::iter::<::orpc::HandlerMetadata> {
                if metadata.path == reg.path && metadata.method == reg.method {
                    matches = #(#conditions)||*;
                    break;
                }
            }
            matches
        }
    }
}

/// Generate state expression - either the provided state or Arc::new(()).
fn generate_state_expr(state: &Option<Expr>) -> TokenStream {
    match state {
        Some(expr) => {
            quote! { ::std::sync::Arc::new(#expr) }
        }
        None => {
            quote! { ::std::sync::Arc::new(()) }
        }
    }
}

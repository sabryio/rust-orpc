//! Domain types for the r! macro AST.
//!
//! These types represent the parsed structure of the macro input without
//! any parsing or generation logic (Clean Architecture: Domain layer).

use std::fmt;
use syn::{Expr, Ident, LitStr};

/// A key in the router definition — either an identifier or a string literal.
///
/// # Examples
///
/// - `ping` → `RouterKey::Ident`
/// - `"list-paginated"` → `RouterKey::Literal`
#[derive(Clone)]
pub enum RouterKey {
    /// Valid Rust identifier (will be stringified)
    Ident(Ident),
    /// String literal (for kebab-case or special characters)
    Literal(LitStr),
}

impl RouterKey {
    /// Returns the span for error reporting.
    #[allow(dead_code)]
    pub fn span(&self) -> proc_macro2::Span {
        match self {
            RouterKey::Ident(ident) => ident.span(),
            RouterKey::Literal(lit) => lit.span(),
        }
    }
}

impl fmt::Display for RouterKey {
    /// Converts the key to a string value.
    ///
    /// - Identifiers are converted to strings: `ping` → `"ping"`
    /// - Literals keep their value: `"list-paginated"` → `"list-paginated"`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouterKey::Ident(ident) => write!(f, "{}", ident),
            RouterKey::Literal(lit) => write!(f, "{}", lit.value()),
        }
    }
}

/// An item in the router definition — either a procedure or a nested router.
///
/// # Examples
///
/// ```ignore
/// // Procedure item:
/// ping: os().output::<String>().handler(...)
///
/// // Nested item:
/// planet: {
///     list: os()...,
///     find: os()...,
/// }
/// ```
#[derive(Clone)]
pub enum RouterItem {
    /// A single procedure definition
    Procedure {
        /// The key for this procedure (e.g., "ping", "list")
        key: RouterKey,
        /// The procedure expression (e.g., `os().output::<T>().handler(...)`)
        expr: Expr,
    },
    /// A nested router definition
    Nested {
        /// The key for this nested router (e.g., "planet")
        key: RouterKey,
        /// The items within the nested router
        items: Vec<RouterItem>,
    },
}

impl RouterItem {
    /// Returns the key for this item.
    #[allow(dead_code)]
    pub fn key(&self) -> &RouterKey {
        match self {
            RouterItem::Procedure { key, .. } => key,
            RouterItem::Nested { key, .. } => key,
        }
    }
}

/// The complete macro input representing the router definition.
///
/// # Examples
///
/// ```ignore
/// r! {
///     ping: os()...,
///     planet: {
///         list: os()...,
///     }
/// }
/// ```
///
/// Parses to:
/// ```ignore
/// RouterMacroInput {
///     items: vec![
///         RouterItem::Procedure { key: "ping", expr: ... },
///         RouterItem::Nested { key: "planet", items: [...] },
///     ]
/// }
/// ```
#[derive(Clone)]
pub struct RouterMacroInput {
    /// Top-level items in the router
    pub items: Vec<RouterItem>,
}

impl RouterMacroInput {
    /// Creates a new router input with the given items.
    pub fn new(items: Vec<RouterItem>) -> Self {
        Self { items }
    }

    /// Returns true if the router has no items.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

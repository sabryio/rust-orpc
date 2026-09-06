//! Structured error type for all orpc-parse parsing and validation failures.
//!
//! Every error carries a `Span` so the Rust compiler points at the exact
//! token that caused the problem, plus a `kind` that produces an actionable
//! `help:` message alongside the main diagnostic.

use proc_macro2::Span;
use quote::quote;

pub type Result<T> = std::result::Result<T, Error>;

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// A parse or validation error with span information and a user-facing suggestion.
///
/// Convert to a compiler diagnostic with [`Error::to_compile_error`].
#[derive(Debug)]
pub struct Error {
    span: Span,
    kind: ErrorKind,
}

// SRP: each variant owns exactly one failure mode and its associated message data.
#[derive(Debug)]
pub enum ErrorKind {
    /// A type was found where a specific wrapper was expected.
    MissingWrapper {
        expected: &'static str,
        found: String,
        suggestion: String,
    },
    /// A wrapper type has no generic arguments (e.g. bare `Json` instead of `Json<T>`).
    EmptyGenericArgs { wrapper: &'static str },
    /// A type that orpc does not know how to handle appears in a handler signature.
    UnsupportedType {
        name: String,
        reason: &'static str,
        suggestion: &'static str,
    },
    /// An attribute key has a value of the wrong kind (e.g. non-string literal).
    InvalidAttrValue {
        attr: String,
        expected: &'static str,
        found: String,
    },
    /// A required attribute key is absent.
    MissingRequiredAttr {
        attr: &'static str,
        context: &'static str,
    },
    /// Two attribute keys that cannot coexist were both provided.
    ConflictingAttrs {
        first: String,
        second: String,
        suggestion: String,
    },
    /// A handler function has no return type annotation.
    MissingReturnType { fn_name: String },
    /// A handler function's signature does not match the expected shape.
    InvalidHandlerSig {
        fn_name: String,
        reason: &'static str,
    },
    /// An unrecognised key was provided in a macro attribute.
    UnknownKey {
        key: String,
        valid_keys: &'static [&'static str],
    },
    /// A `syn` parse error forwarded directly.
    SynError(syn::Error),
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

impl Error {
    pub fn missing_wrapper(span: Span, expected: &'static str, found: &syn::Type) -> Self {
        let found_str = type_display(found);
        Self {
            span,
            kind: ErrorKind::MissingWrapper {
                suggestion: format!("wrap the type: `{}<{}>`", expected, found_str),
                expected,
                found: found_str,
            },
        }
    }

    pub fn empty_generic_args(span: Span, wrapper: &'static str) -> Self {
        Self {
            span,
            kind: ErrorKind::EmptyGenericArgs { wrapper },
        }
    }

    pub fn unsupported_type(
        span: Span,
        ty: &syn::Type,
        reason: &'static str,
        suggestion: &'static str,
    ) -> Self {
        Self {
            span,
            kind: ErrorKind::UnsupportedType {
                name: type_display(ty),
                reason,
                suggestion,
            },
        }
    }

    pub fn invalid_attr_value(span: Span, attr: &str, expected: &'static str, found: &str) -> Self {
        Self {
            span,
            kind: ErrorKind::InvalidAttrValue {
                attr: attr.to_string(),
                expected,
                found: found.to_string(),
            },
        }
    }

    pub fn missing_required_attr(span: Span, attr: &'static str, context: &'static str) -> Self {
        Self {
            span,
            kind: ErrorKind::MissingRequiredAttr { attr, context },
        }
    }

    pub fn conflicting_attrs(span: Span, first: &str, second: &str) -> Self {
        Self {
            span,
            kind: ErrorKind::ConflictingAttrs {
                suggestion: format!("remove either `{}` or `{}`", first, second),
                first: first.to_string(),
                second: second.to_string(),
            },
        }
    }

    pub fn missing_return_type(span: Span, fn_name: &str) -> Self {
        Self {
            span,
            kind: ErrorKind::MissingReturnType {
                fn_name: fn_name.to_string(),
            },
        }
    }

    pub fn invalid_handler_sig(span: Span, fn_name: &str, reason: &'static str) -> Self {
        Self {
            span,
            kind: ErrorKind::InvalidHandlerSig {
                fn_name: fn_name.to_string(),
                reason,
            },
        }
    }

    pub fn unknown_key(span: Span, key: &str, valid_keys: &'static [&'static str]) -> Self {
        Self {
            span,
            kind: ErrorKind::UnknownKey {
                key: key.to_string(),
                valid_keys,
            },
        }
    }

    /// Emit a `compile_error!` token stream pointing at the offending span.
    pub fn to_compile_error(&self) -> proc_macro2::TokenStream {
        syn::Error::new(self.span, self.to_string()).to_compile_error()
    }
}

// ---------------------------------------------------------------------------
// Display — the text the compiler shows after "error:"
// ---------------------------------------------------------------------------

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorKind::*;
        match &self.kind {
            MissingWrapper {
                expected,
                found,
                suggestion,
            } => write!(
                f,
                "expected `{}` wrapper, found `{}`\n  = help: {}",
                expected, found, suggestion
            ),
            EmptyGenericArgs { wrapper } => {
                write!(f, "`{}` requires at least one type argument", wrapper)
            }
            UnsupportedType {
                name,
                reason,
                suggestion,
            } => write!(
                f,
                "unsupported type `{}`\n  = note: {}\n  = help: {}",
                name, reason, suggestion
            ),
            InvalidAttrValue {
                attr,
                expected,
                found,
            } => write!(
                f,
                "invalid value for `{}`\n  = expected: {}\n  = found: {}",
                attr, expected, found
            ),
            MissingRequiredAttr { attr, context } => write!(
                f,
                "missing required attribute `{}`\n  = help: {}",
                attr, context
            ),
            ConflictingAttrs {
                first,
                second,
                suggestion,
            } => write!(
                f,
                "conflicting attributes `{}` and `{}`\n  = help: {}",
                first, second, suggestion
            ),
            MissingReturnType { fn_name } => write!(
                f,
                "`{}` has no return type\n  = help: add `-> Json<T>` or `-> Result<Json<T>, E>`",
                fn_name
            ),
            InvalidHandlerSig { fn_name, reason } => write!(
                f,
                "invalid handler signature for `{}`\n  = note: {}\n  = help: return `Json<T>` or `Result<Json<T>, E>`",
                fn_name, reason
            ),
            UnknownKey { key, valid_keys } => write!(
                f,
                "unknown key `{}`\n  = valid keys: {}",
                key,
                valid_keys.join(", ")
            ),
            SynError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<syn::Error> for Error {
    fn from(e: syn::Error) -> Self {
        Self {
            span: e.span(),
            kind: ErrorKind::SynError(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helper — the only place a Type is rendered to a String
// ---------------------------------------------------------------------------

/// Render a `syn::Type` as a display string for use in error messages only.
/// Never use this for type matching — compare AST idents directly.
pub(crate) fn type_display(ty: &syn::Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;

    fn span() -> Span {
        Span::call_site()
    }

    #[test]
    fn missing_wrapper_message() {
        let ty: syn::Type = syn::parse_str("Vec<Planet>").unwrap();
        let err = Error::missing_wrapper(span(), "Json", &ty);
        let msg = err.to_string();
        assert!(msg.contains("expected `Json` wrapper"));
        assert!(msg.contains("Vec<Planet>"));
        assert!(msg.contains("help:"));
        assert!(msg.contains("Json<Vec<Planet>>"));
    }

    #[test]
    fn empty_generic_args_message() {
        let err = Error::empty_generic_args(span(), "Result");
        let msg = err.to_string();
        assert!(msg.contains("`Result` requires at least one type argument"));
    }

    #[test]
    fn missing_required_attr_message() {
        let err =
            Error::missing_required_attr(span(), "method", "add `method = \"GET\"` to #[orpc]");
        let msg = err.to_string();
        assert!(msg.contains("missing required attribute `method`"));
        assert!(msg.contains("help:"));
    }

    #[test]
    fn unknown_key_message() {
        let err = Error::unknown_key(span(), "routes", &["method", "path", "data"]);
        let msg = err.to_string();
        assert!(msg.contains("unknown key `routes`"));
        assert!(msg.contains("method"));
        assert!(msg.contains("path"));
        assert!(msg.contains("data"));
    }

    #[test]
    fn conflicting_attrs_message() {
        let err = Error::conflicting_attrs(span(), "method", "methods");
        let msg = err.to_string();
        assert!(msg.contains("conflicting attributes"));
        assert!(msg.contains("help: remove either"));
    }

    #[test]
    fn missing_return_type_message() {
        let err = Error::missing_return_type(span(), "handle_ping");
        let msg = err.to_string();
        assert!(msg.contains("`handle_ping` has no return type"));
        assert!(msg.contains("Json<T>"));
    }

    #[test]
    fn invalid_handler_sig_message() {
        let err = Error::invalid_handler_sig(span(), "my_handler", "return type is not Json<T>");
        let msg = err.to_string();
        assert!(msg.contains("invalid handler signature for `my_handler`"));
        assert!(msg.contains("return type is not Json<T>"));
    }

    #[test]
    fn syn_error_forwarded() {
        let syn_err = syn::Error::new(span(), "raw syn error");
        let err = Error::from(syn_err);
        assert!(err.to_string().contains("raw syn error"));
    }

    #[test]
    fn to_compile_error_is_nonempty() {
        let err = Error::empty_generic_args(span(), "Json");
        let tokens = err.to_compile_error();
        assert!(!tokens.is_empty());
    }
}

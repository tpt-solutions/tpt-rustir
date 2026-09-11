//! Concrete Syntax Tree (CST) — a thin wrapper around the AST that preserves
//! source spans and can be used for improved error messages.
//!
//! The parser produces the AST directly, but we maintain this CST module to
//! track source locations and provide structured error reporting.

use crate::ast::Expr;
use std::ops::Range;

pub type Span = Range<usize>;

/// CST node that wraps an AST expression with additional source info
#[derive(Debug, Clone)]
pub struct CstNode {
    pub expr: Expr,
    pub source_text: String,
}

/// Error with source context for better reporting
#[derive(Debug, Clone)]
pub struct CstError {
    pub message: String,
    pub span: Span,
    pub source_text: String,
    pub help: Option<String>,
}

impl std::fmt::Display for CstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.message)?;
        writeln!(f, "  --> {}:{}", self.span.start, self.span.end)?;
        // Show source context
        let lines: Vec<&str> = self.source_text.lines().collect();
        if !lines.is_empty() {
            writeln!(f, "   |")?;
            for (i, line) in lines.iter().enumerate() {
                writeln!(f, "{:4} | {}", i + 1, line)?;
            }
            writeln!(f, "   |")?;
        }
        if let Some(help) = &self.help {
            writeln!(f, "help: {}", help)?;
        }
        Ok(())
    }
}

impl std::error::Error for CstError {}

/// Convert parser errors to CST errors with context
pub fn parse_errors_to_cst_errors(
    src: &str,
    errs: &[chumsky::error::Simple<char>],
) -> Vec<CstError> {
    errs.iter()
        .map(|e| {
            let span = e.span();
            let _snippet: String = src.chars().take(span.end).skip(span.start).collect();
            CstError {
                message: format!("parse error: {}", e),
                span: span.clone(),
                source_text: src.to_string(),
                help: Some("check syntax near the highlighted location".to_string()),
            }
        })
        .collect()
}

/// Convert lowering errors to CST errors with context
pub fn lower_errors_to_cst_errors(
    src: &str,
    err: &crate::lower::LowerError,
    expr: &Expr,
) -> Vec<CstError> {
    let span = expr.span.clone();

    let (message, help) = match err {
        crate::lower::LowerError::ArityMismatch {
            name,
            expected,
            found,
        } => (
            format!("arity mismatch for `{}`", name),
            Some(format!(
                "{} expects {} argument(s), found {}",
                name, expected, found
            )),
        ),
        crate::lower::LowerError::DuplicateInductive(name) => (
            format!("duplicate inductive type `{}`", name),
            Some(format!("inductive type `{}` is already defined", name)),
        ),
        crate::lower::LowerError::DuplicateConstructor(ind, con) => (
            format!("duplicate constructor `{}` for inductive `{}`", con, ind),
            Some(format!(
                "constructor `{}` for inductive `{}` is already defined",
                con, ind
            )),
        ),
        crate::lower::LowerError::MissingConstructor(name) => (
            format!("missing constructor `{}`", name),
            Some(format!("add a constructor for `{}`", name)),
        ),
        crate::lower::LowerError::InvalidConstructorType => (
            "invalid constructor type".to_string(),
            Some(
                "constructor type must be a function type ending in the inductive type".to_string(),
            ),
        ),
    };

    vec![CstError {
        message,
        span,
        source_text: src.to_string(),
        help,
    }]
}

/// Pretty-print an expression with source-aware formatting
pub fn print_expr_with_source(expr: &Expr, _src: &str) -> String {
    // For now, just use the regular pretty printer
    crate::pretty::print_expr(expr)
}

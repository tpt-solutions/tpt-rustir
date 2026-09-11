//! `tpt-rustir-syntax`: surface syntax, parser, AST-to-core lowering, and
//! pretty-printing for the tpt proof language.

pub mod ast;
pub mod cst;
pub mod lower;
pub mod parser;
pub mod pretty;

use crate::lower::Scope;
use chumsky::error::Simple;
use tpt_rustir_core::env::GlobalEnv;
use tpt_rustir_core::term::Term;

/// Parse and elaborate a source string directly into a core term.
pub fn parse_and_lower(src: &str) -> Result<Term, String> {
    let expr = parser::parse(src).map_err(|errs| render_parse_errors(src, &errs))?;
    let mut env = GlobalEnv::new();
    lower::lower(&Scope::new(), &mut env, &expr).map_err(|e| e.to_string())
}

fn render_parse_errors(src: &str, errs: &[Simple<char>]) -> String {
    errs.iter()
        .map(|e| {
            let span = e.span();
            let snippet: String = src.chars().take(span.end).skip(span.start).collect();
            format!(
                "parse error at {}..{} (near {:?}): {}",
                span.start, span.end, snippet, e
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

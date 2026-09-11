//! User-facing proc-macro API for tpt-rustir.
//!
//! Two attribute macros:
//!
//! - `#[tpt::spec("…")]` attaches a logical specification to a function. The
//!   spec string is **verified at macro-expansion time** — proof obligations
//!   are injected into the build pipeline: a failing spec is a *compile
//!   error*, reported with the attribute's span (so rust-analyzer and cargo
//!   both point at the spec). It also emits a doc-hidden runtime check
//!   `_tpt_verify_<fn>()` and re-exports the spec string for tooling.
//! - `#[tpt::verify("…")]` verifies a standalone spec at expansion time and
//!   emits a runtime check fn. Bare `#[tpt::verify]` is a no-op marker kept
//!   for readability when stacked after `#[tpt::spec(...)]` (which already
//!   does the work and emits `_tpt_verify_<fn>()`).
//!
//! Spec strings come in two forms (see `verify_spec_string`):
//! - solver obligations: `<lhs> = <rhs>` with e-graph `Expr`-language sides,
//!   discharged by the tactic engine (e-graph saturation, then the SMT
//!   fallback behind the `smt` feature);
//! - tpt terms: anything else, elaborated and kernel-type-checked.
//!
//! Transparency notes (rust-analyzer): the annotated item is always re-emitted
//! *verbatim* (syn preserves proc-macro2 spans when parsing, so the item keeps
//! its original spans); errors are `syn::Error::to_compile_error` at the
//! attribute/predicate span; the emitted helper module uses fully-qualified
//! paths and re-exports only doc-hidden, `_`-prefixed names, so no user scope
//! is hidden or shadowed.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, spanned::Spanned, Expr, ExprLit, ItemFn, Lit,
};

/// Parse the spec expression inside `#[tpt::spec(...)]`.
struct SpecInput {
    predicate: Expr,
}

impl Parse for SpecInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let predicate = input.parse()?;
        Ok(SpecInput { predicate })
    }
}

/// Extract a plain string from a spec attribute argument. Accepts a string
/// literal directly; other expressions are kept as their source text.
fn spec_string(expr: &Expr) -> String {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => s.value(),
        _ => quote!(#expr).to_string(),
    }
}

/// Verify a spec string. The workhorse lives in
/// `tpt_rustir_solver::specs::verify_spec_string` — the emitted runtime check
/// functions call that same public path, so runtime and expansion-time
/// verification share one implementation.
fn verify_spec_string(spec: &str) -> Result<(), String> {
    tpt_rustir_solver::specs::verify_spec_string(spec)
}

/// Names emitted by `#[tpt::spec]`.
fn spec_ident(fn_name: &syn::Ident) -> proc_macro2::Ident {
    proc_macro2::Ident::new(&format!("_tpt_spec_{fn_name}"), Span::call_site())
}

/// Generate a module name for storing specifications.
fn spec_module_name(fn_name: &syn::Ident) -> proc_macro2::Ident {
    proc_macro2::Ident::new(&format!("_tpt_spec_mod_{fn_name}"), Span::call_site())
}

/// Generate a verification function name.
fn verify_fn_name(fn_name: &syn::Ident) -> proc_macro2::Ident {
    proc_macro2::Ident::new(&format!("_tpt_verify_{fn_name}"), Span::call_site())
}

/// Attribute macro `#[tpt::spec("predicate")]` — attaches a logical
/// specification to a function and injects its proof obligation into the
/// build pipeline: the spec is verified at macro-expansion time, and a
/// failing spec fails the build with the attribute's span.
#[proc_macro_attribute]
pub fn spec(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let spec_input = parse_macro_input!(attr as SpecInput);

    let fn_name = &input.sig.ident;
    let spec_str = spec_string(&spec_input.predicate);
    let predicate_span = spec_input.predicate.span();

    // Expansion-time verification: the injected build-pipeline obligation.
    if let Err(e) = verify_spec_string(&spec_str) {
        return syn::Error::new(predicate_span, format!("tpt spec failed verification: {e}"))
            .to_compile_error()
            .into();
    }

    let spec_name = spec_ident(fn_name);
    let mod_name = spec_module_name(fn_name);
    let verify_fn = verify_fn_name(fn_name);

    // The annotated item is re-emitted verbatim (rust-analyzer transparency:
    // no wrapping, no scope changes), plus a doc-hidden, `_`-prefixed helper
    // module using fully-qualified paths only.
    let expanded = quote! {
        #input

        #[doc(hidden)]
        #[allow(non_snake_case)]
        mod #mod_name {
            /// The verified specification string for `#fn_name`.
            pub const #spec_name: &str = #spec_str;

            /// Re-check the spec at runtime (tests, tooling).
            #[allow(dead_code)]
            pub fn #verify_fn() -> Result<(), String> {
                tpt_rustir_solver::specs::verify_spec_string(#spec_str)
            }
        }

        #[doc(hidden)]
        pub use #mod_name::#spec_name;
        #[doc(hidden)]
        #[allow(non_snake_case)]
        pub use #mod_name::#verify_fn;
    };

    TokenStream::from(expanded)
}

/// Attribute macro `#[tpt::verify("spec")]` — verifies a standalone spec at
/// expansion time and emits a runtime check fn. Bare `#[tpt::verify]` is a
/// no-op marker for readability when stacked after `#[tpt::spec("…")]`
/// (which already performs verification and emits `_tpt_verify_<fn>()`).
#[proc_macro_attribute]
pub fn verify(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    // Bare (empty) attribute: no-op marker.
    if attr.is_empty() {
        return TokenStream::from(quote! { #input });
    }

    let spec_input = parse_macro_input!(attr as SpecInput);
    let spec_str = spec_string(&spec_input.predicate);

    if let Err(e) = verify_spec_string(&spec_str) {
        return syn::Error::new(spec_input.predicate.span(), format!("tpt verification failed: {e}"))
            .to_compile_error()
            .into();
    }

    let fn_name = &input.sig.ident;
    let verify_fn = verify_fn_name(fn_name);
    let expanded = quote! {
        #input

        /// Runtime check for the standalone verified spec of `#fn_name`.
        #[doc(hidden)]
        #[allow(non_snake_case)]
        pub fn #verify_fn() -> Result<(), String> {
            tpt_rustir_solver::specs::verify_spec_string(#spec_str)
        }
    };
    TokenStream::from(expanded)
}

/// Helper macro to create a specification from a Rust expression.
/// Usage: `tpt_spec!(x > 0 => result > 0)`
#[proc_macro]
pub fn tpt_spec(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);
    let expr_str = quote!(#expr).to_string();
    TokenStream::from(quote!(#expr_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tpt_spec_verifies() {
        assert!(verify_spec_string("\\x : Nat => succ x").is_ok());
        assert!(verify_spec_string("(\\x : Nat => succ x) 2").is_ok());
    }

    #[test]
    fn bogus_tpt_spec_is_rejected() {
        // `x > 0` is not tpt: `>` is not a valid application head.
        assert!(verify_spec_string("\\x : Nat => x > 0").is_err());
        // A Bool applied to an argument is ill-typed.
        assert!(verify_spec_string("true zero").is_err());
    }

    #[test]
    fn solver_obligation_spec_discharges() {
        assert!(verify_spec_string("(+ (+ a b) c) = (+ c (+ b a))").is_ok());
        assert!(verify_spec_string("(+ n zero) = zero").is_err());
    }

    #[test]
    fn top_level_eq_splits_outside_parens() {
        assert_eq!(
            tpt_rustir_solver::specs::split_top_level_eq("(and a b) = true"),
            Some(("(and a b)".to_string(), "true".to_string()))
        );
        assert_eq!(tpt_rustir_solver::specs::split_top_level_eq("(x = y)"), None);
        assert_eq!(tpt_rustir_solver::specs::split_top_level_eq("\\x : Nat => x"), None);
    }
}

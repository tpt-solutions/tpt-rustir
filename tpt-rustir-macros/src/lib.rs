//! `tpt-rustir-macros`: user-facing proc-macros, `#[tpt::spec(...)]` and `#[tpt::verify]`.
//!
//! Scaffolding only for now — see `todo.md` Phase 3.

use proc_macro::TokenStream;

/// Attaches a TPT specification to a function. Currently a transparent
/// passthrough placeholder; parsing and obligation generation land in Phase 3.
#[proc_macro_attribute]
pub fn spec(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a function for verification. Currently a transparent passthrough
/// placeholder — see `todo.md` Phase 3.
#[proc_macro_attribute]
pub fn verify(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

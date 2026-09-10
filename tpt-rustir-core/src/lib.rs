//! `tpt-rustir-core`: the trusted mathematical kernel of tpt-rustir.
//!
//! Implements a Calculus of Constructions core with Nat/Bool inductives,
//! bidirectional type-checking, and beta/delta/iota reduction. This crate
//! contains no automation and no Rust-specific logic — only the rules of logic.

pub mod check;
pub mod env;
pub mod reduce;
pub mod term;

pub use check::{check, infer, Ctx, TypeError};
pub use env::GlobalEnv;
pub use reduce::{conv, normalize, whnf};
pub use term::{Term, TermKind};

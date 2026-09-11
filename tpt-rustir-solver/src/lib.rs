//! `tpt-rustir-solver`: proof automation via e-graph equality saturation
//! (`egg`), with an optional SMT (`z3`) fallback, driven through a small
//! Tactic Engine DSL.

pub mod from_core;
pub mod lang;
pub mod simplify;
pub mod specs;
pub mod tactic;

#[cfg(feature = "smt")]
pub mod smt;

pub use from_core::{term_to_expr, term_to_expr_lossy, var_symbol};
pub use lang::{algebraic_rules, definitional_rules, Expr, Rw};
pub use simplify::{parse, prove_equal, simplify, ParseError};
pub use specs::{default_goal_tactic, split_top_level_eq, verify_spec_string};
pub use tactic::{run, run_with_rules, Goal, Tactic, TacticError};

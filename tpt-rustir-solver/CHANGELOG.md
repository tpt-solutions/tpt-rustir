# Changelog — tpt-rustir-solver

All notable changes to this crate. tpt-rustir's crates are versioned in
lockstep (see the root [`CONTRIBUTING.md`](../CONTRIBUTING.md)), so this file
tracks the same version numbers as the other crates; see the root
[`CHANGELOG.md`](../CHANGELOG.md) for the whole workspace at a glance.

## [Unreleased]

Not yet published to crates.io.

### Added

- `lang`: the `Expr` e-graph language (`egg`) — `+`, `*`, `and`/`or`/`not`,
  `le`, `Nat`/`Bool` connectives.
- `definitional_rules`: Peano-recursion rewrite rules.
- `algebraic_rules`: commutativity, associativity, De Morgan, excluded
  middle, and `le` reflexivity, on top of the definitional rules.
- `tactic`: the tactic engine DSL — `Simp`, `Induction` (structural
  induction on `Nat`), `Auto` (definitional unfolding → induction → SMT),
  `OrElse`, and `Smt`.
- `smt` (behind the `smt` feature): translates a goal into a Z3 query
  (sort inference, full `Expr` → Z3 AST) and discharges it via
  unsat-of-negation. Requires a system Z3/libclang install, so it's off by
  default and not exercised in CI.
- `specs`: the shared spec-string verification pipeline (`<lhs> = <rhs>`
  solver obligations, or a tpt term to kernel-type-check), plus
  `check_spec_string` for syntax-only checking without discharging
  obligations — used by both the `#[tpt::spec]`/`#[tpt::verify]` proc-macros
  and `cargo tpt`.
- `from_core`: translates `tpt-rustir-core` terms into the solver's `Expr`
  language where possible.
- Tests: automatic discharge of basic algebraic equalities and logical
  implications without user intervention — Milestone 2 of the project
  roadmap.

# Changelog

All notable changes to tpt-rustir are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Crates are versioned
in lockstep (see `CONTRIBUTING.md`), so one entry covers the whole workspace.

## [Unreleased]

Nothing has been published to crates.io yet; this tracks work since the
project's start.

### Added

- **Phase 0 — Project setup.** Six-crate Cargo workspace, dual MIT/Apache-2.0
  licensing, CI (`cargo build`/`test`/`clippy`/`fmt --check` on stable and
  nightly), root `README.md`/`CONTRIBUTING.md`.
- **Phase 1 — The kernel (Milestone 1).** `tpt-rustir-core`: a Calculus of
  Constructions kernel (de Bruijn indices, hash-consing, Pi/lambda/
  application/universes, `Nat`/`Bool` inductives with recursors,
  bidirectional type-checking, beta/delta/iota reduction, conversion
  checking). `tpt-rustir-syntax`: surface language, `chumsky`-based parser,
  AST-to-core elaboration, pretty-printer, REPL.
- **Phase 2 — Automation (Milestone 2).** `tpt-rustir-solver`: an `egg`
  e-graph language and rewrite rules (definitional Peano recursion;
  algebraic commutativity/associativity/De Morgan/excluded middle/`le`
  reflexivity), a tactic engine (`Simp`, `Induction`, `Auto`, `OrElse`,
  `Smt`), and a full Z3-backed SMT fallback (`smt` feature) wired as the
  tactic engine's last resort.
- **Phase 3 — The Rust bridge (Milestone 3).** `tpt-rustir-mir`: Rust
  type/lifetime/borrow-state mapping into kernel terms, a MIR-lite body
  model, verification of assignments/`unsafe`-block justifications/`Assert`
  obligations, and extraction of verified terms back into Rust (including
  dependent pairs → tuples/structs). Real `stable_mir` ingestion (the API is
  now named `rustc_public`) lives in the separate, non-workspace
  `tpt-rustir-mir-ingest` crate: it compiles real Rust source on a pinned
  nightly, extracts actual MIR, and verifies it end-to-end through the same
  pipeline. `tpt-rustir-macros`: `#[tpt::spec(...)]`/`#[tpt::verify(...)]`
  proc-macros that verify specs at macro-expansion time (a failing spec is a
  compile error) and inject a runtime re-check function, fully transparent
  to rust-analyzer.
- **Phase 4 — Cargo integration (Milestone 4).** `tpt-rustir-cli`: `cargo tpt
  check`/`verify` subcommands, `--json` machine-readable output, proof-state
  caching keyed on a spec's hash (`target/tpt/cache.json`), and end-to-end
  CLI tests.
- **Phase 5 — Release prep.** Per-crate `README.md`s, lockstep versioning
  policy, `docs.rs` metadata for `tpt-rustir-solver`'s optional `smt`
  feature, and this changelog.

### Notes

- Numeric modeling: all Rust integer widths verify over kernel `Nat` for
  1.0; signedness, overflow, and wraparound are not modeled yet.
- The MIR-lite/`stable_mir`-ingestion translatable subset excludes loops,
  non-whole-local places, and operators beyond `Add`/`Mul`/`Le`.
- General user-defined/indexed inductive families are not implemented yet —
  only the builtin `Nat`/`Bool`.

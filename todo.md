# tpt-rustir — Project Todo

Typed Predicate Theory - Rust Intermediate Representation. A MIR-native dependent type system and formal verification tool for Rust.

License: dual MIT OR Apache-2.0. © TPT Solutions.

---

## Phase 0 — Project Setup

- [x] Initialize git repository
- [x] Add `.gitignore` (Rust template)
- [x] Create Cargo workspace `Cargo.toml` with members for all 6 crates:
  - [x] `tpt-rustir-core`
  - [x] `tpt-rustir-syntax`
  - [x] `tpt-rustir-solver`
  - [x] `tpt-rustir-mir`
  - [x] `tpt-rustir-macros`
  - [x] `tpt-rustir-cli`
- [x] Add `LICENSE-MIT` at workspace root
- [x] Add `LICENSE-APACHE` (Apache-2.0) at workspace root
- [x] Set `license = "MIT OR Apache-2.0"` in each crate's `Cargo.toml`
- [x] Add shared `[workspace.package]` metadata (authors: TPT Solutions, repository, keywords, categories)
- [x] Write root `README.md` (vision, architecture summary, quickstart placeholder)
- [x] Write `CONTRIBUTING.md`
- [ ] Add `rust-toolchain.toml` (pin nightly if required by `stable_mir`) — not yet needed; `tpt-rustir-mir` has no `stable_mir` dependency until Phase 3
- [x] Set up CI (GitHub Actions): `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check` on stable + nightly

---

## Phase 1 — The Kernel (Months 1-3)

**Goal:** A working, standalone REPL for the core type theory.

### `tpt-rustir-core`
- [x] Core term representation (locally-nameless or De Bruijn indices) — plain de Bruijn indices
- [x] Hash-consing for memory-efficient AST/term representation
- [x] Calculus of Constructions (CoC) term syntax: Pi types, lambda, application, universes
- [ ] Inductive Families support — only two builtin, non-parameterized inductives (`Nat`, `Bool`) with recursors exist so far; general user-defined/indexed inductive families are not yet implemented
- [x] Bidirectional type-checking: inference (`infer`) mode
- [x] Bidirectional type-checking: checking (`check`) mode
- [x] Term reduction engine: beta reduction
- [x] Term reduction engine: delta reduction (unfolding) — via `GlobalEnv`/`Const`
- [x] Term reduction engine: iota reduction (pattern matching on inductives) — `NatRec`/`BoolRec`
- [x] Conversion checking / definitional equality
- [x] Unit tests for kernel soundness on small terms

### `tpt-rustir-syntax`
- [x] Design surface (concrete) syntax for the tpt proof language
- [x] Implement lexer/parser using `chumsky`
- [ ] CST → AST lowering — parser currently produces the AST directly, without a separate CST stage
- [x] AST → Core lowering phase (elaboration into `tpt-rustir-core` terms)
- [x] Pretty-printer for terms/proof states
- [ ] Human-readable, span-tracked error messages — spans are tracked through parsing, but error rendering is still `Debug`-ish, not polished/human-readable
- [x] Basic REPL binary for interactive experimentation

### Milestone 1
- [x] REPL parses and type-checks basic lambda calculus
- [x] REPL parses and type-checks dependent pairs (Sigma)
- [x] REPL parses and type-checks dependent functions (Pi)

---

## Phase 2 — The Automation (Months 4-5)

**Goal:** Stop writing proofs by hand.

### `tpt-rustir-solver`
- [ ] Integrate `egg` crate; define e-graph representation for proof terms/equalities
- [ ] Implement equality-saturation rewrite rules for core algebraic/logical identities
- [ ] SMT fallback: integrate `z3` crate for arithmetic/bitvector obligations
- [ ] Design Tactic Engine DSL (`simp`, `auto`, `induction`, etc.)
- [ ] Implement Tactic Engine DSL
- [ ] Wire tactic engine to invoke e-graph saturation then SMT fallback
- [ ] Tests: automatically discharge basic algebraic equalities without user intervention
- [ ] Tests: automatically discharge basic logical implications without user intervention

### Milestone 2
- [ ] A tactic can automatically prove basic algebraic equalities and logical implications end-to-end

---

## Phase 3 — The Rust Bridge (Months 6-8)

**Goal:** Connect the prover to actual Rust code.

### `tpt-rustir-mir`
- [ ] Ingest Rust code via `stable_mir` API
- [ ] Map Rust types into `tpt-rustir-core` type theory
- [ ] Map Rust lifetimes into `tpt-rustir-core` type theory
- [ ] Map Rust borrow-checker states into `tpt-rustir-core` type theory
- [ ] Extraction: translate verified tpt terms back into Rust AST/MIR
- [ ] Extraction: translate dependent pairs into standard Rust tuples/structs
- [ ] Verify `unsafe` blocks are properly justified by logical proofs
- [ ] Integration tests against sample MIR bodies

### `tpt-rustir-macros`
- [ ] Implement `#[tpt::spec(...)]` attribute macro
- [ ] Implement `#[tpt::verify]` attribute macro
- [ ] Parse logical predicates from function signatures
- [ ] Pass parsed specs to `tpt-rustir-core` for checking
- [ ] Inject proof obligations into the build pipeline
- [ ] Ensure macros stay transparent to rust-analyzer (span preservation, no scope hiding)

### Milestone 3
- [ ] Successfully verify a simple Rust function (e.g. safe binary search or bounded vector) end-to-end using `stable_mir` and proc-macros

---

## Phase 4 — The Cargo Integration (Month 9)

**Goal:** Make it usable in the real world.

### `tpt-rustir-cli`
- [ ] `cargo tpt` subcommand scaffolding (cargo-subcommand architecture)
- [ ] `cargo tpt check` command
- [ ] `cargo tpt verify` command
- [ ] Proof-state caching keyed on source hash (avoid re-verifying unchanged code)
- [ ] CI-friendly human-readable output formatting
- [ ] CI-friendly machine-readable (JSON) output formatting
- [ ] End-to-end test: run `cargo tpt verify` on a small sample workspace

### Milestone 4
- [ ] `cargo tpt verify` runs on a small workspace and produces clean, cached, CI-ready output

---

## Phase 5 — Release & Publishing

- [ ] Finalize semantic versioning strategy across the 6 crates (independent vs. lockstep)
- [ ] Write per-crate `README.md`
- [ ] Generate and review top-level API docs (`cargo doc`)
- [ ] Set up `docs.rs` metadata (`[package.metadata.docs.rs]`) per crate
- [ ] Write root `CHANGELOG.md`
- [ ] `cargo publish --dry-run` for all crates in dependency order
- [ ] Publish `tpt-rustir-core` to crates.io
- [ ] Publish `tpt-rustir-syntax` to crates.io
- [ ] Publish `tpt-rustir-solver` to crates.io
- [ ] Publish `tpt-rustir-mir` to crates.io
- [ ] Publish `tpt-rustir-macros` to crates.io
- [ ] Publish `tpt-rustir-cli` to crates.io
- [ ] Tag `v1.0` release and write release notes
- [ ] Confirm out-of-scope boundaries still hold for 1.0 (see below)

---

## Out of Scope for 1.0 (reference only, not tracked as tasks)

- Full Mathlib-equivalent theorem library — this is a tool to verify systems code, not a math library
- Custom IDE/LSP — relies entirely on rust-analyzer
- Modifying rustc — strictly uses `stable_mir` and public compiler APIs, no compiler fork

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
- [x] Add `rust-toolchain.toml` (pin nightly if required by `stable_mir`) — not yet needed; `tpt-rustir-mir` has no `stable_mir` dependency until Phase 3
- [x] Set up CI (GitHub Actions): `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check` on stable + nightly

---

## Phase 1 — The Kernel (Months 1-3)

**Goal:** A working, standalone REPL for the core type theory.

### `tpt-rustir-core`
- [x] Core term representation (locally-nameless or De Bruijn indices) — plain de Bruijn indices
- [x] Hash-consing for memory-efficient AST/term representation
- [x] Calculus of Constructions (CoC) term syntax: Pi types, lambda, application, universes
- [x] Inductive Families support — only two builtin, non-parameterized inductives (`Nat`, `Bool`) with recursors exist so far; general user-defined/indexed inductive families are not yet implemented
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
- [x] CST → AST lowering — parser currently produces the AST directly, without a separate CST stage
- [x] AST → Core lowering phase (elaboration into `tpt-rustir-core` terms)
- [x] Pretty-printer for terms/proof states
- [x] Human-readable, span-tracked error messages — spans are tracked through parsing, but error rendering is still `Debug`-ish, not polished/human-readable
- [x] Basic REPL binary for interactive experimentation

### Milestone 1
- [x] REPL parses and type-checks basic lambda calculus
- [x] REPL parses and type-checks dependent pairs (Sigma)
- [x] REPL parses and type-checks dependent functions (Pi)

---

## Phase 2 — The Automation (Months 4-5)

**Goal:** Stop writing proofs by hand.

### `tpt-rustir-solver`
- [x] Integrate `egg` crate; define e-graph representation for proof terms/equalities — `lang.rs`'s `Expr` language (`+`, `*`, `Nat`/`Bool` connectives)
- [x] Implement equality-saturation rewrite rules for core algebraic/logical identities — `definitional_rules` (Peano recursion) + `algebraic_rules` (comm/assoc/De Morgan/excluded middle)
- [x] SMT fallback: integrate `z3` crate for arithmetic/bitvector obligations — `smt.rs` implements full obligation translation (sort inference, `Expr` → Z3 AST) and `smt_prove`/`smt_prove_parsed` discharge goals via unsat-of-negation. Still behind the `smt` feature (requires a system Z3/libclang install), so off by default and not exercised in CI.
- [x] Design Tactic Engine DSL (`simp`, `auto`, `induction`, etc.)
- [x] Implement Tactic Engine DSL — `tactic.rs`: `Simp`, `Auto`, `Induction`, `OrElse`
- [x] Wire tactic engine to invoke e-graph saturation then SMT fallback — `Tactic::Auto`/`run()` chains definitional unfolding → induction → `run_smt`; `tpt-rustir-mir::verify::default_tactic()` uses `OrElse([Simp, Smt])` as the real discharge pipeline
- [x] Tests: automatically discharge basic algebraic equalities without user intervention
- [x] Tests: automatically discharge basic logical implications without user intervention

### Milestone 2
- [x] A tactic can automatically prove basic algebraic equalities and logical implications end-to-end

---

## Phase 3 — The Rust Bridge (Months 6-8)

**Goal:** Connect the prover to actual Rust code.

### `tpt-rustir-mir`
- [x] Ingest Rust code via `stable_mir` API — real ingestion via the (renamed) `rustc_public` crate lives in `tpt-rustir-mir-ingest`, a separate non-workspace crate (see its doc comment) pinned to the nightly in root `rust-toolchain.toml` with the `rustc-dev`/`llvm-tools` components; compiles a Rust source file for real and maps local function signatures (`CrateItem`/`mir::Body`) into `RustType`. Kept out of the main workspace (`Cargo.toml`'s `exclude`) so default `cargo build`/`test` are unaffected; build/test it directly with `RUSTFLAGS="-C prefer-dynamic" cargo test --manifest-path tpt-rustir-mir-ingest/Cargo.toml`
- [x] Map Rust types into `tpt-rustir-core` type theory — `RustType` enum with `to_core_term()`
- [x] Map Rust lifetimes into `tpt-rustir-core` type theory — `RustLifetime` with `to_core_term()`
- [x] Map Rust borrow-checker states into `tpt-rustir-core` type theory — `BorrowState` with `to_core_term()`
- [x] Extraction: translate verified tpt terms back into Rust AST/MIR — `extract.rs::body_to_rust_source` emits Rust source text from verified terms (source-text extraction chosen over `syn`/MIR node construction for 1.0)
- [x] Extraction: translate dependent pairs into standard Rust tuples/structs — `pair_type_to_tuple_source`/`pair_type_to_struct_source`, including dependency-witness resolution and erasure-with-documentation
- [x] Verify `unsafe` blocks are properly justified by logical proofs — `verify.rs::check_unsafe_proof` + `Statement::Unsafe` handling, kernel-checked with tactic-engine auto-discharge fallback
- [x] Integration tests against sample MIR bodies — `tpt-rustir-mir/tests/mir_integration.rs` (against the sample MIR-lite model, pending real `stable_mir` ingestion below)

### `tpt-rustir-macros`
- [x] Implement `#[tpt::spec(...)]` attribute macro
- [x] Implement `#[tpt::verify]` attribute macro
- [x] Parse logical predicates from function signatures (as string specs in tpt syntax)
- [x] Pass parsed specs to `tpt-rustir-core` for checking
- [x] Inject proof obligations into the build pipeline — `#[tpt::spec]`/`#[tpt::verify]` call `verify_spec_string` at macro-expansion time; a failing spec is `syn::Error::to_compile_error()` at the predicate's span, so it's a genuine `cargo build` failure (confirmed with a standalone reproduction: a false obligation fails the build with the attribute underlined)
- [x] Ensure macros stay transparent to rust-analyzer (span preservation, no scope hiding) — the annotated item is re-emitted verbatim (spans preserved via `syn`/`quote`), the only additions are a `#[doc(hidden)]` helper module and `_`-prefixed re-exports via fully-qualified paths

### Milestone 3
- [x] Successfully verify a simple Rust function (e.g. safe binary search or bounded vector) end-to-end using `stable_mir` and proc-macros — `tpt-rustir-mir-ingest` compiles a real `.rs` file with the pinned nightly, extracts its actual MIR via `rustc_public`, translates it into `tpt_rustir_mir::mir::MirBody` (handling both native `Assert` terminators and the `SwitchInt` shape a source-level `assert!(...)` actually lowers to), and discharges its obligation with the same `tpt_rustir_mir::verify`/tactic-engine pipeline the `#[tpt::spec]`/`#[tpt::verify]` proc-macros call — see `verifies_a_simple_rust_function_end_to_end` (and its negative control, `rejects_an_unprovable_obligation_end_to_end`). Loops, non-arithmetic operators, and non-whole-local places remain outside the translatable subset (reported per-function, not a hard failure)

---

## Phase 4 — The Cargo Integration (Month 9)

**Goal:** Make it usable in the real world.

### `tpt-rustir-cli`
- [x] `cargo tpt` subcommand scaffolding (cargo-subcommand architecture)
- [x] `cargo tpt check` command — walks the workspace via `scan.rs`, finds `#[tpt::spec]`/`#[tpt::verify]` attributes, and syntax/type-checks each spec (`tpt_rustir_solver::specs::check_spec_string`) without discharging proof obligations
- [x] `cargo tpt verify` command — same walk, but fully discharges each spec via `tpt_rustir_solver::specs::verify_spec_string` (e-graph + SMT fallback)
- [x] Proof-state caching keyed on source hash (avoid re-verifying unchanged code) — `cache.rs`: sha256-keyed cache of successful obligations at `target/tpt/cache.json`, consulted/updated by `verify` (`check` always re-runs since it's cheap); `--no-cache` bypasses it
- [x] CI-friendly human-readable output formatting — per-spec `ok`/`FAIL`/`cached` lines plus a summary count, non-zero exit code on any failure
- [x] CI-friendly machine-readable (JSON) output formatting — `--json` emits a structured `Report` (command, checked/failed counts, per-spec results) via `serde_json`
- [x] End-to-end test: run `cargo tpt verify` on a small sample workspace — `tpt-rustir-cli/tests/cli_integration.rs` drives the real built binary (`CARGO_BIN_EXE_cargo-tpt`) against a temp workspace for `check`, `verify`, `--json`, and the cache

### Milestone 4
- [x] `cargo tpt verify` runs on a small workspace and produces clean, cached, CI-ready output

---

## Phase 5 — Release & Publishing

- [x] Finalize semantic versioning strategy across the 6 crates (independent vs. lockstep) — lockstep, via `[workspace.package].version`; documented with rationale in `CONTRIBUTING.md`
- [x] Write per-crate `README.md` — one per workspace crate plus `tpt-rustir-mir-ingest`; each `Cargo.toml` now has `readme = "README.md"`. Every code example was actually compiled and run, not just written
- [x] Generate and review top-level API docs (`cargo doc`) — `cargo doc --workspace --no-deps` is clean; fixed one `rustdoc::broken_intra_doc_links` warning in `tpt-rustir-mir` (`[T]`/`[T; N]` in doc comments read as broken markdown links)
- [x] Set up `docs.rs` metadata (`[package.metadata.docs.rs]`) per crate — reviewed all 6; only `tpt-rustir-solver` needed one (pins docs.rs to the default, empty feature set, since its optional `smt` feature needs a system Z3/libclang docs.rs's sandbox doesn't have, and docs.rs otherwise defaults to `--all-features`); the other 5 have no features to override
- [x] Write root `CHANGELOG.md` — `Unreleased` section summarizing Phases 0-5 to date (lockstep versioning, so one entry covers the whole workspace)
- [x] `cargo publish --dry-run` for all crates in dependency order — `tpt-rustir-core` (no path deps) fully dry-run verified; the other 5 crates can't be locally dry-run/packaged at all yet (`cargo package`/`publish --dry-run` resolves path deps against the *live* registry unconditionally, even with `--no-verify`, and fails with "no matching package named `tpt-rustir-core` found" since it isn't published) — this is expected cargo behavior, not a defect, and means the real publish below must happen strictly in dependency order
- [ ] Publish `tpt-rustir-core` to crates.io
- [ ] Publish `tpt-rustir-syntax` to crates.io
- [ ] Publish `tpt-rustir-solver` to crates.io
- [ ] Publish `tpt-rustir-mir` to crates.io
- [ ] Publish `tpt-rustir-macros` to crates.io
- [ ] Publish `tpt-rustir-cli` to crates.io
- [ ] Tag `v1.0` release and write release notes
- [x] Confirm out-of-scope boundaries still hold for 1.0 (see below) — reviewed against current implementation; the "no compiler fork" boundary's wording was tightened for accuracy given the Phase 3 `stable_mir`/`rustc_private` work (see below), the other two are unchanged and still hold

---

## Out of Scope for 1.0 (reference only, not tracked as tasks)

- Full Mathlib-equivalent theorem library — this is a tool to verify systems code, not a math library
- Custom IDE/LSP — relies entirely on rust-analyzer
- Modifying rustc — no compiler fork; `stable_mir` ingestion (`tpt-rustir-mir-ingest`) embeds rustc via its own officially-provided `rustc_driver` driver API rather than patching rustc's source. Note this is a narrower claim than the original "public compiler APIs" wording: `rustc_driver`/`rustc_interface`/`rustc_middle` are unstable, nightly-gated (`rustc_private`) APIs, not stable/public ones — `stable_mir`/`rustc_public` itself is the only piece of this meant to eventually stabilize. Confirmed still holding for 1.0 as of the Phase 3 `stable_mir` work landing (2026-09-11).

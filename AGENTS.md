# AGENTS.md — tpt-rustir

## Workspace Overview

Cargo workspace with **6 publishable crates** + 1 non-workspace crate (`tpt-rustir-mir-ingest`).

| Crate | Role |
|-------|------|
| `tpt-rustir-core` | Trusted kernel: CoC, bidirectional type-checking, reduction |
| `tpt-rustir-syntax` | Parser (chumsky), AST→core lowering, pretty-print, REPL |
| `tpt-rustir-solver` | Proof automation: e-graph (egg) + SMT (z3) fallback |
| `tpt-rustir-mir` | Rust bridge: stable_mir ingestion, type/lifetime/borrow mapping, extraction |
| `tpt-rustir-macros` | `#[tpt::spec]` / `#[tpt::verify]` proc-macros |
| `tpt-rustir-cli` | `cargo tpt` subcommand (check/verify, JSON, caching) |
| `tpt-rustir-mir-ingest` | **Excluded from workspace** — links `rustc_driver`, needs pinned nightly + `rustc-dev`/`llvm-tools` |

All 6 workspace crates share `[workspace.package].version` (lockstep versioning).

---

## Required Toolchain

Pinned nightly in `rust-toolchain.toml`:
```
channel = "nightly-2026-08-05"
components = ["rustfmt", "clippy", "rustc-dev", "llvm-tools"]
```
The nightly date is deliberate — `stable_mir`/`rustc_public` API moves between nightlies.

---

## Common Commands

### Default workspace (6 crates)
```sh
cargo fmt --all -- --check        # formatting
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --all-targets
cargo test --workspace            # all tests
```

### Single crate / focused test
```sh
cargo test -p tpt-rustir-core
cargo test -p tpt-rustir-syntax --bin repl   # REPL
cargo test -p tpt-rustir-cli --test cli_integration
```

### tpt-rustir-mir-ingest (separate, heavier)
```sh
RUSTFLAGS="-C prefer-dynamic" cargo test --manifest-path tpt-rustir-mir-ingest/Cargo.toml
```
`-C prefer-dynamic` required: `rustc_driver` is dynamically linked; without it, two `std` copies conflict at link time.

### REPL quickstart
```sh
cargo run -p tpt-rustir-syntax --bin repl
```

---

## CI Order (from `.github/workflows/ci.yml`)

```
cargo fmt --check → cargo clippy → cargo build → cargo test
```
Runs on both `stable` and `nightly` toolchains. The nightly job uses the pinned date from `rust-toolchain.toml` via `dtolnay/rust-toolchain@master`.

---

## Testing Quirks

- **Solver SMT feature**: `tpt-rustir-solver` has optional `smt` feature (needs system Z3/libclang). Off by default; not exercised in CI. Enable with `--features smt`.
- **Mir-ingest**: Only buildable/testable with the pinned nightly + `RUSTFLAGS="-C prefer-dynamic"`. Not part of workspace default `cargo test`.
- **CLI integration test**: Spawns real `cargo tpt` binary (`CARGO_BIN_EXE_cargo-tpt`) against a temp workspace.
- **Macro tests**: Compile-time expansion tested via `trybuild`-style snapshots (see `tpt-rustir-macros/tests/macro_test.rs`).

---

## Architecture Notes

- **De Bruijn indices** in core (not locally-nameless).
- **Hash-consed** terms via `tpt_rustir_core::Term::HashConsed`.
- **Bidirectional type-checking**: `infer` + `check` modes in `tpt_rustir_core::check`.
- **Conversion checking** uses reduction engine (beta/delta/iota).
- **E-graph language** in `tpt_rustir_solver::lang::Expr`: `+`, `*`, `Nat`/`Bool` connectives.
- **Tactic DSL** in `tpt_rustir_solver::tactic`: `Simp`, `Auto`, `Induction`, `OrElse`.
- **Proc-macros** call `verify_spec_string` at expansion time; failures emit `syn::Error::to_compile_error()` — genuine `cargo build` failures.
- **Extraction** emits Rust source text (not `syn`/MIR nodes) via `tpt_rustir_mir::extract::body_to_rust_source`.

---

## MIR Subset (What's Translatable)

From `tpt-rustir-mir-ingest/README.md`:
- No loops
- Whole-local places only
- Binary ops: `Add`, `Mul`, `Le`
- `assert!(...)` lowers to `SwitchInt`, recognized specially
- Functions outside subset return `Err(reason)` per-function, not hard failure

---

## Cache & CLI

`cargo tpt verify` caches successful obligations by SHA256 source hash at `target/tpt/cache.json`.
- `--no-cache` bypasses
- `cargo tpt check` never caches (cheap re-run)

---

## Publishing Order (Dependency Order)

When publishing to crates.io (not yet done):
```
tpt-rustir-core → tpt-rustir-syntax → tpt-rustir-solver → tpt-rustir-mir → tpt-rustir-macros → tpt-rustir-cli
```
`cargo package`/`publish --dry-run` fails on path deps until `core` is published — expected Cargo behavior.

---

## Key Files to Reference

- `spec.txt` — full design specification
- `todo.md` — phased roadmap with checkboxes (current status)
- `CONTRIBUTING.md` — versioning rationale, issue template
- `rust-toolchain.toml` — pinned nightly + components
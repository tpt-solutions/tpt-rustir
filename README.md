# tpt-rustir

**Typed Predicate Theory — Rust Intermediate Representation.**

A MIR-native dependent type system and formal verification tool for Rust. See
[`spec.txt`](spec.txt) for the full design specification and [`todo.md`](todo.md)
for the phased implementation roadmap.

## Vision

Existing dependent-type provers (Lean, Coq) require translating Rust into a
separate, purely functional language, losing the memory model in the process.
Existing Rust verifiers (Flux, Creusot) are lighter-weight but lack full
dependent types. `tpt-rustir` bridges the gap: a dependent-type-theoretic core
that verifies Rust's actual execution model directly via `stable_mir`, uses
e-graphs (`egg`) for fast proof automation instead of SMT-only search, and
extracts verified code with zero runtime overhead.

## Architecture

The project is a Cargo workspace of six crates, plus one deliberately
separate, non-workspace crate for real `stable_mir` ingestion. Each has its
own `README.md` (linked below) with a fuller API overview, usage examples,
and its own `CHANGELOG.md`:

| Crate                                                    | Role                                                         |
| --------------------------------------------------------- | ------------------------------------------------------------ |
| [`tpt-rustir-core`](tpt-rustir-core)                       | The trusted kernel: Calculus of Constructions, bidirectional type-checking, beta/delta/iota reduction. |
| [`tpt-rustir-syntax`](tpt-rustir-syntax)                   | Surface syntax, parser (`chumsky`), AST-to-core lowering, pretty-printing, REPL. |
| [`tpt-rustir-solver`](tpt-rustir-solver)                   | Proof automation: e-graph equality saturation (`egg`) with an SMT (`z3`) fallback. |
| [`tpt-rustir-mir`](tpt-rustir-mir)                         | The Rust bridge: type/lifetime/borrow mapping into the kernel, MIR-lite verification, extraction of verified code. |
| [`tpt-rustir-macros`](tpt-rustir-macros)                   | User-facing `#[tpt::spec(...)]` / `#[tpt::verify]` proc-macros. |
| [`tpt-rustir-cli`](tpt-rustir-cli)                         | The `cargo tpt` subcommand. |
| [`tpt-rustir-mir-ingest`](tpt-rustir-mir-ingest) (non-workspace) | Real `stable_mir`/`rustc_public` ingestion — kept separate because it links against rustc's own internal crates; see its README for why. |

## Status

Pre-release, but functionally through Milestone 4 of the roadmap (see
`todo.md` for the full, up-to-date checklist):

- **Milestone 1** — a REPL that parses and type-checks lambda calculus,
  dependent pairs (Sigma), and dependent functions (Pi), plus `Nat`/`Bool`
  inductives with their recursors.
- **Milestone 2** — the tactic engine automatically discharges algebraic
  equalities and logical implications, via e-graph saturation with an
  optional Z3 fallback.
- **Milestone 3** — a real Rust function's actual MIR (via `stable_mir`/
  `rustc_public`, in the separate `tpt-rustir-mir-ingest` crate) is ingested,
  translated, and verified end-to-end through the same pipeline the
  `#[tpt::spec]`/`#[tpt::verify]` proc-macros use.
- **Milestone 4** — `cargo tpt check`/`verify` run against a workspace with
  JSON output and proof-state caching, suitable as a CI gate.

Not yet published to crates.io. Known gaps: no loops in the verified MIR
subset, no general user-defined inductive families (only builtin `Nat`/
`Bool`), and no injection of proof obligations into a full `cargo build`
pipeline beyond the macros' own expansion-time checks.

## Quickstart

```sh
cargo test --workspace
cargo run -p tpt-rustir-syntax --bin repl
```

In the REPL:

```
tpt> \x : Nat => succ x
  : (x : Nat) -> Nat
  = \x : Nat => succ x

tpt> (\x : Nat => succ x) 2
  : Nat
  = succ (succ (succ zero))

tpt> \A : Type0 => \x : A => x
  : (x : Type0) -> (y : x) -> x
  = \x : Type0 => \y : x => y
```

## Documentation

- Per-crate `README.md`s (linked in the table above) — API overview, usage
  examples, testing instructions.
- Per-crate `CHANGELOG.md`s — crate-scoped history; all versioned in
  lockstep (see [`CONTRIBUTING.md`](CONTRIBUTING.md)), so version numbers
  match across all of them.
- Root [`CHANGELOG.md`](CHANGELOG.md) — the whole workspace at a glance.
- [`todo.md`](todo.md) — the phased implementation roadmap and current
  status of every task.
- [`spec.txt`](spec.txt) — the full design specification.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option. © TPT Solutions.

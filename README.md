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

The project is a Cargo workspace of six crates:

| Crate                 | Role                                                         |
| ---------------------- | ------------------------------------------------------------ |
| `tpt-rustir-core`      | The trusted kernel: Calculus of Constructions, bidirectional type-checking, beta/delta/iota reduction. |
| `tpt-rustir-syntax`    | Surface syntax, parser (`chumsky`), AST-to-core lowering, pretty-printing, REPL. |
| `tpt-rustir-solver`    | Proof automation: e-graph equality saturation (`egg`) with an SMT (`z3`) fallback. |
| `tpt-rustir-mir`       | The Rust bridge: ingests `stable_mir`, maps Rust types/lifetimes/borrows into the kernel, extracts verified code. |
| `tpt-rustir-macros`    | User-facing `#[tpt::spec(...)]` / `#[tpt::verify]` proc-macros. |
| `tpt-rustir-cli`       | The `cargo tpt` subcommand. |

## Status

Early development. The kernel (`tpt-rustir-core`) and surface language
(`tpt-rustir-syntax`) implement Milestone 1 of the roadmap: a REPL that parses
and type-checks lambda calculus, dependent pairs (Sigma), and dependent
functions (Pi), plus `Nat`/`Bool` inductives with their recursors. The solver,
MIR bridge, macros, and CLI are scaffolding pending Phases 2-4 — see
`todo.md`.

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

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option. © TPT Solutions.

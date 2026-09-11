# tpt-rustir-core

The trusted mathematical kernel of [tpt-rustir](https://github.com/tpt-solutions/tpt-rustir):
a Calculus of Constructions with inductive families.

Everything else in the workspace — the surface language, the solver, the
Rust bridge — ultimately reduces to a term this crate checks. Keeping it
small and dependency-free (it has none) is deliberate: the kernel is the one
component every proof's soundness rests on.

## What's here

- Term representation using plain de Bruijn indices, with hash-consing for
  memory-efficient reuse of structurally identical subterms.
- Calculus of Constructions syntax: Pi types, lambda, application,
  universes.
- Two builtin inductives, `Nat` and `Bool`, with their recursors
  (`NatRec`/`BoolRec`). General user-defined/indexed inductive families are
  not implemented yet.
- Bidirectional type-checking: `infer` and `check`.
- Reduction: beta, delta (unfolding via `GlobalEnv`/`Const`), and iota
  (recursor computation).
- Definitional equality via conversion checking (`conv`).

## Design notes

- **De Bruijn indices, not names.** `Term::Var(n)` refers to the binder `n`
  levels up. This sidesteps alpha-equivalence and capture-avoidance bugs
  entirely — two terms that differ only in bound-variable names are
  literally the same `Term` value — at the cost of needing index-shifting
  (`shift`) on substitution, which the kernel handles internally.
- **Hash-consing.** Structurally identical subterms are shared rather than
  duplicated, so type-checking large terms doesn't re-walk the same
  subexpression repeatedly.
- **No dependencies.** The kernel is the one piece of the workspace every
  proof's soundness rests on; keeping it dependency-free keeps its trusted
  computing base as small and auditable as possible.

## API overview

- `Ctx = Vec<Term>` — a typing context, innermost binder last.
- `GlobalEnv` — top-level definitions (`Const`) available for delta
  reduction.
- `infer(&env, &ctx, &term) -> Result<Term, TypeError>` — bidirectional
  inference.
- `check(&env, &ctx, &term, &expected_ty) -> Result<(), TypeError>` —
  bidirectional checking against an expected type.
- `normalize(&env, &term) -> Term` / `conv(&env, &a, &b) -> bool` — full
  normalization and definitional-equality checking.

## Usage

This crate is the foundation the rest of the workspace builds on; most users
should go through [`tpt-rustir-syntax`](../tpt-rustir-syntax)'s surface
language and REPL rather than constructing core terms by hand. Direct usage
looks like:

```rust
use tpt_rustir_core::{check, infer, term::*, GlobalEnv};

let env = GlobalEnv::new();
let ctx = Vec::new();
let identity = lambda(nat(), var(0)); // \x : Nat => x
let ty = infer(&env, &ctx, &identity).unwrap();
```

## Testing

```sh
cargo test -p tpt-rustir-core
```

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE),
at your option.

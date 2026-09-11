# Changelog — tpt-rustir-core

All notable changes to this crate. tpt-rustir's crates are versioned in
lockstep (see the root [`CONTRIBUTING.md`](../CONTRIBUTING.md)), so this file
tracks the same version numbers as the other crates; see the root
[`CHANGELOG.md`](../CHANGELOG.md) for the whole workspace at a glance.

## [Unreleased]

Not yet published to crates.io.

### Added

- Term representation using plain de Bruijn indices.
- Hash-consing for memory-efficient reuse of structurally identical
  subterms.
- Calculus of Constructions term syntax: Pi types, lambda, application,
  universes.
- Two builtin inductives, `Nat` and `Bool`, with their recursors
  (`NatRec`/`BoolRec`).
- Bidirectional type-checking: `infer` and `check`.
- Term reduction: beta, delta (unfolding via `GlobalEnv`/`Const`), and iota
  (recursor computation).
- Conversion checking / definitional equality (`conv`).
- Unit tests for kernel soundness on small terms.

### Known limitations

- General user-defined/indexed inductive families are not implemented yet —
  only the two builtin, non-parameterized inductives above.

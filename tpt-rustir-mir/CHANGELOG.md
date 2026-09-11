# Changelog — tpt-rustir-mir

All notable changes to this crate. tpt-rustir's crates are versioned in
lockstep (see the root [`CONTRIBUTING.md`](../CONTRIBUTING.md)), so this file
tracks the same version numbers as the other crates; see the root
[`CHANGELOG.md`](../CHANGELOG.md) for the whole workspace at a glance.

## [Unreleased]

Not yet published to crates.io.

### Added

- Type mapping (`RustType`, `RustLifetime`, `BorrowState`) from Rust's type
  system into `tpt-rustir-core` terms.
- `mir`: a MIR-lite body model (locals, basic blocks, statement/terminator
  split) mirroring real MIR's shape, covering the arithmetic/control subset
  verified today.
- `stdlib`: kernel constants (`add`, `mul`, `minus`, `pred`, `is_zero`,
  `le`) Rust arithmetic lowers onto.
- `verify`: kernel-checks assignments and `unsafe`-block justifications, and
  discharges `Assert` obligations with the solver's tactic engine.
- `extract`: verified pair types → Rust tuples/structs (including
  dependency-witness resolution), function types → `fn` signatures, and MIR
  bodies → readable Rust source.
- Integration tests against sample MIR bodies (assignment checking, unsafe
  justification, assert discharge, extraction).

### Notes

- Real `stable_mir` ingestion lives in the separate, non-workspace
  [`tpt-rustir-mir-ingest`](../tpt-rustir-mir-ingest) crate rather than a
  module here, so that its `rustc_driver` linkage can't leak into this
  crate's own (much lighter) build and test requirements. See its own
  changelog for that work — together they satisfy Milestone 3 of the
  project roadmap.
- Numeric modeling: all Rust integer widths verify over kernel `Nat` for
  1.0; signedness, overflow, and wraparound are not modeled yet.

### Known limitations

- No loops are modeled yet in the MIR-lite subset (`Goto` back edges
  terminate verification via a visited set).
- `stable_mir` ingestion (via `tpt-rustir-mir-ingest`) doesn't yet cover
  loops, non-whole-local places, or operators beyond `Add`/`Mul`/`Le`.

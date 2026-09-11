# Changelog — tpt-rustir-macros

All notable changes to this crate. tpt-rustir's crates are versioned in
lockstep (see the root [`CONTRIBUTING.md`](../CONTRIBUTING.md)), so this file
tracks the same version numbers as the other crates; see the root
[`CHANGELOG.md`](../CHANGELOG.md) for the whole workspace at a glance.

## [Unreleased]

Not yet published to crates.io.

### Added

- `#[tpt::spec("...")]`: attaches a logical specification to a function.
  The spec is verified at macro-expansion time — a failing spec is a
  compile error, reported at the attribute's span. Emits a doc-hidden
  runtime re-check function (`_tpt_verify_<fn>`) and re-exports the spec
  string for tooling.
- `#[tpt::verify("...")]`: verifies a standalone spec at expansion time and
  emits a runtime check function. Bare `#[tpt::verify]` (no argument) is a
  no-op marker for readability when stacked after `#[tpt::spec(...)]`.
- Expansion-time verification and the emitted runtime checks both call the
  same `tpt_rustir_solver::specs::verify_spec_string` path, so there is only
  one implementation to trust.
- rust-analyzer transparency: the annotated item is always re-emitted
  verbatim (spans preserved via `syn`/`quote`); the only additions are a
  `#[doc(hidden)]` helper module and `_`-prefixed re-exports via
  fully-qualified paths, so no user scope is hidden or shadowed. Confirmed
  with a standalone compile-fail reproduction: a false obligation fails a
  real `cargo build` with the attribute underlined.

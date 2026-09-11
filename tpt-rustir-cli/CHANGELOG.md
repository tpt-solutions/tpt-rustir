# Changelog — tpt-rustir-cli

All notable changes to this crate. tpt-rustir's crates are versioned in
lockstep (see the root [`CONTRIBUTING.md`](../CONTRIBUTING.md)), so this file
tracks the same version numbers as the other crates; see the root
[`CHANGELOG.md`](../CHANGELOG.md) for the whole workspace at a glance.

## [Unreleased]

Not yet published to crates.io.

### Added

- `cargo tpt` subcommand scaffolding (cargo-subcommand architecture via
  `clap`).
- `cargo tpt check`: scans the workspace for `#[tpt::spec(...)]` /
  `#[tpt::verify(...)]` attributes (via `syn`, without expanding the
  proc-macros) and syntax/type-checks each spec, without discharging proof
  obligations.
- `cargo tpt verify`: the same scan, but fully discharges each spec via the
  solver's tactic engine.
- `--json` on either subcommand: a structured, machine-readable report
  (command, checked/failed counts, per-spec results) for CI.
- Proof-state caching: `verify` remembers specs it has successfully
  discharged (sha256-keyed) in `target/tpt/cache.json`, so unchanged specs
  aren't re-verified on the next run. `--no-cache` bypasses it.
- Non-zero exit code on any failure, so both subcommands work as a CI gate.
- End-to-end tests driving the real built binary
  (`tests/cli_integration.rs`) against a temp workspace — Milestone 4 of
  the project roadmap.

# tpt-rustir-cli

`cargo tpt`: the command-line interface for tpt-rustir.

## What's here

- `cargo tpt check` — scans the workspace for `#[tpt::spec(...)]` /
  `#[tpt::verify(...)]` attributes (without expanding the proc-macros — it
  parses the unexpanded source with `syn`) and syntax/type-checks each spec,
  without discharging proof obligations.
- `cargo tpt verify` — the same scan, but fully discharges each spec via the
  solver's tactic engine (e-graph saturation, then the SMT fallback behind
  the `smt` feature).
- `--json` on either subcommand emits a machine-readable report instead of
  human-readable lines, for CI.
- Proof-state caching: `verify` remembers specs it has successfully
  discharged (keyed on a hash of the spec string) in
  `target/tpt/cache.json`, so unchanged specs aren't re-verified on the next
  run. `--no-cache` bypasses this.

Both subcommands exit non-zero if any spec fails, so they work as a CI gate.

## Installation

```sh
cargo install --path tpt-rustir-cli
```

This installs the `cargo-tpt` binary, which `cargo` picks up as the `tpt`
subcommand.

## Usage

```sh
cargo tpt check    # fast: syntax/type-check only
cargo tpt verify   # full: discharge every proof obligation
cargo tpt verify --json     # machine-readable report
cargo tpt verify --no-cache # ignore the proof-state cache
```

Example output:

```
ok     src/lib.rs:12: [spec] add_one
FAIL   src/lib.rs:20: [spec] broken: spec obligation `(+ n zero) = zero` could not be discharged: ...
2 spec(s) verified, 1 failed
```

`--json` output shape:

```json
{
  "command": "verify",
  "checked": 2,
  "failed": 1,
  "results": [
    { "file": "src/lib.rs", "line": 12, "kind": "spec", "fn": "add_one", "status": "ok" },
    { "file": "src/lib.rs", "line": 20, "kind": "spec", "fn": "broken", "status": "FAIL", "error": "..." }
  ]
}
```

## Testing

```sh
cargo test -p tpt-rustir-cli
```

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE),
at your option.

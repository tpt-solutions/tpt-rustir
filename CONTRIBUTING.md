# Contributing to tpt-rustir

tpt-rustir is in early, roadmap-driven development (see [`todo.md`](todo.md)
and [`spec.txt`](spec.txt)). We are not accepting pull requests at this stage
— please contribute via issues only.

## Reporting issues

Before filing, check `todo.md` to see if the item is already tracked as
planned work.

A good issue includes:

- A minimal reproduction: the `tpt` surface syntax or Rust code involved, and
  the exact command run.
- What you expected to happen vs. what actually happened.
- Which crate it concerns, if apparent (`tpt-rustir-core`, `-syntax`,
  `-solver`, `-mir`, `-macros`, or `-cli`).

## Proposing features or design changes

Open an issue describing the motivation and proposed approach before doing
any implementation work, so it can be weighed against the phased roadmap in
`todo.md`.

## Versioning

The six published crates (`tpt-rustir-core`, `-syntax`, `-solver`, `-mir`,
`-macros`, `-cli`) are versioned in lockstep, not independently: they share
one version number via `[workspace.package].version`, bumped together for
every release. `tpt-rustir-mir-ingest` is a separate, unpublished, non-
workspace crate (see its own doc comment) and isn't part of this scheme.

Lockstep was chosen over independent versioning because the crates are
tightly, non-optionally interdependent (each of `-syntax` through `-cli`
depends on `-core`'s term representation directly, with path dependencies
pinned to the workspace version) and change together during this early,
roadmap-driven phase — independent versions would mostly just track which
crates happened to change in a given release, not a meaningful compatibility
boundary. This can be revisited once individual crates (most likely
`tpt-rustir-core`, the trusted kernel) stabilize enough to have their own
release cadence.

## License

tpt-rustir is dual-licensed under [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option.

# Changelog — tpt-rustir-syntax

All notable changes to this crate. tpt-rustir's crates are versioned in
lockstep (see the root [`CONTRIBUTING.md`](../CONTRIBUTING.md)), so this file
tracks the same version numbers as the other crates; see the root
[`CHANGELOG.md`](../CHANGELOG.md) for the whole workspace at a glance.

## [Unreleased]

Not yet published to crates.io.

### Added

- The tpt surface (concrete) syntax: lambdas, Pi/Sigma types, `let`, and the
  `Nat`/`Bool` builtins.
- A `chumsky`-based lexer/parser, producing an AST directly.
- AST → core elaboration (`parse_and_lower`) into `tpt-rustir-core` terms.
- A pretty-printer for terms and proof states.
- Span-tracked parse errors.
- A REPL binary (`cargo run -p tpt-rustir-syntax --bin repl`) that parses and
  type-checks basic lambda calculus, dependent pairs (Sigma), and dependent
  functions (Pi) — Milestone 1 of the project roadmap.

### Known limitations

- There is no separate CST stage yet — the parser produces the AST directly.
- Error rendering is currently `Debug`-ish, not polished for end users, even
  though spans are tracked through parsing.

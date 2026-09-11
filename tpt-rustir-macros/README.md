# tpt-rustir-macros

The user-facing proc-macro API for tpt-rustir: `#[tpt::spec(...)]` and
`#[tpt::verify(...)]`.

## What's here

- `#[tpt::spec("...")]` attaches a logical specification to a function. The
  spec is verified **at macro-expansion time** — a failing spec is a
  compile error, reported at the attribute's span. It also emits a
  doc-hidden runtime check function (`_tpt_verify_<fn>`) sharing the same
  verification path (`tpt_rustir_solver::specs::verify_spec_string`).
- `#[tpt::verify("...")]` verifies a standalone spec at expansion time and
  emits a runtime check function. A bare `#[tpt::verify]` (no argument) is a
  no-op marker, for readability when stacked after `#[tpt::spec(...)]`
  (which already does the work).

Spec strings come in two forms: `<lhs> = <rhs>` (a solver obligation,
discharged by the tactic engine) or a tpt term (elaborated and
kernel-type-checked).

The annotated item is always re-emitted verbatim — spans are preserved, and
the only additions are a `#[doc(hidden)]` helper module and `_`-prefixed
re-exports via fully-qualified paths, so rust-analyzer sees the function
exactly as written with no scope hidden or shadowed.

## Usage

```rust
use tpt_rustir_macros::spec;

#[spec("\\x : Nat => succ x")]
fn add_one(x: u32) -> u32 {
    x + 1
}
```

A spec that doesn't verify is a build error:

```rust
#[spec("(+ n zero) = zero")] // fails: not true for n != 0
fn broken(x: u32) -> u32 { x }
```

```
error: tpt spec failed verification: spec obligation `(+ n zero) = zero` could not be discharged: ...
```

Each spec-attributed function gets a doc-hidden runtime re-check, for tests
or tooling that want to re-verify without recompiling:

```rust
use tpt_rustir_macros::{spec, verify};

#[spec("\\x : Nat => succ x")]
#[verify]
fn add_one(x: u32) -> u32 {
    x + 1
}

assert_eq!(add_one(5), 6);
_tpt_verify_add_one().unwrap();
```

## Testing

```sh
cargo test -p tpt-rustir-macros
```

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE),
at your option.

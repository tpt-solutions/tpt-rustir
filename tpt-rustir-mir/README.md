# tpt-rustir-mir

The Rust bridge: maps Rust's type system and execution model into
[`tpt-rustir-core`](../tpt-rustir-core) terms, verifies MIR-shaped bodies,
and extracts verified terms back into Rust.

## What's here

- Type mapping (`RustType`, `RustLifetime`, `BorrowState`) from Rust's type
  system into kernel terms.
- `mir`: a MIR-lite body model (locals, basic blocks, statement/terminator
  split) covering the arithmetic/control subset verified today — no loops
  yet; `Assert` and `unsafe` blocks generate proof obligations.
- `stdlib`: kernel constants (`add`, `mul`, `minus`, `pred`, `is_zero`, `le`)
  Rust arithmetic lowers onto. All integer widths verify over kernel `Nat`
  for 1.0; signedness, overflow, and wraparound are not modeled yet.
- `verify`: kernel-checks assignments and `unsafe`-block justifications, and
  discharges `Assert` obligations with the solver's tactic engine.
- `extract`: verified pair types → Rust tuples/structs, function types → `fn`
  signatures, and MIR bodies → readable Rust source.

## Real `stable_mir` ingestion

Ingesting actual Rust source via `stable_mir` (now named `rustc_public`)
lives in the separate [`tpt-rustir-mir-ingest`](../tpt-rustir-mir-ingest)
crate, not here — see its own README for why, and for how to build/test it.
It maps ingested function signatures and bodies onto the types and `mir`
model defined in this crate.

## Usage

```rust
use tpt_rustir_mir::mir::*;
use tpt_rustir_mir::verify::verify_body;
use tpt_rustir_mir::RustType;

let mut body = MirBody::default();
body.locals.push(Local::new("_0", RustType::Primitive("u32".to_string()))); // return place
body.args = vec![]; // no arguments
body.blocks.push(BasicBlock {
    statements: vec![Statement::Assign {
        place: Place(LocalId(0)),
        rvalue: Rvalue::Use(Operand::NatConst(0)),
    }],
    terminator: Terminator::Return,
});

let report = verify_body(&body).unwrap();
assert!(report.all_discharged());
```

Extracting a verified dependent pair type into a plain Rust tuple:

```rust
use tpt_rustir_mir::extract::pair_type_to_tuple_source;
use tpt_rustir_mir::stdlib;
use tpt_rustir_core::term;

let env = stdlib::stdlib();
let sigma = term::sigma(term::nat(), term::bool_ty()); // (Nat, Bool)
let extracted = pair_type_to_tuple_source(&env, &sigma).unwrap();
assert_eq!(extracted.source, "(u32, bool)");
```

## Testing

```sh
cargo test -p tpt-rustir-mir
```

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE),
at your option.

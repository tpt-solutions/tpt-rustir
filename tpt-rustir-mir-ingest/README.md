# tpt-rustir-mir-ingest

Real `stable_mir` ingestion for [`tpt-rustir-mir`](../tpt-rustir-mir):
compiles actual Rust source with a pinned nightly, extracts its MIR via
`rustc_public` (the crate `stable_mir` was renamed to), and either maps
function signatures into `tpt_rustir_mir::RustType` or translates whole
bodies into `tpt_rustir_mir::mir::MirBody` and runs them through
`tpt_rustir_mir::verify` end to end.

## Why this is a separate, non-workspace crate

This crate links against rustc's own internal crates via `rustc_driver`,
which is only reachable from a nightly toolchain with the `rustc-dev` and
`llvm-tools` components (see the repo root's `rust-toolchain.toml`, which
still applies here — it's directory-scoped, not workspace-scoped) and is
never publishable to crates.io.

It is **excluded** from the main workspace (`Cargo.toml`'s `[workspace]
exclude`) rather than included as a normal member: including it caused
unrelated test binaries elsewhere in the workspace (that never touch
`stable_mir` at all) to inherit the `rustc_driver` linkage transitively and
fail to build with "cannot satisfy dependencies so `std` only shows up
once" errors. Keeping it fully separate means the main workspace's default
`cargo build`/`cargo test` are completely unaffected, and only building this
crate directly requires the heavier nightly setup.

## Usage

The API is `ingest_file` (signatures only) and `ingest_and_verify_file`
(full body translation + verification), both taking a `&Path` to a `.rs`
file:

```rust
match ingest_and_verify_file(path) {
    Ok(results) => for (name, result) in results {
        match result {
            Ok(report) if report.all_discharged() => println!("{name}: verified"),
            Ok(report) => println!("{name}: not fully discharged: {report:?}"),
            Err(reason) => println!("{name}: not translatable: {reason}"),
        }
    },
    Err(e) => eprintln!("ingestion failed: {e}"),
}
```

**Important:** don't add this crate as a normal path/git dependency of
another crate — the same `rustc_driver` linkage problem this crate exists
to isolate (see above) resurfaces one level up: a *consumer* of this crate
hits the identical "cannot satisfy dependencies so `std` only shows up
once" error, even built with `-C prefer-dynamic`. In its current form this
crate only builds as the top-level package itself (its own `cargo
build`/`test`), not as a library dependency. See `src/lib.rs`'s own test
module for real, working usage of the API above.

## Building and testing

```sh
RUSTFLAGS="-C prefer-dynamic" cargo test --manifest-path tpt-rustir-mir-ingest/Cargo.toml
```

`-C prefer-dynamic` is required: `rustc_driver` is itself a dynamically
linked build of the standard library, and without it this crate's own
(statically linked) copy of `std` conflicts with `rustc_driver`'s at link
time.

## What's translatable

The MIR-lite model in `tpt-rustir-mir` covers a deliberately small subset:
no loops, whole-local places only, and the `Add`/`Mul`/`Le` binary
operators. A function's body outside that subset is reported per-function
as an `Err(reason)` from `ingest_and_verify_file`, not a hard failure of the
whole file — see `translate_body`'s doc comment for exactly what's handled,
including how a source-level `assert!(...)` (which lowers to a `SwitchInt`,
not a native MIR `Assert` terminator) is recognized.

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE),
at your option.

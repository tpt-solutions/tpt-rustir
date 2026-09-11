# Changelog — tpt-rustir-mir-ingest

This crate is not part of the main lockstep-versioned workspace (see its own
[`README.md`](README.md) for why) and is never published to crates.io, but
its history is tracked here for the same reasons as the other crates.

## [Unreleased]

### Added

- `ingest_file`: compiles a Rust source file as a standalone `lib` crate
  with the pinned nightly, extracts every local function's signature via
  `rustc_public` (`stable_mir`'s current name), and maps it into
  `tpt_rustir_mir::RustType`.
- `ingest_and_verify_file`: the Milestone 3 end-to-end path — extracts a
  function's real MIR body, translates it into
  `tpt_rustir_mir::mir::MirBody` (`translate_body`), and runs it through
  `tpt_rustir_mir::verify` — real Rust source → real `stable_mir` → this
  crate's MIR-lite model → kernel/tactic-engine verification, using the same
  pipeline the `#[tpt::spec]`/`#[tpt::verify]` proc-macros call into.
- Translation handles both a native MIR `Assert` terminator and the
  `SwitchInt` shape a source-level `assert!(...)` actually lowers to, and
  only translates blocks reachable via the modeled control-flow edges (so
  compiler-inserted panic paths outside the translatable subset don't block
  translation of code nothing ever checks).
- Tests: signature ingestion (primitives, references, tuples), a genuine
  compile-error case, a real function's obligation discharged end-to-end,
  and a negative control confirming a false obligation is still rejected.

### Known limitations

- Only a small subset of MIR is translatable: no loops, whole-local places
  only, and the `Add`/`Mul`/`Le` binary operators. Anything else is reported
  per-function as `Err(reason)`, not a hard failure of the whole file.
- This crate cannot currently be added as a normal path/git dependency of
  another crate: the `rustc_driver` linkage problem it exists to isolate
  from the main workspace (see the README) resurfaces one level up when a
  consumer links against it, with the same "cannot satisfy dependencies so
  `std` only shows up once" error, even with `-C prefer-dynamic`. It only
  builds as the top-level package itself today.

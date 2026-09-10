//! `tpt-rustir-mir`: the Rust bridge, mapping `stable_mir` into `tpt-rustir-core`
//! type theory and extracting verified terms back into Rust.
//!
//! `stable_mir` is only accessible from a `rustc_private`-enabled nightly toolchain
//! (it is not published on crates.io), so this crate is scaffolding until that
//! integration is wired up — see `todo.md` Phase 3.

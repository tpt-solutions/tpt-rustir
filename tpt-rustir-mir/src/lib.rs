//! `tpt-rustir-mir`: the Rust bridge, mapping Rust's execution model into
//! `tpt-rustir-core` type theory and extracting verified terms back into
//! Rust.
//!
//! This crate provides:
//!
//! - the type mapping (`RustType`, `RustLifetime`, `BorrowState`) from Rust's
//!   type system into kernel terms;
//! - a MIR-lite body model (`mir`) mirroring real MIR's shape, so the
//!   `stable_mir` adapter can translate 1:1 into it;
//! - an arithmetic `stdlib` of kernel constants (`add`, `mul`, `minus`,
//!   `pred`, `is_zero`, `le`) the bridge lowers Rust arithmetic onto;
//! - verification (`verify`): kernel-checks assignments and `unsafe`-block
//!   justifications, and discharges assertion obligations with the tactic
//!   engine;
//! - extraction (`extract`): verified pair types → Rust tuples/structs,
//!   function types → `fn` signatures, and MIR bodies → readable Rust.
//!
//! Numeric modeling note for 1.0: all Rust integer widths verify over kernel
//! `Nat`; signedness, overflow and wraparound are not modeled yet.
//!
//! Real `stable_mir` ingestion (the API is now named `rustc_public`) lives
//! in the separate `tpt-rustir-mir-ingest` crate (not a workspace member —
//! see its own doc comment) rather than a module here: it links against
//! rustc's own internal crates via `rustc_driver`, which is only reachable
//! from a `rustc_private`-enabled nightly (not published on crates.io), and
//! keeping that out of this crate means nothing else in the workspace
//! (tests included) accidentally inherits that linkage. It maps ingested
//! function signatures into this crate's `RustType`; wiring ingested bodies
//! into `verify`/`extract` end-to-end (Milestone 3) is still open.

pub mod extract;
pub mod mir;
pub mod stdlib;
pub mod verify;

/// Type representation for mapped Rust types in tpt-rustir-core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustType {
    /// Primitive types (u8, i32, bool, etc.)
    Primitive(String),
    /// Reference types (&T, &mut T)
    Reference(Box<RustType>, bool),
    /// Tuple types (T1, T2, ...)
    Tuple(Vec<RustType>),
    /// Array types `[T; N]`
    Array(Box<RustType>, usize),
    /// Slice types `[T]`
    Slice(Box<RustType>),
    /// Function pointer types fn(T1, T2) -> T3
    Function(Vec<RustType>, Box<RustType>),
    /// User-defined types (structs, enums)
    Adt(String, Vec<RustType>),
    /// Generic type parameters
    Generic(String),
    /// Never type (!)
    Never,
}

impl RustType {
    /// Convert a RustType to a tpt-rustir-core term (placeholder).
    pub fn to_core_term(&self) -> tpt_rustir_core::Term {
        use tpt_rustir_core::term::*;
        match self {
            RustType::Primitive(name) => {
                // Map primitive types to core types
                match name.as_str() {
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                        const_(format!("Nat{}", name))
                    }
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
                        const_(format!("Int{}", name))
                    }
                    "bool" => bool_ty(),
                    "char" => const_("Char"),
                    "()" => sigma(nat(), nat()), // Unit as empty sigma
                    _ => const_(format!("Primitive{}", name)),
                }
            }
            RustType::Reference(inner, _is_mut) => {
                let inner_ty = inner.to_core_term();
                // References as dependent pairs (ptr, lifetime_proof)
                let lifetime = const_("Lifetime");
                sigma(inner_ty, lifetime)
            }
            RustType::Tuple(types) => {
                // Encode tuples as nested sigmas
                types
                    .iter()
                    .rev()
                    .fold(nat(), |acc, ty| sigma(ty.to_core_term(), acc))
            }
            RustType::Array(inner, size) => {
                let inner_ty = inner.to_core_term();
                // Arrays as vectors of fixed size
                let size_term = const_(format!("ArraySize{}", size));
                sigma(inner_ty, size_term)
            }
            RustType::Slice(inner) => {
                let inner_ty = inner.to_core_term();
                // Slices as (ptr, len) pair
                let len_ty = const_("SliceLen");
                sigma(inner_ty, len_ty)
            }
            RustType::Function(args, ret) => {
                let mut ty = ret.to_core_term();
                for arg in args.iter().rev() {
                    ty = pi(arg.to_core_term(), ty);
                }
                ty
            }
            RustType::Adt(name, params) => {
                let mut ty = const_(name.clone());
                for param in params {
                    ty = app(ty, param.to_core_term());
                }
                ty
            }
            RustType::Generic(name) => const_(format!("Generic{}", name)),
            RustType::Never => const_("Never"),
        }
    }
}

/// Lifetime representation for borrow checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustLifetime {
    pub name: String,
    pub bounds: Vec<String>,
}

impl RustLifetime {
    pub fn to_core_term(&self) -> tpt_rustir_core::Term {
        use tpt_rustir_core::term::*;
        // Lifetime as a proposition that the reference is valid
        const_(format!("Lifetime_{}", self.name))
    }
}

/// Borrow checker state representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowState {
    /// No active borrows
    Free,
    /// Shared borrow (&T)
    Shared(String), // lifetime name
    /// Mutable borrow (&mut T)
    Mutable(String), // lifetime name
    /// Multiple shared borrows
    SharedMultiple(Vec<String>),
}

impl BorrowState {
    pub fn to_core_term(&self) -> tpt_rustir_core::Term {
        use tpt_rustir_core::term::*;
        match self {
            BorrowState::Free => const_("BorrowFree"),
            BorrowState::Shared(lt) => const_(format!("BorrowShared_{}", lt)),
            BorrowState::Mutable(lt) => const_(format!("BorrowMut_{}", lt)),
            BorrowState::SharedMultiple(lts) => {
                let mut ty = const_("BorrowSharedMulti");
                for lt in lts {
                    ty = app(ty, const_(format!("Lifetime_{}", lt)));
                }
                ty
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_type_conversion() {
        let ty = RustType::Primitive("u32".to_string());
        let _term = ty.to_core_term();
        // Just verify it compiles and produces a term
    }

    #[test]
    fn test_reference_type() {
        let ty = RustType::Reference(Box::new(RustType::Primitive("i32".to_string())), false);
        let _term = ty.to_core_term();
    }

    #[test]
    fn test_tuple_type() {
        let ty = RustType::Tuple(vec![
            RustType::Primitive("u32".to_string()),
            RustType::Primitive("bool".to_string()),
        ]);
        let _term = ty.to_core_term();
    }
}

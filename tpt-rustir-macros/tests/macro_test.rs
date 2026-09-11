use tpt_rustir_macros::{spec, verify};

#[spec("\\x : Nat => succ x")]
#[verify]
fn add_one(x: u32) -> u32 {
    x + 1
}

#[spec("\\x : Nat => x")]
#[verify]
fn identity_value(x: i32) -> i32 {
    x
}

#[verify("true")]
fn trivial() -> bool {
    true
}

#[test]
fn test_macros_compile() {
    assert_eq!(add_one(5), 6);
    assert_eq!(identity_value(-5), -5);
    assert!(trivial());
}

#[test]
fn test_expansion_time_verified_fns_exist() {
    // #[tpt::spec] emits a runtime re-check fn named `_tpt_verify_<fn>`.
    _tpt_verify_add_one().unwrap();
    _tpt_verify_identity_value().unwrap();
    // #[tpt::verify("...")] emits one for standalone specs.
    _tpt_verify_trivial().unwrap();
}

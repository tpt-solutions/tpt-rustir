//! End-to-end test: run the actual `cargo-tpt` binary against a small sample
//! workspace containing both a provable and an unprovable spec.

use std::path::Path;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_cargo-tpt")
}

fn write_sample_workspace(dir: &Path) {
    std::fs::write(
        dir.join("sample.rs"),
        r#"
#[tpt::spec("(+ (+ a b) c) = (+ c (+ b a))")]
fn ok_fn() {}

#[tpt::spec("(+ n zero) = zero")]
fn bad_fn() {}
"#,
    )
    .unwrap();
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tpt-cli-e2e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn check_passes_both_specs_without_discharging() {
    let dir = unique_temp_dir("check");
    write_sample_workspace(&dir);

    let output = Command::new(bin())
        .args(["tpt", "check"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2 spec(s) checked, 0 failed"), "{stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn verify_discharges_true_obligation_and_rejects_false_one() {
    let dir = unique_temp_dir("verify");
    write_sample_workspace(&dir);

    let output = Command::new(bin())
        .args(["tpt", "verify"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ok_fn"), "{stdout}");
    assert!(stdout.contains("FAIL"), "{stdout}");
    assert!(stdout.contains("bad_fn"), "{stdout}");
    assert!(stdout.contains("2 spec(s) verified, 1 failed"), "{stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn verify_json_output_is_well_formed() {
    let dir = unique_temp_dir("json");
    write_sample_workspace(&dir);

    let output = Command::new(bin())
        .args(["tpt", "verify", "--json"])
        .current_dir(&dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(report["checked"], 2);
    assert_eq!(report["failed"], 1);
    assert_eq!(report["results"].as_array().unwrap().len(), 2);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn verify_caches_successful_obligations_across_runs() {
    let dir = unique_temp_dir("cache");
    write_sample_workspace(&dir);

    let first = Command::new(bin())
        .args(["tpt", "verify"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&first.stdout).contains("ok"));
    assert!(dir.join("target/tpt/cache.json").exists());

    let second = Command::new(bin())
        .args(["tpt", "verify"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains("cached"), "{stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

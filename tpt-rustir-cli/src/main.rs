//! `cargo tpt`: command-line interface for tpt-rustir.

mod cache;
mod scan;

use std::path::Path;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "cargo-tpt", bin_name = "cargo")]
struct Cli {
    #[command(subcommand)]
    command: Option<TptCommand>,
}

#[derive(Subcommand)]
enum TptCommand {
    Tpt {
        #[command(subcommand)]
        action: Action,
    },
}

#[derive(Subcommand)]
enum Action {
    /// Type-check specs without discharging proof obligations.
    Check(RunArgs),
    /// Fully verify the current workspace.
    Verify(RunArgs),
}

#[derive(Args, Default)]
struct RunArgs {
    /// Emit machine-readable JSON instead of human-readable text.
    #[arg(long)]
    json: bool,
    /// Ignore (and don't update) the proof-state cache.
    #[arg(long)]
    no_cache: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Some(TptCommand::Tpt {
            action: Action::Check(args),
        }) => run(Path::new("."), false, &args),
        Some(TptCommand::Tpt {
            action: Action::Verify(args),
        }) => run(Path::new("."), true, &args),
        None => {
            println!("usage: cargo tpt <check|verify>");
            ExitCode::SUCCESS
        }
    }
}

#[derive(Serialize)]
struct SpecResult {
    file: String,
    line: usize,
    kind: &'static str,
    #[serde(rename = "fn")]
    fn_name: Option<String>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct Report {
    command: &'static str,
    checked: usize,
    failed: usize,
    results: Vec<SpecResult>,
}

/// Scan `root` for `#[tpt::spec(...)]` / `#[tpt::verify(...)]` attributes and
/// check (or fully verify) every spec found. Prints one line per spec (or a
/// single JSON report) and returns a failing `ExitCode` if any spec did not
/// pass.
fn run(root: &Path, discharge: bool, args: &RunArgs) -> ExitCode {
    let files = match scan::collect_rs_files(root) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("error: could not walk {}: {e}", root.display());
            return ExitCode::FAILURE;
        }
    };

    // The cache only ever helps `verify` (the expensive, discharging path);
    // `check` is cheap syntax/type-checking and always re-runs.
    let mut spec_cache = (discharge && !args.no_cache).then(|| cache::Cache::load(root));

    let mut results = Vec::new();
    let mut failed = 0usize;

    for path in &files {
        let src = match std::fs::read_to_string(path) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("error: could not read {}: {e}", path.display());
                failed += 1;
                continue;
            }
        };
        let attrs = match scan::scan_source(path, &src) {
            Ok(attrs) => attrs,
            Err(e) => {
                eprintln!("error: {e}");
                failed += 1;
                continue;
            }
        };
        for attr in attrs {
            if attr.spec.is_empty() {
                continue;
            }

            let cache_key = cache::hash_spec(&attr.spec);
            let cached = spec_cache
                .as_ref()
                .is_some_and(|c| c.contains(&cache_key));

            let (status, error) = if cached {
                ("cached", None)
            } else {
                let outcome = if discharge {
                    tpt_rustir_solver::specs::verify_spec_string(&attr.spec)
                } else {
                    tpt_rustir_solver::specs::check_spec_string(&attr.spec)
                };
                match outcome {
                    Ok(()) => {
                        if let Some(c) = spec_cache.as_mut() {
                            c.record_ok(&cache_key);
                        }
                        ("ok", None)
                    }
                    Err(e) => {
                        failed += 1;
                        ("FAIL", Some(e))
                    }
                }
            };

            if !args.json {
                let loc = format!("{}:{}", path.display(), attr.line);
                let label = attr.fn_name.as_deref().unwrap_or("<spec>");
                match &error {
                    None => println!("{status:<7}{loc}: [{}] {label}", attr.kind),
                    Some(e) => println!("{status:<7}{loc}: [{}] {label}: {e}", attr.kind),
                }
            }

            results.push(SpecResult {
                file: path.display().to_string(),
                line: attr.line,
                kind: attr.kind,
                fn_name: attr.fn_name,
                status,
                error,
            });
        }
    }

    if let Some(c) = &spec_cache {
        c.save();
    }

    let checked = results.len();
    if args.json {
        let report = Report {
            command: if discharge { "verify" } else { "check" },
            checked,
            failed,
            results,
        };
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("error: could not serialize report: {e}"),
        }
    } else {
        let verb = if discharge { "verified" } else { "checked" };
        println!("{checked} spec(s) {verb}, {failed} failed");
    }

    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

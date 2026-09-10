//! `cargo tpt`: command-line interface for tpt-rustir.
//!
//! Scaffolding only for now — see `todo.md` Phase 4.

use clap::{Parser, Subcommand};

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
    Check,
    /// Fully verify the current workspace.
    Verify,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(TptCommand::Tpt {
            action: Action::Check,
        }) => {
            println!("cargo tpt check: not yet implemented (see todo.md Phase 4)");
        }
        Some(TptCommand::Tpt {
            action: Action::Verify,
        }) => {
            println!("cargo tpt verify: not yet implemented (see todo.md Phase 4)");
        }
        None => {
            println!("usage: cargo tpt <check|verify>");
        }
    }
}

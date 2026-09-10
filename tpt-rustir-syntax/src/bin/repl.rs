//! Basic REPL for interactive experimentation with the tpt kernel.
//!
//! Usage:
//!   cargo run -p tpt-rustir-syntax --bin repl
//!
//! Enter a term to see its inferred type and normal form, e.g.:
//!   \x : Nat => succ x
//!   (\x : Nat => succ x) 2

use std::io::{self, Write};

use tpt_rustir_core::{infer, normalize, GlobalEnv};
use tpt_rustir_syntax::{lower, parser, pretty};

fn main() {
    println!("tpt-rustir REPL — type an expression, or :quit to exit.");
    let env = GlobalEnv::new();
    let ctx = Vec::new();
    let stdin = io::stdin();
    loop {
        print!("tpt> ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == ":quit" || line == ":q" {
            break;
        }

        match parser::parse(line) {
            Ok(expr) => match lower::lower(&lower::Scope::new(), &expr) {
                Ok(term) => match infer(&env, &ctx, &term) {
                    Ok(ty) => {
                        let nf = normalize(&env, &term);
                        println!("  : {}", pretty::print(&ty));
                        println!("  = {}", pretty::print(&nf));
                    }
                    Err(e) => println!("type error: {e}"),
                },
                Err(e) => println!("elaboration error: {e}"),
            },
            Err(errs) => {
                for e in errs {
                    println!("parse error: {e}");
                }
            }
        }
    }
}

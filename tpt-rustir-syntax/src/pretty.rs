//! Pretty-printer for core terms: reconstructs readable names for de Bruijn
//! binders using a simple name-avoiding counter scheme.

use crate::ast::{Expr, ExprKind};
use tpt_rustir_core::term::{Term, TermKind};

fn fresh_name(names: &[String]) -> String {
    let base = ["x", "y", "z", "w", "v", "u"];
    for b in base {
        if !names.contains(&b.to_string()) {
            return b.to_string();
        }
    }
    format!("x{}", names.len())
}

fn go(names: &[String], t: &Term, paren: bool) -> String {
    let wrap = |s: String, needed: bool| if needed && paren { format!("({s})") } else { s };
    match &**t {
        TermKind::Var(n) => names
            .len()
            .checked_sub(1 + n)
            .and_then(|i| names.get(i))
            .cloned()
            .unwrap_or_else(|| format!("#{n}")),
        TermKind::Const(name) => name.clone(),
        TermKind::Universe(i) => format!("Type{i}"),
        TermKind::Nat => "Nat".to_string(),
        TermKind::Bool => "Bool".to_string(),
        TermKind::Zero => "zero".to_string(),
        TermKind::True => "true".to_string(),
        TermKind::False => "false".to_string(),
        TermKind::Succ(n) => wrap(format!("succ {}", go(names, n, true)), true),
        TermKind::Pi(dom, cod) => {
            let name = fresh_name(names);
            let mut inner = names.to_vec();
            inner.push(name.clone());
            wrap(
                format!(
                    "({name} : {}) -> {}",
                    go(names, dom, false),
                    go(&inner, cod, false)
                ),
                false,
            )
        }
        TermKind::Sigma(a, b) => {
            let name = fresh_name(names);
            let mut inner = names.to_vec();
            inner.push(name.clone());
            wrap(
                format!(
                    "({name} : {}) * {}",
                    go(names, a, false),
                    go(&inner, b, false)
                ),
                false,
            )
        }
        TermKind::Lambda(dom, body) => {
            let name = fresh_name(names);
            let mut inner = names.to_vec();
            inner.push(name.clone());
            wrap(
                format!(
                    "\\{name} : {} => {}",
                    go(names, dom, false),
                    go(&inner, body, false)
                ),
                true,
            )
        }
        TermKind::App(f, a) => wrap(
            format!("{} {}", go(names, f, true), go(names, a, true)),
            true,
        ),
        TermKind::Pair(a, b, _) => format!("({}, {})", go(names, a, false), go(names, b, false)),
        TermKind::Fst(p) => wrap(format!("fst {}", go(names, p, true)), true),
        TermKind::Snd(p) => wrap(format!("snd {}", go(names, p, true)), true),
        TermKind::NatRec(m, b, s, t0) => format!(
            "natrec {} {} {} {}",
            go(names, m, true),
            go(names, b, true),
            go(names, s, true),
            go(names, t0, true)
        ),
        TermKind::BoolRec(m, ct, cf, t0) => format!(
            "boolrec {} {} {} {}",
            go(names, m, true),
            go(names, ct, true),
            go(names, cf, true),
            go(names, t0, true)
        ),
        TermKind::Let(ty, val, body) => {
            let name = fresh_name(names);
            let mut inner = names.to_vec();
            inner.push(name.clone());
            wrap(
                format!(
                    "let {name} : {} := {} in {}",
                    go(names, ty, false),
                    go(names, val, false),
                    go(&inner, body, false)
                ),
                true,
            )
        }
        TermKind::Inductive(name, params) => {
            let param_strs: Vec<String> = params.iter().map(|p| go(names, p, true)).collect();
            if param_strs.is_empty() {
                name.clone()
            } else {
                format!("{} {}", name, param_strs.join(" "))
            }
        }
        TermKind::Con(ind_name, con_name, args) => {
            let arg_strs: Vec<String> = args.iter().map(|a| go(names, a, true)).collect();
            format!("{}.{} {}", ind_name, con_name, arg_strs.join(" "))
        }
        TermKind::IndRec(ind_name, motive, cases, _scrutinee) => {
            let case_strs: Vec<String> = cases
                .iter()
                .map(|(cn, cb)| format!("| {} => {}", cn, go(names, cb, false)))
                .collect();
            format!(
                "elim {} {} with {}",
                ind_name,
                go(names, motive, true),
                case_strs.join(" ")
            )
        }
    }
}

pub fn print(t: &Term) -> String {
    go(&[], t, false)
}

/// Pretty-print an AST expression
pub fn print_expr(expr: &Expr) -> String {
    print_expr_inner(expr, &[])
}

fn print_expr_inner(expr: &Expr, names: &[String]) -> String {
    match &expr.kind {
        ExprKind::Var(name) => name.clone(),
        ExprKind::NatLit(n) => n.to_string(),
        ExprKind::Pi(_name, dom, cod) => {
            let name = fresh_name(names);
            let mut inner = names.to_vec();
            inner.push(name.clone());
            format!(
                "({name} : {}) -> {}",
                print_expr_inner(dom, names),
                print_expr_inner(cod, &inner)
            )
        }
        ExprKind::Sigma(_name, fst_ty, snd_ty) => {
            let name = fresh_name(names);
            let mut inner = names.to_vec();
            inner.push(name.clone());
            format!(
                "({name} : {}) * {}",
                print_expr_inner(fst_ty, names),
                print_expr_inner(snd_ty, &inner)
            )
        }
        ExprKind::Lambda(_name, dom, body) => {
            let name = fresh_name(names);
            let mut inner = names.to_vec();
            inner.push(name.clone());
            format!(
                "\\{name} : {} => {}",
                print_expr_inner(dom, names),
                print_expr_inner(body, &inner)
            )
        }
        ExprKind::App(f, a) => format!(
            "{} {}",
            print_expr_inner(f, names),
            print_expr_inner(a, names)
        ),
        ExprKind::Pair(a, b) => format!(
            "({}, {})",
            print_expr_inner(a, names),
            print_expr_inner(b, names)
        ),
        ExprKind::Let(_name, ann_ty, val, body) => {
            let name = fresh_name(names);
            let mut inner = names.to_vec();
            inner.push(name.clone());
            format!(
                "let {name} : {} = {} in {}",
                print_expr_inner(ann_ty, names),
                print_expr_inner(val, names),
                print_expr_inner(body, &inner)
            )
        }
        ExprKind::Ann(inner, ty) => format!(
            "{} : {}",
            print_expr_inner(inner, names),
            print_expr_inner(ty, names)
        ),
        ExprKind::InductiveDecl(name, params, ty, constructors) => {
            let param_strs: Vec<String> = params
                .iter()
                .map(|(n, t)| format!("{}: {}", n, print_expr_inner(t, names)))
                .collect();
            let con_strs: Vec<String> = constructors
                .iter()
                .map(|c| print_expr_inner(c, names))
                .collect();
            format!(
                "inductive {} ({}) : {} where {} end",
                name,
                param_strs.join(", "),
                print_expr_inner(ty, names),
                con_strs.join("; ")
            )
        }
        ExprKind::ConApp(ind, con, args) => {
            let arg_strs: Vec<String> = args.iter().map(|a| print_expr_inner(a, names)).collect();
            format!("{}.{} {}", ind, con, arg_strs.join(" "))
        }
        ExprKind::Elim(ind, motive, cases) => {
            let case_strs: Vec<String> = cases
                .iter()
                .map(|(cn, cb)| format!("| {} => {}", cn, print_expr_inner(cb, names)))
                .collect();
            format!(
                "elim {} {} with {}",
                ind,
                print_expr_inner(motive, names),
                case_strs.join(" ")
            )
        }
    }
}

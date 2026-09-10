//! Pretty-printer for core terms: reconstructs readable names for de Bruijn
//! binders using a simple name-avoiding counter scheme.

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
    }
}

pub fn print(t: &Term) -> String {
    go(&[], t, false)
}

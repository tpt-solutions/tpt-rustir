//! The e-graph term language and rewrite rules used for automation.
//!
//! This is a small arithmetic-and-propositional language distinct from
//! `tpt_rustir_core::Term` — it exists purely so `egg` can perform equality
//! saturation over it. It mirrors Peano-style `Nat` recursion (`+`/`*`
//! defined by recursion on the left argument, matching `NatRec` in the
//! kernel) and classical two-valued `Bool` connectives.

use egg::{define_language, rewrite as rw, Id, Rewrite, Symbol};

define_language! {
    pub enum Expr {
        "+" = Add([Id; 2]),
        "*" = Mul([Id; 2]),
        "zero" = Zero,
        "succ" = Succ([Id; 1]),
        "and" = And([Id; 2]),
        "or" = Or([Id; 2]),
        "not" = Not([Id; 1]),
        "implies" = Implies([Id; 2]),
        "true" = True,
        "false" = False,
        Symbol(Symbol),
    }
}

pub type Rw = Rewrite<Expr, ()>;

/// The definitional rules: how `+`/`*` recurse, and how classical
/// propositional connectives simplify. Sound and terminating on their own
/// (no rule ever grows the term without also making structural progress
/// elsewhere), so they're safe to run to saturation without extra guards.
pub fn definitional_rules() -> Vec<Rw> {
    vec![
        rw!("add-zero-l"; "(+ zero ?a)" => "?a"),
        rw!("add-succ-l"; "(+ (succ ?a) ?b)" => "(succ (+ ?a ?b))"),
        rw!("mul-zero-l"; "(* zero ?a)" => "zero"),
        rw!("mul-succ-l"; "(* (succ ?a) ?b)" => "(+ ?b (* ?a ?b))"),
        rw!("and-true-l"; "(and true ?a)" => "?a"),
        rw!("and-false-l"; "(and false ?a)" => "false"),
        rw!("or-true-l"; "(or true ?a)" => "true"),
        rw!("or-false-l"; "(or false ?a)" => "?a"),
        rw!("not-true"; "(not true)" => "false"),
        rw!("not-false"; "(not false)" => "true"),
        rw!("not-not"; "(not (not ?a))" => "?a"),
        rw!("implies-elim"; "(implies ?a ?b)" => "(or (not ?a) ?b)"),
    ]
}

/// Extra structural rules (commutativity, associativity, excluded middle)
/// that make the definitional rules above powerful enough to discharge full
/// algebraic/propositional equalities without appealing to induction.
pub fn algebraic_rules() -> Vec<Rw> {
    let mut rules = definitional_rules();
    rules.extend(vec![
        rw!("add-comm"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rw!("add-assoc-lr"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
        rw!("add-assoc-rl"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
        rw!("mul-comm"; "(* ?a ?b)" => "(* ?b ?a)"),
        rw!("mul-assoc-lr"; "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),
        rw!("mul-assoc-rl"; "(* ?a (* ?b ?c))" => "(* (* ?a ?b) ?c)"),
        rw!("and-comm"; "(and ?a ?b)" => "(and ?b ?a)"),
        rw!("or-comm"; "(or ?a ?b)" => "(or ?b ?a)"),
        rw!("excluded-middle-l"; "(or (not ?a) ?a)" => "true"),
        rw!("excluded-middle-r"; "(or ?a (not ?a))" => "true"),
    ]);
    rules
}

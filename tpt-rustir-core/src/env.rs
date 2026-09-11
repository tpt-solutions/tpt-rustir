//! Global environment of named definitions, for delta reduction.

use std::collections::HashMap;

use crate::term::Term;

#[derive(Clone, Debug)]
pub struct InductiveDef {
    pub name: String,
    pub params: Vec<Term>,  // parameter types (fixed for all constructors)
    pub indices: Vec<Term>, // index types (vary per constructor)
    pub constructors: Vec<ConstructorDef>,
}

#[derive(Clone, Debug)]
pub struct ConstructorDef {
    pub name: String,
    pub ty: Term, // type of this constructor, e.g. (n : Nat) -> Vec A n -> Vec A (succ n)
}

#[derive(Clone)]
pub struct GlobalDef {
    pub ty: Term,
    pub value: Term,
}

#[derive(Default, Clone)]
pub struct GlobalEnv {
    defs: HashMap<String, GlobalDef>,
    inductives: HashMap<String, InductiveDef>,
}

impl GlobalEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, ty: Term, value: Term) {
        self.defs.insert(name.into(), GlobalDef { ty, value });
    }

    pub fn get(&self, name: &str) -> Option<&GlobalDef> {
        self.defs.get(name)
    }

    pub fn insert_inductive(&mut self, inductive: InductiveDef) {
        self.inductives.insert(inductive.name.clone(), inductive);
    }

    pub fn get_inductive(&self, name: &str) -> Option<&InductiveDef> {
        self.inductives.get(name)
    }

    pub fn get_constructor(
        &self,
        inductive_name: &str,
        constructor_name: &str,
    ) -> Option<&ConstructorDef> {
        self.inductives
            .get(inductive_name)
            .and_then(|ind| ind.constructors.iter().find(|c| c.name == constructor_name))
    }
}

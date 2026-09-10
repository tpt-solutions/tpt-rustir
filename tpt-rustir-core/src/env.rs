//! Global environment of named definitions, for delta reduction.

use std::collections::HashMap;

use crate::term::Term;

#[derive(Clone)]
pub struct GlobalDef {
    pub ty: Term,
    pub value: Term,
}

#[derive(Default, Clone)]
pub struct GlobalEnv {
    defs: HashMap<String, GlobalDef>,
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
}

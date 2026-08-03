use std::collections::HashMap;

use crate::analyzer::symbol::Symbol;

#[derive(Debug, Clone)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    pub fn declare(
        &mut self,
        name: String,
        symbol: Symbol,
    ) -> bool {
        if self.symbols.contains_key(&name) {
            return false;
        }

        self.symbols.insert(name, symbol);

        true
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }
}
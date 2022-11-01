use crate::definitions::Symbol;
use std::collections::HashMap;

pub struct SymbolTable {
    counter: Symbol,
    symbols: HashMap<String, Symbol>,
}

impl SymbolTable {
    /// Lookup a value in the symbol table
    ///
    /// if the value does not exist we create a new symbol for it
    /// and assume that this is the definition or that it will be defined later
    pub fn lookup<'s>(&mut self, ident: impl Into<&'s str>) -> Option<Symbol> {
        self.symbols.get(ident.into()).copied()
    }

    pub fn lookup_or_insert(&mut self, ident: impl Into<String>) -> Symbol {
        *self.symbols.entry(ident.into()).or_insert_with(|| {
            let value = self.counter;
            self.counter += 1;
            value
        })
    }

    /// Set a value in the Symbol Table explicitly
    ///
    /// this is only makes sense for Label instructions, because the Symbol in that case should
    /// be the position inside the bytecode
    pub fn set(&mut self, ident: impl Into<String>, value: Symbol) {
        self.symbols.insert(ident.into(), value);
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        let symbols = HashMap::with_capacity(64);

        Self {
            counter: 16, // don't overwrite SP/LCL/...
            symbols,
        }
    }
}

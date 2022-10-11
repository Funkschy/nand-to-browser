use crate::definitions::{Symbol, Word};
use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub enum FileInfo {
    /// the filename as a string
    Builtin(&'static str),
    /// the module index
    VM(usize),
}

#[derive(Debug)]
pub struct FunctionInfo {
    pub file: FileInfo,
    pub name: String,
    pub n_locals: Word,
}

impl FunctionInfo {
    pub fn builtin(name: String, n_locals: Word, filename: &'static str) -> Self {
        Self {
            file: FileInfo::Builtin(filename),
            name,
            n_locals,
        }
    }

    pub fn vm(name: String, n_locals: Word, module_index: usize) -> Self {
        Self {
            file: FileInfo::VM(module_index),
            name,
            n_locals,
        }
    }
}

#[derive(Default, Debug)]
pub struct MetaInfo {
    // a map from function positions in the bytecode to their meta information
    // this is need to display infos in the UI (like the callstack) and for debugging the VM
    // every function in the bytecode gets some meta info, so that we can give the
    // user useful information
    pub function_meta: HashMap<Symbol, FunctionInfo>,
    // the vm should be able to call functions by their names. This is needed for the stdlib
    pub function_by_name: HashMap<String, Symbol>,
}

impl MetaInfo {
    pub fn new(
        function_meta: HashMap<Symbol, FunctionInfo>,
        function_by_name: HashMap<String, Symbol>,
    ) -> Self {
        Self {
            function_meta,
            function_by_name,
        }
    }

    pub fn sys_init_address(&self) -> Option<Symbol> {
        self.function_by_name.get("Sys.init").copied()
    }
}

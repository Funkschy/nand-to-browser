use super::stdlib::State;
use crate::definitions::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnAddress {
    EndOfProgram, // the return for the first function in the program (usually Sys.init)
    VM(Symbol),
    Builtin(State),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallState {
    TopLevel,
    // the state the function is in and the original args
    Builtin(State, Vec<Word>),
    VM,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallStackEntry {
    pub ret_addr: ReturnAddress,
    pub function: Option<Symbol>,
    pub state: CallState,
}

impl CallStackEntry {
    pub fn top_level() -> Self {
        Self {
            ret_addr: ReturnAddress::EndOfProgram,
            function: None,
            state: CallState::TopLevel,
        }
    }

    pub fn top_level_vm() -> Self {
        Self {
            ret_addr: ReturnAddress::EndOfProgram,
            function: None,
            state: CallState::VM,
        }
    }

    pub fn builtin(
        ret_addr: ReturnAddress,
        function: Symbol,
        state: State,
        args: Vec<Word>,
    ) -> Self {
        Self {
            ret_addr,
            function: Some(function),
            state: CallState::Builtin(state, args),
        }
    }

    pub fn vm(ret_addr: ReturnAddress, function: Symbol) -> Self {
        Self {
            ret_addr,
            function: Some(function),
            state: CallState::VM,
        }
    }
}

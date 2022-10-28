use super::stdlib::StdlibError;
use crate::definitions::Address;
use std::{error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMError {
    IllegalProgramCounter(usize),
    IllegalMemoryAddress(Address),
    CannotGetAddressOfConstant,

    // function call (stdlib) errors
    IllegalCallStackIndex,
    AccessingEmptyCallStack,
    TryingToContinueVMFunction,
    TryingToContinueTopLevelCode,
    NonExistingStdlibFunction,
    StdlibError(StdlibError),
}

impl From<StdlibError> for VMError {
    fn from(e: StdlibError) -> Self {
        // dont't create error linked lists
        if let StdlibError::VMError(vm_error) = e {
            return *vm_error;
        }
        VMError::StdlibError(e)
    }
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IllegalProgramCounter(pc) => write!(f, "Program counter out of bounds: {}", pc),
            Self::IllegalMemoryAddress(a) => write!(f, "Illegal memory address: {}", a),
            Self::CannotGetAddressOfConstant => write!(f, "Trying to get address of constant"),
            Self::IllegalCallStackIndex => write!(f, "Illegal call stack index"),
            Self::AccessingEmptyCallStack => write!(f, "Trying to access empty call stack"),
            Self::TryingToContinueVMFunction => write!(f, "Trying to continue VM Function"),
            Self::TryingToContinueTopLevelCode => write!(f, "Trying to continue top level code"),
            Self::NonExistingStdlibFunction => {
                write!(f, "Trying to call non existing stdlib function")
            }
            Self::StdlibError(error) => write!(f, "{}", error),
        }
    }
}

impl error::Error for VMError {}

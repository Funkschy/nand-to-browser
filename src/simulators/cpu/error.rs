use crate::definitions::Address;
use std::error;
use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum CpuError {
    IllegalProgramCounter(usize),
    IllegalMemoryAddress(Address),
}

impl fmt::Display for CpuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IllegalProgramCounter(pc) => write!(f, "Program counter out of bounds: {}", pc),
            Self::IllegalMemoryAddress(a) => write!(f, "Illegal memory address: {}", a),
        }
    }
}

impl error::Error for CpuError {}

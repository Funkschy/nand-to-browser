use std::str::FromStr;

use crate::definitions::{Symbol, Word};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ByteCodeParseError {
    IllegalSegmentString,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Segment {
    Argument = 0,
    Local = 1,
    Static = 2,
    Constant = 3,
    This = 4,
    That = 5,
    Pointer = 6,
    Temp = 7,
}

impl FromStr for Segment {
    type Err = ByteCodeParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "argument" => Ok(Segment::Argument),
            "local" => Ok(Segment::Local),
            "static" => Ok(Segment::Static),
            "constant" => Ok(Segment::Constant),
            "this" => Ok(Segment::This),
            "that" => Ok(Segment::That),
            "pointer" => Ok(Segment::Pointer),
            "temp" => Ok(Segment::Temp),
            _ => Err(ByteCodeParseError::IllegalSegmentString),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Instruction {
    // arithmetic commands (no arguments)
    Add,
    Sub,
    Eq,
    Gt,
    Lt,
    And,
    Or,
    Not,
    Neg,
    Push { segment: Segment, index: Word },
    Pop { segment: Segment, index: Word },
    Goto { instruction: Symbol },
    IfGoto { instruction: Symbol },
    Function { n_locals: Word },
    Call { function: Symbol, n_args: Word },
    Return,
}

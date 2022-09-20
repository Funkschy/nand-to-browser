use std::convert::TryInto;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ByteCodeParseError {
    IllegalInstruction,
    IllegalSegment,
    SegmentParse,
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
            _ => Err(ByteCodeParseError::SegmentParse),
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Instruction {
    // arithemetic commands (no arguments)
    Add = 0,
    Sub = 1,
    Eq = 2,
    Gt = 3,
    Lt = 4,
    And = 5,
    Or = 6,
    Not = 7,
    Neg = 8,
    // memory access commands (8 bit segment + 16 bit index arguments)
    Push = 9,
    Pop = 10,
    // programflow commands (16 bit symbol as argument)
    Goto = 11,
    IfGoto = 12,
    // function commands (16 bit symbol + 16 bit nargs/nlocals as arguments)
    Function = 13,
    Call = 14,
    // return (no arguments)
    Return = 15,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union Opcode {
    instruction: Instruction,
    segment: Segment,
    constant: u8,
}

impl Opcode {
    pub fn instruction(instruction: Instruction) -> Self {
        Opcode { instruction }
    }
    pub fn segment(segment: Segment) -> Self {
        Opcode { segment }
    }
    pub fn constant(constant: u8) -> Self {
        Opcode { constant }
    }
}

impl TryInto<Instruction> for Opcode {
    type Error = ByteCodeParseError;

    /// Return the opcode as an instance of the Instruction enum
    fn try_into(self) -> Result<Instruction, Self::Error> {
        // SAFETY: this will only return Ok if the value of the enum was actually in the
        // valid range of instructions
        unsafe {
            if self.constant >= Instruction::Add as u8 && self.constant <= Instruction::Return as u8
            {
                Ok(self.instruction)
            } else {
                Err(ByteCodeParseError::IllegalInstruction)
            }
        }
    }
}

impl TryInto<u8> for Opcode {
    type Error = ByteCodeParseError;

    /// Return the opcode as a byte
    fn try_into(self) -> Result<u8, Self::Error> {
        // SAFETY: all enum fields have the same size, so just reading the
        // 8 bits of memory will always be safe
        unsafe { Ok(self.constant) }
    }
}

impl TryInto<Segment> for Opcode {
    type Error = ByteCodeParseError;

    /// Return the opcode as a segment
    fn try_into(self) -> Result<Segment, Self::Error> {
        // SAFETY: this will only return Ok if the value of the enum was actually in the
        // valid range of segments
        unsafe {
            if self.constant >= Segment::Argument as u8 && self.constant <= Segment::Temp as u8 {
                Ok(self.segment)
            } else {
                Err(ByteCodeParseError::IllegalSegment)
            }
        }
    }
}

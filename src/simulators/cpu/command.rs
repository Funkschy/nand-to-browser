#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Register {
    A,
    D,
    M,
}

impl TryFrom<&str> for Register {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "A" => Ok(Register::A),
            "D" => Ok(Register::D),
            "M" => Ok(Register::M),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Computation {
    /// 0
    ConstZero,
    /// 1
    ConstOne,
    /// -1
    ConstNegOne,

    /// R
    UnaryNone(Register),
    /// !R
    UnaryBoolNeg(Register),
    /// -R
    UnaryIntNeg(Register),

    /// R + 1
    BinaryInc(Register),
    /// R - 1
    BinaryDec(Register),
    /// R1 + R2
    BinaryAdd(Register, Register),
    /// R1 - R2
    BinarySub(Register, Register),
    /// R1 & R2
    BinaryAnd(Register, Register),
    /// R1 | R2
    BinaryOr(Register, Register),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum Destination {
    None,
    A,
    D,
    M,
    AD,
    AM,
    DM,
    ADM,
}

impl Destination {
    /// return the destination as a bool tuple (a, d, m)
    pub fn as_bools(&self) -> (bool, bool, bool) {
        match self {
            Self::None => (false, false, false),
            Self::A => (true, false, false),
            Self::D => (false, true, false),
            Self::M => (false, false, true),
            Self::AD => (true, true, false),
            Self::AM => (true, false, true),
            Self::DM => (false, true, true),
            Self::ADM => (true, true, true),
        }
    }
}

impl Default for Destination {
    fn default() -> Self {
        Self::None
    }
}

impl TryFrom<&str> for Destination {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut a = false;
        let mut d = false;
        let mut m = false;
        for c in s.chars() {
            match c {
                'A' => a = true,
                'D' => d = true,
                'M' => m = true,
                _ => return Err(()),
            }
        }

        Ok(match (a, d, m) {
            (false, false, false) => Self::None,
            (false, false, true) => Self::M,
            (false, true, false) => Self::D,
            (false, true, true) => Self::DM,
            (true, false, false) => Self::A,
            (true, false, true) => Self::AM,
            (true, true, false) => Self::AD,
            (true, true, true) => Self::ADM,
        })
    }
}
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Jump {
    Next,
    Gt,
    Eq,
    Ge,
    Lt,
    Ne,
    Le,
    Unconditional,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Instruction {
    /// The A-instruction is used to set the A register to a 15-bit value:
    A(u16),
    C(Destination, Computation, Jump),
}

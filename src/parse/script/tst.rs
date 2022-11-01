use crate::definitions::Word;
use std::convert;
use std::fmt::Debug;

use std::path::PathBuf;

pub type VarName = String;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NumberFormat {
    Binary,
    Decimal,
    Hex,
    String,
}

impl convert::TryFrom<&str> for NumberFormat {
    type Error = &'static str;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "B" => Ok(NumberFormat::Binary),
            "D" => Ok(NumberFormat::Decimal),
            "X" => Ok(NumberFormat::Hex),
            "S" => Ok(NumberFormat::String),
            _ => Err("s should be one of [B, D, X, S]"),
        }
    }
}

impl NumberFormat {
    pub fn format_string(&self, s: &str) -> Result<String, std::num::ParseIntError> {
        if let Self::String = self {
            return Ok(s.to_owned());
        }

        let i = s.parse::<i64>()?;
        Ok(match self {
            Self::Binary => format!("{i:b}"),
            Self::Decimal => format!("{i:}"),
            Self::Hex => format!("{i:x}"),
            Self::String => unreachable!(),
        })
    }
}

/// the output list entries are formatted as <name>%<format><left-padding>.<length>.<right-padding>
/// where format is on of ['B', 'D', 'X', 'S']
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OutputListEntry {
    pub name: VarName,
    pub format: NumberFormat,
    pub left_padding: usize,
    pub length: usize,
    pub right_padding: usize,
}

impl OutputListEntry {
    pub fn new(
        name: VarName,
        format: NumberFormat,
        left_padding: usize,
        length: usize,
        right_padding: usize,
    ) -> Self {
        Self {
            name,
            format,
            left_padding,
            length,
            right_padding,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VMSetTarget {
    Local(Option<usize>),
    Argument(Option<usize>),
    This(Option<usize>),
    That(Option<usize>),
    SP,
    CurrentFunction,
    Line,
    Temp(usize),
    Ram(usize),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CpuSetTarget {
    A,
    D,
    PC,
    Ram(usize),
    Rom(usize),
}
pub trait SimulatorCommand: Debug + PartialEq + Eq + Clone {}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VMEmulatorCommand {
    Load(PathBuf),
    Step,
    Set(VMSetTarget, Word),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CpuEmulatorCommand {
    Load(PathBuf),
    TickTock,
    Set(CpuSetTarget, Word),
}

impl SimulatorCommand for VMEmulatorCommand {}
impl SimulatorCommand for CpuEmulatorCommand {}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CommandKind<SimCmd: SimulatorCommand> {
    Simulator(SimCmd),
    OutputFile(PathBuf),
    CompareTo(PathBuf),
    OutputList(Vec<OutputListEntry>),
    Output,
    Repeat {
        times: usize,
        block: Vec<Command<SimCmd>>,
    },
    Echo(String),
    // Breakpoint,
    // ClearBreakpoints,
    // EndScript,
    // ClearEcho,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Terminator {
    None,
    // ',' execute this and the next command as one Command
    // one click on step button for multiple things to happen
    MiniStep,
    // ';' execute only this command in this step
    // one click on step button for one thing to happen
    SingleStep,
    // '!' stop the program execution entirely
    Stop,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Command<SimCmd: SimulatorCommand> {
    pub kind: CommandKind<SimCmd>,
    pub terminator: Terminator,
}

impl<SimCmd: SimulatorCommand> Command<SimCmd> {
    pub fn new(kind: CommandKind<SimCmd>) -> Self {
        Self {
            kind,
            terminator: Terminator::None,
        }
    }

    #[cfg(test)]
    pub fn terminated(kind: CommandKind<SimCmd>, terminator: Terminator) -> Self {
        Self { kind, terminator }
    }
}

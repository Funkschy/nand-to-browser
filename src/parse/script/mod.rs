pub mod lexer;
pub mod parser;
pub mod tst;

use super::Spanned;
use lexer::Token;
use tst::{Command, SimulatorCommand};

use std::fmt;
use std::mem::Discriminant;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    NoNextItem,
    ExpectedKind(Discriminant<Token>),
    Expected(Token),
    CommandStartingWithSymbol,
    CommandStartingWithInt,
    CommandStartingWithString,
    UnterminatedCommand,
    NotASimulatorCommand(String),
    CouldNotParseOutputListEntry,
    EmptyOutputList,
    InvalidSetTarget(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T> = Result<T, ParseError>;

pub type TokResult = ParseResult<Spanned<Token>>;
pub type CmdResult<SimulatorCommand> = ParseResult<Spanned<Command<SimulatorCommand>>>;

pub trait SimulatorCommandParser<SimCmd: SimulatorCommand> {
    fn parse_simulator_command(&mut self, ident: Spanned<&str>) -> CmdResult<SimCmd>;
}

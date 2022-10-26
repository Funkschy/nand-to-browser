use crate::definitions::Word;
use crate::parse::script::lexer::{ident_kind, int_kind, Token};
use crate::parse::script::parser::ScriptParser;
use crate::parse::script::tst::{Command, CommandKind, VMEmulatorCommand, VMSetTarget};
use crate::parse::script::{CmdResult, ParseError, ParseResult, SimulatorCommandParser};
use crate::parse::Spanned;

use std::path::Path;

mod run;

#[derive(Default)]
pub struct VMEmulatorCommandParser {}

impl<'src> VMEmulatorCommandParser {
    pub fn create(
        path: &'src Path,
        code: &'src str,
    ) -> ScriptParser<'src, Self, VMEmulatorCommand> {
        ScriptParser::new(path, code)
    }
}

impl<'tst> ScriptParser<'tst, VMEmulatorCommandParser, VMEmulatorCommand> {
    fn parse_set_target(&self, ident: &str) -> ParseResult<VMSetTarget> {
        parse_set_target(ident)
    }
}

impl<'tst> SimulatorCommandParser<VMEmulatorCommand>
    for ScriptParser<'tst, VMEmulatorCommandParser, VMEmulatorCommand>
{
    fn parse_simulator_command(&mut self, ident: Spanned<&str>) -> CmdResult<VMEmulatorCommand> {
        match ident.content {
            "load" => {
                // the filename is actually optional
                let (filename, end_idx) = if let Token::Symbol(_) = self.peek_token()?.content {
                    (None, ident.end_idx)
                } else {
                    let filename = self.consume_token_kind(ident_kind())?;
                    if let Token::Identifier(name) = filename.content {
                        (Some(name), filename.end_idx)
                    } else {
                        unreachable!()
                    }
                };

                let mut filepath = self.path.to_owned();
                if let Some(filename) = filename {
                    filepath.set_file_name(filename);
                } else {
                    // remove the file and just pass the directory
                    filepath.pop();
                }

                let cmd = Command::new(CommandKind::Simulator(VMEmulatorCommand::Load(filepath)));
                Ok(Spanned::new(ident.start_idx, end_idx, ident.line_nr, cmd))
            }

            "vmstep" => Ok(ident.with_new_content(Command::new(CommandKind::Simulator(
                VMEmulatorCommand::Step,
            )))),
            "set" => {
                let target = self.consume_token_kind(ident_kind())?;
                let target = if let Token::Identifier(ref target) = target.content {
                    self.parse_set_target(target)?
                } else {
                    unreachable!()
                };

                let value = self.consume_token_kind(int_kind())?;
                let value = if let Token::IntLiteral(value) = value.content {
                    value as Word
                } else {
                    unreachable!()
                };

                Ok(ident.with_new_content(Command::new(CommandKind::Simulator(
                    VMEmulatorCommand::Set(target, value),
                ))))
            }

            _ => Err(ParseError::NotASimulatorCommand(ident.content.to_string())),
        }
    }
}

pub fn parse_set_target(ident: &str) -> ParseResult<VMSetTarget> {
    let bracket_index = ident.find('[').unwrap_or(ident.len());
    let name = &ident[0..bracket_index];

    let opening_index = bracket_index;
    macro_rules! get_index {
        () => {{
            let closing_index = ident
                .find(']')
                .ok_or_else(|| ParseError::InvalidSetTarget(ident.to_string()))?;

            if closing_index <= opening_index {
                return Err(ParseError::InvalidSetTarget(ident.to_string()));
            }

            (&ident[(opening_index + 1)..closing_index])
                .parse::<usize>()
                .map_err(|_| ParseError::InvalidSetTarget(ident.to_string()))?
        }};
    }

    macro_rules! opt_index {
        ($variant:expr) => {
            Ok($variant(if opening_index < ident.len() {
                Some(get_index!())
            } else {
                None
            }))
        };
    }

    macro_rules! no_index {
        ($variant:expr) => {{
            if opening_index != ident.len() {
                return Err(ParseError::InvalidSetTarget(ident.to_string()));
            }

            Ok($variant)
        }};
    }

    macro_rules! req_index {
        ($variant:expr) => {{
            if opening_index < ident.len() {
                Ok($variant(get_index!()))
            } else {
                Err(ParseError::InvalidSetTarget(ident.to_string()))
            }
        }};
    }

    match name {
        "local" => opt_index!(VMSetTarget::Local),
        "argument" => opt_index!(VMSetTarget::Argument),
        "this" => opt_index!(VMSetTarget::This),
        "that" => opt_index!(VMSetTarget::That),
        "sp" => no_index!(VMSetTarget::SP),
        "currentFunction" => no_index!(VMSetTarget::CurrentFunction),
        "line" => no_index!(VMSetTarget::Line),
        "temp" => req_index!(VMSetTarget::Temp),
        "RAM" => req_index!(VMSetTarget::Ram),
        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parse::script::lexer::int_kind;
    use crate::parse::script::tst::*;

    use std::path::PathBuf;

    #[test]
    fn test_parser_consume_repeat_vmstep() {
        let parser = ScriptParser::<VMEmulatorCommandParser, VMEmulatorCommand>::new(
            &Path::new("Test.tst"),
            "repeat 42 {vmstep;}",
        );
        assert_eq!(
            vec![Ok(Spanned::new(
                0,
                19,
                1,
                Command::new(CommandKind::Repeat {
                    times: 42,
                    block: vec![Command::terminated(
                        CommandKind::Simulator(VMEmulatorCommand::Step),
                        Terminator::SingleStep
                    )]
                },)
            ))],
            parser.collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parser_load_vm_file() {
        let parser = ScriptParser::<VMEmulatorCommandParser, VMEmulatorCommand>::new(
            Path::new("Test.tst"),
            "load StackTest.vm,",
        );
        assert_eq!(
            vec![Ok(Spanned::new(
                0,
                18,
                1,
                Command::terminated(
                    CommandKind::Simulator(VMEmulatorCommand::Load(PathBuf::from("StackTest.vm"))),
                    Terminator::MiniStep
                )
            ))],
            parser.collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parser_consume_token_kind_should_return_token_only_if_discriminant_matches() {
        let mut parser = ScriptParser::<VMEmulatorCommandParser, VMEmulatorCommand>::new(
            Path::new("Test.tst"),
            "hello 42",
        );
        assert_eq!(
            Err(ParseError::ExpectedKind(int_kind())),
            parser.consume_token_kind(int_kind())
        );
        assert_eq!(
            Ok(Spanned::new(
                0,
                5,
                1,
                Token::Identifier("hello".to_string())
            )),
            parser.consume_token_kind(ident_kind())
        );
        assert_eq!(
            Ok(Spanned::new(6, 8, 1, Token::IntLiteral(42))),
            parser.consume_token_kind(int_kind())
        );
    }
}

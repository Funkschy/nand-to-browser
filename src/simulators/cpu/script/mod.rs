use crate::definitions::Word;
use crate::parse::script::lexer::{ident_kind, int_kind, Token};
use crate::parse::script::parser::ScriptParser;
use crate::parse::script::tst::{Command, CommandKind, CpuEmulatorCommand, CpuSetTarget};
use crate::parse::script::{CmdResult, ParseError, ParseResult, SimulatorCommandParser};
use crate::parse::Spanned;

mod run;

#[derive(Default)]
pub struct CpuEmulatorCommandParser {}

impl<'tst> ScriptParser<'tst, CpuEmulatorCommandParser, CpuEmulatorCommand> {
    fn parse_set_target(&self, ident: &str) -> ParseResult<CpuSetTarget> {
        parse_set_target(ident)
    }
}

impl<'tst> SimulatorCommandParser<CpuEmulatorCommand>
    for ScriptParser<'tst, CpuEmulatorCommandParser, CpuEmulatorCommand>
{
    fn parse_simulator_command(&mut self, ident: Spanned<&str>) -> CmdResult<CpuEmulatorCommand> {
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

                let cmd = Command::new(CommandKind::Simulator(CpuEmulatorCommand::Load(filepath)));
                Ok(Spanned::new(ident.start_idx, end_idx, ident.line_nr, cmd))
            }

            "ticktock" => Ok(ident.with_new_content(Command::new(CommandKind::Simulator(
                CpuEmulatorCommand::TickTock,
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
                    CpuEmulatorCommand::Set(target, value),
                ))))
            }

            _ => Err(ParseError::NotASimulatorCommand(ident.content.to_string())),
        }
    }
}

pub fn parse_set_target(ident: &str) -> ParseResult<CpuSetTarget> {
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

    match name.to_lowercase().as_str() {
        "a" => no_index!(CpuSetTarget::A),
        "d" => no_index!(CpuSetTarget::D),
        "pc" => no_index!(CpuSetTarget::PC),
        "ram" => req_index!(CpuSetTarget::Ram),
        "rom" => req_index!(CpuSetTarget::Rom),
        _ => todo!(),
    }
}

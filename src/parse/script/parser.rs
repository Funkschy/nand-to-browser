use super::lexer::{ident_kind, int_kind, string_kind, Keyword, Lexer, Symbol, Token};
use super::tst::{Command, CommandKind, OutputListEntry, SimulatorCommand, Terminator};
use super::{CmdResult, ParseError, ParseResult, SimulatorCommandParser, Spanned, TokResult};
use lazy_static::lazy_static;
use regex::Regex;

use std::iter::Peekable;
use std::marker::PhantomData;
use std::mem::{discriminant, Discriminant};
use std::path::{Path, PathBuf};

pub struct ScriptParser<'tst, P, SimCmd> {
    pub(crate) path: &'tst Path,
    lexer: Peekable<Lexer<'tst>>,
    phantom_parser: PhantomData<P>,
    phantom_command: PhantomData<SimCmd>,
}

impl<'tst, P, SimCmd> ScriptParser<'tst, P, SimCmd>
where
    ScriptParser<'tst, P, SimCmd>: SimulatorCommandParser<SimCmd>,
    SimCmd: SimulatorCommand,
{
    pub fn new(path: &'tst Path, source: &'tst str) -> Self {
        ScriptParser {
            path,
            lexer: Lexer::new(source).peekable(),
            phantom_parser: PhantomData,
            phantom_command: PhantomData,
        }
    }

    fn sibling_file(&self, filename: &str) -> PathBuf {
        let mut path = self.path.to_owned();
        path.set_file_name(filename);
        path
    }

    fn controller_command(&mut self, kw: Spanned<Keyword>) -> CmdResult<SimCmd> {
        use CommandKind::{CompareTo, Echo, Output, OutputFile, OutputList, Repeat};

        match kw.content {
            Keyword::Repeat => {
                let Spanned {
                    start_idx, line_nr, ..
                } = kw;

                let count = if let Ok(Spanned {
                    content: Token::IntLiteral(_),
                    ..
                }) = self.peek_token()
                {
                    if let Spanned {
                        content: Token::IntLiteral(count),
                        ..
                    } = self.consume_token_kind(int_kind())?
                    {
                        count as usize
                    } else {
                        unreachable!()
                    }
                } else {
                    // HACK: technically this should be infinite
                    usize::MAX
                };

                self.consume_token_exact(Token::Symbol(Symbol::OpenBrace))?;

                let mut block = vec![];
                while self.lexer.peek() != None {
                    if self.peek_expect_token(Token::Symbol(Symbol::CloseBrace))? {
                        break;
                    }
                    block.push(self.next_command()?.content);
                }

                let closing = self.consume_token_exact(Token::Symbol(Symbol::CloseBrace))?;
                let end_idx = closing.end_idx;

                let cmd = Command::new(Repeat {
                    times: count,
                    block,
                });

                Ok(Spanned::new(start_idx, end_idx, line_nr, cmd))
            }
            Keyword::OutputFile => {
                let mut token = self.consume_token_kind(ident_kind())?;
                let cmd = if let Token::Identifier(ref filename) = token.content {
                    Command::new(OutputFile(self.sibling_file(filename)))
                } else {
                    unreachable!()
                };

                token.start_idx = kw.start_idx;
                self.consume_terminator(token.with_new_content(cmd))
            }
            Keyword::CompareTo => {
                let mut token = self.consume_token_kind(ident_kind())?;
                let cmd = if let Token::Identifier(ref filename) = token.content {
                    Command::new(CompareTo(self.sibling_file(filename)))
                } else {
                    unreachable!()
                };

                token.start_idx = kw.start_idx;
                self.consume_terminator(token.with_new_content(cmd))
            }
            Keyword::Output => {
                let cmd = Command::new(Output);
                self.consume_terminator(kw.with_new_content(cmd))
            }
            Keyword::OutputList => {
                let mut entries = Vec::new();
                loop {
                    let peeked = self.peek_token()?;
                    if let Token::Symbol(_) = peeked.content {
                        break;
                    }

                    let token = self.next_token()?;
                    let entry = Self::ident_to_outputlist_entry(token)?;
                    entries.push(entry);
                }

                if entries.is_empty() {
                    return Err(ParseError::EmptyOutputList);
                }

                let last = entries.first().unwrap();
                let start_idx = kw.start_idx;
                let line_nr = kw.line_nr;
                let end_idx = last.end_idx;

                let entries = entries.into_iter().map(|e| e.content).collect();
                let cmd = Command::new(OutputList(entries));

                self.consume_terminator(Spanned::new(start_idx, end_idx, line_nr, cmd))
            }
            Keyword::Echo => {
                let token = self.consume_token_kind(string_kind())?;
                let mut spanned = token.with_new_content(());
                let cmd = if let Token::StringLiteral(literal) = token.content {
                    Command::new(Echo(literal))
                } else {
                    unreachable!()
                };

                spanned.start_idx = kw.start_idx;
                self.consume_terminator(spanned.with_new_content(cmd))
            }
            _ => unimplemented!("Keyword {:?} not handled", kw),
        }
    }

    fn consume_terminator(&mut self, mut cmd: Spanned<Command<SimCmd>>) -> CmdResult<SimCmd> {
        use Symbol::{Bang, Comma, Semicolon};
        use Terminator::{MiniStep, SingleStep, Stop};

        let peek = self.peek_token()?;
        macro_rules! term {
            ($self:expr, $sym:expr, $term:expr) => {{
                let term = self.consume_token_exact(Token::Symbol($sym))?;
                cmd.end_idx = term.end_idx;
                cmd.content.terminator = $term;
                Ok(cmd)
            }};
        }
        match peek.content {
            Token::Symbol(Comma) => term!(self, Comma, MiniStep),
            Token::Symbol(Semicolon) => term!(self, Semicolon, SingleStep),
            Token::Symbol(Symbol::Bang) => term!(self, Bang, Stop),
            _ => Err(ParseError::UnterminatedCommand),
        }
    }

    fn peek_expect_token(&mut self, expected: Token) -> ParseResult<bool> {
        if let Ok(peek) = self.peek_token() {
            Ok(peek.content == expected)
        } else {
            Err(ParseError::Expected(expected))
        }
    }

    fn ident_to_outputlist_entry(ident: Spanned<Token>) -> ParseResult<Spanned<OutputListEntry>> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new("(?P<name>[a-zA-Z-_]+(\\[\\d+\\])?)%(?P<format>[BDXS])(?P<left_pad>\\d+)\\.(?P<length>\\d+)\\.(?P<right_pad>\\d+)").unwrap();
        }

        if let Token::Identifier(lexeme) = &ident.content {
            let caps = RE
                .captures(lexeme)
                .ok_or(ParseError::CouldNotParseOutputListEntry)?;

            let extract_num = |name: &str| {
                caps[name]
                    .parse()
                    .or(Err(ParseError::CouldNotParseOutputListEntry))
            };

            let name = caps["name"].to_string();
            let format = caps["format"]
                .try_into()
                .or(Err(ParseError::CouldNotParseOutputListEntry))?;
            let left_padding = extract_num("left_pad")?;
            let length = extract_num("length")?;
            let right_padding = extract_num("right_pad")?;

            return Ok(ident.with_new_content(OutputListEntry::new(
                name,
                format,
                left_padding,
                length,
                right_padding,
            )));
        }

        Err(ParseError::CouldNotParseOutputListEntry)
    }

    fn next_command(&mut self) -> CmdResult<SimCmd> {
        let next_token = self.next_token()?;
        match &next_token.content {
            Token::Keyword(kw) => self.controller_command(next_token.with_new_content(*kw)),
            Token::Identifier(ident) => {
                let cmd = self.parse_simulator_command(next_token.with_new_content(ident))?;
                self.consume_terminator(cmd)
            }
            Token::Symbol(_) => Err(ParseError::CommandStartingWithSymbol),
            Token::IntLiteral(_) => Err(ParseError::CommandStartingWithInt),
            Token::StringLiteral(_) => Err(ParseError::CommandStartingWithString),
        }
    }

    pub fn peek_token(&mut self) -> ParseResult<&Spanned<Token>> {
        self.lexer.peek().ok_or(ParseError::NoNextItem)
    }

    pub fn next_token(&mut self) -> ParseResult<Spanned<Token>> {
        self.lexer.next().ok_or(ParseError::NoNextItem)
    }

    pub fn consume_token_kind(&mut self, expected: Discriminant<Token>) -> TokResult {
        if let Ok(peek) = self.peek_token() {
            if discriminant(&peek.content) == expected {
                return self.next_token();
            }
        }

        Err(ParseError::ExpectedKind(expected))
    }

    fn consume_token_exact(&mut self, expected: Token) -> TokResult {
        if let Ok(peek) = self.peek_token() {
            if peek.content == expected {
                return self.next_token();
            }
        }

        Err(ParseError::Expected(expected))
    }
}

impl<'tst, P, SimCmd> Iterator for ScriptParser<'tst, P, SimCmd>
where
    ScriptParser<'tst, P, SimCmd>: SimulatorCommandParser<SimCmd>,
    SimCmd: SimulatorCommand,
{
    type Item = CmdResult<SimCmd>;

    fn next(&mut self) -> Option<Self::Item> {
        let cmd = self.next_command();
        match cmd {
            Err(ParseError::NoNextItem) => None,
            _ => Some(cmd),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parse::script::tst::NumberFormat;

    #[derive(Debug, PartialEq, Eq, Clone)]
    enum MockSimulatorCommand {
        Step,
    }

    impl SimulatorCommand for MockSimulatorCommand {}

    #[derive(Default)]
    struct MockSimulatorParser {}

    impl<'tst> SimulatorCommandParser<MockSimulatorCommand>
        for ScriptParser<'tst, MockSimulatorParser, MockSimulatorCommand>
    {
        fn parse_simulator_command(
            &mut self,
            ident: Spanned<&str>,
        ) -> CmdResult<MockSimulatorCommand> {
            if ident.content == "vmstep" {
                return Ok(ident.with_new_content(Command::new(CommandKind::Simulator(
                    MockSimulatorCommand::Step,
                ))));
            }

            unimplemented!("this should not have been called!")
        }
    }

    #[test]
    fn test_parser_consume_repeat_empty() {
        let parser = ScriptParser::<MockSimulatorParser, MockSimulatorCommand>::new(
            &Path::new("Test.tst"),
            "repeat 42 {}",
        );
        assert_eq!(
            vec![Ok(Spanned::new(
                0,
                12,
                1,
                Command::new(CommandKind::Repeat {
                    times: 42,
                    block: vec![]
                },)
            ))],
            parser.collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parser_consume_repeat_without_count() {
        let mut parser = ScriptParser::<MockSimulatorParser, MockSimulatorCommand>::new(
            &Path::new("Test.tst"),
            "repeat {}",
        );
        assert_eq!(
            Ok(Spanned::new(
                0,
                9,
                1,
                Command::new(CommandKind::Repeat {
                    times: usize::MAX,
                    block: vec![]
                },)
            )),
            parser.next_command()
        );
    }

    #[test]
    fn test_parser_consume_repeat_without_closing() {
        let mut parser = ScriptParser::<MockSimulatorParser, MockSimulatorCommand>::new(
            &Path::new("Test.tst"),
            "repeat 42 {",
        );
        assert_eq!(
            Err(ParseError::Expected(Token::Symbol(Symbol::CloseBrace))),
            parser.next_command()
        );
    }

    #[test]
    fn test_parser_consume_token_kind_should_return_token_only_if_discriminant_matches() {
        let mut parser = ScriptParser::<MockSimulatorParser, MockSimulatorCommand>::new(
            &Path::new("Test.tst"),
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

    #[test]
    fn test_parse_output_list() {
        let mut parser = ScriptParser::<MockSimulatorParser, MockSimulatorCommand>::new(
            &Path::new("Test.tst"),
            "output-list a%B1.16.1 b%X2.2.1 out%D1.1.1;",
        );

        let result = parser.next();
        assert_eq!(
            result,
            Some(Ok(Spanned::new(
                0,
                42,
                1,
                Command::terminated(
                    CommandKind::OutputList(vec![
                        OutputListEntry::new("a".to_string(), NumberFormat::Binary, 1, 16, 1),
                        OutputListEntry::new("b".to_string(), NumberFormat::Hex, 2, 2, 1),
                        OutputListEntry::new("out".to_string(), NumberFormat::Decimal, 1, 1, 1)
                    ]),
                    Terminator::SingleStep
                )
            )))
        );
    }
}

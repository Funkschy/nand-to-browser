use super::symbols::SymbolTable;
use super::{Spanned, StringLexer};
use crate::definitions::Symbol;
use crate::definitions::Word;
use std::iter::Peekable;
use std::num::ParseIntError;

use crate::simulators::cpu::command::{Computation, Destination, Instruction, Jump, Register};

use std::error;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedCharacter(char),
    // technically not a real error, but it's easier to handle it like one
    EndOfFile,
    UnexpectedEndOfFile,
    InvalidIntLiteral(ParseIntError),
    InvalidIntComp(Word),
    InvalidDestination(String),
    ExpectedLabelOrConstant,
    ExpectedComputation,
    ExpectedIdent,
    ExpectedRegister,
    ExpectedOperator,
    ExpectedJump,
    InvalidToken,
}

impl From<ParseIntError> for ParseError {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidIntLiteral(err)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(c) => write!(f, "Unexpected character: {}", c),
            Self::EndOfFile => write!(f, "EOF"),
            Self::UnexpectedEndOfFile => write!(f, "Unexpected end of file"),
            Self::InvalidIntLiteral(error) => write!(f, "Could not parse int: {}", error),
            Self::InvalidIntComp(value) => {
                write!(f, "Only 0, 1 and -1 are allowed, but got {}", value)
            }
            Self::InvalidDestination(dest) => write!(f, "Invalid destination: {}", dest),
            Self::ExpectedLabelOrConstant => {
                write!(f, "Expected label or a positive integer literal after an @")
            }
            Self::ExpectedComputation => write!(f, "Expected a computation"),
            Self::ExpectedIdent => write!(f, "Expected identifier"),
            Self::ExpectedRegister => write!(f, "Expected register"),
            Self::ExpectedOperator => write!(f, "Expected operator"),
            Self::ExpectedJump => write!(f, "Expected jump"),
            Self::InvalidToken => write!(f, "Invalid token"),
        }
    }
}

impl error::Error for ParseError {}

type ParseResult<T> = Result<T, ParseError>;

#[derive(Eq, PartialEq, Debug)]
enum Token<'src> {
    ASym(&'src str),
    AConst(u16),
    Identifier(&'src str),
    Label(&'src str),
    IntLiteral(Word),
    Semi,
    Eq,
    Plus,
    Minus,
    Bang,
    Ampersand,
    Pipe,
}

impl<'src> Token<'src> {
    pub fn is_binary_operator(&self) -> bool {
        match self {
            Self::Plus | Self::Minus | Self::Ampersand | Self::Pipe => true,
            _ => false,
        }
    }
}

struct Lexer<'src> {
    walker: StringLexer<'src>,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            walker: StringLexer::new(source),
        }
    }

    fn consume_ident(&mut self) -> ParseResult<Spanned<&'src str>> {
        self.walker
            .take_chars_while(|c| c.is_alphanumeric() || c == '_' || c == '.')
            .ok_or(ParseError::UnexpectedEndOfFile)
    }

    fn consume_single(&mut self, tok: Token<'src>) -> ParseResult<Spanned<Token<'src>>> {
        self.walker
            .advance()
            .map(|spanned| spanned.with_new_content(tok))
            .ok_or(ParseError::UnexpectedEndOfFile)
    }

    fn current_char(&mut self) -> ParseResult<Spanned<char>> {
        self.walker.current_char().ok_or(ParseError::EndOfFile)
    }

    fn advance(&mut self) -> ParseResult<Spanned<char>> {
        self.walker.advance().ok_or(ParseError::EndOfFile)
    }

    fn scan_token(&mut self) -> ParseResult<Spanned<Token<'src>>> {
        // skip whitespace
        self.walker.take_chars_while(char::is_whitespace);

        let Spanned {
            content: current_char,
            ..
        } = self.current_char()?;

        match current_char {
            '/' => {
                self.walker.advance();
                if self.walker.current_eq('/') {
                    self.walker.take_chars_while(|c| c != '\n');
                    self.scan_token()
                } else {
                    Err(ParseError::UnexpectedCharacter(current_char))
                }
            }
            ';' => self.consume_single(Token::Semi),
            '=' => self.consume_single(Token::Eq),
            '+' => self.consume_single(Token::Plus),
            '!' => self.consume_single(Token::Bang),
            '&' => self.consume_single(Token::Ampersand),
            '|' => self.consume_single(Token::Pipe),
            '-' => {
                let mut minus = self.advance()?;
                let next = self.current_char()?;
                // -1 is a special case
                Ok(if '1' == next.content {
                    let next = self.advance()?;
                    minus.end_idx = next.end_idx;
                    minus.with_new_content(Token::IntLiteral(-1))
                } else {
                    minus.with_new_content(Token::Minus)
                })
            }
            c if c.is_numeric() => {
                let spanned = self
                    .walker
                    .take_chars_while(char::is_numeric)
                    .ok_or(ParseError::UnexpectedEndOfFile)?;
                let parsed_int = spanned.content.parse::<i16>()?;
                Ok(spanned.with_new_content(Token::IntLiteral(parsed_int)))
            }
            '@' => {
                // skip @
                let at = self.advance()?;

                let mut next = self.scan_token()?;
                match next.content {
                    Token::Identifier(name) => {
                        next.start_idx = at.start_idx;
                        Ok(next.with_new_content(Token::ASym(name)))
                    }
                    Token::IntLiteral(constant) if constant >= 0 => {
                        next.start_idx = at.start_idx;
                        Ok(next.with_new_content(Token::AConst(constant as u16)))
                    }
                    _ => Err(ParseError::ExpectedLabelOrConstant),
                }
            }
            '(' => {
                // skip (
                self.advance()?;

                let label = self
                    .walker
                    .take_chars_while(|c| c != ')')
                    .ok_or(ParseError::ExpectedIdent)?;

                // skip )
                // the next char has to be either ) or eof
                self.advance()?;

                Ok(label.with_new_content(Token::Label(label.content)))
            }
            c if c.is_alphabetic() => {
                let ident = self.consume_ident()?;
                let wrapped_content = Token::Identifier(ident.content);
                Ok(ident.with_new_content(wrapped_content))
            }
            _ => Err(ParseError::UnexpectedCharacter(current_char)),
        }
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = ParseResult<Spanned<Token<'src>>>;
    fn next(&mut self) -> Option<Self::Item> {
        let next_token = self.scan_token();
        if let Err(ParseError::EndOfFile) = next_token {
            return None;
        }
        Some(next_token)
    }
}

pub struct SourceFile<'src> {
    lexer: Peekable<Lexer<'src>>,
}

impl<'src> SourceFile<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            lexer: Lexer::new(source).peekable(),
        }
    }
}

pub struct Parser<'src> {
    source: SourceFile<'src>,
    symbols: SymbolTable,
}

impl<'src> Parser<'src> {
    pub fn new(source: SourceFile<'src>) -> Self {
        Self {
            source,
            symbols: SymbolTable::default(),
        }
    }

    fn next_token(&mut self) -> ParseResult<Token<'src>> {
        self.source
            .lexer
            .next()
            .ok_or(ParseError::EndOfFile)?
            .map(|s| s.content)
    }

    fn consume_token(&mut self, token: Token<'src>) -> ParseResult<Token<'src>> {
        if token == self.next_token()? {
            Ok(token)
        } else {
            Err(ParseError::InvalidToken)
        }
    }

    fn lookup_symbol(&mut self, ident: &'src str) -> Result<Symbol, &'src str> {
        if let Some(symbol) = self.symbols.lookup(ident) {
            return Ok(symbol);
        }

        Err(ident)
    }

    fn int_as_comp(&self, value: Word) -> ParseResult<Computation> {
        Ok(match value {
            0 => Computation::ConstZero,
            1 => Computation::ConstOne,
            -1 => Computation::ConstNegOne,
            _ => return Err(ParseError::InvalidIntComp(value)),
        })
    }

    fn consume_reg(&mut self) -> ParseResult<Register> {
        let token = self.next_token()?;
        if let Token::Identifier(ident) = token {
            ident.try_into().map_err(|_| ParseError::ExpectedRegister)
        } else {
            Err(ParseError::ExpectedRegister)
        }
    }

    fn consume_comp(&mut self) -> ParseResult<Computation> {
        let token = self.next_token()?;

        match token {
            // literal
            Token::IntLiteral(value) => self.int_as_comp(value),
            // unary
            Token::Bang => Ok(Computation::UnaryBoolNeg(self.consume_reg()?)),
            Token::Minus => Ok(Computation::UnaryIntNeg(self.consume_reg()?)),
            // binary
            Token::Identifier(lhs) => {
                let lhs: Register = lhs.try_into().map_err(|_| ParseError::ExpectedRegister)?;

                if let Some(Ok(token)) = self.source.lexer.peek() {
                    if let Token::IntLiteral(-1) = token.content {
                        // this is a special case because the lexer will interpret A-1 as token(A), token(-1)
                        // instead of token(A), token(-), token(1)
                        self.next_token()?;
                        return Ok(Computation::BinaryDec(lhs));
                    }

                    if !token.content.is_binary_operator() {
                        return Ok(Computation::UnaryNone(lhs));
                    }
                } else {
                    // no next token => the file ends with something like R1=R2
                    return Ok(Computation::UnaryNone(lhs));
                }

                let operator = self.next_token()?;

                let rhs_peek = if let Some(Ok(spanned)) = self.source.lexer.peek() {
                    Some(&spanned.content)
                } else {
                    None
                };

                match (operator, rhs_peek) {
                    (Token::Plus, Some(Token::Identifier(_))) => {
                        let rhs = self.consume_reg()?;
                        Ok(Computation::BinaryAdd(lhs, rhs))
                    }
                    (Token::Plus, Some(Token::IntLiteral(1))) => {
                        let _ = self.next_token()?;
                        Ok(Computation::BinaryInc(lhs))
                    }
                    (Token::Minus, Some(Token::Identifier(_))) => {
                        let rhs = self.consume_reg()?;
                        Ok(Computation::BinarySub(lhs, rhs))
                    }
                    (Token::Minus, Some(Token::IntLiteral(1))) => {
                        let _ = self.next_token()?;
                        Ok(Computation::BinaryDec(lhs))
                    }
                    (Token::Ampersand, Some(Token::Identifier(_))) => {
                        let rhs = self.consume_reg()?;
                        Ok(Computation::BinaryAnd(lhs, rhs))
                    }
                    (Token::Pipe, Some(Token::Identifier(_))) => {
                        let rhs = self.consume_reg()?;
                        Ok(Computation::BinaryOr(lhs, rhs))
                    }
                    _ => Err(ParseError::ExpectedOperator),
                }
            }
            _ => Err(ParseError::ExpectedComputation),
        }
    }

    fn consume_jump(&mut self) -> ParseResult<Jump> {
        let is_semi = self
            .source
            .lexer
            .peek()
            .and_then(|r| r.as_ref().map(|s| Token::Semi == s.content).ok())
            .unwrap_or(false);

        if !is_semi {
            return Ok(Jump::Next);
        }

        self.consume_token(Token::Semi)?;

        let token = self.next_token()?;
        match token {
            Token::Identifier("JGT") => Ok(Jump::Gt),
            Token::Identifier("JEQ") => Ok(Jump::Eq),
            Token::Identifier("JGE") => Ok(Jump::Ge),
            Token::Identifier("JLT") => Ok(Jump::Lt),
            Token::Identifier("JNE") => Ok(Jump::Ne),
            Token::Identifier("JLE") => Ok(Jump::Le),
            Token::Identifier("JMP") => Ok(Jump::Unconditional),
            _ => Err(ParseError::ExpectedJump),
        }
    }

    pub fn parse(&mut self) -> ParseResult<Vec<Instruction>> {
        enum CodeEntry<'src> {
            Instruction(Instruction),
            WaitingForLabel(&'src str, Instruction),
        }

        let mut code: Vec<CodeEntry<'src>> = Vec::with_capacity(128);

        fn push_instr(code: &mut Vec<CodeEntry>, value: Instruction) {
            code.push(CodeEntry::Instruction(value));
        }

        fn push_target<'src>(
            code: &mut Vec<CodeEntry<'src>>,
            target: Result<Symbol, &'src str>,
            instr: Instruction,
        ) {
            match target {
                Ok(addr) => {
                    if let Instruction::A(_) = instr {
                        push_instr(code, Instruction::A(addr));
                    }
                }
                Err(waiting_for) => code.push(CodeEntry::WaitingForLabel(waiting_for, instr)),
            };
        }

        loop {
            let token = self.next_token();
            if let Err(ParseError::EndOfFile) = token {
                break;
            }

            match token? {
                Token::AConst(value) => push_instr(&mut code, Instruction::A(value)),
                Token::ASym(expected) => {
                    let target = self.lookup_symbol(expected);
                    push_target(&mut code, target, Instruction::A(0));
                }
                Token::Label(label) => {
                    let symbol = code.len() as Symbol;
                    self.symbols.set(label, symbol);
                }
                Token::IntLiteral(value) => {
                    push_instr(
                        &mut code,
                        Instruction::C(
                            Destination::default(),
                            self.int_as_comp(value)?,
                            self.consume_jump()?,
                        ),
                    );
                }
                Token::Identifier(ident) => {
                    if let Some(Ok(Spanned {
                        content: Token::Semi,
                        ..
                    })) = self.source.lexer.peek()
                    {
                        // no dest, just Register + jump
                        let reg = ident.try_into().map_err(|_| ParseError::ExpectedRegister)?;
                        push_instr(
                            &mut code,
                            Instruction::C(
                                Destination::None,
                                Computation::UnaryNone(reg),
                                self.consume_jump()?,
                            ),
                        );
                        continue;
                    }

                    let dest: Destination = ident
                        .try_into()
                        .map_err(|_| ParseError::InvalidDestination(ident.to_owned()))?;

                    self.consume_token(Token::Eq)?;

                    push_instr(
                        &mut code,
                        Instruction::C(dest, self.consume_comp()?, self.consume_jump()?),
                    );
                }

                _ => return Err(ParseError::InvalidToken),
            };
        }

        let mut instructions = Vec::with_capacity(code.capacity());

        for c in code {
            match c {
                CodeEntry::Instruction(instr) => instructions.push(instr),
                CodeEntry::WaitingForLabel(label, _) => {
                    let resolved = self.symbols.lookup_or_insert(label);
                    instructions.push(Instruction::A(resolved));
                }
            }
        }

        Ok(instructions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minus_one_edge_case() {
        let mut parser = Parser::new(SourceFile::new("D=A-1"));
        let instructions = parser.parse();

        assert_eq!(
            instructions,
            Ok(vec![Instruction::C(
                Destination::try_from("D").unwrap(),
                Computation::BinaryDec(Register::A),
                Jump::Next
            )])
        );

        let mut parser = Parser::new(SourceFile::new("D = A - 1 "));
        let instructions = parser.parse();

        assert_eq!(
            instructions,
            Ok(vec![Instruction::C(
                Destination::try_from("D").unwrap(),
                Computation::BinaryDec(Register::A),
                Jump::Next
            )])
        );
    }

    #[test]
    fn test_minus_one_edge_case_with_jump() {
        let mut parser = Parser::new(SourceFile::new("D=A-1;JEQ"));
        let instructions = parser.parse();

        assert_eq!(
            instructions,
            Ok(vec![Instruction::C(
                Destination::try_from("D").unwrap(),
                Computation::BinaryDec(Register::A),
                Jump::Eq
            )])
        );

        let mut parser = Parser::new(SourceFile::new("D = A - 1 ; JEQ "));
        let instructions = parser.parse();

        assert_eq!(
            instructions,
            Ok(vec![Instruction::C(
                Destination::try_from("D").unwrap(),
                Computation::BinaryDec(Register::A),
                Jump::Eq
            )])
        );
    }

    #[test]
    fn test_parse_multiple() {
        let mut parser = Parser::new(SourceFile::new("D=A-1;JEQ\nA=-1"));
        let instructions = parser.parse();

        assert_eq!(
            instructions,
            Ok(vec![
                Instruction::C(
                    Destination::try_from("D").unwrap(),
                    Computation::BinaryDec(Register::A),
                    Jump::Eq
                ),
                Instruction::C(
                    Destination::try_from("A").unwrap(),
                    Computation::ConstNegOne,
                    Jump::Next
                )
            ])
        );
    }

    #[test]
    fn test_parse_full_program() {
        let src = r#"
            // Adds 1 + ... + 100
            @i
            M=1
            // i=1
            @sum
            M=0
            // // sum=0
            (LOOP)
            @i
            D=M
            // D=i
            @100
            D=D-A // D=i-100
            @END
            D;JGT // if (i-100)>0 goto END
            @i
            D=M
            // D=i
            @sum
            M=D+M // sum=sum+i
            @i
            M=M+1 // i=i+1
            @LOOP
            0;JMP // goto LOOP
            (END)
            @END
            0;JMP // infinite loop"#;

        let mut parser = Parser::new(SourceFile::new(src));
        let instructions = parser.parse();

        assert_eq!(
            instructions,
            Ok(vec![
                Instruction::A(16),
                Instruction::C(Destination::M, Computation::ConstOne, Jump::Next),
                Instruction::A(17),
                Instruction::C(Destination::M, Computation::ConstZero, Jump::Next),
                Instruction::A(16),
                Instruction::C(
                    Destination::D,
                    Computation::UnaryNone(Register::M),
                    Jump::Next
                ),
                Instruction::A(100),
                Instruction::C(
                    Destination::D,
                    Computation::BinarySub(Register::D, Register::A),
                    Jump::Next
                ),
                Instruction::A(18),
                Instruction::C(
                    Destination::None,
                    Computation::UnaryNone(Register::D),
                    Jump::Gt
                ),
                Instruction::A(16),
                Instruction::C(
                    Destination::D,
                    Computation::UnaryNone(Register::M),
                    Jump::Next
                ),
                Instruction::A(17),
                Instruction::C(
                    Destination::M,
                    Computation::BinaryAdd(Register::D, Register::M),
                    Jump::Next
                ),
                Instruction::A(16),
                Instruction::C(
                    Destination::M,
                    Computation::BinaryInc(Register::M),
                    Jump::Next
                ),
                Instruction::A(4),
                Instruction::C(
                    Destination::None,
                    Computation::ConstZero,
                    Jump::Unconditional
                ),
                Instruction::A(18),
                Instruction::C(
                    Destination::None,
                    Computation::ConstZero,
                    Jump::Unconditional
                ),
            ])
        );
    }
}

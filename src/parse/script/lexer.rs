use crate::parse::{Spanned, StringLexer};
use std::iter::Iterator;
use std::mem::{discriminant, Discriminant};

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Keyword {
    OutputFile,
    CompareTo,
    OutputList,
    Output,
    Breakpoint,
    ClearBreakpoints,
    Repeat,
    While,
    Echo,
    ClearEcho,
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Symbol {
    OpenBrace,  // {
    CloseBrace, // }
    Comma,      // ,
    Semicolon,  // ;
    Bang,       // !
    Eq,         // =
    Gt,         // >
    St,         // <
}

#[derive(Eq, PartialEq, Debug)]
pub enum Token {
    Keyword(Keyword),
    Symbol(Symbol),
    Identifier(String),
    IntLiteral(i32),
}

pub fn int_kind() -> Discriminant<Token> {
    discriminant(&Token::IntLiteral(0))
}

pub fn ident_kind() -> Discriminant<Token> {
    discriminant(&Token::Identifier(String::new()))
}

pub struct Lexer<'tst> {
    walker: StringLexer<'tst>,
}

impl<'tst> Lexer<'tst> {
    pub fn new(source: &'tst str) -> Self {
        Self {
            walker: StringLexer::new(source),
        }
    }

    fn consume_ident(&mut self) -> Option<Spanned<&str>> {
        self.walker.take_chars_while(|c| {
            c.is_alphanumeric()
                || c == '_'
                || c == '-'
                || c == '.'
                || c == ':'
                || c == '%'
                || c == '['
                || c == ']'
        })
    }

    // TODO: use Result instead of Option
    fn scan_token(&mut self) -> Option<Spanned<Token>> {
        // skip whitespace
        self.walker.take_chars_while(char::is_whitespace);

        let Spanned {
            content: current_char,
            start_idx,
            line_nr,
            ..
        } = self.walker.current_char()?;

        match current_char {
            '/' => {
                self.walker.advance();
                if self.walker.current_eq('/') {
                    // line comments
                    self.walker.take_chars_while(|c| c != '\n');
                    self.scan_token()
                } else if self.walker.current_eq('*') {
                    // block comments
                    self.walker.take_until_substr("*/");
                    self.scan_token()
                } else {
                    // syntax error
                    None
                }
            }
            c if c.is_alphabetic() => {
                let spanned = self.consume_ident()?;
                let ident = spanned.content;

                // check if the identifier is a keyword ...
                if let Some(keyword) = match ident {
                    "output-file" => Some(Keyword::OutputFile),
                    "compare-to" => Some(Keyword::CompareTo),
                    "output-list" => Some(Keyword::OutputList),
                    "output" => Some(Keyword::Output),
                    "echo" => Some(Keyword::Echo),
                    "clear-echo" => Some(Keyword::ClearEcho),
                    "breakpoint" => Some(Keyword::Breakpoint),
                    "clear-breakpoints" => Some(Keyword::ClearBreakpoints),
                    "repeat" => Some(Keyword::Repeat),
                    "while" => Some(Keyword::While),
                    _ => None,
                } {
                    return Some(spanned.with_new_content(Token::Keyword(keyword)));
                }

                // ... otherwise just return it as an identifier
                Some(spanned.with_new_content(Token::Identifier(ident.to_string())))
            }
            c if c.is_numeric() => {
                let spanned = self.walker.take_chars_while(char::is_numeric)?;
                let parsed_int = spanned.content.parse::<i32>().ok()?;
                Some(spanned.with_new_content(Token::IntLiteral(parsed_int)))
            }
            '-' => {
                let minus = self.walker.advance()?;
                let spanned = self.walker.take_chars_while(char::is_numeric)?;
                let parsed_int = spanned.content.parse::<i32>().ok()?;
                Some(Spanned::new(
                    minus.start_idx,
                    spanned.end_idx,
                    minus.line_nr,
                    Token::IntLiteral(parsed_int),
                ))
            }
            '%' => {
                self.walker.advance()?;
                let format = self.walker.advance()?;
                match format.content {
                    'B' => {
                        let literal = self.walker.take_chars_while(|c| c == '1' || c == '0')?;
                        let int = i32::from_str_radix(literal.content, 2).ok()?;
                        Some(Spanned::new(
                            start_idx,
                            literal.end_idx,
                            line_nr,
                            Token::IntLiteral(int),
                        ))
                    }
                    'X' => {
                        let literal = self
                            .walker
                            .take_chars_while(|c| c.is_numeric() || ('A'..='F').contains(&c))?;
                        let int = i32::from_str_radix(literal.content, 16).ok()?;
                        Some(Spanned::new(
                            start_idx,
                            literal.end_idx,
                            line_nr,
                            Token::IntLiteral(int),
                        ))
                    }
                    _ => None,
                }
            }
            _ => {
                let spanned = self.walker.advance()?;
                let Spanned { content, .. } = spanned;
                match content {
                    '{' => Some(Token::Symbol(Symbol::OpenBrace)),
                    '}' => Some(Token::Symbol(Symbol::CloseBrace)),
                    ',' => Some(Token::Symbol(Symbol::Comma)),
                    ';' => Some(Token::Symbol(Symbol::Semicolon)),
                    '!' => Some(Token::Symbol(Symbol::Bang)),
                    '=' => Some(Token::Symbol(Symbol::Eq)),
                    '>' => Some(Token::Symbol(Symbol::Gt)),
                    '<' => Some(Token::Symbol(Symbol::St)),
                    _ => None,
                }
                .map(|sym| spanned.with_new_content(sym))
            }
        }
    }
}

impl<'tst> Iterator for Lexer<'tst> {
    type Item = Spanned<Token>;
    fn next(&mut self) -> Option<Self::Item> {
        self.scan_token()
    }
}

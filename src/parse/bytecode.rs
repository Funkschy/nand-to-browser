use super::{Spanned, StringLexer};
use crate::simulators::vm::command::{ByteCodeParseError, Instruction, Opcode, Segment};
use std::num::ParseIntError;

use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedCharacter,
    // technically not a real error, but it's easier to handle it like one
    EndOfFile,
    UnexpectedEndOfFile,
    InvalidIntLiteral(ParseIntError),
    InvalidFileIndex,
    Bytecode(ByteCodeParseError),
    ExpectedIdent,
    ExpectedSegment,
    ExpectedInt,
    InvalidToken,
    UnresolvedLabel(String),
}

impl From<ByteCodeParseError> for ParseError {
    fn from(err: ByteCodeParseError) -> Self {
        Self::Bytecode(err)
    }
}

impl From<ParseIntError> for ParseError {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidIntLiteral(err)
    }
}

type ParseResult<T> = Result<T, ParseError>;

#[derive(Eq, PartialEq, Debug)]
enum Token<'src> {
    Identifier(&'src str),
    IntLiteral(i16),
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
            .take_chars_while(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
            .ok_or(ParseError::UnexpectedEndOfFile)
    }

    fn scan_token(&mut self) -> ParseResult<Spanned<Token<'src>>> {
        // skip whitespace
        self.walker.take_chars_while(char::is_whitespace);

        let Spanned {
            content: current_char,
            ..
        } = self.walker.current_char().ok_or(ParseError::EndOfFile)?;

        match current_char {
            '/' => {
                self.walker.advance();
                if self.walker.current_eq('/') {
                    self.walker.take_chars_while(|c| c != '\n');
                    self.scan_token()
                } else {
                    Err(ParseError::UnexpectedCharacter)
                }
            }
            c if c.is_alphabetic() => {
                let ident = self.consume_ident()?;
                let wrapped_content = Token::Identifier(ident.content);
                Ok(ident.with_new_content(wrapped_content))
            }
            c if c.is_numeric() => {
                let spanned = self
                    .walker
                    .take_chars_while(char::is_numeric)
                    .ok_or(ParseError::UnexpectedEndOfFile)?;
                let parsed_int = spanned.content.parse::<i16>()?;
                Ok(spanned.with_new_content(Token::IntLiteral(parsed_int)))
            }
            _ => Err(ParseError::UnexpectedCharacter),
        }
    }
}

type Symbol = u16;

struct SymbolTable {
    counter: Symbol,
    symbols: HashMap<String, Symbol>,
}

impl SymbolTable {
    /// Lookup a value in the symbol table
    ///
    /// if the value does not exist we create a new symbol for it
    /// and assume that this is the definition or that it will be defined later
    fn lookup(&mut self, ident: impl Into<String>) -> Option<Symbol> {
        self.symbols.get(&ident.into()).copied()
    }

    fn lookup_or_insert(&mut self, ident: impl Into<String>) -> Symbol {
        *self.symbols.entry(ident.into()).or_insert_with(|| {
            let value = self.counter;
            self.counter += 1;
            value
        })
    }

    /// Set a value in the Symbol Table explicitly
    ///
    /// this is only makes sense for Label instructions, because the Symbol in that case should
    /// be the position inside the bytecode
    fn set(&mut self, ident: impl Into<String>, value: Symbol) {
        self.symbols.insert(ident.into(), value);
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        let symbols = HashMap::with_capacity(64);

        Self {
            counter: 16, // don't overwrite SP/LCL/...
            symbols,
        }
    }
}

pub struct SourceFile<'src> {
    name: String,
    lexer: Lexer<'src>,
}

impl<'src> SourceFile<'src> {
    pub fn new(name: impl Into<String>, source: &'src str) -> Self {
        Self {
            name: name.into(),
            lexer: Lexer::new(source),
        }
    }
}

pub struct Parser<'src> {
    // the current sourcefile
    // source files are iterated in the order they're passed into new
    index: usize,
    // the index of the current instruction
    // this is used for Label Commands, which hold a symbol with their own position in
    // the sourcecode as their value
    instruction_counter: u16,
    sources: Vec<SourceFile<'src>>,
    symbols: SymbolTable,
}

impl<'src> Parser<'src> {
    pub fn new(sources: Vec<SourceFile<'src>>) -> Self {
        Self {
            index: 0,
            instruction_counter: 0,
            sources,
            symbols: SymbolTable::default(),
        }
    }

    fn lexer(&mut self) -> ParseResult<&mut Lexer<'src>> {
        self.sources
            .get_mut(self.index)
            .ok_or(ParseError::InvalidFileIndex)
            .map(|f| &mut f.lexer)
    }

    fn next_token(&mut self) -> ParseResult<Token<'src>> {
        let current_lexer = self.lexer()?;
        match current_lexer.scan_token() {
            Err(ParseError::EndOfFile) => {
                // try to continue with the next file
                self.index += 1;
                if self.index >= self.sources.len() {
                    // no more files, so just return the error
                    Err(ParseError::EndOfFile)
                } else {
                    self.next_token()
                }
            }
            Err(err) => Err(err),
            Ok(tok) => Ok(tok.content),
        }
    }

    fn consume_segment(&mut self) -> ParseResult<Segment> {
        if let Token::Identifier(ident) = self.next_token()? {
            let s = Segment::from_str(ident)?;
            Ok(s)
        } else {
            Err(ParseError::ExpectedSegment)
        }
    }

    fn consume_int(&mut self) -> ParseResult<i16> {
        if let Token::IntLiteral(literal) = self.next_token()? {
            Ok(literal)
        } else {
            Err(ParseError::ExpectedInt)
        }
    }

    fn consume_segment_with_index(&mut self) -> ParseResult<(Segment, i16)> {
        let segment = self.consume_segment()?;
        let mut index = self.consume_int()?;

        if segment == Segment::Static {
            let file_name = &self
                .sources
                .get(self.index)
                .ok_or(ParseError::InvalidFileIndex)?
                .name;
            let symbol = format!("{}.{}", file_name, index);

            index = self.symbols.lookup_or_insert(symbol) as i16;
        }

        Ok((segment, index))
    }

    fn consume_ident(&mut self) -> ParseResult<&'src str> {
        if let Token::Identifier(ident) = self.next_token()? {
            Ok(ident)
        } else {
            Err(ParseError::ExpectedIdent)
        }
    }

    fn consume_symbol(&mut self) -> ParseResult<Result<Symbol, &'src str>> {
        let ident = self.consume_ident()?;
        if let Some(symbol) = self.symbols.lookup(ident) {
            // the label for this symbol was already parsed
            Ok(Ok(symbol))
        } else {
            Ok(Err(ident))
        }
    }

    fn consume_label(&mut self) -> ParseResult<Symbol> {
        let ident = self.consume_ident()?;
        let symbol = self.instruction_counter;
        self.symbols.set(ident, symbol);
        Ok(symbol)
    }

    pub fn parse(&mut self) -> ParseResult<Vec<Opcode>> {
        enum CodeEntry<'src> {
            Opcode(Opcode),
            WaitingForLabel(&'src str),
        }

        fn split_i16(value: i16) -> (u8, u8) {
            let values = value.to_le_bytes();
            (values[0], values[1])
        }

        fn split_u16(value: u16) -> (u8, u8) {
            let values = value.to_le_bytes();
            (values[0], values[1])
        }

        fn push_instr(inst_count: &mut u16, code: &mut Vec<CodeEntry>, value: Instruction) {
            code.push(CodeEntry::Opcode(Opcode::instruction(value)));
            *inst_count += 1;
        }

        fn push_segment(inst_count: &mut u16, code: &mut Vec<CodeEntry>, value: Segment) {
            code.push(CodeEntry::Opcode(Opcode::segment(value)));
            *inst_count += 1;
        }

        fn push_i16(inst_count: &mut u16, code: &mut Vec<CodeEntry>, value: i16) {
            let (first, second) = split_i16(value);
            code.push(CodeEntry::Opcode(Opcode::constant(first)));
            code.push(CodeEntry::Opcode(Opcode::constant(second)));
            *inst_count += 2;
        }

        fn push_u16(inst_count: &mut u16, code: &mut Vec<CodeEntry>, value: u16) {
            let (first, second) = split_u16(value);
            code.push(CodeEntry::Opcode(Opcode::constant(first)));
            code.push(CodeEntry::Opcode(Opcode::constant(second)));
            *inst_count += 2;
        }

        fn wait_for<'src>(inst_count: &mut u16, code: &mut Vec<CodeEntry<'src>>, label: &'src str) {
            code.push(CodeEntry::WaitingForLabel(label));
            *inst_count += 2;
        }

        fn push_target<'src>(
            inst_count: &mut u16,
            code: &mut Vec<CodeEntry<'src>>,
            target: Result<Symbol, &'src str>,
        ) {
            match target {
                Ok(addr) => push_u16(inst_count, code, addr),
                Err(waiting_for) => wait_for(inst_count, code, waiting_for),
            };
        }

        let mut code: Vec<CodeEntry<'src>> = Vec::with_capacity(128);

        self.instruction_counter = 0;

        loop {
            let token = self.next_token();
            if let Err(ParseError::EndOfFile) = token {
                break;
            }

            match token? {
                Token::Identifier("push") => {
                    let (segment, index) = self.consume_segment_with_index()?;
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Push);
                    push_segment(&mut self.instruction_counter, &mut code, segment);
                    push_i16(&mut self.instruction_counter, &mut code, index);
                }
                Token::Identifier("pop") => {
                    let (segment, index) = self.consume_segment_with_index()?;
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Pop);
                    push_segment(&mut self.instruction_counter, &mut code, segment);
                    push_i16(&mut self.instruction_counter, &mut code, index);
                }
                Token::Identifier("if-goto") => {
                    let target = self.consume_symbol()?;
                    push_instr(
                        &mut self.instruction_counter,
                        &mut code,
                        Instruction::IfGoto,
                    );
                    push_target(&mut self.instruction_counter, &mut code, target);
                }
                Token::Identifier("goto") => {
                    let target = self.consume_symbol()?;
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Goto);
                    push_target(&mut self.instruction_counter, &mut code, target);
                }
                Token::Identifier("label") => {
                    self.consume_label()?;
                }
                Token::Identifier("function") => {
                    self.consume_label()?;
                    let n_locals = self.consume_int()?;

                    push_instr(
                        &mut self.instruction_counter,
                        &mut code,
                        Instruction::Function,
                    );
                    push_i16(&mut self.instruction_counter, &mut code, n_locals);
                }
                Token::Identifier("return") => push_instr(
                    &mut self.instruction_counter,
                    &mut code,
                    Instruction::Return,
                ),
                Token::Identifier("call") => {
                    let target = self.consume_symbol()?;
                    let n_args = self.consume_int()?;

                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Call);
                    push_target(&mut self.instruction_counter, &mut code, target);
                    push_i16(&mut self.instruction_counter, &mut code, n_args);
                }
                Token::Identifier("add") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Add);
                }
                Token::Identifier("sub") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Sub);
                }
                Token::Identifier("eq") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Eq);
                }
                Token::Identifier("gt") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Gt);
                }
                Token::Identifier("lt") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Lt);
                }
                Token::Identifier("and") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::And);
                }
                Token::Identifier("or") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Or);
                }
                Token::Identifier("not") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Not);
                }
                Token::Identifier("neg") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Neg);
                }
                _ => return Err(ParseError::InvalidToken),
            };
        }

        let mut opcodes = Vec::with_capacity(code.capacity());
        for c in code {
            match c {
                CodeEntry::Opcode(opcode) => opcodes.push(opcode),
                CodeEntry::WaitingForLabel(label) => {
                    let addr = self
                        .symbols
                        .lookup(label)
                        .ok_or_else(|| ParseError::UnresolvedLabel(label.to_string()))?;
                    let (first, second) = split_u16(addr);
                    opcodes.push(Opcode::constant(first));
                    opcodes.push(Opcode::constant(second));
                }
            }
        }

        Ok(opcodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_simple_program() {
        let string = "
            // mock definitions
            function String.new 1
            return
            function String.appendChar 2
            return";

        let source = "
            function Main.main 1
            push constant 2
            pop local 0
            push local 0
            push constant 3
            add
            pop local 0
            push constant 3
            call String.new 1
            push constant 107
            call String.appendChar 2
            push constant 101
            call String.appendChar 2
            push constant 107
            call String.appendChar 2
            pop static 0
            push constant 0
            return
            ";

        let programs = vec![
            SourceFile::new("String.vm", string),
            SourceFile::new("Simple.vm", source),
        ];
        let mut parser = Parser::new(programs);
        let code = parser.parse().unwrap();

        assert_eq!(
            code,
            vec![
                Opcode::instruction(Instruction::Function),
                Opcode::constant(1),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Return),
                Opcode::instruction(Instruction::Function),
                Opcode::constant(2),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Return),
                Opcode::instruction(Instruction::Function),
                Opcode::constant(1),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(2),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Local),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Local),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(3),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Add),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Local),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(3),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Call),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::constant(1),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(107),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Call),
                Opcode::constant(4),
                Opcode::constant(0),
                Opcode::constant(2),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(101),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Call),
                Opcode::constant(4),
                Opcode::constant(0),
                Opcode::constant(2),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(107),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Call),
                Opcode::constant(4),
                Opcode::constant(0),
                Opcode::constant(2),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(16),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Return)
            ]
        )
    }

    #[test]
    fn parse_loop_to_10() {
        let program = r#"
        push constant 0
        pop local 0
        label LOOP
        push local 0
        push constant 10
        lt
        not
        if-goto END
        push local 0
        push constant 1
        add
        pop local 0
        goto LOOP
        label END
        goto END"#;

        let mut parser = Parser::new(vec![SourceFile::new("Main.vm", program)]);
        let parsed_bytecode = parser.parse().unwrap();

        let expected_bytecode = vec![
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(0),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::Local),
            Opcode::constant(0),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Local),
            Opcode::constant(0),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(10),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Lt),
            //
            Opcode::instruction(Instruction::Not),
            //
            Opcode::instruction(Instruction::IfGoto),
            Opcode::constant(37),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Local),
            Opcode::constant(0),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(1),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Add),
            //
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::Local),
            Opcode::constant(0),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Goto),
            Opcode::constant(8),
            Opcode::constant(0),
            //
            Opcode::instruction(Instruction::Goto),
            Opcode::constant(37),
            Opcode::constant(0),
        ];

        assert_eq!(parsed_bytecode, expected_bytecode);
    }
}

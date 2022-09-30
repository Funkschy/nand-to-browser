use super::{Spanned, StringLexer};
use crate::simulators::vm::command::{ByteCodeParseError, Instruction, Opcode, Segment};
use std::num::ParseIntError;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError<'src> {
    UnexpectedCharacter,
    // technically not a real error, but it's easier to handle it like one
    EndOfFile,
    UnexpectedEndOfFile,
    InvalidIntLiteral(ParseIntError),
    InvalidFileIndex,
    InvalidFunctionIndex,
    Bytecode(ByteCodeParseError),
    ExpectedIdent,
    ExpectedSegment,
    ExpectedInt,
    InvalidToken,
    UnresolvedLocalLabel {
        label: String,
        function_name: String,
    },
    UnresolvedSymbols(HashSet<&'src str>),
}

impl<'src> From<ByteCodeParseError> for ParseError<'src> {
    fn from(err: ByteCodeParseError) -> Self {
        Self::Bytecode(err)
    }
}

impl<'src> From<ParseIntError> for ParseError<'src> {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidIntLiteral(err)
    }
}

type ParseResult<'src, T> = Result<T, ParseError<'src>>;

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

    fn consume_ident(&mut self) -> ParseResult<'src, Spanned<&'src str>> {
        self.walker
            .take_chars_while(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
            .ok_or(ParseError::UnexpectedEndOfFile)
    }

    fn scan_token(&mut self) -> ParseResult<'src, Spanned<Token<'src>>> {
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
    module_index: usize,
    // the index of the current instruction
    // this is used for Label Commands, which hold a symbol with their own position in
    // the sourcecode as their value
    instruction_counter: u16,
    sources: Vec<SourceFile<'src>>,
    // symbols that are available in every module (functions and statics)
    global_symbols: SymbolTable,
    // every entry represents the symbols in the current function (labels)
    function_symbols: Vec<SymbolTable>,
}

impl<'src> Parser<'src> {
    pub fn new(sources: Vec<SourceFile<'src>>) -> Self {
        Self {
            module_index: 0,
            instruction_counter: 0,
            sources,
            global_symbols: SymbolTable::default(),
            function_symbols: vec![SymbolTable::default()],
        }
    }

    // TODO: refactor those 3 functions
    fn function_symbols(&mut self) -> ParseResult<'src, &mut SymbolTable> {
        self.function_symbols
            .last_mut()
            .ok_or(ParseError::InvalidFunctionIndex)
    }

    fn lexer(&mut self) -> ParseResult<'src, &mut Lexer<'src>> {
        self.sources
            .get_mut(self.module_index)
            .ok_or(ParseError::InvalidFileIndex)
            .map(|f| &mut f.lexer)
    }

    fn filename(&self) -> ParseResult<'src, &str> {
        self.sources
            .get(self.module_index)
            .ok_or(ParseError::InvalidFileIndex)
            .map(|s| s.name.as_str())
    }

    fn next_token(&mut self) -> ParseResult<'src, Token<'src>> {
        let current_lexer = self.lexer()?;
        match current_lexer.scan_token() {
            Err(ParseError::EndOfFile) => {
                // try to continue with the next file
                self.module_index += 1;
                if self.module_index >= self.sources.len() {
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

    fn consume_segment(&mut self) -> ParseResult<'src, Segment> {
        if let Token::Identifier(ident) = self.next_token()? {
            let s = Segment::from_str(ident)?;
            Ok(s)
        } else {
            Err(ParseError::ExpectedSegment)
        }
    }

    fn consume_int(&mut self) -> ParseResult<'src, i16> {
        if let Token::IntLiteral(literal) = self.next_token()? {
            Ok(literal)
        } else {
            Err(ParseError::ExpectedInt)
        }
    }

    fn consume_segment_with_index(&mut self) -> ParseResult<'src, (Segment, i16)> {
        let segment = self.consume_segment()?;
        let mut index = self.consume_int()?;

        if segment == Segment::Static {
            let file_name = self.filename()?;
            let symbol = format!("{}.{}", file_name, index);

            index = self.global_symbols.lookup_or_insert(symbol) as i16;
        }

        Ok((segment, index))
    }

    fn consume_ident(&mut self) -> ParseResult<'src, &'src str> {
        if let Token::Identifier(ident) = self.next_token()? {
            Ok(ident)
        } else {
            Err(ParseError::ExpectedIdent)
        }
    }

    fn consume_symbol(&mut self) -> ParseResult<'src, Result<Symbol, &'src str>> {
        let ident = self.consume_ident()?;
        if let Some(symbol) = self.function_symbols()?.lookup(ident) {
            // the label for this symbol was already parsed in the current function
            Ok(Ok(symbol))
        } else if let Some(symbol) = self.global_symbols.lookup(ident) {
            // the label for this symbol was already parsed in some module
            Ok(Ok(symbol))
        } else {
            Ok(Err(ident))
        }
    }

    fn consume_label(&mut self, function_internal: bool) -> ParseResult<'src, &'src str> {
        let ident = self.consume_ident()?;
        let symbol = self.instruction_counter;
        // labels for ifs and such
        if function_internal {
            self.function_symbols()?.set(ident, symbol);
        } else {
            self.global_symbols.set(ident, symbol);
        }
        Ok(ident)
    }

    pub fn parse(&mut self) -> ParseResult<'src, ParsedProgram> {
        enum CodeEntry<'src> {
            FunctionStart { n_locals: i16 },
            Opcode(Opcode),
            WaitingForGlobalLabel(&'src str),
            WaitingForLocalLabel(&'src str),
        }

        let mut code: Vec<CodeEntry<'src>> = Vec::with_capacity(128);
        let mut debug_symbols = HashMap::new();

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

        fn push_function(inst_count: &mut u16, code: &mut Vec<CodeEntry>, n_locals: i16) {
            code.push(CodeEntry::FunctionStart { n_locals });
            *inst_count += 3;
        }

        fn wait_for_global<'src>(
            inst_count: &mut u16,
            code: &mut Vec<CodeEntry<'src>>,
            label: &'src str,
        ) {
            code.push(CodeEntry::WaitingForGlobalLabel(label));
            *inst_count += 2;
        }

        fn wait_for_local<'src>(
            inst_count: &mut u16,
            code: &mut Vec<CodeEntry<'src>>,
            label: &'src str,
        ) {
            code.push(CodeEntry::WaitingForLocalLabel(label));
            *inst_count += 2;
        }

        #[derive(PartialEq, Eq)]
        enum WaitKind {
            Local,
            Global,
        }

        use WaitKind::*;

        fn push_target<'src>(
            inst_count: &mut u16,
            code: &mut Vec<CodeEntry<'src>>,
            target: Result<Symbol, &'src str>,
            kind: WaitKind,
        ) {
            match target {
                Ok(addr) => push_u16(inst_count, code, addr),
                Err(waiting_for) => {
                    if kind == Local {
                        wait_for_local(inst_count, code, waiting_for)
                    } else {
                        wait_for_global(inst_count, code, waiting_for)
                    }
                }
            };
        }

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
                    push_target(&mut self.instruction_counter, &mut code, target, Local);
                }
                Token::Identifier("goto") => {
                    let target = self.consume_symbol()?;
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Goto);
                    push_target(&mut self.instruction_counter, &mut code, target, Local);
                }
                Token::Identifier("label") => {
                    let _label = self.consume_label(true)?;
                    // TODO: debug symbols for local labels
                }
                Token::Identifier("function") => {
                    let label = self.consume_label(false)?;
                    debug_symbols.insert(self.instruction_counter, label.to_owned());

                    let n_locals = self.consume_int()?;
                    self.function_symbols.push(SymbolTable::default());

                    push_function(&mut self.instruction_counter, &mut code, n_locals);
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
                    push_target(&mut self.instruction_counter, &mut code, target, Global);
                    push_i16(&mut self.instruction_counter, &mut code, n_args);
                }
                Token::Identifier("add") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Add)
                }
                Token::Identifier("sub") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Sub)
                }
                Token::Identifier("eq") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Eq)
                }
                Token::Identifier("gt") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Gt)
                }
                Token::Identifier("lt") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Lt)
                }
                Token::Identifier("and") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::And)
                }
                Token::Identifier("or") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Or)
                }
                Token::Identifier("not") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Not)
                }
                Token::Identifier("neg") => {
                    push_instr(&mut self.instruction_counter, &mut code, Instruction::Neg)
                }
                _ => return Err(ParseError::InvalidToken),
            };
        }

        let mut opcodes = Vec::with_capacity(code.capacity());
        let mut unresolved = HashSet::new();
        let mut function_index = 0;
        let mut function_offset = 0;

        for c in code {
            match c {
                CodeEntry::FunctionStart { n_locals } => {
                    function_index += 1;
                    function_offset = opcodes.len();

                    opcodes.push(Opcode::instruction(Instruction::Function));

                    let (first, second) = split_i16(n_locals);
                    opcodes.push(Opcode::constant(first));
                    opcodes.push(Opcode::constant(second));
                }
                CodeEntry::Opcode(opcode) => opcodes.push(opcode),
                CodeEntry::WaitingForLocalLabel(label) => {
                    if let Some(addr) = self.function_symbols[function_index].lookup(label) {
                        let (first, second) = split_u16(addr);
                        opcodes.push(Opcode::constant(first));
                        opcodes.push(Opcode::constant(second));
                    } else {
                        return Err(ParseError::UnresolvedLocalLabel {
                            label: label.to_string(),
                            function_name: debug_symbols
                                .get(&(function_offset as u16))
                                .unwrap_or(&"unknown".to_string())
                                .clone(),
                        });
                    }
                }
                CodeEntry::WaitingForGlobalLabel(label) => {
                    if let Some(addr) = self.global_symbols.lookup(label) {
                        let (first, second) = split_u16(addr);
                        opcodes.push(Opcode::constant(first));
                        opcodes.push(Opcode::constant(second));
                    } else {
                        unresolved.insert(label);
                    }
                }
            }
        }

        if unresolved.is_empty() {
            Ok(ParsedProgram::new(opcodes, debug_symbols))
        } else {
            Err(ParseError::UnresolvedSymbols(unresolved))
        }
    }
}

#[derive(Debug)]
pub struct ParsedProgram {
    pub opcodes: Vec<Opcode>,
    // a map from positions in the bytecode to their corresponding names in the bytecode
    // this is need to display infos in the UI (like the callstack) and for debugging the VM
    pub debug_symbols: HashMap<Symbol, String>,
}

impl ParsedProgram {
    pub fn new(opcodes: Vec<Opcode>, debug_symbols: HashMap<Symbol, String>) -> Self {
        Self {
            opcodes,
            debug_symbols,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parser_should_report_unresolved_labels() {
        let main = r#"
            function Main.main 0
            push constant 12
            call String.new 1
            push constant 72
            call String.appendChar 2
            push constant 101
            call String.appendChar 2
            push constant 108
            call String.appendChar 2
            push constant 108
            call String.appendChar 2
            push constant 111
            call String.appendChar 2
            push constant 32
            call String.appendChar 2
            push constant 119
            call String.appendChar 2
            push constant 111
            call String.appendChar 2
            push constant 114
            call String.appendChar 2
            push constant 108
            call String.appendChar 2
            push constant 100
            call String.appendChar 2
            push constant 33
            call String.appendChar 2
            call Output.printString 1
            pop temp 0
            call Output.println 0
            pop temp 0
            push constant 0
            return
            "#;

        let programs = vec![SourceFile::new("Main.vm", main)];
        let mut parser = Parser::new(programs);
        let result = parser.parse();

        if let Err(ParseError::UnresolvedSymbols(symbols)) = result {
            assert_eq!(
                HashSet::from_iter([
                    "String.new",
                    "String.appendChar",
                    "Output.printString",
                    "Output.println"
                ]),
                symbols
            );
        } else {
            assert_eq!(
                true,
                matches!(result, Err(ParseError::UnresolvedSymbols(_)))
            );
        }
    }

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
            code.opcodes,
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

        assert_eq!(parsed_bytecode.opcodes, expected_bytecode);
    }

    #[test]
    fn test_parse_basic_loop() {
        let bytecode = r#"
            // Computes the sum 1 + 2 + ... + argument[0] and pushes the
            // result onto the stack. Argument[0] is initialized by the test
            // script before this code starts running.
            push constant 0
            pop local 0         // initializes sum = 0
            label LOOP_START
            push argument 0
            push local 0
            add
            pop local 0         // sum = sum + counter
            push argument 0
            push constant 1
            sub
            pop argument 0      // counter--
            push argument 0
            if-goto LOOP_START  // If counter != 0, goto LOOP_START
            push local 0"#;

        let programs = vec![SourceFile::new("BasicLoop.vm", bytecode)];
        let mut parser = Parser::new(programs);
        let code = parser.parse().unwrap();

        assert_eq!(
            code.opcodes,
            vec![
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Local),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Argument),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Local),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Add),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Local),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Argument),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(1),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Sub),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Argument),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Argument),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::IfGoto),
                Opcode::constant(8),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Local),
                Opcode::constant(0),
                Opcode::constant(0),
            ]
        )
    }

    #[test]
    fn test_parse_fib_element() {
        let main = r#"
            // Computes the n'th element of the Fibonacci series, recursively.
            // n is given in argument[0].  Called by the Sys.init function
            // (part of the Sys.vm file), which also pushes the argument[0]
            // parameter before this code starts running.

            function Main.fibonacci 0
            push argument 0
            push constant 2
            lt                     // checks if n<2
            if-goto IF_TRUE
            goto IF_FALSE
            label IF_TRUE          // if n<2, return n
            push argument 0
            return
            label IF_FALSE         // if n>=2, returns fib(n-2)+fib(n-1)
            push argument 0
            push constant 2
            sub
            call Main.fibonacci 1  // computes fib(n-2)
            push argument 0
            push constant 1
            sub
            call Main.fibonacci 1  // computes fib(n-1)
            add                    // returns fib(n-1) + fib(n-2)
            return"#;

        let sys = r#"
            // Pushes a constant, say n, onto the stack, and calls the Main.fibonacii
            // function, which computes the n'th element of the Fibonacci series.
            // Note that by convention, the Sys.init function is called "automatically"
            // by the bootstrap code.

            function Sys.init 0
            push constant 4
            call Main.fibonacci 1   // computes the 4'th fibonacci element
            label WHILE
            goto WHILE              // loops infinitely"#;

        let programs = vec![
            SourceFile::new("Sys.vm", sys),
            SourceFile::new("Main.vm", main),
        ];
        let mut parser = Parser::new(programs);
        let code = parser.parse().unwrap();

        assert_eq!(
            code.opcodes,
            vec![
                Opcode::instruction(Instruction::Function),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(4),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Call),
                Opcode::constant(15),
                Opcode::constant(0),
                Opcode::constant(1),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Goto),
                Opcode::constant(12),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Function),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Argument),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(2),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Lt),
                Opcode::instruction(Instruction::IfGoto),
                Opcode::constant(33),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Goto),
                Opcode::constant(38),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Argument),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Return),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Argument),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(2),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Sub),
                Opcode::instruction(Instruction::Call),
                Opcode::constant(15),
                Opcode::constant(0),
                Opcode::constant(1),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Argument),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Constant),
                Opcode::constant(1),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Sub),
                Opcode::instruction(Instruction::Call),
                Opcode::constant(15),
                Opcode::constant(0),
                Opcode::constant(1),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Add),
                Opcode::instruction(Instruction::Return),
            ]
        )
    }
    #[test]
    fn test_statics_are_resolved_correctly_per_file() {
        let class1 = r#"
            push static 0
            pop static 0
            push static 1
            pop static 0
            pop static 1
            "#;

        let class2 = r#"
            push static 0
            pop static 0
            push static 1
            pop static 0
            pop static 1
            "#;

        let class3 = r#"
            push static 0
            pop static 0
            push static 1
            pop static 0
            pop static 1
            "#;

        let programs = vec![
            SourceFile::new("Class1.vm", class1),
            SourceFile::new("Class2.vm", class2),
            SourceFile::new("Class3.vm", class3),
        ];
        let mut parser = Parser::new(programs);
        let result = parser.parse().unwrap();

        assert_eq!(
            result.opcodes,
            vec![
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Static),
                Opcode::constant(16),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(16),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Static),
                Opcode::constant(17),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(16),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(17),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Static),
                Opcode::constant(18),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(18),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Static),
                Opcode::constant(19),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(18),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(19),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Static),
                Opcode::constant(20),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(20),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Push),
                Opcode::segment(Segment::Static),
                Opcode::constant(21),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(20),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Pop),
                Opcode::segment(Segment::Static),
                Opcode::constant(21),
                Opcode::constant(0),
            ]
        )
    }

    #[test]
    fn test_label_used_before_declaration() {
        let src = r#"
            function Main.main 0
            if-goto IF_TRUE0
            goto IF_FALSE0
            label IF_TRUE0
            goto IF_END0
            label IF_FALSE0
            label IF_END0
            return
            function Main.other 0
            if-goto IF_TRUE0
            goto IF_FALSE0
            label IF_TRUE0
            goto IF_END0
            label IF_FALSE0
            label IF_END0
            return
            "#;

        let programs = vec![SourceFile::new("Main.vm", src)];
        let mut parser = Parser::new(programs);
        let result = parser.parse().unwrap();

        assert_eq!(
            result.opcodes,
            vec![
                Opcode::instruction(Instruction::Function),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::IfGoto),
                Opcode::constant(9),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Goto),
                Opcode::constant(12),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Goto),
                Opcode::constant(12),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Return),
                Opcode::instruction(Instruction::Function),
                Opcode::constant(0),
                Opcode::constant(0),
                Opcode::instruction(Instruction::IfGoto),
                Opcode::constant(22),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Goto),
                Opcode::constant(25),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Goto),
                Opcode::constant(25),
                Opcode::constant(0),
                Opcode::instruction(Instruction::Return),
            ]
        )
    }
}

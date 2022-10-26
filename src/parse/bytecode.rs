use super::{Spanned, StringLexer};
use crate::simulators::vm::command::{ByteCodeParseError, Instruction, Segment};
use crate::simulators::vm::meta::{FunctionInfo, MetaInfo};
use crate::simulators::vm::stdlib::Stdlib;
use crate::simulators::vm::ProgramInfo;
use std::num::ParseIntError;

use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedCharacter(char),
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
    UnresolvedSymbols(HashSet<String>),
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

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(c) => write!(f, "Unexpected character: {}", c),
            Self::EndOfFile => write!(f, "EOF"),
            Self::UnexpectedEndOfFile => write!(f, "Unexpected end of file"),
            Self::InvalidIntLiteral(error) => write!(f, "Could not parse int: {}", error),
            Self::InvalidFileIndex => write!(f, "Invalid file index"),
            Self::InvalidFunctionIndex => write!(f, "Invalid function index"),
            Self::Bytecode(error) => write!(f, "{}", error),
            Self::ExpectedIdent => write!(f, "Expected identifier"),
            Self::ExpectedSegment => write!(f, "Expected segment"),
            Self::ExpectedInt => write!(f, "Expected integer"),
            Self::InvalidToken => write!(f, "Invalid token"),
            Self::UnresolvedLocalLabel {
                label,
                function_name,
            } => write!(f, "Could not resolve '{}' in '{}'", label, function_name),
            Self::UnresolvedSymbols(symbols) => write!(
                f,
                "Could not resolve the following symbols: {}",
                symbols
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        }
    }
}

impl error::Error for ParseError {}

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
                    Err(ParseError::UnexpectedCharacter(current_char))
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
            _ => Err(ParseError::UnexpectedCharacter(current_char)),
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
    fn lookup<'s>(&mut self, ident: impl Into<&'s str>) -> Option<Symbol> {
        self.symbols.get(ident.into()).copied()
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
    sources: Vec<SourceFile<'src>>,
    // symbols that are available in every module (functions and statics)
    global_symbols: SymbolTable,
    // every entry represents the symbols in the current function (labels)
    function_symbols: Vec<SymbolTable>,
    stdlib: Stdlib,
}

impl<'src> Parser<'src> {
    pub fn with_stdlib(sources: Vec<SourceFile<'src>>, stdlib: Stdlib) -> Self {
        Self {
            module_index: 0,
            sources,
            global_symbols: SymbolTable::default(),
            function_symbols: vec![SymbolTable::default()],
            stdlib,
        }
    }

    // TODO: refactor those 3 functions
    fn function_symbols(&mut self) -> ParseResult<&mut SymbolTable> {
        self.function_symbols
            .last_mut()
            .ok_or(ParseError::InvalidFunctionIndex)
    }

    fn lexer(&mut self) -> ParseResult<&mut Lexer<'src>> {
        self.sources
            .get_mut(self.module_index)
            .ok_or(ParseError::InvalidFileIndex)
            .map(|f| &mut f.lexer)
    }

    fn filename(&self) -> ParseResult<&str> {
        self.sources
            .get(self.module_index)
            .ok_or(ParseError::InvalidFileIndex)
            .map(|s| s.name.as_str())
    }

    fn next_token(&mut self) -> ParseResult<Token<'src>> {
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
            let file_name = self.filename()?;
            let symbol = format!("{}.{}", file_name, index);

            index = self.global_symbols.lookup_or_insert(symbol) as i16;
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

        if let Some(symbol) = self.function_symbols()?.lookup(ident) {
            // the label for this symbol was already parsed in the current function
            return Ok(Ok(symbol));
        }

        if let Some(symbol) = self.global_symbols.lookup(ident) {
            // the label for this symbol was already parsed in some module
            return Ok(Ok(symbol));
        }

        Ok(Err(ident))
    }

    fn consume_label(&mut self, symbol: Symbol, function_internal: bool) -> ParseResult<&'src str> {
        let ident = self.consume_ident()?;
        // labels for ifs and such
        if function_internal {
            self.function_symbols()?.set(ident, symbol);
        } else {
            self.global_symbols.set(ident, symbol);
        }
        Ok(ident)
    }

    pub fn parse(&mut self) -> ParseResult<ParsedProgram> {
        enum CodeEntry<'src> {
            Instruction(Instruction),
            WaitingForLabel(&'src str, Instruction),
        }

        let mut code: Vec<CodeEntry<'src>> = Vec::with_capacity(128);
        let mut debug_symbols = HashMap::new();

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
                    let full_instruction = match instr {
                        Instruction::Goto { .. } => Instruction::Goto { instruction: addr },
                        Instruction::IfGoto { .. } => Instruction::IfGoto { instruction: addr },
                        Instruction::Call { n_args, .. } => Instruction::Call {
                            function: addr,
                            n_args,
                        },
                        _ => unreachable!(),
                    };
                    push_instr(code, full_instruction);
                }
                Err(waiting_for) => code.push(CodeEntry::WaitingForLabel(waiting_for, instr)),
            };
        }

        let mut function_addresses = HashMap::new();
        let mut file_start = 0;
        // only add Sys.init if we also have a main function
        let mut had_main_function = false;

        loop {
            let last_module_index = self.module_index;
            let token = self.next_token();
            if let Err(ParseError::EndOfFile) = token {
                break;
            }

            if self.module_index != last_module_index {
                file_start = code.len();
            }

            match token? {
                Token::Identifier("push") => {
                    let (segment, index) = self.consume_segment_with_index()?;
                    push_instr(&mut code, Instruction::Push { segment, index });
                }
                Token::Identifier("pop") => {
                    let (segment, index) = self.consume_segment_with_index()?;
                    push_instr(&mut code, Instruction::Pop { segment, index });
                }
                Token::Identifier("if-goto") => {
                    let target = self.consume_symbol()?;
                    push_target(&mut code, target, Instruction::IfGoto { instruction: 0 });
                }
                Token::Identifier("goto") => {
                    let target = self.consume_symbol()?;
                    push_target(&mut code, target, Instruction::Goto { instruction: 0 });
                }
                Token::Identifier("label") => {
                    let symbol = code.len() as Symbol;
                    let _label = self.consume_label(symbol, true)?;
                    // TODO: debug symbols for local labels
                }
                Token::Identifier("function") => {
                    let symbol = code.len() as Symbol;
                    let label = self.consume_label(symbol, false)?;
                    let n_locals = self.consume_int()?;

                    debug_symbols.insert(
                        code.len() as u16,
                        FunctionInfo::vm(label.to_owned(), n_locals, self.module_index, file_start),
                    );
                    function_addresses.insert(label.to_owned(), symbol);
                    self.function_symbols.push(SymbolTable::default());

                    push_instr(&mut code, Instruction::Function { n_locals });

                    if label == "Main.main" {
                        had_main_function = true;
                    }
                }
                Token::Identifier("return") => {
                    push_instr(&mut code, Instruction::Return);
                }
                Token::Identifier("call") => {
                    let target = self.consume_symbol()?;
                    let n_args = self.consume_int()?;

                    // placeholder
                    let function = 0;
                    push_target(&mut code, target, Instruction::Call { function, n_args });
                }
                Token::Identifier("add") => push_instr(&mut code, Instruction::Add),
                Token::Identifier("sub") => push_instr(&mut code, Instruction::Sub),
                Token::Identifier("eq") => push_instr(&mut code, Instruction::Eq),
                Token::Identifier("gt") => push_instr(&mut code, Instruction::Gt),
                Token::Identifier("lt") => push_instr(&mut code, Instruction::Lt),
                Token::Identifier("and") => push_instr(&mut code, Instruction::And),
                Token::Identifier("or") => push_instr(&mut code, Instruction::Or),
                Token::Identifier("not") => push_instr(&mut code, Instruction::Not),
                Token::Identifier("neg") => push_instr(&mut code, Instruction::Neg),
                _ => return Err(ParseError::InvalidToken),
            };
        }

        // TODO: check program length cannot be larger than u16::MAX - NUMBER_OF_STDLIB_FUNCTIONS
        let mut instructions = Vec::with_capacity(code.capacity());
        let mut unresolved = HashSet::new();
        let mut function_index = 0;
        let mut function_offset = 0;

        for c in code {
            match c {
                CodeEntry::Instruction(Instruction::Function { n_locals }) => {
                    function_index += 1;
                    function_offset = instructions.len();
                    instructions.push(Instruction::Function { n_locals })
                }
                CodeEntry::Instruction(instr) => instructions.push(instr),
                CodeEntry::WaitingForLabel(label, Instruction::Call { n_args, .. }) => {
                    if let Some(function) = self.global_symbols.lookup(label) {
                        instructions.push(Instruction::Call { function, n_args })
                    } else if let Some(builtin_function) = self.stdlib.lookup(label) {
                        // we are calling a builtin function
                        let function = builtin_function.virtual_address();
                        instructions.push(Instruction::Call { function, n_args })
                    } else {
                        unresolved.insert(label);
                    }
                }
                CodeEntry::WaitingForLabel(label, instr) => {
                    if let Some(instruction) = self.function_symbols[function_index].lookup(label) {
                        if let Instruction::Goto { .. } = instr {
                            instructions.push(Instruction::Goto { instruction })
                        } else {
                            instructions.push(Instruction::IfGoto { instruction })
                        }
                    } else {
                        return Err(ParseError::UnresolvedLocalLabel {
                            label: label.to_string(),
                            function_name: debug_symbols
                                .get(&(function_offset as u16))
                                .map(|f| &f.name)
                                .unwrap_or(&"unknown".to_string())
                                .clone(),
                        });
                    }
                }
            }
        }

        // insert the missing stdlib functions
        // we need more than the function called by the program, because the stdlib functions
        // can call each other
        for (&name, &addr) in self.stdlib.by_name() {
            if self.global_symbols.lookup(name).is_none() {
                // we only want to add the Sys.init if the user supplied a Main.main
                // otherwise the Sys.init would throw an error when trying to call that main function
                if name == "Sys.init" && !had_main_function {
                    continue;
                }

                let func = self.stdlib.by_address(addr).unwrap();
                function_addresses.insert(name.to_owned(), addr);
                debug_symbols.insert(
                    addr,
                    FunctionInfo::builtin(func.name().to_string(), 0, func.file()),
                );
            }
        }

        if unresolved.is_empty() {
            Ok(ParsedProgram::new(
                instructions,
                debug_symbols,
                function_addresses,
            ))
        } else {
            Err(ParseError::UnresolvedSymbols(HashSet::from_iter(
                unresolved.iter().copied().map(str::to_owned),
            )))
        }
    }
}

#[derive(Debug)]
pub struct ParsedProgram {
    pub instructions: Vec<Instruction>,
    pub meta: MetaInfo,
}

impl ParsedProgram {
    pub fn new(
        instructions: Vec<Instruction>,
        function_meta: HashMap<Symbol, FunctionInfo>,
        function_by_name: HashMap<String, Symbol>,
    ) -> Self {
        Self {
            instructions,
            meta: MetaInfo::new(function_meta, function_by_name),
        }
    }
}

impl ProgramInfo for ParsedProgram {
    fn take_instructions(&mut self) -> Vec<Instruction> {
        std::mem::take(&mut self.instructions)
    }

    fn take_meta(&mut self) -> MetaInfo {
        std::mem::take(&mut self.meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulators::vm::stdlib::{BuiltinFunction, StdlibOk};

    impl<'src> Parser<'src> {
        pub fn new(sources: Vec<SourceFile<'src>>) -> Self {
            Self::with_stdlib(sources, Stdlib::default())
        }
    }

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
                    "String.new".to_owned(),
                    "String.appendChar".to_owned(),
                    "Output.printString".to_owned(),
                    "Output.println".to_owned()
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
            code.instructions,
            vec![
                Instruction::Function { n_locals: 1 },
                Instruction::Return,
                Instruction::Function { n_locals: 2 },
                Instruction::Return,
                Instruction::Function { n_locals: 1 },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 2
                },
                Instruction::Pop {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 3
                },
                Instruction::Add,
                Instruction::Pop {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 3
                },
                Instruction::Call {
                    function: 0,
                    n_args: 1
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 107
                },
                Instruction::Call {
                    function: 2,
                    n_args: 2
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 101
                },
                Instruction::Call {
                    function: 2,
                    n_args: 2
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 107
                },
                Instruction::Call {
                    function: 2,
                    n_args: 2
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 16
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 0
                },
                Instruction::Return
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
            Instruction::Push {
                segment: Segment::Constant,
                index: 0,
            },
            //
            Instruction::Pop {
                segment: Segment::Local,
                index: 0,
            },
            //
            Instruction::Push {
                segment: Segment::Local,
                index: 0,
            },
            //
            Instruction::Push {
                segment: Segment::Constant,
                index: 10,
            },
            //
            Instruction::Lt,
            //
            Instruction::Not,
            //
            Instruction::IfGoto { instruction: 12 },
            //
            Instruction::Push {
                segment: Segment::Local,
                index: 0,
            },
            //
            Instruction::Push {
                segment: Segment::Constant,
                index: 1,
            },
            //
            Instruction::Add,
            //
            Instruction::Pop {
                segment: Segment::Local,
                index: 0,
            },
            //
            Instruction::Goto { instruction: 2 },
            //
            Instruction::Goto { instruction: 12 },
        ];

        assert_eq!(parsed_bytecode.instructions, expected_bytecode);
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
            code.instructions,
            vec![
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 0
                },
                Instruction::Pop {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Argument,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Add,
                Instruction::Pop {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Argument,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 1
                },
                Instruction::Sub,
                Instruction::Pop {
                    segment: Segment::Argument,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Argument,
                    index: 0
                },
                Instruction::IfGoto { instruction: 2 },
                Instruction::Push {
                    segment: Segment::Local,
                    index: 0
                },
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
            code.instructions,
            vec![
                Instruction::Function { n_locals: 0 },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 4
                },
                Instruction::Call {
                    function: 4,
                    n_args: 1
                },
                Instruction::Goto { instruction: 3 },
                Instruction::Function { n_locals: 0 },
                Instruction::Push {
                    segment: Segment::Argument,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 2
                },
                Instruction::Lt,
                Instruction::IfGoto { instruction: 10 },
                Instruction::Goto { instruction: 12 },
                Instruction::Push {
                    segment: Segment::Argument,
                    index: 0
                },
                Instruction::Return,
                Instruction::Push {
                    segment: Segment::Argument,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 2
                },
                Instruction::Sub,
                Instruction::Call {
                    function: 4,
                    n_args: 1
                },
                Instruction::Push {
                    segment: Segment::Argument,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 1
                },
                Instruction::Sub,
                Instruction::Call {
                    function: 4,
                    n_args: 1
                },
                Instruction::Add,
                Instruction::Return,
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
            result.instructions,
            vec![
                Instruction::Push {
                    segment: Segment::Static,
                    index: 16
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 16
                },
                Instruction::Push {
                    segment: Segment::Static,
                    index: 17
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 16
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 17
                },
                Instruction::Push {
                    segment: Segment::Static,
                    index: 18
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 18
                },
                Instruction::Push {
                    segment: Segment::Static,
                    index: 19
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 18
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 19
                },
                Instruction::Push {
                    segment: Segment::Static,
                    index: 20
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 20
                },
                Instruction::Push {
                    segment: Segment::Static,
                    index: 21
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 20
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 21
                },
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
            result.instructions,
            vec![
                Instruction::Function { n_locals: 0 },
                Instruction::IfGoto { instruction: 3 },
                Instruction::Goto { instruction: 4 },
                Instruction::Goto { instruction: 4 },
                Instruction::Return,
                Instruction::Function { n_locals: 0 },
                Instruction::IfGoto { instruction: 8 },
                Instruction::Goto { instruction: 9 },
                Instruction::Goto { instruction: 9 },
                Instruction::Return,
            ]
        )
    }

    #[test]
    fn test_stdlib_functions_resolve_to_correct_virtual_address() {
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

        let stdlib = Stdlib::new();
        let stdlib_address_space = stdlib.len() as u16..=u16::MAX;
        let new_address = stdlib.lookup("String.new").unwrap().virtual_address();
        let append_address = stdlib
            .lookup("String.appendChar")
            .unwrap()
            .virtual_address();

        assert_eq!(true, stdlib_address_space.contains(&new_address));
        assert_eq!(true, stdlib_address_space.contains(&append_address));

        let programs = vec![SourceFile::new("Simple.vm", source)];
        let mut parser = Parser::with_stdlib(programs, stdlib);
        let code = parser.parse().unwrap();

        assert_eq!(
            code.instructions,
            vec![
                Instruction::Function { n_locals: 1 },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 2
                },
                Instruction::Pop {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 3
                },
                Instruction::Add,
                Instruction::Pop {
                    segment: Segment::Local,
                    index: 0
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 3
                },
                Instruction::Call {
                    function: new_address,
                    n_args: 1
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 107
                },
                Instruction::Call {
                    function: append_address,
                    n_args: 2
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 101
                },
                Instruction::Call {
                    function: append_address,
                    n_args: 2
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 107
                },
                Instruction::Call {
                    function: append_address,
                    n_args: 2
                },
                Instruction::Pop {
                    segment: Segment::Static,
                    index: 16
                },
                Instruction::Push {
                    segment: Segment::Constant,
                    index: 0
                },
                Instruction::Return
            ]
        )
    }

    #[test]
    fn test_stdlib_functions_edge_cases() {
        // the first and last stdlib functions
        let source = "
            call Math.init 0
            call Sys.wait 1
            ";

        let stdlib = Stdlib::new();
        let init_address = stdlib.lookup("Math.init").unwrap().virtual_address();
        let wait_address = stdlib.lookup("Sys.wait").unwrap().virtual_address();

        assert_eq!(49, stdlib.len());
        assert_eq!(u16::MAX - (stdlib.len() as u16 - 1), init_address);
        assert_eq!(u16::MAX, wait_address);

        let programs = vec![SourceFile::new("Simple.vm", source)];
        let mut parser = Parser::with_stdlib(programs, stdlib);
        let code = parser.parse().unwrap();

        assert_eq!(
            code.instructions,
            vec![
                Instruction::Call {
                    function: init_address,
                    n_args: 0
                },
                Instruction::Call {
                    function: wait_address,
                    n_args: 1
                },
            ]
        )
    }

    #[test]
    fn test_sys_init_is_resolve_to_stdlib_if_not_in_bytecode() {
        // the first and last stdlib functions
        let source = "
            function Main.main 0
            call Sys.init 0
            ";

        let mut by_name = HashMap::new();
        let mut by_address = HashMap::new();

        by_name.insert("Sys.init", u16::MAX);
        by_address.insert(
            u16::MAX,
            BuiltinFunction::new(u16::MAX, "Sys.init", "Sys", 0, &|_, _, _| {
                Ok(StdlibOk::Finished(0))
            }),
        );

        let stdlib = Stdlib::of(by_name, by_address);
        let init_address = stdlib.lookup("Sys.init").unwrap().virtual_address();

        assert_eq!(1, stdlib.len());
        assert_eq!(u16::MAX, init_address);

        let programs = vec![SourceFile::new("Simple.vm", source)];
        let mut parser = Parser::with_stdlib(programs, stdlib);
        let code = parser.parse().unwrap();

        assert_eq!(
            code.instructions,
            vec![
                Instruction::Function { n_locals: 0 },
                Instruction::Call {
                    function: init_address,
                    n_args: 0
                },
            ]
        );
    }

    #[test]
    fn test_sys_init_can_be_overwritten_by_bytecode_before() {
        // the first and last stdlib functions
        let source = "
            function Sys.init 0
            return

            function Main.main 0
            call Sys.init 0
            ";

        let mut by_name = HashMap::new();
        let mut by_address = HashMap::new();

        by_name.insert("Sys.init", u16::MAX);
        by_address.insert(
            u16::MAX,
            BuiltinFunction::new(u16::MAX, "Sys.init", "Sys", 0, &|_, _, _| {
                Ok(StdlibOk::Finished(0))
            }),
        );

        let stdlib = Stdlib::of(by_name, by_address);
        let init_address = stdlib.lookup("Sys.init").unwrap().virtual_address();

        assert_eq!(1, stdlib.len());
        assert_eq!(u16::MAX, init_address);

        let programs = vec![SourceFile::new("Simple.vm", source)];
        let mut parser = Parser::with_stdlib(programs, stdlib);
        let code = parser.parse().unwrap();

        assert_eq!(
            code.instructions,
            vec![
                Instruction::Function { n_locals: 0 },
                Instruction::Return,
                Instruction::Function { n_locals: 0 },
                Instruction::Call {
                    function: 0,
                    n_args: 0
                }
            ]
        );
    }

    #[test]
    fn test_sys_init_can_be_overwritten_by_bytecode_after() {
        // the first and last stdlib functions
        let source = "
            function Main.main 0
            call Sys.init 0

            function Sys.init 0
            return
            ";

        let mut by_name = HashMap::new();
        let mut by_address = HashMap::new();

        by_name.insert("Sys.init", u16::MAX);
        by_address.insert(
            u16::MAX,
            BuiltinFunction::new(u16::MAX, "Sys.init", "Sys", 0, &|_, _, _| {
                Ok(StdlibOk::Finished(0))
            }),
        );

        let stdlib = Stdlib::of(by_name, by_address);
        let init_address = stdlib.lookup("Sys.init").unwrap().virtual_address();

        assert_eq!(1, stdlib.len());
        assert_eq!(u16::MAX, init_address);

        let programs = vec![SourceFile::new("Simple.vm", source)];
        let mut parser = Parser::with_stdlib(programs, stdlib);
        let code = parser.parse().unwrap();

        assert_eq!(
            code.instructions,
            vec![
                Instruction::Function { n_locals: 0 },
                Instruction::Call {
                    function: 2,
                    n_args: 0
                },
                Instruction::Function { n_locals: 0 },
                Instruction::Return,
            ]
        );
    }
}

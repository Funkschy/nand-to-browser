pub mod command;

use crate::definitions::SCREEN_START;
use crate::definitions::{Address, Symbol, Word, ARG, INIT_SP, LCL, MEM_SIZE, SP, THAT, THIS};
use command::{Instruction, Opcode, Segment};
use std::collections::HashMap;

pub struct VM {
    // the program counter / instruction pointer
    pc: usize,
    program: Vec<Opcode>,

    // debug information which is used in the UI and for internal debugging
    debug_symbols: HashMap<u16, String>,
    // a stack of source code positions, which can be mapped to strings on demand by mapping them
    // over the debug_symbols map
    call_stack: Vec<u16>,

    // 0-15        virtual registers
    // 16-255      static variables
    // 256-2047    stack
    // 2048-16483  heap
    // 16384-24575 memory mapped io
    memory: Box<[Word; MEM_SIZE]>,
}

macro_rules! tos_binary {
    ($vm:expr, $op:tt) => {{
        if cfg!(feature = "trace_vm") {
            println!("{}", stringify!($op));
        }

        let sp = $vm.memory[SP] as Address;
        // cast up to i32 so that no overflow checks get triggered in debug mode
        $vm.memory[sp - 2] = ($vm.memory[sp - 2] as i32 $op $vm.memory[sp - 1] as i32) as Word;
        $vm.memory[SP] -= 1;
        $vm.pc += 1;
    }};
}

macro_rules! tos_binary_bool {
    ($vm:expr, $op:tt) => {{
        if cfg!(feature = "trace_vm") {
            println!("{}", stringify!($op));
        }

        let sp = $vm.memory[SP] as Address;
        // in the hack architecture, true is actually -1 not 1 so we have to invert the tos
        // if it was already 0 (false) it will stay zero, if it was 1 it will be -1
        $vm.memory[sp - 2] = -(($vm.memory[sp - 2] $op $vm.memory[sp - 1]) as Word);
        $vm.memory[SP] -= 1;
        $vm.pc += 1;
    }};
}

macro_rules! tos_unary {
    ($vm:expr, $op:tt) => {{
        if cfg!(feature = "trace_vm") {
            println!("{}", stringify!($op));
        }

        let sp = $vm.memory[SP] as Address;
        $vm.memory[sp - 1] = $op($vm.memory[sp - 1] as Word);
        $vm.pc += 1;
    }};
}

impl VM {
    #[inline]
    pub fn mem(&mut self, address: Address) -> &mut Word {
        &mut self.memory[address]
    }

    #[inline]
    fn mem_indirect(&mut self, address_of_address: Address, offset: usize) -> &mut Word {
        &mut self.memory[self.memory[address_of_address] as Address + offset]
    }

    #[inline]
    fn pop(&mut self) -> Word {
        *self.mem(SP) -= 1;
        *self.mem_indirect(SP, 0)
    }

    #[inline]
    fn push(&mut self, value: Word) {
        *self.mem_indirect(SP, 0) = value;
        *self.mem(SP) += 1;
    }

    #[inline]
    fn tos(&self) -> Word {
        let sp = self.memory[SP] as Address;
        self.memory[sp - 1]
    }

    fn get_seg_address(&self, segment: Segment, index: i16) -> Address {
        let offset = match segment {
            Segment::Local => self.memory[LCL],
            Segment::Argument => self.memory[ARG],
            Segment::This => self.memory[THIS],
            Segment::That => self.memory[THAT],
            Segment::Temp => 5,
            Segment::Pointer => 3,
            // Static memory segments are actually resolved in the ByteCode Parser
            // The parser will simply set the index to an offset unique for the source file
            // it is currently parsing.
            Segment::Static => 0,
            Segment::Constant => panic!("cannot get address of constant"),
        };
        offset as Address + index as Address
    }

    fn get_value(&self, segment: Segment, index: i16) -> Word {
        if segment == Segment::Constant {
            index
        } else {
            let addr = self.get_seg_address(segment, index);
            self.memory[addr]
        }
    }

    pub fn display(&self) -> &[Word] {
        &self.memory[SCREEN_START..(SCREEN_START + 8192)]
    }

    pub fn load(&mut self, program: Vec<Opcode>, debug_symbols: HashMap<u16, String>) {
        self.program = program;
        self.debug_symbols = debug_symbols;
        self.pc = 0;
        for i in 0..self.memory.len() {
            self.memory[i] = 0;
        }
        // page 162 of the book:
        // the VM implementation can start by generating assembly code that sets SP=256
        *self.mem(SP) = INIT_SP;
    }

    fn consume_segment(&mut self) -> Segment {
        let value = self.program[self.pc + 1];
        self.pc += 1;
        value
            .try_into()
            .expect("argument does not fit into a segment")
    }

    fn consume_short(&mut self) -> i16 {
        // this assumes that the target uses little endian byte ordering, but so does the wasm
        // standard, therefore this should be absolutely fine
        //
        // "WebAssembly portability assumes that execution environments offer the
        // following characteristics: [...] Little-endian byte ordering"
        // See:
        // https://webassembly.org/docs/portability/
        let left_byte: u8 = self.program[self.pc + 1].try_into().unwrap();
        let right_byte: u8 = self.program[self.pc + 2].try_into().unwrap();
        self.pc += 2;
        i16::from_le_bytes([left_byte, right_byte])
    }

    fn push_call(&mut self, function: Symbol) {
        self.call_stack.push(function);
    }

    fn pop_call(&mut self) -> Option<u16> {
        self.call_stack.pop()
    }

    pub fn step(&mut self) {
        use Instruction::{
            Add, And, Call, Eq, Function, Goto, Gt, IfGoto, Lt, Neg, Not, Or, Pop, Push, Return,
            Sub,
        };

        let opcode = self.program[self.pc];
        let instr = opcode.try_into().unwrap();

        match instr {
            Add => tos_binary!(self, +),
            Sub => tos_binary!(self, -),
            Not => tos_unary!(self, !),
            Neg => tos_unary!(self, -),
            And => tos_binary!(self, &),
            Or => tos_binary!(self, |),
            Eq => tos_binary_bool!(self, ==),
            Gt => tos_binary_bool!(self, >),
            Lt => tos_binary_bool!(self, <),
            Push => {
                let segment = self.consume_segment();
                let index = self.consume_short();
                let value = self.get_value(segment, index);

                if cfg!(feature = "trace_vm") {
                    println!("push {:?} {} {}", segment, index, value);
                }

                self.push(value);
                self.pc += 1;
            }
            Pop => {
                let segment = self.consume_segment();
                let index = self.consume_short();
                let address = self.get_seg_address(segment, index);
                let value = self.pop();

                if cfg!(feature = "trace_vm") {
                    println!("pop {:?} {} {} {}", segment, index, address, value);
                }

                *self.mem(address) = value;
                self.pc += 1;
            }
            Goto => {
                let instr = self.consume_short();
                self.pc = instr as usize;
            }
            IfGoto => {
                let instr = self.consume_short();
                let condition = self.pop();
                if condition == 0 {
                    self.pc += 1;
                } else {
                    self.pc = instr as usize;
                }
            }
            Function => {
                if cfg!(any(feature = "trace_vm", feature = "trace_calls")) {
                    println!("function {}", self.debug_symbols[&(self.pc as u16)]);
                    println!("SP   {}", *self.mem(SP));
                    println!("LCL  {}", *self.mem(LCL));
                    println!("ARG  {}", *self.mem(ARG));
                    println!("THIS {}", *self.mem(THIS));
                    println!("THAT {}", *self.mem(THAT));
                }

                self.push_call(self.pc as Symbol);

                let n_locals = self.consume_short();
                for _ in 0..n_locals {
                    self.push(0);
                }
                self.pc += 1;
            }
            Return => {
                if cfg!(any(feature = "trace_vm", feature = "trace_calls")) {
                    println!("return");
                }

                let frame = *self.mem(LCL) as Address;
                // the return address
                let ret = *self.mem(frame - 5) as Address;
                // reposition the return value for the caller
                *self.mem_indirect(ARG, 0) = self.pop();
                // restore the stack for the caller
                *self.mem(SP) = *self.mem(ARG) + 1;
                *self.mem(THAT) = *self.mem(frame - 1);
                *self.mem(THIS) = *self.mem(frame - 2);
                *self.mem(ARG) = *self.mem(frame - 3);
                *self.mem(LCL) = *self.mem(frame - 4);

                self.pc = ret;
                let popped = self.pop_call();
                if cfg!(any(feature = "trace_vm", feature = "trace_calls")) {
                    if let Some(ret_from) = popped {
                        print!("returning from {}", self.debug_symbols[&(ret_from as u16)]);
                        if let Some(&ret_to) = self.call_stack.last() {
                            println!(" to {}", self.debug_symbols[&(ret_to as u16)]);
                        } else {
                            println!(" to nowhere");
                        }
                        println!("at address {}", ret);
                    } else {
                        println!("returning from top level");
                    }
                }
            }
            Call => {
                let function = self.consume_short();
                if cfg!(any(feature = "trace_vm", feature = "trace_calls")) {
                    println!("call {}", self.debug_symbols[&(function as u16)]);
                }

                let n_args = self.consume_short();

                dbg!(function);
                dbg!(n_args);

                let ret_addr = self.pc + 1;
                self.push(ret_addr as i16);

                let lcl = *self.mem(LCL);
                self.push(lcl);
                let arg = *self.mem(ARG);
                self.push(arg);
                let this = *self.mem(THIS);
                self.push(this);
                let that = *self.mem(THAT);
                self.push(that);

                let sp = *self.mem(SP);
                *self.mem(ARG) = sp - n_args - 5;
                *self.mem(LCL) = sp;

                self.pc = function as usize;
            }
        };

        if cfg!(feature = "trace_vm") {
            dbg!(self.pc);
            dbg!(self.memory[SP]);
            dbg!(self.memory[LCL]);
            dbg!(self.memory[ARG]);
            dbg!(self.memory[THIS]);
            dbg!(self.memory[THAT]);
            dbg!(self.tos());
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        let mut vm = Self {
            pc: 0,
            program: vec![],
            debug_symbols: HashMap::new(),
            call_stack: Vec::with_capacity(32),
            memory: Box::new([0; MEM_SIZE]),
        };

        // page 162 of the book:
        // the VM implementation can start by generating assembly code that sets SP=256
        vm.memory[0] = INIT_SP;

        vm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::KBD;
    use crate::definitions::SCREEN_START;
    use crate::parse::bytecode::*;

    #[test]
    fn basic_test_vme_no_parse() {
        let mut vm = VM::default();

        let bytecode = vec![
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(10),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::Local),
            Opcode::constant(0),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(21),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(22),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::Argument),
            Opcode::constant(2),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::Argument),
            Opcode::constant(1),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(36),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::This),
            Opcode::constant(6),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(42),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(45),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::That),
            Opcode::constant(5),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::That),
            Opcode::constant(2),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Constant),
            Opcode::constant(254),
            Opcode::constant(1),
            Opcode::instruction(Instruction::Pop),
            Opcode::segment(Segment::Temp),
            Opcode::constant(6),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Local),
            Opcode::constant(0),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::That),
            Opcode::constant(5),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Add),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Argument),
            Opcode::constant(1),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Sub),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::This),
            Opcode::constant(6),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::This),
            Opcode::constant(6),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Add),
            Opcode::instruction(Instruction::Sub),
            Opcode::instruction(Instruction::Push),
            Opcode::segment(Segment::Temp),
            Opcode::constant(6),
            Opcode::constant(0),
            Opcode::instruction(Instruction::Add),
        ];

        vm.load(bytecode, HashMap::new());

        *vm.mem(SP) = 256;
        *vm.mem(LCL) = 300;
        *vm.mem(ARG) = 400;
        *vm.mem(THIS) = 3000;
        *vm.mem(THAT) = 3010;

        for _ in 0..25 {
            vm.step();
        }

        assert_eq!(472, *vm.mem(256));
        assert_eq!(10, *vm.mem(300));
        assert_eq!(21, *vm.mem(401));
        assert_eq!(22, *vm.mem(402));
        assert_eq!(36, *vm.mem(3006));
        assert_eq!(42, *vm.mem(3012));
        assert_eq!(45, *vm.mem(3015));
        assert_eq!(510, *vm.mem(11));
    }

    #[test]
    fn basic_test_vme() {
        let mut vm = VM::default();

        let bytecode = r#"
            push constant 10
            pop local 0
            push constant 21
            push constant 22
            pop argument 2
            pop argument 1
            push constant 36
            pop this 6
            push constant 42
            push constant 45
            pop that 5
            pop that 2
            push constant 510
            pop temp 6
            push local 0
            push that 5
            add
            push argument 1
            sub
            push this 6
            push this 6
            add
            sub
            push temp 6
            add"#;

        let programs = vec![SourceFile::new("BasicTest.vm", bytecode)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(SP) = 256;
        *vm.mem(LCL) = 300;
        *vm.mem(ARG) = 400;
        *vm.mem(THIS) = 3000;
        *vm.mem(THAT) = 3010;

        for _ in 0..25 {
            vm.step();
        }

        assert_eq!(472, *vm.mem(256));
        assert_eq!(10, *vm.mem(300));
        assert_eq!(21, *vm.mem(401));
        assert_eq!(22, *vm.mem(402));
        assert_eq!(36, *vm.mem(3006));
        assert_eq!(42, *vm.mem(3012));
        assert_eq!(45, *vm.mem(3015));
        assert_eq!(510, *vm.mem(11));
    }

    #[test]
    fn pointer_test_vme() {
        let mut vm = VM::default();

        let bytecode = r#"
            push constant 3030
            pop pointer 0
            push constant 3040
            pop pointer 1
            push constant 32
            pop this 2
            push constant 46
            pop that 6
            push pointer 0
            push pointer 1
            add
            push this 2
            sub
            push that 6
            add"#;

        let programs = vec![SourceFile::new("PointerTest.vm", bytecode)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(0) = 256;

        for _ in 0..15 {
            vm.step();
        }

        assert_eq!(6084, *vm.mem(256));
        assert_eq!(3030, *vm.mem(3));
        assert_eq!(3040, *vm.mem(4));
        assert_eq!(32, *vm.mem(3032));
        assert_eq!(46, *vm.mem(3046));
    }

    #[test]
    fn static_test_vme() {
        let mut vm = VM::default();

        let bytecode = r#"
            push constant 111
            push constant 333
            push constant 888
            pop static 8
            pop static 3
            pop static 1
            push static 3
            push static 1
            sub
            push static 8
            add"#;

        let programs = vec![SourceFile::new("StaticTest.vm", bytecode)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(0) = 256;

        for _ in 0..11 {
            vm.step();
        }

        assert_eq!(1110, *vm.mem(256));
    }

    #[test]
    fn simple_add() {
        let mut vm = VM::default();

        let bytecode = r#"
            push constant 7
            push constant 8
            add"#;

        let programs = vec![SourceFile::new("SimpleAdd.vm", bytecode)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(0) = 256;

        for _ in 0..3 {
            vm.step();
        }

        assert_eq!(257, *vm.mem(0));
        assert_eq!(15, *vm.mem(256));
    }

    #[test]
    fn stack_test() {
        let mut vm = VM::default();

        let bytecode = r#"
            push constant 17
            push constant 17
            eq
            push constant 17
            push constant 16
            eq
            push constant 16
            push constant 17
            eq
            push constant 892
            push constant 891
            lt
            push constant 891
            push constant 892
            lt
            push constant 891
            push constant 891
            lt
            push constant 32767
            push constant 32766
            gt
            push constant 32766
            push constant 32767
            gt
            push constant 32766
            push constant 32766
            gt
            push constant 57
            push constant 31
            push constant 53
            add
            push constant 112
            sub
            neg
            and
            push constant 82
            or
            not"#;

        let programs = vec![SourceFile::new("StackTest.vm", bytecode)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(0) = 256;

        for _ in 0..38 {
            vm.step();
        }

        assert_eq!(266, *vm.mem(0));
        assert_eq!(-1, *vm.mem(256));
        assert_eq!(0, *vm.mem(257));
        assert_eq!(0, *vm.mem(258));
        assert_eq!(0, *vm.mem(259));
        assert_eq!(-1, *vm.mem(260));
        assert_eq!(0, *vm.mem(261));
        assert_eq!(-1, *vm.mem(262));
        assert_eq!(0, *vm.mem(263));
        assert_eq!(0, *vm.mem(264));
        assert_eq!(-91, *vm.mem(265));
    }

    #[test]
    fn basic_loop() {
        let mut vm = VM::default();

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
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(SP) = 256;
        *vm.mem(LCL) = 300;
        *vm.mem(ARG) = 400;
        *vm.mem_indirect(ARG, 0) = 3;

        for _ in 0..33 {
            vm.step();
        }

        assert_eq!(257, *vm.mem(0));
        assert_eq!(6, *vm.mem(256));
    }

    #[test]
    fn fibonacci_series() {
        let mut vm = VM::default();

        let bytecode = r#"
            // Puts the first argument[0] elements of the Fibonacci series
            // in the memory, starting in the address given in argument[1].
            // Argument[0] and argument[1] are initialized by the test script
            // before this code starts running.

            push argument 1
            pop pointer 1           // that = argument[1]

            push constant 0
            pop that 0              // first element in the series = 0
            push constant 1
            pop that 1              // second element in the series = 1

            push argument 0
            push constant 2
            sub
            pop argument 0          // num_of_elements -= 2 (first 2 elements are set)

            label MAIN_LOOP_START

            push argument 0
            if-goto COMPUTE_ELEMENT // if num_of_elements > 0, goto COMPUTE_ELEMENT
            goto END_PROGRAM        // otherwise, goto END_PROGRAM

            label COMPUTE_ELEMENT

            push that 0
            push that 1
            add
            pop that 2              // that[2] = that[0] + that[1]

            push pointer 1
            push constant 1
            add
            pop pointer 1           // that += 1

            push argument 0
            push constant 1
            sub
            pop argument 0          // num_of_elements--

            goto MAIN_LOOP_START

            label END_PROGRAM"#;

        let programs = vec![SourceFile::new("FibonacciSeries.vm", bytecode)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(SP) = 256;
        *vm.mem(LCL) = 300;
        *vm.mem(ARG) = 400;
        *vm.mem_indirect(ARG, 0) = 6;
        *vm.mem_indirect(ARG, 1) = 3000;

        for _ in 0..73 {
            vm.step();
        }

        assert_eq!(0, *vm.mem(3000));
        assert_eq!(1, *vm.mem(3001));
        assert_eq!(1, *vm.mem(3002));
        assert_eq!(2, *vm.mem(3003));
        assert_eq!(3, *vm.mem(3004));
        assert_eq!(5, *vm.mem(3005));
    }

    #[test]
    fn fibonacci_element() {
        let mut vm = VM::default();

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
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(SP) = 261;

        for _ in 0..110 {
            vm.step();
        }

        assert_eq!(262, *vm.mem(0));
        assert_eq!(3, *vm.mem(261));
    }

    #[test]
    fn nested_call() {
        let mut vm = VM::default();

        let sys = r#"
            // Sys.vm for NestedCall test.

            // Sys.init()
            //
            // Calls Sys.main() and stores return value in temp 1.
            // Does not return.  (Enters infinite loop.)

            function Sys.init 0
            push constant 4000	// test THIS and THAT context save
            pop pointer 0
            push constant 5000
            pop pointer 1
            call Sys.main 0
            pop temp 1
            label LOOP
            goto LOOP

            // Sys.main()
            //
            // Sets locals 1, 2 and 3, leaving locals 0 and 4 unchanged to test
            // default local initialization to 0.  (RAM set to -1 by test setup.)
            // Calls Sys.add12(123) and stores return value (135) in temp 0.
            // Returns local 0 + local 1 + local 2 + local 3 + local 4 (456) to confirm
            // that locals were not mangled by function call.

            function Sys.main 5
            push constant 4001
            pop pointer 0
            push constant 5001
            pop pointer 1
            push constant 200
            pop local 1
            push constant 40
            pop local 2
            push constant 6
            pop local 3
            push constant 123
            call Sys.add12 1
            pop temp 0
            push local 0
            push local 1
            push local 2
            push local 3
            push local 4
            add
            add
            add
            add
            return

            // Sys.add12(int n)
            //
            // Returns n+12.

            function Sys.add12 0
            push constant 4002
            pop pointer 0
            push constant 5002
            pop pointer 1
            push argument 0
            push constant 12
            add
            return"#;

        let programs = vec![SourceFile::new("Sys.vm", sys)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(0) = 261;
        *vm.mem(1) = 261;
        *vm.mem(2) = 256;
        *vm.mem(3) = -3;
        *vm.mem(4) = -4;
        *vm.mem(5) = -1; // test results
        *vm.mem(6) = -1;
        *vm.mem(256) = 1234; // fake stack frame from call Sys.init
        *vm.mem(257) = -1;
        *vm.mem(258) = -2;
        *vm.mem(259) = -3;
        *vm.mem(260) = -4;

        for i in 261..=299 {
            *vm.mem(i) = -1;
        }

        *vm.mem(SP) = 261;
        *vm.mem(LCL) = 261;
        *vm.mem(ARG) = 256;
        *vm.mem(THIS) = 3000;
        *vm.mem(THAT) = 4000;

        for _ in 0..50 {
            vm.step();
        }

        assert_eq!(261, *vm.mem(0));
        assert_eq!(261, *vm.mem(1));
        assert_eq!(256, *vm.mem(2));
        assert_eq!(4000, *vm.mem(3));
        assert_eq!(5000, *vm.mem(4));
        assert_eq!(135, *vm.mem(5));
        assert_eq!(246, *vm.mem(6));
    }

    #[test]
    fn simple_function() {
        let mut vm = VM::default();

        let sys = r#"
            function SimpleFunction.test 2
            push local 0
            push local 1
            add
            not
            push argument 0
            add
            push argument 1
            sub
            return"#;

        let programs = vec![SourceFile::new("Sys.vm", sys)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(SP) = 317;
        *vm.mem(LCL) = 317;
        *vm.mem(ARG) = 310;
        *vm.mem(THIS) = 3000;
        *vm.mem(THAT) = 4000;
        *vm.mem_indirect(ARG, 0) = 1234;
        *vm.mem_indirect(ARG, 1) = 37;
        *vm.mem_indirect(ARG, 2) = 9;
        *vm.mem_indirect(ARG, 3) = 305;
        *vm.mem_indirect(ARG, 4) = 300;
        *vm.mem_indirect(ARG, 5) = 3010;
        *vm.mem_indirect(ARG, 6) = 4010;

        for _ in 0..10 {
            vm.step();
        }

        assert_eq!(311, *vm.mem(0));
        assert_eq!(305, *vm.mem(1));
        assert_eq!(300, *vm.mem(2));
        assert_eq!(3010, *vm.mem(3));
        assert_eq!(4010, *vm.mem(4));
        assert_eq!(1196, *vm.mem(310));
    }

    #[test]
    fn statics_test() {
        let mut vm = VM::default();

        let sys = r#"
            // Tests that different functions, stored in two different
            // class files, manipulate the static segment correctly.
            function Sys.init 0
            push constant 6
            push constant 8
            call Class1.set 2
            pop temp 0 // Dumps the return value
            push constant 23
            push constant 15
            call Class2.set 2
            pop temp 0 // Dumps the return value
            call Class1.get 0
            call Class2.get 0
            label WHILE
            goto WHILE
            "#;

        let class1 = r#"
            // Stores two supplied arguments in static[0] and static[1].
            function Class1.set 0
            push argument 0
            pop static 0
            push argument 1
            pop static 1
            push constant 0
            return

            // Returns static[0] - static[1].
            function Class1.get 0
            push static 0
            push static 1
            sub
            return
            "#;

        let class2 = r#"
            // Stores two supplied arguments in static[0] and static[1].
            function Class2.set 0
            push argument 0
            pop static 0
            push argument 1
            pop static 1
            push constant 0
            return

            // Returns static[0] - static[1].
            function Class2.get 0
            push static 0
            push static 1
            sub
            return
            "#;

        let programs = vec![
            SourceFile::new("Sys.vm", sys),
            SourceFile::new("Class1.vm", class1),
            SourceFile::new("Class2.vm", class2),
        ];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        *vm.mem(SP) = 261;

        for _ in 0..36 {
            vm.step();
        }

        assert_eq!(263, *vm.mem(0));
        assert_eq!(-2, *vm.mem(261));
        assert_eq!(8, *vm.mem(262));
    }

    #[test]
    fn display_thick_lines() {
        let mut vm = VM::default();

        let src = r#"
            function Lines.init 0
            call Lines.main 3
            label END
            goto END

            function Lines.main 3
            push constant 16384
            pop local 2
            push constant 8192
            pop local 0
            push constant 0
            pop local 1
            label WHILE_EXP0
            push local 1
            push local 0
            lt
            not
            if-goto WHILE_END0
            push local 1
            push local 2
            add
            push constant 255
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 1
            push constant 1
            add
            pop local 1
            goto WHILE_EXP0
            label WHILE_END0
            push constant 0
            return
            "#;

        let programs = vec![SourceFile::new("Lines.vm", src)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program.opcodes, program.debug_symbols);

        for _ in 0..500000 {
            vm.step();
        }

        for &word in vm.display() {
            assert_eq!(255, word);
        }
    }
}

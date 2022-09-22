pub mod command;

use crate::definitions::{Address, Word, ARG, LCL, MEM_SIZE, SP, THAT, THIS};
use command::{Instruction, Opcode, Segment};

pub struct VM {
    // the program counter / instruction pointer
    pc: usize,
    program: Vec<Opcode>,
    // 0-15        virtual registers
    // 16-255      static variables
    // 256-2047    stack
    // 2048-16483  heap
    // 16384-24575 memory mapped io
    memory: Box<[Word; MEM_SIZE]>,
}

macro_rules! tos_binary {
    ($vm:expr, $op:tt) => {{
        let sp = $vm.memory[SP] as Address;
        $vm.memory[sp - 2] = ($vm.memory[sp - 2] $op $vm.memory[sp - 1]) as Word;
        $vm.memory[SP] -= 1;
        $vm.pc += 1;
    }};
}

macro_rules! tos_binary_bool {
    ($vm:expr, $op:tt) => {{
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

    fn get_value(&self, segment: Segment, index: i16) -> i16 {
        if segment == Segment::Constant {
            index as i16
        } else {
            let addr = self.get_seg_address(segment, index) as Address;
            self.memory[addr]
        }
    }

    pub fn load(&mut self, program: Vec<Opcode>) {
        self.program = program;
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

    pub fn step(&mut self) {
        use Instruction::{
            Add, And, Call, Eq, Function, Goto, Gt, IfGoto, Lt, Neg, Not, Or, Pop, Push, Return,
            Sub,
        };

        let opcode = self.program[self.pc];
        let instr = opcode.try_into().unwrap();
        if cfg!(test) {
            dbg!(self.pc);
            dbg!(instr);
            dbg!(self.memory[SP]);
            dbg!(self.tos());
        }

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
                println!("pushing {}", value);
                self.push(value);
                self.pc += 1;
            }
            Pop => {
                let segment = self.consume_segment();
                let index = self.consume_short();
                let address = self.get_seg_address(segment, index);
                let value = self.pop();
                *self.mem(address) = value;
                println!("memory[{}] = {}", address, value);
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
                    println!("jumping to {}", instr);
                    self.pc = instr as usize;
                }
            }
            Function => unimplemented!(),
            Return => unimplemented!(),
            Call => unimplemented!(),
        };

        if cfg!(test) {
            dbg!(self.memory[SP]);
            dbg!(self.tos());
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        let mut vm = Self {
            pc: 0,
            program: vec![],
            memory: Box::new([0; MEM_SIZE]),
        };

        // page 162 of the book:
        // the VM implementation can start by generating assembly code that sets SP=256
        vm.memory[0] = 256;

        vm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        vm.load(bytecode);

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
        *vm.mem(SP) = 256;
        *vm.mem(LCL) = 300;
        *vm.mem(ARG) = 400;
        *vm.mem(THIS) = 3000;
        *vm.mem(THAT) = 3010;

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

        vm.load(program);

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
        *vm.mem(0) = 256;

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

        vm.load(program);

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
        *vm.mem(0) = 256;

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

        vm.load(program);

        for _ in 0..11 {
            vm.step();
        }

        assert_eq!(1110, *vm.mem(256));
    }

    #[test]
    fn simple_add() {
        let mut vm = VM::default();
        *vm.mem(0) = 256;

        let bytecode = r#"
            push constant 7
            push constant 8
            add"#;

        let programs = vec![SourceFile::new("SimpleAdd.vm", bytecode)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..3 {
            vm.step();
        }

        assert_eq!(257, *vm.mem(0));
        assert_eq!(15, *vm.mem(256));
    }

    #[test]
    fn stack_test() {
        let mut vm = VM::default();
        *vm.mem(0) = 256;

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

        vm.load(program);

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
        *vm.mem(SP) = 256;
        *vm.mem(LCL) = 300;
        *vm.mem(ARG) = 400;
        *vm.mem_indirect(ARG, 0) = 3;

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

        vm.load(program);

        for _ in 0..33 {
            vm.step();
        }

        assert_eq!(257, *vm.mem(0));
        assert_eq!(6, *vm.mem(256));
    }

    #[test]
    fn fibonacci_series() {
        let mut vm = VM::default();
        *vm.mem(SP) = 256;
        *vm.mem(LCL) = 300;
        *vm.mem(ARG) = 400;
        *vm.mem_indirect(ARG, 0) = 6;
        *vm.mem_indirect(ARG, 1) = 3000;

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

        vm.load(program);

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
}

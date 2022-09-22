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

    #[test]
    fn basic_test_vme() {
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
}

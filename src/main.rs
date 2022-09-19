use crate::definitions::*;
use crate::simulators::vm::command::{Instruction, Opcode, Segment};
use crate::simulators::vm::VM;

mod definitions;
mod simulators;

fn main() {
    // push constant 0
    // pop local 0
    // label LOOP
    // push local 0
    // push constant 10
    // lt
    // not
    // if-goto END
    // push local 0
    // push constant 1
    // add
    // pop local 0
    // goto LOOP
    // label END
    // goto END

    let bytecode = vec![
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

    let mut vm = VM::default();
    vm.load(bytecode);
    *vm.mem(LCL) = 300;
    *vm.mem(ARG) = 400;
    *vm.mem(THIS) = 3000;
    *vm.mem(THAT) = 3010;

    for _ in 0..100 {
        vm.step();
    }

    println!("Done");
}

use definitions::{ARG, LCL, THAT, THIS};
use simulators::vm::VM;

use parse::bytecode::{Parser, SourceFile};

mod definitions;
mod parse;
mod simulators;

fn main() {
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
    let bytecode = parser.parse().unwrap();

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

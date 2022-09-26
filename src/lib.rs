use wasm_bindgen::prelude::*;

mod definitions;
mod parse;
mod simulators;

use parse::bytecode::Parser;
use parse::bytecode::SourceFile;
use simulators::vm::VM;

#[wasm_bindgen]
pub struct App {
    vm: VM,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl App {
    pub fn new() -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        let mut vm = VM::default();
        let program = "
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
            ";

        let programs = vec![SourceFile::new("Lines.vm", program)];
        let mut bytecode_parser = Parser::new(programs);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        Self { vm }
    }

    pub fn step(&mut self) {
        self.vm.step();
    }

    pub fn display_buffer(&self) -> Vec<i16> {
        // TODO: maybe do this without copying, or even better do the entire rendering inside of wasm
        self.vm.display().to_vec()
    }
}

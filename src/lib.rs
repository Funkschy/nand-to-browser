use crate::simulators::vm::stdlib::Stdlib;
use wasm_bindgen::prelude::*;

mod definitions;
mod parse;
mod simulators;
mod util;

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

        let mut vm = VM::new(Stdlib::new());

        let sys = include_str!("../res/stdlib/Sys.vm");
        let array = include_str!("../res/stdlib/Array.vm");
        let keyboard = include_str!("../res/stdlib/Keyboard.vm");
        let math = include_str!("../res/stdlib/Math.vm");
        let memory = include_str!("../res/stdlib/Memory.vm");
        let output = include_str!("../res/stdlib/Output.vm");
        let screen = include_str!("../res/stdlib/Screen.vm");
        let string = include_str!("../res/stdlib/String.vm");
        let main = include_str!("../res/tetris/Main.vm");
        let random = include_str!("../res/tetris/Random.vm");
        let render = include_str!("../res/tetris/Render.vm");
        let tetromino = include_str!("../res/tetris/Tetromino.vm");

        let programs = vec![
            SourceFile::new("Sys.vm", sys),
            SourceFile::new("Keyboard.vm", keyboard),
            SourceFile::new("Math.vm", math),
            SourceFile::new("Memory.vm", memory),
            SourceFile::new("Array.vm", array),
            SourceFile::new("Output.vm", output),
            SourceFile::new("Screen.vm", screen),
            SourceFile::new("String.vm", string),
            SourceFile::new("Main.vm", main),
            SourceFile::new("Random.vm", random),
            SourceFile::new("Render.vm", render),
            SourceFile::new("Tetromino.vm", tetromino),
        ];

        let mut bytecode_parser = Parser::with_stdlib(programs, Stdlib::new());
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        Self { vm }
    }

    pub fn step_times(&mut self, times: u32) {
        for _ in 0..times {
            self.vm.step();
        }
    }

    pub fn step(&mut self) {
        self.vm.step();
    }

    pub fn display_buffer(&self) -> Vec<i16> {
        // TODO: maybe do this without copying, or even better do the entire rendering inside of wasm
        self.vm.display().to_vec()
    }

    pub fn set_input_key(&mut self, key: i16) {
        self.vm.set_input_key(key);
    }
}

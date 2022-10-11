use crate::definitions::{Word, BITS_PER_WORD, SCREEN_HEIGHT, SCREEN_WIDTH, SCREEN_WIDTH_IN_WORDS};
use crate::simulators::vm::stdlib::Stdlib;
use crate::simulators::vm::VMError;
use wasm_bindgen::prelude::*;

mod definitions;
mod keyboard;
mod parse;
mod simulators;

use parse::bytecode::Parser;
use parse::bytecode::SourceFile;
use simulators::vm::VM;

use wasm_bindgen::Clamped;
use web_sys::ImageData;

#[wasm_bindgen]
pub fn get_key_code(letter: &str) -> Option<Word> {
    keyboard::get_key_code(letter)
}

#[wasm_bindgen]
pub struct App {
    vm: VM,
    programs: Vec<(String, String)>, // (filename, content)
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl From<VMError> for JsValue {
    fn from(error: VMError) -> Self {
        JsValue::from(error.to_string())
    }
}

#[wasm_bindgen]
impl App {
    pub fn new() -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        let vm = VM::new(Stdlib::new());

        Self {
            vm,
            programs: Vec::new(),
        }
    }

    pub fn reset_files(&mut self) {
        self.programs.clear();
    }

    pub fn add_file(&mut self, name: String, content: String) {
        self.programs.push((name, content));
    }
    pub fn load_files(&mut self) {
        let stdlib = Stdlib::new();
        let programs = self
            .programs
            .iter()
            .map(|(name, content)| SourceFile::new(name, content))
            .collect::<Vec<_>>();

        let mut bytecode_parser = Parser::with_stdlib(programs, stdlib);
        let program = bytecode_parser.parse().unwrap();

        self.vm.load(program);
    }

    pub fn step_times(&mut self, times: u32) -> Result<(), JsValue> {
        for _ in 0..times {
            self.vm.step()?;
        }
        Ok(())
    }

    pub fn step(&mut self) {
        self.vm.step().unwrap();
    }

    pub fn set_input_key(&mut self, key: Word) {
        self.vm.set_input_key(key).unwrap();
    }

    pub fn data_buffer_size() -> usize {
        const BYTES_PER_PIXEL: usize = 4; // rgba
        BYTES_PER_PIXEL * SCREEN_WIDTH * SCREEN_HEIGHT
    }

    pub fn display_data(&self) -> ImageData {
        let display = self.vm.display();
        let mut data = Vec::with_capacity(Self::data_buffer_size());
        for row_idx in 0..SCREEN_HEIGHT {
            for word_idx in 0..SCREEN_WIDTH_IN_WORDS {
                let word = display[row_idx * SCREEN_WIDTH_IN_WORDS + word_idx];
                for pixel_idx in 0..BITS_PER_WORD {
                    let mask = 1 << pixel_idx;
                    let value = word & mask;
                    let color = if value == 0 { 255 } else { 0 };

                    data.push(color);
                    data.push(color);
                    data.push(color);
                    data.push(255);
                }
            }
        }

        ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(data.as_slice()),
            SCREEN_WIDTH as u32,
            SCREEN_HEIGHT as u32,
        )
        .unwrap()
    }
}

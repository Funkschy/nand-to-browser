mod definitions;
mod keyboard;
#[allow(dead_code)]
mod parse;
#[allow(dead_code)]
mod simulators;

use definitions::{
    Address, Word, BITS_PER_WORD, SCREEN_HEIGHT, SCREEN_WIDTH, SCREEN_WIDTH_IN_WORDS,
};
use parse::assembly::{self, AssemblyParseError, AssemblyParser};
use parse::bytecode::{self, BytecodeParseError, BytecodeParser};
use simulators::cpu::{Cpu, CpuError};
use simulators::vm::meta::FileInfo;
use simulators::vm::stdlib::Stdlib;
use simulators::vm::{VMError, VM};
use wasm_bindgen::prelude::*;

use wasm_bindgen::Clamped;
use web_sys::ImageData;

#[wasm_bindgen]
pub fn get_key_code(letter: &str) -> Option<Word> {
    keyboard::get_key_code(letter)
}

enum Simulator {
    None,
    VM(Box<VM>),
    Cpu(Box<Cpu>),
}

impl Simulator {
    fn step(&mut self) -> SimResult {
        match self {
            Self::None => Err("Cannot step without a Simulator".into()),
            Self::VM(vm) => Ok(vm.step_until_vm_instr()?),
            Self::Cpu(cpu) => Ok(cpu.step()?),
        }
    }

    pub fn step_times(&mut self, times: u32) -> SimResult {
        match self {
            Self::None => return Err("Cannot step without a Simulator".into()),
            Self::VM(vm) => {
                for _ in 0..times {
                    vm.step()?;
                }
            }
            Self::Cpu(cpu) => {
                for _ in 0..times {
                    cpu.step()?;
                }
            }
        }
        Ok(())
    }

    pub fn set_input_key(&mut self, key: i16) -> SimResult {
        match self {
            Self::None => Ok(()),
            Self::VM(vm) => Ok(vm.set_input_key(key)?),
            Self::Cpu(cpu) => Ok(cpu.set_input_key(key)?),
        }
    }

    pub fn memory_at(&self, address: Address) -> Option<Word> {
        match self {
            Self::None => None,
            Self::VM(vm) => vm.memory_at(address),
            Self::Cpu(cpu) => cpu.memory_at(address),
        }
    }

    pub fn current_file_offset(&self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::VM(vm) => vm.current_file_offset(),
            Self::Cpu(cpu) => Some(cpu.current_file_offset()),
        }
    }

    pub fn display(&self) -> Option<&[Word]> {
        match self {
            Self::None => None,
            Self::VM(vm) => Some(vm.display()),
            Self::Cpu(cpu) => Some(cpu.display()),
        }
    }
}

#[wasm_bindgen]
pub struct App {
    sim: Simulator,
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

impl From<CpuError> for JsValue {
    fn from(error: CpuError) -> Self {
        JsValue::from(error.to_string())
    }
}

impl From<BytecodeParseError> for JsValue {
    fn from(error: BytecodeParseError) -> Self {
        JsValue::from(error.to_string())
    }
}

impl From<AssemblyParseError> for JsValue {
    fn from(error: AssemblyParseError) -> Self {
        JsValue::from(error.to_string())
    }
}

type SimResult = Result<(), JsValue>;

#[wasm_bindgen]
impl App {
    pub fn new() -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        Self {
            sim: Simulator::None,
            programs: Vec::new(),
        }
    }

    pub fn reset_files(&mut self) {
        self.programs.clear();
    }

    pub fn add_file(&mut self, name: String, content: String) {
        self.programs.push((name, content));
    }

    pub fn load_files(&mut self) -> SimResult {
        let is_vm = self
            .programs
            .first()
            .map(|(name, _)| name.ends_with(".vm"))
            .unwrap_or(false);

        if is_vm {
            for (name, _) in &self.programs {
                if !name.ends_with(".vm") {
                    return Err("Either load multiple .vm files or a single .asm file".into());
                }
            }

            let mut vm = VM::new(Stdlib::new());

            let stdlib = Stdlib::new();
            let programs = self
                .programs
                .iter()
                .map(|(name, content)| bytecode::SourceFile::new(name, content))
                .collect::<Vec<_>>();

            let mut bytecode_parser = BytecodeParser::with_stdlib(programs, stdlib);
            let program = bytecode_parser.parse()?;

            vm.load(program);
            self.sim = Simulator::VM(vm.into());
        } else {
            let (_, content) = self
                .programs
                .get(0)
                .ok_or_else::<JsValue, _>(|| "Trying to load empty program vector".into())?;

            let mut cpu = Cpu::default();
            let mut assembly_parser = AssemblyParser::new(assembly::SourceFile::new(content));
            let program = assembly_parser.parse()?;

            cpu.load(program);
            self.sim = Simulator::Cpu(cpu.into());
        }

        Ok(())
    }

    // --- General Simulator features ---

    pub fn step_times(&mut self, times: u32) -> SimResult {
        self.sim.step_times(times)
    }

    pub fn step(&mut self) -> SimResult {
        self.sim.step()
    }

    pub fn set_input_key(&mut self, key: Word) -> SimResult {
        self.sim.set_input_key(key)
    }

    pub fn memory_at(&self, address: Address) -> Option<Word> {
        self.sim.memory_at(address)
    }

    pub fn current_file_offset(&self) -> Option<usize> {
        self.sim.current_file_offset()
    }

    pub fn data_buffer_size() -> usize {
        const BYTES_PER_PIXEL: usize = 4; // rgba
        BYTES_PER_PIXEL * SCREEN_WIDTH * SCREEN_HEIGHT
    }

    pub fn display_data(&self) -> Option<ImageData> {
        let display = self.sim.display()?;
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
        .ok()
    }
}

// VM Emulator specific stuff
#[wasm_bindgen]
impl App {
    pub fn calls(&self) -> Vec<JsValue> {
        if let Simulator::VM(vm) = &self.sim {
            vm.call_stack_names()
                .into_iter()
                .map(JsValue::from_str)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn locals(&self) -> Vec<Word> {
        if let Simulator::VM(vm) = &self.sim {
            if let Some(locals) = vm.locals() {
                return locals.to_vec();
            }
        }

        Vec::new()
    }

    pub fn args(&self) -> Vec<Word> {
        if let Simulator::VM(vm) = &self.sim {
            if let Some(args) = vm.args() {
                return args.to_vec();
            }
        }

        Vec::new()
    }

    pub fn stack(&self) -> Vec<Word> {
        if let Simulator::VM(vm) = &self.sim {
            if let Some(stack) = vm.stack() {
                return stack.to_vec();
            }
        }

        Vec::new()
    }

    pub fn current_function_name(&self) -> Option<String> {
        if let Simulator::VM(vm) = &self.sim {
            return vm.current_function_name().map(|n| n.to_owned());
        }
        None
    }

    pub fn current_file_name(&self) -> Option<String> {
        if let Simulator::VM(vm) = &self.sim {
            match vm.current_file_info()? {
                FileInfo::VM { module_index, .. } => {
                    self.programs.get(module_index).map(|p| p.0.to_owned())
                }
                FileInfo::Builtin(name) => Some(name.to_owned()),
            }
        } else {
            None
        }
    }
}

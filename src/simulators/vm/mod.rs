pub mod command;

#[allow(dead_code)]
pub mod stdlib;

use crate::definitions::SCREEN_START;
use crate::definitions::{Address, Symbol, Word, ARG, INIT_SP, KBD, LCL, MEM_SIZE, SP, THAT, THIS};
use command::{Instruction, Segment};
use std::collections::HashMap;
use stdlib::{BuiltinFunction, State, Stdlib, StdlibError, StdlibOk, VMCallOk, VirtualMachine};

pub trait ProgramInfo {
    fn instructions(&self) -> &Vec<Instruction>;
    fn debug_symbols(&self) -> &HashMap<Symbol, String>;
    fn function_by_name(&self) -> &HashMap<String, Symbol>;
    fn sys_init_address(&self) -> Option<Symbol>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReturnAddress {
    EndOfProgram, // the return for the first function in the program (usually Sys.init)
    VM(Symbol),
    Builtin(State),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CallState {
    TopLevel,
    // the state the function is in and the original args
    Builtin(State, Vec<Word>),
    VM,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallStackEntry {
    ret_addr: ReturnAddress,
    function: Option<Symbol>,
    state: CallState,
}

impl CallStackEntry {
    pub fn top_level() -> Self {
        Self {
            ret_addr: ReturnAddress::EndOfProgram,
            function: None,
            state: CallState::TopLevel,
        }
    }

    pub fn top_level_vm() -> Self {
        Self {
            ret_addr: ReturnAddress::EndOfProgram,
            function: None,
            state: CallState::VM,
        }
    }

    pub fn builtin(
        ret_addr: ReturnAddress,
        function: Symbol,
        state: State,
        args: Vec<Word>,
    ) -> Self {
        Self {
            ret_addr,
            function: Some(function),
            state: CallState::Builtin(state, args),
        }
    }

    pub fn vm(ret_addr: ReturnAddress, function: Symbol) -> Self {
        Self {
            ret_addr,
            function: Some(function),
            state: CallState::VM,
        }
    }
}

pub struct VM {
    // the program counter / instruction pointer
    pc: usize,
    program: Vec<Instruction>,

    // debug information which is used in the UI and for internal debugging
    debug_symbols: HashMap<Symbol, String>,
    function_by_name: HashMap<String, Symbol>,

    call_stack: Vec<CallStackEntry>,

    stdlib: Stdlib<'static, Self>,
    // if this is set to Some(address) the vm will jump to Sys.init on the next step
    sys_init: Option<Symbol>,

    // 0-15        virtual registers
    // 16-255      static variables
    // 256-2047    stack
    // 2048-16483  heap
    // 16384-24575 memory mapped io
    memory: Box<[Word; MEM_SIZE]>,
}

macro_rules! trace_vm {
    ($block:expr) => {
        if cfg!(feature = "trace_vm") {
            $block
        }
    };
}

macro_rules! trace_calls {
    ($block:expr) => {
        if cfg!(any(feature = "trace_vm", feature = "trace_calls")) {
            $block
        }
    };
}

macro_rules! tos_binary {
    ($vm:expr, $op:tt) => {{
        let sp = $vm.memory[SP] as Address;
        trace_vm!({
            println!("{} {} {}", $vm.memory[sp - 2], stringify!($op), $vm.memory[sp - 1]);
        });
        // cast up to i32 so that no overflow checks get triggered in debug mode
        $vm.memory[sp - 2] = ($vm.memory[sp - 2] as i32 $op $vm.memory[sp - 1] as i32) as Word;
        $vm.memory[SP] -= 1;
        $vm.pc += 1;
    }};
}

macro_rules! tos_binary_bool {
    ($vm:expr, $op:tt) => {{
        trace_vm!({
            println!("{}", stringify!($op));
        });
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
        trace_vm!({
            println!("{}", stringify!($op));
        });
        let sp = $vm.memory[SP] as Address;
        $vm.memory[sp - 1] = $op($vm.memory[sp - 1] as Word);
        $vm.pc += 1;
    }};
}

impl VirtualMachine for VM {
    #[inline]
    fn mem(&self, address: Address) -> Word {
        self.memory[address]
    }

    #[inline]
    fn set_mem(&mut self, address: Address, value: Word) {
        self.memory[address] = value;
    }

    #[inline]
    fn pop(&mut self) -> Word {
        self.add_to_mem(SP, -1);
        self.mem_indirect(SP, 0)
    }

    fn call(&mut self, name: &str, params: &[Word]) -> Result<VMCallOk, StdlibError> {
        trace_calls!({
            println!("Calling {} by name", name);
        });

        let address = self
            .function_by_name
            .get(name)
            .copied()
            .ok_or(StdlibError::CallingNonExistendFunction)?;

        if let Some(&stdlib_function) = self.stdlib.by_address(address) {
            trace_calls!({
                println!("{} is a builtin function", stdlib_function.name());
            });

            self.call_builtin_function(stdlib_function, params);
            Ok(VMCallOk::WasBuiltinFunction)
        } else {
            for &p in params {
                self.push(p);
            }
            self.call_vm_function(address, params.len() as Word);
            Ok(VMCallOk::WasVMFunction)
        }
    }
}

impl VM {
    pub fn new(stdlib: Stdlib<'static, Self>) -> Self {
        Self {
            pc: 0,
            program: vec![],
            debug_symbols: HashMap::new(),
            call_stack: Vec::with_capacity(32),
            memory: Box::new([0; MEM_SIZE]),
            stdlib,
            function_by_name: HashMap::new(),
            sys_init: None,
        }
    }

    #[inline]
    fn mem_indirect(&self, address_of_address: Address, offset: usize) -> Word {
        self.memory[self.memory[address_of_address] as Address + offset]
    }

    #[inline]
    fn set_mem_indirect(&mut self, address_of_address: Address, offset: usize, value: Word) {
        self.set_mem(self.memory[address_of_address] as Address + offset, value);
    }

    #[inline]
    fn add_to_mem(&mut self, address: Address, relative_value: Word) {
        self.set_mem(address, self.mem(address) + relative_value);
    }

    #[inline]
    fn push(&mut self, value: Word) {
        self.set_mem_indirect(SP, 0, value);
        self.add_to_mem(SP, 1);
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

    pub fn set_input_key(&mut self, key: i16) {
        self.set_mem(KBD, key);
    }

    pub fn load(&mut self, info: impl ProgramInfo) {
        self.program = info.instructions().clone();
        self.debug_symbols = info.debug_symbols().clone();
        self.function_by_name = info.function_by_name().clone();
        self.pc = 0;
        for i in 0..self.memory.len() {
            self.memory[i] = 0;
        }
        // page 162 of the book:
        // the VM implementation c
        // an start by generating assembly code that sets SP=256
        self.set_mem(SP, INIT_SP);

        self.call_stack.clear();
        self.push_call(CallStackEntry::top_level());

        match info.sys_init_address() {
            Some(sys_init_address) if sys_init_address != 0 => {
                println!("Sys.init at {}", sys_init_address);
                self.sys_init = Some(sys_init_address);
                self.push_call(CallStackEntry::top_level());
            }
            _ => {
                // the vm must behave slightly differently if there is no Sys.init function
                // in this case the execution will simply begin at the zero'th instruction, instead
                // of calling Sys.init, which means that the top level function is a VM function
                self.push_call(CallStackEntry::top_level_vm());
            }
        }
    }

    fn push_call(&mut self, entry: CallStackEntry) -> usize {
        let idx = self.call_stack.len();
        self.call_stack.push(entry);
        idx
    }

    fn peek_call(&mut self) -> Option<&mut CallStackEntry> {
        self.call_stack.last_mut()
    }

    fn update_call_stack_tos_next_state(&mut self, next_state: State) {
        self.update_call_stack_index_next_state(self.call_stack.len() - 1, next_state);
    }

    fn update_call_stack_index_next_state(&mut self, index: usize, next_state: State) {
        let call = self.call_stack.get_mut(index).unwrap();

        if let CallState::Builtin(ref mut old_state, _) = call.state {
            trace_calls!({
                println!(
                    "updating state of {:?} from {} to {}",
                    call.function, *old_state, next_state
                );
            });
            *old_state = next_state;
        } else {
            panic!("trying to continue a vm function");
        }
    }

    fn return_address(&mut self) -> Option<ReturnAddress> {
        let current_call = self.peek_call()?;
        match current_call.state {
            CallState::TopLevel => Some(ReturnAddress::EndOfProgram),
            CallState::Builtin(state, _) => Some(ReturnAddress::Builtin(0)),
            CallState::VM => Some(ReturnAddress::VM(self.pc as Symbol + 1)),
        }
    }

    fn pop_call(&mut self) -> Option<CallStackEntry> {
        self.call_stack.pop()
    }

    pub fn call_stack_names(&self) -> Vec<&str> {
        self.call_stack
            .iter()
            .filter_map(|c| {
                c.function
                    .and_then(|f| self.debug_symbols.get(&f).map(String::as_str))
            })
            .collect()
    }

    fn handle_builtin_finished(&mut self, ret_val: Word) {
        let this_call = self.pop_call().unwrap();
        self.push(ret_val);
        if let ReturnAddress::VM(ret_addr) = this_call.ret_addr {
            // jump to the appropriate position
            self.pc = ret_addr as usize;
        }
    }

    fn continue_builtin_function(&mut self, entry: CallStackEntry) {
        use StdlibOk::*;

        trace_calls!({
            println!(
                "continuing {:?} {:?}",
                entry.function.and_then(|f| self.debug_symbols.get(&f)),
                entry
            );
        });

        let function = *self.stdlib.by_address(entry.function.unwrap()).unwrap();
        let (state, args) = if let CallState::Builtin(state, args) = entry.state {
            (state, args)
        } else {
            panic!("trying to continue vm function");
        };
        // the call continuation might call another function, so we need to save the index of the
        // current function to use it when updating the call state
        let this_call_idx = self.call_stack.len() - 1;
        let ret_val = function.continue_call(self, state, &args).unwrap();

        match ret_val {
            Finished(ret_val) => {
                trace_calls!({
                    println!(
                        "returning from stdlib function {} with return value {}",
                        function.name(),
                        ret_val
                    );
                });
                self.handle_builtin_finished(ret_val);
            }
            ContinueInNextStep(next_state) => {
                self.update_call_stack_index_next_state(this_call_idx, next_state);
            }
        }
    }

    fn call_builtin_function(&mut self, function: BuiltinFunction<'static, Self>, args: &[Word]) {
        use StdlibOk::{ContinueInNextStep, Finished};

        trace_calls!({
            println!(
                "calling stdlib function {} with {:?}",
                function.name(),
                &args
            );
            println!("{:?}", self.call_stack_names());
        });

        let ret_addr = self.return_address().unwrap();
        let init_state = 0;
        let index = self.push_call(CallStackEntry::builtin(
            ret_addr,
            function.virtual_address(),
            init_state,
            args.to_owned(),
        ));
        // TODO: handle error return value
        let ret_val = function.call(self, args).unwrap();

        match ret_val {
            Finished(ret_val) => {
                trace_calls!({
                    println!(
                        "returning from stdlib function {} with return value {}",
                        function.name(),
                        ret_val
                    );
                });
                self.handle_builtin_finished(ret_val);
            }
            ContinueInNextStep(next_state) => {
                self.update_call_stack_index_next_state(index, next_state);
            }
        }
    }

    fn call_vm_function(&mut self, function: Symbol, n_args: i16) {
        // TODO: handle error if calling a non existing function

        trace_calls!({
            println!("call {} at {}", self.debug_symbols[&function], function);
            println!("{:?}", self.call_stack_names());
        });

        let ret_addr = self.pc + 1;
        self.push(ret_addr as Word);

        let lcl = self.mem(LCL);
        self.push(lcl);
        let arg = self.mem(ARG);
        self.push(arg);
        let this = self.mem(THIS);
        self.push(this);
        let that = self.mem(THAT);
        self.push(that);

        let sp = self.mem(SP);
        let arg = sp - n_args - 5;
        self.set_mem(ARG, arg);
        self.set_mem(LCL, sp);

        let ret_addr = self.return_address().unwrap();
        self.push_call(CallStackEntry::vm(ret_addr, function));
        self.pc = function as usize;
    }

    fn call_function(&mut self, function: Symbol, n_args: i16) -> Result<VMCallOk, StdlibError> {
        if let Some(&stdlib_function) = self.stdlib.by_address(function) {
            trace_calls!({
                println!("{} is a builtin function", stdlib_function.name());
            });

            // TODO: assert that if this was called by bytecode, the n_args matches
            let n_args = stdlib_function.num_args();
            let sp = self.mem(SP) as usize;
            let args = Vec::from(&self.memory[sp - n_args..sp]);
            self.set_mem(SP, (sp - n_args) as i16);

            self.call_builtin_function(stdlib_function, &args);
            Ok(VMCallOk::WasBuiltinFunction)
        } else {
            self.call_vm_function(function, n_args);
            Ok(VMCallOk::WasVMFunction)
        }
    }

    pub fn run(&mut self) {
        self.pc = 0;
        while self.pc < self.program.len() {
            self.step();
        }
    }

    pub fn step(&mut self) {
        use Instruction::{
            Add, And, Call, Eq, Function, Goto, Gt, IfGoto, Lt, Neg, Not, Or, Pop, Push, Return,
            Sub,
        };

        if let Some(sys_init_address) = self.sys_init {
            println!("jumping to Sys.init at {}", sys_init_address);
            self.call_function(sys_init_address, 0).unwrap();
            self.sys_init = None;
            return;
        }

        let currently_in_builtin_f = matches!(
            self.peek_call(),
            Some(CallStackEntry {
                state: CallState::Builtin(_, _),
                ..
            })
        );

        if currently_in_builtin_f {
            let peeked = self.peek_call().unwrap().clone();
            self.continue_builtin_function(peeked);
            return;
        }

        let instr = self.program[self.pc];

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
            Push { segment, index } => {
                let value = self.get_value(segment, index);

                trace_vm!({
                    println!("push {:?} {} {}", segment, index, value);
                });

                self.push(value);
                self.pc += 1;
            }
            Pop { segment, index } => {
                let address = self.get_seg_address(segment, index);
                let value = self.pop();

                trace_vm!({
                    println!("pop {:?} {} {} {}", segment, index, address, value);
                });

                self.set_mem(address, value);
                self.pc += 1;
            }
            Goto { instruction } => {
                // TODO: implement debug symbols for labels
                trace_vm!({
                    println!("goto {}", instruction);
                });
                self.pc = instruction as usize;
            }
            IfGoto { instruction } => {
                let condition = self.pop();
                trace_vm!({
                    println!("if-goto {} {}", condition, instruction);
                });

                if condition == 0 {
                    self.pc += 1;
                } else {
                    self.pc = instruction as usize;
                }
            }
            Function { n_locals } => {
                trace_calls!({
                    println!("function {}", self.debug_symbols[&(self.pc as u16)]);
                    println!("SP   {}", self.mem(SP));
                    println!("LCL  {}", self.mem(LCL));
                    println!("ARG  {}", self.mem(ARG));
                    println!("THIS {}", self.mem(THIS));
                    println!("THAT {}", self.mem(THAT));
                    println!("PC   {}", self.pc);
                });

                for _ in 0..n_locals {
                    self.push(0);
                }
                self.pc += 1;
            }
            Return => {
                trace_calls!({
                    println!("return");
                });

                let frame = self.mem(LCL) as Address;
                if frame < 5 {
                    panic!("{} {} {:?}", frame, self.pc, self.call_stack_names());
                }
                // the return address
                let ret = self.mem(frame - 5) as Address;

                // reposition the return value for the caller
                let return_value = self.pop();
                self.set_mem_indirect(ARG, 0, return_value);

                // restore the stack for the caller
                self.set_mem(SP, self.mem(ARG) + 1);
                self.set_mem(THAT, self.mem(frame - 1));
                self.set_mem(THIS, self.mem(frame - 2));
                self.set_mem(ARG, self.mem(frame - 3));
                self.set_mem(LCL, self.mem(frame - 4));

                let popped = self.pop_call().unwrap();

                if popped.ret_addr != ReturnAddress::EndOfProgram {
                    self.pc = ret;
                }

                trace_calls!({
                    print!(
                        "returning from {:?}",
                        popped.function.map(|f| &self.debug_symbols[&f])
                    );

                    if let Some(ret_to) = self.call_stack.last() {
                        println!(" to {:?}", ret_to.function.map(|f| &self.debug_symbols[&f]));
                        println!("LCL changed from {} to {}", frame, self.memory[LCL]);
                    } else {
                        println!(" to nowhere");
                    }
                    println!("at address {}", ret);
                });
            }
            Call { function, n_args } => {
                self.call_function(function, n_args).unwrap();
            }
        };

        trace_vm!({
            dbg!(self.pc);
            dbg!(self.memory[SP]);
            dbg!(self.memory[LCL]);
            dbg!(self.memory[ARG]);
            dbg!(self.memory[THIS]);
            dbg!(self.memory[THAT]);
            dbg!(self.tos());
        });
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new(Stdlib::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::KBD;
    use crate::definitions::SCREEN_START;
    use crate::parse::bytecode::{ParsedProgram, Parser, SourceFile};
    use crate::simulators::vm::stdlib::{BuiltinFunction, StdResult};

    #[test]
    fn basic_test_vme_no_parse() {
        let mut vm = VM::default();

        let bytecode = vec![
            Instruction::Push {
                segment: Segment::Constant,
                index: 10,
            },
            Instruction::Pop {
                segment: Segment::Local,
                index: 0,
            },
            Instruction::Push {
                segment: Segment::Constant,
                index: 21,
            },
            Instruction::Push {
                segment: Segment::Constant,
                index: 22,
            },
            Instruction::Pop {
                segment: Segment::Argument,
                index: 2,
            },
            Instruction::Pop {
                segment: Segment::Argument,
                index: 1,
            },
            Instruction::Push {
                segment: Segment::Constant,
                index: 36,
            },
            Instruction::Pop {
                segment: Segment::This,
                index: 6,
            },
            Instruction::Push {
                segment: Segment::Constant,
                index: 42,
            },
            Instruction::Push {
                segment: Segment::Constant,
                index: 45,
            },
            Instruction::Pop {
                segment: Segment::That,
                index: 5,
            },
            Instruction::Pop {
                segment: Segment::That,
                index: 2,
            },
            Instruction::Push {
                segment: Segment::Constant,
                index: 510,
            },
            Instruction::Pop {
                segment: Segment::Temp,
                index: 6,
            },
            Instruction::Push {
                segment: Segment::Local,
                index: 0,
            },
            Instruction::Push {
                segment: Segment::That,
                index: 5,
            },
            Instruction::Add,
            Instruction::Push {
                segment: Segment::Argument,
                index: 1,
            },
            Instruction::Sub,
            Instruction::Push {
                segment: Segment::This,
                index: 6,
            },
            Instruction::Push {
                segment: Segment::This,
                index: 6,
            },
            Instruction::Add,
            Instruction::Sub,
            Instruction::Push {
                segment: Segment::Temp,
                index: 6,
            },
            Instruction::Add,
        ];
        let program = ParsedProgram::new(bytecode, HashMap::new(), HashMap::new());
        vm.load(program);

        vm.set_mem(SP, 256);
        vm.set_mem(LCL, 300);
        vm.set_mem(ARG, 400);
        vm.set_mem(THIS, 3000);
        vm.set_mem(THAT, 3010);

        for _ in 0..25 {
            vm.step();
        }

        assert_eq!(472, vm.mem(256));
        assert_eq!(10, vm.mem(300));
        assert_eq!(21, vm.mem(401));
        assert_eq!(22, vm.mem(402));
        assert_eq!(36, vm.mem(3006));
        assert_eq!(42, vm.mem(3012));
        assert_eq!(45, vm.mem(3015));
        assert_eq!(510, vm.mem(11));
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

        vm.load(program);

        vm.set_mem(SP, 256);
        vm.set_mem(LCL, 300);
        vm.set_mem(ARG, 400);
        vm.set_mem(THIS, 3000);
        vm.set_mem(THAT, 3010);

        for _ in 0..25 {
            vm.step();
        }

        assert_eq!(472, vm.mem(256));
        assert_eq!(10, vm.mem(300));
        assert_eq!(21, vm.mem(401));
        assert_eq!(22, vm.mem(402));
        assert_eq!(36, vm.mem(3006));
        assert_eq!(42, vm.mem(3012));
        assert_eq!(45, vm.mem(3015));
        assert_eq!(510, vm.mem(11));
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

        vm.load(program);

        vm.set_mem(0, 256);

        for _ in 0..15 {
            vm.step();
        }

        assert_eq!(6084, vm.mem(256));
        assert_eq!(3030, vm.mem(3));
        assert_eq!(3040, vm.mem(4));
        assert_eq!(32, vm.mem(3032));
        assert_eq!(46, vm.mem(3046));
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

        vm.load(program);

        vm.set_mem(0, 256);

        for _ in 0..11 {
            vm.step();
        }

        assert_eq!(1110, vm.mem(256));
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

        vm.load(program);

        vm.set_mem(0, 256);

        for _ in 0..3 {
            vm.step();
        }

        assert_eq!(257, vm.mem(0));
        assert_eq!(15, vm.mem(256));
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

        vm.load(program);

        vm.set_mem(0, 256);

        for _ in 0..38 {
            vm.step();
        }

        assert_eq!(266, vm.mem(0));
        assert_eq!(-1, vm.mem(256));
        assert_eq!(0, vm.mem(257));
        assert_eq!(0, vm.mem(258));
        assert_eq!(0, vm.mem(259));
        assert_eq!(-1, vm.mem(260));
        assert_eq!(0, vm.mem(261));
        assert_eq!(-1, vm.mem(262));
        assert_eq!(0, vm.mem(263));
        assert_eq!(0, vm.mem(264));
        assert_eq!(-91, vm.mem(265));
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

        vm.load(program);

        vm.set_mem(SP, 256);
        vm.set_mem(LCL, 300);
        vm.set_mem(ARG, 400);
        vm.set_mem_indirect(ARG, 0, 3);

        for _ in 0..33 {
            vm.step();
        }

        assert_eq!(257, vm.mem(0));
        assert_eq!(6, vm.mem(256));
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

        vm.load(program);

        vm.set_mem(SP, 256);
        vm.set_mem(LCL, 300);
        vm.set_mem(ARG, 400);
        vm.set_mem_indirect(ARG, 0, 6);

        vm.set_mem_indirect(ARG, 1, 3000);
        for _ in 0..73 {
            vm.step();
        }

        assert_eq!(0, vm.mem(3000));
        assert_eq!(1, vm.mem(3001));
        assert_eq!(1, vm.mem(3002));
        assert_eq!(2, vm.mem(3003));
        assert_eq!(3, vm.mem(3004));
        assert_eq!(5, vm.mem(3005));
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

        vm.load(program);

        vm.set_mem(SP, 261);

        for _ in 0..110 {
            vm.step();
        }

        assert_eq!(262, vm.mem(0));
        assert_eq!(3, vm.mem(261));
    }

    #[test]
    fn nested_call() {
        let mut vm = VM::default();

        for i in 261..=299 {
            vm.set_mem(i, -1);
        }

        vm.set_mem(SP, 261);
        vm.set_mem(LCL, 261);
        vm.set_mem(ARG, 256);
        vm.set_mem(THIS, 3000);
        vm.set_mem(THAT, 4000);

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

        vm.load(program);

        vm.set_mem(0, 261);
        vm.set_mem(1, 261);
        vm.set_mem(2, 256);
        vm.set_mem(3, -3);
        vm.set_mem(4, -4);
        vm.set_mem(5, -1); // test results
        vm.set_mem(6, -1);
        vm.set_mem(256, 1234); // fake stack frame from call Sys.init
        vm.set_mem(257, -1);
        vm.set_mem(258, -2);
        vm.set_mem(259, -3);
        vm.set_mem(260, -4);

        for _ in 0..50 {
            vm.step();
        }

        assert_eq!(261, vm.mem(0));
        assert_eq!(261, vm.mem(1));
        assert_eq!(256, vm.mem(2));
        assert_eq!(4000, vm.mem(3));
        assert_eq!(5000, vm.mem(4));
        assert_eq!(135, vm.mem(5));
        assert_eq!(246, vm.mem(6));
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

        vm.load(program);

        vm.set_mem(SP, 317);
        vm.set_mem(LCL, 317);
        vm.set_mem(ARG, 310);
        vm.set_mem(THIS, 3000);
        vm.set_mem(THAT, 4000);
        vm.set_mem_indirect(ARG, 0, 1234);
        vm.set_mem_indirect(ARG, 1, 37);
        vm.set_mem_indirect(ARG, 2, 9);
        vm.set_mem_indirect(ARG, 3, 305);
        vm.set_mem_indirect(ARG, 4, 300);
        vm.set_mem_indirect(ARG, 5, 3010);
        vm.set_mem_indirect(ARG, 6, 4010);

        for _ in 0..10 {
            vm.step();
        }

        assert_eq!(311, vm.mem(0));
        assert_eq!(305, vm.mem(1));
        assert_eq!(300, vm.mem(2));
        assert_eq!(3010, vm.mem(3));
        assert_eq!(4010, vm.mem(4));
        assert_eq!(1196, vm.mem(310));
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

        vm.load(program);

        vm.set_mem(SP, 261);

        for _ in 0..36 {
            vm.step();
        }

        assert_eq!(263, vm.mem(0));
        assert_eq!(-2, vm.mem(261));
        assert_eq!(8, vm.mem(262));
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

        vm.load(program);

        for _ in 0..500000 {
            vm.step();
        }

        for i in SCREEN_START..KBD {
            assert_eq!(255, vm.mem(i));
        }
    }

    #[test]
    fn test_should_execute_stdlib_implementation() {
        let mut by_name = HashMap::new();
        let mut by_address = HashMap::new();

        by_name.insert("Math.abs", u16::MAX);
        by_address.insert(
            u16::MAX,
            BuiltinFunction::new(u16::MAX, "Math.abs", 1, &|_, _, params| {
                Ok(StdlibOk::Finished(params[0].abs()))
            }),
        );

        let stdlib = Stdlib::of(by_name, by_address);

        let mut vm = VM::new(stdlib.clone());

        let src = r#"
            function Lines.init 0
            push constant 0
            push constant 42
            sub
            call Math.abs 1
            label LOOP
            goto LOOP
            "#;

        let programs = vec![SourceFile::new("MathsTest.vm", src)];
        let mut bytecode_parser = Parser::with_stdlib(programs, stdlib);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..100 {
            vm.step();
        }

        assert_eq!(vm.pop(), 42);
    }

    #[test]
    fn test_should_execute_stdlib_implementation_multiple_args() {
        let stdlib = Stdlib::new();

        let mut vm = VM::new(stdlib.clone());

        let src = r#"
            function Main.main 0
            push constant 42
            push constant 3
            push constant 4
            call Math.multiply 2
            push constant 2
            call Math.multiply 2
            neg
            call Math.abs 1
            label LOOP
            goto LOOP
            "#;

        let programs = vec![SourceFile::new("MathsTest.vm", src)];
        let mut bytecode_parser = Parser::with_stdlib(programs, stdlib);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..20 {
            vm.step();
        }

        assert_eq!(vm.pop(), 24);
        assert_eq!(vm.pop(), 42);
    }

    #[test]
    fn test_calling_vm_from_builtin_function() {
        let mut by_name = HashMap::new();
        let mut by_address = HashMap::new();

        fn sys_init<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
            match state {
                0 => {
                    if VMCallOk::WasBuiltinFunction == vm.call("Memory.init", &[])? {
                        // continue immediately
                        sys_init(vm, state + 1, params)
                    } else {
                        Ok(StdlibOk::ContinueInNextStep(state + 1))
                    }
                }
                1 => {
                    if VMCallOk::WasBuiltinFunction == vm.call("Main.main", &[])? {
                        // continue immediately
                        sys_init(vm, state + 1, params)
                    } else {
                        Ok(StdlibOk::ContinueInNextStep(state + 1))
                    }
                }
                _ => Ok(StdlibOk::ContinueInNextStep(state)),
            }
        }

        by_name.insert("Sys.init", u16::MAX - 1);
        by_address.insert(
            u16::MAX - 1,
            BuiltinFunction::new(u16::MAX - 1, "Sys.init", 0, &sys_init),
        );

        fn mem_init<VM: VirtualMachine>(_: &mut VM, _: State, _: &[Word]) -> StdResult {
            Ok(StdlibOk::Finished(20))
        }

        by_name.insert("Memory.init", u16::MAX);
        by_address.insert(
            u16::MAX,
            BuiltinFunction::new(u16::MAX, "Memory.init", 0, &mem_init),
        );

        let stdlib = Stdlib::of(by_name, by_address);

        let mut vm = VM::new(stdlib.clone());

        let src = r#"
            function Main.main 0
            call Memory.init 0
            push constant 22
            add // the 20 should have been pushed by the Memory.init call
            return // return to Sys.init
            "#;

        let programs = vec![SourceFile::new("Main.vm", src)];
        let mut bytecode_parser = Parser::with_stdlib(programs, stdlib);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..20 {
            vm.step();
        }

        assert_eq!(vm.pop(), 42);
    }

    #[test]
    fn test_continuing_parked_stdlib_function() {
        let mut by_name = HashMap::new();
        let mut by_address = HashMap::new();

        fn sys_wait<VM: VirtualMachine>(_: &mut VM, state: State, params: &[Word]) -> StdResult {
            if state == 0 {
                if params[0] < 2 {
                    return Ok(StdlibOk::Finished(params[0]));
                }
                // 2 because one tick is already used
                return Ok(StdlibOk::ContinueInNextStep(2));
            }

            if params[0] as State > state {
                return Ok(StdlibOk::ContinueInNextStep(state + 1));
            }

            Ok(StdlibOk::Finished(params[0]))
        }

        fn sys_init<VM: VirtualMachine>(vm: &mut VM, state: State, _: &[Word]) -> StdResult {
            if state == 0 {
                vm.call("Main.main", &[])?;
                return Ok(StdlibOk::ContinueInNextStep(state + 1));
            }
            Ok(StdlibOk::ContinueInNextStep(state))
        }

        by_name.insert("Sys.init", u16::MAX - 1);
        by_address.insert(
            u16::MAX - 1,
            BuiltinFunction::new(u16::MAX - 1, "Sys.init", 0, &sys_init),
        );

        by_name.insert("Sys.wait", u16::MAX);
        by_address.insert(
            u16::MAX,
            BuiltinFunction::new(u16::MAX, "Sys.wait", 1, &sys_wait),
        );

        let stdlib = Stdlib::of(by_name, by_address);

        let mut vm = VM::new(stdlib.clone());

        let src = r#"
            function Main.getReturnValue 0
            push constant 42
            return

            function Main.main 0
            push constant 10 // wait 10 ticks
            call Sys.wait 1
            push constant 2
            call Sys.wait 1 // wait 2 more ticks
            call Main.getReturnValue 0
            return // return to Sys.init
            "#;

        let programs = vec![SourceFile::new("Main.vm", src)];
        let mut bytecode_parser = Parser::with_stdlib(programs, stdlib);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..100 {
            vm.step();
        }

        assert_eq!(vm.pop(), 42);
    }

    #[test]
    fn test_use_return_value_of_vm_function_in_builtin_function() {
        let mut by_name = HashMap::new();
        let mut by_address = HashMap::new();

        fn calc<VM: VirtualMachine>(vm: &mut VM, state: State, _params: &[Word]) -> StdResult {
            match state {
                0 => {
                    // technically this is wrong because the return value isn't incremented, but Main.f
                    // will always be a VM function, so it's no problem
                    stdlib::call_vm!(vm, state, "Main.f", &[])
                }
                1 => {
                    let vm_ret = vm.pop();
                    Ok(StdlibOk::Finished(vm_ret + 1))
                }
                _ => panic!(""),
            }
        }

        by_name.insert("Math.calc", u16::MAX - 1);
        by_address.insert(
            u16::MAX - 1,
            BuiltinFunction::new(u16::MAX - 1, "Math.calc", 0, &calc),
        );

        let stdlib = Stdlib::of(by_name, by_address);

        let mut vm = VM::new(stdlib.clone());

        let src = r#"
            function Main.main 0
            call Math.calc 0
            call Main.f 0
            push constant 1
            sub
            call Math.calc 0
            call Math.calc 0
            label END
            goto END

            function Main.f 0
            push constant 41
            return
            "#;

        let programs = vec![SourceFile::new("Main.vm", src)];
        let mut bytecode_parser = Parser::with_stdlib(programs, stdlib);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..30 {
            vm.step();
        }

        assert_eq!(vm.pop(), 42);
        assert_eq!(vm.pop(), 42);
        assert_eq!(vm.pop(), 40);
        assert_eq!(vm.pop(), 42);
    }

    #[test]
    fn test_calling_vm_from_builtin_function_multiple_times() {
        let mut by_name = HashMap::new();
        let mut by_address = HashMap::new();

        fn sys_init<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
            match state {
                0 => {
                    if let VMCallOk::WasBuiltinFunction = vm.call("Memory.init", &[])? {
                        // continue immediately
                        sys_init(vm, state + 1, params)
                    } else {
                        Ok(StdlibOk::ContinueInNextStep(state + 1))
                    }
                }
                1 => {
                    if let VMCallOk::WasBuiltinFunction = vm.call("Main.main", &[])? {
                        // continue immediately
                        sys_init(vm, state + 1, params)
                    } else {
                        Ok(StdlibOk::ContinueInNextStep(state + 1))
                    }
                }
                // endless loop
                _ => Ok(StdlibOk::ContinueInNextStep(state)),
            }
        }

        by_name.insert("Sys.init", u16::MAX);
        by_address.insert(
            u16::MAX,
            BuiltinFunction::new(u16::MAX, "Sys.init", 0, &sys_init),
        );

        let stdlib = Stdlib::of(by_name, by_address);

        let mut vm = VM::new(stdlib.clone());

        let src = r#"
            function Memory.init 0
            push constant 0
            pop static 0
            push constant 2048
            push static 0
            add
            push constant 14334
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 2049
            push static 0
            add
            push constant 2050
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            return

            function Main.main 0
            call Memory.init 0
            push constant 22
            add
            return // return to Sys.init
            "#;

        let programs = vec![SourceFile::new("Main.vm", src)];
        let mut bytecode_parser = Parser::with_stdlib(programs, stdlib);
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..300 {
            vm.step();
        }

        assert_eq!(vm.pop(), 22);
    }
}

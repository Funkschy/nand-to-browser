mod os_array;
mod os_keyboard;
mod os_math;
mod os_memory;
mod os_output;
mod os_screen;
mod os_string;
mod os_sys;

use crate::definitions::{Address, Symbol, Word};
use std::collections::HashMap;
use std::fmt;

macro_rules! call_vm {
    ($vm:ident, $state:ident, $fn:expr, $args:expr) => {
        if let VMCallOk::WasBuiltinFunction = $vm.call($fn, $args)? {
            // TODO: error handling
            let ret = $vm.pop();
            // function was a builtin continue immediately
            Ok(StdlibOk::Finished(ret))
        } else {
            // function was VM bytecode, so this needs to be continued after the VM function returned
            Ok(StdlibOk::ContinueInNextStep($state + 1))
        }
    };
}

// for unit tests in vm/mod.rs
#[cfg(test)]
pub(crate) use call_vm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMCallOk {
    WasVMFunction,
    WasBuiltinFunction,
}

pub trait VirtualMachine {
    fn mem(&self, address: Address) -> Word;
    fn set_mem(&mut self, address: Address, value: Word);

    fn pop(&mut self) -> Word;

    fn call(&mut self, name: &str, params: &[Word]) -> Result<VMCallOk, StdlibError>;
}

pub type State = usize;

pub enum StdlibOk {
    // The function finished successfully. The Word is the return value
    Finished(Word),
    // The function needs to be run again. The State is the functions internal state.
    // This state must be passed to the next invocation so that the function can continue.
    // Being able to resume the function is needed for functions like Sys.wait because we can't
    // just block the thread in a Web application, so instead we just wait a certain number of ticks
    ContinueInNextStep(State),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdlibError {
    IncorrectNumberOfArgs,
    CallingNonExistendFunction,
    NoReturnValueFromStdlibFunction,

    MathDivideByZero,
    MathNegativeSqrt,

    MemoryAllocNonPositiveSize,
    MemoryHeapOverflow,
}

pub type StdResult = Result<StdlibOk, StdlibError>;

pub struct BuiltinFunction<'f, VM: VirtualMachine> {
    // the fake address which is used to jump to this builtin function
    // Calls in the vm do not work via the function name and instead use the functions address in
    // the bytecode for better performance. This is of course a bit of an issue with functions that
    // aren't actually in the bytecode
    virtual_address: Symbol,
    name: &'static str,
    num_args: usize,
    function: &'f dyn Fn(&mut VM, State, &[Word]) -> StdResult,
}

impl<'f, VM: VirtualMachine> fmt::Debug for BuiltinFunction<'f, VM> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// for some weird reason, the derive implementations weren't recognized by the compiler
impl<'f, VM: VirtualMachine> Clone for BuiltinFunction<'f, VM> {
    fn clone(&self) -> Self {
        Self {
            virtual_address: self.virtual_address,
            name: self.name,
            num_args: self.num_args,
            function: self.function,
        }
    }
}

impl<'f, VM: VirtualMachine> std::marker::Copy for BuiltinFunction<'f, VM> {}

impl<'f, VM: VirtualMachine> BuiltinFunction<'f, VM> {
    pub fn new(
        virtual_address: Symbol,
        name: &'static str,
        num_args: usize,
        function: &'f dyn Fn(&mut VM, State, &[Word]) -> StdResult,
    ) -> Self {
        Self {
            virtual_address,
            name,
            num_args,
            function,
        }
    }

    pub fn call(&self, vm: &mut VM, args: &[Word]) -> StdResult {
        self.continue_call(vm, State::default(), args)
    }

    pub fn continue_call(&self, vm: &mut VM, state: State, args: &[Word]) -> StdResult {
        if args.len() != self.num_args {
            return Err(StdlibError::IncorrectNumberOfArgs);
        }

        let f = self.function;
        f(vm, state, args)
    }

    pub fn num_args(&self) -> usize {
        self.num_args
    }

    pub fn virtual_address(&self) -> Symbol {
        self.virtual_address
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[derive(Default)]
pub struct Stdlib<'f, VM: VirtualMachine> {
    by_name: HashMap<&'static str, Symbol>,
    by_address: HashMap<Symbol, BuiltinFunction<'f, VM>>,
}

// for some weird reason, the derive implementations weren't recognized by the compiler
impl<'f, VM: VirtualMachine> Clone for Stdlib<'f, VM> {
    fn clone(&self) -> Self {
        Self {
            by_name: self.by_name.clone(),
            by_address: self.by_address.clone(),
        }
    }
}

impl<'f, VM: VirtualMachine> Stdlib<'f, VM> {
    pub fn new() -> Self {
        let (by_name, by_address) = stdlib();

        Self {
            by_name,
            by_address,
        }
    }

    pub fn of(
        by_name: HashMap<&'static str, Symbol>,
        by_address: HashMap<Symbol, BuiltinFunction<'f, VM>>,
    ) -> Self {
        Self {
            by_name,
            by_address,
        }
    }

    pub fn by_address(&self, function: Symbol) -> Option<&BuiltinFunction<'f, VM>> {
        self.by_address.get(&function)
    }

    pub fn lookup<'s>(&self, ident: impl Into<&'s str>) -> Option<&BuiltinFunction<'f, VM>> {
        self.by_name
            .get(ident.into())
            .and_then(|&address| self.by_address(address))
    }

    pub fn len(&self) -> usize {
        self.by_address.len()
    }

    pub fn by_name(&self) -> &HashMap<&'static str, Symbol> {
        &self.by_name
    }
}

fn stdlib<'f, VM: VirtualMachine>() -> (
    HashMap<&'static str, Symbol>,
    HashMap<Symbol, BuiltinFunction<'f, VM>>,
) {
    const NUMBER_OF_STDLIB_FUNCTIONS: usize = 49;

    let virtual_function_offset = u16::MAX - (NUMBER_OF_STDLIB_FUNCTIONS as u16 - 1);

    let mut by_name = HashMap::with_capacity(NUMBER_OF_STDLIB_FUNCTIONS);
    let mut by_address = HashMap::with_capacity(NUMBER_OF_STDLIB_FUNCTIONS);

    let mut def = |name, n_args, f| {
        let address = virtual_function_offset + by_address.len() as u16;
        let function = BuiltinFunction::new(address, name, n_args, f);
        by_address.insert(address, function);
        by_name.insert(name, address);
    };

    // Math
    {
        use os_math::{abs, divide, init, max, min, multiply, sqrt};
        def("Math.init", 0, &init);
        def("Math.abs", 1, &abs);
        def("Math.multiply", 2, &multiply);
        def("Math.divide", 2, &divide);
        def("Math.min", 2, &min);
        def("Math.max", 2, &max);
        def("Math.sqrt", 1, &sqrt);
    }

    // String
    {
        use os_string::{
            append_char, backspace, char_at, dispose, double_quote, erase_last_char, int_value,
            length, new, newline, set_char_at, set_int,
        };
        def("String.new", 1, &new);
        def("String.dispose", 1, &dispose);
        def("String.length", 1, &length);
        def("String.charAt", 2, &char_at);
        def("String.setCharAt", 3, &set_char_at);
        def("String.appendChar", 2, &append_char);
        def("String.eraseLastChar", 1, &erase_last_char);
        def("String.intValue", 1, &int_value);
        def("String.setInt", 2, &set_int);
        def("String.backSpace", 0, &backspace);
        def("String.doubleQuote", 0, &double_quote);
        def("String.newLine", 0, &newline);
    }

    // Array
    {
        use os_array::{dispose, new};
        def("Array.new", 1, &new);
        def("Array.dispose", 1, &dispose);
    }

    // Output
    {
        use os_output::{
            backspace, init, move_cursor, print_char, print_int, print_string, println,
        };
        def("Output.init", 0, &init);
        def("Output.moveCursor", 2, &move_cursor);
        def("Output.printChar", 1, &print_char);
        def("Output.printString", 1, &print_string);
        def("Output.printInt", 1, &print_int);
        def("Output.println", 0, &println);
        def("Output.backSpace", 0, &backspace);
    }

    // Screen
    {
        use os_screen::{
            clear_screen, draw_circle, draw_line, draw_pixel, draw_rectangle, init, set_color,
        };
        def("Screen.init", 0, &init);
        def("Screen.clearScreen", 0, &clear_screen);
        def("Screen.setColor", 1, &set_color);
        def("Screen.drawPixel", 2, &draw_pixel);
        def("Screen.drawLine", 4, &draw_line);
        def("Screen.drawRectangle", 4, &draw_rectangle);
        def("Screen.drawCircle", 3, &draw_circle);
    }

    // Keyboard
    {
        use os_keyboard::{init, key_pressed, read_char, read_int, read_line};
        def("Keyboard.init", 0, &init);
        def("Keyboard.keyPressed", 0, &key_pressed);
        def("Keyboard.readChar", 0, &read_char);
        def("Keyboard.readLine", 1, &read_line);
        def("Keyboard.readInt", 1, &read_int);
    }

    // Memory
    {
        use os_memory::{alloc, de_alloc, init, peek, poke};
        def("Memory.init", 0, &init);
        def("Memory.peek", 1, &peek);
        def("Memory.poke", 2, &poke);
        def("Memory.alloc", 1, &alloc);
        def("Memory.deAlloc", 1, &de_alloc);
    }

    // Sys
    {
        use os_sys::{error, halt, init, wait};
        def("Sys.init", 0, &init);
        def("Sys.halt", 0, &halt);
        def("Sys.error", 1, &error);
        def("Sys.wait", 1, &wait);
    }

    (by_name, by_address)
}

mod error;
mod os_array;
mod os_keyboard;
mod os_math;
mod os_memory;
// wasm does not support atomics at this moment, so the warning about using atomics instead of
// mutexes is meaningless
#[allow(clippy::mutex_atomic)]
mod os_output;
#[allow(clippy::mutex_atomic)]
mod os_screen;
mod os_string;
mod os_sys;

use crate::definitions::{Address, Symbol, Word};
use crate::simulators::vm::VM;
pub use error::StdlibError;
use std::collections::HashMap;
use std::fmt;

macro_rules! call_vm {
    ($vm:ident, $state:ident, $fn:expr, $args:expr) => {{
        $vm.call($fn, $args)?;
        Ok(StdlibOk::ContinueInNextStep($state + 1))
    }};
}

macro_rules! set_mutex {
    ($mutex:ident, $value:expr, $error:expr) => {
        *$mutex.lock().map_err(|_| $error)? = $value;
    };
}

macro_rules! get_mutex {
    ($mutex:ident, $error:expr) => {
        *$mutex.lock().map_err(|_| $error)?
    };
}

// make this public in the entire crate for unit tests in vm/mod.rs
pub(crate) use call_vm;
use get_mutex;
use set_mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMCallOk {
    WasVMFunction,
    WasBuiltinFunction,
}

pub type State = u32;

#[derive(Debug)]
pub enum StdlibOk {
    // The function finished successfully. The Word is the return value
    Finished(Word),
    // The function needs to be run again. The State is the functions internal state.
    // This state must be passed to the next invocation so that the function can continue.
    // Being able to resume the function is needed for functions like Sys.wait because we can't
    // just block the thread in a Web application, so instead we just wait a certain number of ticks
    ContinueInNextStep(State),
}

pub type StdResult = Result<StdlibOk, StdlibError>;

#[derive(Clone, Copy)]
pub struct BuiltinFunction {
    // the fake address which is used to jump to this builtin function
    // Calls in the vm do not work via the function name and instead use the functions address in
    // the bytecode for better performance. This is of course a bit of an issue with functions that
    // aren't actually in the bytecode
    virtual_address: Symbol,
    name: &'static str,
    file: &'static str,
    num_args: usize,
    function: &'static dyn Fn(&mut VM, State, &[Word]) -> StdResult,
}

impl fmt::Debug for BuiltinFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl BuiltinFunction {
    pub fn new(
        virtual_address: Symbol,
        name: &'static str,
        file: &'static str,
        num_args: usize,
        function: &'static dyn Fn(&mut VM, State, &[Word]) -> StdResult,
    ) -> Self {
        Self {
            virtual_address,
            name,
            file,
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

    pub fn file(&self) -> &'static str {
        self.file
    }
}

#[derive(Default)]
pub struct Stdlib {
    by_name: HashMap<&'static str, Symbol>,
    by_address: HashMap<Symbol, BuiltinFunction>,
}

// for some weird reason, the derive implementations weren't recognized by the compiler
impl Clone for Stdlib {
    fn clone(&self) -> Self {
        Self {
            by_name: self.by_name.clone(),
            by_address: self.by_address.clone(),
        }
    }
}

impl Stdlib {
    pub fn new() -> Self {
        let (by_name, by_address) = stdlib();

        Self {
            by_name,
            by_address,
        }
    }

    pub fn by_address(&self, function: Symbol) -> Option<&BuiltinFunction> {
        self.by_address.get(&function)
    }

    pub fn lookup<'s>(&self, ident: impl Into<&'s str>) -> Option<&BuiltinFunction> {
        self.by_name
            .get(ident.into())
            .and_then(|&address| self.by_address(address))
    }

    pub fn by_name(&self) -> &HashMap<&'static str, Symbol> {
        &self.by_name
    }
}

#[cfg(test)]
impl Stdlib {
    pub fn len(&self) -> usize {
        self.by_address.len()
    }

    pub fn of(
        by_name: HashMap<&'static str, Symbol>,
        by_address: HashMap<Symbol, BuiltinFunction>,
    ) -> Self {
        Self {
            by_name,
            by_address,
        }
    }
}

fn stdlib() -> (
    HashMap<&'static str, Symbol>,
    HashMap<Symbol, BuiltinFunction>,
) {
    const NUMBER_OF_STDLIB_FUNCTIONS: usize = 49;

    let virtual_function_offset = u16::MAX - (NUMBER_OF_STDLIB_FUNCTIONS as u16 - 1);

    let mut by_name = HashMap::with_capacity(NUMBER_OF_STDLIB_FUNCTIONS);
    let mut by_address = HashMap::with_capacity(NUMBER_OF_STDLIB_FUNCTIONS);

    let mut def = |file, name, n_args, f| {
        let address = virtual_function_offset + by_address.len() as u16;
        let function = BuiltinFunction::new(address, name, file, n_args, f);
        by_address.insert(address, function);
        by_name.insert(name, address);
    };

    // Math
    {
        use os_math::{abs, divide, init, max, min, multiply, sqrt};
        def("Math", "Math.init", 0, &init);
        def("Math", "Math.abs", 1, &abs);
        def("Math", "Math.multiply", 2, &multiply);
        def("Math", "Math.divide", 2, &divide);
        def("Math", "Math.min", 2, &min);
        def("Math", "Math.max", 2, &max);
        def("Math", "Math.sqrt", 1, &sqrt);
    }

    // String
    {
        use os_string::{
            append_char, backspace, char_at, dispose, double_quote, erase_last_char, int_value,
            length, new, newline, set_char_at, set_int,
        };
        def("String", "String.new", 1, &new);
        def("String", "String.dispose", 1, &dispose);
        def("String", "String.length", 1, &length);
        def("String", "String.charAt", 2, &char_at);
        def("String", "String.setCharAt", 3, &set_char_at);
        def("String", "String.appendChar", 2, &append_char);
        def("String", "String.eraseLastChar", 1, &erase_last_char);
        def("String", "String.intValue", 1, &int_value);
        def("String", "String.setInt", 2, &set_int);
        def("String", "String.backSpace", 0, &backspace);
        def("String", "String.doubleQuote", 0, &double_quote);
        def("String", "String.newLine", 0, &newline);
    }

    // Array
    {
        use os_array::{dispose, new};
        def("Array", "Array.new", 1, &new);
        def("Array", "Array.dispose", 1, &dispose);
    }

    // Output
    {
        use os_output::{
            backspace, init, move_cursor, print_char, print_int, print_string, println,
        };
        def("Output", "Output.init", 0, &init);
        def("Output", "Output.moveCursor", 2, &move_cursor);
        def("Output", "Output.printChar", 1, &print_char);
        def("Output", "Output.printString", 1, &print_string);
        def("Output", "Output.printInt", 1, &print_int);
        def("Output", "Output.println", 0, &println);
        def("Output", "Output.backSpace", 0, &backspace);
    }

    // Screen
    {
        use os_screen::{
            clear_screen, draw_circle, draw_line, draw_pixel, draw_rectangle, init, set_color,
        };
        def("Screen", "Screen.init", 0, &init);
        def("Screen", "Screen.clearScreen", 0, &clear_screen);
        def("Screen", "Screen.setColor", 1, &set_color);
        def("Screen", "Screen.drawPixel", 2, &draw_pixel);
        def("Screen", "Screen.drawLine", 4, &draw_line);
        def("Screen", "Screen.drawRectangle", 4, &draw_rectangle);
        def("Screen", "Screen.drawCircle", 3, &draw_circle);
    }

    // Keyboard
    {
        use os_keyboard::{init, key_pressed, read_char, read_int, read_line};
        def("Keyboard", "Keyboard.init", 0, &init);
        def("Keyboard", "Keyboard.keyPressed", 0, &key_pressed);
        def("Keyboard", "Keyboard.readChar", 0, &read_char);
        def("Keyboard", "Keyboard.readLine", 1, &read_line);
        def("Keyboard", "Keyboard.readInt", 1, &read_int);
    }

    // Memory
    {
        use os_memory::{alloc, de_alloc, init, peek, poke};
        def("Memory", "Memory.init", 0, &init);
        def("Memory", "Memory.peek", 1, &peek);
        def("Memory", "Memory.poke", 2, &poke);
        def("Memory", "Memory.alloc", 1, &alloc);
        def("Memory", "Memory.deAlloc", 1, &de_alloc);
    }

    // Sys
    {
        use os_sys::{error, halt, init, wait};
        def("Sys", "Sys.init", 0, &init);
        def("Sys", "Sys.halt", 0, &halt);
        def("Sys", "Sys.error", 1, &error);
        def("Sys", "Sys.wait", 1, &wait);
    }

    (by_name, by_address)
}

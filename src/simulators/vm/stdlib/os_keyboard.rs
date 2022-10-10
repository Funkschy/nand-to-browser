use super::*;
use crate::definitions::{BACKSPACE_KEY, KBD, NEWLINE_KEY};

pub fn init<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(0))
}

pub fn key_pressed<VM: VirtualMachine>(vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(vm.mem(KBD)))
}

pub fn read_char<VM: VirtualMachine>(vm: &mut VM, state: State, _params: &[Word]) -> StdResult {
    match state {
        0 => {
            call_vm!(vm, state, "Output.printChar", &[0])
        }
        1 => {
            let key = vm.mem(KBD);
            // stay in this state until the key state was 0 (don't allow holding the key)
            if key != 0 {
                Ok(StdlibOk::ContinueInNextStep(state))
            } else {
                Ok(StdlibOk::ContinueInNextStep(state + 1))
            }
        }
        2 => {
            let key = vm.mem(KBD);
            // stay in this state until the user presses a key
            if key == 0 {
                Ok(StdlibOk::ContinueInNextStep(state))
            } else {
                // alternatively we could keep the key inside of state, similar to the way
                // Output.printString does it
                vm.push(key);
                vm.push(key);
                Ok(StdlibOk::ContinueInNextStep(state + 1))
            }
        }
        3 => {
            call_vm!(vm, state, "Output.printChar", &[BACKSPACE_KEY])
        }
        4 => {
            vm.pop(); // printChar backspace result
            let key = vm.pop();
            call_vm!(vm, state, "Output.printChar", &[key])
        }
        _ => {
            vm.pop(); // printChar key result
            let key = vm.pop();
            vm.pop(); // printChar 0 result
            Ok(StdlibOk::Finished(key))
        }
    }
}

pub fn read_line<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    // use the upper 16 bits for the string address and the lower 16 bits for the actual state
    let string_s = (state >> 16) & 0xFFFF;
    let line = string_s as Word;
    let state = state & 0xFFFF;

    match state {
        0 => {
            let message = params[0];
            call_vm!(vm, state, "Output.printString", &[message])
        }
        1 => {
            vm.pop(); // printChar 0
            let max_line_length = 80; // same as in the official Keyboard.vm code
            call_vm!(vm, state, "String.new", &[max_line_length])
        }
        2 => {
            call_vm!(vm, state, "Keyboard.readChar", &[])
        }
        3 => {
            let c = vm.pop();
            let string = vm.pop() as State;
            vm.push(c);
            Ok(StdlibOk::ContinueInNextStep((string << 16) | 4))
        }
        4 => {
            let c = vm.pop();

            match c {
                NEWLINE_KEY => Ok(StdlibOk::Finished(line)),
                BACKSPACE_KEY => {
                    vm.call("String.eraseLastChar", &[line])?;
                    Ok(StdlibOk::ContinueInNextStep((string_s << 16) | 5))
                }
                _ => {
                    vm.call("String.appendChar", &[line, c])?;
                    Ok(StdlibOk::ContinueInNextStep((string_s << 16) | 5))
                }
            }
        }
        5 => {
            vm.pop(); // String.eraseLastChar or String.appendChar
            vm.call("Keyboard.readChar", &[])?;
            Ok(StdlibOk::ContinueInNextStep((string_s << 16) | 4))
        }
        _ => unreachable!("reached unreachable state in read_line {}", state),
    }
}

pub fn read_int<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    match state {
        0 => call_vm!(vm, state, "Keyboard.readLine", params),
        1 => {
            let line = vm.pop();
            call_vm!(vm, state, "String.intValue", &[line])
        }
        _ => {
            let int = vm.pop();
            Ok(StdlibOk::Finished(int))
        }
    }
}

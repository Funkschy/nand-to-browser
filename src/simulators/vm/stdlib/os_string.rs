use super::*;
use crate::definitions::{BACKSPACE_KEY, NEWLINE_KEY};

pub fn new<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    let max_len = params[0];
    if max_len < 0 {
        return Err(StdlibError::StringNewNegativeLength);
    }

    match state {
        0 => {
            call_vm!(vm, state, "Memory.alloc", &[max_len + 2])
        }
        1 => {
            let addr = vm.pop();
            vm.set_mem(addr as Address, max_len);
            vm.set_mem(addr as Address + 1, 0);
            Ok(StdlibOk::Finished(addr))
        }
        _ => Err(StdlibError::ContinuingFinishedFunction),
    }
}

pub fn dispose<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    if state == 0 {
        call_vm!(vm, state, "Memory.deAlloc", params)
    } else {
        Ok(StdlibOk::Finished(0))
    }
}

pub fn length<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let string = params[0] as Address;
    Ok(StdlibOk::Finished(vm.mem(string + 1)))
}

pub fn char_at<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let string = params[0] as Address;
    let pos = params[1];

    let len = vm.mem(string + 1);
    if pos < 0 || pos >= len {
        return Err(StdlibError::StringCharAtIllegalIndex);
    }

    Ok(StdlibOk::Finished(vm.mem(string + 2 + pos as Address)))
}

pub fn set_char_at<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let string = params[0] as Address;
    let pos = params[1];
    let c = params[2];

    let len = vm.mem(string + 1);
    if pos < 0 || pos >= len {
        return Err(StdlibError::StringSetCharAtIllegalIndex);
    }

    vm.set_mem(string + 2 + pos as Address, c);
    Ok(StdlibOk::Finished(0))
}

pub fn append_char<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let string = params[0] as Address;
    let c = params[1];

    let cap = vm.mem(string);
    let len = vm.mem(string + 1);
    if len >= cap {
        return Err(StdlibError::StringAppendCharFull);
    }

    vm.set_mem(string + 2 + len as Address, c);
    vm.set_mem(string + 1, len + 1);
    Ok(StdlibOk::Finished(params[0]))
}

pub fn erase_last_char<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let string = params[0] as Address;
    let len = vm.mem(string + 1);
    if len == 0 {
        return Err(StdlibError::StringEraseLastCharEmtpy);
    }

    vm.set_mem(string + 1, len - 1);
    Ok(StdlibOk::Finished(0))
}

pub fn int_value<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let string = params[0] as Address;
    let len = vm.mem(string + 1) as Address;

    let zero = '0' as Word;
    let nine = '9' as Word;

    let mut neg = false;
    let mut value = 0;

    let mut i = 0;

    if vm.mem(string + 2) as u8 as char == '-' {
        neg = true;
        i += 1;
    }

    for i in i..len {
        let c = vm.mem(string + 2 + i);
        if !(zero..=nine).contains(&c) {
            break;
        }
        value = value * 10 + (c - zero);
    }

    if neg {
        Ok(StdlibOk::Finished(-value))
    } else {
        Ok(StdlibOk::Finished(value))
    }
}

pub fn set_int<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let string = params[0] as Address;
    let int = params[1].to_string();
    let int = int.as_bytes();
    let l = int.len() as Word;
    let cap = vm.mem(string);
    if cap < l {
        return Err(StdlibError::StringSetIntInsufficientCapacity);
    }
    vm.set_mem(string + 1, l);
    for (i, &int_char) in int.iter().enumerate() {
        vm.set_mem(string + 2 + i, int_char as Word);
    }

    Ok(StdlibOk::Finished(0))
}

pub fn backspace<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(BACKSPACE_KEY))
}

pub fn double_quote<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished('"' as Word))
}

pub fn newline<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(NEWLINE_KEY))
}

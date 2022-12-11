use super::*;
use crate::simulators::vm::VM;

pub fn init(vm: &mut VM, state: State, _params: &[Word]) -> StdResult {
    match state {
        0 => {
            call_vm!(vm, state, "Memory.init", &[])
        }
        1 => {
            vm.pop()?;
            call_vm!(vm, state, "Math.init", &[])
        }
        2 => {
            vm.pop()?;
            call_vm!(vm, state, "Screen.init", &[])
        }
        3 => {
            vm.pop()?;
            call_vm!(vm, state, "Output.init", &[])
        }
        4 => {
            vm.pop()?;
            call_vm!(vm, state, "Keyboard.init", &[])
        }
        5 => {
            vm.pop()?;
            call_vm!(vm, state, "Main.main", &[])
        }
        _ => {
            vm.pop()?;
            vm.call("Sys.halt", &[])?;
            Ok(StdlibOk::Finished(0))
        }
    }
}

pub fn halt(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Err(StdlibError::Halt)
}

pub fn error(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    Err(StdlibError::SysError(params[0]))
}

pub fn wait(_vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    if params[0] < 0 {
        return Err(StdlibError::SysWaitNegativeDuration);
    }

    // unfortunately this cannot actually be implemented correctly in wasm because wasm currently
    // offers no way to get the current system time. Therefore we can only use ticks as an estimate
    let duration = params[0] as State * 1000;
    if state < duration {
        return Ok(StdlibOk::ContinueInNextStep(state + 1));
    }

    Ok(StdlibOk::Finished(params[0]))
}

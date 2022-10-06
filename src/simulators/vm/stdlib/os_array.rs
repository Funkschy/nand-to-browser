use super::*;

pub fn new<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    if params[0] <= 0 {
        return Err(StdlibError::ArrayNewNonPositiveSize);
    }

    match state {
        0 => {
            call_vm!(vm, state, "Memory.alloc", params)
        }
        1 => {
            let addr = vm.pop();
            Ok(StdlibOk::Finished(addr))
        }
        _ => Err(StdlibError::ContinuingFinishedFunction),
    }
}

pub fn dispose<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    match state {
        0 => {
            call_vm!(vm, state, "Memory.deAlloc", params)
        }
        1 => {
            let addr = vm.pop();
            Ok(StdlibOk::Finished(addr))
        }
        _ => Err(StdlibError::ContinuingFinishedFunction),
    }
}

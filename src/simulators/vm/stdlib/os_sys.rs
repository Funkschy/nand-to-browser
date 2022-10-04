use super::*;

pub fn init<VM: VirtualMachine>(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    println!("Sys.init");
    match state {
        0 => {
            if VMCallOk::WasBuiltinFunction == vm.call("Memory.init", vec![])? {
                // continue immediately
                init(vm, state + 1, params)
            } else {
                Ok(StdlibOk::ContinueInNextStep(state + 1))
            }
        }
        1 => {
            if VMCallOk::WasBuiltinFunction == vm.call("Main.main", vec![])? {
                // continue immediately
                init(vm, state + 1, params)
            } else {
                Ok(StdlibOk::ContinueInNextStep(state + 1))
            }
        }
        _ => Ok(StdlibOk::Finished(0)),
    }
}

pub fn halt<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(0))
}

pub fn error<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(0))
}

pub fn wait<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(0))
}

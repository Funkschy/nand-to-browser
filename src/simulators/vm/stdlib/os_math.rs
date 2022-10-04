use super::*;

pub fn init<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(0))
}

pub fn abs<VM: VirtualMachine>(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(params[0].abs()))
}

pub fn multiply<VM: VirtualMachine>(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    // java doesn't handle overflows for ints, so this casting is needed for compatibility
    Ok(StdlibOk::Finished(
        (params[0] as i32 * params[1] as i32) as i16,
    ))
}

pub fn divide<VM: VirtualMachine>(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn min<VM: VirtualMachine>(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn max<VM: VirtualMachine>(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn sqrt<VM: VirtualMachine>(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    unimplemented!()
}

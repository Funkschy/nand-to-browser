use super::*;

pub fn init<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(0))
}

pub fn key_pressed<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn read_char<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn read_line<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn read_int<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    unimplemented!()
}

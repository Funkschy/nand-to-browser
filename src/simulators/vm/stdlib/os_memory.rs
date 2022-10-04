use super::*;

pub fn init<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    println!("memory init");
    Ok(StdlibOk::Finished(0))
}

pub fn peek<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn poke<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn alloc<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    unimplemented!()
}

pub fn de_alloc<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    unimplemented!()
}

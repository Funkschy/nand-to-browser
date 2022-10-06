use super::*;

use crate::definitions::{HEAP_END, HEAP_START};

pub fn init<VM: VirtualMachine>(vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    vm.set_mem(HEAP_START, ((HEAP_END + 1) - (HEAP_START + 2)) as Word);
    vm.set_mem(HEAP_START + 1, HEAP_END as Word + 1);

    Ok(StdlibOk::Finished(0))
}

pub fn peek<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(vm.mem(params[0] as Address)))
}

pub fn poke<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    vm.set_mem(params[0] as Address, params[1]);
    Ok(StdlibOk::Finished(0))
}

pub fn alloc<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let size = params[0] as usize;
    if size < 1 {
        return Err(StdlibError::MemoryAllocNonPositiveSize);
    }

    let mut seg_addr = HEAP_START;
    let mut seg_cap = 0;
    while seg_addr <= HEAP_END {
        seg_cap = vm.mem(seg_addr) as usize;
        if seg_cap >= size {
            break;
        }
        seg_addr = vm.mem(seg_addr + 1) as usize;
    }

    if seg_addr > HEAP_END {
        return Err(StdlibError::MemoryHeapOverflow);
    }

    if seg_cap > size + 2 {
        vm.set_mem(seg_addr + size + 2, (seg_cap - size - 2) as Word);
        vm.set_mem(seg_addr + size + 3, vm.mem(seg_addr + 1));
        vm.set_mem(seg_addr + 1, (seg_addr + size + 2) as Word);
    }

    vm.set_mem(seg_addr, 0);
    Ok(StdlibOk::Finished(seg_addr as Word + 2))
}

pub fn de_alloc<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let arr = params[0] as usize;
    let seg_addr = arr - 2;
    let next_seg_addr = vm.mem(seg_addr + 1) as usize;

    let next_cap = vm.mem(next_seg_addr) as usize;
    if next_seg_addr > HEAP_END || next_cap == 0 {
        vm.set_mem(seg_addr, (next_seg_addr - seg_addr - 2) as Word);
    } else {
        vm.set_mem(seg_addr, (next_seg_addr - seg_addr + next_cap) as Word);
        vm.set_mem(seg_addr + 1, vm.mem(next_seg_addr + 1));
    }
    Ok(StdlibOk::Finished(0))
}

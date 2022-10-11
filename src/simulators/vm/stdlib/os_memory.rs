use super::*;
use crate::simulators::vm::VM;

use crate::definitions::{HEAP_END, HEAP_START};

pub fn init(vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    vm.set_mem(HEAP_START, ((HEAP_END + 1) - (HEAP_START + 2)) as Word)?;
    vm.set_mem(HEAP_START + 1, HEAP_END as Word + 1)?;

    Ok(StdlibOk::Finished(0))
}

pub fn peek(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(vm.mem(params[0] as Address)?))
}

pub fn poke(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    vm.set_mem(params[0] as Address, params[1])?;
    Ok(StdlibOk::Finished(0))
}

pub fn alloc(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let size = params[0] as usize;
    if size < 1 {
        return Err(StdlibError::MemoryAllocNonPositiveSize);
    }

    let mut seg_addr = HEAP_START;
    let mut seg_cap = 0;
    while seg_addr <= HEAP_END {
        seg_cap = vm.mem(seg_addr)? as usize;
        if seg_cap >= size {
            break;
        }
        seg_addr = vm.mem(seg_addr + 1)? as usize;
    }

    if seg_addr > HEAP_END {
        return Err(StdlibError::MemoryHeapOverflow);
    }

    if seg_cap > size + 2 {
        vm.set_mem(seg_addr + size + 2, (seg_cap - size - 2) as Word)?;
        vm.set_mem(seg_addr + size + 3, vm.mem(seg_addr + 1)?)?;
        vm.set_mem(seg_addr + 1, (seg_addr + size + 2) as Word)?;
    }

    vm.set_mem(seg_addr, 0)?;
    Ok(StdlibOk::Finished(seg_addr as Word + 2))
}

pub fn de_alloc(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let arr = params[0] as usize;
    let seg_addr = arr - 2;
    let next_seg_addr = vm.mem(seg_addr + 1)? as usize;

    let next_cap = vm.mem(next_seg_addr)? as usize;
    if next_seg_addr > HEAP_END || next_cap == 0 {
        vm.set_mem(seg_addr, (next_seg_addr - seg_addr - 2) as Word)?;
    } else {
        vm.set_mem(seg_addr, (next_seg_addr - seg_addr + next_cap) as Word)?;
        vm.set_mem(seg_addr + 1, vm.mem(next_seg_addr + 1)?)?;
    }
    Ok(StdlibOk::Finished(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use crate::SourceFile;
    use crate::VM;

    // this test comes from the MemoryTest directory in project 12
    #[test]
    fn memory_test() {
        macro_rules! stdlib {
            ($name:expr) => {
                include_str!(concat!(
                    concat!(env!("CARGO_MANIFEST_DIR"), "/res/stdlib/"),
                    $name
                ))
            };
        }

        let mut vm = VM::new(Stdlib::new());

        // use the VM implementations for everything except Memory.vm
        let sys = stdlib!("Sys.vm");
        let keyboard = stdlib!("Keyboard.vm");
        let array = stdlib!("Array.vm");
        let math = stdlib!("Math.vm");
        let output = stdlib!("Output.vm");
        let screen = stdlib!("Screen.vm");
        let string = stdlib!("String.vm");

        let test = r#"
            function Main.main 5
            push constant 8000
            push constant 333
            call Memory.poke 2
            pop temp 0
            push constant 8000
            call Memory.peek 1
            pop local 0
            push constant 8001
            push local 0
            push constant 1
            add
            call Memory.poke 2
            pop temp 0
            push constant 3
            call Array.new 1
            pop local 2
            push constant 2
            push local 2
            add
            push constant 222
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 8002
            push constant 2
            push local 2
            add
            pop pointer 1
            push that 0
            call Memory.poke 2
            pop temp 0
            push constant 0
            pop local 1
            push constant 3
            call Array.new 1
            pop local 3
            push constant 1
            push local 3
            add
            push constant 2
            push local 2
            add
            pop pointer 1
            push that 0
            push constant 100
            sub
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 3
            push local 2
            eq
            if-goto IF_TRUE0
            goto IF_FALSE0
            label IF_TRUE0
            push constant 1
            pop local 1
            label IF_FALSE0
            push constant 8003
            push constant 1
            push local 3
            add
            pop pointer 1
            push that 0
            push local 1
            add
            call Memory.poke 2
            pop temp 0
            push constant 0
            pop local 1
            push constant 500
            call Array.new 1
            pop local 4
            push constant 499
            push local 4
            add
            push constant 2
            push local 2
            add
            pop pointer 1
            push that 0
            push constant 1
            push local 3
            add
            pop pointer 1
            push that 0
            sub
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 4
            push local 2
            eq
            if-goto IF_TRUE1
            goto IF_FALSE1
            label IF_TRUE1
            push constant 1
            pop local 1
            label IF_FALSE1
            push local 4
            push local 3
            eq
            if-goto IF_TRUE2
            goto IF_FALSE2
            label IF_TRUE2
            push local 1
            push constant 10
            add
            pop local 1
            label IF_FALSE2
            push constant 8004
            push constant 499
            push local 4
            add
            pop pointer 1
            push that 0
            push local 1
            add
            call Memory.poke 2
            pop temp 0
            push local 2
            call Array.dispose 1
            pop temp 0
            push local 3
            call Array.dispose 1
            pop temp 0
            push constant 0
            pop local 1
            push constant 3
            call Array.new 1
            pop local 3
            push constant 0
            push local 3
            add
            push constant 499
            push local 4
            add
            pop pointer 1
            push that 0
            push constant 90
            sub
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 3
            push local 4
            eq
            if-goto IF_TRUE3
            goto IF_FALSE3
            label IF_TRUE3
            push constant 1
            pop local 1
            label IF_FALSE3
            push constant 8005
            push constant 0
            push local 3
            add
            pop pointer 1
            push that 0
            push local 1
            add
            call Memory.poke 2
            pop temp 0
            push local 4
            call Array.dispose 1
            pop temp 0
            push local 3
            call Array.dispose 1
            pop temp 0
            push constant 0
            return"#;

        let programs = vec![
            SourceFile::new("Keyboard.vm", keyboard),
            SourceFile::new("Sys.vm", sys),
            SourceFile::new("Array.vm", array),
            SourceFile::new("Math.vm", math),
            SourceFile::new("Output.vm", output),
            SourceFile::new("Screen.vm", screen),
            SourceFile::new("String.vm", string),
            SourceFile::new("Main.vm", test),
        ];

        let mut bytecode_parser = Parser::with_stdlib(programs, Stdlib::new());
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..1000000 {
            vm.step().unwrap();
        }

        assert_eq!(Ok(333), vm.mem(8000));
        assert_eq!(Ok(334), vm.mem(8001));
        assert_eq!(Ok(222), vm.mem(8002));
        assert_eq!(Ok(122), vm.mem(8003));
        assert_eq!(Ok(100), vm.mem(8004));
        assert_eq!(Ok(10), vm.mem(8005));
    }

    // this test comes from the MemoryTest directory in project 12
    #[test]
    fn memory_test_diag() {
        macro_rules! stdlib {
            ($name:expr) => {
                include_str!(concat!(
                    concat!(env!("CARGO_MANIFEST_DIR"), "/res/stdlib/"),
                    $name
                ))
            };
        }

        let mut vm = VM::new(Stdlib::new());

        // use the VM implementations for everything except Memory.vm
        let sys = stdlib!("Sys.vm");
        let keyboard = stdlib!("Keyboard.vm");
        let array = stdlib!("Array.vm");
        let math = stdlib!("Math.vm");
        let output = stdlib!("Output.vm");
        let screen = stdlib!("Screen.vm");
        let string = stdlib!("String.vm");

        let test = r#"
            function Main.main 5
            push constant 17000
            pop local 4
            push constant 0
            push local 4
            add
            push constant 10
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 4
            push constant 1
            add
            push constant 333
            call Memory.poke 2
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 11
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 4
            push constant 1
            add
            call Memory.peek 1
            pop local 0
            push constant 2
            push local 4
            add
            push local 0
            push constant 1
            add
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 12
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 20
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 20
            call Memory.alloc 1
            pop local 1
            push constant 3
            push local 4
            add
            push local 1
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 21
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 1
            push constant 20
            call Main.checkRange 2
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 22
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 30
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 3
            call Memory.alloc 1
            pop local 2
            push constant 4
            push local 4
            add
            push local 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 31
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 2
            push constant 3
            call Main.checkRange 2
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 32
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 2
            push constant 3
            push local 1
            push constant 3
            call Main.checkOverlap 4
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 33
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 40
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 500
            call Memory.alloc 1
            pop local 3
            push constant 5
            push local 4
            add
            push local 3
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 41
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 3
            push constant 500
            call Main.checkRange 2
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 42
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 3
            push constant 500
            push local 1
            push constant 3
            call Main.checkOverlap 4
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 43
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 3
            push constant 500
            push local 2
            push constant 3
            call Main.checkOverlap 4
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 44
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 50
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 1
            call Memory.deAlloc 1
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 51
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 2
            call Memory.deAlloc 1
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 52
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 60
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 3
            call Memory.alloc 1
            pop local 2
            push constant 6
            push local 4
            add
            push local 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 61
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 2
            push constant 3
            call Main.checkRange 2
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 62
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 2
            push constant 3
            push local 3
            push constant 500
            call Main.checkOverlap 4
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 63
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 70
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 3
            call Memory.deAlloc 1
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 71
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 2
            call Memory.deAlloc 1
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 72
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 70
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 8000
            call Memory.alloc 1
            pop local 1
            push constant 7
            push local 4
            add
            push local 1
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 71
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 1
            push constant 8000
            call Main.checkRange 2
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 72
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 1
            call Memory.deAlloc 1
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 73
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 7000
            call Memory.alloc 1
            pop local 1
            push constant 0
            push local 4
            add
            push constant 74
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 1
            push constant 7000
            call Main.checkRange 2
            pop temp 0
            push constant 0
            push local 4
            add
            push constant 75
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 1
            call Memory.deAlloc 1
            pop temp 0
            push constant 8
            push local 4
            add
            push local 1
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 4
            add
            push constant 100
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            return
            function Main.checkRange 1
            push argument 0
            push argument 1
            add
            push constant 1
            sub
            pop local 0
            push argument 0
            push constant 2048
            lt
            push local 0
            push constant 16383
            gt
            or
            if-goto IF_TRUE0
            goto IF_FALSE0
            label IF_TRUE0
            call Sys.halt 0
            pop temp 0
            label IF_FALSE0
            push constant 0
            return
            function Main.checkOverlap 2
            push argument 0
            push argument 1
            add
            push constant 1
            sub
            pop local 0
            push argument 2
            push argument 3
            add
            push constant 1
            sub
            pop local 1
            push argument 0
            push local 1
            gt
            push local 0
            push argument 2
            lt
            or
            not
            if-goto IF_TRUE0
            goto IF_FALSE0
            label IF_TRUE0
            call Sys.halt 0
            pop temp 0
            label IF_FALSE0
            push constant 0
            return"#;

        let programs = vec![
            SourceFile::new("Keyboard.vm", keyboard),
            SourceFile::new("Sys.vm", sys),
            SourceFile::new("Array.vm", array),
            SourceFile::new("Math.vm", math),
            SourceFile::new("Output.vm", output),
            SourceFile::new("Screen.vm", screen),
            SourceFile::new("String.vm", string),
            SourceFile::new("Main.vm", test),
        ];

        let mut bytecode_parser = Parser::with_stdlib(programs, Stdlib::new());
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..1000000 {
            vm.step().unwrap();
        }

        assert_eq!(Ok(100), vm.mem(17000));
        assert_eq!(Ok(333), vm.mem(17001));
        assert_eq!(Ok(334), vm.mem(17002));
    }
}

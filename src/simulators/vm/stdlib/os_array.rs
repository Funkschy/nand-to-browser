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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use crate::SourceFile;
    use crate::VM;

    // this test comes from the ArrayTest directory in project 12
    #[test]
    fn array_test() {
        macro_rules! stdlib {
            ($name:expr) => {
                include_str!(concat!(
                    concat!(env!("CARGO_MANIFEST_DIR"), "/res/stdlib/"),
                    $name
                ))
            };
        }

        let mut vm = VM::new(Stdlib::new());

        // use the VM implementations for everything except Array.vm
        let sys = stdlib!("Sys.vm");
        let keyboard = stdlib!("Keyboard.vm");
        let math = stdlib!("Math.vm");
        let memory = stdlib!("Memory.vm");
        let output = stdlib!("Output.vm");
        let screen = stdlib!("Screen.vm");
        let string = stdlib!("String.vm");

        let test = r#"
            function Main.main 4
            push constant 8000
            pop local 0
            push constant 3
            call Array.new 1
            pop local 1
            push constant 2
            push local 1
            add
            push constant 222
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            push local 0
            add
            push constant 2
            push local 1
            add
            pop pointer 1
            push that 0
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 3
            call Array.new 1
            pop local 2
            push constant 1
            push local 2
            add
            push constant 2
            push local 1
            add
            pop pointer 1
            push that 0
            push constant 100
            sub
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 1
            push local 0
            add
            push constant 1
            push local 2
            add
            pop pointer 1
            push that 0
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 500
            call Array.new 1
            pop local 3
            push constant 499
            push local 3
            add
            push constant 2
            push local 1
            add
            pop pointer 1
            push that 0
            push constant 1
            push local 2
            add
            pop pointer 1
            push that 0
            sub
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 2
            push local 0
            add
            push constant 499
            push local 3
            add
            pop pointer 1
            push that 0
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 1
            call Array.dispose 1
            pop temp 0
            push local 2
            call Array.dispose 1
            pop temp 0
            push constant 3
            call Array.new 1
            pop local 2
            push constant 0
            push local 2
            add
            push constant 499
            push local 3
            add
            pop pointer 1
            push that 0
            push constant 90
            sub
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 3
            push local 0
            add
            push constant 0
            push local 2
            add
            pop pointer 1
            push that 0
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push local 3
            call Array.dispose 1
            pop temp 0
            push local 2
            call Array.dispose 1
            pop temp 0
            push constant 0
            return"#;

        let programs = vec![
            SourceFile::new("Sys.vm", sys),
            SourceFile::new("Keyboard.vm", keyboard),
            SourceFile::new("Math.vm", math),
            SourceFile::new("Memory.vm", memory),
            SourceFile::new("Output.vm", output),
            SourceFile::new("Screen.vm", screen),
            SourceFile::new("String.vm", string),
            SourceFile::new("Main.vm", test),
        ];

        let mut bytecode_parser = Parser::with_stdlib(programs, Stdlib::new());
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..1000000 {
            vm.step();
        }

        assert_eq!(222, vm.mem(8000));
        assert_eq!(122, vm.mem(8001));
        assert_eq!(100, vm.mem(8002));
        assert_eq!(10, vm.mem(8003));
    }
}

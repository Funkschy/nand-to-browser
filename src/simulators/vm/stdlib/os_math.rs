use super::*;
use crate::simulators::vm::VM;

pub fn init(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(0))
}

pub fn abs(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(params[0].abs()))
}

pub fn multiply(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    // java doesn't handle overflows for ints, so this casting is needed for compatibility
    Ok(StdlibOk::Finished(
        (params[0] as i32 * params[1] as i32) as i16,
    ))
}

pub fn divide(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    if params[1] != 0 {
        Ok(StdlibOk::Finished(params[0] / params[1]))
    } else {
        Err(StdlibError::MathDivideByZero)
    }
}

pub fn min(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(params[0].min(params[1])))
}

pub fn max(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    Ok(StdlibOk::Finished(params[0].max(params[1])))
}

pub fn sqrt(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    if params[0] >= 0 {
        Ok(StdlibOk::Finished((params[0] as f64).sqrt() as Word))
    } else {
        Err(StdlibError::MathNegativeSqrt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::bytecode::{BytecodeParser, SourceFile};

    // this test comes from the MathTest directory in project 12
    #[test]
    fn math_test() {
        macro_rules! stdlib {
            ($name:expr) => {
                include_str!(concat!(
                    concat!(env!("CARGO_MANIFEST_DIR"), "/res/stdlib/"),
                    $name
                ))
            };
        }

        let mut vm = VM::new(Stdlib::new());

        // use the VM implementations for everything except Math.vm
        let sys = stdlib!("Sys.vm");
        let keyboard = stdlib!("Keyboard.vm");
        let array = stdlib!("Array.vm");
        let memory = stdlib!("Memory.vm");
        let output = stdlib!("Output.vm");
        let screen = stdlib!("Screen.vm");
        let string = stdlib!("String.vm");

        let test = r#"
            function Main.main 1
            push constant 8000
            pop local 0
            push constant 0
            push local 0
            add
            push constant 2
            push constant 3
            call Math.multiply 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 1
            push local 0
            add
            push constant 0
            push local 0
            add
            pop pointer 1
            push that 0
            push constant 30
            neg
            call Math.multiply 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 2
            push local 0
            add
            push constant 1
            push local 0
            add
            pop pointer 1
            push that 0
            push constant 100
            call Math.multiply 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 3
            push local 0
            add
            push constant 1
            push constant 2
            push local 0
            add
            pop pointer 1
            push that 0
            call Math.multiply 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 4
            push local 0
            add
            push constant 3
            push local 0
            add
            pop pointer 1
            push that 0
            push constant 0
            call Math.multiply 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 5
            push local 0
            add
            push constant 9
            push constant 3
            call Math.divide 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 6
            push local 0
            add
            push constant 18000
            neg
            push constant 6
            call Math.divide 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 7
            push local 0
            add
            push constant 32766
            push constant 32767
            neg
            call Math.divide 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 8
            push local 0
            add
            push constant 9
            call Math.sqrt 1
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 9
            push local 0
            add
            push constant 32767
            call Math.sqrt 1
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 10
            push local 0
            add
            push constant 345
            push constant 123
            call Math.min 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 11
            push local 0
            add
            push constant 123
            push constant 345
            neg
            call Math.max 2
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 12
            push local 0
            add
            push constant 27
            call Math.abs 1
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 13
            push local 0
            add
            push constant 32767
            neg
            call Math.abs 1
            pop temp 0
            pop pointer 1
            push temp 0
            pop that 0
            push constant 0
            return"#;

        let programs = vec![
            SourceFile::new("Keyboard.vm", keyboard),
            SourceFile::new("Sys.vm", sys),
            SourceFile::new("Array.vm", array),
            SourceFile::new("Memory.vm", memory),
            SourceFile::new("Output.vm", output),
            SourceFile::new("Screen.vm", screen),
            SourceFile::new("String.vm", string),
            SourceFile::new("Main.vm", test),
        ];

        let mut bytecode_parser = BytecodeParser::with_stdlib(programs, Stdlib::new());
        let program = bytecode_parser.parse().unwrap();

        vm.load(program);

        for _ in 0..1000000 {
            vm.step().unwrap();
        }

        assert_eq!(Ok(6), vm.mem(8000));
        assert_eq!(Ok(-180), vm.mem(8001));
        assert_eq!(Ok(-18000), vm.mem(8002));
        assert_eq!(Ok(-18000), vm.mem(8003));
        assert_eq!(Ok(0), vm.mem(8004));
        assert_eq!(Ok(3), vm.mem(8005));
        assert_eq!(Ok(-3000), vm.mem(8006));
        assert_eq!(Ok(0), vm.mem(8007));
        assert_eq!(Ok(3), vm.mem(8008));
        assert_eq!(Ok(181), vm.mem(8009));
        assert_eq!(Ok(123), vm.mem(8010));
        assert_eq!(Ok(123), vm.mem(8011));
        assert_eq!(Ok(27), vm.mem(8012));
        assert_eq!(Ok(32767), vm.mem(8013));
    }
}

use crate::definitions::{Address, Word, KBD, MEM_SIZE, SCREEN_END, SCREEN_START};
use command::{Computation, Instruction, Jump, Register};
pub use error::CpuError;

pub mod command;
pub mod error;
pub mod script;

pub type CpuResult<T = ()> = Result<T, CpuError>;

pub struct Cpu {
    pc: usize,
    program: Vec<Instruction>,

    a: Word,
    d: Word,
    memory: Box<[Word; MEM_SIZE]>,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            pc: 0,
            program: Vec::new(),
            a: 0,
            d: 0,
            memory: Box::new([0; MEM_SIZE]),
        }
    }
}

impl Cpu {
    #[inline]
    fn mem(&self, address: Address) -> CpuResult<Word> {
        self.memory
            .get(address)
            .copied()
            .ok_or(CpuError::IllegalMemoryAddress(address))
    }

    #[inline]
    fn set_mem(&mut self, address: Address, value: Word) -> CpuResult {
        *self
            .memory
            .get_mut(address)
            .ok_or(CpuError::IllegalMemoryAddress(address))? = value;
        Ok(())
    }

    fn reg(&self, reg: Register) -> CpuResult<Word> {
        match reg {
            Register::A => Ok(self.a),
            Register::D => Ok(self.d),
            Register::M => self.mem(self.a as Address),
        }
    }

    pub fn load(&mut self, program: Vec<Instruction>) {
        self.pc = 0;
        self.program = program;
        self.a = 0;
        self.d = 0;
        for i in 0..self.memory.len() {
            self.memory[i] = 0;
        }
    }

    pub fn step(&mut self) -> CpuResult {
        macro_rules! binary {
            ( $r1:expr, $op:tt, $r2:expr) => {{
                // cast up to i32 so that no overflow checks get triggered in debug mode
                let l = self.reg($r1)? as i32;
                let r = self.reg($r2)? as i32;
                (l $op r) as Word
            }};
        }

        macro_rules! jump_if {
            ($value:expr, $op:tt) => {
                if $value $op 0 {
                    self.a as Address
                } else {
                    self.pc + 1
                }
            }
        }

        let instr = *self
            .program
            .get(self.pc)
            .ok_or(CpuError::IllegalProgramCounter(self.pc))?;

        match instr {
            Instruction::A(value) => {
                self.a = value as Word;
                self.pc += 1;
            }
            Instruction::C(dest, comp, jump) => {
                let value = match comp {
                    Computation::ConstZero => 0,
                    Computation::ConstOne => 1,
                    Computation::ConstNegOne => -1,
                    Computation::UnaryNone(r) => self.reg(r)?,
                    Computation::UnaryBoolNeg(r) => !(self.reg(r)? as i32) as Word,
                    Computation::UnaryIntNeg(r) => -(self.reg(r)? as i32) as Word,
                    Computation::BinaryInc(r) => (self.reg(r)? as i32 + 1) as Word,
                    Computation::BinaryDec(r) => (self.reg(r)? as i32 - 1) as Word,
                    Computation::BinaryAdd(r1, r2) => binary!(r1, +, r2),
                    Computation::BinarySub(r1, r2) => binary!(r1, -, r2),
                    Computation::BinaryAnd(r1, r2) => binary!(r1, &, r2),
                    Computation::BinaryOr(r1, r2) => binary!(r1, |, r2),
                };

                let (a, d, m) = dest.as_bools();
                if m {
                    self.set_mem(self.a as Address, value)?;
                }
                if a {
                    self.a = value;
                }
                if d {
                    self.d = value;
                }

                self.pc = match jump {
                    Jump::Next => self.pc + 1,
                    Jump::Gt => jump_if!(value, >),
                    Jump::Eq => jump_if!(value, ==),
                    Jump::Ge => jump_if!(value, >=),
                    Jump::Lt => jump_if!(value, <),
                    Jump::Ne => jump_if!(value, !=),
                    Jump::Le => jump_if!(value, <=),
                    Jump::Unconditional => self.a as Address,
                }
            }
        }

        Ok(())
    }
}

// UI interaction
#[allow(dead_code)]
impl Cpu {
    pub fn set_input_key(&mut self, key: i16) -> CpuResult {
        self.set_mem(KBD, key)
    }

    pub fn display(&self) -> &[Word] {
        &self.memory[SCREEN_START..=SCREEN_END]
    }

    pub fn memory_at(&self, address: Address) -> Option<Word> {
        self.mem(address).ok()
    }

    pub fn current_file_offset(&self) -> usize {
        self.pc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parse::assembly::{AssemblyParser, SourceFile};

    #[test]
    fn test_sum_1_to_100() {
        let src = r#"
            // Adds 1+...+100.
            @i // i refers to some mem. location.
            M=1 // i=1
            @sum // sum refers to some mem. location.
            M=0 // sum=0
            (LOOP)
            @i
            D=M // D=i
            @100
            D=D-A // D=i-100
            @END
            D;JGT // If (i-100)>0 goto END
            @i
            D=M // D=i
            @sum
            M=D+M // sum=sum+i
            @i
            M=M+1 // i=i+1
            @LOOP
            0;JMP // Goto LOOP
            (END)
            @END
            0;JMP // Infinite loop"#;

        let mut parser = AssemblyParser::new(SourceFile::new(src));
        let program = parser.parse().unwrap();

        let mut cpu = Cpu::default();
        cpu.load(program);

        for _ in 0..10000 {
            cpu.step().unwrap();
        }

        assert_eq!(Ok(5050), cpu.mem(17));
    }
}

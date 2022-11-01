use crate::parse::assembly::{Parser, SourceFile};
use crate::parse::script::tst::{CpuEmulatorCommand, CpuSetTarget};
use crate::simulators::cpu::Cpu;
use crate::simulators::{ExecResult, SimulatorExecutor};

use super::parse_set_target;

use std::fs::read_to_string;

impl SimulatorExecutor<CpuEmulatorCommand> for Cpu {
    fn get_value(&self, name: &str) -> ExecResult<i64> {
        // the address format is the same as the set target format
        let address = parse_set_target(name)?;
        Ok(match address {
            CpuSetTarget::A => self.a as i64,
            CpuSetTarget::D => self.d as i64,
            CpuSetTarget::PC => self.pc as i64,
            CpuSetTarget::Ram(address) => self.mem(address)? as i64,
            CpuSetTarget::Rom(_address) => unimplemented!("no rom access yet"),
        })
    }

    fn exec_sim(&mut self, c: CpuEmulatorCommand) -> ExecResult {
        match c {
            CpuEmulatorCommand::Load(path) => {
                let content = read_to_string(path.clone())?;
                let filename = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_owned())
                    .ok_or("Could not get filename of path")?;
                let file = SourceFile::new(filename, &content);
                let program = Parser::new(file).parse()?;

                self.load(program);
            }
            CpuEmulatorCommand::TickTock => self.step()?,
            CpuEmulatorCommand::Set(target, value) => match target {
                CpuSetTarget::A => self.a = value,
                CpuSetTarget::D => self.d = value,
                CpuSetTarget::PC => self.pc = value as usize,
                CpuSetTarget::Ram(address) => self.set_mem(address, value)?,
                CpuSetTarget::Rom(_address) => unimplemented!("no rom access yet"),
            },
        };
        Ok(())
    }
}

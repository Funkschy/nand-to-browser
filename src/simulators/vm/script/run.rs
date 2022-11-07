use crate::definitions::{ARG, LCL, SP, THAT, THIS};
use crate::parse::bytecode::{BytecodeParser, SourceFile};
use crate::parse::script::tst::{VMEmulatorCommand, VMSetTarget};
use crate::simulators::vm::stdlib::{Stdlib, StdlibError};
use crate::simulators::vm::{VMError, VM};
use crate::simulators::{ExecResult, Halt, SimulatorExecutor};

use super::parse_set_target;

use std::fs::{read_dir, read_to_string};

impl SimulatorExecutor<VMEmulatorCommand> for VM {
    fn get_value(&self, name: &str) -> ExecResult<i64> {
        // the address format is the same as the set target format
        let address = parse_set_target(name)?;
        Ok(match address {
            VMSetTarget::Local(Some(index)) => self.mem_indirect(LCL, index).map(|v| v as i64)?,
            VMSetTarget::Local(None) => self.mem(LCL).map(|v| v as i64)?,
            VMSetTarget::Argument(Some(index)) => {
                self.mem_indirect(ARG, index).map(|v| v as i64)?
            }
            VMSetTarget::Argument(None) => self.mem(ARG).map(|v| v as i64)?,
            VMSetTarget::This(Some(index)) => self.mem_indirect(THIS, index).map(|v| v as i64)?,
            VMSetTarget::This(None) => self.mem(THIS).map(|v| v as i64)?,
            VMSetTarget::That(Some(index)) => self.mem_indirect(THAT, index).map(|v| v as i64)?,
            VMSetTarget::That(None) => self.mem(THAT).map(|v| v as i64)?,
            VMSetTarget::SP => self.mem(SP).map(|v| v as i64)?,
            VMSetTarget::CurrentFunction => todo!("implement current function"),
            VMSetTarget::Line => self.pc as i64,
            VMSetTarget::Temp(_) => todo!("implement temp"),
            VMSetTarget::Ram(address) => self.mem(address).map(|v| v as i64)?,
        })
    }

    fn exec_sim(&mut self, c: VMEmulatorCommand) -> ExecResult {
        match c {
            VMEmulatorCommand::Load(path) => {
                let program = if path.is_dir() {
                    let mut sources = vec![];
                    for entry in read_dir(path)? {
                        let path = entry?.path();
                        if let Some(ext) = path.extension() {
                            if ext == "vm" {
                                let content = read_to_string(path.clone())?;
                                let filename = path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .map(|s| s.to_owned())
                                    .ok_or("Could not get filename of path")?;
                                sources.push((filename, content));
                            }
                        }
                    }

                    let sources = sources
                        .iter()
                        .map(|(name, content)| SourceFile::new(name, content))
                        .collect();

                    BytecodeParser::with_stdlib(sources, Stdlib::new()).parse()?
                } else {
                    let content = read_to_string(path.clone())?;
                    let filename = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .ok_or("Could not get filename of path")?;
                    let file = SourceFile::new(filename, &content);
                    BytecodeParser::with_stdlib(vec![file], Stdlib::new()).parse()?
                };

                self.load(program);
            }
            VMEmulatorCommand::Step => {
                let result = self.step();
                if let Err(VMError::StdlibError(StdlibError::Halt)) = &result {
                    return Err(Halt.into());
                }
                result?;
            }
            VMEmulatorCommand::Set(target, value) => match target {
                VMSetTarget::Local(Some(index)) => self.set_mem_indirect(LCL, index, value)?,
                VMSetTarget::Local(None) => self.set_mem(LCL, value)?,
                VMSetTarget::Argument(Some(index)) => self.set_mem_indirect(ARG, index, value)?,
                VMSetTarget::Argument(None) => self.set_mem(ARG, value)?,
                VMSetTarget::This(Some(index)) => self.set_mem_indirect(THIS, index, value)?,
                VMSetTarget::This(None) => self.set_mem(THIS, value)?,
                VMSetTarget::That(Some(index)) => self.set_mem_indirect(THAT, index, value)?,
                VMSetTarget::That(None) => self.set_mem(THAT, value)?,
                VMSetTarget::SP => self.set_mem(SP, value)?,
                VMSetTarget::CurrentFunction => todo!("implement current function"),
                VMSetTarget::Line => self.pc = value as usize,
                VMSetTarget::Temp(_) => todo!("implement temp"),
                VMSetTarget::Ram(address) => self.set_mem(address, value)?,
            },
        };
        Ok(())
    }
}

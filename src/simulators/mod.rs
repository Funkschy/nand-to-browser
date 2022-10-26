use crate::parse::script::parser::ScriptParser;
use crate::parse::script::tst::{
    Command, CommandKind, NumberFormat, OutputListEntry, SimulatorCommand,
};
use crate::parse::script::SimulatorCommandParser;

use std::error::Error;
use std::io;
use std::marker::PhantomData;
use std::path::PathBuf;

pub mod vm;

pub type ExecResult<T = ()> = Result<T, Box<dyn Error>>;

pub trait SimulatorExecutor<CMD> {
    fn get_value(&self, name: &str) -> ExecResult<i64>;
    fn exec_sim(&mut self, cmd: CMD) -> ExecResult;
}

pub trait ScriptExecutor<CMD>
where
    CMD: SimulatorCommand,
{
    fn exec(&mut self, cmd: Command<CMD>) -> ExecResult;
}

pub struct BaseScriptExecutor<'w, W, CMD, SIM>
where
    W: io::Write,
    CMD: SimulatorCommand,
    SIM: SimulatorExecutor<CMD>,
{
    sim: SIM,
    print_output_header_line: bool,
    writer: &'w mut W,

    output_file: Option<PathBuf>,
    compare_file: Option<PathBuf>,
    output_list: Vec<OutputListEntry>,
    phantom: PhantomData<CMD>,
}

impl<'w, W, CMD, SIM> BaseScriptExecutor<'w, W, CMD, SIM>
where
    W: io::Write,
    CMD: SimulatorCommand,
    SIM: SimulatorExecutor<CMD>,
{
    fn new(sim: SIM, writer: &'w mut W) -> Self {
        BaseScriptExecutor {
            sim,
            writer,
            print_output_header_line: true,
            output_file: None,
            compare_file: None,
            output_list: Vec::new(),
            phantom: PhantomData,
        }
    }

    fn set_output_file(&mut self, path: PathBuf) -> ExecResult {
        self.output_file = Some(path);
        Ok(())
    }

    fn set_compare_file(&mut self, path: PathBuf) -> ExecResult {
        self.compare_file = Some(path);
        Ok(())
    }

    fn set_output_list(&mut self, list: Vec<OutputListEntry>) -> ExecResult {
        self.output_list = list;
        Ok(())
    }

    fn print_output_header_if_needed(&mut self) -> ExecResult {
        if !self.print_output_header_line {
            return Ok(());
        }

        write!(self.writer, "|")?;

        for OutputListEntry {
            name,
            left_padding,
            length,
            right_padding,
            ..
        } in self.output_list.iter()
        {
            let length = *length;
            let left_padding = *left_padding;
            let right_padding = *right_padding;

            let space = left_padding + length + right_padding;
            let name = if name.len() > space {
                &name[0..space]
            } else {
                name
            };

            let left_space = (space - name.len()) / 2;
            let right_space = space - left_space - name.len();

            for _ in 0..left_space {
                write!(self.writer, " ")?;
            }

            write!(self.writer, "{}", name)?;

            for _ in 0..right_space {
                write!(self.writer, " ")?;
            }
            write!(self.writer, "|")?;
        }

        writeln!(self.writer)?;
        self.print_output_header_line = false;
        Ok(())
    }
}

impl<'w, W, CMD, SIM> ScriptExecutor<CMD> for BaseScriptExecutor<'w, W, CMD, SIM>
where
    W: io::Write,
    CMD: SimulatorCommand,
    SIM: SimulatorExecutor<CMD>,
{
    fn exec(&mut self, cmd: Command<CMD>) -> ExecResult {
        match cmd.kind {
            CommandKind::Simulator(sim_cmd) => self.sim.exec_sim(sim_cmd),
            CommandKind::OutputFile(output_file) => self.set_output_file(output_file),
            CommandKind::CompareTo(compare_file) => self.set_compare_file(compare_file),
            CommandKind::OutputList(output_list) => self.set_output_list(output_list),
            CommandKind::Output => {
                self.print_output_header_if_needed()?;

                write!(self.writer, "|")?;

                for OutputListEntry {
                    name,
                    left_padding,
                    length,
                    right_padding,
                    format,
                } in self.output_list.iter()
                {
                    let length = *length;
                    let left_padding = *left_padding;
                    let right_padding = *right_padding;
                    let format = *format;

                    if let NumberFormat::String = format {
                        todo!("support string formats");
                    }

                    let value = self.sim.get_value(name)?.to_string();
                    let value_string = format.format_string(&value)?;

                    let value_string = if value_string.len() > length {
                        &value_string[0..(value_string.len() - length)]
                    } else {
                        &value_string
                    };

                    let left_space = left_padding + (length - value_string.len());
                    let right_space = right_padding;

                    for _ in 0..left_space {
                        write!(self.writer, " ")?;
                    }

                    write!(self.writer, "{}", value_string)?;

                    for _ in 0..right_space {
                        write!(self.writer, " ")?;
                    }
                    write!(self.writer, "|")?;
                }

                writeln!(self.writer)?;
                Ok(())
            }
            CommandKind::Repeat { times, block } => {
                for _ in 0..times {
                    for cmd in block.iter() {
                        self.exec(cmd.clone())?;
                    }
                }
                Ok(())
            }
        }
    }
}

pub fn execute_script<'tst, P, X, C, W>(
    p: ScriptParser<'tst, P, C>,
    sim_executor: X,
    writer: &mut W,
) -> ExecResult
where
    W: io::Write,
    C: SimulatorCommand,
    X: SimulatorExecutor<C>,
    ScriptParser<'tst, P, C>: SimulatorCommandParser<C>,
{
    let mut executor = BaseScriptExecutor::new(sim_executor, writer);
    for cmd in p {
        executor.exec(cmd?.content)?;
    }

    Ok(())
}

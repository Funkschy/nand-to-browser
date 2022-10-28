use crate::parse::script::parser::ScriptParser;
use crate::parse::script::tst::{
    Command, CommandKind, NumberFormat, OutputListEntry, SimulatorCommand,
};
use crate::parse::script::SimulatorCommandParser;

use std::error::Error;
use std::fmt;
use std::fs::{read_to_string, File, OpenOptions};
use std::io::Write;
use std::marker::PhantomData;
use std::path::PathBuf;

pub mod vm;

#[derive(Debug, PartialEq, Eq)]
pub struct Halt;

impl fmt::Display for Halt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "halt")
    }
}

impl Error for Halt {}

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

pub struct BaseScriptExecutor<'w, CMD, SIM>
where
    CMD: SimulatorCommand,
    SIM: SimulatorExecutor<CMD>,
{
    sim: SIM,
    print_output_header_line: bool,
    // if this is not None, it will be used to overwrite the output_file
    // this is useful for testing and the Web UI in the future
    writer: Option<&'w mut dyn Write>,

    output_file: Option<(PathBuf, File)>,
    compare_file: Option<PathBuf>,
    output_list: Vec<OutputListEntry>,
    phantom: PhantomData<CMD>,
}

impl<'w, CMD, SIM> BaseScriptExecutor<'w, CMD, SIM>
where
    CMD: SimulatorCommand,
    SIM: SimulatorExecutor<CMD>,
{
    fn new(sim: SIM, writer: impl Into<Option<&'w mut dyn Write>>) -> Self {
        BaseScriptExecutor {
            sim,
            writer: writer.into(),
            print_output_header_line: true,
            output_file: None,
            compare_file: None,
            output_list: Vec::new(),
            phantom: PhantomData,
        }
    }

    fn writer(&mut self) -> ExecResult<&mut dyn Write> {
        if let Some(w) = &mut self.writer {
            return Ok(w);
        }

        if let Some((_, output_file)) = &mut self.output_file {
            Ok(output_file)
        } else {
            Err("Trying to output without an output file".into())
        }
    }

    fn set_output_file(&mut self, path: PathBuf) -> ExecResult {
        let file = OpenOptions::new().create(true).write(true).open(&path)?;
        self.output_file = Some((path, file));
        Ok(())
    }

    fn set_compare_file(&mut self, path: PathBuf) -> ExecResult {
        self.compare_file = Some(path);
        Ok(())
    }

    fn set_output_list(&mut self, list: Vec<OutputListEntry>) -> ExecResult {
        self.output_list = list;
        self.print_output_header_line = true;
        Ok(())
    }

    fn print_output_header_if_needed(&mut self) -> ExecResult {
        if !self.print_output_header_line {
            return Ok(());
        }

        // this is a bit of a dirty hack because rust
        // the writer() method needs a mut borrow of self, which isn't possible inside the loop,
        // because the loop already borrows the outputlist immutably
        let mut temp_writer = Vec::new();

        for entry in self.output_list.iter() {
            let name = &entry.name;
            let length = entry.length;
            let left_padding = entry.left_padding;
            let right_padding = entry.right_padding;

            let space = left_padding + length + right_padding;
            let name = if name.len() > space {
                &name[0..space]
            } else {
                name
            };

            let left_space = (space - name.len()) / 2;
            let right_space = space - left_space - name.len();

            for _ in 0..left_space {
                write!(temp_writer, " ")?;
            }

            write!(temp_writer, "{}", name)?;

            for _ in 0..right_space {
                write!(temp_writer, " ")?;
            }
            write!(temp_writer, "|")?;
        }

        let real_writer = self.writer()?;
        write!(real_writer, "|")?;
        // actually write the content here
        write!(real_writer, "{}", String::from_utf8(temp_writer)?)?;
        writeln!(real_writer)?;
        self.print_output_header_line = false;
        Ok(())
    }
}

impl<'w, CMD, SIM> ScriptExecutor<CMD> for BaseScriptExecutor<'w, CMD, SIM>
where
    CMD: SimulatorCommand,
    SIM: SimulatorExecutor<CMD>,
{
    fn exec(&mut self, cmd: Command<CMD>) -> ExecResult {
        match cmd.kind {
            CommandKind::Simulator(sim_cmd) => self.sim.exec_sim(sim_cmd),
            CommandKind::OutputFile(output_file) => self.set_output_file(output_file),
            CommandKind::CompareTo(compare_file) => self.set_compare_file(compare_file),
            CommandKind::OutputList(output_list) => self.set_output_list(output_list),
            CommandKind::Echo(message) => {
                println!("{}", message);
                Ok(())
            }
            CommandKind::Output => {
                self.print_output_header_if_needed()?;

                // see: print_output_header_if_needed
                let mut temp_writer = Vec::new();

                for entry in self.output_list.iter() {
                    let name = &entry.name;
                    let length = entry.length;
                    let left_padding = entry.left_padding;
                    let right_padding = entry.right_padding;
                    let format = entry.format;

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
                        write!(temp_writer, " ")?;
                    }

                    write!(temp_writer, "{}", value_string)?;

                    for _ in 0..right_space {
                        write!(temp_writer, " ")?;
                    }
                    write!(temp_writer, "|")?;
                }

                let real_writer = self.writer()?;
                write!(real_writer, "|")?;
                write!(real_writer, "{}", String::from_utf8(temp_writer)?)?;
                writeln!(real_writer)?;
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

#[derive(Debug)]
pub struct ComparisonError {
    cmp_file_name: String,
    line: usize,
    col: usize,
}

impl fmt::Display for ComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error at {}:{}:{}",
            self.cmp_file_name, self.line, self.col
        )
    }
}

impl Error for ComparisonError {}

pub fn execute_script<'tst, 'w, P, X, C>(
    p: ScriptParser<'tst, P, C>,
    sim_executor: X,
    writer: impl Into<Option<&'w mut dyn Write>>,
) -> ExecResult
where
    C: SimulatorCommand,
    X: SimulatorExecutor<C>,
    ScriptParser<'tst, P, C>: SimulatorCommandParser<C>,
{
    let writer = writer.into();
    let use_outfile = writer.is_none();

    let mut executor = BaseScriptExecutor::new(sim_executor, writer);
    for cmd in p {
        let result = executor.exec(cmd?.content);
        // check if the simulator is done
        // this is not an actual error
        if let Err(e) = &result {
            if !e.is::<Halt>() {
                // only report actual errors, not halting
                result?;
            }
        }
    }

    executor.writer()?.flush()?;

    let (cmp_name, cmp_content) = if let Some(cmp) = &executor.compare_file {
        (
            cmp.to_str().ok_or("Illegal compare file path")?,
            read_to_string(cmp)?.replace("\r\n", "\n"),
        )
    } else {
        ("", "".to_owned())
    };

    let out_content = if use_outfile {
        let out_file = executor.output_file.ok_or("missing output file")?.0;
        read_to_string(out_file)?.replace("\r\n", "\n")
    } else {
        "".to_owned()
    };

    if use_outfile {
        let mut line = 1;
        let mut col = 0;

        for (cmp_c, out_c) in cmp_content.chars().zip(out_content.chars()) {
            // '*' are placeholders
            if cmp_c != out_c && cmp_c != '*' {
                let cmp_file_name = cmp_name.to_owned();
                return Err(Box::new(ComparisonError {
                    cmp_file_name,
                    line,
                    col,
                }));
            }

            col += 1;

            if cmp_c == '\n' {
                line += 1;
                col = 0;
            }
        }

        if cmp_content.len() != out_content.len() {
            let cmp_file_name = cmp_name.to_owned();
            return Err(Box::new(ComparisonError {
                cmp_file_name,
                line,
                col,
            }));
        }
    }

    Ok(())
}

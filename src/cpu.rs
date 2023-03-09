use parse::script::parser::ScriptParser;
use simulators::cpu::Cpu;
use simulators::execute_script;

mod definitions;
mod parse;
mod simulators;

use clap::{arg, command, value_parser, ArgAction};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn execute<'w>(
    tst_name: &Path,
    tst_content: String,
    writer: impl Into<Option<&'w mut dyn Write>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parser = ScriptParser::new(tst_name, &tst_content);
    execute_script(parser, Cpu::default(), writer)
}

fn main() {
    let tst_arg = arg!([file] "The .tst file")
        .required(true)
        .value_parser(value_parser!(PathBuf));

    let use_stdout_arg =
        arg!(--"print-outfile" "Use stdout instead of the output-file in the script runner")
            .action(ArgAction::SetTrue);

    let matches = command!().arg(tst_arg).arg(use_stdout_arg).get_matches();

    let tst_name = matches.get_one::<PathBuf>("file").unwrap();
    let use_stdout = *matches.get_one::<bool>("print-outfile").unwrap();

    let tst_content = fs::read_to_string(tst_name).unwrap();

    let mut out = io::stdout();
    let writer = if use_stdout {
        let out: &mut (dyn Write) = &mut out;
        Some(out)
    } else {
        None
    };

    execute(tst_name, tst_content, writer).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! cpu_path {
        ($name:expr) => {
            concat!(
                concat!(env!("CARGO_MANIFEST_DIR"), "/res/tests/cpu/"),
                $name
            )
        };
    }

    macro_rules! cpu_include {
        ($name:expr) => {{
            include_str!(cpu_path!($name))
        }};
    }

    macro_rules! cpu_test {
        ($name:expr) => {{
            let path = PathBuf::from(cpu_path!($name));
            let content = cpu_include!($name).to_owned();
            (path, content)
        }};
    }

    #[test]
    fn test_07_memory_access_basic_test() {
        let (tst_name, tst_content) = cpu_test!("mult/Mult.tst");
        execute(&tst_name, tst_content, None).unwrap();
    }

    #[test]
    fn test_04_fill_test() {
        let (tst_name, tst_content) = cpu_test!("fill/FillAutomatic.tst");
        execute(&tst_name, tst_content, None).unwrap();
    }
}

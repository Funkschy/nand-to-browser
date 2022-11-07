use parse::assembly::{AssemblyParser, SourceFile};
use simulators::cpu::script::CpuEmulatorCommandParser;
use simulators::cpu::Cpu;
use simulators::execute_script;

mod definitions;
mod parse;
mod simulators;

use clap::{arg, command, value_parser, ArgAction};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::exit;

fn run(cpu: &mut Cpu) {
    println!("No tst file found!");
    println!("Running in headless mode");
    loop {
        cpu.step().unwrap();
    }
}

type FileMap<T = String> = HashMap<T, String>;

fn find_files(dir: &PathBuf) -> Result<(FileMap, FileMap<PathBuf>), Box<dyn std::error::Error>> {
    let mut cpu_files = HashMap::new();
    let mut tst_files = HashMap::new();

    // TODO: only execute test when a flag is set, but in that case ensure that there is a tst script
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let filename = path.file_name();
        let extension = path.extension();

        if let (Some(name), Some(ext)) = (
            filename.and_then(|x| x.to_str()),
            extension.and_then(|x| x.to_str()),
        ) {
            let read = || {
                fs::read_to_string(&path).unwrap_or_else(|_| panic!("Could not read '{}'", name))
            };
            match ext {
                "asm" => {
                    let name = name.to_owned();
                    let content = read();
                    cpu_files.insert(name, content);
                }
                "tst" => {
                    let content = read();
                    tst_files.insert(path, content);
                }
                _ => {}
            };
        }
    }

    Ok((cpu_files, tst_files))
}

pub fn execute<'w>(
    asm_file: SourceFile,
    tst_file: Option<(PathBuf, String)>,
    writer: impl Into<Option<&'w mut dyn Write>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cpu = Cpu::default();
    let program = AssemblyParser::new(asm_file).parse()?;

    cpu.load(program);

    if let Some((tst_name, tst_content)) = tst_file {
        let parser = CpuEmulatorCommandParser::create(&tst_name, &tst_content);
        execute_script(parser, cpu, writer)?;
    } else {
        run(&mut cpu);
    }

    Ok(())
}

fn main() {
    let dir_arg = arg!([dir] "The directory which contains the code and tests")
        .required(true)
        .value_parser(value_parser!(PathBuf));

    let use_stdout_arg =
        arg!(--"print-outfile" "Use stdout instead of the output-file in the script runner")
            .action(ArgAction::SetTrue);

    let matches = command!().arg(dir_arg).arg(use_stdout_arg).get_matches();

    let dir = matches.get_one::<PathBuf>("dir").unwrap();
    let use_stdout = *matches.get_one::<bool>("print-outfile").unwrap();

    // load the .asm and .tst, files in the given directory
    let (asm_files, mut tst_files) = find_files(dir).unwrap();

    if tst_files.len() > 2 {
        println!("Expected no more than 2 test scripts");
        exit(1);
    }

    if tst_files.len() == 2 {
        tst_files.retain(|k, _| {
            k.file_name()
                .and_then(|n| n.to_str())
                .map(|n| !n.ends_with("VME.tst"))
                .unwrap_or(false)
        });
    }

    let mut out = io::stdout();
    let writer = if use_stdout {
        let out: &mut (dyn Write) = &mut out;
        Some(out)
    } else {
        None
    };

    let (_, asm_content) = asm_files.into_iter().next().expect("No asm file in folder");
    let tst_file = tst_files.into_iter().next();

    execute(SourceFile::new(&asm_content), tst_file, writer).unwrap();
}

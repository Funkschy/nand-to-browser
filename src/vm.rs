use parse::bytecode::{Parser, SourceFile};
use simulators::execute_script;
use simulators::vm::script::VMEmulatorCommandParser;
use simulators::vm::stdlib::Stdlib;
use simulators::vm::VM;

mod definitions;
mod keyboard;
mod parse;
mod simulators;

use clap::{arg, command, value_parser, ArgAction};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::exit;

#[cfg(feature = "desktop")]
fn run(vm: &mut VM, steps_per_tick: usize) {
    use definitions::{SCREEN_HEIGHT, SCREEN_WIDTH};
    use keyboard::get_key_code;
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::{Color, PixelFormatEnum};

    let logical_width = SCREEN_WIDTH as u32;
    let logical_height = SCREEN_HEIGHT as u32;
    let scale = 4;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window(
            "Nand to Tetris VM Emulator",
            logical_width * scale,
            logical_height * scale,
        )
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();
    let mut bg_texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, logical_width, logical_height)
        .unwrap();

    // only scale by integers instead of fractions to keep everything crisp
    canvas.set_integer_scale(true).unwrap();
    canvas
        .set_logical_size(logical_width, logical_height)
        .unwrap();

    canvas.set_draw_color(Color::RGB(255, 255, 255));

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        canvas.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    if let Some(code) = get_key_code("ArrowLeft") {
                        vm.set_input_key(code).unwrap();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    if let Some(code) = get_key_code("ArrowUp") {
                        vm.set_input_key(code).unwrap();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    if let Some(code) = get_key_code("ArrowRight") {
                        vm.set_input_key(code).unwrap();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    if let Some(code) = get_key_code("ArrowDown") {
                        vm.set_input_key(code).unwrap();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    ..
                } => {
                    if let Some(code) = get_key_code("Enter") {
                        vm.set_input_key(code).unwrap();
                    }
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    // pretty inefficient, but the js version already receives a string
                    // and since that is the main frontend, this can be tolerated
                    if let Some(code) = get_key_code(&keycode.to_string()) {
                        vm.set_input_key(code).unwrap();
                    }
                }
                Event::KeyUp { .. } => {
                    vm.set_input_key(0).unwrap();
                }
                _ => {}
            }
        }

        for _ in 0..steps_per_tick {
            vm.step().unwrap();
        }

        let words_per_row = 32;

        bg_texture
            .with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                let display = vm.display();

                let mut i = 0;

                for y in 0..logical_height {
                    for x in 0..words_per_row {
                        let word = display[(y * words_per_row + x) as usize];
                        for pixel_idx in 0..16 {
                            let mask = 1 << pixel_idx;
                            let value = word & mask;
                            let color = if value == 0 { 255 } else { 0 };

                            buffer[i + 0] = color;
                            buffer[i + 1] = color;
                            buffer[i + 2] = color;
                            i += 3;
                        }
                    }
                }
            })
            .unwrap();

        canvas.copy(&bg_texture, None, None).unwrap();
        canvas.present();
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

#[cfg(not(feature = "desktop"))]
fn run(vm: &mut VM, _: usize) {
    println!("You are running in headless mode!");
    println!("If you want to see the program being executed,");
    println!("you will need to compile the application with the desktop feature enabled");
    loop {
        vm.step().expect("vm error");
    }
}

type FileMap<T = String> = HashMap<T, String>;

fn find_files(dir: &PathBuf) -> Result<(FileMap, FileMap<PathBuf>), Box<dyn std::error::Error>> {
    let mut vm_files = HashMap::new();
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
                "vm" => {
                    let name = name.to_owned();
                    let content = read();
                    vm_files.insert(name, content);
                }
                "tst" => {
                    let content = read();
                    tst_files.insert(path, content);
                }
                _ => {}
            };
        }
    }

    Ok((vm_files, tst_files))
}

pub fn execute<'w>(
    use_vm_stdlib: bool,
    steps_per_tick: usize,
    vm_files: FileMap,
    tst_files: FileMap<PathBuf>,
    writer: impl Into<Option<&'w mut dyn Write>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let stdlib = if use_vm_stdlib {
        Stdlib::default()
    } else {
        Stdlib::new()
    };

    let mut vm = VM::new(stdlib.clone());
    let mut programs = vm_files
        .iter()
        .map(|(name, content)| SourceFile::new(name, content))
        .collect::<Vec<_>>();

    // load the VM implementation of the stdlib if requested
    if use_vm_stdlib {
        let sys = include_str!("../res/stdlib/Sys.vm");
        let array = include_str!("../res/stdlib/Array.vm");
        let keyboard = include_str!("../res/stdlib/Keyboard.vm");
        let math = include_str!("../res/stdlib/Math.vm");
        let memory = include_str!("../res/stdlib/Memory.vm");
        let output = include_str!("../res/stdlib/Output.vm");
        let screen = include_str!("../res/stdlib/Screen.vm");
        let string = include_str!("../res/stdlib/String.vm");

        programs.push(SourceFile::new("Sys.vm", sys));
        programs.push(SourceFile::new("Array.vm", array));
        programs.push(SourceFile::new("Keyboard.vm", keyboard));
        programs.push(SourceFile::new("Math.vm", math));
        programs.push(SourceFile::new("Memory.vm", memory));
        programs.push(SourceFile::new("Output.vm", output));
        programs.push(SourceFile::new("Screen.vm", screen));
        programs.push(SourceFile::new("String.vm", string));
    }

    let program = Parser::with_stdlib(programs, stdlib).parse()?;

    vm.load(program);

    if !tst_files.is_empty() {
        let (tst_name, tst_content) = tst_files.into_iter().next().unwrap();
        let parser = VMEmulatorCommandParser::create(&tst_name, tst_content.as_str());
        execute_script(parser, vm, writer)?;
    } else {
        run(&mut vm, steps_per_tick);
    }

    Ok(())
}

fn main() {
    let dir_arg = arg!([dir] "The directory which contains the code and tests")
        .required(true)
        .value_parser(value_parser!(PathBuf));

    let step_arg = arg!(-s --steps <STEPS> "How many steps per tick should be executed")
        .value_parser(value_parser!(usize))
        .default_value("30000");

    let use_vm_arg = arg!(--vm "Use the VM stdlib implementations").action(ArgAction::SetTrue);
    let use_stdout_arg =
        arg!(--"print-outfile" "Use stdout instead of the output-file in the script runner")
            .action(ArgAction::SetTrue);

    let matches = command!()
        .arg(dir_arg)
        .arg(step_arg)
        .arg(use_vm_arg)
        .arg(use_stdout_arg)
        .get_matches();

    let dir = matches.get_one::<PathBuf>("dir").unwrap();
    let steps_per_tick = *matches.get_one::<usize>("steps").unwrap();
    let use_vm_stdlib = *matches.get_one::<bool>("vm").unwrap();
    let use_stdout = *matches.get_one::<bool>("print-outfile").unwrap();

    // load the files .vm and .tst, files in the given directory
    let (vm_files, mut tst_files) = find_files(dir).unwrap();

    if tst_files.len() > 2 {
        println!("Expected no more than 2 test scripts");
        exit(1);
    }

    if tst_files.len() == 2 {
        tst_files.retain(|k, _| {
            k.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with("VME.tst"))
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

    execute(use_vm_stdlib, steps_per_tick, vm_files, tst_files, writer).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! vm_test {
        ($name:expr) => {
            concat!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/tests/vm/"), $name)
        };
    }

    macro_rules! vm_filename_map {
        ($name:expr) => {{
            let path = PathBuf::from(vm_test!($name));
            let content = include_str!(vm_test!($name));
            let mut m = HashMap::new();
            m.insert(
                path.file_name().unwrap().to_str().unwrap().to_string(),
                content.to_owned(),
            );
            m
        }};
    }

    macro_rules! vm_filepath_map {
        ($name:expr) => {{
            let path = PathBuf::from(vm_test!($name));
            let content = include_str!(vm_test!($name));
            let mut m = HashMap::new();
            m.insert(path, content.to_owned());
            m
        }};
    }

    #[test]
    fn test_07_memory_access_basic_test() {
        let vm = vm_filename_map!("BasicTest/BasicTest.vm");
        let tst = vm_filepath_map!("BasicTest/BasicTestVME.tst");

        let mut v = Vec::new();
        let w: &mut (dyn Write) = &mut v;
        execute(false, 1, vm, tst, w).unwrap();

        // this would usually not happen here, but instead inside of execute
        let cmp = include_str!(vm_test!("BasicTest/BasicTest.cmp")).replace("\r\n", "\n");
        let res = String::from_utf8(v).unwrap();

        assert_eq!(cmp, res);
    }
}

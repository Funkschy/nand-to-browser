use crate::simulators::vm::stdlib::Stdlib;
use parse::bytecode::{Parser, SourceFile};
use simulators::vm::VM;

mod definitions;
mod keyboard;
mod parse;
mod simulators;

use clap::{arg, command, value_parser, ArgAction};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use walkdir::WalkDir;

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

type FileMap = HashMap<String, String>;

fn find_files(dir: &PathBuf) -> (FileMap, FileMap, FileMap) {
    let mut vm_files = HashMap::new();
    let mut tst_files = HashMap::new();
    let mut cmp_files = HashMap::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
    {
        let path = entry.into_path();
        let filename = path.file_name();
        let extension = path.extension();

        if let (Some(name), Some(ext)) = (
            filename.and_then(|x| x.to_str()),
            extension.and_then(|x| x.to_str()),
        ) {
            macro_rules! add {
                ($map:ident) => {{
                    let name = name.to_owned();
                    let content =
                        fs::read_to_string(&path).expect(&format!("Could not read '{}'", name));
                    $map.insert(name, content);
                }};
            }

            match ext {
                "vm" => add!(vm_files),
                "tst" => add!(tst_files),
                "cmp" => add!(cmp_files),
                _ => {}
            };
        }
    }

    (vm_files, tst_files, cmp_files)
}

fn main() {
    let dir_arg = arg!([dir] "The directory which contains the code and tests")
        .required(true)
        .value_parser(value_parser!(PathBuf));

    let step_arg = arg!(-s --steps <STEPS> "How many steps per tick should be executed")
        .value_parser(value_parser!(usize))
        .default_value("30000");

    let use_vm_arg = arg!(--vm "Use the VM stdlib implementations").action(ArgAction::SetTrue);

    let matches = command!()
        .arg(dir_arg)
        .arg(step_arg)
        .arg(use_vm_arg)
        .get_matches();

    let dir = matches.get_one::<PathBuf>("dir").unwrap();
    let steps_per_tick = *matches.get_one::<usize>("steps").unwrap();
    let use_vm_stdlib = *matches.get_one::<bool>("vm").unwrap();

    // load the files .vm, .tst, and .cmp files in the given directory
    let (vm_files, tst_files, cmp_files) = find_files(dir);

    if tst_files.len() > 1 {
        println!("Expected 0 or 1 test scripts");
        exit(1);
    }

    if cmp_files.len() > 1 {
        println!("Expected 0 or 1 compare files");
        exit(2);
    }

    // let (tst_path, tst_content) = tst_files.into_iter().next().unwrap();
    // let (cmp_path, cmp_content) = cmp_files.into_iter().next().unwrap();

    let mut vm = VM::new(Stdlib::new());

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

    let program = Parser::with_stdlib(programs, Stdlib::new())
        .parse()
        .unwrap();

    vm.load(program);

    run(&mut vm, steps_per_tick);
}

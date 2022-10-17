use crate::simulators::vm::stdlib::Stdlib;
use parse::bytecode::{Parser, SourceFile};
use simulators::vm::VM;

mod definitions;
#[cfg(feature = "desktop")]
mod keyboard;
mod parse;
mod simulators;

#[cfg(feature = "desktop")]
fn run_desktop(vm: &mut VM) {
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
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();
    let mut bg_texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, logical_width, logical_height)
        .unwrap();

    // only scale by integers instead of fractions to keep everything crisp
    canvas.set_integer_scale(true).unwrap();

    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    // TODO: remove this after moving the keyboard handling into rust
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
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
                    keycode: Some(Keycode::PageDown),
                    ..
                } => {
                    if let Some(code) = get_key_code("PageDown") {
                        vm.set_input_key(code).unwrap();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Backspace),
                    ..
                } => {
                    if let Some(code) = get_key_code("Backspace") {
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

        for _ in 0..20000 {
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
fn run_desktop(vm: &mut VM) {
    loop {
        vm.step().expect("vm error");
    }
}

fn main() {
    let mut vm = VM::new(Stdlib::new());

    // let sys = include_str!("../res/stdlib/Sys.vm");
    // let array = include_str!("../res/stdlib/Array.vm");
    // let keyboard = include_str!("../res/stdlib/Keyboard.vm");
    // let math = include_str!("../res/stdlib/Math.vm");
    // let memory = include_str!("../res/stdlib/Memory.vm");
    // let output = include_str!("../res/stdlib/Output.vm");
    // let screen = include_str!("../res/stdlib/Screen.vm");
    // let string = include_str!("../res/stdlib/String.vm");

    let main = include_str!("../res/hackenstein/Main.vm");
    let display = include_str!("../res/hackenstein/Display.vm");
    let walls = include_str!("../res/hackenstein/Walls.vm");
    let player = include_str!("../res/hackenstein/Player.vm");

    // let test = include_str!("/home/felix/Downloads/nand2tetris/projects/12/OutputTest/Main.vm");

    let programs = vec![
        // SourceFile::new("Test.vm", test),
        SourceFile::new("Main.vm", main),
        SourceFile::new("Display.vm", display),
        SourceFile::new("Walls.vm", walls),
        SourceFile::new("Player.vm", player),
    ];

    let mut bytecode_parser = Parser::with_stdlib(programs, Stdlib::new());
    let program = bytecode_parser.parse().unwrap();

    vm.load(program);

    run_desktop(&mut vm);
}

use parse::bytecode::{Parser, SourceFile};
use simulators::vm::VM;

mod definitions;
mod parse;
mod simulators;

#[cfg(feature = "desktop")]
fn run_desktop(vm: &mut VM) {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::{Color, PixelFormatEnum};
    use std::time::Duration;

    let logical_width = 512;
    let logical_height = 256;
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
    canvas.set_integer_scale(true);

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
                    vm.set_input_key(130);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    vm.set_input_key(131);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    vm.set_input_key(132);
                }
                Event::KeyUp { .. } => {
                    vm.set_input_key(0);
                }
                _ => {}
            }
        }

        for _ in 0..10000 {
            vm.step();
        }

        let words_per_row = 32;

        bg_texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
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
    vm.run();
}

fn main() {
    let mut vm = VM::default();

    let sys = include_str!("../res/stdlib/Sys.vm");
    let array = include_str!("../res/stdlib/Array.vm");
    let keyboard = include_str!("../res/stdlib/Keyboard.vm");
    let math = include_str!("../res/stdlib/Math.vm");
    let memory = include_str!("../res/stdlib/Memory.vm");
    let output = include_str!("../res/stdlib/Output.vm");
    let screen = include_str!("../res/stdlib/Screen.vm");
    let string = include_str!("../res/stdlib/String.vm");
    let main = include_str!("../res/tetris/Main.vm");
    let random = include_str!("../res/tetris/Random.vm");
    let render = include_str!("../res/tetris/Render.vm");
    let tetromino = include_str!("../res/tetris/Tetromino.vm");

    let programs = vec![
        SourceFile::new("Sys.vm", sys),
        SourceFile::new("Keyboard.vm", keyboard),
        SourceFile::new("Math.vm", math),
        SourceFile::new("Memory.vm", memory),
        SourceFile::new("Array.vm", array),
        SourceFile::new("Output.vm", output),
        SourceFile::new("Screen.vm", screen),
        SourceFile::new("String.vm", string),
        SourceFile::new("Main.vm", main),
        SourceFile::new("Random.vm", random),
        SourceFile::new("Render.vm", render),
        SourceFile::new("Tetromino.vm", tetromino),
    ];

    let mut bytecode_parser = Parser::new(programs);
    let program = bytecode_parser.parse().unwrap();

    vm.load(program.opcodes, program.debug_symbols);

    run_desktop(&mut vm);
}

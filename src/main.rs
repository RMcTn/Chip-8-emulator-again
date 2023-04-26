mod chip;

use chip::*;

use std::time::Duration;
// TODO(reece): Write an assembler for this as well using this reference
// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#3.1
// Add in assembly labels for jumps or loading into register
//
// bunch of useful ROMs https://github.com/kripod/chip8-roms

use sdl2::{
    audio::{AudioCallback, AudioSpecDesired},
    event::Event,
    keyboard::Keycode,
    pixels::Color,
    rect::Rect,
    render::Canvas,
};

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    // Audio example taken straight from the rust sdl2 docs
    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1), // mono
        samples: None,     // default sample size
    };

    let device = audio_subsystem
        .open_playback(None, &desired_spec, |spec| {
            // initialize the audio callback
            SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.25,
            }
        })
        .unwrap();
    const DISPLAY_WIDTH_IN_PIXELS: usize = 1024;
    const DISPLAY_HEIGHT_IN_PIXELS: usize = 768;
    let window = video_subsystem
        .window(
            "Chip 8 Emulator",
            DISPLAY_WIDTH_IN_PIXELS as u32,
            DISPLAY_HEIGHT_IN_PIXELS as u32,
        )
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();

    let args: Vec<String> = std::env::args().collect();

    let file_path = if args.len() >= 2 {
        &args[1]
    } else {
        "roms/test_opcode.ch8"
    };

    // Test ROM from https://github.com/corax89/chip8-test-rom
    // More test ROMS from https://github.com/Timendus/chip8-test-suite#chip-8-splash-screen
    let rom_bytes = std::fs::read(file_path).unwrap();

    let mut chip = Chip8::new(&rom_bytes);

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut executing = true;
    let mut step_once = false;
    let scale = 8;

    let mut last_frame_time = std::time::Instant::now();
    let target_frame_time = Duration::from_millis((1.0 / 60.0 * 1000.0) as u64);
    let target_chip_frame_time = Duration::from_millis((1.0 / 60.0 * 1000.0) as u64);

    let mut keys = [false; 16];
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    ..
                } => {
                    executing = !executing;
                    println!("Toggled executing to {}", executing);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::N),
                    ..
                } => {
                    // Just step through
                    executing = false;
                    step_once = true;
                    println!("Stepping once");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::L),
                    ..
                } => {
                    dbg!(chip.display_buffer);
                    // First index where false

                    for (i, b) in chip.display_buffer.iter().enumerate() {
                        if *b == false {
                            println!("False at 0x{:X} ({})", i, i);
                            break;
                        }
                    }
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    // TODO(reece): Have some better mappings/configurable later. Just making it
                    // work for now
                    Keycode::Num0 => keys[0x0] = true,
                    Keycode::Num1 => keys[0x1] = true,
                    Keycode::Num2 => keys[0x2] = true,
                    Keycode::Num3 => keys[0x3] = true,
                    Keycode::Num4 => keys[0x4] = true,
                    Keycode::Num5 => keys[0x5] = true,
                    Keycode::Num6 => keys[0x6] = true,
                    Keycode::Num7 => keys[0x7] = true,
                    Keycode::Num8 => keys[0x8] = true,
                    Keycode::Num9 => keys[0x9] = true,
                    Keycode::A => keys[0xA] = true,
                    Keycode::B => keys[0xB] = true,
                    Keycode::C => keys[0xC] = true,
                    Keycode::D => keys[0xD] = true,
                    Keycode::E => keys[0xE] = true,
                    Keycode::F => keys[0xF] = true,
                    _ => { /* Left blank intentionally. These keys do nothing */ }
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    // TODO(reece): Have some better mappings/configurable later. Just making it
                    // work for now
                    Keycode::Num0 => keys[0x0] = false,
                    Keycode::Num1 => keys[0x1] = false,
                    Keycode::Num2 => keys[0x2] = false,
                    Keycode::Num3 => keys[0x3] = false,
                    Keycode::Num4 => keys[0x4] = false,
                    Keycode::Num5 => keys[0x5] = false,
                    Keycode::Num6 => keys[0x6] = false,
                    Keycode::Num7 => keys[0x7] = false,
                    Keycode::Num8 => keys[0x8] = false,
                    Keycode::Num9 => keys[0x9] = false,
                    Keycode::A => keys[0xA] = false,
                    Keycode::B => keys[0xB] = false,
                    Keycode::C => keys[0xC] = false,
                    Keycode::D => keys[0xD] = false,
                    Keycode::E => keys[0xE] = false,
                    Keycode::F => keys[0xF] = false,
                    _ => { /* Left blank intentionally. These keys do nothing */ }
                },
                _ => {}
            }
        }

        if executing || step_once {
            if step_once {
                chip.process_next_instruction(keys);
                step_once = false;
                executing = false;
                chip.print_registers();
            } else {
                chip.process_a_frame(
                    keys,
                    target_chip_frame_time.as_micros() as u32, // Should be a safe cast, unless someone wants a ridiculously large amount of processing time for a frame
                );
            }
        }

        // TODO(reece): Score isn't updating for Breakout or Pong, but the BCD test passes on
        // 0xFx33 test for Corax+ opcode test rom
        // TODO(reece): There's some flickering, super noticable with breakout game.
        draw_display(&mut canvas, &chip.display_buffer, scale);

        if chip.should_play_sound() {
            device.resume();
        } else {
            device.pause();
        }

        let current_frame_time = std::time::Instant::now();

        let latest_frame_time = current_frame_time - last_frame_time;
        last_frame_time = current_frame_time;

        let time_to_sleep = target_frame_time.saturating_sub(latest_frame_time);

        if !time_to_sleep.is_zero() {
            std::thread::sleep(time_to_sleep);
        }
    }
}

fn draw_display<T: sdl2::render::RenderTarget>(
    canvas: &mut Canvas<T>,
    display_buffer: &[bool],
    scale: i32,
) {
    canvas.clear();
    for x in 0..CHIP_DISPLAY_WIDTH_IN_PIXELS {
        for y in 0..CHIP_DISPLAY_HEIGHT_IN_PIXELS {
            let color = match display_buffer[idx_for_display(x as u8, y as u8)] {
                false => Color::RGB(0, 0, 0),
                true => Color::RGB(120, 64, 127),
            };
            canvas.set_draw_color(color);
            canvas
                .fill_rect(Rect::new(
                    x as i32 * scale,
                    y as i32 * scale,
                    scale as u32,
                    scale as u32,
                ))
                .unwrap();
        }
    }
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.present();
}

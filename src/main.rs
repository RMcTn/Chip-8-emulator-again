mod chip;
mod disassembler;

use chip::*;

use std::{collections::HashMap, time::Duration};
// TODO(reece): Write an assembler for this as well using this reference
// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#3.1
// Add in assembly labels for jumps or loading into register
//
// bunch of useful ROMs https://github.com/kripod/chip8-roms

use sdl2::{
    audio::{AudioCallback, AudioSpecDesired},
    event::Event,
    keyboard::Keycode,
    pixels::{Color, PixelFormatEnum},
    rect::Rect,
    render::{Canvas, Texture},
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

#[derive(PartialEq, Eq, Hash)]
enum Command {
    Step,
    Pause,
}

type Keymap = HashMap<Keycode, Command>;

fn default_keymap() -> Keymap {
    return Keymap::from([(Keycode::P, Command::Pause), (Keycode::N, Command::Step)]);
}

fn main() {
    let assembly_program = vec![
        "JP 0x555".to_string(),
        "LD I, 0x200".to_string(),
        "LD 0x1, 0x3".to_string(),
        "LD 0x0, 0x1".to_string(),
    ];
    let impromptu_rom = disassembler::disassemble(assembly_program);

    let keymap = default_keymap();

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

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            chip::CHIP_DISPLAY_WIDTH_IN_PIXELS as u32,
            chip::CHIP_DISPLAY_HEIGHT_IN_PIXELS as u32,
        )
        .unwrap();

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
                Event::DropFile { filename, .. } => {
                    // TODO(reece): Handle non .ch8 files gracefully!
                    let rom_bytes = std::fs::read(filename).unwrap();
                    chip = Chip8::new(&rom_bytes);
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
                    key => {
                        if let Some(command) = keymap.get(&key) {
                            match command {
                                Command::Step => {
                                    executing = false;
                                    step_once = true;
                                    println!("Stepping once");
                                }
                                Command::Pause => {
                                    executing = !executing;
                                    println!("Toggled executing to {}", executing);
                                }
                            }
                        }
                    }
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

        if chip.should_play_sound() {
            device.resume();
        } else {
            device.pause();
        }

        // TODO(reece): Score isn't updating for Breakout or Pong, but the BCD test passes on
        // 0xFx33 test for Corax+ opcode test rom
        // UPDATE ON BCD - The values are definitely correct, we're just drawing them wrong
        // TODO(reece): There's some flickering, super noticable with breakout game.
        draw_display(&mut canvas, &mut texture, &chip.display_buffer);

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
    texture: &mut Texture,
    display_buffer: &[bool],
) {
    texture
        .with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for x in 0..CHIP_DISPLAY_WIDTH_IN_PIXELS {
                for y in 0..CHIP_DISPLAY_HEIGHT_IN_PIXELS {
                    let display_buffer_idx = idx_for_display(x as u8, y as u8);
                    let color = match display_buffer[display_buffer_idx] {
                        false => Color::RGB(0, 0, 0),
                        true => Color::RGB(255, 255, 255),
                    };

                    let texture_idx = y * pitch + x * 3;
                    let rgb = color.rgb();
                    buffer[texture_idx] = rgb.0;
                    buffer[texture_idx + 1] = rgb.1;
                    buffer[texture_idx + 2] = rgb.2;
                }
            }
        })
        .unwrap();
    canvas.copy(&texture, None, None).unwrap();
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.present();
}

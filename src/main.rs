mod assembler;
mod chip;

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
    PressKeyOnKeypad(u8),
}

type Keymap = HashMap<Keycode, Command>;

fn default_keymap() -> Keymap {
    return Keymap::from([
        (Keycode::P, Command::Pause),
        (Keycode::N, Command::Step),
        (Keycode::Num0, Command::PressKeyOnKeypad(0x0)),
        (Keycode::Num1, Command::PressKeyOnKeypad(0x1)),
        (Keycode::Num2, Command::PressKeyOnKeypad(0x2)),
        (Keycode::Num3, Command::PressKeyOnKeypad(0x3)),
        (Keycode::Num4, Command::PressKeyOnKeypad(0x4)),
        (Keycode::Num5, Command::PressKeyOnKeypad(0x5)),
        (Keycode::Num6, Command::PressKeyOnKeypad(0x6)),
        (Keycode::Num7, Command::PressKeyOnKeypad(0x7)),
        (Keycode::Num8, Command::PressKeyOnKeypad(0x8)),
        (Keycode::Num9, Command::PressKeyOnKeypad(0x9)),
        (Keycode::A, Command::PressKeyOnKeypad(0xA)),
        (Keycode::B, Command::PressKeyOnKeypad(0xB)),
        (Keycode::C, Command::PressKeyOnKeypad(0xC)),
        (Keycode::D, Command::PressKeyOnKeypad(0xD)),
        (Keycode::E, Command::PressKeyOnKeypad(0xE)),
        (Keycode::F, Command::PressKeyOnKeypad(0xF)),
    ]);
}

fn main() {
    let assembly_program = vec![
        "JP 0x202".to_string(),
        "LD I, 0x200".to_string(),
        "LD 0x1, 0x3".to_string(),
        "LD 0x0, 0x1".to_string(),
    ];

    let assembly_program_v2 = "JP 0x202
        SE V2, 0x33
        SE VC, VA
        AND VA, V2
        SKP 0x5
        SKNP 0x5
        LD V1, 0x3
        LD I, 0x200
        LD VA, 0x1
        CALL 0x500
        SNE VC, VA
        SNE VC, 0xAA
        ADD VA, VB
        ADD I, VB
        ADD VC, 0x2
        OR VA, V2
        XOR VA, V2
        SUB VA, V2
        SUB VA, V2
        RND V2, 0x55
        RET
        CLS
        DRW 0x1, 0x2, 0x5
        "
    .to_string();

    let tokens = assembler::parse(assembly_program_v2);
    dbg!(tokens);

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
                                Command::PressKeyOnKeypad(chip_key) => match chip_key {
                                    0x0..=0xF => {
                                        keys[*chip_key as usize] = true;
                                    }
                                    _ => {
                                        eprintln!(
                                            "There is no chip key with value {} (0x{:X})",
                                            *chip_key, *chip_key
                                        );
                                    }
                                },
                            }
                        }
                    }
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    key => {
                        if let Some(command) = keymap.get(&key) {
                            match command {
                                Command::PressKeyOnKeypad(chip_key) => match chip_key {
                                    0x0..=0xF => {
                                        keys[*chip_key as usize] = false;
                                    }
                                    _ => {
                                        eprintln!(
                                            "There is no chip key with value {} (0x{:X})",
                                            *chip_key, *chip_key
                                        );
                                    }
                                },
                                _ => { /* We don't care about keyup events for non chip8 keys */ }
                            }
                        }
                    }
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

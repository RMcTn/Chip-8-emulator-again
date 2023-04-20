use std::time::Duration;

use sdl2::{event::Event, keyboard::Keycode, pixels::Color, rect::Rect};

struct Chip8 {
    memory: [u8; 4096],
    // uppermost 256 bytes (0xF00-0xFFF) potentially reserved for display refresh
    // 96 bytes down from that (0xEA0-0xEFF) is call stack and other internal usage stuff
    //
    address_register: u16,
    data_registers: [u8; 16],
    program_counter: usize,
    i_register: u16, // Holds memory locations. Better name for this?
}

impl Chip8 {
    fn increment_PC(&mut self) {
        self.program_counter += 2;
    }
}

const PROGRAM_OFFSET: usize = 0x200;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

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

    const CHIP_DISPLAY_WIDTH_IN_PIXELS: usize = 64;
    const CHIP_DISPLAY_HEIGHT_IN_PIXELS: usize = 32;
    let mut display_buffer: [Color; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS] =
        [Color::RGB(77, 33, 11); CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS];
    for i in 0..display_buffer.len() {
        display_buffer[i] = Color::RGB(i as u8, 64, 255 - i as u8);
    }

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        canvas.clear();
        for x in 0..CHIP_DISPLAY_WIDTH_IN_PIXELS {
            for y in 0..CHIP_DISPLAY_HEIGHT_IN_PIXELS {
                // Doesn't feel like we want to be doing it this way, but it gets something on
                // the screen quick.
                // No idea if it even is displaying the buffer properly
                let width_ratio = 1024 / CHIP_DISPLAY_WIDTH_IN_PIXELS as u32;
                let height_ratio = 768 / CHIP_DISPLAY_HEIGHT_IN_PIXELS as u32;
                let color = display_buffer[x * y];
                canvas.set_draw_color(color);
                canvas
                    .fill_rect(Rect::new(
                        (x as u32 * width_ratio) as i32,
                        (y as u32 * height_ratio) as i32,
                        width_ratio,
                        height_ratio,
                    ))
                    .unwrap();
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                _ => {}
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    // Test ROM from https://github.com/corax89/chip8-test-rom
    let rom_bytes = std::fs::read("test_opcode.ch8").unwrap();

    let mut chip = Chip8 {
        memory: [0; 4096],
        address_register: 0,
        data_registers: [0; 16],
        program_counter: PROGRAM_OFFSET,
        i_register: 0,
    };

    // Initialize Chip8
    for (i, byte) in rom_bytes.iter().enumerate() {
        chip.memory[PROGRAM_OFFSET + i] = *byte;
    }

    loop {
        // TODO(reece): All this could be done in a "tick" or "process_next_instruction" function that would
        // let us control by time how often we process intstructions
        let opcode: u16 = (chip.memory[chip.program_counter] as u16) << 8
            | chip.memory[chip.program_counter + 1] as u16;

        let first_nibble = opcode >> 12;
        let second_nibble = opcode << 4 >> 12;
        println!("{:X}", opcode);
        match first_nibble {
            0x0 => match second_nibble {
                0x0 => {
                    unimplemented_opcode(opcode, first_nibble, second_nibble, chip.program_counter);
                }
                _ => {
                    unimplemented_opcode(opcode, first_nibble, second_nibble, chip.program_counter);
                }
            },
            0x1 => {
                // 1nnn - JP addr
                let address_to_jump = opcode & 0x0FFF;
                chip.program_counter = address_to_jump as usize;
            }
            0x6 => {
                // 6xkk - LD Vx, byte
                let register = second_nibble;
                let val_to_load = (opcode & 0x00FF) as u8;
                chip.data_registers[register as usize] = val_to_load;
                chip.increment_PC();
            }
            0xA => {
                // Annn - LD I, addr
                let val_to_load = opcode & 0x0FFF;
                chip.i_register = val_to_load;
                chip.increment_PC();
            }
            0xD => {
                // DRW Vx, Vy, nibble
                unimplemented_opcode(opcode, first_nibble, second_nibble, chip.program_counter);
            }
            _ => unimplemented_opcode(opcode, first_nibble, second_nibble, chip.program_counter),
        }
    }
}

fn unimplemented_opcode(
    opcode: u16,
    first_nibble: u16,
    second_nibble: u16,
    program_counter: usize,
) {
    panic!(
        "Unimplemented opcode {:X}, first nibble: {:X}, second nibble: {:X}, PC: {:X}",
        opcode, first_nibble, second_nibble, program_counter
    );
}

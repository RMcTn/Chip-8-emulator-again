use std::time::Duration;

use sdl2::{event::Event, keyboard::Keycode, pixels::Color, rect::Rect};

const CHIP_DISPLAY_WIDTH_IN_PIXELS: usize = 64;
const CHIP_DISPLAY_HEIGHT_IN_PIXELS: usize = 32;

#[derive(Debug)]
struct Chip8 {
    memory: [u8; 4096],
    // uppermost 256 bytes (0xF00-0xFFF) potentially reserved for display refresh
    // 96 bytes down from that (0xEA0-0xEFF) is call stack and other internal usage stuff
    //
    address_register: u16,
    data_registers: [u8; 16],
    program_counter: usize,
    i_register: u16, // Holds memory locations. Better name for this?
    display_buffer: [u8; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS],
}

impl Chip8 {
    fn increment_pc(&mut self) {
        self.program_counter += 2;
    }

    fn process_next_instruction(&mut self) {
        let opcode: u16 = (self.memory[self.program_counter] as u16) << 8
            | self.memory[self.program_counter + 1] as u16;

        let first_nibble = opcode >> 12;
        let second_nibble = opcode << 4 >> 12;
        println!("{:X}", opcode);
        match first_nibble {
            0x0 => match second_nibble {
                0x0 => {
                    unimplemented_opcode(opcode, first_nibble, second_nibble, self.program_counter);
                }
                _ => {
                    unimplemented_opcode(opcode, first_nibble, second_nibble, self.program_counter);
                }
            },
            0x1 => {
                // 1nnn - JP addr
                let address_to_jump = opcode & 0x0FFF;
                self.program_counter = address_to_jump as usize;
            }
            0x6 => {
                // 6xkk - LD Vx, byte
                let register = second_nibble;
                let val_to_load = (opcode & 0x00FF) as u8;
                self.data_registers[register as usize] = val_to_load;
                self.increment_pc();
            }
            0xA => {
                // Annn - LD I, addr
                let val_to_load = opcode & 0x0FFF;
                self.i_register = val_to_load;
                self.increment_pc();
            }
            0xD => {
                // DRW Vx, Vy, nibble

                let x_register = opcode & 0x0F00 >> 8;
                let x = self.data_registers[x_register as usize];
                let y_register = opcode & 0x00F0 >> 4;
                let y = self.data_registers[y_register as usize];
                let n_bytes = (opcode & 0x000F) as u8;

                // Read n bytes from memory at position I
                let memory_location = self.memory[self.i_register as usize];
                let bytes_to_draw =
                    &self.memory[memory_location as usize..(memory_location + n_bytes) as usize];

                // Display those bytes as sprites at Vx, Vy
                // Sprites should be XOR'd into the display buffer
                let mut was_collision = false;
                for (i, byte) in bytes_to_draw.iter().enumerate() {
                    let previous_pixel = self.display_buffer[(x * y) as usize + i];
                    self.display_buffer[(x * y) as usize + i] = previous_pixel ^ byte;
                    if previous_pixel == 1 && *byte == 1 {
                        self.data_registers[0xF] = 1;
                        was_collision = true;
                    }
                    if !was_collision {
                        self.data_registers[0xF] = 0;
                    }
                }
                // If ANY pixel set to 0 due to the XOR, set VF to 1, otherwise, set VF to 0
                //
                // TODO(reece) The collision logic with wrap
                // TODO(reece) If sprite is outside the screen, wrap around the screen to the same Y coord
                //	Didn't need it for the test program, so just going without this for now

                self.increment_pc();
            }

            _ => unimplemented_opcode(opcode, first_nibble, second_nibble, self.program_counter),
        }
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

    // Test ROM from https://github.com/corax89/chip8-test-rom
    let rom_bytes = std::fs::read("test_opcode.ch8").unwrap();

    let mut chip = Chip8 {
        memory: [0; 4096],
        address_register: 0,
        data_registers: [0; 16],
        program_counter: PROGRAM_OFFSET,
        i_register: 0,
        display_buffer: [0; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS],
    };

    // Initialize Chip8

    // Fonts sit at the start of memory
    for (i, byte) in FONT_SPRITES.iter().enumerate() {
        chip.memory[i] = *byte;
    }

    for (i, byte) in rom_bytes.iter().enumerate() {
        chip.memory[PROGRAM_OFFSET + i] = *byte;
    }

    let mut event_pump = sdl_context.event_pump().unwrap();
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
                _ => {}
            }
        }

        chip.process_next_instruction();

        canvas.clear();
        for x in 0..CHIP_DISPLAY_WIDTH_IN_PIXELS {
            for y in 0..CHIP_DISPLAY_HEIGHT_IN_PIXELS {
                // Doesn't feel like we want to be doing it this way, but it gets something on
                // the screen quick.
                // No idea if it even is displaying the buffer properly
                let width_ratio = 1024 / CHIP_DISPLAY_WIDTH_IN_PIXELS as u32;
                let height_ratio = 768 / CHIP_DISPLAY_HEIGHT_IN_PIXELS as u32;
                let color = match chip.display_buffer[x * y] {
                    1 => Color::RGB(255, 255, 255),
                    _ => Color::RGB(0, 0, 0),
                };
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

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
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

const FONT_SPRITE_LENGTH_IN_BYTES: usize = 5;
const NUMBER_OF_FONT_SPRITES: usize = 16; // 0 - F
                                          //
#[rustfmt::skip]
const FONT_SPRITES: [u8; FONT_SPRITE_LENGTH_IN_BYTES * NUMBER_OF_FONT_SPRITES] = [
//0
    0b01100000,
    0b10010000,
    0b10010000,
    0b10010000,
    0b01100000,

//1
    0b01100000,
    0b00100000,
    0b00100000,
    0b00100000,
    0b01110000,

//2
    0b11100000,
    0b00010000,
    0b00110000,
    0b01100000,
    0b11110000,
//3
    0b11100000,
    0b00010000,
    0b01100000,
    0b00010000,
    0b11100000,
//4
    0b10100000,
    0b10100000,
    0b11100000,
    0b00100000,
    0b00100000,
//5
    0b11110000,
    0b10000000,
    0b11110000,
    0b00010000,
    0b11110000,
//6
    0b10000000,
    0b10000000,
    0b11110000,
    0b10010000,
    0b11110000,
//7
    0b11110000,
    0b00010000,
    0b00100000,
    0b00100000,
    0b00100000,
//8
    0b11110000,
    0b10010000,
    0b11110000,
    0b10010000,
    0b11110000,
//9
    0b11110000,
    0b10010000,
    0b11110000,
    0b00010000,
    0b00010000,
//A
    0b01100000,
    0b10010000,
    0b11110000,
    0b10010000,
    0b10010000,
//B
    0b10000000,
    0b10000000,
    0b11110000,
    0b10010000,
    0b11110000,
//C
    0b11110000,
    0b10000000,
    0b10000000,
    0b10000000,
    0b11110000,
//D
    0b11100000,
    0b10010000,
    0b10010000,
    0b10010000,
    0b11100000,
//E
    0b11110000,
    0b10000000,
    0b11100000,
    0b10000000,
    0b11110000,
//F
    0b11110000,
    0b10000000,
    0b11100000,
    0b10000000,
    0b10000000,
];

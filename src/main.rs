use std::time::Duration;
// TODO(reece): Write an assembler for this as well using this reference
// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#3.1
// Add in assembly labels for jumps or loading into register

use sdl2::{event::Event, keyboard::Keycode, pixels::Color, rect::Rect};

const CHIP_DISPLAY_WIDTH_IN_PIXELS: usize = 64;
const CHIP_DISPLAY_HEIGHT_IN_PIXELS: usize = 32;

#[derive(Debug)]
struct Chip8 {
    memory: [u8; 4096],
    // uppermost 256 bytes (0xF00-0xFFF) potentially reserved for display refresh
    // 96 bytes down from that (0xEA0-0xEFF) is call stack and other internal usage stuff
    //
    // was this meant to be i_register or something?
    address_register: u16,
    data_registers: [u8; 16],
    program_counter: usize,
    stack: [u16; 16],
    stack_pointer: u8,
    // Holds memory locations. Better name for this?
    i_register: u16,
    display_buffer: [bool; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS],
}

fn last_byte(val: u16) -> u8 {
    (val & 0x00FF) as u8
}

fn first_byte(val: u16) -> u8 {
    ((val & 0xFF00) >> 8) as u8
}

fn first_nibble(val: u8) -> u8 {
    (val & 0xF0) >> 4
}

fn last_nibble(val: u8) -> u8 {
    val & 0x0F
}

impl Chip8 {
    fn increment_pc(&mut self) {
        self.program_counter += 2;
    }

    fn print_registers(&self) {
        println!("Program Counter: {:X}", self.program_counter);
        println!("I register: {:X}", self.i_register);
        println!("Stack pointer : {:X}", self.stack_pointer);
        for (register, value) in self.data_registers.iter().enumerate() {
            println!("Register {:X}: {:X}", register, value);
        }
    }

    fn process_next_instruction(&mut self) {
        // Opcodes and most documentation taken from http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#3.1
        let opcode: u16 = (self.memory[self.program_counter] as u16) << 8
            | self.memory[self.program_counter + 1] as u16;

        let first_nibble_first_byte = first_nibble(first_byte(opcode));
        let second_nibble_first_byte = last_nibble(first_byte(opcode));
        println!("PC: {:X}, op: {:X}", self.program_counter, opcode);
        match first_nibble_first_byte {
            0x0 => match last_byte(opcode) {
                0xE0 => {
                    self.display_buffer =
                        [false; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS];
                    self.increment_pc();
                }
                0xEE => {
                    // 00EE - RET
                    // Return from a subroutine.
                    self.program_counter = self.stack[self.stack_pointer as usize] as usize;
                    self.stack_pointer -= 1;
                    self.increment_pc();
                }
                _ => {
                    unimplemented_opcode(
                        opcode,
                        first_nibble_first_byte,
                        second_nibble_first_byte,
                        self.program_counter,
                    );
                }
            },
            0x1 => {
                // 1nnn - Jump (JP) addr
                let address_to_jump = opcode & 0x0FFF;
                self.program_counter = address_to_jump as usize;
            }
            0x2 => {
                // 2nnn - CALL addr
                // Call subroutine at nnn.

                // TODO(reece): Pretty sure this means we're missing out the first stack place always
                self.stack_pointer += 1;
                self.stack[self.stack_pointer as usize] = self.program_counter as u16;
                self.program_counter = (opcode & 0x0FFF) as usize;
            }
            0x3 => {
                // 3xkk - Skip Equal (SE) Vx, byte
                // Skip next instruction if Vx = kk.
                let register = second_nibble_first_byte;
                let val_to_compare = last_byte(opcode);

                self.increment_pc();

                if self.data_registers[register as usize] == val_to_compare {
                    self.increment_pc();
                }
            }
            0x4 => {
                // 4xkk - Skip Not Equal (SNE) Vx, byte
                // Skip next instruction if Vx != kk.
                let register = second_nibble_first_byte;
                let val_to_compare = last_byte(opcode);

                self.increment_pc();

                if self.data_registers[register as usize] != val_to_compare {
                    self.increment_pc();
                }
            }
            0x5 => {
                // 5xy0 - Skip Equal (SE) Vx, Vy
                // Skip next instruction if Vx = Vy.
                let x_register = last_nibble(first_byte(opcode));
                let y_register = first_nibble(last_byte(opcode));
                let x_val = self.data_registers[x_register as usize];
                let y_val = self.data_registers[y_register as usize];

                self.increment_pc();

                if x_val == y_val {
                    self.increment_pc();
                }
            }
            0x6 => {
                // 6xkk - Load (LD) Vx, byte
                // Set Vx = kk.
                let register = second_nibble_first_byte;
                let val_to_load = last_byte(opcode);
                self.data_registers[register as usize] = val_to_load;
                self.increment_pc();
            }
            0x7 => {
                // 7xkk - ADD Vx, byte
                // Set Vx = Vx + kk.
                let register = second_nibble_first_byte;
                let register_val = self.data_registers[register as usize];
                let val_to_add = last_byte(opcode);
                self.data_registers[register as usize] = val_to_add.wrapping_add(register_val);
                self.increment_pc();
            }
            0x8 => {
                // Always gonna use register_x and register_y here
                let x_register = last_nibble(first_byte(opcode));
                let y_register = first_nibble(last_byte(opcode));
                let x = self.data_registers[x_register as usize];
                let y = self.data_registers[y_register as usize];
                match last_nibble(last_byte(opcode)) {
                    0 => {
                        // 8xy0 - LD Vx, Vy
                        // Set Vx = Vy.

                        self.data_registers[x_register as usize] =
                            self.data_registers[y_register as usize];
                        self.increment_pc();
                    }
                    1 => {
                        // 8xy1 - OR Vx, Vy
                        // Set Vx = Vx OR Vy.
                        self.data_registers[x_register as usize] = x | y;
                        self.increment_pc();
                    }
                    2 => {
                        // 8xy2 - AND Vx, Vy
                        // Set Vx = Vx AND Vy.
                        self.data_registers[x_register as usize] = x & y;
                        self.increment_pc();
                    }
                    3 => {
                        // 8xy3 - XOR Vx, Vy
                        // Set Vx = Vx XOR Vy.
                        self.data_registers[x_register as usize] = x ^ y;
                        self.increment_pc();
                    }
                    4 => {
                        // 8xy4 - ADD Vx, Vy
                        // Set Vx = Vx + Vy, set VF = carry.
                        let (new_val, overflow_happened) = x.overflowing_add(y);
                        self.data_registers[0xF] = overflow_happened as u8;
                        self.data_registers[x_register as usize] = new_val;
                        self.increment_pc();
                    }
                    5 => {
                        // 8xy5 - SUB Vx, Vy
                        // Set Vx = Vx - Vy, set VF = NOT borrow. (VF = Vx > Vy)
                        let new_val = x.wrapping_sub(y);
                        if x > y {
                            self.data_registers[0xF] = 1;
                        } else {
                            self.data_registers[0xF] = 0;
                        }
                        self.data_registers[x_register as usize] = new_val;
                        self.increment_pc();
                    }
                    _ => unimplemented_opcode(
                        opcode,
                        first_nibble_first_byte,
                        second_nibble_first_byte,
                        self.program_counter,
                    ),
                }
            }
            0x9 => {
                // 9xy0 - Skip Not Equal (SNE) Vx, Vy
                // Skip next instruction if Vx != Vy.
                let x_register = last_nibble(first_byte(opcode));
                let y_register = first_nibble(last_byte(opcode));
                let x_val = self.data_registers[x_register as usize];
                let y_val = self.data_registers[y_register as usize];

                self.increment_pc();

                if x_val != y_val {
                    self.increment_pc();
                }
            }
            0xA => {
                // Annn - Load (LD) I, addr
                // Set I = nnn.
                let val_to_load = opcode & 0x0FFF;
                self.i_register = val_to_load;
                self.increment_pc();
            }
            0xD => {
                // Draw (DRW) Vx, Vy, nibble
                // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
                // Sprites are XOR'd onto the screen.
                // If ANY pixel set to 0 due to the XOR, then a collision has happened.
                let x_register = last_nibble(first_byte(opcode));
                let x = self.data_registers[x_register as usize];
                let y_register = first_nibble(last_byte(opcode));
                let y = self.data_registers[y_register as usize];
                let n_bytes = last_nibble(last_byte(opcode));

                // Read n bytes from memory at position I
                let memory_location = self.i_register as usize;
                let bytes_to_draw = &self.memory
                    [memory_location as usize..(memory_location as usize + n_bytes as usize)];

                dbg!(n_bytes);
                dbg!(bytes_to_draw);
                // Display those bytes as sprites at Vx, Vy
                // Sprites should be XOR'd into the display buffer
                let mut was_collision = false;

                // TODO(reece) The collision logic with wrap
                // TODO(reece) If sprite is outside the screen, wrap around the screen to the same Y coord
                //	Didn't need it for the test program, so just going without this for now
                println!("Drawing at X {}", x);
                println!("Drawing at Y {}", y);
                for (i, byte) in bytes_to_draw.iter().enumerate() {
                    let byte = *byte;
                    for bit_position in 0..8 {
                        let bit_is_set = ((byte >> 7 - bit_position) & 0x1) > 0;

                        // TODO(reece): Pixel wrapping

                        if Chip8::set_pixel(
                            self.display_buffer.as_mut_slice(),
                            (x + bit_position) as usize,
                            y as usize + i,
                            bit_is_set,
                        ) {
                            was_collision = true;
                        }
                    }
                }
                if !was_collision {
                    self.data_registers[0xF] = 0;
                }

                self.increment_pc();
            }

            _ => unimplemented_opcode(
                opcode,
                first_nibble_first_byte,
                second_nibble_first_byte,
                self.program_counter,
            ),
        }
    }

    /// XOR's the pixel at x,y with value.
    /// Returns true if the pixel was set to 0 as a result of the XOR, false otherwise
    fn set_pixel(display_buffer: &mut [bool], x: usize, y: usize, value: bool) -> bool {
        let idx = idx_for_display(x as u8, y as u8);
        let previous_pixel = display_buffer[idx];
        display_buffer[idx] ^= value;
        if previous_pixel == true && value {
            return true;
        }
        return false;
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

    let args: Vec<String> = std::env::args().collect();

    let file_path = if args.len() >= 2 {
        &args[1]
    } else {
        "test_opcode.ch8"
    };

    // Test ROM from https://github.com/corax89/chip8-test-rom
    // More test ROMS from https://github.com/Timendus/chip8-test-suite#chip-8-splash-screen
    let rom_bytes = std::fs::read(file_path).unwrap();

    let mut chip = Chip8 {
        memory: [0; 4096],
        address_register: 0,
        data_registers: [0; 16],
        program_counter: PROGRAM_OFFSET,
        i_register: 0,
        display_buffer: [false; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS],
        stack_pointer: 0,
        stack: [0; 16],
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
    let mut executing = true;
    let mut step_once = true;
    let scale = 8;
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
                    keycode: Some(Keycode::D),
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
                _ => {}
            }
        }

        if executing || step_once {
            chip.process_next_instruction();
            if step_once {
                step_once = false;
                executing = false;
                chip.print_registers();
            }
        }

        canvas.clear();
        for x in 0..CHIP_DISPLAY_WIDTH_IN_PIXELS {
            for y in 0..CHIP_DISPLAY_HEIGHT_IN_PIXELS {
                let color = match chip.display_buffer[idx_for_display(x as u8, y as u8)] {
                    false => Color::RGB(0, 0, 0),
                    true => Color::RGB(120, 64, 127),
                };
                canvas.set_draw_color(color);
                canvas
                    .fill_rect(Rect::new(
                        x as i32 * scale,
                        y as i32 * scale,
                        1 * scale as u32,
                        1 * scale as u32,
                    ))
                    .unwrap();
            }
        }

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.present();
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

fn unimplemented_opcode(opcode: u16, first_nibble: u8, second_nibble: u8, program_counter: usize) {
    panic!(
        "Unimplemented opcode {:X}, first nibble: {:X}, second nibble: {:X}, PC: {:X}",
        opcode, first_nibble, second_nibble, program_counter
    );
}

const FONT_SPRITE_LENGTH_IN_BYTES: usize = 5;
const NUMBER_OF_FONT_SPRITES: usize = 16; // 0 - F

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

fn idx_for_display(x: u8, y: u8) -> usize {
    x as usize + (y as usize * CHIP_DISPLAY_WIDTH_IN_PIXELS)
}

use std::time::Duration;
// TODO(reece): Write an assembler for this as well using this reference
// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#3.1
// Add in assembly labels for jumps or loading into register
//
// bunch of useful ROMs https://github.com/kripod/chip8-roms

use sdl2::{event::Event, keyboard::Keycode, pixels::Color, rect::Rect, render::Canvas};

const CHIP_DISPLAY_WIDTH_IN_PIXELS: usize = 64;
const CHIP_DISPLAY_HEIGHT_IN_PIXELS: usize = 32;

#[derive(Debug)]
struct Chip8 {
    memory: [u8; 4096],
    // uppermost 256 bytes (0xF00-0xFFF) potentially reserved for display refresh
    // 96 bytes down from that (0xEA0-0xEFF) is call stack and other internal usage stuff
    //
    data_registers: [u8; 16],
    program_counter: usize,
    stack: [u16; 16],
    stack_pointer: u8,
    // Holds memory locations. Better name for this?
    i_register: u16,
    display_buffer: [bool; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS],
    delay_timer: u8,
    sound_timer: u8,
    keys: [bool; 16],
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

type TimeTakenInMicroSeconds = u32;

impl Chip8 {
    fn increment_pc(&mut self) {
        self.program_counter += 2;
    }

    fn print_registers(&self) {
        println!("Program Counter: {:X}", self.program_counter);
        println!("I register: {:X}", self.i_register);
        println!("Stack pointer : {:X}", self.stack_pointer);
        for (register, value) in self.data_registers.iter().enumerate() {
            println!("Register {:X}: 0x{:X} ({})", register, value, value);
        }
    }

    fn process_a_frame(&mut self, keys: [bool; 16], processing_time_target: u32) {
        let mut elapsed_time = 0;

        while elapsed_time < processing_time_target {
            elapsed_time += self.process_next_instruction(keys);
        }
    }

    /// Performs the next instruction at the current program counter.
    /// Returns the AVERAGE micro seconds taken to execute that instruction (Does not accurately
    /// emulate timings. See <https://jackson-s.me/2019/07/13/Chip-8-Instruction-Scheduling-and-Frequency.html>)
    fn process_next_instruction(&mut self, keys: [bool; 16]) -> TimeTakenInMicroSeconds {
        // Opcodes and most documentation taken from http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#3.1
        self.keys = keys;
        if self.delay_timer != 0 {
            self.delay_timer -= 1;
        }
        let opcode: u16 = (self.memory[self.program_counter] as u16) << 8
            | self.memory[self.program_counter + 1] as u16;

        let first_nibble_first_byte = first_nibble(first_byte(opcode));
        let second_nibble_first_byte = last_nibble(first_byte(opcode));
        // println!("PC: 0x{:X}, op: 0x{:X}", self.program_counter, opcode);
        let x_register = last_nibble(first_byte(opcode));
        let x = self.data_registers[x_register as usize];
        let y_register = first_nibble(last_byte(opcode));
        let y = self.data_registers[y_register as usize];

        match first_nibble_first_byte {
            0x0 => match last_byte(opcode) {
                0xE0 => {
                    self.display_buffer =
                        [false; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS];
                    self.increment_pc();
                    return 109;
                }
                0xEE => {
                    // 00EE - RET
                    // Return from a subroutine.
                    self.program_counter = self.stack[self.stack_pointer as usize] as usize;
                    self.stack_pointer -= 1;
                    self.increment_pc();
                    return 105;
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
                return 105;
            }
            0x2 => {
                // 2nnn - CALL addr
                // Call subroutine at nnn.

                // TODO(reece): Pretty sure this means we're missing out the first stack place always
                self.stack_pointer += 1;
                self.stack[self.stack_pointer as usize] = self.program_counter as u16;
                self.program_counter = (opcode & 0x0FFF) as usize;
                return 105;
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
                return 55;
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
                return 55;
            }
            0x5 => {
                // 5xy0 - Skip Equal (SE) Vx, Vy
                // Skip next instruction if Vx = Vy.
                self.increment_pc();

                if x == y {
                    self.increment_pc();
                }
                return 73;
            }
            0x6 => {
                // 6xkk - Load (LD) Vx, byte
                // Set Vx = kk.
                let register = second_nibble_first_byte;
                let val_to_load = last_byte(opcode);
                self.data_registers[register as usize] = val_to_load;
                self.increment_pc();
                return 27;
            }
            0x7 => {
                // 7xkk - ADD Vx, byte
                // Set Vx = Vx + kk.
                let register = second_nibble_first_byte;
                let register_val = self.data_registers[register as usize];
                let val_to_add = last_byte(opcode);
                self.data_registers[register as usize] = val_to_add.wrapping_add(register_val);
                self.increment_pc();
                return 45;
            }
            0x8 => {
                // Always gonna use register_x and register_y here
                match last_nibble(last_byte(opcode)) {
                    0x0 => {
                        // 8xy0 - LD Vx, Vy
                        // Set Vx = Vy.
                        self.data_registers[x_register as usize] =
                            self.data_registers[y_register as usize];
                        self.increment_pc();
                    }
                    0x1 => {
                        // 8xy1 - OR Vx, Vy
                        // Set Vx = Vx OR Vy.
                        self.data_registers[x_register as usize] = x | y;
                        self.increment_pc();
                    }
                    0x2 => {
                        // 8xy2 - AND Vx, Vy
                        // Set Vx = Vx AND Vy.
                        self.data_registers[x_register as usize] = x & y;
                        self.increment_pc();
                    }
                    0x3 => {
                        // 8xy3 - XOR Vx, Vy
                        // Set Vx = Vx XOR Vy.
                        self.data_registers[x_register as usize] = x ^ y;
                        self.increment_pc();
                    }
                    0x4 => {
                        // 8xy4 - ADD Vx, Vy
                        // Set Vx = Vx + Vy, set VF = carry.
                        let (new_val, overflow_happened) = x.overflowing_add(y);
                        self.data_registers[x_register as usize] = new_val;
                        self.data_registers[0xF] = overflow_happened as u8;
                        self.increment_pc();
                    }
                    0x5 => {
                        // 8xy5 - SUB Vx, Vy
                        // Set Vx = Vx - Vy, set VF = NOT borrow. (VF = Vx > Vy)
                        let new_val = x.wrapping_sub(y);
                        self.data_registers[x_register as usize] = new_val;
                        if x > y {
                            self.data_registers[0xF] = 1;
                        } else {
                            self.data_registers[0xF] = 0;
                        }
                        self.increment_pc();
                    }
                    0x6 => {
                        // 8xy6 - SHR Vx {, Vy}
                        // Set Vx = Vx SHR 1.
                        // VF is set if LSB is set on Vx
                        self.data_registers[x_register as usize] = x >> 1;
                        self.data_registers[0xF] = x & 0x1;
                        self.increment_pc();
                    }
                    0x7 => {
                        // 8xy7 - SUBN Vx, Vy
                        // Set Vx = Vy - Vx, set VF = NOT borrow. (VF = Vx < Vy)
                        let new_val = y.wrapping_sub(x);
                        self.data_registers[x_register as usize] = new_val;
                        if y > x {
                            self.data_registers[0xF] = 1;
                        } else {
                            self.data_registers[0xF] = 0;
                        }
                        self.increment_pc();
                    }
                    0xE => {
                        // 8xyE - SHL Vx {, Vy}
                        // Set Vx = Vx SHL 1.
                        // VF is set if MSB is set on Vx
                        self.data_registers[x_register as usize] = x << 1;
                        self.data_registers[0xF] = x >> 7;
                        self.increment_pc();
                    }
                    _ => unimplemented_opcode(
                        opcode,
                        first_nibble_first_byte,
                        second_nibble_first_byte,
                        self.program_counter,
                    ),
                }
                // A hard 200 microseconds for all 0x8xxx opcodes, handy!
                return 200;
            }
            0x9 => {
                // 9xy0 - Skip Not Equal (SNE) Vx, Vy
                // Skip next instruction if Vx != Vy.

                self.increment_pc();

                if x != y {
                    self.increment_pc();
                }
                return 73;
            }
            0xA => {
                // Annn - Load (LD) I, addr
                // Set I = nnn.
                let val_to_load = opcode & 0x0FFF;
                self.i_register = val_to_load;
                self.increment_pc();
                return 55;
            }
            // 0xB - Timing of 105 microseconds
            0xC => {
                // Cxkk - RND Vx, byte
                // Set Vx = random byte AND kk.
                let val_to_and = last_byte(opcode);
                let rand_val: u8 = rand::random();
                self.data_registers[x_register as usize] = rand_val & val_to_and;
                self.increment_pc();
                return 164;
            }
            0xD => {
                // Draw (DRW) Vx, Vy, nibble
                // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
                // Sprites are XOR'd onto the screen.
                // If ANY pixel set to 0 due to the XOR, then a collision has happened.
                let n_bytes = last_nibble(last_byte(opcode));

                // Read n bytes from memory at position I
                let memory_location = self.i_register as usize;
                let bytes_to_draw =
                    &self.memory[memory_location..(memory_location + n_bytes as usize)];

                // Display those bytes as sprites at Vx, Vy
                // Sprites should be XOR'd into the display buffer
                let mut was_collision = false;

                // TODO(reece) The collision logic with wrap
                for (i, byte) in bytes_to_draw.iter().enumerate() {
                    let byte = *byte;
                    for bit_position in 0..8 {
                        let bit_is_set = ((byte >> 7 - bit_position) & 0x1) > 0;

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
                } else {
                    self.data_registers[0xF] = 1;
                }

                self.increment_pc();
                return 22734;
            }
            0xE => match last_byte(opcode) {
                0x9E => {
                    // Ex9E - SKP Vx
                    // Skip next instruction if key with the value of Vx is pressed.
                    let key_value = self.data_registers[x_register as usize];
                    if self.is_key_pressed(key_value) {
                        self.increment_pc();
                    }
                    self.increment_pc();
                    return 73;
                }
                0xA1 => {
                    // ExA1 - SKNP Vx
                    // Skip next instruction if key with the value of Vx is not pressed.
                    let key_value = self.data_registers[x_register as usize];
                    if !self.is_key_pressed(key_value) {
                        self.increment_pc();
                    }
                    self.increment_pc();
                    return 73;
                }
                _ => unimplemented_opcode(
                    opcode,
                    first_nibble_first_byte,
                    second_nibble_first_byte,
                    self.program_counter,
                ),
            },
            0xF => match last_byte(opcode) {
                0x07 => {
                    // Fx07 - LD Vx, DT
                    // Set Vx = delay timer value.
                    self.data_registers[x_register as usize] = self.delay_timer;
                    self.increment_pc();
                    return 45;
                }
                // Fx0A - 0 microseconds to execute (since it's a wait I guess?)
                0x15 => {
                    // Fx15 - LD DT, Vx
                    // Set delay timer = Vx.
                    self.delay_timer = x;
                    self.increment_pc();
                    return 45;
                }
                0x18 => {
                    // Fx18 - LD ST, Vx
                    // Set sound timer = Vx.
                    self.sound_timer = x;
                    self.increment_pc();
                    return 45;
                }
                0x1E => {
                    // Fx1E - ADD I, Vx
                    // Set I = I + Vx.
                    self.i_register = self.i_register.wrapping_add(x as u16);
                    self.increment_pc();
                    return 86;
                }
                0x29 => {
                    // Fx29 - LD F, Vx
                    // Set I = location of sprite for digit Vx.
                    // The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx

                    let sprite_location =
                        (FONT_SPRITE_LENGTH_IN_BYTES * FONT_START_LOCATION) as u16;
                    self.i_register = sprite_location;
                    self.increment_pc();
                    return 91;
                }
                0x33 => {
                    // Fx33 - LD B, Vx
                    // Store BCD (Binary Coded Decimal) representation of Vx in memory locations I, I+1, and I+2.
                    // Store hundreds at position I
                    // Store tens at position I + 1
                    // Store ones at position I + 2

                    let mut x_val = x;
                    let ones = x_val % 10;
                    x_val /= 10;
                    let tens = x_val % 10;
                    x_val /= 10;
                    let hundreds = x_val;

                    self.memory[self.i_register as usize] = hundreds;
                    self.memory[self.i_register as usize + 1] = tens;
                    self.memory[self.i_register as usize + 2] = ones;

                    self.increment_pc();

                    return 927;
                }
                0x55 => {
                    // Fx55 - LD [I], Vx
                    // Store registers V0 through Vx in memory starting at location I.
                    let start_address = self.i_register as usize;
                    let x = x_register;
                    for reg in 0..=x {
                        if reg >= 0xF {
                            break;
                        }
                        self.memory[start_address + reg as usize] =
                            self.data_registers[reg as usize];
                    }
                    self.increment_pc();
                    return 605;
                }
                0x65 => {
                    // Fx65 - LD Vx, [I]
                    // Read registers V0 through Vx from memory starting at location I.
                    let start_address = self.i_register as usize;
                    let x = x_register;

                    for reg in 0..=x {
                        self.data_registers[reg as usize] =
                            self.memory[start_address + reg as usize];
                    }
                    self.increment_pc();
                    return 605;
                }
                _ => unimplemented_opcode(
                    opcode,
                    first_nibble_first_byte,
                    second_nibble_first_byte,
                    self.program_counter,
                ),
            },

            _ => unimplemented_opcode(
                opcode,
                first_nibble_first_byte,
                second_nibble_first_byte,
                self.program_counter,
            ),
        }
        return 0;
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

    fn is_key_pressed(&self, key_value: u8) -> bool {
        return self.keys[key_value as usize];
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
        "roms/test_opcode.ch8"
    };

    // Test ROM from https://github.com/corax89/chip8-test-rom
    // More test ROMS from https://github.com/Timendus/chip8-test-suite#chip-8-splash-screen
    let rom_bytes = std::fs::read(file_path).unwrap();

    let mut chip = Chip8 {
        memory: [0; 4096],
        data_registers: [0; 16],
        program_counter: PROGRAM_OFFSET,
        i_register: 0,
        display_buffer: [false; CHIP_DISPLAY_WIDTH_IN_PIXELS * CHIP_DISPLAY_HEIGHT_IN_PIXELS],
        stack_pointer: 0,
        stack: [0; 16],
        delay_timer: 0,
        sound_timer: 0,
        keys: [false; 16],
    };

    // Initialize Chip8

    // Fonts sit at the start of memory
    for (i, byte) in FONT_SPRITES.iter().enumerate() {
        chip.memory[i + FONT_START_LOCATION] = *byte;
    }

    for (i, byte) in rom_bytes.iter().enumerate() {
        chip.memory[PROGRAM_OFFSET + i] = *byte;
    }

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
            draw_display(&mut canvas, &chip.display_buffer, scale);
        }
        draw_display(&mut canvas, &chip.display_buffer, scale);

        let current_frame_time = std::time::Instant::now();

        let latest_frame_time = current_frame_time - last_frame_time;
        last_frame_time = current_frame_time;

        let time_to_sleep = target_frame_time.saturating_sub(latest_frame_time);

        if !time_to_sleep.is_zero() {
            std::thread::sleep(time_to_sleep);
        }
    }
}

fn unimplemented_opcode(opcode: u16, first_nibble: u8, second_nibble: u8, program_counter: usize) {
    panic!(
        "Unimplemented opcode {:X}, first nibble: {:X}, second nibble: {:X}, PC: {:X}",
        opcode, first_nibble, second_nibble, program_counter
    );
}

const FONT_SPRITE_LENGTH_IN_BYTES: usize = 5;
const FONT_START_LOCATION: usize = 0;
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
    (x as usize % CHIP_DISPLAY_WIDTH_IN_PIXELS)
        + ((y as usize % CHIP_DISPLAY_HEIGHT_IN_PIXELS) * CHIP_DISPLAY_WIDTH_IN_PIXELS)
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
                    1 * scale as u32,
                    1 * scale as u32,
                ))
                .unwrap();
        }
    }
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.present();
}

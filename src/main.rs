use std::fs::File;

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

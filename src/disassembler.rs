use std::collections::HashMap;

fn make_instruction_to_opcode_mapping() -> HashMap<&'static str, u8> {
    HashMap::from([("JP", 0x1)])
}

pub fn disassemble(lines: Vec<String>) -> Vec<u8> {
    // Assume no labels for now
    let instruction_to_opcode_map = make_instruction_to_opcode_mapping();

    // Could do this as enums of what the instructions are, then output something at the end?
    // Is that just a bunch of typing for nothing?
    let mut machine_code: Vec<u8> = Vec::new();
    for line in lines {
        let splits: Vec<&str> = line.split_whitespace().collect();
        match splits.len() {
            3 => {} // Do that 3 split parsing stuff
            2 => {
                let first_token = splits[0];
                let second_token = splits[1];
                match first_token {
                    "JP" => {
                        let opcode = *instruction_to_opcode_map.get(first_token).unwrap();
                        let without_prefix = second_token.trim_start_matches("0x");
                        // TODO(reece): Validate as hexadecimal (Ignore size of number for now)
                        let addr = u16::from_str_radix(without_prefix, 16).unwrap();
                        let last_byte = (addr & 0xFF) as u8;
                        let addr_nibble = (addr >> 8 & 0xF) as u8;
                        let first_byte = (opcode << 4) | addr_nibble;

                        machine_code.push(first_byte);
                        machine_code.push(last_byte);
                    }
                    _ => {
                        todo!()
                    }
                }
            }
            _ => {
                todo!()
            } // TODO(reece): Error when not 2 or 3
        }
    }

    return machine_code;
}

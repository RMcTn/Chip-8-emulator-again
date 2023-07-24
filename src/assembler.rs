use std::{collections::HashMap, panic, todo};

use crate::scanner::{self, tokenize, Token, TokenType};

pub fn assemble(source: String) -> Vec<u8> {
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);

    let machine_code = parser.generate_machine_code();
    return machine_code;
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        return Parser { tokens, current: 0 };
    }

    fn label_pre_pass(&mut self) -> Vec<Token> {
        let mut processed_tokens = Vec::with_capacity(self.tokens.len());
        let mut labels: HashMap<Vec<char>, u16> = HashMap::new();

        while self.current < self.tokens.len() {
            // for i in 0..self.tokens.len() {
            let token = self.tokens[self.current].clone();
            self.advance();
            let i = self.current;
            // SPEEDUP: Don't clone
            match token.token_type {
                TokenType::LabelIdentifier => {
                    // TODO(Reece): Range check
                    if self.match_tokens_consume_if_true(&[
                        TokenType::NumericalValue(scanner::NumericalValue::Number),
                        TokenType::Newline,
                    ]) {
                        let number = self.tokens[i].literal.unwrap();

                        // HACK: Just want to keep moving, Stop the cloning when I can be bothered
                        // to worry about lifetimes
                        let mut token_name = token.word.clone();
                        token_name.remove(0);
                        let token_name_again = token_name.clone();

                        match labels.insert(token_name_again, number) {
                            None => {}
                            Some(x) => {
                                panic!("{:?} was already defined with value {}", token_name, x)
                            }
                        }
                    } else {
                        panic!("Was expecting a number and a newline after LabelIdentifier, found {:?}, {:?}", self.tokens[i], self.tokens[i + 1]);
                    };
                }
                TokenType::Label => {
                    let value = match labels.get(&token.word) {
                        None => panic!("Could not find value for {:?}", &token.word),
                        Some(x) => x,
                    };
                    let mut value_token = token;
                    value_token.token_type =
                        TokenType::NumericalValue(scanner::NumericalValue::Label);
                    value_token.literal = Some(*value);
                    processed_tokens.push(value_token);
                }
                TokenType::Number => {
                    let mut value_token = token;
                    value_token.token_type =
                        TokenType::NumericalValue(scanner::NumericalValue::Number);
                    processed_tokens.push(value_token);
                }
                _ => processed_tokens.push(token),
            }
        }
        self.current = 0;
        return processed_tokens;
    }

    /// Any instruction that uses a register specified by hexadecimal will be assumed to be valid
    /// for now
    fn generate_machine_code(&mut self) -> Vec<u8> {
        // Didn't feel necessary to generate "statements" from the tokens just to generate the
        // machine code at the time
        // HACK: FIXME: Starting to feel more necessary to have an intermediate step between tokenizing
        // and code generation with the addition of labels. But we're just gonna substitute as a
        // pre pass
        //
        let tokens = self.label_pre_pass();
        self.tokens = tokens;
        let mut machine_code = Vec::with_capacity(100);
        while self.current < self.tokens.len() {
            let current_token = self.tokens[self.current].clone();
            self.advance();
            match current_token.token_type {
                TokenType::CLS | TokenType::RET => {
                    let prev = self.current;
                    let following_tokens = self.tokens[prev..=prev].to_owned();
                    if !self.next_token_is_newline() {
                        // TODO(reece): Some way to do line counts for better error messages
                        // TODO(reece): Better error handling for parser errors
                        panic!(
                            "{:?} was expecting a newline. Instead found {:?}",
                            current_token.token_type,
                            self.next_token().token_type
                        );
                    }
                    machine_code.append(&mut Parser::machine_code_for_instruction(
                        &current_token,
                        &following_tokens,
                    ));
                }
                TokenType::JP | TokenType::Call | TokenType::SKP | TokenType::SKNP => {
                    let prev = self.current;
                    let following_tokens = self.tokens[prev..=prev + 1].to_owned();
                    if !self.match_tokens_consume_if_true(&[
                        TokenType::NumericalValue(scanner::NumericalValue::Number),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        // TODO(reece): This matching on the label + number is getting very annoying
                        TokenType::NumericalValue(scanner::NumericalValue::Label),
                        TokenType::Newline,
                    ]) {
                        panic!(
                            "{:?} was expecting a number/label and a new line. Instead found {:?} and {:?}",
                            current_token.token_type,
                            following_tokens[0].token_type,
                            following_tokens[1].token_type,
                        );
                    } else {
                        machine_code.append(&mut Parser::machine_code_for_instruction(
                            &current_token,
                            &following_tokens,
                        ));
                    }
                }
                TokenType::LD => {
                    let prev = self.current;
                    let following_tokens = self.tokens[prev..=prev + 2].to_owned();
                    if !self.match_tokens_consume_if_true(&[
                        TokenType::IRegister,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Number),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Number),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::IRegister,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Label),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Label),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::Register,
                        TokenType::Newline,
                    ]) {
                        // TODO(reece): Better way for parsing messages here. Could just have our
                        // slices of expected Tokens be the thing we create the message from
                        panic!(
                            "{:?} was expecting I, a comma, a number/label, and a new line, or register, comma, number/label, newline, or register, comma, number/label, register.. Instead found {:?} and {:?} and {:?}",
                            current_token.token_type,
                            following_tokens[0].token_type,
                            following_tokens[1].token_type,
                            following_tokens[2].token_type,
                        );
                    } else {
                        machine_code.append(&mut Parser::machine_code_for_instruction(
                            &current_token,
                            following_tokens.as_slice(),
                        ));
                    }
                }
                TokenType::SE | TokenType::SNE => {
                    let prev = self.current;
                    let following_tokens = self.tokens[prev..=prev + 2].to_owned();
                    if !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::Register,
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Number),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Label),
                        TokenType::Newline,
                    ]) {
                        panic!(
                            "{:?} was expecting a register, a comma, a register, and a new line, or a register, a comma, a number/label, and a new line. Instead found {:?} and {:?} and {:?} and {:?}",
                            current_token.token_type,
                            self.tokens[prev].token_type,
                            self.tokens[prev + 1].token_type,
                            self.tokens[prev + 2].token_type,
                            self.tokens[prev + 3].token_type,
                        );
                    } else {
                        machine_code.append(&mut Parser::machine_code_for_instruction(
                            &current_token,
                            &following_tokens,
                        ));
                    }
                }
                TokenType::RND => {
                    let prev = self.current;
                    let following_tokens = self.tokens[prev..=prev + 2].to_owned();
                    if !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Number),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Label),
                        TokenType::Newline,
                    ]) {
                        panic!(
                            "{:?} was expecting a register, a comma, a number/label, and a new line. Instead found {:?} and {:?} and {:?}",
                            current_token.token_type,
                            self.tokens[prev].token_type,
                            self.tokens[prev + 1].token_type,
                            self.tokens[prev + 2].token_type,
                        );
                    } else {
                        machine_code.append(&mut Parser::machine_code_for_instruction(
                            &current_token,
                            &following_tokens,
                        ));
                    }
                }
                TokenType::ADD => {
                    let prev = self.current;
                    let following_tokens = self.tokens[prev..=prev + 3].to_owned();
                    if !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::Register,
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Number),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Label),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::IRegister,
                        TokenType::Comma,
                        TokenType::Register,
                        TokenType::Newline,
                    ]) {
                        panic!(
                            "{:?} was expecting a register, a comma, a register, and a new line, or a register, a comma, a number/label, and a new line, or an iregister, a comma, a register, and a new line. Instead found {:?} and {:?} and {:?} and {:?}",
                            current_token.token_type,
                            self.tokens[prev].token_type,
                            self.tokens[prev + 1].token_type,
                            self.tokens[prev + 2].token_type,
                            self.tokens[prev + 3].token_type,
                        );
                    } else {
                        machine_code.append(&mut Parser::machine_code_for_instruction(
                            &current_token,
                            &following_tokens,
                        ));
                    }
                }
                TokenType::OR
                | TokenType::XOR
                | TokenType::SUB
                | TokenType::AND
                | TokenType::SUBN
                | TokenType::SHL
                | TokenType::SHR => {
                    let prev = self.current;
                    let following_tokens = self.tokens[prev..=prev + 3].to_owned();
                    if !self.match_tokens(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::Register,
                        TokenType::Newline,
                    ]) {
                        panic!(
                            "{:?} was expecting a register, a comma, a register, and a new line. Instead found {:?} and {:?} and {:?} and {:?}",
                            current_token.token_type,
                            self.tokens[prev].token_type,
                            self.tokens[prev + 1].token_type,
                            self.tokens[prev + 2].token_type,
                            self.tokens[prev + 3].token_type,
                        );
                    } else {
                        machine_code.append(&mut Parser::machine_code_for_instruction(
                            &current_token,
                            &following_tokens,
                        ));
                    }
                }
                TokenType::DRAW => {
                    let prev = self.current;
                    let following_tokens = self.tokens[prev..=prev + 5].to_owned();
                    if !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Number),
                        TokenType::Newline,
                    ]) && !self.match_tokens_consume_if_true(&[
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::Register,
                        TokenType::Comma,
                        TokenType::NumericalValue(scanner::NumericalValue::Label),
                        TokenType::Newline,
                    ]) {
                        panic!(
                            "{:?} was expecting a register, a comma, a register, a comma, a number/label and a new line. Instead found {:?} and {:?} and {:?} and {:?} and {:?} and {:?}",
                            current_token.token_type,
                            self.tokens[prev].token_type,
                            self.tokens[prev + 1].token_type,
                            self.tokens[prev + 2].token_type,
                            self.tokens[prev + 3].token_type,
                            self.tokens[prev + 4].token_type,
                            self.tokens[prev + 5].token_type,
                        );
                    } else {
                        machine_code.append(&mut Parser::machine_code_for_instruction(
                            &current_token,
                            &following_tokens,
                        ));
                    }
                }

                TokenType::Newline => {
                    // Do nothing
                }
                TokenType::Number
                | TokenType::Addr
                | TokenType::Comma
                | TokenType::Colon
                | TokenType::IRegister
                | TokenType::Label
                | TokenType::LabelIdentifier
                | TokenType::NumericalValue(_)
                | TokenType::Register => {
                    panic!(
                        "Was not expecting a {:?} ({:?})",
                        current_token.token_type, current_token.word
                    );
                }
            }
        }

        return machine_code;
    }

    /// Assumes there are enough Tokens in following_tokens to generate machine code
    /// i.e for LD following_tokens would contain at least [Register, Comma, Number, Newline]
    fn machine_code_for_instruction(
        instruction_token: &Token,
        following_tokens: &[Token],
    ) -> Vec<u8> {
        // SPEEDUP(reece): Don't just clone the tokens for the following_tokens
        // Might be worth having an intermediate state between Tokens and machine code to make
        // codegen easier. We're going to need to pass a slice of tokens otherwise
        // dbg!(token.token_type);
        let mut machine_code = Vec::new();

        match instruction_token.token_type {
            TokenType::Comma | TokenType::Newline => {
                // Do nothing. Should we error though?
            }
            // TODO(reece): Fair bit of repitition here, any chance of minimizing?
            TokenType::RET => {
                let first_byte = 0x00;
                let second_byte = 0xEE;
                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::CLS => {
                let first_byte = 0x00;
                let second_byte = 0xE0;
                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::JP => {
                // 1nnn
                let opcode = 0x1;
                let addr = following_tokens[0].literal.unwrap();
                let first_byte = opcode << 4 | (addr >> 8) as u8;
                let second_byte = (addr & 0x00FF) as u8;
                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::Call => {
                // 2nnn
                let opcode = 0x2;
                let addr = following_tokens[0].literal.unwrap();
                let first_byte = opcode << 4 | (addr >> 8) as u8;
                let second_byte = (addr & 0x00FF) as u8;
                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::SKP => {
                // Ex9E
                let opcode = 0xE;
                let first_byte = opcode << 4 | (following_tokens[0].literal.unwrap() & 0xF) as u8;
                let second_byte = 0x9E;
                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::SKNP => {
                // ExA1
                let opcode = 0xE;
                let first_byte = opcode << 4 | (following_tokens[0].literal.unwrap() & 0xF) as u8;
                let second_byte = 0xA1;
                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::LD => {
                let token_types_to_consider = [
                    following_tokens[0].token_type,
                    following_tokens[2].token_type,
                ];

                match token_types_to_consider {
                    [TokenType::Register, TokenType::NumericalValue(_)] => {
                        // 6xkk
                        let mut first_byte = 6;
                        first_byte = first_byte << 4;
                        first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                        let second_byte = following_tokens[2].literal.unwrap() as u8;

                        machine_code.push(first_byte);
                        machine_code.push(second_byte);
                    }
                    [TokenType::Register, TokenType::Register] => {
                        // 8xy0
                        let mut first_byte = 8;
                        first_byte = first_byte << 4;
                        first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                        let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                        second_byte = second_byte << 4;

                        machine_code.push(first_byte);
                        machine_code.push(second_byte);
                    }
                    [TokenType::IRegister, TokenType::NumericalValue(_)] => {
                        // Annn
                        let mut first_byte = 0xA;
                        let addr = following_tokens[2].literal.unwrap();
                        first_byte = first_byte << 4;
                        first_byte = first_byte | ((addr >> 8) & 0xF) as u8;
                        let second_byte = (addr & 0xFF) as u8;

                        machine_code.push(first_byte);
                        machine_code.push(second_byte);
                    }
                    x => todo!("Unimplemented or invalid machine code for {:?}.", x),
                }
            }

            TokenType::SE => {
                let token_types_to_consider = [
                    following_tokens[0].token_type,
                    following_tokens[2].token_type,
                ];

                match token_types_to_consider {
                    [TokenType::Register, TokenType::NumericalValue(_)] => {
                        // 3xkk
                        let mut first_byte = 3;
                        first_byte = first_byte << 4;
                        first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                        let second_byte = following_tokens[2].literal.unwrap() as u8;

                        machine_code.push(first_byte);
                        machine_code.push(second_byte);
                    }
                    [TokenType::Register, TokenType::Register] => {
                        // 5xy0
                        let mut first_byte = 5;
                        first_byte = first_byte << 4;
                        first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                        let second_byte = following_tokens[2].literal.unwrap() as u8;

                        machine_code.push(first_byte);
                        machine_code.push(second_byte);
                    }

                    x => todo!("Unimplemented or invalid machine code for {:?}.", x),
                }
            }
            TokenType::SNE => {
                // 9xy0
                let mut first_byte = 9;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let second_byte = following_tokens[2].literal.unwrap() as u8;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::OR => {
                // 8xy1
                let mut first_byte = 8;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                second_byte = second_byte << 4;
                second_byte = second_byte | 1;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::AND => {
                // 8xy2
                let mut first_byte = 8;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                second_byte = second_byte << 4;
                second_byte = second_byte | 2;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::XOR => {
                // 8xy3
                let mut first_byte = 8;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                second_byte = second_byte << 4;
                second_byte = second_byte | 3;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::ADD => {
                let token_types_to_consider = [
                    following_tokens[0].token_type,
                    following_tokens[2].token_type,
                ];

                match token_types_to_consider {
                    [TokenType::Register, TokenType::NumericalValue(_)] => {
                        // 7xkk
                        let mut first_byte = 7;
                        first_byte = first_byte << 4;
                        first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                        let second_byte = following_tokens[2].literal.unwrap() as u8;

                        machine_code.push(first_byte);
                        machine_code.push(second_byte);
                    }
                    [TokenType::Register, TokenType::Register] => {
                        // 8xy4
                        let mut first_byte = 8;
                        first_byte = first_byte << 4;
                        first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                        let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                        second_byte = second_byte << 4;
                        second_byte = second_byte | 4;

                        machine_code.push(first_byte);
                        machine_code.push(second_byte);
                    }
                    [TokenType::IRegister, TokenType::Register] => {
                        // Fx1E
                        let mut first_byte = 0xF;
                        first_byte = first_byte << 4;
                        first_byte =
                            first_byte | (following_tokens[2].literal.unwrap() & 0xF) as u8;
                        let second_byte = 0x1E;

                        machine_code.push(first_byte);
                        machine_code.push(second_byte);
                    }

                    x => todo!("Unimplemented or invalid machine code for {:?}.", x),
                }
            }
            TokenType::SUB => {
                // 8xy5
                let mut first_byte = 8;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                second_byte = second_byte << 4;
                second_byte = second_byte | 5;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::SUBN => {
                // 8xy7
                let mut first_byte = 8;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                second_byte = second_byte << 4;
                second_byte = second_byte | 7;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::RND => {
                // Cxkk
                let mut first_byte = 0xC;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let second_byte = following_tokens[2].literal.unwrap() as u8;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::DRAW => {
                let mut first_byte = 0xD;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;

                let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                second_byte = second_byte << 4;
                second_byte = second_byte | (following_tokens[4].literal.unwrap() as u8 & 0xF);
                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::SHL => {
                // 8xyE
                let mut first_byte = 0x8;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                second_byte = second_byte << 4;
                second_byte = second_byte | 0xE;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }
            TokenType::SHR => {
                // 8xy6
                let mut first_byte = 0x8;
                first_byte = first_byte << 4;
                first_byte = first_byte | following_tokens[0].literal.unwrap() as u8;
                let mut second_byte = following_tokens[2].literal.unwrap() as u8;
                second_byte = second_byte << 4;
                second_byte = second_byte | 0x6;

                machine_code.push(first_byte);
                machine_code.push(second_byte);
            }

            unimplemented_token => todo!("{:?}", unimplemented_token),
        }
        return machine_code;
    }

    fn next_token(&self) -> &Token {
        return &self.tokens[self.current];
    }

    fn next_token_is_newline(&self) -> bool {
        return self.tokens[self.current].token_type == TokenType::Newline;
    }

    /// Does not consume the current token
    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        return self.next_token().token_type == token_type;
    }
    fn check_at(&self, token_type: TokenType, idx: usize) -> bool {
        if self.is_at_end() {
            return false;
        }
        // TODO(reece): Bounds check
        return self.tokens[idx].token_type == token_type;
    }

    fn check_all(&self, token_types: &[TokenType]) -> bool {
        if self.is_at_end() {
            return false;
        }
        for (i, token_type) in token_types.iter().enumerate() {
            if !self.check_at(*token_type, self.current + i) {
                return false;
            }
        }
        return true;
    }

    fn advance(&mut self) {
        self.current += 1;
    }

    fn advance_by(&mut self, amount: usize) {
        self.current += amount;
    }

    /// Consumes the current_token if it matches the given token type, advancing when matching
    fn match_tokens(&mut self, token_types: &[TokenType]) -> bool {
        for token_type in token_types {
            if self.check(*token_type) {
                self.advance();
            } else {
                return false;
            }
        }
        return true;
    }

    /// Consumes the given tokens if they match, advancing only if ALL tokens match
    /// Useful for instructions that have many different forms (Like SE accepting registers or
    /// number), so we don't skip over the same tokens we're trying to match against.
    fn match_tokens_consume_if_true(&mut self, token_types: &[TokenType]) -> bool {
        if self.check_all(token_types) {
            self.advance_by(token_types.len());
            return true;
        } else {
            return false;
        }
    }

    fn is_at_end(&self) -> bool {
        // TODO(reece): DRAW is just a stand in until we decide what end means
        if self.next_token().token_type == TokenType::DRAW {
            panic!("Time to decide an actual end token intead of DRAW");
        } else {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_assembles_maze() {
        let maze_assembly = std::fs::read_to_string("./test_programs/maze.asm").unwrap();
        let maze_machine_code = std::fs::read("./test_programs/maze.ch8").unwrap();
        assert_eq!(assemble(maze_assembly), maze_machine_code);
    }

    #[test]
    fn it_assembles_with_labels() {
        let label_assembly = std::fs::read_to_string("./test_programs/labels.asm").unwrap();
        let label_machine_code = std::fs::read("./test_programs/labels.ch8").unwrap();
        assert_eq!(assemble(label_assembly), label_machine_code);
    }
}

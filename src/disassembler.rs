use std::{collections::HashMap, panic, todo};

fn make_instruction_to_opcode_mapping() -> HashMap<&'static str, u8> {
    HashMap::from([("JP", 0x1), ("LD I", 0xA)])
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
    // TODO(reece): Going to keep the 0x1 syntax for chosing register 1 for now, maybe move to
    // Vx later
    LD,
    JP,
    Call,
    SE,
    SNE,
    // TODO(reece): ADD Vx, Vy and ADD Vx, byte are going to be indistinguishable if we just
    // treat Vx like 0x3 for example, since 0x3 could mean Vy, or just byte
    ADD,
    SUB,
    AND,
    XOR,
    OR,
    RND,
    DRAW,
    SKP,
    SKNP,
    RET,
    CLS,
    Number,
    // Not sure if we want this yet!
    Addr,
    Comma,
    IRegister,
    Newline, // Newline used at the terminator for most statements
}

enum OpcodeType {
    // The plan here was write something that could hold the different types of arguments we get.
    // For example:
    // LD I, 0x200
    // LD 0x1, 0x3
    // JP 0x200
    // Then our TokenTypes could be
    // LD(OpcodeType) -> match on this when doing code generation
    // match opcode_type {
    //  I register -> that stuff
    //  2 arguments -> 2 arg stuff
    // }
    // But this might be confusing what a "token" really means. These almost feel more like
    // statements.
    // Is it necessary to have a tokenizing pass over the assembly code, and then a pass over the
    // tokens to generate "statements", then another pass of statements just to generate machine
    // code for such a simple instruction set?
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    // Just cloning the str's right now so we can move along
    pub word: Vec<char>,
    pub literal: Option<u16>,
}

struct Scanner {
    start_char_idx: usize,
    current_char_idx: usize,
    source_as_chars: Vec<char>,
    keywords: HashMap<String, TokenType>,
}

impl Scanner {
    fn new(source: String) -> Self {
        let keywords: HashMap<String, TokenType> = HashMap::from([
            ("JP".to_string(), TokenType::JP),
            ("LD".to_string(), TokenType::LD),
            ("I".to_string(), TokenType::IRegister),
            ("CALL".to_string(), TokenType::Call),
            ("SE".to_string(), TokenType::SE),
            ("SNE".to_string(), TokenType::SNE),
            ("ADD".to_string(), TokenType::ADD),
            ("SUB".to_string(), TokenType::SUB),
            ("AND".to_string(), TokenType::AND),
            ("XOR".to_string(), TokenType::XOR),
            ("OR".to_string(), TokenType::OR),
            ("RND".to_string(), TokenType::RND),
            ("DRW".to_string(), TokenType::DRAW),
            ("SKP".to_string(), TokenType::SKP),
            ("SKNP".to_string(), TokenType::SKNP),
            ("RET".to_string(), TokenType::RET),
            ("CLS".to_string(), TokenType::CLS),
        ]);

        let scanner = Scanner {
            start_char_idx: 0,
            current_char_idx: 0,
            source_as_chars: source.chars().collect(),
            keywords,
        };
        return scanner;
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = vec![];

        while self.current_char_idx < self.source_as_chars.len() {
            self.start_char_idx = self.current_char_idx;
            if let Some(token) = self.scan_token() {
                tokens.push(token);
            }
        }
        dbg!(&tokens);

        return tokens;
    }

    fn next_char_is(&self, ch: char) -> bool {
        if self.is_at_end() && ch != '\0' {
            return false;
        }
        return self.peek() == ch;
    }

    fn is_at_end(&self) -> bool {
        return self.current_char_idx >= self.source_as_chars.len();
    }

    fn advance(&mut self) {
        self.current_char_idx += 1;
    }

    fn scan_token(&mut self) -> Option<Token> {
        let ch = self.source_as_chars[self.current_char_idx];
        self.advance();
        match ch {
            '0' => {
                if self.next_char_is('x') {
                    self.advance();
                    let val = self.parse_hex_number();
                    Some(Token {
                        token_type: TokenType::Number,
                        word: self.source_as_chars[self.start_char_idx..self.current_char_idx]
                            .to_owned(),
                        literal: Some(val),
                    })
                } else {
                    panic!("Was expecting hex number after 0 character");
                }
            }
            ',' => Some(Token {
                token_type: TokenType::Comma,
                word: self.source_as_chars[self.start_char_idx..self.current_char_idx].to_owned(),
                literal: None,
            }),
            _ => {
                if ch.is_alphabetic() {
                    // TODO(reece): Handle this unwrap
                    let token_type = self.parse_instruction().unwrap();
                    Some(Token {
                        token_type,
                        word: self.source_as_chars[self.start_char_idx..self.current_char_idx]
                            .to_owned(),
                        literal: None,
                    })
                } else if ch == '\n' {
                    Some(Token {
                        token_type: TokenType::Newline,
                        word: self.source_as_chars[self.start_char_idx..self.current_char_idx]
                            .to_owned(),
                        literal: None,
                    })
                } else if ch.is_whitespace() {
                    None
                } else {
                    todo!()
                }
            }
        }
    }

    fn parse_instruction(&mut self) -> Option<TokenType> {
        // TODO(reece): Rename this from parse instruction because it does more than that
        while self.peek().is_alphabetic() && !self.next_char_is(',') {
            self.advance();
        }

        // SPEEDUP(reece): Don't clone the string
        let text: String = self.source_as_chars[self.start_char_idx..self.current_char_idx]
            .iter()
            .collect();
        dbg!(&text);
        if let Some(keyword_type) = self.keywords.get(&text) {
            return Some(*keyword_type);
        }
        dbg!("No keyword type found for {}", text);
        return None;
    }

    fn parse_hex_number(&mut self) -> u16 {
        while self.peek().is_ascii_hexdigit() {
            self.advance();
        }

        let num_as_string: String = self.source_as_chars
            [self.start_char_idx + 2..self.current_char_idx]
            .iter()
            .collect();

        // Seems like a safe unwrap (ignoring numbers too big!)
        let num = u16::from_str_radix(&num_as_string, 16).unwrap();
        return num;
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            // TODO(reece): Is returning a null character something we really want to do at the
            // end?
            return '\0';
        }
        return self.source_as_chars[self.current_char_idx];
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            // TODO(reece): Is returning a null character something we really want to do at the
            // end?
            return '\0';
        }
        return self.source_as_chars[self.current_char_idx + 1];
    }
}

pub fn parse(source: String) -> Vec<Token> {
    let mut scanner = Scanner::new(source);
    let tokens = scanner.tokenize();
    let mut parser = Parser::new(tokens);

    parser.parse();
    return parser.tokens;
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        return Parser { tokens, current: 0 };
    }

    fn parse(&mut self) -> Vec<u8> {
        let mut machine_code = Vec::with_capacity(100);
        while self.current < self.tokens.len() {
            let current_token = self.tokens[self.current].clone();
            self.advance();
            match current_token.token_type {
                TokenType::CLS | TokenType::RET => {
                    if !self.next_token_is_newline() {
                        // TODO(reece): Some way to do line counts for better error messages
                        // TODO(reece): Better error handling for parser errors
                        panic!(
                            "{:?} was expecting a newline. Instead found {:?}",
                            current_token.token_type,
                            self.next_token().token_type
                        );
                    }
                    machine_code
                        .extend_from_slice(Parser::machine_code_for_instruction(&current_token));
                }
                TokenType::JP => {
                    if !self.match_tokens(&[TokenType::Number, TokenType::Newline]) {
                        panic!(
                            "{:?} was expecting a number and a new line. Instead found {:?} and {:?}",
                            current_token.token_type,
                            self.next_token().token_type,
                            // TODO(reece): Bounds check here
                            self.tokens[self.current + 1].token_type
                        );
                    } else {
                        Parser::machine_code_for_instruction(&current_token);
                    }
                }
                TokenType::Newline => {
                    // Do nothing
                }
                _ => todo!(),
            }
        }

        return machine_code;
    }

    fn machine_code_for_instruction(token: &Token) -> &[u8] {
        // Might be worth having an intermediate state between Tokens and machine code to make
        // codegen easier. We're going to need to pass a slice of tokens otherwise
        return &[0];
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

    fn advance(&mut self) {
        self.current += 1;
    }

    /// Consumes the current_token
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

    fn is_at_end(&self) -> bool {
        // TODO(reece): DRAW is just a stand in until we decide what end means
        return self.next_token().token_type == TokenType::DRAW;
    }
}

pub fn disassemble(lines: Vec<String>) -> Vec<u8> {
    // Assume no labels for now
    let instruction_to_opcode_map = make_instruction_to_opcode_mapping();

    // Could do this as enums of what the instructions are, then output something at the end?
    // Is that just a bunch of typing for nothing?
    let mut machine_code: Vec<u8> = Vec::new();
    for line in lines {
        dbg!(&line);
        let splits: Vec<&str> = line.split_whitespace().collect();
        match splits.len() {
            3 => {
                let first_token = splits[0];
                let second_token = splits[1];
                if second_token.chars().last().unwrap() != ',' {
                    // Any opcode with 3 parts always has a comma at the end of the 2nd token.
                    // Could tokenize this better if we really wanted, but this gets us moving
                    // quicker
                    // TODO(reece): Error here
                    panic!("Expected ',' after first argument for opcode");
                }
                let second_token = &second_token[0..second_token.len() - 1];
                let third_token = splits[2];
                match first_token {
                    "LD" => {
                        match second_token.chars().next().unwrap() {
                            'I' => {
                                let opcode = *instruction_to_opcode_map.get("LD I").unwrap();
                                let without_prefix = third_token.trim_start_matches("0x");
                                let addr = u16::from_str_radix(without_prefix, 16).unwrap();
                                let last_byte = (addr & 0xFF) as u8;
                                let addr_nibble = (addr >> 8 & 0xF) as u8;
                                let first_byte = (opcode << 4) | addr_nibble;

                                machine_code.push(first_byte);
                                machine_code.push(last_byte);
                            }
                            'V' => {
                                // This is starting to feel annoying having to validate constantly.
                                // Maybe should just parse as 1 pass, then convert those tokens
                                // into machine code?
                            }
                            _ => {
                                todo!();
                            }
                        }
                    }
                    _ => {
                        todo!();
                    }
                }
            }
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

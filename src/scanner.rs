use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
    LD,
    JP,
    Call,
    SE,
    SNE,
    ADD,
    SUB,
    SUBN,
    AND,
    XOR,
    OR,
    RND,
    DRAW,
    SKP,
    SKNP,
    RET,
    CLS,
    SHL,
    SHR,
    Number,
    // Not sure if we want this yet!
    Addr,
    Comma,
    IRegister,
    Newline,
    // Newline used at the terminator for most statements
    Register,
    Label,
    LabelIdentifier,
    Colon,
    NumericalValue(NumericalValue),
}

/// This is used as a way to represent the valid values for numerical operands of instructions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumericalValue {
    Number,
    Label,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    // Just cloning the str's right now so we can move along
    pub word: Vec<char>,
    pub literal: Option<u16>,
}

pub struct Scanner {
    start_char_idx: usize,
    current_char_idx: usize,
    source_as_chars: Vec<char>,
    keywords: HashMap<String, TokenType>,
}

impl Scanner {
    pub fn new(source: String) -> Self {
        let keywords: HashMap<String, TokenType> = HashMap::from([
            ("JP".to_string(), TokenType::JP),
            ("LD".to_string(), TokenType::LD),
            ("I".to_string(), TokenType::IRegister),
            ("CALL".to_string(), TokenType::Call),
            ("SE".to_string(), TokenType::SE),
            ("SNE".to_string(), TokenType::SNE),
            ("ADD".to_string(), TokenType::ADD),
            ("SUB".to_string(), TokenType::SUB),
            ("SUBN".to_string(), TokenType::SUBN),
            ("AND".to_string(), TokenType::AND),
            ("XOR".to_string(), TokenType::XOR),
            ("OR".to_string(), TokenType::OR),
            ("RND".to_string(), TokenType::RND),
            ("DRW".to_string(), TokenType::DRAW),
            ("SKP".to_string(), TokenType::SKP),
            ("SKNP".to_string(), TokenType::SKNP),
            ("RET".to_string(), TokenType::RET),
            ("CLS".to_string(), TokenType::CLS),
            ("SHL".to_string(), TokenType::SHL),
            ("SHR".to_string(), TokenType::SHR),
        ]);

        let scanner = Scanner {
            start_char_idx: 0,
            current_char_idx: 0,
            source_as_chars: source.chars().collect(),
            keywords,
        };
        return scanner;
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
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
            ':' => {
                if !self.peek().is_alphabetic() {
                    panic!("Was expecting a label name after :");
                }
                // Try parse a label/identifier
                while self.peek().is_alphabetic() {
                    self.advance();
                }
                let label = &self.source_as_chars[self.start_char_idx..self.current_char_idx];
                dbg!(&label);
                // SPEEDUP(reece): Don't clone the string
                Some(Token {
                    token_type: TokenType::LabelIdentifier,
                    word: label.to_owned(),
                    literal: None,
                })
            }
            '0' => {
                if self.next_char_is('x') {
                    self.advance();
                    let val = self.parse_hex_number();
                    Some(Token {
                        token_type: TokenType::NumericalValue(NumericalValue::Number),
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
            'V' => {
                let register_val = self.parse_hex_number();
                return Some(Token {
                    token_type: TokenType::Register,
                    word: self.source_as_chars[self.start_char_idx..self.current_char_idx]
                        .to_owned(),
                    literal: Some(register_val),
                });
            }
            _ => {
                if ch.is_alphabetic() {
                    if let Some(instruction_token_type) = self.parse_instruction() {
                        return Some(Token {
                            token_type: instruction_token_type,
                            word: self.source_as_chars[self.start_char_idx..self.current_char_idx]
                                .to_owned(),
                            literal: None,
                        });
                    } else {
                        // Try parse a label/identifier
                        while self.peek().is_alphabetic() {
                            self.advance();
                        }
                        let label =
                            &self.source_as_chars[self.start_char_idx..self.current_char_idx];
                        dbg!(&label);
                        // SPEEDUP(reece): Don't clone the string
                        Some(Token {
                            token_type: TokenType::Label,
                            word: label.to_owned(),
                            literal: None,
                        })
                    }
                } else if ch == '\n' {
                    Some(Token {
                        token_type: TokenType::Newline,
                        word: self.source_as_chars[self.start_char_idx..self.current_char_idx]
                            .to_owned(),
                        literal: None,
                    })
                } else if ch.is_whitespace() {
                    None
                } else if ch.is_ascii_digit() {
                    let val = self.parse_decimal_number();
                    Some(Token {
                        token_type: TokenType::Number,
                        word: self.source_as_chars[self.start_char_idx..self.current_char_idx]
                            .to_owned(),
                        literal: Some(val),
                    })
                } else {
                    todo!()
                }
            }
        }
    }

    fn parse_instruction(&mut self) -> Option<TokenType> {
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

        // TODO(reece): ROBUSTNESS: Handle V without a register defined immediately afterwards
        let num_as_string: String;
        if self.source_as_chars[self.start_char_idx] == 'V' {
            num_as_string = self.source_as_chars[self.start_char_idx + 1..self.current_char_idx]
                .iter()
                .collect();
        } else {
            num_as_string = self.source_as_chars[self.start_char_idx + 2..self.current_char_idx]
                .iter()
                .collect();
        }

        // Seems like a safe unwrap (ignoring numbers too big!)
        let num = u16::from_str_radix(&num_as_string, 16).unwrap();
        return num;
    }

    fn parse_decimal_number(&mut self) -> u16 {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        let num_as_string: String;
        num_as_string = self.source_as_chars[self.start_char_idx..self.current_char_idx]
            .iter()
            .collect();

        // Seems like a safe unwrap (ignoring numbers too big!)
        let num = u16::from_str_radix(&num_as_string, 10).unwrap();
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

pub fn tokenize(source: String) -> Vec<Token> {
    let mut scanner = Scanner::new(source);
    let tokens = scanner.tokenize();
    return tokens;
}

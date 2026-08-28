use std::io::{self, Write};

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum TokenType {
    Eof = 0,
    Word,
    Number,
    Punctuation,
    Whitespace,
    Newline,
    Identifier,
    Keyword,
    Operator,
    String,
    Comment,
    Error,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub value: Vec<u8>,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

pub struct Tokenizer {
    input_buffer: Vec<u8>,
    buffer_length: usize,
    current_position: usize,
    current_line: i32,
    current_column: i32,
    total_tokens_processed: usize,
    total_lines_processed: usize,
    total_chars_processed: usize,
    lookahead_token: Option<Token>,
}

const KEYWORDS: [&[u8]; 32] = [
    b"if",
    b"else",
    b"while",
    b"for",
    b"return",
    b"int",
    b"char",
    b"float",
    b"double",
    b"void",
    b"struct",
    b"typedef",
    b"const",
    b"static",
    b"extern",
    b"auto",
    b"register",
    b"sizeof",
    b"break",
    b"continue",
    b"switch",
    b"case",
    b"default",
    b"do",
    b"goto",
    b"enum",
    b"union",
    b"signed",
    b"unsigned",
    b"long",
    b"short",
    b"",
];

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            input_buffer: Vec::new(),
            buffer_length: 0,
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens_processed: 0,
            total_lines_processed: 0,
            total_chars_processed: 0,
            lookahead_token: None,
        }
    }

    fn is_keyword(value: &[u8]) -> bool {
        KEYWORDS[..31].iter().any(|keyword| *keyword == value)
    }

    fn peek_char(&self) -> u8 {
        if self.current_position >= self.buffer_length {
            0
        } else {
            self.input_buffer[self.current_position]
        }
    }

    fn advance_char(&mut self) -> u8 {
        if self.current_position >= self.buffer_length {
            return 0;
        }

        let c = self.input_buffer[self.current_position];
        self.current_position += 1;
        self.total_chars_processed = self.total_chars_processed.wrapping_add(1);

        if c == b'\n' {
            self.current_line = self.current_line.wrapping_add(1);
            self.current_column = 1;
            self.total_lines_processed = self.total_lines_processed.wrapping_add(1);
        } else {
            self.current_column = self.current_column.wrapping_add(1);
        }

        c
    }

    fn is_space(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    fn is_alpha(c: u8) -> bool {
        c.is_ascii_alphabetic()
    }

    fn is_alnum(c: u8) -> bool {
        c.is_ascii_alphanumeric()
    }

    fn skip_whitespace(&mut self) {
        while self.peek_char() != 0
            && Self::is_space(self.peek_char())
            && self.peek_char() != b'\n'
        {
            self.advance_char();
        }
    }

    fn create_token(&mut self, token_type: TokenType, value: &[u8]) -> Token {
        let length = value.len().min(MAX_TOKEN_LENGTH - 1);
        let column = self.current_column.wrapping_sub(length as i32);
        self.total_tokens_processed = self.total_tokens_processed.wrapping_add(1);

        Token {
            token_type,
            value: value[..length].to_vec(),
            length,
            line: self.current_line,
            column,
        }
    }

    fn scan_word(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);

        while self.peek_char() != 0
            && (Self::is_alnum(self.peek_char()) || self.peek_char() == b'_')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            let c = self.advance_char();
            buffer.push(c);
        }

        let token_type = if Self::is_keyword(&buffer) {
            TokenType::Keyword
        } else {
            TokenType::Identifier
        };
        self.create_token(token_type, &buffer)
    }

    fn scan_number(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
        let mut has_decimal = false;

        while self.peek_char() != 0
            && (self.peek_char().is_ascii_digit() || self.peek_char() == b'.')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            if self.peek_char() == b'.' {
                if has_decimal {
                    break;
                }
                has_decimal = true;
            }

            let c = self.advance_char();
            buffer.push(c);
        }

        self.create_token(TokenType::Number, &buffer)
    }

    fn scan_string(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH + 1);
        let quote = self.advance_char();
        buffer.push(quote);

        while self.peek_char() != 0
            && self.peek_char() != quote
            && self.peek_char() != b'\n'
            && buffer.len() < MAX_TOKEN_LENGTH - 2
        {
            if self.peek_char() == b'\\' {
                let c = self.advance_char();
                buffer.push(c);
                if self.peek_char() != 0 {
                    let escaped = self.advance_char();
                    buffer.push(escaped);
                }
            } else {
                let c = self.advance_char();
                buffer.push(c);
            }
        }

        if self.peek_char() == quote {
            let c = self.advance_char();
            buffer.push(c);
        }

        self.create_token(TokenType::String, &buffer)
    }

    fn scan_comment(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
        let first = self.advance_char();
        buffer.push(first);

        if self.peek_char() == b'/' {
            let second = self.advance_char();
            buffer.push(second);

            while self.peek_char() != 0
                && self.peek_char() != b'\n'
                && buffer.len() < MAX_TOKEN_LENGTH - 1
            {
                let c = self.advance_char();
                buffer.push(c);
            }
        } else if self.peek_char() == b'*' {
            let star = self.advance_char();
            buffer.push(star);

            while self.peek_char() != 0 && buffer.len() < MAX_TOKEN_LENGTH - 2 {
                if self.peek_char() == b'*' {
                    let c = self.advance_char();
                    buffer.push(c);
                    if self.peek_char() == b'/' {
                        let slash = self.advance_char();
                        buffer.push(slash);
                        break;
                    }
                } else {
                    let c = self.advance_char();
                    buffer.push(c);
                }
            }
        }

        self.create_token(TokenType::Comment, &buffer)
    }

    fn scan_operator(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(2);
        let c = self.peek_char();
        let first = self.advance_char();
        buffer.push(first);

        let next = self.peek_char();
        if matches!(
            (c, next),
            (b'=', b'=')
                | (b'!', b'=')
                | (b'<', b'=')
                | (b'>', b'=')
                | (b'&', b'&')
                | (b'|', b'|')
                | (b'+', b'+')
                | (b'-', b'-')
                | (b'-', b'>')
                | (b'<', b'<')
                | (b'>', b'>')
        ) {
            let second = self.advance_char();
            buffer.push(second);
        }

        self.create_token(TokenType::Operator, &buffer)
    }

    pub fn next_token(&mut self) -> Token {
        if let Some(token) = self.lookahead_token.take() {
            return token;
        }

        self.skip_whitespace();

        if self.current_position >= self.buffer_length {
            return self.create_token(TokenType::Eof, b"");
        }

        let c = self.peek_char();

        if c == b'\n' {
            let newline = [self.advance_char()];
            return self.create_token(TokenType::Newline, &newline);
        }

        if Self::is_alpha(c) || c == b'_' {
            return self.scan_word();
        }

        if c.is_ascii_digit() {
            return self.scan_number();
        }

        if c == b'"' || c == b'\'' {
            return self.scan_string();
        }

        // The C condition peeks at the current slash twice, so every slash
        // enters scan_comment, including division operators.
        if c == b'/' && (self.peek_char() == b'/' || self.peek_char() == b'*') {
            return self.scan_comment();
        }

        if b"+-*/%=<>!&|^~?:".contains(&c) {
            return self.scan_operator();
        }

        if b"(){}[];,.".contains(&c) {
            let punctuation = [self.advance_char()];
            return self.create_token(TokenType::Punctuation, &punctuation);
        }

        let unknown = [self.advance_char()];
        self.create_token(TokenType::Error, &unknown)
    }

    #[allow(dead_code)]
    pub fn peek_token(&mut self) -> Token {
        if self.lookahead_token.is_none() {
            self.lookahead_token = Some(self.next_token());
        }
        self.lookahead_token.clone().unwrap()
    }

    pub fn reset(&mut self) {
        self.current_position = 0;
        self.current_line = 1;
        self.current_column = 1;
        self.lookahead_token = None;
    }

    pub fn load_text(&mut self, text: &[u8]) -> i32 {
        let c_length = text.iter().position(|byte| *byte == 0).unwrap_or(text.len());

        if c_length >= MAX_BUFFER_SIZE {
            let _ = writeln!(io::stderr().lock(), "Error: Input text too large");
            return -1;
        }

        self.input_buffer.clear();
        self.input_buffer.extend_from_slice(&text[..c_length]);
        self.buffer_length = c_length;
        self.reset();
        0
    }

    pub fn get_stats(&self) -> (usize, usize, usize) {
        (
            self.total_lines_processed,
            self.total_tokens_processed,
            self.total_chars_processed,
        )
    }
}

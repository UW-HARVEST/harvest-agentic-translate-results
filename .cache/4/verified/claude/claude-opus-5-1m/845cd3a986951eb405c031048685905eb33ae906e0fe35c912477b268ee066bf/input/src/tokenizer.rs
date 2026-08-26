//! Translation of c_src/src/tokenizer.c
//!
//! The C file keeps its state in file-scope `static` variables; here that state
//! lives in a single `Tokenizer` value that is threaded through the program.

use crate::cio::{c_isalnum, c_isalpha, c_isdigit, c_isspace, err, up_to_nul};

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // mirrors the C enum; some variants are never produced
pub enum TokenType {
    Eof = 0,
    Word = 1,
    Number = 2,
    Punctuation = 3,
    Whitespace = 4,
    Newline = 5,
    Identifier = 6,
    Keyword = 7,
    Operator = 8,
    Str = 9,
    Comment = 10,
    Error = 11,
}

impl TokenType {
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone)]
pub struct Token {
    pub ttype: TokenType,
    /// C string contents of `token.value` (no NUL terminator).
    pub value: Vec<u8>,
    #[allow(dead_code)] // mirrors token_t.length
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

static KEYWORDS: [&[u8]; 31] = [
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
];

fn is_keyword(s: &[u8]) -> bool {
    KEYWORDS.iter().any(|k| *k == s)
}

const OPERATOR_CHARS: &[u8] = b"+-*/%=<>!&|^~?:";
const PUNCTUATION_CHARS: &[u8] = b"(){}[];,.";

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

impl Tokenizer {
    pub fn new() -> Tokenizer {
        Tokenizer {
            input_buffer: vec![0u8; MAX_BUFFER_SIZE],
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

    fn peek_char(&self) -> u8 {
        if self.current_position >= self.buffer_length {
            return 0;
        }
        self.input_buffer[self.current_position]
    }

    fn advance_char(&mut self) -> u8 {
        if self.current_position >= self.buffer_length {
            return 0;
        }

        let c = self.input_buffer[self.current_position];
        self.current_position += 1;
        self.total_chars_processed += 1;

        if c == b'\n' {
            self.current_line = self.current_line.wrapping_add(1);
            self.current_column = 1;
            self.total_lines_processed += 1;
        } else {
            self.current_column = self.current_column.wrapping_add(1);
        }

        c
    }

    fn skip_whitespace(&mut self) {
        while self.peek_char() != 0 && c_isspace(self.peek_char()) && self.peek_char() != b'\n' {
            self.advance_char();
        }
    }

    fn create_token(&mut self, ttype: TokenType, value: &[u8], length: usize) -> Token {
        let len = if length < MAX_TOKEN_LENGTH {
            length
        } else {
            MAX_TOKEN_LENGTH - 1
        };
        // strncpy(token.value, value, len) followed by value[len] = '\0'
        let mut v: Vec<u8> = Vec::with_capacity(len);
        let src = up_to_nul(value);
        let copied = if src.len() < len { src.len() } else { len };
        v.extend_from_slice(&src[..copied]);
        // strncpy zero-pads, so the C string ends at `copied` bytes anyway.

        // `current_column - token.length`: size_t arithmetic truncated back to
        // int, i.e. wrapping 32-bit subtraction.
        let column = self.current_column.wrapping_sub(len as i32);

        self.total_tokens_processed += 1;

        Token {
            ttype,
            value: v,
            length: len,
            line: self.current_line,
            column,
        }
    }

    fn scan_word(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::new();

        while self.peek_char() != 0
            && (c_isalnum(self.peek_char()) || self.peek_char() == b'_')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            let c = self.advance_char();
            buffer.push(c);
        }

        let length = buffer.len();
        if is_keyword(&buffer) {
            return self.create_token(TokenType::Keyword, &buffer, length);
        }

        self.create_token(TokenType::Identifier, &buffer, length)
    }

    fn scan_number(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::new();
        let mut has_decimal = false;

        while self.peek_char() != 0
            && (c_isdigit(self.peek_char()) || self.peek_char() == b'.')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            if self.peek_char() == b'.' {
                if has_decimal {
                    break; // Second decimal point
                }
                has_decimal = true;
            }

            let c = self.advance_char();
            buffer.push(c);
        }

        let length = buffer.len();
        self.create_token(TokenType::Number, &buffer, length)
    }

    fn scan_string(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::new();
        let quote = self.advance_char(); // Consume opening quote

        buffer.push(quote);

        while self.peek_char() != 0
            && self.peek_char() != quote
            && self.peek_char() != b'\n'
            && buffer.len() < MAX_TOKEN_LENGTH - 2
        {
            if self.peek_char() == b'\\' {
                let c = self.advance_char(); // Escape character
                buffer.push(c);
                if self.peek_char() != 0 {
                    let c2 = self.advance_char(); // Escaped character
                    buffer.push(c2);
                }
            } else {
                let c = self.advance_char();
                buffer.push(c);
            }
        }

        if self.peek_char() == quote {
            let c = self.advance_char(); // Closing quote
            buffer.push(c);
        }

        let length = buffer.len();
        self.create_token(TokenType::Str, &buffer, length)
    }

    fn scan_comment(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::new();

        // Assume we've seen '/'
        let c = self.advance_char(); // First '/'
        buffer.push(c);

        if self.peek_char() == b'/' {
            // Single-line comment
            let c = self.advance_char(); // Second '/'
            buffer.push(c);

            while self.peek_char() != 0
                && self.peek_char() != b'\n'
                && buffer.len() < MAX_TOKEN_LENGTH - 1
            {
                let c = self.advance_char();
                buffer.push(c);
            }
        } else if self.peek_char() == b'*' {
            // Multi-line comment
            let c = self.advance_char(); // '*'
            buffer.push(c);

            while self.peek_char() != 0 && buffer.len() < MAX_TOKEN_LENGTH - 2 {
                if self.peek_char() == b'*' {
                    let c = self.advance_char();
                    buffer.push(c);
                    if self.peek_char() == b'/' {
                        let c = self.advance_char();
                        buffer.push(c);
                        break;
                    }
                } else {
                    let c = self.advance_char();
                    buffer.push(c);
                }
            }
        }

        let length = buffer.len();
        self.create_token(TokenType::Comment, &buffer, length)
    }

    fn scan_operator(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::new();
        let c = self.peek_char();

        let first = self.advance_char();
        buffer.push(first);

        // Check for two-character operators
        let next = self.peek_char();
        if (c == b'=' && next == b'=')
            || (c == b'!' && next == b'=')
            || (c == b'<' && next == b'=')
            || (c == b'>' && next == b'=')
            || (c == b'&' && next == b'&')
            || (c == b'|' && next == b'|')
            || (c == b'+' && next == b'+')
            || (c == b'-' && next == b'-')
            || (c == b'-' && next == b'>')
            || (c == b'<' && next == b'<')
            || (c == b'>' && next == b'>')
        {
            let c2 = self.advance_char();
            buffer.push(c2);
        }

        let length = buffer.len();
        self.create_token(TokenType::Operator, &buffer, length)
    }

    pub fn next_token(&mut self) -> Token {
        // Check if we have a lookahead token
        if let Some(tok) = self.lookahead_token.take() {
            return tok;
        }

        self.skip_whitespace();

        if self.current_position >= self.buffer_length {
            return self.create_token(TokenType::Eof, b"", 0);
        }

        let c = self.peek_char();

        // Newline
        if c == b'\n' {
            let nl = self.advance_char();
            return self.create_token(TokenType::Newline, &[nl], 1);
        }

        // Identifier or keyword
        if c_isalpha(c) || c == b'_' {
            return self.scan_word();
        }

        // Number
        if c_isdigit(c) {
            return self.scan_number();
        }

        // String
        if c == b'"' || c == b'\'' {
            return self.scan_string();
        }

        // Comment
        // NOTE: the C source compares `peek_char()` (== c) against '/' and '*',
        // so any '/' takes this branch. Preserved as-is.
        if c == b'/' && (self.peek_char() == b'/' || self.peek_char() == b'*') {
            return self.scan_comment();
        }

        // Operator
        if OPERATOR_CHARS.contains(&c) {
            return self.scan_operator();
        }

        // Punctuation
        if PUNCTUATION_CHARS.contains(&c) {
            let p = self.advance_char();
            return self.create_token(TokenType::Punctuation, &[p], 1);
        }

        // Unknown character
        let u = self.advance_char();
        self.create_token(TokenType::Error, &[u], 1)
    }

    #[allow(dead_code)]
    pub fn peek_token(&mut self) -> Token {
        if self.lookahead_token.is_none() {
            let tok = self.next_token();
            self.lookahead_token = Some(tok);
        }
        self.lookahead_token.clone().unwrap()
    }

    pub fn reset(&mut self) {
        self.current_position = 0;
        self.current_line = 1;
        self.current_column = 1;
        self.lookahead_token = None;
        // Note: We don't reset total statistics
    }

    /// `tokenizer_load_text`: returns 0 on success, -1 on failure.
    pub fn load_text(&mut self, text: &[u8]) -> i32 {
        // The C code checks for a NULL pointer; callers never pass one.
        let s = up_to_nul(text);
        let length = s.len();
        if length >= MAX_BUFFER_SIZE {
            err(b"Error: Input text too large\n");
            return -1;
        }

        for b in self.input_buffer.iter_mut() {
            *b = 0;
        }
        self.input_buffer[..length].copy_from_slice(s);
        self.buffer_length = length;

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

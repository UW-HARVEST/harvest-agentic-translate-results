//! Translation of `tokenizer.c` / `tokenizer.h`.

use std::cell::RefCell;

use crate::cio::{c_str, err_str, is_alnum, is_alpha, is_digit, is_space};

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TokenType {
    Eof = 0,
    #[allow(dead_code)]
    Word = 1,
    Number = 2,
    Punctuation = 3,
    #[allow(dead_code)]
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
    /// Contents of the `char value[MAX_TOKEN_LENGTH]` field as a C string.
    pub value: Vec<u8>,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

impl Token {
    fn empty() -> Token {
        Token {
            ttype: TokenType::Eof,
            value: Vec::new(),
            length: 0,
            line: 0,
            column: 0,
        }
    }
}

/// The function-pointer table from `tokenizer.h`.
#[derive(Copy, Clone)]
pub struct TokenizerOps {
    pub next_token: fn() -> Token,
    #[allow(dead_code)]
    pub peek_token: fn() -> Token,
    pub reset: fn(),
    pub load_text: fn(&[u8]) -> i32,
    pub get_stats: fn() -> (usize, usize, usize),
}

// Static keywords for recognition
const KEYWORDS: [&[u8]; 31] = [
    b"if", b"else", b"while", b"for", b"return", b"int", b"char", b"float",
    b"double", b"void", b"struct", b"typedef", b"const", b"static", b"extern",
    b"auto", b"register", b"sizeof", b"break", b"continue", b"switch", b"case",
    b"default", b"do", b"goto", b"enum", b"union", b"signed", b"unsigned",
    b"long", b"short",
];

fn is_keyword(s: &[u8]) -> bool {
    KEYWORDS.iter().any(|kw| *kw == s)
}

// Static global variables (file-local scope)
struct TokenizerState {
    input_buffer: Vec<u8>,
    buffer_length: usize,
    current_position: usize,
    current_line: i32,
    current_column: i32,
    total_tokens_processed: usize,
    total_lines_processed: usize,
    total_chars_processed: usize,
    lookahead_token: Token,
    lookahead_valid: bool,
}

impl TokenizerState {
    fn new() -> TokenizerState {
        TokenizerState {
            input_buffer: vec![0u8; MAX_BUFFER_SIZE],
            buffer_length: 0,
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens_processed: 0,
            total_lines_processed: 0,
            total_chars_processed: 0,
            lookahead_token: Token::empty(),
            lookahead_valid: false,
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
            self.current_line += 1;
            self.current_column = 1;
            self.total_lines_processed += 1;
        } else {
            self.current_column += 1;
        }

        c
    }

    fn skip_whitespace(&mut self) {
        while self.peek_char() != 0 && is_space(self.peek_char()) && self.peek_char() != b'\n' {
            self.advance_char();
        }
    }

    fn create_token(&mut self, ttype: TokenType, value: &[u8], length: usize) -> Token {
        let mut token = Token::empty();
        token.ttype = ttype;
        token.length = if length < MAX_TOKEN_LENGTH {
            length
        } else {
            MAX_TOKEN_LENGTH - 1
        };
        // strncpy(token.value, value, token.length); token.value[token.length] = 0;
        let copy_len = token.length.min(value.len());
        token.value = c_str(&value[..copy_len]).to_vec();
        token.line = self.current_line;
        // C computes `current_column - token.length` in size_t arithmetic and
        // then truncates the (possibly wrapped) result to `int`.
        token.column = (self.current_column as u64).wrapping_sub(token.length as u64) as u32 as i32;
        self.total_tokens_processed += 1;
        token
    }

    fn scan_word(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::new();

        while self.peek_char() != 0
            && (is_alnum(self.peek_char()) || self.peek_char() == b'_')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            let c = self.advance_char();
            buffer.push(c);
        }

        let length = buffer.len();

        // Check if it's a keyword
        if is_keyword(&buffer) {
            return self.create_token(TokenType::Keyword, &buffer, length);
        }

        self.create_token(TokenType::Identifier, &buffer, length)
    }

    fn scan_number(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::new();
        let mut has_decimal = false;

        while self.peek_char() != 0
            && (is_digit(self.peek_char()) || self.peek_char() == b'.')
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
                    let c = self.advance_char(); // Escaped character
                    buffer.push(c);
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

    fn next_token(&mut self) -> Token {
        // Check if we have a lookahead token
        if self.lookahead_valid {
            self.lookahead_valid = false;
            return self.lookahead_token.clone();
        }

        self.skip_whitespace();

        if self.current_position >= self.buffer_length {
            return self.create_token(TokenType::Eof, b"", 0);
        }

        let c = self.peek_char();

        // Newline
        if c == b'\n' {
            let newline = [self.advance_char()];
            return self.create_token(TokenType::Newline, &newline, 1);
        }

        // Identifier or keyword
        if is_alpha(c) || c == b'_' {
            return self.scan_word();
        }

        // Number
        if is_digit(c) {
            return self.scan_number();
        }

        // String
        if c == b'"' || c == b'\'' {
            return self.scan_string();
        }

        // Comment
        // NOTE: this mirrors the original C exactly.  `peek_char()` still
        // returns the unconsumed '/' itself, so any '/' is scanned as a
        // comment.
        if c == b'/' && (self.peek_char() == b'/' || self.peek_char() == b'*') {
            return self.scan_comment();
        }

        // Operator
        if b"+-*/%=<>!&|^~?:".contains(&c) {
            return self.scan_operator();
        }

        // Punctuation
        if b"(){}[];,.".contains(&c) {
            let punct = [self.advance_char()];
            return self.create_token(TokenType::Punctuation, &punct, 1);
        }

        // Unknown character
        let unknown = [self.advance_char()];
        self.create_token(TokenType::Error, &unknown, 1)
    }

    fn reset(&mut self) {
        self.current_position = 0;
        self.current_line = 1;
        self.current_column = 1;
        self.lookahead_valid = false;
        // Note: We don't reset total statistics
    }
}

thread_local! {
    static STATE: RefCell<TokenizerState> = RefCell::new(TokenizerState::new());
}

// Public functions that use static globals
pub fn tokenizer_next_token() -> Token {
    STATE.with(|cell| cell.borrow_mut().next_token())
}

pub fn tokenizer_peek_token() -> Token {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        if !state.lookahead_valid {
            let token = state.next_token();
            state.lookahead_token = token;
            state.lookahead_valid = true;
        }
        state.lookahead_token.clone()
    })
}

pub fn tokenizer_reset() {
    STATE.with(|cell| cell.borrow_mut().reset());
}

pub fn tokenizer_load_text(text: &[u8]) -> i32 {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();

        let length = text.len();
        if length >= MAX_BUFFER_SIZE {
            err_str("Error: Input text too large\n");
            return -1;
        }

        // strncpy(input_buffer, text, MAX_BUFFER_SIZE - 1) pads with NULs.
        for byte in state.input_buffer.iter_mut() {
            *byte = 0;
        }
        state.input_buffer[..length].copy_from_slice(text);
        state.buffer_length = length;

        state.reset();

        0
    })
}

pub fn tokenizer_get_stats() -> (usize, usize, usize) {
    STATE.with(|cell| {
        let state = cell.borrow();
        (
            state.total_lines_processed,
            state.total_tokens_processed,
            state.total_chars_processed,
        )
    })
}

// Function that returns function pointers
pub fn get_tokenizer_ops() -> TokenizerOps {
    TokenizerOps {
        next_token: tokenizer_next_token,
        peek_token: tokenizer_peek_token,
        reset: tokenizer_reset,
        load_text: tokenizer_load_text,
        get_stats: tokenizer_get_stats,
    }
}

//! Port of c_src/src/tokenizer.c
//!
//! The C file keeps all of its state in file-scope `static` variables and
//! exposes the operations both directly and through a struct of function
//! pointers. That shape is preserved here: the state lives in one
//! thread-local `Tokenizer`, and `TokenizerOps` holds `fn` pointers to the
//! free functions that drive it.

use std::cell::RefCell;

use crate::ceprintf;

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

// token_type_t
pub const TOKEN_EOF: usize = 0;
pub const TOKEN_WORD: usize = 1;
pub const TOKEN_NUMBER: usize = 2;
pub const TOKEN_PUNCTUATION: usize = 3;
pub const TOKEN_WHITESPACE: usize = 4;
pub const TOKEN_NEWLINE: usize = 5;
pub const TOKEN_IDENTIFIER: usize = 6;
pub const TOKEN_KEYWORD: usize = 7;
pub const TOKEN_OPERATOR: usize = 8;
pub const TOKEN_STRING: usize = 9;
pub const TOKEN_COMMENT: usize = 10;
pub const TOKEN_ERROR: usize = 11;

/// token_t. `value` holds the NUL-terminated string contents of the C
/// `char value[MAX_TOKEN_LENGTH]` field.
#[derive(Clone, Default)]
pub struct Token {
    pub ttype: usize,
    pub value: Vec<u8>,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

/// tokenizer_ops_t
#[derive(Clone, Copy)]
pub struct TokenizerOps {
    pub next_token: fn() -> Token,
    #[allow(dead_code)]
    pub peek_token: fn() -> Token,
    pub reset: fn(),
    pub load_text: fn(&[u8]) -> i32,
    pub get_stats: fn() -> (usize, usize, usize),
}

// Static keywords for recognition
static KEYWORDS: [&[u8]; 31] = [
    b"if", b"else", b"while", b"for", b"return", b"int", b"char", b"float",
    b"double", b"void", b"struct", b"typedef", b"const", b"static", b"extern",
    b"auto", b"register", b"sizeof", b"break", b"continue", b"switch", b"case",
    b"default", b"do", b"goto", b"enum", b"union", b"signed", b"unsigned",
    b"long", b"short",
];

// ---------------------------------------------------------------------------
// C locale <ctype.h> predicates.
//
// The C code passes a plain (signed) `char` to these; in the C locale glibc
// classifies both negative values and 0x80..0xff as none of alpha/digit/
// alnum/space, so plain ASCII tests are exact.
// ---------------------------------------------------------------------------

fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn c_isalpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn c_isdigit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn c_isalnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn is_keyword(s: &[u8]) -> bool {
    for kw in KEYWORDS.iter() {
        if s == *kw {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Tokenizer state (the C file's static globals)
// ---------------------------------------------------------------------------

struct Tokenizer {
    input_buffer: [u8; MAX_BUFFER_SIZE],
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

impl Tokenizer {
    fn new() -> Self {
        Tokenizer {
            input_buffer: [0u8; MAX_BUFFER_SIZE],
            buffer_length: 0,
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens_processed: 0,
            total_lines_processed: 0,
            total_chars_processed: 0,
            lookahead_token: Token::default(),
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

    fn create_token(&mut self, ttype: usize, value: &[u8], length: usize) -> Token {
        let length = if length < MAX_TOKEN_LENGTH {
            length
        } else {
            MAX_TOKEN_LENGTH - 1
        };
        // strncpy(token.value, value, token.length); token.value[length] = '\0';
        let mut buf = vec![0u8; length];
        for i in 0..length {
            if i < value.len() {
                buf[i] = value[i];
            } else {
                break;
            }
        }
        // The C string stops at the first NUL written by strncpy's padding.
        let stored = crate::cio::cstr(&buf).to_vec();

        // NOTE (faithful to the C): `current_column - token.length` mixes int
        // with size_t, so the subtraction happens in size_t and the result is
        // truncated back into an int. That wraps for short columns.
        let column = self.current_column.wrapping_sub(length as i32);

        self.total_tokens_processed += 1;

        Token {
            ttype,
            value: stored,
            length,
            line: self.current_line,
            column,
        }
    }

    fn scan_word(&mut self) -> Token {
        let mut buffer = [0u8; MAX_TOKEN_LENGTH];
        let mut length = 0usize;

        while self.peek_char() != 0
            && (c_isalnum(self.peek_char()) || self.peek_char() == b'_')
            && length < MAX_TOKEN_LENGTH - 1
        {
            buffer[length] = self.advance_char();
            length += 1;
        }

        let word = buffer[..length].to_vec();

        if is_keyword(&word) {
            return self.create_token(TOKEN_KEYWORD, &word, length);
        }

        self.create_token(TOKEN_IDENTIFIER, &word, length)
    }

    fn scan_number(&mut self) -> Token {
        let mut buffer = [0u8; MAX_TOKEN_LENGTH];
        let mut length = 0usize;
        let mut has_decimal = false;

        while self.peek_char() != 0
            && (c_isdigit(self.peek_char()) || self.peek_char() == b'.')
            && length < MAX_TOKEN_LENGTH - 1
        {
            if self.peek_char() == b'.' {
                if has_decimal {
                    break; // Second decimal point
                }
                has_decimal = true;
            }

            buffer[length] = self.advance_char();
            length += 1;
        }

        let number = buffer[..length].to_vec();
        self.create_token(TOKEN_NUMBER, &number, length)
    }

    fn scan_string(&mut self) -> Token {
        let mut buffer = [0u8; MAX_TOKEN_LENGTH];
        let mut length = 0usize;
        let quote = self.advance_char(); // Consume opening quote

        buffer[length] = quote;
        length += 1;

        while self.peek_char() != 0
            && self.peek_char() != quote
            && self.peek_char() != b'\n'
            && length < MAX_TOKEN_LENGTH - 2
        {
            if self.peek_char() == b'\\' {
                buffer[length] = self.advance_char(); // Escape character
                length += 1;
                if self.peek_char() != 0 {
                    buffer[length] = self.advance_char(); // Escaped character
                    length += 1;
                }
            } else {
                buffer[length] = self.advance_char();
                length += 1;
            }
        }

        if self.peek_char() == quote {
            buffer[length] = self.advance_char(); // Closing quote
            length += 1;
        }

        let s = buffer[..length].to_vec();
        self.create_token(TOKEN_STRING, &s, length)
    }

    fn scan_comment(&mut self) -> Token {
        let mut buffer = [0u8; MAX_TOKEN_LENGTH];
        let mut length = 0usize;

        // Assume we've seen '/'
        buffer[length] = self.advance_char(); // First '/'
        length += 1;

        if self.peek_char() == b'/' {
            // Single-line comment
            buffer[length] = self.advance_char(); // Second '/'
            length += 1;

            while self.peek_char() != 0
                && self.peek_char() != b'\n'
                && length < MAX_TOKEN_LENGTH - 1
            {
                buffer[length] = self.advance_char();
                length += 1;
            }
        } else if self.peek_char() == b'*' {
            // Multi-line comment
            buffer[length] = self.advance_char(); // '*'
            length += 1;

            while self.peek_char() != 0 && length < MAX_TOKEN_LENGTH - 2 {
                if self.peek_char() == b'*' {
                    buffer[length] = self.advance_char();
                    length += 1;
                    if self.peek_char() == b'/' {
                        buffer[length] = self.advance_char();
                        length += 1;
                        break;
                    }
                } else {
                    buffer[length] = self.advance_char();
                    length += 1;
                }
            }
        }

        let s = buffer[..length].to_vec();
        self.create_token(TOKEN_COMMENT, &s, length)
    }

    fn scan_operator(&mut self) -> Token {
        let mut buffer = [0u8; MAX_TOKEN_LENGTH];
        let mut length = 0usize;
        let c = self.peek_char();

        buffer[length] = self.advance_char();
        length += 1;

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
            buffer[length] = self.advance_char();
            length += 1;
        }

        let s = buffer[..length].to_vec();
        self.create_token(TOKEN_OPERATOR, &s, length)
    }

    fn next_token(&mut self) -> Token {
        // Check if we have a lookahead token
        if self.lookahead_valid {
            self.lookahead_valid = false;
            return self.lookahead_token.clone();
        }

        self.skip_whitespace();

        if self.current_position >= self.buffer_length {
            return self.create_token(TOKEN_EOF, b"", 0);
        }

        let c = self.peek_char();

        // Newline
        if c == b'\n' {
            let newline = [self.advance_char()];
            return self.create_token(TOKEN_NEWLINE, &newline, 1);
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
        //
        // NOTE (faithful to the C): peek_char() has not advanced, so it still
        // returns `c`. When c == '/' the condition `peek_char() == '/'` is
        // always true, so every '/' is scanned as a comment and the operator
        // branch below is unreachable for '/'.
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
            return self.create_token(TOKEN_PUNCTUATION, &punct, 1);
        }

        // Unknown character
        let unknown = [self.advance_char()];
        self.create_token(TOKEN_ERROR, &unknown, 1)
    }

    fn reset(&mut self) {
        self.current_position = 0;
        self.current_line = 1;
        self.current_column = 1;
        self.lookahead_valid = false;
        // Note: We don't reset total statistics
    }

    fn load_text(&mut self, text: &[u8]) -> i32 {
        let length = text.len(); // strlen(text)
        if length >= MAX_BUFFER_SIZE {
            ceprintf!("Error: Input text too large\n");
            return -1;
        }

        // strncpy(input_buffer, text, MAX_BUFFER_SIZE - 1)
        let copy = if length < MAX_BUFFER_SIZE - 1 {
            length
        } else {
            MAX_BUFFER_SIZE - 1
        };
        self.input_buffer[..copy].copy_from_slice(&text[..copy]);
        for b in self.input_buffer[copy..].iter_mut() {
            *b = 0;
        }
        self.buffer_length = length;

        self.reset();

        0
    }
}

thread_local! {
    static TOKENIZER: RefCell<Tokenizer> = RefCell::new(Tokenizer::new());
}

// ---------------------------------------------------------------------------
// Public functions that use the static state
// ---------------------------------------------------------------------------

pub fn tokenizer_next_token() -> Token {
    TOKENIZER.with(|t| t.borrow_mut().next_token())
}

pub fn tokenizer_peek_token() -> Token {
    TOKENIZER.with(|t| {
        let mut tok = t.borrow_mut();
        if !tok.lookahead_valid {
            let next = tok.next_token();
            tok.lookahead_token = next;
            tok.lookahead_valid = true;
        }
        tok.lookahead_token.clone()
    })
}

pub fn tokenizer_reset() {
    TOKENIZER.with(|t| t.borrow_mut().reset());
}

pub fn tokenizer_load_text(text: &[u8]) -> i32 {
    TOKENIZER.with(|t| t.borrow_mut().load_text(text))
}

pub fn tokenizer_get_stats() -> (usize, usize, usize) {
    TOKENIZER.with(|t| {
        let tok = t.borrow();
        (
            tok.total_lines_processed,
            tok.total_tokens_processed,
            tok.total_chars_processed,
        )
    })
}

/// Function that returns function pointers
pub fn get_tokenizer_ops() -> TokenizerOps {
    TokenizerOps {
        next_token: tokenizer_next_token,
        peek_token: tokenizer_peek_token,
        reset: tokenizer_reset,
        load_text: tokenizer_load_text,
        get_stats: tokenizer_get_stats,
    }
}

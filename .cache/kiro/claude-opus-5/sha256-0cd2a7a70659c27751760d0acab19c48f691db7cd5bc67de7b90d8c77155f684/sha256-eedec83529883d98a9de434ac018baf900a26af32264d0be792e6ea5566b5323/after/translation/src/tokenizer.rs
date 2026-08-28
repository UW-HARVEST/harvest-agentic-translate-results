//! Translation of tokenizer.c
//!
//! The C version keeps all state in file-scope statics. Here the same state
//! lives in a `Tokenizer` struct that is threaded through by reference, which
//! is behaviorally identical while staying in safe Rust.

use crate::cio::{err_str, is_alnum, is_alpha, is_digit, is_space};

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

// token_type_t
pub const TOKEN_EOF: usize = 0;
pub const TOKEN_WORD: usize = 1;
pub const TOKEN_NUMBER: usize = 2;
pub const TOKEN_PUNCTUATION: usize = 3;
#[allow(dead_code)]
pub const TOKEN_WHITESPACE: usize = 4;
pub const TOKEN_NEWLINE: usize = 5;
pub const TOKEN_IDENTIFIER: usize = 6;
pub const TOKEN_KEYWORD: usize = 7;
pub const TOKEN_OPERATOR: usize = 8;
pub const TOKEN_STRING: usize = 9;
pub const TOKEN_COMMENT: usize = 10;
pub const TOKEN_ERROR: usize = 11;

#[derive(Clone)]
pub struct Token {
    pub ttype: usize,
    /// Contents of `token.value` up to its NUL terminator.
    pub value: Vec<u8>,
    #[allow(dead_code)]
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

impl Token {
    fn empty() -> Token {
        Token {
            ttype: TOKEN_EOF,
            value: Vec::new(),
            length: 0,
            line: 0,
            column: 0,
        }
    }
}

const KEYWORDS: [&[u8]; 31] = [
    b"if", b"else", b"while", b"for", b"return", b"int", b"char", b"float", b"double", b"void",
    b"struct", b"typedef", b"const", b"static", b"extern", b"auto", b"register", b"sizeof",
    b"break", b"continue", b"switch", b"case", b"default", b"do", b"goto", b"enum", b"union",
    b"signed", b"unsigned", b"long", b"short",
];

fn is_keyword(s: &[u8]) -> bool {
    KEYWORDS.iter().any(|&k| k == s)
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
    lookahead_token: Token,
    lookahead_valid: bool,
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

    fn create_token(&mut self, ttype: usize, value: &[u8], length: usize) -> Token {
        let tlen = if length < MAX_TOKEN_LENGTH {
            length
        } else {
            MAX_TOKEN_LENGTH - 1
        };
        // `token.column = current_column - token.length` mixes int and size_t in
        // C; the unsigned subtraction truncated back to int is a wrapping i32
        // subtraction.
        let token = Token {
            ttype,
            value: value[..tlen].to_vec(),
            length: tlen,
            line: self.current_line,
            column: self.current_column.wrapping_sub(tlen as i32),
        };
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

        if is_keyword(&buffer) {
            return self.create_token(TOKEN_KEYWORD, &buffer, length);
        }

        self.create_token(TOKEN_IDENTIFIER, &buffer, length)
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
        self.create_token(TOKEN_NUMBER, &buffer, length)
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
        self.create_token(TOKEN_STRING, &buffer, length)
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
        self.create_token(TOKEN_COMMENT, &buffer, length)
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
        self.create_token(TOKEN_OPERATOR, &buffer, length)
    }

    pub fn next_token(&mut self) -> Token {
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
            let nl = [self.advance_char()];
            return self.create_token(TOKEN_NEWLINE, &nl, 1);
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
        //
        // NOTE: the original compares `peek_char()` (which is still `c`) against
        // '/' and '*', so this condition is true for *every* '/'. Division is
        // therefore always tokenized as a comment. Reproduced verbatim.
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

    #[allow(dead_code)]
    pub fn peek_token(&mut self) -> Token {
        if !self.lookahead_valid {
            self.lookahead_token = self.next_token();
            self.lookahead_valid = true;
        }
        self.lookahead_token.clone()
    }

    pub fn reset(&mut self) {
        self.current_position = 0;
        self.current_line = 1;
        self.current_column = 1;
        self.lookahead_valid = false;
        // Note: We don't reset total statistics
    }

    /// `text` is a C string (no embedded NUL), so its length is `strlen(text)`.
    pub fn load_text(&mut self, text: &[u8]) -> i32 {
        let length = text.len();
        if length >= MAX_BUFFER_SIZE {
            err_str("Error: Input text too large\n");
            return -1;
        }

        // strncpy into the static buffer: copies `length` bytes and NUL-pads.
        self.input_buffer.clear();
        self.input_buffer.resize(MAX_BUFFER_SIZE, 0);
        self.input_buffer[..length].copy_from_slice(text);
        self.buffer_length = length;

        self.reset();

        0
    }

    /// `tokenizer_get_stats(&lines, &tokens, &chars)`
    pub fn get_stats(&self) -> (usize, usize, usize) {
        (
            self.total_lines_processed,
            self.total_tokens_processed,
            self.total_chars_processed,
        )
    }
}

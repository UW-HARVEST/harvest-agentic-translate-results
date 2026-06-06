// Translated from c_src/src/tokenizer.c

use std::cell::RefCell;

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(usize)]
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
    String = 9,
    Comment = 10,
    Error = 11,
}

#[derive(Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: Vec<u8>, // raw bytes (no null terminator)
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

#[derive(Copy, Clone)]
pub struct TokenizerOps {
    pub next_token: fn() -> Token,
    pub peek_token: fn() -> Token,
    pub reset: fn(),
    pub load_text: fn(&[u8]) -> i32,
    pub get_stats: fn() -> (usize, usize, usize),
}

struct TokenizerState {
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

impl TokenizerState {
    fn new() -> Self {
        TokenizerState {
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
            self.current_line += 1;
            self.current_column = 1;
            self.total_lines_processed += 1;
        } else {
            self.current_column += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        loop {
            let c = self.peek_char();
            if c == 0 || !is_space_c(c) || c == b'\n' {
                break;
            }
            self.advance_char();
        }
    }

    fn create_token(&mut self, ttype: TokenType, value: &[u8]) -> Token {
        let length = if value.len() < MAX_TOKEN_LENGTH {
            value.len()
        } else {
            MAX_TOKEN_LENGTH - 1
        };
        let v = value[..length].to_vec();
        let column = self.current_column - length as i32;
        self.total_tokens_processed += 1;
        Token {
            token_type: ttype,
            value: v,
            length,
            line: self.current_line,
            column,
        }
    }

    fn scan_word(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::with_capacity(MAX_TOKEN_LENGTH);
        loop {
            let c = self.peek_char();
            if c == 0 || !(is_alnum_c(c) || c == b'_') || buffer.len() >= MAX_TOKEN_LENGTH - 1 {
                break;
            }
            buffer.push(self.advance_char());
        }
        if is_keyword(&buffer) {
            self.create_token(TokenType::Keyword, &buffer)
        } else {
            self.create_token(TokenType::Identifier, &buffer)
        }
    }

    fn scan_number(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::with_capacity(MAX_TOKEN_LENGTH);
        let mut has_decimal = false;
        loop {
            let c = self.peek_char();
            if c == 0 || !(is_digit_c(c) || c == b'.') || buffer.len() >= MAX_TOKEN_LENGTH - 1 {
                break;
            }
            if c == b'.' {
                if has_decimal {
                    break;
                }
                has_decimal = true;
            }
            buffer.push(self.advance_char());
        }
        self.create_token(TokenType::Number, &buffer)
    }

    fn scan_string(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::with_capacity(MAX_TOKEN_LENGTH);
        let quote = self.advance_char();
        buffer.push(quote);
        loop {
            let c = self.peek_char();
            if c == 0 || c == quote || c == b'\n' || buffer.len() >= MAX_TOKEN_LENGTH - 2 {
                break;
            }
            if c == b'\\' {
                buffer.push(self.advance_char());
                if self.peek_char() != 0 {
                    buffer.push(self.advance_char());
                }
            } else {
                buffer.push(self.advance_char());
            }
        }
        if self.peek_char() == quote {
            buffer.push(self.advance_char());
        }
        self.create_token(TokenType::String, &buffer)
    }

    fn scan_comment(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::with_capacity(MAX_TOKEN_LENGTH);
        // Consume first '/'
        buffer.push(self.advance_char());
        if self.peek_char() == b'/' {
            // single line
            buffer.push(self.advance_char());
            loop {
                let c = self.peek_char();
                if c == 0 || c == b'\n' || buffer.len() >= MAX_TOKEN_LENGTH - 1 {
                    break;
                }
                buffer.push(self.advance_char());
            }
        } else if self.peek_char() == b'*' {
            // multi line
            buffer.push(self.advance_char());
            loop {
                let c = self.peek_char();
                if c == 0 || buffer.len() >= MAX_TOKEN_LENGTH - 2 {
                    break;
                }
                if c == b'*' {
                    buffer.push(self.advance_char());
                    if self.peek_char() == b'/' {
                        buffer.push(self.advance_char());
                        break;
                    }
                } else {
                    buffer.push(self.advance_char());
                }
            }
        }
        self.create_token(TokenType::Comment, &buffer)
    }

    fn scan_operator(&mut self) -> Token {
        let mut buffer: Vec<u8> = Vec::with_capacity(MAX_TOKEN_LENGTH);
        let c = self.peek_char();
        buffer.push(self.advance_char());
        let next = self.peek_char();
        let two_char = (c == b'=' && next == b'=')
            || (c == b'!' && next == b'=')
            || (c == b'<' && next == b'=')
            || (c == b'>' && next == b'=')
            || (c == b'&' && next == b'&')
            || (c == b'|' && next == b'|')
            || (c == b'+' && next == b'+')
            || (c == b'-' && next == b'-')
            || (c == b'-' && next == b'>')
            || (c == b'<' && next == b'<')
            || (c == b'>' && next == b'>');
        if two_char {
            buffer.push(self.advance_char());
        }
        self.create_token(TokenType::Operator, &buffer)
    }
}

fn is_space_c(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_alpha_c(c: u8) -> bool {
    (b'a'..=b'z').contains(&c) || (b'A'..=b'Z').contains(&c)
}

fn is_digit_c(c: u8) -> bool {
    (b'0'..=b'9').contains(&c)
}

fn is_alnum_c(c: u8) -> bool {
    is_alpha_c(c) || is_digit_c(c)
}

const KEYWORDS: &[&[u8]] = &[
    b"if", b"else", b"while", b"for", b"return", b"int", b"char", b"float", b"double", b"void",
    b"struct", b"typedef", b"const", b"static", b"extern", b"auto", b"register", b"sizeof",
    b"break", b"continue", b"switch", b"case", b"default", b"do", b"goto", b"enum", b"union",
    b"signed", b"unsigned", b"long", b"short",
];

fn is_keyword(s: &[u8]) -> bool {
    KEYWORDS.iter().any(|&k| k == s)
}

thread_local! {
    static STATE: RefCell<TokenizerState> = RefCell::new(TokenizerState::new());
}

pub fn tokenizer_next_token() -> Token {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(tok) = s.lookahead_token.take() {
            return tok;
        }
        s.skip_whitespace();
        if s.current_position >= s.buffer_length {
            return s.create_token(TokenType::Eof, b"");
        }
        let c = s.peek_char();

        // Newline
        if c == b'\n' {
            let nl = s.advance_char();
            return s.create_token(TokenType::Newline, &[nl]);
        }

        // Identifier or keyword
        if is_alpha_c(c) || c == b'_' {
            return s.scan_word();
        }

        // Number
        if is_digit_c(c) {
            return s.scan_number();
        }

        // String
        if c == b'"' || c == b'\'' {
            return s.scan_string();
        }

        // Comment (preserves C bug: peek_char() == c after assignment to c, so always
        // matches when c == '/')
        if c == b'/' && (s.peek_char() == b'/' || s.peek_char() == b'*') {
            return s.scan_comment();
        }

        // Operator
        if b"+-*/%=<>!&|^~?:".contains(&c) {
            return s.scan_operator();
        }

        // Punctuation
        if b"(){}[];,.".contains(&c) {
            let p = s.advance_char();
            return s.create_token(TokenType::Punctuation, &[p]);
        }

        // Unknown
        let unk = s.advance_char();
        s.create_token(TokenType::Error, &[unk])
    })
}

pub fn tokenizer_peek_token() -> Token {
    let needs_fetch = STATE.with(|s| s.borrow().lookahead_token.is_none());
    if needs_fetch {
        let tok = tokenizer_next_token();
        STATE.with(|s| s.borrow_mut().lookahead_token = Some(tok.clone()));
        tok
    } else {
        STATE.with(|s| s.borrow().lookahead_token.clone().unwrap())
    }
}

pub fn tokenizer_reset() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.current_position = 0;
        s.current_line = 1;
        s.current_column = 1;
        s.lookahead_token = None;
    });
}

pub fn tokenizer_load_text(text: &[u8]) -> i32 {
    if text.len() >= MAX_BUFFER_SIZE {
        eprintln!("Error: Input text too large");
        return -1;
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let copy_len = text.len().min(MAX_BUFFER_SIZE - 1);
        for i in 0..copy_len {
            s.input_buffer[i] = text[i];
        }
        // Null terminator at MAX_BUFFER_SIZE - 1 (matches strncpy + manual null)
        s.input_buffer[MAX_BUFFER_SIZE - 1] = 0;
        s.buffer_length = text.len();
    });
    tokenizer_reset();
    0
}

pub fn tokenizer_get_stats() -> (usize, usize, usize) {
    STATE.with(|s| {
        let s = s.borrow();
        (
            s.total_lines_processed,
            s.total_tokens_processed,
            s.total_chars_processed,
        )
    })
}

pub fn get_tokenizer_ops() -> TokenizerOps {
    TokenizerOps {
        next_token: tokenizer_next_token,
        peek_token: tokenizer_peek_token,
        reset: tokenizer_reset,
        load_text: tokenizer_load_text,
        get_stats: tokenizer_get_stats,
    }
}

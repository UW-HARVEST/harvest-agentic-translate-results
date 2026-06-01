// Rust translation of c_src/src/tokenizer.c

#![allow(dead_code)]

use std::cell::RefCell;
use std::io::Write;

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub ty: TokenType,
    /// raw bytes (excludes the trailing null); length matches `length`
    pub value: Vec<u8>,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

impl Token {
    pub fn empty() -> Token {
        Token {
            ty: TokenType::Eof,
            value: Vec::new(),
            length: 0,
            line: 1,
            column: 1,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TokenizerOps {
    pub next_token: fn() -> Token,
    pub peek_token: fn() -> Token,
    pub reset: fn(),
    pub load_text: fn(&[u8]) -> i32,
    pub get_stats: fn() -> (usize, usize, usize),
}

pub struct TokenizerState {
    pub input_buffer: Vec<u8>,
    pub buffer_length: usize,
    pub current_position: usize,
    pub current_line: i32,
    pub current_column: i32,
    pub total_tokens_processed: usize,
    pub total_lines_processed: usize,
    pub total_chars_processed: usize,
    pub lookahead_token: Token,
    pub lookahead_valid: bool,
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
            lookahead_token: Token::empty(),
            lookahead_valid: false,
        }
    }
}

thread_local! {
    static STATE: RefCell<TokenizerState> = RefCell::new(TokenizerState::new());
}

const KEYWORDS: &[&[u8]] = &[
    b"if", b"else", b"while", b"for", b"return", b"int", b"char",
    b"float", b"double", b"void", b"struct", b"typedef", b"const",
    b"static", b"extern", b"auto", b"register", b"sizeof", b"break",
    b"continue", b"switch", b"case", b"default", b"do", b"goto",
    b"enum", b"union", b"signed", b"unsigned", b"long", b"short",
];

fn is_keyword(s: &[u8]) -> bool {
    KEYWORDS.iter().any(|k| *k == s)
}

// C's isspace: space, \t, \n, \v, \f, \r
fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn c_isdigit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn c_isalpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn c_isalnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn peek_char(s: &TokenizerState) -> u8 {
    if s.current_position >= s.buffer_length {
        return 0;
    }
    s.input_buffer[s.current_position]
}

fn advance_char(s: &mut TokenizerState) -> u8 {
    if s.current_position >= s.buffer_length {
        return 0;
    }
    let c = s.input_buffer[s.current_position];
    s.current_position += 1;
    s.total_chars_processed += 1;
    if c == b'\n' {
        s.current_line += 1;
        s.current_column = 1;
        s.total_lines_processed += 1;
    } else {
        s.current_column += 1;
    }
    c
}

fn skip_whitespace(s: &mut TokenizerState) {
    loop {
        let c = peek_char(s);
        if c == 0 || !c_isspace(c) || c == b'\n' {
            break;
        }
        advance_char(s);
    }
}

fn create_token(s: &mut TokenizerState, ty: TokenType, value: &[u8], length: usize) -> Token {
    let length = if length < MAX_TOKEN_LENGTH {
        length
    } else {
        MAX_TOKEN_LENGTH - 1
    };
    let mut buf = Vec::with_capacity(length);
    buf.extend_from_slice(&value[..length.min(value.len())]);
    // Mirror strncpy: if value shorter than length, C reads up to value.len() then pads with zero,
    // but in this codebase length is always <= len(value), so this is fine.
    while buf.len() < length {
        buf.push(0);
    }
    s.total_tokens_processed += 1;
    Token {
        ty,
        value: buf,
        length,
        line: s.current_line,
        column: s.current_column - length as i32,
    }
}

fn scan_word(s: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    let mut length: usize = 0;
    while peek_char(s) != 0
        && (c_isalnum(peek_char(s)) || peek_char(s) == b'_')
        && length < MAX_TOKEN_LENGTH - 1
    {
        let c = advance_char(s);
        buffer.push(c);
        length += 1;
    }
    if is_keyword(&buffer) {
        return create_token(s, TokenType::Keyword, &buffer, length);
    }
    create_token(s, TokenType::Identifier, &buffer, length)
}

fn scan_number(s: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    let mut length: usize = 0;
    let mut has_decimal = false;
    while peek_char(s) != 0
        && (c_isdigit(peek_char(s)) || peek_char(s) == b'.')
        && length < MAX_TOKEN_LENGTH - 1
    {
        if peek_char(s) == b'.' {
            if has_decimal {
                break;
            }
            has_decimal = true;
        }
        let c = advance_char(s);
        buffer.push(c);
        length += 1;
    }
    create_token(s, TokenType::Number, &buffer, length)
}

fn scan_string(s: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    let mut length: usize = 0;
    let quote = advance_char(s); // Consume opening quote
    buffer.push(quote);
    length += 1;

    while peek_char(s) != 0
        && peek_char(s) != quote
        && peek_char(s) != b'\n'
        && length < MAX_TOKEN_LENGTH - 2
    {
        if peek_char(s) == b'\\' {
            let c = advance_char(s);
            buffer.push(c);
            length += 1;
            if peek_char(s) != 0 {
                let c2 = advance_char(s);
                buffer.push(c2);
                length += 1;
            }
        } else {
            let c = advance_char(s);
            buffer.push(c);
            length += 1;
        }
    }

    if peek_char(s) == quote {
        let c = advance_char(s);
        buffer.push(c);
        length += 1;
    }

    create_token(s, TokenType::String, &buffer, length)
}

fn scan_comment(s: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    let mut length: usize = 0;

    // Assume we've seen '/'
    let c = advance_char(s); // First '/'
    buffer.push(c);
    length += 1;

    if peek_char(s) == b'/' {
        let c = advance_char(s); // Second '/'
        buffer.push(c);
        length += 1;
        while peek_char(s) != 0 && peek_char(s) != b'\n' && length < MAX_TOKEN_LENGTH - 1 {
            let c = advance_char(s);
            buffer.push(c);
            length += 1;
        }
    } else if peek_char(s) == b'*' {
        let c = advance_char(s); // '*'
        buffer.push(c);
        length += 1;
        while peek_char(s) != 0 && length < MAX_TOKEN_LENGTH - 2 {
            if peek_char(s) == b'*' {
                let c = advance_char(s);
                buffer.push(c);
                length += 1;
                if peek_char(s) == b'/' {
                    let c2 = advance_char(s);
                    buffer.push(c2);
                    length += 1;
                    break;
                }
            } else {
                let c = advance_char(s);
                buffer.push(c);
                length += 1;
            }
        }
    }

    create_token(s, TokenType::Comment, &buffer, length)
}

fn scan_operator(s: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    let mut length: usize = 0;
    let c = peek_char(s);

    let ch = advance_char(s);
    buffer.push(ch);
    length += 1;

    let next = peek_char(s);
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
        let c2 = advance_char(s);
        buffer.push(c2);
        length += 1;
    }

    create_token(s, TokenType::Operator, &buffer, length)
}

const OPERATOR_CHARS: &[u8] = b"+-*/%=<>!&|^~?:";
const PUNCT_CHARS: &[u8] = b"(){}[];,.";

pub fn tokenizer_next_token() -> Token {
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        if s.lookahead_valid {
            s.lookahead_valid = false;
            return s.lookahead_token.clone();
        }
        skip_whitespace(&mut s);

        if s.current_position >= s.buffer_length {
            return create_token(&mut s, TokenType::Eof, b"", 0);
        }

        let c = peek_char(&s);

        // Newline
        if c == b'\n' {
            let nl = advance_char(&mut s);
            return create_token(&mut s, TokenType::Newline, &[nl], 1);
        }

        // Identifier or keyword
        if c_isalpha(c) || c == b'_' {
            return scan_word(&mut s);
        }

        // Number
        if c_isdigit(c) {
            return scan_number(&mut s);
        }

        // String
        if c == b'"' || c == b'\'' {
            return scan_string(&mut s);
        }

        // Comment - matches C: c == '/' && (peek_char() == '/' || peek_char() == '*')
        // peek_char is the same as c here, which is '/', so this never matches.
        // We must reproduce the bug exactly.
        if c == b'/' && (peek_char(&s) == b'/' || peek_char(&s) == b'*') {
            return scan_comment(&mut s);
        }

        // Operator
        if OPERATOR_CHARS.contains(&c) {
            return scan_operator(&mut s);
        }

        // Punctuation
        if PUNCT_CHARS.contains(&c) {
            let p = advance_char(&mut s);
            return create_token(&mut s, TokenType::Punctuation, &[p], 1);
        }

        // Unknown character
        let u = advance_char(&mut s);
        create_token(&mut s, TokenType::Error, &[u], 1)
    })
}

pub fn tokenizer_peek_token() -> Token {
    let valid = STATE.with(|st| st.borrow().lookahead_valid);
    if !valid {
        let t = tokenizer_next_token();
        STATE.with(|st| {
            let mut s = st.borrow_mut();
            s.lookahead_token = t;
            s.lookahead_valid = true;
        });
    }
    STATE.with(|st| st.borrow().lookahead_token.clone())
}

pub fn tokenizer_reset() {
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        s.current_position = 0;
        s.current_line = 1;
        s.current_column = 1;
        s.lookahead_valid = false;
    });
}

pub fn tokenizer_load_text(text: &[u8]) -> i32 {
    // text is a null-terminated C string in C; we receive the bytes preceding the null.
    let length = text.len();
    if length >= MAX_BUFFER_SIZE {
        let _ = writeln!(std::io::stderr(), "Error: Input text too large");
        return -1;
    }
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        // Mirror strncpy(input_buffer, text, MAX_BUFFER_SIZE - 1) and null terminator.
        for b in s.input_buffer.iter_mut() {
            *b = 0;
        }
        let copy_len = length.min(MAX_BUFFER_SIZE - 1);
        s.input_buffer[..copy_len].copy_from_slice(&text[..copy_len]);
        s.buffer_length = length;
    });
    tokenizer_reset();
    0
}

pub fn tokenizer_get_stats() -> (usize, usize, usize) {
    STATE.with(|st| {
        let s = st.borrow();
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

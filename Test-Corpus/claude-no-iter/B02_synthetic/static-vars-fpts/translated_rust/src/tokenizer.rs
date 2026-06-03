// Translated from c_src/src/tokenizer.c
//
// Preserves the exact behavior of the original C code, including any
// bugs (e.g., the comment-detection short-circuit that classifies every
// `/` as a comment because `peek_char()` is called twice without an
// intervening advance).

#![allow(dead_code)]

use std::cell::RefCell;

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    pub typ: TokenType,
    /// The token's bytes (without the C-style null terminator).
    pub value: Vec<u8>,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

impl Token {
    fn empty() -> Token {
        Token {
            typ: TokenType::Eof,
            value: Vec::new(),
            length: 0,
            line: 0,
            column: 0,
        }
    }
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
    lookahead_token: Token,
    lookahead_valid: bool,
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

fn is_keyword(bytes: &[u8]) -> bool {
    KEYWORDS.iter().any(|&kw| kw == bytes)
}

#[inline]
fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

#[inline]
fn c_isalpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

#[inline]
fn c_isalnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

#[inline]
fn c_isdigit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn peek_char(s: &TokenizerState) -> u8 {
    if s.current_position >= s.buffer_length {
        0
    } else {
        s.input_buffer[s.current_position]
    }
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

fn create_token(s: &mut TokenizerState, typ: TokenType, value: &[u8], length: usize) -> Token {
    let trimmed_len = if length < MAX_TOKEN_LENGTH {
        length
    } else {
        MAX_TOKEN_LENGTH - 1
    };
    let take = std::cmp::min(value.len(), trimmed_len);
    let mut bytes = Vec::with_capacity(trimmed_len);
    bytes.extend_from_slice(&value[..take]);
    // strncpy fills the remainder with zeros if value shorter than length;
    // but the resulting C-string is terminated at `trimmed_len`. We model
    // the logical content as `value[..trimmed_len]`, treating any short copy
    // as filled with zero bytes so that downstream byte-level operations
    // mirror C semantics. In practice every call site passes a `value`
    // that already has at least `length` valid bytes, so this branch
    // rarely triggers.
    if take < trimmed_len {
        bytes.resize(trimmed_len, 0);
    }
    let line = s.current_line;
    let column = s.current_column - trimmed_len as i32;
    s.total_tokens_processed += 1;
    Token {
        typ,
        value: bytes,
        length: trimmed_len,
        line,
        column,
    }
}

fn scan_word(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;
    while {
        let c = peek_char(s);
        c != 0 && (c_isalnum(c) || c == b'_') && length < MAX_TOKEN_LENGTH - 1
    } {
        let c = advance_char(s);
        buffer[length] = c;
        length += 1;
    }
    let bytes = &buffer[..length];
    if is_keyword(bytes) {
        create_token(s, TokenType::Keyword, bytes, length)
    } else {
        create_token(s, TokenType::Identifier, bytes, length)
    }
}

fn scan_number(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;
    let mut has_decimal = false;
    while {
        let c = peek_char(s);
        c != 0 && (c_isdigit(c) || c == b'.') && length < MAX_TOKEN_LENGTH - 1
    } {
        let c = peek_char(s);
        if c == b'.' {
            if has_decimal {
                break;
            }
            has_decimal = true;
        }
        let ch = advance_char(s);
        buffer[length] = ch;
        length += 1;
    }
    let bytes = &buffer[..length];
    create_token(s, TokenType::Number, bytes, length)
}

fn scan_string(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;
    let quote = advance_char(s);
    buffer[length] = quote;
    length += 1;
    while {
        let c = peek_char(s);
        c != 0 && c != quote && c != b'\n' && length < MAX_TOKEN_LENGTH - 2
    } {
        if peek_char(s) == b'\\' {
            let c = advance_char(s);
            buffer[length] = c;
            length += 1;
            if peek_char(s) != 0 {
                let c = advance_char(s);
                buffer[length] = c;
                length += 1;
            }
        } else {
            let c = advance_char(s);
            buffer[length] = c;
            length += 1;
        }
    }
    if peek_char(s) == quote {
        let c = advance_char(s);
        buffer[length] = c;
        length += 1;
    }
    let bytes = &buffer[..length];
    create_token(s, TokenType::String, bytes, length)
}

fn scan_comment(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;
    // Consume first '/'
    let c = advance_char(s);
    buffer[length] = c;
    length += 1;
    if peek_char(s) == b'/' {
        // Single-line comment
        let c = advance_char(s);
        buffer[length] = c;
        length += 1;
        while {
            let c = peek_char(s);
            c != 0 && c != b'\n' && length < MAX_TOKEN_LENGTH - 1
        } {
            let c = advance_char(s);
            buffer[length] = c;
            length += 1;
        }
    } else if peek_char(s) == b'*' {
        // Multi-line comment
        let c = advance_char(s);
        buffer[length] = c;
        length += 1;
        while peek_char(s) != 0 && length < MAX_TOKEN_LENGTH - 2 {
            if peek_char(s) == b'*' {
                let c = advance_char(s);
                buffer[length] = c;
                length += 1;
                if peek_char(s) == b'/' {
                    let c = advance_char(s);
                    buffer[length] = c;
                    length += 1;
                    break;
                }
            } else {
                let c = advance_char(s);
                buffer[length] = c;
                length += 1;
            }
        }
    }
    let bytes = &buffer[..length];
    create_token(s, TokenType::Comment, bytes, length)
}

fn scan_operator(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;
    let c = peek_char(s);
    let first = advance_char(s);
    buffer[length] = first;
    length += 1;
    let next = peek_char(s);
    let two_char = matches!(
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
    );
    if two_char {
        let c = advance_char(s);
        buffer[length] = c;
        length += 1;
    }
    let bytes = &buffer[..length];
    create_token(s, TokenType::Operator, bytes, length)
}

fn next_token_inner(s: &mut TokenizerState) -> Token {
    // Honor lookahead first (matches C exactly).
    if s.lookahead_valid {
        s.lookahead_valid = false;
        return std::mem::replace(&mut s.lookahead_token, Token::empty());
    }

    skip_whitespace(s);

    if s.current_position >= s.buffer_length {
        return create_token(s, TokenType::Eof, b"", 0);
    }

    let c = peek_char(s);

    // Newline
    if c == b'\n' {
        let nl = advance_char(s);
        let arr = [nl];
        return create_token(s, TokenType::Newline, &arr, 1);
    }

    // Identifier or keyword
    if c_isalpha(c) || c == b'_' {
        return scan_word(s);
    }

    // Number
    if c_isdigit(c) {
        return scan_number(s);
    }

    // String
    if c == b'"' || c == b'\'' {
        return scan_string(s);
    }

    // Comment
    //
    // NOTE: The C source reads `c == '/' && (peek_char() == '/' ||
    // peek_char() == '*')`. Because `peek_char()` returns the *current*
    // byte (which is still `c`), this collapses to `c == '/'`. We mirror
    // that exactly so any standalone `/` becomes a TOKEN_COMMENT, just as
    // it does in the C build.
    if c == b'/' && (peek_char(s) == b'/' || peek_char(s) == b'*') {
        return scan_comment(s);
    }

    // Operator
    if matches!(
        c,
        b'+' | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'='
            | b'<'
            | b'>'
            | b'!'
            | b'&'
            | b'|'
            | b'^'
            | b'~'
            | b'?'
            | b':'
    ) {
        return scan_operator(s);
    }

    // Punctuation
    if matches!(
        c,
        b'(' | b')' | b'{' | b'}' | b'[' | b']' | b';' | b',' | b'.'
    ) {
        let p = advance_char(s);
        let arr = [p];
        return create_token(s, TokenType::Punctuation, &arr, 1);
    }

    // Unknown character
    let u = advance_char(s);
    let arr = [u];
    create_token(s, TokenType::Error, &arr, 1)
}

pub fn tokenizer_next_token() -> Token {
    STATE.with(|st| next_token_inner(&mut st.borrow_mut()))
}

pub fn tokenizer_peek_token() -> Token {
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        if !s.lookahead_valid {
            let tok = next_token_inner(&mut s);
            s.lookahead_token = tok;
            s.lookahead_valid = true;
        }
        s.lookahead_token.clone()
    })
}

pub fn tokenizer_reset() {
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        s.current_position = 0;
        s.current_line = 1;
        s.current_column = 1;
        s.lookahead_valid = false;
        // Cumulative statistics are intentionally left untouched (matches C).
    });
}

pub fn tokenizer_load_text(text: &[u8]) -> i32 {
    // Mirror C's strlen semantics by stopping at the first embedded NUL.
    let length = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    if length >= MAX_BUFFER_SIZE {
        eprintln!("Error: Input text too large");
        return -1;
    }
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        // Reset and copy.
        for byte in s.input_buffer.iter_mut() {
            *byte = 0;
        }
        let copy_len = std::cmp::min(length, MAX_BUFFER_SIZE - 1);
        s.input_buffer[..copy_len].copy_from_slice(&text[..copy_len]);
        s.buffer_length = length;
        s.current_position = 0;
        s.current_line = 1;
        s.current_column = 1;
        s.lookahead_valid = false;
    });
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

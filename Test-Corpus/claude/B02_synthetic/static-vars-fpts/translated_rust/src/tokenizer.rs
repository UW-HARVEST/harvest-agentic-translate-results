// Rust translation of c_src/src/tokenizer.c

use std::cell::RefCell;

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
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
    pub ttype: TokenType,
    /// Holds `length` bytes followed by a `0` terminator byte (mirrors C `char value[MAX_TOKEN_LENGTH]`).
    pub value: Vec<u8>,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

#[derive(Clone, Copy)]
pub struct TokenizerOps {
    pub next_token: fn() -> Token,
    pub peek_token: fn() -> Token,
    pub reset: fn(),
    pub load_text: fn(&[u8]) -> i32,
    pub get_stats: fn() -> (usize, usize, usize),
}

struct State {
    input_buffer: Vec<u8>,
    buffer_length: usize,
    current_position: usize,
    current_line: i32,
    current_column: i32,
    total_tokens_processed: usize,
    total_lines_processed: usize,
    total_chars_processed: usize,
    lookahead_token: Option<Token>,
    lookahead_valid: bool,
}

impl State {
    fn new() -> Self {
        State {
            input_buffer: vec![0u8; MAX_BUFFER_SIZE],
            buffer_length: 0,
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens_processed: 0,
            total_lines_processed: 0,
            total_chars_processed: 0,
            lookahead_token: None,
            lookahead_valid: false,
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new());
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

// C isspace() returns nonzero for ' ', '\t', '\n', '\v', '\f', '\r'.
// skip_whitespace excludes '\n', so this set is the rest.
fn is_skippable_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | 0x0B | 0x0C | b'\r')
}

fn is_alpha(c: u8) -> bool {
    (b'a'..=b'z').contains(&c) || (b'A'..=b'Z').contains(&c)
}

fn is_digit(c: u8) -> bool {
    (b'0'..=b'9').contains(&c)
}

fn is_alnum(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}

fn peek_char(s: &State) -> u8 {
    if s.current_position >= s.buffer_length {
        0
    } else {
        s.input_buffer[s.current_position]
    }
}

fn advance_char(s: &mut State) -> u8 {
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

fn skip_whitespace(s: &mut State) {
    loop {
        let c = peek_char(s);
        if c != 0 && is_skippable_ws(c) && c != b'\n' {
            advance_char(s);
        } else {
            break;
        }
    }
}

fn create_token(s: &mut State, ttype: TokenType, value: &[u8]) -> Token {
    let length = if value.len() < MAX_TOKEN_LENGTH {
        value.len()
    } else {
        MAX_TOKEN_LENGTH - 1
    };
    let mut buf = vec![0u8; length + 1];
    buf[..length].copy_from_slice(&value[..length]);
    // Mirror C: token.column = current_column - token.length (with C's signed/unsigned mixing).
    let column = (s.current_column).wrapping_sub(length as i32);
    s.total_tokens_processed += 1;
    Token {
        ttype,
        value: buf,
        length,
        line: s.current_line,
        column,
    }
}

fn scan_word(s: &mut State) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length: usize = 0;
    while peek_char(s) != 0
        && (is_alnum(peek_char(s)) || peek_char(s) == b'_')
        && length < MAX_TOKEN_LENGTH - 1
    {
        buffer[length] = advance_char(s);
        length += 1;
    }
    let val = &buffer[..length];
    if is_keyword(val) {
        create_token(s, TokenType::Keyword, val)
    } else {
        create_token(s, TokenType::Identifier, val)
    }
}

fn scan_number(s: &mut State) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length: usize = 0;
    let mut has_decimal = false;
    while peek_char(s) != 0
        && (is_digit(peek_char(s)) || peek_char(s) == b'.')
        && length < MAX_TOKEN_LENGTH - 1
    {
        if peek_char(s) == b'.' {
            if has_decimal {
                break;
            }
            has_decimal = true;
        }
        buffer[length] = advance_char(s);
        length += 1;
    }
    create_token(s, TokenType::Number, &buffer[..length])
}

fn scan_string(s: &mut State) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length: usize = 0;
    let quote = advance_char(s);
    buffer[length] = quote;
    length += 1;
    while peek_char(s) != 0
        && peek_char(s) != quote
        && peek_char(s) != b'\n'
        && length < MAX_TOKEN_LENGTH - 2
    {
        if peek_char(s) == b'\\' {
            buffer[length] = advance_char(s);
            length += 1;
            if peek_char(s) != 0 {
                buffer[length] = advance_char(s);
                length += 1;
            }
        } else {
            buffer[length] = advance_char(s);
            length += 1;
        }
    }
    if peek_char(s) == quote {
        buffer[length] = advance_char(s);
        length += 1;
    }
    create_token(s, TokenType::String, &buffer[..length])
}

fn scan_comment(s: &mut State) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length: usize = 0;
    // First '/' (already determined to be '/')
    buffer[length] = advance_char(s);
    length += 1;
    if peek_char(s) == b'/' {
        // Single-line comment
        buffer[length] = advance_char(s);
        length += 1;
        while peek_char(s) != 0
            && peek_char(s) != b'\n'
            && length < MAX_TOKEN_LENGTH - 1
        {
            buffer[length] = advance_char(s);
            length += 1;
        }
    } else if peek_char(s) == b'*' {
        // Multi-line comment
        buffer[length] = advance_char(s);
        length += 1;
        while peek_char(s) != 0 && length < MAX_TOKEN_LENGTH - 2 {
            if peek_char(s) == b'*' {
                buffer[length] = advance_char(s);
                length += 1;
                if peek_char(s) == b'/' {
                    buffer[length] = advance_char(s);
                    length += 1;
                    break;
                }
            } else {
                buffer[length] = advance_char(s);
                length += 1;
            }
        }
    }
    create_token(s, TokenType::Comment, &buffer[..length])
}

fn scan_operator(s: &mut State) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length: usize = 0;
    let c = peek_char(s);
    buffer[length] = advance_char(s);
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
        buffer[length] = advance_char(s);
        length += 1;
    }
    create_token(s, TokenType::Operator, &buffer[..length])
}

fn next_token_impl(s: &mut State) -> Token {
    if s.lookahead_valid {
        s.lookahead_valid = false;
        return s.lookahead_token.take().unwrap();
    }

    skip_whitespace(s);

    if s.current_position >= s.buffer_length {
        return create_token(s, TokenType::Eof, b"");
    }

    let c = peek_char(s);

    // Newline
    if c == b'\n' {
        let nl = [advance_char(s)];
        return create_token(s, TokenType::Newline, &nl);
    }

    // Identifier or keyword
    if is_alpha(c) || c == b'_' {
        return scan_word(s);
    }

    // Number
    if is_digit(c) {
        return scan_number(s);
    }

    // String
    if c == b'"' || c == b'\'' {
        return scan_string(s);
    }

    // Comment (preserving the C behavior: peek_char() == c after only peeking,
    // so any '/' triggers scan_comment).
    if c == b'/' && (peek_char(s) == b'/' || peek_char(s) == b'*') {
        return scan_comment(s);
    }

    // Operator
    if b"+-*/%=<>!&|^~?:".contains(&c) {
        return scan_operator(s);
    }

    // Punctuation
    if b"(){}[];,.".contains(&c) {
        let p = [advance_char(s)];
        return create_token(s, TokenType::Punctuation, &p);
    }

    // Unknown character
    let u = [advance_char(s)];
    create_token(s, TokenType::Error, &u)
}

pub fn tokenizer_next_token() -> Token {
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        next_token_impl(&mut s)
    })
}

pub fn tokenizer_peek_token() -> Token {
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        if !s.lookahead_valid {
            let t = next_token_impl(&mut s);
            s.lookahead_token = Some(t);
            s.lookahead_valid = true;
        }
        s.lookahead_token.clone().unwrap()
    })
}

pub fn tokenizer_reset() {
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        s.current_position = 0;
        s.current_line = 1;
        s.current_column = 1;
        s.lookahead_valid = false;
        s.lookahead_token = None;
        // total statistics intentionally not reset
    });
}

pub fn tokenizer_load_text(text: &[u8]) -> i32 {
    let length = text.len();
    if length >= MAX_BUFFER_SIZE {
        eprintln!("Error: Input text too large");
        return -1;
    }
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        // strncpy semantics; we only care about the first `length` bytes anyway.
        for i in 0..length {
            s.input_buffer[i] = text[i];
        }
        s.input_buffer[MAX_BUFFER_SIZE - 1] = 0;
        s.buffer_length = length;
        // Inline reset (avoid borrowing twice).
        s.current_position = 0;
        s.current_line = 1;
        s.current_column = 1;
        s.lookahead_valid = false;
        s.lookahead_token = None;
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

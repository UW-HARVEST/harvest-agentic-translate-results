// Tokenizer module - port of tokenizer.c

use std::cell::RefCell;
use std::io::Write;

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

impl TokenType {
    pub fn from_index(i: usize) -> TokenType {
        match i {
            0 => TokenType::Eof,
            1 => TokenType::Word,
            2 => TokenType::Number,
            3 => TokenType::Punctuation,
            4 => TokenType::Whitespace,
            5 => TokenType::Newline,
            6 => TokenType::Identifier,
            7 => TokenType::Keyword,
            8 => TokenType::Operator,
            9 => TokenType::String,
            10 => TokenType::Comment,
            11 => TokenType::Error,
            _ => TokenType::Eof,
        }
    }
}

#[derive(Clone)]
pub struct Token {
    pub ttype: TokenType,
    // Store value as bytes (no null terminator stored; print up to length)
    pub value: Vec<u8>,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

impl Default for Token {
    fn default() -> Self {
        Token {
            ttype: TokenType::Eof,
            value: Vec::new(),
            length: 0,
            line: 0,
            column: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TokenizerOps {
    pub next_token: fn() -> Token,
    pub peek_token: fn() -> Token,
    pub reset: fn(),
    pub load_text: fn(&[u8]) -> i32,
    pub get_stats: fn(&mut usize, &mut usize, &mut usize),
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
    lookahead_valid: i32,
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
            lookahead_token: Token::default(),
            lookahead_valid: 0,
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
    for kw in KEYWORDS {
        if s == *kw {
            return true;
        }
    }
    false
}

// Mimic C's isspace: ' ', '\t', '\n', '\v', '\f', '\r'
fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn c_isalpha(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z')
}

fn c_isdigit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

fn c_isalnum(c: u8) -> bool {
    c_isalpha(c) || c_isdigit(c)
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
        if c != 0 && c_isspace(c) && c != b'\n' {
            advance_char(s);
        } else {
            break;
        }
    }
}

fn create_token(s: &mut TokenizerState, ttype: TokenType, value: &[u8], length: usize) -> Token {
    let token_length = if length < MAX_TOKEN_LENGTH {
        length
    } else {
        MAX_TOKEN_LENGTH - 1
    };
    let mut v = Vec::with_capacity(token_length);
    let copy_len = std::cmp::min(token_length, value.len());
    v.extend_from_slice(&value[..copy_len]);
    // pad with zeros if value shorter than token_length
    while v.len() < token_length {
        v.push(0);
    }
    let line = s.current_line;
    let column = (s.current_column as i32).wrapping_sub(token_length as i32);
    s.total_tokens_processed += 1;
    Token {
        ttype,
        value: v,
        length: token_length,
        line,
        column,
    }
}

fn scan_word(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;

    loop {
        let c = peek_char(s);
        if c != 0 && (c_isalnum(c) || c == b'_') && length < MAX_TOKEN_LENGTH - 1 {
            buffer[length] = advance_char(s);
            length += 1;
        } else {
            break;
        }
    }

    if is_keyword(&buffer[..length]) {
        return create_token(s, TokenType::Keyword, &buffer[..length], length);
    }
    create_token(s, TokenType::Identifier, &buffer[..length], length)
}

fn scan_number(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;
    let mut has_decimal = false;

    loop {
        let c = peek_char(s);
        if c != 0 && (c_isdigit(c) || c == b'.') && length < MAX_TOKEN_LENGTH - 1 {
            if c == b'.' {
                if has_decimal {
                    break;
                }
                has_decimal = true;
            }
            buffer[length] = advance_char(s);
            length += 1;
        } else {
            break;
        }
    }

    create_token(s, TokenType::Number, &buffer[..length], length)
}

fn scan_string(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;
    let quote = advance_char(s);

    buffer[length] = quote;
    length += 1;

    loop {
        let c = peek_char(s);
        if c != 0 && c != quote && c != b'\n' && length < MAX_TOKEN_LENGTH - 2 {
            if c == b'\\' {
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
        } else {
            break;
        }
    }

    if peek_char(s) == quote {
        buffer[length] = advance_char(s);
        length += 1;
    }

    create_token(s, TokenType::String, &buffer[..length], length)
}

fn scan_comment(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;

    // First '/'
    buffer[length] = advance_char(s);
    length += 1;

    if peek_char(s) == b'/' {
        // Single-line comment
        buffer[length] = advance_char(s);
        length += 1;

        loop {
            let c = peek_char(s);
            if c != 0 && c != b'\n' && length < MAX_TOKEN_LENGTH - 1 {
                buffer[length] = advance_char(s);
                length += 1;
            } else {
                break;
            }
        }
    } else if peek_char(s) == b'*' {
        // Multi-line comment
        buffer[length] = advance_char(s);
        length += 1;

        loop {
            let c = peek_char(s);
            if c == 0 || length >= MAX_TOKEN_LENGTH - 2 {
                break;
            }
            if c == b'*' {
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

    create_token(s, TokenType::Comment, &buffer[..length], length)
}

fn scan_operator(s: &mut TokenizerState) -> Token {
    let mut buffer = [0u8; MAX_TOKEN_LENGTH];
    let mut length = 0usize;
    let c = peek_char(s);

    buffer[length] = advance_char(s);
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
        buffer[length] = advance_char(s);
        length += 1;
    }

    create_token(s, TokenType::Operator, &buffer[..length], length)
}

pub fn tokenizer_next_token() -> Token {
    STATE.with(|st| {
        let mut s = st.borrow_mut();

        if s.lookahead_valid != 0 {
            s.lookahead_valid = 0;
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

        // Comment - matches C's `peek_char() == '/' || peek_char() == '*'` reading current pos
        if c == b'/' && (peek_char(&s) == b'/' || peek_char(&s) == b'*') {
            return scan_comment(&mut s);
        }

        // Operator
        if b"+-*/%=<>!&|^~?:".contains(&c) {
            return scan_operator(&mut s);
        }

        // Punctuation
        if b"(){}[];,.".contains(&c) {
            let p = advance_char(&mut s);
            return create_token(&mut s, TokenType::Punctuation, &[p], 1);
        }

        // Unknown
        let u = advance_char(&mut s);
        create_token(&mut s, TokenType::Error, &[u], 1)
    })
}

pub fn tokenizer_peek_token() -> Token {
    let valid = STATE.with(|st| st.borrow().lookahead_valid);
    if valid == 0 {
        let t = tokenizer_next_token();
        STATE.with(|st| {
            let mut s = st.borrow_mut();
            s.lookahead_token = t;
            s.lookahead_valid = 1;
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
        s.lookahead_valid = 0;
    });
}

pub fn tokenizer_load_text(text: &[u8]) -> i32 {
    // Mimic the C check: `if (!text)` - In Rust, we cannot pass null,
    // but for byte-identical behavior with valid input, we proceed.
    let length = c_strlen(text);
    if length >= MAX_BUFFER_SIZE {
        let _ = writeln!(std::io::stderr(), "Error: Input text too large");
        return -1;
    }

    STATE.with(|st| {
        let mut s = st.borrow_mut();
        // strncpy semantics: copy up to MAX_BUFFER_SIZE - 1 bytes, null pad.
        // We re-init the buffer to 0.
        for b in s.input_buffer.iter_mut() {
            *b = 0;
        }
        let copy_len = std::cmp::min(length, MAX_BUFFER_SIZE - 1);
        for i in 0..copy_len {
            s.input_buffer[i] = text[i];
        }
        s.input_buffer[MAX_BUFFER_SIZE - 1] = 0;
        s.buffer_length = length;
    });

    tokenizer_reset();
    0
}

pub fn tokenizer_get_stats(lines: &mut usize, tokens: &mut usize, chars: &mut usize) {
    STATE.with(|st| {
        let s = st.borrow();
        *lines = s.total_lines_processed;
        *tokens = s.total_tokens_processed;
        *chars = s.total_chars_processed;
    });
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

// Helper: return strlen-equivalent. Treats slice as null-terminated string.
fn c_strlen(s: &[u8]) -> usize {
    for (i, &b) in s.iter().enumerate() {
        if b == 0 {
            return i;
        }
    }
    s.len()
}

use std::cell::RefCell;

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub value: std::string::String,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

pub struct TokenizerOps {
    pub next_token: fn() -> Token,
    pub peek_token: fn() -> Token,
    pub reset: fn(),
    pub load_text: fn(&str) -> i32,
    pub get_stats: fn(&mut usize, &mut usize, &mut usize),
}

struct TokenizerState {
    input_buffer: [u8; MAX_BUFFER_SIZE],
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
    const fn new() -> Self {
        Self {
            input_buffer: [0u8; MAX_BUFFER_SIZE],
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
}

thread_local! {
    static STATE: RefCell<TokenizerState> = RefCell::new(TokenizerState::new());
}

static KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "return", "int", "char",
    "float", "double", "void", "struct", "typedef", "const",
    "static", "extern", "auto", "register", "sizeof", "break",
    "continue", "switch", "case", "default", "do", "goto",
    "enum", "union", "signed", "unsigned", "long", "short",
];

fn is_keyword(s: &str) -> bool {
    KEYWORDS.iter().any(|&k| k == s)
}

fn with_state<R>(f: impl FnOnce(&mut TokenizerState) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn peek_char_s(s: &TokenizerState) -> u8 {
    if s.current_position >= s.buffer_length {
        0
    } else {
        s.input_buffer[s.current_position]
    }
}

fn advance_char_s(s: &mut TokenizerState) -> u8 {
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

fn skip_whitespace_s(s: &mut TokenizerState) {
    while peek_char_s(s) != 0
        && (peek_char_s(s) as char).is_ascii_whitespace()
        && peek_char_s(s) != b'\n'
    {
        advance_char_s(s);
    }
}

fn create_token_s(s: &mut TokenizerState, tt: TokenType, value: &str, length: usize) -> Token {
    let len = if length < MAX_TOKEN_LENGTH { length } else { MAX_TOKEN_LENGTH - 1 };
    let truncated: String = value.chars().take(len).collect();
    s.total_tokens_processed += 1;
    Token {
        token_type: tt,
        value: truncated,
        length: len,
        line: s.current_line,
        column: s.current_column - len as i32,
    }
}

fn scan_word_s(s: &mut TokenizerState) -> Token {
    let mut buffer = std::string::String::new();
    while peek_char_s(s) != 0
        && ((peek_char_s(s) as char).is_ascii_alphanumeric() || peek_char_s(s) == b'_')
        && buffer.len() < MAX_TOKEN_LENGTH - 1
    {
        buffer.push(advance_char_s(s) as char);
    }
    let len = buffer.len();
    if is_keyword(&buffer) {
        create_token_s(s, TokenType::Keyword, &buffer, len)
    } else {
        create_token_s(s, TokenType::Identifier, &buffer, len)
    }
}

fn scan_number_s(s: &mut TokenizerState) -> Token {
    let mut buffer = std::string::String::new();
    let mut has_decimal = false;
    while peek_char_s(s) != 0
        && ((peek_char_s(s) as char).is_ascii_digit() || peek_char_s(s) == b'.')
        && buffer.len() < MAX_TOKEN_LENGTH - 1
    {
        if peek_char_s(s) == b'.' {
            if has_decimal {
                break;
            }
            has_decimal = true;
        }
        buffer.push(advance_char_s(s) as char);
    }
    let len = buffer.len();
    create_token_s(s, TokenType::Number, &buffer, len)
}

fn scan_string_s(s: &mut TokenizerState) -> Token {
    let mut buffer = std::string::String::new();
    let quote = advance_char_s(s);
    buffer.push(quote as char);
    while peek_char_s(s) != 0
        && peek_char_s(s) != quote
        && peek_char_s(s) != b'\n'
        && buffer.len() < MAX_TOKEN_LENGTH - 2
    {
        if peek_char_s(s) == b'\\' {
            buffer.push(advance_char_s(s) as char);
            if peek_char_s(s) != 0 {
                buffer.push(advance_char_s(s) as char);
            }
        } else {
            buffer.push(advance_char_s(s) as char);
        }
    }
    if peek_char_s(s) == quote {
        buffer.push(advance_char_s(s) as char);
    }
    let len = buffer.len();
    create_token_s(s, TokenType::String, &buffer, len)
}

fn scan_comment_s(s: &mut TokenizerState) -> Token {
    let mut buffer = std::string::String::new();
    buffer.push(advance_char_s(s) as char); // First '/'
    if peek_char_s(s) == b'/' {
        buffer.push(advance_char_s(s) as char); // Second '/'
        while peek_char_s(s) != 0
            && peek_char_s(s) != b'\n'
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            buffer.push(advance_char_s(s) as char);
        }
    } else if peek_char_s(s) == b'*' {
        buffer.push(advance_char_s(s) as char); // '*'
        while peek_char_s(s) != 0 && buffer.len() < MAX_TOKEN_LENGTH - 2 {
            if peek_char_s(s) == b'*' {
                buffer.push(advance_char_s(s) as char);
                if peek_char_s(s) == b'/' {
                    buffer.push(advance_char_s(s) as char);
                    break;
                }
            } else {
                buffer.push(advance_char_s(s) as char);
            }
        }
    }
    let len = buffer.len();
    create_token_s(s, TokenType::Comment, &buffer, len)
}

fn scan_operator_s(s: &mut TokenizerState) -> Token {
    let mut buffer = std::string::String::new();
    let c = peek_char_s(s);
    buffer.push(advance_char_s(s) as char);
    let next = peek_char_s(s);
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
        buffer.push(advance_char_s(s) as char);
    }
    let len = buffer.len();
    create_token_s(s, TokenType::Operator, &buffer, len)
}

fn next_token_impl(s: &mut TokenizerState) -> Token {
    if let Some(tok) = s.lookahead_token.take() {
        return tok;
    }
    skip_whitespace_s(s);
    if s.current_position >= s.buffer_length {
        return create_token_s(s, TokenType::Eof, "", 0);
    }
    let c = peek_char_s(s);
    if c == b'\n' {
        let ch = advance_char_s(s);
        let val = std::string::String::from(ch as char);
        return create_token_s(s, TokenType::Newline, &val, 1);
    }
    if (c as char).is_ascii_alphabetic() || c == b'_' {
        return scan_word_s(s);
    }
    if (c as char).is_ascii_digit() {
        return scan_number_s(s);
    }
    if c == b'"' || c == b'\'' {
        return scan_string_s(s);
    }
    // Bug-compatible: C checks c == '/' && (peek_char() == '/' || peek_char() == '*')
    // but peek_char() still returns c (not advanced), so this is just c == '/'
    if c == b'/' && (peek_char_s(s) == b'/' || peek_char_s(s) == b'*') {
        return scan_comment_s(s);
    }
    if b"+-*/%=<>!&|^~?:".contains(&c) {
        return scan_operator_s(s);
    }
    if b"(){}[];,.".contains(&c) {
        let ch = advance_char_s(s);
        let val = std::string::String::from(ch as char);
        return create_token_s(s, TokenType::Punctuation, &val, 1);
    }
    let ch = advance_char_s(s);
    let val = std::string::String::from(ch as char);
    create_token_s(s, TokenType::Error, &val, 1)
}

pub fn tokenizer_next_token() -> Token {
    with_state(next_token_impl)
}

pub fn tokenizer_peek_token() -> Token {
    with_state(|s| {
        if s.lookahead_token.is_none() {
            let tok = next_token_impl(s);
            s.lookahead_token = Some(tok);
        }
        s.lookahead_token.clone().unwrap()
    })
}

pub fn tokenizer_reset() {
    with_state(|s| {
        s.current_position = 0;
        s.current_line = 1;
        s.current_column = 1;
        s.lookahead_token = None;
    });
}

pub fn tokenizer_load_text(text: &str) -> i32 {
    with_state(|s| {
        let bytes = text.as_bytes();
        if bytes.len() >= MAX_BUFFER_SIZE {
            eprint!("Error: Input text too large\n");
            return -1;
        }
        s.input_buffer[..bytes.len()].copy_from_slice(bytes);
        s.input_buffer[bytes.len()] = 0;
        s.buffer_length = bytes.len();
        // inline reset (don't reset total stats)
        s.current_position = 0;
        s.current_line = 1;
        s.current_column = 1;
        s.lookahead_token = None;
        0
    })
}

pub fn tokenizer_get_stats(lines: &mut usize, tokens: &mut usize, chars: &mut usize) {
    with_state(|s| {
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
        load_text: |text| tokenizer_load_text(text),
        get_stats: |l, t, c| tokenizer_get_stats(l, t, c),
    }
}

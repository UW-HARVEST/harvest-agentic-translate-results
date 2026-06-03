use std::sync::Mutex;

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
    pub type_: TokenType,
    pub value: String,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

impl Token {
    pub fn empty() -> Self {
        Token {
            type_: TokenType::Eof,
            value: String::new(),
            length: 0,
            line: 0,
            column: 0,
        }
    }
}

// Function pointer types matching the C tokenizer_ops_t
pub type TokenizerNextFn = fn() -> Token;
pub type TokenizerPeekFn = fn() -> Token;
pub type TokenizerResetFn = fn();
pub type TokenizerLoadFn = fn(&str) -> i32;
pub type TokenizerGetStatsFn = fn() -> (usize, usize, usize);

#[derive(Copy, Clone)]
pub struct TokenizerOps {
    pub next_token: TokenizerNextFn,
    pub peek_token: TokenizerPeekFn,
    pub reset: TokenizerResetFn,
    pub load_text: TokenizerLoadFn,
    pub get_stats: TokenizerGetStatsFn,
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
    const fn new() -> Self {
        TokenizerState {
            input_buffer: Vec::new(),
            buffer_length: 0,
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens_processed: 0,
            total_lines_processed: 0,
            total_chars_processed: 0,
            lookahead_token: Token {
                type_: TokenType::Eof,
                value: String::new(),
                length: 0,
                line: 0,
                column: 0,
            },
            lookahead_valid: false,
        }
    }
}

static STATE: Mutex<TokenizerState> = Mutex::new(TokenizerState::new());

const KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "return", "int", "char", "float", "double",
    "void", "struct", "typedef", "const", "static", "extern", "auto",
    "register", "sizeof", "break", "continue", "switch", "case", "default",
    "do", "goto", "enum", "union", "signed", "unsigned", "long", "short",
];

fn is_keyword(s: &str) -> bool {
    KEYWORDS.iter().any(|kw| *kw == s)
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn is_alpha(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z')
}

fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

fn is_alnum(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}

fn peek_char(state: &TokenizerState) -> u8 {
    if state.current_position >= state.buffer_length {
        0
    } else {
        state.input_buffer[state.current_position]
    }
}

fn advance_char(state: &mut TokenizerState) -> u8 {
    if state.current_position >= state.buffer_length {
        return 0;
    }
    let c = state.input_buffer[state.current_position];
    state.current_position += 1;
    state.total_chars_processed += 1;

    if c == b'\n' {
        state.current_line += 1;
        state.current_column = 1;
        state.total_lines_processed += 1;
    } else {
        state.current_column += 1;
    }
    c
}

fn skip_whitespace(state: &mut TokenizerState) {
    loop {
        let c = peek_char(state);
        if c == 0 || !is_space(c) || c == b'\n' {
            break;
        }
        advance_char(state);
    }
}

fn create_token(state: &mut TokenizerState, type_: TokenType, value: &[u8]) -> Token {
    let length = value.len();
    let length = if length < MAX_TOKEN_LENGTH {
        length
    } else {
        MAX_TOKEN_LENGTH - 1
    };
    let s = String::from_utf8_lossy(&value[..length]).into_owned();
    let token = Token {
        type_,
        value: s,
        length,
        line: state.current_line,
        column: state.current_column - length as i32,
    };
    state.total_tokens_processed += 1;
    token
}

fn scan_word(state: &mut TokenizerState) -> Token {
    let mut buffer: Vec<u8> = Vec::new();
    while peek_char(state) != 0
        && (is_alnum(peek_char(state)) || peek_char(state) == b'_')
        && buffer.len() < MAX_TOKEN_LENGTH - 1
    {
        buffer.push(advance_char(state));
    }
    let word = String::from_utf8_lossy(&buffer).into_owned();
    if is_keyword(&word) {
        create_token(state, TokenType::Keyword, &buffer)
    } else {
        create_token(state, TokenType::Identifier, &buffer)
    }
}

fn scan_number(state: &mut TokenizerState) -> Token {
    let mut buffer: Vec<u8> = Vec::new();
    let mut has_decimal = false;
    while peek_char(state) != 0
        && (is_digit(peek_char(state)) || peek_char(state) == b'.')
        && buffer.len() < MAX_TOKEN_LENGTH - 1
    {
        if peek_char(state) == b'.' {
            if has_decimal {
                break;
            }
            has_decimal = true;
        }
        buffer.push(advance_char(state));
    }
    create_token(state, TokenType::Number, &buffer)
}

fn scan_string(state: &mut TokenizerState) -> Token {
    let mut buffer: Vec<u8> = Vec::new();
    let quote = advance_char(state); // consume opening quote
    buffer.push(quote);

    while peek_char(state) != 0
        && peek_char(state) != quote
        && peek_char(state) != b'\n'
        && buffer.len() < MAX_TOKEN_LENGTH - 2
    {
        if peek_char(state) == b'\\' {
            buffer.push(advance_char(state));
            if peek_char(state) != 0 {
                buffer.push(advance_char(state));
            }
        } else {
            buffer.push(advance_char(state));
        }
    }

    if peek_char(state) == quote {
        buffer.push(advance_char(state));
    }

    create_token(state, TokenType::String, &buffer)
}

fn scan_comment(state: &mut TokenizerState) -> Token {
    let mut buffer: Vec<u8> = Vec::new();
    buffer.push(advance_char(state)); // First '/'

    if peek_char(state) == b'/' {
        buffer.push(advance_char(state));
        while peek_char(state) != 0
            && peek_char(state) != b'\n'
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            buffer.push(advance_char(state));
        }
    } else if peek_char(state) == b'*' {
        buffer.push(advance_char(state));
        while peek_char(state) != 0 && buffer.len() < MAX_TOKEN_LENGTH - 2 {
            if peek_char(state) == b'*' {
                buffer.push(advance_char(state));
                if peek_char(state) == b'/' {
                    buffer.push(advance_char(state));
                    break;
                }
            } else {
                buffer.push(advance_char(state));
            }
        }
    }

    create_token(state, TokenType::Comment, &buffer)
}

fn scan_operator(state: &mut TokenizerState) -> Token {
    let mut buffer: Vec<u8> = Vec::new();
    let c = peek_char(state);
    buffer.push(advance_char(state));

    let next = peek_char(state);
    let is_two_char = (c == b'=' && next == b'=')
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
    if is_two_char {
        buffer.push(advance_char(state));
    }

    create_token(state, TokenType::Operator, &buffer)
}

pub fn tokenizer_next_token() -> Token {
    let mut state = STATE.lock().unwrap();

    if state.lookahead_valid {
        state.lookahead_valid = false;
        return state.lookahead_token.clone();
    }

    skip_whitespace(&mut state);

    if state.current_position >= state.buffer_length {
        return create_token(&mut state, TokenType::Eof, b"");
    }

    let c = peek_char(&state);

    if c == b'\n' {
        let nl = advance_char(&mut state);
        return create_token(&mut state, TokenType::Newline, &[nl]);
    }

    if is_alpha(c) || c == b'_' {
        return scan_word(&mut state);
    }

    if is_digit(c) {
        return scan_number(&mut state);
    }

    if c == b'"' || c == b'\'' {
        return scan_string(&mut state);
    }

    // Comment - mirrors original C logic which tests peek_char twice
    if c == b'/' && (peek_char(&state) == b'/' || peek_char(&state) == b'*') {
        return scan_comment(&mut state);
    }

    if b"+-*/%=<>!&|^~?:".contains(&c) {
        return scan_operator(&mut state);
    }

    if b"(){}[];,.".contains(&c) {
        let p = advance_char(&mut state);
        return create_token(&mut state, TokenType::Punctuation, &[p]);
    }

    let p = advance_char(&mut state);
    create_token(&mut state, TokenType::Error, &[p])
}

pub fn tokenizer_peek_token() -> Token {
    let valid = {
        let state = STATE.lock().unwrap();
        state.lookahead_valid
    };
    if !valid {
        let tok = tokenizer_next_token();
        let mut state = STATE.lock().unwrap();
        state.lookahead_token = tok.clone();
        state.lookahead_valid = true;
        return tok;
    }
    let state = STATE.lock().unwrap();
    state.lookahead_token.clone()
}

pub fn tokenizer_reset() {
    let mut state = STATE.lock().unwrap();
    state.current_position = 0;
    state.current_line = 1;
    state.current_column = 1;
    state.lookahead_valid = false;
}

pub fn tokenizer_load_text(text: &str) -> i32 {
    let bytes = text.as_bytes();
    let length = bytes.len();
    if length >= MAX_BUFFER_SIZE {
        eprintln!("Error: Input text too large");
        return -1;
    }
    {
        let mut state = STATE.lock().unwrap();
        state.input_buffer = bytes.to_vec();
        state.buffer_length = length;
    }
    tokenizer_reset();
    0
}

pub fn tokenizer_get_stats() -> (usize, usize, usize) {
    let state = STATE.lock().unwrap();
    (
        state.total_lines_processed,
        state.total_tokens_processed,
        state.total_chars_processed,
    )
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

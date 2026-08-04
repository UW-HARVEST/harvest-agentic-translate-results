use std::sync::{Mutex, OnceLock};

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    TokenEof = 0,
    TokenWord,
    TokenNumber,
    TokenPunctuation,
    TokenWhitespace,
    TokenNewline,
    TokenIdentifier,
    TokenKeyword,
    TokenOperator,
    TokenString,
    TokenComment,
    TokenError,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub type_: TokenType,
    pub value: String,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

pub type TokenizerNextFn = fn() -> Token;
pub type TokenizerPeekFn = fn() -> Token;
pub type TokenizerResetFn = fn();
pub type TokenizerLoadFn = fn(&str) -> i32;
pub type TokenizerGetStatsFn = fn(&mut usize, &mut usize, &mut usize);

#[derive(Clone, Copy)]
pub struct TokenizerOps {
    pub next_token: TokenizerNextFn,
    pub peek_token: TokenizerPeekFn,
    pub reset: TokenizerResetFn,
    pub load_text: TokenizerLoadFn,
    pub get_stats: TokenizerGetStatsFn,
}

struct TokenizerState {
    input_buffer: String,
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
        Self {
            input_buffer: String::new(),
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

    fn keywords() -> &'static [&'static str] {
        &[
            "if", "else", "while", "for", "return", "int", "char", "float", "double",
            "void", "struct", "typedef", "const", "static", "extern", "auto", "register",
            "sizeof", "break", "continue", "switch", "case", "default", "do", "goto",
            "enum", "union", "signed", "unsigned", "long", "short",
        ]
    }

    fn is_keyword(&self, s: &str) -> bool {
        Self::keywords().contains(&s)
    }

    fn peek_char(&self) -> Option<char> {
        self.input_buffer[self.current_position..].chars().next()
    }

    fn peek_nth_char(&self, n: usize) -> Option<char> {
        self.input_buffer[self.current_position..].chars().nth(n)
    }

    fn advance_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.current_position += c.len_utf8();
        self.total_chars_processed += 1;
        if c == '\n' {
            self.current_line += 1;
            self.current_column = 1;
            self.total_lines_processed += 1;
        } else {
            self.current_column += 1;
        }
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() && c != '\n' {
                let _ = self.advance_char();
            } else {
                break;
            }
        }
    }

    fn create_token(&mut self, type_: TokenType, value: String, length: usize) -> Token {
        let token_length = length.min(MAX_TOKEN_LENGTH - 1);
        let truncated: String = value.chars().take(token_length).collect();
        let token = Token {
            type_,
            value: truncated,
            length: token_length,
            line: self.current_line,
            column: self.current_column - token_length as i32,
        };
        self.total_tokens_processed += 1;
        token
    }

    fn scan_word(&mut self) -> Token {
        let mut buffer = String::new();
        while let Some(c) = self.peek_char() {
            if (c.is_alphanumeric() || c == '_') && buffer.chars().count() < MAX_TOKEN_LENGTH - 1 {
                buffer.push(self.advance_char().unwrap_or(c));
            } else {
                break;
            }
        }
        if self.is_keyword(&buffer) {
            self.create_token(TokenType::TokenKeyword, buffer.clone(), buffer.chars().count())
        } else {
            self.create_token(TokenType::TokenIdentifier, buffer.clone(), buffer.chars().count())
        }
    }

    fn scan_number(&mut self) -> Token {
        let mut buffer = String::new();
        let mut has_decimal = false;
        while let Some(c) = self.peek_char() {
            if (c.is_ascii_digit() || c == '.') && buffer.chars().count() < MAX_TOKEN_LENGTH - 1 {
                if c == '.' {
                    if has_decimal {
                        break;
                    }
                    has_decimal = true;
                }
                buffer.push(self.advance_char().unwrap_or(c));
            } else {
                break;
            }
        }
        self.create_token(TokenType::TokenNumber, buffer.clone(), buffer.chars().count())
    }

    fn scan_string(&mut self) -> Token {
        let mut buffer = String::new();
        let quote = self.advance_char().unwrap_or('"');
        buffer.push(quote);
        while let Some(c) = self.peek_char() {
            if c == quote || c == '\n' || buffer.chars().count() >= MAX_TOKEN_LENGTH - 2 {
                break;
            }
            if c == '\\' {
                if let Some(ch) = self.advance_char() {
                    buffer.push(ch);
                }
                if let Some(ch) = self.peek_char() {
                    let _ = self.advance_char();
                    buffer.push(ch);
                }
            } else if let Some(ch) = self.advance_char() {
                buffer.push(ch);
            }
        }
        if self.peek_char() == Some(quote) {
            if let Some(ch) = self.advance_char() {
                buffer.push(ch);
            }
        }
        self.create_token(TokenType::TokenString, buffer.clone(), buffer.chars().count())
    }

    fn scan_comment(&mut self) -> Token {
        let mut buffer = String::new();
        if let Some(ch) = self.advance_char() {
            buffer.push(ch);
        }
        if self.peek_char() == Some('/') {
            if let Some(ch) = self.advance_char() {
                buffer.push(ch);
            }
            while let Some(c) = self.peek_char() {
                if c == '\n' || buffer.chars().count() >= MAX_TOKEN_LENGTH - 1 {
                    break;
                }
                if let Some(ch) = self.advance_char() {
                    buffer.push(ch);
                }
            }
        } else if self.peek_char() == Some('*') {
            if let Some(ch) = self.advance_char() {
                buffer.push(ch);
            }
            while self.peek_char().is_some() && buffer.chars().count() < MAX_TOKEN_LENGTH - 2 {
                if self.peek_char() == Some('*') {
                    if let Some(ch) = self.advance_char() {
                        buffer.push(ch);
                    }
                    if self.peek_char() == Some('/') {
                        if let Some(ch) = self.advance_char() {
                            buffer.push(ch);
                        }
                        break;
                    }
                } else if let Some(ch) = self.advance_char() {
                    buffer.push(ch);
                }
            }
        }
        self.create_token(TokenType::TokenComment, buffer.clone(), buffer.chars().count())
    }

    fn scan_operator(&mut self) -> Token {
        let mut buffer = String::new();
        let c = self.peek_char().unwrap_or('\0');
        if let Some(ch) = self.advance_char() {
            buffer.push(ch);
        }
        let next = self.peek_char().unwrap_or('\0');
        if (c == '=' && next == '=')
            || (c == '!' && next == '=')
            || (c == '<' && next == '=')
            || (c == '>' && next == '=')
            || (c == '&' && next == '&')
            || (c == '|' && next == '|')
            || (c == '+' && next == '+')
            || (c == '-' && next == '-')
            || (c == '-' && next == '>')
            || (c == '<' && next == '<')
            || (c == '>' && next == '>')
        {
            if let Some(ch) = self.advance_char() {
                buffer.push(ch);
            }
        }
        self.create_token(TokenType::TokenOperator, buffer.clone(), buffer.chars().count())
    }

    fn next_token_internal(&mut self) -> Token {
        if let Some(token) = self.lookahead_token.take() {
            return token;
        }
        self.skip_whitespace();
        if self.current_position >= self.buffer_length {
            return self.create_token(TokenType::TokenEof, String::new(), 0);
        }
        let c = self.peek_char().unwrap_or('\0');
        if c == '\n' {
            let ch = self.advance_char().unwrap_or('\n');
            return self.create_token(TokenType::TokenNewline, ch.to_string(), 1);
        }
        if c.is_alphabetic() || c == '_' {
            return self.scan_word();
        }
        if c.is_ascii_digit() {
            return self.scan_number();
        }
        if c == '"' || c == '\'' {
            return self.scan_string();
        }
        if c == '/' && matches!(self.peek_nth_char(1), Some('/') | Some('*')) {
            return self.scan_comment();
        }
        if "+-*/%=<>!&|^~?:".contains(c) {
            return self.scan_operator();
        }
        if "(){}[];,.".contains(c) {
            let ch = self.advance_char().unwrap_or(c);
            return self.create_token(TokenType::TokenPunctuation, ch.to_string(), 1);
        }
        let ch = self.advance_char().unwrap_or(c);
        self.create_token(TokenType::TokenError, ch.to_string(), 1)
    }
}

fn state() -> &'static Mutex<TokenizerState> {
    static STATE: OnceLock<Mutex<TokenizerState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TokenizerState::new()))
}

pub fn tokenizer_next_token() -> Token {
    state().lock().unwrap().next_token_internal()
}

pub fn tokenizer_peek_token() -> Token {
    let mut s = state().lock().unwrap();
    if s.lookahead_token.is_none() {
        let token = s.next_token_internal();
        s.lookahead_token = Some(token.clone());
    }
    s.lookahead_token.clone().unwrap()
}

pub fn tokenizer_reset() {
    let mut s = state().lock().unwrap();
    s.current_position = 0;
    s.current_line = 1;
    s.current_column = 1;
    s.lookahead_token = None;
}

pub fn tokenizer_load_text(text: &str) -> i32 {
    if text.len() >= MAX_BUFFER_SIZE {
        eprintln!("Error: Input text too large");
        return -1;
    }
    let mut s = state().lock().unwrap();
    s.input_buffer = text.to_string();
    s.buffer_length = s.input_buffer.len();
    s.current_position = 0;
    s.current_line = 1;
    s.current_column = 1;
    s.lookahead_token = None;
    0
}

pub fn tokenizer_get_stats(lines: &mut usize, tokens: &mut usize, chars: &mut usize) {
    let s = state().lock().unwrap();
    *lines = s.total_lines_processed;
    *tokens = s.total_tokens_processed;
    *chars = s.total_chars_processed;
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

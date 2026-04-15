use std::cell::RefCell;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TokenType {
    Eof = 0,
    Word,
    Number,
    Punctuation,
    Whitespace,
    Newline,
    Identifier,
    Keyword,
    Operator,
    String,
    Comment,
    Error,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
    pub length: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct TokenizerOps {
    pub next_token: fn() -> Token,
    pub peek_token: fn() -> Token,
    pub reset: fn(),
    pub load_text: fn(&str) -> Result<(), ()>,
    pub get_stats: fn() -> (usize, usize, usize),
}

struct TokenizerState {
    input_buffer: String,
    current_position: usize,
    current_line: usize,
    current_column: usize,
    total_tokens: usize,
    total_lines: usize,
    total_chars: usize,
    lookahead_token: Option<Token>,
}

impl Default for TokenizerState {
    fn default() -> Self {
        Self {
            input_buffer: String::new(),
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens: 0,
            total_lines: 0,
            total_chars: 0,
            lookahead_token: None,
        }
    }
}

thread_local! {
    static STATE: RefCell<TokenizerState> = RefCell::new(TokenizerState::default());
}

const KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "return", "int", "char", 
    "float", "double", "void", "struct", "typedef", "const",
    "static", "extern", "auto", "register", "sizeof", "break",
    "continue", "switch", "case", "default", "do", "goto",
    "enum", "union", "signed", "unsigned", "long", "short"
];

fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

impl TokenizerState {
    fn peek_char(&self) -> Option<char> {
        self.input_buffer[self.current_position..].chars().next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.current_position += c.len_utf8();
        self.total_chars += 1;
        if c == '\n' {
            self.current_line += 1;
            self.current_column = 1;
            self.total_lines += 1;
        } else {
            self.current_column += 1;
        }
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() && c != '\n' {
                self.advance_char();
            } else {
                break;
            }
        }
    }

    fn create_token(&mut self, token_type: TokenType, value: String) -> Token {
        let length = value.chars().count();
        let token = Token {
            token_type,
            value,
            length,
            line: self.current_line,
            column: self.current_column.saturating_sub(length),
        };
        self.total_tokens += 1;
        token
    }

    fn scan_word(&mut self) -> Token {
        let mut value = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                if let Some(ch) = self.advance_char() {
                    value.push(ch);
                }
            } else {
                break;
            }
        }
        let token_type = if is_keyword(&value) {
            TokenType::Keyword
        } else {
            TokenType::Identifier
        };
        self.create_token(token_type, value)
    }

    fn scan_number(&mut self) -> Token {
        let mut value = String::new();
        let mut has_decimal = false;
        while let Some(c) = self.peek_char() {
            if c == '.' {
                if has_decimal {
                    break;
                }
                has_decimal = true;
                if let Some(ch) = self.advance_char() {
                    value.push(ch);
                }
            } else if c.is_ascii_digit() {
                if let Some(ch) = self.advance_char() {
                    value.push(ch);
                }
            } else {
                break;
            }
        }
        self.create_token(TokenType::Number, value)
    }

    fn scan_string(&mut self) -> Token {
        let mut value = String::new();
        if let Some(quote) = self.advance_char() {
            value.push(quote);

            while let Some(c) = self.peek_char() {
                if c == quote || c == '\n' {
                    break;
                }
                if c == '\\' {
                    if let Some(escaped_slash) = self.advance_char() {
                        value.push(escaped_slash);
                    }
                    if let Some(escaped) = self.advance_char() {
                        value.push(escaped);
                    }
                } else {
                    if let Some(ch) = self.advance_char() {
                        value.push(ch);
                    }
                }
            }

            if self.peek_char() == Some(quote) {
                if let Some(ch) = self.advance_char() {
                    value.push(ch);
                }
            }
        }

        self.create_token(TokenType::String, value)
    }

    fn scan_comment(&mut self) -> Token {
        let mut value = String::new();
        if let Some(first) = self.advance_char() {
            value.push(first);

            if self.peek_char() == Some('/') {
                if let Some(second) = self.advance_char() {
                    value.push(second);
                }
                while let Some(c) = self.peek_char() {
                    if c == '\n' {
                        break;
                    }
                    if let Some(ch) = self.advance_char() {
                        value.push(ch);
                    }
                }
            } else if self.peek_char() == Some('*') {
                if let Some(second) = self.advance_char() {
                    value.push(second);
                }
                while let Some(c) = self.peek_char() {
                    if c == '*' {
                        if let Some(star) = self.advance_char() {
                            value.push(star);
                        }
                        if self.peek_char() == Some('/') {
                            if let Some(slash) = self.advance_char() {
                                value.push(slash);
                            }
                            break;
                        }
                    } else {
                        if let Some(ch) = self.advance_char() {
                            value.push(ch);
                        }
                    }
                }
            }
        }

        self.create_token(TokenType::Comment, value)
    }

    fn scan_operator(&mut self) -> Token {
        let mut value = String::new();
        if let Some(c) = self.advance_char() {
            value.push(c);

            if let Some(next) = self.peek_char() {
                let two_char = match (c, next) {
                    ('=', '=') | ('!', '=') | ('<', '=') | ('>', '=') |
                    ('&', '&') | ('|', '|') | ('+', '+') | ('-', '-') |
                    ('-', '>') | ('<', '<') | ('>', '>') => true,
                    _ => false,
                };
                if two_char {
                    if let Some(ch) = self.advance_char() {
                        value.push(ch);
                    }
                }
            }
        }

        self.create_token(TokenType::Operator, value)
    }

    fn next_token(&mut self) -> Token {
        if let Some(token) = self.lookahead_token.take() {
            return token;
        }

        self.skip_whitespace();

        let c = match self.peek_char() {
            Some(c) => c,
            None => return self.create_token(TokenType::Eof, String::new()),
        };

        if c == '\n' {
            let val = self.advance_char().unwrap_or('\n').to_string();
            return self.create_token(TokenType::Newline, val);
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

        if c == '/' {
            let mut chars = self.input_buffer[self.current_position..].chars();
            chars.next();
            let next_c = chars.next();
            if next_c == Some('/') || next_c == Some('*') {
                return self.scan_comment();
            }
        }

        if "+-*/%=<>!&|^~?:".contains(c) {
            return self.scan_operator();
        }

        if "(){}[];,.".contains(c) {
            let val = self.advance_char().unwrap_or(c).to_string();
            return self.create_token(TokenType::Punctuation, val);
        }

        let val = self.advance_char().unwrap_or(c).to_string();
        self.create_token(TokenType::Error, val)
    }
}

pub fn tokenizer_next_token() -> Token {
    STATE.with(|state| state.borrow_mut().next_token())
}

pub fn tokenizer_peek_token() -> Token {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.lookahead_token.is_none() {
            state.lookahead_token = Some(state.next_token());
        }
        state.lookahead_token.clone().unwrap()
    })
}

pub fn tokenizer_reset() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.current_position = 0;
        state.current_line = 1;
        state.current_column = 1;
        state.lookahead_token = None;
    });
}

pub fn tokenizer_load_text(text: &str) -> Result<(), ()> {
    if text.len() >= 8192 {
        eprintln!("Error: Input text too large");
        return Err(());
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.input_buffer = text.to_string();
        state.current_position = 0;
        state.current_line = 1;
        state.current_column = 1;
        state.lookahead_token = None;
    });
    Ok(())
}

pub fn tokenizer_get_stats() -> (usize, usize, usize) {
    STATE.with(|state| {
        let state = state.borrow();
        (state.total_lines, state.total_tokens, state.total_chars)
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

use std::cell::RefCell;

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

impl Token {
    pub fn new(token_type: TokenType, value: &str, length: usize, line: i32, column: i32) -> Self {
        Token {
            token_type,
            value: value.to_string(),
            length,
            line,
            column,
        }
    }
}

thread_local! {
    static INPUT_BUFFER: RefCell<String> = RefCell::new(String::new());
    static BUFFER_LENGTH: RefCell<usize> = RefCell::new(0);
    static CURRENT_POSITION: RefCell<usize> = RefCell::new(0);
    static CURRENT_LINE: RefCell<i32> = RefCell::new(1);
    static CURRENT_COLUMN: RefCell<i32> = RefCell::new(1);
    static TOTAL_TOKENS_PROCESSED: RefCell<usize> = RefCell::new(0);
    static TOTAL_LINES_PROCESSED: RefCell<usize> = RefCell::new(0);
    static TOTAL_CHARS_PROCESSED: RefCell<usize> = RefCell::new(0);
    static LOOKAHEAD_TOKEN: RefCell<Option<Token>> = RefCell::new(None);
    static LOOKAHEAD_VALID: RefCell<bool> = RefCell::new(false);
}

const KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "return", "int", "char",
    "float", "double", "void", "struct", "typedef", "const",
    "static", "extern", "auto", "register", "sizeof", "break",
    "continue", "switch", "case", "default", "do", "goto",
    "enum", "union", "signed", "unsigned", "long", "short",
];

fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

fn peek_char() -> char {
    INPUT_BUFFER.with(|buf| {
        let buf = buf.borrow();
        CURRENT_POSITION.with(|pos| {
            let pos = *pos.borrow();
            if pos >= buf.len() {
                '\0'
            } else {
                buf.chars().nth(pos).unwrap_or('\0')
            }
        })
    })
}

fn advance_char() -> char {
    INPUT_BUFFER.with(|buf| {
        let mut buf = buf.borrow_mut();
        CURRENT_POSITION.with(|pos| {
            let mut pos = pos.borrow_mut();
            CURRENT_LINE.with(|line| {
                let mut line = line.borrow_mut();
                CURRENT_COLUMN.with(|col| {
                    let mut col = col.borrow_mut();
                    TOTAL_CHARS_PROCESSED.with(|chars| {
                        let mut chars = chars.borrow_mut();
                        TOTAL_LINES_PROCESSED.with(|lines| {
                            let mut lines = lines.borrow_mut();
                            
                            if *pos >= buf.len() {
                                return '\0';
                            }
                            
                            let c = buf.chars().nth(*pos).unwrap_or('\0');
                            *pos += 1;
                            *chars += 1;
                            
                            if c == '\n' {
                                *line += 1;
                                *col = 1;
                                *lines += 1;
                            } else {
                                *col += 1;
                            }
                            
                            c
                        })
                    })
                })
            })
        })
    })
}

fn skip_whitespace() {
    while peek_char() != '\0' && peek_char().is_ascii_whitespace() && peek_char() != '\n' {
        advance_char();
    }
}

fn create_token(token_type: TokenType, value: &str, length: usize) -> Token {
    let actual_length = length.min(MAX_TOKEN_LENGTH - 1);
    let truncated_value = &value[..actual_length.min(value.len())];
    
    CURRENT_COLUMN.with(|col| {
        let col = *col.borrow();
        CURRENT_LINE.with(|line| {
            let line = *line.borrow();
            TOTAL_TOKENS_PROCESSED.with(|tokens| {
                *tokens.borrow_mut() += 1;
            });
            Token::new(token_type, truncated_value, actual_length, line, col - actual_length as i32)
        })
    })
}

fn scan_word() -> Token {
    let mut buffer = String::new();
    
    while peek_char() != '\0' && (peek_char().is_alphanumeric() || peek_char() == '_') && buffer.len() < MAX_TOKEN_LENGTH - 1 {
        buffer.push(advance_char());
    }
    
    if is_keyword(&buffer) {
        create_token(TokenType::Keyword, &buffer, buffer.len())
    } else {
        create_token(TokenType::Identifier, &buffer, buffer.len())
    }
}

fn scan_number() -> Token {
    let mut buffer = String::new();
    let mut has_decimal = false;
    
    while peek_char() != '\0' && (peek_char().is_ascii_digit() || peek_char() == '.') && buffer.len() < MAX_TOKEN_LENGTH - 1 {
        if peek_char() == '.' {
            if has_decimal {
                break;
            }
            has_decimal = true;
        }
        buffer.push(advance_char());
    }
    
    create_token(TokenType::Number, &buffer, buffer.len())
}

fn scan_string() -> Token {
    let mut buffer = String::new();
    let quote = advance_char();
    buffer.push(quote);
    
    while peek_char() != '\0' && peek_char() != quote && peek_char() != '\n' && buffer.len() < MAX_TOKEN_LENGTH - 2 {
        if peek_char() == '\\' {
            buffer.push(advance_char());
            if peek_char() != '\0' {
                buffer.push(advance_char());
            }
        } else {
            buffer.push(advance_char());
        }
    }
    
    if peek_char() == quote {
        buffer.push(advance_char());
    }
    
    create_token(TokenType::String, &buffer, buffer.len())
}

fn scan_comment() -> Token {
    let mut buffer = String::new();
    buffer.push(advance_char());
    
    if peek_char() == '/' {
        buffer.push(advance_char());
        while peek_char() != '\0' && peek_char() != '\n' && buffer.len() < MAX_TOKEN_LENGTH - 1 {
            buffer.push(advance_char());
        }
    } else if peek_char() == '*' {
        buffer.push(advance_char());
        while peek_char() != '\0' && buffer.len() < MAX_TOKEN_LENGTH - 2 {
            if peek_char() == '*' {
                buffer.push(advance_char());
                if peek_char() == '/' {
                    buffer.push(advance_char());
                    break;
                }
            } else {
                buffer.push(advance_char());
            }
        }
    }
    
    create_token(TokenType::Comment, &buffer, buffer.len())
}

fn scan_operator() -> Token {
    let mut buffer = String::new();
    let c = peek_char();
    buffer.push(advance_char());
    
    let next = peek_char();
    let two_char_ops = [
        ("==", '='), ("!=", '!'), ("<=", '<'), (">=", '>'),
        ("&&", '&'), ("||", '|'), ("++", '+'), ("--", '-'),
        ("->", '-'), ("<<", '<'), (">>", '>'),
    ];
    
    for (op, first) in &two_char_ops {
        if c == *first && next == op.chars().nth(1).unwrap() {
            buffer.push(advance_char());
            break;
        }
    }
    
    create_token(TokenType::Operator, &buffer, buffer.len())
}

pub fn tokenizer_next_token() -> Token {
    LOOKAHEAD_VALID.with(|valid| {
        if *valid.borrow() {
            *valid.borrow_mut() = false;
            return LOOKAHEAD_TOKEN.with(|tok| tok.borrow().clone().unwrap());
        }
        Token::new(TokenType::Eof, "", 0, 0, 0)
    });
    
    skip_whitespace();
    
    CURRENT_POSITION.with(|pos| {
        BUFFER_LENGTH.with(|len| {
            if *pos.borrow() >= *len.borrow() {
                return create_token(TokenType::Eof, "", 0);
            }
            Token::new(TokenType::Eof, "", 0, 0, 0)
        })
    });
    
    let c = peek_char();
    
    if c == '\n' {
        let newline = advance_char().to_string();
        return create_token(TokenType::Newline, &newline, 1);
    }
    
    if c.is_alphabetic() || c == '_' {
        return scan_word();
    }
    
    if c.is_ascii_digit() {
        return scan_number();
    }
    
    if c == '"' || c == '\'' {
        return scan_string();
    }
    
    if c == '/' {
        let next = INPUT_BUFFER.with(|buf| {
            let buf = buf.borrow();
            CURRENT_POSITION.with(|pos| {
                let pos = *pos.borrow();
                if pos + 1 < buf.len() {
                    buf.chars().nth(pos + 1)
                } else {
                    None
                }
            })
        });
        if next == Some('/') || next == Some('*') {
            return scan_comment();
        }
    }
    
    if "+-*/%=<>!&|^~?:".contains(c) {
        return scan_operator();
    }
    
    if "(){}[];,.".contains(c) {
        let punct = advance_char().to_string();
        return create_token(TokenType::Punctuation, &punct, 1);
    }
    
    let unknown = advance_char().to_string();
    create_token(TokenType::Error, &unknown, 1)
}

pub fn tokenizer_peek_token() -> Token {
    LOOKAHEAD_VALID.with(|valid| {
        if !*valid.borrow() {
            let token = tokenizer_next_token();
            LOOKAHEAD_TOKEN.with(|tok| {
                *tok.borrow_mut() = Some(token);
            });
            *valid.borrow_mut() = true;
        }
        LOOKAHEAD_TOKEN.with(|tok| tok.borrow().clone().unwrap())
    })
}

pub fn tokenizer_reset() {
    CURRENT_POSITION.with(|p| *p.borrow_mut() = 0);
    CURRENT_LINE.with(|l| *l.borrow_mut() = 1);
    CURRENT_COLUMN.with(|c| *c.borrow_mut() = 1);
    LOOKAHEAD_VALID.with(|v| *v.borrow_mut() = false);
}

pub fn tokenizer_load_text(text: &str) -> i32 {
    if text.len() >= MAX_BUFFER_SIZE {
        eprintln!("Error: Input text too large");
        return -1;
    }
    
    INPUT_BUFFER.with(|buf| {
        *buf.borrow_mut() = text.to_string();
    });
    BUFFER_LENGTH.with(|len| {
        *len.borrow_mut() = text.len();
    });
    
    tokenizer_reset();
    0
}

pub fn tokenizer_get_stats() -> (usize, usize, usize) {
    let lines = TOTAL_LINES_PROCESSED.with(|l| *l.borrow());
    let tokens = TOTAL_TOKENS_PROCESSED.with(|t| *t.borrow());
    let chars = TOTAL_CHARS_PROCESSED.with(|c| *c.borrow());
    (lines, tokens, chars)
}

pub struct TokenizerOps;

impl TokenizerOps {
    pub fn next_token(&self) -> Token {
        tokenizer_next_token()
    }
    
    pub fn peek_token(&self) -> Token {
        tokenizer_peek_token()
    }
    
    pub fn reset(&self) {
        tokenizer_reset()
    }
    
    pub fn load_text(&self, text: &str) -> i32 {
        tokenizer_load_text(text)
    }
    
    pub fn get_stats(&self) -> (usize, usize, usize) {
        tokenizer_get_stats()
    }
}

pub fn get_tokenizer_ops() -> TokenizerOps {
    TokenizerOps
}

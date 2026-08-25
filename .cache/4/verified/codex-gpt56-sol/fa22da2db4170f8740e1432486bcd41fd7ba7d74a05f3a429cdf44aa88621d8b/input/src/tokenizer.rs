pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
#[allow(dead_code)]
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

#[derive(Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: Vec<u8>,
    pub line: i32,
    pub column: i32,
}

pub struct Tokenizer {
    input_buffer: Vec<u8>,
    current_position: usize,
    current_line: i32,
    current_column: i32,
    total_tokens_processed: usize,
    total_lines_processed: usize,
    total_chars_processed: usize,
    lookahead_token: Option<Token>,
}

const KEYWORDS: &[&[u8]] = &[
    b"if",
    b"else",
    b"while",
    b"for",
    b"return",
    b"int",
    b"char",
    b"float",
    b"double",
    b"void",
    b"struct",
    b"typedef",
    b"const",
    b"static",
    b"extern",
    b"auto",
    b"register",
    b"sizeof",
    b"break",
    b"continue",
    b"switch",
    b"case",
    b"default",
    b"do",
    b"goto",
    b"enum",
    b"union",
    b"signed",
    b"unsigned",
    b"long",
    b"short",
];

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            input_buffer: Vec::new(),
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens_processed: 0,
            total_lines_processed: 0,
            total_chars_processed: 0,
            lookahead_token: None,
        }
    }

    fn is_keyword(value: &[u8]) -> bool {
        KEYWORDS.contains(&value)
    }

    fn peek_char(&self) -> u8 {
        self.input_buffer
            .get(self.current_position)
            .copied()
            .unwrap_or(0)
    }

    fn advance_char(&mut self) -> u8 {
        if self.current_position >= self.input_buffer.len() {
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
        while self.peek_char() != 0 && is_space(self.peek_char()) && self.peek_char() != b'\n' {
            self.advance_char();
        }
    }

    fn create_token(&mut self, token_type: TokenType, value: &[u8], length: usize) -> Token {
        let token_length = if length < MAX_TOKEN_LENGTH {
            length
        } else {
            MAX_TOKEN_LENGTH - 1
        };
        let column = (self.current_column as usize).wrapping_sub(token_length) as i32;
        self.total_tokens_processed += 1;

        Token {
            token_type,
            value: value[..token_length].to_vec(),
            line: self.current_line,
            column,
        }
    }

    fn scan_word(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);

        while self.peek_char() != 0
            && (is_alphanumeric(self.peek_char()) || self.peek_char() == b'_')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            buffer.push(self.advance_char());
        }

        let token_type = if Self::is_keyword(&buffer) {
            TokenType::Keyword
        } else {
            TokenType::Identifier
        };
        self.create_token(token_type, &buffer, buffer.len())
    }

    fn scan_number(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
        let mut has_decimal = false;

        while self.peek_char() != 0
            && (is_digit(self.peek_char()) || self.peek_char() == b'.')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            if self.peek_char() == b'.' {
                if has_decimal {
                    break;
                }
                has_decimal = true;
            }
            buffer.push(self.advance_char());
        }

        self.create_token(TokenType::Number, &buffer, buffer.len())
    }

    fn scan_string(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
        let quote = self.advance_char();
        buffer.push(quote);

        while self.peek_char() != 0
            && self.peek_char() != quote
            && self.peek_char() != b'\n'
            && buffer.len() < MAX_TOKEN_LENGTH - 2
        {
            if self.peek_char() == b'\\' {
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

        self.create_token(TokenType::String, &buffer, buffer.len())
    }

    fn scan_comment(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
        buffer.push(self.advance_char());

        if self.peek_char() == b'/' {
            buffer.push(self.advance_char());
            while self.peek_char() != 0
                && self.peek_char() != b'\n'
                && buffer.len() < MAX_TOKEN_LENGTH - 1
            {
                buffer.push(self.advance_char());
            }
        } else if self.peek_char() == b'*' {
            buffer.push(self.advance_char());
            while self.peek_char() != 0 && buffer.len() < MAX_TOKEN_LENGTH - 2 {
                if self.peek_char() == b'*' {
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

        self.create_token(TokenType::Comment, &buffer, buffer.len())
    }

    fn scan_operator(&mut self) -> Token {
        let mut buffer = Vec::with_capacity(2);
        let c = self.peek_char();
        buffer.push(self.advance_char());

        let next = self.peek_char();
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
            buffer.push(self.advance_char());
        }

        self.create_token(TokenType::Operator, &buffer, buffer.len())
    }

    pub fn next_token(&mut self) -> Token {
        if let Some(token) = self.lookahead_token.take() {
            return token;
        }

        self.skip_whitespace();

        if self.current_position >= self.input_buffer.len() {
            return self.create_token(TokenType::Eof, b"", 0);
        }

        let c = self.peek_char();
        if c == b'\n' {
            let newline = [self.advance_char()];
            return self.create_token(TokenType::Newline, &newline, 1);
        }

        if is_alpha(c) || c == b'_' {
            return self.scan_word();
        }

        if is_digit(c) {
            return self.scan_number();
        }

        if c == b'"' || c == b'\'' {
            return self.scan_string();
        }

        // The second peek in the C condition still sees c, so every slash enters here.
        if c == b'/' && (self.peek_char() == b'/' || self.peek_char() == b'*') {
            return self.scan_comment();
        }

        if b"+-*/%=<>!&|^~?:".contains(&c) {
            return self.scan_operator();
        }

        if b"(){}[];,.".contains(&c) {
            let punctuation = [self.advance_char()];
            return self.create_token(TokenType::Punctuation, &punctuation, 1);
        }

        let unknown = [self.advance_char()];
        self.create_token(TokenType::Error, &unknown, 1)
    }

    #[allow(dead_code)]
    pub fn peek_token(&mut self) -> Token {
        if self.lookahead_token.is_none() {
            self.lookahead_token = Some(self.next_token());
        }
        self.lookahead_token.as_ref().unwrap().clone()
    }

    pub fn reset(&mut self) {
        self.current_position = 0;
        self.current_line = 1;
        self.current_column = 1;
        self.lookahead_token = None;
    }

    pub fn load_text(&mut self, text: &[u8], stderr: &mut Vec<u8>) -> bool {
        let length = text.iter().position(|&c| c == 0).unwrap_or(text.len());
        if length >= MAX_BUFFER_SIZE {
            stderr.extend_from_slice(b"Error: Input text too large\n");
            return false;
        }

        self.input_buffer.clear();
        self.input_buffer.extend_from_slice(&text[..length]);
        self.reset();
        true
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.total_lines_processed,
            self.total_tokens_processed,
            self.total_chars_processed,
        )
    }
}

fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_alphanumeric(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

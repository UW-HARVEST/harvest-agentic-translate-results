use crate::token;
pub const SIMPLE_LANG_LEXER_H: bool = true;

/// Replicates: Token* tokenize(const char* source);
/// In Rust: returns a vector of Tokens.
pub fn tokenize(source: &str) -> Vec<token::Token> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut pos: usize = 0;
    let mut tokens: Vec<token::Token> = Vec::new();

    while pos < len {
        let c = bytes[pos] as char;
        if c.is_whitespace() {
            pos += 1;
        } else if c.is_ascii_digit() {
            let start = pos;
            while pos < len && (bytes[pos] as char).is_ascii_digit() {
                pos += 1;
            }
            let num: String = std::str::from_utf8(&bytes[start..pos])
                .unwrap_or("")
                .to_string();
            tokens.push(new_token(token::TokenType::TOKEN_INT, &num));
        } else if c.is_ascii_alphabetic() {
            let start = pos;
            while pos < len && (bytes[pos] as char).is_ascii_alphabetic() {
                pos += 1;
            }
            let ident: String = std::str::from_utf8(&bytes[start..pos])
                .unwrap_or("")
                .to_string();
            if ident == "let" {
                tokens.push(new_token(token::TokenType::TOKEN_LET, &ident));
            } else if ident == "dis" {
                tokens.push(new_token(token::TokenType::TOKEN_DIS, &ident));
            } else {
                tokens.push(new_token(token::TokenType::TOKEN_IDENTIFIER, &ident));
            }
        } else {
            match c {
                '+' => tokens.push(new_token(token::TokenType::TOKEN_PLUS, "+")),
                '-' => tokens.push(new_token(token::TokenType::TOKEN_MINUS, "-")),
                ';' => tokens.push(new_token(token::TokenType::TOKEN_SEMICOLON, ";")),
                '=' => tokens.push(new_token(token::TokenType::TOKEN_ASSIGN, "=")),
                _ => {
                    eprintln!("SyntaxError: unexpected character {}", c);
                    std::process::exit(1);
                }
            }
            pos += 1;
        }
    }

    tokens.push(new_token(token::TokenType::TOKEN_EOF, ""));
    tokens
}

/// Replicates: void free_token(Token* token);
pub fn free_token(_token: token::Token) {
    // In Rust, Drop handles cleanup automatically.
}

/// Replicates: Token* new_token(TokenType type, const char* value);
/// In Rust: returns a new Token struct.
pub fn new_token(type_: token::TokenType, value: &str) -> token::Token {
    token::Token {
        token_type: type_,
        value: value.to_string(),
    }
}

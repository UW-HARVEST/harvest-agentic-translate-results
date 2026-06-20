use crate::token;
pub const SIMPLE_LANG_LEXER_H: bool = true;

fn syntax_error(ch: char) -> ! {
    eprintln!("SyntaxError: unexpected character {}", ch);
    std::process::exit(1);
}

/// Replicates: Token* tokenize(const char* source);
/// In Rust: returns a vector of Tokens.
pub fn tokenize(source: &str) -> Vec<token::Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut pos = 0;

    while pos < chars.len() {
        let ch = chars[pos];
        if ch.is_whitespace() {
            pos += 1;
        } else if ch.is_ascii_digit() {
            let start = pos;
            while pos < chars.len() && chars[pos].is_ascii_digit() {
                pos += 1;
            }
            let num: String = chars[start..pos].iter().collect();
            tokens.push(new_token(token::TokenType::TOKEN_INT, &num));
        } else if ch.is_ascii_alphabetic() {
            let start = pos;
            while pos < chars.len() && chars[pos].is_ascii_alphabetic() {
                pos += 1;
            }
            let ident: String = chars[start..pos].iter().collect();
            let token_type = match ident.as_str() {
                "let" => token::TokenType::TOKEN_LET,
                "dis" => token::TokenType::TOKEN_DIS,
                _ => token::TokenType::TOKEN_IDENTIFIER,
            };
            tokens.push(new_token(token_type, &ident));
        } else {
            let tok = match ch {
                '+' => new_token(token::TokenType::TOKEN_PLUS, "+"),
                '-' => new_token(token::TokenType::TOKEN_MINUS, "-"),
                ';' => new_token(token::TokenType::TOKEN_SEMICOLON, ";"),
                '=' => new_token(token::TokenType::TOKEN_ASSIGN, "="),
                _ => syntax_error(ch),
            };
            tokens.push(tok);
            pos += 1;
        }
    }

    tokens.push(new_token(token::TokenType::TOKEN_EOF, ""));
    tokens
}
/// Replicates: void free_token(Token* token);
pub fn free_token(_token: token::Token) {
    // Rust drops owned data automatically.
}
/// Replicates: Token* new_token(TokenType type, const char* value);
/// In Rust: returns a new Token struct.
pub fn new_token(type_: token::TokenType, value: &str) -> token::Token {
    token::new_token(type_, value)
}

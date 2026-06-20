use crate::token;
pub const SIMPLE_LANG_LEXER_H: bool = true;
/// Replicates: Token* tokenize(const char* source);
/// In Rust: returns a vector of Tokens.
pub fn tokenize(source: &str) -> Vec<token::Token> {
    let bytes = source.as_bytes();
    let mut pos = 0;
    let mut tokens = Vec::new();

    while pos < bytes.len() {
        let ch = bytes[pos];
        if ch.is_ascii_whitespace() {
            pos += 1;
        } else if ch.is_ascii_digit() {
            let start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            let num = crate::utils::strndup(&source[start..pos], pos - start);
            tokens.push(new_token(token::TokenType::TOKEN_INT, &num));
        } else if ch.is_ascii_alphabetic() {
            let start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_alphabetic() {
                pos += 1;
            }
            let ident = crate::utils::strndup(&source[start..pos], pos - start);
            let token_type = match ident.as_str() {
                "let" => token::TokenType::TOKEN_LET,
                "dis" => token::TokenType::TOKEN_DIS,
                _ => token::TokenType::TOKEN_IDENTIFIER,
            };
            tokens.push(new_token(token_type, &ident));
        } else {
            let (token_type, value) = match ch {
                b'+' => (token::TokenType::TOKEN_PLUS, "+"),
                b'-' => (token::TokenType::TOKEN_MINUS, "-"),
                b';' => (token::TokenType::TOKEN_SEMICOLON, ";"),
                b'=' => (token::TokenType::TOKEN_ASSIGN, "="),
                _ => {
                    eprintln!("SyntaxError: unexpected character {}", ch as char);
                    std::process::exit(1);
                }
            };
            tokens.push(new_token(token_type, value));
            pos += 1;
        }
    }

    tokens.push(new_token(token::TokenType::TOKEN_EOF, ""));
    tokens
}
/// Replicates: void free_token(Token* token);
pub fn free_token(_token: token::Token) {
}
/// Replicates: Token* new_token(TokenType type, const char* value);
/// In Rust: returns a new Token struct.
pub fn new_token(type_: token::TokenType, value: &str) -> token::Token {
    token::new_token(type_, value)
}

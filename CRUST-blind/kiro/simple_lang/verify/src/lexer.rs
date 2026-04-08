use crate::token;
pub const SIMPLE_LANG_LEXER_H: bool = true;
/// Replicates: Token* tokenize(const char* source);
/// In Rust: returns a vector of Tokens.
pub fn tokenize(source: &str) -> Vec<token::Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut pos = 0;

    while pos < len {
        if chars[pos].is_whitespace() {
            pos += 1;
        } else if chars[pos].is_ascii_digit() {
            let start = pos;
            while pos < len && chars[pos].is_ascii_digit() {
                pos += 1;
            }
            let num: String = chars[start..pos].iter().collect();
            tokens.push(new_token(token::TokenType::TOKEN_INT, &num));
        } else if chars[pos].is_ascii_alphabetic() {
            let start = pos;
            while pos < len && chars[pos].is_ascii_alphabetic() {
                pos += 1;
            }
            let ident: String = chars[start..pos].iter().collect();
            if ident == "let" {
                tokens.push(new_token(token::TokenType::TOKEN_LET, &ident));
            } else if ident == "dis" {
                tokens.push(new_token(token::TokenType::TOKEN_DIS, &ident));
            } else {
                tokens.push(new_token(token::TokenType::TOKEN_IDENTIFIER, &ident));
            }
        } else {
            match chars[pos] {
                '+' => tokens.push(new_token(token::TokenType::TOKEN_PLUS, "+")),
                '-' => tokens.push(new_token(token::TokenType::TOKEN_MINUS, "-")),
                ';' => tokens.push(new_token(token::TokenType::TOKEN_SEMICOLON, ";")),
                '=' => tokens.push(new_token(token::TokenType::TOKEN_ASSIGN, "=")),
                c => {
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
    // Rust handles deallocation automatically.
}
/// Replicates: Token* new_token(TokenType type, const char* value);
/// In Rust: returns a new Token struct.
pub fn new_token(type_: token::TokenType, value: &str) -> token::Token {
    token::Token {
        token_type: type_,
        value: value.to_string(),
    }
}

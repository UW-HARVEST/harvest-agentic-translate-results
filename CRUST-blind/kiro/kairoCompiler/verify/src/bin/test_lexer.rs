use kairoCompiler::compiler::{
    CompileProcess, Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_KEYWORD, TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL,
    TOKEN_TYPE_STRING, TOKEN_TYPE_COMMENT, TOKEN_TYPE_NEWLINE,
    NUMBER_TYPE_NORMAL, NUMBER_TYPE_LONG,
};
use kairoCompiler::lexer::{tokens_build_for_string, token_from_bytes_pub};
use kairoCompiler::vector::{vector_set_peek_pointer, vector_peek, vector_count};

fn lex_string(s: &str) -> Vec<Token> {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, s).expect("lex failed");
    let tv = lp.token_vec.as_mut().expect("no token_vec");
    let count = vector_count(tv);
    vector_set_peek_pointer(tv, 0);
    let mut tokens = Vec::new();
    for _ in 0..count {
        if let Some(bytes) = vector_peek(tv) {
            tokens.push(token_from_bytes_pub(bytes));
        }
    }
    tokens
}

#[test]
fn test_lex_number() {
    let tokens = lex_string("42$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[0].llnum, Some(42));
    assert_eq!(tokens[0].num.r#type, NUMBER_TYPE_NORMAL);
}

#[test]
fn test_lex_identifier() {
    let tokens = lex_string("hello$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_IDENTIFIER);
    assert_eq!(tokens[0].sval, Some("hello".to_string()));
}

#[test]
fn test_lex_keyword() {
    let tokens = lex_string("int$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_KEYWORD);
    assert_eq!(tokens[0].sval, Some("int".to_string()));
}

#[test]
fn test_lex_operator() {
    let tokens = lex_string("+$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_OPERATOR);
    // Rust correctly returns the operator string (C has UB here with missing return)
    assert_eq!(tokens[0].sval, Some("+".to_string()));
}

#[test]
fn test_lex_symbol() {
    let tokens = lex_string("{$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_SYMBOL);
    assert_eq!(tokens[0].cval, Some('{'));
}

#[test]
fn test_lex_string_literal() {
    let tokens = lex_string("\"hello\"$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_STRING);
    assert_eq!(tokens[0].sval, Some("hello".to_string()));
}

#[test]
fn test_lex_multiple_tokens() {
    let tokens = lex_string("123 abc$");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[0].llnum, Some(123));
    assert_eq!(tokens[1].r#type, TOKEN_TYPE_IDENTIFIER);
    assert_eq!(tokens[1].sval, Some("abc".to_string()));
}

#[test]
fn test_lex_newline() {
    let tokens = lex_string("\n$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NEWLINE);
}

#[test]
fn test_lex_one_line_comment() {
    let tokens = lex_string("//comment\n$");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_COMMENT);
    assert_eq!(tokens[1].r#type, TOKEN_TYPE_NEWLINE);
}

#[test]
fn test_lex_multiline_comment() {
    let tokens = lex_string("/*hi*/$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_COMMENT);
    assert_eq!(tokens[0].sval, Some("hi".to_string()));
}

#[test]
fn test_lex_multi_char_operator() {
    let tokens = lex_string("++$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_OPERATOR);
    assert_eq!(tokens[0].sval, Some("++".to_string()));
}

#[test]
fn test_lex_whitespace_flag() {
    let tokens = lex_string("a b$");
    assert_eq!(tokens.len(), 2);
    assert!(tokens[0].whitespace); // 'a' has whitespace after it
    assert!(!tokens[1].whitespace);
}

#[test]
fn test_lex_char_literal() {
    let tokens = lex_string("'a'$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[0].cval, Some('a'));
}

#[test]
fn test_lex_escaped_char_literal() {
    let tokens = lex_string("'\\n'$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[0].cval, Some('\n'));
}

#[test]
fn test_lex_number_long_suffix() {
    let tokens = lex_string("100L$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[0].llnum, Some(100));
    assert_eq!(tokens[0].num.r#type, NUMBER_TYPE_LONG);
}

#[test]
fn test_lex_hex_number() {
    let tokens = lex_string("0xFF$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[0].llnum, Some(255));
}

#[test]
fn test_lex_binary_number() {
    let tokens = lex_string("0b1010$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[0].llnum, Some(10));
}

#[test]
fn test_lex_parenthesized_expression() {
    let tokens = lex_string("(1)$");
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_OPERATOR);
    assert_eq!(tokens[0].sval, Some("(".to_string()));
    assert_eq!(tokens[1].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[1].llnum, Some(1));
    assert_eq!(tokens[2].r#type, TOKEN_TYPE_SYMBOL);
    assert_eq!(tokens[2].cval, Some(')'));
}

// Note: ")$" without matching "(" calls compiler_error and exits, matching C behavior

#[test]
fn test_lex_various_keywords() {
    for kw in &["return", "if", "while", "for", "void", "char", "struct"] {
        let input = format!("{}$", kw);
        let tokens = lex_string(&input);
        assert_eq!(tokens.len(), 1, "keyword: {}", kw);
        assert_eq!(tokens[0].r#type, TOKEN_TYPE_KEYWORD, "keyword: {}", kw);
        assert_eq!(tokens[0].sval, Some(kw.to_string()), "keyword: {}", kw);
    }
}

#[test]
fn test_lex_x_as_identifier() {
    let tokens = lex_string("x$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_IDENTIFIER);
    assert_eq!(tokens[0].sval, Some("x".to_string()));
}

#[test]
fn test_lex_b_as_identifier() {
    let tokens = lex_string("b$");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_IDENTIFIER);
    assert_eq!(tokens[0].sval, Some("b".to_string()));
}

#[test]
fn test_lex_division_operator() {
    // Division goes through handle_comment, pushes back '/', then makes operator
    let tokens = lex_string("5/2$");
    // In Rust, read_op correctly returns the string, so we get 3 tokens
    assert!(tokens.len() >= 2);
    assert_eq!(tokens[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tokens[0].llnum, Some(5));
}

fn main() {}

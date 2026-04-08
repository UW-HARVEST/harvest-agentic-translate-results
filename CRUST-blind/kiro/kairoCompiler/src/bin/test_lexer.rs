use kairoCompiler::compiler::{
    CompileProcess, Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_KEYWORD, TOKEN_TYPE_STRING, TOKEN_TYPE_SYMBOL,
    TOKEN_TYPE_COMMENT, TOKEN_TYPE_NEWLINE, NUMBER_TYPE_LONG,
    NUMBER_TYPE_NORMAL,
};
use kairoCompiler::lexer::{tokens_build_for_string, TOKENS};
use kairoCompiler::vector::{vector_count, vector_at};

fn get_token_from_lp(lp: &mut kairoCompiler::lex_process::LexProcess, index: i32) -> Token {
    let tv = lp.token_vec.as_mut().unwrap();
    let bytes = vector_at(tv, index).unwrap();
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    let idx = u64::from_le_bytes(arr) as usize;
    let tokens = TOKENS.lock().unwrap();
    tokens[idx].clone()
}

fn token_count(lp: &kairoCompiler::lex_process::LexProcess) -> i32 {
    vector_count(lp.token_vec.as_ref().unwrap())
}

#[test]
fn test_lex_number() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "42$").unwrap();
    assert_eq!(token_count(&lp), 1);
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t.llnum, Some(42));
}

#[test]
fn test_lex_identifier() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "hello$").unwrap();
    assert_eq!(token_count(&lp), 1);
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_IDENTIFIER);
    assert_eq!(t.sval, Some("hello".to_string()));
}

#[test]
fn test_lex_keyword() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "int$").unwrap();
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_KEYWORD);
    assert_eq!(t.sval, Some("int".to_string()));
}

#[test]
fn test_lex_string() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "\"hello world\"$").unwrap();
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_STRING);
    assert_eq!(t.sval, Some("hello world".to_string()));
}

#[test]
fn test_lex_symbol() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "{$").unwrap();
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_SYMBOL);
    assert_eq!(t.cval, Some('{'));
}

#[test]
fn test_lex_quote_char() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "'a'$").unwrap();
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t.cval, Some('a'));
}

#[test]
fn test_lex_number_long() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "42L$").unwrap();
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t.llnum, Some(42));
    assert_eq!(t.num.r#type, NUMBER_TYPE_LONG);
}

#[test]
fn test_lex_escaped_char() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "'\\n'$").unwrap();
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t.cval, Some('\n')); // newline = 10
}

#[test]
fn test_lex_single_line_comment() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "// comment\n42$").unwrap();
    // C produces 3 tokens: comment, newline, number
    assert_eq!(token_count(&lp), 3);
    let t0 = get_token_from_lp(&mut lp, 0);
    assert_eq!(t0.r#type, TOKEN_TYPE_COMMENT);
}

#[test]
fn test_lex_multiline_comment() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "/* multi */42$").unwrap();
    assert_eq!(token_count(&lp), 2);
}

#[test]
fn test_lex_whitespace_flag() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "42 43$").unwrap();
    assert_eq!(token_count(&lp), 2);
    let t0 = get_token_from_lp(&mut lp, 0);
    assert!(t0.whitespace);
    let t1 = get_token_from_lp(&mut lp, 1);
    assert_eq!(t1.llnum, Some(43));
}

#[test]
fn test_lex_newline() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "\n42$").unwrap();
    assert_eq!(token_count(&lp), 2);
    let t0 = get_token_from_lp(&mut lp, 0);
    assert_eq!(t0.r#type, TOKEN_TYPE_NEWLINE);
}

#[test]
fn test_lex_hex_number() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "0xFF$").unwrap();
    assert_eq!(token_count(&lp), 1);
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t.llnum, Some(255));
}

#[test]
fn test_lex_various_keywords() {
    for kw in &["return", "if", "while", "for", "struct", "void", "char"] {
        let cp = CompileProcess::default();
        let input = format!("{}$", kw);
        let mut lp = tokens_build_for_string(cp, &input).unwrap();
        let t = get_token_from_lp(&mut lp, 0);
        assert_eq!(t.r#type, TOKEN_TYPE_KEYWORD, "Expected keyword for '{}'", kw);
        assert_eq!(t.sval, Some(kw.to_string()));
    }
}

#[test]
fn test_lex_number_normal_type() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "42$").unwrap();
    let t = get_token_from_lp(&mut lp, 0);
    assert_eq!(t.num.r#type, NUMBER_TYPE_NORMAL);
}

fn main() {}

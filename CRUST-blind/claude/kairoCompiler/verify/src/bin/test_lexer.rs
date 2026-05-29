use kairoCompiler::lexer::{read_next_token, lex, COMPILER_LEX_FUNCTIONS};
use kairoCompiler::lex_process::lex_process_create;
use kairoCompiler::cprocess::compile_process_create;
use kairoCompiler::compiler::{
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_STRING,
    TOKEN_TYPE_SYMBOL, TOKEN_TYPE_OPERATOR, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_NORMAL, NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT,
    LEXICAL_ANALYSIS_ALL_OK,
};
use kairoCompiler::vector::vector_count;
use std::fs;
use std::io::Write;

fn create_lex_for(input: &str, path_suffix: &str) -> kairoCompiler::lex_process::LexProcess {
    let path = format!("/tmp/lex_test_{}.txt", path_suffix);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(input.as_bytes()).unwrap();
    let process = compile_process_create(&path, "", 0).unwrap();
    lex_process_create(process, COMPILER_LEX_FUNCTIONS, None)
}

#[test]
fn test_lex_simple_number() {
    let mut lp = create_lex_for("42$", "num");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tok.llnum, Some(42));
    assert_eq!(tok.num.r#type, NUMBER_TYPE_NORMAL);
    let none = read_next_token(&mut lp);
    assert!(none.is_none());
}

#[test]
fn test_lex_number_long_suffix() {
    let mut lp = create_lex_for("123L$", "long");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tok.llnum, Some(123));
    assert_eq!(tok.num.r#type, NUMBER_TYPE_LONG);
}

#[test]
fn test_lex_number_float_suffix() {
    let mut lp = create_lex_for("123f$", "float");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tok.llnum, Some(123));
    assert_eq!(tok.num.r#type, NUMBER_TYPE_FLOAT);
}

#[test]
fn test_lex_identifier() {
    let mut lp = create_lex_for("hello$", "ident");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_IDENTIFIER);
    assert_eq!(tok.sval, Some("hello".to_string()));
}

#[test]
fn test_lex_keyword() {
    let mut lp = create_lex_for("if$", "keyword");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_KEYWORD);
    assert_eq!(tok.sval, Some("if".to_string()));
}

#[test]
fn test_lex_string() {
    let mut lp = create_lex_for("\"hello\"$", "str");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_STRING);
    assert_eq!(tok.sval, Some("hello".to_string()));
}

#[test]
fn test_lex_symbol() {
    let mut lp = create_lex_for("{$", "sym");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_SYMBOL);
    assert_eq!(tok.cval, Some('{'));
}

#[test]
fn test_lex_two_symbols() {
    let mut lp = create_lex_for("{}$", "twosym");
    let t1 = read_next_token(&mut lp).unwrap();
    assert_eq!(t1.r#type, TOKEN_TYPE_SYMBOL);
    assert_eq!(t1.cval, Some('{'));
    let t2 = read_next_token(&mut lp).unwrap();
    assert_eq!(t2.r#type, TOKEN_TYPE_SYMBOL);
    assert_eq!(t2.cval, Some('}'));
}

#[test]
fn test_lex_newline() {
    let mut lp = create_lex_for("\n$", "nl");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_NEWLINE);
}

#[test]
fn test_lex_one_line_comment() {
    let mut lp = create_lex_for("//abc\n42$", "olc");
    let t1 = read_next_token(&mut lp).expect("c1");
    assert_eq!(t1.r#type, TOKEN_TYPE_COMMENT);
    let t2 = read_next_token(&mut lp).expect("nl");
    assert_eq!(t2.r#type, TOKEN_TYPE_NEWLINE);
    let t3 = read_next_token(&mut lp).expect("num");
    assert_eq!(t3.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t3.llnum, Some(42));
}

#[test]
fn test_lex_multi_line_comment() {
    let mut lp = create_lex_for("/*abc*/\n42$", "mlc");
    let t1 = read_next_token(&mut lp).expect("c1");
    assert_eq!(t1.r#type, TOKEN_TYPE_COMMENT);
    let t2 = read_next_token(&mut lp).expect("nl");
    assert_eq!(t2.r#type, TOKEN_TYPE_NEWLINE);
    let t3 = read_next_token(&mut lp).expect("num");
    assert_eq!(t3.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t3.llnum, Some(42));
}

#[test]
fn test_lex_quoted_char() {
    let mut lp = create_lex_for("'a'$", "qa");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tok.cval, Some('a'));
}

#[test]
fn test_lex_escaped_quoted_newline() {
    let mut lp = create_lex_for("'\\n'$", "qn");
    let tok = read_next_token(&mut lp).expect("expected token");
    assert_eq!(tok.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(tok.cval, Some('\n'));
}

#[test]
fn test_lex_two_numbers_with_whitespace() {
    let mut lp = create_lex_for("123 456$", "twonum");
    let t1 = read_next_token(&mut lp).expect("first");
    assert_eq!(t1.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t1.llnum, Some(123));
    let t2 = read_next_token(&mut lp).expect("second");
    assert_eq!(t2.r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(t2.llnum, Some(456));
}

#[test]
fn test_lex_eof_dollar() {
    let mut lp = create_lex_for("$", "dol");
    let tok = read_next_token(&mut lp);
    assert!(tok.is_none());
}

#[test]
fn test_lex_full_returns_ok() {
    let mut lp = create_lex_for("42 hello$", "full");
    let res = lex(&mut lp);
    assert_eq!(res, LEXICAL_ANALYSIS_ALL_OK);
    // The token vec should contain 2 entries
    let v = lp.token_vec.as_ref().unwrap();
    assert_eq!(vector_count(v), 2);
}

#[test]
fn test_lex_full_two_numbers() {
    let mut lp = create_lex_for("100 200 300$", "three");
    let res = lex(&mut lp);
    assert_eq!(res, LEXICAL_ANALYSIS_ALL_OK);
    let v = lp.token_vec.as_ref().unwrap();
    assert_eq!(vector_count(v), 3);
}

#[test]
fn test_lex_full_with_comment() {
    let mut lp = create_lex_for("//hi\n42$", "comnl");
    let res = lex(&mut lp);
    assert_eq!(res, LEXICAL_ANALYSIS_ALL_OK);
    let v = lp.token_vec.as_ref().unwrap();
    // tokens: COMMENT, NEWLINE, NUMBER => 3
    assert_eq!(vector_count(v), 3);
}

#[test]
fn test_lex_function_pointers() {
    // Just check that COMPILER_LEX_FUNCTIONS is non-null/usable
    let funcs = COMPILER_LEX_FUNCTIONS;
    let _ = funcs.next_char;
    let _ = funcs.peek_char;
    let _ = funcs.push_char;
}

fn main() {}

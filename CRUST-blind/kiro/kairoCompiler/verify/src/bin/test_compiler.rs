use kairoCompiler::compiler::{
    CompileProcess, compile_file, compile_process_create,
    COMPILER_FILE_COMPILED_OK, COMPILER_FAILED_WITH_ERRORS,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER,
    tokens_build_for_string, token_is_keyword, token_is_symbol,
    token_is_nl_or_comment_or_newline_separator,
    Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE,
    PARSE_ALL_OK,
};
use std::io::Write;

fn create_temp_file(name: &str, content: &str) -> String {
    let path = format!("/tmp/crust_test_compiler_{}", name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn test_compile_file_valid() {
    let path = create_temp_file("valid.txt", "5467 abcd $");
    let out_path = create_temp_file("valid_out.txt", "");
    let result = compile_file(&path, &out_path, 0);
    assert_eq!(result, COMPILER_FILE_COMPILED_OK);
}

#[test]
fn test_compile_file_invalid() {
    let result = compile_file("/nonexistent/file.txt", "/tmp/out.txt", 0);
    assert_eq!(result, COMPILER_FAILED_WITH_ERRORS);
}

#[test]
fn test_compile_process_create_fn() {
    let path = create_temp_file("cp.txt", "test$");
    let cp = compile_process_create(&path, "", 0);
    assert!(cp.cfile.fp.is_some());
}

#[test]
fn test_compiler_tokens_build_for_string() {
    let cp = CompileProcess::default();
    let lp = tokens_build_for_string(cp, "42$");
    assert!(lp.token_vec.is_some());
}

#[test]
fn test_compiler_token_is_keyword() {
    // The compiler module's token_is_keyword is the "correct" version (uses ==)
    let t = Token {
        r#type: TOKEN_TYPE_KEYWORD,
        sval: Some("int".to_string()),
        ..Default::default()
    };
    assert!(token_is_keyword(&t, "int"));
}

#[test]
fn test_compiler_token_is_keyword_wrong() {
    let t = Token {
        r#type: TOKEN_TYPE_NUMBER,
        sval: Some("int".to_string()),
        ..Default::default()
    };
    assert!(!token_is_keyword(&t, "int"));
}

#[test]
fn test_compiler_token_is_symbol() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some('{'),
        ..Default::default()
    };
    assert!(token_is_symbol(&t, '{'));
    assert!(!token_is_symbol(&t, '}'));
}

#[test]
fn test_compiler_token_is_nl() {
    let t = Token { r#type: TOKEN_TYPE_NEWLINE, ..Default::default() };
    assert!(token_is_nl_or_comment_or_newline_separator(&t));
}

fn main() {}

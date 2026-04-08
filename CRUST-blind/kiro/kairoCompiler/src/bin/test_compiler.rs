use kairoCompiler::compiler::{
    CompileProcess, Pos, CompileProcessInputFile,
    COMPILER_FILE_COMPILED_OK, COMPILER_FAILED_WITH_ERRORS,
    LEXICAL_ANALYSIS_ALL_OK, LEXICAL_ANALYSIS_INPUT_ERROR,
    TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_OPERATOR,
    TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING,
    TOKEN_TYPE_COMMENT, TOKEN_TYPE_NEWLINE,
    NUMBER_TYPE_NORMAL, NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_DOUBLE,
    NODE_TYPE_EXPRESSION, NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
    NODE_TYPE_BLANK,
    PARSE_ALL_OK, PARSE_GENERAL_ERROR,
};

#[test]
fn test_compiler_constants_token_types() {
    assert_eq!(TOKEN_TYPE_IDENTIFIER, 0);
    assert_eq!(TOKEN_TYPE_KEYWORD, 1);
    assert_eq!(TOKEN_TYPE_OPERATOR, 2);
    assert_eq!(TOKEN_TYPE_SYMBOL, 3);
    assert_eq!(TOKEN_TYPE_NUMBER, 4);
    assert_eq!(TOKEN_TYPE_STRING, 5);
    assert_eq!(TOKEN_TYPE_COMMENT, 6);
    assert_eq!(TOKEN_TYPE_NEWLINE, 7);
}

#[test]
fn test_compiler_constants_number_types() {
    assert_eq!(NUMBER_TYPE_NORMAL, 0);
    assert_eq!(NUMBER_TYPE_LONG, 1);
    assert_eq!(NUMBER_TYPE_FLOAT, 2);
    assert_eq!(NUMBER_TYPE_DOUBLE, 3);
}

#[test]
fn test_compiler_constants_node_types() {
    assert_eq!(NODE_TYPE_EXPRESSION, 0);
    assert_eq!(NODE_TYPE_NUMBER, 2);
    assert_eq!(NODE_TYPE_IDENTIFIER, 3);
    assert_eq!(NODE_TYPE_STRING, 4);
    assert_eq!(NODE_TYPE_BLANK, 28);
}

#[test]
fn test_compiler_constants_results() {
    assert_eq!(COMPILER_FILE_COMPILED_OK, 0);
    assert_eq!(COMPILER_FAILED_WITH_ERRORS, 1);
    assert_eq!(LEXICAL_ANALYSIS_ALL_OK, 0);
    assert_eq!(LEXICAL_ANALYSIS_INPUT_ERROR, 1);
    assert_eq!(PARSE_ALL_OK, 0);
    assert_eq!(PARSE_GENERAL_ERROR, 1);
}

#[test]
fn test_compile_process_default() {
    let cp = CompileProcess::default();
    assert_eq!(cp.flags, 0);
    assert_eq!(cp.pos.line, 0);
    assert_eq!(cp.pos.col, 0);
}

#[test]
fn test_token_default() {
    let t = kairoCompiler::compiler::Token::default();
    assert_eq!(t.r#type, 0);
    assert_eq!(t.flags, 0);
    assert!(!t.whitespace);
    assert!(t.sval.is_none());
    assert!(t.cval.is_none());
}

fn main() {}

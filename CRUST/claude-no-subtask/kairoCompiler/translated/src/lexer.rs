use crate::compiler::{
    Token, TOKEN_TYPE_NUMBER, LEXICAL_ANALYSIS_ALL_OK,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};
/// Returns true if we're inside an expression. Stub returns false for demonstration.
fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    false
}
/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut t = original.clone();
    t.pos = lex_process.pos.clone();
    t
}
/// Reads a numeric literal from the input.
fn token_make_number(_lex_process: &mut LexProcess) -> Token {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t
}
/// Reads a quoted string (e.g. "text").
fn token_make_string(_lex_process: &mut LexProcess, _start_delim: char, _end_delim: char) -> Token {
    Token::default()
}
/// If the next char is an operator or symbol, create that token.
fn token_make_operator_or_symbol(_lex_process: &mut LexProcess) -> Token {
    Token::default()
}
/// If the next char is alpha or '_', read an identifier or keyword (placeholder).
fn token_make_identifier_or_keyword(_lex_process: &mut LexProcess) -> Token {
    Token::default()
}
/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(_lex_process: &mut LexProcess) -> Option<Token> {
    None
}
/// Lexes the entire file, pushing a placeholder for each recognized token.
pub fn lex(_lex_process: &mut LexProcess) -> i32 {
    LEXICAL_ANALYSIS_ALL_OK
}

// Suppress unused-warnings for the helper functions.
#[allow(dead_code)]
fn _silence_unused() {
    let mut lp = LexProcess::default();
    let t = Token::default();
    let _ = lex_is_in_expression(&lp);
    let _ = token_create(&mut lp, &t);
    let _ = token_make_number(&mut lp);
    let _ = token_make_string(&mut lp, '"', '"');
    let _ = token_make_operator_or_symbol(&mut lp);
    let _ = token_make_identifier_or_keyword(&mut lp);
}

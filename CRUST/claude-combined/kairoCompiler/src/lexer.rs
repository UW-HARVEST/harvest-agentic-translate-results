use crate::compiler::{
    Token, TokenNumber, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_NEWLINE,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_OPERATOR, TOKEN_TYPE_STRING, TOKEN_TYPE_SYMBOL,
    LEXICAL_ANALYSIS_ALL_OK, NUMBER_TYPE_FLOAT, NUMBER_TYPE_LONG, NUMBER_TYPE_NORMAL,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::vector::vector_push_token;
use std::sync::Mutex;
use lazy_static::lazy_static;

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

// Globals to mirror the static state used by the C lexer.
lazy_static! {
    static ref EXPR_COUNT: Mutex<i32> = Mutex::new(0);
}

/// Returns true if we're currently inside a parenthesised expression.
fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    *EXPR_COUNT.lock().unwrap() > 0
}

fn peekc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.expect("missing lex functions");
    (f.peek_char)(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.expect("missing lex functions");
    let c = (f.next_char)(lex_process);
    lex_process.pos.col += 1;
    if c == '\n' {
        lex_process.pos.line += 1;
        lex_process.pos.col = 1;
    }
    c
}

#[allow(dead_code)]
fn pushc(lex_process: &mut LexProcess, c: char) {
    let f = lex_process.function.expect("missing lex functions");
    (f.push_char)(lex_process, c);
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut t = original.clone();
    t.pos = lex_process.pos.clone();
    t
}

fn read_number_str(lex_process: &mut LexProcess) -> String {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while c >= '0' && c <= '9' {
        s.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    s
}

fn read_number(lex_process: &mut LexProcess) -> u64 {
    let s = read_number_str(lex_process);
    s.parse::<u64>().unwrap_or(0)
}

fn lexer_number_type(c: char) -> i32 {
    if c == 'L' {
        NUMBER_TYPE_LONG
    } else if c == 'f' {
        NUMBER_TYPE_FLOAT
    } else {
        NUMBER_TYPE_NORMAL
    }
}

fn token_make_number_for_value(lex_process: &mut LexProcess, value: u64) -> Token {
    let nt = lexer_number_type(peekc(lex_process));
    if nt != NUMBER_TYPE_NORMAL {
        nextc(lex_process);
    }
    let mut tmpl = Token::default();
    tmpl.r#type = TOKEN_TYPE_NUMBER;
    tmpl.llnum = Some(value);
    tmpl.num = TokenNumber { r#type: nt };
    token_create(lex_process, &tmpl)
}

/// Reads a numeric literal from the input.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let value = read_number(lex_process);
    token_make_number_for_value(lex_process, value)
}

/// Reads a quoted string (e.g. "text").
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    // Consume start delimiter.
    let first = nextc(lex_process);
    debug_assert_eq!(first, start_delim);
    let mut s = String::new();
    let mut c = nextc(lex_process);
    while c != end_delim && c != '\0' {
        if c == '\\' {
            // Skip the escape; the next char will be consumed normally.
            c = nextc(lex_process);
            continue;
        }
        s.push(c);
        c = nextc(lex_process);
    }
    let mut tmpl = Token::default();
    tmpl.r#type = TOKEN_TYPE_STRING;
    tmpl.sval = Some(s);
    token_create(lex_process, &tmpl)
}

fn is_symbol_char(c: char) -> bool {
    matches!(c, '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']')
}

fn token_make_symbol(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    if c == ')' {
        let mut count = EXPR_COUNT.lock().unwrap();
        *count -= 1;
    }
    let mut tmpl = Token::default();
    tmpl.r#type = TOKEN_TYPE_SYMBOL;
    tmpl.cval = Some(c);
    token_create(lex_process, &tmpl)
}

/// If the next char is an operator, create that token (simple single-char form).
fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    if c == '(' {
        let mut count = EXPR_COUNT.lock().unwrap();
        *count += 1;
    }
    let mut tmpl = Token::default();
    tmpl.r#type = TOKEN_TYPE_OPERATOR;
    tmpl.sval = Some(c.to_string());
    token_create(lex_process, &tmpl)
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "auto"
            | "break"
            | "case"
            | "char"
            | "const"
            | "continue"
            | "default"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "extern"
            | "float"
            | "for"
            | "goto"
            | "if"
            | "inline"
            | "int"
            | "long"
            | "register"
            | "restrict"
            | "return"
            | "short"
            | "signed"
            | "sizeof"
            | "static"
            | "struct"
            | "switch"
            | "typedef"
            | "union"
            | "unsigned"
            | "void"
            | "volatile"
            | "while"
            | "_Alignas"
            | "_Alignof"
            | "_Atomic"
            | "_Bool"
            | "_Complex"
            | "_Generic"
            | "_Imaginary"
            | "_Noreturn"
            | "_Static_assert"
            | "_Thread_local"
            | "__ignore_typecheck"
    )
}

/// If the next char is alpha or '_', read an identifier or keyword.
fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while (c >= 'a' && c <= 'z')
        || (c >= 'A' && c <= 'Z')
        || (c >= '0' && c <= '9')
        || c == '_'
    {
        s.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    let mut tmpl = Token::default();
    if is_keyword(&s) {
        tmpl.r#type = TOKEN_TYPE_KEYWORD;
    } else {
        tmpl.r#type = TOKEN_TYPE_IDENTIFIER;
    }
    tmpl.sval = Some(s);
    token_create(lex_process, &tmpl)
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(tv) = lex_process.token_vec.as_mut() {
        if let Some(last) = crate::vector::vector_back_token_mut(tv) {
            last.whitespace = true;
        }
    }
    nextc(lex_process);
    read_next_token(lex_process)
}

fn token_make_newline(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process);
    let mut tmpl = Token::default();
    tmpl.r#type = TOKEN_TYPE_NEWLINE;
    token_create(lex_process, &tmpl)
}

/// Reads the next token, returns Some(Token) or None on EOF / '$'.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    match c {
        '\0' => None,
        '$' => {
            // Stop signal -- consume so tests for arbitrary text can stop cleanly.
            nextc(lex_process);
            None
        }
        '0'..='9' => Some(token_make_number(lex_process)),
        c if is_symbol_char(c) => Some(token_make_symbol(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        c if c.is_ascii_alphabetic() || c == '_' => {
            Some(token_make_identifier_or_keyword(lex_process))
        }
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '['
        | ',' | '.' | '?' | '/' => Some(token_make_operator_or_symbol(lex_process)),
        _ => {
            // Unknown character; consume it to avoid infinite loops, but produce nothing.
            nextc(lex_process);
            None
        }
    }
}

/// Lexes the entire input, pushing each recognized token into the lex process token vector.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    *EXPR_COUNT.lock().unwrap() = 0;
    if let Some(c) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = c.cfile.abs_path.clone();
    }
    while let Some(token) = read_next_token(lex_process) {
        if let Some(tv) = lex_process.token_vec.as_mut() {
            vector_push_token(tv, token);
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

// Wrapper module: the real lexing logic now lives in `crate::compiler`,
// where it can share helpers and the `LexProcess` type. This module exposes
// the original public surface area expected by callers.

use crate::compiler::{
    compiler_error, CompileProcess, LexProcess, Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_NUMBER,
    TOKEN_TYPE_STRING, NUMBER_TYPE_FLOAT, NUMBER_TYPE_LONG, NUMBER_TYPE_NORMAL,
    LEXICAL_ANALYSIS_ALL_OK,
};
use crate::lex_process::LexProcessFunctions;
use crate::buffer::Buffer;
use crate::vector::{vector_pop, vector_push};

// Function pointer table that uses the real `LexProcess` type from `crate::compiler`.
// Note: this differs from `lex_process::LexProcessFunctions` (which uses
// `lex_process::LexProcess`). Both function pointer tables ultimately call into
// the same `compile_process_*` functions.
pub static COMPILER_LEX_FUNCTIONS: crate::compiler::LexProcessFunctions =
    crate::compiler::LexProcessFunctions {
        next_char: crate::compiler::compile_process_next_char,
        peek_char: crate::compiler::compile_process_peek_char,
        push_char: crate::compiler::compile_process_push_char,
    };

/// Returns true if we're inside an expression (i.e. inside parentheses).
fn lex_is_in_expression(lex_process: &LexProcess) -> bool {
    lex_process.current_expression_count > 0
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut t = original.clone();
    t.pos = lex_process.pos.clone();
    t
}

/// Reads a numeric literal from the input. Delegates to the compiler module.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    // Reuse the read logic via small helpers exposed on `LexProcess` by the
    // function-pointer table.
    let mut s = String::new();
    loop {
        let c = (lex_process.function.as_ref().unwrap().peek_char)(lex_process);
        if !(c >= '0' && c <= '9') {
            break;
        }
        s.push(c);
        (lex_process.function.as_ref().unwrap().next_char)(lex_process);
    }
    let value: u64 = s.parse().unwrap_or(0);
    let suffix = (lex_process.function.as_ref().unwrap().peek_char)(lex_process);
    let nt = match suffix {
        'L' => {
            (lex_process.function.as_ref().unwrap().next_char)(lex_process);
            NUMBER_TYPE_LONG
        }
        'f' => {
            (lex_process.function.as_ref().unwrap().next_char)(lex_process);
            NUMBER_TYPE_FLOAT
        }
        _ => NUMBER_TYPE_NORMAL,
    };
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.llnum = Some(value);
    t.num.r#type = nt;
    token_create(lex_process, &t)
}

/// Reads a quoted string (e.g. "text").
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let first = (lex_process.function.as_ref().unwrap().next_char)(lex_process);
    debug_assert_eq!(first, start_delim);
    let mut s = String::new();
    loop {
        let c = (lex_process.function.as_ref().unwrap().next_char)(lex_process);
        if c == end_delim || c == '\0' {
            break;
        }
        if c == '\\' {
            continue;
        }
        s.push(c);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_STRING;
    t.sval = Some(s);
    token_create(lex_process, &t)
}

/// If the next char is an operator or symbol, create that token.
fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let c = (lex_process.function.as_ref().unwrap().next_char)(lex_process);
    let mut s = c.to_string();
    let nxt = (lex_process.function.as_ref().unwrap().peek_char)(lex_process);
    let is_op2 = matches!(
        nxt,
        '+' | '-' | '*' | '/' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' | '!'
    );
    if is_op2 && c != '(' && c != '[' && c != ',' && c != '.' && c != '?' {
        s.push(nxt);
        (lex_process.function.as_ref().unwrap().next_char)(lex_process);
    }
    let mut t = Token::default();
    t.r#type = crate::compiler::TOKEN_TYPE_OPERATOR;
    t.sval = Some(s);
    token_create(lex_process, &t)
}

/// If the next char is alpha or '_', read an identifier or keyword (placeholder).
fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    loop {
        let c = (lex_process.function.as_ref().unwrap().peek_char)(lex_process);
        if !((c >= 'a' && c <= 'z')
            || (c >= 'A' && c <= 'Z')
            || (c >= '0' && c <= '9')
            || c == '_')
        {
            break;
        }
        s.push(c);
        (lex_process.function.as_ref().unwrap().next_char)(lex_process);
    }
    let mut t = Token::default();
    t.sval = Some(s.clone());
    t.r#type = if is_keyword_str(&s) {
        TOKEN_TYPE_KEYWORD
    } else {
        crate::compiler::TOKEN_TYPE_IDENTIFIER
    };
    token_create(lex_process, &t)
}

fn is_keyword_str(s: &str) -> bool {
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

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    loop {
        let c = (lex_process.function.as_ref().unwrap().peek_char)(lex_process);
        match c {
            '\0' | '$' => return None,
            ' ' | '\t' => {
                (lex_process.function.as_ref().unwrap().next_char)(lex_process);
                continue;
            }
            '\n' => {
                (lex_process.function.as_ref().unwrap().next_char)(lex_process);
                let mut t = Token::default();
                t.r#type = crate::compiler::TOKEN_TYPE_NEWLINE;
                return Some(token_create(lex_process, &t));
            }
            '"' => return Some(token_make_string(lex_process, '"', '"')),
            ch if ch >= '0' && ch <= '9' => return Some(token_make_number(lex_process)),
            ch if (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch == '_' => {
                return Some(token_make_identifier_or_keyword(lex_process));
            }
            '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '['
            | ',' | '.' | '?' | '/' => {
                return Some(token_make_operator_or_symbol(lex_process));
            }
            '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => {
                let cc = (lex_process.function.as_ref().unwrap().next_char)(lex_process);
                let mut t = Token::default();
                t.r#type = crate::compiler::TOKEN_TYPE_SYMBOL;
                t.cval = Some(cc);
                return Some(token_create(lex_process, &t));
            }
            _ => {
                (lex_process.function.as_ref().unwrap().next_char)(lex_process);
            }
        }
    }
}

/// Lexes the entire file by repeatedly calling `read_next_token`.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    crate::compiler::lex(lex_process)
}

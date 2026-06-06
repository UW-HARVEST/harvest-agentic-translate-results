use crate::compiler::{
    Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
    TokenNumber, Pos,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::buffer::{
    buffer_create, buffer_write, Buffer,
};
use std::sync::Mutex;
use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    /// Per-lex-process token storage, keyed by an identifier (e.g., the source file path).
    pub(crate) static ref TOKEN_STORAGE: Mutex<HashMap<String, Vec<Token>>> = Mutex::new(HashMap::new());
}

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

const EOF_CHAR: char = '\u{FFFF}';

/// Returns true if we're inside an expression. Stub returns false for demonstration.
fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    // We don't track current_expression_count in this safe Rust LexProcess struct (it's not
    // a field), so we conservatively report `false` like the original placeholder.
    false
}

fn peekc(lex_process: &mut LexProcess) -> char {
    if let Some(funcs) = lex_process.function {
        (funcs.peek_char)(lex_process)
    } else {
        EOF_CHAR
    }
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let c = if let Some(funcs) = lex_process.function {
        (funcs.next_char)(lex_process)
    } else {
        EOF_CHAR
    };
    lex_process.pos.col += 1;
    if c == '\n' {
        lex_process.pos.line += 1;
        lex_process.pos.col = 1;
    }
    c
}

fn pushc(lex_process: &mut LexProcess, c: char) {
    if let Some(funcs) = lex_process.function {
        (funcs.push_char)(lex_process, c);
    }
}

fn lex_file_position(lex_process: &LexProcess) -> Pos {
    lex_process.pos.clone()
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut t = original.clone();
    t.pos = lex_file_position(lex_process);
    t
}

fn read_number_str(lex_process: &mut LexProcess) -> String {
    let mut buf = String::new();
    let mut c = peekc(lex_process);
    while c >= '0' && c <= '9' {
        buf.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    buf
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

fn token_make_number_for_value(lex_process: &mut LexProcess, number: u64) -> Token {
    let nt = lexer_number_type(peekc(lex_process));
    if nt != NUMBER_TYPE_NORMAL {
        nextc(lex_process);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.llnum = Some(number);
    t.num = TokenNumber { r#type: nt };
    token_create(lex_process, &t)
}

/// Reads a numeric literal from the input.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let s = read_number_str(lex_process);
    let n: u64 = s.parse().unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

/// Reads a quoted string (e.g. "text").
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let s = nextc(lex_process);
    debug_assert_eq!(s, start_delim);
    let mut buf = String::new();
    let mut c = nextc(lex_process);
    while c != end_delim && c != EOF_CHAR {
        if c == '\\' {
            c = nextc(lex_process);
            continue;
        }
        buf.push(c);
        c = nextc(lex_process);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_STRING;
    t.sval = Some(buf);
    token_create(lex_process, &t)
}

fn op_treated_as_one(op: char) -> bool {
    op == '(' || op == '[' || op == ',' || op == '.' || op == '*' || op == '?'
}

fn is_single_operator(op: char) -> bool {
    matches!(
        op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' |
        '%' | '~' | '!' | '(' | '[' | ',' | '.' | '?'
    )
}

fn op_valid(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "/" | "!" | "^" | "+=" | "-=" | "*=" | "/=" |
        ">>" | "<<" | ">=" | "<=" | ">" | "<" | "||" | "&&" | "|" | "&" |
        "++" | "--" | "= " | "!=" | "==" | "->" | "(" | "[" | "," | "." |
        "..." | "~" | "?" | "%"
    )
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let op = nextc(lex_process);
    let mut s = String::new();
    s.push(op);

    if !op_treated_as_one(op) {
        let op2 = peekc(lex_process);
        if is_single_operator(op2) {
            s.push(op2);
            nextc(lex_process);
            single_operator = false;
        }
    }

    if !single_operator && !op_valid(&s) {
        // push back extras, keep first
        let chars: Vec<char> = s.chars().collect();
        for i in (1..chars.len()).rev() {
            pushc(lex_process, chars[i]);
        }
        s.truncate(1);
    }
    s
}

/// If the next char is an operator or symbol, create that token.
fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let op = read_op(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_OPERATOR;
    t.sval = Some(op);
    token_create(lex_process, &t)
}

fn token_make_symbol(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some(c);
    token_create(lex_process, &t)
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default" |
        "do" | "double" | "else" | "enum" | "extern" | "float" | "for" |
        "goto" | "if" | "inline" | "int" | "long" | "register" | "restrict" |
        "return" | "short" | "signed" | "sizeof" | "static" | "struct" |
        "switch" | "typedef" | "union" | "unsigned" | "void" | "volatile" |
        "while" | "_Alignas" | "_Alignof" | "_Atomic" | "_Bool" | "_Complex" |
        "_Generic" | "_Imaginary" | "_Noreturn" | "_Static_assert" |
        "_Thread_local" | "__ignore_typecheck"
    )
}

/// If the next char is alpha or '_', read an identifier or keyword (placeholder).
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
    let mut t = Token::default();
    if is_keyword(&s) {
        t.r#type = TOKEN_TYPE_KEYWORD;
    } else {
        t.r#type = TOKEN_TYPE_IDENTIFIER;
    }
    t.sval = Some(s);
    token_create(lex_process, &t)
}

fn token_make_newline(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NEWLINE;
    token_create(lex_process, &t)
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    let mut c = peekc(lex_process);
    while c != '\n' && c != EOF_CHAR {
        nextc(lex_process);
        c = peekc(lex_process);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    token_create(lex_process, &t)
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Token {
    let mut buf = String::new();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c != EOF_CHAR {
            buf.push(c);
            nextc(lex_process);
            c = peekc(lex_process);
        }
        if c == EOF_CHAR {
            // unterminated; bail out
            break;
        }
        // c == '*'
        nextc(lex_process);
        if peekc(lex_process) == '/' {
            nextc(lex_process);
            break;
        } else {
            buf.push('*');
        }
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    t.sval = Some(buf);
    token_create(lex_process, &t)
}

fn handle_comment(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c == '/' {
        nextc(lex_process);
        let p = peekc(lex_process);
        if p == '/' {
            nextc(lex_process);
            return Some(token_make_one_line_comment(lex_process));
        } else if p == '*' {
            nextc(lex_process);
            return Some(token_make_multiline_comment(lex_process));
        }
        pushc(lex_process, '/');
        return Some(token_make_operator_or_symbol(lex_process));
    }
    None
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    nextc(lex_process);
    read_next_token(lex_process)
}

fn is_numeric_char(c: char) -> bool {
    c >= '0' && c <= '9'
}

fn is_operator_char_excluding_division(c: char) -> bool {
    matches!(
        c,
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' |
        '|' | '&' | '(' | '[' | ',' | '.' | '?'
    )
}

fn is_symbol_char(c: char) -> bool {
    matches!(c, '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']')
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c == EOF_CHAR {
        return None;
    }

    if let Some(t) = handle_comment(lex_process) {
        return Some(t);
    }

    if is_numeric_char(c) {
        return Some(token_make_number(lex_process));
    }
    if is_operator_char_excluding_division(c) {
        return Some(token_make_operator_or_symbol(lex_process));
    }
    if is_symbol_char(c) {
        return Some(token_make_symbol(lex_process));
    }
    if c == '"' {
        return Some(token_make_string(lex_process, '"', '"'));
    }
    if c == ' ' || c == '\t' {
        return handle_whitespace(lex_process);
    }
    if c == '\n' {
        return Some(token_make_newline(lex_process));
    }
    if c == '$' {
        // EOF marker
        return None;
    }
    if c.is_alphabetic() || c == '_' {
        return Some(token_make_identifier_or_keyword(lex_process));
    }
    // unknown character: consume to avoid infinite loop
    nextc(lex_process);
    None
}

/// Lexes the entire file, pushing a placeholder for each recognized token.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    if let Some(comp) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = comp.cfile.abs_path.clone();
    }
    // Collect tokens into a Vec for the compiler's token_vec to use later.
    let mut tokens: Vec<Token> = Vec::new();
    loop {
        match read_next_token(lex_process) {
            Some(t) => tokens.push(t),
            None => break,
        }
    }
    // Store tokens by serializing into the token_vec via a side channel:
    // We attach a thread-local list of tokens keyed by lex_process address won't
    // work. Instead, we just stash them on the compiler via global state.
    TOKEN_STORAGE
        .lock()
        .unwrap()
        .insert(lex_process_id(lex_process), tokens);
    LEXICAL_ANALYSIS_ALL_OK
}

/// A simple identifier for a lex_process based on its compiler's filename.
pub(crate) fn lex_process_id(lex_process: &LexProcess) -> String {
    lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone())
        .unwrap_or_else(|| String::from("<anon>"))
}

// silence unused warnings for helpers that we keep around
#[allow(dead_code)]
fn _silence(_b: Buffer) {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'x');
}

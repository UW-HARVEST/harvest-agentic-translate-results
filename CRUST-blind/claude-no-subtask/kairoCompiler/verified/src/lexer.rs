use crate::compiler::{
    Token, TokenNumber, Pos,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::buffer::{buffer_create, buffer_write, Buffer};
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    /// Global storage for all created tokens. Vector stores u64 indices.
    pub(crate) static ref TOKENS: Mutex<Vec<Token>> = Mutex::new(Vec::new());
}

pub(crate) fn store_token(token: Token) -> u64 {
    let mut tokens = TOKENS.lock().unwrap();
    let idx = tokens.len() as u64;
    tokens.push(token);
    idx
}

pub(crate) fn get_token(idx: u64) -> Option<Token> {
    let tokens = TOKENS.lock().unwrap();
    tokens.get(idx as usize).cloned()
}

pub(crate) fn set_token(idx: u64, token: Token) {
    let mut tokens = TOKENS.lock().unwrap();
    if let Some(slot) = tokens.get_mut(idx as usize) {
        *slot = token;
    }
}

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

fn peekc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.as_ref().expect("no function").peek_char;
    f(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.as_ref().expect("no function").next_char;
    let c = f(lex_process);
    if lex_is_in_expression(lex_process) {
        if let Some(buf) = lex_process.parentheses_buffer.as_mut() {
            buffer_write(buf, c);
        }
    }
    lex_process.pos.col += 1;
    if c == '\n' {
        lex_process.pos.line += 1;
        lex_process.pos.col = 1;
    }
    c
}

fn pushc(lex_process: &mut LexProcess, c: char) {
    let f = lex_process.function.as_ref().expect("no function").push_char;
    f(lex_process, c);
}

fn lex_file_position(lex_process: &LexProcess) -> Pos {
    lex_process.pos.clone()
}

/// Returns true if we're inside an expression.
fn lex_is_in_expression(lex_process: &LexProcess) -> bool {
    lex_process.current_expression_count > 0
}

fn buffer_to_string(buf: &Buffer) -> String {
    let len = buf.len;
    let mut bytes = buf.data[..len.min(buf.data.len())].to_vec();
    while bytes.last() == Some(&0u8) {
        bytes.pop();
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut t = original.clone();
    t.pos = lex_file_position(lex_process);
    if lex_is_in_expression(lex_process) {
        if let Some(buf) = lex_process.parentheses_buffer.as_ref() {
            t.between_brackets = Some(buffer_to_string(buf));
        }
    }
    t
}

fn read_number_str(lex_process: &mut LexProcess) -> String {
    let mut buf = buffer_create();
    loop {
        let c = peekc(lex_process);
        if c >= '0' && c <= '9' {
            buffer_write(&mut buf, c);
            nextc(lex_process);
        } else {
            break;
        }
    }
    buffer_to_string(&buf)
}

fn read_number(lex_process: &mut LexProcess) -> u64 {
    let s = read_number_str(lex_process);
    s.parse::<u64>().unwrap_or(0)
}

fn lexer_number_type(c: char) -> i32 {
    match c {
        'L' => NUMBER_TYPE_LONG,
        'f' => NUMBER_TYPE_FLOAT,
        _ => NUMBER_TYPE_NORMAL,
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

fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let n = read_number(lex_process);
    token_make_number_for_value(lex_process, n)
}

fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let mut buf = buffer_create();
    let first = nextc(lex_process);
    debug_assert!(first == start_delim);
    let mut c = nextc(lex_process);
    while c != end_delim && c != '\0' {
        if c == '\\' {
            c = nextc(lex_process);
            continue;
        }
        buffer_write(&mut buf, c);
        c = nextc(lex_process);
    }
    let s = buffer_to_string(&buf);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_STRING;
    t.sval = Some(s);
    token_create(lex_process, &t)
}

fn op_treated_as_one(op: char) -> bool {
    matches!(op, '(' | '[' | ',' | '.' | '*' | '?')
}

fn is_single_operator(op: char) -> bool {
    matches!(
        op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' | '!'
            | '(' | '[' | ',' | '.' | '?'
    )
}

fn op_valid(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "/" | "!" | "^"
            | "+=" | "-=" | "*=" | "/="
            | ">>" | "<<" | ">=" | "<="
            | ">" | "<" | "||" | "&&" | "|" | "&" | "++" | "--"
            | "= " | "!=" | "==" | "->"
            | "(" | "[" | "," | "." | "..." | "~" | "?" | "%"
    )
}

fn read_op_flush_back_keep_first(lex_process: &mut LexProcess, buffer: &Buffer) {
    let len = buffer.len;
    if len < 2 {
        return;
    }
    for i in (1..len).rev() {
        let c = buffer.data[i];
        if c == 0 {
            continue;
        }
        pushc(lex_process, c as char);
    }
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let op = nextc(lex_process);
    let mut buf = buffer_create();
    buffer_write(&mut buf, op);

    if !op_treated_as_one(op) {
        let next_op = peekc(lex_process);
        if is_single_operator(next_op) {
            buffer_write(&mut buf, next_op);
            nextc(lex_process);
            single_operator = false;
        }
    }
    let s = buffer_to_string(&buf);
    if !single_operator && !op_valid(&s) {
        read_op_flush_back_keep_first(lex_process, &buf);
        return s.chars().next().map(|c| c.to_string()).unwrap_or_default();
    }
    s
}

fn lex_new_expression(lex_process: &mut LexProcess) {
    lex_process.current_expression_count += 1;
    if lex_process.current_expression_count == 1 {
        lex_process.parentheses_buffer = Some(buffer_create());
    }
}

fn lex_finish_expression(lex_process: &mut LexProcess) {
    lex_process.current_expression_count -= 1;
    if lex_process.current_expression_count < 0 {
        lex_process.current_expression_count = 0;
    }
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default"
            | "do" | "double" | "else" | "enum" | "extern" | "float" | "for"
            | "goto" | "if" | "inline" | "int" | "long" | "register" | "restrict"
            | "return" | "short" | "signed" | "sizeof" | "static" | "struct"
            | "switch" | "typedef" | "union" | "unsigned" | "void" | "volatile"
            | "while" | "_Alignas" | "_Alignof" | "_Atomic" | "_Bool" | "_Complex"
            | "_Generic" | "_Imaginary" | "_Noreturn" | "_Static_assert"
            | "_Thread_local" | "__ignore_typecheck"
    )
}

/// Returns the last token (a clone), if any.
fn lexer_last_token(lex_process: &mut LexProcess) -> Option<Token> {
    let vec = lex_process.token_vec.as_mut()?;
    let bytes = crate::vector::vector_back_or_null(vec)?;
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    let idx = u64::from_le_bytes(arr);
    get_token(idx)
}

fn lexer_last_token_is_keyword(lex_process: &mut LexProcess, value: &str) -> bool {
    match lexer_last_token(lex_process) {
        Some(mut t) => crate::token::token_is_keyword(&mut t, value),
        None => false,
    }
}

fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let op = peekc(lex_process);
    if op == '<' && lexer_last_token_is_keyword(lex_process, "include") {
        return token_make_string(lex_process, '<', '>');
    }
    let s = read_op(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_OPERATOR;
    t.sval = Some(s);
    let token = token_create(lex_process, &t);
    if op == '(' {
        lex_new_expression(lex_process);
    }
    token
}

fn token_make_symbol(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    if c == ')' {
        lex_finish_expression(lex_process);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some(c);
    token_create(lex_process, &t)
}

fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut buf = buffer_create();
    loop {
        let c = peekc(lex_process);
        if (c >= 'a' && c <= 'z')
            || (c >= 'A' && c <= 'Z')
            || (c >= '0' && c <= '9')
            || c == '_'
        {
            buffer_write(&mut buf, c);
            nextc(lex_process);
        } else {
            break;
        }
    }
    let s = buffer_to_string(&buf);
    let mut t = Token::default();
    if is_keyword(&s) {
        t.r#type = TOKEN_TYPE_KEYWORD;
    } else {
        t.r#type = TOKEN_TYPE_IDENTIFIER;
    }
    t.sval = Some(s);
    token_create(lex_process, &t)
}

fn read_special_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c.is_ascii_alphabetic() || c == '_' {
        return Some(token_make_identifier_or_keyword(lex_process));
    }
    None
}

fn token_make_newline(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NEWLINE;
    token_create(lex_process, &t)
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    let mut buf = buffer_create();
    loop {
        let c = peekc(lex_process);
        if c != '\n' && c != '\0' {
            buffer_write(&mut buf, c);
            nextc(lex_process);
        } else {
            break;
        }
    }
    let _ = buf;
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    token_create(lex_process, &t)
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Token {
    let mut buf = buffer_create();
    loop {
        loop {
            let c = peekc(lex_process);
            if c != '*' && c != '\0' {
                buffer_write(&mut buf, c);
                nextc(lex_process);
            } else {
                break;
            }
        }
        let c = peekc(lex_process);
        if c == '\0' {
            break;
        } else if c == '*' {
            nextc(lex_process);
            if peekc(lex_process) == '/' {
                nextc(lex_process);
                break;
            }
        }
    }
    let s = buffer_to_string(&buf);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    t.sval = Some(s);
    token_create(lex_process, &t)
}

fn handle_comment(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c == '/' {
        nextc(lex_process);
        if peekc(lex_process) == '/' {
            nextc(lex_process);
            return Some(token_make_one_line_comment(lex_process));
        } else if peekc(lex_process) == '*' {
            nextc(lex_process);
            return Some(token_make_multiline_comment(lex_process));
        }
        pushc(lex_process, '/');
        return Some(token_make_operator_or_symbol(lex_process));
    }
    None
}

fn lex_get_escaped_char(c: char) -> char {
    match c {
        'n' => '\n',
        '\\' => '\\',
        't' => '\t',
        'b' => '\u{0008}',
        '\'' => '\'',
        _ => '\0',
    }
}

fn is_hex_char(c: char) -> bool {
    let lc = c.to_ascii_lowercase();
    (lc >= '0' && lc <= '9') || (lc >= 'a' && lc <= 'f')
}

fn read_hex_number_str(lex_process: &mut LexProcess) -> String {
    let mut buf = buffer_create();
    loop {
        let c = peekc(lex_process);
        if is_hex_char(c) {
            buffer_write(&mut buf, c);
            nextc(lex_process);
        } else {
            break;
        }
    }
    buffer_to_string(&buf)
}

fn token_make_special_number_hexadecimal(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process); // skip 'x'
    let s = read_hex_number_str(lex_process);
    let n = u64::from_str_radix(&s, 16).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn token_make_special_number_binary(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process); // skip 'b'
    let s = read_number_str(lex_process);
    let n = u64::from_str_radix(&s, 2).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn lexer_pop_token(lex_process: &mut LexProcess) {
    if let Some(vec) = lex_process.token_vec.as_mut() {
        if !crate::vector::vector_empty(vec) {
            crate::vector::vector_pop(vec);
        }
    }
}

fn token_make_special_number(lex_process: &mut LexProcess) -> Option<Token> {
    let last_token = lexer_last_token(lex_process);
    let is_zero_num = match &last_token {
        Some(t) => t.r#type == TOKEN_TYPE_NUMBER && t.llnum == Some(0),
        None => false,
    };
    if !is_zero_num {
        return Some(token_make_identifier_or_keyword(lex_process));
    }
    lexer_pop_token(lex_process);
    let c = peekc(lex_process);
    if c == 'x' {
        Some(token_make_special_number_hexadecimal(lex_process))
    } else if c == 'b' {
        Some(token_make_special_number_binary(lex_process))
    } else {
        None
    }
}

fn token_make_quote(lex_process: &mut LexProcess) -> Token {
    let _open = nextc(lex_process);
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = nextc(lex_process);
        c = lex_get_escaped_char(c);
    }
    let _close = nextc(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.cval = Some(c);
    token_create(lex_process, &t)
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    // Mark previous token as having trailing whitespace.
    if let Some(mut last) = lexer_last_token(lex_process) {
        last.whitespace = true;
        // Update the storage at the same index.
        if let Some(vec) = lex_process.token_vec.as_mut() {
            if let Some(bytes) = crate::vector::vector_back_or_null(vec) {
                if bytes.len() >= 8 {
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(&bytes[..8]);
                    let idx = u64::from_le_bytes(arr);
                    set_token(idx, last);
                }
            }
        }
    }
    nextc(lex_process);
    read_next_token(lex_process)
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(t) = handle_comment(lex_process) {
        return Some(t);
    }
    let c = peekc(lex_process);
    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '['
        | ',' | '.' | '?' => Some(token_make_operator_or_symbol(lex_process)),
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => Some(token_make_symbol(lex_process)),
        'b' | 'x' => token_make_special_number(lex_process),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '\0' | '$' => None,
        _ => read_special_token(lex_process),
    }
}

/// Lexes the entire file, pushing token indices.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    lex_process.current_expression_count = 0;
    lex_process.parentheses_buffer = None;
    if let Some(c) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = c.cfile.abs_path.clone();
    }

    loop {
        let token_opt = read_next_token(lex_process);
        let token = match token_opt {
            Some(t) => t,
            None => break,
        };
        let idx = store_token(token);
        if let Some(vec) = lex_process.token_vec.as_mut() {
            let bytes = idx.to_le_bytes();
            crate::vector::vector_push(vec, &bytes);
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

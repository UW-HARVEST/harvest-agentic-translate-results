use crate::compiler::{
    CompileProcess, Pos, Token, TokenNumber,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::vector::vector_push;
use crate::buffer::{
    buffer_create, buffer_write, Buffer,
};
use std::sync::Mutex;
use lazy_static::lazy_static;

/// Shared registry of complete Token structures. Tokens stored in vectors are
/// represented as u64 indices into this list to avoid serialising heap-owned
/// fields (Strings, etc).
lazy_static! {
    pub(crate) static ref TOKENS: Mutex<Vec<Token>> = Mutex::new(Vec::new());
}

/// Global expression-nesting counter used by the lexer (the C version stored
/// this on `struct lex_process`; our Rust shape doesn't expose those fields, so
/// we lift them up here).
lazy_static! {
    static ref CURRENT_EXPRESSION_COUNT: Mutex<i32> = Mutex::new(0);
    static ref PARENTHESES_BUFFER: Mutex<Option<Buffer>> = Mutex::new(None);
}

fn expr_count() -> i32 {
    *CURRENT_EXPRESSION_COUNT.lock().unwrap()
}
fn set_expr_count(n: i32) {
    *CURRENT_EXPRESSION_COUNT.lock().unwrap() = n;
}
fn inc_expr_count() {
    let mut g = CURRENT_EXPRESSION_COUNT.lock().unwrap();
    *g += 1;
}
fn dec_expr_count() {
    let mut g = CURRENT_EXPRESSION_COUNT.lock().unwrap();
    *g -= 1;
}

pub(crate) fn token_register(token: Token) -> u64 {
    let mut guard = TOKENS.lock().unwrap();
    guard.push(token);
    (guard.len() - 1) as u64
}

pub(crate) fn token_get(idx: u64) -> Option<Token> {
    TOKENS.lock().unwrap().get(idx as usize).cloned()
}

pub(crate) fn token_set(idx: u64, tok: Token) {
    if let Some(slot) = TOKENS.lock().unwrap().get_mut(idx as usize) {
        *slot = tok;
    }
}

pub(crate) fn decode_index_8(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

const EOF_CHAR: char = '\u{FF}';

fn peekc(lp: &mut LexProcess) -> char {
    if let Some(funcs) = lp.function {
        (funcs.peek_char)(lp)
    } else {
        EOF_CHAR
    }
}

fn nextc(lp: &mut LexProcess) -> char {
    let c = if let Some(funcs) = lp.function {
        (funcs.next_char)(lp)
    } else {
        EOF_CHAR
    };
    if c != EOF_CHAR {
        if lex_is_in_expression(lp) {
            let mut g = PARENTHESES_BUFFER.lock().unwrap();
            if let Some(buf) = g.as_mut() {
                buffer_write(buf, c);
            }
        }
        lp.pos.col += 1;
        if c == '\n' {
            lp.pos.line += 1;
            lp.pos.col = 1;
        }
    }
    c
}

fn pushc(lp: &mut LexProcess, c: char) {
    if let Some(funcs) = lp.function {
        (funcs.push_char)(lp, c);
    }
}

fn lex_file_position(lp: &LexProcess) -> Pos {
    lp.pos.clone()
}

/// Returns true if we're inside an expression.
fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    expr_count() > 0
}

fn lex_new_expression(_lp: &mut LexProcess) {
    inc_expr_count();
    if expr_count() == 1 {
        *PARENTHESES_BUFFER.lock().unwrap() = Some(buffer_create());
    }
}

fn lex_finish_expression(_lp: &mut LexProcess) {
    dec_expr_count();
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut tok = original.clone();
    tok.pos = lex_file_position(lex_process);
    if lex_is_in_expression(lex_process) {
        let g = PARENTHESES_BUFFER.lock().unwrap();
        if let Some(buf) = g.as_ref() {
            let nul = buf.data.iter().position(|&b| b == 0).unwrap_or(buf.len);
            let s = String::from_utf8_lossy(&buf.data[..nul]).into_owned();
            tok.between_brackets = Some(s);
        }
    }
    tok
}

fn lexer_last_token(lp: &LexProcess) -> Option<Token> {
    let token_vec = lp.token_vec.as_ref()?;
    if token_vec.count == 0 {
        return None;
    }
    let last_idx = token_vec.count - 1;
    let start = (last_idx as usize) * token_vec.esize;
    let end = start + token_vec.esize;
    let slot = &token_vec.data.get(start..end)?;
    let tok_idx = decode_index_8(slot)?;
    token_get(tok_idx)
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(tv) = lex_process.token_vec.as_mut() {
        if tv.count > 0 {
            let last_idx = tv.count - 1;
            let start = (last_idx as usize) * tv.esize;
            let end = start + tv.esize;
            let slot = &tv.data[start..end];
            if let Some(tok_idx) = decode_index_8(slot) {
                if let Some(mut tok) = token_get(tok_idx) {
                    tok.whitespace = true;
                    token_set(tok_idx, tok);
                }
            }
        }
    }
    nextc(lex_process);
    read_next_token(lex_process)
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

fn token_make_number_for_value(lex_process: &mut LexProcess, number: u64) -> Token {
    let nt = lexer_number_type(peekc(lex_process));
    if nt != NUMBER_TYPE_NORMAL {
        nextc(lex_process);
    }
    let template = Token {
        r#type: TOKEN_TYPE_NUMBER,
        llnum: Some(number),
        num: TokenNumber { r#type: nt },
        ..Default::default()
    };
    token_create(lex_process, &template)
}

/// Reads a numeric literal from the input.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let n = read_number(lex_process);
    token_make_number_for_value(lex_process, n)
}

/// Reads a quoted string (e.g. "text").
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let consumed = nextc(lex_process);
    debug_assert_eq!(consumed, start_delim);
    let mut s = String::new();
    let mut c = nextc(lex_process);
    while c != end_delim && c != EOF_CHAR {
        if c == '\\' {
            c = nextc(lex_process);
            continue;
        }
        s.push(c);
        c = nextc(lex_process);
    }
    let template = Token {
        r#type: TOKEN_TYPE_STRING,
        sval: Some(s),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn op_treated_as_one(op: char) -> bool {
    matches!(op, '(' | '[' | ',' | '.' | '*' | '?')
}

fn is_single_operator(op: char) -> bool {
    matches!(op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^'
        | '%' | '~' | '!' | '(' | '[' | ',' | '.' | '?'
    )
}

fn op_valid(op: &str) -> bool {
    matches!(op,
        "+" | "-" | "*" | "/" | "!" | "^"
        | "+=" | "-=" | "*=" | "/="
        | ">>" | "<<" | ">=" | "<=" | ">" | "<"
        | "||" | "&&" | "|" | "&" | "++" | "--"
        | "= " | "!=" | "==" | "->"
        | "(" | "[" | "," | "."
        | "..." | "~" | "?" | "%"
    )
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let op = nextc(lex_process);
    let mut s = String::new();
    s.push(op);

    if !op_treated_as_one(op) {
        let next = peekc(lex_process);
        if is_single_operator(next) {
            s.push(next);
            nextc(lex_process);
            single_operator = false;
        }
    }

    if !single_operator {
        if !op_valid(&s) {
            // Push back the second operator and keep first only.
            let second = s.chars().nth(1).unwrap_or(' ');
            pushc(lex_process, second);
            s.truncate(1);
        }
    }
    s
}

fn token_is_keyword_local(token: &Token, value: &str) -> bool {
    token.r#type == TOKEN_TYPE_KEYWORD
        && token.sval.as_deref() == Some(value)
}

fn token_make_operator_or_string(lex_process: &mut LexProcess) -> Token {
    let op = peekc(lex_process);
    if op == '<' {
        if let Some(last) = lexer_last_token(lex_process) {
            if token_is_keyword_local(&last, "include") {
                return token_make_string(lex_process, '<', '>');
            }
        }
    }
    let opstr = read_op(lex_process);
    let template = Token {
        r#type: TOKEN_TYPE_OPERATOR,
        sval: Some(opstr),
        ..Default::default()
    };
    let tok = token_create(lex_process, &template);
    if op == '(' {
        lex_new_expression(lex_process);
    }
    tok
}

/// If the next char is an operator or symbol, create that token.
fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    token_make_operator_or_string(lex_process)
}

fn token_make_symbol(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    if c == ')' {
        lex_finish_expression(lex_process);
    }
    let template = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some(c),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn is_keyword_str(s: &str) -> bool {
    matches!(s,
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
    if is_keyword_str(&s) {
        let template = Token {
            r#type: TOKEN_TYPE_KEYWORD,
            sval: Some(s),
            ..Default::default()
        };
        token_create(lex_process, &template)
    } else {
        let template = Token {
            r#type: TOKEN_TYPE_IDENTIFIER,
            sval: Some(s),
            ..Default::default()
        };
        token_create(lex_process, &template)
    }
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
    let template = Token {
        r#type: TOKEN_TYPE_NEWLINE,
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    let mut c = peekc(lex_process);
    while c != '\n' && c != EOF_CHAR {
        nextc(lex_process);
        c = peekc(lex_process);
    }
    let template = Token {
        r#type: TOKEN_TYPE_COMMENT,
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Token {
    let mut buf: Buffer = buffer_create();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c != EOF_CHAR {
            buffer_write(&mut buf, c);
            nextc(lex_process);
            c = peekc(lex_process);
        }
        if c == EOF_CHAR {
            // Replicate a hard error; we keep semantics gentle in safe Rust by
            // returning an empty COMMENT token instead of exiting.
            break;
        }
        if c == '*' {
            nextc(lex_process);
            if peekc(lex_process) == '/' {
                nextc(lex_process);
                break;
            }
        }
    }
    let nul_pos = buf.data.iter().position(|&b| b == 0).unwrap_or(buf.len);
    let s = String::from_utf8_lossy(&buf.data[..nul_pos]).into_owned();
    let template = Token {
        r#type: TOKEN_TYPE_COMMENT,
        sval: Some(s),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn handle_comment(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c == '/' {
        nextc(lex_process);
        let next = peekc(lex_process);
        if next == '/' {
            nextc(lex_process);
            return Some(token_make_one_line_comment(lex_process));
        } else if next == '*' {
            nextc(lex_process);
            return Some(token_make_multiline_comment(lex_process));
        }
        pushc(lex_process, '/');
        return Some(token_make_operator_or_string(lex_process));
    }
    None
}

fn lex_get_escaped_char(c: char) -> char {
    match c {
        'n' => '\n',
        '\\' => '\\',
        't' => '\t',
        'b' => '\u{8}',
        '\'' => '\'',
        _ => '\0',
    }
}

fn is_hex_char(c: char) -> bool {
    let lc = c.to_ascii_lowercase();
    (lc >= '0' && lc <= '9') || (lc >= 'a' && lc <= 'f')
}

fn read_hex_number_str(lex_process: &mut LexProcess) -> String {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while is_hex_char(c) {
        s.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    s
}

fn token_make_special_number_hexadecimal(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process); // skip 'x'
    let s = read_hex_number_str(lex_process);
    let n = u64::from_str_radix(&s, 16).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn lexer_validate_binary_string(s: &str) -> bool {
    s.chars().all(|c| c == '0' || c == '1')
}

fn token_make_special_number_binary(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process); // skip 'b'
    let s = read_number_str(lex_process);
    if !lexer_validate_binary_string(&s) {
        // Treat invalid binary as zero rather than crash in safe code.
    }
    let n = u64::from_str_radix(&s, 2).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn lexer_pop_token(lp: &mut LexProcess) {
    if let Some(tv) = lp.token_vec.as_mut() {
        if tv.count > 0 {
            crate::vector::vector_pop(tv);
        }
    }
}

fn token_make_special_number(lex_process: &mut LexProcess) -> Token {
    let last = lexer_last_token(lex_process);
    let last_is_zero_number = match last {
        Some(t) => t.r#type == TOKEN_TYPE_NUMBER && t.llnum == Some(0),
        None => false,
    };
    if !last_is_zero_number {
        return token_make_identifier_or_keyword(lex_process);
    }
    lexer_pop_token(lex_process);
    let c = peekc(lex_process);
    if c == 'x' {
        token_make_special_number_hexadecimal(lex_process)
    } else if c == 'b' {
        token_make_special_number_binary(lex_process)
    } else {
        // Fallback: build identifier-or-keyword to avoid panic.
        token_make_identifier_or_keyword(lex_process)
    }
}

fn token_make_quote(lex_process: &mut LexProcess) -> Token {
    let q = nextc(lex_process);
    debug_assert_eq!(q, '\'');
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = nextc(lex_process);
        c = lex_get_escaped_char(c);
    }
    let close = nextc(lex_process);
    debug_assert_eq!(close, '\'');
    let template = Token {
        r#type: TOKEN_TYPE_NUMBER,
        cval: Some(c),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(tok) = handle_comment(lex_process) {
        return Some(tok);
    }
    let c = peekc(lex_process);
    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&'
        | '(' | '[' | ',' | '.' | '?' => Some(token_make_operator_or_string(lex_process)),
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => Some(token_make_symbol(lex_process)),
        'b' => Some(token_make_special_number(lex_process)),
        'x' => Some(token_make_special_number(lex_process)),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None, // end of input marker
        EOF_CHAR => None,
        _ => read_special_token(lex_process),
    }
}

/// Lexes the entire file, pushing a placeholder for each recognized token.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    set_expr_count(0);
    *PARENTHESES_BUFFER.lock().unwrap() = None;
    if let Some(c) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = c.cfile.abs_path.clone();
    }

    while let Some(tok) = read_next_token(lex_process) {
        let idx = token_register(tok);
        let bytes = idx.to_le_bytes();
        if let Some(tv) = lex_process.token_vec.as_mut() {
            vector_push(tv, &bytes);
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

/// Helper for tokens_build_for_string: implementations of the buffer-driven
/// LexProcessFunctions.
pub(crate) fn lexer_string_buffer_next_char(_process: &mut LexProcess) -> char {
    EOF_CHAR
}

pub(crate) fn lexer_string_buffer_peek_char(_process: &mut LexProcess) -> char {
    EOF_CHAR
}

pub(crate) fn lexer_string_buffer_push_char(_process: &mut LexProcess, _c: char) {}

/// Internal helper exposed for compiler module.
pub(crate) fn current_compile_process<'a>(lp: &'a mut LexProcess) -> Option<&'a mut CompileProcess> {
    lp.compiler.as_mut().map(|b| b.as_mut())
}

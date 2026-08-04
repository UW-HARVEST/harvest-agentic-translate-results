use crate::compiler::{
    Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_OPERATOR, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_KEYWORD, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
    TokenNumber, Pos,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::vector::{vector_push, vector_pop, vector_back_or_null};
use crate::buffer::{
    buffer_create, buffer_write, Buffer,
};
use std::sync::Mutex;
use lazy_static::lazy_static;

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

lazy_static! {
    /// Global token store. Token vector entries are 8-byte little-endian indices into this store.
    pub static ref TOKENS: Mutex<Vec<Token>> = Mutex::new(Vec::new());
}

fn encode_index(idx: u64) -> [u8; 8] {
    idx.to_le_bytes()
}

fn decode_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

/// Push a token into the lex_process token_vec (and the global storage)
pub fn store_token(lex_process: &mut LexProcess, tok: Token) {
    let mut tokens = TOKENS.lock().unwrap();
    let idx = tokens.len() as u64;
    tokens.push(tok);
    drop(tokens);
    if let Some(v) = lex_process.token_vec.as_mut() {
        vector_push(v, &encode_index(idx));
    }
}

/// Returns the last stored token via the lex_process token vec.
fn lexer_last_token(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(v) = lex_process.token_vec.as_mut() {
        if let Some(bytes) = vector_back_or_null(v) {
            if let Some(idx) = decode_index(bytes) {
                let tokens = TOKENS.lock().unwrap();
                if let Some(t) = tokens.get(idx as usize) {
                    return Some(t.clone());
                }
            }
        }
    }
    None
}

/// Pop the most recently pushed token from token_vec.
fn lexer_pop_token(lex_process: &mut LexProcess) {
    if let Some(v) = lex_process.token_vec.as_mut() {
        vector_pop(v);
    }
}

fn peekc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.as_ref().expect("no function table").peek_char;
    f(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.as_ref().expect("no function table").next_char;
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
    let f = lex_process.function.as_ref().expect("no function table").push_char;
    f(lex_process, c);
}

fn assert_next_char(lex_process: &mut LexProcess, c: char) -> char {
    let next_c = nextc(lex_process);
    assert_eq!(c, next_c);
    next_c
}

fn lex_file_position(lex_process: &LexProcess) -> Pos {
    lex_process.pos.clone()
}

pub fn lex_is_in_expression(lex_process: &LexProcess) -> bool {
    lex_process.current_expression_count > 0
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
        // Equivalent to compiler_error -- we just panic on this in safe Rust
        eprintln!("You closed an expression that you never opened");
        std::process::exit(1);
    }
}

/// Create a token by cloning `original` and updating position.
pub fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut new_tok = original.clone();
    new_tok.pos = lex_file_position(lex_process);
    if lex_is_in_expression(lex_process) {
        if let Some(buf) = lex_process.parentheses_buffer.as_ref() {
            let bytes: Vec<u8> = buf.data[..buf.len].iter().cloned().collect();
            new_tok.between_brackets = Some(String::from_utf8_lossy(&bytes).into_owned());
        }
    }
    new_tok
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
    let c = peekc(lex_process);
    let number_type = lexer_number_type(c);
    if number_type != NUMBER_TYPE_NORMAL {
        nextc(lex_process);
    }
    let template = Token {
        r#type: TOKEN_TYPE_NUMBER,
        llnum: Some(number),
        num: TokenNumber { r#type: number_type },
        ..Default::default()
    };
    token_create(lex_process, &template)
}

/// Reads a numeric literal from the input.
pub fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let n = read_number(lex_process);
    token_make_number_for_value(lex_process, n)
}

/// Reads a quoted string (e.g. "text").
pub fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let mut buf = String::new();
    assert_next_char(lex_process, start_delim);
    let mut c = nextc(lex_process);
    while c != end_delim && c != '\u{FF}' {
        if c == '\\' {
            c = nextc(lex_process);
            continue;
        }
        buf.push(c);
        c = nextc(lex_process);
    }
    let template = Token {
        r#type: TOKEN_TYPE_STRING,
        sval: Some(buf),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn op_treated_as_one(op: char) -> bool {
    matches!(op, '(' | '[' | ',' | '.' | '*' | '?')
}

fn is_single_operator(op: char) -> bool {
    matches!(
        op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' | '!' | '(' | '[' | ',' | '.' | '?'
    )
}

fn op_valid(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "/" | "!" | "^" | "+=" | "-=" | "*=" | "/=" | ">>" | "<<" | ">=" | "<="
        | ">" | "<" | "||" | "&&" | "|" | "&" | "++" | "--" | "= " | "!=" | "==" | "->" | "(" | "["
        | "," | "." | "..." | "~" | "?" | "%"
    )
}

fn read_op_flush_back_keep_first(lex_process: &mut LexProcess, buf: &Buffer) {
    let data = &buf.data[..buf.len];
    let len = buf.len;
    if len < 2 {
        return;
    }
    for i in (1..len).rev() {
        let b = data[i];
        if b == 0 {
            continue;
        }
        pushc(lex_process, b as char);
    }
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let mut op = nextc(lex_process);
    let mut buffer = buffer_create();
    buffer_write(&mut buffer, op);

    if !op_treated_as_one(op) {
        op = peekc(lex_process);
        if is_single_operator(op) {
            buffer_write(&mut buffer, op);
            nextc(lex_process);
            single_operator = false;
        }
    }
    buffer_write(&mut buffer, '\0');
    let mut s: String = String::from_utf8_lossy(&buffer.data[..buffer.len.saturating_sub(1)]).into_owned();

    if !single_operator {
        if !op_valid(&s) {
            read_op_flush_back_keep_first(lex_process, &buffer);
            // keep only the first character
            s.truncate(1);
        }
    }
    // (else branch in C is incorrect: `else if (!op_valid)` -> always false; we match that)
    s
}

/// If the next char is an operator or symbol, create that token.
pub fn token_make_operator_or_string(lex_process: &mut LexProcess) -> Token {
    let op = peekc(lex_process);
    if op == '<' {
        if let Some(mut last_token) = lexer_last_token(lex_process) {
            if crate::token::token_is_keyword(&mut last_token, "include") {
                return token_make_string(lex_process, '<', '>');
            }
        }
    }
    let op_str = read_op(lex_process);
    let template = Token {
        r#type: TOKEN_TYPE_OPERATOR,
        sval: Some(op_str),
        ..Default::default()
    };
    let token = token_create(lex_process, &template);
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
    let template = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some(c),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default" | "do" | "double"
        | "else" | "enum" | "extern" | "float" | "for" | "goto" | "if" | "inline" | "int"
        | "long" | "register" | "restrict" | "return" | "short" | "signed" | "sizeof" | "static"
        | "struct" | "switch" | "typedef" | "union" | "unsigned" | "void" | "volatile" | "while"
        | "_Alignas" | "_Alignof" | "_Atomic" | "_Bool" | "_Complex" | "_Generic" | "_Imaginary"
        | "_Noreturn" | "_Static_assert" | "_Thread_local" | "__ignore_typecheck"
    )
}

/// If the next char is alpha or '_', read an identifier or keyword.
pub fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' {
        s.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    if is_keyword(&s) {
        let template = Token {
            r#type: TOKEN_TYPE_KEYWORD,
            sval: Some(s),
            ..Default::default()
        };
        return token_create(lex_process, &template);
    }
    let template = Token {
        r#type: TOKEN_TYPE_IDENTIFIER,
        sval: Some(s),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn read_special_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c.is_alphabetic() || c == '_' {
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
    while c != '\n' && c != '\u{FF}' {
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
    let mut buf = String::new();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c != '\u{FF}' {
            buf.push(c);
            nextc(lex_process);
            c = peekc(lex_process);
        }
        if c == '\u{FF}' {
            eprintln!("You did not close this multiline comment");
            std::process::exit(1);
        } else if c == '*' {
            nextc(lex_process);
            if peekc(lex_process) == '/' {
                nextc(lex_process);
                break;
            }
        }
    }
    let template = Token {
        r#type: TOKEN_TYPE_COMMENT,
        sval: Some(buf),
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
        'b' => '\u{08}',
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
    nextc(lex_process); // skip x
    let s = read_hex_number_str(lex_process);
    let n = u64::from_str_radix(&s, 16).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn lexer_validate_binary_string(s: &str) {
    for c in s.chars() {
        if c != '0' && c != '1' {
            eprintln!("Invalid Binary number");
            std::process::exit(1);
        }
    }
}

fn token_make_special_number_binary(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process); // skip b
    let s = read_number_str(lex_process);
    lexer_validate_binary_string(&s);
    let n = u64::from_str_radix(&s, 2).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn token_make_special_number(lex_process: &mut LexProcess) -> Option<Token> {
    let last_token = lexer_last_token(lex_process);
    // Replicate C bug: assignment, condition is always true if last_token exists.
    // But effect: if last_token is None OR (last_token's llnum is 0 after assignment), proceed special.
    // The C condition: `!last_token || !(last_token->type=TOKEN_TYPE_NUMBER && last_token->llnum == 0)`
    // After fixing the bug: if last_token is None or NOT (it's a number with value 0), do identifier.
    let is_number_zero = match &last_token {
        Some(t) => t.r#type == TOKEN_TYPE_NUMBER && t.llnum == Some(0),
        None => false,
    };
    if last_token.is_none() || !is_number_zero {
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
    assert_next_char(lex_process, '\'');
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = nextc(lex_process);
        c = lex_get_escaped_char(c);
    }
    if nextc(lex_process) != '\'' {
        eprintln!("You opened a quote ' but did not close it");
        std::process::exit(1);
    }
    let template = Token {
        r#type: TOKEN_TYPE_NUMBER,
        cval: Some(c),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    // Mark last token as having whitespace following
    if let Some(v) = lex_process.token_vec.as_mut() {
        if let Some(bytes) = vector_back_or_null(v) {
            if let Some(idx) = decode_index(bytes) {
                let mut tokens = TOKENS.lock().unwrap();
                if let Some(t) = tokens.get_mut(idx as usize) {
                    t.whitespace = true;
                }
            }
        }
    }
    nextc(lex_process);
    read_next_token(lex_process)
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);

    if let Some(t) = handle_comment(lex_process) {
        return Some(t);
    }

    let token = match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '['
        | ',' | '.' | '?' => Some(token_make_operator_or_string(lex_process)),
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => Some(token_make_symbol(lex_process)),
        'b' => token_make_special_number(lex_process),
        'x' => token_make_special_number(lex_process),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => return handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None, // end of lexical analysis
        '\u{FF}' => None, // EOF
        _ => {
            let token = read_special_token(lex_process);
            if token.is_none() {
                eprintln!("Unexpected token");
                std::process::exit(1);
            }
            token
        }
    };

    token
}

/// Lexes the entire file, pushing a placeholder for each recognized token.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    lex_process.current_expression_count = 0;
    lex_process.parentheses_buffer = None;
    if let Some(comp) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = comp.cfile.abs_path.clone();
    }

    loop {
        match read_next_token(lex_process) {
            Some(tok) => {
                store_token(lex_process, tok);
            }
            None => break,
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

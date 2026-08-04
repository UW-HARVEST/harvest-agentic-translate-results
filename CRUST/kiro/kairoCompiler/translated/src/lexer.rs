use crate::compiler::{
compiler_error, CompileProcess, Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD,
NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_COMMENT,
TOKEN_TYPE_NEWLINE,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::vector::{vector_push, vector_pop};
use crate::buffer::{
buffer_create, buffer_write, buffer_ptr, Buffer,
buffer_read, buffer_peek as buf_peek, buffer_printf,
};
use std::sync::Mutex;
use lazy_static::lazy_static;

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
next_char: crate::cprocess::compile_process_next_char,
peek_char: crate::cprocess::compile_process_peek_char,
push_char: crate::cprocess::compile_process_push_char,
};

// Global state for lexer (since lex_process::LexProcess doesn't have expression fields)
lazy_static! {
    static ref CURRENT_EXPRESSION_COUNT: Mutex<i32> = Mutex::new(0);
    static ref PARENTHESES_BUFFER: Mutex<Option<Buffer>> = Mutex::new(None);
    static ref STRING_BUFFER: Mutex<Option<Buffer>> = Mutex::new(None);
    static ref TOKEN_STORE: Mutex<Vec<Token>> = Mutex::new(Vec::new());
}

fn store_token(token: &Token) -> u64 {
    let mut store = TOKEN_STORE.lock().unwrap();
    let idx = store.len() as u64;
    store.push(token.clone());
    idx
}

fn get_token(idx: u64) -> Token {
    let store = TOKEN_STORE.lock().unwrap();
    store[idx as usize].clone()
}

pub fn serde_token_encode(token: &Token) -> Vec<u8> {
    let idx = store_token(token);
    idx.to_le_bytes().to_vec()
}

pub fn serde_token_decode(bytes: &[u8]) -> Token {
    if bytes.len() < 8 {
        return Token::default();
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    let idx = u64::from_le_bytes(arr);
    get_token(idx)
}

fn update_token_in_bytes(bytes: &mut [u8], token: &Token) {
    if bytes.len() < 8 {
        return;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    let idx = u64::from_le_bytes(arr);
    let mut store = TOKEN_STORE.lock().unwrap();
    if (idx as usize) < store.len() {
        store[idx as usize] = token.clone();
    }
}

fn peekc(lex_process: &mut LexProcess) -> char {
    let func = lex_process.function.unwrap();
    (func.peek_char)(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let func = lex_process.function.unwrap();
    let c = (func.next_char)(lex_process);
    if lex_is_in_expression() {
        let mut pb = PARENTHESES_BUFFER.lock().unwrap();
        if let Some(ref mut buf) = *pb {
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
    let func = lex_process.function.unwrap();
    (func.push_char)(lex_process, c);
}

/// Returns true if we're inside an expression.
fn lex_is_in_expression() -> bool {
    *CURRENT_EXPRESSION_COUNT.lock().unwrap() > 0
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut tok = original.clone();
    tok.pos = lex_process.pos.clone();
    if lex_is_in_expression() {
        let pb = PARENTHESES_BUFFER.lock().unwrap();
        if let Some(ref buf) = *pb {
            let data = buffer_ptr(buf);
            tok.between_brackets = Some(String::from_utf8_lossy(data).to_string());
        }
    }
    tok
}

fn lexer_last_token(lex_process: &mut LexProcess) -> Option<Token> {
    let vec = lex_process.token_vec.as_mut()?;
    let bytes = crate::vector::vector_back_or_null(vec)?;
    Some(serde_token_decode(bytes))
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(ref mut vec) = lex_process.token_vec {
        if let Some(bytes) = crate::vector::vector_back_or_null(vec) {
            let mut tok = serde_token_decode(bytes);
            tok.whitespace = true;
            update_token_in_bytes(bytes, &tok);
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
    read_number_str(lex_process).parse::<u64>().unwrap_or(0)
}

fn lexer_number_type(c: char) -> i32 {
    match c {
        'L' => NUMBER_TYPE_LONG,
        'f' => NUMBER_TYPE_FLOAT,
        _ => NUMBER_TYPE_NORMAL,
    }
}

fn token_make_number_for_value(lex_process: &mut LexProcess, number: u64) -> Token {
    let num_type = lexer_number_type(peekc(lex_process));
    if num_type != NUMBER_TYPE_NORMAL {
        nextc(lex_process);
    }
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_NUMBER,
        llnum: Some(number),
        num: crate::compiler::TokenNumber { r#type: num_type },
        ..Token::default()
    })
}

/// Reads a numeric literal from the input.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let num = read_number(lex_process);
    token_make_number_for_value(lex_process, num)
}

/// Reads a quoted string (e.g. "text").
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let c = nextc(lex_process);
    assert_eq!(c, start_delim);
    let mut s = String::new();
    let mut c = nextc(lex_process);
    while c != end_delim && c != '\0' {
        if c == '\\' {
            c = nextc(lex_process);
            continue;
        }
        s.push(c);
        c = nextc(lex_process);
    }
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_STRING,
        sval: Some(s),
        ..Token::default()
    })
}

fn op_treated_as_one(op: char) -> bool {
    matches!(op, '(' | '[' | ',' | '.' | '*' | '?')
}

fn is_single_operator(op: char) -> bool {
    matches!(op, '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' | '!' | '(' | '[' | ',' | '.' | '?')
}

fn op_valid(op: &str) -> bool {
    matches!(op, "+" | "-" | "*" | "/" | "!" | "^" | "+=" | "-=" | "*=" | "/=" |
        ">>" | "<<" | ">=" | "<=" | ">" | "<" | "||" | "&&" | "|" | "&" |
        "++" | "--" | "= " | "!=" | "==" | "->" | "(" | "[" | "," | "." |
        "..." | "~" | "?" | "%")
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let op = nextc(lex_process);
    let mut result = String::new();
    result.push(op);

    if !op_treated_as_one(op) {
        let op2 = peekc(lex_process);
        if is_single_operator(op2) {
            result.push(op2);
            nextc(lex_process);
            single_operator = false;
        }
    }

    if !single_operator && !op_valid(&result) {
        let chars: Vec<char> = result.chars().collect();
        for i in (1..chars.len()).rev() {
            pushc(lex_process, chars[i]);
        }
        result.truncate(1);
    }
    result
}

fn lex_new_expression(lex_process: &mut LexProcess) {
    let mut count = CURRENT_EXPRESSION_COUNT.lock().unwrap();
    *count += 1;
    if *count == 1 {
        let mut pb = PARENTHESES_BUFFER.lock().unwrap();
        *pb = Some(buffer_create());
    }
}

fn lex_finish_expression(lex_process: &mut LexProcess) {
    let mut count = CURRENT_EXPRESSION_COUNT.lock().unwrap();
    *count -= 1;
    if *count < 0 {
        drop(count);
        let compiler = lex_process.compiler.as_mut().unwrap();
        compiler_error(compiler, "You closed an expression that you never opened\n");
    }
}

fn is_keyword(s: &str) -> bool {
    matches!(s, "auto" | "break" | "case" | "char" | "const" | "continue" | "default" |
        "do" | "double" | "else" | "enum" | "extern" | "float" | "for" | "goto" |
        "if" | "inline" | "int" | "long" | "register" | "restrict" | "return" |
        "short" | "signed" | "sizeof" | "static" | "struct" | "switch" | "typedef" |
        "union" | "unsigned" | "void" | "volatile" | "while" |
        "_Alignas" | "_Alignof" | "_Atomic" | "_Bool" | "_Complex" | "_Generic" |
        "_Imaginary" | "_Noreturn" | "_Static_assert" | "_Thread_local" | "__ignore_typecheck")
}

fn token_make_operator_or_string(lex_process: &mut LexProcess) -> Token {
    let op = peekc(lex_process);
    if op == '<' {
        if let Some(last) = lexer_last_token(lex_process) {
            if last.r#type == TOKEN_TYPE_KEYWORD {
                if let Some(ref sv) = last.sval {
                    if sv == "include" {
                        return token_make_string(lex_process, '<', '>');
                    }
                }
            }
        }
    }
    let op_str = read_op(lex_process);
    let token = token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_OPERATOR,
        sval: Some(op_str),
        ..Token::default()
    });
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
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some(c),
        ..Token::default()
    })
}

fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while c.is_ascii_alphanumeric() || c == '_' {
        s.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    if is_keyword(&s) {
        token_create(lex_process, &Token {
            r#type: TOKEN_TYPE_KEYWORD,
            sval: Some(s),
            ..Token::default()
        })
    } else {
        token_create(lex_process, &Token {
            r#type: TOKEN_TYPE_IDENTIFIER,
            sval: Some(s),
            ..Token::default()
        })
    }
}

fn token_make_newline(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process);
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_NEWLINE,
        ..Token::default()
    })
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    let mut c = peekc(lex_process);
    while c != '\n' && c != '\0' {
        nextc(lex_process);
        c = peekc(lex_process);
    }
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_COMMENT,
        ..Token::default()
    })
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Token {
    let mut comment = String::new();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c != '\0' {
            comment.push(c);
            nextc(lex_process);
            c = peekc(lex_process);
        }
        if c == '\0' {
            let compiler = lex_process.compiler.as_mut().unwrap();
            compiler_error(compiler, "You did not close this multiline comment\n");
        } else if c == '*' {
            nextc(lex_process);
            if peekc(lex_process) == '/' {
                nextc(lex_process);
                break;
            }
        }
    }
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_COMMENT,
        sval: Some(comment),
        ..Token::default()
    })
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
        'b' => '\x08',
        '\'' => '\'',
        _ => '\0',
    }
}

fn lexer_pop_token(lex_process: &mut LexProcess) {
    if let Some(ref mut vec) = lex_process.token_vec {
        vector_pop(vec);
    }
}

fn is_hex_char(c: char) -> bool {
    let c = c.to_ascii_lowercase();
    (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')
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
    nextc(lex_process);
    let number_str = read_hex_number_str(lex_process);
    let number = u64::from_str_radix(&number_str, 16).unwrap_or(0);
    token_make_number_for_value(lex_process, number)
}

fn lexer_validate_binary_string(lex_process: &mut LexProcess, s: &str) {
    for c in s.chars() {
        if c != '0' && c != '1' {
            let compiler = lex_process.compiler.as_mut().unwrap();
            compiler_error(compiler, "Invalid Binary number\n");
        }
    }
}

fn token_make_special_number_binary(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process);
    let number_str = read_number_str(lex_process);
    lexer_validate_binary_string(lex_process, &number_str);
    let number = u64::from_str_radix(&number_str, 2).unwrap_or(0);
    token_make_number_for_value(lex_process, number)
}

fn token_make_special_number(lex_process: &mut LexProcess) -> Token {
    let last = lexer_last_token(lex_process);
    let should_be_ident = match last {
        None => true,
        Some(ref t) => !(t.r#type == TOKEN_TYPE_NUMBER && t.llnum == Some(0)),
    };
    if should_be_ident {
        return token_make_identifier_or_keyword(lex_process);
    }
    lexer_pop_token(lex_process);
    let c = peekc(lex_process);
    if c == 'x' {
        token_make_special_number_hexadecimal(lex_process)
    } else if c == 'b' {
        token_make_special_number_binary(lex_process)
    } else {
        Token::default()
    }
}

fn token_make_quote(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    assert_eq!(c, '\'');
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = nextc(lex_process);
        c = lex_get_escaped_char(c);
    }
    let close = nextc(lex_process);
    if close != '\'' {
        let compiler = lex_process.compiler.as_mut().unwrap();
        compiler_error(compiler, "You opened a quote ' but did not close it\n");
    }
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_NUMBER,
        cval: Some(c),
        ..Token::default()
    })
}

/// If the next char is an operator or symbol, create that token.
fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    token_make_operator_or_string(lex_process)
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);

    if let Some(tok) = handle_comment(lex_process) {
        return Some(tok);
    }

    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '[' | ',' | '.' | '?' => {
            Some(token_make_operator_or_string(lex_process))
        }
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => {
            Some(token_make_symbol(lex_process))
        }
        'b' | 'x' => Some(token_make_special_number(lex_process)),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None,
        '\0' => None,
        _ => {
            if c.is_ascii_alphabetic() || c == '_' {
                Some(token_make_identifier_or_keyword(lex_process))
            } else {
                let compiler = lex_process.compiler.as_mut().unwrap();
                compiler_error(compiler, "Unexpected token\n");
                None
            }
        }
    }
}

/// Lexes the entire file, pushing tokens into the token vector.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    {
        let mut count = CURRENT_EXPRESSION_COUNT.lock().unwrap();
        *count = 0;
    }
    {
        let mut pb = PARENTHESES_BUFFER.lock().unwrap();
        *pb = None;
    }
    lex_process.pos.filename = lex_process.compiler.as_ref()
        .and_then(|c| c.cfile.abs_path.clone());

    while let Some(token) = read_next_token(lex_process) {
        let encoded = serde_token_encode(&token);
        if let Some(ref mut vec) = lex_process.token_vec {
            vector_push(vec, &encoded);
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

// String buffer lexer functions
fn lexer_string_buffer_next_char(_process: &mut LexProcess) -> char {
    let mut buf = STRING_BUFFER.lock().unwrap();
    if let Some(ref mut b) = *buf {
        buffer_read(b)
    } else {
        '\0'
    }
}

fn lexer_string_buffer_peek_char(_process: &mut LexProcess) -> char {
    let buf = STRING_BUFFER.lock().unwrap();
    if let Some(ref b) = *buf {
        buf_peek(b)
    } else {
        '\0'
    }
}

fn lexer_string_buffer_push_char(_process: &mut LexProcess, c: char) {
    let mut buf = STRING_BUFFER.lock().unwrap();
    if let Some(ref mut b) = *buf {
        if b.rindex > 0 {
            b.rindex -= 1;
        }
    }
}

pub static LEXER_STRING_BUFFER_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: lexer_string_buffer_next_char,
    peek_char: lexer_string_buffer_peek_char,
    push_char: lexer_string_buffer_push_char,
};

pub fn tokens_build_for_string(compiler: CompileProcess, s: &str) -> Option<LexProcess> {
    let mut buf = buffer_create();
    buffer_printf(&mut buf, s);
    {
        let mut global_buf = STRING_BUFFER.lock().unwrap();
        *global_buf = Some(buf);
    }
    let mut lp = crate::lex_process::lex_process_create(
        compiler,
        LEXER_STRING_BUFFER_FUNCTIONS,
        None,
    );
    if lex(&mut lp) != LEXICAL_ANALYSIS_ALL_OK {
        return None;
    }
    Some(lp)
}

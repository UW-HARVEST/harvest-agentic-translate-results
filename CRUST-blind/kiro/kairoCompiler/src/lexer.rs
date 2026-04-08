use crate::compiler::{
    compiler_error, CompileProcess, Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_COMMENT,
    TOKEN_TYPE_NEWLINE,
};
use crate::lex_process::{LexProcess, LexProcessFunctions, lex_process_create, lex_process_private};
use crate::vector::{vector_push, vector_pop, vector_back_or_null};
use crate::buffer::{
    buffer_create, buffer_write, buffer_ptr, buffer_free, buffer_printf, Buffer,
};
use crate::token::token_is_keyword;

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

fn peekc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.unwrap();
    (f.peek_char)(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.unwrap();
    let c = (f.next_char)(lex_process);
    if lex_is_in_expression(lex_process) {
        if let Some(ref mut buf) = lex_process.parentheses_buffer {
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
    let f = lex_process.function.unwrap();
    (f.push_char)(lex_process, c);
}

fn assert_next_char(lex_process: &mut LexProcess, c: char) -> char {
    let next_c = nextc(lex_process);
    assert_eq!(c, next_c);
    next_c
}

/// Returns true if we're inside an expression.
fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    _lex_process.current_expression_count > 0
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut token = original.clone();
    token.pos = lex_process.pos.clone();
    if lex_is_in_expression(lex_process) {
        if let Some(ref buf) = lex_process.parentheses_buffer {
            let bytes = buffer_ptr(buf);
            token.between_brackets = Some(String::from_utf8_lossy(bytes).to_string());
        }
    }
    token
}

fn lexer_last_token(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(ref mut tv) = lex_process.token_vec {
        if let Some(bytes) = vector_back_or_null(tv) {
            let token = deserialize_token(bytes);
            return Some(token);
        }
    }
    None
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    // Set whitespace on last token
    if let Some(ref mut tv) = lex_process.token_vec {
        if let Some(bytes) = vector_back_or_null(tv) {
            let mut token = deserialize_token(bytes);
            token.whitespace = true;
            let ser = serialize_token(&token);
            bytes[..ser.len()].copy_from_slice(&ser);
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
    if c == 'L' { NUMBER_TYPE_LONG }
    else if c == 'f' { NUMBER_TYPE_FLOAT }
    else { NUMBER_TYPE_NORMAL }
}

fn token_make_number_for_value(lex_process: &mut LexProcess, number: u64) -> Token {
    let number_type = lexer_number_type(peekc(lex_process));
    if number_type != NUMBER_TYPE_NORMAL {
        nextc(lex_process);
    }
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_NUMBER,
        llnum: Some(number),
        num: crate::compiler::TokenNumber { r#type: number_type },
        ..Token::default()
    })
}

/// Reads a numeric literal from the input.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let num = read_number(lex_process);
    token_make_number_for_value(lex_process, num)
}

/// Reads a quoted string.
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    assert_eq!(nextc(lex_process), start_delim);
    let mut s = String::new();
    let mut c = nextc(lex_process);
    while c != end_delim && c as u8 != 0xFF {
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
    matches!(op,
        "+" | "-" | "*" | "/" | "!" | "^" | "+=" | "-=" | "*=" | "/=" |
        ">>" | "<<" | ">=" | "<=" | ">" | "<" | "||" | "&&" | "|" | "&" |
        "++" | "--" | "= " | "!=" | "==" | "->" | "(" | "[" | "," | "." |
        "..." | "~" | "?" | "%"
    )
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let op1 = nextc(lex_process);
    let mut buf = String::new();
    buf.push(op1);

    if !op_treated_as_one(op1) {
        let op2 = peekc(lex_process);
        if is_single_operator(op2) {
            buf.push(op2);
            nextc(lex_process);
            single_operator = false;
        }
    }

    if !single_operator {
        if !op_valid(&buf) {
            // flush back all but first char
            let chars: Vec<char> = buf.chars().collect();
            for i in (1..chars.len()).rev() {
                pushc(lex_process, chars[i]);
            }
            buf.truncate(1);
        }
    }
    // Note: C code has `else if (!op_valid)` which is always false (function pointer is non-null)
    // so we skip that branch
    buf
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
        if let Some(ref mut compiler) = lex_process.compiler {
            compiler_error(compiler, "You closed an expression that you never opened\n");
        }
    }
}

fn is_keyword(s: &str) -> bool {
    matches!(s,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default" | "do" |
        "double" | "else" | "enum" | "extern" | "float" | "for" | "goto" | "if" |
        "inline" | "int" | "long" | "register" | "restrict" | "return" | "short" |
        "signed" | "sizeof" | "static" | "struct" | "switch" | "typedef" | "union" |
        "unsigned" | "void" | "volatile" | "while" | "_Alignas" | "_Alignof" | "_Atomic" |
        "_Bool" | "_Complex" | "_Generic" | "_Imaginary" | "_Noreturn" | "_Static_assert" |
        "_Thread_local" | "__ignore_typecheck"
    )
}

/// If the next char is an operator or symbol, create that token.
fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let op = peekc(lex_process);
    if op == '<' {
        // Check if last token is "include" keyword
        let is_include = {
            if let Some(ref mut tv) = lex_process.token_vec {
                if let Some(bytes) = vector_back_or_null(tv) {
                    let mut t = deserialize_token(bytes);
                    token_is_keyword(&mut t, "include")
                } else {
                    false
                }
            } else {
                false
            }
        };
        if is_include {
            return token_make_string(lex_process, '<', '>');
        }
    }
    let op_str = read_op(lex_process);
    let mut token = token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_OPERATOR,
        sval: Some(op_str),
        ..Token::default()
    });
    if op == '(' {
        lex_new_expression(lex_process);
    }
    token
}

fn token_make_symbol_token(lex_process: &mut LexProcess) -> Token {
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

/// If the next char is alpha or '_', read an identifier or keyword.
fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' {
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
    while c != '\n' && c as u8 != 0xFF {
        nextc(lex_process);
        c = peekc(lex_process);
    }
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_COMMENT,
        ..Token::default()
    })
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c as u8 != 0xFF {
            s.push(c);
            nextc(lex_process);
            c = peekc(lex_process);
        }
        if c as u8 == 0xFF {
            if let Some(ref mut compiler) = lex_process.compiler {
                compiler_error(compiler, "You did not close this multiline comment\n");
            }
            break;
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
        sval: Some(s),
        ..Token::default()
    })
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
        'b' => '\x08',
        '\'' => '\'',
        _ => '\0',
    }
}

fn lexer_pop_token(lex_process: &mut LexProcess) {
    if let Some(ref mut tv) = lex_process.token_vec {
        vector_pop(tv);
    }
}

fn is_hex_char(c: char) -> bool {
    let cl = c.to_ascii_lowercase();
    (cl >= '0' && cl <= '9') || (cl >= 'a' && cl <= 'f')
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
    let number_str = read_hex_number_str(lex_process);
    let number = u64::from_str_radix(&number_str, 16).unwrap_or(0);
    token_make_number_for_value(lex_process, number)
}

fn lexer_validate_binary_string(lex_process: &mut LexProcess, s: &str) {
    for c in s.chars() {
        if c != '0' && c != '1' {
            if let Some(ref mut compiler) = lex_process.compiler {
                compiler_error(compiler, "Invalid Binary number\n");
            }
        }
    }
}

fn token_make_special_number_binary(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process); // skip 'b'
    let number_str = read_number_str(lex_process);
    lexer_validate_binary_string(lex_process, &number_str);
    let number = u64::from_str_radix(&number_str, 2).unwrap_or(0);
    token_make_number_for_value(lex_process, number)
}

fn token_make_special_number(lex_process: &mut LexProcess) -> Option<Token> {
    // Check last token - replicate C bug: `last_token->type=TOKEN_TYPE_NUMBER`
    let should_process = {
        if let Some(ref mut tv) = lex_process.token_vec {
            if let Some(bytes) = vector_back_or_null(tv) {
                let mut t = deserialize_token(bytes);
                // C code: `!(last_token->type=TOKEN_TYPE_NUMBER && last_token->llnum == 0)`
                // This is a bug: it assigns TOKEN_TYPE_NUMBER to type, then checks && llnum==0
                let eq = t.r#type == TOKEN_TYPE_NUMBER && t.llnum == Some(0);
                t.r#type = if eq { 1 } else { 0 };
                // Write back modified token
                let ser = serialize_token(&t);
                bytes[..ser.len()].copy_from_slice(&ser);
                // The condition is `if(!last_token || !(result))` 
                // Since last_token exists, we check !(eq)
                !eq
            } else {
                true // no last token
            }
        } else {
            true
        }
    };
    if should_process {
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
        if let Some(ref mut compiler) = lex_process.compiler {
            compiler_error(compiler, "You opened a quote ' but did not close it\n");
        }
    }
    token_create(lex_process, &Token {
        r#type: TOKEN_TYPE_NUMBER,
        cval: Some(c),
        ..Token::default()
    })
}

fn read_special_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c.is_ascii_alphabetic() || c == '_' {
        return Some(token_make_identifier_or_keyword(lex_process));
    }
    None
}

// Token serialization helpers for storing in Vector
fn serialize_token(token: &Token) -> Vec<u8> {
    // We use a simple binary format that fits in the Token-sized vector element
    // For simplicity, we'll store the token as JSON bytes, but we need fixed size.
    // Actually, let's just store tokens in a side Vec and use indices, similar to nodes.
    // But the signatures require &[u8]. Let's use bincode-style manual serialization.
    // 
    // Simpler approach: store tokens in a thread-local Vec<Token> and put indices in the vector.
    let idx = {
        TOKENS.lock().unwrap().len()
    };
    TOKENS.lock().unwrap().push(token.clone());
    (idx as u64).to_le_bytes().to_vec()
}

fn deserialize_token(bytes: &[u8]) -> Token {
    if bytes.len() >= 8 {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes[..8]);
        let idx = u64::from_le_bytes(arr) as usize;
        let tokens = TOKENS.lock().unwrap();
        if idx < tokens.len() {
            return tokens[idx].clone();
        }
    }
    Token::default()
}

use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref TOKENS: Mutex<Vec<Token>> = Mutex::new(Vec::new());
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);

    // Handle comments first
    if let Some(token) = handle_comment(lex_process) {
        return Some(token);
    }

    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '[' | ',' | '.' | '?' => {
            Some(token_make_operator_or_symbol(lex_process))
        }
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => {
            Some(token_make_symbol_token(lex_process))
        }
        'b' | 'x' => token_make_special_number(lex_process),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None, // EOF marker
        _ => {
            let token = read_special_token(lex_process);
            if token.is_none() {
                if let Some(ref mut compiler) = lex_process.compiler {
                    compiler_error(compiler, "Unexpected token\n");
                }
            }
            token
        }
    }
}

/// Lexes the entire file.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    lex_process.current_expression_count = 0;
    lex_process.parentheses_buffer = None;
    if let Some(ref compiler) = lex_process.compiler {
        lex_process.pos.filename = compiler.cfile.abs_path.clone();
    }

    loop {
        let token = read_next_token(lex_process);
        match token {
            Some(t) => {
                let ser = serialize_token(&t);
                if let Some(ref mut tv) = lex_process.token_vec {
                    vector_push(tv, &ser);
                }
            }
            None => break,
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

// String buffer lexer functions for tokens_build_for_string
fn lexer_string_buffer_next_char(process: &mut LexProcess) -> char {
    // private data is a Buffer stored in LEXER_BUFFERS
    let idx = process.private_buffer_idx;
    let mut bufs = LEXER_BUFFERS.lock().unwrap();
    if let Some(buf) = bufs.get_mut(idx) {
        crate::buffer::buffer_read(buf)
    } else {
        0xFF as char
    }
}

fn lexer_string_buffer_peek_char(process: &mut LexProcess) -> char {
    let idx = process.private_buffer_idx;
    let bufs = LEXER_BUFFERS.lock().unwrap();
    if let Some(buf) = bufs.get(idx) {
        crate::buffer::buffer_peek(buf)
    } else {
        0xFF as char
    }
}

fn lexer_string_buffer_push_char(process: &mut LexProcess, c: char) {
    let idx = process.private_buffer_idx;
    let mut bufs = LEXER_BUFFERS.lock().unwrap();
    if let Some(buf) = bufs.get_mut(idx) {
        buffer_write(buf, c);
    }
}

lazy_static! {
    static ref LEXER_BUFFERS: Mutex<Vec<Buffer>> = Mutex::new(Vec::new());
}

static LEXER_STRING_BUFFER_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: lexer_string_buffer_next_char,
    peek_char: lexer_string_buffer_peek_char,
    push_char: lexer_string_buffer_push_char,
};

pub fn tokens_build_for_string(compiler: CompileProcess, s: &str) -> Option<LexProcess> {
    let mut buffer = buffer_create();
    buffer_printf(&mut buffer, s);
    let buf_idx = {
        let mut bufs = LEXER_BUFFERS.lock().unwrap();
        let idx = bufs.len();
        bufs.push(buffer);
        idx
    };
    let mut lp = lex_process_create(compiler, LEXER_STRING_BUFFER_FUNCTIONS, None);
    lp.private_buffer_idx = buf_idx;
    if lex(&mut lp) != LEXICAL_ANALYSIS_ALL_OK {
        return None;
    }
    Some(lp)
}

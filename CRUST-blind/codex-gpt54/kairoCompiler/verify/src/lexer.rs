use crate::buffer::{buffer_create, buffer_ptr, buffer_write};
use crate::compiler::{
    compiler_error, Token, TOKEN_TYPE_COMMENT, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_KEYWORD,
    TOKEN_TYPE_NEWLINE, TOKEN_TYPE_NUMBER, TOKEN_TYPE_OPERATOR, TOKEN_TYPE_STRING,
    TOKEN_TYPE_SYMBOL, LEXICAL_ANALYSIS_ALL_OK, NUMBER_TYPE_FLOAT, NUMBER_TYPE_LONG,
    NUMBER_TYPE_NORMAL,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::token::token_is_keyword;
use crate::vector::{vector_back_or_null, vector_pop, vector_push};
use lazy_static::lazy_static;
use std::sync::Mutex;

pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

lazy_static! {
    static ref TOKENS: Mutex<Vec<Token>> = Mutex::new(Vec::new());
}

fn encode_index(idx: u64) -> [u8; 8] {
    idx.to_le_bytes()
}

pub(crate) fn decode_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

pub(crate) fn store_token(token: Token) -> [u8; 8] {
    let mut tokens = TOKENS.lock().expect("token registry poisoned");
    tokens.push(token);
    encode_index(tokens.len() as u64 - 1)
}

pub(crate) fn get_token(bytes: &[u8]) -> Option<Token> {
    let index = decode_index(bytes)? as usize;
    let tokens = TOKENS.lock().ok()?;
    tokens.get(index).cloned()
}

fn peekc(lex_process: &mut LexProcess) -> char {
    lex_process
        .function
        .map(|f| (f.peek_char)(lex_process))
        .unwrap_or('\0')
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let c = lex_process
        .function
        .map(|f| (f.next_char)(lex_process))
        .unwrap_or('\0');

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
    if let Some(functions) = lex_process.function {
        (functions.push_char)(lex_process, c);
    }
}

fn assert_next_char(lex_process: &mut LexProcess, c: char) -> char {
    let next_c = nextc(lex_process);
    if next_c != c {
        if let Some(compiler) = lex_process.compiler.as_mut() {
            compiler_error(compiler, "Unexpected character");
        }
    }
    next_c
}

fn lex_is_in_expression(lex_process: &LexProcess) -> bool {
    lex_process.current_expression_count > 0
}

fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut token = original.clone();
    token.pos = lex_process.pos.clone();
    if lex_is_in_expression(lex_process) {
        if let Some(buf) = lex_process.parentheses_buffer.as_ref() {
            token.between_brackets = String::from_utf8(buffer_ptr(buf).to_vec()).ok();
        }
    }
    token
}

fn lexer_last_token(lex_process: &mut LexProcess) -> Option<Token> {
    let bytes = vector_back_or_null(lex_process.token_vec.as_mut()?)?.to_vec();
    get_token(&bytes)
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(last_bytes) = lex_process
        .token_vec
        .as_mut()
        .and_then(vector_back_or_null)
        .map(|slice| slice.to_vec())
    {
        if let Some(index) = decode_index(&last_bytes) {
            if let Ok(mut tokens) = TOKENS.lock() {
                if let Some(token) = tokens.get_mut(index as usize) {
                    token.whitespace = true;
                }
            }
        }
    }

    nextc(lex_process);
    read_next_token(lex_process)
}

fn read_number_str(lex_process: &mut LexProcess) -> String {
    let mut buffer = buffer_create();
    let mut c = peekc(lex_process);
    while c.is_ascii_digit() {
        buffer_write(&mut buffer, c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    buffer_write(&mut buffer, '\0');
    String::from_utf8_lossy(buffer_ptr(&buffer))
        .trim_end_matches('\0')
        .to_string()
}

fn read_number(lex_process: &mut LexProcess) -> u64 {
    read_number_str(lex_process).parse::<u64>().unwrap_or(0)
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
    let number_type = lexer_number_type(peekc(lex_process));
    if number_type != NUMBER_TYPE_NORMAL {
        nextc(lex_process);
    }

    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_NUMBER,
            llnum: Some(number),
            num: crate::compiler::TokenNumber { r#type: number_type },
            ..Token::default()
        },
    )
}

fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let number = read_number(lex_process);
    token_make_number_for_value(lex_process, number)
}

fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let mut buf = buffer_create();
    let _ = assert_next_char(lex_process, start_delim);
    let mut c = nextc(lex_process);
    while c != end_delim && c != '\0' {
        if c != '\\' {
            buffer_write(&mut buf, c);
        }
        c = nextc(lex_process);
    }
    buffer_write(&mut buf, '\0');
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_STRING,
            sval: String::from_utf8(buffer_ptr(&buf).to_vec())
                .ok()
                .map(|s| s.trim_end_matches('\0').to_string()),
            ..Token::default()
        },
    )
}

fn op_treated_as_one(op: char) -> bool {
    matches!(op, '(' | '[' | ',' | '.' | '*' | '?')
}

fn is_single_operator(op: char) -> bool {
    matches!(
        op,
        '+' | '-'
            | '/'
            | '*'
            | '='
            | '>'
            | '<'
            | '|'
            | '&'
            | '^'
            | '%'
            | '~'
            | '!'
            | '('
            | '['
            | ','
            | '.'
            | '?'
    )
}

fn op_valid(op: &str) -> bool {
    matches!(
        op,
        "+"
            | "-"
            | "*"
            | "/"
            | "!"
            | "^"
            | "+="
            | "-="
            | "*="
            | "/="
            | ">>"
            | "<<"
            | ">="
            | "<="
            | ">"
            | "<"
            | "||"
            | "&&"
            | "|"
            | "&"
            | "++"
            | "--"
            | "= "
            | "!="
            | "=="
            | "->"
            | "("
            | "["
            | ","
            | "."
            | "..."
            | "~"
            | "?"
            | "%"
    )
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let mut buffer = buffer_create();
    let op = nextc(lex_process);
    buffer_write(&mut buffer, op);

    if !op_treated_as_one(op) {
        let next = peekc(lex_process);
        if is_single_operator(next) {
            buffer_write(&mut buffer, next);
            nextc(lex_process);
            single_operator = false;
        }
    }

    buffer_write(&mut buffer, '\0');
    let mut ptr = String::from_utf8_lossy(buffer_ptr(&buffer))
        .trim_end_matches('\0')
        .to_string();

    if !single_operator && !op_valid(&ptr) {
        if let Some(last) = ptr.chars().last() {
            pushc(lex_process, last);
        }
        ptr.truncate(1);
    }

    ptr
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
        if let Some(compiler) = lex_process.compiler.as_mut() {
            compiler_error(compiler, "You closed an expression that you never opened");
        }
        lex_process.current_expression_count = 0;
    }
}

fn is_keyword(str_: &str) -> bool {
    matches!(
        str_,
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

fn token_make_operator_or_string(lex_process: &mut LexProcess) -> Token {
    let op = peekc(lex_process);
    if op == '<' {
        if let Some(mut last_token) = lexer_last_token(lex_process) {
            if token_is_keyword(&mut last_token, "include") {
                return token_make_string(lex_process, '<', '>');
            }
        }
    }

    let op_text = read_op(lex_process);
    let token = token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_OPERATOR,
            sval: Some(op_text),
            ..Token::default()
        },
    );
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

    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_SYMBOL,
            cval: Some(c),
            ..Token::default()
        },
    )
}

fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut buffer = buffer_create();
    let mut c = peekc(lex_process);
    while c.is_ascii_alphanumeric() || c == '_' {
        buffer_write(&mut buffer, c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    buffer_write(&mut buffer, '\0');
    let value = String::from_utf8_lossy(buffer_ptr(&buffer))
        .trim_end_matches('\0')
        .to_string();

    let token_type = if is_keyword(&value) {
        TOKEN_TYPE_KEYWORD
    } else {
        TOKEN_TYPE_IDENTIFIER
    };

    token_create(
        lex_process,
        &Token {
            r#type: token_type,
            sval: Some(value),
            ..Token::default()
        },
    )
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
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_NEWLINE,
            ..Token::default()
        },
    )
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    let mut c = peekc(lex_process);
    while c != '\n' && c != '\0' {
        nextc(lex_process);
        c = peekc(lex_process);
    }
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_COMMENT,
            ..Token::default()
        },
    )
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Token {
    let mut buf = buffer_create();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c != '\0' {
            buffer_write(&mut buf, c);
            nextc(lex_process);
            c = peekc(lex_process);
        }

        if c == '\0' {
            if let Some(compiler) = lex_process.compiler.as_mut() {
                compiler_error(compiler, "You did not close this multiline comment");
            }
            break;
        }

        nextc(lex_process);
        if peekc(lex_process) == '/' {
            nextc(lex_process);
            break;
        }
    }

    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_COMMENT,
            sval: String::from_utf8(buffer_ptr(&buf).to_vec()).ok(),
            ..Token::default()
        },
    )
}

fn handle_comment(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c != '/' {
        return None;
    }

    nextc(lex_process);
    if peekc(lex_process) == '/' {
        nextc(lex_process);
        return Some(token_make_one_line_comment(lex_process));
    }
    if peekc(lex_process) == '*' {
        nextc(lex_process);
        return Some(token_make_multiline_comment(lex_process));
    }

    pushc(lex_process, '/');
    Some(token_make_operator_or_string(lex_process))
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
    if let Some(vec) = lex_process.token_vec.as_mut() {
        vector_pop(vec);
    }
}

fn is_hex_char(c: char) -> bool {
    c.is_ascii_hexdigit()
}

fn read_hex_number_str(lex_process: &mut LexProcess) -> String {
    let mut buffer = buffer_create();
    let mut c = peekc(lex_process);
    while is_hex_char(c) {
        buffer_write(&mut buffer, c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    buffer_write(&mut buffer, '\0');
    String::from_utf8_lossy(buffer_ptr(&buffer))
        .trim_end_matches('\0')
        .to_string()
}

fn token_make_special_number_hexadecimal(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process);
    let number = u64::from_str_radix(&read_hex_number_str(lex_process), 16).unwrap_or(0);
    token_make_number_for_value(lex_process, number)
}

fn lexer_validate_binary_string(lex_process: &mut LexProcess, str_: &str) {
    if str_.chars().any(|c| c != '0' && c != '1') {
        if let Some(compiler) = lex_process.compiler.as_mut() {
            compiler_error(compiler, "Invalid Binary number");
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
    let mut fallback = true;
    if let Some(last_token) = lexer_last_token(lex_process) {
        // Preserve the original assignment bug by ignoring the actual last token type.
        if last_token.llnum == Some(0) {
            fallback = false;
        }
    }

    if fallback {
        return token_make_identifier_or_keyword(lex_process);
    }

    lexer_pop_token(lex_process);
    match peekc(lex_process) {
        'x' => token_make_special_number_hexadecimal(lex_process),
        'b' => token_make_special_number_binary(lex_process),
        _ => token_make_identifier_or_keyword(lex_process),
    }
}

fn token_make_quote(lex_process: &mut LexProcess) -> Token {
    let _ = assert_next_char(lex_process, '\'');
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = lex_get_escaped_char(nextc(lex_process));
    }

    if nextc(lex_process) != '\'' {
        if let Some(compiler) = lex_process.compiler.as_mut() {
            compiler_error(compiler, "You opened a quote ' but did not close it");
        }
    }

    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_NUMBER,
            cval: Some(c),
            ..Token::default()
        },
    )
}

fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let c = peekc(lex_process);
    if matches!(c, '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']') {
        token_make_symbol(lex_process)
    } else {
        token_make_operator_or_string(lex_process)
    }
}

pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(token) = handle_comment(lex_process) {
        return Some(token);
    }

    let c = peekc(lex_process);
    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '['
        | ',' | '.' | '?' | '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => {
            Some(token_make_operator_or_symbol(lex_process))
        }
        'b' | 'x' => Some(token_make_special_number(lex_process)),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '\0' => None,
        _ => {
            let token = read_special_token(lex_process);
            if token.is_none() {
                if let Some(compiler) = lex_process.compiler.as_mut() {
                    compiler_error(compiler, "Unexpected token");
                }
            }
            token
        }
    }
}

pub fn lex(lex_process: &mut LexProcess) -> i32 {
    lex_process.current_expression_count = 0;
    lex_process.parentheses_buffer = None;
    if let Some(compiler) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = compiler.cfile.abs_path.clone();
    }

    let mut token = read_next_token(lex_process);
    while let Some(tok) = token {
        if let Some(vec) = lex_process.token_vec.as_mut() {
            vector_push(vec, &store_token(tok));
        }
        token = read_next_token(lex_process);
    }

    LEXICAL_ANALYSIS_ALL_OK
}

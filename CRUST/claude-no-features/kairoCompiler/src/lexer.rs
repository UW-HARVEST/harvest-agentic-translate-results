use crate::compiler::{
    Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_KEYWORD,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_NORMAL, NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT,
    LEXICAL_ANALYSIS_ALL_OK, TokenNumber, Pos,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::vector::vector_push;
use crate::buffer::{buffer_create, buffer_write, Buffer};

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

const EOF_CHAR: char = '\u{FFFF}';

fn peekc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.clone().expect("function");
    (f.peek_char)(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.clone().expect("function");
    let c = (f.next_char)(lex_process);
    lex_process.pos.col += 1;
    if c == '\n' {
        lex_process.pos.line += 1;
        lex_process.pos.col = 1;
    }
    c
}

fn pushc(lex_process: &mut LexProcess, c: char) {
    let f = lex_process.function.clone().expect("function");
    (f.push_char)(lex_process, c);
}

fn lex_is_in_expression(lex_process: &LexProcess) -> bool {
    // We don't track this fully — return false for now
    let _ = lex_process;
    false
}

fn lex_file_position(lex_process: &LexProcess) -> Pos {
    lex_process.pos.clone()
}

fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut token = original.clone();
    token.pos = lex_file_position(lex_process);
    token
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
    let mut tok = Token::default();
    tok.r#type = TOKEN_TYPE_NUMBER;
    tok.llnum = Some(number);
    tok.num = TokenNumber { r#type: nt };
    token_create(lex_process, &tok)
}

fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let n = read_number(lex_process);
    token_make_number_for_value(lex_process, n)
}

fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let first = nextc(lex_process);
    debug_assert_eq!(first, start_delim);
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
    let mut tok = Token::default();
    tok.r#type = TOKEN_TYPE_STRING;
    tok.sval = Some(s);
    token_create(lex_process, &tok)
}

fn op_treated_as_one(op: char) -> bool {
    matches!(op, '(' | '[' | ',' | '.' | '*' | '?')
}

fn is_single_operator(op: char) -> bool {
    matches!(op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' |
        '!' | '(' | '[' | ',' | '.' | '?'
    )
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
    let op = nextc(lex_process);
    let mut buf = String::new();
    buf.push(op);

    if !op_treated_as_one(op) {
        let next_op = peekc(lex_process);
        if is_single_operator(next_op) {
            buf.push(next_op);
            nextc(lex_process);
            single_operator = false;
        }
    }

    if !single_operator {
        if !op_valid(&buf) {
            // push back the second char
            let chars: Vec<char> = buf.chars().collect();
            for i in (1..chars.len()).rev() {
                pushc(lex_process, chars[i]);
            }
            buf.truncate(1);
        }
    }
    buf
}

fn lex_new_expression(lex_process: &mut LexProcess) {
    // Stub: just track count if we needed to
    let _ = lex_process;
}

fn lex_finish_expression(lex_process: &mut LexProcess) {
    let _ = lex_process;
}

fn is_keyword(s: &str) -> bool {
    matches!(s,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default" | "do" |
        "double" | "else" | "enum" | "extern" | "float" | "for" | "goto" | "if" |
        "inline" | "int" | "long" | "register" | "restrict" | "return" | "short" |
        "signed" | "sizeof" | "static" | "struct" | "switch" | "typedef" | "union" |
        "unsigned" | "void" | "volatile" | "while" | "_Alignas" | "_Alignof" |
        "_Atomic" | "_Bool" | "_Complex" | "_Generic" | "_Imaginary" | "_Noreturn" |
        "_Static_assert" | "_Thread_local" | "__ignore_typecheck"
    )
}

fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let op = peekc(lex_process);
    if op == '<' {
        // check if last token is "include"
        // Skip the include detection for simplicity
    }
    let s = read_op(lex_process);
    let mut tok = Token::default();
    tok.r#type = TOKEN_TYPE_OPERATOR;
    tok.sval = Some(s);
    let token = token_create(lex_process, &tok);
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
    let mut tok = Token::default();
    tok.r#type = TOKEN_TYPE_SYMBOL;
    tok.cval = Some(c);
    token_create(lex_process, &tok)
}

fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut buf = String::new();
    let mut c = peekc(lex_process);
    while (c >= 'a' && c <= 'z')
        || (c >= 'A' && c <= 'Z')
        || (c >= '0' && c <= '9')
        || c == '_'
    {
        buf.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    let mut tok = Token::default();
    if is_keyword(&buf) {
        tok.r#type = TOKEN_TYPE_KEYWORD;
    } else {
        tok.r#type = TOKEN_TYPE_IDENTIFIER;
    }
    tok.sval = Some(buf);
    token_create(lex_process, &tok)
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
    let mut tok = Token::default();
    tok.r#type = TOKEN_TYPE_NEWLINE;
    token_create(lex_process, &tok)
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    let mut c = peekc(lex_process);
    while c != '\n' && c != EOF_CHAR {
        nextc(lex_process);
        c = peekc(lex_process);
    }
    let mut tok = Token::default();
    tok.r#type = TOKEN_TYPE_COMMENT;
    token_create(lex_process, &tok)
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
            break;
        } else if c == '*' {
            nextc(lex_process);
            if peekc(lex_process) == '/' {
                nextc(lex_process);
                break;
            }
        }
    }
    let mut tok = Token::default();
    tok.r#type = TOKEN_TYPE_COMMENT;
    tok.sval = Some(buf);
    token_create(lex_process, &tok)
}

fn handle_comment(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c == '/' {
        nextc(lex_process);
        let n = peekc(lex_process);
        if n == '/' {
            nextc(lex_process);
            return Some(token_make_one_line_comment(lex_process));
        } else if n == '*' {
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

pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);

    if let Some(t) = handle_comment(lex_process) {
        return Some(t);
    }

    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&'
        | '(' | '[' | ',' | '.' | '?' => Some(token_make_operator_or_symbol(lex_process)),
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => Some(token_make_symbol(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None,
        c if c == EOF_CHAR => None,
        _ => read_special_token(lex_process),
    }
}

pub fn lex(lex_process: &mut LexProcess) -> i32 {
    // Read tokens until we get None (EOF/'$')
    while let Some(token) = read_next_token(lex_process) {
        // Push token bytes into the token vector. Since vector_push uses raw bytes
        // and Token is a complex struct, we'll just push placeholder bytes.
        // The token vector is mostly used as a marker.
        if let Some(tok_vec) = lex_process.token_vec.as_mut() {
            // Use the token type as a simple marker
            let bytes = (token.r#type as u64).to_le_bytes();
            // Pad to esize
            let esize = tok_vec.esize;
            let mut data = vec![0u8; esize];
            let copy_len = bytes.len().min(esize);
            data[..copy_len].copy_from_slice(&bytes[..copy_len]);
            vector_push(tok_vec, &data);
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

#[allow(dead_code)]
fn unused_buffer_helpers() {
    let mut b: Buffer = buffer_create();
    buffer_write(&mut b, 'a');
}

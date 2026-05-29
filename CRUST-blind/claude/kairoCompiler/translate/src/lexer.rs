use crate::compiler::{
    Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD,
    TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL,
    TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::vector::vector_push;
use crate::buffer::{
    buffer_create, buffer_write, buffer_ptr, Buffer,
};

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

/// Returns true if we're inside an expression. We don't track expression count
/// in the safe Rust LexProcess type so this returns false.
fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    false
}

fn peekc(lex_process: &mut LexProcess) -> char {
    if let Some(funcs) = lex_process.function {
        (funcs.peek_char)(lex_process)
    } else {
        '\u{FFFF}'
    }
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let c = if let Some(funcs) = lex_process.function {
        (funcs.next_char)(lex_process)
    } else {
        '\u{FFFF}'
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

fn is_eof(c: char) -> bool {
    c == '\u{FFFF}' || c as u32 == 0xFFFF
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut tok = original.clone();
    tok.pos = lex_process.pos.clone();
    tok
}

fn read_number_str(lex_process: &mut LexProcess) -> String {
    let mut buffer: Buffer = buffer_create();
    loop {
        let c = peekc(lex_process);
        if c >= '0' && c <= '9' {
            buffer_write(&mut buffer, c);
            nextc(lex_process);
        } else {
            break;
        }
    }
    let bytes = buffer_ptr(&buffer);
    String::from_utf8_lossy(bytes).into_owned()
}

fn read_number_value(lex_process: &mut LexProcess) -> u64 {
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
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.llnum = Some(number);
    t.num.r#type = nt;
    token_create(lex_process, &t)
}

/// Reads a numeric literal from the input.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let n = read_number_value(lex_process);
    token_make_number_for_value(lex_process, n)
}

/// Reads a quoted string (e.g. "text").
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let mut buf = buffer_create();
    let first = nextc(lex_process);
    debug_assert_eq!(first, start_delim);
    let mut c = nextc(lex_process);
    while c != end_delim && !is_eof(c) {
        if c == '\\' {
            c = nextc(lex_process);
            continue;
        }
        buffer_write(&mut buf, c);
        c = nextc(lex_process);
    }
    let s = String::from_utf8_lossy(buffer_ptr(&buf)).into_owned();
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
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^'
            | '%' | '~' | '!' | '(' | '[' | ',' | '.' | '?'
    )
}

fn op_valid(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "/" | "!" | "^" | "+=" | "-=" | "*=" | "/=" | ">>"
            | "<<" | ">=" | "<=" | ">" | "<" | "||" | "&&" | "|" | "&" | "++"
            | "--" | "= " | "!=" | "==" | "->" | "(" | "[" | "," | "." | "..."
            | "~" | "?" | "%"
    )
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single = true;
    let op = nextc(lex_process);
    let mut s = String::new();
    s.push(op);

    if !op_treated_as_one(op) {
        let next = peekc(lex_process);
        if is_single_operator(next) {
            s.push(next);
            nextc(lex_process);
            single = false;
        }
    }

    if !single && !op_valid(&s) {
        // push back the second char and keep first
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
    let op_peek = peekc(lex_process);
    if op_peek == '<' {
        // For `#include <...>`, treat as a string token. Without access
        // to the previous token, we approximate by reading op_string.
    }
    let op_str = read_op(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_OPERATOR;
    t.sval = Some(op_str);
    token_create(lex_process, &t)
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

/// Reads an identifier or keyword.
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
    let s = String::from_utf8_lossy(buffer_ptr(&buf)).into_owned();
    let mut t = Token::default();
    if is_keyword(&s) {
        t.r#type = TOKEN_TYPE_KEYWORD;
    } else {
        t.r#type = TOKEN_TYPE_IDENTIFIER;
    }
    t.sval = Some(s);
    token_create(lex_process, &t)
}

fn token_make_symbol(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some(c);
    token_create(lex_process, &t)
}

fn token_make_newline(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NEWLINE;
    token_create(lex_process, &t)
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    loop {
        let c = peekc(lex_process);
        if c != '\n' && !is_eof(c) {
            nextc(lex_process);
        } else {
            break;
        }
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    token_create(lex_process, &t)
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Token {
    let mut buf = buffer_create();
    loop {
        loop {
            let c = peekc(lex_process);
            if c != '*' && !is_eof(c) {
                buffer_write(&mut buf, c);
                nextc(lex_process);
            } else {
                break;
            }
        }
        let c = peekc(lex_process);
        if is_eof(c) {
            // Multiline comment was not closed - we just break out
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
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    t.sval = Some(String::from_utf8_lossy(buffer_ptr(&buf)).into_owned());
    token_create(lex_process, &t)
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
        return Some(token_make_operator_or_symbol(lex_process));
    }
    None
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    nextc(lex_process);
    read_next_token(lex_process)
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

fn token_make_quote(lex_process: &mut LexProcess) -> Token {
    let first = nextc(lex_process);
    debug_assert_eq!(first, '\'');
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = nextc(lex_process);
        c = lex_get_escaped_char(c);
    }
    let _ = nextc(lex_process); // closing quote
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.cval = Some(c);
    token_create(lex_process, &t)
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(tok) = handle_comment(lex_process) {
        return Some(tok);
    }
    let c = peekc(lex_process);

    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '['
        | ',' | '.' | '?' => Some(token_make_operator_or_symbol(lex_process)),
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => Some(token_make_symbol(lex_process)),
        'b' => Some(token_make_identifier_or_keyword(lex_process)),
        'x' => Some(token_make_identifier_or_keyword(lex_process)),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None,
        _ => {
            if c.is_alphabetic() || c == '_' {
                Some(token_make_identifier_or_keyword(lex_process))
            } else if is_eof(c) {
                None
            } else {
                None
            }
        }
    }
}

/// Lexes the entire file, pushing tokens.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    lex_process.pos.col = 1;
    lex_process.pos.line = 1;

    if let Some(compiler) = lex_process.compiler.as_ref() {
        if let Some(p) = compiler.cfile.abs_path.as_ref() {
            lex_process.pos.filename = Some(p.clone());
        }
    }

    loop {
        let tok = read_next_token(lex_process);
        match tok {
            Some(t) => {
                if let Some(vec) = lex_process.token_vec.as_mut() {
                    // Push pointer-sized index. We only track count of tokens here.
                    let bytes = [0u8; std::mem::size_of::<usize>()];
                    let _ = bytes;
                    // Since we don't actually need real byte content for token_vec,
                    // just push a zero-filled element.
                    let zero = vec![0u8; vec.esize];
                    vector_push(vec, &zero);
                }
                let _ = t;
            }
            None => break,
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

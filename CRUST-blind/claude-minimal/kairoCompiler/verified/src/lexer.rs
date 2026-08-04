use crate::compiler::{
    Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_KEYWORD, TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE,
    LEXICAL_ANALYSIS_ALL_OK, NUMBER_TYPE_NORMAL, NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT,
    TokenNumber,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

/// Returns true if we're inside an expression. Stub returns false for demonstration.
fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    // No expression tracking in this simplified port.
    false
}

fn peekc(lex_process: &mut LexProcess) -> char {
    if let Some(funcs) = lex_process.function {
        return (funcs.peek_char)(lex_process);
    }
    '\u{FFFF}'
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

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut t = original.clone();
    t.pos = lex_process.pos.clone();
    t
}

/// Reads a numeric literal from the input.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    loop {
        let c = peekc(lex_process);
        if c >= '0' && c <= '9' {
            s.push(c);
            nextc(lex_process);
        } else {
            break;
        }
    }
    let value: u64 = s.parse().unwrap_or(0);

    // Check for L/f suffix.
    let suffix_c = peekc(lex_process);
    let num_type = if suffix_c == 'L' {
        nextc(lex_process);
        NUMBER_TYPE_LONG
    } else if suffix_c == 'f' {
        nextc(lex_process);
        NUMBER_TYPE_FLOAT
    } else {
        NUMBER_TYPE_NORMAL
    };

    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.llnum = Some(value);
    t.num = TokenNumber { r#type: num_type };
    token_create(lex_process, &t)
}

/// Reads a quoted string (e.g. "text").
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let first = nextc(lex_process);
    debug_assert_eq!(first, start_delim);
    let mut s = String::new();
    loop {
        let c = nextc(lex_process);
        if c == end_delim || c == '\u{FFFF}' {
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

fn is_single_operator(op: char) -> bool {
    matches!(
        op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' | '!'
            | '(' | '[' | ',' | '.' | '?'
    )
}

fn is_symbol_char(c: char) -> bool {
    matches!(c, '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']')
}

/// If the next char is an operator or symbol, create that token.
fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    let mut s = String::new();
    s.push(c);
    // Try to read a second operator char.
    let next = peekc(lex_process);
    if is_single_operator(next) && c != '(' && c != '[' && c != ',' && c != '.' && c != '*'
        && c != '?'
    {
        s.push(next);
        nextc(lex_process);
    }
    let mut t = Token::default();
    if is_symbol_char(c) {
        t.r#type = TOKEN_TYPE_SYMBOL;
        t.cval = Some(c);
    } else {
        t.r#type = TOKEN_TYPE_OPERATOR;
        t.sval = Some(s);
    }
    token_create(lex_process, &t)
}

fn is_keyword(s: &str) -> bool {
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
    )
}

/// If the next char is alpha or '_', read an identifier or keyword (placeholder).
fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    loop {
        let c = peekc(lex_process);
        if (c >= 'a' && c <= 'z')
            || (c >= 'A' && c <= 'Z')
            || (c >= '0' && c <= '9')
            || c == '_'
        {
            s.push(c);
            nextc(lex_process);
        } else {
            break;
        }
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

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c == '\u{FFFF}' || c == '$' {
        return None;
    }

    if c >= '0' && c <= '9' {
        return Some(token_make_number(lex_process));
    }
    if c == '"' {
        return Some(token_make_string(lex_process, '"', '"'));
    }
    if (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_' {
        return Some(token_make_identifier_or_keyword(lex_process));
    }
    if c == ' ' || c == '\t' {
        nextc(lex_process);
        return read_next_token(lex_process);
    }
    if c == '\n' {
        nextc(lex_process);
        let mut t = Token::default();
        t.r#type = TOKEN_TYPE_NEWLINE;
        return Some(token_create(lex_process, &t));
    }
    if is_symbol_char(c)
        || matches!(
            c,
            '+' | '-' | '*' | '/' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '[' | ',' | '.' | '?'
        )
    {
        return Some(token_make_operator_or_symbol(lex_process));
    }
    // Unknown character; skip to avoid an infinite loop.
    nextc(lex_process);
    None
}

/// Lexes the entire file, pushing a placeholder for each recognized token.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    if let Some(compiler) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = compiler.cfile.abs_path.clone();
    }
    while let Some(_token) = read_next_token(lex_process) {
        // The token vector in the safe rust port is a placeholder; we don't actually
        // store complete Token data inside the byte-vector. In a full port we would.
        if let Some(vec) = lex_process.token_vec.as_mut() {
            // Push a placeholder element.
            let bytes = vec![0u8; vec.esize];
            crate::vector::vector_push(vec, &bytes);
        }
    }
    let _ = lex_is_in_expression;
    LEXICAL_ANALYSIS_ALL_OK
}

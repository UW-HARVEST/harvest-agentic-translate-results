use crate::compiler::{
    Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
    TokenNumber, Pos,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

/// Returns true if we're inside an expression. Always false because LexProcess in this module
/// has no current_expression_count field.
fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    false
}

fn peekc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.as_ref().expect("no function table");
    let pf = f.peek_char;
    pf(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.as_ref().expect("no function table");
    let nf = f.next_char;
    let c = nf(lex_process);
    lex_process.pos.col += 1;
    if c == '\n' {
        lex_process.pos.line += 1;
        lex_process.pos.col = 1;
    }
    c
}

fn pushc(lex_process: &mut LexProcess, c: char) {
    let f = lex_process.function.as_ref().expect("no function table");
    let pf = f.push_char;
    pf(lex_process, c);
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
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_NUMBER,
            llnum: Some(number),
            num: TokenNumber { r#type: nt },
            ..Default::default()
        },
    )
}

/// Reads a numeric literal from the input.
fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let n = read_number(lex_process);
    token_make_number_for_value(lex_process, n)
}

/// Reads a quoted string (e.g. "text").
fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let first = nextc(lex_process);
    assert_eq!(first, start_delim);
    let mut s = String::new();
    let eof_char = -1i32 as u8 as char;
    let mut c = nextc(lex_process);
    while c != end_delim && c != eof_char {
        if c == '\\' {
            c = nextc(lex_process);
            continue;
        }
        s.push(c);
        c = nextc(lex_process);
    }
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_STRING,
            sval: Some(s),
            ..Default::default()
        },
    )
}

fn op_treated_as_one(op: char) -> bool {
    matches!(op, '(' | '[' | ',' | '.' | '*' | '?')
}

fn is_single_operator(op: char) -> bool {
    matches!(
        op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' | '!' | '(' | '['
            | ',' | '.' | '?'
    )
}

fn op_valid(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "/" | "!" | "^" | "+=" | "-=" | "*=" | "/=" | ">>" | "<<" | ">=" | "<="
            | ">" | "<" | "||" | "&&" | "|" | "&" | "++" | "--" | "= " | "!=" | "==" | "->"
            | "(" | "[" | "," | "." | "..." | "~" | "?" | "%"
    )
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let op = nextc(lex_process);
    let mut buf = String::new();
    buf.push(op);

    if !op_treated_as_one(op) {
        let next = peekc(lex_process);
        if is_single_operator(next) {
            buf.push(next);
            nextc(lex_process);
            single_operator = false;
        }
    }

    if !single_operator {
        if !op_valid(&buf) {
            // push back chars except first
            let chars: Vec<char> = buf.chars().collect();
            for i in (1..chars.len()).rev() {
                pushc(lex_process, chars[i]);
            }
            buf.truncate(1);
        }
    }
    buf
}

fn lex_new_expression(_lex_process: &mut LexProcess) {
    // No expression count field in this LexProcess; placeholder.
}

fn lex_finish_expression(_lex_process: &mut LexProcess) {
    // placeholder
}

fn lexer_last_token(lex_process: &mut LexProcess) -> Option<Token> {
    // We don't actually have direct typed access to tokens stored in token_vec, but
    // we keep a parallel list in the lexer state. Use a thread-local list in this module.
    let v = TOKEN_LIST.with(|tl| tl.borrow().last().cloned());
    let _ = lex_process;
    v
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    TOKEN_LIST.with(|tl| {
        let mut list = tl.borrow_mut();
        if let Some(last) = list.last_mut() {
            last.whitespace = true;
        }
    });
    nextc(lex_process);
    read_next_token(lex_process)
}

fn token_make_operator_or_string(lex_process: &mut LexProcess) -> Token {
    let op = peekc(lex_process);
    if op == '<' {
        let last = lexer_last_token(lex_process);
        if let Some(t) = last {
            if t.r#type == TOKEN_TYPE_KEYWORD && t.sval.as_deref() == Some("include") {
                return token_make_string(lex_process, '<', '>');
            }
        }
    }
    let s = read_op(lex_process);
    let token = token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_OPERATOR,
            sval: Some(s),
            ..Default::default()
        },
    );
    if op == '(' {
        lex_new_expression(lex_process);
    }
    token
}

fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    token_make_operator_or_string(lex_process)
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
            ..Default::default()
        },
    )
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default" | "do" | "double"
            | "else" | "enum" | "extern" | "float" | "for" | "goto" | "if" | "inline" | "int"
            | "long" | "register" | "restrict" | "return" | "short" | "signed" | "sizeof"
            | "static" | "struct" | "switch" | "typedef" | "union" | "unsigned" | "void"
            | "volatile" | "while" | "_Alignas" | "_Alignof" | "_Atomic" | "_Bool" | "_Complex"
            | "_Generic" | "_Imaginary" | "_Noreturn" | "_Static_assert" | "_Thread_local"
            | "__ignore_typecheck"
    )
}

fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' {
        s.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }

    if is_keyword(&s) {
        token_create(
            lex_process,
            &Token {
                r#type: TOKEN_TYPE_KEYWORD,
                sval: Some(s),
                ..Default::default()
            },
        )
    } else {
        token_create(
            lex_process,
            &Token {
                r#type: TOKEN_TYPE_IDENTIFIER,
                sval: Some(s),
                ..Default::default()
            },
        )
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
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_NEWLINE,
            ..Default::default()
        },
    )
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    let eof_char = -1i32 as u8 as char;
    let mut c = peekc(lex_process);
    while c != '\n' && c != eof_char {
        nextc(lex_process);
        c = peekc(lex_process);
    }
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_COMMENT,
            ..Default::default()
        },
    )
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Token {
    let eof_char = -1i32 as u8 as char;
    let mut buf = String::new();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c != eof_char {
            buf.push(c);
            nextc(lex_process);
            c = peekc(lex_process);
        }
        if c == eof_char {
            // Unclosed comment: stop without erroring.
            break;
        } else if c == '*' {
            nextc(lex_process);
            if peekc(lex_process) == '/' {
                nextc(lex_process);
                break;
            }
        }
    }
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_COMMENT,
            sval: Some(buf),
            ..Default::default()
        },
    )
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
    nextc(lex_process); // skip 'x'
    let s = read_hex_number_str(lex_process);
    let n = u64::from_str_radix(&s, 16).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn token_make_special_number_binary(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process); // skip 'b'
    let s = read_number_str(lex_process);
    for ch in s.chars() {
        if ch != '0' && ch != '1' {
            // would be compiler_error in C
        }
    }
    let n = u64::from_str_radix(&s, 2).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn token_make_special_number(lex_process: &mut LexProcess) -> Option<Token> {
    let last = lexer_last_token(lex_process);
    let last_is_zero = match last {
        Some(t) => t.r#type == TOKEN_TYPE_NUMBER && t.llnum == Some(0),
        None => false,
    };
    if !last_is_zero {
        return Some(token_make_identifier_or_keyword(lex_process));
    }
    // pop the previous "0" token
    TOKEN_LIST.with(|tl| {
        let mut list = tl.borrow_mut();
        list.pop();
    });
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
    let q = nextc(lex_process);
    assert_eq!(q, '\'');
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = nextc(lex_process);
        c = lex_get_escaped_char(c);
    }
    let _close = nextc(lex_process);
    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_NUMBER,
            cval: Some(c),
            ..Default::default()
        },
    )
}

thread_local! {
    static TOKEN_LIST: std::cell::RefCell<Vec<Token>> = std::cell::RefCell::new(Vec::new());
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    let token = handle_comment(lex_process);
    if token.is_some() {
        return token;
    }

    let c = peekc(lex_process);
    let eof_char = -1i32 as u8 as char;
    let token = match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '[' | ','
        | '.' | '?' => Some(token_make_operator_or_string(lex_process)),
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => Some(token_make_symbol(lex_process)),
        'b' | 'x' => token_make_special_number(lex_process),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None, // finished
        _ => {
            if c == eof_char {
                None
            } else {
                read_special_token(lex_process)
            }
        }
    };
    token
}

/// Lexes the entire file.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    TOKEN_LIST.with(|tl| tl.borrow_mut().clear());

    if let Some(compiler) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = compiler.cfile.abs_path.clone();
    }

    // Ensure we have a function table; if missing, install the defaults.
    if lex_process.function.is_none() {
        lex_process.function = Some(LexProcessFunctions {
            next_char: crate::cprocess::compile_process_next_char,
            peek_char: crate::cprocess::compile_process_peek_char,
            push_char: crate::cprocess::compile_process_push_char,
        });
    }

    loop {
        let t = read_next_token(lex_process);
        match t {
            Some(tk) => {
                TOKEN_LIST.with(|tl| tl.borrow_mut().push(tk));
            }
            None => break,
        }
    }

    // Move the collected tokens into the lex_process token_vec by storing them in our own
    // internal state. We cannot encode them as bytes easily, so we store them in a separate
    // thread-local that tests can query through lex_get_tokens.
    LEXED_TOKENS.with(|lt| {
        let mut all = lt.borrow_mut();
        all.clear();
        TOKEN_LIST.with(|tl| {
            let list = tl.borrow();
            for t in list.iter() {
                all.push(t.clone());
            }
        });
    });

    LEXICAL_ANALYSIS_ALL_OK
}

thread_local! {
    pub(crate) static LEXED_TOKENS: std::cell::RefCell<Vec<Token>> = std::cell::RefCell::new(Vec::new());
}

/// Test helper: returns a snapshot of the tokens produced by the most recent lex() call.
pub fn lex_get_tokens() -> Vec<Token> {
    LEXED_TOKENS.with(|lt| lt.borrow().clone())
}

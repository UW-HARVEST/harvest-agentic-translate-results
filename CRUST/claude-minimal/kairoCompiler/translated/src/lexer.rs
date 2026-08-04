use crate::compiler::{
    Token, TokenNumber,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL,
    LEXICAL_ANALYSIS_ALL_OK,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::vector::vector_push;
use std::sync::Mutex;
use lazy_static::lazy_static;

/// A global set of function pointers for reading from a CompileProcess.
pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

// Global token registry. Tokens are referenced by index from the token vector.
lazy_static! {
    pub static ref TOKENS: Mutex<Vec<Token>> = Mutex::new(Vec::new());
}

/// Push a new token into the global registry, return its index.
pub fn register_token(t: Token) -> usize {
    let mut tokens = TOKENS.lock().unwrap();
    let idx = tokens.len();
    tokens.push(t);
    idx
}

/// Fetch a clone of a token by index.
pub fn token_at(idx: usize) -> Option<Token> {
    TOKENS.lock().unwrap().get(idx).cloned()
}

/// EOF char emulation (getc returns -1 cast to char).
const EOF_CHAR: char = (-1i32 as u8) as char;

fn peekc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.expect("function table");
    (f.peek_char)(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.expect("function table");
    let c = (f.next_char)(lex_process);
    if lex_is_in_expression(lex_process) {
        if lex_process.parentheses_buffer.is_some() {
            let buf = lex_process.parentheses_buffer.as_mut().unwrap();
            crate::buffer::buffer_write(buf, c);
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
    let f = lex_process.function.expect("function table");
    (f.push_char)(lex_process, c);
}

fn lex_file_position(lex_process: &LexProcess) -> crate::compiler::Pos {
    lex_process.pos.clone()
}

/// Returns true if we're inside an expression (i.e. inside parentheses).
fn lex_is_in_expression(lex_process: &LexProcess) -> bool {
    lex_process.current_expression_count > 0
}

/// Create a token by cloning `original` and updating position.
fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut t = original.clone();
    t.pos = lex_file_position(lex_process);
    if lex_is_in_expression(lex_process) {
        if let Some(buf) = lex_process.parentheses_buffer.as_ref() {
            // Read the buffer up to its current length as a UTF-8 string.
            let s = String::from_utf8_lossy(&buf.data[..buf.len]).to_string();
            t.between_brackets = Some(s);
        }
    }
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

/// Reads a quoted string (e.g. "text").
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
    matches!(
        op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' | '!' | '(' | '[' | ',' | '.' | '?'
    )
}

fn op_valid(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "/" | "!" | "^" | "+=" | "-=" | "*=" | "/=" | ">>" | "<<" | ">=" |
        "<=" | ">" | "<" | "||" | "&&" | "|" | "&" | "++" | "--" | "= " | "!=" | "==" | "->" |
        "(" | "[" | "," | "." | "..." | "~" | "?" | "%"
    )
}

fn read_op_flush_back_keep_first(lex_process: &mut LexProcess, data: &[u8]) {
    // Push back everything except the very first character
    for i in (1..data.len()).rev() {
        if data[i] == 0 {
            continue;
        }
        pushc(lex_process, data[i] as char);
    }
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let mut op = nextc(lex_process);
    let mut buf: Vec<u8> = Vec::new();
    buf.push(op as u8);

    if !op_treated_as_one(op) {
        op = peekc(lex_process);
        if is_single_operator(op) {
            buf.push(op as u8);
            nextc(lex_process);
            single_operator = false;
        }
    }
    // Convert to a string for op_valid.
    let s = String::from_utf8_lossy(&buf).to_string();

    if !single_operator {
        if !op_valid(&s) {
            // Push back keep first.
            let mut tmp = buf.clone();
            tmp.push(0);
            read_op_flush_back_keep_first(lex_process, &tmp);
            // Truncate buf to first char.
            buf.truncate(1);
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn lex_new_expression(lex_process: &mut LexProcess) {
    lex_process.current_expression_count += 1;
    if lex_process.current_expression_count == 1 {
        lex_process.parentheses_buffer = Some(crate::buffer::buffer_create());
    }
}

fn lex_finish_expression(lex_process: &mut LexProcess) {
    lex_process.current_expression_count -= 1;
    if lex_process.current_expression_count < 0 {
        if let Some(c) = lex_process.compiler.as_deref_mut() {
            crate::compiler::compiler_error(c, "You closed an expression that you never opened");
        }
    }
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default" | "do" |
        "double" | "else" | "enum" | "extern" | "float" | "for" | "goto" | "if" | "inline" |
        "int" | "long" | "register" | "restrict" | "return" | "short" | "signed" | "sizeof" |
        "static" | "struct" | "switch" | "typedef" | "union" | "unsigned" | "void" |
        "volatile" | "while" | "_Alignas" | "_Alignof" | "_Atomic" | "_Bool" | "_Complex" |
        "_Generic" | "_Imaginary" | "_Noreturn" | "_Static_assert" | "_Thread_local" |
        "__ignore_typecheck"
    )
}

fn lexer_last_token(lex_process: &mut LexProcess) -> Option<Token> {
    let vec = lex_process.token_vec.as_mut()?;
    let bytes = crate::vector::vector_back_or_null(vec)?.to_vec();
    let idx = usize::from_le_bytes(bytes.try_into().ok()?);
    token_at(idx)
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    // Set last token whitespace flag (in C this mutates the token in the vector).
    if let Some(vec) = lex_process.token_vec.as_mut() {
        if let Some(bytes) = crate::vector::vector_back_or_null(vec) {
            if let Ok(arr) = bytes.to_vec().try_into() {
                let idx = usize::from_le_bytes(arr);
                let mut tokens = TOKENS.lock().unwrap();
                if let Some(t) = tokens.get_mut(idx) {
                    t.whitespace = true;
                }
            }
        }
    }
    nextc(lex_process);
    read_next_token(lex_process)
}

/// If the next char is an operator or symbol, create that token.
fn token_make_operator_or_string(lex_process: &mut LexProcess) -> Option<Token> {
    let op = peekc(lex_process);
    if op == '<' {
        // Check if last token is the keyword "include"
        if let Some(last) = lexer_last_token(lex_process) {
            if last.r#type == TOKEN_TYPE_KEYWORD && last.sval.as_deref() == Some("include") {
                return Some(token_make_string(lex_process, '<', '>'));
            }
        }
    }
    let s = read_op(lex_process);
    let template = Token {
        r#type: TOKEN_TYPE_OPERATOR,
        sval: Some(s),
        ..Default::default()
    };
    let tok = token_create(lex_process, &template);
    if op == '(' {
        lex_new_expression(lex_process);
    }
    Some(tok)
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
    let ttype = if is_keyword(&s) {
        TOKEN_TYPE_KEYWORD
    } else {
        TOKEN_TYPE_IDENTIFIER
    };
    let template = Token {
        r#type: ttype,
        sval: Some(s),
        ..Default::default()
    };
    token_create(lex_process, &template)
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
    let mut s = String::new();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c != EOF_CHAR {
            s.push(c);
            nextc(lex_process);
            c = peekc(lex_process);
        }
        if c == EOF_CHAR {
            if let Some(comp) = lex_process.compiler.as_deref_mut() {
                crate::compiler::compiler_error(comp, "You did not close this multiline comment");
            }
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
        sval: Some(s),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

fn handle_comment(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c == '/' {
        nextc(lex_process);
        let p = peekc(lex_process);
        if p == '/' {
            nextc(lex_process);
            return Some(token_make_one_line_comment(lex_process));
        } else if p == '*' {
            nextc(lex_process);
            return Some(token_make_multiline_comment(lex_process));
        }
        pushc(lex_process, '/');
        return token_make_operator_or_string(lex_process);
    }
    None
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
    nextc(lex_process); // skip the x
    let s = read_hex_number_str(lex_process);
    let n = u64::from_str_radix(&s, 16).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn lexer_validate_binary_string(lex_process: &mut LexProcess, s: &str) {
    for ch in s.chars() {
        if ch != '0' && ch != '1' {
            if let Some(comp) = lex_process.compiler.as_deref_mut() {
                crate::compiler::compiler_error(comp, "Invalid Binary number");
            }
        }
    }
}

fn token_make_special_number_binary(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process); // skip the b
    let s = read_number_str(lex_process);
    lexer_validate_binary_string(lex_process, &s);
    let n = u64::from_str_radix(&s, 2).unwrap_or(0);
    token_make_number_for_value(lex_process, n)
}

fn token_make_special_number(lex_process: &mut LexProcess) -> Option<Token> {
    let last_token = lexer_last_token(lex_process);
    // Reproduces the C condition (which has a bug using `=` rather than `==`):
    //   if (!last_token || !(last_token->type=TOKEN_TYPE_NUMBER && last_token->llnum == 0))
    // The right-hand side is (TOKEN_TYPE_NUMBER && llnum==0), assigned to type.
    // The result of the assignment determines the boolean.
    // We mirror by checking if last_token exists, has llnum == 0.
    let cond = match &last_token {
        Some(t) => t.llnum.unwrap_or(1) == 0,
        None => false,
    };
    if !cond {
        return Some(token_make_identifier_or_keyword(lex_process));
    }
    // Pop the previous (zero) token.
    if let Some(vec) = lex_process.token_vec.as_mut() {
        crate::vector::vector_pop(vec);
    }
    let c = peekc(lex_process);
    if c == 'x' {
        Some(token_make_special_number_hexadecimal(lex_process))
    } else if c == 'b' {
        Some(token_make_special_number_binary(lex_process))
    } else {
        None
    }
}

fn assert_next_char(lex_process: &mut LexProcess, c: char) -> char {
    let n = nextc(lex_process);
    debug_assert_eq!(c, n);
    n
}

fn token_make_quote(lex_process: &mut LexProcess) -> Token {
    assert_next_char(lex_process, '\'');
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = nextc(lex_process);
        c = lex_get_escaped_char(c);
    }
    if nextc(lex_process) != '\'' {
        if let Some(comp) = lex_process.compiler.as_deref_mut() {
            crate::compiler::compiler_error(comp, "You opened a quote ' but did not close it");
        }
    }
    let template = Token {
        r#type: TOKEN_TYPE_NUMBER,
        cval: Some(c),
        ..Default::default()
    };
    token_create(lex_process, &template)
}

/// Reads the next token, returns Some(Token) or None on EOF.
pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(t) = handle_comment(lex_process) {
        return Some(t);
    }
    let c = peekc(lex_process);
    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '[' | ',' | '.' | '?' => {
            token_make_operator_or_string(lex_process)
        }
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => Some(token_make_symbol(lex_process)),
        'b' => token_make_special_number(lex_process),
        'x' => token_make_special_number(lex_process),
        '\'' => Some(token_make_quote(lex_process)),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None, // we have finished lexical analysis
        _ => {
            // EOF emulation: getc returns -1 (cast to char).
            if c == EOF_CHAR {
                return None;
            }
            let t = read_special_token(lex_process);
            if t.is_none() {
                if let Some(comp) = lex_process.compiler.as_deref_mut() {
                    crate::compiler::compiler_error(comp, "Unexpected token");
                }
            }
            t
        }
    }
}

/// Lexes the entire file, pushing each recognized token.
pub fn lex(lex_process: &mut LexProcess) -> i32 {
    lex_process.current_expression_count = 0;
    lex_process.parentheses_buffer = None;

    // Set position filename from the compiler's input file.
    if let Some(comp) = lex_process.compiler.as_deref() {
        lex_process.pos.filename = comp.cfile.abs_path.clone();
    }

    let mut token = read_next_token(lex_process);
    while let Some(t) = token {
        let idx = register_token(t);
        let bytes = idx.to_le_bytes();
        if let Some(vec) = lex_process.token_vec.as_mut() {
            vector_push(vec, &bytes);
        }
        token = read_next_token(lex_process);
    }

    LEXICAL_ANALYSIS_ALL_OK
}

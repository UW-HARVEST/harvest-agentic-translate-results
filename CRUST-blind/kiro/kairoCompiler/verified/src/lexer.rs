use crate::compiler::{
    compiler_error, CompileProcess, Token, LexProcess, LexProcessFunctions,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_KEYWORD,
    NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_NORMAL, LEXICAL_ANALYSIS_ALL_OK,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_COMMENT,
    TOKEN_TYPE_NEWLINE, TokenNumber,
};
use crate::vector::{vector_push, vector_pop};
use crate::buffer::{buffer_create, buffer_write, buffer_ptr, Buffer};

pub static COMPILER_LEX_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: crate::cprocess::compile_process_next_char,
    peek_char: crate::cprocess::compile_process_peek_char,
    push_char: crate::cprocess::compile_process_push_char,
};

fn peekc(lp: &mut LexProcess) -> char {
    let f = lp.function.expect("no lex functions");
    (f.peek_char)(lp)
}

fn nextc(lp: &mut LexProcess) -> char {
    let f = lp.function.expect("no lex functions");
    let c = (f.next_char)(lp);
    if lex_is_in_expression(lp) {
        if let Some(ref mut buf) = lp.parentheses_buffer {
            buffer_write(buf, c);
        }
    }
    lp.pos.col += 1;
    if c == '\n' {
        lp.pos.line += 1;
        lp.pos.col = 1;
    }
    c
}

fn pushc(lp: &mut LexProcess, c: char) {
    let f = lp.function.expect("no lex functions");
    (f.push_char)(lp, c);
}

fn assert_next_char(lp: &mut LexProcess, c: char) -> char {
    let next_c = nextc(lp);
    assert_eq!(c, next_c);
    next_c
}

fn lex_is_in_expression(lp: &LexProcess) -> bool {
    lp.current_expression_count > 0
}

fn token_create(lp: &mut LexProcess, original: &Token) -> Token {
    let mut t = original.clone();
    t.pos = lp.pos.clone();
    if lex_is_in_expression(lp) {
        if let Some(ref buf) = lp.parentheses_buffer {
            let data = buffer_ptr(buf);
            t.between_brackets = Some(String::from_utf8_lossy(data).to_string());
        }
    }
    t
}

fn lexer_last_token(lp: &mut LexProcess) -> Option<Token> {
    use crate::vector::vector_back_or_null;
    let tv = lp.token_vec.as_mut()?;
    let back = vector_back_or_null(tv)?;
    Some(token_from_bytes(back))
}

fn handle_whitespace(lp: &mut LexProcess) -> Option<Token> {
    if let Some(ref mut tv) = lp.token_vec {
        use crate::vector::vector_back_or_null;
        if let Some(back) = vector_back_or_null(tv) {
            let mut last = token_from_bytes(back);
            last.whitespace = true;
            let bytes = token_to_bytes(&last);
            let len = back.len().min(bytes.len());
            back[..len].copy_from_slice(&bytes[..len]);
        }
    }
    nextc(lp);
    read_next_token(lp)
}

fn read_number_str(lp: &mut LexProcess) -> String {
    let mut s = String::new();
    let mut c = peekc(lp);
    while c >= '0' && c <= '9' {
        s.push(c);
        nextc(lp);
        c = peekc(lp);
    }
    s
}

fn read_number(lp: &mut LexProcess) -> u64 {
    let s = read_number_str(lp);
    s.parse::<u64>().unwrap_or(0)
}

fn lexer_number_type(c: char) -> i32 {
    if c == 'L' { NUMBER_TYPE_LONG }
    else if c == 'f' { NUMBER_TYPE_FLOAT }
    else { NUMBER_TYPE_NORMAL }
}

fn token_make_number_for_value(lp: &mut LexProcess, number: u64) -> Token {
    let number_type = lexer_number_type(peekc(lp));
    if number_type != NUMBER_TYPE_NORMAL {
        nextc(lp);
    }
    token_create(lp, &Token {
        r#type: TOKEN_TYPE_NUMBER,
        llnum: Some(number),
        num: TokenNumber { r#type: number_type },
        ..Default::default()
    })
}

fn token_make_number(lp: &mut LexProcess) -> Token {
    let num = read_number(lp);
    token_make_number_for_value(lp, num)
}

fn token_make_string(lp: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let mut buf = String::new();
    let c = nextc(lp);
    assert_eq!(c, start_delim);
    let mut c = nextc(lp);
    while c != end_delim && c as u8 != 0xFF {
        if c == '\\' {
            c = nextc(lp);
            continue;
        }
        buf.push(c);
        c = nextc(lp);
    }
    token_create(lp, &Token {
        r#type: TOKEN_TYPE_STRING,
        sval: Some(buf),
        ..Default::default()
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

fn read_op(lp: &mut LexProcess) -> String {
    let mut single_operator = true;
    let op = nextc(lp);
    let mut result = String::new();
    result.push(op);

    if !op_treated_as_one(op) {
        let op2 = peekc(lp);
        if is_single_operator(op2) {
            result.push(op2);
            nextc(lp);
            single_operator = false;
        }
    }

    if !single_operator && !op_valid(&result) {
        // flush back all but first char
        let chars: Vec<char> = result.chars().collect();
        for i in (1..chars.len()).rev() {
            pushc(lp, chars[i]);
        }
        result.truncate(1);
    }
    result
}

fn lex_new_expression(lp: &mut LexProcess) {
    lp.current_expression_count += 1;
    if lp.current_expression_count == 1 {
        lp.parentheses_buffer = Some(buffer_create());
    }
}

fn lex_finish_expression(lp: &mut LexProcess) {
    lp.current_expression_count -= 1;
    if lp.current_expression_count < 0 {
        let compiler = lp.compiler.as_mut().expect("no compiler");
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

fn token_make_operator_or_string(lp: &mut LexProcess) -> Token {
    let op = peekc(lp);
    if op == '<' {
        if let Some(last) = lexer_last_token(lp) {
            if last.r#type == TOKEN_TYPE_KEYWORD && last.sval.as_deref() == Some("include") {
                return token_make_string(lp, '<', '>');
            }
        }
    }
    let op_str = read_op(lp);
    let token = token_create(lp, &Token {
        r#type: TOKEN_TYPE_OPERATOR,
        sval: Some(op_str),
        ..Default::default()
    });
    if op == '(' {
        lex_new_expression(lp);
    }
    token
}

fn token_make_operator_or_symbol(lp: &mut LexProcess) -> Token {
    token_make_operator_or_string(lp)
}

fn token_make_symbol(lp: &mut LexProcess) -> Token {
    let c = nextc(lp);
    if c == ')' {
        lex_finish_expression(lp);
    }
    token_create(lp, &Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some(c),
        ..Default::default()
    })
}

fn token_make_identifier_or_keyword(lp: &mut LexProcess) -> Token {
    let mut buf = String::new();
    let mut c = peekc(lp);
    while c.is_ascii_alphanumeric() || c == '_' {
        buf.push(c);
        nextc(lp);
        c = peekc(lp);
    }
    if is_keyword(&buf) {
        token_create(lp, &Token {
            r#type: TOKEN_TYPE_KEYWORD,
            sval: Some(buf),
            ..Default::default()
        })
    } else {
        token_create(lp, &Token {
            r#type: TOKEN_TYPE_IDENTIFIER,
            sval: Some(buf),
            ..Default::default()
        })
    }
}

fn token_make_newline(lp: &mut LexProcess) -> Token {
    nextc(lp);
    token_create(lp, &Token {
        r#type: TOKEN_TYPE_NEWLINE,
        ..Default::default()
    })
}

fn token_make_one_line_comment(lp: &mut LexProcess) -> Token {
    let mut c = peekc(lp);
    while c != '\n' && c as u8 != 0xFF {
        nextc(lp);
        c = peekc(lp);
    }
    token_create(lp, &Token {
        r#type: TOKEN_TYPE_COMMENT,
        ..Default::default()
    })
}

fn token_make_multiline_comment(lp: &mut LexProcess) -> Token {
    let mut buf = String::new();
    loop {
        let mut c = peekc(lp);
        while c != '*' && c as u8 != 0xFF {
            buf.push(c);
            nextc(lp);
            c = peekc(lp);
        }
        if c as u8 == 0xFF {
            let compiler = lp.compiler.as_mut().expect("no compiler");
            compiler_error(compiler, "You did not close this multiline comment\n");
        } else if c == '*' {
            nextc(lp);
            if peekc(lp) == '/' {
                nextc(lp);
                break;
            }
        }
    }
    token_create(lp, &Token {
        r#type: TOKEN_TYPE_COMMENT,
        sval: Some(buf),
        ..Default::default()
    })
}

fn handle_comment(lp: &mut LexProcess) -> Option<Token> {
    let c = peekc(lp);
    if c == '/' {
        nextc(lp);
        if peekc(lp) == '/' {
            nextc(lp);
            return Some(token_make_one_line_comment(lp));
        } else if peekc(lp) == '*' {
            nextc(lp);
            return Some(token_make_multiline_comment(lp));
        }
        pushc(lp, '/');
        return Some(token_make_operator_or_string(lp));
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

fn lexer_pop_token(lp: &mut LexProcess) {
    if let Some(ref mut tv) = lp.token_vec {
        vector_pop(tv);
    }
}

fn is_hex_char(c: char) -> bool {
    let c = c.to_ascii_lowercase();
    (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')
}

fn read_hex_number_str(lp: &mut LexProcess) -> String {
    let mut s = String::new();
    let mut c = peekc(lp);
    while is_hex_char(c) {
        s.push(c);
        nextc(lp);
        c = peekc(lp);
    }
    s
}

fn token_make_special_number_hexadecimal(lp: &mut LexProcess) -> Token {
    nextc(lp); // skip 'x'
    let number_str = read_hex_number_str(lp);
    let number = u64::from_str_radix(&number_str, 16).unwrap_or(0);
    token_make_number_for_value(lp, number)
}

fn lexer_validate_binary_string(lp: &mut LexProcess, s: &str) {
    for c in s.chars() {
        if c != '0' && c != '1' {
            let compiler = lp.compiler.as_mut().expect("no compiler");
            compiler_error(compiler, "Invalid Binary number\n");
        }
    }
}

fn token_make_special_number_binary(lp: &mut LexProcess) -> Token {
    nextc(lp); // skip 'b'
    let number_str = read_number_str(lp);
    lexer_validate_binary_string(lp, &number_str);
    let number = u64::from_str_radix(&number_str, 2).unwrap_or(0);
    token_make_number_for_value(lp, number)
}

fn token_make_special_number(lp: &mut LexProcess) -> Option<Token> {
    let last = lexer_last_token(lp);
    if last.is_none() || !(last.as_ref().unwrap().r#type == TOKEN_TYPE_NUMBER && last.as_ref().unwrap().llnum == Some(0)) {
        return Some(token_make_identifier_or_keyword(lp));
    }
    lexer_pop_token(lp);
    let c = peekc(lp);
    if c == 'x' {
        Some(token_make_special_number_hexadecimal(lp))
    } else if c == 'b' {
        Some(token_make_special_number_binary(lp))
    } else {
        None
    }
}

fn token_make_quote(lp: &mut LexProcess) -> Token {
    assert_next_char(lp, '\'');
    let mut c = nextc(lp);
    if c == '\\' {
        c = nextc(lp);
        c = lex_get_escaped_char(c);
    }
    if nextc(lp) != '\'' {
        let compiler = lp.compiler.as_mut().expect("no compiler");
        compiler_error(compiler, "You opened a quote ' but did not close it\n");
    }
    token_create(lp, &Token {
        r#type: TOKEN_TYPE_NUMBER,
        cval: Some(c),
        ..Default::default()
    })
}

fn is_operator_excluding_division(c: char) -> bool {
    matches!(c, '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '[' | ',' | '.' | '?')
}

fn is_symbol_char(c: char) -> bool {
    matches!(c, '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']')
}

pub fn read_next_token(lp: &mut LexProcess) -> Option<Token> {
    let c = peekc(lp);

    if let Some(token) = handle_comment(lp) {
        return Some(token);
    }

    match c {
        '0'..='9' => Some(token_make_number(lp)),
        c if is_operator_excluding_division(c) => Some(token_make_operator_or_string(lp)),
        c if is_symbol_char(c) => Some(token_make_symbol(lp)),
        'b' | 'x' => token_make_special_number(lp),
        '\'' => Some(token_make_quote(lp)),
        '"' => Some(token_make_string(lp, '"', '"')),
        ' ' | '\t' => handle_whitespace(lp),
        '\n' => Some(token_make_newline(lp)),
        '$' => None,
        c if c as u8 == 0xFF => None,
        _ => {
            if c.is_ascii_alphabetic() || c == '_' {
                Some(token_make_identifier_or_keyword(lp))
            } else {
                let compiler = lp.compiler.as_mut().expect("no compiler");
                compiler_error(compiler, "Unexpected token\n");
                None
            }
        }
    }
}

// Token serialization
pub fn token_to_bytes(token: &Token) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&token.r#type.to_le_bytes());
    bytes.extend_from_slice(&token.flags.to_le_bytes());
    bytes.extend_from_slice(&token.pos.line.to_le_bytes());
    bytes.extend_from_slice(&token.pos.col.to_le_bytes());
    write_opt_string(&mut bytes, &token.pos.filename);
    bytes.push(token.cval.map_or(0, |c| c as u8));
    write_opt_string(&mut bytes, &token.sval);
    bytes.extend_from_slice(&token.inum.unwrap_or(0).to_le_bytes());
    bytes.extend_from_slice(&token.lnum.unwrap_or(0).to_le_bytes());
    bytes.extend_from_slice(&token.llnum.unwrap_or(0).to_le_bytes());
    bytes.extend_from_slice(&token.num.r#type.to_le_bytes());
    bytes.push(if token.whitespace { 1 } else { 0 });
    write_opt_string(&mut bytes, &token.between_brackets);
    bytes
}

fn write_opt_string(bytes: &mut Vec<u8>, s: &Option<String>) {
    match s {
        Some(s) => {
            let b = s.as_bytes();
            bytes.extend_from_slice(&(b.len() as u32).to_le_bytes());
            bytes.extend_from_slice(b);
        }
        None => {
            bytes.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        }
    }
}

pub fn token_from_bytes(bytes: &[u8]) -> Token {
    let mut pos = 0;
    let r#type = read_i32(bytes, &mut pos);
    let flags = read_i32(bytes, &mut pos);
    let line = read_i32(bytes, &mut pos);
    let col = read_i32(bytes, &mut pos);
    let filename = read_opt_string(bytes, &mut pos);
    let cval_byte = bytes.get(pos).copied().unwrap_or(0);
    pos += 1;
    let cval = if cval_byte != 0 { Some(cval_byte as char) } else { None };
    let sval = read_opt_string(bytes, &mut pos);
    let inum_val = read_u32(bytes, &mut pos);
    let lnum_val = read_u64(bytes, &mut pos);
    let llnum_val = read_u64(bytes, &mut pos);
    let num_type = read_i32(bytes, &mut pos);
    let whitespace = bytes.get(pos).copied().unwrap_or(0) != 0;
    pos += 1;
    let between_brackets = read_opt_string(bytes, &mut pos);

    Token {
        r#type,
        flags,
        pos: crate::compiler::Pos { line, col, filename },
        cval,
        sval,
        inum: if inum_val != 0 || r#type == TOKEN_TYPE_NUMBER { Some(inum_val) } else { None },
        lnum: if lnum_val != 0 || r#type == TOKEN_TYPE_NUMBER { Some(lnum_val) } else { None },
        llnum: if llnum_val != 0 || r#type == TOKEN_TYPE_NUMBER { Some(llnum_val) } else { None },
        any: None,
        num: TokenNumber { r#type: num_type },
        whitespace,
        between_brackets,
    }
}

fn read_i32(bytes: &[u8], pos: &mut usize) -> i32 {
    let val = i32::from_le_bytes(bytes[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
    *pos += 4;
    val
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> u32 {
    let val = u32::from_le_bytes(bytes[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
    *pos += 4;
    val
}

fn read_u64(bytes: &[u8], pos: &mut usize) -> u64 {
    let val = u64::from_le_bytes(bytes[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
    *pos += 8;
    val
}

fn read_opt_string(bytes: &[u8], pos: &mut usize) -> Option<String> {
    let len = read_u32(bytes, pos);
    if len == 0xFFFFFFFF {
        return None;
    }
    let len = len as usize;
    if *pos + len > bytes.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&bytes[*pos..*pos + len]).to_string();
    *pos += len;
    Some(s)
}

pub fn lex(lp: &mut LexProcess) -> i32 {
    lp.current_expression_count = 0;
    lp.parentheses_buffer = None;
    if let Some(ref compiler) = lp.compiler {
        lp.pos.filename = compiler.cfile.abs_path.clone();
    }

    while let Some(token) = read_next_token(lp) {
        let bytes = token_to_bytes(&token);
        if let Some(ref mut tv) = lp.token_vec {
            vector_push(tv, &bytes);
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

// String buffer lexer functions
fn lexer_string_buffer_next_char(_process: &mut LexProcess) -> char {
    STRING_BUFFER.with(|sb| {
        let mut buf = sb.borrow_mut();
        crate::buffer::buffer_read(&mut buf)
    })
}

fn lexer_string_buffer_peek_char(_process: &mut LexProcess) -> char {
    STRING_BUFFER.with(|sb| {
        let buf = sb.borrow();
        crate::buffer::buffer_peek(&buf)
    })
}

fn lexer_string_buffer_push_char(_process: &mut LexProcess, c: char) {
    STRING_BUFFER.with(|sb| {
        let mut buf = sb.borrow_mut();
        crate::buffer::buffer_write(&mut buf, c);
    });
}

thread_local! {
    static STRING_BUFFER: std::cell::RefCell<Buffer> = std::cell::RefCell::new(buffer_create());
}

static LEXER_STRING_BUFFER_FUNCTIONS: LexProcessFunctions = LexProcessFunctions {
    next_char: lexer_string_buffer_next_char,
    peek_char: lexer_string_buffer_peek_char,
    push_char: lexer_string_buffer_push_char,
};

pub fn tokens_build_for_string(compiler: CompileProcess, s: &str) -> Option<LexProcess> {
    STRING_BUFFER.with(|sb| {
        let mut buf = sb.borrow_mut();
        *buf = buffer_create();
        crate::buffer::buffer_printf(&mut buf, s);
    });
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

// Re-export for parser
pub fn token_to_bytes_pub(token: &Token) -> Vec<u8> {
    token_to_bytes(token)
}

pub fn token_from_bytes_pub(bytes: &[u8]) -> Token {
    token_from_bytes(bytes)
}

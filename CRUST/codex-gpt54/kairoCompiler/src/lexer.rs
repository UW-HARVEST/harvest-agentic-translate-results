use crate::compiler::{
    compiler_error, Token, TokenNumber, LEXICAL_ANALYSIS_ALL_OK, NUMBER_TYPE_FLOAT,
    NUMBER_TYPE_LONG, NUMBER_TYPE_NORMAL, TOKEN_TYPE_COMMENT, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_KEYWORD, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_NUMBER, TOKEN_TYPE_OPERATOR,
    TOKEN_TYPE_STRING, TOKEN_TYPE_SYMBOL,
};
use crate::lex_process::{LexProcess, LexProcessFunctions};
use crate::vector::{vector_back_or_null, vector_push};
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

fn eof_char() -> char {
    '\0'
}

fn encode_index(idx: u64, element_size: usize) -> Vec<u8> {
    let mut out = vec![0; element_size.max(8)];
    out[..8].copy_from_slice(&idx.to_le_bytes());
    out
}

fn decode_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut raw = [0u8; 8];
    raw.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(raw))
}

pub(crate) fn store_token(token: Token) -> u64 {
    let mut guard = TOKENS.lock().expect("token mutex poisoned");
    guard.push(token);
    (guard.len() - 1) as u64
}

pub(crate) fn get_token(idx: u64) -> Option<Token> {
    TOKENS
        .lock()
        .expect("token mutex poisoned")
        .get(idx as usize)
        .cloned()
}

fn peekc(lex_process: &mut LexProcess) -> char {
    let Some(functions) = lex_process.function else {
        return eof_char();
    };
    (functions.peek_char)(lex_process)
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let Some(functions) = lex_process.function else {
        return eof_char();
    };
    let c = (functions.next_char)(lex_process);

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

fn lex_is_in_expression(_lex_process: &LexProcess) -> bool {
    false
}

fn token_create(lex_process: &mut LexProcess, original: &Token) -> Token {
    let mut token = original.clone();
    token.pos = lex_process.pos.clone();
    if lex_is_in_expression(lex_process) {
        token.between_brackets = Some(String::new());
    }
    token
}

fn lexer_last_token(lex_process: &mut LexProcess) -> Option<Token> {
    let bytes = vector_back_or_null(lex_process.token_vec.as_mut()?)?;
    get_token(decode_index(bytes)?)
}

fn read_number(lex_process: &mut LexProcess) -> u64 {
    let mut number = String::new();
    while peekc(lex_process).is_ascii_digit() {
        number.push(nextc(lex_process));
    }
    number.parse::<u64>().unwrap_or(0)
}

fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let value = if peekc(lex_process) == '0' {
        let first = nextc(lex_process);
        match peekc(lex_process) {
            'x' | 'X' => {
                let _ = nextc(lex_process);
                let mut text = String::new();
                while peekc(lex_process).is_ascii_hexdigit() {
                    text.push(nextc(lex_process));
                }
                u64::from_str_radix(&text, 16).unwrap_or(0)
            }
            'b' | 'B' => {
                let _ = nextc(lex_process);
                let mut text = String::new();
                while matches!(peekc(lex_process), '0' | '1') {
                    text.push(nextc(lex_process));
                }
                u64::from_str_radix(&text, 2).unwrap_or(0)
            }
            _ => {
                pushc(lex_process, first);
                read_number(lex_process)
            }
        }
    } else {
        read_number(lex_process)
    };

    let suffix = peekc(lex_process);
    let number_type = if suffix == 'L' {
        let _ = nextc(lex_process);
        NUMBER_TYPE_LONG
    } else if suffix == 'f' {
        let _ = nextc(lex_process);
        NUMBER_TYPE_FLOAT
    } else {
        NUMBER_TYPE_NORMAL
    };

    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_NUMBER,
            llnum: Some(value),
            num: TokenNumber { r#type: number_type },
            ..Token::default()
        },
    )
}

fn token_make_string(lex_process: &mut LexProcess, _start_delim: char, end_delim: char) -> Token {
    let _ = nextc(lex_process);
    let mut out = String::new();
    loop {
        let c = nextc(lex_process);
        if c == eof_char() || c == end_delim {
            break;
        }
        if c == '\\' {
            let escaped = nextc(lex_process);
            let resolved = match escaped {
                'n' => '\n',
                't' => '\t',
                'b' => '\u{0008}',
                '\\' => '\\',
                '\'' => '\'',
                other => other,
            };
            out.push(resolved);
            continue;
        }
        out.push(c);
    }

    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_STRING,
            sval: Some(out),
            ..Token::default()
        },
    )
}

fn is_keyword(text: &str) -> bool {
    matches!(
        text,
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

fn token_make_operator_or_symbol(lex_process: &mut LexProcess) -> Token {
    let first = nextc(lex_process);
    let is_symbol = "{}:;#\\)]".contains(first);

    if is_symbol {
        return token_create(
            lex_process,
            &Token {
                r#type: TOKEN_TYPE_SYMBOL,
                cval: Some(first),
                ..Token::default()
            },
        );
    }

    let mut op = String::from(first);
    let second = peekc(lex_process);
    if "+-/*><^%!~=|&.?[".contains(second) && !matches!(first, '(' | '[' | ',' | '.' | '*' | '?') {
        op.push(nextc(lex_process));
        if (op == "<<" || op == ">>") && peekc(lex_process) == '=' {
            op.push(nextc(lex_process));
        }
    }

    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_OPERATOR,
            sval: Some(op),
            ..Token::default()
        },
    )
}

fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut text = String::new();
    while {
        let c = peekc(lex_process);
        c.is_ascii_alphanumeric() || c == '_'
    } {
        text.push(nextc(lex_process));
    }

    let r#type = if is_keyword(&text) {
        TOKEN_TYPE_KEYWORD
    } else {
        TOKEN_TYPE_IDENTIFIER
    };
    token_create(
        lex_process,
        &Token {
            r#type,
            sval: Some(text),
            ..Token::default()
        },
    )
}

fn make_comment(lex_process: &mut LexProcess, multiline: bool) -> Token {
    let mut text = String::new();
    loop {
        let c = nextc(lex_process);
        if c == eof_char() {
            break;
        }
        if !multiline && c == '\n' {
            break;
        }
        if multiline && c == '*' && peekc(lex_process) == '/' {
            let _ = nextc(lex_process);
            break;
        }
        text.push(c);
    }

    token_create(
        lex_process,
        &Token {
            r#type: TOKEN_TYPE_COMMENT,
            sval: Some(text),
            ..Token::default()
        },
    )
}

pub fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    loop {
        let c = peekc(lex_process);
        match c {
            '\0' | '$' => return None,
            ' ' | '\t' => {
                if let Some(last) = lexer_last_token(lex_process) {
                    let idx = store_token(Token {
                        whitespace: true,
                        ..last
                    });
                    let _ = idx;
                }
                let _ = nextc(lex_process);
            }
            '\n' => {
                let _ = nextc(lex_process);
                return Some(token_create(
                    lex_process,
                    &Token {
                        r#type: TOKEN_TYPE_NEWLINE,
                        ..Token::default()
                    },
                ));
            }
            '/' => {
                let _ = nextc(lex_process);
                match peekc(lex_process) {
                    '/' => {
                        let _ = nextc(lex_process);
                        return Some(make_comment(lex_process, false));
                    }
                    '*' => {
                        let _ = nextc(lex_process);
                        return Some(make_comment(lex_process, true));
                    }
                    _ => {
                        pushc(lex_process, '/');
                        return Some(token_make_operator_or_symbol(lex_process));
                    }
                }
            }
            '\'' => {
                let _ = nextc(lex_process);
                let mut c = nextc(lex_process);
                if c == '\\' {
                    c = match nextc(lex_process) {
                        'n' => '\n',
                        't' => '\t',
                        'b' => '\u{0008}',
                        '\\' => '\\',
                        '\'' => '\'',
                        other => other,
                    };
                }
                let closing = nextc(lex_process);
                if closing != '\'' {
                    if let Some(compiler) = lex_process.compiler.as_mut() {
                        compiler_error(compiler, "You opened a quote ' but did not close it");
                    }
                }
                return Some(token_create(
                    lex_process,
                    &Token {
                        r#type: TOKEN_TYPE_NUMBER,
                        cval: Some(c),
                        llnum: Some(c as u64),
                        ..Token::default()
                    },
                ));
            }
            '"' => return Some(token_make_string(lex_process, '"', '"')),
            _ if c.is_ascii_digit() => return Some(token_make_number(lex_process)),
            _ if c.is_ascii_alphabetic() || c == '_' => {
                return Some(token_make_identifier_or_keyword(lex_process))
            }
            _ if "+-*><^%!~=|&([,.?{}:;#\\)]".contains(c) => {
                return Some(token_make_operator_or_symbol(lex_process))
            }
            _ => {
                if let Some(compiler) = lex_process.compiler.as_mut() {
                    compiler_error(compiler, "Unexpected token");
                }
                let _ = nextc(lex_process);
            }
        }
    }
}

pub fn lex(lex_process: &mut LexProcess) -> i32 {
    if let Some(compiler) = lex_process.compiler.as_ref() {
        lex_process.pos.filename = compiler.cfile.abs_path.clone();
    }

    while let Some(token) = read_next_token(lex_process) {
        let idx = store_token(token);
        if let Some(vec) = lex_process.token_vec.as_mut() {
            vector_push(vec, &encode_index(idx, std::mem::size_of::<Token>()));
        }
    }

    LEXICAL_ANALYSIS_ALL_OK
}

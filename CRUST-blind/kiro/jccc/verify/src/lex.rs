use std::io::Read;
use crate::token::{Token, TokenType};
pub const TOKEN_PUTBACKS: usize = 5;

#[derive(Debug)]
pub struct Lexer {
    pub fp: Option<std::fs::File>,
    pub current_file: String,
    pub buffer: [u8; 1],
    pub position: i64,
    pub last_column: i32,
    pub column: i32,
    pub line: i32,
    pub unlexed: [Token; TOKEN_PUTBACKS],
    pub unlexed_count: u32,
}

const SINGLE_CHAR_TOKENS: &str = "(){}[];~#,.:?~";

const OPERATOR_STRINGS: &[&str] = &[
    "-", "+", "*", "/", "=", ":", "%", "&", "&&", "|", "||",
    "-=", "+=", "++", "--", "/=", "*=", "%=", "&=", "|=", "&&=", "||=",
    ">", "<", "<=", ">=", "<<", ">>", "!", "==", "!=", "^", "^=", "->", "<<=", ">>=",
];

pub fn in_string(c: char, s: &str) -> i32 {
    if s.contains(c) { 1 } else { 0 }
}

pub fn starts_operator(c: char) -> i32 {
    match c {
        '-' | '+' | '*' | '/' | '=' | ':' | '%' | '&' | '|' | '<' | '>' | '!' | '~' | '^' => 1,
        _ => 0,
    }
}

pub fn valid_operator_sequence(op: &str) -> i32 {
    if OPERATOR_STRINGS.contains(&op) { 1 } else { 0 }
}

pub fn is_valid_numeric_or_id_char(c: char) -> i32 {
    if c.is_alphanumeric() || c == '_' || c == '.' { 1 } else { 0 }
}

pub fn lexer_getchar(l: &mut Lexer) -> i32 {
    l.position += 1;
    l.last_column = l.column;
    if let Some(ref mut fp) = l.fp {
        let mut buf = [0u8; 1];
        match fp.read(&mut buf) {
            Ok(0) | Err(_) => {
                l.buffer[0] = 0;
                return -1; // EOF
            }
            Ok(_) => {
                l.buffer[0] = buf[0];
                let ch = buf[0] as char;
                if ch == '\n' {
                    l.line += 1;
                    l.column = 0;
                } else {
                    l.column += 1;
                }
                return buf[0] as i32;
            }
        }
    }
    -1
}

pub fn lexer_ungetchar(l: &mut Lexer) -> i32 {
    l.position -= 1;
    l.column = l.last_column;
    let ch = l.buffer[0] as char;
    if ch == '\n' {
        l.line -= 1;
    }
    // Seek back one byte in the file
    if let Some(ref mut fp) = l.fp {
        use std::io::Seek;
        let _ = fp.seek(std::io::SeekFrom::Current(-1));
    }
    1
}

pub fn lex(l: &mut Lexer, token: &mut Token) -> i32 {
    loop {
        real_lex(l, token);
        if token.token_type != TokenType::TT_NEWLINE {
            break;
        }
    }
    0
}

pub fn real_lex(l: &mut Lexer, t: &mut Token) -> i32 {
    if l.unlexed_count > 0 {
        l.unlexed_count -= 1;
        let idx = l.unlexed_count as usize;
        t.token_type = std::mem::replace(&mut l.unlexed[idx].token_type, TokenType::TT_NO_TOKEN);
        t.contents = std::mem::take(&mut l.unlexed[idx].contents);
        t.length = l.unlexed[idx].length;
        t.source_file = l.unlexed[idx].source_file.clone();
        t.line = l.unlexed[idx].line;
        t.column = l.unlexed[idx].column;
        return 0;
    }

    skip_to_token(l);
    let init = lexer_getchar(l);

    t.contents.clear();
    t.source_file = l.current_file.clone();

    if init == -1 {
        t.contents = "[end of file]".to_string();
        t.length = t.contents.len();
        t.token_type = TokenType::TT_EOF;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    let init_ch = (init as u8) as char;

    if init_ch == ' ' || init_ch == '\t' {
        eprintln!("\x1b[31mError: jccc: internal error: did not skip whitespace correctly\x1b[0m");
        return -1;
    }

    if init_ch == '\n' {
        t.contents = "[newline]".to_string();
        t.length = t.contents.len();
        t.token_type = TokenType::TT_NEWLINE;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    t.contents.push(init_ch);

    if in_string(init_ch, SINGLE_CHAR_TOKENS) == 1 {
        t.length = 1;
        t.token_type = ttype_one_char(init_ch);
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    if is_valid_numeric_or_id_char(init_ch) == 1 {
        let starting_line = l.line;
        let starting_col = l.column;
        loop {
            let c = lexer_getchar(l);
            if c == -1 {
                break;
            }
            let ch = (c as u8) as char;
            if is_valid_numeric_or_id_char(ch) == 0 {
                break;
            }
            if t.contents.len() >= 255 {
                eprintln!("\x1b[31mError: jccc: identifier too long, over 256 characters\x1b[0m");
                return -1;
            }
            t.contents.push(ch);
        }
        lexer_ungetchar(l);
        t.token_type = ttype_many_chars(&t.contents);
        t.length = t.contents.len();
        t.line = starting_line;
        t.column = starting_col;
        return 0;
    }

    if starts_operator(init_ch) == 1 {
        while valid_operator_sequence(&t.contents) == 1 {
            let c = lexer_getchar(l);
            if c == -1 {
                break;
            }
            t.contents.push((c as u8) as char);
        }
        lexer_ungetchar(l);
        t.contents.pop();
        t.token_type = ttype_from_string(&t.contents);
        t.length = t.contents.len();
        return 0;
    }

    eprintln!("\x1b[31mError: jccc: lexer unable to identify token starting with: {}\x1b[0m", init_ch);
    0
}

pub fn unlex(l: &mut Lexer, t: &Token) -> i32 {
    if l.unlexed_count as usize >= TOKEN_PUTBACKS {
        eprintln!("\x1b[31mError: jccc: internal: tried to unlex more than {} tokens at a time\x1b[0m", TOKEN_PUTBACKS);
        return -1;
    }
    let idx = l.unlexed_count as usize;
    l.unlexed[idx].token_type = t.token_type.clone();
    l.unlexed[idx].contents = t.contents.clone();
    l.unlexed[idx].length = t.length;
    l.unlexed[idx].source_file = t.source_file.clone();
    l.unlexed[idx].line = t.line;
    l.unlexed[idx].column = t.column;
    l.unlexed_count += 1;
    0
}

pub fn skip_to_token(l: &mut Lexer) -> i32 {
    let mut in_block = 0;
    let mut pass: i32 = 0;

    let cur = lexer_getchar(l);
    if cur == -1 {
        return -1;
    }
    let cur_ch = (cur as u8) as char;
    let mut prev = cur_ch;
    if !(cur_ch == ' ' || cur_ch == '\t' || cur_ch == '/') {
        lexer_ungetchar(l);
        return 0;
    }

    loop {
        let cur = lexer_getchar(l);
        if cur == -1 {
            return -1;
        }
        let cur_ch = (cur as u8) as char;

        if cur_ch == '/' && prev == '/' && in_block == 0 {
            in_block = 1;
        } else if cur_ch == '*' && prev == '/' && in_block == 0 {
            in_block = 2;
            pass = 2;
        } else if (in_block == 1 && cur_ch == '\n') || (in_block == 2 && cur_ch == '/' && prev == '*' && pass <= 0) {
            in_block = 0;
        } else if prev == '/' && !(cur_ch == '*' || cur_ch == '/') && in_block == 0 {
            lexer_ungetchar(l);
            return 0;
        }
        if !(cur_ch == ' ' || cur_ch == '\t' || cur_ch == '/') && in_block == 0 {
            lexer_ungetchar(l);
            return 0;
        }

        pass -= 1;
        prev = cur_ch;
    }
}

pub fn ttype_one_char(c: char) -> TokenType {
    match c {
        '(' => TokenType::TT_OPAREN,
        ')' => TokenType::TT_CPAREN,
        '{' => TokenType::TT_OBRACE,
        '}' => TokenType::TT_CBRACE,
        '[' => TokenType::TT_OBRACKET,
        ']' => TokenType::TT_CBRACKET,
        ';' => TokenType::TT_SEMI,
        '.' => TokenType::TT_PERIOD,
        ',' => TokenType::TT_COMMA,
        '-' => TokenType::TT_MINUS,
        '+' => TokenType::TT_PLUS,
        '*' => TokenType::TT_STAR,
        '/' => TokenType::TT_SLASH,
        '=' => TokenType::TT_ASSIGN,
        ':' => TokenType::TT_COLON,
        '%' => TokenType::TT_MOD,
        '&' => TokenType::TT_BAND,
        '|' => TokenType::TT_BOR,
        '>' => TokenType::TT_GREATER,
        '<' => TokenType::TT_LESS,
        '!' => TokenType::TT_LNOT,
        '~' => TokenType::TT_BNOT,
        '^' => TokenType::TT_XOR,
        '#' => TokenType::TT_POUND,
        '?' => TokenType::TT_QMARK,
        _ => {
            if c.is_ascii_digit() {
                TokenType::TT_LITERAL
            } else {
                TokenType::TT_IDENTIFIER
            }
        }
    }
}

pub fn ttype_many_chars(contents: &str) -> TokenType {
    match contents {
        "auto" => TokenType::TT_AUTO,
        "break" => TokenType::TT_BREAK,
        "continue" => TokenType::TT_CONTINUE,
        "const" => TokenType::TT_CONST,
        "case" => TokenType::TT_CASE,
        "char" => TokenType::TT_CHAR,
        "do" => TokenType::TT_DO,
        "double" => TokenType::TT_DOUBLE,
        "default" => TokenType::TT_DEFAULT,
        "enum" => TokenType::TT_ENUM,
        "else" => TokenType::TT_ELSE,
        "extern" => TokenType::TT_EXTERN,
        "float" => TokenType::TT_FLOAT,
        "for" => TokenType::TT_FOR,
        "goto" => TokenType::TT_GOTO,
        "int" => TokenType::TT_INT,
        "if" => TokenType::TT_IF,
        "long" => TokenType::TT_LONG,
        "return" => TokenType::TT_RETURN,
        "register" => TokenType::TT_REGISTER,
        "struct" => TokenType::TT_STRUCT,
        "signed" => TokenType::TT_SIGNED,
        "sizeof" => TokenType::TT_SIZEOF,
        "static" => TokenType::TT_STATIC,
        "short" => TokenType::TT_SHORT,
        "switch" => TokenType::TT_SWITCH,
        "typedef" => TokenType::TT_TYPEDEF,
        "union" => TokenType::TT_UNION,
        "unsigned" => TokenType::TT_UNSIGNED,
        "void" => TokenType::TT_VOID,
        "volatile" => TokenType::TT_VOLATILE,
        "while" => TokenType::TT_WHILE,
        "&&" => TokenType::TT_LAND,
        "||" => TokenType::TT_LOR,
        "-=" => TokenType::TT_DEC,
        "+=" => TokenType::TT_INC,
        "++" => TokenType::TT_PLUSPLUS,
        "--" => TokenType::TT_MINUSMINUS,
        "/=" => TokenType::TT_DIVEQ,
        "*=" => TokenType::TT_MULEQ,
        "%=" => TokenType::TT_MODEQ,
        "&=" => TokenType::TT_BANDEQ,
        "|=" => TokenType::TT_BOREQ,
        "&&=" => TokenType::TT_LANDEQ,
        "||=" => TokenType::TT_LOREQ,
        "<=" => TokenType::TT_LESSEQ,
        ">=" => TokenType::TT_GREATEREQ,
        "<<" => TokenType::TT_LEFTSHIFT,
        ">>" => TokenType::TT_RIGHTSHIFT,
        "==" => TokenType::TT_EQUALS,
        "^=" => TokenType::TT_XOREQ,
        "->" => TokenType::TT_POINT,
        "<<=" => TokenType::TT_LEFTSHIFTEQUALS,
        ">>=" => TokenType::TT_RIGHTSHIFTEQUALS,
        "!=" => TokenType::TT_NOTEQ,
        _ => {
            let bytes = contents.as_bytes();
            if bytes.is_empty() {
                return TokenType::TT_NO_TOKEN;
            }
            // Check for period (float literal)
            if contents.contains('.') {
                return TokenType::TT_LITERAL;
            }
            // Check for quote literals
            if bytes[0] == b'\'' || bytes[0] == b'"' {
                return TokenType::TT_LITERAL;
            }
            let mut all_numeric = true;
            let mut count_us = 0;
            for &b in bytes {
                let c = b as char;
                if c == '\'' || c == '"' {
                    return TokenType::TT_LITERAL;
                }
                if c == 'u' {
                    count_us += 1;
                }
                if (c > '9' || c < '0') && c != 'u' {
                    all_numeric = false;
                }
            }
            if all_numeric {
                let last = bytes[bytes.len() - 1] as char;
                if count_us == 1 && last == 'u' {
                    return TokenType::TT_LITERAL;
                }
                if count_us == 0 {
                    return TokenType::TT_LITERAL;
                }
            }
            TokenType::TT_IDENTIFIER
        }
    }
}

pub fn ttype_from_string(contents: &str) -> TokenType {
    if contents.len() == 1 {
        return ttype_one_char(contents.chars().next().unwrap());
    }
    ttype_many_chars(contents)
}

const TTYPE_NAMES: &[&str] = &[
    "literal", "identifier", "open paren", "close paren", "open brace", "close brace",
    "open bracket", "close bracket", "semicolon", "no token", "end of file", "newline", "pound",
    ".", ",", "?", "-", "+", "*", "/", "=", ":", "%", "&", "&&", "|", "||",
    "-=", "+=", "++", "--", "/=", "*=", "%=", "&=", "|=", "&&=", "||=",
    ">", "<", "<=", ">=", "<<", ">>", "!", "~", "==", "!=", "^", "^=", "->", "<<=", ">>=",
    "auto", "break", "char", "const", "case", "continue", "double", "do", "default",
    "enum", "else", "extern", "float", "for", "goto", "if", "int", "long",
    "return", "register", "static", "switch", "short", "signed", "struct", "sizeof",
    "typedef", "unsigned", "union", "void", "volatile", "while",
];

pub fn ttype_name(tt: TokenType) -> String {
    let idx = tt as usize;
    if idx < TTYPE_NAMES.len() {
        TTYPE_NAMES[idx].to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn test_ttype_many_chars() -> i32 {
    assert_eq!(ttype_many_chars("foo"), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_many_chars("struct"), TokenType::TT_STRUCT);
    assert_eq!(ttype_many_chars("while"), TokenType::TT_WHILE);
    0
}

pub fn test_ttype_one_char() -> i32 {
    assert_eq!(ttype_one_char('a'), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_one_char('1'), TokenType::TT_LITERAL);
    assert_eq!(ttype_one_char('+'), TokenType::TT_PLUS);
    assert_eq!(ttype_one_char('-'), TokenType::TT_MINUS);
    assert_eq!(ttype_one_char('>'), TokenType::TT_GREATER);
    assert_eq!(ttype_one_char('~'), TokenType::TT_BNOT);
    0
}

pub fn test_ttype_name() -> i32 {
    assert_eq!(ttype_name(TokenType::TT_LITERAL), "literal");
    assert_eq!(ttype_name(TokenType::TT_PLUS), "+");
    assert_eq!(ttype_name(TokenType::TT_SIZEOF), "sizeof");
    assert_eq!(ttype_name(TokenType::TT_WHILE), "while");
    0
}

pub fn test_ttype_from_string() -> i32 {
    assert_eq!(ttype_from_string("+"), TokenType::TT_PLUS);
    assert_eq!(ttype_from_string("="), TokenType::TT_ASSIGN);
    assert_eq!(ttype_from_string("1"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("1.2"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("1u"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("1.2f"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("1.f"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("\"Planck\""), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("'Language'"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("Jaba"), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_from_string("cat_"), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_from_string("("), TokenType::TT_OPAREN);
    assert_eq!(ttype_from_string("}"), TokenType::TT_CBRACE);
    assert_eq!(ttype_from_string(";"), TokenType::TT_SEMI);
    0
}

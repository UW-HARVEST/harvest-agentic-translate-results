use crate::token::{Token, TokenType, TOKEN_LENGTH};
use std::io::{Read, Seek, SeekFrom};

pub const TOKEN_PUTBACKS: usize = 5;

/// Represents a lexer for tokenizing input.
#[derive(Debug)]
pub struct Lexer {
    /// An optional file; in C this would be a FILE*, here we keep it as a File for demonstration.
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
    "-", "+", "*", "/", "=", ":", "%", "&", "&&", "|", "||", "-=", "+=", "++", "--", "/=", "*=",
    "%=", "&=", "|=", "&&=", "||=", ">", "<", "<=", ">=", "<<", ">>", "!", "==", "!=", "^", "^=",
    "->", "<<=", ">>=",
];

const TTYPE_NAMES: &[&str] = &[
    "literal",
    "identifier",
    "open paren",
    "close paren",
    "open brace",
    "close brace",
    "open bracket",
    "close bracket",
    "semicolon",
    "no token",
    "end of file",
    "newline",
    "pound",
    ".",
    ",",
    "?",
    "-",
    "+",
    "*",
    "/",
    "=",
    ":",
    "%",
    "&",
    "&&",
    "|",
    "||",
    "-=",
    "+=",
    "++",
    "--",
    "/=",
    "*=",
    "%=",
    "&=",
    "|=",
    "&&=",
    "||=",
    ">",
    "<",
    "<=",
    ">=",
    "<<",
    ">>",
    "!",
    "~",
    "==",
    "!=",
    "^",
    "^=",
    "->",
    "<<=",
    ">>=",
    "auto",
    "break",
    "char",
    "const",
    "case",
    "continue",
    "double",
    "do",
    "default",
    "enum",
    "else",
    "extern",
    "float",
    "for",
    "goto",
    "if",
    "int",
    "long",
    "return",
    "register",
    "static",
    "switch",
    "short",
    "signed",
    "struct",
    "sizeof",
    "typedef",
    "unsigned",
    "union",
    "void",
    "volatile",
    "while",
];

fn ttype_index(tt: &TokenType) -> usize {
    match tt {
        TokenType::TT_LITERAL => 0,
        TokenType::TT_IDENTIFIER => 1,
        TokenType::TT_OPAREN => 2,
        TokenType::TT_CPAREN => 3,
        TokenType::TT_OBRACE => 4,
        TokenType::TT_CBRACE => 5,
        TokenType::TT_OBRACKET => 6,
        TokenType::TT_CBRACKET => 7,
        TokenType::TT_SEMI => 8,
        TokenType::TT_NO_TOKEN => 9,
        TokenType::TT_EOF => 10,
        TokenType::TT_NEWLINE => 11,
        TokenType::TT_POUND => 12,
        TokenType::TT_PERIOD => 13,
        TokenType::TT_COMMA => 14,
        TokenType::TT_QMARK => 15,
        TokenType::TT_MINUS => 16,
        TokenType::TT_PLUS => 17,
        TokenType::TT_STAR => 18,
        TokenType::TT_SLASH => 19,
        TokenType::TT_ASSIGN => 20,
        TokenType::TT_COLON => 21,
        TokenType::TT_MOD => 22,
        TokenType::TT_BAND => 23,
        TokenType::TT_LAND => 24,
        TokenType::TT_BOR => 25,
        TokenType::TT_LOR => 26,
        TokenType::TT_DEC => 27,
        TokenType::TT_INC => 28,
        TokenType::TT_PLUSPLUS => 29,
        TokenType::TT_MINUSMINUS => 30,
        TokenType::TT_DIVEQ => 31,
        TokenType::TT_MULEQ => 32,
        TokenType::TT_MODEQ => 33,
        TokenType::TT_BANDEQ => 34,
        TokenType::TT_BOREQ => 35,
        TokenType::TT_LANDEQ => 36,
        TokenType::TT_LOREQ => 37,
        TokenType::TT_GREATER => 38,
        TokenType::TT_LESS => 39,
        TokenType::TT_LESSEQ => 40,
        TokenType::TT_GREATEREQ => 41,
        TokenType::TT_LEFTSHIFT => 42,
        TokenType::TT_RIGHTSHIFT => 43,
        TokenType::TT_LNOT => 44,
        TokenType::TT_BNOT => 45,
        TokenType::TT_EQUALS => 46,
        TokenType::TT_NOTEQ => 47,
        TokenType::TT_XOR => 48,
        TokenType::TT_XOREQ => 49,
        TokenType::TT_POINT => 50,
        TokenType::TT_LEFTSHIFTEQUALS => 51,
        TokenType::TT_RIGHTSHIFTEQUALS => 52,
        TokenType::TT_AUTO => 53,
        TokenType::TT_BREAK => 54,
        TokenType::TT_CHAR => 55,
        TokenType::TT_CONST => 56,
        TokenType::TT_CASE => 57,
        TokenType::TT_CONTINUE => 58,
        TokenType::TT_DOUBLE => 59,
        TokenType::TT_DO => 60,
        TokenType::TT_DEFAULT => 61,
        TokenType::TT_ENUM => 62,
        TokenType::TT_ELSE => 63,
        TokenType::TT_EXTERN => 64,
        TokenType::TT_FLOAT => 65,
        TokenType::TT_FOR => 66,
        TokenType::TT_GOTO => 67,
        TokenType::TT_IF => 68,
        TokenType::TT_INT => 69,
        TokenType::TT_LONG => 70,
        TokenType::TT_RETURN => 71,
        TokenType::TT_REGISTER => 72,
        TokenType::TT_STATIC => 73,
        TokenType::TT_SWITCH => 74,
        TokenType::TT_SHORT => 75,
        TokenType::TT_SIGNED => 76,
        TokenType::TT_STRUCT => 77,
        TokenType::TT_SIZEOF => 78,
        TokenType::TT_TYPEDEF => 79,
        TokenType::TT_UNSIGNED => 80,
        TokenType::TT_UNION => 81,
        TokenType::TT_VOID => 82,
        TokenType::TT_VOLATILE => 83,
        TokenType::TT_WHILE => 84,
    }
}

/// Gets the next character from the lexer.
pub fn lexer_getchar(l: &mut Lexer) -> i32 {
    l.position += 1;
    l.last_column = l.column;
    let c = if let Some(fp) = l.fp.as_mut() {
        let mut buf = [0u8; 1];
        match fp.read(&mut buf) {
            Ok(0) => -1i32, // EOF
            Ok(_) => buf[0] as i32,
            Err(_) => -1i32,
        }
    } else {
        -1i32
    };
    if c == -1 {
        l.buffer[0] = 0;
    } else {
        l.buffer[0] = c as u8;
    }
    if c == b'\n' as i32 {
        l.line += 1;
        l.column = 0;
    } else {
        l.column += 1;
    }
    c
}

/// Retrieves the next token from the lexer.
pub fn real_lex(l: &mut Lexer, t: &mut Token) -> i32 {
    if l.unlexed_count > 0 {
        l.unlexed_count -= 1;
        let idx = l.unlexed_count as usize;
        // Move out using std::mem::replace with a default token
        let saved = std::mem::replace(
            &mut l.unlexed[idx],
            Token {
                token_type: TokenType::TT_NO_TOKEN,
                contents: String::new(),
                length: 0,
                source_file: String::new(),
                line: 0,
                column: 0,
            },
        );
        *t = saved;
        return 0;
    }

    skip_to_token(l);
    let init = lexer_getchar(l);

    t.contents.clear();
    t.source_file = l.current_file.clone();

    if init == -1 {
        t.contents = "[end of file]".to_string();
        t.length = "[end of file]".len();
        t.token_type = TokenType::TT_EOF;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    if init == b' ' as i32 || init == b'\t' as i32 {
        return -1;
    }

    if init == b'\n' as i32 {
        t.contents = "[newline]".to_string();
        t.length = "[newline]".len();
        t.token_type = TokenType::TT_NEWLINE;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    let init_c = init as u8 as char;
    let mut contents = String::new();
    contents.push(init_c);

    if in_string_internal(init_c, SINGLE_CHAR_TOKENS) {
        t.length = contents.len();
        t.token_type = ttype_one_char(init_c);
        t.contents = contents;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    if is_valid_numeric_or_id_char_internal(init_c) {
        let starting_line = l.line;
        let starting_col = l.column;
        loop {
            let c = lexer_getchar(l);
            if c == -1 {
                break;
            }
            let cc = c as u8 as char;
            if !is_valid_numeric_or_id_char_internal(cc) {
                break;
            }
            if contents.len() >= TOKEN_LENGTH - 1 {
                return -1;
            }
            contents.push(cc);
        }
        lexer_ungetchar(l);
        t.length = contents.len();
        t.token_type = ttype_many_chars(&contents);
        t.contents = contents;
        t.line = starting_line;
        t.column = starting_col;
        return 0;
    }

    if starts_operator_internal(init_c) {
        // C semantics: while contents is a valid operator, append next char.
        // Then drop the trailing char (truncating string by 1).
        loop {
            if !valid_operator_sequence_internal(&contents) {
                break;
            }
            let c = lexer_getchar(l);
            if c == -1 {
                contents.push('\0');
                break;
            }
            contents.push(c as u8 as char);
        }
        lexer_ungetchar(l);
        // Truncate the last char (mimicking t->contents[pos - 1] = '\0')
        let pos = contents.len();
        if pos > 0 {
            // t->length = pos in C, mimic before truncation
            t.length = pos;
            contents.truncate(pos - 1);
        } else {
            t.length = 0;
        }
        t.token_type = ttype_from_string(&contents);
        t.contents = contents;
        // Note: C does not set line/column here — leave whatever was there.
        return 0;
    }

    // Could not identify
    t.contents = contents;
    t.token_type = TokenType::TT_NO_TOKEN;
    t.length = 1;
    t.line = l.line;
    t.column = l.column;
    0
}

/// Un-gets (pushes back) the last character.
pub fn lexer_ungetchar(l: &mut Lexer) -> i32 {
    if l.position < 0 {
        return -1;
    }
    l.position -= 1;
    l.column = l.last_column;
    if l.buffer[0] == b'\n' {
        l.line -= 1;
    }
    if let Some(fp) = l.fp.as_mut() {
        let _ = fp.seek(SeekFrom::Current(-1));
    }
    1
}

/// Determines a token type from a multi-character sequence.
pub fn ttype_many_chars(contents: &str) -> TokenType {
    match contents {
        "auto" => return TokenType::TT_AUTO,
        "break" => return TokenType::TT_BREAK,
        "continue" => return TokenType::TT_CONTINUE,
        "const" => return TokenType::TT_CONST,
        "case" => return TokenType::TT_CASE,
        "char" => return TokenType::TT_CHAR,
        "do" => return TokenType::TT_DO,
        "double" => return TokenType::TT_DOUBLE,
        "default" => return TokenType::TT_DEFAULT,
        "enum" => return TokenType::TT_ENUM,
        "else" => return TokenType::TT_ELSE,
        "extern" => return TokenType::TT_EXTERN,
        "float" => return TokenType::TT_FLOAT,
        "for" => return TokenType::TT_FOR,
        "goto" => return TokenType::TT_GOTO,
        "int" => return TokenType::TT_INT,
        "if" => return TokenType::TT_IF,
        "long" => return TokenType::TT_LONG,
        "return" => return TokenType::TT_RETURN,
        "register" => return TokenType::TT_REGISTER,
        "struct" => return TokenType::TT_STRUCT,
        "signed" => return TokenType::TT_SIGNED,
        "sizeof" => return TokenType::TT_SIZEOF,
        "static" => return TokenType::TT_STATIC,
        "short" => return TokenType::TT_SHORT,
        "switch" => return TokenType::TT_SWITCH,
        "typedef" => return TokenType::TT_TYPEDEF,
        "union" => return TokenType::TT_UNION,
        "unsigned" => return TokenType::TT_UNSIGNED,
        "void" => return TokenType::TT_VOID,
        "volatile" => return TokenType::TT_VOLATILE,
        "while" => return TokenType::TT_WHILE,
        "&&" => return TokenType::TT_LAND,
        "||" => return TokenType::TT_LOR,
        "-=" => return TokenType::TT_DEC,
        "+=" => return TokenType::TT_INC,
        "++" => return TokenType::TT_PLUSPLUS,
        "--" => return TokenType::TT_MINUSMINUS,
        "/=" => return TokenType::TT_DIVEQ,
        "*=" => return TokenType::TT_MULEQ,
        "%=" => return TokenType::TT_MODEQ,
        "&=" => return TokenType::TT_BANDEQ,
        "|=" => return TokenType::TT_BOREQ,
        "&&=" => return TokenType::TT_LANDEQ,
        "||=" => return TokenType::TT_LOREQ,
        "<=" => return TokenType::TT_LESSEQ,
        ">=" => return TokenType::TT_GREATEREQ,
        "<<" => return TokenType::TT_LEFTSHIFT,
        ">>" => return TokenType::TT_RIGHTSHIFT,
        "==" => return TokenType::TT_EQUALS,
        "^=" => return TokenType::TT_XOREQ,
        "->" => return TokenType::TT_POINT,
        "<<=" => return TokenType::TT_LEFTSHIFTEQUALS,
        ">>=" => return TokenType::TT_RIGHTSHIFTEQUALS,
        "!=" => return TokenType::TT_NOTEQ,
        _ => {}
    }

    if contents.is_empty() {
        return TokenType::TT_NO_TOKEN;
    }

    let mut all_numeric = 1;
    let mut count_fs = 0;
    let mut count_us = 0;

    let bytes = contents.as_bytes();
    for &c in bytes {
        if c == b'.' {
            return TokenType::TT_LITERAL;
        }
        if c == b'f' {
            count_fs += 1;
        }
        if c == b'u' {
            count_us += 1;
        }
        if (c > b'9' || c < b'0') && c != b'u' {
            all_numeric = 0;
        }
        if c == b'\'' || c == b'"' {
            return TokenType::TT_LITERAL;
        }
    }
    let _ = count_fs;

    if all_numeric == 1 {
        let last = *bytes.last().unwrap();
        if count_us == 1 && last == b'u' {
            return TokenType::TT_LITERAL;
        }
        if count_us == 0 {
            return TokenType::TT_LITERAL;
        }
    }

    TokenType::TT_IDENTIFIER
}

/// Tests the function that identifies token types by name.
pub fn test_ttype_name() -> i32 {
    if ttype_name(TokenType::TT_LITERAL) != "literal" {
        return -1;
    }
    if ttype_name(TokenType::TT_PLUS) != "+" {
        return -1;
    }
    if ttype_name(TokenType::TT_SIZEOF) != "sizeof" {
        return -1;
    }
    if ttype_name(TokenType::TT_WHILE) != "while" {
        return -1;
    }
    0
}

/// Determines a token type from a single character.
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
        c if c.is_ascii_digit() => TokenType::TT_LITERAL,
        _ => TokenType::TT_IDENTIFIER,
    }
}

/// Returns the name of a token type as a string.
pub fn ttype_name(tt: TokenType) -> String {
    let i = ttype_index(&tt);
    TTYPE_NAMES[i].to_string()
}

fn valid_operator_sequence_internal(op: &str) -> bool {
    OPERATOR_STRINGS.contains(&op)
}

/// Checks if the provided operator sequence is valid.
pub fn valid_operator_sequence(op: &str) -> i32 {
    if valid_operator_sequence_internal(op) {
        1
    } else {
        0
    }
}

/// Main lex function to tokenize input into a given Token.
pub fn lex(l: &mut Lexer, token: &mut Token) -> i32 {
    loop {
        let r = real_lex(l, token);
        if r != 0 {
            return r;
        }
        if token.token_type != TokenType::TT_NEWLINE {
            break;
        }
    }
    0
}

/// Tests the function that determines token types for a single character.
pub fn test_ttype_one_char() -> i32 {
    if ttype_one_char('a') != TokenType::TT_IDENTIFIER {
        return -1;
    }
    if ttype_one_char('1') != TokenType::TT_LITERAL {
        return -1;
    }
    if ttype_one_char('+') != TokenType::TT_PLUS {
        return -1;
    }
    if ttype_one_char('-') != TokenType::TT_MINUS {
        return -1;
    }
    if ttype_one_char('>') != TokenType::TT_GREATER {
        return -1;
    }
    if ttype_one_char('~') != TokenType::TT_BNOT {
        return -1;
    }
    0
}

fn starts_operator_internal(c: char) -> bool {
    matches!(
        c,
        '-' | '+'
            | '*'
            | '/'
            | '='
            | ':'
            | '%'
            | '&'
            | '|'
            | '<'
            | '>'
            | '!'
            | '~'
            | '^'
    )
}

/// Checks if a character starts an operator sequence.
pub fn starts_operator(c: char) -> i32 {
    if starts_operator_internal(c) {
        1
    } else {
        0
    }
}

/// Tests the function that determines token types from a string.
pub fn test_ttype_from_string() -> i32 {
    let cases: &[(&str, TokenType)] = &[
        ("+", TokenType::TT_PLUS),
        ("=", TokenType::TT_ASSIGN),
        ("1", TokenType::TT_LITERAL),
        ("1.2", TokenType::TT_LITERAL),
        ("1u", TokenType::TT_LITERAL),
        ("1.2f", TokenType::TT_LITERAL),
        ("1.f", TokenType::TT_LITERAL),
        ("\"Planck\"", TokenType::TT_LITERAL),
        ("'Language'", TokenType::TT_LITERAL),
        ("Jaba", TokenType::TT_IDENTIFIER),
        ("cat_", TokenType::TT_IDENTIFIER),
        ("(", TokenType::TT_OPAREN),
        ("}", TokenType::TT_CBRACE),
        (";", TokenType::TT_SEMI),
    ];
    for (s, expected) in cases {
        if ttype_from_string(s) != *expected {
            return -1;
        }
    }
    0
}

/// Derives a TokenType from a string input.
pub fn ttype_from_string(contents: &str) -> TokenType {
    let len = contents.chars().count();
    if len == 1 {
        return ttype_one_char(contents.chars().next().unwrap());
    }
    ttype_many_chars(contents)
}

fn in_string_internal(c: char, s: &str) -> bool {
    s.contains(c)
}

/// Checks if character c is in the string s.
pub fn in_string(c: char, s: &str) -> i32 {
    if in_string_internal(c, s) {
        1
    } else {
        0
    }
}

/// Tests the function that determines token types from multiple characters.
pub fn test_ttype_many_chars() -> i32 {
    if ttype_many_chars("foo") != TokenType::TT_IDENTIFIER {
        return -1;
    }
    if ttype_many_chars("struct") != TokenType::TT_STRUCT {
        return -1;
    }
    if ttype_many_chars("while") != TokenType::TT_WHILE {
        return -1;
    }
    0
}

/// Skips characters until the next token is found.
pub fn skip_to_token(l: &mut Lexer) -> i32 {
    let mut prev: i32;
    let mut in_block = 0;
    let mut pass = 0;

    let cur = lexer_getchar(l);
    if cur == -1 {
        return -1;
    }
    prev = cur;
    if !(cur == b' ' as i32 || cur == b'\t' as i32 || cur == b'/' as i32) {
        lexer_ungetchar(l);
        return 0;
    }

    loop {
        let cur = lexer_getchar(l);
        if cur == -1 {
            return -1;
        }
        let curb = cur as u8;

        if curb == b'/' && prev == b'/' as i32 && in_block == 0 {
            in_block = 1;
        } else if curb == b'*' && prev == b'/' as i32 && in_block == 0 {
            in_block = 2;
            pass = 2;
        } else if (in_block == 1 && curb == b'\n')
            || (in_block == 2 && curb == b'/' && prev == b'*' as i32 && pass <= 0)
        {
            in_block = 0;
        } else if prev == b'/' as i32
            && !(curb == b'*' || curb == b'/')
            && in_block == 0
        {
            lexer_ungetchar(l);
            return 0;
        }

        if !(curb == b' ' || curb == b'\t' || curb == b'/') && in_block == 0 {
            lexer_ungetchar(l);
            return 0;
        }

        pass -= 1;
        prev = cur;
    }
}

/// Pushes a token back into the lexer's buffer.
pub fn unlex(l: &mut Lexer, t: &Token) -> i32 {
    if l.unlexed_count as usize >= TOKEN_PUTBACKS {
        return -1;
    }
    let idx = l.unlexed_count as usize;
    l.unlexed[idx] = Token {
        token_type: clone_tt(&t.token_type),
        contents: t.contents.clone(),
        length: t.length,
        source_file: t.source_file.clone(),
        line: t.line,
        column: t.column,
    };
    l.unlexed_count += 1;
    0
}

fn clone_tt(tt: &TokenType) -> TokenType {
    match tt {
        TokenType::TT_LITERAL => TokenType::TT_LITERAL,
        TokenType::TT_IDENTIFIER => TokenType::TT_IDENTIFIER,
        TokenType::TT_OPAREN => TokenType::TT_OPAREN,
        TokenType::TT_CPAREN => TokenType::TT_CPAREN,
        TokenType::TT_OBRACE => TokenType::TT_OBRACE,
        TokenType::TT_CBRACE => TokenType::TT_CBRACE,
        TokenType::TT_OBRACKET => TokenType::TT_OBRACKET,
        TokenType::TT_CBRACKET => TokenType::TT_CBRACKET,
        TokenType::TT_SEMI => TokenType::TT_SEMI,
        TokenType::TT_NO_TOKEN => TokenType::TT_NO_TOKEN,
        TokenType::TT_EOF => TokenType::TT_EOF,
        TokenType::TT_NEWLINE => TokenType::TT_NEWLINE,
        TokenType::TT_POUND => TokenType::TT_POUND,
        TokenType::TT_PERIOD => TokenType::TT_PERIOD,
        TokenType::TT_COMMA => TokenType::TT_COMMA,
        TokenType::TT_QMARK => TokenType::TT_QMARK,
        TokenType::TT_MINUS => TokenType::TT_MINUS,
        TokenType::TT_PLUS => TokenType::TT_PLUS,
        TokenType::TT_STAR => TokenType::TT_STAR,
        TokenType::TT_SLASH => TokenType::TT_SLASH,
        TokenType::TT_ASSIGN => TokenType::TT_ASSIGN,
        TokenType::TT_COLON => TokenType::TT_COLON,
        TokenType::TT_MOD => TokenType::TT_MOD,
        TokenType::TT_BAND => TokenType::TT_BAND,
        TokenType::TT_LAND => TokenType::TT_LAND,
        TokenType::TT_BOR => TokenType::TT_BOR,
        TokenType::TT_LOR => TokenType::TT_LOR,
        TokenType::TT_DEC => TokenType::TT_DEC,
        TokenType::TT_INC => TokenType::TT_INC,
        TokenType::TT_PLUSPLUS => TokenType::TT_PLUSPLUS,
        TokenType::TT_MINUSMINUS => TokenType::TT_MINUSMINUS,
        TokenType::TT_DIVEQ => TokenType::TT_DIVEQ,
        TokenType::TT_MULEQ => TokenType::TT_MULEQ,
        TokenType::TT_MODEQ => TokenType::TT_MODEQ,
        TokenType::TT_BANDEQ => TokenType::TT_BANDEQ,
        TokenType::TT_BOREQ => TokenType::TT_BOREQ,
        TokenType::TT_LANDEQ => TokenType::TT_LANDEQ,
        TokenType::TT_LOREQ => TokenType::TT_LOREQ,
        TokenType::TT_GREATER => TokenType::TT_GREATER,
        TokenType::TT_LESS => TokenType::TT_LESS,
        TokenType::TT_LESSEQ => TokenType::TT_LESSEQ,
        TokenType::TT_GREATEREQ => TokenType::TT_GREATEREQ,
        TokenType::TT_LEFTSHIFT => TokenType::TT_LEFTSHIFT,
        TokenType::TT_RIGHTSHIFT => TokenType::TT_RIGHTSHIFT,
        TokenType::TT_LNOT => TokenType::TT_LNOT,
        TokenType::TT_BNOT => TokenType::TT_BNOT,
        TokenType::TT_EQUALS => TokenType::TT_EQUALS,
        TokenType::TT_NOTEQ => TokenType::TT_NOTEQ,
        TokenType::TT_XOR => TokenType::TT_XOR,
        TokenType::TT_XOREQ => TokenType::TT_XOREQ,
        TokenType::TT_POINT => TokenType::TT_POINT,
        TokenType::TT_LEFTSHIFTEQUALS => TokenType::TT_LEFTSHIFTEQUALS,
        TokenType::TT_RIGHTSHIFTEQUALS => TokenType::TT_RIGHTSHIFTEQUALS,
        TokenType::TT_AUTO => TokenType::TT_AUTO,
        TokenType::TT_BREAK => TokenType::TT_BREAK,
        TokenType::TT_CHAR => TokenType::TT_CHAR,
        TokenType::TT_CONST => TokenType::TT_CONST,
        TokenType::TT_CASE => TokenType::TT_CASE,
        TokenType::TT_CONTINUE => TokenType::TT_CONTINUE,
        TokenType::TT_DOUBLE => TokenType::TT_DOUBLE,
        TokenType::TT_DO => TokenType::TT_DO,
        TokenType::TT_DEFAULT => TokenType::TT_DEFAULT,
        TokenType::TT_ENUM => TokenType::TT_ENUM,
        TokenType::TT_ELSE => TokenType::TT_ELSE,
        TokenType::TT_EXTERN => TokenType::TT_EXTERN,
        TokenType::TT_FLOAT => TokenType::TT_FLOAT,
        TokenType::TT_FOR => TokenType::TT_FOR,
        TokenType::TT_GOTO => TokenType::TT_GOTO,
        TokenType::TT_IF => TokenType::TT_IF,
        TokenType::TT_INT => TokenType::TT_INT,
        TokenType::TT_LONG => TokenType::TT_LONG,
        TokenType::TT_RETURN => TokenType::TT_RETURN,
        TokenType::TT_REGISTER => TokenType::TT_REGISTER,
        TokenType::TT_STATIC => TokenType::TT_STATIC,
        TokenType::TT_SWITCH => TokenType::TT_SWITCH,
        TokenType::TT_SHORT => TokenType::TT_SHORT,
        TokenType::TT_SIGNED => TokenType::TT_SIGNED,
        TokenType::TT_STRUCT => TokenType::TT_STRUCT,
        TokenType::TT_SIZEOF => TokenType::TT_SIZEOF,
        TokenType::TT_TYPEDEF => TokenType::TT_TYPEDEF,
        TokenType::TT_UNSIGNED => TokenType::TT_UNSIGNED,
        TokenType::TT_UNION => TokenType::TT_UNION,
        TokenType::TT_VOID => TokenType::TT_VOID,
        TokenType::TT_VOLATILE => TokenType::TT_VOLATILE,
        TokenType::TT_WHILE => TokenType::TT_WHILE,
    }
}

fn is_valid_numeric_or_id_char_internal(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

/// Checks if c is a valid numeric or identifier character.
pub fn is_valid_numeric_or_id_char(c: char) -> i32 {
    if is_valid_numeric_or_id_char_internal(c) {
        1
    } else {
        0
    }
}

use crate::token::{Token, TokenType, TOKEN_LENGTH};
use std::io::{Read, Seek, SeekFrom};

pub const TOKEN_PUTBACKS: usize = 5;

const SINGLE_CHAR_TOKENS: &str = "(){}[];~#,.:?~";

const OPERATOR_STRINGS: &[&str] = &[
    "-", "+", "*", "/", "=", ":", "%", "&", "&&", "|", "||", "-=", "+=", "++", "--", "/=", "*=",
    "%=", "&=", "|=", "&&=", "||=", ">", "<", "<=", ">=", "<<", ">>", "!", "==", "!=", "^", "^=",
    "->", "<<=", ">>=",
];

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

/// EOF marker we use internally — `-1`, like C's EOF.
const EOF: i32 = -1;

fn empty_token() -> Token {
    Token {
        token_type: TokenType::TT_NO_TOKEN,
        contents: String::new(),
        length: 0,
        source_file: String::new(),
        line: 0,
        column: 0,
    }
}

/// Helper: read one byte from the file (no buffering); returns -1 on EOF.
fn read_one(l: &mut Lexer) -> i32 {
    let mut buf = [0u8; 1];
    match l.fp.as_mut() {
        Some(f) => match f.read(&mut buf) {
            Ok(0) => EOF,
            Ok(_) => buf[0] as i32,
            Err(_) => EOF,
        },
        None => EOF,
    }
}

/// Gets the next character from the lexer.
pub fn lexer_getchar(l: &mut Lexer) -> i32 {
    l.position += 1;
    l.last_column = l.column;
    let c = read_one(l);
    if c == EOF {
        // Mirror getc returning EOF; do not advance line/column for EOF.
        l.buffer[0] = 0;
        return EOF;
    }
    l.buffer[0] = c as u8;
    if l.buffer[0] == b'\n' {
        l.line += 1;
        l.column = 0;
    } else {
        l.column += 1;
    }
    l.buffer[0] as i32
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
    if let Some(f) = l.fp.as_mut() {
        // Seek back one byte if possible.
        let _ = f.seek(SeekFrom::Current(-1));
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

    let mut all_numeric = true;
    let mut count_us = 0;
    let mut last_char: char = '\0';

    for c in contents.chars() {
        last_char = c;
        if c == '.' {
            return TokenType::TT_LITERAL;
        }
        if c == 'u' {
            count_us += 1;
        }
        // Equivalent of: ((c > '9' || c < '0') && c != 'u')
        if (c > '9' || c < '0') && c != 'u' {
            all_numeric = false;
        }
        if c == '\'' || c == '"' {
            return TokenType::TT_LITERAL;
        }
    }

    if all_numeric {
        // 100u
        if count_us == 1 && last_char == 'u' {
            return TokenType::TT_LITERAL;
        }
        // 100
        if count_us == 0 {
            return TokenType::TT_LITERAL;
        }
    }

    TokenType::TT_IDENTIFIER
}

/// Tests the function that identifies token types by name.
pub fn test_ttype_name() -> i32 {
    if ttype_name(TokenType::TT_LITERAL) != "literal" {
        return 1;
    }
    if ttype_name(TokenType::TT_PLUS) != "+" {
        return 1;
    }
    if ttype_name(TokenType::TT_SIZEOF) != "sizeof" {
        return 1;
    }
    if ttype_name(TokenType::TT_WHILE) != "while" {
        return 1;
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
        _ => {
            if c.is_ascii_digit() {
                TokenType::TT_LITERAL
            } else {
                TokenType::TT_IDENTIFIER
            }
        }
    }
}

/// Returns the name of a token type as a string.
pub fn ttype_name(tt: TokenType) -> String {
    let s = match tt {
        TokenType::TT_LITERAL => "literal",
        TokenType::TT_IDENTIFIER => "identifier",
        TokenType::TT_OPAREN => "open paren",
        TokenType::TT_CPAREN => "close paren",
        TokenType::TT_OBRACE => "open brace",
        TokenType::TT_CBRACE => "close brace",
        TokenType::TT_OBRACKET => "open bracket",
        TokenType::TT_CBRACKET => "close bracket",
        TokenType::TT_SEMI => "semicolon",
        TokenType::TT_NO_TOKEN => "no token",
        TokenType::TT_EOF => "end of file",
        TokenType::TT_NEWLINE => "newline",
        TokenType::TT_POUND => "pound",
        TokenType::TT_PERIOD => ".",
        TokenType::TT_COMMA => ",",
        TokenType::TT_QMARK => "?",
        TokenType::TT_MINUS => "-",
        TokenType::TT_PLUS => "+",
        TokenType::TT_STAR => "*",
        TokenType::TT_SLASH => "/",
        TokenType::TT_ASSIGN => "=",
        TokenType::TT_COLON => ":",
        TokenType::TT_MOD => "%",
        TokenType::TT_BAND => "&",
        TokenType::TT_LAND => "&&",
        TokenType::TT_BOR => "|",
        TokenType::TT_LOR => "||",
        TokenType::TT_DEC => "-=",
        TokenType::TT_INC => "+=",
        TokenType::TT_PLUSPLUS => "++",
        TokenType::TT_MINUSMINUS => "--",
        TokenType::TT_DIVEQ => "/=",
        TokenType::TT_MULEQ => "*=",
        TokenType::TT_MODEQ => "%=",
        TokenType::TT_BANDEQ => "&=",
        TokenType::TT_BOREQ => "|=",
        TokenType::TT_LANDEQ => "&&=",
        TokenType::TT_LOREQ => "||=",
        TokenType::TT_GREATER => ">",
        TokenType::TT_LESS => "<",
        TokenType::TT_LESSEQ => "<=",
        TokenType::TT_GREATEREQ => ">=",
        TokenType::TT_LEFTSHIFT => "<<",
        TokenType::TT_RIGHTSHIFT => ">>",
        TokenType::TT_LNOT => "!",
        TokenType::TT_BNOT => "~",
        TokenType::TT_EQUALS => "==",
        TokenType::TT_NOTEQ => "!=",
        TokenType::TT_XOR => "^",
        TokenType::TT_XOREQ => "^=",
        TokenType::TT_POINT => "->",
        TokenType::TT_LEFTSHIFTEQUALS => "<<=",
        TokenType::TT_RIGHTSHIFTEQUALS => ">>=",
        TokenType::TT_AUTO => "auto",
        TokenType::TT_BREAK => "break",
        TokenType::TT_CHAR => "char",
        TokenType::TT_CONST => "const",
        TokenType::TT_CASE => "case",
        TokenType::TT_CONTINUE => "continue",
        TokenType::TT_DOUBLE => "double",
        TokenType::TT_DO => "do",
        TokenType::TT_DEFAULT => "default",
        TokenType::TT_ENUM => "enum",
        TokenType::TT_ELSE => "else",
        TokenType::TT_EXTERN => "extern",
        TokenType::TT_FLOAT => "float",
        TokenType::TT_FOR => "for",
        TokenType::TT_GOTO => "goto",
        TokenType::TT_IF => "if",
        TokenType::TT_INT => "int",
        TokenType::TT_LONG => "long",
        TokenType::TT_RETURN => "return",
        TokenType::TT_REGISTER => "register",
        TokenType::TT_STATIC => "static",
        TokenType::TT_SWITCH => "switch",
        TokenType::TT_SHORT => "short",
        TokenType::TT_SIGNED => "signed",
        TokenType::TT_STRUCT => "struct",
        TokenType::TT_SIZEOF => "sizeof",
        TokenType::TT_TYPEDEF => "typedef",
        TokenType::TT_UNSIGNED => "unsigned",
        TokenType::TT_UNION => "union",
        TokenType::TT_VOID => "void",
        TokenType::TT_VOLATILE => "volatile",
        TokenType::TT_WHILE => "while",
    };
    s.to_string()
}

/// Checks if the provided operator sequence is valid.
pub fn valid_operator_sequence(op: &str) -> i32 {
    for &s in OPERATOR_STRINGS {
        if s == op {
            return 1;
        }
    }
    0
}

/// Main lex function to tokenize input into a given Token.
pub fn lex(l: &mut Lexer, token: &mut Token) -> i32 {
    loop {
        let r = real_lex(l, token);
        if r != 0 {
            return r;
        }
        if !matches!(token.token_type, TokenType::TT_NEWLINE) {
            break;
        }
    }
    0
}

/// Retrieves the next token from the lexer.
pub fn real_lex(l: &mut Lexer, t: &mut Token) -> i32 {
    // Pull from putback buffer if available.
    if l.unlexed_count > 0 {
        l.unlexed_count -= 1;
        let idx = l.unlexed_count as usize;
        // Swap out the stored token to preserve lifetimes.
        let stored = std::mem::replace(&mut l.unlexed[idx], empty_token());
        *t = stored;
        return 0;
    }

    skip_to_token(l);
    let init = lexer_getchar(l);

    t.contents.clear();
    t.source_file = l.current_file.clone();

    if init == EOF {
        let eof = "[end of file]";
        t.contents = eof.to_string();
        t.length = eof.len();
        t.token_type = TokenType::TT_EOF;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    let init_c = init as u8 as char;

    if init_c == ' ' || init_c == '\t' {
        eprintln!("Error: jccc: internal error: did not skip whitespace correctly");
        return -1;
    }

    if init_c == '\n' {
        let nl = "[newline]";
        t.contents = nl.to_string();
        t.length = nl.len();
        t.token_type = TokenType::TT_NEWLINE;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    let mut buf = String::new();
    buf.push(init_c);

    if in_string(init_c, SINGLE_CHAR_TOKENS) != 0 {
        t.length = buf.len();
        t.token_type = ttype_one_char(init_c);
        t.contents = buf;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    if is_valid_numeric_or_id_char(init_c) != 0 {
        let starting_line = l.line;
        let starting_col = l.column;
        loop {
            let c = lexer_getchar(l);
            if c == EOF {
                break;
            }
            let cc = c as u8 as char;
            if is_valid_numeric_or_id_char(cc) == 0 {
                lexer_ungetchar(l);
                break;
            }
            if buf.len() >= TOKEN_LENGTH - 1 {
                eprintln!("Error: jccc: identifier too long, over {} characters", TOKEN_LENGTH);
                return -1;
            }
            buf.push(cc);
        }
        t.token_type = ttype_many_chars(&buf);
        t.length = buf.len();
        t.contents = buf;
        t.line = starting_line;
        t.column = starting_col;
        return 0;
    }

    if starts_operator(init_c) != 0 {
        // Read characters as long as the resulting string is a valid operator
        // prefix. Mirrors the C implementation closely.
        while valid_operator_sequence(&buf) != 0 {
            let c = lexer_getchar(l);
            if c == EOF {
                break;
            }
            buf.push(c as u8 as char);
        }
        // We've ended! Push back the last character that broke the sequence.
        lexer_ungetchar(l);
        if !buf.is_empty() {
            buf.pop();
        }
        t.token_type = ttype_from_string(&buf);
        t.length = buf.len() + 1; // mirror C's `pos`
        t.contents = buf;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    eprintln!("Error: jccc: lexer unable to identify token starting with: {}", init_c);
    0
}

/// Tests the function that determines token types for a single character.
pub fn test_ttype_one_char() -> i32 {
    if !matches!(ttype_one_char('a'), TokenType::TT_IDENTIFIER) {
        return 1;
    }
    if !matches!(ttype_one_char('1'), TokenType::TT_LITERAL) {
        return 1;
    }
    if !matches!(ttype_one_char('+'), TokenType::TT_PLUS) {
        return 1;
    }
    if !matches!(ttype_one_char('-'), TokenType::TT_MINUS) {
        return 1;
    }
    if !matches!(ttype_one_char('>'), TokenType::TT_GREATER) {
        return 1;
    }
    if !matches!(ttype_one_char('~'), TokenType::TT_BNOT) {
        return 1;
    }
    0
}

/// Checks if a character starts an operator sequence.
pub fn starts_operator(c: char) -> i32 {
    match c {
        '-' | '+' | '*' | '/' | '=' | ':' | '%' | '&' | '|' | '<' | '>' | '!' | '~' | '^' => 1,
        _ => 0,
    }
}

/// Tests the function that determines token types from a string.
pub fn test_ttype_from_string() -> i32 {
    if !matches!(ttype_from_string("+"), TokenType::TT_PLUS) {
        return 1;
    }
    if !matches!(ttype_from_string("="), TokenType::TT_ASSIGN) {
        return 1;
    }
    if !matches!(ttype_from_string("1"), TokenType::TT_LITERAL) {
        return 1;
    }
    if !matches!(ttype_from_string("1.2"), TokenType::TT_LITERAL) {
        return 1;
    }
    if !matches!(ttype_from_string("1u"), TokenType::TT_LITERAL) {
        return 1;
    }
    if !matches!(ttype_from_string("1.2f"), TokenType::TT_LITERAL) {
        return 1;
    }
    if !matches!(ttype_from_string("1.f"), TokenType::TT_LITERAL) {
        return 1;
    }
    if !matches!(ttype_from_string("\"Planck\""), TokenType::TT_LITERAL) {
        return 1;
    }
    if !matches!(ttype_from_string("'Language'"), TokenType::TT_LITERAL) {
        return 1;
    }
    if !matches!(ttype_from_string("Jaba"), TokenType::TT_IDENTIFIER) {
        return 1;
    }
    if !matches!(ttype_from_string("cat_"), TokenType::TT_IDENTIFIER) {
        return 1;
    }
    if !matches!(ttype_from_string("("), TokenType::TT_OPAREN) {
        return 1;
    }
    if !matches!(ttype_from_string("}"), TokenType::TT_CBRACE) {
        return 1;
    }
    if !matches!(ttype_from_string(";"), TokenType::TT_SEMI) {
        return 1;
    }
    0
}

/// Derives a TokenType from a string input.
pub fn ttype_from_string(contents: &str) -> TokenType {
    if contents.chars().count() == 1 {
        return ttype_one_char(contents.chars().next().unwrap());
    }
    ttype_many_chars(contents)
}

/// Checks if character c is in the string s.
pub fn in_string(c: char, s: &str) -> i32 {
    if s.chars().any(|x| x == c) {
        1
    } else {
        0
    }
}

/// Tests the function that determines token types from multiple characters.
pub fn test_ttype_many_chars() -> i32 {
    if !matches!(ttype_many_chars("foo"), TokenType::TT_IDENTIFIER) {
        return 1;
    }
    if !matches!(ttype_many_chars("struct"), TokenType::TT_STRUCT) {
        return 1;
    }
    if !matches!(ttype_many_chars("while"), TokenType::TT_WHILE) {
        return 1;
    }
    0
}

/// Skips characters until the next token is found.
pub fn skip_to_token(l: &mut Lexer) -> i32 {
    let mut prev: char;
    let mut in_block: i32 = 0;
    let mut pass: i32 = 0;

    // Read the first character
    let first = lexer_getchar(l);
    if first == EOF {
        return -1;
    }
    let cur = first as u8 as char;
    prev = cur;
    if !(cur == ' ' || cur == '\t' || cur == '/') {
        lexer_ungetchar(l);
        return 0;
    }

    loop {
        let c = lexer_getchar(l);
        if c == EOF {
            return -1;
        }
        let cur = c as u8 as char;
        if cur == '/' && prev == '/' && in_block == 0 {
            in_block = 1;
        } else if cur == '*' && prev == '/' && in_block == 0 {
            in_block = 2;
            pass = 2;
        } else if (in_block == 1 && cur == '\n')
            || (in_block == 2 && cur == '/' && prev == '*' && pass <= 0)
        {
            in_block = 0;
        } else if prev == '/' && !(cur == '*' || cur == '/') && in_block == 0 {
            lexer_ungetchar(l);
            return 0;
        }
        if !(cur == ' ' || cur == '\t' || cur == '/') && in_block == 0 {
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
        eprintln!(
            "Error: jccc: internal: tried to unlex more than {} tokens at a time",
            TOKEN_PUTBACKS
        );
        return -1;
    }
    let idx = l.unlexed_count as usize;
    l.unlexed[idx] = Token {
        token_type: clone_ttype(&t.token_type),
        contents: t.contents.clone(),
        length: t.length,
        source_file: t.source_file.clone(),
        line: t.line,
        column: t.column,
    };
    l.unlexed_count += 1;
    0
}

/// Clones a TokenType (since the enum doesn't derive Clone).
fn clone_ttype(tt: &TokenType) -> TokenType {
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

/// Checks if c is a valid numeric or identifier character.
pub fn is_valid_numeric_or_id_char(c: char) -> i32 {
    if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
        1
    } else {
        0
    }
}

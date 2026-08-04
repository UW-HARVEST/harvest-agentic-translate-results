use crate::token::{Token, TokenType};
use std::fs::File;
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

fn clone_token_type(tt: &TokenType) -> TokenType {
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

pub fn token_type_name(tt: &TokenType) -> String {
    ttype_name(clone_token_type(tt))
}

fn clone_token(t: &Token) -> Token {
    Token {
        token_type: clone_token_type(&t.token_type),
        contents: t.contents.clone(),
        length: t.length,
        source_file: t.source_file.clone(),
        line: t.line,
        column: t.column,
    }
}

pub fn empty_token() -> Token {
    Token {
        token_type: TokenType::TT_NO_TOKEN,
        contents: String::new(),
        length: 0,
        source_file: String::new(),
        line: 0,
        column: 0,
    }
}

fn file_mut(l: &mut Lexer) -> Option<&mut File> {
    l.fp.as_mut()
}

/// Gets the next character from the lexer.
pub fn lexer_getchar(l: &mut Lexer) -> i32 {
    l.position += 1;
    l.last_column = l.column;

    let Some(file) = file_mut(l) else {
        return -1;
    };

    let mut buf = [0_u8; 1];
    match file.read(&mut buf) {
        Ok(0) | Err(_) => {
            l.column += 1;
            -1
        }
        Ok(_) => {
            l.buffer[0] = buf[0];
            if buf[0] == b'\n' {
                l.line += 1;
                l.column = 0;
            } else {
                l.column += 1;
            }
            i32::from(buf[0])
        }
    }
}
/// Retrieves the next token from the lexer.
pub fn real_lex(l: &mut Lexer, t: &mut Token) -> i32 {
    static SINGLE_CHAR_TOKENS: &str = "(){}[];~#,.:?~";

    if l.unlexed_count > 0 {
        l.unlexed_count -= 1;
        *t = clone_token(&l.unlexed[l.unlexed_count as usize]);
        return 0;
    }

    let _ = skip_to_token(l);
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

    let init_char = char::from_u32(init as u32).unwrap_or('\0');

    if init_char == ' ' || init_char == '\t' {
        return -1;
    }

    if init_char == '\n' {
        t.contents = "[newline]".to_string();
        t.length = t.contents.len();
        t.token_type = TokenType::TT_NEWLINE;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    let mut pos = 0usize;
    t.contents.push(init_char);
    pos += 1;

    if in_string(init_char, SINGLE_CHAR_TOKENS) != 0 {
        t.length = pos;
        t.token_type = ttype_one_char(init_char);
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

    if is_valid_numeric_or_id_char(init_char) != 0 {
        let starting_line = l.line;
        let starting_col = l.column;
        loop {
            let c = lexer_getchar(l);
            if c == -1 {
                break;
            }
            let c_char = char::from_u32(c as u32).unwrap_or('\0');
            if is_valid_numeric_or_id_char(c_char) == 0 {
                break;
            }
            if pos >= crate::token::TOKEN_LENGTH - 1 {
                return -1;
            }
            t.contents.push(c_char);
            pos += 1;
        }
        let _ = lexer_ungetchar(l);
        t.token_type = ttype_many_chars(&t.contents);
        t.length = pos;
        t.line = starting_line;
        t.column = starting_col;
        return 0;
    }

    if starts_operator(init_char) != 0 {
        while valid_operator_sequence(&t.contents) != 0 {
            let c = lexer_getchar(l);
            if c == -1 {
                break;
            }
            let c_char = char::from_u32(c as u32).unwrap_or('\0');
            t.contents.push(c_char);
            pos += 1;
        }
        let _ = lexer_ungetchar(l);
        if !t.contents.is_empty() {
            t.contents.pop();
        }
        t.token_type = ttype_from_string(&t.contents);
        t.length = pos;
        t.line = l.line;
        t.column = l.column;
        return 0;
    }

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

    if let Some(file) = file_mut(l) {
        let _ = file.seek(SeekFrom::Current(-1));
    }
    1
}
/// Determines a token type from a multi-character sequence.
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
            if contents.is_empty() {
                return TokenType::TT_NO_TOKEN;
            }

            let mut all_numeric = true;
            let mut count_us = 0usize;

            for c in contents.chars() {
                if c == '.' || c == '\'' || c == '"' {
                    return TokenType::TT_LITERAL;
                }
                if c == 'u' {
                    count_us += 1;
                }
                if !c.is_ascii_digit() && c != 'u' {
                    all_numeric = false;
                }
            }

            if all_numeric {
                if count_us == 1 && contents.ends_with('u') {
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
/// Tests the function that identifies token types by name.
pub fn test_ttype_name() -> i32 {
    assert_eq!(ttype_name(TokenType::TT_LITERAL), "literal");
    assert_eq!(ttype_name(TokenType::TT_PLUS), "+");
    assert_eq!(ttype_name(TokenType::TT_SIZEOF), "sizeof");
    assert_eq!(ttype_name(TokenType::TT_WHILE), "while");
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
        _ if c.is_ascii_digit() => TokenType::TT_LITERAL,
        _ => TokenType::TT_IDENTIFIER,
    }
}
/// Returns the name of a token type as a string.
pub fn ttype_name(tt: TokenType) -> String {
    match tt {
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
    }
    .to_string()
}
/// Checks if the provided operator sequence is valid.
pub fn valid_operator_sequence(op: &str) -> i32 {
    const OPS: &[&str] = &[
        "-", "+", "*", "/", "=", ":", "%", "&", "&&", "|", "||", "-=", "+=", "++", "--",
        "/=", "*=", "%=", "&=", "|=", "&&=", "||=", ">", "<", "<=", ">=", "<<", ">>", "!",
        "==", "!=", "^", "^=", "->", "<<=", ">>=",
    ];
    if OPS.contains(&op) { 1 } else { 0 }
}
/// Main lex function to tokenize input into a given Token.
pub fn lex(l: &mut Lexer, token: &mut Token) -> i32 {
    loop {
        real_lex(l, token);
        if token.token_type != TokenType::TT_NEWLINE {
            break;
        }
    }
    0
}
/// Tests the function that determines token types for a single character.
pub fn test_ttype_one_char() -> i32 {
    assert_eq!(ttype_one_char('a'), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_one_char('1'), TokenType::TT_LITERAL);
    assert_eq!(ttype_one_char('+'), TokenType::TT_PLUS);
    assert_eq!(ttype_one_char('-'), TokenType::TT_MINUS);
    assert_eq!(ttype_one_char('>'), TokenType::TT_GREATER);
    assert_eq!(ttype_one_char('~'), TokenType::TT_BNOT);
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
/// Derives a TokenType from a string input.
pub fn ttype_from_string(contents: &str) -> TokenType {
    if contents.len() == 1 {
        ttype_one_char(contents.chars().next().unwrap_or('\0'))
    } else {
        ttype_many_chars(contents)
    }
}
/// Checks if character c is in the string s.
pub fn in_string(c: char, s: &str) -> i32 {
    if s.contains(c) { 1 } else { 0 }
}
/// Tests the function that determines token types from multiple characters.
pub fn test_ttype_many_chars() -> i32 {
    assert_eq!(ttype_many_chars("foo"), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_many_chars("struct"), TokenType::TT_STRUCT);
    assert_eq!(ttype_many_chars("while"), TokenType::TT_WHILE);
    0
}
/// Skips characters until the next token is found.
pub fn skip_to_token(l: &mut Lexer) -> i32 {
    let mut in_block = 0;
    let mut pass = 0;

    let mut cur = lexer_getchar(l);
    if cur == -1 {
        return -1;
    }
    let mut prev = cur;
    let cur_char = char::from_u32(cur as u32).unwrap_or('\0');
    if cur_char != ' ' && cur_char != '\t' && cur_char != '/' {
        let _ = lexer_ungetchar(l);
        return 0;
    }

    loop {
        cur = lexer_getchar(l);
        if cur == -1 {
            return -1;
        }

        let cur_char = char::from_u32(cur as u32).unwrap_or('\0');
        let prev_char = char::from_u32(prev as u32).unwrap_or('\0');

        if cur_char == '/' && prev_char == '/' && in_block == 0 {
            in_block = 1;
        } else if cur_char == '*' && prev_char == '/' && in_block == 0 {
            in_block = 2;
            pass = 2;
        } else if (in_block == 1 && cur_char == '\n')
            || (in_block == 2 && cur_char == '/' && prev_char == '*' && pass <= 0)
        {
            in_block = 0;
        } else if prev_char == '/' && cur_char != '*' && cur_char != '/' && in_block == 0 {
            let _ = lexer_ungetchar(l);
            return 0;
        }

        if cur_char != ' ' && cur_char != '\t' && cur_char != '/' && in_block == 0 {
            let _ = lexer_ungetchar(l);
            return 0;
        }

        pass -= 1;
        prev = cur;
    }
}
/// Pushes a token back into the lexer's buffer.
pub fn unlex(l: &mut Lexer, t: &Token) -> i32 {
    if l.unlexed_count >= TOKEN_PUTBACKS as u32 {
        return -1;
    }
    l.unlexed[l.unlexed_count as usize] = clone_token(t);
    l.unlexed_count += 1;
    0
}
/// Checks if c is a valid numeric or identifier character.
pub fn is_valid_numeric_or_id_char(c: char) -> i32 {
    if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
        1
    } else {
        0
    }
}

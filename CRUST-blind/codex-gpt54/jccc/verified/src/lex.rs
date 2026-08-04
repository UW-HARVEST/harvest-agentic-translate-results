use crate::token::{Token, TokenType};
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
    "-", "+", "*", "/", "=", ":", "%", "&", "&&", "|", "||", "-=", "+=", "++", "--", "/=",
    "*=", "%=", "&=", "|=", "&&=", "||=", ">", "<", "<=", ">=", "<<", ">>", "!", "==", "!=",
    "^", "^=", "->", "<<=", ">>=",
];

fn set_token_basic_fields(l: &Lexer, t: &mut Token) {
    t.source_file = l.current_file.clone();
    t.line = l.line;
    t.column = l.column;
}
/// Gets the next character from the lexer.
pub fn lexer_getchar(l: &mut Lexer) -> i32 {
    l.position += 1;
    l.last_column = l.column;

    let Some(fp) = l.fp.as_mut() else {
        l.column += 1;
        return -1;
    };

    let mut byte = [0_u8; 1];
    match fp.read(&mut byte) {
        Ok(1) => {
            l.buffer[0] = byte[0];
            if byte[0] == b'\n' {
                l.line += 1;
                l.column = 0;
            } else {
                l.column += 1;
            }
            i32::from(byte[0])
        }
        Ok(0) | Err(_) => {
            l.buffer[0] = 0;
            l.column += 1;
            -1
        }
        Ok(_) => {
            l.column += 1;
            -1
        }
    }
}
/// Retrieves the next token from the lexer.
pub fn real_lex(l: &mut Lexer, t: &mut Token) -> i32 {
    if l.unlexed_count > 0 {
        l.unlexed_count -= 1;
        *t = l.unlexed[l.unlexed_count as usize].clone();
        return 0;
    }

    let _ = skip_to_token(l);
    let init = lexer_getchar(l);

    t.contents.clear();
    t.length = 0;
    t.token_type = TokenType::TT_NO_TOKEN;
    t.source_file = l.current_file.clone();
    t.line = 0;
    t.column = 0;

    if init == -1 {
        t.contents = "[end of file]".to_string();
        t.length = t.contents.len();
        t.token_type = TokenType::TT_EOF;
        set_token_basic_fields(l, t);
        return 0;
    }

    if init == i32::from(b' ') || init == i32::from(b'\t') {
        return -1;
    }

    if init == i32::from(b'\n') {
        t.contents = "[newline]".to_string();
        t.length = t.contents.len();
        t.token_type = TokenType::TT_NEWLINE;
        set_token_basic_fields(l, t);
        return 0;
    }

    let init_char = char::from_u32(init as u32).unwrap_or('\0');
    t.contents.push(init_char);

    if in_string(init_char, SINGLE_CHAR_TOKENS) != 0 {
        t.length = t.contents.len();
        t.token_type = ttype_one_char(init_char);
        set_token_basic_fields(l, t);
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
            if t.contents.len() >= crate::token::TOKEN_LENGTH - 1 {
                return -1;
            }
            t.contents.push(c_char);
        }
        let _ = lexer_ungetchar(l);
        t.token_type = ttype_many_chars(&t.contents);
        t.length = t.contents.len();
        t.line = starting_line;
        t.column = starting_col;
        return 0;
    }

    if starts_operator(init_char) != 0 {
        loop {
            if valid_operator_sequence(&t.contents) == 0 {
                break;
            }
            let c = lexer_getchar(l);
            if c == -1 {
                break;
            }
            t.contents.push(char::from_u32(c as u32).unwrap_or('\0'));
        }
        let _ = lexer_ungetchar(l);
        t.contents.pop();
        t.token_type = ttype_from_string(&t.contents);
        t.length = t.contents.len() + 1;
        set_token_basic_fields(l, t);
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

    let Some(fp) = l.fp.as_mut() else {
        return -1;
    };

    if fp.seek(SeekFrom::Current(-1)).is_ok() {
        1
    } else {
        -1
    }
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
                return TokenType::TT_IDENTIFIER;
            }

            let mut all_numeric = true;
            let mut count_us = 0;

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
    if ttype_name(TokenType::TT_LITERAL) == "literal"
        && ttype_name(TokenType::TT_PLUS) == "+"
        && ttype_name(TokenType::TT_SIZEOF) == "sizeof"
        && ttype_name(TokenType::TT_WHILE) == "while"
    {
        0
    } else {
        -1
    }
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
    let name = match tt {
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
    name.to_string()
}
/// Checks if the provided operator sequence is valid.
pub fn valid_operator_sequence(op: &str) -> i32 {
    if OPERATOR_STRINGS.iter().any(|candidate| *candidate == op) {
        1
    } else {
        0
    }
}
/// Main lex function to tokenize input into a given Token.
pub fn lex(l: &mut Lexer, token: &mut Token) -> i32 {
    loop {
        let ret = real_lex(l, token);
        if ret != 0 {
            return ret;
        }
        if token.token_type != TokenType::TT_NEWLINE {
            break;
        }
    }
    0
}
/// Tests the function that determines token types for a single character.
pub fn test_ttype_one_char() -> i32 {
    if ttype_one_char('a') == TokenType::TT_IDENTIFIER
        && ttype_one_char('1') == TokenType::TT_LITERAL
        && ttype_one_char('+') == TokenType::TT_PLUS
        && ttype_one_char('-') == TokenType::TT_MINUS
        && ttype_one_char('>') == TokenType::TT_GREATER
        && ttype_one_char('~') == TokenType::TT_BNOT
    {
        0
    } else {
        -1
    }
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
    if ttype_from_string("+") == TokenType::TT_PLUS
        && ttype_from_string("=") == TokenType::TT_ASSIGN
        && ttype_from_string("1") == TokenType::TT_LITERAL
        && ttype_from_string("1.2") == TokenType::TT_LITERAL
        && ttype_from_string("1u") == TokenType::TT_LITERAL
        && ttype_from_string("1.2f") == TokenType::TT_LITERAL
        && ttype_from_string("1.f") == TokenType::TT_LITERAL
        && ttype_from_string("\"Planck\"") == TokenType::TT_LITERAL
        && ttype_from_string("'Language'") == TokenType::TT_LITERAL
        && ttype_from_string("Jaba") == TokenType::TT_IDENTIFIER
        && ttype_from_string("cat_") == TokenType::TT_IDENTIFIER
        && ttype_from_string("(") == TokenType::TT_OPAREN
        && ttype_from_string("}") == TokenType::TT_CBRACE
        && ttype_from_string(";") == TokenType::TT_SEMI
    {
        0
    } else {
        -1
    }
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
    if s.chars().any(|candidate| candidate == c) {
        1
    } else {
        0
    }
}
/// Tests the function that determines token types from multiple characters.
pub fn test_ttype_many_chars() -> i32 {
    if ttype_many_chars("foo") == TokenType::TT_IDENTIFIER
        && ttype_many_chars("struct") == TokenType::TT_STRUCT
        && ttype_many_chars("while") == TokenType::TT_WHILE
    {
        0
    } else {
        -1
    }
}
/// Skips characters until the next token is found.
pub fn skip_to_token(l: &mut Lexer) -> i32 {
    let mut in_block = 0;
    let mut pass = 0;

    let mut cur = lexer_getchar(l);
    let mut prev;

    if cur != -1 {
        prev = cur;
        if cur != i32::from(b' ') && cur != i32::from(b'\t') && cur != i32::from(b'/') {
            let _ = lexer_ungetchar(l);
            return 0;
        }
    } else {
        return -1;
    }

    loop {
        cur = lexer_getchar(l);
        if cur == -1 {
            break;
        }

        if cur == i32::from(b'/') && prev == i32::from(b'/') && in_block == 0 {
            in_block = 1;
        } else if cur == i32::from(b'*') && prev == i32::from(b'/') && in_block == 0 {
            in_block = 2;
            pass = 2;
        } else if (in_block == 1 && cur == i32::from(b'\n'))
            || (in_block == 2
                && cur == i32::from(b'/')
                && prev == i32::from(b'*')
                && pass <= 0)
        {
            in_block = 0;
        } else if prev == i32::from(b'/')
            && cur != i32::from(b'*')
            && cur != i32::from(b'/')
            && in_block == 0
        {
            let _ = lexer_ungetchar(l);
            return 0;
        }

        if cur != i32::from(b' ')
            && cur != i32::from(b'\t')
            && cur != i32::from(b'/')
            && in_block == 0
        {
            let _ = lexer_ungetchar(l);
            return 0;
        }

        pass -= 1;
        prev = cur;
    }

    -1
}
/// Pushes a token back into the lexer's buffer.
pub fn unlex(l: &mut Lexer, t: &Token) -> i32 {
    if l.unlexed_count as usize >= TOKEN_PUTBACKS {
        return -1;
    }
    l.unlexed[l.unlexed_count as usize] = t.clone();
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

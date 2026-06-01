use jccc::lex::{lex, lexer_getchar, lexer_ungetchar, real_lex, skip_to_token, unlex, Lexer};
use jccc::token::{Token, TokenType};
use std::fs::File;
use std::io::Write;

fn make_default_token() -> Token {
    Token {
        token_type: TokenType::TT_NO_TOKEN,
        contents: String::new(),
        length: 0,
        source_file: String::new(),
        line: 0,
        column: 0,
    }
}

fn make_default_lexer() -> Lexer {
    Lexer {
        fp: None,
        current_file: String::new(),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: [
            make_default_token(),
            make_default_token(),
            make_default_token(),
            make_default_token(),
            make_default_token(),
        ],
        unlexed_count: 0,
    }
}

fn make_lexer_for(content: &str, fname: &str) -> Lexer {
    let dir = std::env::temp_dir();
    let path = dir.join(fname);
    let mut f = File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    drop(f);
    let f = File::open(&path).unwrap();
    let mut l = make_default_lexer();
    l.fp = Some(f);
    l
}

#[test]
fn test_lexer_getchar_simple() {
    let mut l = make_lexer_for("ab", "lex_getchar.c");
    let c = lexer_getchar(&mut l);
    assert_eq!(c, b'a' as i32);
    let c = lexer_getchar(&mut l);
    assert_eq!(c, b'b' as i32);
    let c = lexer_getchar(&mut l);
    assert_eq!(c, -1);
}

#[test]
fn test_lexer_getchar_newline_updates_line() {
    let mut l = make_lexer_for("a\nb", "lex_newline.c");
    lexer_getchar(&mut l); // 'a'
    let line_before = l.line;
    lexer_getchar(&mut l); // '\n'
    assert_eq!(l.line, line_before + 1);
    assert_eq!(l.column, 0);
}

#[test]
fn test_lexer_ungetchar_decreases_position() {
    let mut l = make_lexer_for("ab", "lex_unget.c");
    lexer_getchar(&mut l);
    let before = l.position;
    lexer_ungetchar(&mut l);
    assert_eq!(l.position, before - 1);
}

#[test]
fn test_lex_int_keyword() {
    let mut l = make_lexer_for("int", "lex_int.c");
    let mut t = make_default_token();
    let r = lex(&mut l, &mut t);
    assert_eq!(r, 0);
    assert_eq!(t.contents, "int");
    assert!(matches!(t.token_type, TokenType::TT_INT));
}

#[test]
fn test_lex_simple_main_program() {
    let mut l = make_lexer_for("int main() { return 0; }", "lex_main.c");
    // Expected sequence:
    let mut tokens = Vec::new();
    loop {
        let mut t = make_default_token();
        let r = lex(&mut l, &mut t);
        assert_eq!(r, 0);
        let is_eof = matches!(t.token_type, TokenType::TT_EOF);
        tokens.push(t);
        if is_eof {
            break;
        }
    }
    let expected_types = vec![
        TokenType::TT_INT,
        TokenType::TT_IDENTIFIER,
        TokenType::TT_OPAREN,
        TokenType::TT_CPAREN,
        TokenType::TT_OBRACE,
        TokenType::TT_RETURN,
        TokenType::TT_LITERAL,
        TokenType::TT_SEMI,
        TokenType::TT_CBRACE,
        TokenType::TT_EOF,
    ];
    assert_eq!(tokens.len(), expected_types.len());
    for (i, expected) in expected_types.iter().enumerate() {
        assert_eq!(
            std::mem::discriminant(&tokens[i].token_type),
            std::mem::discriminant(expected),
            "mismatch at index {}",
            i
        );
    }
    assert_eq!(tokens[1].contents, "main");
    assert_eq!(tokens[6].contents, "0");
}

#[test]
fn test_lex_identifier_with_decl() {
    let mut l = make_lexer_for("int main() {\n    int a = 100;\n    return a * 5;\n}", "lex_ident.c");
    let mut tokens = Vec::new();
    loop {
        let mut t = make_default_token();
        let r = lex(&mut l, &mut t);
        assert_eq!(r, 0);
        let is_eof = matches!(t.token_type, TokenType::TT_EOF);
        tokens.push(t);
        if is_eof {
            break;
        }
    }
    let expected_types = vec![
        TokenType::TT_INT,
        TokenType::TT_IDENTIFIER,
        TokenType::TT_OPAREN,
        TokenType::TT_CPAREN,
        TokenType::TT_OBRACE,
        TokenType::TT_INT,
        TokenType::TT_IDENTIFIER,
        TokenType::TT_ASSIGN,
        TokenType::TT_LITERAL,
        TokenType::TT_SEMI,
        TokenType::TT_RETURN,
        TokenType::TT_IDENTIFIER,
        TokenType::TT_STAR,
        TokenType::TT_LITERAL,
        TokenType::TT_SEMI,
        TokenType::TT_CBRACE,
        TokenType::TT_EOF,
    ];
    assert_eq!(tokens.len(), expected_types.len());
    for (i, expected) in expected_types.iter().enumerate() {
        assert_eq!(
            std::mem::discriminant(&tokens[i].token_type),
            std::mem::discriminant(expected),
            "mismatch at index {}",
            i
        );
    }
    assert_eq!(tokens[6].contents, "a");
    assert_eq!(tokens[8].contents, "100");
    assert_eq!(tokens[11].contents, "a");
    assert_eq!(tokens[13].contents, "5");
}

#[test]
fn test_lex_skip_comments() {
    let mut l = make_lexer_for("// comment\nint a;", "lex_comment.c");
    let mut tokens = Vec::new();
    loop {
        let mut t = make_default_token();
        let r = lex(&mut l, &mut t);
        assert_eq!(r, 0);
        let is_eof = matches!(t.token_type, TokenType::TT_EOF);
        tokens.push(t);
        if is_eof {
            break;
        }
    }
    let expected = vec![
        TokenType::TT_INT,
        TokenType::TT_IDENTIFIER,
        TokenType::TT_SEMI,
        TokenType::TT_EOF,
    ];
    assert_eq!(tokens.len(), expected.len());
    for (i, e) in expected.iter().enumerate() {
        assert_eq!(
            std::mem::discriminant(&tokens[i].token_type),
            std::mem::discriminant(e)
        );
    }
}

#[test]
fn test_skip_to_token_basic() {
    let mut l = make_lexer_for("   abc", "lex_skip.c");
    let r = skip_to_token(&mut l);
    assert_eq!(r, 0);
    // Now next char should be 'a'
    let c = lexer_getchar(&mut l);
    assert_eq!(c, b'a' as i32);
}

#[test]
fn test_unlex_then_lex_returns_same_token() {
    let mut l = make_default_lexer();
    let pushed = Token {
        token_type: TokenType::TT_INT,
        contents: "int".to_string(),
        length: 3,
        source_file: "f.c".to_string(),
        line: 5,
        column: 7,
    };
    let r = unlex(&mut l, &pushed);
    assert_eq!(r, 0);
    assert_eq!(l.unlexed_count, 1);
    let mut t = make_default_token();
    real_lex(&mut l, &mut t);
    assert!(matches!(t.token_type, TokenType::TT_INT));
    assert_eq!(t.contents, "int");
    assert_eq!(t.length, 3);
    assert_eq!(t.line, 5);
    assert_eq!(t.column, 7);
}

#[test]
fn test_unlex_overflow() {
    let mut l = make_default_lexer();
    let pushed = Token {
        token_type: TokenType::TT_INT,
        contents: "int".to_string(),
        length: 3,
        source_file: "f.c".to_string(),
        line: 1,
        column: 1,
    };
    for _ in 0..5 {
        let r = unlex(&mut l, &pushed);
        assert_eq!(r, 0);
    }
    let r = unlex(&mut l, &pushed);
    assert_eq!(r, -1);
}

fn main() {}

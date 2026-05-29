use jccc::cst::{BlockStatement, ConcreteFileTree, Expression, FunctionDeclaration, NodeType};
use jccc::lex::{Lexer, TOKEN_PUTBACKS};
use jccc::list::create_list;
use jccc::parse::{
    make_cst, parse, parse_blockstmt, parse_expr, parse_funccall, parse_funcdecl,
    parse_simple_main_func,
};
use jccc::token::{Token, TokenType};
use std::io::Write;

fn make_empty_token() -> Token {
    Token {
        token_type: TokenType::TT_NO_TOKEN,
        contents: String::new(),
        length: 0,
        source_file: String::new(),
        line: 0,
        column: 0,
    }
}

fn make_unlexed_array() -> [Token; TOKEN_PUTBACKS] {
    [
        make_empty_token(),
        make_empty_token(),
        make_empty_token(),
        make_empty_token(),
        make_empty_token(),
    ]
}

fn make_lexer() -> Lexer {
    Lexer {
        fp: None,
        current_file: String::new(),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: make_unlexed_array(),
        unlexed_count: 0,
    }
}

#[test]
fn test_parse_simple_main_func_returns_zero() {
    // The C function has no body (so its return value is undefined per C
    // standard); the Rust translation explicitly returns 0.
    assert_eq!(parse_simple_main_func(), 0);
}

#[test]
fn test_parse_funcdecl_todo_returns_zero() {
    // The C version is unimplemented; the Rust translation returns 0.
    let mut lexer = make_lexer();
    let mut fd = FunctionDeclaration {
        body: BlockStatement {
            stmts: create_list(4),
        },
        name: String::new(),
    };
    assert_eq!(parse_funcdecl(&mut lexer, &mut fd), 0);
}

#[test]
fn test_make_cst_todo_returns_zero() {
    let mut lexer = make_lexer();
    let mut tree = ConcreteFileTree {
        decls: create_list(4),
    };
    assert_eq!(make_cst(&mut lexer, &mut tree), 0);
}

#[test]
fn test_parse_expr_todo_returns_zero() {
    let mut lexer = make_lexer();
    let mut ex = Expression {
        fc: None,
        literal: None,
        node_type: NodeType::NT_LITERAL,
    };
    assert_eq!(parse_expr(&mut lexer, &mut ex), 0);
}

#[test]
fn test_parse_blockstmt_todo_returns_zero() {
    let mut lexer = make_lexer();
    let mut bs = BlockStatement {
        stmts: create_list(4),
    };
    assert_eq!(parse_blockstmt(&mut lexer, &mut bs), 0);
}

#[test]
fn test_parse_funccall_todo_returns_zero() {
    let mut lexer = make_lexer();
    let mut ex = Expression {
        fc: None,
        literal: None,
        node_type: NodeType::NT_FUNCCALL,
    };
    assert_eq!(parse_funccall(&mut lexer, &mut ex), 0);
}

#[test]
fn test_parse_missing_file_returns_one() {
    // Mirrors the C behavior: when fopen fails, parse returns 1.
    let r = parse("/this/does/not/exist/anywhere.c");
    assert_eq!(r, 1);
}

#[test]
fn test_parse_simple_main_returns_zero() {
    // Build a simple main.c file in a temp location.
    let path = std::env::temp_dir().join("crust_test_parse_main.c");
    {
        let mut f = std::fs::File::create(&path).expect("create temp file");
        writeln!(f, "int main() {{").unwrap();
        writeln!(f, "    return 42;").unwrap();
        writeln!(f, "}}").unwrap();
    }
    // C parse() returns 0 for any input it could open (it prints diagnostics
    // for unrecognized structure but still returns 0 once the file is open).
    let r = parse(path.to_str().unwrap());
    assert_eq!(r, 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_parse_other_file_still_returns_zero() {
    // Even when the file isn't a valid main, C parse() returns 0 once the
    // file is opened (it just prints "Not correct main function").
    let path = std::env::temp_dir().join("crust_test_parse_garbage.c");
    {
        let mut f = std::fs::File::create(&path).expect("create temp file");
        writeln!(f, "void foo() {{ return; }}").unwrap();
    }
    let r = parse(path.to_str().unwrap());
    assert_eq!(r, 0);
    let _ = std::fs::remove_file(&path);
}

fn main() {}

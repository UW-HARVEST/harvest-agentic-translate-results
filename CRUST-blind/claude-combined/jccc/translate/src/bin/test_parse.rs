use jccc::cst::{
    BlockStatement, ConcreteFileTree, Expression, FunctionDeclaration, NodeType,
};
use jccc::lex::Lexer;
use jccc::list::create_list;
use jccc::parse::{
    make_cst, parse, parse_blockstmt, parse_expr, parse_funccall, parse_funcdecl,
    parse_simple_main_func,
};
use jccc::token::{Token, TokenType};
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

#[test]
fn test_parse_missing_file_returns_one() {
    let r = parse("/this/path/does/not/exist/file.c");
    assert_eq!(r, 1);
}

#[test]
fn test_parse_simple_main_returns_zero() {
    let dir = std::env::temp_dir();
    let path = dir.join("jccc_test_simplemain.c");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "int main() {{ return 0; }}\n").unwrap();
    }
    let r = parse(path.to_str().unwrap());
    assert_eq!(r, 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_parse_with_return_42() {
    let dir = std::env::temp_dir();
    let path = dir.join("jccc_test_42.c");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "int main() {{ return 42; }}\n").unwrap();
    }
    let r = parse(path.to_str().unwrap());
    assert_eq!(r, 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_parse_simple_main_func_returns_zero() {
    let r = parse_simple_main_func();
    assert_eq!(r, 0);
}

#[test]
fn test_parse_expr_returns_zero() {
    let mut l = make_default_lexer();
    let mut ex = Expression {
        fc: None,
        literal: None,
        node_type: NodeType::NT_EXPR,
    };
    let r = parse_expr(&mut l, &mut ex);
    assert_eq!(r, 0);
}

#[test]
fn test_parse_funccall_returns_zero() {
    let mut l = make_default_lexer();
    let mut ex = Expression {
        fc: None,
        literal: None,
        node_type: NodeType::NT_FUNCCALL,
    };
    let r = parse_funccall(&mut l, &mut ex);
    assert_eq!(r, 0);
}

#[test]
fn test_parse_blockstmt_returns_zero() {
    let mut l = make_default_lexer();
    let mut bs = BlockStatement {
        stmts: create_list(8),
    };
    let r = parse_blockstmt(&mut l, &mut bs);
    assert_eq!(r, 0);
}

#[test]
fn test_parse_funcdecl_returns_zero() {
    let mut l = make_default_lexer();
    let mut fd = FunctionDeclaration {
        body: BlockStatement {
            stmts: create_list(8),
        },
        name: String::new(),
    };
    let r = parse_funcdecl(&mut l, &mut fd);
    assert_eq!(r, 0);
}

#[test]
fn test_make_cst_returns_zero() {
    let mut l = make_default_lexer();
    let mut tree = ConcreteFileTree {
        decls: create_list(8),
    };
    let r = make_cst(&mut l, &mut tree);
    assert_eq!(r, 0);
}

fn main() {}

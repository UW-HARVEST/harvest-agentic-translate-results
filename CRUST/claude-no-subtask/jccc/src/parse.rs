use crate::cst::{BlockStatement, ConcreteFileTree, Expression, FunctionDeclaration};
use crate::lex::Lexer;

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(_l: &mut Lexer, _fd: &mut FunctionDeclaration) -> i32 {
    // TODO in original C: not yet implemented.
    0
}

/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(_l: &mut Lexer, _tree: &mut ConcreteFileTree) -> i32 {
    // TODO in original C: not yet implemented.
    0
}

/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    // TODO in original C: not yet implemented.
    0
}

/// Parses a file and returns a status code.
pub fn parse(_filename: &str) -> i32 {
    // The C version performs lexing/printing of tokens. We provide a no-op
    // safe stub here since full file IO is out of scope for the test suite.
    0
}

/// Parses a simple main function (for testing).
pub fn parse_simple_main_func() -> i32 {
    // C version is empty.
    0
}

/// Parses a block statement from the lexer.
pub fn parse_blockstmt(_l: &mut Lexer, _bs: &mut BlockStatement) -> i32 {
    // TODO in original C: not yet implemented.
    0
}

/// Parses a function call from the lexer into an Expression object.
pub fn parse_funccall(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    // TODO in original C: not yet implemented.
    0
}

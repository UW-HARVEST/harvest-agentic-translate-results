use simple_lang::compiler;
use simple_lang::lexer;
use simple_lang::parser;
use simple_lang::ast;
use simple_lang::token;
use simple_lang::vm;

#[test]
fn test_compile_full_example() {
    let source = "let x = 5 + 3 - 2; dis x;";
    let mut count: i32 = 0;
    let instr = compiler::compile(source, &mut count);
    assert_eq!(count, 8);
    assert_eq!(instr.len(), 8);

    assert_eq!(instr[0].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[0].operand, "5");

    assert_eq!(instr[1].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[1].operand, "3");

    assert_eq!(instr[2].opcode, vm::OpCode::BINARY_ADD);

    assert_eq!(instr[3].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[3].operand, "2");

    assert_eq!(instr[4].opcode, vm::OpCode::BINARY_SUB);

    assert_eq!(instr[5].opcode, vm::OpCode::STORE_NAME);
    assert_eq!(instr[5].operand, "x");

    assert_eq!(instr[6].opcode, vm::OpCode::LOAD_NAME);
    assert_eq!(instr[6].operand, "x");

    assert_eq!(instr[7].opcode, vm::OpCode::STK_DIS);
}

#[test]
fn test_compile_let_only() {
    let source = "let x = 5 + 3 - 2;";
    let mut count: i32 = 0;
    let instr = compiler::compile(source, &mut count);
    // From C ground truth: 6 instructions
    assert_eq!(count, 6);
    assert_eq!(instr[0].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[0].operand, "5");
    assert_eq!(instr[1].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[1].operand, "3");
    assert_eq!(instr[2].opcode, vm::OpCode::BINARY_ADD);
    assert_eq!(instr[3].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[3].operand, "2");
    assert_eq!(instr[4].opcode, vm::OpCode::BINARY_SUB);
    assert_eq!(instr[5].opcode, vm::OpCode::STORE_NAME);
    assert_eq!(instr[5].operand, "x");
}

#[test]
fn test_compile_two_lets() {
    let source = "let x = 5; let y = x + 3;";
    let mut count: i32 = 0;
    let instr = compiler::compile(source, &mut count);
    assert_eq!(count, 6);
    assert_eq!(instr[0].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[0].operand, "5");
    assert_eq!(instr[1].opcode, vm::OpCode::STORE_NAME);
    assert_eq!(instr[1].operand, "x");
    assert_eq!(instr[2].opcode, vm::OpCode::LOAD_NAME);
    assert_eq!(instr[2].operand, "x");
    assert_eq!(instr[3].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[3].operand, "3");
    assert_eq!(instr[4].opcode, vm::OpCode::BINARY_ADD);
    assert_eq!(instr[5].opcode, vm::OpCode::STORE_NAME);
    assert_eq!(instr[5].operand, "y");
}

#[test]
fn test_compile_dis_literal() {
    let source = "dis 42;";
    let mut count: i32 = 0;
    let instr = compiler::compile(source, &mut count);
    assert_eq!(count, 2);
    assert_eq!(instr[0].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[0].operand, "42");
    assert_eq!(instr[1].opcode, vm::OpCode::STK_DIS);
}

#[test]
fn test_compile_asts_directly() {
    let tokens = lexer::tokenize("let x = 1 + 2;");
    let asts = parser::parse(&tokens);
    let mut count: i32 = 0;
    let instr = compiler::compile_asts(&asts, &mut count);
    assert_eq!(count, 4);
    assert_eq!(instr[0].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[0].operand, "1");
    assert_eq!(instr[1].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[1].operand, "2");
    assert_eq!(instr[2].opcode, vm::OpCode::BINARY_ADD);
    assert_eq!(instr[3].opcode, vm::OpCode::STORE_NAME);
    assert_eq!(instr[3].operand, "x");
}

#[test]
fn test_compile_statement_directly() {
    let tokens = lexer::tokenize("let z = 7;");
    let asts = parser::parse(&tokens);
    let mut count: i32 = 0;
    let instr = compiler::compile_statement(&asts[0], &mut count);
    assert_eq!(count, 2);
    assert_eq!(instr[0].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr[0].operand, "7");
    assert_eq!(instr[1].opcode, vm::OpCode::STORE_NAME);
    assert_eq!(instr[1].operand, "z");
}

#[test]
fn test_compile_expression_directly() {
    // Build AST: 2 + 3
    let mut node = ast::new_ast_node(token::TokenType::TOKEN_PLUS, "+");
    node.left = Some(ast::new_ast_node(token::TokenType::TOKEN_INT, "2"));
    node.right = Some(ast::new_ast_node(token::TokenType::TOKEN_INT, "3"));

    let mut instructions: Vec<vm::Instruction> = Vec::new();
    let mut count: i32 = 0;
    compiler::compile_expression(&node, &mut instructions, &mut count);
    assert_eq!(count, 3);
    assert_eq!(instructions[0].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instructions[0].operand, "2");
    assert_eq!(instructions[1].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instructions[1].operand, "3");
    assert_eq!(instructions[2].opcode, vm::OpCode::BINARY_ADD);
}

#[test]
fn test_emit_directly() {
    let mut instructions: Vec<vm::Instruction> = Vec::new();
    let mut count: i32 = 0;
    compiler::emit(&mut instructions, &mut count, vm::OpCode::LOAD_CONST, "10");
    compiler::emit(&mut instructions, &mut count, vm::OpCode::LOAD_NAME, "x");
    assert_eq!(count, 2);
    assert_eq!(instructions[0].opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instructions[0].operand, "10");
    assert_eq!(instructions[1].opcode, vm::OpCode::LOAD_NAME);
    assert_eq!(instructions[1].operand, "x");
}

fn main() {}

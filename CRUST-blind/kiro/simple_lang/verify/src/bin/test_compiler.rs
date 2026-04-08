use simple_lang::compiler;
use simple_lang::vm::OpCode;

#[test]
fn test_compile_let_expr_dis() {
    // C ground truth: "let x = 5 + 3 - 2; dis x;" -> 8 instructions
    // [LOAD_CONST "5", LOAD_CONST "3", BINARY_ADD "", LOAD_CONST "2", BINARY_SUB "", STORE_NAME "x", LOAD_NAME "x", STK_DIS ""]
    let mut instr_count = 0;
    let instrs = compiler::compile("let x = 5 + 3 - 2; dis x;", &mut instr_count);
    assert_eq!(instr_count, 8);
    assert_eq!(instrs.len(), 8);

    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "5");
    assert_eq!(instrs[1].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[1].operand, "3");
    assert_eq!(instrs[2].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[2].operand, "");  // C uses NULL, Rust uses ""
    assert_eq!(instrs[3].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[3].operand, "2");
    assert_eq!(instrs[4].opcode, OpCode::BINARY_SUB);
    assert_eq!(instrs[4].operand, "");
    assert_eq!(instrs[5].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[5].operand, "x");
    assert_eq!(instrs[6].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[6].operand, "x");
    assert_eq!(instrs[7].opcode, OpCode::STK_DIS);
    assert_eq!(instrs[7].operand, "");
}

#[test]
fn test_compile_dis_literal() {
    // C ground truth: "dis 42;" -> 2 instructions: LOAD_CONST "42", STK_DIS ""
    let mut instr_count = 0;
    let instrs = compiler::compile("dis 42;", &mut instr_count);
    assert_eq!(instr_count, 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "42");
    assert_eq!(instrs[1].opcode, OpCode::STK_DIS);
    assert_eq!(instrs[1].operand, "");
}

#[test]
fn test_compile_multi_let() {
    // C ground truth: "let a = 1; let b = 2; let c = a + b;" -> 8 instructions
    let mut instr_count = 0;
    let instrs = compiler::compile("let a = 1; let b = 2; let c = a + b;", &mut instr_count);
    assert_eq!(instr_count, 8);

    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "1");
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[1].operand, "a");
    assert_eq!(instrs[2].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[2].operand, "2");
    assert_eq!(instrs[3].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[3].operand, "b");
    assert_eq!(instrs[4].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[4].operand, "a");
    assert_eq!(instrs[5].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[5].operand, "b");
    assert_eq!(instrs[6].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[6].operand, "");
    assert_eq!(instrs[7].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[7].operand, "c");
}

#[test]
fn test_compile_expression_directly() {
    // Test compile_expression via compile_statement
    use simple_lang::ast;
    use simple_lang::token::TokenType;
    use simple_lang::vm;

    let mut instrs = Vec::new();
    let mut count = 0;

    // INT node
    let node = ast::new_ast_node(TokenType::TOKEN_INT, "7");
    compiler::compile_expression(&node, &mut instrs, &mut count);
    assert_eq!(count, 1);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "7");

    // IDENTIFIER node
    instrs.clear();
    count = 0;
    let node = ast::new_ast_node(TokenType::TOKEN_IDENTIFIER, "myvar");
    compiler::compile_expression(&node, &mut instrs, &mut count);
    assert_eq!(count, 1);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[0].operand, "myvar");
}

#[test]
fn test_emit() {
    use simple_lang::vm;
    let mut instrs = Vec::new();
    let mut count = 0;
    compiler::emit(&mut instrs, &mut count, OpCode::LOAD_CONST, "42");
    assert_eq!(count, 1);
    assert_eq!(instrs.len(), 1);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "42");

    compiler::emit(&mut instrs, &mut count, OpCode::BINARY_ADD, "");
    assert_eq!(count, 2);
    assert_eq!(instrs.len(), 2);
    assert_eq!(instrs[1].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[1].operand, "");
}

#[test]
fn test_header_guard() {
    assert_eq!(compiler::SIMPLE_LANG_COMPILER_H, true);
}

fn main() {}

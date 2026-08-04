use simple_lang::ast::new_ast_node;
use simple_lang::compiler::{compile, compile_asts, compile_expression, compile_statement, emit};
use simple_lang::lexer::tokenize;
use simple_lang::parser::parse;
use simple_lang::token::TokenType;
use simple_lang::vm::{eval, init_frame, OpCode};

#[test]
fn test_compile_let_with_arithmetic() {
    let mut count: i32 = 0;
    let instrs = compile("let x = 5 + 3 - 2;", &mut count);
    assert_eq!(count, 6);
    assert_eq!(instrs.len(), 6);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "5");
    assert_eq!(instrs[1].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[1].operand, "3");
    assert_eq!(instrs[2].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[2].operand, "");
    assert_eq!(instrs[3].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[3].operand, "2");
    assert_eq!(instrs[4].opcode, OpCode::BINARY_SUB);
    assert_eq!(instrs[4].operand, "");
    assert_eq!(instrs[5].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[5].operand, "x");
}

#[test]
fn test_compile_dis_program() {
    let mut count: i32 = 0;
    let instrs = compile("let x = 5 + 3 - 2; dis x;", &mut count);
    assert_eq!(count, 8);
    assert_eq!(instrs.len(), 8);
    assert_eq!(instrs[6].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[6].operand, "x");
    assert_eq!(instrs[7].opcode, OpCode::STK_DIS);
    // STK_DIS in C uses ast->value which for `dis x;` is NULL → empty string.
    assert_eq!(instrs[7].operand, "");
}

#[test]
fn test_compile_multi_let() {
    let mut count: i32 = 0;
    let instrs = compile("let x = 5; let y = 10; let z = x + y;", &mut count);
    assert_eq!(count, 8);
    assert_eq!(instrs.len(), 8);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "5");
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[1].operand, "x");
    assert_eq!(instrs[2].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[2].operand, "10");
    assert_eq!(instrs[3].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[3].operand, "y");
    assert_eq!(instrs[4].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[4].operand, "x");
    assert_eq!(instrs[5].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[5].operand, "y");
    assert_eq!(instrs[6].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[6].operand, "");
    assert_eq!(instrs[7].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[7].operand, "z");
}

#[test]
fn test_compile_only_dis() {
    let mut count: i32 = 0;
    let instrs = compile("dis 42;", &mut count);
    assert_eq!(count, 2);
    assert_eq!(instrs.len(), 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "42");
    assert_eq!(instrs[1].opcode, OpCode::STK_DIS);
    assert_eq!(instrs[1].operand, "");
}

#[test]
fn test_compile_assignment() {
    let mut count: i32 = 0;
    let instrs = compile("let x = 5; x = 10; dis x;", &mut count);
    assert_eq!(count, 6);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "5");
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[1].operand, "x");
    assert_eq!(instrs[2].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[2].operand, "10");
    assert_eq!(instrs[3].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[3].operand, "x");
    assert_eq!(instrs[4].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[4].operand, "x");
    assert_eq!(instrs[5].opcode, OpCode::STK_DIS);
    assert_eq!(instrs[5].operand, "");
}

#[test]
fn test_compile_complex_arithmetic() {
    let mut count: i32 = 0;
    let instrs = compile("let a = 1 + 2 + 3 - 4 + 5;", &mut count);
    assert_eq!(count, 10);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "1");
    assert_eq!(instrs[1].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[1].operand, "2");
    assert_eq!(instrs[2].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[3].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[3].operand, "3");
    assert_eq!(instrs[4].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[5].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[5].operand, "4");
    assert_eq!(instrs[6].opcode, OpCode::BINARY_SUB);
    assert_eq!(instrs[7].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[7].operand, "5");
    assert_eq!(instrs[8].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[9].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[9].operand, "a");
}

#[test]
fn test_compile_then_eval_simple() {
    let mut count: i32 = 0;
    let instrs = compile("let x = 5 + 3 - 2;", &mut count);
    let mut frame = init_frame();
    eval(&mut frame, &instrs);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.variables[0], 6);
    assert_eq!(frame.sp, 0);
}

#[test]
fn test_compile_then_eval_complex() {
    let mut count: i32 = 0;
    let instrs = compile("let a = 1 + 2 + 3 - 4 + 5;", &mut count);
    let mut frame = init_frame();
    eval(&mut frame, &instrs);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.var_names[0], "a");
    assert_eq!(frame.variables[0], 7);
    assert_eq!(frame.sp, 0);
}

#[test]
fn test_compile_asts_directly() {
    let tokens = tokenize("let x = 5;");
    let asts = parse(&tokens);
    let mut count: i32 = 0;
    let instrs = compile_asts(&asts, &mut count);
    assert_eq!(count, 2);
    assert_eq!(instrs.len(), 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "5");
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[1].operand, "x");
}

#[test]
fn test_compile_statement_let() {
    // Build AST manually for `let x = 5;`
    let mut node = new_ast_node(TokenType::TOKEN_LET, "x");
    node.left = Some(new_ast_node(TokenType::TOKEN_INT, "5"));
    let mut count: i32 = 0;
    let instrs = compile_statement(&node, &mut count);
    assert_eq!(count, 2);
    assert_eq!(instrs.len(), 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "5");
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[1].operand, "x");
}

#[test]
fn test_compile_statement_assign() {
    // Build AST manually for `x = 9;`
    let mut node = new_ast_node(TokenType::TOKEN_ASSIGN, "x");
    node.left = Some(new_ast_node(TokenType::TOKEN_INT, "9"));
    let mut count: i32 = 0;
    let instrs = compile_statement(&node, &mut count);
    assert_eq!(count, 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "9");
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[1].operand, "x");
}

#[test]
fn test_compile_statement_dis() {
    // dis 42; — note that the parser sets value = "" for TOKEN_DIS
    let mut node = new_ast_node(TokenType::TOKEN_DIS, "");
    node.left = Some(new_ast_node(TokenType::TOKEN_INT, "42"));
    let mut count: i32 = 0;
    let instrs = compile_statement(&node, &mut count);
    assert_eq!(count, 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "42");
    assert_eq!(instrs[1].opcode, OpCode::STK_DIS);
    assert_eq!(instrs[1].operand, "");
}

#[test]
fn test_compile_expression_int() {
    let node = new_ast_node(TokenType::TOKEN_INT, "7");
    let mut instrs = Vec::new();
    let mut count: i32 = 0;
    compile_expression(&node, &mut instrs, &mut count);
    assert_eq!(count, 1);
    assert_eq!(instrs.len(), 1);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "7");
}

#[test]
fn test_compile_expression_identifier() {
    let node = new_ast_node(TokenType::TOKEN_IDENTIFIER, "y");
    let mut instrs = Vec::new();
    let mut count: i32 = 0;
    compile_expression(&node, &mut instrs, &mut count);
    assert_eq!(count, 1);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[0].operand, "y");
}

#[test]
fn test_compile_expression_plus() {
    let mut node = new_ast_node(TokenType::TOKEN_PLUS, "+");
    node.left = Some(new_ast_node(TokenType::TOKEN_INT, "1"));
    node.right = Some(new_ast_node(TokenType::TOKEN_INT, "2"));
    let mut instrs = Vec::new();
    let mut count: i32 = 0;
    compile_expression(&node, &mut instrs, &mut count);
    assert_eq!(count, 3);
    assert_eq!(instrs.len(), 3);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "1");
    assert_eq!(instrs[1].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[1].operand, "2");
    assert_eq!(instrs[2].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[2].operand, "");
}

#[test]
fn test_compile_expression_minus() {
    let mut node = new_ast_node(TokenType::TOKEN_MINUS, "-");
    node.left = Some(new_ast_node(TokenType::TOKEN_INT, "10"));
    node.right = Some(new_ast_node(TokenType::TOKEN_INT, "4"));
    let mut instrs = Vec::new();
    let mut count: i32 = 0;
    compile_expression(&node, &mut instrs, &mut count);
    assert_eq!(count, 3);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "10");
    assert_eq!(instrs[1].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[1].operand, "4");
    assert_eq!(instrs[2].opcode, OpCode::BINARY_SUB);
    assert_eq!(instrs[2].operand, "");
}

#[test]
fn test_emit_appends_instruction_and_increments_count() {
    let mut instrs = Vec::new();
    let mut count: i32 = 0;
    emit(&mut instrs, &mut count, OpCode::LOAD_CONST, "55");
    assert_eq!(count, 1);
    assert_eq!(instrs.len(), 1);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "55");

    emit(&mut instrs, &mut count, OpCode::BINARY_ADD, "");
    assert_eq!(count, 2);
    assert_eq!(instrs.len(), 2);
    assert_eq!(instrs[1].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[1].operand, "");
}

#[test]
fn test_compile_subtract_only() {
    let mut count: i32 = 0;
    let instrs = compile("let x = 100 - 50;", &mut count);
    assert_eq!(count, 4);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "100");
    assert_eq!(instrs[1].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[1].operand, "50");
    assert_eq!(instrs[2].opcode, OpCode::BINARY_SUB);
    assert_eq!(instrs[2].operand, "");
    assert_eq!(instrs[3].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[3].operand, "x");
}

#[test]
fn test_compile_zero() {
    let mut count: i32 = 0;
    let instrs = compile("let z = 0;", &mut count);
    assert_eq!(count, 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "0");
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[1].operand, "z");
}

fn main() {}

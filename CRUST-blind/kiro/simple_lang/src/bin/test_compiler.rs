use simple_lang::compiler;
use simple_lang::vm::OpCode;

#[test]
fn test_compile_let_statement() {
    let mut count = 0;
    let instrs = compiler::compile("let x = 10;", &mut count);
    assert_eq!(count, 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[0].operand, "10");
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[1].operand, "x");
}

#[test]
fn test_compile_dis_statement() {
    let mut count = 0;
    let instrs = compiler::compile("let x = 10; dis x;", &mut count);
    // let x = 10 -> LOAD_CONST 10, STORE_NAME x
    // dis x -> LOAD_NAME x, STK_DIS
    assert_eq!(count, 4);
    assert_eq!(instrs[2].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[2].operand, "x");
    assert_eq!(instrs[3].opcode, OpCode::STK_DIS);
}

#[test]
fn test_compile_addition() {
    let mut count = 0;
    let instrs = compiler::compile("let x = 10; let y = 20; dis x + y;", &mut count);
    // let x=10: LOAD_CONST 10, STORE_NAME x (2)
    // let y=20: LOAD_CONST 20, STORE_NAME y (2)
    // dis x+y: LOAD_NAME x, LOAD_NAME y, BINARY_ADD, STK_DIS (4)
    assert_eq!(count, 8);
    assert_eq!(instrs[4].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[5].opcode, OpCode::LOAD_NAME);
    assert_eq!(instrs[6].opcode, OpCode::BINARY_ADD);
    assert_eq!(instrs[7].opcode, OpCode::STK_DIS);
}

#[test]
fn test_compile_subtraction() {
    let mut count = 0;
    let instrs = compiler::compile("let a = 5; let b = 3; dis a - b;", &mut count);
    assert_eq!(count, 8);
    assert_eq!(instrs[6].opcode, OpCode::BINARY_SUB);
}

#[test]
fn test_compile_reassignment() {
    let mut count = 0;
    let instrs = compiler::compile("let x = 1; x = 2;", &mut count);
    // let x=1: LOAD_CONST 1, STORE_NAME x (2)
    // x=2: LOAD_CONST 2, STORE_NAME x (2)
    assert_eq!(count, 4);
    assert_eq!(instrs[2].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[2].operand, "2");
    assert_eq!(instrs[3].opcode, OpCode::STORE_NAME);
    assert_eq!(instrs[3].operand, "x");
}

#[test]
fn test_compile_asts() {
    let tokens = simple_lang::lexer::tokenize("let x = 10;");
    let asts = simple_lang::parser::parse(&tokens);
    let mut count = 0;
    let instrs = compiler::compile_asts(&asts, &mut count);
    assert_eq!(count, 2);
    assert_eq!(instrs[0].opcode, OpCode::LOAD_CONST);
    assert_eq!(instrs[1].opcode, OpCode::STORE_NAME);
}

fn main() {}

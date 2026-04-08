use simple_lang::vm::{self, OpCode};
use simple_lang::compiler;

#[test]
fn test_init_frame() {
    let frame = vm::init_frame();
    assert_eq!(frame.sp, 0);
    assert_eq!(frame.var_count, 0);
}

#[test]
fn test_free_frame_no_panic() {
    let mut frame = vm::init_frame();
    vm::free_frame(&mut frame);
}

#[test]
fn test_new_instruction() {
    let instr = vm::new_instruction(OpCode::LOAD_CONST, "42");
    assert_eq!(instr.opcode, OpCode::LOAD_CONST);
    assert_eq!(instr.operand, "42");
}

#[test]
fn test_new_instruction_empty_operand() {
    let instr = vm::new_instruction(OpCode::BINARY_ADD, "");
    assert_eq!(instr.opcode, OpCode::BINARY_ADD);
    assert_eq!(instr.operand, "");
}

#[test]
fn test_free_instruction_no_panic() {
    let mut instr = vm::new_instruction(OpCode::LOAD_CONST, "1");
    vm::free_instruction(&mut instr);
}

#[test]
fn test_eval_load_const() {
    let mut frame = vm::init_frame();
    let instrs = vec![vm::new_instruction(OpCode::LOAD_CONST, "42")];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 42);
}

#[test]
fn test_eval_store_and_load_name() {
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "10"),
        vm::new_instruction(OpCode::STORE_NAME, "x"),
        vm::new_instruction(OpCode::LOAD_NAME, "x"),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.variables[0], 10);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 10);
}

#[test]
fn test_eval_binary_add() {
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "10"),
        vm::new_instruction(OpCode::LOAD_CONST, "20"),
        vm::new_instruction(OpCode::BINARY_ADD, ""),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 30);
}

#[test]
fn test_eval_binary_sub() {
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "5"),
        vm::new_instruction(OpCode::LOAD_CONST, "3"),
        vm::new_instruction(OpCode::BINARY_SUB, ""),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 2);
}

#[test]
fn test_eval_reassignment() {
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let x = 1; x = 2; dis x;", &mut count);
    vm::eval(&mut frame, &instrs);
    // After reassignment, x should be 2
    assert_eq!(frame.variables[0], 2);
    assert_eq!(frame.var_count, 1);
}

#[test]
fn test_eval_multiple_variables() {
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let a = 1; let b = 2; let c = 3;", &mut count);
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.var_count, 3);
    assert_eq!(frame.variables[0], 1);
    assert_eq!(frame.variables[1], 2);
    assert_eq!(frame.variables[2], 3);
}

// End-to-end tests matching C ground truth
#[test]
fn test_e2e_dis_single_var() {
    // C ground truth: 'let x=10; dis x;' -> 10
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let x = 10; dis x;", &mut count);
    vm::eval(&mut frame, &instrs);
    // After eval, top of stack should be 10 (from STK_DIS which doesn't pop)
    assert_eq!(frame.stack[frame.sp as usize], 10);
}

#[test]
fn test_e2e_addition() {
    // C ground truth: 'let x=10; let y=20; dis x+y;' -> 30
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let x = 10; let y = 20; dis x + y;", &mut count);
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.stack[frame.sp as usize], 30);
}

#[test]
fn test_e2e_subtraction() {
    // C ground truth: 'let a=5; let b=3; dis a-b;' -> 2
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let a = 5; let b = 3; dis a - b;", &mut count);
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.stack[frame.sp as usize], 2);
}

#[test]
fn test_e2e_reassignment() {
    // C ground truth: 'let x=1; x=2; dis x;' -> 2
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let x = 1; x = 2; dis x;", &mut count);
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.stack[frame.sp as usize], 2);
}

#[test]
fn test_e2e_computed_variable() {
    // C ground truth: 'let x=10; let y=20; let z=x+y; dis z;' -> 30
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let x = 10; let y = 20; let z = x + y; dis z;", &mut count);
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.stack[frame.sp as usize], 30);
}

#[test]
fn test_e2e_mixed_ops() {
    // C ground truth: 'let x=100; let y=50; dis x-y+10;' -> 60
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let x = 100; let y = 50; dis x - y + 10;", &mut count);
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.stack[frame.sp as usize], 60);
}

#[test]
fn test_e2e_three_var_addition() {
    // C ground truth: 'let a=1; let b=2; let c=3; dis a+b+c;' -> 6
    let mut frame = vm::init_frame();
    let mut count = 0;
    let instrs = compiler::compile("let a = 1; let b = 2; let c = 3; dis a + b + c;", &mut count);
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.stack[frame.sp as usize], 6);
}

#[test]
fn test_eval_zero_value() {
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "0"),
        vm::new_instruction(OpCode::STORE_NAME, "x"),
        vm::new_instruction(OpCode::LOAD_NAME, "x"),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.variables[0], 0);
    assert_eq!(frame.stack[frame.sp as usize], 0);
}

fn main() {}

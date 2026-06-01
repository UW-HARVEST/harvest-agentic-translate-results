use simple_lang::compiler;
use simple_lang::vm;

#[test]
fn test_init_frame() {
    let frame = vm::init_frame();
    assert_eq!(frame.sp, 0);
    assert_eq!(frame.var_count, 0);
}

#[test]
fn test_new_instruction() {
    let instr = vm::new_instruction(vm::OpCode::LOAD_CONST, "42");
    assert_eq!(instr.opcode, vm::OpCode::LOAD_CONST);
    assert_eq!(instr.operand, "42");
}

#[test]
fn test_eval_single_let() {
    // From C ground truth: "let x = 5 + 3 - 2;" => x = 6
    let mut count: i32 = 0;
    let instr = compiler::compile("let x = 5 + 3 - 2;", &mut count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instr);
    assert_eq!(frame.sp, 0);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.variables[0], 6);
}

#[test]
fn test_eval_two_lets() {
    // From C ground truth: "let x = 5; let y = x + 3;" => x=5, y=8
    let mut count: i32 = 0;
    let instr = compiler::compile("let x = 5; let y = x + 3;", &mut count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instr);
    assert_eq!(frame.sp, 0);
    assert_eq!(frame.var_count, 2);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.variables[0], 5);
    assert_eq!(frame.var_names[1], "y");
    assert_eq!(frame.variables[1], 8);
}

#[test]
fn test_eval_let_then_assign() {
    // From C ground truth: "let x = 5; x = x + 1;" => x = 6, var_count = 1
    let mut count: i32 = 0;
    let instr = compiler::compile("let x = 5; x = x + 1;", &mut count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instr);
    assert_eq!(frame.sp, 0);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.variables[0], 6);
}

#[test]
fn test_eval_single_int() {
    // From C ground truth: "let x = 100;" => x = 100
    let mut count: i32 = 0;
    let instr = compiler::compile("let x = 100;", &mut count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instr);
    assert_eq!(frame.sp, 0);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.variables[0], 100);
}

#[test]
fn test_eval_dis_literal() {
    // From C ground truth: "dis 42;" => sp=1 var_count=0 stack[1]=42
    let mut count: i32 = 0;
    let instr = compiler::compile("dis 42;", &mut count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instr);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.var_count, 0);
    assert_eq!(frame.stack[1], 42);
}

#[test]
fn test_eval_load_const_only() {
    // Build a simple program: just LOAD_CONST 42
    let instructions = vec![vm::new_instruction(vm::OpCode::LOAD_CONST, "42")];
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instructions);
    // After ++sp, sp=1 (started at 0), stack[1]=42
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 42);
}

#[test]
fn test_eval_binary_add() {
    let instructions = vec![
        vm::new_instruction(vm::OpCode::LOAD_CONST, "10"),
        vm::new_instruction(vm::OpCode::LOAD_CONST, "5"),
        vm::new_instruction(vm::OpCode::BINARY_ADD, ""),
    ];
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instructions);
    // sp went 0->1 (10), 1->2 (5), then BINARY_ADD: stack[1] = 10+5 = 15, sp=1
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 15);
}

#[test]
fn test_eval_binary_sub() {
    let instructions = vec![
        vm::new_instruction(vm::OpCode::LOAD_CONST, "10"),
        vm::new_instruction(vm::OpCode::LOAD_CONST, "3"),
        vm::new_instruction(vm::OpCode::BINARY_SUB, ""),
    ];
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instructions);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 7);
}

#[test]
fn test_free_instruction_no_panic() {
    let mut instr = vm::new_instruction(vm::OpCode::LOAD_CONST, "5");
    vm::free_instruction(&mut instr);
}

#[test]
fn test_free_frame_no_panic() {
    let mut frame = vm::init_frame();
    vm::free_frame(&mut frame);
}

fn main() {}

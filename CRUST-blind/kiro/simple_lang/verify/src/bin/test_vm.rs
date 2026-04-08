use simple_lang::vm::{self, OpCode};
use simple_lang::compiler;

#[test]
fn test_init_frame() {
    let frame = vm::init_frame();
    assert_eq!(frame.sp, 0);
    assert_eq!(frame.var_count, 0);
    assert_eq!(frame.stack[0], 0);
    assert_eq!(frame.variables[0], 0);
    assert_eq!(frame.var_names[0], "");
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
fn test_eval_load_const() {
    // C ground truth: LOAD_CONST does ++sp then stores. sp goes 0->1
    let mut frame = vm::init_frame();
    let instrs = vec![vm::new_instruction(OpCode::LOAD_CONST, "5")];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 5);
}

#[test]
fn test_eval_store_name_new_var() {
    // C ground truth: LOAD_CONST 5, STORE_NAME x -> sp=0, var_count=1, x=5
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "5"),
        vm::new_instruction(OpCode::STORE_NAME, "x"),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 0);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.variables[0], 5);
}

#[test]
fn test_eval_binary_add() {
    // C ground truth: 10+3=13, sp=1
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "10"),
        vm::new_instruction(OpCode::LOAD_CONST, "3"),
        vm::new_instruction(OpCode::BINARY_ADD, ""),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 13);
}

#[test]
fn test_eval_binary_sub() {
    // C ground truth: 10-3=7, sp=1
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "10"),
        vm::new_instruction(OpCode::LOAD_CONST, "3"),
        vm::new_instruction(OpCode::BINARY_SUB, ""),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 7);
}

#[test]
fn test_eval_full_let_dis() {
    // C ground truth: "let x = 5 + 3 - 2; dis x;" -> sp=1, var_count=1, x=6
    let mut instr_count = 0;
    let instrs = compiler::compile("let x = 5 + 3 - 2; dis x;", &mut instr_count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.variables[0], 6);
}

#[test]
fn test_eval_multi_var() {
    // C ground truth: "let x=10; let y=20; let z=x+y; dis z;" -> sp=1, var_count=3, x=10, y=20, z=30
    let mut instr_count = 0;
    let instrs = compiler::compile("let x = 10; let y = 20; let z = x + y; dis z;", &mut instr_count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.var_count, 3);
    assert_eq!(frame.variables[0], 10);
    assert_eq!(frame.variables[1], 20);
    assert_eq!(frame.variables[2], 30);
    assert_eq!(frame.var_names[0], "x");
    assert_eq!(frame.var_names[1], "y");
    assert_eq!(frame.var_names[2], "z");
}

#[test]
fn test_eval_reassign() {
    // C ground truth: "let x = 5; x = 10; dis x;" -> sp=1, var_count=1, x=10
    let mut instr_count = 0;
    let instrs = compiler::compile("let x = 5; x = 10; dis x;", &mut instr_count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.variables[0], 10);
}

#[test]
fn test_eval_subtraction_chain() {
    // C ground truth: "let a = 100 - 30 - 20; dis a;" -> sp=1, var_count=1, a=50
    let mut instr_count = 0;
    let instrs = compiler::compile("let a = 100 - 30 - 20; dis a;", &mut instr_count);
    let mut frame = vm::init_frame();
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.variables[0], 50);
    assert_eq!(frame.var_names[0], "a");
}

#[test]
fn test_eval_load_name() {
    // Store a var, then load it
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "42"),
        vm::new_instruction(OpCode::STORE_NAME, "v"),
        vm::new_instruction(OpCode::LOAD_NAME, "v"),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 42);
    assert_eq!(frame.var_count, 1);
    assert_eq!(frame.variables[0], 42);
}

#[test]
fn test_eval_stk_dis_no_pop() {
    // STK_DIS prints but doesn't pop - sp stays the same
    let mut frame = vm::init_frame();
    let instrs = vec![
        vm::new_instruction(OpCode::LOAD_CONST, "99"),
        vm::new_instruction(OpCode::STK_DIS, ""),
    ];
    vm::eval(&mut frame, &instrs);
    assert_eq!(frame.sp, 1);
    assert_eq!(frame.stack[1], 99);
}

#[test]
fn test_free_frame_no_panic() {
    let mut frame = vm::init_frame();
    vm::free_frame(&mut frame);
}

#[test]
fn test_free_instruction_no_panic() {
    let mut instr = vm::new_instruction(OpCode::LOAD_CONST, "1");
    vm::free_instruction(&mut instr);
}

#[test]
fn test_header_guard() {
    assert_eq!(vm::SIMPLE_LANG_VM_H, true);
}

#[test]
fn test_opcode_clone_eq() {
    let a = OpCode::LOAD_CONST;
    let b = a.clone();
    assert_eq!(a, b);
    assert_ne!(a, OpCode::STORE_NAME);
}

fn main() {}

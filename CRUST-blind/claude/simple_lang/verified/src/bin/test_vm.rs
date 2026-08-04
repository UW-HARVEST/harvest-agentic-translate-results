use simple_lang::settings::STAKE_LENGTH;
use simple_lang::vm::{eval, free_frame, free_instruction, init_frame, new_instruction, OpCode};

#[test]
fn test_init_frame_defaults() {
    let f = init_frame();
    assert_eq!(f.sp, 0);
    assert_eq!(f.var_count, 0);
    assert_eq!(f.stack.len(), STAKE_LENGTH as usize);
    assert_eq!(f.variables.len(), 100);
    assert_eq!(f.var_names.len(), 100);
    for v in f.variables.iter() {
        assert_eq!(*v, 0);
    }
    for s in f.stack.iter() {
        assert_eq!(*s, 0);
    }
    for n in f.var_names.iter() {
        assert_eq!(n, "");
    }
}

#[test]
fn test_new_instruction_with_operand() {
    let i = new_instruction(OpCode::LOAD_CONST, "42");
    assert_eq!(i.opcode, OpCode::LOAD_CONST);
    assert_eq!(i.operand, "42");
}

#[test]
fn test_new_instruction_no_operand() {
    let i = new_instruction(OpCode::BINARY_ADD, "");
    assert_eq!(i.opcode, OpCode::BINARY_ADD);
    assert_eq!(i.operand, "");
}

#[test]
fn test_free_instruction_no_panic() {
    let mut i = new_instruction(OpCode::BINARY_ADD, "");
    free_instruction(&mut i);
}

#[test]
fn test_free_frame_no_panic() {
    let mut f = init_frame();
    free_frame(&mut f);
}

#[test]
fn test_eval_load_const() {
    let mut f = init_frame();
    let instrs = vec![new_instruction(OpCode::LOAD_CONST, "42")];
    eval(&mut f, &instrs);
    assert_eq!(f.sp, 1);
    assert_eq!(f.stack[1], 42);
    assert_eq!(f.var_count, 0);
}

#[test]
fn test_eval_binary_add() {
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "5"),
        new_instruction(OpCode::LOAD_CONST, "3"),
        new_instruction(OpCode::BINARY_ADD, ""),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.sp, 1);
    assert_eq!(f.stack[1], 8);
    assert_eq!(f.var_count, 0);
}

#[test]
fn test_eval_binary_sub() {
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "10"),
        new_instruction(OpCode::LOAD_CONST, "4"),
        new_instruction(OpCode::BINARY_SUB, ""),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.sp, 1);
    assert_eq!(f.stack[1], 6);
}

#[test]
fn test_eval_store_name_creates_var() {
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "5"),
        new_instruction(OpCode::STORE_NAME, "x"),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.var_count, 1);
    assert_eq!(f.var_names[0], "x");
    assert_eq!(f.variables[0], 5);
    assert_eq!(f.sp, 0);
}

#[test]
fn test_eval_store_name_repeated_same_var() {
    // C: let x = 5; x = 10;
    // After: var_count=1, x=10, sp=0
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "5"),
        new_instruction(OpCode::STORE_NAME, "x"),
        new_instruction(OpCode::LOAD_CONST, "10"),
        new_instruction(OpCode::STORE_NAME, "x"),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.var_count, 1);
    assert_eq!(f.var_names[0], "x");
    assert_eq!(f.variables[0], 10);
    assert_eq!(f.sp, 0);
}

#[test]
fn test_eval_store_two_distinct_vars() {
    // let x=5; let y=10;
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "5"),
        new_instruction(OpCode::STORE_NAME, "x"),
        new_instruction(OpCode::LOAD_CONST, "10"),
        new_instruction(OpCode::STORE_NAME, "y"),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.var_count, 2);
    assert_eq!(f.var_names[0], "x");
    assert_eq!(f.variables[0], 5);
    assert_eq!(f.var_names[1], "y");
    assert_eq!(f.variables[1], 10);
    assert_eq!(f.sp, 0);
}

#[test]
fn test_eval_load_name() {
    // let x = 5; let y = x + 3;
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "5"),
        new_instruction(OpCode::STORE_NAME, "x"),
        new_instruction(OpCode::LOAD_NAME, "x"),
        new_instruction(OpCode::LOAD_CONST, "3"),
        new_instruction(OpCode::BINARY_ADD, ""),
        new_instruction(OpCode::STORE_NAME, "y"),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.var_count, 2);
    assert_eq!(f.var_names[0], "x");
    assert_eq!(f.variables[0], 5);
    assert_eq!(f.var_names[1], "y");
    assert_eq!(f.variables[1], 8);
    assert_eq!(f.sp, 0);
}

#[test]
fn test_eval_reassign_after_other_var_quirk() {
    // C quirk: let a=1; let b=2; a=3 produces 3 entries in var_names
    // (matches the C bug where update + append both happen).
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "1"),
        new_instruction(OpCode::STORE_NAME, "a"),
        new_instruction(OpCode::LOAD_CONST, "2"),
        new_instruction(OpCode::STORE_NAME, "b"),
        new_instruction(OpCode::LOAD_CONST, "3"),
        new_instruction(OpCode::STORE_NAME, "a"),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.var_count, 3);
    assert_eq!(f.var_names[0], "a");
    assert_eq!(f.variables[0], 3);
    assert_eq!(f.var_names[1], "b");
    assert_eq!(f.variables[1], 2);
    assert_eq!(f.var_names[2], "a");
    assert_eq!(f.variables[2], 0);
    assert_eq!(f.sp, -1);
}

#[test]
fn test_eval_dis_does_not_pop() {
    // STK_DIS prints frame.stack[sp] but does NOT pop.
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "42"),
        new_instruction(OpCode::STK_DIS, ""),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.sp, 1);
    assert_eq!(f.stack[1], 42);
    assert_eq!(f.var_count, 0);
}

#[test]
fn test_eval_subtraction_only() {
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "100"),
        new_instruction(OpCode::LOAD_CONST, "50"),
        new_instruction(OpCode::BINARY_SUB, ""),
        new_instruction(OpCode::STORE_NAME, "x"),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.var_count, 1);
    assert_eq!(f.var_names[0], "x");
    assert_eq!(f.variables[0], 50);
    assert_eq!(f.sp, 0);
}

#[test]
fn test_eval_complex_arithmetic() {
    // 1 + 2 + 3 - 4 + 5 = 7
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "1"),
        new_instruction(OpCode::LOAD_CONST, "2"),
        new_instruction(OpCode::BINARY_ADD, ""),
        new_instruction(OpCode::LOAD_CONST, "3"),
        new_instruction(OpCode::BINARY_ADD, ""),
        new_instruction(OpCode::LOAD_CONST, "4"),
        new_instruction(OpCode::BINARY_SUB, ""),
        new_instruction(OpCode::LOAD_CONST, "5"),
        new_instruction(OpCode::BINARY_ADD, ""),
        new_instruction(OpCode::STORE_NAME, "a"),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.var_count, 1);
    assert_eq!(f.var_names[0], "a");
    assert_eq!(f.variables[0], 7);
    assert_eq!(f.sp, 0);
}

#[test]
fn test_eval_zero_constant() {
    let mut f = init_frame();
    let instrs = vec![
        new_instruction(OpCode::LOAD_CONST, "0"),
        new_instruction(OpCode::STORE_NAME, "z"),
    ];
    eval(&mut f, &instrs);
    assert_eq!(f.var_count, 1);
    assert_eq!(f.var_names[0], "z");
    assert_eq!(f.variables[0], 0);
    assert_eq!(f.sp, 0);
}

fn main() {}

use jccc::codegen::{
    code_gen_init, end_func, end_main, end_main_custom_return, init_int_literal,
    op_on_rax_with_rdi, start_func, start_main, test_init_int_literal,
    test_op_on_rax_with_rdi, GenState, Op,
};

#[test]
fn test_start_main_string() {
    let s = start_main();
    assert_eq!(s, "global _start\nsection .text\n\n_start:\n");
}

#[test]
fn test_end_main_custom_return_zero() {
    let s = end_main_custom_return(0);
    assert_eq!(s, "\tmov rax, 60\n\tmov rdi, 0\n\tsyscall\n");
}

#[test]
fn test_end_main_custom_return_99() {
    let s = end_main_custom_return(99);
    assert_eq!(s, "\tmov rax, 60\n\tmov rdi, 99\n\tsyscall\n");
}

#[test]
fn test_end_main_custom_return_negative() {
    let s = end_main_custom_return(-1);
    assert_eq!(s, "\tmov rax, 60\n\tmov rdi, -1\n\tsyscall\n");
}

#[test]
fn test_op_on_rax_with_rdi_add() {
    let s = op_on_rax_with_rdi(Op::OpAdd);
    assert_eq!(s, "\tadd rax, rdi\n");
}

#[test]
fn test_op_on_rax_with_rdi_sub() {
    let s = op_on_rax_with_rdi(Op::OpSub);
    assert_eq!(s, "\tsub rax, rdi\n");
}

#[test]
fn test_op_on_rax_with_rdi_mov() {
    let s = op_on_rax_with_rdi(Op::OpMov);
    assert_eq!(s, "\tmov rax, rdi\n");
}

#[test]
fn test_init_int_literal_basic() {
    code_gen_init();
    let s = init_int_literal(100);
    assert_eq!(s, "\tmov [rsp+8], 100");
}

#[test]
fn test_init_int_literal_increments_offset() {
    code_gen_init();
    let s1 = init_int_literal(42);
    let s2 = init_int_literal(7);
    assert_eq!(s1, "\tmov [rsp+8], 42");
    assert_eq!(s2, "\tmov [rsp+16], 7");
}

#[test]
fn test_init_int_literal_negative() {
    code_gen_init();
    let s = init_int_literal(-5);
    assert_eq!(s, "\tmov [rsp+8], -5");
}

#[test]
fn test_test_init_int_literal() {
    assert_eq!(test_init_int_literal(), 0);
}

#[test]
fn test_test_op_on_rax_with_rdi() {
    assert_eq!(test_op_on_rax_with_rdi(), 0);
}

#[test]
fn test_start_func_nonempty() {
    // The C version returns an unusual non-newline-separated string.
    // We just verify it returns something non-empty.
    let s = start_func();
    assert!(!s.is_empty());
    assert!(s.contains("sub rsp, 32"));
}

#[test]
fn test_end_func_nonempty() {
    let s = end_func();
    assert!(!s.is_empty());
    assert!(s.contains("add rsp, 32"));
}

#[test]
fn test_end_main_nonempty() {
    let s = end_main();
    assert!(!s.is_empty());
    assert!(s.contains("mov rax, 60"));
    assert!(s.contains("mov rdi, 0"));
    assert!(s.contains("syscall"));
}

#[test]
fn test_gen_state_init_values() {
    let g = GenState {
        registers_in_use: 5,
        rsp_offset: 16,
    };
    assert_eq!(g.registers_in_use, 5);
    assert_eq!(g.rsp_offset, 16);
}

#[test]
fn test_code_gen_init_resets_state() {
    code_gen_init();
    init_int_literal(1);
    init_int_literal(2);
    code_gen_init();
    let s = init_int_literal(1);
    assert_eq!(s, "\tmov [rsp+8], 1");
}

fn main() {}

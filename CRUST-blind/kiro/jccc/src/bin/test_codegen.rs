use jccc::codegen::*;

#[test]
fn test_op_on_rax_with_rdi_add() {
    assert_eq!(op_on_rax_with_rdi(Op::OpAdd), "\tadd rax, rdi\n");
}

#[test]
fn test_op_on_rax_with_rdi_sub() {
    assert_eq!(op_on_rax_with_rdi(Op::OpSub), "\tsub rax, rdi\n");
}

#[test]
fn test_op_on_rax_with_rdi_mov() {
    assert_eq!(op_on_rax_with_rdi(Op::OpMov), "\tmov rax, rdi\n");
}

#[test]
fn test_op_on_rax_with_rdi_nop() {
    assert_eq!(op_on_rax_with_rdi(Op::OpNop), "\tnop rax, rdi\n");
}

#[test]
fn test_start_main() {
    assert_eq!(start_main(), "global _start\nsection .text\n\n_start:\n");
}

#[test]
fn test_end_main() {
    assert_eq!(end_main(), "\tmov rax, 60\tmov rdi, 0\tsyscall");
}

#[test]
fn test_end_main_custom_return_zero() {
    assert_eq!(end_main_custom_return(0), "\tmov rax, 60\n\tmov rdi, 0\n\tsyscall\n");
}

#[test]
fn test_end_main_custom_return_42() {
    assert_eq!(end_main_custom_return(42), "\tmov rax, 60\n\tmov rdi, 42\n\tsyscall\n");
}

#[test]
fn test_start_func() {
    assert_eq!(start_func(), "\tsub rsp, 32\tmov [rsp], r12\tmov [rsp+8], r13\tmov [rsp+16], r14\tmov [rsp+24], r15");
}

#[test]
fn test_end_func() {
    assert_eq!(end_func(), "\tmov r12, [rsp]\tmov r13, [rsp+8]\tmov r14, [rsp+16]\tmov r15, [rsp+24]\tadd rsp, 32");
}

#[test]
fn test_init_int_literal() {
    code_gen_init();
    assert_eq!(init_int_literal(100), "\tmov [rsp+8], 100");
}

#[test]
fn test_code_gen_init_resets() {
    // Call init, then init_int_literal, then init again to verify reset
    code_gen_init();
    let _ = init_int_literal(1);
    code_gen_init();
    assert_eq!(init_int_literal(200), "\tmov [rsp+8], 200");
}

fn main() {}

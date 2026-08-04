use jccc::codegen::{
    code_gen_init, end_func, end_main, end_main_custom_return, init_int_literal,
    op_on_rax_with_rdi, start_func, start_main, test_init_int_literal, test_op_on_rax_with_rdi,
    Op,
};

#[test]
fn test_start_main_value() {
    assert_eq!(
        start_main(),
        "global _start\nsection .text\n\n_start:\n"
    );
}

#[test]
fn test_end_main_value() {
    assert_eq!(end_main(), "\tmov rax, 60\tmov rdi, 0\tsyscall");
}

#[test]
fn test_end_main_custom_return_basic() {
    assert_eq!(
        end_main_custom_return(0),
        "\tmov rax, 60\n\tmov rdi, 0\n\tsyscall\n"
    );
    assert_eq!(
        end_main_custom_return(42),
        "\tmov rax, 60\n\tmov rdi, 42\n\tsyscall\n"
    );
    assert_eq!(
        end_main_custom_return(123),
        "\tmov rax, 60\n\tmov rdi, 123\n\tsyscall\n"
    );
}

#[test]
fn test_end_main_custom_return_negative() {
    assert_eq!(
        end_main_custom_return(-1),
        "\tmov rax, 60\n\tmov rdi, -1\n\tsyscall\n"
    );
}

#[test]
fn test_start_func_value() {
    assert_eq!(
        start_func(),
        "\tsub rsp, 32\tmov [rsp], r12\tmov [rsp+8], r13\tmov [rsp+16], r14\tmov [rsp+24], r15"
    );
}

#[test]
fn test_end_func_value() {
    assert_eq!(
        end_func(),
        "\tmov r12, [rsp]\tmov r13, [rsp+8]\tmov r14, [rsp+16]\tmov r15, [rsp+24]\tadd rsp, 32"
    );
}

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
fn test_init_int_literal_sequence() {
    code_gen_init();
    assert_eq!(init_int_literal(100), "\tmov [rsp+8], 100");
    // Each successive call increments rsp_offset by 8.
    assert_eq!(init_int_literal(50), "\tmov [rsp+16], 50");
    assert_eq!(init_int_literal(7), "\tmov [rsp+24], 7");
}

#[test]
fn test_init_int_literal_after_reset() {
    code_gen_init();
    let r = init_int_literal(100);
    assert_eq!(r, "\tmov [rsp+8], 100");
    code_gen_init();
    let r = init_int_literal(99);
    assert_eq!(r, "\tmov [rsp+8], 99");
}

#[test]
fn test_test_init_int_literal_helper() {
    assert_eq!(test_init_int_literal(), 0);
}

#[test]
fn test_test_op_on_rax_with_rdi_helper() {
    assert_eq!(test_op_on_rax_with_rdi(), 0);
}

fn main() {}

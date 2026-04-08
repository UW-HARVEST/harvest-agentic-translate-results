use jccc::codegen::{
    code_gen_init, end_func, end_main, end_main_custom_return, init_int_literal,
    op_on_rax_with_rdi, start_func, start_main, ttype_to_op, Op,
};
use jccc::token::TokenType;

#[test]
fn test_start_main() {
    assert_eq!(start_main(), "global _start\nsection .text\n\n_start:\n");
}

#[test]
fn test_end_main() {
    assert_eq!(end_main(), "\tmov rax, 60\tmov rdi, 0\tsyscall");
}

#[test]
fn test_end_main_custom_return() {
    assert_eq!(
        end_main_custom_return(42),
        "\tmov rax, 60\n\tmov rdi, 42\n\tsyscall\n"
    );
    assert_eq!(
        end_main_custom_return(0),
        "\tmov rax, 60\n\tmov rdi, 0\n\tsyscall\n"
    );
}

#[test]
fn test_op_on_rax_with_rdi() {
    assert_eq!(op_on_rax_with_rdi(Op::OP_ADD), "\tadd rax, rdi\n");
    assert_eq!(op_on_rax_with_rdi(Op::OP_SUB), "\tsub rax, rdi\n");
    assert_eq!(op_on_rax_with_rdi(Op::OP_MOV), "\tmov rax, rdi\n");
    assert_eq!(op_on_rax_with_rdi(Op::OP_NOP), "\tnop rax, rdi\n");
}

#[test]
fn test_start_func() {
    assert_eq!(
        start_func(),
        "\tsub rsp, 32\tmov [rsp], r12\tmov [rsp+8], r13\tmov [rsp+16], r14\tmov [rsp+24], r15"
    );
}

#[test]
fn test_end_func() {
    assert_eq!(
        end_func(),
        "\tmov r12, [rsp]\tmov r13, [rsp+8]\tmov r14, [rsp+16]\tmov r15, [rsp+24]\tadd rsp, 32"
    );
}

#[test]
fn test_init_int_literal() {
    code_gen_init();
    assert_eq!(init_int_literal(100), "\tmov [rsp+8], 100");
}

#[test]
fn test_ttype_to_op() {
    match ttype_to_op(&TokenType::TT_PLUS) {
        Op::OP_ADD => {}
        _ => panic!("TT_PLUS should map to OP_ADD"),
    }
    match ttype_to_op(&TokenType::TT_MINUS) {
        Op::OP_SUB => {}
        _ => panic!("TT_MINUS should map to OP_SUB"),
    }
    match ttype_to_op(&TokenType::TT_STAR) {
        Op::OP_NOP => {}
        _ => panic!("TT_STAR should map to OP_NOP"),
    }
}

fn main() {}

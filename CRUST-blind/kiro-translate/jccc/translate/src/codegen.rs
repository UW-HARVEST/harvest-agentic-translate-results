use crate::token::TokenType;
use std::sync::Mutex;

/// A placeholder enum for an operation type that might be applied to registers.
#[derive(Debug)]
pub enum Op {
OP_ADD,
OP_SUB,
OP_MOV,
OP_NOP,
}
/// Represents the code generation state.
#[derive(Debug)]
pub struct GenState {
pub registers_in_use: u32,
pub rsp_offset: u32,
}

static GEN_STATE: Mutex<GenState> = Mutex::new(GenState {
    registers_in_use: 0,
    rsp_offset: 0,
});

pub fn ttype_to_op(t: &TokenType) -> Op {
    match t {
        TokenType::TT_PLUS => Op::OP_ADD,
        TokenType::TT_MINUS => Op::OP_SUB,
        _ => Op::OP_NOP,
    }
}

/// Initializes the code generator.
pub fn code_gen_init() {
    let mut state = GEN_STATE.lock().unwrap();
    state.registers_in_use = 0;
    state.rsp_offset = 0;
}

/// Starts the main function (assembly or IR) generation.
pub fn start_main() -> String {
    "global _start\nsection .text\n\n_start:\n".to_string()
}

/// Ends the main function with a default return.
pub fn end_main() -> String {
    "\tmov rax, 60\tmov rdi, 0\tsyscall".to_string()
}

/// Ends the main function with a custom return value.
pub fn end_main_custom_return(val: i32) -> String {
    format!("\tmov rax, 60\n\tmov rdi, {}\n\tsyscall\n", val)
}

/// Generates code to operate on RAX with RDI according to the provided Op.
pub fn op_on_rax_with_rdi(op: Op) -> String {
    let op_str = match op {
        Op::OP_ADD => "add",
        Op::OP_SUB => "sub",
        Op::OP_MOV => "mov",
        Op::OP_NOP => "nop",
    };
    format!("\t{} rax, rdi\n", op_str)
}

/// Starts a generic function definition.
pub fn start_func() -> String {
    "\tsub rsp, 32\tmov [rsp], r12\tmov [rsp+8], r13\tmov [rsp+16], r14\tmov [rsp+24], r15".to_string()
}

/// Ends a function definition (assembly or IR).
pub fn end_func() -> String {
    "\tmov r12, [rsp]\tmov r13, [rsp+8]\tmov r14, [rsp+16]\tmov r15, [rsp+24]\tadd rsp, 32".to_string()
}

/// Initializes code for an integer literal.
pub fn init_int_literal(val: i32) -> String {
    let mut state = GEN_STATE.lock().unwrap();
    state.rsp_offset += 8;
    format!("\tmov [rsp+{}], {}", state.rsp_offset, val)
}

/// Tests the function that initializes integer literals.
pub fn test_init_int_literal() -> i32 {
    code_gen_init();
    assert_eq!(init_int_literal(100), "\tmov [rsp+8], 100");
    0
}

/// Tests the operation on RAX with RDI.
pub fn test_op_on_rax_with_rdi() -> i32 {
    let out = op_on_rax_with_rdi(Op::OP_ADD);
    assert_eq!(out, "\tadd rax, rdi\n");
    let out2 = op_on_rax_with_rdi(Op::OP_MOV);
    assert_eq!(out2, "\tmov rax, rdi\n");
    0
}

use std::sync::{Mutex, OnceLock};

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

fn state() -> &'static Mutex<GenState> {
    static GEN_STATE: OnceLock<Mutex<GenState>> = OnceLock::new();
    GEN_STATE.get_or_init(|| {
        Mutex::new(GenState {
            registers_in_use: 0,
            rsp_offset: 0,
        })
    })
}

/// Generates code to operate on RAX with RDI according to the provided Op.
pub fn op_on_rax_with_rdi(op: Op) -> String {
    let op_str = match op {
        Op::OP_ADD => "add",
        Op::OP_SUB => "sub",
        Op::OP_MOV => "mov",
        Op::OP_NOP => "",
    };
    format!("\t{} rax, rdi\n", op_str)
}
/// Initializes code for an integer literal.
pub fn init_int_literal(val: i32) -> String {
    let mut state = state().lock().expect("codegen state poisoned");
    state.rsp_offset += 8;
    format!("\tmov [rsp+{}], {}", state.rsp_offset, val)
}
/// Tests the function that initializes integer literals.
pub fn test_init_int_literal() -> i32 {
    code_gen_init();
    assert_eq!(init_int_literal(100), "\tmov [rsp+8], 100");
    0
}
/// Initializes the code generator.
pub fn code_gen_init() {
    let mut state = state().lock().expect("codegen state poisoned");
    state.registers_in_use = 0;
    state.rsp_offset = 0;
}
/// Starts the main function (assembly or IR) generation.
pub fn start_main() -> String {
    "global _start\nsection .text\n\n_start:\n".to_string()
}
/// Starts a generic function definition.
pub fn start_func() -> String {
    "\tsub rsp, 32\tmov [rsp], r12\tmov [rsp+8], r13\tmov [rsp+16], r14\tmov [rsp+24], r15"
        .to_string()
}
/// Tests the operation on RAX with RDI.
pub fn test_op_on_rax_with_rdi() -> i32 {
    assert_eq!(op_on_rax_with_rdi(Op::OP_ADD), "\tadd rax, rdi\n");
    assert_eq!(op_on_rax_with_rdi(Op::OP_MOV), "\tmov rax, rdi\n");
    0
}
/// Ends a function definition (assembly or IR).
pub fn end_func() -> String {
    "\tmov r12, [rsp]\tmov r13, [rsp+8]\tmov r14, [rsp+16]\tmov r15, [rsp+24]\tadd rsp, 32"
        .to_string()
}
/// Ends the main function with a custom return value.
pub fn end_main_custom_return(val: i32) -> String {
    format!("\tmov rax, 60\n\tmov rdi, {}\n\tsyscall\n", val)
}
/// Ends the main function with a default return.
pub fn end_main() -> String {
    "\tmov rax, 60\tmov rdi, 0\tsyscall".to_string()
}

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

/// A placeholder enum for an operation type that might be applied to registers.
#[derive(Debug, Clone, Copy)]
pub enum Op {
    OpAdd,
    OpSub,
    OpMov,
    OpNop,
}

/// Represents the code generation state.
#[derive(Debug)]
pub struct GenState {
    pub registers_in_use: u32,
    pub rsp_offset: u32,
}

// Global generator state mirroring the C `GEN_STATE` global.
// Use atomic primitives for thread safety without unsafe.
static GEN_REGISTERS_IN_USE: AtomicU32 = AtomicU32::new(0);
static GEN_RSP_OFFSET: AtomicU32 = AtomicU32::new(0);

/// Generates code to operate on RAX with RDI according to the provided Op.
pub fn op_on_rax_with_rdi(op: Op) -> String {
    let op_strs = ["add", "sub", "mov"];
    let op_str = match op {
        Op::OpAdd => op_strs[0],
        Op::OpSub => op_strs[1],
        Op::OpMov => op_strs[2],
        // For OP_NOP the C code would index out of bounds; we handle gracefully.
        Op::OpNop => "",
    };
    format!("\t{} rax, rdi\n", op_str)
}
/// Initializes code for an integer literal.
pub fn init_int_literal(val: i32) -> String {
    let new_offset = GEN_RSP_OFFSET.fetch_add(8, Ordering::SeqCst) + 8;
    format!("\tmov [rsp+{}], {}", new_offset, val)
}
/// Tests the function that initializes integer literals.
pub fn test_init_int_literal() -> i32 {
    code_gen_init();
    assert_eq!(init_int_literal(100), "\tmov [rsp+8], 100");
    0
}
/// Initializes the code generator.
pub fn code_gen_init() {
    GEN_REGISTERS_IN_USE.store(0, Ordering::SeqCst);
    GEN_RSP_OFFSET.store(0, Ordering::SeqCst);
}
/// Starts the main function (assembly or IR) generation.
pub fn start_main() -> String {
    String::from("global _start\nsection .text\n\n_start:\n")
}
/// Starts a generic function definition.
pub fn start_func() -> String {
    String::from(
        "\tsub rsp, 32\
        \tmov [rsp], r12\
        \tmov [rsp+8], r13\
        \tmov [rsp+16], r14\
        \tmov [rsp+24], r15",
    )
}
/// Tests the operation on RAX with RDI.
pub fn test_op_on_rax_with_rdi() -> i32 {
    let out = op_on_rax_with_rdi(Op::OpAdd);
    assert_eq!(out, "\tadd rax, rdi\n");
    let out2 = op_on_rax_with_rdi(Op::OpMov);
    assert_eq!(out2, "\tmov rax, rdi\n");
    0
}
/// Ends a function definition (assembly or IR).
pub fn end_func() -> String {
    String::from(
        "\tmov r12, [rsp]\
        \tmov r13, [rsp+8]\
        \tmov r14, [rsp+16]\
        \tmov r15, [rsp+24]\
        \tadd rsp, 32",
    )
}
/// Ends the main function with a custom return value.
pub fn end_main_custom_return(val: i32) -> String {
    format!("\tmov rax, 60\n\tmov rdi, {}\n\tsyscall\n", val)
}
/// Ends the main function with a default return.
pub fn end_main() -> String {
    String::from("\tmov rax, 60\tmov rdi, 0\tsyscall")
}

// Suppress unused warnings on structures we keep for parity with C.
#[allow(dead_code)]
fn _unused_refcell() -> RefCell<GenState> {
    RefCell::new(GenState {
        registers_in_use: 0,
        rsp_offset: 0,
    })
}

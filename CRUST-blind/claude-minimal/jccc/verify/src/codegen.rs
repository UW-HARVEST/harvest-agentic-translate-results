/// A placeholder enum for an operation type that might be applied to registers.
#[derive(Debug, Clone, Copy)]
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

/// Global code-generation state mirroring the C `GEN_STATE` global.
static mut GEN_STATE: GenState = GenState {
    registers_in_use: 0,
    rsp_offset: 0,
};

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
/// Initializes code for an integer literal.
pub fn init_int_literal(val: i32) -> String {
    // Mirrors the C version: bumps rsp_offset by 8 and emits the mov.
    unsafe {
        GEN_STATE.rsp_offset += 8;
        format!("\tmov [rsp+{}], {}", GEN_STATE.rsp_offset, val)
    }
}
/// Tests the function that initializes integer literals.
pub fn test_init_int_literal() -> i32 {
    code_gen_init();
    let result = init_int_literal(100);
    if result != "\tmov [rsp+8], 100" {
        return -1;
    }
    0
}
/// Initializes the code generator.
pub fn code_gen_init() {
    unsafe {
        GEN_STATE.registers_in_use = 0;
        GEN_STATE.rsp_offset = 0;
    }
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
    let out = op_on_rax_with_rdi(Op::OP_ADD);
    if out != "\tadd rax, rdi\n" {
        return -1;
    }
    let out2 = op_on_rax_with_rdi(Op::OP_MOV);
    if out2 != "\tmov rax, rdi\n" {
        return -1;
    }
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

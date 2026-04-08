use std::sync::Mutex;

#[derive(Debug)]
pub enum Op {
    OpAdd,
    OpSub,
    OpMov,
    OpNop,
}

#[derive(Debug)]
pub struct GenState {
    pub registers_in_use: u32,
    pub rsp_offset: u32,
}

static GEN_STATE: Mutex<GenState> = Mutex::new(GenState {
    registers_in_use: 0,
    rsp_offset: 0,
});

pub fn code_gen_init() {
    let mut state = GEN_STATE.lock().unwrap();
    state.registers_in_use = 0;
    state.rsp_offset = 0;
}

pub fn op_on_rax_with_rdi(op: Op) -> String {
    let op_str = match op {
        Op::OpAdd => "add",
        Op::OpSub => "sub",
        Op::OpMov => "mov",
        Op::OpNop => "nop",
    };
    format!("\t{} rax, rdi\n", op_str)
}

pub fn init_int_literal(val: i32) -> String {
    let mut state = GEN_STATE.lock().unwrap();
    state.rsp_offset += 8;
    format!("\tmov [rsp+{}], {}", state.rsp_offset, val)
}

pub fn test_init_int_literal() -> i32 {
    code_gen_init();
    assert_eq!(init_int_literal(100), "\tmov [rsp+8], 100");
    0
}

pub fn start_main() -> String {
    "global _start\nsection .text\n\n_start:\n".to_string()
}

pub fn end_main() -> String {
    "\tmov rax, 60\tmov rdi, 0\tsyscall".to_string()
}

pub fn end_main_custom_return(val: i32) -> String {
    format!("\tmov rax, 60\n\tmov rdi, {}\n\tsyscall\n", val)
}

pub fn start_func() -> String {
    "\tsub rsp, 32\tmov [rsp], r12\tmov [rsp+8], r13\tmov [rsp+16], r14\tmov [rsp+24], r15".to_string()
}

pub fn end_func() -> String {
    "\tmov r12, [rsp]\tmov r13, [rsp+8]\tmov r14, [rsp+16]\tmov r15, [rsp+24]\tadd rsp, 32".to_string()
}

pub fn test_op_on_rax_with_rdi() -> i32 {
    let out = op_on_rax_with_rdi(Op::OpAdd);
    assert_eq!(out, "\tadd rax, rdi\n");
    let out2 = op_on_rax_with_rdi(Op::OpMov);
    assert_eq!(out2, "\tmov rax, rdi\n");
    0
}

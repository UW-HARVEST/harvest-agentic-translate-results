use crate::{ast, settings};
pub const SIMPLE_LANG_VM_H: bool = true;

#[allow(unused_imports)]
use ast::ASTNode;

/// Replicates the C enum:
/// typedef enum { LOAD_CONST, LOAD_NAME, STORE_NAME, BINARY_ADD, BINARY_SUB, STK_DIS } OpCode;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    LOAD_CONST,
    LOAD_NAME,
    STORE_NAME,
    BINARY_ADD,
    BINARY_SUB,
    STK_DIS,
}
/// Replicates the C struct:
/// typedef struct {
///     OpCode opcode;
///     char* operand;
/// } Instruction;
#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: OpCode,
    pub operand: String,
}
/// Replicates the C struct Frame, referencing STAKE_LENGTH from settings.
#[derive(Debug, Clone)]
pub struct Frame {
    pub stack: [i32; settings::STAKE_LENGTH as usize],
    pub sp: i32,
    pub variables: [i32; 100],
    pub var_names: [String; 100],
    pub var_count: i32,
}
/// Replicates: Instruction* new_instruction(OpCode opcode, const char* operand);
pub fn new_instruction(opcode: OpCode, operand: &str) -> Instruction {
    Instruction {
        opcode,
        operand: operand.to_string(),
    }
}
/// Replicates: void free_instruction(Instruction* instruction);
pub fn free_instruction(_instruction: &mut Instruction) {
    // No-op in Rust; ownership/Drop manages memory.
}
/// Replicates: void eval(Frame* frame, Instruction* instructions, int instr_count);
/// In Rust: instructions is a slice of Instruction.
pub fn eval(frame: &mut Frame, instructions: &[Instruction]) {
    let instr_count = instructions.len();
    for pc in 0..instr_count {
        let instr = &instructions[pc];
        match instr.opcode {
            OpCode::LOAD_CONST => {
                // C: frame->stack[++frame->sp] = atoi(instr.operand);
                frame.sp += 1;
                let v: i32 = instr.operand.parse::<i32>().unwrap_or_else(|_| {
                    // emulate atoi: parse leading digits, default 0
                    let mut sign = 1i32;
                    let mut acc: i32 = 0;
                    let bytes = instr.operand.as_bytes();
                    let mut i = 0;
                    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
                        i += 1;
                    }
                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                        if bytes[i] == b'-' {
                            sign = -1;
                        }
                        i += 1;
                    }
                    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                        acc = acc.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i32);
                        i += 1;
                    }
                    sign.wrapping_mul(acc)
                });
                frame.stack[frame.sp as usize] = v;
            }
            OpCode::LOAD_NAME => {
                for i in 0..frame.var_count as usize {
                    if frame.var_names[i] == instr.operand {
                        frame.sp += 1;
                        frame.stack[frame.sp as usize] = frame.variables[i];
                        break;
                    }
                }
            }
            OpCode::STORE_NAME => {
                let mut found = false;
                for i in 0..frame.var_count as usize {
                    if frame.var_names[i] == instr.operand {
                        frame.variables[i] = frame.stack[frame.sp as usize];
                        frame.sp -= 1;
                        found = true;
                        break;
                    }
                }
                // Replicates the buggy C behavior where the second branch
                // appends the variable if either var_count==0 OR the LAST
                // variable's name doesn't match the operand.
                let var_count = frame.var_count as usize;
                let last_matches =
                    var_count > 0 && frame.var_names[var_count - 1] == instr.operand;
                if var_count == 0 || !last_matches {
                    frame.var_names[var_count] = instr.operand.clone();
                    frame.variables[var_count] = frame.stack[frame.sp as usize];
                    frame.sp -= 1;
                    frame.var_count += 1;
                }
                let _ = found;
            }
            OpCode::BINARY_ADD => {
                let sp = frame.sp as usize;
                frame.stack[sp - 1] = frame.stack[sp - 1].wrapping_add(frame.stack[sp]);
                frame.sp -= 1;
            }
            OpCode::BINARY_SUB => {
                let sp = frame.sp as usize;
                frame.stack[sp - 1] = frame.stack[sp - 1].wrapping_sub(frame.stack[sp]);
                frame.sp -= 1;
            }
            OpCode::STK_DIS => {
                println!("{}", frame.stack[frame.sp as usize]);
            }
        }
    }
}
/// Replicates: Frame* init_frame();
pub fn init_frame() -> Frame {
    const EMPTY_STR: String = String::new();
    Frame {
        stack: [0i32; settings::STAKE_LENGTH as usize],
        sp: 0,
        variables: [0i32; 100],
        var_names: [EMPTY_STR; 100],
        var_count: 0,
    }
}
/// Replicates: void free_frame(Frame* frame);
pub fn free_frame(_frame: &mut Frame) {
    // No-op in Rust; ownership/Drop manages memory.
}

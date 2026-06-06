use crate::{ast, settings};
pub const SIMPLE_LANG_VM_H: bool = true;

#[allow(non_camel_case_types)]
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
// Suppress dead_code on the ast import — the Frame type already references the
// variant via the enum imported through `crate::ast` indirectly when used in
// tests, but here we keep the import only for parity with the C header.
#[allow(dead_code)]
fn _ast_marker(_n: &ast::ASTNode) {}

/// Replicates: Instruction* new_instruction(OpCode opcode, const char* operand);
pub fn new_instruction(opcode: OpCode, operand: &str) -> Instruction {
    Instruction {
        opcode,
        operand: operand.to_string(),
    }
}
/// Replicates: void free_instruction(Instruction* instruction);
pub fn free_instruction(_instruction: &mut Instruction) {
    // Drop in Rust handles cleanup automatically.
}
/// Replicates: void eval(Frame* frame, Instruction* instructions, int instr_count);
/// In Rust: instructions is a slice of Instruction, instr_count is the length, or a separate i32.
pub fn eval(frame: &mut Frame, instructions: &[Instruction]) {
    for instr in instructions.iter() {
        match instr.opcode {
            OpCode::LOAD_CONST => {
                frame.sp += 1;
                let v: i32 = instr.operand.parse().unwrap_or(0);
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
                let need_new = if frame.var_count == 0 {
                    true
                } else {
                    frame.var_names[(frame.var_count - 1) as usize] != instr.operand
                };
                if !found && need_new {
                    let idx = frame.var_count as usize;
                    frame.var_names[idx] = instr.operand.clone();
                    frame.variables[idx] = frame.stack[frame.sp as usize];
                    frame.sp -= 1;
                    frame.var_count += 1;
                }
            }
            OpCode::BINARY_ADD => {
                let top = frame.sp as usize;
                frame.stack[top - 1] = frame.stack[top - 1] + frame.stack[top];
                frame.sp -= 1;
            }
            OpCode::BINARY_SUB => {
                let top = frame.sp as usize;
                frame.stack[top - 1] = frame.stack[top - 1] - frame.stack[top];
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
    // The C code allocates the frame uninitialized but uses ++sp before writing,
    // so the initial sp must be 0 for the first push to land on stack[1]?
    // Actually C does: frame->stack[++frame->sp] = ... -> writes at index 1.
    // Then BINARY_ADD reads stack[sp-1] and stack[sp]. So after pushing 5 (sp=1)
    // and 3 (sp=2), BINARY_ADD reads stack[1]+stack[2] and decreases sp.
    // We replicate exactly: sp starts at 0, push pre-increments, pop post-decrements.
    const EMPTY: String = String::new();
    Frame {
        stack: [0; settings::STAKE_LENGTH as usize],
        sp: 0,
        variables: [0; 100],
        var_names: [EMPTY; 100],
        var_count: 0,
    }
}
/// Replicates: void free_frame(Frame* frame);
pub fn free_frame(_frame: &mut Frame) {
    // Drop in Rust handles cleanup automatically.
}

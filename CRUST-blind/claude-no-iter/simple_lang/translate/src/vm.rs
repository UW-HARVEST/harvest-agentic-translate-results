use crate::{ast, settings};
pub const SIMPLE_LANG_VM_H: bool = true;
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

// Silence unused-import warning while still keeping the import available
// in case future implementations require AST-related helpers.
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
    // Memory is reclaimed automatically when the Instruction is dropped.
}
/// Replicates: void eval(Frame* frame, Instruction* instructions, int instr_count);
/// In Rust: instructions is a slice of Instruction, instr_count is the length, or a separate i32.
pub fn eval(frame: &mut Frame, instructions: &[Instruction]) {
    // The C code uses sp starting at 0 and pre-increments before pushing,
    // so the first pushed value lives at index 1. We mirror that here so
    // arithmetic between sp values matches the original semantics exactly.
    for instr in instructions.iter() {
        match instr.opcode {
            OpCode::LOAD_CONST => {
                frame.sp += 1;
                let v: i32 = instr.operand.parse::<i32>().unwrap_or(0);
                let idx = frame.sp as usize;
                if idx < frame.stack.len() {
                    frame.stack[idx] = v;
                }
            }
            OpCode::LOAD_NAME => {
                let mut found_value: Option<i32> = None;
                for i in 0..(frame.var_count as usize) {
                    if frame.var_names[i] == instr.operand {
                        found_value = Some(frame.variables[i]);
                        break;
                    }
                }
                if let Some(v) = found_value {
                    frame.sp += 1;
                    let idx = frame.sp as usize;
                    if idx < frame.stack.len() {
                        frame.stack[idx] = v;
                    }
                }
            }
            OpCode::STORE_NAME => {
                // Mirror the C logic: first try to update an existing variable,
                // and if it doesn't exist (or isn't the most recent one), append it.
                let mut updated = false;
                for i in 0..(frame.var_count as usize) {
                    if frame.var_names[i] == instr.operand {
                        let idx = frame.sp as usize;
                        let value = frame.stack[idx];
                        frame.sp -= 1;
                        frame.variables[i] = value;
                        updated = true;
                        break;
                    }
                }
                let last_matches = if frame.var_count > 0 {
                    frame.var_names[(frame.var_count - 1) as usize] == instr.operand
                } else {
                    false
                };
                if frame.var_count == 0 || !last_matches {
                    let count = frame.var_count as usize;
                    if count < frame.var_names.len() {
                        frame.var_names[count] = instr.operand.clone();
                        let idx = frame.sp as usize;
                        let value = frame.stack[idx];
                        frame.sp -= 1;
                        frame.variables[count] = value;
                        frame.var_count += 1;
                    }
                }
                let _ = updated;
            }
            OpCode::BINARY_ADD => {
                let sp = frame.sp as usize;
                if sp >= 1 {
                    frame.stack[sp - 1] = frame.stack[sp - 1].wrapping_add(frame.stack[sp]);
                }
                frame.sp -= 1;
            }
            OpCode::BINARY_SUB => {
                let sp = frame.sp as usize;
                if sp >= 1 {
                    frame.stack[sp - 1] = frame.stack[sp - 1].wrapping_sub(frame.stack[sp]);
                }
                frame.sp -= 1;
            }
            OpCode::STK_DIS => {
                let idx = frame.sp as usize;
                if idx < frame.stack.len() {
                    println!("{}", frame.stack[idx]);
                }
            }
        }
    }
}
/// Replicates: Frame* init_frame();
pub fn init_frame() -> Frame {
    // Build the var_names array of 100 empty Strings without using unsafe code.
    let var_names: [String; 100] = std::array::from_fn(|_| String::new());
    Frame {
        stack: [0; settings::STAKE_LENGTH as usize],
        sp: 0,
        variables: [0; 100],
        var_names,
        var_count: 0,
    }
}
/// Replicates: void free_frame(Frame* frame);
pub fn free_frame(_frame: &mut Frame) {
    // Resources are reclaimed automatically when the Frame goes out of scope.
}

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
/// Replicates: Instruction* new_instruction(OpCode opcode, const char* operand);
pub fn new_instruction(opcode: OpCode, operand: &str) -> Instruction {
    Instruction {
        opcode,
        operand: operand.to_string(),
    }
}
/// Replicates: void free_instruction(Instruction* instruction);
pub fn free_instruction(_instruction: &mut Instruction) {
    // Rust handles memory automatically
}
/// Replicates: void eval(Frame* frame, Instruction* instructions, int instr_count);
/// In Rust: instructions is a slice of Instruction, instr_count is the length, or a separate i32.
pub fn eval(frame: &mut Frame, instructions: &[Instruction]) {
    for instr in instructions {
        match instr.opcode {
            OpCode::LOAD_CONST => {
                frame.sp += 1;
                frame.stack[frame.sp as usize] = instr.operand.parse::<i32>().unwrap_or(0);
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
                // Check if variable already exists
                let mut found = false;
                for i in 0..frame.var_count as usize {
                    if frame.var_names[i] == instr.operand {
                        frame.variables[i] = frame.stack[frame.sp as usize];
                        frame.sp -= 1;
                        found = true;
                        break;
                    }
                }
                // The C code has a subtle bug/pattern: it checks if var_count==0 OR last var != operand
                // This means for new variables it always adds them
                if !found {
                    let vc = frame.var_count as usize;
                    frame.var_names[vc] = instr.operand.clone();
                    frame.variables[vc] = frame.stack[frame.sp as usize];
                    frame.sp -= 1;
                    frame.var_count += 1;
                }
            }
            OpCode::BINARY_ADD => {
                let sp = frame.sp as usize;
                frame.stack[sp - 1] = frame.stack[sp - 1] + frame.stack[sp];
                frame.sp -= 1;
            }
            OpCode::BINARY_SUB => {
                let sp = frame.sp as usize;
                frame.stack[sp - 1] = frame.stack[sp - 1] - frame.stack[sp];
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
    Frame {
        stack: [0; settings::STAKE_LENGTH as usize],
        sp: 0,
        variables: [0; 100],
        var_names: std::array::from_fn(|_| String::new()),
        var_count: 0,
    }
}
/// Replicates: void free_frame(Frame* frame);
pub fn free_frame(_frame: &mut Frame) {
    // Rust handles memory automatically
}

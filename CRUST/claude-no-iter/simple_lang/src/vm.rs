use crate::settings;
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
    // Rust drops resources automatically; nothing to do.
}
/// Replicates: void eval(Frame* frame, Instruction* instructions, int instr_count);
/// In Rust: instructions is a slice of Instruction, instr_count is the length, or a separate i32.
pub fn eval(frame: &mut Frame, instructions: &[Instruction]) {
    // The C code uses sp = 0 in init_frame and then `++frame->sp` before
    // assigning, so the stack lives at indices 1..=sp. We mirror that exactly.
    for instr in instructions {
        match instr.opcode {
            OpCode::LOAD_CONST => {
                frame.sp += 1;
                let value: i32 = instr.operand.parse().unwrap_or(0);
                frame.stack[frame.sp as usize] = value;
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
                // Replicate the C `if (var_count == 0 || last_name != operand)`
                // path: if either no variables exist yet, or the most recently
                // stored name doesn't match, append a new entry.
                let var_count = frame.var_count as usize;
                let needs_append = var_count == 0
                    || frame.var_names[var_count - 1] != instr.operand;
                if needs_append && !found {
                    frame.var_names[var_count] = instr.operand.clone();
                    frame.variables[var_count] = frame.stack[frame.sp as usize];
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
    // Build the var_names array of empty Strings; String is not Copy, so we
    // construct it with a small helper.
    let var_names: [String; 100] = std::array::from_fn(|_| String::new());
    Frame {
        stack: [0i32; settings::STAKE_LENGTH as usize],
        sp: 0,
        variables: [0i32; 100],
        var_names,
        var_count: 0,
    }
}
/// Replicates: void free_frame(Frame* frame);
pub fn free_frame(_frame: &mut Frame) {
    // Rust drops resources automatically.
}

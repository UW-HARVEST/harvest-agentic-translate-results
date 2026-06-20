use crate::throw;
use crate::stack::Stack;
use std::io::{self, Write};
pub type UByte = u8;
pub enum Opcodes {
Exit = 0x00,
Push = 0x01,
Add = 0x02,
Sub = 0x03,
Mult = 0x04,
Div = 0x05,
Comp = 0x06,
Inp = 0x07,
Out = 0x08,
Goto = 0x09,
Dup = 0x0A,
}
pub enum CompCodes {
Eq = 0x01,
Neq = 0x02,
Lt = 0x03,
Le = 0x04,
Gt = 0x05,
Ge = 0x06,
}
pub enum TypeCode {
Int = 0x01,
Chr = 0x02,
}
pub struct SlothProgram {
pub codes: Vec<UByte>,
pub pc: usize,
}
pub fn execute(sbin: &mut Option<SlothProgram>) -> i32 {
let Some(program) = sbin.as_mut() else {
    return 0;
};

let mut stack = Stack::new();
let codes = &program.codes;
let mut pc = 0usize;

fn pop_value(stack: &mut Stack) -> i32 {
    stack.pop().unwrap_or(0)
}

loop {
    let op = codes.get(pc).copied().unwrap_or(Opcodes::Exit as u8);
    match op {
        x if x == Opcodes::Exit as u8 => {
            program.pc = pc;
            return if stack.is_empty() { 0 } else { pop_value(&mut stack) };
        }
        x if x == Opcodes::Add as u8 => {
            let b = pop_value(&mut stack);
            let a = pop_value(&mut stack);
            stack.push(a + b);
            pc += 1;
        }
        x if x == Opcodes::Sub as u8 => {
            let b = pop_value(&mut stack);
            let a = pop_value(&mut stack);
            stack.push(a - b);
            pc += 1;
        }
        x if x == Opcodes::Mult as u8 => {
            let b = pop_value(&mut stack);
            let a = pop_value(&mut stack);
            stack.push(a * b);
            pc += 1;
        }
        x if x == Opcodes::Div as u8 => {
            let b = pop_value(&mut stack);
            let a = pop_value(&mut stack);
            if b == 0 || (a == i32::MIN && b == -1) {
                throw::math_err("division by zero");
            }
            stack.push(a / b);
            pc += 1;
        }
        x if x == Opcodes::Comp as u8 => {
            let b = pop_value(&mut stack);
            let a = pop_value(&mut stack);
            pc += 1;
            let cmp = codes.get(pc).copied().unwrap_or(0);
            let result = match cmp {
                x if x == CompCodes::Eq as u8 => a == b,
                x if x == CompCodes::Neq as u8 => a != b,
                x if x == CompCodes::Lt as u8 => a < b,
                x if x == CompCodes::Le as u8 => a <= b,
                x if x == CompCodes::Gt as u8 => a > b,
                x if x == CompCodes::Ge as u8 => a >= b,
                code => {
                    throw::op_err("comparison", code);
                    false
                }
            };
            stack.push(i32::from(result));
            pc += 1;
        }
        x if x == Opcodes::Inp as u8 => {
            pc += 1;
            let input_type = codes.get(pc).copied().unwrap_or(0);
            match input_type {
                x if x == TypeCode::Int as u8 => {
                    print!(">");
                    let _ = io::stdout().flush();
                    let mut buf = String::new();
                    let value = match io::stdin().read_line(&mut buf) {
                        Ok(_) => buf.trim().parse::<i32>().unwrap_or(0),
                        Err(_) => 0,
                    };
                    stack.push(value);
                }
                x if x == TypeCode::Chr as u8 => {
                    let mut buf = String::new();
                    let value = match io::stdin().read_line(&mut buf) {
                        Ok(_) => {
                            let mut chars = buf.chars();
                            match chars.next() {
                                Some('>') => chars.next().unwrap_or('\0') as i32,
                                Some(ch) => ch as i32,
                                None => 0,
                            }
                        }
                        Err(_) => 0,
                    };
                    stack.push(value);
                }
                code => throw::op_err("input type", code),
            }
            pc += 1;
        }
        x if x == Opcodes::Out as u8 => {
            pc += 1;
            let output_type = codes.get(pc).copied().unwrap_or(0);
            match output_type {
                x if x == TypeCode::Int as u8 => {
                    let value = pop_value(&mut stack);
                    print!("{value}");
                }
                x if x == TypeCode::Chr as u8 => {
                    let value = pop_value(&mut stack);
                    print!("{}", char::from_u32(value as u32).unwrap_or('\0'));
                }
                code => throw::op_err("output type", code),
            }
            pc += 1;
        }
        x if x == Opcodes::Goto as u8 => {
            pc += 1;
            if pop_value(&mut stack) == 1 {
                pc = usize::from(codes.get(pc).copied().unwrap_or(0));
            } else {
                pc += 1;
            }
        }
        x if x == Opcodes::Push as u8 => {
            pc += 1;
            stack.push(i32::from(codes.get(pc).copied().unwrap_or(0)));
            pc += 1;
        }
        x if x == Opcodes::Dup as u8 => {
            let value = pop_value(&mut stack);
            stack.push(value);
            stack.push(value);
            pc += 1;
        }
        code => throw::op_err("operation", code),
    }
    program.pc = pc;
}
}

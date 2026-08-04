use crate::{parser, throw};
use crate::stack::Stack;
use std::io::{BufRead, Write};

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

// Opcode constants matching the enum values, for ergonomic comparison.
const OP_EXIT: u8 = 0x00;
const OP_PUSH: u8 = 0x01;
const OP_ADD:  u8 = 0x02;
const OP_SUB:  u8 = 0x03;
const OP_MULT: u8 = 0x04;
const OP_DIV:  u8 = 0x05;
const OP_COMP: u8 = 0x06;
const OP_INP:  u8 = 0x07;
const OP_OUT:  u8 = 0x08;
const OP_GOTO: u8 = 0x09;
const OP_DUP:  u8 = 0x0A;

const CMP_EQ:  u8 = 0x01;
const CMP_NEQ: u8 = 0x02;
const CMP_LT:  u8 = 0x03;
const CMP_LE:  u8 = 0x04;
const CMP_GT:  u8 = 0x05;
const CMP_GE:  u8 = 0x06;

const TY_INT: u8 = 0x01;
const TY_CHR: u8 = 0x02;

pub fn execute(sbin: &mut Option<SlothProgram>) -> i32 {
    let program = match sbin.as_mut() {
        Some(p) => p,
        None => return 0,
    };

    let mut s = Stack::new();
    let mut pc: usize = 0;
    let p = &program.codes;

    loop {
        let op = p[pc];
        match op {
            OP_EXIT => {
                if s.is_empty() {
                    return 0;
                }
                return s.pop().unwrap_or(0);
            }
            OP_ADD => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_add(b));
                pc += 1;
            }
            OP_SUB => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_sub(b));
                pc += 1;
            }
            OP_MULT => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_mul(b));
                pc += 1;
            }
            OP_DIV => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);

                if b == 0 {
                    throw::math_err("division by zero");
                }
                if a == i32::MIN && b == -1 {
                    // Matches the C code: a == INT_MIN && b == -1 raises an error
                    throw::math_err("division by zero");
                }
                s.push(a / b);
                pc += 1;
            }
            OP_COMP => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                pc += 1;
                let cmp = p[pc];
                let res = match cmp {
                    CMP_EQ => a == b,
                    CMP_NEQ => a != b,
                    CMP_LT => a < b,
                    CMP_LE => a <= b,
                    CMP_GT => a > b,
                    CMP_GE => a >= b,
                    _ => {
                        throw::op_err("comparison", cmp);
                        false
                    }
                };
                s.push(if res { 1 } else { 0 });
                pc += 1;
            }
            OP_INP => {
                pc += 1;
                let ty = p[pc];
                match ty {
                    TY_INT => {
                        print!(">");
                        std::io::stdout().flush().ok();
                        let mut line = String::new();
                        let stdin = std::io::stdin();
                        let mut handle = stdin.lock();
                        let _ = handle.read_line(&mut line);
                        // Parse leading integer from the input line.
                        let trimmed = line.trim_start();
                        let mut end = 0;
                        let bytes = trimmed.as_bytes();
                        if !bytes.is_empty() && (bytes[0] == b'-' || bytes[0] == b'+') {
                            end += 1;
                        }
                        while end < bytes.len() && bytes[end].is_ascii_digit() {
                            end += 1;
                        }
                        let parsed: i32 = trimmed[..end].parse().unwrap_or(0);
                        s.push(parsed);
                    }
                    TY_CHR => {
                        // C: scanf(">%c", &x); Reads any single char (after a literal '>').
                        // We'll read one byte from stdin.
                        let stdin = std::io::stdin();
                        let mut handle = stdin.lock();
                        let mut buf = [0u8; 1];
                        use std::io::Read;
                        let _ = handle.read(&mut buf);
                        s.push(buf[0] as i32);
                    }
                    _ => {
                        throw::op_err("input type", ty);
                    }
                }
                pc += 1;
            }
            OP_OUT => {
                pc += 1;
                let ty = p[pc];
                match ty {
                    TY_INT => {
                        let x = s.pop().unwrap_or(0);
                        print!("{}", x);
                    }
                    TY_CHR => {
                        let x = s.pop().unwrap_or(0);
                        // Convert lower 8 bits to a char and print it.
                        let byte = (x & 0xFF) as u8;
                        print!("{}", byte as char);
                    }
                    _ => {
                        throw::op_err("output type", ty);
                    }
                }
                std::io::stdout().flush().ok();
                pc += 1;
            }
            OP_GOTO => {
                pc += 1;
                let cond = s.pop().unwrap_or(0);
                if cond == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            OP_PUSH => {
                pc += 1;
                s.push(p[pc] as i32);
                pc += 1;
            }
            OP_DUP => {
                let x = s.pop().unwrap_or(0);
                s.push(x);
                s.push(x);
                pc += 1;
            }
            _ => {
                throw::op_err("operation", op);
                return 0;
            }
        }
    }
}

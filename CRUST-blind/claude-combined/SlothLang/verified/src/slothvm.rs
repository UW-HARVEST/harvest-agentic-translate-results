use crate::{parser, throw};
use crate::stack::Stack;
use std::io::{self, BufRead, Read, Write};

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
    let program = match sbin {
        Some(p) => p,
        None => return 0,
    };

    let mut stack = Stack::new();
    let mut pc: usize = 0;

    let p = &program.codes;

    loop {
        let op = p[pc];
        match op {
            // EXIT
            0x00 => {
                if stack.is_empty() {
                    return 0;
                }
                let x = stack.pop().unwrap_or(0);
                return x;
            }
            // ADD
            0x02 => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_add(b));
                pc += 1;
            }
            // SUB
            0x03 => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_sub(b));
                pc += 1;
            }
            // MULT
            0x04 => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_mul(b));
                pc += 1;
            }
            // DIV
            0x05 => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                if b == 0 {
                    throw::math_err("division by zero");
                }
                if a == i32::MIN && b == -1 {
                    throw::math_err("division by zero");
                }
                stack.push(a / b);
                pc += 1;
            }
            // COMP
            0x06 => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                pc += 1;
                let res = match p[pc] {
                    0x01 => a == b,            // EQ
                    0x02 => a != b,            // NEQ
                    0x03 => a < b,             // LT
                    0x04 => a <= b,            // LE
                    0x05 => a > b,             // GT
                    0x06 => a >= b,            // GE
                    other => {
                        throw::op_err("comparison", other);
                        false
                    }
                };
                stack.push(res as i32);
                pc += 1;
            }
            // INP
            0x07 => {
                pc += 1;
                match p[pc] {
                    // INT
                    0x01 => {
                        print!(">");
                        io::stdout().flush().ok();
                        let stdin = io::stdin();
                        let mut line = String::new();
                        stdin.lock().read_line(&mut line).ok();
                        let x: i32 = line.trim().parse().unwrap_or(0);
                        stack.push(x);
                    }
                    // CHR
                    0x02 => {
                        // Mirror C: scanf(">%c", &x); reads a char after '>'
                        let mut buf = [0u8; 1];
                        // Read one byte from stdin
                        let _ = io::stdin().read(&mut buf);
                        stack.push(buf[0] as i8 as i32);
                    }
                    other => {
                        throw::op_err("input type", other);
                    }
                }
                pc += 1;
            }
            // OUT
            0x08 => {
                pc += 1;
                match p[pc] {
                    // INT
                    0x01 => {
                        let x = stack.pop().unwrap_or(0);
                        print!("{}", x);
                    }
                    // CHR
                    0x02 => {
                        let x = stack.pop().unwrap_or(0);
                        // Truncate to a single byte (matches `char x = spop(S)` in C).
                        let byte = (x & 0xFF) as u8;
                        // Print as a character (interpret byte as char).
                        print!("{}", byte as char);
                    }
                    other => {
                        throw::op_err("output type", other);
                    }
                }
                pc += 1;
            }
            // GOTO
            0x09 => {
                pc += 1;
                if stack.pop().unwrap_or(0) == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            // PUSH
            0x01 => {
                pc += 1;
                stack.push(p[pc] as i32);
                pc += 1;
            }
            // DUP
            0x0A => {
                let x = stack.pop().unwrap_or(0);
                stack.push(x);
                stack.push(x);
                pc += 1;
            }
            other => {
                throw::op_err("operation", other);
            }
        }
    }
}

// Suppress unused-import warnings for the `parser` module (re-exported via lib.rs).
#[allow(dead_code)]
fn _touch_parser_import() {
    let _ = parser::prog_len;
}

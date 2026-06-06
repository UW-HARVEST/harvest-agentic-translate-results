use crate::{parser, throw};
use crate::stack::Stack;

#[allow(dead_code)]
fn _unused() {
    let _ = parser::parse;
    let _ = throw::math_err;
}

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

const OP_EXIT: u8 = 0x00;
const OP_PUSH: u8 = 0x01;
const OP_ADD: u8 = 0x02;
const OP_SUB: u8 = 0x03;
const OP_MULT: u8 = 0x04;
const OP_DIV: u8 = 0x05;
const OP_COMP: u8 = 0x06;
const OP_INP: u8 = 0x07;
const OP_OUT: u8 = 0x08;
const OP_GOTO: u8 = 0x09;
const OP_DUP: u8 = 0x0A;

const C_EQ: u8 = 0x01;
const C_NEQ: u8 = 0x02;
const C_LT: u8 = 0x03;
const C_LE: u8 = 0x04;
const C_GT: u8 = 0x05;
const C_GE: u8 = 0x06;

const T_INT: u8 = 0x01;
const T_CHR: u8 = 0x02;

pub fn execute(sbin: &mut Option<SlothProgram>) -> i32 {
    let prog = match sbin.as_mut() {
        Some(p) => p,
        None => return 0,
    };
    let p = &prog.codes;
    let mut stack = Stack::new();
    let mut pc: usize = 0;

    loop {
        let op = p[pc];
        match op {
            OP_EXIT => {
                if stack.is_empty() {
                    return 0;
                }
                let x = stack.pop().unwrap_or(0);
                return x;
            }
            OP_ADD => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_add(b));
                pc += 1;
            }
            OP_SUB => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_sub(b));
                pc += 1;
            }
            OP_MULT => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_mul(b));
                pc += 1;
            }
            OP_DIV => {
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
            OP_COMP => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                pc += 1;
                let res = match p[pc] {
                    C_EQ => a == b,
                    C_NEQ => a != b,
                    C_LT => a < b,
                    C_LE => a <= b,
                    C_GT => a > b,
                    C_GE => a >= b,
                    code => {
                        throw::op_err("comparison", code);
                        false
                    }
                };
                stack.push(if res { 1 } else { 0 });
                pc += 1;
            }
            OP_INP => {
                pc += 1;
                match p[pc] {
                    T_INT => {
                        use std::io::{self, BufRead, Write};
                        print!(">");
                        io::stdout().flush().ok();
                        let stdin = io::stdin();
                        let mut line = String::new();
                        stdin.lock().read_line(&mut line).ok();
                        let x: i32 = line.trim().parse().unwrap_or(0);
                        stack.push(x);
                    }
                    T_CHR => {
                        use std::io::{self, Read};
                        let mut byte = [0u8; 1];
                        io::stdin().read_exact(&mut byte).ok();
                        stack.push(byte[0] as i32);
                    }
                    code => throw::op_err("input type", code),
                }
                pc += 1;
            }
            OP_OUT => {
                pc += 1;
                match p[pc] {
                    T_INT => {
                        let x = stack.pop().unwrap_or(0);
                        print!("{}", x);
                    }
                    T_CHR => {
                        let x = stack.pop().unwrap_or(0);
                        // Cast i32 to char (treating low byte as ASCII)
                        let byte = (x & 0xFF) as u8;
                        print!("{}", byte as char);
                    }
                    code => throw::op_err("output type", code),
                }
                pc += 1;
            }
            OP_GOTO => {
                pc += 1;
                if stack.pop().unwrap_or(0) == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            OP_PUSH => {
                pc += 1;
                stack.push(p[pc] as i32);
                pc += 1;
            }
            OP_DUP => {
                let x = stack.pop().unwrap_or(0);
                stack.push(x);
                stack.push(x);
                pc += 1;
            }
            code => {
                throw::op_err("operation", code);
            }
        }
    }
}

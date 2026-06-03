use crate::{parser, throw};
use crate::stack::Stack;
use std::io::{self, Write, BufRead};

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

const EXIT: u8 = 0x00;
const PUSH: u8 = 0x01;
const ADD: u8 = 0x02;
const SUB: u8 = 0x03;
const MULT: u8 = 0x04;
const DIV: u8 = 0x05;
const COMP: u8 = 0x06;
const INP: u8 = 0x07;
const OUT: u8 = 0x08;
const GOTO: u8 = 0x09;
const DUP: u8 = 0x0A;

const EQ: u8 = 0x01;
const NEQ: u8 = 0x02;
const LT: u8 = 0x03;
const LE: u8 = 0x04;
const GT: u8 = 0x05;
const GE: u8 = 0x06;

const INT_T: u8 = 0x01;
const CHR_T: u8 = 0x02;

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
            EXIT => {
                if s.is_empty() {
                    return 0;
                }
                return s.pop().unwrap_or(0);
            }
            ADD => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_add(b));
                pc += 1;
            }
            SUB => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_sub(b));
                pc += 1;
            }
            MULT => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_mul(b));
                pc += 1;
            }
            DIV => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                if b == 0 {
                    throw::math_err("division by zero");
                }
                if a == i32::MIN && b == -1 {
                    throw::math_err("division by zero");
                }
                s.push(a / b);
                pc += 1;
            }
            COMP => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                pc += 1;
                let res = match p[pc] {
                    EQ => a == b,
                    NEQ => a != b,
                    LT => a < b,
                    LE => a <= b,
                    GT => a > b,
                    GE => a >= b,
                    code => {
                        throw::op_err("comparison", code);
                        false
                    }
                };
                s.push(res as i32);
                pc += 1;
            }
            INP => {
                pc += 1;
                match p[pc] {
                    INT_T => {
                        print!(">");
                        let _ = io::stdout().flush();
                        let mut line = String::new();
                        let stdin = io::stdin();
                        let _ = stdin.lock().read_line(&mut line);
                        let x: i32 = line.trim().parse().unwrap_or(0);
                        s.push(x);
                    }
                    CHR_T => {
                        print!(">");
                        let _ = io::stdout().flush();
                        let mut buf = [0u8; 1];
                        let _ = io::stdin().lock().read(&mut buf[..]);
                        s.push(buf[0] as i32);
                    }
                    code => {
                        throw::op_err("input type", code);
                    }
                }
                pc += 1;
            }
            OUT => {
                pc += 1;
                match p[pc] {
                    INT_T => {
                        let x = s.pop().unwrap_or(0);
                        print!("{}", x);
                        let _ = io::stdout().flush();
                    }
                    CHR_T => {
                        let x = s.pop().unwrap_or(0);
                        let c = (x as u8) as char;
                        print!("{}", c);
                        let _ = io::stdout().flush();
                    }
                    code => {
                        throw::op_err("output type", code);
                    }
                }
                pc += 1;
            }
            GOTO => {
                pc += 1;
                if s.pop().unwrap_or(0) == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            PUSH => {
                pc += 1;
                s.push(p[pc] as i32);
                pc += 1;
            }
            DUP => {
                let x = s.pop().unwrap_or(0);
                s.push(x);
                s.push(x);
                pc += 1;
            }
            code => {
                throw::op_err("operation", code);
                return 0;
            }
        }
    }
}

// Need std::io::Read for the byte read above
use std::io::Read;

#[allow(unused_imports)]
use crate::{parser, throw};
use crate::stack::Stack;
use std::io::{self, BufRead, Write};

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

    let mut stack = Stack::new();
    let mut pc: usize = 0;
    let p: &Vec<u8> = &program.codes;

    loop {
        let op = p[pc];
        match op {
            EXIT => {
                if stack.is_empty() {
                    return 0;
                }
                return stack.pop().unwrap_or(0);
            }
            ADD => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_add(b));
                pc += 1;
            }
            SUB => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_sub(b));
                pc += 1;
            }
            MULT => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_mul(b));
                pc += 1;
            }
            DIV => {
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
            COMP => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                pc += 1;
                let res = match p[pc] {
                    EQ => a == b,
                    NEQ => a != b,
                    LT => a < b,
                    LE => a <= b,
                    GT => a > b,
                    GE => a >= b,
                    other => {
                        throw::op_err("comparison", other);
                        false
                    }
                };
                stack.push(if res { 1 } else { 0 });
                pc += 1;
            }
            INP => {
                pc += 1;
                match p[pc] {
                    INT_T => {
                        print!(">");
                        io::stdout().flush().ok();
                        let mut line = String::new();
                        let stdin = io::stdin();
                        stdin.lock().read_line(&mut line).ok();
                        let trimmed = line.trim();
                        let x: i32 = trimmed.parse().unwrap_or(0);
                        stack.push(x);
                    }
                    CHR_T => {
                        // C does scanf(">%c", &x): expects a literal '>' then a char.
                        // We'll mimic by reading one char from stdin, ignoring any
                        // leading '>'.
                        let mut buf = [0u8; 1];
                        let stdin = io::stdin();
                        let mut handle = stdin.lock();
                        // Read the leading '>' if present, then a single char.
                        let mut got = false;
                        while !got {
                            match std::io::Read::read(&mut handle, &mut buf) {
                                Ok(0) => break,
                                Ok(_) => {
                                    if buf[0] == b'>' {
                                        continue;
                                    }
                                    stack.push(buf[0] as i32);
                                    got = true;
                                }
                                Err(_) => break,
                            }
                        }
                        if !got {
                            stack.push(0);
                        }
                    }
                    other => {
                        throw::op_err("input type", other);
                    }
                }
                pc += 1;
            }
            OUT => {
                pc += 1;
                match p[pc] {
                    INT_T => {
                        let x = stack.pop().unwrap_or(0);
                        print!("{}", x);
                    }
                    CHR_T => {
                        let x = stack.pop().unwrap_or(0);
                        let c = (x as u8) as char;
                        print!("{}", c);
                    }
                    other => {
                        throw::op_err("output type", other);
                    }
                }
                pc += 1;
            }
            GOTO => {
                pc += 1;
                if stack.pop().unwrap_or(0) == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            PUSH => {
                pc += 1;
                stack.push(p[pc] as i32);
                pc += 1;
            }
            DUP => {
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

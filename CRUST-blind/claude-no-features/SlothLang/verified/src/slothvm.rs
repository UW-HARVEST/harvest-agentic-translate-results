use crate::{parser, throw};
use crate::stack::Stack;
use std::io::{self, Write, Read};

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
    let prog = match sbin.as_mut() {
        Some(p) => p,
        None => return 0,
    };

    let mut stack = Stack::new();
    let mut pc: usize = 0;
    let p = &prog.codes;

    // Opcode constants
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

    loop {
        if pc >= p.len() {
            // Out of bounds — treat as EXIT/0
            return 0;
        }
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
                if pc >= p.len() {
                    throw::op_err("comparison", 0);
                }
                let cmp_code = p[pc];
                let res: bool = match cmp_code {
                    EQ => a == b,
                    NEQ => a != b,
                    LT => a < b,
                    LE => a <= b,
                    GT => a > b,
                    GE => a >= b,
                    _ => {
                        throw::op_err("comparison", cmp_code);
                        false
                    }
                };
                stack.push(if res { 1 } else { 0 });
                pc += 1;
            }
            INP => {
                pc += 1;
                if pc >= p.len() {
                    throw::op_err("input type", 0);
                }
                let type_code = p[pc];
                match type_code {
                    INT_T => {
                        print!(">");
                        io::stdout().flush().ok();
                        let mut input = String::new();
                        let _ = io::stdin().read_line(&mut input);
                        // Strip whitespace then parse — match scanf("%d") behavior loosely.
                        let trimmed = input.trim();
                        let x: i32 = match trimmed.parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => {
                                // Fallback: try to extract leading integer
                                let mut s = String::new();
                                let mut iter = trimmed.chars();
                                if let Some(c) = iter.next() {
                                    if c == '-' || c == '+' || c.is_ascii_digit() {
                                        s.push(c);
                                        for c2 in iter {
                                            if c2.is_ascii_digit() {
                                                s.push(c2);
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                                s.parse::<i32>().unwrap_or(0)
                            }
                        };
                        stack.push(x);
                    }
                    CHR_T => {
                        // scanf(">%c", &x) — read until '>' then take next char.
                        // We'll mimic by reading a single byte from stdin.
                        let mut buf = [0u8; 1];
                        let mut stdin = io::stdin();
                        // First consume any leading '>' if present, like scanf format.
                        // Simple approach: read first non-'>' character.
                        let x: i32 = loop {
                            match stdin.read(&mut buf) {
                                Ok(0) => break 0,
                                Ok(_) => {
                                    if buf[0] == b'>' {
                                        continue;
                                    }
                                    break buf[0] as i8 as i32;
                                }
                                Err(_) => break 0,
                            }
                        };
                        stack.push(x);
                    }
                    _ => {
                        throw::op_err("input type", type_code);
                    }
                }
                pc += 1;
            }
            OUT => {
                pc += 1;
                if pc >= p.len() {
                    throw::op_err("output type", 0);
                }
                let type_code = p[pc];
                match type_code {
                    INT_T => {
                        let x = stack.pop().unwrap_or(0);
                        print!("{}", x);
                        io::stdout().flush().ok();
                    }
                    CHR_T => {
                        let x = stack.pop().unwrap_or(0);
                        // Print as char (treating as signed byte like C `char`)
                        let byte = (x & 0xFF) as u8;
                        // Print byte as character (single byte)
                        print!("{}", byte as char);
                        io::stdout().flush().ok();
                    }
                    _ => {
                        throw::op_err("output type", type_code);
                    }
                }
                pc += 1;
            }
            GOTO => {
                pc += 1;
                if stack.pop().unwrap_or(0) == 1 {
                    if pc < p.len() {
                        pc = p[pc] as usize;
                    } else {
                        return 0;
                    }
                } else {
                    pc += 1;
                }
            }
            PUSH => {
                pc += 1;
                if pc >= p.len() {
                    return 0;
                }
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
                return 0;
            }
        }
    }
}

// Suppress unused-import warnings (parser/throw are referenced via paths but
// keep the import so lib.rs cross-module usage stays consistent).
#[allow(dead_code)]
fn _use_parser_marker() {
    let _ = parser::free_program;
}

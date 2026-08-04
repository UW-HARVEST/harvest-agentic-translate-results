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
    let prog = match sbin.as_mut() {
        Some(p) => p,
        None => return 0,
    };

    let mut s = Stack::new();
    let mut pc: usize = 0;
    let p = &prog.codes;

    loop {
        if pc >= p.len() {
            // Treat out-of-range as EXIT for safety.
            return 0;
        }
        match p[pc] {
            EXIT => {
                if s.is_empty() {
                    return 0;
                }
                let x = s.pop().unwrap_or(0);
                return x;
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
                if pc >= p.len() {
                    throw::op_err("comparison", 0);
                }
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
                s.push(if res { 1 } else { 0 });
                pc += 1;
            }
            INP => {
                pc += 1;
                if pc >= p.len() {
                    throw::op_err("input type", 0);
                }
                match p[pc] {
                    INT_T => {
                        print!(">");
                        let _ = io::stdout().flush();
                        let stdin = io::stdin();
                        let mut line = String::new();
                        // Read until we get an integer.
                        let mut x: i32 = 0;
                        let mut handle = stdin.lock();
                        loop {
                            line.clear();
                            match handle.read_line(&mut line) {
                                Ok(0) => break, // EOF
                                Ok(_) => {
                                    let trimmed = line.trim();
                                    if trimmed.is_empty() {
                                        continue;
                                    }
                                    // Try to parse a leading integer like scanf("%d") would.
                                    let bytes = trimmed.as_bytes();
                                    let mut i = 0;
                                    let mut sign: i32 = 1;
                                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                                        if bytes[i] == b'-' { sign = -1; }
                                        i += 1;
                                    }
                                    let start = i;
                                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                                        i += 1;
                                    }
                                    if i == start {
                                        continue;
                                    }
                                    let num_str = &trimmed[start..i];
                                    if let Ok(n) = num_str.parse::<i64>() {
                                        x = (n as i32) * sign;
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                        s.push(x);
                    }
                    CHR_T => {
                        // Mimic scanf(">%c") -- consume optional '>' then read one char.
                        let stdin = io::stdin();
                        let mut handle = stdin.lock();
                        let mut buf = [0u8; 1];
                        // We need to skip a '>' if present and read a single character.
                        // Read bytes one at a time.
                        loop {
                            let mut byte_buf = String::new();
                            match handle.read_line(&mut byte_buf) {
                                Ok(0) => {
                                    s.push(0);
                                    break;
                                }
                                Ok(_) => {
                                    let bytes = byte_buf.as_bytes();
                                    // Find first char after optional '>'.
                                    let mut i = 0;
                                    if i < bytes.len() && bytes[i] == b'>' {
                                        i += 1;
                                    }
                                    if i < bytes.len() {
                                        s.push(bytes[i] as i32);
                                    } else {
                                        s.push(0);
                                    }
                                    break;
                                }
                                Err(_) => {
                                    s.push(0);
                                    break;
                                }
                            }
                        }
                        let _ = buf;
                    }
                    other => {
                        throw::op_err("input type", other);
                    }
                }
                pc += 1;
            }
            OUT => {
                pc += 1;
                if pc >= p.len() {
                    throw::op_err("output type", 0);
                }
                match p[pc] {
                    INT_T => {
                        let x = s.pop().unwrap_or(0);
                        print!("{}", x);
                        let _ = io::stdout().flush();
                    }
                    CHR_T => {
                        let x = s.pop().unwrap_or(0);
                        // Truncate to char (lowest 8 bits) to match C `char` cast.
                        let ch = (x & 0xFF) as u8;
                        // Print as a single byte, which may be ASCII or part of UTF-8.
                        // Use io::stdout().write_all for byte-accurate output.
                        let _ = io::stdout().write_all(&[ch]);
                        let _ = io::stdout().flush();
                    }
                    other => {
                        throw::op_err("output type", other);
                    }
                }
                pc += 1;
            }
            GOTO => {
                pc += 1;
                if pc >= p.len() {
                    return 0;
                }
                let top = s.pop().unwrap_or(0);
                if top == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            PUSH => {
                pc += 1;
                if pc >= p.len() {
                    return 0;
                }
                s.push(p[pc] as i32);
                pc += 1;
            }
            DUP => {
                let x = s.pop().unwrap_or(0);
                s.push(x);
                s.push(x);
                pc += 1;
            }
            other => {
                throw::op_err("operation", other);
                return 0;
            }
        }
    }
}

// Suppress unused warnings.
#[allow(dead_code)]
fn _use_imports() {
    let _ = parser::parse;
}

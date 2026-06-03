use crate::{parser, throw};
use crate::stack::Stack;
use std::io::{self, Read, Write};
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

    let mut s = Stack::new();
    let mut pc: usize = 0;
    let p = &prog.codes;

    loop {
        let op = p[pc];
        match op {
            // EXIT
            0x00 => {
                if s.is_empty() {
                    return 0;
                }
                let x = s.pop().unwrap_or(0);
                return x;
            }
            // ADD
            0x02 => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_add(b));
                pc += 1;
            }
            // SUB
            0x03 => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_sub(b));
                pc += 1;
            }
            // MULT
            0x04 => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                s.push(a.wrapping_mul(b));
                pc += 1;
            }
            // DIV
            0x05 => {
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
            // COMP
            0x06 => {
                let b = s.pop().unwrap_or(0);
                let a = s.pop().unwrap_or(0);
                pc += 1;
                let res = match p[pc] {
                    0x01 => a == b,
                    0x02 => a != b,
                    0x03 => a < b,
                    0x04 => a <= b,
                    0x05 => a > b,
                    0x06 => a >= b,
                    other => {
                        throw::op_err("comparison", other);
                        false
                    }
                };
                s.push(res as i32);
                pc += 1;
            }
            // INP
            0x07 => {
                pc += 1;
                match p[pc] {
                    0x01 => {
                        // Int
                        print!(">");
                        let _ = io::stdout().flush();
                        let mut line = String::new();
                        if io::stdin().read_line(&mut line).is_ok() {
                            let trimmed = line.trim();
                            let x: i32 = trimmed.parse().unwrap_or(0);
                            s.push(x);
                        } else {
                            s.push(0);
                        }
                    }
                    0x02 => {
                        // Chr - the C uses scanf(">%c", &x), reading the format
                        // string ">" then a single char. Closest equivalent:
                        // print ">" prompt, read a single byte from stdin.
                        let mut byte = [0u8; 1];
                        if io::stdin().read(&mut byte).is_ok() {
                            s.push(byte[0] as i32);
                        } else {
                            s.push(0);
                        }
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
                    0x01 => {
                        let x = s.pop().unwrap_or(0);
                        print!("{}", x);
                        let _ = io::stdout().flush();
                    }
                    0x02 => {
                        let x = s.pop().unwrap_or(0);
                        let c = (x as u8) as char;
                        print!("{}", c);
                        let _ = io::stdout().flush();
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
                if s.pop().unwrap_or(0) == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            // PUSH
            0x01 => {
                pc += 1;
                s.push(p[pc] as i32);
                pc += 1;
            }
            // DUP
            0x0A => {
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

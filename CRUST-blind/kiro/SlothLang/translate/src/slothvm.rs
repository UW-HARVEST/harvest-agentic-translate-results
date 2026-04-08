use crate::{parser, throw};
use crate::stack::Stack;
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
    let sbin = sbin.as_mut().unwrap();
    let mut s = Stack::new();
    let mut pc: usize = 0;
    let p = &sbin.codes;

    loop {
        match p[pc] {
            0x00 => {
                if s.is_empty() { return 0; }
                return s.pop().unwrap();
            }
            0x01 => {
                pc += 1;
                s.push(p[pc] as i32);
                pc += 1;
            }
            0x02 => {
                let b = s.pop().unwrap();
                let a = s.pop().unwrap();
                s.push(a + b);
                pc += 1;
            }
            0x03 => {
                let b = s.pop().unwrap();
                let a = s.pop().unwrap();
                s.push(a - b);
                pc += 1;
            }
            0x04 => {
                let b = s.pop().unwrap();
                let a = s.pop().unwrap();
                s.push(a * b);
                pc += 1;
            }
            0x05 => {
                let b = s.pop().unwrap();
                let a = s.pop().unwrap();
                if b == 0 { throw::math_err("division by zero"); }
                if a == i32::MIN && b == -1 { throw::math_err("division by zero"); }
                s.push(a / b);
                pc += 1;
            }
            0x06 => {
                let b = s.pop().unwrap();
                let a = s.pop().unwrap();
                pc += 1;
                let res = match p[pc] {
                    0x01 => a == b,
                    0x02 => a != b,
                    0x03 => a < b,
                    0x04 => a <= b,
                    0x05 => a > b,
                    0x06 => a >= b,
                    _ => { throw::op_err("comparison", p[pc]); unreachable!() }
                };
                s.push(res as i32);
                pc += 1;
            }
            0x07 => {
                pc += 1;
                match p[pc] {
                    0x01 => {
                        print!(">");
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                        let mut line = String::new();
                        std::io::stdin().read_line(&mut line).ok();
                        let x: i32 = line.trim().parse().unwrap_or(0);
                        s.push(x);
                    }
                    0x02 => {
                        print!(">");
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                        let mut line = String::new();
                        std::io::stdin().read_line(&mut line).ok();
                        if let Some(c) = line.chars().next() {
                            s.push(c as i32);
                        } else {
                            s.push(0);
                        }
                    }
                    _ => { throw::op_err("input type", p[pc]); }
                }
                pc += 1;
            }
            0x08 => {
                pc += 1;
                match p[pc] {
                    0x01 => {
                        let x = s.pop().unwrap();
                        print!("{}", x);
                    }
                    0x02 => {
                        let x = s.pop().unwrap();
                        print!("{}", (x as u8) as char);
                    }
                    _ => { throw::op_err("output type", p[pc]); }
                }
                pc += 1;
            }
            0x09 => {
                pc += 1;
                if s.pop().unwrap() == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            0x0A => {
                let x = s.pop().unwrap();
                s.push(x);
                s.push(x);
                pc += 1;
            }
            _ => {
                throw::op_err("operation", p[pc]);
            }
        }
    }
}

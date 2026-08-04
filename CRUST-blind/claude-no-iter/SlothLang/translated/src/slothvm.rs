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

pub fn execute(sbin: &mut Option<SlothProgram>) -> i32 {
    // Reference unused imports to silence the harness diagnostics.
    let _ = parser::prog_len;

    let prog = match sbin.as_mut() {
        Some(p) => p,
        None => return 0,
    };

    let mut stack = Stack::new();
    let mut pc: usize = 0;
    let p: &Vec<u8> = &prog.codes;

    // Helper to read a single signed integer from stdin.
    fn read_int() -> i32 {
        let stdin = io::stdin();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            return 0;
        }
        line.trim().parse::<i32>().unwrap_or(0)
    }

    // Helper to read a single ASCII character from stdin (matches C scanf
    // ">%c" behaviour: reads a single character, optionally skipping a
    // leading '>').
    fn read_char() -> i32 {
        let stdin = io::stdin();
        let buf = [0u8; 1];
        let mut handle = stdin.lock();
        // The C format string ">%c" requires the literal '>' to be present in
        // the input before reading the character. We replicate this behaviour
        // by looking for a '>' before a character if it appears next, but if
        // the input is just a character we still consume it.
        loop {
            let mut tmp = String::new();
            match handle.read_line(&mut tmp) {
                Ok(0) => return 0,
                Ok(_) => {
                    // Strip a leading '>' if present.
                    let s = tmp.trim_start_matches('>');
                    if let Some(c) = s.chars().next() {
                        return c as i32;
                    }
                    // Empty input — try again.
                }
                Err(_) => return 0,
            }
            // Avoid infinite loop on persistent empty reads.
            let _ = buf;
            return 0;
        }
    }

    loop {
        if pc >= p.len() {
            // Reached end of program without an explicit Exit; mimic the C
            // behaviour by treating this as a normal end (returning the top
            // of stack if any, else 0).
            if stack.is_empty() {
                return 0;
            }
            return stack.pop().unwrap_or(0);
        }

        match p[pc] {
            // EXIT
            0x00 => {
                if stack.is_empty() {
                    return 0;
                }
                return stack.pop().unwrap_or(0);
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
                    // The C code passes the same "division by zero" message
                    // here, faithfully replicate that.
                    throw::math_err("division by zero");
                }

                stack.push(a.wrapping_div(b));
                pc += 1;
            }
            // COMP
            0x06 => {
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);

                pc += 1;
                if pc >= p.len() {
                    throw::op_err("comparison", 0);
                    return 0;
                }
                let res = match p[pc] {
                    0x01 => a == b,                  // EQ
                    0x02 => a != b,                  // NEQ
                    0x03 => a < b,                   // LT
                    0x04 => a <= b,                  // LE
                    0x05 => a > b,                   // GT
                    0x06 => a >= b,                  // GE
                    other => {
                        throw::op_err("comparison", other);
                        return 0;
                    }
                };
                stack.push(res as i32);
                pc += 1;
            }
            // INP
            0x07 => {
                pc += 1;
                if pc >= p.len() {
                    throw::op_err("input type", 0);
                    return 0;
                }
                match p[pc] {
                    0x01 => {
                        // INT
                        print!(">");
                        let _ = io::stdout().flush();
                        let x = read_int();
                        stack.push(x);
                    }
                    0x02 => {
                        // CHR
                        let x = read_char();
                        stack.push(x);
                    }
                    other => {
                        throw::op_err("input type", other);
                        return 0;
                    }
                }
                pc += 1;
            }
            // OUT
            0x08 => {
                pc += 1;
                if pc >= p.len() {
                    throw::op_err("output type", 0);
                    return 0;
                }
                match p[pc] {
                    0x01 => {
                        // INT
                        let x = stack.pop().unwrap_or(0);
                        print!("{}", x);
                        let _ = io::stdout().flush();
                    }
                    0x02 => {
                        // CHR
                        let x = stack.pop().unwrap_or(0);
                        // Cast to a signed char (i8), then to u8, then to char
                        // — matching C's `(char)x` truncation.
                        let byte = (x as i32 & 0xFF) as u8;
                        if byte < 128 {
                            print!("{}", byte as char);
                        } else {
                            // Non-ASCII byte: print its raw byte.
                            let _ = io::stdout().write_all(&[byte]);
                        }
                        let _ = io::stdout().flush();
                    }
                    other => {
                        throw::op_err("output type", other);
                        return 0;
                    }
                }
                pc += 1;
            }
            // GOTO
            0x09 => {
                pc += 1;
                if pc >= p.len() {
                    return 0;
                }
                let top = stack.pop().unwrap_or(0);
                if top == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            // PUSH
            0x01 => {
                pc += 1;
                if pc >= p.len() {
                    return 0;
                }
                // The bytecode value pushed is the operand byte, treated as
                // an unsigned integer (matches C `spush(S, P[pc])`).
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
                return 0;
            }
        }
    }
}

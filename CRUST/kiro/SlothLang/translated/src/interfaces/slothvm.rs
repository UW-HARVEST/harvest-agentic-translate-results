use crate::{parser, throw};
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
    let program = sbin.as_ref().expect("No program");
    let p = &program.codes;
    let mut stack = crate::stack::Stack::new();
    let mut pc: usize = 0;

    loop {
        match p[pc] {
            0x00 => { // EXIT
                return stack.pop().unwrap_or(0);
            }
            0x02 => { // ADD
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(a.wrapping_add(b));
                pc += 1;
            }
            0x03 => { // SUB
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(a.wrapping_sub(b));
                pc += 1;
            }
            0x04 => { // MULT
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(a.wrapping_mul(b));
                pc += 1;
            }
            0x05 => { // DIV
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                if b == 0 { throw::math_err("division by zero"); }
                if a == i32::MIN && b == -1 { throw::math_err("division by zero"); }
                stack.push(a / b);
                pc += 1;
            }
            0x06 => { // COMP
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                pc += 1;
                let res = match p[pc] {
                    0x01 => a == b,
                    0x02 => a != b,
                    0x03 => a < b,
                    0x04 => a <= b,
                    0x05 => a > b,
                    0x06 => a >= b,
                    _ => { throw::op_err("comparison", p[pc]); false }
                };
                stack.push(res as i32);
                pc += 1;
            }
            0x07 => { // INP
                pc += 1;
                match p[pc] {
                    0x01 => { // INT
                        print!(">");
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        let x: i32 = input.trim().parse().unwrap_or(0);
                        stack.push(x);
                    }
                    0x02 => { // CHR
                        print!(">");
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        let x = input.bytes().next().unwrap_or(0) as i32;
                        stack.push(x);
                    }
                    _ => { throw::op_err("input type", p[pc]); }
                }
                pc += 1;
            }
            0x08 => { // OUT
                pc += 1;
                match p[pc] {
                    0x01 => { // INT
                        let x = stack.pop().unwrap();
                        print!("{}", x);
                    }
                    0x02 => { // CHR
                        let x = stack.pop().unwrap() as u8 as char;
                        print!("{}", x);
                    }
                    _ => { throw::op_err("output type", p[pc]); }
                }
                pc += 1;
            }
            0x09 => { // GOTO
                pc += 1;
                if stack.pop().unwrap() == 1 {
                    pc = p[pc] as usize;
                } else {
                    pc += 1;
                }
            }
            0x01 => { // PUSH
                pc += 1;
                stack.push(p[pc] as i32);
                pc += 1;
            }
            0x0A => { // DUP
                let x = stack.pop().unwrap();
                stack.push(x);
                stack.push(x);
                pc += 1;
            }
            _ => {
                throw::op_err("operation", p[pc]);
            }
        }
    }
}

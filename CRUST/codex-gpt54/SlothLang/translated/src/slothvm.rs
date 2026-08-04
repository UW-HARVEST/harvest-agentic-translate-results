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
let Some(program) = sbin.as_mut() else {
return 0;
};

fn pop_or_die(stack: &mut crate::stack::Stack) -> i32 {
match stack.pop() {
Some(value) => value,
None => {
eprintln!("[ERROR] stack underflow");
std::process::exit(1);
}
}
}

fn read_int() -> i32 {
use std::io::Write;

print!(">");
let _ = std::io::stdout().flush();
let mut input = String::new();
if std::io::stdin().read_line(&mut input).is_err() {
return 0;
}
input.trim().parse::<i32>().unwrap_or(0)
}

fn read_char() -> i32 {
let mut input = String::new();
if std::io::stdin().read_line(&mut input).is_err() {
return 0;
}
let mut chars = input.chars();
match chars.next() {
Some('>') => chars.next().unwrap_or('\0') as i32,
Some(ch) => ch as i32,
None => 0,
}
}

let mut stack = crate::stack::Stack::new();
let codes = &program.codes;
let mut pc = 0usize;

loop {
match codes.get(pc).copied().unwrap_or(0) {
0x00 => {
return if stack.is_empty() { 0 } else { pop_or_die(&mut stack) };
}
0x02 => {
let b = pop_or_die(&mut stack);
let a = pop_or_die(&mut stack);
stack.push(a + b);
pc += 1;
}
0x03 => {
let b = pop_or_die(&mut stack);
let a = pop_or_die(&mut stack);
stack.push(a - b);
pc += 1;
}
0x04 => {
let b = pop_or_die(&mut stack);
let a = pop_or_die(&mut stack);
stack.push(a * b);
pc += 1;
}
0x05 => {
let b = pop_or_die(&mut stack);
let a = pop_or_die(&mut stack);
if b == 0 || (a == i32::MIN && b == -1) {
throw::math_err("division by zero");
}
stack.push(a / b);
pc += 1;
}
0x06 => {
let b = pop_or_die(&mut stack);
let a = pop_or_die(&mut stack);
pc += 1;

let res = match codes.get(pc).copied().unwrap_or(0) {
0x01 => a == b,
0x02 => a != b,
0x03 => a < b,
0x04 => a <= b,
0x05 => a > b,
0x06 => a >= b,
code => {
throw::op_err("comparison", code);
false
}
};
stack.push(i32::from(res));
pc += 1;
}
0x07 => {
pc += 1;
match codes.get(pc).copied().unwrap_or(0) {
0x01 => stack.push(read_int()),
0x02 => stack.push(read_char()),
code => {
throw::op_err("input type", code);
}
}
pc += 1;
}
0x08 => {
use std::io::Write;

pc += 1;
match codes.get(pc).copied().unwrap_or(0) {
0x01 => {
print!("{}", pop_or_die(&mut stack));
}
0x02 => {
let x = pop_or_die(&mut stack);
print!("{}", char::from(x as u8));
}
code => {
throw::op_err("output type", code);
}
}
let _ = std::io::stdout().flush();
pc += 1;
}
0x09 => {
pc += 1;
if pop_or_die(&mut stack) == 1 {
pc = codes.get(pc).copied().unwrap_or(0) as usize;
} else {
pc += 1;
}
}
0x01 => {
pc += 1;
stack.push(codes.get(pc).copied().unwrap_or(0) as i32);
pc += 1;
}
0x0A => {
let x = pop_or_die(&mut stack);
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

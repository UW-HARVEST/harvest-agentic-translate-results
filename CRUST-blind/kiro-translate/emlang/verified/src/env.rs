use std::fmt;
use std::io::Write;
use crate::{
    data::{self, DataValue},
    em::{self, EmType, Program},
    stack,
};
pub const GC_FREQUENCY_IN_TICKS: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuntimeError {
    StackUnderflow,
    InvalidAccess,
    DivByZero,
    IncorrectType,
}
impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::StackUnderflow => write!(f, "Stack underflow"),
            RuntimeError::InvalidAccess => write!(f, "Invalid access"),
            RuntimeError::DivByZero => write!(f, "Division by zero"),
            RuntimeError::IncorrectType => write!(f, "Incorrect type"),
        }
    }
}
#[derive(Debug)]
pub struct RuntimeResult {
    pub ex: i64,
    pub em: Result<em::Em, RuntimeError>,
}
#[derive(Debug)]
pub struct Env<'a> {
    pub prog: &'a Program,
    pub stack: stack::Stack,
    pub ip: usize,
    pub ex: usize,
    pub tick: usize,
    pub halt: bool,
    pub print: bool,
    pub print_from: usize,
}

fn result_ok(ex: i64) -> RuntimeResult {
    RuntimeResult { ex, em: Ok(em::Em::new(EmType::Push)) }
}

#[allow(dead_code)]
fn result_err(_err: RuntimeError, _em: &em::Em) -> RuntimeResult {
    RuntimeResult { ex: 0, em: Err(_err) }
}

impl<'a> Env<'a> {
    pub fn new(stack_cap: usize, popped_cap: usize) -> Self {
        // Use a dummy reference that will be replaced in run()
        // We need a static empty program to satisfy the borrow checker
        static EMPTY: Program = Program { ems: Vec::new(), cap: 0, size: 0 };
        Env {
            prog: &EMPTY,
            stack: stack::Stack::new(stack_cap, popped_cap),
            ip: 0,
            ex: 0,
            tick: 0,
            halt: false,
            print: false,
            print_from: 0,
        }
    }
    pub fn run(&mut self, prog: &Program) -> RuntimeResult {
        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;

        // We need to work with the program directly since lifetime issues
        // prevent storing the reference. We'll use indices.
        let mut ip: usize = 0;
        let prog_size = prog.size;
        // Track which ems have been ran
        let mut ran = vec![false; prog_size];

        while ip < prog_size && !self.halt {
            ran[ip] = true;
            let em_type = prog.ems[ip].em_type;

            macro_rules! stack_pop {
                () => {
                    match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        },
                    }
                };
            }

            macro_rules! pop_int {
                () => {{
                    let v = stack_pop!();
                    if v.dtype != data::DataType::Int {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    match v.value { DataValue::Int(i) => i, _ => 0 }
                }};
            }

            match em_type {
                EmType::Push => {
                    self.stack.push(prog.ems[ip].data.clone());
                }
                EmType::Pop => {
                    stack_pop!();
                    if self.print && self.print_from > self.stack.size {
                        self.print_from = self.stack.size;
                    }
                }
                EmType::Add | EmType::Sub | EmType::Mul | EmType::Div |
                EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
                    let b = stack_pop!();
                    let a = stack_pop!();
                    if a.dtype != data::DataType::Int || a.dtype != b.dtype {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    let ai = match a.value { DataValue::Int(v) => v, _ => 0 };
                    let bi = match b.value { DataValue::Int(v) => v, _ => 0 };
                    let result = match em_type {
                        EmType::Add => ai.wrapping_add(bi),
                        EmType::Sub => ai.wrapping_sub(bi),
                        EmType::Mul => ai.wrapping_mul(bi),
                        EmType::Div => {
                            if bi == 0 {
                                return RuntimeResult { ex: 0, em: Err(RuntimeError::DivByZero) };
                            }
                            ai / bi
                        }
                        EmType::Grt => if ai > bi { 1 } else { 0 },
                        EmType::Less => if ai < bi { 1 } else { 0 },
                        EmType::Equ => if ai == bi { 1 } else { 0 },
                        EmType::Nequ => if ai != bi { 1 } else { 0 },
                        _ => 0,
                    };
                    self.stack.push(data::Data::new_int(result));
                }
                EmType::PrintBegin => {
                    let em_ref = prog.ems[ip].r#ref;
                    if ip == em_ref - 1 {
                        let d = stack_pop!();
                        let target_val = match prog.ems[em_ref].data.value {
                            DataValue::Int(v) => v,
                            _ => 0,
                        };
                        if target_val == em::DATA_STDOUT as i64 {
                            println!("{}", d);
                            let _ = std::io::stdout().flush();
                        } else {
                            eprintln!("{}", d);
                            let _ = std::io::stderr().flush();
                        }
                    } else {
                        self.print = true;
                        self.print_from = self.stack.size;
                    }
                }
                EmType::PrintEnd => {
                    if !self.print || self.print_from == self.stack.size {
                        // do nothing
                    } else {
                        self.print = false;
                        let target_val = match prog.ems[ip].data.value {
                            DataValue::Int(v) => v,
                            _ => 0,
                        };
                        let is_stdout = target_val == em::DATA_STDOUT as i64;
                        let mut first = true;
                        for i in self.print_from..self.stack.size {
                            if !first {
                                if is_stdout {
                                    print!(" ");
                                } else {
                                    eprint!(" ");
                                }
                            }
                            first = false;
                            if is_stdout {
                                print!("{}", self.stack.buf[i]);
                            } else {
                                eprint!("{}", self.stack.buf[i]);
                            }
                        }
                        self.stack.shrink_to(self.print_from);
                        if is_stdout {
                            println!();
                            let _ = std::io::stdout().flush();
                        } else {
                            eprintln!();
                            let _ = std::io::stderr().flush();
                        }
                    }
                }
                EmType::IfBegin => {
                    let cond = pop_int!();
                    if cond == 0 {
                        ip = prog.ems[ip].r#ref;
                    }
                }
                EmType::IfEnd => {}
                EmType::LoopBegin => {
                    let cond = pop_int!();
                    if cond == 0 {
                        ip = prog.ems[ip].r#ref;
                    }
                }
                EmType::LoopEnd => {
                    ip = prog.ems[ip].r#ref - 1;
                }
                EmType::Exit => {
                    let ex = pop_int!();
                    self.ex = ex as usize;
                    self.halt = true;
                }
                EmType::Dup => {
                    let off = pop_int!();
                    if self.stack.dup(off as usize) != 0 {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::InvalidAccess) };
                    }
                }
                EmType::Swap => {
                    let off = pop_int!();
                    if self.stack.swap(off as usize) != 0 {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::InvalidAccess) };
                    }
                }
                #[cfg(debug_assertions)]
                EmType::Debug => {
                    for i in 0..self.stack.size {
                        println!("stack[{}]: {}", i, self.stack.buf[i]);
                    }
                }
            }

            ip += 1;
            self.tick += 1;
            if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
                self.stack.gc();
            }
        }

        self.stack.clear();
        // env_gc: free data strings of ems that haven't been ran
        // In Rust, this is a no-op since we don't own the program's strings

        result_ok(self.ex as i64)
    }
    pub fn gc(&mut self) {
        // env_gc in C frees program em data strings that haven't been ran
        // In Rust this is handled by ownership, so nothing to do
    }
}

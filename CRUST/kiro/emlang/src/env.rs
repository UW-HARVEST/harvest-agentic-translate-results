use std::fmt;
use crate::{
    data::{self, Data, DataType, DataValue},
    em::{self, Em, EmType, Program, DATA_STDOUT},
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
        let s = match self {
            RuntimeError::StackUnderflow => "Stack underflow",
            RuntimeError::InvalidAccess => "Invalid access",
            RuntimeError::DivByZero => "Division by zero",
            RuntimeError::IncorrectType => "Incorrect type",
        };
        write!(f, "{}", s)
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
impl<'a> Env<'a> {
    pub fn new(stack_cap: usize, popped_cap: usize) -> Self {
        // Use a static empty program as placeholder
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

        let mut ip: usize = 0;
        while ip < prog.size && !self.halt {
            let em = &prog.ems[ip];
            // We track ran on a clone basis - but since we can't mutate prog, we skip ran tracking
            // The C code sets em->ran = true but we only need it for env_gc which frees unran string data
            // In Rust, ownership handles this automatically

            match em.em_type {
                EmType::Push => {
                    self.stack.push(em.data.clone());
                }
                EmType::Pop => {
                    if self.stack.pop().is_none() {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) };
                    }
                    if self.print && self.print_from > self.stack.size {
                        self.print_from = self.stack.size;
                    }
                }
                EmType::Add | EmType::Sub | EmType::Mul | EmType::Div |
                EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
                    let b = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                    };
                    if a.dtype != DataType::Int || b.dtype != DataType::Int {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    let av = match a.value { DataValue::Int(v) => v, _ => 0 };
                    let bv = match b.value { DataValue::Int(v) => v, _ => 0 };

                    let result = match em.em_type {
                        EmType::Add => av + bv,
                        EmType::Sub => av - bv,
                        EmType::Mul => av * bv,
                        EmType::Div => {
                            if bv == 0 {
                                return RuntimeResult { ex: 0, em: Err(RuntimeError::DivByZero) };
                            }
                            av / bv
                        }
                        EmType::Grt => (av > bv) as i64,
                        EmType::Less => (av < bv) as i64,
                        EmType::Equ => (av == bv) as i64,
                        EmType::Nequ => (av != bv) as i64,
                        _ => 0,
                    };
                    self.stack.push(Data::new_int(result));
                }
                EmType::PrintBegin => {
                    if ip == em.r#ref - 1 {
                        // Single-element print shorthand
                        let d = match self.stack.pop() {
                            Some(d) => d,
                            None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                        };
                        let file_val = match &prog.ems[em.r#ref].data.value {
                            DataValue::Int(v) => *v,
                            _ => DATA_STDOUT as i64,
                        };
                        if file_val == DATA_STDOUT as i64 {
                            println!("{}", d);
                        } else {
                            eprintln!("{}", d);
                        }
                    } else {
                        self.print = true;
                        self.print_from = self.stack.size;
                    }
                }
                EmType::PrintEnd => {
                    if !self.print || self.print_from == self.stack.size {
                        // nothing to print
                    } else {
                        self.print = false;
                        let file_val = match &em.data.value {
                            DataValue::Int(v) => *v,
                            _ => DATA_STDOUT as i64,
                        };
                        let mut parts = Vec::new();
                        for i in self.print_from..self.stack.size {
                            parts.push(format!("{}", self.stack.buf[i]));
                        }
                        let line = parts.join(" ");
                        if file_val == DATA_STDOUT as i64 {
                            println!("{}", line);
                        } else {
                            eprintln!("{}", line);
                        }
                        self.stack.shrink_to(self.print_from);
                    }
                }
                EmType::IfBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                    };
                    if cond.dtype != DataType::Int {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    let v = match cond.value { DataValue::Int(v) => v, _ => 0 };
                    if v == 0 {
                        ip = em.r#ref;
                    }
                }
                EmType::IfEnd => {}
                EmType::LoopBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                    };
                    if cond.dtype != DataType::Int {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    let v = match cond.value { DataValue::Int(v) => v, _ => 0 };
                    if v == 0 {
                        ip = em.r#ref;
                    }
                }
                EmType::LoopEnd => {
                    ip = em.r#ref.wrapping_sub(1); // will be incremented at end of loop
                }
                EmType::Exit => {
                    let ex = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                    };
                    if ex.dtype != DataType::Int {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    let v = match ex.value { DataValue::Int(v) => v, _ => 0 };
                    self.ex = v as usize;
                    self.halt = true;
                }
                EmType::Dup => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                    };
                    if off.dtype != DataType::Int {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    let v = match off.value { DataValue::Int(v) => v, _ => 0 };
                    if self.stack.dup(v as usize) != 0 {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::InvalidAccess) };
                    }
                }
                EmType::Swap => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                    };
                    if off.dtype != DataType::Int {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    let v = match off.value { DataValue::Int(v) => v, _ => 0 };
                    if self.stack.swap(v as usize) != 0 {
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

            self.tick += 1;
            if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
                self.stack.gc();
            }
            ip += 1;
        }

        self.stack.clear();
        RuntimeResult { ex: self.ex as i64, em: Ok(Em::new(EmType::Push)) }
    }
    pub fn gc(&mut self) {
        // No-op in Rust - ownership handles memory
    }
}

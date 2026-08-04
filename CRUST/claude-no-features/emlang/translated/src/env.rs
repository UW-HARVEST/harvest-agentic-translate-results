use std::fmt;
use std::io::Write;
use crate::{
    data::{self, DataType, DataValue},
    em::{self, EmType, Program, DATA_STDOUT},
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
        // Use a 'static reference workaround through Box::leak so that we can
        // construct an Env without an existing program. The reference is
        // overwritten by `run` before being used.
        let empty: &'static Program = Box::leak(Box::new(Program::new(1)));
        Env {
            prog: empty,
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
        // Take a snapshot of the program to operate on. Because the env's
        // prog reference outlives the run call only for 'a, we work on a
        // local copy instead.
        let mut local: Program = prog.clone();
        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;

        while self.ip < local.size && !self.halt {
            let em_clone = local.ems[self.ip].clone();
            local.ems[self.ip].ran = true;

            let res = self.exec_em(&em_clone, &mut local);
            match res {
                Ok(()) => {}
                Err(e) => {
                    return RuntimeResult {
                        ex: 0,
                        em: Err(e),
                    };
                }
            }

            self.ip = self.ip.wrapping_add(1);
            self.tick += 1;
            if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
                self.stack.gc();
            }
        }

        self.stack.clear();
        self.gc_program(&local);
        RuntimeResult {
            ex: self.ex as i64,
            em: Ok(em::Em::new(EmType::Exit)),
        }
    }
    pub fn gc(&mut self) {
        self.stack.gc();
    }
}

impl<'a> Env<'a> {
    fn gc_program(&mut self, _prog: &Program) {
        // Strings owned by Em data are dropped when prog is dropped; safe
        // Rust handles this. No-op.
    }

    fn exec_em(&mut self, em: &em::Em, prog: &mut Program) -> Result<(), RuntimeError> {
        match em.em_type {
            EmType::Push => {
                self.stack.push(em.data.clone());
            }
            EmType::Pop => {
                self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if self.print && self.print_from > self.stack.size {
                    self.print_from = self.stack.size;
                }
            }
            EmType::Add | EmType::Sub | EmType::Mul | EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
                let b = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let a = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if a.dtype != DataType::Int || a.dtype != b.dtype {
                    return Err(RuntimeError::IncorrectType);
                }
                let av = match a.value {
                    DataValue::Int(v) => v,
                    _ => unreachable!(),
                };
                let bv = match b.value {
                    DataValue::Int(v) => v,
                    _ => unreachable!(),
                };
                let result = match em.em_type {
                    EmType::Add => av.wrapping_add(bv),
                    EmType::Sub => av.wrapping_sub(bv),
                    EmType::Mul => av.wrapping_mul(bv),
                    EmType::Grt => if av > bv { 1 } else { 0 },
                    EmType::Less => if av < bv { 1 } else { 0 },
                    EmType::Equ => if av == bv { 1 } else { 0 },
                    EmType::Nequ => if av != bv { 1 } else { 0 },
                    _ => unreachable!(),
                };
                self.stack.push(data::Data::new_int(result));
            }
            EmType::Div => {
                let b = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let a = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if a.dtype != DataType::Int || a.dtype != b.dtype {
                    return Err(RuntimeError::IncorrectType);
                }
                let av = match a.value {
                    DataValue::Int(v) => v,
                    _ => unreachable!(),
                };
                let bv = match b.value {
                    DataValue::Int(v) => v,
                    _ => unreachable!(),
                };
                if bv == 0 {
                    return Err(RuntimeError::DivByZero);
                }
                self.stack.push(data::Data::new_int(av / bv));
            }
            EmType::PrintBegin => {
                if self.ip == em.r#ref.wrapping_sub(1) {
                    let data = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    let dest_int = match &prog.ems[em.r#ref].data.value {
                        DataValue::Int(v) => *v as i32,
                        _ => DATA_STDOUT,
                    };
                    if dest_int == DATA_STDOUT {
                        let mut out = std::io::stdout();
                        let _ = write!(out, "{}", data);
                        let _ = writeln!(out);
                        let _ = out.flush();
                    } else {
                        let mut out = std::io::stderr();
                        let _ = write!(out, "{}", data);
                        let _ = writeln!(out);
                        let _ = out.flush();
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
                    let dest_int = match &em.data.value {
                        DataValue::Int(v) => *v as i32,
                        _ => DATA_STDOUT,
                    };
                    let stdout = std::io::stdout();
                    let stderr = std::io::stderr();
                    let mut buf = String::new();
                    for i in self.print_from..self.stack.size {
                        if i > self.print_from {
                            buf.push(' ');
                        }
                        buf.push_str(&format!("{}", self.stack.buf[i]));
                    }
                    if dest_int == DATA_STDOUT {
                        let mut s = stdout.lock();
                        let _ = writeln!(s, "{}", buf);
                        let _ = s.flush();
                    } else {
                        let mut s = stderr.lock();
                        let _ = writeln!(s, "{}", buf);
                        let _ = s.flush();
                    }
                    let from = self.print_from;
                    self.stack.shrink_to(from);
                }
            }
            EmType::IfBegin => {
                let cond = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if cond.dtype != DataType::Int {
                    return Err(RuntimeError::IncorrectType);
                }
                let v = match cond.value {
                    DataValue::Int(v) => v,
                    _ => unreachable!(),
                };
                if v == 0 {
                    self.ip = em.r#ref;
                }
            }
            EmType::IfEnd => {}
            EmType::LoopBegin => {
                let cond = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if cond.dtype != DataType::Int {
                    return Err(RuntimeError::IncorrectType);
                }
                let v = match cond.value {
                    DataValue::Int(v) => v,
                    _ => unreachable!(),
                };
                if v == 0 {
                    self.ip = em.r#ref;
                }
            }
            EmType::LoopEnd => {
                // ip will be incremented by main loop, so set to ref-1
                self.ip = em.r#ref.wrapping_sub(1);
            }
            EmType::Exit => {
                let ex = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if ex.dtype != DataType::Int {
                    return Err(RuntimeError::IncorrectType);
                }
                let v = match ex.value {
                    DataValue::Int(v) => v,
                    _ => unreachable!(),
                };
                self.ex = v as usize;
                self.halt = true;
            }
            EmType::Dup => {
                let off = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if off.dtype != DataType::Int {
                    return Err(RuntimeError::IncorrectType);
                }
                let v = match off.value {
                    DataValue::Int(v) => v as usize,
                    _ => unreachable!(),
                };
                if self.stack.dup(v) != 0 {
                    return Err(RuntimeError::InvalidAccess);
                }
            }
            EmType::Swap => {
                let off = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if off.dtype != DataType::Int {
                    return Err(RuntimeError::IncorrectType);
                }
                let v = match off.value {
                    DataValue::Int(v) => v as usize,
                    _ => unreachable!(),
                };
                if self.stack.swap(v) != 0 {
                    return Err(RuntimeError::InvalidAccess);
                }
            }
            #[cfg(debug_assertions)]
            EmType::Debug => {
                for i in 0..self.stack.size {
                    println!("stack[{}]: {}", i, self.stack.buf[i]);
                }
            }
        }
        Ok(())
    }
}

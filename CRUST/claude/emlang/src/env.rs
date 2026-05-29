use std::fmt;
use std::io::Write;

use crate::{
    data::{self, DataType, DataValue},
    em::{self, Program, Em, EmType, DATA_STDOUT},
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

// Static empty program used as a placeholder for `new`. Tied to 'static lifetime.
fn empty_program_placeholder() -> &'static Program {
    use std::sync::OnceLock;
    static PROG: OnceLock<Program> = OnceLock::new();
    PROG.get_or_init(|| Program::new(em::DEFAULT_PROGRAM_CAP))
}

fn data_to_string(data: &data::Data) -> String {
    match &data.value {
        DataValue::Int(i) => format!("{}", *i as i32),
        DataValue::Str(s) => s.clone(),
    }
}

impl<'a> Env<'a> {
    pub fn new(stack_cap: usize, popped_cap: usize) -> Self {
        Env {
            prog: empty_program_placeholder(),
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
        // Make a local copy of the program. Since 'a lifetime is fixed, we
        // operate on the borrowed program by index.
        // But we need to mutate `ran` flags and update refs - the C version does
        // this via mutating the program. For our Rust version, we'll keep a
        // local mutable copy.
        let mut local_prog = prog.clone();
        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;

        let mut result_em: Option<Em> = None;
        let mut runtime_err: Option<RuntimeError> = None;

        while self.ip < local_prog.size && !self.halt {
            // Get a snapshot copy of the current em, mark as ran.
            local_prog.ems[self.ip].ran = true;
            let em_clone = local_prog.ems[self.ip].clone();

            let outcome = self.exec_em(&em_clone, &local_prog);
            match outcome {
                Ok(_) => {}
                Err((err, em)) => {
                    runtime_err = Some(err);
                    result_em = Some(em);
                    break;
                }
            }

            self.tick += 1;
            if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
                self.stack.gc();
            }
            // ip is advanced after instruction unless it was modified by the instruction
            // To match the C version's `for (e->ip = 0; ...; ++e->ip)`, we need to
            // increment ip as part of the loop step.
            self.ip += 1;
        }

        self.stack.clear();
        // env_gc: in C, this frees data of unran instructions. In Rust, ownership handles this.

        if let Some(err) = runtime_err {
            RuntimeResult {
                ex: 0,
                em: Err(err),
            }
        } else {
            RuntimeResult {
                ex: self.ex as i64,
                em: Ok(result_em.unwrap_or_else(|| Em::new(EmType::Push))),
            }
        }
    }
    pub fn gc(&mut self) {
        self.stack.gc();
    }
}

impl<'a> Env<'a> {
    fn exec_em(
        &mut self,
        em: &Em,
        prog: &Program,
    ) -> Result<(), (RuntimeError, Em)> {
        match em.em_type {
            EmType::Push => {
                self.stack.push(em.data.clone());
            }
            EmType::Pop => {
                if self.stack.pop().is_none() {
                    return Err((RuntimeError::StackUnderflow, em.clone()));
                }
                if self.print && self.print_from > self.stack.size {
                    self.print_from = self.stack.size;
                }
            }
            EmType::Add | EmType::Sub | EmType::Mul | EmType::Grt
            | EmType::Less | EmType::Equ | EmType::Nequ => {
                let b = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                let a = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                if a.dtype != DataType::Int || a.dtype != b.dtype {
                    return Err((RuntimeError::IncorrectType, em.clone()));
                }
                let av = match a.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                let bv = match b.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                let res: i64 = match em.em_type {
                    EmType::Add => av.wrapping_add(bv),
                    EmType::Sub => av.wrapping_sub(bv),
                    EmType::Mul => av.wrapping_mul(bv),
                    EmType::Grt => (av > bv) as i64,
                    EmType::Less => (av < bv) as i64,
                    EmType::Equ => (av == bv) as i64,
                    EmType::Nequ => (av != bv) as i64,
                    _ => unreachable!(),
                };
                self.stack.push(data::Data::new_int(res));
            }
            EmType::Div => {
                let b = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                let a = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                if a.dtype != DataType::Int || a.dtype != b.dtype {
                    return Err((RuntimeError::IncorrectType, em.clone()));
                }
                let av = match a.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                let bv = match b.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                if bv == 0 {
                    return Err((RuntimeError::DivByZero, em.clone()));
                }
                self.stack.push(data::Data::new_int(av / bv));
            }
            EmType::PrintBegin => {
                if self.ip == em.r#ref.wrapping_sub(1) && em.r#ref > 0 {
                    let data = self
                        .stack
                        .pop()
                        .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                    let to_stdout = match &prog.ems[em.r#ref].data.value {
                        DataValue::Int(i) => *i == DATA_STDOUT as i64,
                        _ => false,
                    };
                    let line = format!("{}\n", data_to_string(&data));
                    if to_stdout {
                        let stdout = std::io::stdout();
                        let mut h = stdout.lock();
                        let _ = h.write_all(line.as_bytes());
                        let _ = h.flush();
                    } else {
                        let stderr = std::io::stderr();
                        let mut h = stderr.lock();
                        let _ = h.write_all(line.as_bytes());
                        let _ = h.flush();
                    }
                } else {
                    self.print = true;
                    self.print_from = self.stack.size;
                }
            }
            EmType::PrintEnd => {
                if !self.print || self.print_from == self.stack.size {
                    return Ok(());
                }
                self.print = false;
                let to_stdout = match &em.data.value {
                    DataValue::Int(i) => *i == DATA_STDOUT as i64,
                    _ => false,
                };
                let mut output = String::new();
                for i in self.print_from..self.stack.size {
                    if i > self.print_from {
                        output.push(' ');
                    }
                    output.push_str(&data_to_string(&self.stack.buf[i]));
                }
                output.push('\n');
                let pf = self.print_from;
                self.stack.shrink_to(pf);
                if to_stdout {
                    let stdout = std::io::stdout();
                    let mut h = stdout.lock();
                    let _ = h.write_all(output.as_bytes());
                    let _ = h.flush();
                } else {
                    let stderr = std::io::stderr();
                    let mut h = stderr.lock();
                    let _ = h.write_all(output.as_bytes());
                    let _ = h.flush();
                }
            }
            EmType::IfBegin => {
                let cond = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                if cond.dtype != DataType::Int {
                    return Err((RuntimeError::IncorrectType, em.clone()));
                }
                let v = match cond.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                if v == 0 {
                    self.ip = em.r#ref;
                }
            }
            EmType::IfEnd => {}
            EmType::LoopBegin => {
                let cond = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                if cond.dtype != DataType::Int {
                    return Err((RuntimeError::IncorrectType, em.clone()));
                }
                let v = match cond.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                if v == 0 {
                    self.ip = em.r#ref;
                }
            }
            EmType::LoopEnd => {
                self.ip = em.r#ref.wrapping_sub(1);
            }
            EmType::Exit => {
                let ex = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                if ex.dtype != DataType::Int {
                    return Err((RuntimeError::IncorrectType, em.clone()));
                }
                let v = match ex.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                self.ex = v as usize;
                self.halt = true;
            }
            EmType::Dup => {
                let off = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                if off.dtype != DataType::Int {
                    return Err((RuntimeError::IncorrectType, em.clone()));
                }
                let v = match off.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                if self.stack.dup(v as usize) != 0 {
                    return Err((RuntimeError::InvalidAccess, em.clone()));
                }
            }
            EmType::Swap => {
                let off = self
                    .stack
                    .pop()
                    .ok_or((RuntimeError::StackUnderflow, em.clone()))?;
                if off.dtype != DataType::Int {
                    return Err((RuntimeError::IncorrectType, em.clone()));
                }
                let v = match off.value {
                    DataValue::Int(i) => i,
                    _ => unreachable!(),
                };
                if self.stack.swap(v as usize) != 0 {
                    return Err((RuntimeError::InvalidAccess, em.clone()));
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

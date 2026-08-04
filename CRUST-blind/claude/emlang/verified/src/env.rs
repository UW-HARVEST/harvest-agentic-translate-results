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

fn empty_program() -> &'static Program {
    use std::sync::OnceLock;
    static EMPTY: OnceLock<Program> = OnceLock::new();
    EMPTY.get_or_init(|| Program {
        ems: Vec::new(),
        cap: 1,
        size: 0,
    })
}

impl<'a> Env<'a> {
    pub fn new(stack_cap: usize, popped_cap: usize) -> Self {
        Env {
            prog: empty_program(),
            stack: stack::Stack::make(stack_cap, popped_cap),
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
        self.print_from = 0;
        self.tick = 0;
        self.ip = 0;

        let prog_size = prog.ems.len();
        while self.ip < prog_size && !self.halt {
            let em_clone = prog.ems[self.ip].clone();
            let em_type = em_clone.em_type;
            let em_ref = em_clone.r#ref;

            match em_type {
                EmType::Push => {
                    self.stack.do_push(em_clone.data.clone());
                }
                EmType::Pop => {
                    if self.stack.do_pop().is_none() {
                        return mk_err(RuntimeError::StackUnderflow, em_clone);
                    }
                    if self.print && self.print_from > self.stack.size {
                        self.print_from = self.stack.size;
                    }
                }
                EmType::Add | EmType::Sub | EmType::Mul
                | EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
                    let (a, b) = match self.pop_two_int(&em_clone) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                    let r = match em_type {
                        EmType::Add => a.wrapping_add(b),
                        EmType::Sub => a.wrapping_sub(b),
                        EmType::Mul => a.wrapping_mul(b),
                        EmType::Grt => (a > b) as i64,
                        EmType::Less => (a < b) as i64,
                        EmType::Equ => (a == b) as i64,
                        EmType::Nequ => (a != b) as i64,
                        _ => unreachable!(),
                    };
                    self.stack.do_push(data::Data::new_int(r));
                }
                EmType::Div => {
                    let (a, b) = match self.pop_two_int(&em_clone) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                    if b == 0 {
                        return mk_err(RuntimeError::DivByZero, em_clone);
                    }
                    self.stack.do_push(data::Data::new_int(a / b));
                }
                EmType::PrintBegin => {
                    if self.ip + 1 == em_ref {
                        let datum = match self.stack.do_pop() {
                            Some(d) => d,
                            None => return mk_err(RuntimeError::StackUnderflow, em_clone),
                        };
                        let target = match &prog.ems[em_ref].data.value {
                            DataValue::Int(i) => *i as i32,
                            _ => 0,
                        };
                        if target == em::DATA_STDOUT {
                            print!("{}", datum);
                            println!();
                            let _ = std::io::stdout().flush();
                        } else {
                            eprint!("{}", datum);
                            eprintln!();
                            let _ = std::io::stderr().flush();
                        }
                    } else {
                        self.print = true;
                        self.print_from = self.stack.size;
                    }
                }
                EmType::PrintEnd => {
                    if self.print && self.print_from != self.stack.size {
                        self.print = false;
                        let target = match &em_clone.data.value {
                            DataValue::Int(i) => *i as i32,
                            _ => 0,
                        };
                        let mut output = String::new();
                        for i in self.print_from..self.stack.size {
                            if i > self.print_from {
                                output.push(' ');
                            }
                            output.push_str(&format!("{}", self.stack.buf[i]));
                        }
                        output.push('\n');
                        if target == em::DATA_STDOUT {
                            print!("{}", output);
                            let _ = std::io::stdout().flush();
                        } else {
                            eprint!("{}", output);
                            let _ = std::io::stderr().flush();
                        }
                        let pf = self.print_from;
                        self.stack.do_shrink_to(pf);
                    }
                }
                EmType::IfBegin => {
                    let cond = match self.pop_int(&em_clone) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                    if cond == 0 {
                        self.ip = em_ref;
                    }
                }
                EmType::IfEnd => {}
                EmType::LoopBegin => {
                    let cond = match self.pop_int(&em_clone) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                    if cond == 0 {
                        self.ip = em_ref;
                    }
                }
                EmType::LoopEnd => {
                    if em_ref > 0 {
                        self.ip = em_ref - 1;
                    } else {
                        self.ip = 0;
                    }
                }
                EmType::Exit => {
                    let exi = match self.pop_int(&em_clone) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                    self.ex = exi as usize;
                    self.halt = true;
                }
                EmType::Dup => {
                    let oi = match self.pop_int(&em_clone) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                    if self.stack.do_dup(oi as usize) != 0 {
                        return mk_err(RuntimeError::InvalidAccess, em_clone);
                    }
                }
                EmType::Swap => {
                    let oi = match self.pop_int(&em_clone) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                    if self.stack.do_swap(oi as usize) != 0 {
                        return mk_err(RuntimeError::InvalidAccess, em_clone);
                    }
                }
                #[cfg(debug_assertions)]
                EmType::Debug => {
                    for i in 0..self.stack.size {
                        println!("stack[{}]: {}", i, self.stack.buf[i]);
                    }
                }
            }

            self.ip += 1;
            self.tick += 1;
            if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
                self.stack.do_gc();
            }
        }

        self.stack.do_clear();
        self.gc();

        // Use a default em as the placeholder for "no error" (Ok variant carries last em
        // or a default).
        let last_em = if !prog.ems.is_empty() {
            prog.ems[prog.ems.len() - 1].clone()
        } else {
            em::Em::new(EmType::Pop)
        };
        RuntimeResult {
            ex: self.ex as i64,
            em: Ok(last_em),
        }
    }

    pub fn gc(&mut self) {
        // No-op: in Rust, all data is owned, so the GC of unused string-data
        // referenced by program is unnecessary—dropping the program drops them.
        let _ = self.prog;
    }

    fn pop_int(&mut self, em: &em::Em) -> Result<i64, RuntimeResult> {
        let d = match self.stack.do_pop() {
            Some(d) => d,
            None => return Err(mk_err(RuntimeError::StackUnderflow, em.clone())),
        };
        if d.dtype != data::DataType::Int {
            return Err(mk_err(RuntimeError::IncorrectType, em.clone()));
        }
        match d.value {
            DataValue::Int(i) => Ok(i),
            _ => unreachable!(),
        }
    }

    fn pop_two_int(&mut self, em: &em::Em) -> Result<(i64, i64), RuntimeResult> {
        let b = match self.stack.do_pop() {
            Some(d) => d,
            None => return Err(mk_err(RuntimeError::StackUnderflow, em.clone())),
        };
        let a = match self.stack.do_pop() {
            Some(d) => d,
            None => return Err(mk_err(RuntimeError::StackUnderflow, em.clone())),
        };
        if a.dtype != data::DataType::Int || b.dtype != data::DataType::Int {
            return Err(mk_err(RuntimeError::IncorrectType, em.clone()));
        }
        let ai = match a.value {
            DataValue::Int(i) => i,
            _ => unreachable!(),
        };
        let bi = match b.value {
            DataValue::Int(i) => i,
            _ => unreachable!(),
        };
        Ok((ai, bi))
    }
}

fn mk_err(err: RuntimeError, _em: em::Em) -> RuntimeResult {
    // The struct stores em as Result<Em, RuntimeError>. We use Err(err) to indicate an
    // error occurred (location info is lost in this encoding).
    RuntimeResult { ex: 0, em: Err(err) }
}

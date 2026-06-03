use std::fmt;
use std::io::Write;
use crate::{
    data,
    em::{self, Em, EmType, Program},
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
        // Use a leaked empty Program reference as a placeholder until run() is called.
        // We use a static dummy via Box::leak only once is not possible in safe Rust easily.
        // Instead, we use unsafe to construct an Env that will have its prog set before use.
        // For simplicity, create a static empty program reference.
        // The C version stores prog as a pointer that gets set in env_run.
        // We'll work around the lifetime by using a static empty program.
        static EMPTY: std::sync::OnceLock<Program> = std::sync::OnceLock::new();
        let empty = EMPTY.get_or_init(|| Program {
            ems: Vec::new(),
            cap: 1,
            size: 0,
        });
        // Safety: we cast the static lifetime to 'a; since this static never moves, it's safe.
        let prog: &'a Program = unsafe { std::mem::transmute(empty as &Program) };
        Env {
            prog,
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
        // Re-bind prog with extended lifetime; we need to access prog via self.prog.
        // Safety: caller guarantees prog outlives the call.
        let prog_ref: &'a Program = unsafe { std::mem::transmute(prog) };
        self.prog = prog_ref;
        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;

        // We will track ran flags locally because Program is borrowed immutably.
        let prog_size = self.prog.size;
        let mut ran: Vec<bool> = vec![false; prog_size];

        while self.ip < prog_size && !self.halt {
            // clone the em for processing (no mutable access to prog)
            let em = self.prog.ems[self.ip].clone();
            ran[self.ip] = true;

            match em.em_type {
                EmType::Push => {
                    self.stack.push(em.data.clone());
                }
                EmType::Pop => {
                    if self.stack.pop().is_none() {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        };
                    }
                    if self.print && self.print_from > self.stack.size {
                        self.print_from = self.stack.size;
                    }
                }
                EmType::Add | EmType::Sub | EmType::Mul | EmType::Grt
                | EmType::Less | EmType::Equ | EmType::Nequ => {
                    let b = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let (av, bv) = match (&a.value, &b.value) {
                        (data::DataValue::Int(av), data::DataValue::Int(bv)) => (*av, *bv),
                        _ => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::IncorrectType),
                            };
                        }
                    };
                    let result = match em.em_type {
                        EmType::Add => av + bv,
                        EmType::Sub => av - bv,
                        EmType::Mul => av * bv,
                        EmType::Grt => (av > bv) as i64,
                        EmType::Less => (av < bv) as i64,
                        EmType::Equ => (av == bv) as i64,
                        EmType::Nequ => (av != bv) as i64,
                        _ => unreachable!(),
                    };
                    self.stack.push(data::Data::new_int(result));
                }
                EmType::Div => {
                    let b = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let (av, bv) = match (&a.value, &b.value) {
                        (data::DataValue::Int(av), data::DataValue::Int(bv)) => (*av, *bv),
                        _ => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::IncorrectType),
                            };
                        }
                    };
                    if bv == 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::DivByZero),
                        };
                    }
                    self.stack.push(data::Data::new_int(av / bv));
                }
                EmType::PrintBegin => {
                    if self.ip == em.r#ref.wrapping_sub(1) {
                        // Single-arg print form
                        let d = match self.stack.pop() {
                            Some(d) => d,
                            None => {
                                return RuntimeResult {
                                    ex: 0,
                                    em: Err(RuntimeError::StackUnderflow),
                                };
                            }
                        };
                        let stream_int = match &self.prog.ems[em.r#ref].data.value {
                            data::DataValue::Int(i) => *i,
                            _ => 0,
                        };
                        if stream_int == em::DATA_STDOUT as i64 {
                            print!("{}", d);
                            println!();
                            std::io::stdout().flush().ok();
                        } else {
                            eprint!("{}", d);
                            eprintln!();
                            std::io::stderr().flush().ok();
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
                        let stream_int = match &em.data.value {
                            data::DataValue::Int(i) => *i,
                            _ => 0,
                        };
                        let to_stdout = stream_int == em::DATA_STDOUT as i64;
                        let mut out = String::new();
                        for i in self.print_from..self.stack.size {
                            if i > self.print_from {
                                out.push(' ');
                            }
                            out.push_str(&format!("{}", self.stack.buf[i]));
                        }
                        if to_stdout {
                            print!("{}", out);
                            println!();
                            std::io::stdout().flush().ok();
                        } else {
                            eprint!("{}", out);
                            eprintln!();
                            std::io::stderr().flush().ok();
                        }
                        self.stack.shrink_to(self.print_from);
                    }
                }
                EmType::IfBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let cv = match &cond.value {
                        data::DataValue::Int(i) => *i,
                        _ => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::IncorrectType),
                            };
                        }
                    };
                    if cv == 0 {
                        self.ip = em.r#ref;
                    }
                }
                EmType::IfEnd => {}
                EmType::LoopBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let cv = match &cond.value {
                        data::DataValue::Int(i) => *i,
                        _ => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::IncorrectType),
                            };
                        }
                    };
                    if cv == 0 {
                        self.ip = em.r#ref;
                    }
                }
                EmType::LoopEnd => {
                    self.ip = em.r#ref.wrapping_sub(1);
                }
                EmType::Exit => {
                    let ex_val = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let ev = match &ex_val.value {
                        data::DataValue::Int(i) => *i,
                        _ => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::IncorrectType),
                            };
                        }
                    };
                    self.ex = ev as usize;
                    self.halt = true;
                }
                EmType::Dup => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let ov = match &off.value {
                        data::DataValue::Int(i) => *i,
                        _ => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::IncorrectType),
                            };
                        }
                    };
                    if self.stack.dup(ov as usize) != 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::InvalidAccess),
                        };
                    }
                }
                EmType::Swap => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    let ov = match &off.value {
                        data::DataValue::Int(i) => *i,
                        _ => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::IncorrectType),
                            };
                        }
                    };
                    if self.stack.swap(ov as usize) != 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::InvalidAccess),
                        };
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
                self.stack.gc();
            }
        }

        self.stack.clear();
        self.gc();
        RuntimeResult {
            ex: self.ex as i64,
            em: Ok(Em::new(EmType::Push)),
        }
    }

    pub fn gc(&mut self) {
        // Strings in `data` are owned by the program in the C version (heap-allocated).
        // In Rust the strings are owned, so we don't need to free anything explicitly.
        // No-op for memory cleanup; provided for API parity with C.
    }
}

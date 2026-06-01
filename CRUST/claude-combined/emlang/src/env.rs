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

// Empty/dummy program reference for initial Env construction.
// We use a thread-local trick or just a static empty Program.
// Since `prog` is `&'a Program`, we need a default. We'll use a 'static dummy.
fn dummy_program() -> &'static Program {
    use std::sync::OnceLock;
    static DUMMY: OnceLock<Program> = OnceLock::new();
    DUMMY.get_or_init(|| Program {
        ems: Vec::new(),
        cap: 1,
        size: 0,
    })
}

impl<'a> Env<'a> {
    pub fn new(stack_cap: usize, popped_cap: usize) -> Self {
        Env {
            prog: dummy_program(),
            stack: stack::Stack::new(stack_cap, popped_cap),
            ip: 0,
            ex: 0,
            tick: 0,
            halt: false,
            print: false,
            print_from: 0,
        }
    }
    pub fn run(&mut self, prog: &'a Program) -> RuntimeResult {
        self.ex = 0;
        self.prog = prog;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;

        let stdout = std::io::stdout();
        let stderr = std::io::stderr();

        while self.ip < self.prog.size && !self.halt {
            let em = self.prog.ems[self.ip].clone();
            // We can't easily mark the original program's ems as ran since we
            // hold an immutable reference. The C code uses `ran` for GC of
            // string data. In Rust this isn't relevant - skip.
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
                EmType::Add | EmType::Sub | EmType::Mul
                | EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
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
                    if a.dtype != data::DataType::Int || a.dtype != b.dtype {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let av = match a.value { data::DataValue::Int(v) => v, _ => 0 };
                    let bv = match b.value { data::DataValue::Int(v) => v, _ => 0 };
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
                    if a.dtype != data::DataType::Int || a.dtype != b.dtype {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let av = match a.value { data::DataValue::Int(v) => v, _ => 0 };
                    let bv = match b.value { data::DataValue::Int(v) => v, _ => 0 };
                    if bv == 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::DivByZero),
                        };
                    }
                    self.stack.push(data::Data::new_int(av / bv));
                }
                EmType::PrintBegin => {
                    if self.ip == em.r#ref - 1 {
                        let data = match self.stack.pop() {
                            Some(d) => d,
                            None => {
                                return RuntimeResult {
                                    ex: 0,
                                    em: Err(RuntimeError::StackUnderflow),
                                };
                            }
                        };
                        let target_int = match &self.prog.ems[em.r#ref].data.value {
                            data::DataValue::Int(v) => *v as i32,
                            _ => 0,
                        };
                        if target_int == em::DATA_STDOUT {
                            let mut h = stdout.lock();
                            let _ = write!(h, "{}\n", data);
                            let _ = h.flush();
                        } else {
                            let mut h = stderr.lock();
                            let _ = write!(h, "{}\n", data);
                            let _ = h.flush();
                        }
                    } else {
                        self.print = true;
                        self.print_from = self.stack.size;
                    }
                }
                EmType::PrintEnd => {
                    if !self.print || self.print_from == self.stack.size {
                        // skip
                    } else {
                        self.print = false;
                        let target_int = match &em.data.value {
                            data::DataValue::Int(v) => *v as i32,
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
                        if target_int == em::DATA_STDOUT {
                            let mut h = stdout.lock();
                            let _ = h.write_all(output.as_bytes());
                            let _ = h.flush();
                        } else {
                            let mut h = stderr.lock();
                            let _ = h.write_all(output.as_bytes());
                            let _ = h.flush();
                        }
                        let pf = self.print_from;
                        self.stack.shrink_to(pf);
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
                    if cond.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = match cond.value { data::DataValue::Int(v) => v, _ => 0 };
                    if v == 0 {
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
                    if cond.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = match cond.value { data::DataValue::Int(v) => v, _ => 0 };
                    if v == 0 {
                        self.ip = em.r#ref;
                    }
                }
                EmType::LoopEnd => {
                    self.ip = em.r#ref - 1;
                }
                EmType::Exit => {
                    let ex = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            };
                        }
                    };
                    if ex.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = match ex.value { data::DataValue::Int(v) => v, _ => 0 };
                    self.ex = v as usize;
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
                    if off.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = match off.value { data::DataValue::Int(v) => v, _ => 0 };
                    if self.stack.dup(v as usize) != 0 {
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
                    if off.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = match off.value { data::DataValue::Int(v) => v, _ => 0 };
                    if self.stack.swap(v as usize) != 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::InvalidAccess),
                        };
                    }
                }
                #[cfg(debug_assertions)]
                EmType::Debug => {
                    let mut h = stdout.lock();
                    for i in 0..self.stack.size {
                        let _ = write!(h, "stack[{}]: {}\n", i, self.stack.buf[i]);
                    }
                    let _ = h.flush();
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
        // No-op for Rust since strings are owned.
    }
}

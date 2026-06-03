use std::fmt;
use std::io::Write;
use crate::{
    data,
    em::{self, Em, EmType, Program},
    stack::{self, Stack},
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
// Empty static program for default
fn empty_prog() -> &'static Program {
    use std::sync::OnceLock;
    static EMPTY: OnceLock<Program> = OnceLock::new();
    EMPTY.get_or_init(|| Program::new(1))
}

impl<'a> Env<'a> {
    pub fn new(stack_cap: usize, popped_cap: usize) -> Self {
        Env {
            prog: empty_prog(),
            stack: Stack::new(stack_cap, popped_cap),
            ip: 0,
            ex: 0,
            tick: 0,
            halt: false,
            print: false,
            print_from: 0,
        }
    }
    pub fn run(&mut self, prog: &Program) -> RuntimeResult {
        // SAFETY: We extend the lifetime of prog. The reference is only used during run().
        let prog_ref: &'a Program = unsafe { std::mem::transmute(prog) };
        self.prog = prog_ref;
        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;
        while self.ip < self.prog.size && !self.halt {
            let em_clone = self.prog.ems[self.ip].clone();
            // Mark ran (note: we can't actually mutate prog here since it's immutable; we skip ran tracking
            // because the reference is immutable. The C version mutates ran on the prog — but in Rust
            // since the reference is &Program, we cannot. The 'ran' field exists but is not used here.)

            let em_type = em_clone.em_type;
            match em_type {
                EmType::Push => {
                    self.stack.push(em_clone.data.clone());
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
                EmType::Add | EmType::Sub | EmType::Mul | EmType::Div |
                EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
                    let b = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        },
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        },
                    };
                    if a.dtype != data::DataType::Int || a.dtype != b.dtype {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let av = match a.value {
                        data::DataValue::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let bv = match b.value {
                        data::DataValue::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let result = match em_type {
                        EmType::Add => av + bv,
                        EmType::Sub => av - bv,
                        EmType::Mul => av * bv,
                        EmType::Div => {
                            if bv == 0 {
                                return RuntimeResult {
                                    ex: 0,
                                    em: Err(RuntimeError::DivByZero),
                                };
                            }
                            av / bv
                        }
                        EmType::Grt => if av > bv { 1 } else { 0 },
                        EmType::Less => if av < bv { 1 } else { 0 },
                        EmType::Equ => if av == bv { 1 } else { 0 },
                        EmType::Nequ => if av != bv { 1 } else { 0 },
                        _ => unreachable!(),
                    };
                    self.stack.push(data::Data::new_int(result));
                }
                EmType::PrintBegin => {
                    if self.ip == em_clone.r#ref.wrapping_sub(1) {
                        let d = match self.stack.pop() {
                            Some(d) => d,
                            None => return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            },
                        };
                        let to_stdout = match &self.prog.ems[em_clone.r#ref].data.value {
                            data::DataValue::Int(i) => *i == em::DATA_STDOUT as i64,
                            _ => true,
                        };
                        let line = format!("{}", d);
                        if to_stdout {
                            println!("{}", line);
                            let _ = std::io::stdout().flush();
                        } else {
                            eprintln!("{}", line);
                            let _ = std::io::stderr().flush();
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
                        let to_stdout = match &em_clone.data.value {
                            data::DataValue::Int(i) => *i == em::DATA_STDOUT as i64,
                            _ => true,
                        };
                        let mut output = String::new();
                        for i in self.print_from..self.stack.size {
                            if i > self.print_from {
                                output.push(' ');
                            }
                            output.push_str(&format!("{}", self.stack.buf[i]));
                        }
                        if to_stdout {
                            println!("{}", output);
                            let _ = std::io::stdout().flush();
                        } else {
                            eprintln!("{}", output);
                            let _ = std::io::stderr().flush();
                        }
                        let target = self.print_from;
                        self.stack.shrink_to(target);
                    }
                }
                EmType::IfBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        },
                    };
                    if cond.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let cv = match cond.value {
                        data::DataValue::Int(i) => i,
                        _ => unreachable!(),
                    };
                    if cv == 0 {
                        self.ip = em_clone.r#ref;
                    }
                }
                EmType::IfEnd => {}
                EmType::LoopBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        },
                    };
                    if cond.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let cv = match cond.value {
                        data::DataValue::Int(i) => i,
                        _ => unreachable!(),
                    };
                    if cv == 0 {
                        self.ip = em_clone.r#ref;
                    }
                }
                EmType::LoopEnd => {
                    self.ip = em_clone.r#ref.wrapping_sub(1);
                }
                EmType::Exit => {
                    let ex = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        },
                    };
                    if ex.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let ev = match ex.value {
                        data::DataValue::Int(i) => i,
                        _ => unreachable!(),
                    };
                    self.ex = ev as usize;
                    self.halt = true;
                }
                EmType::Dup => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        },
                    };
                    if off.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let ov = match off.value {
                        data::DataValue::Int(i) => i,
                        _ => unreachable!(),
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
                        None => return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        },
                    };
                    if off.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let ov = match off.value {
                        data::DataValue::Int(i) => i,
                        _ => unreachable!(),
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

            self.tick += 1;
            if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
                self.stack.gc();
            }
            self.ip += 1;
        }
        self.stack.clear();
        self.gc();
        RuntimeResult {
            ex: self.ex as i64,
            em: Ok(Em::new(EmType::Exit)),
        }
    }
    pub fn gc(&mut self) {
        // In Rust, strings are owned, so GC is a no-op
    }
}

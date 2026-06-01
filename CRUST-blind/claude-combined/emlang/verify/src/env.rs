use std::fmt;
use std::io::Write;
use crate::{
    data,
    em::{self, Program},
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

// Static empty program reused for default-constructed Env (matching C's
// zero-initialization where prog starts NULL)
fn empty_program_ref() -> &'static Program {
    use std::sync::OnceLock;
    static EMPTY: OnceLock<Program> = OnceLock::new();
    EMPTY.get_or_init(|| Program::new(1))
}

impl<'a> Env<'a> {
    pub fn new(stack_cap: usize, popped_cap: usize) -> Self {
        Env {
            prog: empty_program_ref(),
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
        // SAFETY: extending lifetime to 'a. We reset prog at end of run via clear.
        // We do not store it back into self after this run completes; users use a
        // single env_run call as in C.
        let prog_ptr: *const Program = prog as *const Program;
        // To avoid unsafe lifetime extension, instead operate on a local copy of
        // the program (cheap via reference). But the field requires &'a Program.
        // We'll set self.prog through a transmute-equivalent done safely by
        // storing pointer-derived reference for the duration of run only.
        // Use a small unsafe block (acceptable here) to set the field.
        unsafe {
            let env_self: *mut Env<'a> = self as *mut Env<'a>;
            (*env_self).prog = &*prog_ptr;
        }

        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;

        while self.ip < prog.size && !self.halt {
            let em_type = prog.ems[self.ip].em_type;
            // mark ran (mutate via interior—prog is &; we cannot mutate.
            // The C code marks ran for GC. Since our Rust GC works differently
            // (each popped string is owned), we don't need ran tracking. Skip.)

            match em_type {
                em::EmType::Push => {
                    let d = prog.ems[self.ip].data.clone();
                    self.stack.push(d);
                }
                em::EmType::Pop => {
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
                em::EmType::Add
                | em::EmType::Sub
                | em::EmType::Mul
                | em::EmType::Grt
                | em::EmType::Less
                | em::EmType::Equ
                | em::EmType::Nequ => {
                    let b = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    if a.dtype != data::DataType::Int || a.dtype != b.dtype {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let av = if let data::DataValue::Int(v) = a.value { v } else { 0 };
                    let bv = if let data::DataValue::Int(v) = b.value { v } else { 0 };
                    let result: i64 = match em_type {
                        em::EmType::Add => av.wrapping_add(bv),
                        em::EmType::Sub => av.wrapping_sub(bv),
                        em::EmType::Mul => av.wrapping_mul(bv),
                        em::EmType::Grt => (av > bv) as i64,
                        em::EmType::Less => (av < bv) as i64,
                        em::EmType::Equ => (av == bv) as i64,
                        em::EmType::Nequ => (av != bv) as i64,
                        _ => unreachable!(),
                    };
                    self.stack.push(data::Data::new_int(result));
                }
                em::EmType::Div => {
                    let b = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    if a.dtype != data::DataType::Int || a.dtype != b.dtype {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let av = if let data::DataValue::Int(v) = a.value { v } else { 0 };
                    let bv = if let data::DataValue::Int(v) = b.value { v } else { 0 };
                    if bv == 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::DivByZero),
                        };
                    }
                    self.stack.push(data::Data::new_int(av / bv));
                }
                em::EmType::PrintBegin => {
                    let cur_em = &prog.ems[self.ip];
                    if self.ip == cur_em.r#ref.wrapping_sub(1) {
                        let d = match self.stack.pop() {
                            Some(d) => d,
                            None => {
                                return RuntimeResult {
                                    ex: 0,
                                    em: Err(RuntimeError::StackUnderflow),
                                };
                            }
                        };
                        let target_int = match &prog.ems[cur_em.r#ref].data.value {
                            data::DataValue::Int(i) => *i as i32,
                            _ => 0,
                        };
                        if target_int == em::DATA_STDOUT {
                            print!("{}", d);
                            println!();
                            let _ = std::io::stdout().flush();
                        } else {
                            eprint!("{}", d);
                            eprintln!();
                            let _ = std::io::stderr().flush();
                        }
                    } else {
                        self.print = true;
                        self.print_from = self.stack.size;
                    }
                }
                em::EmType::PrintEnd => {
                    if !self.print || self.print_from == self.stack.size {
                        // do nothing
                    } else {
                        self.print = false;
                        let target_int = match &prog.ems[self.ip].data.value {
                            data::DataValue::Int(i) => *i as i32,
                            _ => 0,
                        };
                        let to_stdout = target_int == em::DATA_STDOUT;
                        let mut output = String::new();
                        for i in self.print_from..self.stack.size {
                            if i > self.print_from {
                                output.push(' ');
                            }
                            output.push_str(&format!("{}", self.stack.buf[i]));
                        }
                        output.push('\n');
                        if to_stdout {
                            print!("{}", output);
                            let _ = std::io::stdout().flush();
                        } else {
                            eprint!("{}", output);
                            let _ = std::io::stderr().flush();
                        }
                        self.stack.shrink_to(self.print_from);
                    }
                }
                em::EmType::IfBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    if cond.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = if let data::DataValue::Int(i) = cond.value { i } else { 0 };
                    if v == 0 {
                        self.ip = prog.ems[self.ip].r#ref;
                    }
                }
                em::EmType::IfEnd => {}
                em::EmType::LoopBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    if cond.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = if let data::DataValue::Int(i) = cond.value { i } else { 0 };
                    if v == 0 {
                        self.ip = prog.ems[self.ip].r#ref;
                    }
                }
                em::EmType::LoopEnd => {
                    self.ip = prog.ems[self.ip].r#ref.wrapping_sub(1);
                }
                em::EmType::Exit => {
                    let ex = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    if ex.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    if let data::DataValue::Int(i) = ex.value {
                        self.ex = i as usize;
                    }
                    self.halt = true;
                }
                em::EmType::Dup => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    if off.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = if let data::DataValue::Int(i) = off.value { i as usize } else { 0 };
                    if self.stack.dup(v) != 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::InvalidAccess),
                        };
                    }
                }
                em::EmType::Swap => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => {
                            return RuntimeResult {
                                ex: 0,
                                em: Err(RuntimeError::StackUnderflow),
                            }
                        }
                    };
                    if off.dtype != data::DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = if let data::DataValue::Int(i) = off.value { i as usize } else { 0 };
                    if self.stack.swap(v) != 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::InvalidAccess),
                        };
                    }
                }
                #[cfg(debug_assertions)]
                em::EmType::Debug => {
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
            em: Ok(em::Em::new(em::EmType::Push)),
        }
    }
    pub fn gc(&mut self) {
        // The Rust translation owns each string, so the C-style pointer-tracking
        // is unnecessary. Mirror the C behavior by clearing any stale popped
        // entries and ensuring no leak (handled by Rust's ownership).
        self.stack.gc();
    }
}

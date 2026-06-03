use std::fmt;
use std::io::Write;
use crate::{
    data::{Data, DataType, DataValue},
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

// A tiny placeholder program that we use for the lifetime parameter on Env::new.
// Once `run` is called, the env's prog reference is updated to point to the real program.
fn empty_program_ref() -> &'static Program {
    use std::sync::OnceLock;
    static EMPTY: OnceLock<Program> = OnceLock::new();
    EMPTY.get_or_init(|| Program {
        ems: Vec::new(),
        cap: 0,
        size: 0,
    })
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
        // SAFETY: We store a reference into the Env. Since the lifetime
        // tying gets relaxed via a transmute, the caller must ensure prog
        // outlives the env after this call (which is the case in tests).
        // To avoid unsafe, we instead drive the run from a local clone of
        // the program's instructions and avoid storing the reference.
        // Reset state.
        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.print_from = 0;

        let prog_size = prog.size;
        // We need to mutate ems[i].ran = true for the run, but `prog` is &.
        // We track which ems were run locally, so we don't actually mutate.
        let mut ran: Vec<bool> = vec![false; prog_size];
        // Make local copies of all ems we may need for return values.
        // We'll reference the ems via index throughout.

        self.ip = 0;
        while self.ip < prog_size && !self.halt {
            let em_ref = &prog.ems[self.ip];
            ran[self.ip] = true;
            let em_clone = em_ref.clone();

            match em_ref.em_type {
                EmType::Push => {
                    self.stack.push(em_ref.data.clone());
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
                    if a.dtype != DataType::Int || a.dtype != b.dtype {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let av = match a.value {
                        DataValue::Int(v) => v,
                        _ => 0,
                    };
                    let bv = match b.value {
                        DataValue::Int(v) => v,
                        _ => 0,
                    };
                    let result = match em_ref.em_type {
                        EmType::Add => av.wrapping_add(bv),
                        EmType::Sub => av.wrapping_sub(bv),
                        EmType::Mul => av.wrapping_mul(bv),
                        EmType::Grt => (av > bv) as i64,
                        EmType::Less => (av < bv) as i64,
                        EmType::Equ => (av == bv) as i64,
                        EmType::Nequ => (av != bv) as i64,
                        _ => unreachable!(),
                    };
                    self.stack.push(Data::new_int(result));
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
                    if a.dtype != DataType::Int || a.dtype != b.dtype {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let av = match a.value { DataValue::Int(v) => v, _ => 0 };
                    let bv = match b.value { DataValue::Int(v) => v, _ => 0 };
                    if bv == 0 {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::DivByZero),
                        };
                    }
                    self.stack.push(Data::new_int(av / bv));
                }
                EmType::PrintBegin => {
                    if self.ip == em_ref.r#ref.wrapping_sub(1) {
                        let data = match self.stack.pop() {
                            Some(d) => d,
                            None => {
                                return RuntimeResult {
                                    ex: 0,
                                    em: Err(RuntimeError::StackUnderflow),
                                };
                            }
                        };
                        let target_em = &prog.ems[em_ref.r#ref];
                        let target_int = match &target_em.data.value {
                            DataValue::Int(v) => *v as i32,
                            _ => 0,
                        };
                        if target_int == DATA_STDOUT {
                            print!("{}", data);
                            println!();
                            std::io::stdout().flush().ok();
                        } else {
                            eprint!("{}", data);
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
                        // Skip
                    } else {
                        self.print = false;
                        let target_int = match &em_ref.data.value {
                            DataValue::Int(v) => *v as i32,
                            _ => 0,
                        };
                        let mut output = String::new();
                        for i in self.print_from..self.stack.size {
                            if i > self.print_from {
                                output.push(' ');
                            }
                            output.push_str(&format!("{}", self.stack.buf[i]));
                        }
                        if target_int == DATA_STDOUT {
                            println!("{}", output);
                            std::io::stdout().flush().ok();
                        } else {
                            eprintln!("{}", output);
                            std::io::stderr().flush().ok();
                        }
                        let from = self.print_from;
                        self.stack.shrink_to(from);
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
                    if cond.dtype != DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let cv = match cond.value { DataValue::Int(v) => v, _ => 0 };
                    if cv == 0 {
                        self.ip = em_ref.r#ref;
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
                    if cond.dtype != DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let cv = match cond.value { DataValue::Int(v) => v, _ => 0 };
                    if cv == 0 {
                        self.ip = em_ref.r#ref;
                    }
                }
                EmType::LoopEnd => {
                    self.ip = em_ref.r#ref.wrapping_sub(1);
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
                    if ex.dtype != DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let v = match ex.value { DataValue::Int(v) => v, _ => 0 };
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
                    if off.dtype != DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let off_v = match off.value { DataValue::Int(v) => v, _ => 0 };
                    if self.stack.dup(off_v as usize) != 0 {
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
                    if off.dtype != DataType::Int {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::IncorrectType),
                        };
                    }
                    let off_v = match off.value { DataValue::Int(v) => v, _ => 0 };
                    if self.stack.swap(off_v as usize) != 0 {
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

            // ip increment
            self.ip = self.ip.wrapping_add(1);

            self.tick = self.tick.wrapping_add(1);
            if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
                self.stack.gc();
            }

            // Suppress unused warnings
            let _ = &em_clone;
        }

        self.stack.clear();
        self.gc();
        let _ = ran;
        RuntimeResult {
            ex: self.ex as i64,
            em: Ok(Em::new(EmType::Push)),
        }
    }

    pub fn gc(&mut self) {
        // GC traverses program ems and frees data for non-run ems.
        // Since we use Rust's owned strings, we don't need to do anything here.
    }
}

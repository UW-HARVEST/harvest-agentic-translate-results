use std::fmt;
use std::io::Write;
use crate::{
    data::{self, DataValue, DataType},
    em::{self, Program, EmType, DATA_STDOUT},
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

// We need a placeholder Program to satisfy the lifetime when Env is created
// without a program. We use a static empty program via lazy_static-like trick.
// Since we can't easily have a static Program, we use an unsafe trick: store
// a reference to a leaked Box<Program> as the default initial program.
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
        // Replace the program reference. Since we can't store a non-static
        // reference, we'll work with a local copy and use indices.
        // We use unsafe to extend lifetime since Env<'a> requires it.
        // But to keep this safe, we instead just iterate with prog directly.

        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;

        // Make a mutable copy of program ems so we can mark them as ran.
        let mut ems: Vec<em::Em> = prog.ems.clone();

        while self.ip < ems.len() && !self.halt {
            // Mark ran
            ems[self.ip].ran = true;
            let em_clone = ems[self.ip].clone();
            let em_type = em_clone.em_type;

            match em_type {
                EmType::Push => {
                    self.stack.push(em_clone.data.clone());
                }
                EmType::Pop => {
                    if self.stack.pop().is_none() {
                        return self.err(RuntimeError::StackUnderflow, &em_clone);
                    }
                    if self.print && self.print_from > self.stack.size {
                        self.print_from = self.stack.size;
                    }
                }
                EmType::Add | EmType::Sub | EmType::Mul
                | EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
                    let b = match self.stack.pop() {
                        Some(d) => d,
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    if a.dtype != DataType::Int || b.dtype != DataType::Int {
                        return self.err(RuntimeError::IncorrectType, &em_clone);
                    }
                    let av = match a.value { DataValue::Int(v) => v, _ => 0 };
                    let bv = match b.value { DataValue::Int(v) => v, _ => 0 };
                    let result = match em_type {
                        EmType::Add => av.wrapping_add(bv),
                        EmType::Sub => av.wrapping_sub(bv),
                        EmType::Mul => av.wrapping_mul(bv),
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
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    if a.dtype != DataType::Int || b.dtype != DataType::Int {
                        return self.err(RuntimeError::IncorrectType, &em_clone);
                    }
                    let av = match a.value { DataValue::Int(v) => v, _ => 0 };
                    let bv = match b.value { DataValue::Int(v) => v, _ => 0 };
                    if bv == 0 {
                        return self.err(RuntimeError::DivByZero, &em_clone);
                    }
                    self.stack.push(data::Data::new_int(av.wrapping_div(bv)));
                }
                EmType::PrintBegin => {
                    let target_ref = em_clone.r#ref;
                    if self.ip == target_ref.wrapping_sub(1) {
                        // Single-element print case
                        let d = match self.stack.pop() {
                            Some(d) => d,
                            None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                        };
                        let target_em = &ems[target_ref];
                        let to_stdout = match &target_em.data.value {
                            DataValue::Int(v) => *v as i32 == DATA_STDOUT,
                            _ => false,
                        };
                        Self::write_data(&d, to_stdout);
                        Self::write_newline_flush(to_stdout);
                    } else {
                        self.print = true;
                        self.print_from = self.stack.size;
                    }
                }
                EmType::PrintEnd => {
                    if !self.print || self.print_from == self.stack.size {
                        // No-op
                    } else {
                        self.print = false;
                        let to_stdout = match &em_clone.data.value {
                            DataValue::Int(v) => *v as i32 == DATA_STDOUT,
                            _ => false,
                        };
                        for i in self.print_from..self.stack.size {
                            if i > self.print_from {
                                Self::write_char(' ', to_stdout);
                            }
                            let d = self.stack.buf[i].clone();
                            Self::write_data(&d, to_stdout);
                        }
                        let pf = self.print_from;
                        self.stack.shrink_to(pf);
                        Self::write_newline_flush(to_stdout);
                    }
                }
                EmType::IfBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    if cond.dtype != DataType::Int {
                        return self.err(RuntimeError::IncorrectType, &em_clone);
                    }
                    let cv = match cond.value { DataValue::Int(v) => v, _ => 0 };
                    if cv == 0 {
                        self.ip = em_clone.r#ref;
                    }
                }
                EmType::IfEnd => {}
                EmType::LoopBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    if cond.dtype != DataType::Int {
                        return self.err(RuntimeError::IncorrectType, &em_clone);
                    }
                    let cv = match cond.value { DataValue::Int(v) => v, _ => 0 };
                    if cv == 0 {
                        self.ip = em_clone.r#ref;
                    }
                }
                EmType::LoopEnd => {
                    self.ip = em_clone.r#ref.wrapping_sub(1);
                }
                EmType::Exit => {
                    let ex_val = match self.stack.pop() {
                        Some(d) => d,
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    if ex_val.dtype != DataType::Int {
                        return self.err(RuntimeError::IncorrectType, &em_clone);
                    }
                    let v = match ex_val.value { DataValue::Int(v) => v, _ => 0 };
                    self.ex = v as usize;
                    self.halt = true;
                }
                EmType::Dup => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    if off.dtype != DataType::Int {
                        return self.err(RuntimeError::IncorrectType, &em_clone);
                    }
                    let v = match off.value { DataValue::Int(v) => v, _ => 0 };
                    if self.stack.dup(v as usize) != 0 {
                        return self.err(RuntimeError::InvalidAccess, &em_clone);
                    }
                }
                EmType::Swap => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => return self.err(RuntimeError::StackUnderflow, &em_clone),
                    };
                    if off.dtype != DataType::Int {
                        return self.err(RuntimeError::IncorrectType, &em_clone);
                    }
                    let v = match off.value { DataValue::Int(v) => v, _ => 0 };
                    if self.stack.swap(v as usize) != 0 {
                        return self.err(RuntimeError::InvalidAccess, &em_clone);
                    }
                }
                #[cfg(debug_assertions)]
                EmType::Debug => {
                    for i in 0..self.stack.size {
                        print!("stack[{}]: ", i);
                        print!("{}", self.stack.buf[i]);
                        println!();
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
            em: Ok(em::Em::new(EmType::Exit)),
        }
    }

    pub fn gc(&mut self) {
        // No-op in Rust; ownership/Drop already handles freeing memory.
        // The C version frees data strings of un-run ems, but in Rust these
        // strings are owned inside the Em struct and freed on drop.
    }
}

impl<'a> Env<'a> {
    fn err(&self, err: RuntimeError, _em: &em::Em) -> RuntimeResult {
        RuntimeResult {
            ex: 0,
            em: Err(err),
        }
    }

    fn write_data(d: &data::Data, to_stdout: bool) {
        let s = format!("{}", d);
        if to_stdout {
            print!("{}", s);
        } else {
            eprint!("{}", s);
        }
    }

    fn write_char(c: char, to_stdout: bool) {
        if to_stdout {
            print!("{}", c);
        } else {
            eprint!("{}", c);
        }
    }

    fn write_newline_flush(to_stdout: bool) {
        if to_stdout {
            println!();
            let _ = std::io::stdout().flush();
        } else {
            eprintln!();
            let _ = std::io::stderr().flush();
        }
    }
}


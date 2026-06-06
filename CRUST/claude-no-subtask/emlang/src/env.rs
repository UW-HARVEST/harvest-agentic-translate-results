use std::fmt;
use std::io::Write;
use crate::{
    data::{Data, DataType, DataValue},
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

// We need a static reference to a program for the default Env<'a>::new since we don't have a Program at that point
// Use a workaround: use a dummy static program reference. Actually, the lifetime of 'a in new is unclear
// because new doesn't take a Program. Looking at the C code: env_new() doesn't take a program either;
// it's set via env_run. The Rust struct stores `prog: &'a Program`. To handle this, we can use a
// 'static empty program. But that conflicts with the borrowing in env_run(&prog).
//
// Actually, looking at the test: env::new() is called and then env.run(&program). The lifetime
// must work out. Let me use a leaked static reference for the initial creation, or better,
// change the field access pattern. Since we can't change struct definitions, use static empty program.

use std::sync::OnceLock;

fn empty_program() -> &'static Program {
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
            prog: empty_program(),
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
        // Use unsafe to extend lifetime so we can store reference. We just need to access fields.
        // Actually, the cleanest is to NOT store prog and just operate on a local reference here.
        // But the struct definition has `prog: &'a Program` -- we still need to set it.
        // Let's use a local variable for our work.

        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;

        // Make local clones of the program we'll modify (for the `ran` field)
        // Actually the C code modifies em->ran in place. We need a mutable copy.
        let mut prog_clone = prog.clone();

        while self.ip < prog_clone.size && !self.halt {
            // Get a clone of the current em to inspect type and data
            let em_type;
            let em_data;
            let em_ref;
            {
                let em = &mut prog_clone.ems[self.ip];
                em.ran = true;
                em_type = em.em_type;
                em_data = em.data.clone();
                em_ref = em.r#ref;
            }

            match em_type {
                EmType::Push => {
                    self.stack.push(em_data);
                }
                EmType::Pop => {
                    if self.stack.pop().is_none() {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) };
                    }
                    if self.print && self.print_from > self.stack.size {
                        self.print_from = self.stack.size;
                    }
                }
                EmType::Add | EmType::Sub | EmType::Mul | EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
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
                    let av = match a.value { DataValue::Int(i) => i, _ => 0 };
                    let bv = match b.value { DataValue::Int(i) => i, _ => 0 };
                    let result = match em_type {
                        EmType::Add => av.wrapping_add(bv),
                        EmType::Sub => av.wrapping_sub(bv),
                        EmType::Mul => av.wrapping_mul(bv),
                        EmType::Grt => if av > bv { 1 } else { 0 },
                        EmType::Less => if av < bv { 1 } else { 0 },
                        EmType::Equ => if av == bv { 1 } else { 0 },
                        EmType::Nequ => if av != bv { 1 } else { 0 },
                        _ => 0,
                    };
                    self.stack.push(Data::new_int(result));
                }
                EmType::Div => {
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
                    let av = match a.value { DataValue::Int(i) => i, _ => 0 };
                    let bv = match b.value { DataValue::Int(i) => i, _ => 0 };
                    if bv == 0 {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::DivByZero) };
                    }
                    self.stack.push(Data::new_int(av / bv));
                }
                EmType::PrintBegin => {
                    if self.ip == em_ref.wrapping_sub(1) {
                        let data = match self.stack.pop() {
                            Some(d) => d,
                            None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                        };
                        let target_em = &prog_clone.ems[em_ref];
                        let int_val = match &target_em.data.value {
                            DataValue::Int(i) => *i,
                            _ => 0,
                        };
                        if int_val == DATA_STDOUT as i64 {
                            print!("{}\n", data);
                            std::io::stdout().flush().ok();
                        } else {
                            eprint!("{}\n", data);
                            std::io::stderr().flush().ok();
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
                        let int_val = match &em_data.value {
                            DataValue::Int(i) => *i,
                            _ => 0,
                        };
                        let to_stdout = int_val == DATA_STDOUT as i64;
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
                            std::io::stdout().flush().ok();
                        } else {
                            eprint!("{}", output);
                            std::io::stderr().flush().ok();
                        }
                        let pf = self.print_from;
                        self.stack.shrink_to(pf);
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
                    let cv = match cond.value { DataValue::Int(i) => i, _ => 0 };
                    if cv == 0 {
                        self.ip = em_ref;
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
                    let cv = match cond.value { DataValue::Int(i) => i, _ => 0 };
                    if cv == 0 {
                        self.ip = em_ref;
                    }
                }
                EmType::LoopEnd => {
                    self.ip = em_ref.wrapping_sub(1);
                }
                EmType::Exit => {
                    let ex = match self.stack.pop() {
                        Some(d) => d,
                        None => return RuntimeResult { ex: 0, em: Err(RuntimeError::StackUnderflow) },
                    };
                    if ex.dtype != DataType::Int {
                        return RuntimeResult { ex: 0, em: Err(RuntimeError::IncorrectType) };
                    }
                    let ev = match ex.value { DataValue::Int(i) => i, _ => 0 };
                    self.ex = ev as usize;
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
                    let ov = match off.value { DataValue::Int(i) => i, _ => 0 };
                    if self.stack.dup(ov as usize) != 0 {
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
                    let ov = match off.value { DataValue::Int(i) => i, _ => 0 };
                    if self.stack.swap(ov as usize) != 0 {
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

            self.ip += 1;
        }

        self.stack.clear();
        self.gc();

        // Build a synthetic ok em to return; we can't return None because em is Result<Em, ...>
        let ok_em = em::Em::new(EmType::Pop);
        RuntimeResult {
            ex: self.ex as i64,
            em: Ok(ok_em),
        }
    }
    pub fn gc(&mut self) {
        // In Rust, no manual deallocation needed since strings are managed
        // In the C version, this iterates over program ems and frees DATA_STR for ems that didn't run
    }
}

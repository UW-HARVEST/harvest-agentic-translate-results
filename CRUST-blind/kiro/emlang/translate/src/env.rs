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
        // Use a dummy static program reference; will be overwritten in run()
        static EMPTY: Program = Program { ems: Vec::new(), cap: 0, size: 0 };
        Env {
            prog: &EMPTY,
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
        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;

        // We can't store the borrow in self.prog due to lifetime issues,
        // so we work directly with prog.
        let mut ip: usize = 0;
        let mut ex: usize = 0;
        let mut halt = false;
        let mut print = false;
        let mut print_from: usize = 0;
        let mut tick: usize = 0;

        while ip < prog.size && !halt {
            let em_type = prog.ems[ip].em_type;
            // mark ran - we can't mutate prog, but it's not needed for Rust version

            match em_type {
                EmType::Push => {
                    self.stack.push(prog.ems[ip].data.clone());
                }
                EmType::Pop => {
                    if self.stack.pop().is_none() {
                        return RuntimeResult {
                            ex: 0,
                            em: Err(RuntimeError::StackUnderflow),
                        };
                    }
                    if print && print_from > self.stack.size {
                        print_from = self.stack.size;
                    }
                }
                EmType::Add | EmType::Sub | EmType::Mul | EmType::Div |
                EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
                    let b = match self.stack.pop() {
                        Some(d) => d,
                        None => return err_result(RuntimeError::StackUnderflow),
                    };
                    let a = match self.stack.pop() {
                        Some(d) => d,
                        None => return err_result(RuntimeError::StackUnderflow),
                    };
                    if a.dtype != DataType::Int || a.dtype != b.dtype {
                        return err_result(RuntimeError::IncorrectType);
                    }
                    let av = match a.value { DataValue::Int(v) => v, _ => 0 };
                    let bv = match b.value { DataValue::Int(v) => v, _ => 0 };
                    let result = match em_type {
                        EmType::Add => av.wrapping_add(bv),
                        EmType::Sub => av.wrapping_sub(bv),
                        EmType::Mul => av.wrapping_mul(bv),
                        EmType::Div => {
                            if bv == 0 {
                                return err_result(RuntimeError::DivByZero);
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
                    if ip == prog.ems[ip].r#ref - 1 {
                        // Single item print
                        let d = match self.stack.pop() {
                            Some(d) => d,
                            None => return err_result(RuntimeError::StackUnderflow),
                        };
                        let ref_idx = prog.ems[ip].r#ref;
                        let is_stdout = match prog.ems[ref_idx].data.value {
                            DataValue::Int(v) => v == DATA_STDOUT as i64,
                            _ => true,
                        };
                        if is_stdout {
                            print!("{}", d);
                            println!();
                            std::io::stdout().flush().ok();
                        } else {
                            eprint!("{}", d);
                            eprintln!();
                            std::io::stderr().flush().ok();
                        }
                    } else {
                        print = true;
                        print_from = self.stack.size;
                    }
                }
                EmType::PrintEnd => {
                    if !print || print_from == self.stack.size {
                        if print {
                            // Still need to print newline and flush
                            let is_stdout = match prog.ems[ip].data.value {
                                DataValue::Int(v) => v == DATA_STDOUT as i64,
                                _ => true,
                            };
                            if is_stdout {
                                println!();
                                std::io::stdout().flush().ok();
                            } else {
                                eprintln!();
                                std::io::stderr().flush().ok();
                            }
                            print = false;
                        }
                        // If not printing at all, just break (no-op)
                    } else {
                        print = false;
                        let is_stdout = match prog.ems[ip].data.value {
                            DataValue::Int(v) => v == DATA_STDOUT as i64,
                            _ => true,
                        };
                        // Print all items from print_from to stack.size
                        for i in print_from..self.stack.size {
                            if i > print_from {
                                if is_stdout {
                                    print!(" ");
                                } else {
                                    eprint!(" ");
                                }
                            }
                            if is_stdout {
                                print!("{}", self.stack.buf[i]);
                            } else {
                                eprint!("{}", self.stack.buf[i]);
                            }
                        }
                        self.stack.shrink_to(print_from);
                        if is_stdout {
                            println!();
                            std::io::stdout().flush().ok();
                        } else {
                            eprintln!();
                            std::io::stderr().flush().ok();
                        }
                    }
                }
                EmType::IfBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => return err_result(RuntimeError::StackUnderflow),
                    };
                    if cond.dtype != DataType::Int {
                        return err_result(RuntimeError::IncorrectType);
                    }
                    if let DataValue::Int(v) = cond.value {
                        if v == 0 {
                            ip = prog.ems[ip].r#ref;
                        }
                    }
                }
                EmType::IfEnd => {}
                EmType::LoopBegin => {
                    let cond = match self.stack.pop() {
                        Some(d) => d,
                        None => return err_result(RuntimeError::StackUnderflow),
                    };
                    if cond.dtype != DataType::Int {
                        return err_result(RuntimeError::IncorrectType);
                    }
                    if let DataValue::Int(v) = cond.value {
                        if v == 0 {
                            ip = prog.ems[ip].r#ref;
                        }
                    }
                }
                EmType::LoopEnd => {
                    ip = prog.ems[ip].r#ref - 1;
                }
                EmType::Exit => {
                    let ex_data = match self.stack.pop() {
                        Some(d) => d,
                        None => return err_result(RuntimeError::StackUnderflow),
                    };
                    if ex_data.dtype != DataType::Int {
                        return err_result(RuntimeError::IncorrectType);
                    }
                    if let DataValue::Int(v) = ex_data.value {
                        ex = v as usize;
                    }
                    halt = true;
                }
                EmType::Dup => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => return err_result(RuntimeError::StackUnderflow),
                    };
                    if off.dtype != DataType::Int {
                        return err_result(RuntimeError::IncorrectType);
                    }
                    if let DataValue::Int(v) = off.value {
                        if self.stack.dup(v as usize) != 0 {
                            return err_result(RuntimeError::InvalidAccess);
                        }
                    }
                }
                EmType::Swap => {
                    let off = match self.stack.pop() {
                        Some(d) => d,
                        None => return err_result(RuntimeError::StackUnderflow),
                    };
                    if off.dtype != DataType::Int {
                        return err_result(RuntimeError::IncorrectType);
                    }
                    if let DataValue::Int(v) = off.value {
                        if self.stack.swap(v as usize) != 0 {
                            return err_result(RuntimeError::InvalidAccess);
                        }
                    }
                }
                #[cfg(debug_assertions)]
                EmType::Debug => {
                    for i in 0..self.stack.size {
                        println!("stack[{}]: {}", i, self.stack.buf[i]);
                    }
                }
            }

            ip += 1;
            tick += 1;
            if tick % GC_FREQUENCY_IN_TICKS == 0 {
                self.stack.gc();
            }
        }

        self.stack.clear();
        RuntimeResult {
            ex: ex as i64,
            em: Ok(em::Em::new(EmType::Push)), // dummy ok value
        }
    }
    pub fn gc(&mut self) {
        self.stack.gc();
    }
}

fn err_result(err: RuntimeError) -> RuntimeResult {
    RuntimeResult { ex: 0, em: Err(err) }
}

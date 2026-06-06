use std::fmt;
use std::io::Write;
use crate::{
    data,
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

fn empty_program() -> &'static Program {
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
        self.ex = 0;
        self.halt = false;
        self.print = false;
        self.tick = 0;
        self.ip = 0;
        let result = exec(self, prog);
        self.stack.clear();
        result
    }
    pub fn gc(&mut self) {
        // Strings on instructions are owned via Rust's String type, so
        // they're freed when the program is dropped. No explicit work needed.
    }
}

fn exec(env: &mut Env<'_>, prog: &Program) -> RuntimeResult {
    while env.ip < prog.size && !env.halt {
        let em_clone = prog.ems[env.ip].clone();
        match em_clone.em_type {
            EmType::Push => {
                env.stack.push(em_clone.data.clone());
            }
            EmType::Pop => {
                if env.stack.pop().is_none() {
                    return err_at(prog, env.ip, RuntimeError::StackUnderflow);
                }
                if env.print && env.print_from > env.stack.size {
                    env.print_from = env.stack.size;
                }
            }
            EmType::Add | EmType::Sub | EmType::Mul
            | EmType::Grt | EmType::Less | EmType::Equ | EmType::Nequ => {
                let (a, b) = match pop2_int(&mut env.stack) {
                    Ok(v) => v,
                    Err(e) => return err_at(prog, env.ip, e),
                };
                let result = match em_clone.em_type {
                    EmType::Add => a.wrapping_add(b),
                    EmType::Sub => a.wrapping_sub(b),
                    EmType::Mul => a.wrapping_mul(b),
                    EmType::Grt => if a > b { 1 } else { 0 },
                    EmType::Less => if a < b { 1 } else { 0 },
                    EmType::Equ => if a == b { 1 } else { 0 },
                    EmType::Nequ => if a != b { 1 } else { 0 },
                    _ => unreachable!(),
                };
                env.stack.push(data::Data::new_int(result));
            }
            EmType::Div => {
                let (a, b) = match pop2_int(&mut env.stack) {
                    Ok(v) => v,
                    Err(e) => return err_at(prog, env.ip, e),
                };
                if b == 0 {
                    return err_at(prog, env.ip, RuntimeError::DivByZero);
                }
                env.stack.push(data::Data::new_int(a / b));
            }
            EmType::PrintBegin => {
                let r = em_clone.r#ref;
                if r > 0 && env.ip == r - 1 {
                    let data_val = match env.stack.pop() {
                        Some(d) => d,
                        None => return err_at(prog, env.ip, RuntimeError::StackUnderflow),
                    };
                    let stream_int = match &prog.ems[r].data.value {
                        data::DataValue::Int(v) => *v as i32,
                        _ => DATA_STDOUT,
                    };
                    let line = format!("{}\n", data_val);
                    if stream_int == DATA_STDOUT {
                        print!("{}", line);
                        let _ = std::io::stdout().flush();
                    } else {
                        eprint!("{}", line);
                        let _ = std::io::stderr().flush();
                    }
                } else {
                    env.print = true;
                    env.print_from = env.stack.size;
                }
            }
            EmType::PrintEnd => {
                if !env.print || env.print_from == env.stack.size {
                    // skip
                } else {
                    env.print = false;
                    let stream_int = match &em_clone.data.value {
                        data::DataValue::Int(v) => *v as i32,
                        _ => DATA_STDOUT,
                    };
                    let mut out = String::new();
                    for i in env.print_from..env.stack.size {
                        if i > env.print_from {
                            out.push(' ');
                        }
                        out.push_str(&format!("{}", env.stack.buf[i]));
                    }
                    out.push('\n');
                    if stream_int == DATA_STDOUT {
                        print!("{}", out);
                        let _ = std::io::stdout().flush();
                    } else {
                        eprint!("{}", out);
                        let _ = std::io::stderr().flush();
                    }
                    let new_size = env.print_from;
                    env.stack.shrink_to(new_size);
                }
            }
            EmType::IfBegin => {
                let cond = match pop_int(&mut env.stack) {
                    Ok(v) => v,
                    Err(e) => return err_at(prog, env.ip, e),
                };
                if cond == 0 {
                    env.ip = em_clone.r#ref;
                }
            }
            EmType::IfEnd => {}
            EmType::LoopBegin => {
                let cond = match pop_int(&mut env.stack) {
                    Ok(v) => v,
                    Err(e) => return err_at(prog, env.ip, e),
                };
                if cond == 0 {
                    env.ip = em_clone.r#ref;
                }
            }
            EmType::LoopEnd => {
                env.ip = em_clone.r#ref.wrapping_sub(1);
            }
            EmType::Exit => {
                let ex = match pop_int(&mut env.stack) {
                    Ok(v) => v,
                    Err(e) => return err_at(prog, env.ip, e),
                };
                env.ex = ex as usize;
                env.halt = true;
            }
            EmType::Dup => {
                let off = match pop_int(&mut env.stack) {
                    Ok(v) => v,
                    Err(e) => return err_at(prog, env.ip, e),
                };
                if env.stack.dup(off as usize) != 0 {
                    return err_at(prog, env.ip, RuntimeError::InvalidAccess);
                }
            }
            EmType::Swap => {
                let off = match pop_int(&mut env.stack) {
                    Ok(v) => v,
                    Err(e) => return err_at(prog, env.ip, e),
                };
                if env.stack.swap(off as usize) != 0 {
                    return err_at(prog, env.ip, RuntimeError::InvalidAccess);
                }
            }
            #[cfg(debug_assertions)]
            EmType::Debug => {
                for i in 0..env.stack.size {
                    println!("stack[{}]: {}", i, env.stack.buf[i]);
                }
            }
        }

        env.ip += 1;
        env.tick += 1;
        if env.tick % GC_FREQUENCY_IN_TICKS == 0 {
            env.stack.gc();
        }
    }

    RuntimeResult {
        ex: env.ex as i64,
        em: Ok(Em::new(EmType::Exit)),
    }
}

fn err_at(prog: &Program, ip: usize, err: RuntimeError) -> RuntimeResult {
    let _em = if ip < prog.size {
        prog.ems[ip].clone()
    } else {
        Em::new(EmType::Exit)
    };
    RuntimeResult {
        ex: 0,
        em: Err(err),
    }
}

fn pop_int(stack: &mut stack::Stack) -> Result<i64, RuntimeError> {
    let d = stack.pop().ok_or(RuntimeError::StackUnderflow)?;
    match d.value {
        data::DataValue::Int(v) => Ok(v),
        _ => Err(RuntimeError::IncorrectType),
    }
}

fn pop2_int(stack: &mut stack::Stack) -> Result<(i64, i64), RuntimeError> {
    let b = stack.pop().ok_or(RuntimeError::StackUnderflow)?;
    let a = stack.pop().ok_or(RuntimeError::StackUnderflow)?;
    if a.dtype != data::DataType::Int || a.dtype != b.dtype {
        return Err(RuntimeError::IncorrectType);
    }
    let av = match a.value {
        data::DataValue::Int(v) => v,
        _ => return Err(RuntimeError::IncorrectType),
    };
    let bv = match b.value {
        data::DataValue::Int(v) => v,
        _ => return Err(RuntimeError::IncorrectType),
    };
    Ok((av, bv))
}

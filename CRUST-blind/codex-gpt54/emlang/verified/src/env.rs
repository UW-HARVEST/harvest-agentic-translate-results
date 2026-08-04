use std::fmt;
use crate::{
    data::{Data, DataType, DataValue},
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
        let msg = match self {
            RuntimeError::StackUnderflow => "Stack underflow",
            RuntimeError::InvalidAccess => "Invalid access",
            RuntimeError::DivByZero => "Division by zero",
            RuntimeError::IncorrectType => "Incorrect type",
        };

        write!(f, "{msg}")
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
        Self {
            prog: Box::leak(Box::new(Program::new(em::DEFAULT_PROGRAM_CAP))),
            stack: new_stack(stack_cap, popped_cap),
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

        while self.ip < prog.size && !self.halt {
            let em = &prog.ems[self.ip];

            match em.em_type {
                em::EmType::Push => stack_push(&mut self.stack, em.data.clone()),
                em::EmType::Pop => {
                    if stack_pop(&mut self.stack).is_none() {
                        return RuntimeResult {
                            ex: self.ex as i64,
                            em: Err(RuntimeError::StackUnderflow),
                        };
                    }

                    if self.print && self.print_from > self.stack.size {
                        self.print_from = self.stack.size;
                    }
                }
                em::EmType::Add => {
                    let (a, b) = match pop2_int(&mut self.stack) {
                        Ok(values) => values,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };
                    stack_push(&mut self.stack, Data::new_int(a + b));
                }
                em::EmType::Sub => {
                    let (a, b) = match pop2_int(&mut self.stack) {
                        Ok(values) => values,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };
                    stack_push(&mut self.stack, Data::new_int(a - b));
                }
                em::EmType::Mul => {
                    let (a, b) = match pop2_int(&mut self.stack) {
                        Ok(values) => values,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };
                    stack_push(&mut self.stack, Data::new_int(a * b));
                }
                em::EmType::Div => {
                    let (a, b) = match pop2_int(&mut self.stack) {
                        Ok(values) => values,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };

                    if b == 0 {
                        return RuntimeResult {
                            ex: self.ex as i64,
                            em: Err(RuntimeError::DivByZero),
                        };
                    }

                    stack_push(&mut self.stack, Data::new_int(a / b));
                }
                em::EmType::Grt => {
                    let (a, b) = match pop2_int(&mut self.stack) {
                        Ok(values) => values,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };
                    stack_push(&mut self.stack, Data::new_int((a > b) as i64));
                }
                em::EmType::Less => {
                    let (a, b) = match pop2_int(&mut self.stack) {
                        Ok(values) => values,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };
                    stack_push(&mut self.stack, Data::new_int((a < b) as i64));
                }
                em::EmType::Equ => {
                    let (a, b) = match pop2_int(&mut self.stack) {
                        Ok(values) => values,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };
                    stack_push(&mut self.stack, Data::new_int((a == b) as i64));
                }
                em::EmType::Nequ => {
                    let (a, b) = match pop2_int(&mut self.stack) {
                        Ok(values) => values,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };
                    stack_push(&mut self.stack, Data::new_int((a != b) as i64));
                }
                em::EmType::PrintBegin => {
                    if self.ip == em.r#ref.saturating_sub(1) {
                        let data = match stack_pop(&mut self.stack) {
                            Some(data) => data,
                            None => {
                                return RuntimeResult {
                                    ex: self.ex as i64,
                                    em: Err(RuntimeError::StackUnderflow),
                                }
                            }
                        };
                        println!("{data}");
                    } else {
                        self.print = true;
                        self.print_from = self.stack.size;
                    }
                }
                em::EmType::PrintEnd => {
                    if !self.print || self.print_from == self.stack.size {
                        self.step_gc();
                        self.ip += 1;
                        continue;
                    }

                    self.print = false;
                    let output = self.stack.buf[self.print_from..self.stack.size]
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!("{output}");
                    stack_shrink_to(&mut self.stack, self.print_from);
                }
                em::EmType::IfBegin => {
                    let cond = match pop_int(&mut self.stack) {
                        Ok(cond) => cond,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };

                    if cond == 0 {
                        self.ip = em.r#ref;
                    }
                }
                em::EmType::IfEnd => {}
                em::EmType::LoopBegin => {
                    let cond = match pop_int(&mut self.stack) {
                        Ok(cond) => cond,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };

                    if cond == 0 {
                        self.ip = em.r#ref;
                    }
                }
                em::EmType::LoopEnd => self.ip = em.r#ref.saturating_sub(1),
                em::EmType::Exit => {
                    let ex = match pop_int(&mut self.stack) {
                        Ok(ex) => ex,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };
                    self.ex = ex as usize;
                    self.halt = true;
                }
                em::EmType::Dup => {
                    let off = match pop_int(&mut self.stack) {
                        Ok(off) => off,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };

                    if stack_dup(&mut self.stack, off as usize) != 0 {
                        return RuntimeResult {
                            ex: self.ex as i64,
                            em: Err(RuntimeError::InvalidAccess),
                        };
                    }
                }
                em::EmType::Swap => {
                    let off = match pop_int(&mut self.stack) {
                        Ok(off) => off,
                        Err(error) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(error),
                            }
                        }
                    };

                    if stack_swap(&mut self.stack, off as usize) != 0 {
                        return RuntimeResult {
                            ex: self.ex as i64,
                            em: Err(RuntimeError::InvalidAccess),
                        };
                    }
                }
                #[cfg(debug_assertions)]
                em::EmType::Debug => {
                    for (i, value) in self.stack.buf.iter().enumerate() {
                        println!("stack[{i}]: {value}");
                    }
                }
            }

            self.step_gc();
            self.ip += 1;
        }

        stack_clear(&mut self.stack);
        self.gc();
        RuntimeResult {
            ex: self.ex as i64,
            em: Ok(em::Em::new(em::EmType::Exit)),
        }
    }
    pub fn gc(&mut self) {
        let _ = &self.prog;
    }

    fn step_gc(&mut self) {
        self.tick += 1;
        if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
            stack_gc(&mut self.stack);
        }
    }
}

fn pop_int(stack: &mut stack::Stack) -> Result<i64, RuntimeError> {
    match stack_pop(stack) {
        Some(Data {
            dtype: DataType::Int,
            value: DataValue::Int(value),
        }) => Ok(value),
        Some(_) => Err(RuntimeError::IncorrectType),
        None => Err(RuntimeError::StackUnderflow),
    }
}

fn pop2_int(stack: &mut stack::Stack) -> Result<(i64, i64), RuntimeError> {
    let b = pop_data(stack)?;
    let a = pop_data(stack)?;

    match (a, b) {
        (
            Data {
                dtype: DataType::Int,
                value: DataValue::Int(a),
            },
            Data {
                dtype: DataType::Int,
                value: DataValue::Int(b),
            },
        ) => Ok((a, b)),
        _ => Err(RuntimeError::IncorrectType),
    }
}

fn pop_data(stack: &mut stack::Stack) -> Result<Data, RuntimeError> {
    stack_pop(stack).ok_or(RuntimeError::StackUnderflow)
}

fn new_stack(cap: usize, popped_cap: usize) -> stack::Stack {
    stack::Stack {
        buf: Vec::with_capacity(cap),
        cap,
        size: 0,
        popped: Vec::with_capacity(popped_cap),
        popped_cap,
        popped_size: 0,
    }
}

fn stack_push(stack: &mut stack::Stack, data: Data) {
    if stack.size >= stack.cap {
        stack.cap = stack.cap.saturating_mul(2).max(1);
        let additional = stack.cap.saturating_sub(stack.buf.capacity());
        if additional > 0 {
            stack.buf.reserve(additional);
        }
    }

    stack.buf.push(data);
    stack.size = stack.buf.len();
}

fn stack_pop(stack: &mut stack::Stack) -> Option<Data> {
    let data = stack.buf.pop()?;
    stack.size = stack.buf.len();

    if let DataValue::Str(value) = &data.value {
        if stack.popped_size >= stack.popped_cap {
            stack.popped_cap = stack.popped_cap.saturating_mul(2).max(1);
            let additional = stack.popped_cap.saturating_sub(stack.popped.capacity());
            if additional > 0 {
                stack.popped.reserve(additional);
            }
        }

        stack.popped.push(stack::Popped {
            str: value.clone(),
            marked: false,
        });
        stack.popped_size = stack.popped.len();
    }

    Some(data)
}

fn stack_dup(stack: &mut stack::Stack, off: usize) -> i32 {
    if off + 1 > stack.size {
        return -1;
    }

    let idx = stack.size - off - 1;
    stack_push(stack, stack.buf[idx].clone());
    0
}

fn stack_swap(stack: &mut stack::Stack, off: usize) -> i32 {
    if off + 1 > stack.size {
        return -1;
    }

    let idx = stack.size - off - 1;
    stack.buf.swap(idx, stack.size - 1);
    0
}

fn stack_shrink_to(stack: &mut stack::Stack, size: usize) {
    if size == stack.size {
        return;
    }

    assert!(size < stack.size);

    for data in &stack.buf[size..stack.size] {
        if let DataValue::Str(value) = &data.value {
            if stack.popped_size >= stack.popped_cap {
                stack.popped_cap = stack.popped_cap.saturating_mul(2).max(1);
                let additional = stack.popped_cap.saturating_sub(stack.popped.capacity());
                if additional > 0 {
                    stack.popped.reserve(additional);
                }
            }

            stack.popped.push(stack::Popped {
                str: value.clone(),
                marked: false,
            });
            stack.popped_size = stack.popped.len();
        }
    }

    stack.buf.truncate(size);
    stack.size = stack.buf.len();
}

fn stack_clear(stack: &mut stack::Stack) {
    stack_shrink_to(stack, 0);
    stack_gc(stack);
}

fn stack_gc(stack: &mut stack::Stack) {
    for popped in &mut stack.popped {
        popped.marked = false;
    }

    stack.popped.clear();
    stack.popped_size = 0;
}

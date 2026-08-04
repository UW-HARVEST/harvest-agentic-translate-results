use std::fmt;
use crate::{
    data::{Data, DataType, DataValue},
    em::{self, Em, EmType, Program, DEFAULT_PROGRAM_CAP, DATA_STDOUT},
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
        let text = match self {
            RuntimeError::StackUnderflow => "Stack underflow",
            RuntimeError::InvalidAccess => "Invalid access",
            RuntimeError::DivByZero => "Division by zero",
            RuntimeError::IncorrectType => "Incorrect type",
        };
        f.write_str(text)
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
        let empty_program = Box::leak(Box::new(Program::new(DEFAULT_PROGRAM_CAP)));
        Self {
            prog: empty_program,
            stack: stack::stack_new(stack_cap, popped_cap),
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
        self.ip = 0;
        self.tick = 0;
        self.halt = false;
        self.print = false;
        self.print_from = 0;

        while self.ip < prog.size && !self.halt {
            let em = prog.ems[self.ip].clone();
            match em.em_type {
                EmType::Push => stack::stack_push(&mut self.stack, em.data.clone()),
                EmType::Pop => {
                    if stack::stack_pop(&mut self.stack).is_none() {
                        return RuntimeResult {
                            ex: self.ex as i64,
                            em: Err(RuntimeError::StackUnderflow),
                        };
                    }

                    if self.print && self.print_from > self.stack.size {
                        self.print_from = self.stack.size;
                    }
                }
                EmType::Add | EmType::Sub | EmType::Mul | EmType::Div | EmType::Grt | EmType::Less
                | EmType::Equ | EmType::Nequ => {
                    let (a, b) = match self.pop_two_ints() {
                        Ok(values) => values,
                        Err(err) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(err),
                            }
                        }
                    };

                    let result = match em.em_type {
                        EmType::Add => a + b,
                        EmType::Sub => a - b,
                        EmType::Mul => a * b,
                        EmType::Div => {
                            if b == 0 {
                                return RuntimeResult {
                                    ex: self.ex as i64,
                                    em: Err(RuntimeError::DivByZero),
                                };
                            }
                            a / b
                        }
                        EmType::Grt => (a > b) as i64,
                        EmType::Less => (a < b) as i64,
                        EmType::Equ => (a == b) as i64,
                        EmType::Nequ => (a != b) as i64,
                        _ => unreachable!(),
                    };

                    stack::stack_push(&mut self.stack, Data::new_int(result));
                }
                EmType::PrintBegin => {
                    if self.ip == em.r#ref.saturating_sub(1) {
                        let data = match stack::stack_pop(&mut self.stack) {
                            Some(data) => data,
                            None => {
                                return RuntimeResult {
                                    ex: self.ex as i64,
                                    em: Err(RuntimeError::StackUnderflow),
                                }
                            }
                        };
                        self.print_data_line(&data, output_from_data(&prog.ems[em.r#ref].data));
                    } else {
                        self.print = true;
                        self.print_from = self.stack.size;
                    }
                }
                EmType::PrintEnd => {
                    if self.print && self.print_from != self.stack.size {
                        self.print = false;
                        let file = output_from_data(&em.data);
                        for (index, data) in self.stack.buf[self.print_from..self.stack.size]
                            .iter()
                            .enumerate()
                        {
                            if index > 0 {
                                match file {
                                    Output::Stdout => print!(" "),
                                    Output::Stderr => eprint!(" "),
                                }
                            }
                            self.print_data(data, file);
                        }
                        stack::stack_shrink_to(&mut self.stack, self.print_from);
                        match file {
                            Output::Stdout => println!(),
                            Output::Stderr => eprintln!(),
                        }
                    }
                }
                EmType::IfBegin => {
                    let cond = match self.pop_int() {
                        Ok(value) => value,
                        Err(err) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(err),
                            }
                        }
                    };
                    if cond == 0 {
                        self.ip = em.r#ref;
                    }
                }
                EmType::IfEnd => {}
                EmType::LoopBegin => {
                    let cond = match self.pop_int() {
                        Ok(value) => value,
                        Err(err) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(err),
                            }
                        }
                    };
                    if cond == 0 {
                        self.ip = em.r#ref;
                    }
                }
                EmType::LoopEnd => self.ip = em.r#ref.saturating_sub(1),
                EmType::Exit => {
                    let ex = match self.pop_int() {
                        Ok(value) => value,
                        Err(err) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(err),
                            }
                        }
                    };
                    self.ex = ex as usize;
                    self.halt = true;
                }
                EmType::Dup => {
                    let off = match self.pop_int() {
                        Ok(value) => value,
                        Err(err) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(err),
                            }
                        }
                    };
                    if stack::stack_dup(&mut self.stack, off as usize) != 0 {
                        return RuntimeResult {
                            ex: self.ex as i64,
                            em: Err(RuntimeError::InvalidAccess),
                        };
                    }
                }
                EmType::Swap => {
                    let off = match self.pop_int() {
                        Ok(value) => value,
                        Err(err) => {
                            return RuntimeResult {
                                ex: self.ex as i64,
                                em: Err(err),
                            }
                        }
                    };
                    if stack::stack_swap(&mut self.stack, off as usize) != 0 {
                        return RuntimeResult {
                            ex: self.ex as i64,
                            em: Err(RuntimeError::InvalidAccess),
                        };
                    }
                }
                #[cfg(debug_assertions)]
                EmType::Debug => {
                    for (i, data) in self.stack.buf.iter().enumerate() {
                        println!("stack[{i}]: {data}");
                    }
                }
            }

            self.tick += 1;
            if self.tick % GC_FREQUENCY_IN_TICKS == 0 {
                stack::stack_gc(&mut self.stack);
            }
            self.ip += 1;
        }

        stack::stack_clear(&mut self.stack);
        self.gc();
        RuntimeResult {
            ex: self.ex as i64,
            em: Ok(Em::new(EmType::Exit)),
        }
    }
    pub fn gc(&mut self) {
        stack::stack_gc(&mut self.stack);
    }
}

#[derive(Clone, Copy)]
enum Output {
    Stdout,
    Stderr,
}

fn output_from_data(data: &Data) -> Output {
    match data.value {
        DataValue::Int(value) if value == DATA_STDOUT as i64 => Output::Stdout,
        _ => Output::Stderr,
    }
}

impl<'a> Env<'a> {
    fn pop_int(&mut self) -> Result<i64, RuntimeError> {
        match stack::stack_pop(&mut self.stack) {
            Some(Data {
                dtype: DataType::Int,
                value: DataValue::Int(value),
            }) => Ok(value),
            Some(_) => Err(RuntimeError::IncorrectType),
            None => Err(RuntimeError::StackUnderflow),
        }
    }

    fn pop_two_ints(&mut self) -> Result<(i64, i64), RuntimeError> {
        let b = self.pop_int()?;
        let a = self.pop_int()?;
        Ok((a, b))
    }

    fn print_data_line(&self, data: &Data, file: Output) {
        self.print_data(data, file);
        match file {
            Output::Stdout => println!(),
            Output::Stderr => eprintln!(),
        }
    }

    fn print_data(&self, data: &Data, file: Output) {
        match file {
            Output::Stdout => print!("{data}"),
            Output::Stderr => eprint!("{data}"),
        }
    }
}

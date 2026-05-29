use crate::data;
use core::fmt;
pub const DEFAULT_PROGRAM_CAP: usize = 256;
pub const DATA_STDOUT: i32 = 1;
pub const DATA_STDERR: i32 = 2;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmType {
    Push,
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Grt,
    Less,
    Equ,
    Nequ,
    PrintBegin,
    PrintEnd,
    IfBegin,
    IfEnd,
    LoopBegin,
    LoopEnd,
    Exit,
    Dup,
    Swap,
    #[cfg(debug_assertions)]
    Debug,
}
impl fmt::Display for EmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EmType::Push => "push",
            EmType::Pop => "pop",
            EmType::Add => "add",
            EmType::Sub => "sub",
            EmType::Mul => "mul",
            EmType::Div => "div",
            EmType::Grt => "grt",
            EmType::Less => "less",
            EmType::Equ => "equ",
            EmType::Nequ => "nequ",
            EmType::PrintBegin => "print_begin",
            EmType::PrintEnd => "print_end",
            EmType::IfBegin => "if_begin",
            EmType::IfEnd => "if_end",
            EmType::LoopBegin => "loop_begin",
            EmType::LoopEnd => "loop_end",
            EmType::Exit => "exit",
            EmType::Dup => "dup",
            EmType::Swap => "swap",
            #[cfg(debug_assertions)]
            EmType::Debug => "debug",
        };
        write!(f, "{}", s)
    }
}
#[derive(Debug, Clone)]
pub struct Em {
    pub data: data::Data,
    pub em_type: EmType,
    pub path: String,
    pub row: usize,
    pub col: usize,
    pub r#ref: usize,
    pub ran: bool,
}
impl fmt::Display for Em {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}", self.em_type)?;
        match self.em_type {
            EmType::Push => {
                write!(f, " ")?;
                write!(f, "{}", self.data)?;
            }
            EmType::PrintEnd => {
                let which = match &self.data.value {
                    data::DataValue::Int(i) => *i,
                    _ => 0,
                };
                if which == DATA_STDOUT as i64 {
                    write!(f, " stdout")?;
                } else {
                    write!(f, " stderr")?;
                }
            }
            EmType::PrintBegin | EmType::IfBegin => {
                write!(f, " ref: {}", self.r#ref)?;
            }
            _ => {}
        }
        write!(f, " {}:{}:{}>\n", self.path, self.row, self.col)
    }
}
impl Em {
    pub fn new(typ: EmType) -> Self {
        Em {
            data: data::Data::new(data::DataType::Int),
            em_type: typ,
            path: String::new(),
            row: 0,
            col: 0,
            r#ref: 0,
            ran: false,
        }
    }
    pub fn new_with_data(typ: EmType, data: data::Data) -> Self {
        Em {
            data,
            em_type: typ,
            path: String::new(),
            row: 0,
            col: 0,
            r#ref: 0,
            ran: false,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Program {
    pub ems: Vec<Em>,
    pub cap: usize,
    pub size: usize,
}
impl Program {
    pub fn new(cap: usize) -> Self {
        assert!(cap > 0);
        Program {
            ems: Vec::with_capacity(cap),
            cap,
            size: 0,
        }
    }
    pub fn push(&mut self, em: Em) {
        if self.size >= self.cap {
            self.cap *= 2;
        }
        self.ems.push(em);
        self.size += 1;
    }
}

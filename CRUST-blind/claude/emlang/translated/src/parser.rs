use core::fmt;
use std::fs;
use crate::{data, em};
pub const PARSER_MAX_TOKEN_LENGTH: usize = 1024;
pub const PARSER_MAX_NESTS: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParserError {
    UnexpectedEscape,
    UnknownEscape,
    UnterminatedQuotes,
    UnexpectedEnd,
    IllegalPrintNest,
    ExpectedEnd,
}
impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ParserError::UnexpectedEscape => "Unexpected escape",
            ParserError::UnknownEscape => "Unknown escape",
            ParserError::UnterminatedQuotes => "Unterminated quotes",
            ParserError::UnexpectedEnd => "Unexpected end",
            ParserError::IllegalPrintNest => "Illegal print nesting",
            ParserError::ExpectedEnd => "Expected matching end",
        };
        write!(f, "{}", s)
    }
}
#[derive(Debug)]
pub struct ParserResult {
    pub path: String,
    pub row: usize,
    pub col: usize,
    pub prog: Result<em::Program, ParserError>,
}
#[derive(Debug)]
pub struct Parser {
    pub path: String,
    pub row: usize,
    pub col: usize,
    pub from_file: bool,
    pub input: String,
    pub ch: i32,
    pub pos: usize,
    pub tok: String,
    pub tok_len: usize,
    pub prog: em::Program,
}

fn em_to_keyword(t: em::EmType) -> Option<&'static str> {
    use em::EmType::*;
    match t {
        Push => None,
        Pop => Some(":P"),
        Add => Some(";)"),
        Sub => Some(";("),
        Mul => Some("x)"),
        Div => Some("x("),
        Grt => Some(":>"),
        Less => Some(":<"),
        Equ => Some(":|"),
        Nequ => Some("x|"),
        PrintBegin => Some(":O"),
        PrintEnd => None,
        IfBegin => Some(":/"),
        IfEnd => Some(":\\"),
        LoopBegin => Some(":@"),
        LoopEnd => Some("@:"),
        Exit => Some("X_X"),
        Dup => Some(":D"),
        Swap => Some(":S"),
        #[cfg(debug_assertions)]
        Debug => Some("D:"),
    }
}

const ALL_EM_TYPES: &[em::EmType] = &[
    em::EmType::Push,
    em::EmType::Pop,
    em::EmType::Add,
    em::EmType::Sub,
    em::EmType::Mul,
    em::EmType::Div,
    em::EmType::Grt,
    em::EmType::Less,
    em::EmType::Equ,
    em::EmType::Nequ,
    em::EmType::PrintBegin,
    em::EmType::PrintEnd,
    em::EmType::IfBegin,
    em::EmType::IfEnd,
    em::EmType::LoopBegin,
    em::EmType::LoopEnd,
    em::EmType::Exit,
    em::EmType::Dup,
    em::EmType::Swap,
    #[cfg(debug_assertions)]
    em::EmType::Debug,
];

fn is_space(ch: i32) -> bool {
    if ch == 0 {
        return false;
    }
    matches!(ch as u8 as char, ' ' | '\t' | '\n' | '\r' | '\x0c' | '\x0b')
}

fn is_digit(ch: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let c = ch as u8 as char;
    c.is_ascii_digit()
}

fn ok_result(prog: em::Program) -> ParserResult {
    ParserResult {
        path: String::new(),
        row: 0,
        col: 0,
        prog: Ok(prog),
    }
}

fn err_result(err: ParserError, path: &str, row: usize, col: usize) -> ParserResult {
    ParserResult {
        path: path.to_string(),
        row,
        col,
        prog: Err(err),
    }
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            path: String::new(),
            row: 1,
            col: 0,
            from_file: false,
            input: String::new(),
            ch: 0,
            pos: 0,
            tok: String::new(),
            tok_len: 0,
            prog: em::Program::new(em::DEFAULT_PROGRAM_CAP),
        }
    }
    pub fn load_mem(&mut self, input: &str) {
        self.input = input.to_string();
    }
    pub fn load_file(&mut self, path: &str) -> i32 {
        match fs::read_to_string(path) {
            Ok(content) => {
                self.from_file = true;
                self.path = path.to_string();
                self.input = content;
                0
            }
            Err(_) => -1,
        }
    }
    pub fn parse(&mut self) -> ParserResult {
        self.advance();
        if self.is_end() {
            return ParserResult {
                path: self.path.clone(),
                row: self.row,
                col: self.col,
                prog: Ok(std::mem::replace(
                    &mut self.prog,
                    em::Program::new(em::DEFAULT_PROGRAM_CAP),
                )),
            };
        }

        loop {
            let result = self.parse_next();
            if result.prog.is_err() {
                return result;
            }
            if self.is_end() {
                break;
            }
        }

        let cross_result = self.cross_ref();
        if cross_result.prog.is_err() {
            return cross_result;
        }

        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(std::mem::replace(
                &mut self.prog,
                em::Program::new(em::DEFAULT_PROGRAM_CAP),
            )),
        }
    }
    pub fn cross_ref(&mut self) -> ParserResult {
        let mut expects: Vec<em::EmType> = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut begins: Vec<usize> = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut print = false;

        for i in 0..self.prog.ems.len() {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                em::EmType::PrintBegin => {
                    if print {
                        let em = &self.prog.ems[i];
                        return err_result(
                            ParserError::IllegalPrintNest,
                            &em.path,
                            em.row,
                            em.col,
                        );
                    }
                    print = true;
                    let end_type = match em_type {
                        em::EmType::PrintBegin => em::EmType::PrintEnd,
                        em::EmType::IfBegin => em::EmType::IfEnd,
                        em::EmType::LoopBegin => em::EmType::LoopEnd,
                        _ => em_type,
                    };
                    expects.push(end_type);
                    begins.push(i);
                }
                em::EmType::IfBegin | em::EmType::LoopBegin => {
                    let end_type = match em_type {
                        em::EmType::IfBegin => em::EmType::IfEnd,
                        em::EmType::LoopBegin => em::EmType::LoopEnd,
                        _ => em_type,
                    };
                    expects.push(end_type);
                    begins.push(i);
                }
                em::EmType::PrintEnd => {
                    print = false;
                    if expects.is_empty() {
                        let em = &self.prog.ems[i];
                        return err_result(
                            ParserError::UnexpectedEnd,
                            &em.path,
                            em.row,
                            em.col,
                        );
                    }
                    if *expects.last().unwrap() != em_type {
                        let em = &self.prog.ems[i];
                        return err_result(
                            ParserError::UnexpectedEnd,
                            &em.path,
                            em.row,
                            em.col,
                        );
                    }
                    let begin = begins.pop().unwrap();
                    expects.pop();
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                em::EmType::IfEnd | em::EmType::LoopEnd => {
                    if expects.is_empty() {
                        let em = &self.prog.ems[i];
                        return err_result(
                            ParserError::UnexpectedEnd,
                            &em.path,
                            em.row,
                            em.col,
                        );
                    }
                    if *expects.last().unwrap() != em_type {
                        let em = &self.prog.ems[i];
                        return err_result(
                            ParserError::UnexpectedEnd,
                            &em.path,
                            em.row,
                            em.col,
                        );
                    }
                    let begin = begins.pop().unwrap();
                    expects.pop();
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                _ => {}
            }
        }

        if !begins.is_empty() {
            let begin = begins[begins.len() - 1];
            let em = &self.prog.ems[begin];
            return err_result(ParserError::ExpectedEnd, &em.path, em.row, em.col);
        }

        ok_result(em::Program::new(em::DEFAULT_PROGRAM_CAP))
    }
    pub fn advance(&mut self) {
        if self.ch == '\n' as i32 {
            self.row += 1;
            self.col = 0;
        }
        let bytes = self.input.as_bytes();
        if self.pos >= bytes.len() {
            self.ch = 0;
            self.pos += 1;
            return;
        }
        self.ch = bytes[self.pos] as i32;
        self.pos += 1;
        if self.ch == 0 {
            return;
        }
        self.col += 1;
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == '\\' as i32 {
            self.advance();
            if self.is_end() || is_space(self.ch) {
                return err_result(
                    ParserError::UnexpectedEscape,
                    &self.path,
                    start_row,
                    start_col,
                );
            } else if self.ch != '"' as i32 {
                self.tok_add('\\');
            }
        }

        let mut is_int = true;
        loop {
            if is_int && !(self.tok_len == 0 && self.ch == '-' as i32) {
                if !is_digit(self.ch) {
                    is_int = false;
                }
            }
            self.tok_add(self.ch as u8 as char);
            self.advance();
            if self.is_end() {
                break;
            }
            if is_space(self.ch) {
                break;
            }
        }

        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }

        let tok_str = self.tok.clone();

        let mut em_opt: Option<em::Em> = None;

        for &t in ALL_EM_TYPES {
            if let Some(kw) = em_to_keyword(t) {
                if tok_str == kw {
                    em_opt = Some(em::Em::new(t));
                    break;
                }
            }
        }

        if em_opt.is_none() {
            if tok_str == ":x" {
                while !self.is_end() && self.ch != '\n' as i32 {
                    self.advance();
                }
                return ok_result(em::Program::new(em::DEFAULT_PROGRAM_CAP));
            } else if tok_str == ":)" {
                em_opt = Some(em::Em::new_with_data(
                    em::EmType::PrintEnd,
                    data::Data::new_int(em::DATA_STDOUT as i64),
                ));
            } else if tok_str == ":(" {
                em_opt = Some(em::Em::new_with_data(
                    em::EmType::PrintEnd,
                    data::Data::new_int(em::DATA_STDERR as i64),
                ));
            } else if tok_str == ":3"
                || tok_str == ";3"
                || tok_str == "<3"
                || tok_str == "x3"
                || tok_str == "><>"
            {
                let text = match tok_str.as_bytes()[0] {
                    b':' => "meow",
                    b';' => "nya",
                    b'x' => "rawr",
                    b'>' => "le fishe",
                    b'<' => "i <3 emlang",
                    _ => unreachable!(),
                };
                em_opt = Some(em::Em::new_with_data(
                    em::EmType::Push,
                    data::Data::new_str(text.to_string()),
                ));
            } else if is_int {
                let n: i64 = tok_str.parse().unwrap_or(0);
                em_opt = Some(em::Em::new_with_data(
                    em::EmType::Push,
                    data::Data::new_int(n),
                ));
            } else {
                em_opt = Some(em::Em::new_with_data(
                    em::EmType::Push,
                    data::Data::new_str(tok_str.clone()),
                ));
            }
        }

        if let Some(mut em) = em_opt {
            em.row = start_row;
            em.col = start_col;
            em.path = self.path.clone();
            self.prog.push(em);
        }

        ok_result(em::Program::new(em::DEFAULT_PROGRAM_CAP))
    }
    pub fn parse_quotes(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;

        let mut escape = false;
        loop {
            self.advance();
            if self.is_end() || self.ch == '\n' as i32 {
                return err_result(
                    ParserError::UnterminatedQuotes,
                    &self.path,
                    start_row,
                    start_col,
                );
            }
            if escape {
                let c = match self.ch as u8 as char {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'f' => '\x0c',
                    'v' => '\x0b',
                    'b' => '\x08',
                    'a' => '\x07',
                    '"' => '"',
                    'e' => 27 as char,
                    '\\' => '\\',
                    _ => {
                        return err_result(
                            ParserError::UnknownEscape,
                            &self.path,
                            self.row,
                            self.col,
                        );
                    }
                };
                self.tok_add(c);
                escape = false;
            } else if self.ch == '\\' as i32 {
                escape = true;
            } else if self.ch == '"' as i32 {
                break;
            } else {
                self.tok_add(self.ch as u8 as char);
            }
        }
        self.advance();

        let mut em = em::Em::new_with_data(
            em::EmType::Push,
            data::Data::new_str(self.tok.clone()),
        );
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        ok_result(em::Program::new(em::DEFAULT_PROGRAM_CAP))
    }
    pub fn parse_next(&mut self) -> ParserResult {
        while is_space(self.ch) {
            self.advance();
            if self.is_end() {
                return ok_result(em::Program::new(em::DEFAULT_PROGRAM_CAP));
            }
        }
        if self.ch == '"' as i32 {
            self.parse_quotes()
        } else {
            self.parse_plain()
        }
    }
}

impl Parser {
    fn is_end(&self) -> bool {
        self.ch == 0
    }
    fn tok_add(&mut self, c: char) {
        assert!(self.tok_len < PARSER_MAX_TOKEN_LENGTH);
        self.tok.push(c);
        self.tok_len += 1;
    }
}

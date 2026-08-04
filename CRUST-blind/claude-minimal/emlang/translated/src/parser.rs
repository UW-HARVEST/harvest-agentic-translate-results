use core::fmt;
use std::fs;
use crate::data;
use crate::em::{self, Em, EmType};
use crate::utils;
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
            tok: String::with_capacity(PARSER_MAX_TOKEN_LENGTH),
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

    fn is_end(&self) -> bool {
        self.ch == 0
    }

    pub fn advance(&mut self) {
        if self.ch == b'\n' as i32 {
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
        if self.is_end() {
            return;
        }
        self.col += 1;
    }

    fn tok_clear(&mut self) {
        self.tok.clear();
        self.tok_len = 0;
    }

    fn tok_add(&mut self, c: u8) {
        assert!(self.tok_len < PARSER_MAX_TOKEN_LENGTH);
        self.tok.push(c as char);
        self.tok_len += 1;
    }

    pub fn parse_quotes(&mut self) -> ParserResult {
        self.tok_clear();
        let start_row = self.row;
        let start_col = self.col;

        let mut escape = false;
        loop {
            self.advance();
            if self.is_end() || self.ch == b'\n' as i32 {
                return ParserResult {
                    path: self.path.clone(),
                    row: start_row,
                    col: start_col,
                    prog: Err(ParserError::UnterminatedQuotes),
                };
            }

            if escape {
                let c = self.ch as u8;
                let mapped = match c {
                    b'n' => Some(b'\n'),
                    b'r' => Some(b'\r'),
                    b't' => Some(b'\t'),
                    b'f' => Some(0x0C),
                    b'v' => Some(0x0B),
                    b'b' => Some(0x08),
                    b'a' => Some(0x07),
                    b'"' => Some(b'"'),
                    b'e' => Some(27u8),
                    b'\\' => Some(b'\\'),
                    _ => None,
                };
                match mapped {
                    Some(m) => self.tok_add(m),
                    None => {
                        return ParserResult {
                            path: self.path.clone(),
                            row: self.row,
                            col: self.col,
                            prog: Err(ParserError::UnknownEscape),
                        };
                    }
                }
                escape = false;
            } else if self.ch == b'\\' as i32 {
                escape = true;
            } else if self.ch == b'"' as i32 {
                break;
            } else {
                self.tok_add(self.ch as u8);
            }
        }
        self.advance();

        let s = utils::strcpy_to_heap(&self.tok);
        let mut em = Em::new_with_data(EmType::Push, data::Data::new_str(s));
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);

        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Ok(em::Program::new(1)), // placeholder ok
        }
    }

    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok_clear();
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == b'\\' as i32 {
            self.advance();
            if self.is_end() || (self.ch as u8 as char).is_whitespace() {
                return ParserResult {
                    path: self.path.clone(),
                    row: start_row,
                    col: start_col,
                    prog: Err(ParserError::UnexpectedEscape),
                };
            } else if self.ch != b'"' as i32 {
                self.tok_add(b'\\');
            }
        }

        let mut is_int = true;
        loop {
            if is_int && !(self.tok_len == 0 && self.ch == b'-' as i32) {
                let c = self.ch as u8;
                if !(c as char).is_ascii_digit() {
                    is_int = false;
                }
            }

            self.tok_add(self.ch as u8);

            self.advance();
            if self.is_end() {
                break;
            }
            if (self.ch as u8 as char).is_whitespace() {
                break;
            }
        }

        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }

        // Match keywords
        let keyword_map: &[(EmType, &str)] = &[
            (EmType::Pop, ":P"),
            (EmType::Add, ";)"),
            (EmType::Sub, ";("),
            (EmType::Mul, "x)"),
            (EmType::Div, "x("),
            (EmType::Grt, ":>"),
            (EmType::Less, ":<"),
            (EmType::Equ, ":|"),
            (EmType::Nequ, "x|"),
            (EmType::PrintBegin, ":O"),
            (EmType::IfBegin, ":/"),
            (EmType::IfEnd, ":\\"),
            (EmType::LoopBegin, ":@"),
            (EmType::LoopEnd, "@:"),
            (EmType::Exit, "X_X"),
            (EmType::Dup, ":D"),
            (EmType::Swap, ":S"),
            #[cfg(debug_assertions)]
            (EmType::Debug, "D:"),
        ];

        let mut em_opt: Option<Em> = None;
        for (et, kw) in keyword_map {
            if self.tok == *kw {
                em_opt = Some(Em::new(*et));
                break;
            }
        }

        if em_opt.is_none() {
            if self.tok == ":x" {
                while !self.is_end() && self.ch != b'\n' as i32 {
                    self.advance();
                }
                return ParserResult {
                    path: self.path.clone(),
                    row: 0,
                    col: 0,
                    prog: Ok(em::Program::new(1)),
                };
            } else if self.tok == ":)" {
                em_opt = Some(Em::new_with_data(
                    EmType::PrintEnd,
                    data::Data::new_int(em::DATA_STDOUT as i64),
                ));
            } else if self.tok == ":(" {
                em_opt = Some(Em::new_with_data(
                    EmType::PrintEnd,
                    data::Data::new_int(em::DATA_STDERR as i64),
                ));
            } else if self.tok == ":3"
                || self.tok == ";3"
                || self.tok == "<3"
                || self.tok == "x3"
                || self.tok == "><>"
            {
                let first = self.tok.as_bytes()[0];
                let text = match first {
                    b':' => "meow",
                    b';' => "nya",
                    b'x' => "rawr",
                    b'>' => "le fishe",
                    b'<' => "i <3 emlang",
                    _ => unreachable!(),
                };
                let s = utils::strcpy_to_heap(text);
                em_opt = Some(Em::new_with_data(EmType::Push, data::Data::new_str(s)));
            } else if is_int {
                let parsed: i64 = self.tok.parse().unwrap_or(0);
                em_opt = Some(Em::new_with_data(EmType::Push, data::Data::new_int(parsed)));
            } else {
                let s = utils::strcpy_to_heap(&self.tok);
                em_opt = Some(Em::new_with_data(EmType::Push, data::Data::new_str(s)));
            }
        }

        let mut em = em_opt.unwrap();
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);

        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Ok(em::Program::new(1)),
        }
    }

    pub fn parse_next(&mut self) -> ParserResult {
        while (self.ch as u8 as char).is_whitespace() && self.ch != 0 {
            self.advance();
            if self.is_end() {
                return ParserResult {
                    path: self.path.clone(),
                    row: 0,
                    col: 0,
                    prog: Ok(em::Program::new(1)),
                };
            }
        }

        if self.ch == b'"' as i32 {
            self.parse_quotes()
        } else {
            self.parse_plain()
        }
    }

    pub fn cross_ref(&mut self) -> ParserResult {
        let mut expects: Vec<EmType> = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut begins: Vec<usize> = Vec::with_capacity(PARSER_MAX_NESTS);

        let mut print = false;
        for i in 0..self.prog.size {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                EmType::PrintBegin => {
                    if print {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::IllegalPrintNest),
                        };
                    }
                    print = true;
                    let end = match em_type {
                        EmType::PrintBegin => EmType::PrintEnd,
                        EmType::IfBegin => EmType::IfEnd,
                        EmType::LoopBegin => EmType::LoopEnd,
                        _ => unreachable!(),
                    };
                    expects.push(end);
                    begins.push(i);
                }
                EmType::IfBegin | EmType::LoopBegin => {
                    let end = match em_type {
                        EmType::IfBegin => EmType::IfEnd,
                        EmType::LoopBegin => EmType::LoopEnd,
                        _ => unreachable!(),
                    };
                    expects.push(end);
                    begins.push(i);
                }
                EmType::PrintEnd => {
                    print = false;
                    if expects.is_empty() {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    let exp = *expects.last().unwrap();
                    if em_type != exp {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    expects.pop();
                    let begin = begins.pop().unwrap();
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                EmType::IfEnd | EmType::LoopEnd => {
                    if expects.is_empty() {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    let exp = *expects.last().unwrap();
                    if em_type != exp {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    expects.pop();
                    let begin = begins.pop().unwrap();
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                _ => {}
            }
        }

        if !expects.is_empty() {
            let begin = begins[expects.len() - 1];
            let em = &self.prog.ems[begin];
            return ParserResult {
                path: em.path.clone(),
                row: em.row,
                col: em.col,
                prog: Err(ParserError::ExpectedEnd),
            };
        }

        ParserResult {
            path: String::new(),
            row: 0,
            col: 0,
            prog: Ok(em::Program::new(1)),
        }
    }

    pub fn parse(&mut self) -> ParserResult {
        self.advance();
        if self.is_end() {
            return ParserResult {
                path: self.path.clone(),
                row: 0,
                col: 0,
                prog: Ok(self.prog.clone()),
            };
        }

        let mut result;
        loop {
            result = self.parse_next();
            if result.prog.is_err() {
                return result;
            }
            if self.is_end() {
                break;
            }
        }

        let result2 = self.cross_ref();
        if result2.prog.is_err() {
            return result2;
        }

        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Ok(self.prog.clone()),
        }
    }
}

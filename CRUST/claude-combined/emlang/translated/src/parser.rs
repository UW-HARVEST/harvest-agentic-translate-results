use core::fmt;
use std::fs;
use crate::data::{Data, DataType};
use crate::em::{self, Em, EmType, Program};
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
            tok: String::new(),
            tok_len: 0,
            prog: Program::new(em::DEFAULT_PROGRAM_CAP),
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
                prog: Ok(self.prog.clone()),
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
        let result = self.cross_ref();
        if result.prog.is_err() {
            return result;
        }
        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
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
                    expects.push(EmType::PrintEnd);
                    begins.push(i);
                }
                EmType::IfBegin => {
                    expects.push(EmType::IfEnd);
                    begins.push(i);
                }
                EmType::LoopBegin => {
                    expects.push(EmType::LoopEnd);
                    begins.push(i);
                }
                EmType::PrintEnd | EmType::IfEnd | EmType::LoopEnd => {
                    if em_type == EmType::PrintEnd {
                        print = false;
                    }
                    if expects.is_empty() {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    let last_expect = *expects.last().unwrap();
                    if em_type != last_expect {
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
            let begin_idx = *begins.last().unwrap();
            let em = &self.prog.ems[begin_idx];
            return ParserResult {
                path: em.path.clone(),
                row: em.row,
                col: em.col,
                prog: Err(ParserError::ExpectedEnd),
            };
        }
        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
        }
    }
    pub fn advance(&mut self) {
        if self.ch == b'\n' as i32 {
            self.row += 1;
            self.col = 0;
        }
        let bytes = self.input.as_bytes();
        if self.pos < bytes.len() {
            self.ch = bytes[self.pos] as i32;
            self.pos += 1;
        } else {
            self.ch = 0;
            self.pos += 1;
            return;
        }
        if self.is_end() {
            return;
        }
        self.col += 1;
    }
    fn is_end(&self) -> bool {
        self.ch == 0
    }
    fn tok_clear(&mut self) {
        self.tok.clear();
        self.tok_len = 0;
    }
    fn tok_add(&mut self, ch: char) {
        assert!(self.tok_len < PARSER_MAX_TOKEN_LENGTH);
        self.tok.push(ch);
        self.tok_len += 1;
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok_clear();
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == b'\\' as i32 {
            self.advance();
            if self.is_end() || is_space(self.ch) {
                return ParserResult {
                    path: self.path.clone(),
                    row: start_row,
                    col: start_col,
                    prog: Err(ParserError::UnexpectedEscape),
                };
            } else if self.ch != b'"' as i32 {
                self.tok_add('\\');
            }
        }

        let mut is_int = true;
        loop {
            if is_int && !(self.tok_len == 0 && self.ch == b'-' as i32) {
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

        // Try to match keywords
        let keyword_map: &[(&str, EmType)] = &[
            (":P", EmType::Pop),
            (";)", EmType::Add),
            (";(", EmType::Sub),
            ("x)", EmType::Mul),
            ("x(", EmType::Div),
            (":>", EmType::Grt),
            (":<", EmType::Less),
            (":|", EmType::Equ),
            ("x|", EmType::Nequ),
            (":O", EmType::PrintBegin),
            (":/", EmType::IfBegin),
            (":\\", EmType::IfEnd),
            (":@", EmType::LoopBegin),
            ("@:", EmType::LoopEnd),
            ("X_X", EmType::Exit),
            (":D", EmType::Dup),
            (":S", EmType::Swap),
            #[cfg(debug_assertions)]
            ("D:", EmType::Debug),
        ];

        let mut em: Option<Em> = None;
        for (kw, ty) in keyword_map {
            if self.tok == *kw {
                em = Some(Em::new(*ty));
                break;
            }
        }

        if em.is_none() {
            if self.tok == ":x" {
                while !self.is_end() && self.ch != b'\n' as i32 {
                    self.advance();
                }
                return ParserResult {
                    path: self.path.clone(),
                    row: self.row,
                    col: self.col,
                    prog: Ok(self.prog.clone()),
                };
            } else if self.tok == ":)" {
                em = Some(Em::new_with_data(
                    EmType::PrintEnd,
                    Data::new_int(em::DATA_STDOUT as i64),
                ));
            } else if self.tok == ":(" {
                em = Some(Em::new_with_data(
                    EmType::PrintEnd,
                    Data::new_int(em::DATA_STDERR as i64),
                ));
            } else if self.tok == ":3"
                || self.tok == ";3"
                || self.tok == "<3"
                || self.tok == "x3"
                || self.tok == "><>"
            {
                let text = match self.tok.as_bytes()[0] {
                    b':' => "meow",
                    b';' => "nya",
                    b'x' => "rawr",
                    b'>' => "le fishe",
                    b'<' => "i <3 emlang",
                    _ => unreachable!(),
                };
                em = Some(Em::new_with_data(
                    EmType::Push,
                    Data::new_str(text.to_string()),
                ));
            } else if is_int {
                let val: i64 = self.tok.parse().unwrap_or(0);
                em = Some(Em::new_with_data(
                    EmType::Push,
                    Data::new_int(val),
                ));
            } else {
                em = Some(Em::new_with_data(
                    EmType::Push,
                    Data::new_str(self.tok.clone()),
                ));
            }
        }

        let mut em = em.unwrap();
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
        }
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
                let c = match self.ch as u8 {
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'f' => '\x0C',
                    b'v' => '\x0B',
                    b'b' => '\x08',
                    b'a' => '\x07',
                    b'"' => '"',
                    b'e' => 27 as char,
                    b'\\' => '\\',
                    _ => {
                        return ParserResult {
                            path: self.path.clone(),
                            row: self.row,
                            col: self.col,
                            prog: Err(ParserError::UnknownEscape),
                        };
                    }
                };
                self.tok_add(c);
                escape = false;
            } else if self.ch == b'\\' as i32 {
                escape = true;
            } else if self.ch == b'"' as i32 {
                break;
            } else {
                self.tok_add(self.ch as u8 as char);
            }
        }
        self.advance();
        let s = self.tok.clone();
        let mut em = Em::new_with_data(EmType::Push, Data::new_str(s));
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
        }
    }
    pub fn parse_next(&mut self) -> ParserResult {
        while is_space(self.ch) {
            self.advance();
            if self.is_end() {
                return ParserResult {
                    path: self.path.clone(),
                    row: self.row,
                    col: self.col,
                    prog: Ok(self.prog.clone()),
                };
            }
        }
        if self.ch == b'"' as i32 {
            self.parse_quotes()
        } else {
            self.parse_plain()
        }
    }
}

fn is_space(ch: i32) -> bool {
    matches!(ch as u8, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn is_digit(ch: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&ch)
}

// Suppress unused warnings
#[allow(dead_code)]
const _: () = {
    let _ = DataType::Int;
};

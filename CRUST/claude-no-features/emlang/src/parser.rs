use core::fmt;
use std::fs;
use crate::em::{self, Em, EmType, Program, DATA_STDERR, DATA_STDOUT, DEFAULT_PROGRAM_CAP};
use crate::data;
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
            prog: Program::new(DEFAULT_PROGRAM_CAP),
        }
    }
    pub fn load_mem(&mut self, input: &str) {
        self.input = input.to_string();
    }
    pub fn load_file(&mut self, path: &str) -> i32 {
        self.from_file = true;
        self.path = path.to_string();
        match fs::read_to_string(path) {
            Ok(content) => {
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
            let res = self.parse_next();
            if let Err(e) = res.prog {
                return ParserResult {
                    path: res.path,
                    row: res.row,
                    col: res.col,
                    prog: Err(e),
                };
            }
            if self.is_end() {
                break;
            }
        }

        let res = self.cross_ref_internal();
        if let Err((err, path, row, col)) = res {
            return ParserResult {
                path,
                row,
                col,
                prog: Err(err),
            };
        }

        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
        }
    }
    pub fn cross_ref(&mut self) -> ParserResult {
        match self.cross_ref_internal() {
            Ok(()) => ParserResult {
                path: self.path.clone(),
                row: self.row,
                col: self.col,
                prog: Ok(self.prog.clone()),
            },
            Err((err, path, row, col)) => ParserResult {
                path,
                row,
                col,
                prog: Err(err),
            },
        }
    }
    pub fn advance(&mut self) {
        if self.ch == b'\n' as i32 {
            self.row += 1;
            self.col = 0;
        }

        let bytes = self.input.as_bytes();
        if self.pos >= bytes.len() {
            self.ch = 0;
        } else {
            self.ch = bytes[self.pos] as i32;
            self.pos += 1;
        }
        if self.is_end() {
            return;
        }
        self.col += 1;
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        match self.parse_plain_internal() {
            Ok(()) => ParserResult {
                path: self.path.clone(),
                row: self.row,
                col: self.col,
                prog: Ok(self.prog.clone()),
            },
            Err((err, path, row, col)) => ParserResult {
                path,
                row,
                col,
                prog: Err(err),
            },
        }
    }
    pub fn parse_quotes(&mut self) -> ParserResult {
        match self.parse_quotes_internal() {
            Ok(()) => ParserResult {
                path: self.path.clone(),
                row: self.row,
                col: self.col,
                prog: Ok(self.prog.clone()),
            },
            Err((err, path, row, col)) => ParserResult {
                path,
                row,
                col,
                prog: Err(err),
            },
        }
    }
    pub fn parse_next(&mut self) -> ParserResult {
        while self.is_space() {
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

impl Parser {
    fn is_end(&self) -> bool {
        self.ch == 0
    }
    fn is_space(&self) -> bool {
        if self.ch <= 0 {
            return false;
        }
        let c = self.ch as u32;
        if c > 127 {
            return false;
        }
        let b = c as u8 as char;
        b.is_ascii_whitespace()
    }
    fn is_digit(&self) -> bool {
        if self.ch <= 0 {
            return false;
        }
        let c = self.ch as u32;
        if c > 127 {
            return false;
        }
        (c as u8 as char).is_ascii_digit()
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
    fn parse_quotes_internal(&mut self) -> Result<(), (ParserError, String, usize, usize)> {
        self.tok_clear();
        let start_row = self.row;
        let start_col = self.col;

        let mut escape = false;
        loop {
            self.advance();
            if self.is_end() || self.ch == b'\n' as i32 {
                return Err((
                    ParserError::UnterminatedQuotes,
                    self.path.clone(),
                    start_row,
                    start_col,
                ));
            }
            let c = self.ch as u8 as char;
            if escape {
                match c {
                    'n' => self.tok_add('\n'),
                    'r' => self.tok_add('\r'),
                    't' => self.tok_add('\t'),
                    'f' => self.tok_add('\x0c'),
                    'v' => self.tok_add('\x0b'),
                    'b' => self.tok_add('\x08'),
                    'a' => self.tok_add('\x07'),
                    '"' => self.tok_add('"'),
                    'e' => self.tok_add(27 as u8 as char),
                    '\\' => self.tok_add('\\'),
                    _ => {
                        return Err((
                            ParserError::UnknownEscape,
                            self.path.clone(),
                            self.row,
                            self.col,
                        ));
                    }
                }
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                break;
            } else {
                self.tok_add(c);
            }
        }
        self.advance();

        let str = self.tok.clone();
        let mut em = Em::new_with_data(EmType::Push, data::Data::new_str(str));
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        Ok(())
    }
    fn parse_plain_internal(&mut self) -> Result<(), (ParserError, String, usize, usize)> {
        self.tok_clear();
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == b'\\' as i32 {
            self.advance();
            if self.is_end() || self.is_space() {
                return Err((
                    ParserError::UnexpectedEscape,
                    self.path.clone(),
                    start_row,
                    start_col,
                ));
            } else if self.ch != b'"' as i32 {
                self.tok_add('\\');
            }
        }

        let mut is_int = true;
        loop {
            if is_int && !(self.tok_len == 0 && self.ch == b'-' as i32) {
                if !self.is_digit() {
                    is_int = false;
                }
            }

            let c = self.ch as u8 as char;
            self.tok_add(c);

            self.advance();
            if self.is_end() {
                break;
            }
            if self.is_space() {
                break;
            }
        }

        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }

        // Match keywords
        let keywords: &[(EmType, &str)] = &[
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
        for (typ, kw) in keywords {
            if self.tok == *kw {
                em_opt = Some(Em::new(*typ));
                break;
            }
        }

        if em_opt.is_none() {
            // Comment
            if self.tok == ":x" {
                while !self.is_end() && self.ch != b'\n' as i32 {
                    self.advance();
                }
                return Ok(());
            } else if self.tok == ":)" {
                em_opt = Some(Em::new_with_data(
                    EmType::PrintEnd,
                    data::Data::new_int(DATA_STDOUT as i64),
                ));
            } else if self.tok == ":(" {
                em_opt = Some(Em::new_with_data(
                    EmType::PrintEnd,
                    data::Data::new_int(DATA_STDERR as i64),
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
                em_opt = Some(Em::new_with_data(
                    EmType::Push,
                    data::Data::new_str(text.to_string()),
                ));
            } else if is_int {
                let val: i64 = self.tok.parse().unwrap_or(0);
                em_opt = Some(Em::new_with_data(EmType::Push, data::Data::new_int(val)));
            } else {
                em_opt = Some(Em::new_with_data(
                    EmType::Push,
                    data::Data::new_str(self.tok.clone()),
                ));
            }
        }

        let mut em = em_opt.unwrap();
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        Ok(())
    }
    fn cross_ref_internal(&mut self) -> Result<(), (ParserError, String, usize, usize)> {
        let mut expects: Vec<EmType> = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut begins: Vec<usize> = Vec::with_capacity(PARSER_MAX_NESTS);

        let mut print = false;
        for i in 0..self.prog.size {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                EmType::PrintBegin => {
                    if print {
                        let em = &self.prog.ems[i];
                        return Err((
                            ParserError::IllegalPrintNest,
                            em.path.clone(),
                            em.row,
                            em.col,
                        ));
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
                        return Err((
                            ParserError::UnexpectedEnd,
                            em.path.clone(),
                            em.row,
                            em.col,
                        ));
                    }
                    let expected = *expects.last().unwrap();
                    if em_type != expected {
                        let em = &self.prog.ems[i];
                        return Err((
                            ParserError::UnexpectedEnd,
                            em.path.clone(),
                            em.row,
                            em.col,
                        ));
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
            let begin = *begins.last().unwrap();
            let em = &self.prog.ems[begin];
            return Err((
                ParserError::ExpectedEnd,
                em.path.clone(),
                em.row,
                em.col,
            ));
        }

        Ok(())
    }
}

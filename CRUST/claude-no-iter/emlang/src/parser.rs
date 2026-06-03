use core::fmt;
use std::fs;
use crate::data::{Data, DataValue};
use crate::em::{self, Em, EmType, DATA_STDOUT, DATA_STDERR, DEFAULT_PROGRAM_CAP};
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

fn is_space_byte(b: i32) -> bool {
    matches!(b as u8, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn is_digit_byte(b: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&b)
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
            prog: em::Program::new(DEFAULT_PROGRAM_CAP),
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
        let prog_size = self.prog.size;
        for i in 0..prog_size {
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
            let begin = *begins.last().unwrap();
            let em = &self.prog.ems[begin];
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
        } else {
            self.ch = 0;
        }
        self.pos += 1;
        if self.is_end() {
            return;
        }
        self.col += 1;
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == b'\\' as i32 {
            self.advance();
            if self.is_end() || is_space_byte(self.ch) {
                return ParserResult {
                    path: self.path.clone(),
                    row: start_row,
                    col: start_col,
                    prog: Err(ParserError::UnexpectedEscape),
                };
            } else if self.ch != b'"' as i32 {
                self.tok.push('\\');
                self.tok_len += 1;
            }
        }

        let mut is_int = true;
        loop {
            if is_int && !(self.tok_len == 0 && self.ch == b'-' as i32) {
                if !is_digit_byte(self.ch) {
                    is_int = false;
                }
            }
            self.tok.push(self.ch as u8 as char);
            self.tok_len += 1;
            self.advance();
            if self.is_end() {
                // Process the token at end-of-input - same as below
                return self.process_plain_token(start_row, start_col, is_int);
            }
            if is_space_byte(self.ch) {
                break;
            }
        }

        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }

        self.process_plain_token(start_row, start_col, is_int)
    }
    pub fn parse_quotes(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
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
                let ch = match self.ch as u8 {
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'f' => '\u{000C}',
                    b'v' => '\u{000B}',
                    b'b' => '\u{0008}',
                    b'a' => '\u{0007}',
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
                self.tok.push(ch);
                self.tok_len += 1;
                escape = false;
            } else if self.ch == b'\\' as i32 {
                escape = true;
            } else if self.ch == b'"' as i32 {
                break;
            } else {
                self.tok.push(self.ch as u8 as char);
                self.tok_len += 1;
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
        while is_space_byte(self.ch) {
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

    fn keyword_to_em_type(tok: &str) -> Option<EmType> {
        match tok {
            ":P" => Some(EmType::Pop),
            ";)" => Some(EmType::Add),
            ";(" => Some(EmType::Sub),
            "x)" => Some(EmType::Mul),
            "x(" => Some(EmType::Div),
            ":>" => Some(EmType::Grt),
            ":<" => Some(EmType::Less),
            ":|" => Some(EmType::Equ),
            "x|" => Some(EmType::Nequ),
            ":O" => Some(EmType::PrintBegin),
            ":/" => Some(EmType::IfBegin),
            ":\\" => Some(EmType::IfEnd),
            ":@" => Some(EmType::LoopBegin),
            "@:" => Some(EmType::LoopEnd),
            "X_X" => Some(EmType::Exit),
            ":D" => Some(EmType::Dup),
            ":S" => Some(EmType::Swap),
            _ => None,
        }
    }

    fn process_plain_token(&mut self, start_row: usize, start_col: usize, is_int: bool) -> ParserResult {
        let tok = self.tok.clone();

        // Check keyword map
        let mut em: Option<Em> = None;
        if let Some(et) = Self::keyword_to_em_type(&tok) {
            em = Some(Em::new(et));
        }

        if em.is_none() {
            if tok == ":x" {
                // Comment: skip until newline
                while !self.is_end() && self.ch != b'\n' as i32 {
                    self.advance();
                }
                return ParserResult {
                    path: self.path.clone(),
                    row: self.row,
                    col: self.col,
                    prog: Ok(self.prog.clone()),
                };
            } else if tok == ":)" {
                em = Some(Em::new_with_data(EmType::PrintEnd, Data::new_int(DATA_STDOUT as i64)));
            } else if tok == ":(" {
                em = Some(Em::new_with_data(EmType::PrintEnd, Data::new_int(DATA_STDERR as i64)));
            } else if tok == ":3" || tok == ";3" || tok == "<3" || tok == "x3" || tok == "><>" {
                let text = match tok.as_bytes()[0] {
                    b':' => "meow",
                    b';' => "nya",
                    b'x' => "rawr",
                    b'>' => "le fishe",
                    b'<' => "i <3 emlang",
                    _ => unreachable!(),
                };
                em = Some(Em::new_with_data(EmType::Push, Data::new_str(text.to_string())));
            } else if is_int {
                let val: i64 = tok.parse().unwrap_or(0);
                em = Some(Em::new_with_data(EmType::Push, Data::new_int(val)));
            } else {
                em = Some(Em::new_with_data(EmType::Push, Data::new_str(tok.clone())));
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
}

// Suppress unused warning for DataValue import
#[allow(dead_code)]
fn _unused(_: DataValue) {}

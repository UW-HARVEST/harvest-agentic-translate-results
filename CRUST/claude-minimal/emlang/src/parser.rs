use core::fmt;
use std::fs;
use crate::{
    data,
    em::{self, Em, EmType, Program},
    utils,
};
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

fn is_space(ch: i32) -> bool {
    matches!(ch as u8 as char, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
}

fn is_digit(ch: i32) -> bool {
    let c = ch as u8 as char;
    c.is_ascii_digit()
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

        let mut result;
        loop {
            result = self.parse_next();
            if result.prog.is_err() || self.is_end() {
                break;
            }
        }

        if let Err(_) = result.prog {
            return result;
        }

        let result = self.cross_ref();
        if let Err(_) = result.prog {
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
                        return ParserResult {
                            path: self.prog.ems[i].path.clone(),
                            row: self.prog.ems[i].row,
                            col: self.prog.ems[i].col,
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
                EmType::PrintEnd => {
                    print = false;
                    if expects.is_empty() {
                        return ParserResult {
                            path: self.prog.ems[i].path.clone(),
                            row: self.prog.ems[i].row,
                            col: self.prog.ems[i].col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    let exp = *expects.last().unwrap();
                    if em_type != exp {
                        return ParserResult {
                            path: self.prog.ems[i].path.clone(),
                            row: self.prog.ems[i].row,
                            col: self.prog.ems[i].col,
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
                        return ParserResult {
                            path: self.prog.ems[i].path.clone(),
                            row: self.prog.ems[i].row,
                            col: self.prog.ems[i].col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    let exp = *expects.last().unwrap();
                    if em_type != exp {
                        return ParserResult {
                            path: self.prog.ems[i].path.clone(),
                            row: self.prog.ems[i].row,
                            col: self.prog.ems[i].col,
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

        if !begins.is_empty() {
            let i = *begins.last().unwrap();
            return ParserResult {
                path: self.prog.ems[i].path.clone(),
                row: self.prog.ems[i].row,
                col: self.prog.ems[i].col,
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
        if self.ch == '\n' as i32 {
            self.row += 1;
            self.col = 0;
        }

        if self.pos >= self.input.len() {
            self.ch = 0;
            self.pos += 1;
            return;
        }

        let bytes = self.input.as_bytes();
        self.ch = bytes[self.pos] as i32;
        self.pos += 1;

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

        if self.ch == '\\' as i32 {
            self.advance();
            if self.is_end() || is_space(self.ch) {
                return ParserResult {
                    path: self.path.clone(),
                    row: start_row,
                    col: start_col,
                    prog: Err(ParserError::UnexpectedEscape),
                };
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
                // Build em from tok and push, similar to below
                return self.finalize_plain_token(is_int, start_row, start_col);
            }
            if is_space(self.ch) {
                break;
            }
        }

        self.finalize_plain_token(is_int, start_row, start_col)
    }

    fn finalize_plain_token(&mut self, mut is_int: bool, start_row: usize, start_col: usize) -> ParserResult {
        if self.tok_len == 1 && self.tok.starts_with('-') {
            is_int = false;
        }

        let tok = self.tok.clone();

        // Check em_to_keyword_map
        let keyword_em = match tok.as_str() {
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
        };

        let mut em_obj = if let Some(et) = keyword_em {
            Em::new(et)
        } else if tok == ":x" {
            // Comment - skip until newline
            while !self.is_end() && self.ch != '\n' as i32 {
                self.advance();
            }
            return ParserResult {
                path: self.path.clone(),
                row: self.row,
                col: self.col,
                prog: Ok(self.prog.clone()),
            };
        } else if tok == ":)" {
            Em::new_with_data(EmType::PrintEnd, data::Data::new_int(em::DATA_STDOUT as i64))
        } else if tok == ":(" {
            Em::new_with_data(EmType::PrintEnd, data::Data::new_int(em::DATA_STDERR as i64))
        } else if tok == ":3" || tok == ";3" || tok == "<3" || tok == "x3" || tok == "><>" {
            let text = match tok.chars().next().unwrap() {
                ':' => "meow",
                ';' => "nya",
                'x' => "rawr",
                '>' => "le fishe",
                '<' => "i <3 emlang",
                _ => unreachable!(),
            };
            let s = utils::strcpy_to_heap(text);
            Em::new_with_data(EmType::Push, data::Data::new_str(s))
        } else if is_int {
            let val: i64 = tok.parse().unwrap_or(0);
            Em::new_with_data(EmType::Push, data::Data::new_int(val))
        } else {
            let s = utils::strcpy_to_heap(&tok);
            Em::new_with_data(EmType::Push, data::Data::new_str(s))
        };

        em_obj.row = start_row;
        em_obj.col = start_col;
        em_obj.path = self.path.clone();
        self.prog.push(em_obj);

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
            if self.is_end() || self.ch == '\n' as i32 {
                return ParserResult {
                    path: self.path.clone(),
                    row: start_row,
                    col: start_col,
                    prog: Err(ParserError::UnterminatedQuotes),
                };
            }

            if escape {
                let escaped = match self.ch as u8 as char {
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
                        return ParserResult {
                            path: self.path.clone(),
                            row: self.row,
                            col: self.col,
                            prog: Err(ParserError::UnknownEscape),
                        };
                    }
                };
                self.tok_add(escaped);
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

        let s = utils::strcpy_to_heap(&self.tok);
        let mut em_obj = Em::new_with_data(EmType::Push, data::Data::new_str(s));
        em_obj.row = start_row;
        em_obj.col = start_col;
        em_obj.path = self.path.clone();
        self.prog.push(em_obj);

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

        if self.ch == '"' as i32 {
            self.parse_quotes()
        } else {
            self.parse_plain()
        }
    }
}

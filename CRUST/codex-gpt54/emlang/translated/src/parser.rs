use core::fmt;
use std::fs;

use crate::{
    data::Data,
    em::{self, Em, EmType, Program, DATA_STDERR, DATA_STDOUT, DEFAULT_PROGRAM_CAP},
    utils::strcpy_to_heap,
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
        let text = match self {
            ParserError::UnexpectedEscape => "Unexpected escape",
            ParserError::UnknownEscape => "Unknown escape",
            ParserError::UnterminatedQuotes => "Unterminated quotes",
            ParserError::UnexpectedEnd => "Unexpected end",
            ParserError::IllegalPrintNest => "Illegal print nesting",
            ParserError::ExpectedEnd => "Expected matching end",
        };
        f.write_str(text)
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
        Self {
            path: String::new(),
            row: 1,
            col: 0,
            from_file: false,
            input: String::new(),
            ch: 0,
            pos: 0,
            tok: String::with_capacity(PARSER_MAX_TOKEN_LENGTH),
            tok_len: 0,
            prog: Program::new(DEFAULT_PROGRAM_CAP),
        }
    }
    pub fn load_mem(&mut self, input: &str) {
        self.from_file = false;
        self.input = input.to_owned();
    }
    pub fn load_file(&mut self, path: &str) -> i32 {
        self.from_file = true;
        self.path = path.to_owned();
        match fs::read_to_string(path) {
            Ok(contents) => {
                self.input = contents;
                0
            }
            Err(_) => -1,
        }
    }
    pub fn parse(&mut self) -> ParserResult {
        self.advance();
        if self.ch == 0 {
            return self.ok_result();
        }

        let mut result;
        loop {
            result = self.parse_next();
            if result.prog.is_err() || self.ch == 0 {
                break;
            }
        }

        if result.prog.is_err() {
            return result;
        }

        result = self.cross_ref();
        if result.prog.is_err() {
            return result;
        }

        self.ok_result()
    }
    pub fn cross_ref(&mut self) -> ParserResult {
        let mut expects = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut begins = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut print = false;

        for i in 0..self.prog.size {
            let em = &self.prog.ems[i];
            match em.em_type {
                EmType::PrintBegin => {
                    if print {
                        return self.err_result(ParserError::IllegalPrintNest, &em.path, em.row, em.col);
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
                    let Some(expected) = expects.pop() else {
                        return self.err_result(ParserError::UnexpectedEnd, &em.path, em.row, em.col);
                    };
                    let begin = begins.pop().unwrap();
                    if expected != EmType::PrintEnd {
                        return self.err_result(ParserError::UnexpectedEnd, &em.path, em.row, em.col);
                    }
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                EmType::IfEnd => {
                    let Some(expected) = expects.pop() else {
                        return self.err_result(ParserError::UnexpectedEnd, &em.path, em.row, em.col);
                    };
                    let begin = begins.pop().unwrap();
                    if expected != EmType::IfEnd {
                        return self.err_result(ParserError::UnexpectedEnd, &em.path, em.row, em.col);
                    }
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                EmType::LoopEnd => {
                    let Some(expected) = expects.pop() else {
                        return self.err_result(ParserError::UnexpectedEnd, &em.path, em.row, em.col);
                    };
                    let begin = begins.pop().unwrap();
                    if expected != EmType::LoopEnd {
                        return self.err_result(ParserError::UnexpectedEnd, &em.path, em.row, em.col);
                    }
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                _ => {}
            }
        }

        if let Some(begin) = begins.pop() {
            let em = &self.prog.ems[begin];
            return self.err_result(ParserError::ExpectedEnd, &em.path, em.row, em.col);
        }

        self.ok_result()
    }
    pub fn advance(&mut self) {
        if self.ch == '\n' as i32 {
            self.row += 1;
            self.col = 0;
        }

        self.ch = self
            .input
            .as_bytes()
            .get(self.pos)
            .copied()
            .unwrap_or(0) as i32;
        self.pos += 1;

        if self.ch != 0 {
            self.col += 1;
        }
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == '\\' as i32 {
            self.advance();
            if self.ch == 0 || is_space(self.ch) {
                return self.err_result(
                    ParserError::UnexpectedEscape,
                    &self.path,
                    start_row,
                    start_col,
                );
            } else if self.ch != '"' as i32 {
                self.tok.push('\\');
                self.tok_len += 1;
            }
        }

        let mut is_int = true;
        loop {
            let ch = self.ch as u8 as char;
            if is_int && !(self.tok_len == 0 && ch == '-') && !ch.is_ascii_digit() {
                is_int = false;
            }

            self.push_tok(ch);
            self.advance();
            if self.ch == 0 {
                return self.ok_result();
            }
            if is_space(self.ch) {
                break;
            }
        }

        if self.tok_len == 1 && self.tok == "-" {
            is_int = false;
        }

        let token = self.tok.clone();
        let em_type = keyword_to_em_type(&token);

        let mut em = if let Some(em_type) = em_type {
            Em::new(em_type)
        } else if token == ":x" {
            while self.ch != 0 && self.ch != '\n' as i32 {
                self.advance();
            }
            return self.ok_result();
        } else if token == ":)" {
            Em::new_with_data(EmType::PrintEnd, Data::new_int(DATA_STDOUT as i64))
        } else if token == ":(" {
            Em::new_with_data(EmType::PrintEnd, Data::new_int(DATA_STDERR as i64))
        } else if matches!(token.as_str(), ":3" | ";3" | "<3" | "x3" | "><>") {
            let text = match token.as_bytes()[0] as char {
                ':' => "meow",
                ';' => "nya",
                'x' => "rawr",
                '>' => "le fishe",
                '<' => "i <3 emlang",
                _ => unreachable!(),
            };
            Em::new_with_data(EmType::Push, Data::new_str(strcpy_to_heap(text)))
        } else if is_int {
            Em::new_with_data(
                EmType::Push,
                Data::new_int(token.parse::<i64>().unwrap_or(0)),
            )
        } else {
            Em::new_with_data(EmType::Push, Data::new_str(strcpy_to_heap(&token)))
        };

        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        self.ok_result()
    }
    pub fn parse_quotes(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;
        let mut escape = false;

        loop {
            self.advance();
            if self.ch == 0 || self.ch == '\n' as i32 {
                return self.err_result(
                    ParserError::UnterminatedQuotes,
                    &self.path,
                    start_row,
                    start_col,
                );
            }

            let ch = self.ch as u8 as char;
            if escape {
                let escaped = match ch {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'f' => '\u{000C}',
                    'v' => '\u{000B}',
                    'b' => '\u{0008}',
                    'a' => '\u{0007}',
                    '"' => '"',
                    'e' => '\u{001B}',
                    '\\' => '\\',
                    _ => {
                        return self.err_result(
                            ParserError::UnknownEscape,
                            &self.path,
                            self.row,
                            self.col,
                        )
                    }
                };
                self.push_tok(escaped);
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                break;
            } else {
                self.push_tok(ch);
            }
        }
        self.advance();

        let mut em = Em::new_with_data(EmType::Push, Data::new_str(strcpy_to_heap(&self.tok)));
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        self.ok_result()
    }
    pub fn parse_next(&mut self) -> ParserResult {
        while is_space(self.ch) {
            self.advance();
            if self.ch == 0 {
                return self.ok_result();
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
    fn ok_result(&self) -> ParserResult {
        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
        }
    }

    fn err_result(&self, err: ParserError, path: &str, row: usize, col: usize) -> ParserResult {
        ParserResult {
            path: path.to_owned(),
            row,
            col,
            prog: Err(err),
        }
    }

    fn push_tok(&mut self, ch: char) {
        assert!(self.tok_len < PARSER_MAX_TOKEN_LENGTH);
        self.tok.push(ch);
        self.tok_len += 1;
    }
}

fn is_space(ch: i32) -> bool {
    (ch as u8 as char).is_ascii_whitespace()
}

fn keyword_to_em_type(token: &str) -> Option<EmType> {
    match token {
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
        #[cfg(debug_assertions)]
        "D:" => Some(EmType::Debug),
        _ => None,
    }
}

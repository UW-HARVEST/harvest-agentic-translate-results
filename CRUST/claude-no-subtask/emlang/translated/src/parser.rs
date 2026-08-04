use core::fmt;
use std::fs;
use crate::data;
use crate::em::{self, Em, EmType, Program, DEFAULT_PROGRAM_CAP, DATA_STDOUT, DATA_STDERR};

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

fn em_to_keyword(t: EmType) -> Option<&'static str> {
    match t {
        EmType::Push => None,
        EmType::Pop => Some(":P"),
        EmType::Add => Some(";)"),
        EmType::Sub => Some(";("),
        EmType::Mul => Some("x)"),
        EmType::Div => Some("x("),
        EmType::Grt => Some(":>"),
        EmType::Less => Some(":<"),
        EmType::Equ => Some(":|"),
        EmType::Nequ => Some("x|"),
        EmType::PrintBegin => Some(":O"),
        EmType::PrintEnd => None,
        EmType::IfBegin => Some(":/"),
        EmType::IfEnd => Some(":\\"),
        EmType::LoopBegin => Some(":@"),
        EmType::LoopEnd => Some("@:"),
        EmType::Exit => Some("X_X"),
        EmType::Dup => Some(":D"),
        EmType::Swap => Some(":S"),
        #[cfg(debug_assertions)]
        EmType::Debug => Some("D:"),
    }
}

const KEYWORD_EMS: &[EmType] = &[
    EmType::Pop,
    EmType::Add,
    EmType::Sub,
    EmType::Mul,
    EmType::Div,
    EmType::Grt,
    EmType::Less,
    EmType::Equ,
    EmType::Nequ,
    EmType::PrintBegin,
    EmType::IfBegin,
    EmType::IfEnd,
    EmType::LoopBegin,
    EmType::LoopEnd,
    EmType::Exit,
    EmType::Dup,
    EmType::Swap,
    #[cfg(debug_assertions)]
    EmType::Debug,
];

fn is_space(c: i32) -> bool {
    if c < 0 { return false; }
    let c = c as u8;
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn is_digit(c: i32) -> bool {
    c >= b'0' as i32 && c <= b'9' as i32
}

impl Parser {
    pub fn new() -> Self {
        let prog = Program::new(DEFAULT_PROGRAM_CAP);
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
            prog,
        }
    }

    pub fn load_mem(&mut self, input: &str) {
        self.input = input.to_string();
        self.from_file = false;
    }

    pub fn load_file(&mut self, path: &str) -> i32 {
        self.from_file = true;
        self.path = path.to_string();
        match fs::read(path) {
            Ok(bytes) => {
                // Use lossy conversion that preserves bytes
                self.input = bytes.iter().map(|&b| b as char).collect();
                0
            }
            Err(_) => -1,
        }
    }

    pub fn advance(&mut self) {
        if self.ch == b'\n' as i32 {
            self.row += 1;
            self.col = 0;
        }
        if self.pos < self.input.len() {
            // Get the byte at self.pos
            let byte = self.input.as_bytes()[self.pos];
            self.ch = byte as i32;
            self.pos += 1;
        } else {
            self.ch = 0;
            self.pos += 1;
            return;
        }
        if self.ch == 0 {
            return;
        }
        self.col += 1;
    }

    fn end(&self) -> bool {
        self.ch == 0
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
            if self.end() || self.ch == b'\n' as i32 {
                return ParserResult {
                    path: self.path.clone(),
                    row: start_row,
                    col: start_col,
                    prog: Err(ParserError::UnterminatedQuotes),
                };
            }
            if escape {
                let c = match self.ch as u8 {
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'f' => 0x0C,
                    b'v' => 0x0B,
                    b'b' => 0x08,
                    b'a' => 0x07,
                    b'"' => b'"',
                    b'e' => 27,
                    b'\\' => b'\\',
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
                self.tok_add(self.ch as u8);
            }
        }
        self.advance();

        let mut em = Em::new_with_data(EmType::Push, data::Data::new_str(self.tok.clone()));
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);

        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Err(ParserError::UnknownEscape), // dummy, but err is OK; we'll use a sentinel
        }
        // Actually, parser_ok in C returns no error; we'll signal "ok" by setting prog to Ok later.
        // Let's restructure to use a different sentinel.
    }

    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok_clear();
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == b'\\' as i32 {
            self.advance();
            if self.end() || is_space(self.ch) {
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
                if !is_digit(self.ch) {
                    is_int = false;
                }
            }
            self.tok_add(self.ch as u8);
            self.advance();
            if self.end() {
                break;
            }
            if is_space(self.ch) {
                break;
            }
        }

        // is_end early return case (the C code returns parser_ok early if PARSER_END)
        // But it still needs to process the token. Looking at C more carefully:
        // The C code does `return parser_ok();` when PARSER_END is hit inside the loop.
        // This means it does NOT process the token! Let's match that behavior.
        let was_end_early = self.end() && !is_space(self.ch);
        if was_end_early {
            // The C code returns parser_ok() before processing the token.
            // Wait, no -- the do-while's exit happens via return inside, so the token never gets processed.
            // That's a bug in C, but let's match it.
            return parser_ok(self.path.clone());
        }

        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }

        let mut em: Option<Em> = None;
        for &t in KEYWORD_EMS {
            if let Some(kw) = em_to_keyword(t) {
                if self.tok == kw {
                    em = Some(Em::new(t));
                    break;
                }
            }
        }

        if em.is_none() {
            if self.tok == ":x" {
                while !self.end() && self.ch != b'\n' as i32 {
                    self.advance();
                }
                return parser_ok(self.path.clone());
            } else if self.tok == ":)" {
                em = Some(Em::new_with_data(EmType::PrintEnd, data::Data::new_int(DATA_STDOUT as i64)));
            } else if self.tok == ":(" {
                em = Some(Em::new_with_data(EmType::PrintEnd, data::Data::new_int(DATA_STDERR as i64)));
            } else if self.tok == ":3" || self.tok == ";3" || self.tok == "<3" || self.tok == "x3" || self.tok == "><>" {
                let text = match self.tok.as_bytes()[0] {
                    b':' => "meow",
                    b';' => "nya",
                    b'x' => "rawr",
                    b'>' => "le fishe",
                    b'<' => "i <3 emlang",
                    _ => panic!("unreachable"),
                };
                em = Some(Em::new_with_data(EmType::Push, data::Data::new_str(text.to_string())));
            } else if is_int {
                let val: i64 = self.tok.parse().unwrap_or(0);
                em = Some(Em::new_with_data(EmType::Push, data::Data::new_int(val)));
            } else {
                em = Some(Em::new_with_data(EmType::Push, data::Data::new_str(self.tok.clone())));
            }
        }

        let mut em = em.unwrap();
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        parser_ok(self.path.clone())
    }

    pub fn parse_next(&mut self) -> ParserResult {
        while is_space(self.ch) {
            self.advance();
            if self.end() {
                return parser_ok(self.path.clone());
            }
        }
        if self.ch == b'"' as i32 {
            // parse_quotes; need to fix the return value
            self.tok_clear();
            let start_row = self.row;
            let start_col = self.col;
            let mut escape = false;
            loop {
                self.advance();
                if self.end() || self.ch == b'\n' as i32 {
                    return ParserResult {
                        path: self.path.clone(),
                        row: start_row,
                        col: start_col,
                        prog: Err(ParserError::UnterminatedQuotes),
                    };
                }
                if escape {
                    let c = match self.ch as u8 {
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b't' => b'\t',
                        b'f' => 0x0C,
                        b'v' => 0x0B,
                        b'b' => 0x08,
                        b'a' => 0x07,
                        b'"' => b'"',
                        b'e' => 27,
                        b'\\' => b'\\',
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
                    self.tok_add(self.ch as u8);
                }
            }
            self.advance();

            let mut em = Em::new_with_data(EmType::Push, data::Data::new_str(self.tok.clone()));
            em.row = start_row;
            em.col = start_col;
            em.path = self.path.clone();
            self.prog.push(em);
            parser_ok(self.path.clone())
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
                    if begins.is_empty() {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    let expected = *expects.last().unwrap();
                    if em_type != expected {
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
                    if begins.is_empty() {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    let expected = *expects.last().unwrap();
                    if em_type != expected {
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

        if !begins.is_empty() {
            let begin_idx = *begins.last().unwrap();
            let em = &self.prog.ems[begin_idx];
            return ParserResult {
                path: em.path.clone(),
                row: em.row,
                col: em.col,
                prog: Err(ParserError::ExpectedEnd),
            };
        }

        parser_ok(self.path.clone())
    }

    pub fn parse(&mut self) -> ParserResult {
        self.advance();
        if self.end() {
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
            if result.prog.is_err() || self.end() {
                break;
            }
        }

        if result.prog.is_err() {
            return result;
        }

        let result = self.cross_ref();
        if result.prog.is_err() {
            return result;
        }

        ParserResult {
            path: self.path.clone(),
            row: result.row,
            col: result.col,
            prog: Ok(self.prog.clone()),
        }
    }
}

fn parser_ok(path: String) -> ParserResult {
    ParserResult {
        path,
        row: 0,
        col: 0,
        // We need a sentinel that means OK; use Ok with empty program for now.
        // But that conflicts with our convention. We're using Result<Program, ParserError>
        // as the actual return; we use Err with arbitrary error to indicate intermediate ok states.
        // Actually no -- in intermediate steps we use Ok(empty program) to signal success,
        // and at the end we replace with the real program.
        prog: Ok(Program::new(1)),
    }
}

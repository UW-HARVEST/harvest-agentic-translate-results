use core::fmt;
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

fn is_space_byte(c: i32) -> bool {
    if c < 0 || c > 127 {
        return false;
    }
    let c = c as u8;
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn is_digit_byte(c: i32) -> bool {
    c >= b'0' as i32 && c <= b'9' as i32
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
        self.from_file = true;
        self.path = path.to_string();
        match std::fs::read(path) {
            Ok(bytes) => {
                // Treat as raw bytes string (Latin-1-style)
                let s: String = bytes.iter().map(|b| *b as char).collect();
                self.input = s;
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
    fn parser_end(&self) -> bool {
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
            if self.parser_end() || self.ch == b'\n' as i32 {
                return ParserResult {
                    path: self.path.clone(),
                    row: start_row,
                    col: start_col,
                    prog: Err(ParserError::UnterminatedQuotes),
                };
            }
            if escape {
                let c = self.ch as u8;
                let ok_byte: u8 = match c {
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
                self.tok_add(ok_byte);
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
        let tok = self.tok.clone();
        let mut em_inst = em::Em::new_with_data(em::EmType::Push, data::Data::new_str(tok));
        em_inst.row = start_row;
        em_inst.col = start_col;
        em_inst.path = self.path.clone();
        self.prog.push(em_inst);
        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Ok(em::Program::new(1)), // placeholder Ok
        }
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok_clear();
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == b'\\' as i32 {
            self.advance();
            if self.parser_end() || is_space_byte(self.ch) {
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
                if !is_digit_byte(self.ch) {
                    is_int = false;
                }
            }
            self.tok_add(self.ch as u8);
            self.advance();
            if self.parser_end() {
                // Match C behavior: returns ok without pushing token
                return ParserResult {
                    path: self.path.clone(),
                    row: 0,
                    col: 0,
                    prog: Ok(em::Program::new(1)),
                };
            }
            if is_space_byte(self.ch) {
                break;
            }
        }

        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }

        let tok = self.tok.clone();

        let keyword_map: &[(&str, em::EmType)] = &[
            (":P", em::EmType::Pop),
            (";)", em::EmType::Add),
            (";(", em::EmType::Sub),
            ("x)", em::EmType::Mul),
            ("x(", em::EmType::Div),
            (":>", em::EmType::Grt),
            (":<", em::EmType::Less),
            (":|", em::EmType::Equ),
            ("x|", em::EmType::Nequ),
            (":O", em::EmType::PrintBegin),
            (":/", em::EmType::IfBegin),
            (":\\", em::EmType::IfEnd),
            (":@", em::EmType::LoopBegin),
            ("@:", em::EmType::LoopEnd),
            ("X_X", em::EmType::Exit),
            (":D", em::EmType::Dup),
            (":S", em::EmType::Swap),
            #[cfg(debug_assertions)]
            ("D:", em::EmType::Debug),
        ];

        let mut em_inst: Option<em::Em> = None;

        for (kw, t) in keyword_map {
            if tok == *kw {
                em_inst = Some(em::Em::new(*t));
                break;
            }
        }

        if em_inst.is_none() {
            if tok == ":x" {
                while !self.parser_end() && self.ch != b'\n' as i32 {
                    self.advance();
                }
                return ParserResult {
                    path: self.path.clone(),
                    row: 0,
                    col: 0,
                    prog: Ok(em::Program::new(1)),
                };
            } else if tok == ":)" {
                em_inst = Some(em::Em::new_with_data(
                    em::EmType::PrintEnd,
                    data::Data::new_int(em::DATA_STDOUT as i64),
                ));
            } else if tok == ":(" {
                em_inst = Some(em::Em::new_with_data(
                    em::EmType::PrintEnd,
                    data::Data::new_int(em::DATA_STDERR as i64),
                ));
            } else if tok == ":3" || tok == ";3" || tok == "<3" || tok == "x3" || tok == "><>" {
                let text = match tok.as_bytes()[0] {
                    b':' => "meow",
                    b';' => "nya",
                    b'x' => "rawr",
                    b'>' => "le fishe",
                    b'<' => "i <3 emlang",
                    _ => unreachable!(),
                };
                em_inst = Some(em::Em::new_with_data(
                    em::EmType::Push,
                    data::Data::new_str(text.to_string()),
                ));
            } else if is_int {
                let v: i64 = tok.parse::<i64>().unwrap_or(0);
                em_inst = Some(em::Em::new_with_data(
                    em::EmType::Push,
                    data::Data::new_int(v),
                ));
            } else {
                em_inst = Some(em::Em::new_with_data(
                    em::EmType::Push,
                    data::Data::new_str(tok.clone()),
                ));
            }
        }

        let mut em_inst = em_inst.unwrap();
        em_inst.row = start_row;
        em_inst.col = start_col;
        em_inst.path = self.path.clone();
        self.prog.push(em_inst);
        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Ok(em::Program::new(1)),
        }
    }
    pub fn parse_next(&mut self) -> ParserResult {
        while is_space_byte(self.ch) {
            self.advance();
            if self.parser_end() {
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
        let mut expects: Vec<em::EmType> = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut begins: Vec<usize> = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut print = false;

        for i in 0..self.prog.size {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                em::EmType::PrintBegin => {
                    if print {
                        let em_ref = &self.prog.ems[i];
                        return ParserResult {
                            path: em_ref.path.clone(),
                            row: em_ref.row,
                            col: em_ref.col,
                            prog: Err(ParserError::IllegalPrintNest),
                        };
                    }
                    print = true;
                    expects.push(em::EmType::PrintEnd);
                    begins.push(i);
                }
                em::EmType::IfBegin => {
                    expects.push(em::EmType::IfEnd);
                    begins.push(i);
                }
                em::EmType::LoopBegin => {
                    expects.push(em::EmType::LoopEnd);
                    begins.push(i);
                }
                em::EmType::PrintEnd | em::EmType::IfEnd | em::EmType::LoopEnd => {
                    if em_type == em::EmType::PrintEnd {
                        print = false;
                    }
                    if expects.is_empty() {
                        let em_ref = &self.prog.ems[i];
                        return ParserResult {
                            path: em_ref.path.clone(),
                            row: em_ref.row,
                            col: em_ref.col,
                            prog: Err(ParserError::UnexpectedEnd),
                        };
                    }
                    let last_expect = *expects.last().unwrap();
                    if em_type != last_expect {
                        let em_ref = &self.prog.ems[i];
                        return ParserResult {
                            path: em_ref.path.clone(),
                            row: em_ref.row,
                            col: em_ref.col,
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
            let em_ref = &self.prog.ems[begin];
            return ParserResult {
                path: em_ref.path.clone(),
                row: em_ref.row,
                col: em_ref.col,
                prog: Err(ParserError::ExpectedEnd),
            };
        }

        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Ok(em::Program::new(1)),
        }
    }
    pub fn parse(&mut self) -> ParserResult {
        self.advance();
        if self.parser_end() {
            return ParserResult {
                path: self.path.clone(),
                row: 0,
                col: 0,
                prog: Ok(self.prog.clone()),
            };
        }

        loop {
            let result = self.parse_next();
            if let Err(e) = result.prog {
                return ParserResult {
                    path: result.path,
                    row: result.row,
                    col: result.col,
                    prog: Err(e),
                };
            }
            if self.parser_end() {
                break;
            }
        }

        let cross = self.cross_ref();
        if let Err(e) = cross.prog {
            return ParserResult {
                path: cross.path,
                row: cross.row,
                col: cross.col,
                prog: Err(e),
            };
        }

        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Ok(self.prog.clone()),
        }
    }
}

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

const NULL_CH: i32 = 0;

fn is_space(c: i32) -> bool {
    if c <= 0 {
        return false;
    }
    let ch = c as u8 as char;
    ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == '\x0b' || ch == '\x0c'
}

fn is_digit(c: i32) -> bool {
    if c < 0 {
        return false;
    }
    let ch = c as u8 as char;
    ch.is_ascii_digit()
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
        match std::fs::read_to_string(path) {
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

        let res = self.cross_ref();
        if let Err(e) = res.prog {
            return ParserResult {
                path: res.path,
                row: res.row,
                col: res.col,
                prog: Err(e),
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
        let mut expects: Vec<em::EmType> = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut begins: Vec<usize> = Vec::with_capacity(PARSER_MAX_NESTS);
        let mut print = false;

        for i in 0..self.prog.size {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                em::EmType::PrintBegin => {
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
                em::EmType::PrintEnd => {
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
                em::EmType::IfEnd | em::EmType::LoopEnd => {
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
            if self.is_end() {
                return;
            }
            self.col += 1;
        } else {
            self.ch = NULL_CH;
            self.pos += 1;
        }
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
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
                self.tok.push('\\');
                self.tok_len += 1;
            }
        }

        let mut is_int = true;
        loop {
            if is_int && !(self.tok_len == 0 && self.ch == b'-' as i32) {
                if !is_digit(self.ch) {
                    is_int = false;
                }
            }
            self.tok.push(self.ch as u8 as char);
            self.tok_len += 1;
            self.advance();
            if self.is_end() {
                return self.finalize_plain(is_int, start_row, start_col);
            }
            if is_space(self.ch) {
                break;
            }
        }
        self.finalize_plain(is_int, start_row, start_col)
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
                let ch_char = match self.ch as u8 as char {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'f' => '\x0c',
                    'v' => '\x0b',
                    'b' => '\x08',
                    'a' => '\x07',
                    '"' => '"',
                    'e' => '\x1b',
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
                self.tok.push(ch_char);
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
        let mut em_inst =
            em::Em::new_with_data(em::EmType::Push, data::Data::new_str(s));
        em_inst.row = start_row;
        em_inst.col = start_col;
        em_inst.path = self.path.clone();
        self.prog.push(em_inst);

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

impl Parser {
    fn is_end(&self) -> bool {
        self.ch == NULL_CH
    }

    fn keyword_to_em_type(tok: &str) -> Option<em::EmType> {
        match tok {
            ":P" => Some(em::EmType::Pop),
            ";)" => Some(em::EmType::Add),
            ";(" => Some(em::EmType::Sub),
            "x)" => Some(em::EmType::Mul),
            "x(" => Some(em::EmType::Div),
            ":>" => Some(em::EmType::Grt),
            ":<" => Some(em::EmType::Less),
            ":|" => Some(em::EmType::Equ),
            "x|" => Some(em::EmType::Nequ),
            ":O" => Some(em::EmType::PrintBegin),
            ":/" => Some(em::EmType::IfBegin),
            ":\\" => Some(em::EmType::IfEnd),
            ":@" => Some(em::EmType::LoopBegin),
            "@:" => Some(em::EmType::LoopEnd),
            "X_X" => Some(em::EmType::Exit),
            ":D" => Some(em::EmType::Dup),
            ":S" => Some(em::EmType::Swap),
            _ => None,
        }
    }

    fn finalize_plain(
        &mut self,
        mut is_int: bool,
        start_row: usize,
        start_col: usize,
    ) -> ParserResult {
        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }

        let tok_str = self.tok.clone();
        let mut em_inst: Option<em::Em>;
        if let Some(em_type) = Self::keyword_to_em_type(&tok_str) {
            em_inst = Some(em::Em::new(em_type));
        } else if tok_str == ":x" {
            // Comment - read until newline
            while !self.is_end() && self.ch != b'\n' as i32 {
                self.advance();
            }
            return ParserResult {
                path: self.path.clone(),
                row: self.row,
                col: self.col,
                prog: Ok(self.prog.clone()),
            };
        } else if tok_str == ":)" {
            em_inst = Some(em::Em::new_with_data(
                em::EmType::PrintEnd,
                data::Data::new_int(em::DATA_STDOUT as i64),
            ));
        } else if tok_str == ":(" {
            em_inst = Some(em::Em::new_with_data(
                em::EmType::PrintEnd,
                data::Data::new_int(em::DATA_STDERR as i64),
            ));
        } else if tok_str == ":3"
            || tok_str == ";3"
            || tok_str == "<3"
            || tok_str == "x3"
            || tok_str == "><>"
        {
            let text = match tok_str.as_bytes()[0] as char {
                ':' => "meow",
                ';' => "nya",
                'x' => "rawr",
                '>' => "le fishe",
                '<' => "i <3 emlang",
                _ => unreachable!(),
            };
            em_inst = Some(em::Em::new_with_data(
                em::EmType::Push,
                data::Data::new_str(text.to_string()),
            ));
        } else if is_int {
            let val: i64 = tok_str.parse().unwrap_or(0);
            em_inst = Some(em::Em::new_with_data(
                em::EmType::Push,
                data::Data::new_int(val),
            ));
        } else {
            em_inst = Some(em::Em::new_with_data(
                em::EmType::Push,
                data::Data::new_str(tok_str.clone()),
            ));
        }

        if let Some(mut em_inst) = em_inst.take() {
            em_inst.row = start_row;
            em_inst.col = start_col;
            em_inst.path = self.path.clone();
            self.prog.push(em_inst);
        }

        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
        }
    }
}

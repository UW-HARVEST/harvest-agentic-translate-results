use core::fmt;
use crate::em;
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

fn em_to_keyword(t: em::EmType) -> Option<&'static str> {
    match t {
        em::EmType::Pop => Some(":P"),
        em::EmType::Add => Some(";)"),
        em::EmType::Sub => Some(";("),
        em::EmType::Mul => Some("x)"),
        em::EmType::Div => Some("x("),
        em::EmType::Grt => Some(":>"),
        em::EmType::Less => Some(":<"),
        em::EmType::Equ => Some(":|"),
        em::EmType::Nequ => Some("x|"),
        em::EmType::PrintBegin => Some(":O"),
        em::EmType::IfBegin => Some(":/"),
        em::EmType::IfEnd => Some(":\\"),
        em::EmType::LoopBegin => Some(":@"),
        em::EmType::LoopEnd => Some("@:"),
        em::EmType::Exit => Some("X_X"),
        em::EmType::Dup => Some(":D"),
        em::EmType::Swap => Some(":S"),
        _ => None,
    }
}

const ALL_EM_TYPES: &[em::EmType] = &[
    em::EmType::Push, em::EmType::Pop,
    em::EmType::Add, em::EmType::Sub, em::EmType::Mul, em::EmType::Div,
    em::EmType::Grt, em::EmType::Less, em::EmType::Equ, em::EmType::Nequ,
    em::EmType::PrintBegin, em::EmType::PrintEnd,
    em::EmType::IfBegin, em::EmType::IfEnd,
    em::EmType::LoopBegin, em::EmType::LoopEnd,
    em::EmType::Exit, em::EmType::Dup, em::EmType::Swap,
];

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
            Ok(contents) => {
                self.input = contents;
                0
            }
            Err(_) => -1,
        }
    }

    fn is_end(&self) -> bool {
        self.ch == 0
    }

    pub fn advance(&mut self) {
        if self.ch == '\n' as i32 {
            self.row += 1;
            self.col = 0;
        }
        let bytes = self.input.as_bytes();
        if self.pos < bytes.len() {
            self.ch = bytes[self.pos] as i32;
            self.pos += 1;
        } else {
            self.ch = 0;
            return;
        }
        if self.is_end() {
            return;
        }
        self.col += 1;
    }

    fn ok_result(&self) -> ParserResult {
        ParserResult {
            path: String::new(),
            row: 0,
            col: 0,
            prog: Ok(em::Program::new(0)),
        }
    }

    fn err_result(&self, err: ParserError, path: &str, row: usize, col: usize) -> ParserResult {
        ParserResult {
            path: path.to_string(),
            row,
            col,
            prog: Err(err),
        }
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
                return self.err_result(ParserError::UnterminatedQuotes, &self.path.clone(), start_row, start_col);
            }
            if escape {
                let c = match self.ch as u8 as char {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'f' => '\x0C',
                    'v' => '\x0B',
                    'b' => '\x08',
                    'a' => '\x07',
                    '"' => '"',
                    'e' => '\x1B',
                    '\\' => '\\',
                    _ => {
                        return self.err_result(ParserError::UnknownEscape, &self.path.clone(), self.row, self.col);
                    }
                };
                self.tok.push(c);
                self.tok_len += 1;
                escape = false;
            } else if self.ch == '\\' as i32 {
                escape = true;
            } else if self.ch == '"' as i32 {
                break;
            } else {
                self.tok.push(self.ch as u8 as char);
                self.tok_len += 1;
            }
        }
        self.advance();

        let str_val = self.tok.clone();
        let mut em = em::Em::new_with_data(em::EmType::Push, data::Data::new_str(str_val));
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        self.ok_result()
    }

    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == '\\' as i32 {
            self.advance();
            if self.is_end() || is_space(self.ch) {
                return self.err_result(ParserError::UnexpectedEscape, &self.path.clone(), start_row, start_col);
            } else if self.ch != '"' as i32 {
                self.tok.push('\\');
                self.tok_len += 1;
            }
        }

        let mut is_int = true;
        loop {
            if is_int && !(self.tok_len == 0 && self.ch == '-' as i32) {
                if !(self.ch as u8 as char).is_ascii_digit() {
                    is_int = false;
                }
            }
            self.tok.push(self.ch as u8 as char);
            self.tok_len += 1;

            self.advance();
            if self.is_end() {
                return self.ok_result();
            }
            if is_space(self.ch) {
                break;
            }
        }

        if self.tok_len == 1 && self.tok.starts_with('-') {
            is_int = false;
        }

        // Match against keywords
        for &t in ALL_EM_TYPES {
            if let Some(kw) = em_to_keyword(t) {
                if self.tok == kw {
                    let mut em = em::Em::new(t);
                    em.row = start_row;
                    em.col = start_col;
                    em.path = self.path.clone();
                    self.prog.push(em);
                    return self.ok_result();
                }
            }
        }

        // Special tokens
        if self.tok == ":x" {
            while !self.is_end() && self.ch != '\n' as i32 {
                self.advance();
            }
            return self.ok_result();
        }

        let em;
        if self.tok == ":)" {
            em = em::Em::new_with_data(em::EmType::PrintEnd, data::Data::new_int(em::DATA_STDOUT as i64));
        } else if self.tok == ":(" {
            em = em::Em::new_with_data(em::EmType::PrintEnd, data::Data::new_int(em::DATA_STDERR as i64));
        } else if self.tok == ":3" || self.tok == ";3" || self.tok == "<3" || self.tok == "x3" || self.tok == "><>" {
            let text = match self.tok.as_bytes()[0] as char {
                ':' => "meow",
                ';' => "nya",
                'x' => "rawr",
                '>' => "le fishe",
                '<' => "i <3 emlang",
                _ => unreachable!(),
            };
            em = em::Em::new_with_data(em::EmType::Push, data::Data::new_str(text.to_string()));
        } else if is_int {
            let val: i64 = self.tok.parse().unwrap_or(0);
            em = em::Em::new_with_data(em::EmType::Push, data::Data::new_int(val));
        } else {
            em = em::Em::new_with_data(em::EmType::Push, data::Data::new_str(self.tok.clone()));
        };

        let mut em = em;
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        self.ok_result()
    }

    pub fn parse_next(&mut self) -> ParserResult {
        while is_space(self.ch) {
            self.advance();
            if self.is_end() {
                return self.ok_result();
            }
        }
        if self.ch == '"' as i32 {
            self.parse_quotes()
        } else {
            self.parse_plain()
        }
    }

    pub fn cross_ref(&mut self) -> ParserResult {
        let mut expects: Vec<em::EmType> = Vec::new();
        let mut begins: Vec<usize> = Vec::new();
        let mut print = false;

        for i in 0..self.prog.size {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                em::EmType::PrintBegin => {
                    if print {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return self.err_result(ParserError::IllegalPrintNest, &path, row, col);
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
                    if expects.is_empty() || *expects.last().unwrap() != em::EmType::PrintEnd {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return self.err_result(ParserError::UnexpectedEnd, &path, row, col);
                    }
                    expects.pop();
                    let begin = begins.pop().unwrap();
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                em::EmType::IfEnd => {
                    if expects.is_empty() || *expects.last().unwrap() != em::EmType::IfEnd {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return self.err_result(ParserError::UnexpectedEnd, &path, row, col);
                    }
                    expects.pop();
                    let begin = begins.pop().unwrap();
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                em::EmType::LoopEnd => {
                    if expects.is_empty() || *expects.last().unwrap() != em::EmType::LoopEnd {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return self.err_result(ParserError::UnexpectedEnd, &path, row, col);
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
            let last = *begins.last().unwrap();
            let path = self.prog.ems[last].path.clone();
            let row = self.prog.ems[last].row;
            let col = self.prog.ems[last].col;
            return self.err_result(ParserError::ExpectedEnd, &path, row, col);
        }

        self.ok_result()
    }

    pub fn parse(&mut self) -> ParserResult {
        self.advance();
        if self.is_end() {
            return ParserResult {
                path: String::new(),
                row: 0,
                col: 0,
                prog: Ok(std::mem::replace(&mut self.prog, em::Program::new(0))),
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
            path: String::new(),
            row: 0,
            col: 0,
            prog: Ok(std::mem::replace(&mut self.prog, em::Program::new(0))),
        }
    }
}

fn is_space(ch: i32) -> bool {
    if ch <= 0 { return false; }
    (ch as u8 as char).is_ascii_whitespace()
}

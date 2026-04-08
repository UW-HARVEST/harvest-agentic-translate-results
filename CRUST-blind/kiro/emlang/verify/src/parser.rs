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
        match self {
            ParserError::UnexpectedEscape => write!(f, "Unexpected escape"),
            ParserError::UnknownEscape => write!(f, "Unknown escape"),
            ParserError::UnterminatedQuotes => write!(f, "Unterminated quotes"),
            ParserError::UnexpectedEnd => write!(f, "Unexpected end"),
            ParserError::IllegalPrintNest => write!(f, "Illegal print nesting"),
            ParserError::ExpectedEnd => write!(f, "Expected matching end"),
        }
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

fn parser_ok() -> ParserResult {
    ParserResult { path: String::new(), row: 0, col: 0, prog: Ok(em::Program::new(0)) }
}

fn parser_err(err: ParserError, path: &str, row: usize, col: usize) -> ParserResult {
    ParserResult { path: path.to_string(), row, col, prog: Err(err) }
}

fn is_end(ch: i32) -> bool {
    ch == 0
}

// Map of EmType to keyword
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
        #[cfg(debug_assertions)]
        em::EmType::Debug => Some("D:"),
        _ => None,
    }
}

static KEYWORD_TYPES: &[em::EmType] = &[
    em::EmType::Pop,
    em::EmType::Add, em::EmType::Sub, em::EmType::Mul, em::EmType::Div,
    em::EmType::Grt, em::EmType::Less, em::EmType::Equ, em::EmType::Nequ,
    em::EmType::PrintBegin,
    em::EmType::IfBegin, em::EmType::IfEnd,
    em::EmType::LoopBegin, em::EmType::LoopEnd,
    em::EmType::Exit,
    em::EmType::Dup, em::EmType::Swap,
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
    pub fn advance(&mut self) {
        if self.ch == '\n' as i32 {
            self.row += 1;
            self.col = 0;
        }
        if self.pos < self.input.len() {
            self.ch = self.input.as_bytes()[self.pos] as i32;
            self.pos += 1;
        } else {
            self.ch = 0;
            self.pos += 1;
            return;
        }
        if is_end(self.ch) {
            return;
        }
        self.col += 1;
    }
    pub fn parse_quotes(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;

        let mut escape = false;
        loop {
            self.advance();
            if is_end(self.ch) || self.ch == '\n' as i32 {
                return parser_err(ParserError::UnterminatedQuotes, &self.path, start_row, start_col);
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
                    _ => return parser_err(ParserError::UnknownEscape, &self.path, self.row, self.col),
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
        let mut em_inst = em::Em::new_with_data(em::EmType::Push, data::Data::new_str(str_val));
        em_inst.row = start_row;
        em_inst.col = start_col;
        em_inst.path = self.path.clone();
        self.prog.push(em_inst);
        parser_ok()
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == '\\' as i32 {
            self.advance();
            if is_end(self.ch) || (self.ch as u8 as char).is_ascii_whitespace() {
                return parser_err(ParserError::UnexpectedEscape, &self.path, start_row, start_col);
            } else if self.ch != '"' as i32 {
                self.tok.push('\\');
                self.tok_len += 1;
            }
        }

        let mut is_int = true;
        let mut first = true;
        loop {
            let c = self.ch as u8 as char;
            if is_int && !(first && c == '-') {
                if !c.is_ascii_digit() {
                    is_int = false;
                }
            }
            self.tok.push(c);
            self.tok_len += 1;
            first = false;

            self.advance();
            if is_end(self.ch) {
                return parser_ok();
            }
            if (self.ch as u8 as char).is_ascii_whitespace() {
                break;
            }
        }

        if self.tok_len == 1 && self.tok.starts_with('-') {
            is_int = false;
        }

        // Check keyword match
        for &t in KEYWORD_TYPES {
            if let Some(kw) = em_to_keyword(t) {
                if self.tok == kw {
                    let mut em_inst = em::Em::new(t);
                    em_inst.row = start_row;
                    em_inst.col = start_col;
                    em_inst.path = self.path.clone();
                    self.prog.push(em_inst);
                    return parser_ok();
                }
            }
        }

        // Check special tokens
        if self.tok == ":x" {
            // Comment: skip to end of line
            while !is_end(self.ch) && self.ch != '\n' as i32 {
                self.advance();
            }
            return parser_ok();
        } else if self.tok == ":)" {
            let mut em_inst = em::Em::new_with_data(em::EmType::PrintEnd, data::Data::new_int(em::DATA_STDOUT as i64));
            em_inst.row = start_row;
            em_inst.col = start_col;
            em_inst.path = self.path.clone();
            self.prog.push(em_inst);
        } else if self.tok == ":(" {
            let mut em_inst = em::Em::new_with_data(em::EmType::PrintEnd, data::Data::new_int(em::DATA_STDERR as i64));
            em_inst.row = start_row;
            em_inst.col = start_col;
            em_inst.path = self.path.clone();
            self.prog.push(em_inst);
        } else if self.tok == ":3" || self.tok == ";3" || self.tok == "<3" || self.tok == "x3" || self.tok == "><>" {
            let text = match self.tok.as_bytes()[0] as char {
                ':' => "meow",
                ';' => "nya",
                'x' => "rawr",
                '>' => "le fishe",
                '<' => "i <3 emlang",
                _ => "",
            };
            let mut em_inst = em::Em::new_with_data(em::EmType::Push, data::Data::new_str(text.to_string()));
            em_inst.row = start_row;
            em_inst.col = start_col;
            em_inst.path = self.path.clone();
            self.prog.push(em_inst);
        } else if is_int {
            let val: i64 = self.tok.parse().unwrap_or(0);
            let mut em_inst = em::Em::new_with_data(em::EmType::Push, data::Data::new_int(val));
            em_inst.row = start_row;
            em_inst.col = start_col;
            em_inst.path = self.path.clone();
            self.prog.push(em_inst);
        } else {
            let mut em_inst = em::Em::new_with_data(em::EmType::Push, data::Data::new_str(self.tok.clone()));
            em_inst.row = start_row;
            em_inst.col = start_col;
            em_inst.path = self.path.clone();
            self.prog.push(em_inst);
        }
        parser_ok()
    }
    pub fn parse_next(&mut self) -> ParserResult {
        while (self.ch as u8 as char).is_ascii_whitespace() {
            self.advance();
            if is_end(self.ch) {
                return parser_ok();
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
        let mut nest: usize = 0;
        let mut print = false;

        for i in 0..self.prog.size {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                em::EmType::PrintBegin => {
                    if print {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return parser_err(ParserError::IllegalPrintNest, &path, row, col);
                    }
                    print = true;
                    if nest >= expects.len() {
                        expects.push(em::EmType::PrintEnd);
                        begins.push(i);
                    } else {
                        expects[nest] = em::EmType::PrintEnd;
                        begins[nest] = i;
                    }
                    nest += 1;
                }
                em::EmType::IfBegin => {
                    if nest >= expects.len() {
                        expects.push(em::EmType::IfEnd);
                        begins.push(i);
                    } else {
                        expects[nest] = em::EmType::IfEnd;
                        begins[nest] = i;
                    }
                    nest += 1;
                }
                em::EmType::LoopBegin => {
                    if nest >= expects.len() {
                        expects.push(em::EmType::LoopEnd);
                        begins.push(i);
                    } else {
                        expects[nest] = em::EmType::LoopEnd;
                        begins[nest] = i;
                    }
                    nest += 1;
                }
                em::EmType::PrintEnd => {
                    print = false;
                    if nest == 0 {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return parser_err(ParserError::UnexpectedEnd, &path, row, col);
                    }
                    if em_type != expects[nest - 1] {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return parser_err(ParserError::UnexpectedEnd, &path, row, col);
                    }
                    nest -= 1;
                    let begin = begins[nest];
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                em::EmType::IfEnd | em::EmType::LoopEnd => {
                    if nest == 0 {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return parser_err(ParserError::UnexpectedEnd, &path, row, col);
                    }
                    if em_type != expects[nest - 1] {
                        let path = self.prog.ems[i].path.clone();
                        let row = self.prog.ems[i].row;
                        let col = self.prog.ems[i].col;
                        return parser_err(ParserError::UnexpectedEnd, &path, row, col);
                    }
                    nest -= 1;
                    let begin = begins[nest];
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                _ => {}
            }
        }

        if nest != 0 {
            let idx = begins[nest - 1];
            let path = self.prog.ems[idx].path.clone();
            let row = self.prog.ems[idx].row;
            let col = self.prog.ems[idx].col;
            return parser_err(ParserError::ExpectedEnd, &path, row, col);
        }

        parser_ok()
    }
    pub fn parse(&mut self) -> ParserResult {
        self.advance();
        if is_end(self.ch) {
            return ParserResult {
                path: self.path.clone(),
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
            if is_end(self.ch) {
                break;
            }
        }

        let result = self.cross_ref();
        if result.prog.is_err() {
            return result;
        }

        ParserResult {
            path: self.path.clone(),
            row: 0,
            col: 0,
            prog: Ok(std::mem::replace(&mut self.prog, em::Program::new(0))),
        }
    }
}

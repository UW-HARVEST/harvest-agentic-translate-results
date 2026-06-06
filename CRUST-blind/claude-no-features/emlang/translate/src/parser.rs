use core::fmt;
use crate::data;
use crate::em::{self, Em, EmType, DEFAULT_PROGRAM_CAP, DATA_STDOUT, DATA_STDERR};
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
            prog: em::Program::new(DEFAULT_PROGRAM_CAP),
        }
    }
    pub fn load_mem(&mut self, input: &str) {
        self.input = input.to_string();
    }
    pub fn load_file(&mut self, path: &str) -> i32 {
        self.from_file = true;
        self.path = path.to_string();
        match std::fs::read_to_string(path) {
            Ok(s) => {
                self.input = s;
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
            if let Err(_) = &result.prog {
                return result;
            }
            if self.is_end() {
                break;
            }
        }
        let cross = self.cross_ref();
        match &cross.prog {
            Err(_) => return cross,
            Ok(_) => {}
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
        let mut print_in = false;
        let size = self.prog.size;
        for i in 0..size {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                EmType::PrintBegin => {
                    if print_in {
                        let em = &self.prog.ems[i];
                        return ParserResult {
                            path: em.path.clone(),
                            row: em.row,
                            col: em.col,
                            prog: Err(ParserError::IllegalPrintNest),
                        };
                    }
                    print_in = true;
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
                    print_in = false;
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
                EmType::IfEnd | EmType::LoopEnd => {
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
            if self.is_end() || is_c_space(self.ch) {
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
                if !is_c_digit(self.ch) {
                    is_int = false;
                }
            }
            self.tok_add(self.ch as u8);
            self.advance();
            if self.is_end() {
                return self.finalize_plain(start_row, start_col, is_int);
            }
            if is_c_space(self.ch) {
                break;
            }
        }

        self.finalize_plain(start_row, start_col, is_int)
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
                let to_add: u8 = match self.ch as u8 as char {
                    'n' => b'\n',
                    'r' => b'\r',
                    't' => b'\t',
                    'f' => 0x0C,
                    'v' => 0x0B,
                    'b' => 0x08,
                    'a' => 0x07,
                    '"' => b'"',
                    'e' => 27,
                    '\\' => b'\\',
                    _ => {
                        return ParserResult {
                            path: self.path.clone(),
                            row: self.row,
                            col: self.col,
                            prog: Err(ParserError::UnknownEscape),
                        };
                    }
                };
                self.tok_add(to_add);
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
        let mut em = Em::new_with_data(EmType::Push, data::Data::new_str(tok));
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
        while is_c_space(self.ch) {
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

    fn tok_add(&mut self, ch: u8) {
        assert!(self.tok_len < PARSER_MAX_TOKEN_LENGTH);
        self.tok.push(ch as char);
        self.tok_len += 1;
    }

    fn finalize_plain(&mut self, start_row: usize, start_col: usize, mut is_int: bool) -> ParserResult {
        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }
        let tok = self.tok.clone();
        let em = self.token_to_em(&tok, is_int);
        match em {
            Ok(mut em) => {
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
            Err(IsComment) => {
                // skip rest of line
                while !self.is_end() && self.ch != b'\n' as i32 {
                    self.advance();
                }
                ParserResult {
                    path: self.path.clone(),
                    row: self.row,
                    col: self.col,
                    prog: Ok(self.prog.clone()),
                }
            }
        }
    }

    fn token_to_em(&self, tok: &str, is_int: bool) -> Result<Em, IsComment> {
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
        ];
        for (kw, ty) in keyword_map {
            if tok == *kw {
                return Ok(Em::new(*ty));
            }
        }
        #[cfg(debug_assertions)]
        if tok == "D:" {
            return Ok(Em::new(EmType::Debug));
        }
        if tok == ":x" {
            return Err(IsComment);
        }
        if tok == ":)" {
            return Ok(Em::new_with_data(EmType::PrintEnd, data::Data::new_int(DATA_STDOUT as i64)));
        }
        if tok == ":(" {
            return Ok(Em::new_with_data(EmType::PrintEnd, data::Data::new_int(DATA_STDERR as i64)));
        }
        if tok == ":3" || tok == ";3" || tok == "<3" || tok == "x3" || tok == "><>" {
            let text = match tok.as_bytes()[0] as char {
                ':' => "meow",
                ';' => "nya",
                'x' => "rawr",
                '>' => "le fishe",
                '<' => "i <3 emlang",
                _ => unreachable!(),
            };
            return Ok(Em::new_with_data(
                EmType::Push,
                data::Data::new_str(text.to_string()),
            ));
        }
        if is_int {
            // atoll: parse leading optional sign + digits, ignore garbage
            let v = parse_atoll(tok);
            return Ok(Em::new_with_data(EmType::Push, data::Data::new_int(v)));
        }
        Ok(Em::new_with_data(
            EmType::Push,
            data::Data::new_str(tok.to_string()),
        ))
    }
}

struct IsComment;

fn parse_atoll(s: &str) -> i64 {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if neg { v.wrapping_neg() } else { v }
}

fn is_c_space(ch: i32) -> bool {
    matches!(ch as u8, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn is_c_digit(ch: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&ch)
}

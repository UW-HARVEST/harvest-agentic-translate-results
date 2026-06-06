use core::fmt;
use std::fs;
use crate::em::{self, Em, EmType, DATA_STDOUT, DATA_STDERR};
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
            tok: String::with_capacity(PARSER_MAX_TOKEN_LENGTH),
            tok_len: 0,
            prog: em::Program::new(em::DEFAULT_PROGRAM_CAP),
        }
    }

    pub fn load_mem(&mut self, input: &str) {
        self.input = input.to_string();
        self.from_file = false;
    }

    pub fn load_file(&mut self, path: &str) -> i32 {
        self.from_file = true;
        self.path = path.to_string();
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
        if self.is_end() {
            return ParserResult {
                path: self.path.clone(),
                row: self.row,
                col: self.col,
                prog: Ok(self.prog.clone()),
            };
        }

        let mut result = self.parse_next();
        while result.prog.is_ok() && !self.is_end() {
            result = self.parse_next();
        }

        if result.prog.is_err() {
            return result;
        }

        let cross = self.cross_ref();
        if cross.prog.is_err() {
            return cross;
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
        let n = self.prog.size;

        for i in 0..n {
            let em_type = self.prog.ems[i].em_type;
            match em_type {
                EmType::PrintBegin => {
                    if print {
                        let em = &self.prog.ems[i];
                        return self.err_at(ParserError::IllegalPrintNest, &em.path.clone(), em.row, em.col);
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
                        return self.err_at(ParserError::UnexpectedEnd, &em.path.clone(), em.row, em.col);
                    }
                    let expected = *expects.last().unwrap();
                    if em_type != expected {
                        let em = &self.prog.ems[i];
                        return self.err_at(ParserError::UnexpectedEnd, &em.path.clone(), em.row, em.col);
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
            let begin = *begins.last().unwrap();
            let em = &self.prog.ems[begin];
            return self.err_at(ParserError::ExpectedEnd, &em.path.clone(), em.row, em.col);
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

    pub fn parse_plain(&mut self) -> ParserResult {
        self.tok.clear();
        self.tok_len = 0;
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == b'\\' as i32 {
            self.advance();
            if self.is_end() || is_space(self.ch) {
                return self.err_at(
                    ParserError::UnexpectedEscape,
                    &self.path.clone(),
                    start_row,
                    start_col,
                );
            } else if self.ch != b'"' as i32 {
                self.tok_add(b'\\' as i32);
            }
        }

        let mut is_int = true;
        let mut early_end = false;
        loop {
            if is_int && !(self.tok_len == 0 && self.ch == b'-' as i32) {
                if !is_digit(self.ch) {
                    is_int = false;
                }
            }
            self.tok_add(self.ch);
            self.advance();
            if self.is_end() {
                early_end = true;
                break;
            }
            if !is_space(self.ch) {
                continue;
            }
            break;
        }

        if early_end {
            // Match C behavior: returns parser_ok() without processing the token
            return self.ok();
        }

        if self.tok_len == 1 && self.tok.as_bytes()[0] == b'-' {
            is_int = false;
        }

        // tok already contains the chars we added
        let tok_str = self.tok.clone();

        // Match against keywords
        let keywords: &[(&str, EmType)] = &[
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
            #[cfg(debug_assertions)]
            ("D:", EmType::Debug),
        ];

        let mut em: Option<Em> = None;
        for (kw, ty) in keywords {
            if tok_str == *kw {
                em = Some(Em::new(*ty));
                break;
            }
        }

        if em.is_none() {
            if tok_str == ":x" {
                // Comment - skip rest of line
                while !self.is_end() && self.ch != b'\n' as i32 {
                    self.advance();
                }
                return self.ok();
            } else if tok_str == ":)" {
                em = Some(Em::new_with_data(
                    EmType::PrintEnd,
                    data::Data::new_int(DATA_STDOUT as i64),
                ));
            } else if tok_str == ":(" {
                em = Some(Em::new_with_data(
                    EmType::PrintEnd,
                    data::Data::new_int(DATA_STDERR as i64),
                ));
            } else if tok_str == ":3"
                || tok_str == ";3"
                || tok_str == "<3"
                || tok_str == "x3"
                || tok_str == "><>"
            {
                let text = match tok_str.as_bytes()[0] {
                    b':' => "meow",
                    b';' => "nya",
                    b'x' => "rawr",
                    b'>' => "le fishe",
                    b'<' => "i <3 emlang",
                    _ => unreachable!(),
                };
                em = Some(Em::new_with_data(
                    EmType::Push,
                    data::Data::new_str(text.to_string()),
                ));
            } else if is_int {
                let parsed: i64 = tok_str.parse().unwrap_or(0);
                em = Some(Em::new_with_data(
                    EmType::Push,
                    data::Data::new_int(parsed),
                ));
            } else {
                em = Some(Em::new_with_data(
                    EmType::Push,
                    data::Data::new_str(tok_str.clone()),
                ));
            }
        }

        let mut em = em.unwrap();
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        self.ok()
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
                return self.err_at(
                    ParserError::UnterminatedQuotes,
                    &self.path.clone(),
                    start_row,
                    start_col,
                );
            }

            if escape {
                let c = match self.ch as u8 {
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'f' => 0x0C, // \f
                    b'v' => 0x0B, // \v
                    b'b' => 0x08, // \b
                    b'a' => 0x07, // \a
                    b'"' => b'"',
                    b'e' => 27,
                    b'\\' => b'\\',
                    _ => {
                        return self.err_at(
                            ParserError::UnknownEscape,
                            &self.path.clone(),
                            self.row,
                            self.col,
                        );
                    }
                };
                self.tok_add(c as i32);
                escape = false;
            } else if self.ch == b'\\' as i32 {
                escape = true;
            } else if self.ch == b'"' as i32 {
                break;
            } else {
                self.tok_add(self.ch);
            }
        }
        self.advance();

        let str_val = self.tok.clone();
        let mut em = Em::new_with_data(EmType::Push, data::Data::new_str(str_val));
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        self.ok()
    }

    pub fn parse_next(&mut self) -> ParserResult {
        while is_space(self.ch) {
            self.advance();
            if self.is_end() {
                return self.ok();
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

    fn tok_add(&mut self, ch: i32) {
        assert!(self.tok_len < PARSER_MAX_TOKEN_LENGTH);
        // Push the byte as a char. Since input is bytes, treat as Latin-1.
        let byte = ch as u8;
        if byte < 128 {
            self.tok.push(byte as char);
        } else {
            // For non-ASCII, push as char. Since input bytes are read one at a
            // time, this preserves the byte values for round-trip.
            self.tok.push(byte as char);
        }
        self.tok_len += 1;
    }

    fn ok(&self) -> ParserResult {
        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
        }
    }

    fn err_at(
        &self,
        err: ParserError,
        path: &str,
        row: usize,
        col: usize,
    ) -> ParserResult {
        ParserResult {
            path: path.to_string(),
            row,
            col,
            prog: Err(err),
        }
    }
}

fn is_space(ch: i32) -> bool {
    matches!(ch as u8, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn is_digit(ch: i32) -> bool {
    (ch as u8).is_ascii_digit()
}

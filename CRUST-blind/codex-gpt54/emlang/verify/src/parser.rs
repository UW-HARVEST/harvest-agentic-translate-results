use core::fmt;
use crate::em;
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
        let msg = match self {
            ParserError::UnexpectedEscape => "Unexpected escape",
            ParserError::UnknownEscape => "Unknown escape",
            ParserError::UnterminatedQuotes => "Unterminated quotes",
            ParserError::UnexpectedEnd => "Unexpected end",
            ParserError::IllegalPrintNest => "Illegal print nesting",
            ParserError::ExpectedEnd => "Expected matching end",
        };

        write!(f, "{msg}")
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
            prog: em::Program::new(em::DEFAULT_PROGRAM_CAP),
        }
    }
    pub fn load_mem(&mut self, input: &str) {
        self.input = input.to_string();
        self.from_file = false;
    }
    pub fn load_file(&mut self, path: &str) -> i32 {
        match std::fs::read(path) {
            Ok(bytes) => {
                self.from_file = true;
                self.path = path.to_string();
                self.input = String::from_utf8_lossy(&bytes).into_owned();
                0
            }
            Err(_) => -1,
        }
    }
    pub fn parse(&mut self) -> ParserResult {
        self.advance();
        if self.is_end() {
            return self.ok_result();
        }

        loop {
            let result = self.parse_next();
            if result.prog.is_err() || self.is_end() {
                if result.prog.is_err() {
                    return result;
                }
                break;
            }
        }

        let result = self.cross_ref();
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
                em::EmType::PrintBegin => {
                    if print {
                        return self.err_result_at(
                            ParserError::IllegalPrintNest,
                            em.path.clone(),
                            em.row,
                            em.col,
                        );
                    }
                    print = true;
                    assert!(expects.len() < PARSER_MAX_NESTS);
                    expects.push(em::EmType::PrintEnd);
                    begins.push(i);
                }
                em::EmType::IfBegin => {
                    assert!(expects.len() < PARSER_MAX_NESTS);
                    expects.push(em::EmType::IfEnd);
                    begins.push(i);
                }
                em::EmType::LoopBegin => {
                    assert!(expects.len() < PARSER_MAX_NESTS);
                    expects.push(em::EmType::LoopEnd);
                    begins.push(i);
                }
                em::EmType::PrintEnd => {
                    print = false;
                    let Some(expected) = expects.pop() else {
                        return self.err_result_at(
                            ParserError::UnexpectedEnd,
                            em.path.clone(),
                            em.row,
                            em.col,
                        );
                    };
                    let begin = begins.pop().unwrap();
                    if em.em_type != expected {
                        return self.err_result_at(
                            ParserError::UnexpectedEnd,
                            em.path.clone(),
                            em.row,
                            em.col,
                        );
                    }
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                em::EmType::IfEnd | em::EmType::LoopEnd => {
                    let Some(expected) = expects.pop() else {
                        return self.err_result_at(
                            ParserError::UnexpectedEnd,
                            em.path.clone(),
                            em.row,
                            em.col,
                        );
                    };
                    let begin = begins.pop().unwrap();
                    if em.em_type != expected {
                        return self.err_result_at(
                            ParserError::UnexpectedEnd,
                            em.path.clone(),
                            em.row,
                            em.col,
                        );
                    }
                    self.prog.ems[begin].r#ref = i;
                    self.prog.ems[i].r#ref = begin;
                }
                _ => {}
            }
        }

        if let Some(&begin) = begins.last() {
            let em = &self.prog.ems[begin];
            return self.err_result_at(ParserError::ExpectedEnd, em.path.clone(), em.row, em.col);
        }

        self.ok_result()
    }
    pub fn advance(&mut self) {
        if self.ch == '\n' as i32 {
            self.row += 1;
            self.col = 0;
        }

        let bytes = self.input.as_bytes();
        self.ch = if self.pos < bytes.len() {
            let ch = bytes[self.pos] as i32;
            self.pos += 1;
            ch
        } else {
            self.pos += 1;
            0
        };

        if !self.is_end() {
            self.col += 1;
        }
    }
    pub fn parse_plain(&mut self) -> ParserResult {
        self.clear_tok();
        let start_row = self.row;
        let start_col = self.col;

        if self.ch == '\\' as i32 {
            self.advance();
            if self.is_end() || self.current_char().is_ascii_whitespace() {
                return self.err_result(ParserError::UnexpectedEscape, start_row, start_col);
            } else if self.ch != '"' as i32 {
                self.push_tok('\\');
            }
        }

        let mut is_int = true;
        loop {
            let ch = self.current_char();
            if is_int && !(self.tok_len == 0 && ch == '-') && !ch.is_ascii_digit() {
                is_int = false;
            }

            self.push_tok(ch);
            self.advance();
            if self.is_end() {
                return self.ok_result();
            }
            if self.current_char().is_ascii_whitespace() {
                break;
            }
        }

        if self.tok_len == 1 && self.tok == "-" {
            is_int = false;
        }

        let token = self.tok.clone();
        let mut em = if let Some(em_type) = keyword_to_em_type(&token) {
            em::Em::new(em_type)
        } else if token == ":x" {
            while !self.is_end() && self.current_char() != '\n' {
                self.advance();
            }
            return self.ok_result();
        } else if token == ":)" {
            em::Em::new_with_data(
                em::EmType::PrintEnd,
                crate::data::Data::new_int(em::DATA_STDOUT as i64),
            )
        } else if token == ":(" {
            em::Em::new_with_data(
                em::EmType::PrintEnd,
                crate::data::Data::new_int(em::DATA_STDERR as i64),
            )
        } else if let Some(text) = shorthand_string(&token) {
            em::Em::new_with_data(
                em::EmType::Push,
                crate::data::Data::new_str(text.to_string()),
            )
        } else if is_int {
            em::Em::new_with_data(
                em::EmType::Push,
                crate::data::Data::new_int(token.parse::<i64>().unwrap_or(0)),
            )
        } else {
            em::Em::new_with_data(
                em::EmType::Push,
                crate::data::Data::new_str(token.clone()),
            )
        };

        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        self.ok_result()
    }
    pub fn parse_quotes(&mut self) -> ParserResult {
        self.clear_tok();
        let start_row = self.row;
        let start_col = self.col;

        let mut escape = false;
        loop {
            self.advance();
            if self.is_end() || self.current_char() == '\n' {
                return self.err_result(ParserError::UnterminatedQuotes, start_row, start_col);
            }

            let ch = self.current_char();
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
                        return self.err_result_at(
                            ParserError::UnknownEscape,
                            self.path.clone(),
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

        let mut em = em::Em::new_with_data(
            em::EmType::Push,
            crate::data::Data::new_str(self.tok.clone()),
        );
        em.row = start_row;
        em.col = start_col;
        em.path = self.path.clone();
        self.prog.push(em);
        self.ok_result()
    }
    pub fn parse_next(&mut self) -> ParserResult {
        while self.current_char().is_ascii_whitespace() {
            self.advance();
            if self.is_end() {
                return self.ok_result();
            }
        }

        if self.current_char() == '"' {
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

    fn current_char(&self) -> char {
        char::from_u32(self.ch as u32).unwrap_or('\0')
    }

    fn clear_tok(&mut self) {
        self.tok.clear();
        self.tok_len = 0;
    }

    fn push_tok(&mut self, ch: char) {
        assert!(self.tok_len < PARSER_MAX_TOKEN_LENGTH);
        self.tok.push(ch);
        self.tok_len += 1;
    }

    fn ok_result(&self) -> ParserResult {
        ParserResult {
            path: self.path.clone(),
            row: self.row,
            col: self.col,
            prog: Ok(self.prog.clone()),
        }
    }

    fn err_result(&self, err: ParserError, row: usize, col: usize) -> ParserResult {
        self.err_result_at(err, self.path.clone(), row, col)
    }

    fn err_result_at(&self, err: ParserError, path: String, row: usize, col: usize) -> ParserResult {
        ParserResult {
            path,
            row,
            col,
            prog: Err(err),
        }
    }
}

fn keyword_to_em_type(token: &str) -> Option<em::EmType> {
    match token {
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
        #[cfg(debug_assertions)]
        "D:" => Some(em::EmType::Debug),
        _ => None,
    }
}

fn shorthand_string(token: &str) -> Option<&'static str> {
    match token {
        ":3" => Some("meow"),
        ";3" => Some("nya"),
        "<3" => Some("i <3 emlang"),
        "x3" => Some("rawr"),
        "><>" => Some("le fishe"),
        _ => None,
    }
}

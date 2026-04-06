use std::collections::HashMap;
use std::fmt;
#[derive(Debug, Clone, PartialEq)]
pub enum CJson {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<CJson>),
    Object(HashMap<String, CJson>),
}
#[derive(Debug, Clone)]
pub enum CJsonError {
    UnexpectedEOF { pos: usize },
    UnexpectedToken { ch: char, pos: usize },
    InvalidLiteral { expected: &'static str, pos: usize },
    InvalidNumber { pos: usize },
    InvalidEscape { pos: usize },
    InvalidUnicodeEscape { pos: usize },
    ExpectedColon { pos: usize },
    ExpectedCommaOrEnd { pos: usize },
}
impl fmt::Display for CJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CJsonError::UnexpectedEOF { pos } => write!(f, "Unexpected end of input at position {}", pos),
            CJsonError::UnexpectedToken { ch, pos } => write!(f, "Unexpected token '{}' at position {}", ch, pos),
            CJsonError::InvalidLiteral { expected, pos } => write!(f, "Invalid literal, expected '{}' at position {}", expected, pos),
            CJsonError::InvalidNumber { pos } => write!(f, "Invalid number at position {}", pos),
            CJsonError::InvalidEscape { pos } => write!(f, "Invalid escape sequence at position {}", pos),
            CJsonError::InvalidUnicodeEscape { pos } => write!(f, "Invalid unicode escape at position {}", pos),
            CJsonError::ExpectedColon { pos } => write!(f, "Expected ':' at position {}", pos),
            CJsonError::ExpectedCommaOrEnd { pos } => write!(f, "Expected ',' or closing bracket at position {}", pos),
        }
    }
}
impl std::error::Error for CJsonError {}
struct Parser<'a> {
    input: &'a str,
    pos: usize,
}
impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Parser { input, pos: 0 }
    }
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }
    fn next_char(&mut self) -> Option<char> {
        let ch = self.input[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
    fn take_while<F>(&mut self, mut predicate: F) -> &'a str
    where
        F: FnMut(char) -> bool,
    {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if predicate(ch) {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        &self.input[start..self.pos]
    }
    fn skip_whitespace(&mut self) {
        self.take_while(|c| c.is_ascii_whitespace());
    }
    fn expect_char(&mut self, expected: char) -> Result<(), CJsonError> {
        match self.next_char() {
            Some(ch) if ch == expected => Ok(()),
            Some(ch) => Err(CJsonError::UnexpectedToken { ch, pos: self.pos - ch.len_utf8() }),
            None => Err(CJsonError::UnexpectedEOF { pos: self.pos }),
        }
    }
    fn parse_value(&mut self) -> Result<CJson, CJsonError> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            Some('n') => self.parse_null(),
            Some('t') | Some('f') => self.parse_bool(),
            Some('"') => Ok(CJson::String(self.parse_string()?)),
            Some('-') | Some('0'..='9') => self.parse_number(),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some(ch) => Err(CJsonError::UnexpectedToken { ch, pos: self.pos }),
        }
    }
    fn parse_null(&mut self) -> Result<CJson, CJsonError> {
        let pos = self.pos;
        if self.input[self.pos..].starts_with("null") {
            self.pos += 4;
            Ok(CJson::Null)
        } else {
            Err(CJsonError::InvalidLiteral { expected: "null", pos })
        }
    }
    fn parse_bool(&mut self) -> Result<CJson, CJsonError> {
        let pos = self.pos;
        if self.input[self.pos..].starts_with("true") {
            self.pos += 4;
            Ok(CJson::Bool(true))
        } else if self.input[self.pos..].starts_with("false") {
            self.pos += 5;
            Ok(CJson::Bool(false))
        } else {
            Err(CJsonError::InvalidLiteral { expected: "true/false", pos })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        // Match the C parse_number logic exactly
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        if self.peek() == Some('0') {
            self.pos += 1;
        } else if matches!(self.peek(), Some('1'..='9')) {
            self.take_while(|c| c.is_ascii_digit());
        }
        if self.peek() == Some('.') {
            self.pos += 1;
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(CJsonError::InvalidNumber { pos: start });
            }
            self.take_while(|c| c.is_ascii_digit());
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.pos += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            self.take_while(|c| c.is_ascii_digit());
        }
        let num_str = &self.input[start..self.pos];
        match num_str.parse::<f64>() {
            Ok(n) => Ok(CJson::Number(n)),
            Err(_) => Err(CJsonError::InvalidNumber { pos: start }),
        }
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        let pos = self.pos;
        self.expect_char('"')?;
        let mut result = std::string::String::new();
        loop {
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos }),
                Some('"') => return Ok(result),
                Some('\\') => {
                    match self.next_char() {
                        None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                        Some('b') => result.push('\u{0008}'),
                        Some('f') => result.push('\u{000C}'),
                        Some('n') => result.push('\n'),
                        Some('r') => result.push('\r'),
                        Some('t') => result.push('\t'),
                        Some('"') => result.push('"'),
                        Some('\\') => result.push('\\'),
                        Some('/') => result.push('/'),
                        Some('u') => {
                            let uc = self.parse_hex4()?;
                            if uc == 0 || (0xDC00..=0xDFFF).contains(&uc) {
                                // invalid, just skip like C does
                            } else if (0xD800..=0xDBFF).contains(&uc) {
                                // surrogate pair
                                if self.input[self.pos..].starts_with("\\u") {
                                    self.pos += 2;
                                    let uc2 = self.parse_hex4()?;
                                    if (0xDC00..=0xDFFF).contains(&uc2) {
                                        let cp = 0x10000 + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF));
                                        if let Some(c) = char::from_u32(cp) {
                                            result.push(c);
                                        }
                                    }
                                }
                            } else if let Some(c) = char::from_u32(uc) {
                                result.push(c);
                            }
                        }
                        Some(c) => result.push(c),
                    }
                }
                Some(c) => result.push(c),
            }
        }
    }
    fn parse_hex4(&mut self) -> Result<u32, CJsonError> {
        let pos = self.pos;
        let mut h: u32 = 0;
        for _ in 0..4 {
            match self.next_char() {
                Some(c) if c.is_ascii_hexdigit() => {
                    h = (h << 4) | c.to_digit(16).unwrap();
                }
                _ => return Err(CJsonError::InvalidUnicodeEscape { pos }),
            }
        }
        Ok(h)
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(CJson::Array(Vec::new()));
        }
        let mut items = Vec::new();
        loop {
            let val = self.parse_value()?;
            items.push(val);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => { self.pos += 1; }
                Some(']') => { self.pos += 1; return Ok(CJson::Array(items)); }
                Some(ch) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
    fn parse_object(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('{')?;
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(CJson::Object(HashMap::new()));
        }
        let mut map = HashMap::new();
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_char(':')?;
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => { self.pos += 1; }
                Some('}') => { self.pos += 1; return Ok(CJson::Object(map)); }
                Some(_) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
}
pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    if require_end {
        parser.skip_whitespace();
        if parser.pos < parser.input.len() {
            if let Some(ch) = parser.peek() {
                return Err(CJsonError::UnexpectedToken { ch, pos: parser.pos });
            }
        }
    }
    Ok(value)
}
fn escape_string(s: &str) -> String {
    let mut out = std::string::String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn format_number(d: f64) -> String {
    let i = d as i32;
    if (i as f64 - d).abs() <= f64::EPSILON && d <= i32::MAX as f64 && d >= i32::MIN as f64 {
        format!("{}", i)
    } else if (d.floor() - d).abs() <= f64::EPSILON && d.abs() < 1.0e60 {
        format!("{:.0}", d)
    } else if d.abs() < 1.0e-6 || d.abs() > 1.0e9 {
        format!("{:e}", d)
    } else {
        format!("{}", d)
    }
}

fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => write!(f, "null"),
        CJson::Bool(true) => write!(f, "true"),
        CJson::Bool(false) => write!(f, "false"),
        CJson::Number(n) => write!(f, "{}", format_number(*n)),
        CJson::String(s) => write!(f, "{}", escape_string(s)),
        CJson::Array(items) => {
            write!(f, "[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 { write!(f, ", ")?; }
                write_json_compact(f, item)?;
            }
            write!(f, "]")
        }
        CJson::Object(map) => {
            write!(f, "{{")?;
            let mut first = true;
            for (key, val) in map {
                if !first { write!(f, ",")?; }
                first = false;
                write!(f, "{}:", escape_string(key))?;
                write_json_compact(f, val)?;
            }
            write!(f, "}}")
        }
    }
}
fn write_json_pretty(f: &mut impl fmt::Write, value: &CJson, indent: usize) -> fmt::Result {
    match value {
        CJson::Null => write!(f, "null"),
        CJson::Bool(true) => write!(f, "true"),
        CJson::Bool(false) => write!(f, "false"),
        CJson::Number(n) => write!(f, "{}", format_number(*n)),
        CJson::String(s) => write!(f, "{}", escape_string(s)),
        CJson::Array(items) => {
            if items.is_empty() {
                return write!(f, "[]");
            }
            write!(f, "[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 { write!(f, ",")?; }
                write!(f, " ")?;
                write_json_pretty(f, item, indent + 1)?;
            }
            write!(f, "]")
        }
        CJson::Object(map) => {
            if map.is_empty() {
                write!(f, "{{")?;
                write!(f, "\n")?;
                for _ in 0..indent.saturating_sub(1) { write!(f, "\t")?; }
                return write!(f, "}}");
            }
            write!(f, "{{\n")?;
            let keys: Vec<&String> = map.keys().collect();
            for (i, key) in keys.iter().enumerate() {
                let val = &map[*key];
                for _ in 0..=indent { write!(f, "\t")?; }
                write!(f, "{}:\t", escape_string(key))?;
                write_json_pretty(f, val, indent + 1)?;
                if i < keys.len() - 1 { write!(f, ",")?; }
                write!(f, "\n")?;
            }
            for _ in 0..indent { write!(f, "\t")?; }
            write!(f, "}}")
        }
    }
}
impl CJson {
    pub fn print_unformatted(&self) -> String {
        let mut s = std::string::String::new();
        write_json_compact(&mut s, self).unwrap();
        s
    }
    pub fn print_formatted(&self) -> String {
        let mut s = std::string::String::new();
        write_json_pretty(&mut s, self, 0).unwrap();
        s
    }
    pub fn get_array_size(&self) -> Option<usize> {
        match self {
            CJson::Array(v) => Some(v.len()),
            _ => None,
        }
    }
    pub fn get_array_item(&self, index: usize) -> Option<&CJson> {
        match self {
            CJson::Array(v) => v.get(index),
            _ => None,
        }
    }
    pub fn get_object_item(&self, key: &str) -> Option<&CJson> {
        match self {
            CJson::Object(map) => {
                let key_lower = key.to_ascii_lowercase();
                map.iter().find(|(k, _)| k.to_ascii_lowercase() == key_lower).map(|(_, v)| v)
            }
            _ => None,
        }
    }
    pub fn create_null() -> Self {
        CJson::Null
    }
    pub fn create_bool(b: bool) -> Self {
        CJson::Bool(b)
    }
    pub fn create_number(n: f64) -> Self {
        CJson::Number(n)
    }
    pub fn create_string<S: Into<String>>(s: S) -> Self {
        CJson::String(s.into())
    }
    pub fn create_array() -> Self {
        CJson::Array(Vec::new())
    }
    pub fn create_object() -> Self {
        CJson::Object(HashMap::new())
    }
    pub fn add_item_to_array(&mut self, item: CJson) -> Result<(), &'static str> {
        match self {
            CJson::Array(v) => { v.push(item); Ok(()) }
            _ => Err("not an array"),
        }
    }
    pub fn add_item_to_object<S: Into<String>>(
        &mut self,
        key: S,
        value: CJson,
    ) -> Result<(), &'static str> {
        match self {
            CJson::Object(map) => { map.insert(key.into(), value); Ok(()) }
            _ => Err("not an object"),
        }
    }
}
impl fmt::Display for CJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_json_pretty(f, self, 0)
    }
}

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
            CJsonError::UnexpectedEOF { pos } => {
                write!(f, "unexpected end of input at position {}", pos)
            }
            CJsonError::UnexpectedToken { ch, pos } => {
                write!(f, "unexpected token '{}' at position {}", ch, pos)
            }
            CJsonError::InvalidLiteral { expected, pos } => {
                write!(f, "invalid literal, expected '{}' at position {}", expected, pos)
            }
            CJsonError::InvalidNumber { pos } => {
                write!(f, "invalid number at position {}", pos)
            }
            CJsonError::InvalidEscape { pos } => {
                write!(f, "invalid escape sequence at position {}", pos)
            }
            CJsonError::InvalidUnicodeEscape { pos } => {
                write!(f, "invalid unicode escape at position {}", pos)
            }
            CJsonError::ExpectedColon { pos } => {
                write!(f, "expected ':' at position {}", pos)
            }
            CJsonError::ExpectedCommaOrEnd { pos } => {
                write!(f, "expected ',' or end at position {}", pos)
            }
        }
    }
}
impl std::error::Error for CJsonError {}
struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

fn read_hex4(parser: &mut Parser<'_>) -> Result<u32, CJsonError> {
    let start = parser.pos;
    let mut value: u32 = 0;
    for _ in 0..4 {
        let ch = parser.next_char().ok_or(CJsonError::UnexpectedEOF { pos: parser.pos })?;
        let digit = match ch {
            '0'..='9' => (ch as u32) - ('0' as u32),
            'a'..='f' => 10 + (ch as u32) - ('a' as u32),
            'A'..='F' => 10 + (ch as u32) - ('A' as u32),
            _ => return Err(CJsonError::InvalidUnicodeEscape { pos: start }),
        };
        value = (value << 4) | digit;
    }
    Ok(value)
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }
    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
    fn take_while<F>(&mut self, mut predicate: F) -> &'a str
    where
        F: FnMut(char) -> bool,
    {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if !predicate(ch) {
                break;
            }
            self.pos += ch.len_utf8();
        }
        &self.input[start..self.pos]
    }
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if (ch as u32) <= 32 {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
    }
    fn expect_char(&mut self, expected: char) -> Result<(), CJsonError> {
        match self.peek() {
            Some(ch) if ch == expected => {
                self.pos += ch.len_utf8();
                Ok(())
            }
            Some(ch) => Err(CJsonError::UnexpectedToken { ch, pos: self.pos }),
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
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some(ch) if ch == '-' || ch.is_ascii_digit() => self.parse_number(),
            Some(ch) => Err(CJsonError::UnexpectedToken { ch, pos: self.pos }),
        }
    }
    fn parse_null(&mut self) -> Result<CJson, CJsonError> {
        if self.input[self.pos..].starts_with("null") {
            self.pos += 4;
            Ok(CJson::Null)
        } else {
            Err(CJsonError::InvalidLiteral {
                expected: "null",
                pos: self.pos,
            })
        }
    }
    fn parse_bool(&mut self) -> Result<CJson, CJsonError> {
        if self.input[self.pos..].starts_with("true") {
            self.pos += 4;
            Ok(CJson::Bool(true))
        } else if self.input[self.pos..].starts_with("false") {
            self.pos += 5;
            Ok(CJson::Bool(false))
        } else {
            Err(CJsonError::InvalidLiteral {
                expected: "true or false",
                pos: self.pos,
            })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        // integer part
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        // fractional
        if self.peek() == Some('.') {
            self.pos += 1;
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        // exponent
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.pos += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        let s = &self.input[start..self.pos];
        if s.is_empty() || s == "-" {
            return Err(CJsonError::InvalidNumber { pos: start });
        }
        s.parse::<f64>()
            .map(CJson::Number)
            .map_err(|_| CJsonError::InvalidNumber { pos: start })
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        self.expect_char('"')?;
        let mut out = String::new();
        loop {
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                Some('"') => return Ok(out),
                Some('\\') => {
                    let esc_pos = self.pos;
                    match self.next_char() {
                        None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('/') => out.push('/'),
                        Some('b') => out.push('\u{0008}'),
                        Some('f') => out.push('\u{000C}'),
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some('u') => {
                            let uc = read_hex4(self)?;
                            if (0xDC00..=0xDFFF).contains(&uc) || uc == 0 {
                                // Per C: invalid; skip
                            } else if (0xD800..=0xDBFF).contains(&uc) {
                                // need surrogate pair
                                if self.peek() == Some('\\') {
                                    self.next_char();
                                    if self.peek() == Some('u') {
                                        self.next_char();
                                        let uc2 = read_hex4(self)?;
                                        if (0xDC00..=0xDFFF).contains(&uc2) {
                                            let code = 0x10000
                                                + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF));
                                            if let Some(ch) = char::from_u32(code) {
                                                out.push(ch);
                                            }
                                        }
                                    }
                                }
                            } else if let Some(ch) = char::from_u32(uc) {
                                out.push(ch);
                            }
                            let _ = esc_pos;
                        }
                        Some(c) => {
                            // C: default copies char as-is
                            out.push(c);
                        }
                    }
                }
                Some(ch) => out.push(ch),
            }
        }
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        self.skip_whitespace();
        let mut items: Vec<CJson> = Vec::new();
        if self.peek() == Some(']') {
            self.next_char();
            return Ok(CJson::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.next_char();
                }
                Some(']') => {
                    self.next_char();
                    return Ok(CJson::Array(items));
                }
                Some(_) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
    fn parse_object(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('{')?;
        self.skip_whitespace();
        let mut map: HashMap<String, CJson> = HashMap::new();
        if self.peek() == Some('}') {
            self.next_char();
            return Ok(CJson::Object(map));
        }
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('"') => {}
                Some(ch) => return Err(CJsonError::UnexpectedToken { ch, pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(CJsonError::ExpectedColon { pos: self.pos });
            }
            self.next_char();
            self.skip_whitespace();
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.next_char();
                }
                Some('}') => {
                    self.next_char();
                    return Ok(CJson::Object(map));
                }
                Some(_) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
}
pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let value = parser.parse_value()?;
    if require_end {
        parser.skip_whitespace();
        if let Some(ch) = parser.peek() {
            return Err(CJsonError::UnexpectedToken {
                ch,
                pos: parser.pos,
            });
        }
    }
    Ok(value)
}
fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn write_number(f: &mut impl fmt::Write, n: f64) -> fmt::Result {
    if !n.is_finite() {
        // JSON does not have a representation for NaN/Inf; emit null
        return f.write_str("null");
    }
    write!(f, "{}", n)
}

fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Number(n) => write_number(f, *n),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(arr) => {
            f.write_char('[')?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_char(',')?;
                }
                write_json_compact(f, item)?;
            }
            f.write_char(']')
        }
        CJson::Object(obj) => {
            f.write_char('{')?;
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 {
                    f.write_char(',')?;
                }
                f.write_str(&escape_string(k))?;
                f.write_char(':')?;
                write_json_compact(f, v)?;
            }
            f.write_char('}')
        }
    }
}
fn write_json_pretty(f: &mut impl fmt::Write, value: &CJson, indent: usize) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Number(n) => write_number(f, *n),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(arr) => {
            if arr.is_empty() {
                return f.write_str("[]");
            }
            f.write_char('[')?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            f.write_char(']')
        }
        CJson::Object(obj) => {
            if obj.is_empty() {
                f.write_char('{')?;
                f.write_char('\n')?;
                for _ in 0..indent.saturating_sub(1) {
                    f.write_char('\t')?;
                }
                f.write_char('}')?;
                return Ok(());
            }
            let new_indent = indent + 1;
            f.write_char('{')?;
            f.write_char('\n')?;
            let len = obj.len();
            for (i, (k, v)) in obj.iter().enumerate() {
                for _ in 0..new_indent {
                    f.write_char('\t')?;
                }
                f.write_str(&escape_string(k))?;
                f.write_char(':')?;
                f.write_char('\t')?;
                write_json_pretty(f, v, new_indent)?;
                if i + 1 < len {
                    f.write_char(',')?;
                }
                f.write_char('\n')?;
            }
            for _ in 0..indent {
                f.write_char('\t')?;
            }
            f.write_char('}')
        }
    }
}
impl CJson {
    pub fn print_unformatted(&self) -> String {
        let mut out = String::new();
        let _ = write_json_compact(&mut out, self);
        out
    }
    pub fn print_formatted(&self) -> String {
        let mut out = String::new();
        let _ = write_json_pretty(&mut out, self, 0);
        out
    }
    pub fn get_array_size(&self) -> Option<usize> {
        match self {
            CJson::Array(arr) => Some(arr.len()),
            CJson::Object(obj) => Some(obj.len()),
            _ => None,
        }
    }
    pub fn get_array_item(&self, index: usize) -> Option<&CJson> {
        match self {
            CJson::Array(arr) => arr.get(index),
            _ => None,
        }
    }
    pub fn get_object_item(&self, key: &str) -> Option<&CJson> {
        match self {
            CJson::Object(obj) => {
                // Case-insensitive lookup, matching cJSON_GetObjectItem.
                obj.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| v)
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
            CJson::Array(arr) => {
                arr.push(item);
                Ok(())
            }
            _ => Err("not an array"),
        }
    }
    pub fn add_item_to_object<S: Into<String>>(
        &mut self,
        key: S,
        value: CJson,
    ) -> Result<(), &'static str> {
        match self {
            CJson::Object(obj) => {
                obj.insert(key.into(), value);
                Ok(())
            }
            _ => Err("not an object"),
        }
    }
}
impl fmt::Display for CJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_json_compact(f, self)
    }
}

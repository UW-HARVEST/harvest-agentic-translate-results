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
                write!(
                    f,
                    "invalid literal, expected '{}' at position {}",
                    expected, pos
                )
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
                write!(f, "expected ',' or end of container at position {}", pos)
            }
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
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }
    fn take_while<F>(&mut self, mut predicate: F) -> &'a str
    where
        F: FnMut(char) -> bool,
    {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        &self.input[start..self.pos]
    }
    fn skip_whitespace(&mut self) {
        // The C version skips any character with (unsigned char) value <= 32.
        while let Some(c) = self.peek() {
            if (c as u32) <= 32 {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }
    fn expect_char(&mut self, expected: char) -> Result<(), CJsonError> {
        match self.peek() {
            Some(c) if c == expected => {
                self.pos += c.len_utf8();
                Ok(())
            }
            Some(c) => Err(CJsonError::UnexpectedToken { ch: c, pos: self.pos }),
            None => Err(CJsonError::UnexpectedEOF { pos: self.pos }),
        }
    }
    fn parse_value(&mut self) -> Result<CJson, CJsonError> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            Some('n') => self.parse_null(),
            Some('t') | Some('f') => self.parse_bool(),
            Some('"') => self.parse_string().map(CJson::String),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(CJsonError::UnexpectedToken { ch: c, pos: self.pos }),
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
        // Optional minus sign.
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        // Integer part: either '0' or [1-9][0-9]*
        match self.peek() {
            Some('0') => {
                self.pos += 1;
            }
            Some(c) if c.is_ascii_digit() => {
                let _ = self.take_while(|c| c.is_ascii_digit());
            }
            _ => {
                return Err(CJsonError::InvalidNumber { pos: start });
            }
        }
        // Fractional part: '.' digits
        if self.peek() == Some('.') {
            // Look at the character after '.' to decide.
            let next = self.input[self.pos + 1..].chars().next();
            if matches!(next, Some(c) if c.is_ascii_digit()) {
                self.pos += 1; // consume '.'
                let _ = self.take_while(|c| c.is_ascii_digit());
            }
        }
        // Exponent part.
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.pos += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            let _ = self.take_while(|c| c.is_ascii_digit());
        }
        let num_str = &self.input[start..self.pos];
        let n: f64 = num_str
            .parse()
            .map_err(|_| CJsonError::InvalidNumber { pos: start })?;
        Ok(CJson::Number(n))
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        match self.peek() {
            Some('"') => {
                self.pos += 1;
            }
            Some(c) => {
                return Err(CJsonError::UnexpectedToken { ch: c, pos: self.pos });
            }
            None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
        }
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                Some('"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    self.pos += 1;
                    let esc_pos = self.pos;
                    let esc = self
                        .next_char()
                        .ok_or(CJsonError::UnexpectedEOF { pos: esc_pos })?;
                    match esc {
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000C}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'u' => {
                            let uc = self.parse_hex4()?;
                            if uc == 0 || (0xDC00..=0xDFFF).contains(&uc) {
                                // Invalid lone surrogate or NUL — match cJSON
                                // behavior of skipping.
                                continue;
                            }
                            let final_uc = if (0xD800..=0xDBFF).contains(&uc) {
                                // Expect a paired low surrogate via "\uXXXX".
                                if !self.input[self.pos..].starts_with("\\u") {
                                    continue;
                                }
                                self.pos += 2;
                                let uc2 = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&uc2) {
                                    continue;
                                }
                                0x10000 + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF))
                            } else {
                                uc
                            };
                            if let Some(ch) = char::from_u32(final_uc) {
                                out.push(ch);
                            }
                        }
                        _ => {
                            // Match cJSON behavior: pass through unknown
                            // escape characters literally.
                            out.push(esc);
                        }
                    }
                }
                Some(c) => {
                    self.pos += c.len_utf8();
                    out.push(c);
                }
            }
        }
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        self.skip_whitespace();
        let mut arr: Vec<CJson> = Vec::new();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(CJson::Array(arr));
        }
        loop {
            let value = self.parse_value()?;
            arr.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                    self.skip_whitespace();
                }
                Some(']') => {
                    self.pos += 1;
                    return Ok(CJson::Array(arr));
                }
                Some(_) => {
                    return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos });
                }
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
    fn parse_object(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('{')?;
        self.skip_whitespace();
        let mut obj: HashMap<String, CJson> = HashMap::new();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(CJson::Object(obj));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(CJsonError::ExpectedColon { pos: self.pos });
            }
            self.pos += 1;
            let value = self.parse_value()?;
            obj.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(CJson::Object(obj));
                }
                Some(_) => {
                    return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos });
                }
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
}

impl<'a> Parser<'a> {
    fn parse_hex4(&mut self) -> Result<u32, CJsonError> {
        let pos = self.pos;
        let slice = self
            .input
            .get(pos..pos + 4)
            .ok_or(CJsonError::InvalidUnicodeEscape { pos })?;
        let n = u32::from_str_radix(slice, 16)
            .map_err(|_| CJsonError::InvalidUnicodeEscape { pos })?;
        self.pos += 4;
        Ok(n)
    }
}

pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let value = parser.parse_value()?;
    if require_end {
        parser.skip_whitespace();
        if let Some(c) = parser.peek() {
            return Err(CJsonError::UnexpectedToken {
                ch: c,
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

fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    let int_val = n as i32;
    if (int_val as f64 - n).abs() <= f64::EPSILON
        && n <= i32::MAX as f64
        && n >= i32::MIN as f64
    {
        return format!("{}", int_val);
    }
    let abs = n.abs();
    if (n.floor() - n).abs() <= f64::EPSILON && abs < 1.0e60 {
        return format!("{:.0}", n);
    }
    if abs < 1.0e-6 || abs > 1.0e9 {
        // Scientific notation. Rust's default `{:e}` is acceptable JSON.
        return format!("{:e}", n);
    }
    // Default decimal representation.
    format!("{}", n)
}

fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Number(n) => f.write_str(&format_number(*n)),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(arr) => {
            f.write_str("[")?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_str(",")?;
                }
                write_json_compact(f, item)?;
            }
            f.write_str("]")
        }
        CJson::Object(obj) => {
            f.write_str("{")?;
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 {
                    f.write_str(",")?;
                }
                f.write_str(&escape_string(k))?;
                f.write_str(":")?;
                write_json_compact(f, v)?;
            }
            f.write_str("}")
        }
    }
}

fn write_json_pretty(f: &mut impl fmt::Write, value: &CJson, indent: usize) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Number(n) => f.write_str(&format_number(*n)),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(arr) => {
            if arr.is_empty() {
                return f.write_str("[]");
            }
            f.write_str("[")?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            f.write_str("]")
        }
        CJson::Object(obj) => {
            if obj.is_empty() {
                return f.write_str("{}");
            }
            f.write_str("{\n")?;
            let len = obj.len();
            for (i, (k, v)) in obj.iter().enumerate() {
                for _ in 0..(indent + 1) {
                    f.write_str("\t")?;
                }
                f.write_str(&escape_string(k))?;
                f.write_str(":\t")?;
                write_json_pretty(f, v, indent + 1)?;
                if i + 1 < len {
                    f.write_str(",")?;
                }
                f.write_str("\n")?;
            }
            for _ in 0..indent {
                f.write_str("\t")?;
            }
            f.write_str("}")
        }
    }
}

impl CJson {
    pub fn print_unformatted(&self) -> String {
        let mut s = String::new();
        write_json_compact(&mut s, self).expect("writing to String cannot fail");
        s
    }
    pub fn print_formatted(&self) -> String {
        let mut s = String::new();
        write_json_pretty(&mut s, self, 0).expect("writing to String cannot fail");
        s
    }
    pub fn get_array_size(&self) -> Option<usize> {
        match self {
            CJson::Array(arr) => Some(arr.len()),
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
                // cJSON uses case-insensitive comparison for object lookups.
                for (k, v) in obj.iter() {
                    if k.eq_ignore_ascii_case(key) {
                        return Some(v);
                    }
                }
                None
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
            _ => Err("cannot add item: receiver is not an array"),
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
            _ => Err("cannot add item: receiver is not an object"),
        }
    }
}
impl fmt::Display for CJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_json_compact(f, self)
    }
}

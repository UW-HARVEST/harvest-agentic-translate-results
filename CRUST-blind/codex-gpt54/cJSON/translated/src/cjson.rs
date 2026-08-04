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
            Self::UnexpectedEOF { pos } => write!(f, "unexpected end of input at byte {}", pos),
            Self::UnexpectedToken { ch, pos } => {
                write!(f, "unexpected token '{}' at byte {}", ch, pos)
            }
            Self::InvalidLiteral { expected, pos } => {
                write!(f, "invalid literal, expected {} at byte {}", expected, pos)
            }
            Self::InvalidNumber { pos } => write!(f, "invalid number at byte {}", pos),
            Self::InvalidEscape { pos } => write!(f, "invalid escape at byte {}", pos),
            Self::InvalidUnicodeEscape { pos } => {
                write!(f, "invalid unicode escape at byte {}", pos)
            }
            Self::ExpectedColon { pos } => write!(f, "expected ':' at byte {}", pos),
            Self::ExpectedCommaOrEnd { pos } => {
                write!(f, "expected ',' or closing delimiter at byte {}", pos)
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
        self.take_while(|ch| ch <= ' ');
    }
    fn expect_char(&mut self, expected: char) -> Result<(), CJsonError> {
        match self.next_char() {
            Some(ch) if ch == expected => Ok(()),
            Some(ch) => Err(CJsonError::UnexpectedToken {
                ch,
                pos: self.pos.saturating_sub(ch.len_utf8()),
            }),
            None => Err(CJsonError::UnexpectedEOF { pos: self.pos }),
        }
    }
    fn parse_value(&mut self) -> Result<CJson, CJsonError> {
        match self.peek() {
            Some('n') => self.parse_null(),
            Some('f') | Some('t') => self.parse_bool(),
            Some('"') => self.parse_string().map(CJson::String),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some('-') | Some('0'..='9') => self.parse_number(),
            Some(ch) => Err(CJsonError::UnexpectedToken { ch, pos: self.pos }),
            None => Err(CJsonError::UnexpectedEOF { pos: self.pos }),
        }
    }
    fn parse_null(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        if self.input[self.pos..].starts_with("null") {
            self.pos += 4;
            Ok(CJson::Null)
        } else {
            Err(CJsonError::InvalidLiteral {
                expected: "null",
                pos: start,
            })
        }
    }
    fn parse_bool(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        if self.input[self.pos..].starts_with("true") {
            self.pos += 4;
            Ok(CJson::Bool(true))
        } else if self.input[self.pos..].starts_with("false") {
            self.pos += 5;
            Ok(CJson::Bool(false))
        } else {
            let expected = if matches!(self.peek(), Some('t')) {
                "true"
            } else {
                "false"
            };
            Err(CJsonError::InvalidLiteral {
                expected,
                pos: start,
            })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let bytes = self.input.as_bytes();
        let len = bytes.len();
        let start = self.pos;
        let mut idx = self.pos;
        let mut n = 0.0f64;
        let mut sign = 1.0f64;
        let mut scale = 0.0f64;
        let mut subscale = 0.0f64;
        let mut signsubscale = 1.0f64;

        if idx < len && bytes[idx] == b'-' {
            sign = -1.0;
            idx += 1;
        }
        if idx < len && bytes[idx] == b'0' {
            idx += 1;
        }
        if idx < len && (b'1'..=b'9').contains(&bytes[idx]) {
            while idx < len && bytes[idx].is_ascii_digit() {
                n = (n * 10.0) + f64::from(bytes[idx] - b'0');
                idx += 1;
            }
        }
        if idx + 1 < len && bytes[idx] == b'.' && bytes[idx + 1].is_ascii_digit() {
            idx += 1;
            while idx < len && bytes[idx].is_ascii_digit() {
                n = (n * 10.0) + f64::from(bytes[idx] - b'0');
                scale -= 1.0;
                idx += 1;
            }
        }
        if idx < len && (bytes[idx] == b'e' || bytes[idx] == b'E') {
            idx += 1;
            if idx < len && bytes[idx] == b'+' {
                idx += 1;
            } else if idx < len && bytes[idx] == b'-' {
                signsubscale = -1.0;
                idx += 1;
            }
            while idx < len && bytes[idx].is_ascii_digit() {
                subscale = (subscale * 10.0) + f64::from(bytes[idx] - b'0');
                idx += 1;
            }
        }

        if idx == start || (idx == start + 1 && bytes.get(start) == Some(&b'-')) {
            return Err(CJsonError::InvalidNumber { pos: start });
        }

        self.pos = idx;
        Ok(CJson::Number(
            sign * n * 10.0f64.powf(scale + (subscale * signsubscale)),
        ))
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        self.expect_char('"')?;
        let mut out = String::new();

        loop {
            let Some(ch) = self.next_char() else {
                return Err(CJsonError::UnexpectedEOF { pos: self.pos });
            };

            match ch {
                '"' => return Ok(out),
                '\\' => {
                    let esc_pos = self.pos;
                    let escaped = self
                        .next_char()
                        .ok_or(CJsonError::UnexpectedEOF { pos: self.pos })?;
                    match escaped {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000C}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let code = self.parse_hex4(esc_pos)?;
                            if let Some(high) = Self::decode_utf16_unit(code, esc_pos)? {
                                out.push(high);
                            } else {
                                let pair_pos = self.pos;
                                if self.next_char() != Some('\\') || self.next_char() != Some('u') {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: pair_pos });
                                }
                                let low = self.parse_hex4(pair_pos + 2)?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: pair_pos });
                                }
                                let scalar =
                                    0x10000 + (((u32::from(code) & 0x3FF) << 10) | (u32::from(low) & 0x3FF));
                                let ch = char::from_u32(scalar)
                                    .ok_or(CJsonError::InvalidUnicodeEscape { pos: pair_pos })?;
                                out.push(ch);
                            }
                        }
                        _ => return Err(CJsonError::InvalidEscape { pos: esc_pos }),
                    }
                }
                _ => {
                    if ch.is_control() {
                        return Err(CJsonError::UnexpectedToken {
                            ch,
                            pos: self.pos.saturating_sub(ch.len_utf8()),
                        });
                    }
                    out.push(ch);
                }
            }
        }
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(CJson::Array(items));
        }

        loop {
            self.skip_whitespace();
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                    self.skip_whitespace();
                }
                Some(']') => {
                    self.pos += 1;
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
        let mut entries = HashMap::new();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(CJson::Object(entries));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(CJsonError::ExpectedColon { pos: self.pos });
            }
            self.pos += 1;
            self.skip_whitespace();
            let value = self.parse_value()?;
            entries.insert(key, value);
            self.skip_whitespace();

            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                    self.skip_whitespace();
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(CJson::Object(entries));
                }
                Some(_) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }

    fn parse_hex4(&mut self, pos: usize) -> Result<u16, CJsonError> {
        let mut value = 0u16;
        for _ in 0..4 {
            let ch = self
                .next_char()
                .ok_or(CJsonError::UnexpectedEOF { pos: self.pos })?;
            let digit = ch
                .to_digit(16)
                .ok_or(CJsonError::InvalidUnicodeEscape { pos })?;
            value = (value << 4) | digit as u16;
        }
        Ok(value)
    }

    fn decode_utf16_unit(code: u16, pos: usize) -> Result<Option<char>, CJsonError> {
        match code {
            0xD800..=0xDBFF => Ok(None),
            0xDC00..=0xDFFF => Err(CJsonError::InvalidUnicodeEscape { pos }),
            _ => char::from_u32(u32::from(code))
                .map(Some)
                .ok_or(CJsonError::InvalidUnicodeEscape { pos }),
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
            return Err(CJsonError::UnexpectedToken { ch, pos: parser.pos });
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
            ch if ch <= '\u{001F}' => {
                let _ = fmt::write(&mut out, format_args!("\\u{:04x}", ch as u32));
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Number(n) => f.write_str(&format_number(*n)),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(items) => {
            f.write_str("[")?;
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    f.write_str(",")?;
                }
                write_json_compact(f, item)?;
            }
            f.write_str("]")
        }
        CJson::Object(map) => {
            f.write_str("{")?;
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));
            for (idx, (key, item)) in entries.into_iter().enumerate() {
                if idx > 0 {
                    f.write_str(",")?;
                }
                f.write_str(&escape_string(key))?;
                f.write_str(":")?;
                write_json_compact(f, item)?;
            }
            f.write_str("}")
        }
    }
}
fn write_json_pretty(f: &mut impl fmt::Write, value: &CJson, indent: usize) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Number(n) => f.write_str(&format_number(*n)),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(items) => {
            f.write_str("[")?;
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    f.write_str(", ")?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            f.write_str("]")
        }
        CJson::Object(map) => {
            if map.is_empty() {
                return f.write_str("{\n}");
            }
            f.write_str("{\n")?;
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));
            let next_indent = indent + 1;
            for (idx, (key, item)) in entries.into_iter().enumerate() {
                for _ in 0..next_indent {
                    f.write_str("\t")?;
                }
                f.write_str(&escape_string(key))?;
                f.write_str(":\t")?;
                write_json_pretty(f, item, next_indent)?;
                if idx + 1 != map.len() {
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
            Self::Array(items) => Some(items.len()),
            _ => None,
        }
    }
    pub fn get_array_item(&self, index: usize) -> Option<&CJson> {
        match self {
            Self::Array(items) => items.get(index),
            _ => None,
        }
    }
    pub fn get_object_item(&self, key: &str) -> Option<&CJson> {
        match self {
            Self::Object(map) => map
                .get(key)
                .or_else(|| map.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)),
            _ => None,
        }
    }
    pub fn create_null() -> Self {
        Self::Null
    }
    pub fn create_bool(b: bool) -> Self {
        Self::Bool(b)
    }
    pub fn create_number(n: f64) -> Self {
        Self::Number(n)
    }
    pub fn create_string<S: Into<String>>(s: S) -> Self {
        Self::String(s.into())
    }
    pub fn create_array() -> Self {
        Self::Array(Vec::new())
    }
    pub fn create_object() -> Self {
        Self::Object(HashMap::new())
    }
    pub fn add_item_to_array(&mut self, item: CJson) -> Result<(), &'static str> {
        match self {
            Self::Array(items) => {
                items.push(item);
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
            Self::Object(map) => {
                map.insert(key.into(), value);
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

fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }

    if n.is_finite()
        && (n.trunc() - n).abs() <= f64::EPSILON
        && n <= i32::MAX as f64
        && n >= i32::MIN as f64
    {
        return format!("{}", n as i32);
    }

    if n.is_finite() && (n.floor() - n).abs() <= f64::EPSILON && n.abs() < 1.0e60 {
        return format!("{:.0}", n);
    }

    if n.abs() < 1.0e-6 || n.abs() > 1.0e9 {
        return format!("{:e}", n);
    }

    format!("{:.6}", n)
}

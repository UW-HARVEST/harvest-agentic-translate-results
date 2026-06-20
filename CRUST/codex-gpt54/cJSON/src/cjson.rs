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
            CJsonError::UnexpectedEOF { pos } => write!(f, "unexpected end of input at byte {}", pos),
            CJsonError::UnexpectedToken { ch, pos } => {
                write!(f, "unexpected token {:?} at byte {}", ch, pos)
            }
            CJsonError::InvalidLiteral { expected, pos } => {
                write!(f, "invalid literal, expected {} at byte {}", expected, pos)
            }
            CJsonError::InvalidNumber { pos } => write!(f, "invalid number at byte {}", pos),
            CJsonError::InvalidEscape { pos } => write!(f, "invalid escape at byte {}", pos),
            CJsonError::InvalidUnicodeEscape { pos } => {
                write!(f, "invalid unicode escape at byte {}", pos)
            }
            CJsonError::ExpectedColon { pos } => write!(f, "expected ':' at byte {}", pos),
            CJsonError::ExpectedCommaOrEnd { pos } => {
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
        self.take_while(|ch| ch.is_ascii_whitespace());
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
        self.skip_whitespace();
        match self.peek() {
            Some('n') => self.parse_null(),
            Some('t') | Some('f') => self.parse_bool(),
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
            Err(CJsonError::InvalidLiteral {
                expected: "true or false",
                pos: start,
            })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        let bytes = self.input.as_bytes();
        let mut sign = 1.0f64;
        let mut n = 0.0f64;
        let mut scale = 0.0f64;
        let mut subscale = 0i32;
        let mut signsubscale = 1.0f64;

        if bytes.get(self.pos) == Some(&b'-') {
            sign = -1.0;
            self.pos += 1;
        }

        if bytes.get(self.pos) == Some(&b'0') {
            self.pos += 1;
        }

        while let Some(digit @ b'1'..=b'9') | Some(digit @ b'0') = bytes.get(self.pos).copied() {
            n = (n * 10.0) + f64::from(digit - b'0');
            self.pos += 1;
        }

        if bytes.get(self.pos) == Some(&b'.')
            && matches!(bytes.get(self.pos + 1), Some(b'0'..=b'9'))
        {
            self.pos += 1;
            while let Some(digit @ b'0'..=b'9') = bytes.get(self.pos).copied() {
                n = (n * 10.0) + f64::from(digit - b'0');
                scale -= 1.0;
                self.pos += 1;
            }
        }

        if matches!(bytes.get(self.pos), Some(b'e' | b'E')) {
            self.pos += 1;
            if bytes.get(self.pos) == Some(&b'+') {
                self.pos += 1;
            } else if bytes.get(self.pos) == Some(&b'-') {
                signsubscale = -1.0;
                self.pos += 1;
            }
            while let Some(digit @ b'0'..=b'9') = bytes.get(self.pos).copied() {
                subscale = (subscale * 10) + i32::from(digit - b'0');
                self.pos += 1;
            }
        }

        if self.pos == start {
            return Err(CJsonError::InvalidNumber { pos: start });
        }

        Ok(CJson::Number(
            sign * n * 10f64.powf(scale + f64::from(subscale) * signsubscale),
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
                    let escape_pos = self.pos.saturating_sub(1);
                    let Some(escaped) = self.next_char() else {
                        return Err(CJsonError::UnexpectedEOF { pos: self.pos });
                    };
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
                            let uc = self.parse_hex4()?;
                            if (0xDC00..=0xDFFF).contains(&uc) || uc == 0 {
                                return Err(CJsonError::InvalidUnicodeEscape { pos: escape_pos });
                            }

                            let scalar = if (0xD800..=0xDBFF).contains(&uc) {
                                if self.next_char() != Some('\\') || self.next_char() != Some('u') {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: escape_pos });
                                }
                                let uc2 = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&uc2) {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: escape_pos });
                                }
                                0x10000 + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF))
                            } else {
                                uc
                            };

                            let Some(decoded) = char::from_u32(scalar) else {
                                return Err(CJsonError::InvalidUnicodeEscape { pos: escape_pos });
                            };
                            out.push(decoded);
                        }
                        _ => return Err(CJsonError::InvalidEscape { pos: escape_pos }),
                    }
                }
                ch => out.push(ch),
            }
        }
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        self.skip_whitespace();
        let mut items = Vec::new();

        if self.peek() == Some(']') {
            self.next_char();
            return Ok(CJson::Array(items));
        }

        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.next_char();
                    self.skip_whitespace();
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
        let mut members = HashMap::new();

        if self.peek() == Some('}') {
            self.next_char();
            return Ok(CJson::Object(members));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(CJsonError::ExpectedColon { pos: self.pos });
            }
            self.next_char();
            let value = self.parse_value()?;
            members.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.next_char();
                    self.skip_whitespace();
                }
                Some('}') => {
                    self.next_char();
                    return Ok(CJson::Object(members));
                }
                Some(_) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u32, CJsonError> {
        let start = self.pos;
        let mut value = 0u32;
        for _ in 0..4 {
            let Some(ch) = self.next_char() else {
                return Err(CJsonError::UnexpectedEOF { pos: self.pos });
            };
            let Some(digit) = ch.to_digit(16) else {
                return Err(CJsonError::InvalidUnicodeEscape { pos: start });
            };
            value = (value << 4) | digit;
        }
        Ok(value)
    }
}
pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
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
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 32 => {
                let _ = fmt::Write::write_fmt(&mut out, format_args!("\\u{:04x}", ch as u32));
            }
            ch => out.push(ch),
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
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    f.write_str(",")?;
                }
                write_json_compact(f, item)?;
            }
            f.write_str("]")
        }
        CJson::Object(map) => {
            f.write_str("{")?;
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
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
            if items.is_empty() {
                return f.write_str("[]");
            }
            f.write_str("[")?;
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    f.write_str(", ")?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            f.write_str("]")
        }
        CJson::Object(map) => {
            if map.is_empty() {
                f.write_str("{\n")?;
                return f.write_str("}");
            }

            f.write_str("{\n")?;
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            let next_indent = indent + 1;

            for (index, (key, item)) in entries.into_iter().enumerate() {
                for _ in 0..next_indent {
                    f.write_str("\t")?;
                }
                f.write_str(&escape_string(key))?;
                f.write_str(":\t")?;
                write_json_pretty(f, item, next_indent)?;
                if index + 1 != map.len() {
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
            CJson::Array(items) => Some(items.len()),
            CJson::Object(map) => Some(map.len()),
            _ => None,
        }
    }
    pub fn get_array_item(&self, index: usize) -> Option<&CJson> {
        match self {
            CJson::Array(items) => items.get(index),
            _ => None,
        }
    }
    pub fn get_object_item(&self, key: &str) -> Option<&CJson> {
        match self {
            CJson::Object(map) => map
                .iter()
                .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key))
                .map(|(_, value)| value),
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
            CJson::Array(items) => {
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
            CJson::Object(map) => {
                map.insert(key.into(), value);
                Ok(())
            }
            _ => Err("not an object"),
        }
    }
}
impl fmt::Display for CJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_json_pretty(f, self, 0)
    }
}

fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }

    let int_candidate = n as i32;
    if (f64::from(int_candidate) - n).abs() <= f64::EPSILON {
        return int_candidate.to_string();
    }

    if (n.floor() - n).abs() <= f64::EPSILON && n.abs() < 1.0e60 {
        return format!("{:.0}", n);
    }

    if n.abs() < 1.0e-6 || n.abs() > 1.0e9 {
        let exp = format!("{:.6e}", n);
        if let Some((mantissa, exponent)) = exp.split_once('e') {
            let parsed_exp = exponent.parse::<i32>().unwrap_or(0);
            return format!("{mantissa}e{parsed_exp:+03}");
        }
        return exp;
    }

    format!("{:.6}", n)
}

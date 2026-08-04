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
        while let Some(c) = self.peek() {
            // Mirror C's behavior: skip any byte <= 32 (space, tab, CR, LF, etc.)
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
            Some(c) => match c {
                'n' => self.parse_null(),
                't' | 'f' => self.parse_bool(),
                '"' => {
                    let s = self.parse_string()?;
                    Ok(CJson::String(s))
                }
                '[' => self.parse_array(),
                '{' => self.parse_object(),
                '-' | '0'..='9' => self.parse_number(),
                _ => Err(CJsonError::UnexpectedToken { ch: c, pos: self.pos }),
            },
        }
    }
    fn parse_null(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        if self.input[self.pos..].starts_with("null") {
            self.pos += 4;
            Ok(CJson::Null)
        } else {
            Err(CJsonError::InvalidLiteral { expected: "null", pos: start })
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
            Err(CJsonError::InvalidLiteral { expected: "true/false", pos: start })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        let s = self.take_while(|c| {
            matches!(c, '-' | '+' | '0'..='9' | '.' | 'e' | 'E')
        });
        match s.parse::<f64>() {
            Ok(n) => Ok(CJson::Number(n)),
            Err(_) => Err(CJsonError::InvalidNumber { pos: start }),
        }
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        // Expect opening quote
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
                        Some(c) => match c {
                            '"' => out.push('"'),
                            '\\' => out.push('\\'),
                            '/' => out.push('/'),
                            'b' => out.push('\u{0008}'),
                            'f' => out.push('\u{000C}'),
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
                            'u' => {
                                let uc = self.parse_hex4(esc_pos)?;
                                // Handle UTF-16 surrogate pairs
                                if (0xD800..=0xDBFF).contains(&uc) {
                                    // Need a low surrogate next
                                    if self.peek() != Some('\\') {
                                        // mimic C: just skip if missing
                                        continue;
                                    }
                                    let save = self.pos;
                                    self.next_char(); // consume '\\'
                                    if self.peek() != Some('u') {
                                        self.pos = save;
                                        continue;
                                    }
                                    self.next_char(); // consume 'u'
                                    let uc2 = self.parse_hex4(esc_pos)?;
                                    if !(0xDC00..=0xDFFF).contains(&uc2) {
                                        continue;
                                    }
                                    let combined = 0x10000
                                        + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF));
                                    if let Some(ch) = char::from_u32(combined) {
                                        out.push(ch);
                                    }
                                } else if (0xDC00..=0xDFFF).contains(&uc) || uc == 0 {
                                    // Invalid lone low surrogate or null - skip
                                } else if let Some(ch) = char::from_u32(uc) {
                                    out.push(ch);
                                } else {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: esc_pos });
                                }
                            }
                            other => {
                                // Mirror C's default: include the literal char
                                out.push(other);
                            }
                        },
                    }
                }
                Some(c) => out.push(c),
            }
        }
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        self.skip_whitespace();
        let mut items = Vec::new();
        if let Some(']') = self.peek() {
            self.pos += 1;
            return Ok(CJson::Array(items));
        }
        loop {
            let v = self.parse_value()?;
            items.push(v);
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
                Some(c) => {
                    return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos })
                        .map_err(|_| CJsonError::UnexpectedToken { ch: c, pos: self.pos });
                }
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
    fn parse_object(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('{')?;
        self.skip_whitespace();
        let mut map = HashMap::new();
        if let Some('}') = self.peek() {
            self.pos += 1;
            return Ok(CJson::Object(map));
        }
        loop {
            self.skip_whitespace();
            // Key
            if self.peek() != Some('"') {
                return Err(CJsonError::UnexpectedToken {
                    ch: self.peek().unwrap_or('\0'),
                    pos: self.pos,
                });
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            // Colon
            if self.peek() != Some(':') {
                return Err(CJsonError::ExpectedColon { pos: self.pos });
            }
            self.pos += 1;
            self.skip_whitespace();
            // Value
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                    self.skip_whitespace();
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(CJson::Object(map));
                }
                Some(_) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
}

impl<'a> Parser<'a> {
    fn parse_hex4(&mut self, err_pos: usize) -> Result<u32, CJsonError> {
        let mut h: u32 = 0;
        for _ in 0..4 {
            let c = self
                .next_char()
                .ok_or(CJsonError::InvalidUnicodeEscape { pos: err_pos })?;
            let d = match c {
                '0'..='9' => c as u32 - '0' as u32,
                'a'..='f' => 10 + c as u32 - 'a' as u32,
                'A'..='F' => 10 + c as u32 - 'A' as u32,
                _ => return Err(CJsonError::InvalidUnicodeEscape { pos: err_pos }),
            };
            h = (h << 4) | d;
        }
        Ok(h)
    }
}

pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let value = parser.parse_value()?;
    if require_end {
        parser.skip_whitespace();
        if parser.peek().is_some() {
            return Err(CJsonError::UnexpectedToken {
                ch: parser.peek().unwrap(),
                pos: parser.pos,
            });
        }
    }
    Ok(value)
}
fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
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

fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    let as_int = n as i64;
    // Integer-like check: matches C's "fabs(((double)item->valueint)-d) <= DBL_EPSILON
    // && d <= INT_MAX && d >= INT_MIN" check (using i32 range).
    let int_val = n as i32;
    if ((int_val as f64) - n).abs() <= f64::EPSILON
        && n <= i32::MAX as f64
        && n >= i32::MIN as f64
    {
        return format!("{}", int_val);
    }
    let _ = as_int;

    let abs_n = n.abs();
    if (n.floor() - n).abs() <= f64::EPSILON && abs_n < 1.0e60 {
        // %.0f equivalent
        format!("{:.0}", n)
    } else if abs_n < 1.0e-6 || abs_n > 1.0e9 {
        // %e equivalent (C default precision = 6)
        format_exponential(n)
    } else {
        // %f equivalent (default precision = 6)
        format!("{:.6}", n)
    }
}

fn format_exponential(n: f64) -> String {
    // C-style %e: 1.234560e+10
    let s = format!("{:e}", n);
    // Rust: "1.23456e10" or "-1.23456e-10"
    // We need: "1.234560e+10"
    if let Some(e_pos) = s.find('e') {
        let (mantissa, exp) = s.split_at(e_pos);
        let exp = &exp[1..]; // skip 'e'
        // Pad mantissa to have 6 digits after the decimal point
        let mantissa = if mantissa.contains('.') {
            let (intp, frac) = mantissa.split_once('.').unwrap();
            let mut frac = frac.to_string();
            while frac.len() < 6 {
                frac.push('0');
            }
            frac.truncate(6);
            format!("{}.{}", intp, frac)
        } else {
            format!("{}.000000", mantissa)
        };
        let (sign, digits) = if let Some(stripped) = exp.strip_prefix('-') {
            ('-', stripped)
        } else if let Some(stripped) = exp.strip_prefix('+') {
            ('+', stripped)
        } else {
            ('+', exp)
        };
        let mut padded_digits = digits.to_string();
        while padded_digits.len() < 2 {
            padded_digits.insert(0, '0');
        }
        format!("{}e{}{}", mantissa, sign, padded_digits)
    } else {
        s
    }
}

fn write_value_compact(out: &mut String, value: &CJson) {
    match value {
        CJson::Null => out.push_str("null"),
        CJson::Bool(true) => out.push_str("true"),
        CJson::Bool(false) => out.push_str("false"),
        CJson::Number(n) => out.push_str(&format_number(*n)),
        CJson::String(s) => out.push_str(&escape_string(s)),
        CJson::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value_compact(out, v);
            }
            out.push(']');
        }
        CJson::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&escape_string(k));
                out.push(':');
                write_value_compact(out, v);
            }
            out.push('}');
        }
    }
}

fn write_value_pretty(out: &mut String, value: &CJson, depth: usize) {
    match value {
        CJson::Null => out.push_str("null"),
        CJson::Bool(true) => out.push_str("true"),
        CJson::Bool(false) => out.push_str("false"),
        CJson::Number(n) => out.push_str(&format_number(*n)),
        CJson::String(s) => out.push_str(&escape_string(s)),
        CJson::Array(arr) => {
            if arr.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                write_value_pretty(out, v, depth + 1);
                if i + 1 < arr.len() {
                    out.push_str(", ");
                }
            }
            out.push(']');
        }
        CJson::Object(map) => {
            if map.is_empty() {
                out.push('{');
                out.push('\n');
                for _ in 0..depth.saturating_sub(0) {
                    out.push('\t');
                }
                out.push('}');
                return;
            }
            out.push('{');
            out.push('\n');
            let new_depth = depth + 1;
            let entries: Vec<_> = map.iter().collect();
            for (i, (k, v)) in entries.iter().enumerate() {
                for _ in 0..new_depth {
                    out.push('\t');
                }
                out.push_str(&escape_string(k));
                out.push_str(":\t");
                write_value_pretty(out, v, new_depth);
                if i + 1 < entries.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            for _ in 0..new_depth.saturating_sub(1) {
                out.push('\t');
            }
            out.push('}');
        }
    }
}

fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    let mut s = String::new();
    write_value_compact(&mut s, value);
    f.write_str(&s)
}
fn write_json_pretty(f: &mut impl fmt::Write, value: &CJson, indent: usize) -> fmt::Result {
    let mut s = String::new();
    write_value_pretty(&mut s, value, indent);
    f.write_str(&s)
}
impl CJson {
    pub fn print_unformatted(&self) -> String {
        let mut s = String::new();
        write_value_compact(&mut s, self);
        s
    }
    pub fn print_formatted(&self) -> String {
        let mut s = String::new();
        write_value_pretty(&mut s, self, 0);
        s
    }
    pub fn get_array_size(&self) -> Option<usize> {
        match self {
            CJson::Array(a) => Some(a.len()),
            CJson::Object(o) => Some(o.len()),
            _ => None,
        }
    }
    pub fn get_array_item(&self, index: usize) -> Option<&CJson> {
        match self {
            CJson::Array(a) => a.get(index),
            _ => None,
        }
    }
    pub fn get_object_item(&self, key: &str) -> Option<&CJson> {
        match self {
            CJson::Object(o) => {
                // Case-insensitive lookup like cJSON_GetObjectItem
                for (k, v) in o.iter() {
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
            CJson::Array(a) => {
                a.push(item);
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
            CJson::Object(o) => {
                o.insert(key.into(), value);
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

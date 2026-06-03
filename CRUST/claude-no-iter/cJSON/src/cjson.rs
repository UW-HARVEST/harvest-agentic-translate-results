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
                write!(f, "Unexpected end of input at position {}", pos)
            }
            CJsonError::UnexpectedToken { ch, pos } => {
                write!(f, "Unexpected token '{}' at position {}", ch, pos)
            }
            CJsonError::InvalidLiteral { expected, pos } => write!(
                f,
                "Invalid literal, expected '{}' at position {}",
                expected, pos
            ),
            CJsonError::InvalidNumber { pos } => {
                write!(f, "Invalid number at position {}", pos)
            }
            CJsonError::InvalidEscape { pos } => {
                write!(f, "Invalid escape sequence at position {}", pos)
            }
            CJsonError::InvalidUnicodeEscape { pos } => {
                write!(f, "Invalid unicode escape sequence at position {}", pos)
            }
            CJsonError::ExpectedColon { pos } => {
                write!(f, "Expected ':' at position {}", pos)
            }
            CJsonError::ExpectedCommaOrEnd { pos } => {
                write!(f, "Expected ',' or end of collection at position {}", pos)
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
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
    fn take_while<F>(&mut self, mut predicate: F) -> &'a str
    where
        F: FnMut(char) -> bool,
    {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if !predicate(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        &self.input[start..self.pos]
    }
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            // Match C's `skip`: any byte <= 32 (space, tab, CR, LF, etc.)
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
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        // integer part
        let _int = self.take_while(|c| c.is_ascii_digit());
        // fractional
        if self.peek() == Some('.') {
            self.pos += 1;
            let _frac = self.take_while(|c| c.is_ascii_digit());
        }
        // exponent
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.pos += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            let _exp = self.take_while(|c| c.is_ascii_digit());
        }
        let num_str = &self.input[start..self.pos];
        if num_str.is_empty() || num_str == "-" {
            return Err(CJsonError::InvalidNumber { pos: start });
        }
        num_str
            .parse::<f64>()
            .map(CJson::Number)
            .map_err(|_| CJsonError::InvalidNumber { pos: start })
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        self.expect_char('"')?;
        let mut s = String::new();
        loop {
            let cur_pos = self.pos;
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                Some('"') => return Ok(s),
                Some('\\') => {
                    let esc_pos = cur_pos;
                    match self.next_char() {
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some('/') => s.push('/'),
                        Some('b') => s.push('\u{0008}'),
                        Some('f') => s.push('\u{000C}'),
                        Some('n') => s.push('\n'),
                        Some('r') => s.push('\r'),
                        Some('t') => s.push('\t'),
                        Some('u') => {
                            let cp = self.parse_hex4()?;
                            // Lone low surrogate or NUL: skip per cJSON behavior
                            if (0xDC00..=0xDFFF).contains(&cp) || cp == 0 {
                                continue;
                            }
                            let final_cp = if (0xD800..=0xDBFF).contains(&cp) {
                                // high surrogate, expect low surrogate
                                if self.peek() != Some('\\') {
                                    continue;
                                }
                                self.pos += 1;
                                if self.peek() != Some('u') {
                                    continue;
                                }
                                self.pos += 1;
                                let cp2 = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&cp2) {
                                    continue;
                                }
                                0x10000 + (((cp - 0xD800) << 10) | (cp2 - 0xDC00))
                            } else {
                                cp
                            };
                            match char::from_u32(final_cp) {
                                Some(c) => s.push(c),
                                None => {
                                    return Err(CJsonError::InvalidUnicodeEscape {
                                        pos: esc_pos,
                                    });
                                }
                            }
                        }
                        Some(other) => {
                            // Match C's permissive default: copy literal char
                            s.push(other);
                        }
                        None => {
                            return Err(CJsonError::UnexpectedEOF { pos: self.pos });
                        }
                    }
                }
                Some(c) => s.push(c),
            }
        }
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        let mut arr: Vec<CJson> = Vec::new();
        self.skip_whitespace();
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
        let mut map: HashMap<String, CJson> = HashMap::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(CJson::Object(map));
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
        let start = self.pos;
        let mut h: u32 = 0;
        for _ in 0..4 {
            let c = match self.next_char() {
                Some(c) => c,
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            };
            let v = c
                .to_digit(16)
                .ok_or(CJsonError::InvalidUnicodeEscape { pos: start })?;
            h = (h << 4) | v;
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
        if parser.pos < parser.input.len() {
            return Err(CJsonError::UnexpectedToken {
                ch: parser.peek().unwrap_or(' '),
                pos: parser.pos,
            });
        }
    }
    Ok(value)
}

fn escape_string(s: &str) -> String {
    let mut out = String::new();
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
    if !d.is_finite() {
        return "0".to_string();
    }
    if d == 0.0 {
        return "0".to_string();
    }
    // If it round-trips as an integer, render as integer.
    if d.abs() < 1.0e16 {
        let i = d as i64;
        if (i as f64) == d {
            return format!("{}", i);
        }
    }
    // Otherwise use Rust's default Display, which yields a representation
    // that round-trips through f64::from_str and is valid JSON.
    let s = format!("{}", d);
    // Ensure we did not produce something that lacks a fractional or exponent
    // marker (defensive — Rust's Display for non-integer f64 always includes
    // a '.' or 'e').
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        return format!("{}.0", s);
    }
    s
}

fn write_value_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Number(n) => f.write_str(&format_number(*n)),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(arr) => {
            f.write_char('[')?;
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_char(',')?;
                }
                write_value_compact(f, v)?;
            }
            f.write_char(']')
        }
        CJson::Object(map) => {
            f.write_char('{')?;
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    f.write_char(',')?;
                }
                f.write_str(&escape_string(k))?;
                f.write_char(':')?;
                write_value_compact(f, v)?;
            }
            f.write_char('}')
        }
    }
}

fn write_value_pretty(
    f: &mut impl fmt::Write,
    value: &CJson,
    depth: usize,
) -> fmt::Result {
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
            f.write_char('[')?;
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write_value_pretty(f, v, depth + 1)?;
            }
            f.write_char(']')
        }
        CJson::Object(map) => {
            if map.is_empty() {
                f.write_char('{')?;
                f.write_char('\n')?;
                for _ in 0..depth.saturating_sub(0) {
                    // C writes depth-1 tabs (but only if depth >= 1).
                    // For our top-level call depth is 0, so write nothing.
                }
                f.write_char('}')?;
                return Ok(());
            }
            f.write_char('{')?;
            f.write_char('\n')?;
            let new_depth = depth + 1;
            let len = map.len();
            for (i, (k, v)) in map.iter().enumerate() {
                for _ in 0..new_depth {
                    f.write_char('\t')?;
                }
                f.write_str(&escape_string(k))?;
                f.write_str(":\t")?;
                write_value_pretty(f, v, new_depth)?;
                if i + 1 < len {
                    f.write_char(',')?;
                }
                f.write_char('\n')?;
            }
            for _ in 0..depth {
                f.write_char('\t')?;
            }
            f.write_char('}')
        }
    }
}

fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    write_value_compact(f, value)
}

fn write_json_pretty(f: &mut impl fmt::Write, value: &CJson, indent: usize) -> fmt::Result {
    write_value_pretty(f, value, indent)
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
            CJson::Object(map) => Some(map.len()),
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
            CJson::Object(map) => {
                // Case-insensitive lookup, mirroring cJSON_GetObjectItem.
                if let Some(v) = map.get(key) {
                    return Some(v);
                }
                for (k, v) in map.iter() {
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
            _ => Err("add_item_to_array: receiver is not an array"),
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
            _ => Err("add_item_to_object: receiver is not an object"),
        }
    }
}
impl fmt::Display for CJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value_compact(f, self)
    }
}

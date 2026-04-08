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
            Err(CJsonError::InvalidLiteral { expected: "true or false", pos })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        // Match the C parse_number logic exactly
        let mut n: f64 = 0.0;
        let mut sign: f64 = 1.0;
        let mut scale: f64 = 0.0;
        let mut subscale: i32 = 0;
        let mut signsubscale: i32 = 1;

        if self.peek() == Some('-') {
            sign = -1.0;
            self.pos += 1;
        }
        if self.peek() == Some('0') {
            self.pos += 1;
        } else if matches!(self.peek(), Some('1'..='9')) {
            while let Some(ch @ '0'..='9') = self.peek() {
                n = n * 10.0 + (ch as u8 - b'0') as f64;
                self.pos += 1;
            }
        }
        if self.peek() == Some('.') {
            let next = self.input[self.pos + 1..].chars().next();
            if matches!(next, Some('0'..='9')) {
                self.pos += 1; // skip '.'
                while let Some(ch @ '0'..='9') = self.peek() {
                    n = n * 10.0 + (ch as u8 - b'0') as f64;
                    scale -= 1.0;
                    self.pos += 1;
                }
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.pos += 1;
            if self.peek() == Some('+') {
                self.pos += 1;
            } else if self.peek() == Some('-') {
                signsubscale = -1;
                self.pos += 1;
            }
            while let Some(ch @ '0'..='9') = self.peek() {
                subscale = subscale * 10 + (ch as u8 - b'0') as i32;
                self.pos += 1;
            }
        }

        if self.pos == start || (self.pos == start + 1 && sign < 0.0 && n == 0.0 && scale == 0.0 && subscale == 0) {
            // Check we actually consumed something meaningful
            if self.pos == start {
                return Err(CJsonError::InvalidNumber { pos: start });
            }
        }

        let result = sign * n * 10.0_f64.powf(scale + (subscale * signsubscale) as f64);
        Ok(CJson::Number(result))
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        let pos = self.pos;
        self.expect_char('"').map_err(|_| CJsonError::UnexpectedToken {
            ch: self.input[pos..].chars().next().unwrap_or('\0'),
            pos,
        })?;

        let mut result = std::string::String::new();
        loop {
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                Some('"') => return Ok(result),
                Some('\\') => {
                    let esc_pos = self.pos;
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
                            let uc = self.parse_hex4().map_err(|_| CJsonError::InvalidUnicodeEscape { pos: esc_pos })?;
                            if (0xDC00..=0xDFFF).contains(&uc) || uc == 0 {
                                // invalid lone low surrogate or null - C code just breaks (skips)
                                continue;
                            }
                            let code_point;
                            if (0xD800..=0xDBFF).contains(&uc) {
                                // high surrogate, expect \uXXXX
                                if self.peek() == Some('\\') {
                                    self.pos += 1;
                                    if self.peek() == Some('u') {
                                        self.pos += 1;
                                        let uc2 = self.parse_hex4().map_err(|_| CJsonError::InvalidUnicodeEscape { pos: esc_pos })?;
                                        if (0xDC00..=0xDFFF).contains(&uc2) {
                                            code_point = 0x10000 + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF));
                                        } else {
                                            // invalid second half
                                            continue;
                                        }
                                    } else {
                                        // missing 'u' after backslash
                                        self.pos -= 1; // put back the backslash
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            } else {
                                code_point = uc;
                            }
                            if let Some(ch) = char::from_u32(code_point) {
                                result.push(ch);
                            }
                        }
                        Some(other) => result.push(other),
                    }
                }
                Some(ch) => result.push(ch),
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u32, CJsonError> {
        let pos = self.pos;
        let mut h: u32 = 0;
        for _ in 0..4 {
            match self.next_char() {
                Some(ch @ '0'..='9') => h = (h << 4) + (ch as u32 - '0' as u32),
                Some(ch @ 'A'..='F') => h = (h << 4) + 10 + (ch as u32 - 'A' as u32),
                Some(ch @ 'a'..='f') => h = (h << 4) + 10 + (ch as u32 - 'a' as u32),
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
        items.push(self.parse_value()?);
        self.skip_whitespace();
        while self.peek() == Some(',') {
            self.pos += 1;
            items.push(self.parse_value()?);
            self.skip_whitespace();
        }
        if self.peek() == Some(']') {
            self.pos += 1;
            Ok(CJson::Array(items))
        } else {
            Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos })
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
        // Use a vec to preserve insertion order for printing (HashMap doesn't guarantee order,
        // but the C code uses a linked list which preserves insertion order)
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_char(':').map_err(|_| CJsonError::ExpectedColon { pos: self.pos })?;
            let value = self.parse_value()?;
            self.skip_whitespace();
            map.insert(key, value);
            if self.peek() == Some(',') {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.peek() == Some('}') {
            self.pos += 1;
            Ok(CJson::Object(map))
        } else {
            Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos })
        }
    }
}
pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    if require_end {
        parser.skip_whitespace();
        if parser.pos < parser.input.len() {
            let ch = parser.input[parser.pos..].chars().next().unwrap();
            return Err(CJsonError::UnexpectedToken { ch, pos: parser.pos });
        }
    }
    Ok(value)
}
fn escape_string(s: &str) -> String {
    let mut out = std::string::String::with_capacity(s.len() + 2);
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

fn print_number(d: f64) -> String {
    // Match C print_number behavior
    let i = d as i32;
    if d == 0.0 {
        "0".to_string()
    } else if (i as f64 - d).abs() <= f64::EPSILON && d <= i32::MAX as f64 && d >= i32::MIN as f64 {
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
        CJson::Number(n) => write!(f, "{}", print_number(*n)),
        CJson::String(s) => write!(f, "{}", escape_string(s)),
        CJson::Array(items) => {
            write!(f, "[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write_json_compact(f, item)?;
            }
            write!(f, "]")
        }
        CJson::Object(map) => {
            write!(f, "{{")?;
            let mut first = true;
            for (key, val) in map {
                if !first {
                    write!(f, ", ")?;
                }
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
        CJson::Number(n) => write!(f, "{}", print_number(*n)),
        CJson::String(s) => write!(f, "{}", escape_string(s)),
        CJson::Array(items) => {
            if items.is_empty() {
                return write!(f, "[]");
            }
            write!(f, "[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, "\n")?;
                for _ in 0..=indent {
                    write!(f, "\t")?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            write!(f, "\n")?;
            for _ in 0..indent {
                write!(f, "\t")?;
            }
            write!(f, "]")
        }
        CJson::Object(map) => {
            if map.is_empty() {
                write!(f, "{{\n")?;
                for _ in 0..indent.saturating_sub(1) {
                    write!(f, "\t")?;
                }
                return write!(f, "}}");
            }
            write!(f, "{{\n")?;
            let entries: Vec<_> = map.iter().collect();
            for (i, (key, val)) in entries.iter().enumerate() {
                for _ in 0..=indent {
                    write!(f, "\t")?;
                }
                write!(f, "{}:\t", escape_string(key))?;
                write_json_pretty(f, val, indent + 1)?;
                if i < entries.len() - 1 {
                    write!(f, ",")?;
                }
                write!(f, "\n")?;
            }
            for _ in 0..indent {
                write!(f, "\t")?;
            }
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
                // C version uses case-insensitive comparison
                let key_lower = key.to_ascii_lowercase();
                map.iter()
                    .find(|(k, _)| k.to_ascii_lowercase() == key_lower)
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
            CJson::Array(v) => {
                v.push(item);
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
        write_json_compact(f, self)
    }
}

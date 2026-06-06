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
            CJsonError::InvalidLiteral { expected, pos } => {
                write!(f, "Invalid literal, expected '{}' at position {}", expected, pos)
            }
            CJsonError::InvalidNumber { pos } => {
                write!(f, "Invalid number at position {}", pos)
            }
            CJsonError::InvalidEscape { pos } => {
                write!(f, "Invalid escape sequence at position {}", pos)
            }
            CJsonError::InvalidUnicodeEscape { pos } => {
                write!(f, "Invalid unicode escape at position {}", pos)
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
            if !predicate(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        &self.input[start..self.pos]
    }
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            // Mirror C's `skip`: any byte <= 32 is whitespace.
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
            Some('"') => Ok(CJson::String(self.parse_string()?)),
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
        let num_str = self.take_while(|c| {
            matches!(c, '0'..='9' | '-' | '+' | '.' | 'e' | 'E')
        });
        match num_str.parse::<f64>() {
            Ok(n) => Ok(CJson::Number(n)),
            Err(_) => Err(CJsonError::InvalidNumber { pos: start }),
        }
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        self.expect_char('"')?;
        let mut result = String::new();
        loop {
            let pos_before = self.pos;
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                Some('"') => return Ok(result),
                Some('\\') => {
                    match self.next_char() {
                        None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                        Some('"') => result.push('"'),
                        Some('\\') => result.push('\\'),
                        Some('/') => result.push('/'),
                        Some('b') => result.push('\u{0008}'),
                        Some('f') => result.push('\u{000C}'),
                        Some('n') => result.push('\n'),
                        Some('r') => result.push('\r'),
                        Some('t') => result.push('\t'),
                        Some('u') => {
                            let uc = self.parse_hex4()?;
                            // Skip invalid (low surrogate without high, or NUL)
                            // matching C's behavior of `break;`.
                            if (0xDC00..=0xDFFF).contains(&uc) || uc == 0 {
                                continue;
                            }
                            let code_point = if (0xD800..=0xDBFF).contains(&uc) {
                                // High surrogate -- expect low surrogate next.
                                if self.peek() != Some('\\') {
                                    continue;
                                }
                                self.next_char(); // consume '\\'
                                if self.peek() != Some('u') {
                                    continue;
                                }
                                self.next_char(); // consume 'u'
                                let uc2 = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&uc2) {
                                    continue;
                                }
                                0x10000 + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF))
                            } else {
                                uc
                            };
                            if let Some(c) = char::from_u32(code_point) {
                                result.push(c);
                            } else {
                                return Err(CJsonError::InvalidUnicodeEscape { pos: pos_before });
                            }
                        }
                        Some(other) => {
                            // Match C: default copies the literal char after the backslash.
                            result.push(other);
                        }
                    }
                }
                Some(c) => result.push(c),
            }
        }
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        self.skip_whitespace();
        let mut arr: Vec<CJson> = Vec::new();
        if self.peek() == Some(']') {
            self.next_char();
            return Ok(CJson::Array(arr));
        }
        loop {
            self.skip_whitespace();
            let value = self.parse_value()?;
            arr.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.next_char();
                    continue;
                }
                Some(']') => {
                    self.next_char();
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
        let mut map: HashMap<String, CJson> = HashMap::new();
        if self.peek() == Some('}') {
            self.next_char();
            return Ok(CJson::Object(map));
        }
        loop {
            self.skip_whitespace();
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
                    continue;
                }
                Some('}') => {
                    self.next_char();
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
        let mut value: u32 = 0;
        for _ in 0..4 {
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                Some(c) => {
                    value <<= 4;
                    if let Some(d) = c.to_digit(16) {
                        value |= d;
                    } else {
                        return Err(CJsonError::InvalidUnicodeEscape { pos: start });
                    }
                }
            }
        }
        Ok(value)
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

fn format_number(n: f64) -> String {
    // Mirror cJSON's print_number behavior closely.
    if n == 0.0 {
        return "0".to_string();
    }
    // Truncate-towards-zero conversion, like C's `(int)num`.
    let int_truncated = n.trunc();
    if int_truncated >= i32::MIN as f64
        && int_truncated <= i32::MAX as f64
        && (int_truncated - n).abs() <= f64::EPSILON
    {
        return format!("{}", int_truncated as i32);
    }
    if (n.floor() - n).abs() <= f64::EPSILON && n.abs() < 1e60 {
        return format!("{:.0}", n);
    }
    if n.abs() < 1e-6 || n.abs() > 1e9 {
        return format_scientific(n);
    }
    // C "%f" style: 6 digits after the decimal.
    format!("{:.6}", n)
}

fn format_scientific(n: f64) -> String {
    // Approximate C's "%e" format: m.dddddde[+-]NN with 6 digits after the
    // decimal and at least 2-digit exponent.
    if n == 0.0 {
        return "0.000000e+00".to_string();
    }
    let neg = n < 0.0;
    let abs_n = n.abs();
    let exp = abs_n.log10().floor() as i32;
    let mantissa = abs_n / 10f64.powi(exp);
    let sign_char = if neg { "-" } else { "" };
    let exp_sign = if exp < 0 { '-' } else { '+' };
    format!("{}{:.6}e{}{:02}", sign_char, mantissa, exp_sign, exp.abs())
}

fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{0008}' => result.push_str("\\b"),
            '\u{000C}' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if (c as u32) < 32 => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
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
        CJson::Object(map) => {
            f.write_str("{")?;
            for (i, (k, v)) in map.iter().enumerate() {
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

fn write_json_pretty(f: &mut impl fmt::Write, value: &CJson, depth: usize) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Number(n) => f.write_str(&format_number(*n)),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(arr) => {
            // C's print_array always renders arrays inline: "[a, b, c]".
            if arr.is_empty() {
                return f.write_str("[]");
            }
            f.write_str("[")?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write_json_pretty(f, item, depth + 1)?;
            }
            f.write_str("]")
        }
        CJson::Object(map) => {
            if map.is_empty() {
                // Mirror C: "{\n" + (depth-1) tabs + "}"
                f.write_str("{\n")?;
                for _ in 0..depth.saturating_sub(1) {
                    f.write_str("\t")?;
                }
                return f.write_str("}");
            }
            f.write_str("{\n")?;
            let inner_depth = depth + 1;
            let entries: Vec<(&String, &CJson)> = map.iter().collect();
            let last = entries.len() - 1;
            for (i, (k, v)) in entries.iter().enumerate() {
                for _ in 0..inner_depth {
                    f.write_str("\t")?;
                }
                f.write_str(&escape_string(k))?;
                f.write_str(":\t")?;
                write_json_pretty(f, v, inner_depth)?;
                if i < last {
                    f.write_str(",")?;
                }
                f.write_str("\n")?;
            }
            for _ in 0..depth {
                f.write_str("\t")?;
            }
            f.write_str("}")
        }
    }
}

impl CJson {
    pub fn print_unformatted(&self) -> String {
        let mut out = String::new();
        write_json_compact(&mut out, self).expect("writing to String never fails");
        out
    }
    pub fn print_formatted(&self) -> String {
        let mut out = String::new();
        write_json_pretty(&mut out, self, 0).expect("writing to String never fails");
        out
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
                // Mirror cJSON_GetObjectItem: case-insensitive key lookup.
                let lower = key.to_ascii_lowercase();
                map.iter()
                    .find(|(k, _)| k.to_ascii_lowercase() == lower)
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

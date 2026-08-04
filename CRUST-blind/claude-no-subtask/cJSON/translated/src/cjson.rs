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
                write!(f, "invalid unicode escape sequence at position {}", pos)
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
            if !predicate(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        &self.input[start..self.pos]
    }
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            // Match C behavior: any byte <= 32 is treated as whitespace.
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
            Err(CJsonError::InvalidLiteral { expected: "null", pos: self.pos })
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
            Err(CJsonError::InvalidLiteral { expected: "true or false", pos: self.pos })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        // Integer part
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        // Fractional part
        if self.peek() == Some('.') {
            self.pos += 1;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        // Exponent part
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.pos += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
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
        let mut result = String::new();
        loop {
            let pos_before = self.pos;
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                Some('"') => return Ok(result),
                Some('\\') => {
                    let esc_pos = self.pos;
                    match self.next_char() {
                        None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                        Some('"') => result.push('"'),
                        Some('\\') => result.push('\\'),
                        Some('/') => result.push('/'),
                        Some('b') => result.push('\u{08}'),
                        Some('f') => result.push('\u{0C}'),
                        Some('n') => result.push('\n'),
                        Some('r') => result.push('\r'),
                        Some('t') => result.push('\t'),
                        Some('u') => {
                            let uc = self.parse_hex4()?;
                            if (0xD800..=0xDBFF).contains(&uc) {
                                // High surrogate; expect low surrogate next
                                if self.peek() != Some('\\') {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos });
                                }
                                self.pos += 1;
                                if self.peek() != Some('u') {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos });
                                }
                                self.pos += 1;
                                let uc2 = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&uc2) {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos });
                                }
                                let cp = 0x10000
                                    + (((uc - 0xD800) << 10) | (uc2 - 0xDC00));
                                match char::from_u32(cp) {
                                    Some(ch) => result.push(ch),
                                    None => return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos }),
                                }
                            } else if (0xDC00..=0xDFFF).contains(&uc) {
                                return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos });
                            } else {
                                match char::from_u32(uc) {
                                    Some(ch) => result.push(ch),
                                    None => return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos }),
                                }
                            }
                        }
                        Some(_) => return Err(CJsonError::InvalidEscape { pos: esc_pos }),
                    }
                }
                Some(c) => {
                    let _ = pos_before;
                    result.push(c)
                }
            }
        }
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        let mut items: Vec<CJson> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(CJson::Array(items));
        }
        loop {
            self.skip_whitespace();
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
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
        let mut items: HashMap<String, CJson> = HashMap::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(CJson::Object(items));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some('"') {
                return Err(CJsonError::UnexpectedToken {
                    ch: self.peek().unwrap_or('\0'),
                    pos: self.pos,
                });
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(CJsonError::ExpectedColon { pos: self.pos });
            }
            self.pos += 1;
            self.skip_whitespace();
            let value = self.parse_value()?;
            items.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(CJson::Object(items));
                }
                Some(_) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
}
impl<'a> Parser<'a> {
    fn parse_hex4(&mut self) -> Result<u32, CJsonError> {
        let mut h: u32 = 0;
        for _ in 0..4 {
            let c = match self.next_char() {
                Some(c) => c,
                None => return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos }),
            };
            let digit = match c {
                '0'..='9' => (c as u32) - ('0' as u32),
                'a'..='f' => 10 + (c as u32) - ('a' as u32),
                'A'..='F' => 10 + (c as u32) - ('A' as u32),
                _ => return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos }),
            };
            h = (h << 4) | digit;
        }
        Ok(h)
    }
}
pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    if require_end {
        parser.skip_whitespace();
        if parser.pos < parser.input.len() {
            let ch = parser.peek().unwrap_or('\0');
            return Err(CJsonError::UnexpectedToken { ch, pos: parser.pos });
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
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
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
    let as_int = n as i32;
    let int_diff = (as_int as f64 - n).abs();
    // Match the C behavior: if the value fits in an int and is essentially
    // integer-valued, print as integer.
    if int_diff <= f64::EPSILON && n <= i32::MAX as f64 && n >= i32::MIN as f64 {
        return format!("{}", as_int);
    }
    let abs = n.abs();
    if (n.floor() - n).abs() <= f64::EPSILON && abs < 1.0e60 {
        // %.0f equivalent (no decimal point)
        format!("{:.0}", n)
    } else if abs < 1.0e-6 || abs > 1.0e9 {
        // %e equivalent: C printf %e produces e.g. 1.234500e+02
        format_e(n)
    } else {
        // %f equivalent: 6 decimal places
        format!("{:.6}", n)
    }
}
fn format_e(n: f64) -> String {
    // Emulate C printf "%e" which produces e.g. "1.234500e+02".
    if n == 0.0 {
        return "0.000000e+00".to_string();
    }
    let sign = if n < 0.0 { "-" } else { "" };
    let abs = n.abs();
    let exp = abs.log10().floor() as i32;
    let mantissa = abs / 10f64.powi(exp);
    // Handle floating point edge that may push mantissa to 10.0 due to rounding.
    let (mantissa, exp) = if mantissa >= 10.0 {
        (mantissa / 10.0, exp + 1)
    } else if mantissa < 1.0 {
        (mantissa * 10.0, exp - 1)
    } else {
        (mantissa, exp)
    };
    let exp_sign = if exp < 0 { '-' } else { '+' };
    let exp_abs = exp.abs();
    format!("{}{:.6}e{}{:02}", sign, mantissa, exp_sign, exp_abs)
}
fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Number(n) => f.write_str(&format_number(*n)),
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
        CJson::Number(n) => f.write_str(&format_number(*n)),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(arr) => {
            if arr.is_empty() {
                return f.write_str("[]");
            }
            f.write_char('[')?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_char(',')?;
                    f.write_char(' ')?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            f.write_char(']')
        }
        CJson::Object(obj) => {
            if obj.is_empty() {
                f.write_char('{')?;
                f.write_char('\n')?;
                if indent >= 1 {
                    for _ in 0..(indent.saturating_sub(1)) {
                        f.write_char('\t')?;
                    }
                }
                f.write_char('}')?;
                return Ok(());
            }
            f.write_char('{')?;
            f.write_char('\n')?;
            let inner_indent = indent + 1;
            let entries: Vec<(&String, &CJson)> = obj.iter().collect();
            for (i, (k, v)) in entries.iter().enumerate() {
                for _ in 0..inner_indent {
                    f.write_char('\t')?;
                }
                f.write_str(&escape_string(k))?;
                f.write_char(':')?;
                f.write_char('\t')?;
                write_json_pretty(f, v, inner_indent)?;
                if i + 1 != entries.len() {
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
        let mut s = String::new();
        let _ = write_json_compact(&mut s, self);
        s
    }
    pub fn print_formatted(&self) -> String {
        let mut s = String::new();
        let _ = write_json_pretty(&mut s, self, 0);
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
                // Match C cJSON which performs case-insensitive lookup.
                if let Some(v) = o.get(key) {
                    return Some(v);
                }
                let key_lower = key.to_ascii_lowercase();
                for (k, v) in o.iter() {
                    if k.to_ascii_lowercase() == key_lower {
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
        write_json_pretty(f, self, 0)
    }
}

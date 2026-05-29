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
                write!(f, "Expected ',' or end at position {}", pos)
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
        let c = self.input[self.pos..].chars().next()?;
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
            if c.is_ascii_whitespace() || (c as u32) <= 32 {
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
                '-' | '0'..='9' => self.parse_number(),
                '[' => self.parse_array(),
                '{' => self.parse_object(),
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
            Err(CJsonError::InvalidLiteral { expected: "true or false", pos: start })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        // Optional sign
        if let Some('-') = self.peek() {
            self.pos += 1;
        }
        // Integer part
        if let Some('0') = self.peek() {
            self.pos += 1;
        } else {
            let mut found_digit = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                    found_digit = true;
                } else {
                    break;
                }
            }
            if !found_digit {
                return Err(CJsonError::InvalidNumber { pos: start });
            }
        }
        // Fractional part
        if let Some('.') = self.peek() {
            self.pos += 1;
            let mut found_digit = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                    found_digit = true;
                } else {
                    break;
                }
            }
            if !found_digit {
                return Err(CJsonError::InvalidNumber { pos: start });
            }
        }
        // Exponent
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                self.pos += 1;
                if let Some(s) = self.peek() {
                    if s == '+' || s == '-' {
                        self.pos += 1;
                    }
                }
                let mut found_digit = false;
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        self.pos += 1;
                        found_digit = true;
                    } else {
                        break;
                    }
                }
                if !found_digit {
                    return Err(CJsonError::InvalidNumber { pos: start });
                }
            }
        }
        let num_str = &self.input[start..self.pos];
        match num_str.parse::<f64>() {
            Ok(n) => Ok(CJson::Number(n)),
            Err(_) => Err(CJsonError::InvalidNumber { pos: start }),
        }
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        self.expect_char('"')?;
        let mut out = String::new();
        loop {
            let start = self.pos;
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos: start }),
                Some('"') => return Ok(out),
                Some('\\') => {
                    let esc_pos = self.pos;
                    match self.next_char() {
                        None => return Err(CJsonError::UnexpectedEOF { pos: esc_pos }),
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('/') => out.push('/'),
                        Some('b') => out.push('\u{0008}'),
                        Some('f') => out.push('\u{000C}'),
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some('u') => {
                            let uc = self.parse_hex4(esc_pos)?;
                            // Handle surrogate pair
                            if (0xD800..=0xDBFF).contains(&uc) {
                                // High surrogate, expect low surrogate
                                if self.peek() != Some('\\') {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: esc_pos });
                                }
                                self.pos += 1;
                                if self.peek() != Some('u') {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: esc_pos });
                                }
                                self.pos += 1;
                                let uc2 = self.parse_hex4(esc_pos)?;
                                if !(0xDC00..=0xDFFF).contains(&uc2) {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: esc_pos });
                                }
                                let combined = 0x10000
                                    + (((uc - 0xD800) << 10) | (uc2 - 0xDC00));
                                if let Some(c) = char::from_u32(combined) {
                                    out.push(c);
                                } else {
                                    return Err(CJsonError::InvalidUnicodeEscape { pos: esc_pos });
                                }
                            } else if (0xDC00..=0xDFFF).contains(&uc) {
                                return Err(CJsonError::InvalidUnicodeEscape { pos: esc_pos });
                            } else if let Some(c) = char::from_u32(uc) {
                                out.push(c);
                            } else {
                                return Err(CJsonError::InvalidUnicodeEscape { pos: esc_pos });
                            }
                        }
                        Some(_) => return Err(CJsonError::InvalidEscape { pos: esc_pos }),
                    }
                }
                Some(c) => out.push(c),
            }
        }
    }
    fn parse_hex4(&mut self, err_pos: usize) -> Result<u32, CJsonError> {
        let mut h: u32 = 0;
        for _ in 0..4 {
            match self.next_char() {
                None => return Err(CJsonError::UnexpectedEOF { pos: err_pos }),
                Some(c) => {
                    let v = match c {
                        '0'..='9' => (c as u32) - ('0' as u32),
                        'a'..='f' => (c as u32) - ('a' as u32) + 10,
                        'A'..='F' => (c as u32) - ('A' as u32) + 10,
                        _ => return Err(CJsonError::InvalidUnicodeEscape { pos: err_pos }),
                    };
                    h = (h << 4) | v;
                }
            }
        }
        Ok(h)
    }
    fn parse_array(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if let Some(']') = self.peek() {
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
                Some(c) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }).map_err(|_| CJsonError::UnexpectedToken { ch: c, pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
    fn parse_object(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('{')?;
        let mut map = HashMap::new();
        self.skip_whitespace();
        if let Some('}') = self.peek() {
            self.pos += 1;
            return Ok(CJson::Object(map));
        }
        loop {
            self.skip_whitespace();
            // Parse key
            if self.peek() != Some('"') {
                return Err(CJsonError::UnexpectedToken {
                    ch: self.peek().unwrap_or(' '),
                    pos: self.pos,
                });
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            // Expect colon
            if self.peek() != Some(':') {
                return Err(CJsonError::ExpectedColon { pos: self.pos });
            }
            self.pos += 1;
            self.skip_whitespace();
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
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
pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let value = parser.parse_value()?;
    if require_end {
        parser.skip_whitespace();
        if parser.pos < parser.input.len() {
            let ch = parser.peek().unwrap_or(' ');
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
    // Check whether the number is an integer that fits in i32
    if n.is_finite() {
        let i = n as i32;
        if (i as f64 - n).abs() <= f64::EPSILON
            && n <= i32::MAX as f64
            && n >= i32::MIN as f64
        {
            return format!("{}", i);
        }
        // Check if integer-valued double
        if (n.floor() - n).abs() <= f64::EPSILON && n.abs() < 1.0e60 {
            return format!("{:.0}", n);
        }
        if n.abs() < 1.0e-6 || n.abs() > 1.0e9 {
            // Use exponent notation similar to printf %e (e.g., 1.234560e+08)
            return format_exp(n);
        }
        // Default %f-like (6 decimal places)
        return format!("{:.6}", n);
    }
    // For NaN/Infinity, fall back to a standard representation
    format!("{}", n)
}
fn format_exp(n: f64) -> String {
    // Format like C's printf "%e" -> e.g., 1.234560e+08
    if n == 0.0 {
        return "0.000000e+00".to_string();
    }
    let neg = n < 0.0;
    let abs_n = n.abs();
    let exp = abs_n.log10().floor() as i32;
    let mantissa = abs_n / 10f64.powi(exp);
    let sign = if exp < 0 { '-' } else { '+' };
    let abs_exp = exp.abs();
    let mantissa_str = format!("{:.6}", mantissa);
    if neg {
        format!("-{}e{}{:02}", mantissa_str, sign, abs_exp)
    } else {
        format!("{}e{}{:02}", mantissa_str, sign, abs_exp)
    }
}
fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(b) => {
            if *b {
                f.write_str("true")
            } else {
                f.write_str("false")
            }
        }
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
        CJson::Bool(b) => {
            if *b {
                f.write_str("true")
            } else {
                f.write_str("false")
            }
        }
        CJson::Number(n) => f.write_str(&format_number(*n)),
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
            f.write_char('{')?;
            f.write_char('\n')?;
            let len = obj.len();
            for (i, (k, v)) in obj.iter().enumerate() {
                for _ in 0..(indent + 1) {
                    f.write_char('\t')?;
                }
                f.write_str(&escape_string(k))?;
                f.write_str(":\t")?;
                write_json_pretty(f, v, indent + 1)?;
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
                // Case-insensitive lookup, like cJSON
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

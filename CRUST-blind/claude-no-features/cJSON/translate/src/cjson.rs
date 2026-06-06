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
            // C cJSON treats any byte <=32 as whitespace
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
            Some('"') => {
                let s = self.parse_string()?;
                Ok(CJson::String(s))
            }
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(CJsonError::UnexpectedToken { ch: c, pos: self.pos }),
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
        // optional sign
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        // integer part: 0 or [1-9][0-9]*
        let int_start = self.pos;
        match self.peek() {
            Some('0') => {
                self.pos += 1;
            }
            Some(c) if c.is_ascii_digit() => {
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            }
            _ => {
                return Err(CJsonError::InvalidNumber { pos: start });
            }
        }
        if self.pos == int_start {
            return Err(CJsonError::InvalidNumber { pos: start });
        }
        // fractional part
        if self.peek() == Some('.') {
            self.pos += 1;
            let frac_start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.pos == frac_start {
                return Err(CJsonError::InvalidNumber { pos: start });
            }
        }
        // exponent part
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                self.pos += 1;
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        self.pos += 1;
                    }
                }
                let exp_start = self.pos;
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                if self.pos == exp_start {
                    return Err(CJsonError::InvalidNumber { pos: start });
                }
            }
        }
        let num_str = &self.input[start..self.pos];
        num_str
            .parse::<f64>()
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
                    let escape_pos = self.pos.saturating_sub(1);
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
                            let uc = self.parse_hex4()?;
                            // handle surrogate pair
                            if (0xD800..=0xDBFF).contains(&uc) {
                                // need low surrogate
                                if self.peek() == Some('\\') {
                                    self.pos += 1;
                                    if self.peek() == Some('u') {
                                        self.pos += 1;
                                        let uc2 = self.parse_hex4()?;
                                        if !(0xDC00..=0xDFFF).contains(&uc2) {
                                            return Err(CJsonError::InvalidUnicodeEscape {
                                                pos: escape_pos,
                                            });
                                        }
                                        let combined = 0x10000
                                            + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF));
                                        if let Some(ch) = char::from_u32(combined) {
                                            out.push(ch);
                                        } else {
                                            return Err(CJsonError::InvalidUnicodeEscape {
                                                pos: escape_pos,
                                            });
                                        }
                                    } else {
                                        return Err(CJsonError::InvalidUnicodeEscape {
                                            pos: escape_pos,
                                        });
                                    }
                                } else {
                                    return Err(CJsonError::InvalidUnicodeEscape {
                                        pos: escape_pos,
                                    });
                                }
                            } else if (0xDC00..=0xDFFF).contains(&uc) {
                                return Err(CJsonError::InvalidUnicodeEscape {
                                    pos: escape_pos,
                                });
                            } else {
                                if let Some(ch) = char::from_u32(uc) {
                                    out.push(ch);
                                } else {
                                    return Err(CJsonError::InvalidUnicodeEscape {
                                        pos: escape_pos,
                                    });
                                }
                            }
                        }
                        Some(_) => {
                            return Err(CJsonError::InvalidEscape { pos: escape_pos });
                        }
                    }
                }
                Some(c) => out.push(c),
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
                Some(_) => {
                    return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos });
                }
                None => {
                    return Err(CJsonError::UnexpectedEOF { pos: self.pos });
                }
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
            match self.peek() {
                Some(':') => {
                    self.pos += 1;
                }
                Some(_) => {
                    return Err(CJsonError::ExpectedColon { pos: self.pos });
                }
                None => {
                    return Err(CJsonError::UnexpectedEOF { pos: self.pos });
                }
            }
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
                Some(_) => {
                    return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos });
                }
                None => {
                    return Err(CJsonError::UnexpectedEOF { pos: self.pos });
                }
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
                    let digit = match c {
                        '0'..='9' => (c as u32) - ('0' as u32),
                        'a'..='f' => 10 + (c as u32) - ('a' as u32),
                        'A'..='F' => 10 + (c as u32) - ('A' as u32),
                        _ => {
                            return Err(CJsonError::InvalidUnicodeEscape { pos: start });
                        }
                    };
                    value = (value << 4) | digit;
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
    // If integer-representable in i32 range, print as integer (matches C cJSON behavior)
    if n.is_finite() {
        let as_int = n as i32;
        if (as_int as f64 - n).abs() <= f64::EPSILON
            && n <= i32::MAX as f64
            && n >= i32::MIN as f64
        {
            return format!("{}", as_int);
        }
        let abs = n.abs();
        if (n.floor() - n).abs() <= f64::EPSILON && abs < 1.0e60 {
            return format!("{:.0}", n);
        } else if abs < 1.0e-6 || abs > 1.0e9 {
            // print in scientific notation similar to "%e"
            return format_scientific(n);
        } else {
            return format!("{:.6}", n);
        }
    }
    // non-finite numbers - just produce something
    format!("{}", n)
}

fn format_scientific(n: f64) -> String {
    // Replicate "%e" style: e.g. 1.234560e+05
    if n == 0.0 {
        return "0.000000e+00".to_string();
    }
    let sign = if n < 0.0 { "-" } else { "" };
    let abs = n.abs();
    let exp = abs.log10().floor() as i32;
    let mantissa = abs / 10f64.powi(exp);
    let exp_sign = if exp >= 0 { '+' } else { '-' };
    format!("{}{:.6}e{}{:02}", sign, mantissa, exp_sign, exp.abs())
}

fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => write!(f, "null"),
        CJson::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
        CJson::Number(n) => write!(f, "{}", format_number(*n)),
        CJson::String(s) => write!(f, "{}", escape_string(s)),
        CJson::Array(arr) => {
            write!(f, "[")?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write_json_compact(f, item)?;
            }
            write!(f, "]")
        }
        CJson::Object(obj) => {
            write!(f, "{{")?;
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, "{}", escape_string(k))?;
                write!(f, ":")?;
                write_json_compact(f, v)?;
            }
            write!(f, "}}")
        }
    }
}

fn write_json_pretty(f: &mut impl fmt::Write, value: &CJson, indent: usize) -> fmt::Result {
    match value {
        CJson::Null => write!(f, "null"),
        CJson::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
        CJson::Number(n) => write!(f, "{}", format_number(*n)),
        CJson::String(s) => write!(f, "{}", escape_string(s)),
        CJson::Array(arr) => {
            if arr.is_empty() {
                return write!(f, "[]");
            }
            write!(f, "[")?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    write!(f, ",\t")?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            write!(f, "]")
        }
        CJson::Object(obj) => {
            if obj.is_empty() {
                write!(f, "{{")?;
                write!(f, "\n")?;
                for _ in 0..indent {
                    write!(f, "\t")?;
                }
                write!(f, "}}")?;
                return Ok(());
            }
            write!(f, "{{\n")?;
            let new_depth = indent + 1;
            let last = obj.len() - 1;
            for (i, (k, v)) in obj.iter().enumerate() {
                for _ in 0..new_depth {
                    write!(f, "\t")?;
                }
                write!(f, "{}", escape_string(k))?;
                write!(f, ":\t")?;
                write_json_pretty(f, v, new_depth)?;
                if i != last {
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
                // case-insensitive lookup matches C cJSON_GetObjectItem
                if let Some(v) = obj.get(key) {
                    return Some(v);
                }
                let key_lower = key.to_lowercase();
                for (k, v) in obj.iter() {
                    if k.to_lowercase() == key_lower {
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

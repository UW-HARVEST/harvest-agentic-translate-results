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
                write!(f, "Expected ',' or end of container at position {}", pos)
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
            // cJSON's skip(): consumes all bytes <= 32
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
        let c = self.peek().ok_or(CJsonError::UnexpectedEOF { pos: self.pos })?;
        match c {
            'n' => self.parse_null(),
            't' | 'f' => self.parse_bool(),
            '"' => Ok(CJson::String(self.parse_string()?)),
            '[' => self.parse_array(),
            '{' => self.parse_object(),
            '-' => self.parse_number(),
            c if c.is_ascii_digit() => self.parse_number(),
            c => Err(CJsonError::UnexpectedToken { ch: c, pos: self.pos }),
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
            Err(CJsonError::InvalidLiteral { expected: "true/false", pos: self.pos })
        }
    }
    fn parse_number(&mut self) -> Result<CJson, CJsonError> {
        let start = self.pos;
        let bytes = self.input.as_bytes();
        let len = bytes.len();
        let mut p = self.pos;
        let mut sign: f64 = 1.0;
        let mut n: f64 = 0.0;
        let mut scale: i32 = 0;
        let mut subscale: i32 = 0;
        let mut signsubscale: i32 = 1;

        if p < len && bytes[p] == b'-' {
            sign = -1.0;
            p += 1;
        }
        if p < len && bytes[p] == b'0' {
            p += 1;
        }
        while p < len && bytes[p] >= b'1' && bytes[p] <= b'9' {
            n = n * 10.0 + (bytes[p] - b'0') as f64;
            p += 1;
            while p < len && bytes[p] >= b'0' && bytes[p] <= b'9' {
                n = n * 10.0 + (bytes[p] - b'0') as f64;
                p += 1;
            }
        }
        if p < len && bytes[p] == b'.' && p + 1 < len && bytes[p + 1] >= b'0' && bytes[p + 1] <= b'9' {
            p += 1;
            while p < len && bytes[p] >= b'0' && bytes[p] <= b'9' {
                n = n * 10.0 + (bytes[p] - b'0') as f64;
                scale -= 1;
                p += 1;
            }
        }
        if p < len && (bytes[p] == b'e' || bytes[p] == b'E') {
            p += 1;
            if p < len && bytes[p] == b'+' {
                p += 1;
            } else if p < len && bytes[p] == b'-' {
                signsubscale = -1;
                p += 1;
            }
            while p < len && bytes[p] >= b'0' && bytes[p] <= b'9' {
                subscale = subscale * 10 + (bytes[p] - b'0') as i32;
                p += 1;
            }
        }

        if p == start {
            return Err(CJsonError::InvalidNumber { pos: start });
        }

        self.pos = p;
        let value = sign * n * 10f64.powi(scale + subscale * signsubscale);
        Ok(CJson::Number(value))
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        // Expect opening quote
        if self.peek() != Some('"') {
            return Err(CJsonError::UnexpectedToken {
                ch: self.peek().unwrap_or('\0'),
                pos: self.pos,
            });
        }
        self.pos += 1;

        let mut out = String::new();

        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            };

            if c == '"' {
                self.pos += 1;
                return Ok(out);
            }

            if c == '\\' {
                self.pos += 1;
                let esc = self.peek().ok_or(CJsonError::UnexpectedEOF { pos: self.pos })?;
                match esc {
                    'b' => {
                        out.push('\u{0008}');
                        self.pos += 1;
                    }
                    'f' => {
                        out.push('\u{000C}');
                        self.pos += 1;
                    }
                    'n' => {
                        out.push('\n');
                        self.pos += 1;
                    }
                    'r' => {
                        out.push('\r');
                        self.pos += 1;
                    }
                    't' => {
                        out.push('\t');
                        self.pos += 1;
                    }
                    '"' | '\\' | '/' => {
                        out.push(esc);
                        self.pos += 1;
                    }
                    'u' => {
                        self.pos += 1; // skip 'u'
                        let uc = self.parse_hex4()?;
                        if (0xDC00..=0xDFFF).contains(&uc) || uc == 0 {
                            // invalid - skip (matching cJSON's lenient behavior of break)
                            continue;
                        }
                        let code_point = if (0xD800..=0xDBFF).contains(&uc) {
                            // high surrogate - need low surrogate
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
                        if let Some(ch) = char::from_u32(code_point) {
                            out.push(ch);
                        } else {
                            return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos });
                        }
                    }
                    other => {
                        // cJSON's default: just push the literal char
                        out.push(other);
                        self.pos += other.len_utf8();
                    }
                }
            } else {
                out.push(c);
                self.pos += c.len_utf8();
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
            let item = self.parse_value()?;
            items.push(item);
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
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
    fn parse_object(&mut self) -> Result<CJson, CJsonError> {
        self.expect_char('{')?;
        self.skip_whitespace();
        let mut map = HashMap::new();
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

impl<'a> Parser<'a> {
    fn parse_hex4(&mut self) -> Result<u32, CJsonError> {
        let bytes = self.input.as_bytes();
        if self.pos + 4 > bytes.len() {
            return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos });
        }
        let mut h: u32 = 0;
        for i in 0..4 {
            let b = bytes[self.pos + i];
            let d = match b {
                b'0'..=b'9' => (b - b'0') as u32,
                b'A'..=b'F' => (b - b'A' + 10) as u32,
                b'a'..=b'f' => (b - b'a' + 10) as u32,
                _ => return Err(CJsonError::InvalidUnicodeEscape { pos: self.pos }),
            };
            h = (h << 4) | d;
        }
        self.pos += 4;
        Ok(h)
    }
}

pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let value = parser.parse_value()?;
    if require_end {
        parser.skip_whitespace();
        if parser.pos != input.len() {
            let ch = parser.peek().unwrap_or('\0');
            return Err(CJsonError::UnexpectedToken {
                ch,
                pos: parser.pos,
            });
        }
    }
    Ok(value)
}

fn escape_string(s: &str) -> String {
    // Determine if we need any escaping
    let mut needs_escape = false;
    for c in s.chars() {
        let v = c as u32;
        if v < 32 || c == '"' || c == '\\' {
            needs_escape = true;
            break;
        }
    }
    let mut out = String::new();
    out.push('"');
    if !needs_escape {
        out.push_str(s);
        out.push('"');
        return out;
    }
    for c in s.chars() {
        let v = c as u32;
        if v >= 32 && c != '"' && c != '\\' {
            out.push(c);
        } else {
            out.push('\\');
            match c {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                '\u{0008}' => out.push('b'),
                '\u{000C}' => out.push('f'),
                '\n' => out.push('n'),
                '\r' => out.push('r'),
                '\t' => out.push('t'),
                _ => {
                    out.push_str(&format!("u{:04x}", v));
                }
            }
        }
    }
    out.push('"');
    out
}

fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    let int_n = n as i32;
    if (int_n as f64 - n).abs() <= f64::EPSILON
        && n <= i32::MAX as f64
        && n >= i32::MIN as f64
    {
        return format!("{}", int_n);
    }
    let abs_n = n.abs();
    if (n.floor() - n).abs() <= f64::EPSILON && abs_n < 1.0e60 {
        return format!("{:.0}", n);
    }
    if abs_n < 1.0e-6 || abs_n > 1.0e9 {
        // C-style "%e" output, e.g. "1.234560e+10" or "1.234560e-05"
        // Rust default {:e} -> "1.23456e10", we need "1.234560e+10"
        return format_exponential(n);
    }
    // "%f" -> 6 decimal places by default
    format!("{:.6}", n)
}

fn format_exponential(n: f64) -> String {
    // Mimic C's "%e" with 6 digits of precision, e.g. "1.234560e+10"
    // Approach: use Rust's {:e} then reformat.
    let s = format!("{:e}", n);
    // s looks like "1.23456e10" or "-1.23456e-5" etc.
    // We need: mantissa to 6 decimal places, exponent with sign and at least 2 digits.
    if let Some(epos) = s.find('e') {
        let (mantissa, exp) = s.split_at(epos);
        let exp = &exp[1..]; // skip 'e'
        // Parse mantissa
        let m: f64 = mantissa.parse().unwrap_or(0.0);
        let mantissa_str = format!("{:.6}", m);
        // Parse exponent
        let exp_i: i32 = exp.parse().unwrap_or(0);
        let exp_sign = if exp_i < 0 { '-' } else { '+' };
        let exp_abs = exp_i.unsigned_abs();
        format!("{}e{}{:02}", mantissa_str, exp_sign, exp_abs)
    } else {
        s
    }
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
        CJson::Object(map) => {
            f.write_char('{')?;
            let mut first = true;
            for (k, v) in map.iter() {
                if !first {
                    f.write_char(',')?;
                }
                first = false;
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
            // cJSON prints arrays inline with ", " separators (no newlines)
            f.write_char('[')?;
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            f.write_char(']')
        }
        CJson::Object(map) => {
            if map.is_empty() {
                f.write_char('{')?;
                f.write_char('\n')?;
                for _ in 0..indent {
                    f.write_char('\t')?;
                }
                f.write_char('}')?;
                return Ok(());
            }
            f.write_char('{')?;
            f.write_char('\n')?;
            let inner = indent + 1;
            let n = map.len();
            for (i, (k, v)) in map.iter().enumerate() {
                for _ in 0..inner {
                    f.write_char('\t')?;
                }
                f.write_str(&escape_string(k))?;
                f.write_str(":\t")?;
                write_json_pretty(f, v, inner)?;
                if i + 1 != n {
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
        write_json_compact(&mut s, self).unwrap();
        s
    }
    pub fn print_formatted(&self) -> String {
        let mut s = String::new();
        write_json_pretty(&mut s, self, 0).unwrap();
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
                // Case-insensitive lookup matching cJSON
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
            _ => Err("Not an array"),
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
            _ => Err("Not an object"),
        }
    }
}
impl fmt::Display for CJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_json_pretty(f, self, 0)
    }
}

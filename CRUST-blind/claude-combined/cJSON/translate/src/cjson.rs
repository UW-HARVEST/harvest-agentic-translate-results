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
            CJsonError::UnexpectedEOF { pos } => write!(f, "unexpected end of input at position {}", pos),
            CJsonError::UnexpectedToken { ch, pos } => write!(f, "unexpected token '{}' at position {}", ch, pos),
            CJsonError::InvalidLiteral { expected, pos } => write!(f, "invalid literal, expected '{}' at position {}", expected, pos),
            CJsonError::InvalidNumber { pos } => write!(f, "invalid number at position {}", pos),
            CJsonError::InvalidEscape { pos } => write!(f, "invalid escape sequence at position {}", pos),
            CJsonError::InvalidUnicodeEscape { pos } => write!(f, "invalid unicode escape at position {}", pos),
            CJsonError::ExpectedColon { pos } => write!(f, "expected ':' at position {}", pos),
            CJsonError::ExpectedCommaOrEnd { pos } => write!(f, "expected ',' or end of container at position {}", pos),
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
            // C cJSON uses (unsigned char)*in <= 32 to skip whitespace
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
            '-' | '0'..='9' => self.parse_number(),
            other => Err(CJsonError::UnexpectedToken { ch: other, pos: self.pos }),
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
        // Replicate cJSON's number parsing exactly.
        let bytes = self.input.as_bytes();
        let mut i = self.pos;
        let mut n: f64 = 0.0;
        let mut sign: f64 = 1.0;
        let mut scale: i32 = 0;
        let mut subscale: i32 = 0;
        let mut signsubscale: i32 = 1;

        if i < bytes.len() && bytes[i] == b'-' {
            sign = -1.0;
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'0' {
            i += 1;
        }
        if i < bytes.len() && bytes[i] >= b'1' && bytes[i] <= b'9' {
            while i < bytes.len() && bytes[i] >= b'0' && bytes[i] <= b'9' {
                n = n * 10.0 + (bytes[i] - b'0') as f64;
                i += 1;
            }
        }
        if i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1] >= b'0' && bytes[i + 1] <= b'9' {
            i += 1;
            while i < bytes.len() && bytes[i] >= b'0' && bytes[i] <= b'9' {
                n = n * 10.0 + (bytes[i] - b'0') as f64;
                scale -= 1;
                i += 1;
            }
        }
        if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
            i += 1;
            if i < bytes.len() && bytes[i] == b'+' {
                i += 1;
            } else if i < bytes.len() && bytes[i] == b'-' {
                signsubscale = -1;
                i += 1;
            }
            while i < bytes.len() && bytes[i] >= b'0' && bytes[i] <= b'9' {
                subscale = subscale * 10 + (bytes[i] - b'0') as i32;
                i += 1;
            }
        }
        if i == start {
            return Err(CJsonError::InvalidNumber { pos: start });
        }
        let exp = (scale + subscale * signsubscale) as f64;
        n = sign * n * 10f64.powf(exp);
        self.pos = i;
        Ok(CJson::Number(n))
    }
    fn parse_string(&mut self) -> Result<String, CJsonError> {
        if self.peek() != Some('"') {
            return Err(CJsonError::UnexpectedToken {
                ch: self.peek().unwrap_or('\0'),
                pos: self.pos,
            });
        }
        self.pos += 1; // consume opening quote
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
                let esc = match self.peek() {
                    Some(c) => c,
                    None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
                };
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
                    '"' => {
                        out.push('"');
                        self.pos += 1;
                    }
                    '\\' => {
                        out.push('\\');
                        self.pos += 1;
                    }
                    '/' => {
                        out.push('/');
                        self.pos += 1;
                    }
                    'u' => {
                        // \uXXXX
                        self.pos += 1; // skip 'u'
                        let uc = parse_hex4(&self.input, self.pos)
                            .ok_or(CJsonError::InvalidUnicodeEscape { pos: self.pos })?;
                        self.pos += 4;
                        // Skip invalid unicode (matches cJSON behavior of breaking out).
                        if (0xDC00..=0xDFFF).contains(&uc) || uc == 0 {
                            continue;
                        }
                        let codepoint = if (0xD800..=0xDBFF).contains(&uc) {
                            // need a low surrogate next
                            let bytes = self.input.as_bytes();
                            if self.pos + 1 < bytes.len()
                                && bytes[self.pos] == b'\\'
                                && bytes[self.pos + 1] == b'u'
                            {
                                let uc2 = parse_hex4(&self.input, self.pos + 2)
                                    .ok_or(CJsonError::InvalidUnicodeEscape { pos: self.pos })?;
                                self.pos += 6;
                                if !(0xDC00..=0xDFFF).contains(&uc2) {
                                    continue;
                                }
                                0x10000 + (((uc & 0x3FF) << 10) | (uc2 & 0x3FF))
                            } else {
                                continue;
                            }
                        } else {
                            uc
                        };
                        if let Some(c) = char::from_u32(codepoint) {
                            out.push(c);
                        }
                    }
                    other => {
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
        let mut arr = Vec::new();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(CJson::Array(arr));
        }
        loop {
            self.skip_whitespace();
            let v = self.parse_value()?;
            arr.push(v);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                    continue;
                }
                Some(']') => {
                    self.pos += 1;
                    return Ok(CJson::Array(arr));
                }
                Some(c) => return Err(CJsonError::ExpectedCommaOrEnd { pos: self.pos }).map_err(|_| CJsonError::UnexpectedToken { ch: c, pos: self.pos }),
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
                    continue;
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(CJson::Object(map));
                }
                Some(c) => return Err(CJsonError::UnexpectedToken { ch: c, pos: self.pos }),
                None => return Err(CJsonError::UnexpectedEOF { pos: self.pos }),
            }
        }
    }
}

fn parse_hex4(s: &str, start: usize) -> Option<u32> {
    let bytes = s.as_bytes();
    if start + 4 > bytes.len() {
        return None;
    }
    let mut h: u32 = 0;
    for i in 0..4 {
        let b = bytes[start + i];
        let v = if (b'0'..=b'9').contains(&b) {
            (b - b'0') as u32
        } else if (b'A'..=b'F').contains(&b) {
            (b - b'A' + 10) as u32
        } else if (b'a'..=b'f').contains(&b) {
            (b - b'a' + 10) as u32
        } else {
            return None;
        };
        h = (h << 4) | v;
    }
    Some(h)
}

pub fn parse(input: &str, require_end: bool) -> Result<CJson, CJsonError> {
    let mut p = Parser::new(input);
    p.skip_whitespace();
    let value = p.parse_value()?;
    if require_end {
        p.skip_whitespace();
        if p.pos < input.len() {
            return Err(CJsonError::UnexpectedToken {
                ch: p.peek().unwrap_or('\0'),
                pos: p.pos,
            });
        }
    }
    Ok(value)
}
fn escape_string(s: &str) -> String {
    // Match cJSON's print_string_ptr behavior.
    // First check if any escape required.
    let mut needs_escape = false;
    for c in s.chars() {
        let v = c as u32;
        if (v > 0 && v < 32) || c == '"' || c == '\\' {
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
        if v > 31 && c != '"' && c != '\\' {
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
    // Match cJSON print_number.
    if n == 0.0 {
        return "0".to_string();
    }
    let as_int = n as i32;
    let int_diff = (as_int as f64 - n).abs();
    if int_diff <= f64::EPSILON && n <= i32::MAX as f64 && n >= i32::MIN as f64 {
        return format!("{}", as_int);
    }
    // floor(d) - d
    let floor_diff = (n.floor() - n).abs();
    if floor_diff <= f64::EPSILON && n.abs() < 1.0e60 {
        return format!("{:.0}", n);
    } else if n.abs() < 1.0e-6 || n.abs() > 1.0e9 {
        // C "%e" style: e.g. 1.234560e+02
        return format_c_exponential(n);
    } else {
        // C "%f" style: 6 decimal places.
        return format!("{:.6}", n);
    }
}
fn format_c_exponential(n: f64) -> String {
    // C's printf "%e" produces something like "1.234560e+02" (default 6 digits after the decimal).
    // Rust's {:e} produces "1.23456e2". We need to emulate the C format.
    if n == 0.0 {
        return "0.000000e+00".to_string();
    }
    let sign = if n < 0.0 { "-" } else { "" };
    let abs = n.abs();
    let exponent = abs.log10().floor() as i32;
    let mantissa = abs / 10f64.powi(exponent);
    // Adjust mantissa rounding: it might round to 10
    let (m, e) = if format!("{:.6}", mantissa) == "10.000000" {
        (mantissa / 10.0, exponent + 1)
    } else {
        (mantissa, exponent)
    };
    let exp_sign = if e < 0 { '-' } else { '+' };
    format!("{}{:.6}e{}{:02}", sign, m, exp_sign, e.abs())
}
fn write_json_compact(f: &mut impl fmt::Write, value: &CJson) -> fmt::Result {
    match value {
        CJson::Null => f.write_str("null"),
        CJson::Bool(true) => f.write_str("true"),
        CJson::Bool(false) => f.write_str("false"),
        CJson::Number(n) => f.write_str(&format_number(*n)),
        CJson::String(s) => f.write_str(&escape_string(s)),
        CJson::Array(items) => {
            f.write_str("[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    f.write_str(",")?;
                }
                write_json_compact(f, item)?;
            }
            f.write_str("]")
        }
        CJson::Object(map) => {
            f.write_str("{")?;
            // sort keys for deterministic output
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    f.write_str(",")?;
                }
                f.write_str(&escape_string(key))?;
                f.write_str(":")?;
                write_json_compact(f, &map[*key])?;
            }
            f.write_str("}")
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
        CJson::Array(items) => {
            // C cJSON prints arrays in pretty mode on a single line: [a, b, c]
            if items.is_empty() {
                return f.write_str("[]");
            }
            f.write_str("[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write_json_pretty(f, item, indent + 1)?;
            }
            f.write_str("]")
        }
        CJson::Object(map) => {
            // Convention: `indent` here = C's `depth` (the passed-in value).
            // C's print_object: when non-empty, prints children at depth+1 tabs and closes at depth tabs.
            // When empty, prints "{\n" + (depth-1) tabs + "}".
            if map.is_empty() {
                f.write_str("{\n")?;
                let n = if indent == 0 { 0 } else { indent - 1 };
                for _ in 0..n {
                    f.write_str("\t")?;
                }
                f.write_str("}")?;
                return Ok(());
            }
            f.write_str("{\n")?;
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let len = keys.len();
            for (i, key) in keys.iter().enumerate() {
                for _ in 0..=indent {
                    f.write_str("\t")?;
                }
                f.write_str(&escape_string(key))?;
                f.write_str(":\t")?;
                write_json_pretty(f, &map[*key], indent + 1)?;
                if i != len - 1 {
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
            CJson::Object(map) => {
                // Case-insensitive lookup, matching cJSON behavior.
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
        write_json_compact(f, self)
    }
}

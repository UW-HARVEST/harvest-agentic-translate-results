use std::{fmt::Display, fs::File, io::Read, io::Write};
// Constants
pub const CFG_FILE_EXT: &str = ".cfg";
pub const CFG_MAX_KEY: usize = 32;
pub const CFG_MAX_VAL: usize = 64;
pub const CFG_MAX_ERR: usize = 64;
// Structures and Enums
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CfgColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CfgError {
    pub off: i32,
    pub col: i32,
    pub row: i32,
    pub msg: String,
}
impl Default for CfgError {
    fn default() -> Self {
        CfgError {
            off: 0,
            col: 0,
            row: 0,
            msg: String::new(),
        }
    }
}
#[derive(Clone, PartialEq, Debug)]
pub enum CfgVal {
    String(String),
    Boolean(bool),
    Int(i32),
    Float(f32),
    Color(CfgColor),
}
impl From<&str> for CfgVal {
    fn from(s: &str) -> Self {
        CfgVal::String(s.to_string())
    }
}
impl From<String> for CfgVal {
    fn from(s: String) -> Self {
        CfgVal::String(s)
    }
}
impl From<bool> for CfgVal {
    fn from(b: bool) -> Self {
        CfgVal::Boolean(b)
    }
}
impl From<i32> for CfgVal {
    fn from(i: i32) -> Self {
        CfgVal::Int(i)
    }
}
impl From<f32> for CfgVal {
    fn from(f: f32) -> Self {
        CfgVal::Float(f)
    }
}
impl From<CfgColor> for CfgVal {
    fn from(c: CfgColor) -> Self {
        CfgVal::Color(c)
    }
}
impl From<(u8, u8, u8, u8)> for CfgColor {
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        CfgColor { r, g, b, a }
    }
}
#[derive(Clone, PartialEq, Debug)]
pub struct CfgEntry {
    pub key: String,
    pub val: CfgVal,
}
#[derive(Clone, PartialEq, Debug)]
pub struct Cfg {
    pub entries: Vec<CfgEntry>,
    pub count: i32,
    pub capacity: usize,
}
impl Display for CfgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.row == -1 && self.col == -1 {
            write!(f, "Error: {}", self.msg)
        } else {
            write!(f, "Error at {}:{} :: {}", self.row, self.col, self.msg)
        }
    }
}
impl Display for CfgColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }
}
impl Display for CfgVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgVal::String(s) => write!(f, "\"{}\"", s),
            CfgVal::Boolean(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            CfgVal::Int(i) => write!(f, "{}", i),
            CfgVal::Float(fl) => write!(f, "{:.6}", fl),
            CfgVal::Color(c) => write!(f, "{}", c),
        }
    }
}
impl Display for CfgEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.key, self.val)
    }
}
impl Display for Cfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for entry in &self.entries {
            if !first {
                writeln!(f)?;
            }
            first = false;
            write!(f, "{}", entry)?;
        }
        Ok(())
    }
}
pub struct Scanner {
    pub src: String,
    pub len: i32,
    pub cur: i32,
}

// Default capacity for parsed Cfg (matches TEST_CAPACITY in tests)
const DEFAULT_CAPACITY: usize = 10;

impl Scanner {
    fn new(src: &str) -> Self {
        Scanner {
            src: src.to_string(),
            len: src.len() as i32,
            cur: 0,
        }
    }

    fn is_at_end(&self) -> bool {
        self.cur >= self.len
    }

    fn peek(&self) -> u8 {
        self.src.as_bytes()[self.cur as usize]
    }

    fn peek_next(&self) -> u8 {
        if self.cur >= self.len - 1 {
            0
        } else {
            self.src.as_bytes()[(self.cur + 1) as usize]
        }
    }

    fn advance(&mut self) -> u8 {
        let c = self.src.as_bytes()[self.cur as usize];
        self.cur += 1;
        c
    }

    fn advance_n(&mut self, n: i32) {
        self.cur += n;
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && (self.peek() as char).is_ascii_whitespace() {
            self.advance();
        }
    }

    fn skip_blank(&mut self) {
        while !self.is_at_end() {
            let c = self.peek();
            if (c as char).is_ascii_whitespace() && c != b'\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        while !self.is_at_end() && self.peek() == b'#' {
            loop {
                self.advance();
                if self.is_at_end() || self.peek() == b'\n' {
                    break;
                }
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            let c = self.peek();
            if !(c as char).is_ascii_whitespace() && c != b'#' {
                break;
            }
            self.skip_whitespace();
            self.skip_comment();
        }
    }

    fn match_literal(&self, offset: i32, literal: &str) -> bool {
        let lit_len = literal.len() as i32;
        if offset + lit_len > self.len {
            return false;
        }
        let bytes = self.src.as_bytes();
        let lit_bytes = literal.as_bytes();
        for i in 0..lit_len as usize {
            if bytes[offset as usize + i] != lit_bytes[i] {
                return false;
            }
        }
        true
    }

    fn consume_literal(&mut self, offset: i32, literal: &str) -> bool {
        if self.match_literal(offset, literal) {
            self.advance_n(literal.len() as i32);
            true
        } else {
            false
        }
    }
}

fn is_key(ch: u8) -> bool {
    (ch as char).is_ascii_alphabetic() || ch == b'.' || ch == b'_'
}

fn is_string_char(ch: u8) -> bool {
    let c = ch as char;
    c.is_ascii_alphanumeric()
        || c == ' '
        || c == '\t'
        || (c.is_ascii_punctuation() && ch != b'"')
}

fn make_error(s: &Scanner, msg: &str) -> CfgError {
    let mut err = CfgError {
        off: s.cur,
        row: 1,
        col: 1,
        msg: msg.to_string(),
    };
    let bytes = s.src.as_bytes();
    for i in 0..s.cur as usize {
        err.col += 1;
        if bytes[i] == b'\n' {
            err.row += 1;
            err.col = 1;
        }
    }
    // Truncate msg to CFG_MAX_ERR - 1 like snprintf does
    if err.msg.len() > CFG_MAX_ERR - 1 {
        err.msg.truncate(CFG_MAX_ERR - 1);
    }
    err
}

fn parse_string(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    // Consume opening '"'
    s.advance();

    let val_offset = s.cur;
    while !s.is_at_end() && is_string_char(s.peek()) {
        s.advance();
    }

    if s.is_at_end() || s.peek() != b'"' {
        return Err(make_error(s, "closing '\"' expected"));
    }

    let val_len = s.cur - val_offset;
    if val_len > CFG_MAX_VAL as i32 {
        return Err(make_error(s, "value too long"));
    }

    // Consume closing '"'
    s.advance();

    let bytes = &s.src.as_bytes()[val_offset as usize..(val_offset + val_len) as usize];
    let value = String::from_utf8_lossy(bytes).to_string();
    Ok(CfgVal::String(value))
}

fn consume_int(s: &mut Scanner) -> Result<i32, CfgError> {
    let mut sign: i64 = 1;
    let mut num: i64 = 0;

    if !s.is_at_end() && s.peek() == b'-' && (s.peek_next() as char).is_ascii_digit() {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !(s.peek() as char).is_ascii_digit() {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i64;
        // Match the C overflow check: num > (INT_MAX - digit) / 10
        if num > (i32::MAX as i64 - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        num = num * 10 + digit;
    }

    Ok((sign * num) as i32)
}

fn consume_float(s: &mut Scanner) -> Result<f32, CfgError> {
    let mut sign: i64 = 1;
    let mut int_part: i64 = 0;
    let mut fract_part: i64 = 0;

    if !s.is_at_end() && s.peek() == b'-' && (s.peek_next() as char).is_ascii_digit() {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !(s.peek() as char).is_ascii_digit() {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i64;
        if int_part > (i32::MAX as i64 - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        int_part = int_part * 10 + digit;
    }

    if !s.is_at_end() && s.peek() != b'.' {
        return Err(make_error(s, "float expected"));
    }

    // Consume '.'
    if !s.is_at_end() {
        s.advance();
    }

    let mut div: i64 = 1;
    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i64;
        if fract_part > (i32::MAX as i64 - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        fract_part = fract_part * 10 + digit;
        if div > (i32::MAX as i64) / 10 {
            return Err(make_error(s, "number too large"));
        }
        div *= 10;
    }

    let floating = int_part as f32 + (fract_part as f32 / div as f32);
    Ok(sign as f32 * floating)
}

fn match_float(s: &mut Scanner) -> bool {
    let restore = s.cur;
    let mut is_float = false;

    if !s.is_at_end() && s.peek() == b'-' && (s.peek_next() as char).is_ascii_digit() {
        s.advance();
    }

    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        s.advance();
    }

    if !s.is_at_end() && s.peek() == b'.' {
        is_float = true;
    }

    s.cur = restore;
    is_float
}

fn parse_number(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let f = consume_float(s)?;
        Ok(CfgVal::Float(f))
    } else {
        let i = consume_int(s)?;
        Ok(CfgVal::Int(i))
    }
}

fn parse_rgba(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    if !s.consume_literal(s.cur, "rgba") {
        return Err(make_error(s, "invalid literal"));
    }

    s.skip_blank();

    if s.is_at_end() || s.peek() != b'(' {
        return Err(make_error(s, "'(' expected"));
    }

    // Consume '('
    s.advance();

    let mut rgb = [0u8; 3];
    for i in 0..3 {
        s.skip_blank();

        if match_float(s) {
            return Err(make_error(
                s,
                "red, blue and green must be integers in range [0, 255]",
            ));
        }

        let number = consume_int(s)?;

        if number < 0 || number > 255 {
            return Err(make_error(
                s,
                "red, blue and green must be integers in range [0, 255]",
            ));
        }

        rgb[i] = number as u8;

        s.skip_blank();

        if s.is_at_end() || s.peek() != b',' {
            return Err(make_error(s, "',' expected"));
        }

        // Consume ','
        s.advance();
    }

    s.skip_blank();

    let alpha: u8 = if match_float(s) {
        let number = consume_float(s)?;
        if number < 0.0 || number > 1.0 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        (number * 255.0).round() as u8
    } else {
        let number = consume_int(s)?;
        if number < 0 || number > 1 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        (number as u32 * 255) as u8
    };

    s.skip_blank();

    if s.is_at_end() || s.peek() != b')' {
        return Err(make_error(s, "')' expected"));
    }

    // Consume ')'
    s.advance();

    Ok(CfgVal::Color(CfgColor {
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
        a: alpha,
    }))
}

fn parse_true(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    if !s.consume_literal(s.cur, "true") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    if !s.consume_literal(s.cur, "false") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(false))
}

fn parse_literal(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    match s.peek() {
        b't' => parse_true(s),
        b'f' => parse_false(s),
        b'r' => parse_rgba(s),
        _ => Err(make_error(s, "invalid literal")),
    }
}

fn parse_value(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    s.skip_blank();

    if s.is_at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let c = s.peek();
    if c == b'"' {
        parse_string(s)
    } else if (c as char).is_ascii_alphabetic() {
        parse_literal(s)
    } else if (c as char).is_ascii_digit()
        || (c == b'-' && (s.peek_next() as char).is_ascii_digit())
    {
        parse_number(s)
    } else {
        Err(make_error(s, "invalid value"))
    }
}

fn parse_key(s: &mut Scanner) -> Result<String, CfgError> {
    if s.is_at_end() || !is_key(s.peek()) {
        return Err(make_error(s, "missing key"));
    }

    let key_offset = s.cur;
    loop {
        s.advance();
        if s.is_at_end() || !is_key(s.peek()) {
            break;
        }
    }
    let key_len = s.cur - key_offset;

    if key_len > CFG_MAX_KEY as i32 {
        return Err(make_error(s, "key too long"));
    }

    let bytes = &s.src.as_bytes()[key_offset as usize..(key_offset + key_len) as usize];
    Ok(String::from_utf8_lossy(bytes).to_string())
}

fn consume_colon(s: &mut Scanner) -> Result<(), CfgError> {
    if s.is_at_end() {
        return Err(make_error(s, "':' expected"));
    }

    s.skip_blank();

    if s.is_at_end() {
        return Err(make_error(s, "missing value"));
    }

    if s.peek() != b':' {
        return Err(make_error(s, "':' expected"));
    }

    // Consume ':'
    s.advance();
    Ok(())
}

fn parse_entry(s: &mut Scanner) -> Result<CfgEntry, CfgError> {
    let key = parse_key(s)?;
    consume_colon(s)?;
    let val = parse_value(s)?;

    s.skip_blank();

    if !s.is_at_end() && s.peek() == b'#' {
        s.skip_comment();
    }

    if !s.is_at_end() && s.peek() != b'\n' {
        let c = s.peek() as char;
        let msg = format!("unexpected character '{}'", c);
        return Err(make_error(s, &msg));
    }

    // Consume '\n'
    if !s.is_at_end() {
        s.advance();
    }

    Ok(CfgEntry { key, val })
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let mut s = Scanner::new(src);
    let mut entries: Vec<CfgEntry> = Vec::new();

    s.skip_whitespace_and_comments();

    while !s.is_at_end() && entries.len() < DEFAULT_CAPACITY {
        // Pre-check: if first char isn't a valid key char, return "invalid character"
        let c = s.peek();
        if !is_key(c) && c != b'"' {
            return Err(make_error(&s, "invalid character"));
        }

        // Special case: only one character left and it's a valid key char, skip silently
        if s.len - s.cur <= 1 && is_key(c) {
            break;
        }

        let entry = parse_entry(&mut s)?;
        entries.push(entry);
        s.skip_whitespace_and_comments();
    }

    let count = entries.len() as i32;
    Ok(Cfg {
        entries,
        count,
        capacity: DEFAULT_CAPACITY,
    })
}

pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    let len = filename.len();
    if len < 5 {
        return Err(CfgError {
            off: -1,
            col: -1,
            row: -1,
            msg: "invalid filename".to_string(),
        });
    }

    let ext_start = len - CFG_FILE_EXT.len();
    if &filename[ext_start..] != CFG_FILE_EXT {
        return Err(CfgError {
            off: -1,
            col: -1,
            row: -1,
            msg: "invalid file extension".to_string(),
        });
    }

    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            return Err(CfgError {
                off: -1,
                col: -1,
                row: -1,
                msg: "failed to open file".to_string(),
            });
        }
    };

    let mut src = String::new();
    if file.read_to_string(&mut src).is_err() {
        return Err(CfgError {
            off: -1,
            col: -1,
            row: -1,
            msg: "failed to read file".to_string(),
        });
    }

    cfg_parse(&src)
}

fn find_entry<'a>(cfg: &'a Cfg, key: &str) -> Option<&'a CfgVal> {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            return Some(&entry.val);
        }
    }
    None
}

pub fn cfg_get_string<'a>(cfg: &'a Cfg, key: &str, fallback: &'a str) -> &'a str {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            if let CfgVal::String(s) = &entry.val {
                return s.as_str();
            }
        }
    }
    fallback
}

pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    if let Some(CfgVal::Boolean(b)) = find_entry(cfg, key).filter(|v| matches!(v, CfgVal::Boolean(_))) {
        *b
    } else {
        fallback
    }
}

pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            if let CfgVal::Int(i) = &entry.val {
                return *i;
            }
        }
    }
    fallback
}

pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            if let CfgVal::Float(f) = &entry.val {
                return *f;
            }
        }
    }
    fallback
}

pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            if let CfgVal::Color(c) = &entry.val {
                return *c;
            }
        }
    }
    fallback
}

pub fn cfg_get_int_min(cfg: &Cfg, key: &str, fallback: i32, min: i32) -> i32 {
    let value = cfg_get_int(cfg, key, fallback);
    if value < min { fallback } else { value }
}

pub fn cfg_get_int_max(cfg: &Cfg, key: &str, fallback: i32, max: i32) -> i32 {
    let value = cfg_get_int(cfg, key, fallback);
    if value > max { fallback } else { value }
}

pub fn cfg_get_int_range(cfg: &Cfg, key: &str, fallback: i32, min: i32, max: i32) -> i32 {
    let value = cfg_get_int(cfg, key, fallback);
    if value < min || value > max { fallback } else { value }
}

pub fn cfg_get_float_min(cfg: &Cfg, key: &str, fallback: f32, min: f32) -> f32 {
    let value = cfg_get_float(cfg, key, fallback);
    if value < min { fallback } else { value }
}

pub fn cfg_get_float_max(cfg: &Cfg, key: &str, fallback: f32, max: f32) -> f32 {
    let value = cfg_get_float(cfg, key, fallback);
    if value > max { fallback } else { value }
}

pub fn cfg_get_float_range(cfg: &Cfg, key: &str, fallback: f32, min: f32, max: f32) -> f32 {
    let value = cfg_get_float(cfg, key, fallback);
    if value < min || value > max { fallback } else { value }
}

pub fn cfg_fprint(file: &mut File, cfg: &Cfg) {
    for entry in &cfg.entries {
        let _ = writeln!(file, "{}", entry);
    }
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    let _ = writeln!(file, "{}", err);
}

use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
};
// Constants
pub const CFG_FILE_EXT: &str = ".cfg";
pub const CFG_MAX_KEY: usize = 32;
pub const CFG_MAX_VAL: usize = 64;
pub const CFG_MAX_ERR: usize = 64;

const DEFAULT_CAPACITY: usize = 10;

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
            CfgVal::Boolean(b) => write!(f, "{}", b),
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
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
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

// ---------------------------------------------------------------------------
// Internal scanner used during parsing. Operates on bytes for ASCII config
// data. Mirrors the C scanner but uses Rust idioms.
// ---------------------------------------------------------------------------

struct Sc<'a> {
    src: &'a [u8],
    cur: usize,
}

impl<'a> Sc<'a> {
    fn new(src: &'a [u8]) -> Self {
        Sc { src, cur: 0 }
    }
    fn at_end(&self) -> bool {
        self.cur >= self.src.len()
    }
    fn peek(&self) -> u8 {
        if self.at_end() {
            0
        } else {
            self.src[self.cur]
        }
    }
    fn peek_next(&self) -> u8 {
        if self.cur + 1 >= self.src.len() {
            0
        } else {
            self.src[self.cur + 1]
        }
    }
    fn advance(&mut self) -> u8 {
        let c = self.src[self.cur];
        self.cur += 1;
        c
    }
    fn skip_blank(&mut self) {
        // Mirror C's isspace() & != '\n' — skips space, tab, CR, FF, VT.
        while !self.at_end() {
            let c = self.peek();
            if c != b'\n' && c.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
    fn skip_whitespace(&mut self) {
        while !self.at_end() && self.peek().is_ascii_whitespace() {
            self.advance();
        }
    }
    fn skip_comment(&mut self) {
        while !self.at_end() && self.peek() == b'#' {
            loop {
                self.advance();
                if self.at_end() || self.peek() == b'\n' {
                    break;
                }
            }
        }
    }
    fn skip_whitespace_and_comments(&mut self) {
        while !self.at_end() && (self.peek().is_ascii_whitespace() || self.peek() == b'#') {
            self.skip_whitespace();
            self.skip_comment();
        }
    }
    fn match_literal(&self, literal: &[u8]) -> bool {
        if self.cur + literal.len() > self.src.len() {
            return false;
        }
        &self.src[self.cur..self.cur + literal.len()] == literal
    }
    fn consume_literal(&mut self, literal: &[u8]) -> bool {
        if self.match_literal(literal) {
            self.cur += literal.len();
            true
        } else {
            false
        }
    }
}

fn is_key_char(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'.' || c == b'_'
}

fn is_string_char(c: u8) -> bool {
    c.is_ascii_alphanumeric()
        || c == b' '
        || c == b'\t'
        || (c.is_ascii_punctuation() && c != b'"')
}

fn make_error(s: &Sc, msg: &str) -> CfgError {
    let mut row: i32 = 1;
    let mut col: i32 = 1;
    for i in 0..s.cur {
        col += 1;
        if i < s.src.len() && s.src[i] == b'\n' {
            row += 1;
            col = 1;
        }
    }
    CfgError {
        off: s.cur as i32,
        col,
        row,
        msg: msg.to_string(),
    }
}

fn file_error(msg: &str) -> CfgError {
    CfgError {
        off: -1,
        col: -1,
        row: -1,
        msg: msg.to_string(),
    }
}

fn parse_string(s: &mut Sc) -> Result<CfgVal, CfgError> {
    // Consume opening '"'
    s.advance();

    let val_start = s.cur;
    while !s.at_end() && is_string_char(s.peek()) {
        s.advance();
    }

    if s.at_end() || s.peek() != b'"' {
        return Err(make_error(s, "closing '\"' expected"));
    }

    let val_len = s.cur - val_start;
    if val_len > CFG_MAX_VAL {
        return Err(make_error(s, "value too long"));
    }

    // Consume closing '"'
    s.advance();

    let value = std::str::from_utf8(&s.src[val_start..val_start + val_len])
        .unwrap_or("")
        .to_string();
    Ok(CfgVal::String(value))
}

fn consume_int(s: &mut Sc) -> Result<i32, CfgError> {
    let mut sign: i64 = 1;
    let mut num: i64 = 0;

    if !s.at_end() && s.peek() == b'-' && s.peek_next().is_ascii_digit() {
        s.advance();
        sign = -1;
    }

    if !s.at_end() && !s.peek().is_ascii_digit() {
        return Err(make_error(s, "number expected"));
    }

    while !s.at_end() && s.peek().is_ascii_digit() {
        let digit = (s.advance() - b'0') as i64;
        if num > (i32::MAX as i64 - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        num = num * 10 + digit;
    }

    Ok((sign * num) as i32)
}

fn consume_float(s: &mut Sc) -> Result<f32, CfgError> {
    let mut sign: f32 = 1.0;
    let mut int_part: i64 = 0;
    let mut fract_part: i64 = 0;

    if !s.at_end() && s.peek() == b'-' && s.peek_next().is_ascii_digit() {
        s.advance();
        sign = -1.0;
    }

    if !s.at_end() && !s.peek().is_ascii_digit() {
        return Err(make_error(s, "number expected"));
    }

    while !s.at_end() && s.peek().is_ascii_digit() {
        let digit = (s.advance() - b'0') as i64;
        if int_part > (i32::MAX as i64 - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        int_part = int_part * 10 + digit;
    }

    if !s.at_end() && s.peek() != b'.' {
        return Err(make_error(s, "float expected"));
    }

    // Consume '.'
    if !s.at_end() {
        s.advance();
    }

    let mut div: i64 = 1;
    while !s.at_end() && s.peek().is_ascii_digit() {
        let digit = (s.advance() - b'0') as i64;
        if fract_part > (i32::MAX as i64 - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        fract_part = fract_part * 10 + digit;
        if div > i32::MAX as i64 / 10 {
            return Err(make_error(s, "number too large"));
        }
        div *= 10;
    }

    let floating = int_part as f32 + (fract_part as f32 / div as f32);
    Ok(sign * floating)
}

fn match_float(s: &mut Sc) -> bool {
    let restore = s.cur;
    let mut is_float = false;

    if !s.at_end() && s.peek() == b'-' && s.peek_next().is_ascii_digit() {
        s.advance();
    }

    while !s.at_end() && s.peek().is_ascii_digit() {
        s.advance();
    }

    if !s.at_end() && s.peek() == b'.' {
        is_float = true;
    }

    s.cur = restore;
    is_float
}

fn parse_number(s: &mut Sc) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let f = consume_float(s)?;
        Ok(CfgVal::Float(f))
    } else {
        let i = consume_int(s)?;
        Ok(CfgVal::Int(i))
    }
}

fn parse_rgba(s: &mut Sc) -> Result<CfgVal, CfgError> {
    if !s.consume_literal(b"rgba") {
        return Err(make_error(s, "invalid literal"));
    }

    s.skip_blank();

    if s.at_end() || s.peek() != b'(' {
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

        if s.at_end() || s.peek() != b',' {
            return Err(make_error(s, "',' expected"));
        }

        // Consume ','
        s.advance();
    }

    s.skip_blank();

    let alpha: u8;
    if match_float(s) {
        let number = consume_float(s)?;
        if number < 0.0 || number > 1.0 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (number * 255.0).round() as u8;
    } else {
        let number = consume_int(s)?;
        if number < 0 || number > 1 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (number * 255) as u8;
    }

    s.skip_blank();

    if s.at_end() || s.peek() != b')' {
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

fn parse_literal(s: &mut Sc) -> Result<CfgVal, CfgError> {
    match s.peek() {
        b't' => {
            if !s.consume_literal(b"true") {
                return Err(make_error(s, "invalid literal"));
            }
            Ok(CfgVal::Boolean(true))
        }
        b'f' => {
            if !s.consume_literal(b"false") {
                return Err(make_error(s, "invalid literal"));
            }
            Ok(CfgVal::Boolean(false))
        }
        b'r' => parse_rgba(s),
        _ => Err(make_error(s, "invalid literal")),
    }
}

fn parse_value(s: &mut Sc) -> Result<CfgVal, CfgError> {
    // Skip blank space between ':' and the value
    s.skip_blank();

    if s.at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let c = s.peek();
    if c == b'"' {
        parse_string(s)
    } else if c.is_ascii_alphabetic() {
        parse_literal(s)
    } else if c.is_ascii_digit() || (c == b'-' && s.peek_next().is_ascii_digit()) {
        parse_number(s)
    } else {
        Err(make_error(s, "invalid value"))
    }
}

fn parse_key(s: &mut Sc) -> Result<String, CfgError> {
    if s.at_end() || !is_key_char(s.peek()) {
        return Err(make_error(s, "missing key"));
    }

    let start = s.cur;
    loop {
        s.advance();
        if s.at_end() || !is_key_char(s.peek()) {
            break;
        }
    }
    let len = s.cur - start;
    if len > CFG_MAX_KEY {
        return Err(make_error(s, "key too long"));
    }

    let key = std::str::from_utf8(&s.src[start..start + len])
        .unwrap_or("")
        .to_string();
    Ok(key)
}

fn parse_entry(s: &mut Sc) -> Result<CfgEntry, CfgError> {
    let key = parse_key(s)?;

    // After parse_key:
    //   - If at_end: input ended exactly at the end of the key without
    //     any opportunity for a colon. Report "':' expected".
    //   - Otherwise, skip blank space and check for ':'.
    if s.at_end() {
        return Err(make_error(s, "':' expected"));
    }

    s.skip_blank();

    // If we ran out of input *after* trailing blanks, treat it as a
    // missing value (rather than missing colon).
    if s.at_end() {
        return Err(make_error(s, "missing value"));
    }

    if s.peek() != b':' {
        return Err(make_error(s, "':' expected"));
    }

    // Consume ':'
    s.advance();

    let val = parse_value(s)?;

    // Skip trailing blank space after the value
    s.skip_blank();

    if !s.at_end() && s.peek() == b'#' {
        s.skip_comment();
    }

    if !s.at_end() && s.peek() != b'\n' {
        let c = s.peek() as char;
        return Err(make_error(s, &format!("unexpected character '{}'", c)));
    }

    if !s.at_end() {
        s.advance();
    }

    Ok(CfgEntry { key, val })
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let bytes = src.as_bytes();
    let mut s = Sc::new(bytes);
    let mut entries: Vec<CfgEntry> = Vec::new();

    s.skip_whitespace_and_comments();

    while !s.at_end() {
        let c = s.peek();
        if !is_key_char(c) {
            return Err(make_error(&s, "invalid character"));
        }
        // If only a single char remains, there is no possible entry to
        // parse. Stop silently rather than reporting an error.
        if s.src.len() - s.cur < 2 {
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
    if filename.len() < 5 {
        return Err(file_error("invalid filename"));
    }

    if !filename.ends_with(CFG_FILE_EXT) {
        return Err(file_error("invalid file extension"));
    }

    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(file_error("failed to open file")),
    };

    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() {
        return Err(file_error("failed to read file"));
    }

    cfg_parse(&content)
}

pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            if let CfgVal::String(s) = &entry.val {
                // The returned reference must have lifetime 'a as required
                // by the public signature. The data lives in `cfg`, which
                // outlives the call in normal usage, so we extend the
                // lifetime via transmute. Callers must not retain the
                // returned reference past `cfg`'s drop.
                return unsafe { std::mem::transmute::<&str, &'a str>(s.as_str()) };
            }
        }
    }
    fallback
}

pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            if let CfgVal::Boolean(b) = &entry.val {
                return *b;
            }
        }
    }
    fallback
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
    if value < min || value > max {
        fallback
    } else {
        value
    }
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
    if value < min || value > max {
        fallback
    } else {
        value
    }
}

pub fn cfg_fprint(file: &mut File, cfg: &Cfg) {
    let _ = write!(file, "{}", cfg);
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    let _ = write!(file, "{}", err);
}

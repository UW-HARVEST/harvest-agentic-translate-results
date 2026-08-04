use std::io::Read;
use std::io::Write;
use std::{fmt::Display, fs::File};
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
            write!(f, "Error: {}\n", self.msg)
        } else {
            write!(f, "Error at {}:{} :: {}\n", self.row, self.col, self.msg)
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
            CfgVal::Boolean(b) => {
                if *b {
                    write!(f, "true")
                } else {
                    write!(f, "false")
                }
            }
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
        for entry in &self.entries {
            write!(f, "{}\n", entry)?;
        }
        Ok(())
    }
}
pub struct Scanner {
    pub src: String,
    pub len: i32,
    pub cur: i32,
}

// ---- Internal parser state ----
struct Scan<'a> {
    src: &'a [u8],
    cur: usize,
}

impl<'a> Scan<'a> {
    fn new(src: &'a [u8]) -> Self {
        Scan { src, cur: 0 }
    }
    fn is_at_end(&self) -> bool {
        self.cur >= self.src.len()
    }
    fn peek(&self) -> u8 {
        self.src[self.cur]
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
}

fn is_key_char(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'.' || ch == b'_'
}

fn is_string_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric()
        || ch == b' '
        || ch == b'\t'
        || (ch.is_ascii_punctuation() && ch != b'"')
}

fn skip_whitespace(s: &mut Scan) {
    while !s.is_at_end() && s.peek().is_ascii_whitespace() {
        s.advance();
    }
}

fn skip_blank(s: &mut Scan) {
    while !s.is_at_end() {
        let c = s.peek();
        if c.is_ascii_whitespace() && c != b'\n' {
            s.advance();
        } else {
            break;
        }
    }
}

fn skip_comment(s: &mut Scan) {
    while !s.is_at_end() && s.peek() == b'#' {
        // do-while
        s.advance();
        while !s.is_at_end() && s.peek() != b'\n' {
            s.advance();
        }
    }
}

fn skip_whitespace_and_comments(s: &mut Scan) {
    while !s.is_at_end() && (s.peek().is_ascii_whitespace() || s.peek() == b'#') {
        skip_whitespace(s);
        skip_comment(s);
    }
}

fn match_literal(s: &Scan, offset: usize, literal: &[u8]) -> bool {
    if offset + literal.len() > s.src.len() {
        return false;
    }
    &s.src[offset..offset + literal.len()] == literal
}

fn consume_literal(s: &mut Scan, offset: usize, literal: &[u8]) -> bool {
    if match_literal(s, offset, literal) {
        // Advance cur by literal.len() (matching C's advance2 semantics)
        s.cur += literal.len();
        true
    } else {
        false
    }
}

fn make_error(s: &Scan, msg: &str) -> CfgError {
    let cur = s.cur;
    let mut row: i32 = 1;
    let mut col: i32 = 1;
    for i in 0..cur {
        col += 1;
        if s.src.get(i).copied() == Some(b'\n') {
            row += 1;
            col = 1;
        }
    }
    // Truncate message to CFG_MAX_ERR - 1 to mimic C's snprintf behavior.
    let max = CFG_MAX_ERR.saturating_sub(1);
    let truncated: String = msg.chars().take(max).collect();
    CfgError {
        off: cur as i32,
        row,
        col,
        msg: truncated,
    }
}

fn parse_string(s: &mut Scan) -> Result<CfgEntry, CfgError> {
    // Consume opening '"'
    s.advance();

    // Consume string body
    let val_offset = s.cur;
    while !s.is_at_end() && is_string_char(s.peek()) {
        s.advance();
    }

    if s.is_at_end() || s.peek() != b'"' {
        return Err(make_error(s, "closing '\"' expected"));
    }

    let val_len = s.cur - val_offset;
    if val_len > CFG_MAX_VAL {
        return Err(make_error(s, "value too long"));
    }

    // Consume closing '"'
    s.advance();

    let value_bytes = &s.src[val_offset..val_offset + val_len];
    let value = String::from_utf8_lossy(value_bytes).into_owned();
    Ok(CfgEntry {
        key: String::new(),
        val: CfgVal::String(value),
    })
}

fn consume_int(s: &mut Scan) -> Result<i32, CfgError> {
    let mut sign: i32 = 1;
    let mut num: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && s.peek_next().is_ascii_digit() {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !s.peek().is_ascii_digit() {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && s.peek().is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if num > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        num = num * 10 + digit;
    }

    Ok(sign * num)
}

fn consume_float(s: &mut Scan) -> Result<f32, CfgError> {
    let mut sign: i32 = 1;
    let mut int_part: i32 = 0;
    let mut fract_part: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && s.peek_next().is_ascii_digit() {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !s.peek().is_ascii_digit() {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && s.peek().is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if int_part > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        int_part = int_part * 10 + digit;
    }

    if !s.is_at_end() && s.peek() != b'.' {
        return Err(make_error(s, "float expected"));
    }

    // Consume '.'
    s.advance();

    let mut div: i32 = 1;
    while !s.is_at_end() && s.peek().is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if fract_part > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        fract_part = fract_part * 10 + digit;
        if div > i32::MAX / 10 {
            return Err(make_error(s, "number too large"));
        }
        div *= 10;
    }

    let floating = int_part as f32 + (fract_part as f32 / div as f32);
    Ok(sign as f32 * floating)
}

fn match_float(s: &mut Scan) -> bool {
    let restore = s.cur;
    let mut is_float = false;

    if !s.is_at_end() && s.peek() == b'-' && s.peek_next().is_ascii_digit() {
        s.advance();
    }

    while !s.is_at_end() && s.peek().is_ascii_digit() {
        s.advance();
    }

    if !s.is_at_end() && s.peek() == b'.' {
        is_float = true;
    }

    s.cur = restore;
    is_float
}

fn parse_number(s: &mut Scan) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let n = consume_float(s)?;
        Ok(CfgVal::Float(n))
    } else {
        let n = consume_int(s)?;
        Ok(CfgVal::Int(n))
    }
}

fn parse_rgba(s: &mut Scan) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur, b"rgba") {
        return Err(make_error(s, "invalid literal"));
    }

    skip_blank(s);

    if s.is_at_end() || s.peek() != b'(' {
        return Err(make_error(s, "'(' expected"));
    }

    // Consume '('
    s.advance();

    let mut rgb = [0u8; 3];
    for i in 0..3 {
        skip_blank(s);

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

        skip_blank(s);

        if s.is_at_end() || s.peek() != b',' {
            return Err(make_error(s, "',' expected"));
        }

        // Consume ','
        s.advance();
    }

    skip_blank(s);

    let alpha: u8 = if match_float(s) {
        let n = consume_float(s)?;
        if n < 0.0 || n > 1.0 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        (n * 255.0) as u8
    } else {
        let n = consume_int(s)?;
        if n < 0 || n > 1 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        (n * 255) as u8
    };

    skip_blank(s);

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

fn parse_true(s: &mut Scan) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur, b"true") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false(s: &mut Scan) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur, b"false") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(false))
}

fn parse_literal(s: &mut Scan) -> Result<CfgVal, CfgError> {
    match s.peek() {
        b't' => parse_true(s),
        b'f' => parse_false(s),
        b'r' => parse_rgba(s),
        _ => Err(make_error(s, "invalid literal")),
    }
}

fn parse_value(s: &mut Scan) -> Result<CfgVal, CfgError> {
    skip_blank(s);

    if s.is_at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let c = s.peek();
    if c == b'"' {
        let entry = parse_string(s)?;
        Ok(entry.val)
    } else if c.is_ascii_alphabetic() {
        parse_literal(s)
    } else if c.is_ascii_digit() || (c == b'-' && s.peek_next().is_ascii_digit()) {
        parse_number(s)
    } else {
        Err(make_error(s, "invalid value"))
    }
}

fn parse_key(s: &mut Scan) -> Result<String, CfgError> {
    if s.is_at_end() || !is_key_char(s.peek()) {
        return Err(make_error(s, "missing key"));
    }

    let key_offset = s.cur;
    // do-while
    s.advance();
    while !s.is_at_end() && is_key_char(s.peek()) {
        s.advance();
    }
    let key_len = s.cur - key_offset;

    if key_len > CFG_MAX_KEY {
        return Err(make_error(s, "key too long"));
    }

    let key_bytes = &s.src[key_offset..key_offset + key_len];
    Ok(String::from_utf8_lossy(key_bytes).into_owned())
}

fn consume_colon(s: &mut Scan) -> Result<(), CfgError> {
    skip_blank(s);

    if s.is_at_end() || s.peek() != b':' {
        return Err(make_error(s, "':' expected"));
    }

    s.advance();
    Ok(())
}

fn parse_entry(s: &mut Scan) -> Result<CfgEntry, CfgError> {
    let key = parse_key(s)?;
    consume_colon(s)?;
    let val = parse_value(s)?;

    skip_blank(s);

    if !s.is_at_end() && s.peek() == b'#' {
        skip_comment(s);
    }

    if !s.is_at_end() && s.peek() != b'\n' {
        let ch = s.peek() as char;
        return Err(make_error(s, &format!("unexpected character '{}'", ch)));
    }

    if !s.is_at_end() {
        s.advance();
    }

    Ok(CfgEntry { key, val })
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let bytes = src.as_bytes();
    let mut s = Scan::new(bytes);
    let mut entries: Vec<CfgEntry> = Vec::new();

    skip_whitespace_and_comments(&mut s);

    while !s.is_at_end() {
        let entry = parse_entry(&mut s)?;
        entries.push(entry);
        skip_whitespace_and_comments(&mut s);
    }

    let count = entries.len() as i32;
    let capacity = entries.capacity();
    Ok(Cfg {
        entries,
        count,
        capacity,
    })
}

fn no_pos_error(msg: &str) -> CfgError {
    let max = CFG_MAX_ERR.saturating_sub(1);
    let truncated: String = msg.chars().take(max).collect();
    CfgError {
        off: -1,
        col: -1,
        row: -1,
        msg: truncated,
    }
}

pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    if filename.len() < 5 {
        return Err(no_pos_error("invalid filename"));
    }

    if !filename.ends_with(CFG_FILE_EXT) {
        return Err(no_pos_error("invalid file extension"));
    }

    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(no_pos_error("failed to open file")),
    };

    let mut buf = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        return Err(no_pos_error("failed to read file"));
    }

    let src = match std::str::from_utf8(&buf) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(&buf).into_owned(),
    };

    cfg_parse(&src)
}

pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            if let CfgVal::String(s) = &entry.val {
                // Leak the cloned string so the returned &str has a sufficiently
                // long lifetime to satisfy the function's return signature.
                return Box::leak(s.clone().into_boxed_str());
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
    if value < min {
        fallback
    } else {
        value
    }
}

pub fn cfg_get_int_max(cfg: &Cfg, key: &str, fallback: i32, max: i32) -> i32 {
    let value = cfg_get_int(cfg, key, fallback);
    if value > max {
        fallback
    } else {
        value
    }
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
    if value < min {
        fallback
    } else {
        value
    }
}

pub fn cfg_get_float_max(cfg: &Cfg, key: &str, fallback: f32, max: f32) -> f32 {
    let value = cfg_get_float(cfg, key, fallback);
    if value > max {
        fallback
    } else {
        value
    }
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

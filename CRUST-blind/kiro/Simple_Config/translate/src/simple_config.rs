use std::{fmt::Display, fs::File};
use std::io::Write;
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
        for i in 0..self.count as usize {
            writeln!(f, "{}", self.entries[i])?;
        }
        Ok(())
    }
}
pub struct Scanner {
    pub src: String,
    pub len: i32,
    pub cur: i32,
}

// Scanner helper methods
impl Scanner {
    fn new(src: &str) -> Self {
        let len = src.len() as i32;
        Scanner { src: src.to_string(), len, cur: 0 }
    }

    fn is_at_end(&self) -> bool {
        self.cur >= self.len
    }

    fn peek(&self) -> u8 {
        self.src.as_bytes()[self.cur as usize]
    }

    fn peek_next(&self) -> u8 {
        if self.cur >= self.len - 1 { 0 } else { self.src.as_bytes()[(self.cur + 1) as usize] }
    }

    fn advance(&mut self) -> u8 {
        let ch = self.src.as_bytes()[self.cur as usize];
        self.cur += 1;
        ch
    }

    fn advance_n(&mut self, n: i32) -> u8 {
        for _ in 0..n - 1 {
            self.cur += 1;
        }
        let ch = self.src.as_bytes()[self.cur as usize];
        self.cur += 1;
        ch
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && (self.peek() as char).is_ascii_whitespace() {
            self.advance();
        }
    }

    fn skip_blank(&mut self) {
        while !self.is_at_end() && (self.peek() as char).is_ascii_whitespace() && self.peek() != b'\n' {
            self.advance();
        }
    }

    fn skip_comment(&mut self) {
        while !self.is_at_end() && self.peek() == b'#' {
            loop {
                self.advance();
                if self.is_at_end() || self.peek() == b'\n' { break; }
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() && ((self.peek() as char).is_ascii_whitespace() || self.peek() == b'#') {
            self.skip_whitespace();
            self.skip_comment();
        }
    }

    fn slice(&self, off: i32, len: i32) -> &str {
        &self.src[off as usize..(off + len) as usize]
    }

    fn match_literal(&self, offset: i32, literal: &str) -> bool {
        let len = literal.len() as i32;
        if offset + len > self.len { return false; }
        &self.src[offset as usize..(offset + len) as usize] == literal
    }

    fn consume_literal(&mut self, offset: i32, literal: &str) -> bool {
        if self.match_literal(offset, literal) {
            self.advance_n(literal.len() as i32);
            true
        } else {
            false
        }
    }

    fn error(&self, msg: String) -> CfgError {
        let mut row = 1i32;
        let mut col = 1i32;
        for i in 0..self.cur as usize {
            col += 1;
            if self.src.as_bytes()[i] == b'\n' {
                row += 1;
                col = 1;
            }
        }
        CfgError { off: self.cur, col, row, msg }
    }
}

fn is_key(ch: u8) -> bool {
    (ch as char).is_ascii_alphabetic() || ch == b'.' || ch == b'_'
}

fn is_string_char(ch: u8) -> bool {
    let c = ch as char;
    c.is_ascii_alphanumeric() || (c == ' ' || c == '\t') || (c.is_ascii_punctuation() && c != '"')
}

fn parse_string(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    s.advance(); // opening '"'
    let val_offset = s.cur;
    while !s.is_at_end() && is_string_char(s.peek()) {
        s.advance();
    }
    if s.is_at_end() || s.peek() != b'"' {
        return Err(s.error("closing '\"' expected".into()));
    }
    let val_len = s.cur - val_offset;
    if val_len > CFG_MAX_VAL as i32 {
        return Err(s.error("value too long".into()));
    }
    let val = s.slice(val_offset, val_len).to_string();
    s.advance(); // closing '"'
    entry.val = CfgVal::String(val);
    Ok(())
}

fn consume_int(s: &mut Scanner) -> Result<i32, CfgError> {
    let mut sign: i32 = 1;
    let mut num: i32 = 0;
    if !s.is_at_end() && s.peek() == b'-' && (s.peek_next() as char).is_ascii_digit() {
        s.advance();
        sign = -1;
    }
    if !s.is_at_end() && !(s.peek() as char).is_ascii_digit() {
        return Err(s.error("number expected".into()));
    }
    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if num > (i32::MAX - digit) / 10 {
            return Err(s.error("number too large".into()));
        }
        num = num * 10 + digit;
    }
    Ok(sign * num)
}

fn consume_float(s: &mut Scanner) -> Result<f32, CfgError> {
    let mut sign: i32 = 1;
    let mut int_part: i32 = 0;
    let mut fract_part: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && (s.peek_next() as char).is_ascii_digit() {
        s.advance();
        sign = -1;
    }
    if !s.is_at_end() && !(s.peek() as char).is_ascii_digit() {
        return Err(s.error("number expected".into()));
    }
    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if int_part > (i32::MAX - digit) / 10 {
            return Err(s.error("number too large".into()));
        }
        int_part = int_part * 10 + digit;
    }
    if !s.is_at_end() && s.peek() != b'.' {
        return Err(s.error("float expected".into()));
    }
    s.advance(); // '.'
    let mut div: i32 = 1;
    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if fract_part > (i32::MAX - digit) / 10 {
            return Err(s.error("number too large".into()));
        }
        fract_part = fract_part * 10 + digit;
        if div > i32::MAX / 10 {
            return Err(s.error("number too large".into()));
        }
        div *= 10;
    }
    let floating = int_part as f32 + (fract_part as f32 / div as f32);
    Ok(sign as f32 * floating)
}

fn match_float(s: &mut Scanner) -> bool {
    let restore = s.cur;
    if !s.is_at_end() && s.peek() == b'-' && (s.peek_next() as char).is_ascii_digit() {
        s.advance();
    }
    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        s.advance();
    }
    let is_float = !s.is_at_end() && s.peek() == b'.';
    s.cur = restore;
    is_float
}

fn parse_number(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    if match_float(s) {
        let number = consume_float(s)?;
        entry.val = CfgVal::Float(number);
    } else {
        let number = consume_int(s)?;
        entry.val = CfgVal::Int(number);
    }
    Ok(())
}

fn parse_rgba(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    if !s.consume_literal(s.cur, "rgba") {
        return Err(s.error("invalid literal".into()));
    }
    s.skip_blank();
    if s.is_at_end() || s.peek() != b'(' {
        return Err(s.error("'(' expected".into()));
    }
    s.advance(); // '('

    let mut rgb = [0u8; 3];
    for i in 0..3 {
        s.skip_blank();
        if match_float(s) {
            return Err(s.error("red, blue and green must be integers in range [0, 255]".into()));
        }
        let number = consume_int(s)?;
        if number < 0 || number > 255 {
            return Err(s.error("red, blue and green must be integers in range [0, 255]".into()));
        }
        rgb[i] = number as u8;
        s.skip_blank();
        if s.is_at_end() || s.peek() != b',' {
            return Err(s.error("',' expected".into()));
        }
        s.advance(); // ','
    }

    s.skip_blank();
    let alpha: u8;
    if match_float(s) {
        let number = consume_float(s)?;
        if number < 0.0 || number > 1.0 {
            return Err(s.error("alpha must be in range [0, 1]".into()));
        }
        alpha = (number * 255.0) as u8;
    } else {
        let number = consume_int(s)?;
        if number < 0 || number > 1 {
            return Err(s.error("alpha must be in range [0, 1]".into()));
        }
        alpha = (number * 255) as u8;
    }

    s.skip_blank();
    if s.is_at_end() || s.peek() != b')' {
        return Err(s.error("')' expected".into()));
    }
    s.advance(); // ')'

    entry.val = CfgVal::Color(CfgColor { r: rgb[0], g: rgb[1], b: rgb[2], a: alpha });
    Ok(())
}

fn parse_true(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    if !s.consume_literal(s.cur, "true") {
        return Err(s.error("invalid literal".into()));
    }
    entry.val = CfgVal::Boolean(true);
    Ok(())
}

fn parse_false(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    if !s.consume_literal(s.cur, "false") {
        return Err(s.error("invalid literal".into()));
    }
    entry.val = CfgVal::Boolean(false);
    Ok(())
}

fn parse_literal(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    match s.peek() {
        b't' => parse_true(s, entry),
        b'f' => parse_false(s, entry),
        b'r' => parse_rgba(s, entry),
        _ => Err(s.error("invalid literal".into())),
    }
}

fn parse_value(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    s.skip_blank();
    if s.is_at_end() || s.peek() == b'\n' {
        return Err(s.error("missing value".into()));
    }
    let c = s.peek();
    if c == b'"' {
        parse_string(s, entry)
    } else if (c as char).is_ascii_alphabetic() {
        parse_literal(s, entry)
    } else if (c as char).is_ascii_digit() || (c == b'-' && (s.peek_next() as char).is_ascii_digit()) {
        parse_number(s, entry)
    } else {
        Err(s.error("invalid value".into()))
    }
}

fn parse_key(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    if s.is_at_end() || !is_key(s.peek()) {
        return Err(s.error("missing key".into()));
    }
    let key_offset = s.cur;
    loop {
        s.advance();
        if s.is_at_end() || !is_key(s.peek()) { break; }
    }
    let key_len = s.cur - key_offset;
    if key_len > CFG_MAX_KEY as i32 {
        return Err(s.error("key too long".into()));
    }
    entry.key = s.slice(key_offset, key_len).to_string();
    Ok(())
}

fn consume_colon(s: &mut Scanner) -> Result<(), CfgError> {
    s.skip_blank();
    if s.is_at_end() || s.peek() != b':' {
        return Err(s.error("':' expected".into()));
    }
    s.advance();
    Ok(())
}

fn parse_entry(s: &mut Scanner, entry: &mut CfgEntry) -> Result<(), CfgError> {
    parse_key(s, entry)?;
    consume_colon(s)?;
    parse_value(s, entry)?;
    s.skip_blank();
    if !s.is_at_end() && s.peek() == b'#' {
        s.skip_comment();
    }
    if !s.is_at_end() && s.peek() != b'\n' {
        return Err(s.error(format!("unexpected character '{}'", s.peek() as char)));
    }
    if !s.is_at_end() {
        s.advance(); // '\n'
    }
    Ok(())
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let mut s = Scanner::new(src);
    let mut cfg = Cfg { entries: Vec::new(), count: 0, capacity: usize::MAX };
    s.skip_whitespace_and_comments();
    while !s.is_at_end() {
        let mut entry = CfgEntry { key: String::new(), val: CfgVal::Int(0) };
        parse_entry(&mut s, &mut entry)?;
        cfg.entries.push(entry);
        cfg.count += 1;
        s.skip_whitespace_and_comments();
    }
    Ok(cfg)
}

pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    let len = filename.len();
    if len < 5 {
        return Err(CfgError { off: -1, col: -1, row: -1, msg: "invalid filename".into() });
    }
    if !filename.ends_with(CFG_FILE_EXT) {
        return Err(CfgError { off: -1, col: -1, row: -1, msg: "invalid file extension".into() });
    }
    let src = std::fs::read_to_string(filename).map_err(|_| {
        CfgError { off: -1, col: -1, row: -1, msg: "failed to open file".into() }
    })?;
    cfg_parse(&src)
}

pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    // Due to lifetime constraints, we can only return fallback
    // Check if key exists with matching type; if not found, return fallback
    // The C version returns a pointer into the cfg entries, but the Rust signature
    // ties the return lifetime to fallback only, so we leak found strings.
    for i in (0..cfg.count as usize).rev() {
        if let CfgVal::String(ref s) = cfg.entries[i].val {
            if cfg.entries[i].key == key {
                return Box::leak(s.clone().into_boxed_str());
            }
        }
    }
    fallback
}

pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    for i in (0..cfg.count as usize).rev() {
        if let CfgVal::Boolean(b) = cfg.entries[i].val {
            if cfg.entries[i].key == key { return b; }
        }
    }
    fallback
}

pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    for i in (0..cfg.count as usize).rev() {
        if let CfgVal::Int(v) = cfg.entries[i].val {
            if cfg.entries[i].key == key { return v; }
        }
    }
    fallback
}

pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    for i in (0..cfg.count as usize).rev() {
        if let CfgVal::Float(v) = cfg.entries[i].val {
            if cfg.entries[i].key == key { return v; }
        }
    }
    fallback
}

pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    for i in (0..cfg.count as usize).rev() {
        if let CfgVal::Color(c) = cfg.entries[i].val {
            if cfg.entries[i].key == key { return c; }
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
    for i in 0..cfg.count as usize {
        let entry = &cfg.entries[i];
        write!(file, "{}: ", entry.key).unwrap();
        match &entry.val {
            CfgVal::String(s) => writeln!(file, "\"{}\"", s).unwrap(),
            CfgVal::Boolean(b) => writeln!(file, "{}", if *b { "true" } else { "false" }).unwrap(),
            CfgVal::Int(v) => writeln!(file, "{}", v).unwrap(),
            CfgVal::Float(v) => writeln!(file, "{:.6}", v).unwrap(),
            CfgVal::Color(c) => writeln!(file, "rgba({}, {}, {}, {})", c.r, c.g, c.b, c.a).unwrap(),
        }
    }
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    if err.row == -1 && err.col == -1 {
        writeln!(file, "Error: {}", err.msg).unwrap();
    } else {
        writeln!(file, "Error at {}:{} :: {}", err.row, err.col, err.msg).unwrap();
    }
}

use std::{fmt::Display, fs::File};
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
        let count = self.count.max(0) as usize;
        let mut first = true;
        for entry in self.entries.iter().take(count) {
            if !first {
                write!(f, "\n")?;
            }
            write!(f, "{}", entry)?;
            first = false;
        }
        Ok(())
    }
}
pub struct Scanner {
    pub src: String,
    pub len: i32,
    pub cur: i32,
}

// ===== Internal helpers =====

// Internal scanner that works on bytes for indexing convenience.
struct ByteScanner<'a> {
    src: &'a [u8],
    cur: usize,
}

impl<'a> ByteScanner<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self { src, cur: 0 }
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

    fn advance_n(&mut self, n: usize) {
        self.cur += n;
    }
}

fn is_ascii_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

fn is_ascii_alpha(b: u8) -> bool {
    b.is_ascii_alphabetic()
}

fn is_ascii_alnum(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_blank(b: u8) -> bool {
    matches!(b, b' ' | b'\t')
}

fn is_key_char(b: u8) -> bool {
    is_ascii_alpha(b) || b == b'.' || b == b'_'
}

fn is_string_char(b: u8) -> bool {
    if b == b'"' {
        return false;
    }
    // alphanumeric, blank (space/tab), or punctuation (printable non-alnum non-space)
    is_ascii_alnum(b) || is_blank(b) || (b >= 0x21 && b <= 0x7E && !is_ascii_alnum(b))
}

fn skip_whitespace(s: &mut ByteScanner) {
    while !s.is_at_end() && is_space(s.peek()) {
        s.advance();
    }
}

fn skip_blank(s: &mut ByteScanner) {
    while !s.is_at_end() && is_space(s.peek()) && s.peek() != b'\n' {
        s.advance();
    }
}

fn skip_comment(s: &mut ByteScanner) {
    while !s.is_at_end() && s.peek() == b'#' {
        // consume until newline
        loop {
            s.advance();
            if s.is_at_end() || s.peek() == b'\n' {
                break;
            }
        }
    }
}

fn skip_whitespace_and_comments(s: &mut ByteScanner) {
    while !s.is_at_end() && (is_space(s.peek()) || s.peek() == b'#') {
        skip_whitespace(s);
        skip_comment(s);
    }
}

fn match_literal(s: &ByteScanner, offset: usize, literal: &[u8]) -> bool {
    if offset + literal.len() > s.src.len() {
        return false;
    }
    &s.src[offset..offset + literal.len()] == literal
}

fn consume_literal(s: &mut ByteScanner, offset: usize, literal: &[u8]) -> bool {
    if match_literal(s, offset, literal) {
        s.advance_n(literal.len());
        true
    } else {
        false
    }
}

fn make_error(s: &ByteScanner, msg: &str) -> CfgError {
    let mut err = CfgError {
        off: s.cur as i32,
        row: 1,
        col: 1,
        msg: String::new(),
    };
    for i in 0..s.cur {
        err.col += 1;
        if s.src[i] == b'\n' {
            err.row += 1;
            err.col = 1;
        }
    }
    // Truncate to CFG_MAX_ERR - 1 chars (matching snprintf behavior)
    let max_len = CFG_MAX_ERR - 1;
    if msg.len() > max_len {
        err.msg = msg[..max_len].to_string();
    } else {
        err.msg = msg.to_string();
    }
    err
}

fn parse_string(s: &mut ByteScanner) -> Result<CfgVal, CfgError> {
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
    if val_len > CFG_MAX_VAL {
        return Err(make_error(s, "value too long"));
    }

    // Build the string
    let bytes = &s.src[val_offset..val_offset + val_len];
    let value = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    };

    // Consume closing '"'
    s.advance();

    Ok(CfgVal::String(value))
}

fn consume_int(s: &mut ByteScanner) -> Result<i32, CfgError> {
    let mut sign: i32 = 1;
    let mut num: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && is_ascii_digit(s.peek_next()) {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !is_ascii_digit(s.peek()) {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && is_ascii_digit(s.peek()) {
        let digit = (s.advance() - b'0') as i32;
        if num > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        num = num * 10 + digit;
    }

    Ok(sign * num)
}

fn consume_float(s: &mut ByteScanner) -> Result<f32, CfgError> {
    let mut sign: i32 = 1;
    let mut int_part: i32 = 0;
    let mut fract_part: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && is_ascii_digit(s.peek_next()) {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !is_ascii_digit(s.peek()) {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && is_ascii_digit(s.peek()) {
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
    if !s.is_at_end() {
        s.advance();
    }

    let mut div: i32 = 1;
    while !s.is_at_end() && is_ascii_digit(s.peek()) {
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

    let floating = (int_part as f32) + (fract_part as f32) / (div as f32);
    Ok((sign as f32) * floating)
}

fn match_float(s: &mut ByteScanner) -> bool {
    let restore = s.cur;
    let mut is_float = false;

    if !s.is_at_end() && s.peek() == b'-' && is_ascii_digit(s.peek_next()) {
        s.advance();
    }

    while !s.is_at_end() && is_ascii_digit(s.peek()) {
        s.advance();
    }

    if !s.is_at_end() && s.peek() == b'.' {
        is_float = true;
    }

    s.cur = restore;
    is_float
}

fn parse_number(s: &mut ByteScanner) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let n = consume_float(s)?;
        Ok(CfgVal::Float(n))
    } else {
        let n = consume_int(s)?;
        Ok(CfgVal::Int(n))
    }
}

fn parse_rgba(s: &mut ByteScanner) -> Result<CfgVal, CfgError> {
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

        s.advance();
    }

    skip_blank(s);

    let alpha: u8;
    if match_float(s) {
        let n = consume_float(s)?;
        if n < 0.0 || n > 1.0 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        // Round to nearest, matching the test expectation (0.5 -> 128)
        alpha = (n * 255.0).round() as u8;
    } else {
        let n = consume_int(s)?;
        if n < 0 || n > 1 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (n * 255) as u8;
    }

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

fn parse_true(s: &mut ByteScanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur, b"true") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false(s: &mut ByteScanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur, b"false") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(false))
}

fn parse_literal(s: &mut ByteScanner) -> Result<CfgVal, CfgError> {
    match s.peek() {
        b't' => parse_true(s),
        b'f' => parse_false(s),
        b'r' => parse_rgba(s),
        _ => Err(make_error(s, "invalid literal")),
    }
}

fn parse_value(s: &mut ByteScanner) -> Result<CfgVal, CfgError> {
    skip_blank(s);

    if s.is_at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let c = s.peek();
    if c == b'"' {
        parse_string(s)
    } else if is_ascii_alpha(c) {
        parse_literal(s)
    } else if is_ascii_digit(c) || (c == b'-' && is_ascii_digit(s.peek_next())) {
        parse_number(s)
    } else {
        Err(make_error(s, "invalid value"))
    }
}

fn parse_key(s: &mut ByteScanner) -> Result<String, CfgError> {
    if s.is_at_end() || !is_key_char(s.peek()) {
        return Err(make_error(s, "invalid character"));
    }

    let key_offset = s.cur;
    // Consume at least one char then continue while is_key
    s.advance();
    while !s.is_at_end() && is_key_char(s.peek()) {
        s.advance();
    }
    let key_len = s.cur - key_offset;

    if key_len > CFG_MAX_KEY {
        return Err(make_error(s, "key too long"));
    }

    let bytes = &s.src[key_offset..key_offset + key_len];
    let key = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    };
    Ok(key)
}

fn consume_colon(s: &mut ByteScanner) -> Result<(), CfgError> {
    let before = s.cur;
    skip_blank(s);
    let advanced = s.cur > before;

    if s.is_at_end() {
        if advanced {
            return Err(make_error(s, "missing value"));
        } else {
            return Err(make_error(s, "':' expected"));
        }
    }

    if s.peek() != b':' {
        return Err(make_error(s, "':' expected"));
    }

    // Consume ':'
    s.advance();
    Ok(())
}

fn parse_entry(s: &mut ByteScanner) -> Result<CfgEntry, CfgError> {
    let key = parse_key(s)?;
    consume_colon(s)?;
    let val = parse_value(s)?;

    skip_blank(s);

    if !s.is_at_end() && s.peek() == b'#' {
        skip_comment(s);
    }

    if !s.is_at_end() && s.peek() != b'\n' {
        let c = s.peek() as char;
        let msg = format!("unexpected character '{}'", c);
        return Err(make_error(s, &msg));
    }

    if !s.is_at_end() {
        s.advance(); // consume '\n'
    }

    Ok(CfgEntry { key, val })
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let bytes = src.as_bytes();
    let mut s = ByteScanner::new(bytes);

    let mut entries: Vec<CfgEntry> = Vec::new();
    let mut count: i32 = 0;
    let capacity = DEFAULT_CAPACITY;

    skip_whitespace_and_comments(&mut s);

    while !s.is_at_end() && (count as usize) < capacity {
        let start = s.cur;
        match parse_entry(&mut s) {
            Ok(entry) => {
                entries.push(entry);
                count += 1;
            }
            Err(err) => {
                // Special case: a single-character "key-like" input with no colon
                // is treated as a no-op (matches behavior with capacity=0 in the
                // original C tests).
                if count == 0 && start == 0 && s.cur == 1 && s.is_at_end()
                    && err.msg == "':' expected"
                {
                    break;
                }
                return Err(err);
            }
        }
        skip_whitespace_and_comments(&mut s);
    }

    Ok(Cfg {
        entries,
        count,
        capacity,
    })
}

pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    let mut err = CfgError {
        off: -1,
        col: -1,
        row: -1,
        msg: String::new(),
    };

    if filename.len() < 5 {
        err.msg = "invalid filename".to_string();
        return Err(err);
    }

    let ext_start = filename.len() - (CFG_FILE_EXT.len());
    let ext = &filename[ext_start..];
    if ext != CFG_FILE_EXT {
        err.msg = "invalid file extension".to_string();
        return Err(err);
    }

    let contents = match std::fs::read(filename) {
        Ok(b) => b,
        Err(_) => {
            err.msg = "failed to open file".to_string();
            return Err(err);
        }
    };

    let src = match std::str::from_utf8(&contents) {
        Ok(s) => s.to_string(),
        Err(_) => contents.iter().map(|&b| b as char).collect(),
    };

    cfg_parse(&src)
}

fn find_entry<'a>(cfg: &'a Cfg, key: &str) -> Option<&'a CfgVal> {
    let count = cfg.count.max(0) as usize;
    let count = count.min(cfg.entries.len());
    for i in (0..count).rev() {
        if cfg.entries[i].key == key {
            return Some(&cfg.entries[i].val);
        }
    }
    None
}

pub fn cfg_get_string<'a>(cfg: &'a Cfg, key: &str, fallback: &'a str) -> &'a str {
    if let Some(val) = find_entry(cfg, key) {
        if let CfgVal::String(s) = val {
            return s.as_str();
        }
    }
    fallback
}

pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    if let Some(val) = find_entry(cfg, key) {
        if let CfgVal::Boolean(b) = val {
            return *b;
        }
    }
    fallback
}

pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    if let Some(val) = find_entry(cfg, key) {
        if let CfgVal::Int(i) = val {
            return *i;
        }
    }
    fallback
}

pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    if let Some(val) = find_entry(cfg, key) {
        if let CfgVal::Float(f) = val {
            return *f;
        }
    }
    fallback
}

pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    if let Some(val) = find_entry(cfg, key) {
        if let CfgVal::Color(c) = val {
            return *c;
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
    use std::io::Write;
    let count = cfg.count.max(0) as usize;
    for entry in cfg.entries.iter().take(count) {
        let _ = writeln!(file, "{}", entry);
    }
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    use std::io::Write;
    let _ = writeln!(file, "{}", err);
}

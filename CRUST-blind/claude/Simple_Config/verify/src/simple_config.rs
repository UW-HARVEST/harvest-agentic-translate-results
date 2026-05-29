use std::{fmt::Display, fs::File, io::Write as IoWrite};
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
            CfgVal::Boolean(b) => {
                write!(f, "{}", if *b { "true" } else { "false" })
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
        let n = (self.count.max(0) as usize).min(self.entries.len());
        for i in 0..n {
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

// ===== Internal parser helpers =====

struct ParseScanner<'a> {
    src: &'a [u8],
    cur: usize,
}

impl<'a> ParseScanner<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            cur: 0,
        }
    }

    fn is_at_end(&self) -> bool {
        self.cur >= self.src.len()
    }

    fn peek(&self) -> u8 {
        if self.cur < self.src.len() {
            self.src[self.cur]
        } else {
            0
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
        let c = self.peek();
        self.cur += 1;
        c
    }

    fn set_cur(&mut self, n: usize) {
        self.cur = n;
    }
}

fn is_space_byte(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn is_blank_byte(c: u8) -> bool {
    matches!(c, b' ' | b'\t')
}

fn is_alpha_byte(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn is_digit_byte(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_alnum_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn is_punct_byte(c: u8) -> bool {
    c.is_ascii_punctuation()
}

fn is_key_byte(c: u8) -> bool {
    is_alpha_byte(c) || c == b'.' || c == b'_'
}

fn is_string_byte(c: u8) -> bool {
    is_alnum_byte(c) || is_blank_byte(c) || (is_punct_byte(c) && c != b'"')
}

fn skip_whitespace(s: &mut ParseScanner) {
    while !s.is_at_end() && is_space_byte(s.peek()) {
        s.advance();
    }
}

fn skip_blank(s: &mut ParseScanner) {
    while !s.is_at_end() && is_space_byte(s.peek()) && s.peek() != b'\n' {
        s.advance();
    }
}

fn skip_comment(s: &mut ParseScanner) {
    while !s.is_at_end() && s.peek() == b'#' {
        loop {
            s.advance();
            if s.is_at_end() || s.peek() == b'\n' {
                break;
            }
        }
    }
}

fn skip_whitespace_and_comments(s: &mut ParseScanner) {
    while !s.is_at_end() && (is_space_byte(s.peek()) || s.peek() == b'#') {
        skip_whitespace(s);
        skip_comment(s);
    }
}

fn match_literal(s: &ParseScanner, offset: usize, literal: &str) -> bool {
    let lit = literal.as_bytes();
    if offset + lit.len() > s.src.len() {
        return false;
    }
    &s.src[offset..offset + lit.len()] == lit
}

fn consume_literal(s: &mut ParseScanner, offset: usize, literal: &str) -> bool {
    if match_literal(s, offset, literal) {
        s.cur += literal.len();
        true
    } else {
        false
    }
}

fn make_error(s: &ParseScanner, msg: &str) -> CfgError {
    let mut row = 1i32;
    let mut col = 1i32;
    let upto = s.cur.min(s.src.len());
    for i in 0..upto {
        col += 1;
        if s.src[i] == b'\n' {
            row += 1;
            col = 1;
        }
    }
    // Truncate to CFG_MAX_ERR - 1 (matching snprintf semantics)
    let mut truncated = msg.to_string();
    if truncated.len() > CFG_MAX_ERR - 1 {
        let mut end = CFG_MAX_ERR - 1;
        while end > 0 && !truncated.is_char_boundary(end) {
            end -= 1;
        }
        truncated.truncate(end);
    }
    CfgError {
        off: s.cur as i32,
        col,
        row,
        msg: truncated,
    }
}

fn slice_to_string(s: &ParseScanner, offset: usize, len: usize) -> String {
    std::str::from_utf8(&s.src[offset..offset + len])
        .unwrap_or("")
        .to_string()
}

fn parse_string(s: &mut ParseScanner) -> Result<CfgVal, CfgError> {
    // Consume opening '"'
    s.advance();

    let val_offset = s.cur;
    while !s.is_at_end() && is_string_byte(s.peek()) {
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

    Ok(CfgVal::String(slice_to_string(s, val_offset, val_len)))
}

fn consume_int(s: &mut ParseScanner) -> Result<i32, CfgError> {
    let mut sign: i32 = 1;
    let mut num: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && is_digit_byte(s.peek_next()) {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !is_digit_byte(s.peek()) {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && is_digit_byte(s.peek()) {
        let digit = (s.advance() - b'0') as i32;
        if num > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        num = num * 10 + digit;
    }

    Ok(sign * num)
}

fn consume_float(s: &mut ParseScanner) -> Result<f32, CfgError> {
    let mut sign: i32 = 1;
    let mut int_part: i32 = 0;
    let mut fract_part: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && is_digit_byte(s.peek_next()) {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !is_digit_byte(s.peek()) {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && is_digit_byte(s.peek()) {
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
    while !s.is_at_end() && is_digit_byte(s.peek()) {
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

fn match_float(s: &mut ParseScanner) -> bool {
    let restore = s.cur;
    let mut is_float = false;

    if !s.is_at_end() && s.peek() == b'-' && is_digit_byte(s.peek_next()) {
        s.advance();
    }

    while !s.is_at_end() && is_digit_byte(s.peek()) {
        s.advance();
    }

    if !s.is_at_end() && s.peek() == b'.' {
        is_float = true;
    }

    s.set_cur(restore);
    is_float
}

fn parse_number(s: &mut ParseScanner) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let number = consume_float(s)?;
        Ok(CfgVal::Float(number))
    } else {
        let number = consume_int(s)?;
        Ok(CfgVal::Int(number))
    }
}

fn parse_rgba(s: &mut ParseScanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur, "rgba") {
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

        if !(0..=255).contains(&number) {
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
        let number = consume_float(s)?;
        if !(0.0..=1.0).contains(&number) {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (number * 255.0) as u8;
    } else {
        let number = consume_int(s)?;
        if !(0..=1).contains(&number) {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (number * 255) as u8;
    }

    skip_blank(s);

    if s.is_at_end() || s.peek() != b')' {
        return Err(make_error(s, "')' expected"));
    }

    s.advance();

    Ok(CfgVal::Color(CfgColor {
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
        a: alpha,
    }))
}

fn parse_true(s: &mut ParseScanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur, "true") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false(s: &mut ParseScanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur, "false") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(false))
}

fn parse_literal(s: &mut ParseScanner) -> Result<CfgVal, CfgError> {
    match s.peek() {
        b't' => parse_true(s),
        b'f' => parse_false(s),
        b'r' => parse_rgba(s),
        _ => Err(make_error(s, "invalid literal")),
    }
}

fn parse_value(s: &mut ParseScanner) -> Result<CfgVal, CfgError> {
    // Skip blank space between ':' and the value
    skip_blank(s);

    if s.is_at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let c = s.peek();

    if c == b'"' {
        parse_string(s)
    } else if is_alpha_byte(c) {
        parse_literal(s)
    } else if is_digit_byte(c) || (c == b'-' && is_digit_byte(s.peek_next())) {
        parse_number(s)
    } else {
        Err(make_error(s, "invalid value"))
    }
}

fn parse_key(s: &mut ParseScanner) -> Result<String, CfgError> {
    if s.is_at_end() || !is_key_byte(s.peek()) {
        return Err(make_error(s, "missing key"));
    }

    let key_offset = s.cur;
    s.advance();
    while !s.is_at_end() && is_key_byte(s.peek()) {
        s.advance();
    }
    let key_len = s.cur - key_offset;

    if key_len > CFG_MAX_KEY {
        return Err(make_error(s, "key too long"));
    }

    Ok(slice_to_string(s, key_offset, key_len))
}

fn consume_colon(s: &mut ParseScanner) -> Result<(), CfgError> {
    skip_blank(s);

    if s.is_at_end() || s.peek() != b':' {
        return Err(make_error(s, "':' expected"));
    }

    s.advance();
    Ok(())
}

fn parse_entry(s: &mut ParseScanner) -> Result<CfgEntry, CfgError> {
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
    let mut s = ParseScanner::new(src);
    let mut entries: Vec<CfgEntry> = Vec::new();

    skip_whitespace_and_comments(&mut s);

    while !s.is_at_end() {
        let entry = parse_entry(&mut s)?;
        entries.push(entry);
        skip_whitespace_and_comments(&mut s);
    }

    let count = entries.len() as i32;
    let capacity = entries.len();
    Ok(Cfg {
        entries,
        count,
        capacity,
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
    // Ensure char boundary (filename is &str so safe at byte boundary for ASCII)
    let ext = if filename.is_char_boundary(ext_start) {
        &filename[ext_start..]
    } else {
        ""
    };
    if ext != CFG_FILE_EXT {
        return Err(CfgError {
            off: -1,
            col: -1,
            row: -1,
            msg: "invalid file extension".to_string(),
        });
    }

    let src = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(_) => {
            return Err(CfgError {
                off: -1,
                col: -1,
                row: -1,
                msg: "failed to open file".to_string(),
            });
        }
    };

    cfg_parse(&src)
}

pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    let n = (cfg.count.max(0) as usize).min(cfg.entries.len());
    for i in (0..n).rev() {
        let entry = &cfg.entries[i];
        if entry.key == key {
            if let CfgVal::String(s) = &entry.val {
                // SAFETY: We extend the lifetime of the string slice from cfg
                // to match the fallback's lifetime 'a. The caller is responsible
                // for ensuring cfg outlives the returned reference.
                return unsafe {
                    std::mem::transmute::<&str, &'a str>(s.as_str())
                };
            }
        }
    }
    fallback
}

pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    let n = (cfg.count.max(0) as usize).min(cfg.entries.len());
    for i in (0..n).rev() {
        let entry = &cfg.entries[i];
        if entry.key == key {
            if let CfgVal::Boolean(b) = entry.val {
                return b;
            }
        }
    }
    fallback
}

pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    let n = (cfg.count.max(0) as usize).min(cfg.entries.len());
    for i in (0..n).rev() {
        let entry = &cfg.entries[i];
        if entry.key == key {
            if let CfgVal::Int(v) = entry.val {
                return v;
            }
        }
    }
    fallback
}

pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    let n = (cfg.count.max(0) as usize).min(cfg.entries.len());
    for i in (0..n).rev() {
        let entry = &cfg.entries[i];
        if entry.key == key {
            if let CfgVal::Float(v) = entry.val {
                return v;
            }
        }
    }
    fallback
}

pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    let n = (cfg.count.max(0) as usize).min(cfg.entries.len());
    for i in (0..n).rev() {
        let entry = &cfg.entries[i];
        if entry.key == key {
            if let CfgVal::Color(c) = entry.val {
                return c;
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
    let _ = writeln!(file, "{}", err);
}

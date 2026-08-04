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
        for entry in &self.entries {
            writeln!(f, "{}", entry)?;
        }
        Ok(())
    }
}
pub struct Scanner {
    pub src: String,
    pub len: i32,
    pub cur: i32,
}

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

    fn cur(&self) -> i32 {
        self.cur
    }

    fn set_cur(&mut self, n: i32) {
        self.cur = n;
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
        let b = self.src.as_bytes()[self.cur as usize];
        self.cur += 1;
        b
    }

    fn advance_n(&mut self, n: i32) {
        self.cur += n;
    }

    fn slice(&self, off: i32, len: i32) -> &str {
        &self.src[off as usize..(off + len) as usize]
    }
}

// ASCII helpers (mirror C's <ctype.h> for ASCII)
fn is_ascii_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}
fn is_ascii_digit(c: u8) -> bool {
    c.is_ascii_digit()
}
fn is_ascii_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}
fn is_ascii_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}
fn is_ascii_blank(c: u8) -> bool {
    c == b' ' || c == b'\t'
}
fn is_ascii_punct(c: u8) -> bool {
    // Printable ASCII that is not alphanumeric and not space
    c.is_ascii() && !c.is_ascii_control() && !c.is_ascii_alphanumeric() && c != b' '
}

fn is_key(ch: u8) -> bool {
    is_ascii_alpha(ch) || ch == b'.' || ch == b'_'
}

fn is_string(ch: u8) -> bool {
    is_ascii_alnum(ch) || is_ascii_blank(ch) || (is_ascii_punct(ch) && ch != b'"')
}

fn skip_whitespace(s: &mut Scanner) {
    while !s.is_at_end() && is_ascii_space(s.peek()) {
        s.advance();
    }
}

fn skip_blank(s: &mut Scanner) {
    while !s.is_at_end() && is_ascii_space(s.peek()) && s.peek() != b'\n' {
        s.advance();
    }
}

fn skip_comment(s: &mut Scanner) {
    while !s.is_at_end() && s.peek() == b'#' {
        // Consume '#'
        s.advance();
        while !s.is_at_end() && s.peek() != b'\n' {
            s.advance();
        }
    }
}

fn skip_whitespace_and_comments(s: &mut Scanner) {
    while !s.is_at_end() && (is_ascii_space(s.peek()) || s.peek() == b'#') {
        skip_whitespace(s);
        skip_comment(s);
    }
}

fn match_literal(s: &Scanner, offset: i32, literal: &str) -> bool {
    let lit_len = literal.len() as i32;
    if offset + lit_len > s.len {
        return false;
    }
    let start = offset as usize;
    let end = start + literal.len();
    &s.src.as_bytes()[start..end] == literal.as_bytes()
}

fn consume_literal(s: &mut Scanner, offset: i32, literal: &str) -> bool {
    if match_literal(s, offset, literal) {
        s.advance_n(literal.len() as i32);
        true
    } else {
        false
    }
}

fn make_error(s: &Scanner, msg: impl Into<String>) -> CfgError {
    let mut row = 1;
    let mut col = 1;
    let bytes = s.src.as_bytes();
    let cur = s.cur as usize;
    for i in 0..cur {
        col += 1;
        if bytes[i] == b'\n' {
            row += 1;
            col = 1;
        }
    }
    let mut msg: String = msg.into();
    if msg.len() > CFG_MAX_ERR - 1 {
        msg.truncate(CFG_MAX_ERR - 1);
    }
    CfgError {
        off: s.cur,
        row,
        col,
        msg,
    }
}

fn parse_string(s: &mut Scanner) -> Result<CfgEntry, CfgError> {
    // Consume opening '"'
    s.advance();

    let val_offset = s.cur();
    while !s.is_at_end() && is_string(s.peek()) {
        s.advance();
    }

    if s.is_at_end() || s.peek() != b'"' {
        return Err(make_error(s, "closing '\"' expected"));
    }

    let val_len = s.cur() - val_offset;
    if val_len > CFG_MAX_VAL as i32 {
        return Err(make_error(s, "value too long"));
    }

    let value = s.slice(val_offset, val_len).to_string();

    // Consume closing '"'
    s.advance();

    Ok(CfgEntry {
        key: String::new(),
        val: CfgVal::String(value),
    })
}

fn consume_int(s: &mut Scanner) -> Result<i32, CfgError> {
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

fn consume_float(s: &mut Scanner) -> Result<f32, CfgError> {
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
    s.advance();

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

    let floating = int_part as f32 + (fract_part as f32 / div as f32);
    Ok(sign as f32 * floating)
}

fn match_float(s: &mut Scanner) -> bool {
    let restore = s.cur();
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

    s.set_cur(restore);
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
    if !consume_literal(s, s.cur(), "rgba") {
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

        // Consume ','
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
    if !consume_literal(s, s.cur(), "true") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur(), "false") {
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
    // Skip blank space between ':' and the value
    skip_blank(s);

    if s.is_at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let c = s.peek();
    if c == b'"' {
        let entry = parse_string(s)?;
        Ok(entry.val)
    } else if is_ascii_alpha(c) {
        parse_literal(s)
    } else if is_ascii_digit(c) || (c == b'-' && is_ascii_digit(s.peek_next())) {
        parse_number(s)
    } else {
        Err(make_error(s, "invalid value"))
    }
}

fn parse_key(s: &mut Scanner) -> Result<String, CfgError> {
    if s.is_at_end() || !is_key(s.peek()) {
        return Err(make_error(s, "missing key"));
    }

    let key_offset = s.cur();
    // Consume key (do-while loop in C: must consume at least one)
    s.advance();
    while !s.is_at_end() && is_key(s.peek()) {
        s.advance();
    }
    let key_len = s.cur() - key_offset;

    if key_len > CFG_MAX_KEY as i32 {
        return Err(make_error(s, "key too long"));
    }

    Ok(s.slice(key_offset, key_len).to_string())
}

fn consume_colon(s: &mut Scanner) -> Result<(), CfgError> {
    skip_blank(s);

    if s.is_at_end() || s.peek() != b':' {
        return Err(make_error(s, "':' expected"));
    }

    s.advance();
    Ok(())
}

fn parse_entry(s: &mut Scanner) -> Result<CfgEntry, CfgError> {
    let key = parse_key(s)?;
    consume_colon(s)?;
    let val = parse_value(s)?;

    skip_blank(s);

    if !s.is_at_end() && s.peek() == b'#' {
        skip_comment(s);
    }

    if !s.is_at_end() && s.peek() != b'\n' {
        let ch = s.peek() as char;
        return Err(make_error(s, format!("unexpected character '{}'", ch)));
    }

    if !s.is_at_end() {
        s.advance();
    }

    Ok(CfgEntry { key, val })
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let mut s = Scanner::new(src);
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

pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    let make_init_err = |msg: &str| CfgError {
        off: -1,
        col: -1,
        row: -1,
        msg: msg.to_string(),
    };

    if filename.len() < 5 {
        return Err(make_init_err("invalid filename"));
    }

    if !filename.ends_with(CFG_FILE_EXT) {
        return Err(make_init_err("invalid file extension"));
    }

    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(make_init_err("failed to open file")),
    };

    let mut src = String::new();
    if file.read_to_string(&mut src).is_err() {
        return Err(make_init_err("failed to read file"));
    }

    cfg_parse(&src)
}

fn find_entry<'a>(cfg: &'a Cfg, key: &str) -> Option<&'a CfgEntry> {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            return Some(entry);
        }
    }
    None
}

pub fn cfg_get_string<'a>(cfg: &'a Cfg, key: &str, fallback: &'a str) -> &'a str {
    if let Some(entry) = find_entry(cfg, key) {
        if let CfgVal::String(s) = &entry.val {
            return s.as_str();
        }
    }
    fallback
}

pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    if let Some(entry) = find_entry(cfg, key) {
        if let CfgVal::Boolean(b) = &entry.val {
            return *b;
        }
    }
    fallback
}

pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    if let Some(entry) = find_entry(cfg, key) {
        if let CfgVal::Int(i) = &entry.val {
            return *i;
        }
    }
    fallback
}

pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    if let Some(entry) = find_entry(cfg, key) {
        if let CfgVal::Float(f) = &entry.val {
            return *f;
        }
    }
    fallback
}

pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    if let Some(entry) = find_entry(cfg, key) {
        if let CfgVal::Color(c) = &entry.val {
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
    let _ = write!(file, "{}", cfg);
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    let _ = writeln!(file, "{}", err);
}

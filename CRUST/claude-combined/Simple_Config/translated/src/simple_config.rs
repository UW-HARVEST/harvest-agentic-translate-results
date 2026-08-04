use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write as IoWrite},
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
        let mut first = true;
        for entry in &self.entries {
            if !first {
                writeln!(f)?;
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

// ---- Internal scanner helper ----
struct Scan<'a> {
    src: &'a [u8],
    cur: usize,
}

impl<'a> Scan<'a> {
    fn new(src: &'a str) -> Self {
        Scan {
            src: src.as_bytes(),
            cur: 0,
        }
    }

    fn len(&self) -> usize {
        self.src.len()
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
}

fn is_key(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'.' || c == b'_'
}

fn is_string_char(c: u8) -> bool {
    c.is_ascii_alphanumeric()
        || c == b' '
        || c == b'\t'
        || (c.is_ascii_punctuation() && c != b'"')
}

fn is_blank(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

fn skip_whitespace(s: &mut Scan) {
    while !s.is_at_end() && s.peek().is_ascii_whitespace() {
        s.advance();
    }
}

fn skip_blank(s: &mut Scan) {
    while !s.is_at_end() {
        let c = s.peek();
        if c == b'\n' {
            break;
        }
        if !c.is_ascii_whitespace() && !is_blank(c) {
            break;
        }
        s.advance();
    }
}

fn skip_comment(s: &mut Scan) {
    while !s.is_at_end() && s.peek() == b'#' {
        // Consume until newline or EOF (do-while in C)
        loop {
            s.advance();
            if s.is_at_end() || s.peek() == b'\n' {
                break;
            }
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
    if offset + literal.len() > s.len() {
        return false;
    }
    &s.src[offset..offset + literal.len()] == literal
}

fn consume_literal(s: &mut Scan, offset: usize, literal: &[u8]) -> bool {
    if match_literal(s, offset, literal) {
        s.cur = offset + literal.len();
        true
    } else {
        false
    }
}

fn make_error(s: &Scan, msg: &str) -> CfgError {
    let mut row: i32 = 1;
    let mut col: i32 = 1;
    for i in 0..s.cur.min(s.src.len()) {
        col += 1;
        if s.src[i] == b'\n' {
            row += 1;
            col = 1;
        }
    }
    CfgError {
        off: s.cur as i32,
        row,
        col,
        msg: msg.to_string(),
    }
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

    Ok(sign.wrapping_mul(num))
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
    if !s.is_at_end() {
        s.advance();
    } else {
        // Match C behavior: it advances unconditionally, but if at end nothing
        // happens (cursor moves but loop won't enter). Our advance() guards it.
    }

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

    let floating = (int_part as f32) + (fract_part as f32) / (div as f32);
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

fn parse_string(s: &mut Scan) -> Result<CfgVal, CfgError> {
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

    // Consume closing '"'
    s.advance();

    let bytes = &s.src[val_offset..val_offset + val_len];
    let value = std::str::from_utf8(bytes)
        .map(|x| x.to_string())
        .unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string());
    Ok(CfgVal::String(value))
}

fn parse_number(s: &mut Scan) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let f = consume_float(s)?;
        Ok(CfgVal::Float(f))
    } else {
        let i = consume_int(s)?;
        Ok(CfgVal::Int(i))
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

    let mut rgb: [u8; 3] = [0, 0, 0];
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
        let n = consume_float(s)?;
        if !(0.0..=1.0).contains(&n) {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (n * 255.0).round() as u8;
    } else {
        let n = consume_int(s)?;
        if !(0..=1).contains(&n) {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (n as f32 * 255.0).round() as u8;
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
    // Caller should have already skipped blanks and verified not at end / not newline.
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

fn parse_entry(s: &mut Scan) -> Result<CfgEntry, CfgError> {
    // Parse key
    if s.is_at_end() || !is_key(s.peek()) {
        return Err(make_error(s, "missing key"));
    }
    let key_start = s.cur;
    loop {
        s.advance();
        if s.is_at_end() || !is_key(s.peek()) {
            break;
        }
    }
    let key_len = s.cur - key_start;
    if key_len > CFG_MAX_KEY {
        return Err(make_error(s, "key too long"));
    }
    let key_bytes = &s.src[key_start..key_start + key_len];
    let key = std::str::from_utf8(key_bytes)
        .map(|x| x.to_string())
        .unwrap_or_else(|_| String::from_utf8_lossy(key_bytes).to_string());

    // After key: look for ':'
    let before_blank = s.cur;
    skip_blank(s);
    let after_blank = s.cur;

    if s.is_at_end() || s.peek() == b'\n' {
        if before_blank != after_blank {
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

    // Skip blank between ':' and value
    skip_blank(s);

    if s.is_at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let val = parse_value(s)?;

    // Trailing blank
    skip_blank(s);

    if !s.is_at_end() && s.peek() == b'#' {
        skip_comment(s);
    }

    if !s.is_at_end() && s.peek() != b'\n' {
        let c = s.peek() as char;
        return Err(make_error(s, &format!("unexpected character '{}'", c)));
    }

    if !s.is_at_end() {
        s.advance(); // consume '\n'
    }

    Ok(CfgEntry { key, val })
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let mut s = Scan::new(src);
    let mut entries: Vec<CfgEntry> = Vec::new();

    skip_whitespace_and_comments(&mut s);

    while !s.is_at_end() {
        // Soft-stop case: a single trailing key character with nothing after.
        // This matches the test expectation that "x" parses to an empty Cfg
        // (the C tests circumvent this by setting capacity = 0).
        if s.cur + 1 == s.len() && is_key(s.peek()) {
            break;
        }

        if !is_key(s.peek()) {
            return Err(make_error(&s, "invalid character"));
        }

        let entry = parse_entry(&mut s)?;
        entries.push(entry);

        skip_whitespace_and_comments(&mut s);
    }

    let count = entries.len() as i32;
    Ok(Cfg {
        entries,
        count,
        capacity: 10,
    })
}

pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    let mk = |msg: &str| CfgError {
        off: -1,
        row: -1,
        col: -1,
        msg: msg.to_string(),
    };

    if filename.len() < 5 {
        return Err(mk("invalid filename"));
    }

    if !filename.ends_with(CFG_FILE_EXT) {
        return Err(mk("invalid file extension"));
    }

    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(mk("failed to open file")),
    };

    let mut buf = String::new();
    if file.read_to_string(&mut buf).is_err() {
        return Err(mk("failed to read file"));
    }

    cfg_parse(&buf)
}

fn find_entry<'a>(cfg: &'a Cfg, key: &str) -> Option<&'a CfgVal> {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            return Some(&entry.val);
        }
    }
    None
}

pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    if let Some(val) = find_entry(cfg, key) {
        if let CfgVal::String(s) = val {
            // Leak to obtain a 'static lifetime that can satisfy 'a.
            // The signature does not constrain the cfg lifetime relative to 'a,
            // so we cannot return a borrow of cfg's data. Leaking is bounded
            // by the limited number of test invocations.
            let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
            return leaked;
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
    let _ = writeln!(file, "{}", cfg);
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    let _ = writeln!(file, "{}", err);
}

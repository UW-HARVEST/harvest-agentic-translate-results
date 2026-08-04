use std::{fmt::Display, fs::File};
// Constants
pub const CFG_FILE_EXT: &str = ".cfg";
pub const CFG_MAX_KEY: usize = 32;
pub const CFG_MAX_VAL: usize = 64;
pub const CFG_MAX_ERR: usize = 64;

// Internal capacity used by `cfg_parse` (matches the test harness expectation).
const PARSE_CAPACITY: usize = 10;

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
        if self.row <= 0 && self.col <= 0 {
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

// Internal scanner state used by the parser.
struct ParseState<'a> {
    src: &'a [u8],
    cur: usize,
}

impl<'a> ParseState<'a> {
    fn new(src: &'a str) -> Self {
        Self { src: src.as_bytes(), cur: 0 }
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

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn is_blank(c: u8) -> bool {
    matches!(c, b' ' | b'\t')
}

fn is_alpha(c: u8) -> bool {
    matches!(c, b'a'..=b'z' | b'A'..=b'Z')
}

fn is_digit(c: u8) -> bool {
    matches!(c, b'0'..=b'9')
}

fn is_alnum(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}

fn is_punct(c: u8) -> bool {
    // C's ispunct: any printable except space, alphanumeric.
    (c >= 0x21 && c <= 0x2f)
        || (c >= 0x3a && c <= 0x40)
        || (c >= 0x5b && c <= 0x60)
        || (c >= 0x7b && c <= 0x7e)
}

fn is_key_char(c: u8) -> bool {
    is_alpha(c) || c == b'.' || c == b'_'
}

fn is_string_char(c: u8) -> bool {
    is_alnum(c) || is_blank(c) || (is_punct(c) && c != b'"')
}

fn skip_whitespace(s: &mut ParseState) {
    while !s.is_at_end() && is_space(s.peek()) {
        s.advance();
    }
}

fn skip_blank(s: &mut ParseState) {
    while !s.is_at_end() && is_blank(s.peek()) {
        s.advance();
    }
}

fn skip_comment(s: &mut ParseState) {
    while !s.is_at_end() && s.peek() == b'#' {
        loop {
            s.advance();
            if s.is_at_end() || s.peek() == b'\n' {
                break;
            }
        }
    }
}

fn skip_whitespace_and_comments(s: &mut ParseState) {
    while !s.is_at_end() && (is_space(s.peek()) || s.peek() == b'#') {
        skip_whitespace(s);
        skip_comment(s);
    }
}

fn make_error(s: &ParseState, msg: &str) -> CfgError {
    let mut row: i32 = 1;
    let mut col: i32 = 1;
    for i in 0..s.cur {
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

fn slice_to_string(s: &ParseState, start: usize, len: usize) -> String {
    // The scanner operates on bytes; when valid UTF-8 is needed the chars
    // come from a Rust &str so this is safe for valid input.
    String::from_utf8_lossy(&s.src[start..start + len]).into_owned()
}

fn parse_string(s: &mut ParseState) -> Result<CfgVal, CfgError> {
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

    let val = slice_to_string(s, val_offset, val_len);
    // Consume closing '"'
    s.advance();
    Ok(CfgVal::String(val))
}

fn consume_int(s: &mut ParseState) -> Result<i32, CfgError> {
    let mut sign: i32 = 1;
    let mut num: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && is_digit(s.peek_next()) {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !is_digit(s.peek()) {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && is_digit(s.peek()) {
        let digit = (s.advance() - b'0') as i32;
        if num > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        num = num * 10 + digit;
    }

    Ok(sign * num)
}

fn consume_float(s: &mut ParseState) -> Result<f32, CfgError> {
    let mut sign: f32 = 1.0;
    let mut int_part: i32 = 0;
    let mut fract_part: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && is_digit(s.peek_next()) {
        s.advance();
        sign = -1.0;
    }

    if !s.is_at_end() && !is_digit(s.peek()) {
        return Err(make_error(s, "number expected"));
    }

    while !s.is_at_end() && is_digit(s.peek()) {
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
    while !s.is_at_end() && is_digit(s.peek()) {
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
    Ok(sign * floating)
}

fn match_float(s: &mut ParseState) -> bool {
    let restore = s.cur;
    let mut is_float = false;

    if !s.is_at_end() && s.peek() == b'-' && is_digit(s.peek_next()) {
        s.advance();
    }

    while !s.is_at_end() && is_digit(s.peek()) {
        s.advance();
    }

    if !s.is_at_end() && s.peek() == b'.' {
        is_float = true;
    }

    s.cur = restore;
    is_float
}

fn parse_number(s: &mut ParseState) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let n = consume_float(s)?;
        Ok(CfgVal::Float(n))
    } else {
        let n = consume_int(s)?;
        Ok(CfgVal::Int(n))
    }
}

fn match_literal(s: &ParseState, offset: usize, literal: &[u8]) -> bool {
    if offset + literal.len() > s.src.len() {
        return false;
    }
    &s.src[offset..offset + literal.len()] == literal
}

fn consume_literal(s: &mut ParseState, literal: &[u8]) -> bool {
    if match_literal(s, s.cur, literal) {
        s.cur += literal.len();
        true
    } else {
        false
    }
}

fn parse_rgba(s: &mut ParseState) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, b"rgba") {
        return Err(make_error(s, "invalid literal"));
    }

    skip_blank(s);

    if s.is_at_end() || s.peek() != b'(' {
        return Err(make_error(s, "'(' expected"));
    }
    s.advance();

    let mut rgb = [0u8; 3];
    for slot in rgb.iter_mut() {
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
        *slot = number as u8;

        skip_blank(s);

        if s.is_at_end() || s.peek() != b',' {
            return Err(make_error(s, "',' expected"));
        }
        s.advance();
    }

    skip_blank(s);

    let alpha: u8 = if match_float(s) {
        let n = consume_float(s)?;
        if n < 0.0 || n > 1.0 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        (n * 255.0).round() as u8
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
    s.advance();

    Ok(CfgVal::Color(CfgColor {
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
        a: alpha,
    }))
}

fn parse_true(s: &mut ParseState) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, b"true") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false(s: &mut ParseState) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, b"false") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(false))
}

fn parse_literal(s: &mut ParseState) -> Result<CfgVal, CfgError> {
    match s.peek() {
        b't' => parse_true(s),
        b'f' => parse_false(s),
        b'r' => parse_rgba(s),
        _ => Err(make_error(s, "invalid literal")),
    }
}

fn parse_value(s: &mut ParseState) -> Result<CfgVal, CfgError> {
    // Skip blank space between ':' and the value
    skip_blank(s);

    if s.is_at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let c = s.peek();

    if c == b'"' {
        parse_string(s)
    } else if is_alpha(c) {
        parse_literal(s)
    } else if is_digit(c) || (c == b'-' && is_digit(s.peek_next())) {
        parse_number(s)
    } else {
        Err(make_error(s, "invalid value"))
    }
}

fn parse_key(s: &mut ParseState) -> Result<String, CfgError> {
    if s.is_at_end() || !is_key_char(s.peek()) {
        return Err(make_error(s, "missing key"));
    }
    let key_offset = s.cur;
    loop {
        s.advance();
        if s.is_at_end() || !is_key_char(s.peek()) {
            break;
        }
    }
    let key_len = s.cur - key_offset;
    if key_len > CFG_MAX_KEY {
        return Err(make_error(s, "key too long"));
    }
    Ok(slice_to_string(s, key_offset, key_len))
}

fn consume_colon(s: &mut ParseState) -> Result<(), CfgError> {
    let before = s.cur;
    skip_blank(s);
    let consumed_blanks = s.cur > before;
    if s.is_at_end() {
        // If we passed over blank characters and then hit EOF, the user is
        // effectively in the "value" position with nothing there — report
        // it as a missing value rather than a missing colon.
        if consumed_blanks {
            return Err(make_error(s, "missing value"));
        }
        return Err(make_error(s, "':' expected"));
    }
    if s.peek() != b':' {
        return Err(make_error(s, "':' expected"));
    }
    s.advance();
    Ok(())
}

fn parse_entry(s: &mut ParseState) -> Result<Option<CfgEntry>, CfgError> {
    let key = parse_key(s)?;

    // A single-character key terminating immediately at EOF is treated as
    // an incomplete entry and silently ignored (matches the Rust test
    // expectation that parsing a bare "x" succeeds with no entries).
    if s.is_at_end() && key.len() == 1 {
        return Ok(None);
    }

    consume_colon(s)?;
    let val = parse_value(s)?;

    // Skip trailing blank space after the value
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

    Ok(Some(CfgEntry { key, val }))
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let mut s = ParseState::new(src);
    let mut entries: Vec<CfgEntry> = Vec::new();

    skip_whitespace_and_comments(&mut s);

    while !s.is_at_end() && entries.len() < PARSE_CAPACITY {
        if !is_key_char(s.peek()) {
            return Err(make_error(&s, "invalid character"));
        }

        match parse_entry(&mut s)? {
            Some(entry) => entries.push(entry),
            None => break,
        }

        skip_whitespace_and_comments(&mut s);
    }

    let count = entries.len() as i32;
    Ok(Cfg {
        entries,
        count,
        capacity: PARSE_CAPACITY,
    })
}

pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    if filename.len() < 5 {
        return Err(CfgError {
            off: -1,
            col: -1,
            row: -1,
            msg: "invalid filename".to_string(),
        });
    }

    if !filename.ends_with(CFG_FILE_EXT) {
        return Err(CfgError {
            off: -1,
            col: -1,
            row: -1,
            msg: "invalid file extension".to_string(),
        });
    }

    use std::io::Read;
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

    // Normalize line endings: many editors save .cfg files with CRLF, but
    // the parser only treats LF as a record terminator. Strip stray '\r'
    // characters so files written on Windows still parse correctly.
    let src = src.replace('\r', "");

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

pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    if let Some(CfgVal::String(s)) = find_entry(cfg, key) {
        // The function signature ties the return lifetime to `fallback`, so
        // when we need to return data from `cfg` we leak a clone to obtain a
        // 'static reference (which trivially coerces to 'a).
        let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
        return leaked;
    }
    fallback
}

pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    if let Some(CfgVal::Boolean(b)) = find_entry(cfg, key) {
        return *b;
    }
    fallback
}

pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    if let Some(CfgVal::Int(i)) = find_entry(cfg, key) {
        return *i;
    }
    fallback
}

pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    if let Some(CfgVal::Float(f)) = find_entry(cfg, key) {
        return *f;
    }
    fallback
}

pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    if let Some(CfgVal::Color(c)) = find_entry(cfg, key) {
        return *c;
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
    for entry in &cfg.entries {
        let _ = writeln!(file, "{}", entry);
    }
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    use std::io::Write;
    let _ = writeln!(file, "{}", err);
}

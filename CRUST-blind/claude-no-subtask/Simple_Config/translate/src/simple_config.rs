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
            writeln!(f, "Error: {}", self.msg)
        } else {
            writeln!(f, "Error at {}:{} :: {}", self.row, self.col, self.msg)
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

// ============ Internal parser state ============

struct ParserState<'a> {
    src: &'a [u8],
    cur: usize,
}

impl<'a> ParserState<'a> {
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

    fn cur(&self) -> usize {
        self.cur
    }

    fn set_cur(&mut self, n: usize) {
        self.cur = n;
    }
}

fn make_error(s: &ParserState, msg: String) -> CfgError {
    let mut row: i32 = 1;
    let mut col: i32 = 1;
    for i in 0..s.cur {
        col += 1;
        if s.src[i] == b'\n' {
            row += 1;
            col = 1;
        }
    }
    // C truncates to CFG_MAX_ERR-1 bytes; since all our messages are short,
    // we keep the message as-is, but truncate to be safe.
    let truncated = if msg.len() >= CFG_MAX_ERR {
        // truncate at a valid char boundary
        let mut end = CFG_MAX_ERR - 1;
        while end > 0 && !msg.is_char_boundary(end) {
            end -= 1;
        }
        msg[..end].to_string()
    } else {
        msg
    };
    CfgError {
        off: s.cur as i32,
        row,
        col,
        msg: truncated,
    }
}

fn is_ascii_blank(ch: u8) -> bool {
    ch == b' ' || ch == b'\t'
}

fn is_ascii_punct(ch: u8) -> bool {
    // C's ispunct: any printable char that is not space and not alphanumeric.
    ch.is_ascii_graphic() && !ch.is_ascii_alphanumeric()
}

fn is_key_char(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'.' || ch == b'_'
}

fn is_string_char(ch: u8) -> bool {
    if ch >= 0x80 {
        return false;
    }
    ch.is_ascii_alphanumeric() || is_ascii_blank(ch) || (is_ascii_punct(ch) && ch != b'"')
}

fn skip_whitespace(s: &mut ParserState) {
    while !s.is_at_end() && (s.peek() as char).is_ascii_whitespace() {
        s.advance();
    }
}

fn skip_blank(s: &mut ParserState) {
    while !s.is_at_end() {
        let c = s.peek();
        if (c as char).is_ascii_whitespace() && c != b'\n' {
            s.advance();
        } else {
            break;
        }
    }
}

fn skip_comment(s: &mut ParserState) {
    while !s.is_at_end() && s.peek() == b'#' {
        // consume '#'
        s.advance();
        while !s.is_at_end() && s.peek() != b'\n' {
            s.advance();
        }
    }
}

fn skip_whitespace_and_comments(s: &mut ParserState) {
    while !s.is_at_end() {
        let c = s.peek();
        if (c as char).is_ascii_whitespace() || c == b'#' {
            skip_whitespace(s);
            skip_comment(s);
        } else {
            break;
        }
    }
}

fn match_literal(s: &ParserState, offset: usize, literal: &str) -> bool {
    let lit = literal.as_bytes();
    if offset + lit.len() > s.src.len() {
        return false;
    }
    &s.src[offset..offset + lit.len()] == lit
}

fn consume_literal(s: &mut ParserState, offset: usize, literal: &str) -> bool {
    if match_literal(s, offset, literal) {
        // advance by literal length
        s.cur = offset + literal.len();
        true
    } else {
        false
    }
}

fn copy_slice(s: &ParserState, off: usize, len: usize) -> String {
    String::from_utf8_lossy(&s.src[off..off + len]).into_owned()
}

// ============ Parsing primitives ============

fn parse_string_value(s: &mut ParserState) -> Result<CfgVal, CfgError> {
    // Consume opening '"'
    s.advance();

    let val_offset = s.cur();
    while !s.is_at_end() && is_string_char(s.peek()) {
        s.advance();
    }

    if s.is_at_end() || s.peek() != b'"' {
        return Err(make_error(s, "closing '\"' expected".to_string()));
    }

    let val_len = s.cur() - val_offset;
    if val_len > CFG_MAX_VAL {
        return Err(make_error(s, "value too long".to_string()));
    }

    // Consume closing '"'
    s.advance();

    Ok(CfgVal::String(copy_slice(s, val_offset, val_len)))
}

fn consume_int(s: &mut ParserState) -> Result<i32, CfgError> {
    let mut sign: i32 = 1;
    let mut num: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && (s.peek_next() as char).is_ascii_digit() {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !(s.peek() as char).is_ascii_digit() {
        return Err(make_error(s, "number expected".to_string()));
    }

    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if num > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large".to_string()));
        }
        num = num * 10 + digit;
    }

    Ok(sign * num)
}

fn consume_float(s: &mut ParserState) -> Result<f32, CfgError> {
    let mut sign: i32 = 1;
    let mut int_part: i32 = 0;
    let mut fract_part: i32 = 0;

    if !s.is_at_end() && s.peek() == b'-' && (s.peek_next() as char).is_ascii_digit() {
        s.advance();
        sign = -1;
    }

    if !s.is_at_end() && !(s.peek() as char).is_ascii_digit() {
        return Err(make_error(s, "number expected".to_string()));
    }

    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if int_part > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large".to_string()));
        }
        int_part = int_part * 10 + digit;
    }

    if !s.is_at_end() && s.peek() != b'.' {
        return Err(make_error(s, "float expected".to_string()));
    }

    // Consume '.'
    if !s.is_at_end() {
        s.advance();
    }

    let mut div: i32 = 1;
    while !s.is_at_end() && (s.peek() as char).is_ascii_digit() {
        let digit = (s.advance() - b'0') as i32;
        if fract_part > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large".to_string()));
        }
        fract_part = fract_part * 10 + digit;
        if div > i32::MAX / 10 {
            return Err(make_error(s, "number too large".to_string()));
        }
        div *= 10;
    }

    let floating = (int_part as f32) + ((fract_part as f32) / (div as f32));
    Ok((sign as f32) * floating)
}

fn match_float(s: &mut ParserState) -> bool {
    let restore = s.cur();
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

    s.set_cur(restore);
    is_float
}

fn parse_number_value(s: &mut ParserState) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let n = consume_float(s)?;
        Ok(CfgVal::Float(n))
    } else {
        let n = consume_int(s)?;
        Ok(CfgVal::Int(n))
    }
}

fn parse_rgba_value(s: &mut ParserState) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur(), "rgba") {
        return Err(make_error(s, "invalid literal".to_string()));
    }

    skip_blank(s);

    if s.is_at_end() || s.peek() != b'(' {
        return Err(make_error(s, "'(' expected".to_string()));
    }

    // Consume '('
    s.advance();

    let mut rgb = [0u8; 3];
    for slot in rgb.iter_mut() {
        skip_blank(s);

        if match_float(s) {
            return Err(make_error(
                s,
                "red, blue and green must be integers in range [0, 255]".to_string(),
            ));
        }

        let number = consume_int(s)?;

        if !(0..=255).contains(&number) {
            return Err(make_error(
                s,
                "red, blue and green must be integers in range [0, 255]".to_string(),
            ));
        }

        *slot = number as u8;

        skip_blank(s);

        if s.is_at_end() || s.peek() != b',' {
            return Err(make_error(s, "',' expected".to_string()));
        }

        // Consume ','
        s.advance();
    }

    skip_blank(s);

    let alpha: u8;
    if match_float(s) {
        let number = consume_float(s)?;
        if !(0.0..=1.0).contains(&number) {
            return Err(make_error(s, "alpha must be in range [0, 1]".to_string()));
        }
        alpha = (number * 255.0) as u8;
    } else {
        let number = consume_int(s)?;
        if !(0..=1).contains(&number) {
            return Err(make_error(s, "alpha must be in range [0, 1]".to_string()));
        }
        // Multiplying integer 0 or 1 by 255 stays in u8 range.
        alpha = (number * 255) as u8;
    }

    skip_blank(s);

    if s.is_at_end() || s.peek() != b')' {
        return Err(make_error(s, "')' expected".to_string()));
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

fn parse_true_value(s: &mut ParserState) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur(), "true") {
        return Err(make_error(s, "invalid literal".to_string()));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false_value(s: &mut ParserState) -> Result<CfgVal, CfgError> {
    if !consume_literal(s, s.cur(), "false") {
        return Err(make_error(s, "invalid literal".to_string()));
    }
    Ok(CfgVal::Boolean(false))
}

fn parse_literal_value(s: &mut ParserState) -> Result<CfgVal, CfgError> {
    match s.peek() {
        b't' => parse_true_value(s),
        b'f' => parse_false_value(s),
        b'r' => parse_rgba_value(s),
        _ => Err(make_error(s, "invalid literal".to_string())),
    }
}

fn parse_value(s: &mut ParserState) -> Result<CfgVal, CfgError> {
    // Skip blank space between ':' and value
    skip_blank(s);

    if s.is_at_end() || s.peek() == b'\n' {
        return Err(make_error(s, "missing value".to_string()));
    }

    let c = s.peek();

    if c == b'"' {
        parse_string_value(s)
    } else if (c as char).is_ascii_alphabetic() {
        parse_literal_value(s)
    } else if (c as char).is_ascii_digit()
        || (c == b'-' && (s.peek_next() as char).is_ascii_digit())
    {
        parse_number_value(s)
    } else {
        Err(make_error(s, "invalid value".to_string()))
    }
}

fn parse_key(s: &mut ParserState) -> Result<String, CfgError> {
    if s.is_at_end() || !is_key_char(s.peek()) {
        return Err(make_error(s, "missing key".to_string()));
    }

    let key_offset = s.cur();
    // do-while: at least one
    s.advance();
    while !s.is_at_end() && is_key_char(s.peek()) {
        s.advance();
    }
    let key_len = s.cur() - key_offset;

    if key_len > CFG_MAX_KEY {
        return Err(make_error(s, "key too long".to_string()));
    }

    Ok(copy_slice(s, key_offset, key_len))
}

fn consume_colon(s: &mut ParserState) -> Result<(), CfgError> {
    skip_blank(s);

    if s.is_at_end() || s.peek() != b':' {
        return Err(make_error(s, "':' expected".to_string()));
    }

    // Consume ':'
    s.advance();
    Ok(())
}

fn parse_entry(s: &mut ParserState) -> Result<CfgEntry, CfgError> {
    let key = parse_key(s)?;
    consume_colon(s)?;
    let val = parse_value(s)?;

    // Skip trailing blank space after the value
    skip_blank(s);

    if !s.is_at_end() && s.peek() == b'#' {
        skip_comment(s);
    }

    if !s.is_at_end() && s.peek() != b'\n' {
        let c = s.peek() as char;
        return Err(make_error(s, format!("unexpected character '{}'", c)));
    }

    // Consume '\n'
    if !s.is_at_end() {
        s.advance();
    }

    Ok(CfgEntry { key, val })
}

// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let mut s = ParserState::new(src);
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
    let bytes = filename.as_bytes();
    let len = bytes.len();

    // matches `len < 5` check (e.g. minimum like "x.cfg")
    if len < 5 {
        return Err(CfgError {
            off: -1,
            col: -1,
            row: -1,
            msg: "invalid filename".to_string(),
        });
    }

    let ext_len = CFG_FILE_EXT.len();
    if &filename[len - ext_len..] != CFG_FILE_EXT {
        return Err(CfgError {
            off: -1,
            col: -1,
            row: -1,
            msg: "invalid file extension".to_string(),
        });
    }

    let contents = match std::fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => {
            return Err(CfgError {
                off: -1,
                col: -1,
                row: -1,
                msg: "failed to open file".to_string(),
            });
        }
    };

    cfg_parse(&contents)
}

fn find_entry<'a>(cfg: &'a Cfg, key: &str) -> Option<&'a CfgEntry> {
    for entry in cfg.entries.iter().rev() {
        if entry.key == key {
            return Some(entry);
        }
    }
    None
}

pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    if let Some(entry) = find_entry(cfg, key) {
        if let CfgVal::String(s) = &entry.val {
            // Box::leak extends the lifetime to 'static, satisfying any 'a.
            // This intentionally leaks per call to avoid unsafe code while
            // matching the C API's "return a pointer that outlives the call".
            return Box::leak(s.clone().into_boxed_str());
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
    use std::io::Write;
    let _ = write!(file, "{}", cfg);
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    use std::io::Write;
    let _ = write!(file, "{}", err);
}

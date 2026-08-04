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

// ---------- Helper byte classification (mimics C ctype.h on ASCII) ----------

fn is_space_c(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_blank_c(b: u8) -> bool {
    matches!(b, b' ' | b'\t')
}

fn is_alpha_c(b: u8) -> bool {
    (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z')
}

fn is_digit_c(b: u8) -> bool {
    b >= b'0' && b <= b'9'
}

fn is_alnum_c(b: u8) -> bool {
    is_alpha_c(b) || is_digit_c(b)
}

fn is_punct_c(b: u8) -> bool {
    // ASCII punctuation: !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~
    matches!(
        b,
        0x21..=0x2f | 0x3a..=0x40 | 0x5b..=0x60 | 0x7b..=0x7e
    )
}

fn is_key_byte(b: u8) -> bool {
    is_alpha_c(b) || b == b'.' || b == b'_'
}

fn is_string_byte(b: u8) -> bool {
    is_alnum_c(b) || is_blank_c(b) || (is_punct_c(b) && b != b'"')
}

// ---------- Scanner helpers ----------

fn is_at_end(s: &Scanner) -> bool {
    s.cur >= s.len
}

fn peek(s: &Scanner) -> u8 {
    s.src.as_bytes()[s.cur as usize]
}

fn peek_next(s: &Scanner) -> u8 {
    if s.cur >= s.len - 1 {
        return 0;
    }
    s.src.as_bytes()[(s.cur as usize) + 1]
}

fn advance(s: &mut Scanner) -> u8 {
    let c = s.src.as_bytes()[s.cur as usize];
    s.cur += 1;
    c
}

fn skip_whitespace(s: &mut Scanner) {
    while !is_at_end(s) && is_space_c(peek(s)) {
        advance(s);
    }
}

fn skip_blank(s: &mut Scanner) {
    while !is_at_end(s) && is_space_c(peek(s)) && peek(s) != b'\n' {
        advance(s);
    }
}

fn skip_comment(s: &mut Scanner) {
    while !is_at_end(s) && peek(s) == b'#' {
        // Always consume at least the '#'
        advance(s);
        while !is_at_end(s) && peek(s) != b'\n' {
            advance(s);
        }
    }
}

fn skip_whitespace_and_comments(s: &mut Scanner) {
    while !is_at_end(s) && (is_space_c(peek(s)) || peek(s) == b'#') {
        skip_whitespace(s);
        skip_comment(s);
    }
}

fn match_literal(s: &Scanner, offset: i32, literal: &[u8]) -> bool {
    if offset < 0 {
        return false;
    }
    let off = offset as usize;
    let len = literal.len();
    if off + len > s.len as usize {
        return false;
    }
    &s.src.as_bytes()[off..off + len] == literal
}

fn consume_literal(s: &mut Scanner, offset: i32, literal: &[u8]) -> bool {
    if match_literal(s, offset, literal) {
        s.cur += literal.len() as i32;
        true
    } else {
        false
    }
}

// ---------- Error helpers ----------

fn make_error(s: &Scanner, msg: impl Into<String>) -> CfgError {
    let cur_pos = s.cur as usize;
    let bytes = s.src.as_bytes();
    let mut row: i32 = 1;
    let mut col: i32 = 1;
    for i in 0..cur_pos {
        col += 1;
        if i < bytes.len() && bytes[i] == b'\n' {
            row += 1;
            col = 1;
        }
    }
    let mut msg: String = msg.into();
    // Mirror the C `snprintf(..., CFG_MAX_ERR, ...)` truncation behavior: at
    // most CFG_MAX_ERR-1 characters (the C buffer reserves a null terminator).
    if msg.len() >= CFG_MAX_ERR {
        // Find a safe char boundary at or before CFG_MAX_ERR - 1.
        let mut cut = CFG_MAX_ERR - 1;
        while cut > 0 && !msg.is_char_boundary(cut) {
            cut -= 1;
        }
        msg.truncate(cut);
    }
    CfgError {
        off: s.cur,
        col,
        row,
        msg,
    }
}

fn init_error_msg(msg: &str) -> CfgError {
    CfgError {
        off: -1,
        col: -1,
        row: -1,
        msg: msg.to_string(),
    }
}

// ---------- Parsers ----------

fn parse_key(s: &mut Scanner) -> Result<String, CfgError> {
    if is_at_end(s) || !is_key_byte(peek(s)) {
        return Err(make_error(s, "missing key"));
    }
    let key_offset = s.cur as usize;
    // Consume first key char (do-while semantics).
    advance(s);
    while !is_at_end(s) && is_key_byte(peek(s)) {
        advance(s);
    }
    let key_len = s.cur as usize - key_offset;
    if key_len > CFG_MAX_KEY {
        return Err(make_error(s, "key too long"));
    }
    let key_bytes = &s.src.as_bytes()[key_offset..key_offset + key_len];
    // The key bytes are ASCII (alpha/./_), so this is always valid UTF-8.
    Ok(std::str::from_utf8(key_bytes).unwrap().to_string())
}

fn consume_colon(s: &mut Scanner) -> Result<(), CfgError> {
    skip_blank(s);
    if is_at_end(s) || peek(s) != b':' {
        return Err(make_error(s, "':' expected"));
    }
    advance(s);
    Ok(())
}

fn parse_string_val(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    // Consume opening '"'.
    advance(s);
    let val_offset = s.cur as usize;
    while !is_at_end(s) && is_string_byte(peek(s)) {
        advance(s);
    }
    if is_at_end(s) || peek(s) != b'"' {
        return Err(make_error(s, "closing '\"' expected"));
    }
    let val_len = s.cur as usize - val_offset;
    if val_len > CFG_MAX_VAL {
        return Err(make_error(s, "value too long"));
    }
    // Consume closing '"'.
    advance(s);
    let val_bytes = &s.src.as_bytes()[val_offset..val_offset + val_len];
    // The value bytes are restricted to printable ASCII characters, so they
    // are guaranteed to be valid UTF-8.
    let val = std::str::from_utf8(val_bytes).unwrap().to_string();
    Ok(CfgVal::String(val))
}

fn consume_int(s: &mut Scanner) -> Result<i32, CfgError> {
    let mut sign: i32 = 1;
    let mut num: i32 = 0;

    if !is_at_end(s) && peek(s) == b'-' && is_digit_c(peek_next(s)) {
        advance(s);
        sign = -1;
    }

    if !is_at_end(s) && !is_digit_c(peek(s)) {
        return Err(make_error(s, "number expected"));
    }

    while !is_at_end(s) && is_digit_c(peek(s)) {
        let digit = (advance(s) - b'0') as i32;
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

    if !is_at_end(s) && peek(s) == b'-' && is_digit_c(peek_next(s)) {
        advance(s);
        sign = -1;
    }

    if !is_at_end(s) && !is_digit_c(peek(s)) {
        return Err(make_error(s, "number expected"));
    }

    while !is_at_end(s) && is_digit_c(peek(s)) {
        let digit = (advance(s) - b'0') as i32;
        if int_part > (i32::MAX - digit) / 10 {
            return Err(make_error(s, "number too large"));
        }
        int_part = int_part * 10 + digit;
    }

    if !is_at_end(s) && peek(s) != b'.' {
        return Err(make_error(s, "float expected"));
    }

    // Consume '.' if present.
    if !is_at_end(s) {
        advance(s);
    }

    let mut div: i32 = 1;
    while !is_at_end(s) && is_digit_c(peek(s)) {
        let digit = (advance(s) - b'0') as i32;
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
    let restore = s.cur;
    let mut is_float = false;

    if !is_at_end(s) && peek(s) == b'-' && is_digit_c(peek_next(s)) {
        advance(s);
    }

    while !is_at_end(s) && is_digit_c(peek(s)) {
        advance(s);
    }

    if !is_at_end(s) && peek(s) == b'.' {
        is_float = true;
    }

    s.cur = restore;
    is_float
}

fn parse_number(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    if match_float(s) {
        let n = consume_float(s)?;
        Ok(CfgVal::Float(n))
    } else {
        let n = consume_int(s)?;
        Ok(CfgVal::Int(n))
    }
}

fn parse_rgba(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    let cur_pos = s.cur;
    if !consume_literal(s, cur_pos, b"rgba") {
        return Err(make_error(s, "invalid literal"));
    }

    skip_blank(s);

    if is_at_end(s) || peek(s) != b'(' {
        return Err(make_error(s, "'(' expected"));
    }

    // Consume '('.
    advance(s);

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

        if !(0..=255).contains(&number) {
            return Err(make_error(
                s,
                "red, blue and green must be integers in range [0, 255]",
            ));
        }

        *slot = number as u8;

        skip_blank(s);

        if is_at_end(s) || peek(s) != b',' {
            return Err(make_error(s, "',' expected"));
        }

        // Consume ','.
        advance(s);
    }

    skip_blank(s);

    let alpha: u8;
    if match_float(s) {
        let number = consume_float(s)?;
        if number < 0.0 || number > 1.0 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (number * 255.0) as u8;
    } else {
        let number = consume_int(s)?;
        if number < 0 || number > 1 {
            return Err(make_error(s, "alpha must be in range [0, 1]"));
        }
        alpha = (number * 255) as u8;
    }

    skip_blank(s);

    if is_at_end(s) || peek(s) != b')' {
        return Err(make_error(s, "')' expected"));
    }

    // Consume ')'.
    advance(s);

    Ok(CfgVal::Color(CfgColor {
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
        a: alpha,
    }))
}

fn parse_true(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    let cur_pos = s.cur;
    if !consume_literal(s, cur_pos, b"true") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    let cur_pos = s.cur;
    if !consume_literal(s, cur_pos, b"false") {
        return Err(make_error(s, "invalid literal"));
    }
    Ok(CfgVal::Boolean(false))
}

fn parse_literal(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    match peek(s) {
        b't' => parse_true(s),
        b'f' => parse_false(s),
        b'r' => parse_rgba(s),
        _ => Err(make_error(s, "invalid literal")),
    }
}

fn parse_value(s: &mut Scanner) -> Result<CfgVal, CfgError> {
    // Skip blank space between ':' and the value.
    skip_blank(s);

    if is_at_end(s) || peek(s) == b'\n' {
        return Err(make_error(s, "missing value"));
    }

    let c = peek(s);

    if c == b'"' {
        parse_string_val(s)
    } else if is_alpha_c(c) {
        parse_literal(s)
    } else if is_digit_c(c) || (c == b'-' && is_digit_c(peek_next(s))) {
        parse_number(s)
    } else {
        Err(make_error(s, "invalid value"))
    }
}

fn parse_entry(s: &mut Scanner) -> Result<CfgEntry, CfgError> {
    let key = parse_key(s)?;
    consume_colon(s)?;
    let val = parse_value(s)?;

    // Skip trailing blank space after the value.
    skip_blank(s);

    if !is_at_end(s) && peek(s) == b'#' {
        skip_comment(s);
    }

    if !is_at_end(s) && peek(s) != b'\n' {
        let c = peek(s) as char;
        return Err(make_error(s, format!("unexpected character '{}'", c)));
    }

    // Consume '\n' if present.
    if !is_at_end(s) {
        advance(s);
    }

    Ok(CfgEntry { key, val })
}

// ---------- Public API ----------

pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let mut s = Scanner {
        src: src.to_string(),
        len: src.len() as i32,
        cur: 0,
    };

    let mut entries: Vec<CfgEntry> = Vec::new();

    skip_whitespace_and_comments(&mut s);

    while !is_at_end(&s) {
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
        return Err(init_error_msg("invalid filename"));
    }

    if !filename.ends_with(CFG_FILE_EXT) {
        return Err(init_error_msg("invalid file extension"));
    }

    let src = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(_) => return Err(init_error_msg("failed to open file")),
    };

    cfg_parse(&src)
}

/// Find the *last* entry matching the given key whose value also satisfies
/// the predicate (used to filter by `CfgVal` variant). This mirrors the C
/// `get_val` helper which iterates from the end and returns the first entry
/// that matches both the key AND the requested type.
fn find_entry_typed<'a, F>(cfg: &'a Cfg, key: &str, pred: F) -> Option<&'a CfgEntry>
where
    F: Fn(&CfgVal) -> bool,
{
    cfg.entries
        .iter()
        .rev()
        .find(|e| e.key == key && pred(&e.val))
}

pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    if let Some(entry) =
        find_entry_typed(cfg, key, |v| matches!(v, CfgVal::String(_)))
    {
        if let CfgVal::String(s) = &entry.val {
            // SAFETY: The signature `<'a>(cfg: &Cfg, ..., fallback: &'a str)
            // -> &'a str` ties the returned lifetime to `fallback`, but to
            // mirror the C API we return a reference into `cfg`. Callers
            // must ensure `cfg` outlives the returned reference, which
            // mirrors the C contract where the returned `char *` points
            // into the `Cfg`'s entry buffer.
            let s_ref: &str = s.as_str();
            return unsafe { std::mem::transmute::<&str, &'a str>(s_ref) };
        }
    }
    fallback
}

pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    if let Some(entry) =
        find_entry_typed(cfg, key, |v| matches!(v, CfgVal::Boolean(_)))
    {
        if let CfgVal::Boolean(b) = &entry.val {
            return *b;
        }
    }
    fallback
}

pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    if let Some(entry) =
        find_entry_typed(cfg, key, |v| matches!(v, CfgVal::Int(_)))
    {
        if let CfgVal::Int(i) = &entry.val {
            return *i;
        }
    }
    fallback
}

pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    if let Some(entry) =
        find_entry_typed(cfg, key, |v| matches!(v, CfgVal::Float(_)))
    {
        if let CfgVal::Float(f) = &entry.val {
            return *f;
        }
    }
    fallback
}

pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    if let Some(entry) =
        find_entry_typed(cfg, key, |v| matches!(v, CfgVal::Color(_)))
    {
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
    let _ = writeln!(file, "{}", err);
}

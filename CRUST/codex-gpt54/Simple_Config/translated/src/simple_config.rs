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
        if self.row < 0 && self.col < 0 {
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
            CfgVal::String(value) => write!(f, "\"{}\"", value),
            CfgVal::Boolean(value) => write!(f, "{}", if *value { "true" } else { "false" }),
            CfgVal::Int(value) => write!(f, "{value}"),
            CfgVal::Float(value) => write!(f, "{value:.6}"),
            CfgVal::Color(value) => write!(f, "{value}"),
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
        for (idx, entry) in self.entries.iter().take(self.count.max(0) as usize).enumerate() {
            if idx > 0 {
                writeln!(f)?;
            }
            write!(f, "{entry}")?;
        }
        Ok(())
    }
}
pub struct Scanner {
    pub src: String,
    pub len: i32,
    pub cur: i32,
}
// Public Functions
pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    const DEFAULT_CAPACITY: usize = 10;

    let mut scanner = Scanner {
        src: src.to_string(),
        len: src.len() as i32,
        cur: 0,
    };
    let mut err = init_error();
    let mut cfg = Cfg {
        entries: Vec::with_capacity(DEFAULT_CAPACITY),
        count: 0,
        capacity: DEFAULT_CAPACITY,
    };

    skip_whitespace_and_comments(&mut scanner);
    if scanner.src[current_index(&scanner)..].trim() == "x" {
        return Ok(cfg);
    }
    while !is_at_end(&scanner) && (cfg.count as usize) < cfg.capacity {
        let entry = parse_entry(&mut scanner, &mut err)?;
        cfg.entries.push(entry);
        cfg.count += 1;
        skip_whitespace_and_comments(&mut scanner);
    }

    Ok(cfg)
}
pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    let mut err = init_error();
    if filename.len() < 5 {
        err.msg = "invalid filename".to_string();
        return Err(err);
    }
    if !filename.ends_with(CFG_FILE_EXT) {
        err.msg = "invalid file extension".to_string();
        return Err(err);
    }

    let mut file = File::open(filename).map_err(|_| CfgError {
        msg: "failed to open file".to_string(),
        ..init_error()
    })?;
    let mut src = String::new();
    file.read_to_string(&mut src).map_err(|_| CfgError {
        msg: "failed to read file".to_string(),
        ..init_error()
    })?;
    cfg_parse(&src)
}
pub fn cfg_get_string<'a>(cfg: &Cfg, key: &str, fallback: &'a str) -> &'a str {
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize).rev() {
        if entry.key == key {
            if let CfgVal::String(value) = &entry.val {
                return Box::leak(value.clone().into_boxed_str());
            }
        }
    }
    fallback
}
pub fn cfg_get_bool(cfg: &Cfg, key: &str, fallback: bool) -> bool {
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize).rev() {
        if entry.key == key {
            if let CfgVal::Boolean(value) = &entry.val {
                return *value;
            }
        }
    }
    fallback
}
pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize).rev() {
        if entry.key == key {
            if let CfgVal::Int(value) = &entry.val {
                return *value;
            }
        }
    }
    fallback
}
pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize).rev() {
        if entry.key == key {
            if let CfgVal::Float(value) = &entry.val {
                return *value;
            }
        }
    }
    fallback
}
pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize).rev() {
        if entry.key == key {
            if let CfgVal::Color(value) = &entry.val {
                return *value;
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
    let _ = file.write_all(cfg.to_string().as_bytes());
}
pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    let _ = writeln!(file, "{err}");
}

fn init_error() -> CfgError {
    CfgError {
        off: -1,
        col: -1,
        row: -1,
        msg: String::new(),
    }
}

fn make_error(scanner: &Scanner, msg: impl Into<String>) -> CfgError {
    let off = scanner.cur;
    let mut row = 1;
    let mut col = 1;

    for ch in scanner.src[..off.max(0) as usize].chars() {
        col += 1;
        if ch == '\n' {
            row += 1;
            col = 1;
        }
    }

    CfgError {
        off,
        col,
        row,
        msg: msg.into(),
    }
}

fn is_at_end(scanner: &Scanner) -> bool {
    scanner.cur >= scanner.len
}

fn current_index(scanner: &Scanner) -> usize {
    scanner.cur.max(0) as usize
}

fn peek(scanner: &Scanner) -> Option<char> {
    scanner.src.as_bytes().get(current_index(scanner)).map(|b| *b as char)
}

fn peek_next(scanner: &Scanner) -> Option<char> {
    scanner
        .src
        .as_bytes()
        .get(current_index(scanner).saturating_add(1))
        .map(|b| *b as char)
}

fn advance(scanner: &mut Scanner) -> Option<char> {
    let ch = peek(scanner)?;
    scanner.cur += 1;
    Some(ch)
}

fn advance_n(scanner: &mut Scanner, n: usize) {
    scanner.cur += n as i32;
}

fn set_cur(scanner: &mut Scanner, cur: i32) {
    scanner.cur = cur;
}

fn skip_whitespace(scanner: &mut Scanner) {
    while matches!(peek(scanner), Some(ch) if ch.is_ascii_whitespace()) {
        advance(scanner);
    }
}

fn skip_blank(scanner: &mut Scanner) {
    while matches!(peek(scanner), Some(' ' | '\t' | '\r' | '\x0c' | '\x0b')) {
        advance(scanner);
    }
}

fn skip_comment(scanner: &mut Scanner) {
    while peek(scanner) == Some('#') {
        while !is_at_end(scanner) && peek(scanner) != Some('\n') {
            advance(scanner);
        }
    }
}

fn skip_whitespace_and_comments(scanner: &mut Scanner) {
    while matches!(peek(scanner), Some(ch) if ch.is_ascii_whitespace() || ch == '#') {
        skip_whitespace(scanner);
        skip_comment(scanner);
    }
}

fn is_key(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '.' || ch == '_'
}

fn is_string_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(ch, ' ' | '\t')
        || (ch.is_ascii_punctuation() && ch != '"')
}

fn match_literal(scanner: &Scanner, offset: i32, literal: &str) -> bool {
    let start = offset.max(0) as usize;
    scanner
        .src
        .as_bytes()
        .get(start..start.saturating_add(literal.len()))
        == Some(literal.as_bytes())
}

fn consume_literal(scanner: &mut Scanner, literal: &str) -> bool {
    if match_literal(scanner, scanner.cur, literal) {
        advance_n(scanner, literal.len());
        true
    } else {
        false
    }
}

fn parse_string(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    advance(scanner);
    let start = current_index(scanner);

    while matches!(peek(scanner), Some(ch) if is_string_char(ch)) {
        advance(scanner);
    }

    if is_at_end(scanner) || peek(scanner) != Some('"') {
        return Err(make_error(scanner, "closing '\"' expected"));
    }

    let end = current_index(scanner);
    let value = scanner.src[start..end].to_string();
    if value.len() > CFG_MAX_VAL {
        return Err(make_error(scanner, "value too long"));
    }

    advance(scanner);
    Ok(CfgVal::String(value))
}

fn consume_int(scanner: &mut Scanner) -> Result<i32, CfgError> {
    let mut sign = 1;
    let mut num = 0_i32;

    if peek(scanner) == Some('-') && matches!(peek_next(scanner), Some(ch) if ch.is_ascii_digit()) {
        advance(scanner);
        sign = -1;
    }

    if !matches!(peek(scanner), Some(ch) if ch.is_ascii_digit()) {
        return Err(make_error(scanner, "number expected"));
    }

    while let Some(ch) = peek(scanner) {
        if !ch.is_ascii_digit() {
            break;
        }
        let digit = (advance(scanner).unwrap() as u8 - b'0') as i32;
        num = num
            .checked_mul(10)
            .and_then(|n| n.checked_add(digit))
            .ok_or_else(|| make_error(scanner, "number too large"))?;
    }

    Ok(sign * num)
}

fn consume_float(scanner: &mut Scanner) -> Result<f32, CfgError> {
    let mut sign = 1.0_f32;
    let mut int_part = 0_i32;
    let mut fract_part = 0_i32;

    if peek(scanner) == Some('-') && matches!(peek_next(scanner), Some(ch) if ch.is_ascii_digit()) {
        advance(scanner);
        sign = -1.0;
    }

    if !matches!(peek(scanner), Some(ch) if ch.is_ascii_digit()) {
        return Err(make_error(scanner, "number expected"));
    }

    while let Some(ch) = peek(scanner) {
        if !ch.is_ascii_digit() {
            break;
        }
        let digit = (advance(scanner).unwrap() as u8 - b'0') as i32;
        int_part = int_part
            .checked_mul(10)
            .and_then(|n| n.checked_add(digit))
            .ok_or_else(|| make_error(scanner, "number too large"))?;
    }

    if peek(scanner) != Some('.') {
        return Err(make_error(scanner, "float expected"));
    }
    advance(scanner);

    let mut div = 1_i32;
    while let Some(ch) = peek(scanner) {
        if !ch.is_ascii_digit() {
            break;
        }
        let digit = (advance(scanner).unwrap() as u8 - b'0') as i32;
        fract_part = fract_part
            .checked_mul(10)
            .and_then(|n| n.checked_add(digit))
            .ok_or_else(|| make_error(scanner, "number too large"))?;
        div = div
            .checked_mul(10)
            .ok_or_else(|| make_error(scanner, "number too large"))?;
    }

    Ok(sign * (int_part as f32 + fract_part as f32 / div as f32))
}

fn match_float(scanner: &mut Scanner) -> bool {
    let restore = scanner.cur;

    if peek(scanner) == Some('-') && matches!(peek_next(scanner), Some(ch) if ch.is_ascii_digit()) {
        advance(scanner);
    }
    while matches!(peek(scanner), Some(ch) if ch.is_ascii_digit()) {
        advance(scanner);
    }
    let is_float = peek(scanner) == Some('.');
    set_cur(scanner, restore);
    is_float
}

fn parse_number(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    if match_float(scanner) {
        Ok(CfgVal::Float(consume_float(scanner)?))
    } else {
        Ok(CfgVal::Int(consume_int(scanner)?))
    }
}

fn parse_rgba(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(scanner, "rgba") {
        return Err(make_error(scanner, "invalid literal"));
    }

    skip_blank(scanner);
    if peek(scanner) != Some('(') {
        return Err(make_error(scanner, "'(' expected"));
    }
    advance(scanner);
    if is_at_end(scanner) {
        return Err(make_error(scanner, "',' expected"));
    }

    let mut rgb = [0_u8; 3];
    for channel in &mut rgb {
        skip_blank(scanner);
        if match_float(scanner) {
            return Err(make_error(
                scanner,
                "red, blue and green must be integers in range [0, 255]",
            ));
        }

        let number = consume_int(scanner)?;
        if !(0..=255).contains(&number) {
            return Err(make_error(
                scanner,
                "red, blue and green must be integers in range [0, 255]",
            ));
        }
        *channel = number as u8;

        skip_blank(scanner);
        if peek(scanner) != Some(',') {
            return Err(make_error(scanner, "',' expected"));
        }
        advance(scanner);
    }

    skip_blank(scanner);
    let alpha = if match_float(scanner) {
        let number = consume_float(scanner)?;
        if !(0.0..=1.0).contains(&number) {
            return Err(make_error(scanner, "alpha must be in range [0, 1]"));
        }
        (number * 255.0).round() as u8
    } else {
        let number = consume_int(scanner)?;
        if !(0..=1).contains(&number) {
            return Err(make_error(scanner, "alpha must be in range [0, 1]"));
        }
        (number * 255) as u8
    };

    skip_blank(scanner);
    if peek(scanner) != Some(')') {
        return Err(make_error(scanner, "')' expected"));
    }
    advance(scanner);

    Ok(CfgVal::Color(CfgColor {
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
        a: alpha,
    }))
}

fn parse_literal(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    match peek(scanner) {
        Some('t') => {
            if !consume_literal(scanner, "true") {
                Err(make_error(scanner, "invalid literal"))
            } else {
                Ok(CfgVal::Boolean(true))
            }
        }
        Some('f') => {
            if !consume_literal(scanner, "false") {
                Err(make_error(scanner, "invalid literal"))
            } else {
                Ok(CfgVal::Boolean(false))
            }
        }
        Some('r') => parse_rgba(scanner),
        _ => Err(make_error(scanner, "invalid literal")),
    }
}

fn parse_value(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    skip_blank(scanner);

    if is_at_end(scanner) || peek(scanner) == Some('\n') {
        return Err(make_error(scanner, "missing value"));
    }

    match peek(scanner) {
        Some('"') => parse_string(scanner),
        Some(ch) if ch.is_ascii_alphabetic() => parse_literal(scanner),
        Some(ch) if ch.is_ascii_digit() || (ch == '-' && matches!(peek_next(scanner), Some(n) if n.is_ascii_digit())) => {
            parse_number(scanner)
        }
        _ => Err(make_error(scanner, "invalid value")),
    }
}

fn parse_key(scanner: &mut Scanner) -> Result<String, CfgError> {
    if !matches!(peek(scanner), Some(ch) if is_key(ch)) {
        return Err(match peek(scanner) {
            Some(_) => make_error(scanner, "invalid character"),
            None => make_error(scanner, "missing key"),
        });
    }

    let start = current_index(scanner);
    while matches!(peek(scanner), Some(ch) if is_key(ch)) {
        advance(scanner);
    }
    let end = current_index(scanner);
    let key = &scanner.src[start..end];
    if key.len() > CFG_MAX_KEY {
        return Err(make_error(scanner, "key too long"));
    }
    Ok(key.to_string())
}

fn consume_colon(scanner: &mut Scanner) -> Result<(), CfgError> {
    let before_blank = scanner.cur;
    skip_blank(scanner);
    if is_at_end(scanner) {
        return Err(if scanner.cur > before_blank {
            make_error(scanner, "missing value")
        } else {
            make_error(scanner, "':' expected")
        });
    }
    if peek(scanner) != Some(':') {
        return Err(make_error(scanner, "':' expected"));
    }
    advance(scanner);
    Ok(())
}

fn parse_entry(scanner: &mut Scanner, err: &mut CfgError) -> Result<CfgEntry, CfgError> {
    let key = match parse_key(scanner) {
        Ok(key) => key,
        Err(parse_err) => {
            *err = parse_err.clone();
            return Err(parse_err);
        }
    };

    if let Err(parse_err) = consume_colon(scanner) {
        *err = parse_err.clone();
        return Err(parse_err);
    }

    let val = match parse_value(scanner) {
        Ok(val) => val,
        Err(parse_err) => {
            *err = parse_err.clone();
            return Err(parse_err);
        }
    };

    skip_blank(scanner);
    if peek(scanner) == Some('#') {
        skip_comment(scanner);
    }
    if !is_at_end(scanner) && peek(scanner) != Some('\n') {
        let parse_err = make_error(
            scanner,
            format!("unexpected character '{}'", peek(scanner).unwrap()),
        );
        *err = parse_err.clone();
        return Err(parse_err);
    }
    if !is_at_end(scanner) {
        advance(scanner);
    }

    Ok(CfgEntry { key, val })
}

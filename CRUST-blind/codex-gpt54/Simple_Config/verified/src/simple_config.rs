use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
};

pub const CFG_FILE_EXT: &str = ".cfg";
pub const CFG_MAX_KEY: usize = 32;
pub const CFG_MAX_VAL: usize = 64;
pub const CFG_MAX_ERR: usize = 64;

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
            CfgVal::Int(i) => write!(f, "{i}"),
            CfgVal::Float(n) => write!(f, "{n:.6}"),
            CfgVal::Color(c) => write!(f, "{c}"),
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

impl Scanner {
    fn new(src: &str) -> Self {
        Self {
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
            b'\0'
        } else {
            self.src.as_bytes()[self.cur as usize + 1]
        }
    }

    fn advance(&mut self) -> u8 {
        let ch = self.src.as_bytes()[self.cur as usize];
        self.cur += 1;
        ch
    }

    fn advance_n(&mut self, n: usize) -> u8 {
        for _ in 0..n.saturating_sub(1) {
            self.cur += 1;
        }
        self.advance()
    }
}

fn init_error() -> CfgError {
    CfgError {
        off: -1,
        col: -1,
        row: -1,
        msg: String::new(),
    }
}

fn truncate_msg(msg: &str) -> String {
    msg.chars().take(CFG_MAX_ERR - 1).collect()
}

fn error(scanner: &Scanner, msg: impl Into<String>) -> CfgError {
    let off = scanner.cur();
    let mut row = 1;
    let mut col = 1;

    for &byte in &scanner.src.as_bytes()[..off as usize] {
        col += 1;
        if byte == b'\n' {
            row += 1;
            col = 1;
        }
    }

    CfgError {
        off,
        col,
        row,
        msg: truncate_msg(&msg.into()),
    }
}

fn make_plain_error(msg: &str) -> CfgError {
    let mut err = init_error();
    err.msg = truncate_msg(msg);
    err
}

fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | 0x0b | 0x0c | b'\r')
}

fn is_key(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'.' || byte == b'_'
}

fn is_string(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || is_blank(byte)
        || (byte.is_ascii_punctuation() && byte != b'"')
}

fn skip_whitespace(scanner: &mut Scanner) {
    while !scanner.is_at_end() && is_whitespace(scanner.peek()) {
        scanner.advance();
    }
}

fn skip_blank(scanner: &mut Scanner) {
    while !scanner.is_at_end() && is_blank(scanner.peek()) {
        scanner.advance();
    }
}

fn skip_comment(scanner: &mut Scanner) {
    while !scanner.is_at_end() && scanner.peek() == b'#' {
        loop {
            scanner.advance();
            if scanner.is_at_end() || scanner.peek() == b'\n' {
                break;
            }
        }
    }
}

fn skip_whitespace_and_comments(scanner: &mut Scanner) {
    while !scanner.is_at_end()
        && (is_whitespace(scanner.peek()) || scanner.peek() == b'#')
    {
        skip_whitespace(scanner);
        skip_comment(scanner);
    }
}

fn match_literal(scanner: &Scanner, offset: i32, literal: &[u8]) -> bool {
    let start = offset as usize;
    let end = start + literal.len();
    if end > scanner.src.len() {
        return false;
    }
    &scanner.src.as_bytes()[start..end] == literal
}

fn consume_literal(scanner: &mut Scanner, offset: i32, literal: &[u8]) -> bool {
    if match_literal(scanner, offset, literal) {
        scanner.advance_n(literal.len());
        true
    } else {
        false
    }
}

fn parse_string(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    scanner.advance();

    let val_offset = scanner.cur() as usize;
    while !scanner.is_at_end() && is_string(scanner.peek()) {
        scanner.advance();
    }

    if scanner.is_at_end() || scanner.peek() != b'"' {
        return Err(error(scanner, "closing '\"' expected"));
    }

    let val_len = scanner.cur() as usize - val_offset;
    if val_len > CFG_MAX_VAL {
        return Err(error(scanner, "value too long"));
    }

    scanner.advance();

    Ok(CfgVal::String(
        scanner.src[val_offset..val_offset + val_len].to_string(),
    ))
}

fn consume_int(scanner: &mut Scanner) -> Result<i32, CfgError> {
    let mut sign = 1;
    let mut num: i32 = 0;

    if !scanner.is_at_end() && scanner.peek() == b'-' && scanner.peek_next().is_ascii_digit() {
        scanner.advance();
        sign = -1;
    }

    if scanner.is_at_end() || !scanner.peek().is_ascii_digit() {
        return Err(error(scanner, "number expected"));
    }

    while !scanner.is_at_end() && scanner.peek().is_ascii_digit() {
        let digit = (scanner.advance() - b'0') as i32;
        if num > (i32::MAX - digit) / 10 {
            return Err(error(scanner, "number too large"));
        }
        num = num * 10 + digit;
    }

    Ok(sign * num)
}

fn consume_float(scanner: &mut Scanner) -> Result<f32, CfgError> {
    let mut sign = 1.0f32;
    let mut int_part: i32 = 0;
    let mut fract_part: i32 = 0;

    if !scanner.is_at_end() && scanner.peek() == b'-' && scanner.peek_next().is_ascii_digit() {
        scanner.advance();
        sign = -1.0;
    }

    if scanner.is_at_end() || !scanner.peek().is_ascii_digit() {
        return Err(error(scanner, "number expected"));
    }

    while !scanner.is_at_end() && scanner.peek().is_ascii_digit() {
        let digit = (scanner.advance() - b'0') as i32;
        if int_part > (i32::MAX - digit) / 10 {
            return Err(error(scanner, "number too large"));
        }
        int_part = int_part * 10 + digit;
    }

    if scanner.is_at_end() || scanner.peek() != b'.' {
        return Err(error(scanner, "float expected"));
    }

    scanner.advance();

    let mut div: i32 = 1;
    while !scanner.is_at_end() && scanner.peek().is_ascii_digit() {
        let digit = (scanner.advance() - b'0') as i32;
        if fract_part > (i32::MAX - digit) / 10 {
            return Err(error(scanner, "number too large"));
        }

        fract_part = fract_part * 10 + digit;
        if div > i32::MAX / 10 {
            return Err(error(scanner, "number too large"));
        }
        div *= 10;
    }

    Ok(sign * (int_part as f32 + fract_part as f32 / div as f32))
}

fn match_float(scanner: &mut Scanner) -> bool {
    let restore = scanner.cur();
    let mut is_float = false;

    if !scanner.is_at_end() && scanner.peek() == b'-' && scanner.peek_next().is_ascii_digit() {
        scanner.advance();
    }

    while !scanner.is_at_end() && scanner.peek().is_ascii_digit() {
        scanner.advance();
    }

    if !scanner.is_at_end() && scanner.peek() == b'.' {
        is_float = true;
    }

    scanner.set_cur(restore);
    is_float
}

fn parse_number(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    if match_float(scanner) {
        consume_float(scanner).map(CfgVal::Float)
    } else {
        consume_int(scanner).map(CfgVal::Int)
    }
}

fn parse_rgba(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(scanner, scanner.cur(), b"rgba") {
        return Err(error(scanner, "invalid literal"));
    }

    skip_blank(scanner);

    if scanner.is_at_end() || scanner.peek() != b'(' {
        return Err(error(scanner, "'(' expected"));
    }

    scanner.advance();

    let mut rgb = [0u8; 3];
    for slot in &mut rgb {
        skip_blank(scanner);

        if match_float(scanner) {
            return Err(error(
                scanner,
                "red, blue and green must be integers in range [0, 255]",
            ));
        }

        let number = consume_int(scanner)?;
        if !(0..=255).contains(&number) {
            return Err(error(
                scanner,
                "red, blue and green must be integers in range [0, 255]",
            ));
        }

        *slot = number as u8;

        skip_blank(scanner);

        if scanner.is_at_end() || scanner.peek() != b',' {
            return Err(error(scanner, "',' expected"));
        }

        scanner.advance();
    }

    skip_blank(scanner);

    let alpha = if match_float(scanner) {
        let number = consume_float(scanner)?;
        if !(0.0..=1.0).contains(&number) {
            return Err(error(scanner, "alpha must be in range [0, 1]"));
        }
        (number * 255.0) as u8
    } else {
        let number = consume_int(scanner)?;
        if !(0..=1).contains(&number) {
            return Err(error(scanner, "alpha must be in range [0, 1]"));
        }
        (number * 255) as u8
    };

    skip_blank(scanner);

    if scanner.is_at_end() || scanner.peek() != b')' {
        return Err(error(scanner, "')' expected"));
    }

    scanner.advance();

    Ok(CfgVal::Color(CfgColor {
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
        a: alpha,
    }))
}

fn parse_true(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(scanner, scanner.cur(), b"true") {
        return Err(error(scanner, "invalid literal"));
    }
    Ok(CfgVal::Boolean(true))
}

fn parse_false(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    if !consume_literal(scanner, scanner.cur(), b"false") {
        return Err(error(scanner, "invalid literal"));
    }
    Ok(CfgVal::Boolean(false))
}

fn parse_literal(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    match scanner.peek() {
        b't' => parse_true(scanner),
        b'f' => parse_false(scanner),
        b'r' => parse_rgba(scanner),
        _ => Err(error(scanner, "invalid literal")),
    }
}

fn parse_value(scanner: &mut Scanner) -> Result<CfgVal, CfgError> {
    skip_blank(scanner);

    if scanner.is_at_end() || scanner.peek() == b'\n' {
        return Err(error(scanner, "missing value"));
    }

    match scanner.peek() {
        b'"' => parse_string(scanner),
        byte if byte.is_ascii_alphabetic() => parse_literal(scanner),
        byte if byte.is_ascii_digit()
            || (byte == b'-' && scanner.peek_next().is_ascii_digit()) =>
        {
            parse_number(scanner)
        }
        _ => Err(error(scanner, "invalid value")),
    }
}

fn parse_key(scanner: &mut Scanner) -> Result<String, CfgError> {
    if scanner.is_at_end() || !is_key(scanner.peek()) {
        return Err(error(scanner, "missing key"));
    }

    let key_offset = scanner.cur() as usize;
    loop {
        scanner.advance();
        if scanner.is_at_end() || !is_key(scanner.peek()) {
            break;
        }
    }
    let key_len = scanner.cur() as usize - key_offset;

    if key_len > CFG_MAX_KEY {
        return Err(error(scanner, "key too long"));
    }

    Ok(scanner.src[key_offset..key_offset + key_len].to_string())
}

fn consume_colon(scanner: &mut Scanner) -> Result<(), CfgError> {
    skip_blank(scanner);

    if scanner.is_at_end() || scanner.peek() != b':' {
        return Err(error(scanner, "':' expected"));
    }

    scanner.advance();
    Ok(())
}

fn parse_entry(scanner: &mut Scanner) -> Result<CfgEntry, CfgError> {
    let key = parse_key(scanner)?;
    consume_colon(scanner)?;
    let val = parse_value(scanner)?;

    skip_blank(scanner);

    if !scanner.is_at_end() && scanner.peek() == b'#' {
        skip_comment(scanner);
    }

    if !scanner.is_at_end() && scanner.peek() != b'\n' {
        return Err(error(
            scanner,
            format!("unexpected character '{}'", scanner.peek() as char),
        ));
    }

    if !scanner.is_at_end() {
        scanner.advance();
    }

    Ok(CfgEntry { key, val })
}

pub fn cfg_parse(src: &str) -> Result<Cfg, CfgError> {
    let mut scanner = Scanner::new(src);
    let mut entries = Vec::new();

    skip_whitespace_and_comments(&mut scanner);

    while !scanner.is_at_end() {
        let entry = parse_entry(&mut scanner)?;
        entries.push(entry);
        skip_whitespace_and_comments(&mut scanner);
    }

    Ok(Cfg {
        count: entries.len() as i32,
        capacity: entries.len(),
        entries,
    })
}

pub fn cfg_parse_file(filename: &str) -> Result<Cfg, CfgError> {
    if filename.len() < 5 {
        return Err(make_plain_error("invalid filename"));
    }

    if !filename.ends_with(CFG_FILE_EXT) {
        return Err(make_plain_error("invalid file extension"));
    }

    let mut file = File::open(filename).map_err(|_| make_plain_error("failed to open file"))?;
    let mut src = String::new();
    file.read_to_string(&mut src)
        .map_err(|_| make_plain_error("failed to read file"))?;

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
            if let CfgVal::Boolean(value) = entry.val {
                return value;
            }
        }
    }
    fallback
}

pub fn cfg_get_int(cfg: &Cfg, key: &str, fallback: i32) -> i32 {
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize).rev() {
        if entry.key == key {
            if let CfgVal::Int(value) = entry.val {
                return value;
            }
        }
    }
    fallback
}

pub fn cfg_get_float(cfg: &Cfg, key: &str, fallback: f32) -> f32 {
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize).rev() {
        if entry.key == key {
            if let CfgVal::Float(value) = entry.val {
                return value;
            }
        }
    }
    fallback
}

pub fn cfg_get_color(cfg: &Cfg, key: &str, fallback: CfgColor) -> CfgColor {
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize).rev() {
        if entry.key == key {
            if let CfgVal::Color(value) = entry.val {
                return value;
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
    for entry in cfg.entries.iter().take(cfg.count.max(0) as usize) {
        let _ = writeln!(file, "{entry}");
    }
}

pub fn cfg_fprint_error(file: &mut File, err: &CfgError) {
    let _ = writeln!(file, "{err}");
}


#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// ============================================================================
// Type definitions
// ============================================================================

#[derive(Copy, Clone, Debug, Default)]
pub struct analysis_result_t {
    pub word_count: usize,
    pub number_count: usize,
    pub keyword_count: usize,
    pub operator_count: usize,
    pub comment_count: usize,
    pub string_count: usize,
    pub line_count: usize,
    pub char_count: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum token_type_t {
    TOKEN_EOF = 0,
    TOKEN_WORD = 1,
    TOKEN_NUMBER = 2,
    TOKEN_PUNCTUATION = 3,
    TOKEN_WHITESPACE = 4,
    TOKEN_NEWLINE = 5,
    TOKEN_IDENTIFIER = 6,
    TOKEN_KEYWORD = 7,
    TOKEN_OPERATOR = 8,
    TOKEN_STRING = 9,
    TOKEN_COMMENT = 10,
    TOKEN_ERROR = 11,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct token_t {
    pub type_: token_type_t,
    pub value: [u8; MAX_TOKEN_LENGTH as usize],
    pub length: usize,
    pub line: i32,
    pub column: i32,
}

impl Default for token_t {
    fn default() -> Self {
        token_t {
            type_: token_type_t::TOKEN_EOF,
            value: [0u8; MAX_TOKEN_LENGTH as usize],
            length: 0,
            line: 0,
            column: 0,
        }
    }
}

// Function pointer types (extern "C" for FFI compatibility)
pub type tokenizer_next_fn = extern "C" fn() -> token_t;
pub type tokenizer_peek_fn = extern "C" fn() -> token_t;
pub type tokenizer_reset_fn = extern "C" fn();
pub type tokenizer_load_fn = extern "C" fn(*const std::os::raw::c_char) -> i32;
pub type tokenizer_get_stats_fn =
    extern "C" fn(*mut usize, *mut usize, *mut usize);

#[derive(Copy, Clone)]
#[repr(C)]
pub struct tokenizer_ops_t {
    pub next_token: Option<tokenizer_next_fn>,
    pub peek_token: Option<tokenizer_peek_fn>,
    pub reset: Option<tokenizer_reset_fn>,
    pub load_text: Option<tokenizer_load_fn>,
    pub get_stats: Option<tokenizer_get_stats_fn>,
}

impl Default for tokenizer_ops_t {
    fn default() -> Self {
        tokenizer_ops_t {
            next_token: None,
            peek_token: None,
            reset: None,
            load_text: None,
            get_stats: None,
        }
    }
}

// ============================================================================
// Global variables shared across the FFI boundary
// ============================================================================

unsafe extern "C" {
    static mut buffer_length: usize;
    static mut current_column: i32;
    static mut current_line: i32;
    static mut current_position: usize;
    static mut input_buffer: [u8; MAX_BUFFER_SIZE as usize];
    static mut total_chars_processed: usize;
    static mut total_lines_processed: usize;
    static mut total_tokens_processed: usize;
    static mut lookahead_valid: i32;
    static mut keywords: [*const std::os::raw::c_char; 31];
    static mut num_keywords: i32;
    static mut common_word_counts: [i32; 100];
    static mut common_words: [[u8; MAX_TOKEN_LENGTH as usize]; 100];
    static mut num_common_words: i32;
    static mut initialized: i32;
    static mut token_type_counts: [i32; 20];
    static mut tokenizer_ops: tokenizer_ops_t;
}

// ---- Getters and setters ----

pub fn rust_get_buffer_length() -> usize {
    unsafe { buffer_length }
}
pub fn rust_set_buffer_length(val: usize) {
    unsafe { buffer_length = val; }
}

pub fn rust_get_current_column() -> i32 {
    unsafe { current_column }
}
pub fn rust_set_current_column(val: i32) {
    unsafe { current_column = val; }
}

pub fn rust_get_current_line() -> i32 {
    unsafe { current_line }
}
pub fn rust_set_current_line(val: i32) {
    unsafe { current_line = val; }
}

pub fn rust_get_current_position() -> usize {
    unsafe { current_position }
}
pub fn rust_set_current_position(val: usize) {
    unsafe { current_position = val; }
}

pub fn rust_get_input_buffer() -> [u8; MAX_BUFFER_SIZE as usize] {
    unsafe { input_buffer }
}
pub fn rust_set_input_buffer(val: [u8; MAX_BUFFER_SIZE as usize]) {
    unsafe { input_buffer = val; }
}

pub fn rust_get_total_chars_processed() -> usize {
    unsafe { total_chars_processed }
}
pub fn rust_set_total_chars_processed(val: usize) {
    unsafe { total_chars_processed = val; }
}

pub fn rust_get_total_lines_processed() -> usize {
    unsafe { total_lines_processed }
}
pub fn rust_set_total_lines_processed(val: usize) {
    unsafe { total_lines_processed = val; }
}

pub fn rust_get_total_tokens_processed() -> usize {
    unsafe { total_tokens_processed }
}
pub fn rust_set_total_tokens_processed(val: usize) {
    unsafe { total_tokens_processed = val; }
}

pub fn rust_get_lookahead_valid() -> i32 {
    unsafe { lookahead_valid }
}
pub fn rust_set_lookahead_valid(val: i32) {
    unsafe { lookahead_valid = val; }
}

pub fn rust_get_keywords() -> [*const std::os::raw::c_char; 31] {
    unsafe { keywords }
}
pub fn rust_set_keywords(val: [*const std::os::raw::c_char; 31]) {
    unsafe { keywords = val; }
}

pub fn rust_get_num_keywords() -> i32 {
    unsafe { num_keywords }
}
pub fn rust_set_num_keywords(val: i32) {
    unsafe { num_keywords = val; }
}

pub fn rust_get_common_word_counts() -> [i32; 100] {
    unsafe { common_word_counts }
}
pub fn rust_set_common_word_counts(val: [i32; 100]) {
    unsafe { common_word_counts = val; }
}

pub fn rust_get_common_words() -> [[u8; MAX_TOKEN_LENGTH as usize]; 100] {
    unsafe { common_words }
}
pub fn rust_set_common_words(val: [[u8; MAX_TOKEN_LENGTH as usize]; 100]) {
    unsafe { common_words = val; }
}

pub fn rust_get_num_common_words() -> i32 {
    unsafe { num_common_words }
}
pub fn rust_set_num_common_words(val: i32) {
    unsafe { num_common_words = val; }
}

pub fn rust_get_initialized() -> i32 {
    unsafe { initialized }
}
pub fn rust_set_initialized(val: i32) {
    unsafe { initialized = val; }
}

pub fn rust_get_token_type_counts() -> [i32; 20] {
    unsafe { token_type_counts }
}
pub fn rust_set_token_type_counts(val: [i32; 20]) {
    unsafe { token_type_counts = val; }
}

pub fn rust_get_tokenizer_ops() -> tokenizer_ops_t {
    unsafe { tokenizer_ops }
}
pub fn rust_set_tokenizer_ops(val: tokenizer_ops_t) {
    unsafe { tokenizer_ops = val; }
}

// ============================================================================
// Helpers
// ============================================================================

fn cstr_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn bytes_to_str(buf: &[u8]) -> &str {
    let len = cstr_len(buf);
    std::str::from_utf8(&buf[..len]).unwrap_or("")
}

/// Keywords list, matching the C `keywords[]` array.
const KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "return", "int", "char",
    "float", "double", "void", "struct", "typedef", "const",
    "static", "extern", "auto", "register", "sizeof", "break",
    "continue", "switch", "case", "default", "do", "goto",
    "enum", "union", "signed", "unsigned", "long", "short",
];

// ============================================================================
// Tokenizer / analyzer functions (safe Rust wrappers)
// ============================================================================

fn rust_advance_char() -> u8 {
    let pos = rust_get_current_position();
    let buflen = rust_get_buffer_length();

    if pos >= buflen {
        return 0;
    }

    let buf = rust_get_input_buffer();
    let c = buf[pos];

    rust_set_current_position(pos + 1);
    rust_set_total_chars_processed(rust_get_total_chars_processed() + 1);

    if c == b'\n' {
        rust_set_current_line(rust_get_current_line() + 1);
        rust_set_current_column(1);
        rust_set_total_lines_processed(rust_get_total_lines_processed() + 1);
    } else {
        rust_set_current_column(rust_get_current_column() + 1);
    }

    c
}

fn rust_peek_char() -> u8 {
    let pos = rust_get_current_position();
    let buflen = rust_get_buffer_length();

    if pos >= buflen {
        return 0;
    }
    rust_get_input_buffer()[pos]
}

fn rust_track_word(word: &[u8]) {
    let word_str = bytes_to_str(word);
    let num = rust_get_num_common_words();
    let mut words = rust_get_common_words();
    let mut counts = rust_get_common_word_counts();

    // Find if word already exists
    for i in 0..num as usize {
        if bytes_to_str(&words[i]) == word_str {
            counts[i] += 1;
            rust_set_common_word_counts(counts);
            return;
        }
    }

    // Add new word
    if num < 100 {
        let idx = num as usize;
        let dst = &mut words[idx];
        for b in dst.iter_mut() {
            *b = 0;
        }
        let max_copy = (MAX_TOKEN_LENGTH as usize) - 1;
        let src_len = cstr_len(word).min(max_copy);
        dst[..src_len].copy_from_slice(&word[..src_len]);
        dst[MAX_TOKEN_LENGTH as usize - 1] = 0;

        counts[idx] = 1;

        rust_set_common_words(words);
        rust_set_common_word_counts(counts);
        rust_set_num_common_words(num + 1);
    }
}

fn rust_analyze_text(text: &str) -> analysis_result_t {
    let mut result = analysis_result_t::default();

    if rust_get_initialized() == 0 {
        eprintln!("Error: Analyzer not initialized");
        return result;
    }

    // Load text: prepare a NUL-terminated C string and call the load_text fp
    let ops = rust_get_tokenizer_ops();
    let mut c_text: Vec<u8> = text.as_bytes().to_vec();
    c_text.push(0);

    let load_result = match ops.load_text {
        Some(f) => f(c_text.as_ptr() as *const std::os::raw::c_char),
        None => -1,
    };
    if load_result != 0 {
        eprintln!("Error: Failed to load text");
        return result;
    }

    // Process all tokens
    let next_fn = ops.next_token;
    loop {
        let token = match next_fn {
            Some(f) => f(),
            None => break,
        };
        if token.type_ == token_type_t::TOKEN_EOF {
            break;
        }

        // Update counts
        let type_idx = token.type_ as usize;
        if type_idx < 20 {
            let mut counts = rust_get_token_type_counts();
            counts[type_idx] += 1;
            rust_set_token_type_counts(counts);
        }

        match token.type_ {
            token_type_t::TOKEN_WORD | token_type_t::TOKEN_IDENTIFIER => {
                result.word_count += 1;
                rust_track_word(&token.value);
            }
            token_type_t::TOKEN_NUMBER => {
                result.number_count += 1;
            }
            token_type_t::TOKEN_KEYWORD => {
                result.keyword_count += 1;
            }
            token_type_t::TOKEN_OPERATOR => {
                result.operator_count += 1;
            }
            token_type_t::TOKEN_COMMENT => {
                result.comment_count += 1;
            }
            token_type_t::TOKEN_STRING => {
                result.string_count += 1;
            }
            token_type_t::TOKEN_NEWLINE => {
                result.line_count += 1;
            }
            _ => {}
        }
    }

    // Get final statistics using function pointer
    let mut lines: usize = 0;
    let mut tokens: usize = 0;
    let mut chars: usize = 0;
    if let Some(f) = ops.get_stats {
        f(&mut lines as *mut usize, &mut tokens as *mut usize, &mut chars as *mut usize);
    }

    result.line_count = lines;
    result.char_count = chars;

    result
}

fn rust_analyzer_init(ops: tokenizer_ops_t) {
    rust_set_tokenizer_ops(ops);
    rust_set_initialized(1);

    // Reset tracking arrays
    rust_set_token_type_counts([0i32; 20]);
    rust_set_common_word_counts([0i32; 100]);
    rust_set_num_common_words(0);
}

fn rust_calculate_complexity_score() -> i32 {
    let counts = rust_get_token_type_counts();

    let mut score: i32 = 0;
    // Base score on keyword density
    score += counts[token_type_t::TOKEN_KEYWORD as usize] * 2;
    // Add points for operators
    score += counts[token_type_t::TOKEN_OPERATOR as usize];
    // Nesting indicators (braces)
    score += counts[token_type_t::TOKEN_PUNCTUATION as usize] / 10;
    // Comments reduce complexity (good documentation)
    score -= counts[token_type_t::TOKEN_COMMENT as usize];

    if score < 0 {
        score = 0;
    }
    score
}

fn rust_create_token(type_: token_type_t, value: &[u8], length: usize) -> token_t {
    let mut token = token_t::default();
    token.type_ = type_;
    let max_len = MAX_TOKEN_LENGTH as usize;
    token.length = if length < max_len { length } else { max_len - 1 };

    let copy_len = token.length.min(value.len());
    token.value[..copy_len].copy_from_slice(&value[..copy_len]);
    if token.length < max_len {
        token.value[token.length] = 0;
    }

    token.line = rust_get_current_line();
    token.column = rust_get_current_column() - token.length as i32;
    rust_set_total_tokens_processed(rust_get_total_tokens_processed() + 1);

    token
}

fn rust_find_patterns(pattern: &str) {
    if rust_get_initialized() == 0 {
        return;
    }

    println!("\n=== Searching for pattern: '{}' ===", pattern);

    let ops = rust_get_tokenizer_ops();

    // Reset tokenizer using function pointer
    if let Some(f) = ops.reset {
        f();
    }

    let mut count: i32 = 0;
    let next_fn = ops.next_token;

    loop {
        let token = match next_fn {
            Some(f) => f(),
            None => break,
        };
        if token.type_ == token_type_t::TOKEN_EOF {
            break;
        }

        let value_str = bytes_to_str(&token.value);
        if value_str.contains(pattern) {
            println!("Line {}, Column {}: {}", token.line, token.column, value_str);
            count += 1;
        }
    }

    println!("Found {} occurrences", count);
}

fn rust_tokenizer_get_stats(lines: &mut usize, tokens: &mut usize, chars: &mut usize) {
    *lines = rust_get_total_lines_processed();
    *tokens = rust_get_total_tokens_processed();
    *chars = rust_get_total_chars_processed();
}

fn rust_tokenizer_reset() {
    rust_set_current_position(0);
    rust_set_current_line(1);
    rust_set_current_column(1);
    rust_set_lookahead_valid(0);
    // Note: We don't reset total statistics
}

fn rust_tokenizer_load_text(text: &str) -> i32 {
    let length = text.len();
    if length >= MAX_BUFFER_SIZE as usize {
        eprintln!("Error: Input text too large");
        return -1;
    }

    let bytes = text.as_bytes();
    let mut buf = [0u8; MAX_BUFFER_SIZE as usize];
    let max_copy = (MAX_BUFFER_SIZE as usize) - 1;
    let copy_len = bytes.len().min(max_copy);
    buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
    // Last byte remains 0 (NUL terminator)

    rust_set_input_buffer(buf);
    rust_set_buffer_length(length);

    rust_tokenizer_reset();
    0
}

fn rust_scan_comment() -> token_t {
    let max_len = MAX_TOKEN_LENGTH as usize;
    let mut buffer = vec![0u8; max_len];
    let mut length: usize = 0;

    // Assume we've seen '/'
    buffer[length] = rust_advance_char(); // First '/'
    length += 1;

    if rust_peek_char() == b'/' {
        // Single-line comment
        buffer[length] = rust_advance_char(); // Second '/'
        length += 1;

        while rust_peek_char() != 0
            && rust_peek_char() != b'\n'
            && length < max_len - 1
        {
            buffer[length] = rust_advance_char();
            length += 1;
        }
    } else if rust_peek_char() == b'*' {
        // Multi-line comment
        buffer[length] = rust_advance_char(); // '*'
        length += 1;

        while rust_peek_char() != 0 && length < max_len - 2 {
            if rust_peek_char() == b'*' {
                buffer[length] = rust_advance_char();
                length += 1;
                if rust_peek_char() == b'/' {
                    buffer[length] = rust_advance_char();
                    length += 1;
                    break;
                }
            } else {
                buffer[length] = rust_advance_char();
                length += 1;
            }
        }
    }

    if length < max_len {
        buffer[length] = 0;
    }
    rust_create_token(token_type_t::TOKEN_COMMENT, &buffer, length)
}

fn rust_scan_number() -> token_t {
    let max_len = MAX_TOKEN_LENGTH as usize;
    let mut buffer = vec![0u8; max_len];
    let mut length: usize = 0;
    let mut has_decimal = false;

    loop {
        let c = rust_peek_char();
        if c == 0 || length >= max_len - 1 {
            break;
        }
        if !(c as char).is_ascii_digit() && c != b'.' {
            break;
        }
        if c == b'.' {
            if has_decimal {
                break; // Second decimal point
            }
            has_decimal = true;
        }

        buffer[length] = rust_advance_char();
        length += 1;
    }

    if length < max_len {
        buffer[length] = 0;
    }
    rust_create_token(token_type_t::TOKEN_NUMBER, &buffer, length)
}

fn rust_scan_operator() -> token_t {
    let max_len = MAX_TOKEN_LENGTH as usize;
    let mut buffer = vec![0u8; max_len];
    let mut length: usize = 0;
    let c = rust_peek_char();

    buffer[length] = rust_advance_char();
    length += 1;

    // Check for two-character operators
    let next = rust_peek_char();
    let is_two_char = matches!(
        (c, next),
        (b'=', b'=')
            | (b'!', b'=')
            | (b'<', b'=')
            | (b'>', b'=')
            | (b'&', b'&')
            | (b'|', b'|')
            | (b'+', b'+')
            | (b'-', b'-')
            | (b'-', b'>')
            | (b'<', b'<')
            | (b'>', b'>')
    );

    if is_two_char {
        buffer[length] = rust_advance_char();
        length += 1;
    }

    if length < max_len {
        buffer[length] = 0;
    }
    rust_create_token(token_type_t::TOKEN_OPERATOR, &buffer, length)
}

fn rust_scan_string() -> token_t {
    let max_len = MAX_TOKEN_LENGTH as usize;
    let mut buffer = vec![0u8; max_len];
    let mut length: usize = 0;
    let quote = rust_advance_char(); // Consume opening quote

    buffer[length] = quote;
    length += 1;

    while rust_peek_char() != 0
        && rust_peek_char() != quote
        && rust_peek_char() != b'\n'
        && length < max_len - 2
    {
        if rust_peek_char() == b'\\' {
            buffer[length] = rust_advance_char(); // Escape character
            length += 1;
            if rust_peek_char() != 0 {
                buffer[length] = rust_advance_char(); // Escaped character
                length += 1;
            }
        } else {
            buffer[length] = rust_advance_char();
            length += 1;
        }
    }

    if rust_peek_char() == quote {
        buffer[length] = rust_advance_char(); // Closing quote
        length += 1;
    }

    if length < max_len {
        buffer[length] = 0;
    }
    rust_create_token(token_type_t::TOKEN_STRING, &buffer, length)
}

fn rust_is_keyword(s: &str) -> bool {
    KEYWORDS.iter().any(|kw| *kw == s)
}

fn rust_scan_word() -> token_t {
    let max_len = MAX_TOKEN_LENGTH as usize;
    let mut buffer = vec![0u8; max_len];
    let mut length: usize = 0;

    while rust_peek_char() != 0
        && ((rust_peek_char() as char).is_ascii_alphanumeric() || rust_peek_char() == b'_')
        && length < max_len - 1
    {
        buffer[length] = rust_advance_char();
        length += 1;
    }

    if length < max_len {
        buffer[length] = 0;
    }

    // Check if it's a keyword
    if rust_is_keyword(bytes_to_str(&buffer)) {
        return rust_create_token(token_type_t::TOKEN_KEYWORD, &buffer, length);
    }

    rust_create_token(token_type_t::TOKEN_IDENTIFIER, &buffer, length)
}

// ============================================================================
// lookahead_token global variable (FFI boundary)
// ============================================================================

unsafe extern "C" {
    static mut lookahead_token: token_t;
}

pub fn rust_get_lookahead_token() -> token_t {
    unsafe { lookahead_token }
}
pub fn rust_set_lookahead_token(val: token_t) {
    unsafe { lookahead_token = val; }
}

// ============================================================================
// Tokenizer helper: skip_whitespace
// ============================================================================

fn rust_skip_whitespace() {
    loop {
        let c = rust_peek_char();
        if c == 0 || c == b'\n' || !c.is_ascii_whitespace() {
            break;
        }
        rust_advance_char();
    }
}

// ============================================================================
// Tokenizer entry points (called through function pointers)
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_next_token() -> token_t {
    // Check if we have a lookahead token
    if rust_get_lookahead_valid() != 0 {
        rust_set_lookahead_valid(0);
        return rust_get_lookahead_token();
    }

    rust_skip_whitespace();

    if rust_get_current_position() >= rust_get_buffer_length() {
        return rust_create_token(token_type_t::TOKEN_EOF, b"", 0);
    }

    let c = rust_peek_char();

    // Newline
    if c == b'\n' {
        let ch = rust_advance_char();
        let newline = [ch, 0u8];
        return rust_create_token(token_type_t::TOKEN_NEWLINE, &newline, 1);
    }

    // Identifier or keyword
    if c.is_ascii_alphabetic() || c == b'_' {
        return rust_scan_word();
    }

    // Number
    if c.is_ascii_digit() {
        return rust_scan_number();
    }

    // String
    if c == b'"' || c == b'\'' {
        return rust_scan_string();
    }

    // Comment (matches original C behavior: peek_char() after seeing '/' still returns '/')
    if c == b'/' && (rust_peek_char() == b'/' || rust_peek_char() == b'*') {
        return rust_scan_comment();
    }

    // Operator
    if b"+-*/%=<>!&|^~?:".contains(&c) {
        return rust_scan_operator();
    }

    // Punctuation
    if b"(){}[];,.".contains(&c) {
        let ch = rust_advance_char();
        let punct = [ch, 0u8];
        return rust_create_token(token_type_t::TOKEN_PUNCTUATION, &punct, 1);
    }

    // Unknown character
    let ch = rust_advance_char();
    let unknown = [ch, 0u8];
    rust_create_token(token_type_t::TOKEN_ERROR, &unknown, 1)
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_peek_token() -> token_t {
    if rust_get_lookahead_valid() == 0 {
        let tok = tokenizer_next_token();
        rust_set_lookahead_token(tok);
        rust_set_lookahead_valid(1);
    }
    rust_get_lookahead_token()
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_reset() {
    rust_tokenizer_reset();
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_load_text(text: *const std::os::raw::c_char) -> i32 {
    if text.is_null() {
        return -1;
    }
    let s = match unsafe { std::ffi::CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    rust_tokenizer_load_text(s)
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_get_stats(lines: *mut usize, tokens: *mut usize, chars: *mut usize) {
    unsafe {
        if let Some(p) = lines.as_mut() {
            *p = rust_get_total_lines_processed();
        }
        if let Some(p) = tokens.as_mut() {
            *p = rust_get_total_tokens_processed();
        }
        if let Some(p) = chars.as_mut() {
            *p = rust_get_total_chars_processed();
        }
    }
}

fn rust_get_tokenizer_ops_fn() -> tokenizer_ops_t {
    tokenizer_ops_t {
        next_token: Some(tokenizer_next_token),
        peek_token: Some(tokenizer_peek_token),
        reset: Some(tokenizer_reset),
        load_text: Some(tokenizer_load_text),
        get_stats: Some(tokenizer_get_stats),
    }
}

// ============================================================================
// Interactive UI helpers
// ============================================================================

/// Read lines from stdin until an empty line (or EOF), concatenating into `dest`
/// while respecting the given maximum capacity. Mirrors the C fgets-loop.
fn read_lines_until_blank(dest: &mut String, max_size: usize) {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    loop {
        let mut line = String::new();
        match handle.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        if line == "\n" || line == "\r\n" {
            break;
        }
        let remaining = max_size.saturating_sub(dest.len()).saturating_sub(1);
        if remaining == 0 {
            continue;
        }
        let take = line.len().min(remaining);
        dest.push_str(&line[..take]);
    }
}

/// Read a single line from stdin, trimming trailing newline characters.
fn read_line_trimmed() -> Option<String> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line.trim_end_matches(|c| c == '\n' || c == '\r').to_string()),
        Err(_) => None,
    }
}

fn flush_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

fn rust_interactive_tokenizer(ops: tokenizer_ops_t) {
    println!("\nEnter text (empty line to stop):");

    let mut input = String::new();
    read_lines_until_blank(&mut input, MAX_INPUT_SIZE as usize);

    let mut c_input: Vec<u8> = input.into_bytes();
    c_input.push(0);

    let load_result = match ops.load_text {
        Some(f) => f(c_input.as_ptr() as *const std::os::raw::c_char),
        None => -1,
    };
    if load_result != 0 {
        println!("Failed to load text");
        return;
    }

    println!("\n=== Tokens ===");

    const TOKEN_TYPE_NAMES: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE",
        "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let next_fn = match ops.next_token {
        Some(f) => f,
        None => return,
    };

    let mut count: i32 = 0;
    loop {
        let token = next_fn();
        if token.type_ == token_type_t::TOKEN_EOF {
            break;
        }

        let type_idx = token.type_ as usize;
        let type_name = TOKEN_TYPE_NAMES.get(type_idx).copied().unwrap_or("UNKNOWN");
        let value_str = bytes_to_str(&token.value);
        println!("[{}] '{}' (L{}:C{})", type_name, value_str, token.line, token.column);
        count += 1;

        if count > 100 {
            println!("... (truncated, too many tokens)");
            break;
        }
    }
}

fn rust_print_analysis_result(result: analysis_result_t) {
    println!("\n=== Analysis Results ===");
    println!("Words/Identifiers: {}", result.word_count);
    println!("Numbers: {}", result.number_count);
    println!("Keywords: {}", result.keyword_count);
    println!("Operators: {}", result.operator_count);
    println!("Comments: {}", result.comment_count);
    println!("Strings: {}", result.string_count);
    println!("Lines: {}", result.line_count);
    println!("Characters: {}", result.char_count);
}

fn rust_print_menu() {
    println!("\n=== Text Analyzer ===");
    println!("1. Analyze text");
    println!("2. Load text from file");
    println!("3. Show token distribution");
    println!("4. Calculate complexity score");
    println!("5. Find pattern");
    println!("6. Interactive tokenizer");
    println!("7. Exit");
    print!("Choice: ");
    flush_stdout();
}

fn rust_print_token_distribution() {
    println!("\n=== Token Distribution ===");

    const TOKEN_NAMES: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let counts = rust_get_token_type_counts();
    for (name, &count) in TOKEN_NAMES.iter().zip(counts.iter()) {
        if count > 0 {
            println!("{}: {}", name, count);
        }
    }

    println!("\n=== Most Common Words ===");

    let num = rust_get_num_common_words().max(0) as usize;
    let mut words = rust_get_common_words();
    let mut word_counts = rust_get_common_word_counts();

    // Sort by count descending using indices (equivalent to the C bubble sort's result).
    let mut indices: Vec<usize> = (0..num).collect();
    indices.sort_by(|&a, &b| word_counts[b].cmp(&word_counts[a]));

    let sorted_words: Vec<[u8; MAX_TOKEN_LENGTH as usize]> =
        indices.iter().map(|&i| words[i]).collect();
    let sorted_counts: Vec<i32> = indices.iter().map(|&i| word_counts[i]).collect();
    for (i, (w, c)) in sorted_words.iter().zip(sorted_counts.iter()).enumerate() {
        words[i] = *w;
        word_counts[i] = *c;
    }

    rust_set_common_words(words);
    rust_set_common_word_counts(word_counts);

    // Print top 10
    let limit = num.min(10);
    for i in 0..limit {
        println!("{}. {}: {} times", i + 1, bytes_to_str(&words[i]), word_counts[i]);
    }
}

fn rust_read_file(filename: &str) -> Option<String> {
    let metadata = match std::fs::metadata(filename) {
        Ok(m) => m,
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", filename);
            return None;
        }
    };
    if metadata.len() > MAX_BUFFER_SIZE as u64 {
        eprintln!("Error: File too large");
        return None;
    }
    match std::fs::read_to_string(filename) {
        Ok(s) => Some(s),
        Err(_) => {
            eprintln!("Error: Could not read file '{}'", filename);
            None
        }
    }
}

// ============================================================================
// Main entry point (FFI boundary function)
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    // Get tokenizer operations (function pointers)
    let ops = rust_get_tokenizer_ops_fn();

    // Initialize analyzer with function pointers
    rust_analyzer_init(ops);

    println!("Text Analysis and Tokenization System");
    println!("This system demonstrates function pointers and static globals");

    loop {
        rust_print_menu();

        let input = match read_line_trimmed() {
            Some(s) => s,
            None => break,
        };

        let choice: i32 = match input.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => {
                println!("Enter text to analyze (empty line to stop):");
                let mut text = String::new();
                read_lines_until_blank(&mut text, MAX_INPUT_SIZE as usize);
                let result = rust_analyze_text(&text);
                rust_print_analysis_result(result);
            }
            2 => {
                print!("Enter filename: ");
                flush_stdout();
                let filename = match read_line_trimmed() {
                    Some(s) => s,
                    None => break,
                };
                if let Some(content) = rust_read_file(&filename) {
                    let result = rust_analyze_text(&content);
                    rust_print_analysis_result(result);
                }
            }
            3 => {
                rust_print_token_distribution();
            }
            4 => {
                let score = rust_calculate_complexity_score();
                println!("\nComplexity Score: {}", score);
                let category = if score < 10 {
                    "Low"
                } else if score < 50 {
                    "Medium"
                } else {
                    "High"
                };
                println!("Complexity: {}", category);
            }
            5 => {
                print!("Enter pattern to search: ");
                flush_stdout();
                let pattern = match read_line_trimmed() {
                    Some(s) => s,
                    None => break,
                };
                rust_find_patterns(&pattern);
            }
            6 => {
                rust_interactive_tokenizer(ops);
            }
            7 => {
                println!("Goodbye!");
                return 0;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }

    0
}
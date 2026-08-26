mod tokenizer;

use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::{Mutex, OnceLock};

use tokenizer::{Token, TokenType, Tokenizer, MAX_TOKEN_LENGTH};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CToken {
    pub token_type: c_int,
    pub value: [c_char; MAX_TOKEN_LENGTH],
    pub length: usize,
    pub line: c_int,
    pub column: c_int,
}

type NextTokenFn = unsafe extern "C" fn() -> CToken;
type PeekTokenFn = unsafe extern "C" fn() -> CToken;
type ResetFn = unsafe extern "C" fn();
type LoadTextFn = unsafe extern "C" fn(*const c_char) -> c_int;
type GetStatsFn = unsafe extern "C" fn(*mut usize, *mut usize, *mut usize);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TokenizerOps {
    pub next_token: Option<NextTokenFn>,
    pub peek_token: Option<PeekTokenFn>,
    pub reset: Option<ResetFn>,
    pub load_text: Option<LoadTextFn>,
    pub get_stats: Option<GetStatsFn>,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AnalysisResult {
    pub word_count: usize,
    pub number_count: usize,
    pub keyword_count: usize,
    pub operator_count: usize,
    pub comment_count: usize,
    pub string_count: usize,
    pub line_count: usize,
    pub char_count: usize,
}

struct AnalyzerState {
    ops: TokenizerOps,
    initialized: bool,
    token_type_counts: [c_int; 20],
    common_words: Vec<(Vec<u8>, c_int)>,
}

impl AnalyzerState {
    fn new() -> Self {
        Self {
            ops: TokenizerOps {
                next_token: None,
                peek_token: None,
                reset: None,
                load_text: None,
                get_stats: None,
            },
            initialized: false,
            token_type_counts: [0; 20],
            common_words: Vec::new(),
        }
    }

    fn track_word(&mut self, word: &[u8]) {
        if let Some((_, count)) = self
            .common_words
            .iter_mut()
            .find(|(known, _)| known.as_slice() == word)
        {
            *count = count.wrapping_add(1);
        } else if self.common_words.len() < 100 {
            self.common_words.push((word.to_vec(), 1));
        }
    }
}

fn tokenizer() -> &'static Mutex<Tokenizer> {
    static TOKENIZER: OnceLock<Mutex<Tokenizer>> = OnceLock::new();
    TOKENIZER.get_or_init(|| Mutex::new(Tokenizer::new()))
}

fn analyzer() -> &'static Mutex<AnalyzerState> {
    static ANALYZER: OnceLock<Mutex<AnalyzerState>> = OnceLock::new();
    ANALYZER.get_or_init(|| Mutex::new(AnalyzerState::new()))
}

fn to_c_token(token: Token) -> CToken {
    let mut value = [0; MAX_TOKEN_LENGTH];
    for (destination, source) in value.iter_mut().zip(&token.value) {
        *destination = *source as c_char;
    }
    CToken {
        token_type: token.token_type as c_int,
        value,
        length: token.value.len(),
        line: token.line,
        column: token.column,
    }
}

fn token_value(token: &CToken) -> &[u8] {
    let length = token
        .value
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(MAX_TOKEN_LENGTH);
    unsafe { std::slice::from_raw_parts(token.value.as_ptr().cast::<u8>(), length) }
}

unsafe fn required<T: Copy>(callback: Option<T>) -> T {
    callback.unwrap_unchecked()
}

unsafe extern "C" {
    static mut stdout: *mut c_void;
    static mut stderr: *mut c_void;
    fn fwrite(buffer: *const c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
}

fn write_stream(stream: *mut c_void, bytes: &[u8]) {
    if !bytes.is_empty() {
        unsafe {
            fwrite(bytes.as_ptr().cast(), 1, bytes.len(), stream);
        }
    }
}

fn write_stdout(bytes: &[u8]) {
    write_stream(unsafe { stdout }, bytes);
}

fn write_stderr(bytes: &[u8]) {
    write_stream(unsafe { stderr }, bytes);
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_next_token() -> CToken {
    to_c_token(tokenizer().lock().unwrap().next_token())
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_peek_token() -> CToken {
    to_c_token(tokenizer().lock().unwrap().peek_token())
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_reset() {
    tokenizer().lock().unwrap().reset();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tokenizer_load_text(text: *const c_char) -> c_int {
    if text.is_null() {
        return -1;
    }

    let text = CStr::from_ptr(text).to_bytes();
    let mut error = Vec::new();
    if tokenizer().lock().unwrap().load_text(text, &mut error) {
        0
    } else {
        write_stderr(&error);
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tokenizer_get_stats(
    lines: *mut usize,
    tokens: *mut usize,
    chars: *mut usize,
) {
    let (line_count, token_count, char_count) = tokenizer().lock().unwrap().stats();
    if let Some(lines) = lines.as_mut() {
        *lines = line_count;
    }
    if let Some(tokens) = tokens.as_mut() {
        *tokens = token_count;
    }
    if let Some(chars) = chars.as_mut() {
        *chars = char_count;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_tokenizer_ops() -> TokenizerOps {
    TokenizerOps {
        next_token: Some(tokenizer_next_token),
        peek_token: Some(tokenizer_peek_token),
        reset: Some(tokenizer_reset),
        load_text: Some(tokenizer_load_text),
        get_stats: Some(tokenizer_get_stats),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn analyzer_init(ops: TokenizerOps) {
    let mut state = analyzer().lock().unwrap();
    state.ops = ops;
    state.initialized = true;
    state.token_type_counts.fill(0);
    state.common_words.clear();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn analyze_text(text: *const c_char) -> AnalysisResult {
    let mut state = analyzer().lock().unwrap();
    let mut result = AnalysisResult::default();

    if !state.initialized {
        write_stderr(b"Error: Analyzer not initialized\n");
        return result;
    }

    if required(state.ops.load_text)(text) != 0 {
        write_stderr(b"Error: Failed to load text\n");
        return result;
    }

    loop {
        let token = required(state.ops.next_token)();
        if token.token_type == TokenType::Eof as c_int {
            break;
        }

        let index = token.token_type as usize;
        state.token_type_counts[index] = state.token_type_counts[index].wrapping_add(1);
        match token.token_type {
            value
                if value == TokenType::Word as c_int || value == TokenType::Identifier as c_int =>
            {
                result.word_count = result.word_count.wrapping_add(1);
                state.track_word(token_value(&token));
            }
            value if value == TokenType::Number as c_int => {
                result.number_count = result.number_count.wrapping_add(1)
            }
            value if value == TokenType::Keyword as c_int => {
                result.keyword_count = result.keyword_count.wrapping_add(1)
            }
            value if value == TokenType::Operator as c_int => {
                result.operator_count = result.operator_count.wrapping_add(1)
            }
            value if value == TokenType::Comment as c_int => {
                result.comment_count = result.comment_count.wrapping_add(1)
            }
            value if value == TokenType::String as c_int => {
                result.string_count = result.string_count.wrapping_add(1)
            }
            value if value == TokenType::Newline as c_int => {
                result.line_count = result.line_count.wrapping_add(1)
            }
            _ => {}
        }
    }

    let mut lines = 0;
    let mut tokens = 0;
    let mut chars = 0;
    required(state.ops.get_stats)(&mut lines, &mut tokens, &mut chars);
    result.line_count = lines;
    result.char_count = chars;
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn print_token_distribution() {
    let mut state = analyzer().lock().unwrap();
    let mut output = Vec::new();
    output.extend_from_slice(b"\n=== Token Distribution ===\n");

    const TOKEN_NAMES: [&[u8]; 12] = [
        b"EOF",
        b"WORD",
        b"NUMBER",
        b"PUNCTUATION",
        b"WHITESPACE",
        b"NEWLINE",
        b"IDENTIFIER",
        b"KEYWORD",
        b"OPERATOR",
        b"STRING",
        b"COMMENT",
        b"ERROR",
    ];
    for (index, name) in TOKEN_NAMES.iter().enumerate() {
        let count = state.token_type_counts[index];
        if count > 0 {
            output.extend_from_slice(name);
            output.extend_from_slice(format!(": {count}\n").as_bytes());
        }
    }

    output.extend_from_slice(b"\n=== Most Common Words ===\n");
    let len = state.common_words.len();
    for i in 0..len.saturating_sub(1) {
        for j in 0..len - i - 1 {
            if state.common_words[j].1 < state.common_words[j + 1].1 {
                state.common_words.swap(j, j + 1);
            }
        }
    }
    for (index, (word, count)) in state.common_words.iter().take(10).enumerate() {
        output.extend_from_slice(format!("{}. ", index + 1).as_bytes());
        output.extend_from_slice(word);
        output.extend_from_slice(format!(": {count} times\n").as_bytes());
    }
    write_stdout(&output);
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_complexity_score() -> c_int {
    let state = analyzer().lock().unwrap();
    let mut score = state.token_type_counts[TokenType::Keyword as usize].wrapping_mul(2);
    score = score.wrapping_add(state.token_type_counts[TokenType::Operator as usize]);
    score = score.wrapping_add(state.token_type_counts[TokenType::Punctuation as usize] / 10);
    score = score.wrapping_sub(state.token_type_counts[TokenType::Comment as usize]);
    score.max(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_patterns(pattern: *const c_char) {
    let state = analyzer().lock().unwrap();
    if !state.initialized || pattern.is_null() {
        return;
    }
    let pattern = CStr::from_ptr(pattern).to_bytes();
    let mut output = Vec::new();
    output.extend_from_slice(b"\n=== Searching for pattern: '");
    output.extend_from_slice(pattern);
    output.extend_from_slice(b"' ===\n");

    required(state.ops.reset)();
    let mut count = 0i32;
    loop {
        let token = required(state.ops.next_token)();
        if token.token_type == TokenType::Eof as c_int {
            break;
        }
        let value = token_value(&token);
        if pattern.is_empty()
            || value
                .windows(pattern.len())
                .any(|candidate| candidate == pattern)
        {
            output.extend_from_slice(
                format!("Line {}, Column {}: ", token.line, token.column).as_bytes(),
            );
            output.extend_from_slice(value);
            output.push(b'\n');
            count = count.wrapping_add(1);
        }
    }
    output.extend_from_slice(format!("Found {count} occurrences\n").as_bytes());
    write_stdout(&output);
}

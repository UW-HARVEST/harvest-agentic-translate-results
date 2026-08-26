use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use crate::tokenizer::*;

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
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<String>,
    common_word_counts: Vec<i32>,
}

impl AnalyzerState {
    fn new() -> Self {
        Self {
            initialized: false,
            token_type_counts: [0; 20],
            common_words: Vec::new(),
            common_word_counts: Vec::new(),
        }
    }
}

thread_local! {
    static ASTATE: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::new());
}

fn with_astate<F, R>(f: F) -> R
where
    F: FnOnce(&mut AnalyzerState) -> R,
{
    ASTATE.with(|s| f(&mut s.borrow_mut()))
}

pub fn analyzer_init_rs() {
    with_astate(|s| {
        s.initialized = true;
        s.token_type_counts = [0; 20];
        s.common_words.clear();
        s.common_word_counts.clear();
    });
}

fn track_word(a: &mut AnalyzerState, word: &str) {
    for i in 0..a.common_words.len() {
        if a.common_words[i] == word {
            a.common_word_counts[i] += 1;
            return;
        }
    }
    if a.common_words.len() < 100 {
        let mut w = String::from(word);
        w.truncate(MAX_TOKEN_LENGTH - 1);
        a.common_words.push(w);
        a.common_word_counts.push(1);
    }
}

pub fn analyze_text_rs(text: &str) -> AnalysisResult {
    let mut result = AnalysisResult {
        word_count: 0, number_count: 0, keyword_count: 0, operator_count: 0,
        comment_count: 0, string_count: 0, line_count: 0, char_count: 0,
    };

    let initialized = with_astate(|s| s.initialized);
    if !initialized {
        eprint!("Error: Analyzer not initialized\n");
        return result;
    }

    if tokenizer_load_text_rs(text) != 0 {
        eprint!("Error: Failed to load text\n");
        return result;
    }

    loop {
        let token = tokenizer_next_token_rs();
        if token.token_type == TokenType::Eof {
            break;
        }
        with_astate(|s| {
            s.token_type_counts[token.token_type as usize] += 1;
        });
        match token.token_type {
            TokenType::Word | TokenType::Identifier => {
                result.word_count += 1;
                with_astate(|s| track_word(s, &token.value));
            }
            TokenType::Number => result.number_count += 1,
            TokenType::Keyword => result.keyword_count += 1,
            TokenType::Operator => result.operator_count += 1,
            TokenType::Comment => result.comment_count += 1,
            TokenType::String => result.string_count += 1,
            TokenType::Newline => result.line_count += 1,
            _ => {}
        }
    }

    let (lines, _tokens, chars) = tokenizer_get_stats_rs();
    result.line_count = lines;
    result.char_count = chars;
    result
}

pub fn print_token_distribution_rs() {
    print!("\n=== Token Distribution ===\n");

    let token_names = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    with_astate(|s| {
        for i in 0..12 {
            if s.token_type_counts[i] > 0 {
                print!("{}: {}\n", token_names[i], s.token_type_counts[i]);
            }
        }

        print!("\n=== Most Common Words ===\n");

        // Bubble sort (matching C exactly)
        let n = s.common_words.len();
        for i in 0..n.saturating_sub(1) {
            for j in 0..n - i - 1 {
                if s.common_word_counts[j] < s.common_word_counts[j + 1] {
                    s.common_word_counts.swap(j, j + 1);
                    s.common_words.swap(j, j + 1);
                }
            }
        }

        let limit = if n < 10 { n } else { 10 };
        for i in 0..limit {
            print!("{}. {}: {} times\n", i + 1, s.common_words[i], s.common_word_counts[i]);
        }
    });
}

pub fn calculate_complexity_score_rs() -> i32 {
    with_astate(|s| {
        let mut score: i32 = 0;
        score += s.token_type_counts[TokenType::Keyword as usize] * 2;
        score += s.token_type_counts[TokenType::Operator as usize];
        score += s.token_type_counts[TokenType::Punctuation as usize] / 10;
        score -= s.token_type_counts[TokenType::Comment as usize];
        if score < 0 { score = 0; }
        score
    })
}

pub fn find_patterns_rs(pattern: &str) {
    let initialized = with_astate(|s| s.initialized);
    if !initialized || pattern.is_empty() {
        return;
    }

    print!("\n=== Searching for pattern: '{}' ===\n", pattern);

    tokenizer_reset_rs();

    let mut count = 0;
    loop {
        let token = tokenizer_next_token_rs();
        if token.token_type == TokenType::Eof {
            break;
        }
        if token.value.contains(pattern) {
            print!("Line {}, Column {}: {}\n", token.line, token.column, token.value);
            count += 1;
        }
    }
    print!("Found {} occurrences\n", count);
}

// C-compatible result struct
#[repr(C)]
pub struct CAnalysisResult {
    pub word_count: usize,
    pub number_count: usize,
    pub keyword_count: usize,
    pub operator_count: usize,
    pub comment_count: usize,
    pub string_count: usize,
    pub line_count: usize,
    pub char_count: usize,
}

// FFI exports

#[no_mangle]
pub extern "C" fn analyzer_init_ffi(_ops: CTokenizerOps) {
    analyzer_init_rs();
}

#[no_mangle]
pub extern "C" fn analyze_text_ffi(text: *const c_char) -> CAnalysisResult {
    if text.is_null() {
        return CAnalysisResult { word_count: 0, number_count: 0, keyword_count: 0, operator_count: 0, comment_count: 0, string_count: 0, line_count: 0, char_count: 0 };
    }
    let cstr = unsafe { CStr::from_ptr(text) };
    let s = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return CAnalysisResult { word_count: 0, number_count: 0, keyword_count: 0, operator_count: 0, comment_count: 0, string_count: 0, line_count: 0, char_count: 0 },
    };
    let r = analyze_text_rs(s);
    CAnalysisResult {
        word_count: r.word_count, number_count: r.number_count,
        keyword_count: r.keyword_count, operator_count: r.operator_count,
        comment_count: r.comment_count, string_count: r.string_count,
        line_count: r.line_count, char_count: r.char_count,
    }
}

#[no_mangle]
pub extern "C" fn print_token_distribution_ffi() {
    print_token_distribution_rs();
}

#[no_mangle]
pub extern "C" fn calculate_complexity_score_ffi() -> c_int {
    calculate_complexity_score_rs()
}

#[no_mangle]
pub extern "C" fn find_patterns_ffi(pattern: *const c_char) {
    if pattern.is_null() { return; }
    let cstr = unsafe { CStr::from_ptr(pattern) };
    if let Ok(s) = cstr.to_str() {
        find_patterns_rs(s);
    }
}

// Exact C-name exports

#[no_mangle]
pub extern "C" fn analyzer_init(ops: CTokenizerOps) { analyzer_init_ffi(ops) }

#[no_mangle]
pub extern "C" fn analyze_text(text: *const c_char) -> CAnalysisResult { analyze_text_ffi(text) }

#[no_mangle]
pub extern "C" fn print_token_distribution() { print_token_distribution_ffi() }

#[no_mangle]
pub extern "C" fn calculate_complexity_score() -> c_int { calculate_complexity_score_ffi() }

#[no_mangle]
pub extern "C" fn find_patterns(pattern: *const c_char) { find_patterns_ffi(pattern) }

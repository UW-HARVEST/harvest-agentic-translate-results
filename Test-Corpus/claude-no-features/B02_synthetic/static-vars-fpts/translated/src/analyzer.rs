// Translated from c_src/src/analyzer.c

use std::cell::RefCell;

use crate::tokenizer::{Token, TokenType, TokenizerOps, MAX_TOKEN_LENGTH};

#[derive(Default, Clone, Copy)]
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
    ops: Option<TokenizerOps>,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<Vec<u8>>,    // up to 100 entries
    common_word_counts: Vec<i32>,  // up to 100 entries
    num_common_words: usize,
}

impl AnalyzerState {
    fn new() -> Self {
        AnalyzerState {
            ops: None,
            initialized: false,
            token_type_counts: [0; 20],
            common_words: Vec::with_capacity(100),
            common_word_counts: Vec::with_capacity(100),
            num_common_words: 0,
        }
    }
}

thread_local! {
    static STATE: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::new());
}

pub fn analyzer_init(ops: TokenizerOps) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.ops = Some(ops);
        s.initialized = true;
        for v in s.token_type_counts.iter_mut() {
            *v = 0;
        }
        s.common_words.clear();
        s.common_word_counts.clear();
        s.num_common_words = 0;
    });
}

fn track_word(word: &[u8]) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        // Find existing
        for i in 0..s.num_common_words {
            if s.common_words[i] == word {
                s.common_word_counts[i] += 1;
                return;
            }
        }
        if s.num_common_words < 100 {
            // Truncate to MAX_TOKEN_LENGTH - 1 like strncpy with explicit null
            let copy_len = word.len().min(MAX_TOKEN_LENGTH - 1);
            let stored = word[..copy_len].to_vec();
            s.common_words.push(stored);
            s.common_word_counts.push(1);
            s.num_common_words += 1;
        }
    });
}

pub fn analyze_text(text: &[u8]) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let (initialized, ops_opt) = STATE.with(|s| {
        let s = s.borrow();
        (s.initialized, s.ops)
    });

    if !initialized {
        eprintln!("Error: Analyzer not initialized");
        return result;
    }

    let ops = ops_opt.expect("ops must be set when initialized");

    if (ops.load_text)(text) != 0 {
        eprintln!("Error: Failed to load text");
        return result;
    }

    loop {
        let token: Token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            let idx = token.token_type as usize;
            if idx < s.token_type_counts.len() {
                s.token_type_counts[idx] += 1;
            }
        });

        match token.token_type {
            TokenType::Word | TokenType::Identifier => {
                result.word_count += 1;
                track_word(&token.value);
            }
            TokenType::Number => {
                result.number_count += 1;
            }
            TokenType::Keyword => {
                result.keyword_count += 1;
            }
            TokenType::Operator => {
                result.operator_count += 1;
            }
            TokenType::Comment => {
                result.comment_count += 1;
            }
            TokenType::String => {
                result.string_count += 1;
            }
            TokenType::Newline => {
                result.line_count += 1;
            }
            _ => {}
        }
    }

    let (lines, _tokens, chars) = (ops.get_stats)();
    result.line_count = lines;
    result.char_count = chars;

    result
}

pub fn print_token_distribution() {
    crate::out_write(b"\n=== Token Distribution ===\n");

    let token_names: [&str; 12] = [
        "EOF",
        "WORD",
        "NUMBER",
        "PUNCTUATION",
        "WHITESPACE",
        "NEWLINE",
        "IDENTIFIER",
        "KEYWORD",
        "OPERATOR",
        "STRING",
        "COMMENT",
        "ERROR",
    ];

    let counts = STATE.with(|s| s.borrow().token_type_counts);

    for i in 0..12 {
        if counts[i] > 0 {
            crate::out_write(format!("{}: {}\n", token_names[i], counts[i]).as_bytes());
        }
    }

    crate::out_write(b"\n=== Most Common Words ===\n");

    // Sort words and counts together with bubble sort (descending by count)
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let n = s.num_common_words;
        if n >= 2 {
            for i in 0..(n - 1) {
                for j in 0..(n - i - 1) {
                    if s.common_word_counts[j] < s.common_word_counts[j + 1] {
                        s.common_word_counts.swap(j, j + 1);
                        s.common_words.swap(j, j + 1);
                    }
                }
            }
        }
    });

    let (words_snapshot, counts_snapshot, num) = STATE.with(|s| {
        let s = s.borrow();
        (
            s.common_words.clone(),
            s.common_word_counts.clone(),
            s.num_common_words,
        )
    });

    let limit = if num < 10 { num } else { 10 };
    for i in 0..limit {
        crate::out_write(format!("{}. ", i + 1).as_bytes());
        crate::out_write(&words_snapshot[i]);
        crate::out_write(format!(": {} times\n", counts_snapshot[i]).as_bytes());
    }
}

pub fn calculate_complexity_score() -> i32 {
    let counts = STATE.with(|s| s.borrow().token_type_counts);

    let mut score: i32 = 0;
    score += counts[TokenType::Keyword as usize] * 2;
    score += counts[TokenType::Operator as usize];
    score += counts[TokenType::Punctuation as usize] / 10;
    score -= counts[TokenType::Comment as usize];
    if score < 0 {
        score = 0;
    }
    score
}

pub fn find_patterns(pattern: &[u8]) {
    let (initialized, ops_opt) = STATE.with(|s| {
        let s = s.borrow();
        (s.initialized, s.ops)
    });

    if !initialized {
        return;
    }

    crate::out_write(b"\n=== Searching for pattern: '");
    crate::out_write(pattern);
    crate::out_write(b"' ===\n");

    let ops = ops_opt.expect("ops must be set when initialized");
    (ops.reset)();

    let mut count: i32 = 0;
    loop {
        let token: Token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        if contains_subslice(&token.value, pattern) {
            crate::out_write(
                format!("Line {}, Column {}: ", token.line, token.column).as_bytes(),
            );
            crate::out_write(&token.value);
            crate::out_write(b"\n");
            count += 1;
        }
    }

    crate::out_write(format!("Found {} occurrences\n", count).as_bytes());
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    // strstr behavior: empty needle matches at position 0 -> true
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == needle {
            return true;
        }
    }
    false
}

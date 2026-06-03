use std::sync::Mutex;

use crate::tokenizer::{TokenType, TokenizerOps};

#[derive(Default, Copy, Clone, Debug)]
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
    tokenizer_ops: Option<TokenizerOps>,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<String>,
    common_word_counts: Vec<i32>,
}

impl AnalyzerState {
    const fn new() -> Self {
        AnalyzerState {
            tokenizer_ops: None,
            initialized: false,
            token_type_counts: [0; 20],
            common_words: Vec::new(),
            common_word_counts: Vec::new(),
        }
    }
}

static STATE: Mutex<AnalyzerState> = Mutex::new(AnalyzerState::new());

pub fn analyzer_init(ops: TokenizerOps) {
    let mut state = STATE.lock().unwrap();
    state.tokenizer_ops = Some(ops);
    state.initialized = true;
    state.token_type_counts = [0; 20];
    state.common_words.clear();
    state.common_word_counts.clear();
}

fn track_word(state: &mut AnalyzerState, word: &str) {
    for i in 0..state.common_words.len() {
        if state.common_words[i] == word {
            state.common_word_counts[i] += 1;
            return;
        }
    }
    if state.common_words.len() < 100 {
        state.common_words.push(word.to_string());
        state.common_word_counts.push(1);
    }
}

pub fn analyze_text(text: &str) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let ops = {
        let state = STATE.lock().unwrap();
        if !state.initialized {
            eprintln!("Error: Analyzer not initialized");
            return result;
        }
        state.tokenizer_ops.unwrap()
    };

    if (ops.load_text)(text) != 0 {
        eprintln!("Error: Failed to load text");
        return result;
    }

    loop {
        let token = (ops.next_token)();
        if token.type_ == TokenType::Eof {
            break;
        }

        {
            let mut state = STATE.lock().unwrap();
            let idx = token.type_ as usize;
            if idx < state.token_type_counts.len() {
                state.token_type_counts[idx] += 1;
            }

            match token.type_ {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    track_word(&mut state, &token.value);
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
    }

    let (lines, _tokens, chars) = (ops.get_stats)();
    result.line_count = lines;
    result.char_count = chars;

    result
}

pub fn print_token_distribution() {
    println!("\n=== Token Distribution ===");

    let token_names = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE", "NEWLINE",
        "IDENTIFIER", "KEYWORD", "OPERATOR", "STRING", "COMMENT", "ERROR",
    ];

    let counts;
    {
        let state = STATE.lock().unwrap();
        counts = state.token_type_counts;
    }

    for i in 0..12 {
        if counts[i] > 0 {
            println!("{}: {}", token_names[i], counts[i]);
        }
    }

    println!("\n=== Most Common Words ===");

    let mut state = STATE.lock().unwrap();
    let n = state.common_words.len();
    if n > 1 {
        for i in 0..(n - 1) {
            for j in 0..(n - i - 1) {
                if state.common_word_counts[j] < state.common_word_counts[j + 1] {
                    state.common_word_counts.swap(j, j + 1);
                    state.common_words.swap(j, j + 1);
                }
            }
        }
    }

    let limit = if n < 10 { n } else { 10 };
    for i in 0..limit {
        println!(
            "{}. {}: {} times",
            i + 1,
            state.common_words[i],
            state.common_word_counts[i]
        );
    }
}

pub fn calculate_complexity_score() -> i32 {
    let state = STATE.lock().unwrap();
    let mut score: i32 = 0;
    score += state.token_type_counts[TokenType::Keyword as usize] * 2;
    score += state.token_type_counts[TokenType::Operator as usize];
    score += state.token_type_counts[TokenType::Punctuation as usize] / 10;
    score -= state.token_type_counts[TokenType::Comment as usize];
    if score < 0 {
        score = 0;
    }
    score
}

pub fn find_patterns(pattern: &str) {
    let ops = {
        let state = STATE.lock().unwrap();
        if !state.initialized {
            return;
        }
        state.tokenizer_ops.unwrap()
    };

    if pattern.is_empty() {
        // Mirror C behavior: NULL pattern returns; empty string still proceeds.
        // (We treat empty similarly to allow it through.)
    }

    println!("\n=== Searching for pattern: '{}' ===", pattern);

    (ops.reset)();

    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.type_ == TokenType::Eof {
            break;
        }
        if token.value.contains(pattern) {
            println!(
                "Line {}, Column {}: {}",
                token.line, token.column, token.value
            );
            count += 1;
        }
    }

    println!("Found {} occurrences", count);
}

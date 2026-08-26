use std::sync::{Mutex, OnceLock};

use crate::tokenizer::{TokenType, TokenizerOps, MAX_TOKEN_LENGTH};

#[derive(Clone, Copy, Debug, Default)]
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
    fn new() -> Self {
        Self {
            tokenizer_ops: None,
            initialized: false,
            token_type_counts: [0; 20],
            common_words: Vec::new(),
            common_word_counts: Vec::new(),
        }
    }

    fn track_word(&mut self, word: &str) {
        if let Some(pos) = self.common_words.iter().position(|w| w == word) {
            self.common_word_counts[pos] += 1;
            return;
        }
        if self.common_words.len() < 100 {
            let truncated: String = word.chars().take(MAX_TOKEN_LENGTH - 1).collect();
            self.common_words.push(truncated);
            self.common_word_counts.push(1);
        }
    }
}

fn state() -> &'static Mutex<AnalyzerState> {
    static STATE: OnceLock<Mutex<AnalyzerState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AnalyzerState::new()))
}

pub fn analyzer_init(ops: TokenizerOps) {
    let mut s = state().lock().unwrap();
    s.tokenizer_ops = Some(ops);
    s.initialized = true;
    s.token_type_counts = [0; 20];
    s.common_word_counts.fill(0);
    s.common_words.clear();
    s.common_word_counts.clear();
}

pub fn analyze_text(text: &str) -> AnalysisResult {
    let ops = {
        let s = state().lock().unwrap();
        if !s.initialized {
            eprintln!("Error: Analyzer not initialized");
            return AnalysisResult::default();
        }
        s.tokenizer_ops.unwrap()
    };

    if (ops.load_text)(text) != 0 {
        eprintln!("Error: Failed to load text");
        return AnalysisResult::default();
    }

    let mut result = AnalysisResult::default();

    loop {
        let token = (ops.next_token)();
        if token.type_ == TokenType::TokenEof {
            break;
        }

        let mut s = state().lock().unwrap();
        let idx = token.type_ as usize;
        if idx < s.token_type_counts.len() {
            s.token_type_counts[idx] += 1;
        }

        match token.type_ {
            TokenType::TokenWord | TokenType::TokenIdentifier => {
                result.word_count += 1;
                s.track_word(&token.value);
            }
            TokenType::TokenNumber => result.number_count += 1,
            TokenType::TokenKeyword => result.keyword_count += 1,
            TokenType::TokenOperator => result.operator_count += 1,
            TokenType::TokenComment => result.comment_count += 1,
            TokenType::TokenString => result.string_count += 1,
            TokenType::TokenNewline => result.line_count += 1,
            _ => {}
        }
    }

    let mut lines = 0;
    let mut tokens = 0;
    let mut chars = 0;
    (ops.get_stats)(&mut lines, &mut tokens, &mut chars);
    let _ = tokens;
    result.line_count = lines;
    result.char_count = chars;
    result
}

pub fn print_token_distribution() {
    let mut s = state().lock().unwrap();
    println!("\n=== Token Distribution ===");
    let token_names = [
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
    for (i, name) in token_names.iter().enumerate() {
        if s.token_type_counts[i] > 0 {
            println!("{}: {}", name, s.token_type_counts[i]);
        }
    }
    println!("\n=== Most Common Words ===");
    let mut combined: Vec<(String, i32)> = s
        .common_words
        .iter()
        .cloned()
        .zip(s.common_word_counts.iter().copied())
        .collect();
    combined.sort_by(|a, b| b.1.cmp(&a.1));
    for (i, (word, count)) in combined.into_iter().take(10).enumerate() {
        println!("{}. {}: {} times", i + 1, word, count);
    }
}

pub fn calculate_complexity_score() -> i32 {
    let s = state().lock().unwrap();
    let mut score = 0;
    score += s.token_type_counts[TokenType::TokenKeyword as usize] * 2;
    score += s.token_type_counts[TokenType::TokenOperator as usize];
    score += s.token_type_counts[TokenType::TokenPunctuation as usize] / 10;
    score -= s.token_type_counts[TokenType::TokenComment as usize];
    score.max(0)
}

pub fn find_patterns(pattern: &str) {
    let ops = {
        let s = state().lock().unwrap();
        if !s.initialized || pattern.is_empty() {
            return;
        }
        s.tokenizer_ops.unwrap()
    };

    println!("\n=== Searching for pattern: '{}' ===", pattern);
    (ops.reset)();
    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.type_ == TokenType::TokenEof {
            break;
        }
        if token.value.contains(pattern) {
            println!("Line {}, Column {}: {}", token.line, token.column, token.value);
            count += 1;
        }
    }
    println!("Found {} occurrences", count);
}

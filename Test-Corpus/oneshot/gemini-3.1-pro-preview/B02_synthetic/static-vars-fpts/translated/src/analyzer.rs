use crate::tokenizer::{TokenizerOps, TokenType};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default, Clone, Debug)]
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
    token_type_counts: HashMap<TokenType, usize>,
    common_words: Vec<String>,
    common_word_counts: Vec<usize>,
}

impl Default for AnalyzerState {
    fn default() -> Self {
        Self {
            tokenizer_ops: None,
            initialized: false,
            token_type_counts: HashMap::new(),
            common_words: Vec::new(),
            common_word_counts: Vec::new(),
        }
    }
}

thread_local! {
    static ANALYZER_STATE: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::default());
}

pub fn analyzer_init(ops: TokenizerOps) {
    ANALYZER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.tokenizer_ops = Some(ops);
        state.initialized = true;
        state.token_type_counts.clear();
        state.common_words.clear();
        state.common_word_counts.clear();
    });
}

fn track_word(state: &mut AnalyzerState, word: &str) {
    if let Some(idx) = state.common_words.iter().position(|w| w == word) {
        state.common_word_counts[idx] += 1;
    } else if state.common_words.len() < 100 {
        state.common_words.push(word.to_string());
        state.common_word_counts.push(1);
    }
}

pub fn analyze_text(text: &str) -> AnalysisResult {
    ANALYZER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let mut result = AnalysisResult::default();

        if !state.initialized {
            eprintln!("Error: Analyzer not initialized");
            return result;
        }

        let ops = state.tokenizer_ops.clone().unwrap();
        if (ops.load_text)(text).is_err() {
            eprintln!("Error: Failed to load text");
            return result;
        }

        loop {
            let token = (ops.next_token)();
            if token.token_type == TokenType::Eof {
                break;
            }

            *state.token_type_counts.entry(token.token_type).or_insert(0) += 1;

            match token.token_type {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    track_word(&mut state, &token.value);
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

        let (lines, _tokens, chars) = (ops.get_stats)();
        result.line_count = lines;
        result.char_count = chars;

        result
    })
}

pub fn print_token_distribution() {
    ANALYZER_STATE.with(|state| {
        let state = state.borrow();
        println!("\n=== Token Distribution ===");

        let token_names = [
            (TokenType::Eof, "EOF"),
            (TokenType::Word, "WORD"),
            (TokenType::Number, "NUMBER"),
            (TokenType::Punctuation, "PUNCTUATION"),
            (TokenType::Whitespace, "WHITESPACE"),
            (TokenType::Newline, "NEWLINE"),
            (TokenType::Identifier, "IDENTIFIER"),
            (TokenType::Keyword, "KEYWORD"),
            (TokenType::Operator, "OPERATOR"),
            (TokenType::String, "STRING"),
            (TokenType::Comment, "COMMENT"),
            (TokenType::Error, "ERROR"),
        ];

        for (tt, name) in &token_names {
            if let Some(&count) = state.token_type_counts.get(tt) {
                if count > 0 {
                    println!("{}: {}", name, count);
                }
            }
        }

        println!("\n=== Most Common Words ===");

        let mut combined: Vec<_> = state.common_words.iter().zip(state.common_word_counts.iter()).collect();
        combined.sort_by(|a, b| b.1.cmp(a.1));

        let limit = combined.len().min(10);
        for i in 0..limit {
            println!("{}. {}: {} times", i + 1, combined[i].0, combined[i].1);
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
    ANALYZER_STATE.with(|state| {
        let state = state.borrow();
        let mut score = 0;

        score += (*state.token_type_counts.get(&TokenType::Keyword).unwrap_or(&0) as i32) * 2;
        score += *state.token_type_counts.get(&TokenType::Operator).unwrap_or(&0) as i32;
        score += (*state.token_type_counts.get(&TokenType::Punctuation).unwrap_or(&0) as i32) / 10;
        score -= *state.token_type_counts.get(&TokenType::Comment).unwrap_or(&0) as i32;

        if score < 0 {
            score = 0;
        }
        score
    })
}

pub fn find_patterns(pattern: &str) {
    ANALYZER_STATE.with(|state| {
        let state = state.borrow();
        if !state.initialized || pattern.is_empty() {
            return;
        }

        println!("\n=== Searching for pattern: '{}' ===", pattern);

        let ops = state.tokenizer_ops.as_ref().unwrap();
        (ops.reset)();

        let mut count = 0;
        loop {
            let token = (ops.next_token)();
            if token.token_type == TokenType::Eof {
                break;
            }

            if token.value.contains(pattern) {
                println!("Line {}, Column {}: {}", token.line, token.column, token.value);
                count += 1;
            }
        }

        println!("Found {} occurrences", count);
    });
}

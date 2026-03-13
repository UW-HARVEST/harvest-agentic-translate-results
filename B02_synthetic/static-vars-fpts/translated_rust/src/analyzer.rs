use std::cell::RefCell;
use crate::tokenizer::*;

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
}

thread_local! {
    static ASTATE: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::new());
}

fn with_astate<R>(f: impl FnOnce(&mut AnalyzerState) -> R) -> R {
    ASTATE.with(|s| f(&mut s.borrow_mut()))
}

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

impl AnalysisResult {
    fn new() -> Self {
        Self {
            word_count: 0,
            number_count: 0,
            keyword_count: 0,
            operator_count: 0,
            comment_count: 0,
            string_count: 0,
            line_count: 0,
            char_count: 0,
        }
    }
}

pub fn analyzer_init(ops: TokenizerOps) {
    with_astate(|s| {
        s.tokenizer_ops = Some(ops);
        s.initialized = true;
        s.token_type_counts = [0; 20];
        s.common_words.clear();
        s.common_word_counts.clear();
    });
}

fn track_word(s: &mut AnalyzerState, word: &str) {
    for i in 0..s.common_words.len() {
        if s.common_words[i] == word {
            s.common_word_counts[i] += 1;
            return;
        }
    }
    if s.common_words.len() < 100 {
        let mut w = String::from(word);
        w.truncate(MAX_TOKEN_LENGTH - 1);
        s.common_words.push(w);
        s.common_word_counts.push(1);
    }
}

pub fn analyze_text(text: &str) -> AnalysisResult {
    let mut result = AnalysisResult::new();
    let (initialized, ops_load, ops_next, ops_get_stats) = with_astate(|s| {
        if !s.initialized {
            return (false, None, None, None);
        }
        let ops = s.tokenizer_ops.as_ref().unwrap();
        (true, Some(ops.load_text), Some(ops.next_token), Some(ops.get_stats))
    });
    if !initialized {
        eprint!("Error: Analyzer not initialized\n");
        return result;
    }
    let load_text = ops_load.unwrap();
    let next_token = ops_next.unwrap();
    let get_stats = ops_get_stats.unwrap();
    if load_text(text) != 0 {
        eprint!("Error: Failed to load text\n");
        return result;
    }
    loop {
        let token = next_token();
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
    let mut lines: usize = 0;
    let mut tokens: usize = 0;
    let mut chars: usize = 0;
    get_stats(&mut lines, &mut tokens, &mut chars);
    result.line_count = lines;
    result.char_count = chars;
    result
}

pub fn print_token_distribution() {
    println!("\n=== Token Distribution ===");
    let token_names = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];
    with_astate(|s| {
        for i in 0..12 {
            if s.token_type_counts[i] > 0 {
                println!("{}: {}", token_names[i], s.token_type_counts[i]);
            }
        }
        println!("\n=== Most Common Words ===");
        // Bubble sort
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
            println!("{}. {}: {} times", i + 1, s.common_words[i], s.common_word_counts[i]);
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
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

pub fn find_patterns(pattern: &str) {
    let (initialized, ops_reset, ops_next) = with_astate(|s| {
        if !s.initialized {
            return (false, None, None);
        }
        let ops = s.tokenizer_ops.as_ref().unwrap();
        (true, Some(ops.reset), Some(ops.next_token))
    });
    if !initialized || pattern.is_empty() {
        return;
    }
    let reset = ops_reset.unwrap();
    let next_token = ops_next.unwrap();
    println!("\n=== Searching for pattern: '{}' ===", pattern);
    reset();
    let mut count = 0;
    loop {
        let token = next_token();
        if token.token_type == TokenType::Eof {
            break;
        }
        if token.value.contains(pattern) {
            println!("Line {}, Column {}: {}", token.line, token.column, token.value);
            count += 1;
        }
    }
    println!("Found {} occurrences", count);
}

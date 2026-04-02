use std::sync::Mutex;

use crate::tokenizer::*;

struct AnalyzerState {
    tokenizer_ops: Option<TokenizerOps>,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<String>,
    common_word_counts: Vec<i32>,
    num_common_words: usize,
}

static ANALYZER: Mutex<Option<AnalyzerState>> = Mutex::new(None);

fn init_analyzer_state() {
    let mut guard = ANALYZER.lock().unwrap();
    if guard.is_none() {
        *guard = Some(AnalyzerState {
            tokenizer_ops: None,
            initialized: false,
            token_type_counts: [0; 20],
            common_words: vec![String::new(); 100],
            common_word_counts: vec![0; 100],
            num_common_words: 0,
        });
    }
}

pub fn analyzer_init(ops: TokenizerOps) {
    init_analyzer_state();
    let mut guard = ANALYZER.lock().unwrap();
    let s = guard.as_mut().unwrap();
    s.tokenizer_ops = Some(ops);
    s.initialized = true;
    s.token_type_counts = [0; 20];
    s.common_word_counts = vec![0; 100];
    s.num_common_words = 0;
}

fn track_word(s: &mut AnalyzerState, word: &str) {
    for i in 0..s.num_common_words {
        if s.common_words[i] == word {
            s.common_word_counts[i] += 1;
            return;
        }
    }
    if s.num_common_words < 100 {
        let mut w = String::from(word);
        w.truncate(MAX_TOKEN_LENGTH - 1);
        s.common_words[s.num_common_words] = w;
        s.common_word_counts[s.num_common_words] = 1;
        s.num_common_words += 1;
    }
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

pub fn analyze_text(text: &str) -> AnalysisResult {
    init_analyzer_state();
    let mut result = AnalysisResult {
        word_count: 0,
        number_count: 0,
        keyword_count: 0,
        operator_count: 0,
        comment_count: 0,
        string_count: 0,
        line_count: 0,
        char_count: 0,
    };

    let ops = {
        let guard = ANALYZER.lock().unwrap();
        let s = guard.as_ref().unwrap();
        if !s.initialized {
            eprint!("Error: Analyzer not initialized\n");
            return result;
        }
        s.tokenizer_ops.clone().unwrap()
    };

    if (ops.load_text)(text) != 0 {
        eprint!("Error: Failed to load text\n");
        return result;
    }

    loop {
        let token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        {
            let mut guard = ANALYZER.lock().unwrap();
            let s = guard.as_mut().unwrap();
            s.token_type_counts[token.token_type as usize] += 1;
            match token.token_type {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    track_word(s, &token.value);
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
    }

    let mut lines: usize = 0;
    let mut tokens: usize = 0;
    let mut chars: usize = 0;
    (ops.get_stats)(&mut lines, &mut tokens, &mut chars);

    result.line_count = lines;
    result.char_count = chars;

    result
}

pub fn print_token_distribution() {
    init_analyzer_state();
    let mut guard = ANALYZER.lock().unwrap();
    let s = guard.as_mut().unwrap();

    print!("\n=== Token Distribution ===\n");

    let token_names = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    for i in 0..12 {
        if s.token_type_counts[i] > 0 {
            print!("{}: {}\n", token_names[i], s.token_type_counts[i]);
        }
    }

    print!("\n=== Most Common Words ===\n");

    // Bubble sort
    let n = s.num_common_words;
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
        print!(
            "{}. {}: {} times\n",
            i + 1,
            s.common_words[i],
            s.common_word_counts[i]
        );
    }
}

pub fn calculate_complexity_score() -> i32 {
    init_analyzer_state();
    let guard = ANALYZER.lock().unwrap();
    let s = guard.as_ref().unwrap();

    let mut score: i32 = 0;
    score += s.token_type_counts[TokenType::Keyword as usize] * 2;
    score += s.token_type_counts[TokenType::Operator as usize];
    score += s.token_type_counts[TokenType::Punctuation as usize] / 10;
    score -= s.token_type_counts[TokenType::Comment as usize];
    if score < 0 {
        score = 0;
    }
    score
}

pub fn find_patterns(pattern: &str) {
    init_analyzer_state();

    let ops = {
        let guard = ANALYZER.lock().unwrap();
        let s = guard.as_ref().unwrap();
        if !s.initialized {
            return;
        }
        s.tokenizer_ops.clone().unwrap()
    };

    print!("\n=== Searching for pattern: '{}' ===\n", pattern);

    (ops.reset)();

    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        if token.value.contains(pattern) {
            print!(
                "Line {}, Column {}: {}\n",
                token.line, token.column, token.value
            );
            count += 1;
        }
    }

    print!("Found {} occurrences\n", count);
}

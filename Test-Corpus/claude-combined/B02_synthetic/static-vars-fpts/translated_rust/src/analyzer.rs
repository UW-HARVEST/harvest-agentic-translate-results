// Rust translation of c_src/src/analyzer.c

use std::cell::RefCell;
use std::io::Write;

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

pub struct AnalyzerState {
    pub ops: Option<TokenizerOps>,
    pub initialized: bool,
    pub token_type_counts: [i32; 20],
    pub common_words: Vec<Vec<u8>>, // up to 100 entries, each up to MAX_TOKEN_LENGTH-1 bytes
    pub common_word_counts: Vec<i32>,
    pub num_common_words: i32,
}

impl AnalyzerState {
    fn new() -> Self {
        AnalyzerState {
            ops: None,
            initialized: false,
            token_type_counts: [0i32; 20],
            common_words: vec![Vec::new(); 100],
            common_word_counts: vec![0i32; 100],
            num_common_words: 0,
        }
    }
}

thread_local! {
    static ANALYZER: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::new());
}

pub fn analyzer_init(ops: TokenizerOps) {
    ANALYZER.with(|a| {
        let mut a = a.borrow_mut();
        a.ops = Some(ops);
        a.initialized = true;
        for v in a.token_type_counts.iter_mut() {
            *v = 0;
        }
        for v in a.common_word_counts.iter_mut() {
            *v = 0;
        }
        a.num_common_words = 0;
    });
}

fn track_word(a: &mut AnalyzerState, word: &[u8]) {
    // Find existing word
    for i in 0..a.num_common_words as usize {
        if a.common_words[i] == word {
            a.common_word_counts[i] += 1;
            return;
        }
    }
    if a.num_common_words < 100 {
        let idx = a.num_common_words as usize;
        // strncpy with MAX_TOKEN_LENGTH - 1 truncation
        let copy_len = word.len().min(MAX_TOKEN_LENGTH - 1);
        a.common_words[idx] = word[..copy_len].to_vec();
        a.common_word_counts[idx] = 1;
        a.num_common_words += 1;
    }
}

pub fn analyze_text(text: &[u8]) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let (initialized, ops) = ANALYZER.with(|a| {
        let a = a.borrow();
        (a.initialized, a.ops)
    });

    if !initialized {
        let _ = writeln!(std::io::stderr(), "Error: Analyzer not initialized");
        return result;
    }
    let ops = ops.unwrap();

    if (ops.load_text)(text) != 0 {
        let _ = writeln!(std::io::stderr(), "Error: Failed to load text");
        return result;
    }

    loop {
        let token: Token = (ops.next_token)();
        if token.ty == TokenType::Eof {
            break;
        }

        ANALYZER.with(|a| {
            let mut a = a.borrow_mut();
            let idx = token.ty as usize;
            if idx < a.token_type_counts.len() {
                a.token_type_counts[idx] += 1;
            }
            match token.ty {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    track_word(&mut a, &token.value);
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
        });
    }

    let (lines, _tokens, chars) = (ops.get_stats)();
    result.line_count = lines;
    result.char_count = chars;
    result
}

pub fn print_token_distribution() {
    print!("\n=== Token Distribution ===\n");

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

    ANALYZER.with(|a| {
        let counts = a.borrow().token_type_counts;
        for i in 0..12 {
            if counts[i] > 0 {
                print!("{}: {}\n", token_names[i], counts[i]);
            }
        }
    });

    print!("\n=== Most Common Words ===\n");

    // Bubble-sort-and-print
    ANALYZER.with(|a| {
        let mut a = a.borrow_mut();
        let n = a.num_common_words;
        // Reproduce the "for (i = 0; i < num - 1; i++)" loop including the
        // case where n == 0 — in C with int n=0 we get 0 < -1 false, fine.
        // With usize that would underflow, so guard explicitly.
        if n > 0 {
            for i in 0..(n - 1) as usize {
                let bound = (n - 1) as usize - i;
                for j in 0..bound {
                    if a.common_word_counts[j] < a.common_word_counts[j + 1] {
                        a.common_word_counts.swap(j, j + 1);
                        a.common_words.swap(j, j + 1);
                    }
                }
            }
        }

        let limit = if a.num_common_words < 10 {
            a.num_common_words
        } else {
            10
        };
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        for i in 0..limit as usize {
            // Print "i+1. word: count times\n" matching `printf("%d. %s: %d times\n", ...)`
            let _ = out.write_all(format!("{}. ", i + 1).as_bytes());
            let _ = out.write_all(&a.common_words[i]);
            let _ = out.write_all(format!(": {} times\n", a.common_word_counts[i]).as_bytes());
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
    ANALYZER.with(|a| {
        let counts = a.borrow().token_type_counts;
        let mut score: i32 = 0;
        score += counts[TokenType::Keyword as usize] * 2;
        score += counts[TokenType::Operator as usize];
        score += counts[TokenType::Punctuation as usize] / 10;
        score -= counts[TokenType::Comment as usize];
        if score < 0 {
            score = 0;
        }
        score
    })
}

pub fn find_patterns(pattern: &[u8]) {
    let (initialized, ops) = ANALYZER.with(|a| {
        let a = a.borrow();
        (a.initialized, a.ops)
    });
    if !initialized {
        return;
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\n=== Searching for pattern: '");
    let _ = out.write_all(pattern);
    let _ = out.write_all(b"' ===\n");
    drop(out);

    let ops = ops.unwrap();
    (ops.reset)();

    let mut count: i32 = 0;
    loop {
        let token: Token = (ops.next_token)();
        if token.ty == TokenType::Eof {
            break;
        }
        if contains(&token.value, pattern) {
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            let _ = out.write_all(format!("Line {}, Column {}: ", token.line, token.column).as_bytes());
            let _ = out.write_all(&token.value);
            let _ = out.write_all(b"\n");
            count += 1;
        }
    }
    print!("Found {} occurrences\n", count);
}

/// Mirrors C strstr(haystack, needle) != NULL — but on null-terminated strings.
/// In C, both haystack and needle are null-terminated. If pattern is empty
/// (i.e. needle == ""), strstr returns haystack (so always matches).
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
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

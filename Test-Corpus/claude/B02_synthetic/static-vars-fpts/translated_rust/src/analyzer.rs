// Rust translation of c_src/src/analyzer.c

use std::cell::RefCell;
use std::io::Write;

use crate::tokenizer::{TokenType, TokenizerOps, MAX_TOKEN_LENGTH};

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
    tokenizer_ops: Option<TokenizerOps>,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<Vec<u8>>, // up to 100 entries; each is null-terminated like C
    common_word_counts: [i32; 100],
    num_common_words: usize,
}

impl AnalyzerState {
    fn new() -> Self {
        let mut common_words = Vec::with_capacity(100);
        for _ in 0..100 {
            common_words.push(vec![0u8; MAX_TOKEN_LENGTH]);
        }
        AnalyzerState {
            tokenizer_ops: None,
            initialized: false,
            token_type_counts: [0; 20],
            common_words,
            common_word_counts: [0; 100],
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
        a.tokenizer_ops = Some(ops);
        a.initialized = true;
        a.token_type_counts = [0; 20];
        a.common_word_counts = [0; 100];
        a.num_common_words = 0;
    });
}

fn cstr_eq(a: &[u8], b: &[u8]) -> bool {
    let alen = a.iter().position(|&c| c == 0).unwrap_or(a.len());
    let blen = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    a[..alen] == b[..blen]
}

fn track_word(state: &mut AnalyzerState, word: &[u8]) {
    // Find existing.
    for i in 0..state.num_common_words {
        if cstr_eq(&state.common_words[i], word) {
            state.common_word_counts[i] += 1;
            return;
        }
    }
    if state.num_common_words < 100 {
        let idx = state.num_common_words;
        // Mirror strncpy(dest, src, MAX_TOKEN_LENGTH - 1) followed by null-termination.
        let dst = &mut state.common_words[idx];
        for b in dst.iter_mut() {
            *b = 0;
        }
        let src_len = word.iter().position(|&c| c == 0).unwrap_or(word.len());
        let copy_n = src_len.min(MAX_TOKEN_LENGTH - 1);
        dst[..copy_n].copy_from_slice(&word[..copy_n]);
        dst[MAX_TOKEN_LENGTH - 1] = 0;
        state.common_word_counts[idx] = 1;
        state.num_common_words += 1;
    }
}

pub fn analyze_text(text: &[u8]) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let ops = ANALYZER.with(|a| {
        let a = a.borrow();
        if !a.initialized {
            None
        } else {
            a.tokenizer_ops
        }
    });

    let ops = match ops {
        None => {
            eprintln!("Error: Analyzer not initialized");
            return result;
        }
        Some(o) => o,
    };

    // Load text using function pointer.
    if (ops.load_text)(text) != 0 {
        eprintln!("Error: Failed to load text");
        return result;
    }

    loop {
        let token = (ops.next_token)();
        if token.ttype == TokenType::Eof {
            break;
        }

        ANALYZER.with(|a| {
            let mut a = a.borrow_mut();
            let idx = token.ttype as usize;
            if idx < a.token_type_counts.len() {
                a.token_type_counts[idx] += 1;
            }
        });

        match token.ttype {
            TokenType::Word | TokenType::Identifier => {
                result.word_count += 1;
                ANALYZER.with(|a| {
                    let mut a = a.borrow_mut();
                    track_word(&mut a, &token.value);
                });
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
}

pub fn print_token_distribution() {
    print!("\n=== Token Distribution ===\n");

    let token_names: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    ANALYZER.with(|a| {
        let a = a.borrow();
        for i in 0..12 {
            if a.token_type_counts[i] > 0 {
                print!("{}: {}\n", token_names[i], a.token_type_counts[i]);
            }
        }
    });

    print!("\n=== Most Common Words ===\n");

    // Bubble sort top common words by count descending.
    ANALYZER.with(|a| {
        let mut a = a.borrow_mut();
        let n = a.num_common_words;
        if n >= 1 {
            // Note: C does `for (int i = 0; i < num_common_words - 1; i++)` which when
            // num_common_words == 0, becomes `i < -1` -> no iterations (signed). Our `n` is usize;
            // we guard with n >= 1 to avoid underflow (matches the C behavior of skipping when n==0).
            for i in 0..(n - 1) {
                for j in 0..(n - i - 1) {
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
        for i in 0..limit {
            // Print the word as a C string (up to first null).
            let w = &a.common_words[i];
            let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
            // Use stdout write for byte-faithful output.
            let mut stdout = std::io::stdout();
            let _ = write!(stdout, "{}. ", i + 1);
            let _ = stdout.write_all(&w[..end]);
            let _ = write!(stdout, ": {} times\n", a.common_word_counts[i]);
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
    ANALYZER.with(|a| {
        let a = a.borrow();
        let mut score: i32 = 0;
        score = score.wrapping_add(a.token_type_counts[TokenType::Keyword as usize].wrapping_mul(2));
        score = score.wrapping_add(a.token_type_counts[TokenType::Operator as usize]);
        score = score.wrapping_add(a.token_type_counts[TokenType::Punctuation as usize] / 10);
        score = score.wrapping_sub(a.token_type_counts[TokenType::Comment as usize]);
        if score < 0 {
            score = 0;
        }
        score
    })
}

pub fn find_patterns(pattern: &[u8]) {
    let ops = ANALYZER.with(|a| {
        let a = a.borrow();
        if !a.initialized {
            None
        } else {
            a.tokenizer_ops
        }
    });

    let ops = match ops {
        None => return,
        Some(o) => o,
    };

    // C: if (!initialized || !pattern) return;
    // We treat an empty pattern as a non-null C pointer (equivalent to ""), so we still proceed.

    // Print "=== Searching for pattern: '<pattern>' ==="
    {
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(b"\n=== Searching for pattern: '");
        let pat_end = pattern.iter().position(|&c| c == 0).unwrap_or(pattern.len());
        let _ = stdout.write_all(&pattern[..pat_end]);
        let _ = stdout.write_all(b"' ===\n");
    }

    (ops.reset)();

    let mut count: i32 = 0;

    let pat_end = pattern.iter().position(|&c| c == 0).unwrap_or(pattern.len());
    let pat = &pattern[..pat_end];

    loop {
        let token = (ops.next_token)();
        if token.ttype == TokenType::Eof {
            break;
        }
        // strstr semantics: find pat as a substring of token.value's c-string.
        let val_end = token
            .value
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(token.value.len());
        let value = &token.value[..val_end];
        let found = if pat.is_empty() {
            true // strstr(s, "") returns s, which is non-null.
        } else {
            value
                .windows(pat.len())
                .any(|w| w == pat)
        };
        if found {
            let mut stdout = std::io::stdout();
            let _ = write!(stdout, "Line {}, Column {}: ", token.line, token.column);
            let _ = stdout.write_all(value);
            let _ = stdout.write_all(b"\n");
            count += 1;
        }
    }

    print!("Found {} occurrences\n", count);
}

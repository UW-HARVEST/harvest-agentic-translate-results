// Analyzer module - port of analyzer.c

use std::cell::RefCell;
use std::io::Write;

use crate::tokenizer::{Token, TokenType, TokenizerOps, MAX_TOKEN_LENGTH};
use crate::io_buf;

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
    common_words: Vec<Vec<u8>>,
    common_word_counts: [i32; 100],
    num_common_words: i32,
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
    static ASTATE: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::new());
}

pub fn analyzer_init(ops: TokenizerOps) {
    ASTATE.with(|st| {
        let mut s = st.borrow_mut();
        s.tokenizer_ops = Some(ops);
        s.initialized = true;
        for v in s.token_type_counts.iter_mut() {
            *v = 0;
        }
        for v in s.common_word_counts.iter_mut() {
            *v = 0;
        }
        s.num_common_words = 0;
    });
}

fn bytes_eq_cstr(word: &[u8], stored: &[u8]) -> bool {
    let stored_len = stored.iter().position(|&b| b == 0).unwrap_or(stored.len());
    word == &stored[..stored_len]
}

fn track_word(s: &mut AnalyzerState, word: &[u8]) {
    let word_len = word.iter().position(|&b| b == 0).unwrap_or(word.len());
    let word_eff = &word[..word_len];

    for i in 0..s.num_common_words as usize {
        if bytes_eq_cstr(word_eff, &s.common_words[i]) {
            s.common_word_counts[i] += 1;
            return;
        }
    }

    if (s.num_common_words as usize) < 100 {
        let idx = s.num_common_words as usize;
        for b in s.common_words[idx].iter_mut() {
            *b = 0;
        }
        let copy_len = std::cmp::min(word_eff.len(), MAX_TOKEN_LENGTH - 1);
        for i in 0..copy_len {
            s.common_words[idx][i] = word_eff[i];
        }
        s.common_words[idx][MAX_TOKEN_LENGTH - 1] = 0;
        s.common_word_counts[idx] = 1;
        s.num_common_words += 1;
    }
}

pub fn analyze_text(text: &[u8]) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let (initialized, ops) = ASTATE.with(|st| {
        let s = st.borrow();
        (s.initialized, s.tokenizer_ops)
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
        if token.ttype == TokenType::Eof {
            break;
        }

        ASTATE.with(|st| {
            let mut s = st.borrow_mut();
            let idx = token.ttype as usize;
            if idx < s.token_type_counts.len() {
                s.token_type_counts[idx] += 1;
            }
        });

        match token.ttype {
            TokenType::Word | TokenType::Identifier => {
                result.word_count += 1;
                ASTATE.with(|st| {
                    let mut s = st.borrow_mut();
                    track_word(&mut s, &token.value);
                });
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

    let mut lines: usize = 0;
    let mut tokens: usize = 0;
    let mut chars: usize = 0;
    (ops.get_stats)(&mut lines, &mut tokens, &mut chars);

    result.line_count = lines;
    result.char_count = chars;

    result
}

pub fn print_token_distribution() {
    io_buf::write_bytes(b"\n=== Token Distribution ===\n");

    let token_names: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    ASTATE.with(|st| {
        let s = st.borrow();
        for i in 0..12 {
            if s.token_type_counts[i] > 0 {
                let line = format!("{}: {}\n", token_names[i], s.token_type_counts[i]);
                io_buf::write_bytes(line.as_bytes());
            }
        }
    });

    io_buf::write_bytes(b"\n=== Most Common Words ===\n");

    ASTATE.with(|st| {
        let mut s = st.borrow_mut();
        let n = s.num_common_words;
        if n > 1 {
            for i in 0..(n - 1) as usize {
                for j in 0..(n as usize - i - 1) {
                    if s.common_word_counts[j] < s.common_word_counts[j + 1] {
                        let tmp = s.common_word_counts[j];
                        s.common_word_counts[j] = s.common_word_counts[j + 1];
                        s.common_word_counts[j + 1] = tmp;

                        s.common_words.swap(j, j + 1);
                    }
                }
            }
        }

        let limit = if (n as usize) < 10 { n as usize } else { 10 };
        for i in 0..limit {
            let word = &s.common_words[i];
            let word_len = word.iter().position(|&b| b == 0).unwrap_or(word.len());
            let prefix = format!("{}. ", i + 1);
            io_buf::write_bytes(prefix.as_bytes());
            io_buf::write_bytes(&word[..word_len]);
            let suffix = format!(": {} times\n", s.common_word_counts[i]);
            io_buf::write_bytes(suffix.as_bytes());
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
    ASTATE.with(|st| {
        let s = st.borrow();
        let mut score: i32 = 0;
        score += s.token_type_counts[TokenType::Keyword as usize] * 2;
        score += s.token_type_counts[TokenType::Operator as usize];
        score += s.token_type_counts[TokenType::Punctuation as usize] / 10;
        score -= s.token_type_counts[TokenType::Comment as usize];
        if score < 0 {
            score = 0;
        }
        score
    })
}

pub fn find_patterns(pattern: &[u8]) {
    let (initialized, ops) = ASTATE.with(|st| {
        let s = st.borrow();
        (s.initialized, s.tokenizer_ops)
    });

    if !initialized {
        return;
    }

    {
        io_buf::write_bytes(b"\n=== Searching for pattern: '");
        let plen = pattern.iter().position(|&b| b == 0).unwrap_or(pattern.len());
        io_buf::write_bytes(&pattern[..plen]);
        io_buf::write_bytes(b"' ===\n");
    }

    let ops = ops.unwrap();
    (ops.reset)();

    let mut count: i32 = 0;
    let pattern_len = pattern.iter().position(|&b| b == 0).unwrap_or(pattern.len());
    let pattern_eff = &pattern[..pattern_len];

    loop {
        let token: Token = (ops.next_token)();
        if token.ttype == TokenType::Eof {
            break;
        }
        let value_len = token
            .value
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(token.value.len());
        let haystack = &token.value[..value_len];

        let found = if pattern_eff.is_empty() {
            true
        } else {
            haystack.windows(pattern_eff.len()).any(|w| w == pattern_eff)
        };

        if found {
            let prefix = format!("Line {}, Column {}: ", token.line, token.column);
            io_buf::write_bytes(prefix.as_bytes());
            io_buf::write_bytes(haystack);
            io_buf::write_bytes(b"\n");
            count += 1;
        }
    }

    let suffix = format!("Found {} occurrences\n", count);
    io_buf::write_bytes(suffix.as_bytes());
}

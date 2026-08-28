//! Translation of `analyzer.c` / `analyzer.h`.

use std::cell::RefCell;

use crate::cio::{c_strstr, err_str, out_bytes, out_str};
use crate::tokenizer::{TokenType, TokenizerOps, MAX_TOKEN_LENGTH};

#[derive(Default, Copy, Clone)]
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

// Static storage for tokenizer function pointers / static arrays for tracking
struct AnalyzerState {
    tokenizer_ops: Option<TokenizerOps>,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<Vec<u8>>,
    common_word_counts: Vec<i32>,
    num_common_words: i32,
}

impl AnalyzerState {
    fn new() -> AnalyzerState {
        AnalyzerState {
            tokenizer_ops: None,
            initialized: false,
            token_type_counts: [0; 20],
            common_words: vec![Vec::new(); 100],
            common_word_counts: vec![0; 100],
            num_common_words: 0,
        }
    }

    fn track_word(&mut self, word: &[u8]) {
        // Find if word already exists
        for i in 0..self.num_common_words as usize {
            if self.common_words[i] == word {
                self.common_word_counts[i] += 1;
                return;
            }
        }

        // Add new word
        if self.num_common_words < 100 {
            let idx = self.num_common_words as usize;
            let copy_len = word.len().min(MAX_TOKEN_LENGTH - 1);
            self.common_words[idx] = word[..copy_len].to_vec();
            self.common_word_counts[idx] = 1;
            self.num_common_words += 1;
        }
    }
}

thread_local! {
    static STATE: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::new());
}

pub fn analyzer_init(ops: TokenizerOps) {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        state.tokenizer_ops = Some(ops);
        state.initialized = true;

        // Reset tracking arrays
        state.token_type_counts = [0; 20];
        for count in state.common_word_counts.iter_mut() {
            *count = 0;
        }
        state.num_common_words = 0;
    });
}

pub fn analyze_text(text: &[u8]) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let ops = STATE.with(|cell| {
        let state = cell.borrow();
        if !state.initialized {
            None
        } else {
            state.tokenizer_ops
        }
    });

    let ops = match ops {
        Some(ops) => ops,
        None => {
            err_str("Error: Analyzer not initialized\n");
            return result;
        }
    };

    // Load text using function pointer
    if (ops.load_text)(text) != 0 {
        err_str("Error: Failed to load text\n");
        return result;
    }

    // Process all tokens using function pointers
    loop {
        let token = (ops.next_token)();
        if token.ttype == TokenType::Eof {
            break;
        }

        // Update counts
        STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            state.token_type_counts[token.ttype.index()] += 1;

            match token.ttype {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    state.track_word(&token.value);
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

                TokenType::Str => {
                    result.string_count += 1;
                }

                TokenType::Newline => {
                    result.line_count += 1;
                }

                _ => {}
            }
        });
    }

    // Get final statistics using function pointer
    let (lines, _tokens, chars) = (ops.get_stats)();

    result.line_count = lines;
    result.char_count = chars;

    result
}

pub fn print_token_distribution() {
    out_str("\n=== Token Distribution ===\n");

    const TOKEN_NAMES: [&str; 12] = [
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

    STATE.with(|cell| {
        let mut state = cell.borrow_mut();

        for i in 0..12 {
            if state.token_type_counts[i] > 0 {
                out_str(&format!("{}: {}\n", TOKEN_NAMES[i], state.token_type_counts[i]));
            }
        }

        out_str("\n=== Most Common Words ===\n");

        // Simple bubble sort for display
        let num = state.num_common_words;
        let mut i = 0i32;
        while i < num - 1 {
            let mut j = 0i32;
            while j < num - i - 1 {
                let a = j as usize;
                let b = (j + 1) as usize;
                if state.common_word_counts[a] < state.common_word_counts[b] {
                    // Swap counts
                    state.common_word_counts.swap(a, b);

                    // Swap words
                    state.common_words.swap(a, b);
                }
                j += 1;
            }
            i += 1;
        }

        // Print top 10
        let limit = if state.num_common_words < 10 {
            state.num_common_words
        } else {
            10
        };
        for i in 0..limit {
            let idx = i as usize;
            out_str(&format!("{}. ", i + 1));
            out_bytes(&state.common_words[idx]);
            out_str(&format!(": {} times\n", state.common_word_counts[idx]));
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
    STATE.with(|cell| {
        let state = cell.borrow();
        let mut score: i32 = 0;

        // Base score on keyword density
        score += state.token_type_counts[TokenType::Keyword.index()] * 2;

        // Add points for operators
        score += state.token_type_counts[TokenType::Operator.index()];

        // Nesting indicators (braces)
        score += state.token_type_counts[TokenType::Punctuation.index()] / 10;

        // Comments reduce complexity (good documentation)
        score -= state.token_type_counts[TokenType::Comment.index()];

        if score < 0 {
            score = 0;
        }

        score
    })
}

pub fn find_patterns(pattern: &[u8]) {
    let ops = STATE.with(|cell| {
        let state = cell.borrow();
        if !state.initialized {
            None
        } else {
            state.tokenizer_ops
        }
    });

    let ops = match ops {
        Some(ops) => ops,
        None => return,
    };

    out_str("\n=== Searching for pattern: '");
    out_bytes(pattern);
    out_str("' ===\n");

    // Reset tokenizer using function pointer
    (ops.reset)();

    let mut count: i32 = 0;

    loop {
        let token = (ops.next_token)();
        if token.ttype == TokenType::Eof {
            break;
        }
        if c_strstr(&token.value, pattern) {
            out_str(&format!("Line {}, Column {}: ", token.line, token.column));
            out_bytes(&token.value);
            out_str("\n");
            count += 1;
        }
    }

    out_str(&format!("Found {} occurrences\n", count));
}

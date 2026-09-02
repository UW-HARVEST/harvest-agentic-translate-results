//! Port of c_src/src/analyzer.c
//!
//! As in the C, the analyzer's tracking arrays are static state that is only
//! cleared by `analyzer_init` — they accumulate across every analysis.

use std::cell::RefCell;
use std::io::Write;

use crate::cio;
use crate::tokenizer::{
    Token, TokenizerOps, MAX_TOKEN_LENGTH, TOKEN_COMMENT, TOKEN_EOF, TOKEN_IDENTIFIER,
    TOKEN_KEYWORD, TOKEN_NEWLINE, TOKEN_NUMBER, TOKEN_OPERATOR, TOKEN_PUNCTUATION, TOKEN_STRING,
    TOKEN_WORD,
};
use crate::{ceprintf, cprintf};

/// analysis_result_t
#[derive(Clone, Copy, Default)]
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

// Static storage for tokenizer function pointers + static tracking arrays
struct Analyzer {
    tokenizer_ops: Option<TokenizerOps>,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<Vec<u8>>,
    common_word_counts: [i32; 100],
    num_common_words: i32,
}

impl Analyzer {
    fn new() -> Self {
        Analyzer {
            tokenizer_ops: None,
            initialized: false,
            token_type_counts: [0; 20],
            common_words: vec![Vec::new(); 100],
            common_word_counts: [0; 100],
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
            // strncpy(..., MAX_TOKEN_LENGTH - 1) then force-terminate
            let take = if word.len() < MAX_TOKEN_LENGTH - 1 {
                word.len()
            } else {
                MAX_TOKEN_LENGTH - 1
            };
            self.common_words[idx] = word[..take].to_vec();
            self.common_word_counts[idx] = 1;
            self.num_common_words += 1;
        }
    }
}

thread_local! {
    static ANALYZER: RefCell<Analyzer> = RefCell::new(Analyzer::new());
}

pub fn analyzer_init(ops: TokenizerOps) {
    ANALYZER.with(|a| {
        let mut an = a.borrow_mut();
        an.tokenizer_ops = Some(ops);
        an.initialized = true;

        // Reset tracking arrays
        an.token_type_counts = [0; 20];
        an.common_word_counts = [0; 100];
        an.num_common_words = 0;
    });
}

pub fn analyze_text(text: &[u8]) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let ops = ANALYZER.with(|a| {
        let an = a.borrow();
        if !an.initialized {
            None
        } else {
            an.tokenizer_ops
        }
    });

    let ops = match ops {
        Some(ops) => ops,
        None => {
            ceprintf!("Error: Analyzer not initialized\n");
            return result;
        }
    };

    // Load text using function pointer
    if (ops.load_text)(text) != 0 {
        ceprintf!("Error: Failed to load text\n");
        return result;
    }

    // Process all tokens using function pointers
    loop {
        let token: Token = (ops.next_token)();
        if token.ttype == TOKEN_EOF {
            break;
        }

        // Update counts
        ANALYZER.with(|a| {
            let mut an = a.borrow_mut();
            an.token_type_counts[token.ttype] += 1;

            match token.ttype {
                TOKEN_WORD | TOKEN_IDENTIFIER => {
                    result.word_count += 1;
                    an.track_word(&token.value);
                }
                TOKEN_NUMBER => result.number_count += 1,
                TOKEN_KEYWORD => result.keyword_count += 1,
                TOKEN_OPERATOR => result.operator_count += 1,
                TOKEN_COMMENT => result.comment_count += 1,
                TOKEN_STRING => result.string_count += 1,
                TOKEN_NEWLINE => result.line_count += 1,
                _ => {}
            }
        });
    }

    // Get final statistics using function pointer
    let (lines, _tokens, chars) = (ops.get_stats)();

    // NOTE (faithful to the C): this discards the newline-token tally computed
    // above and reports the tokenizer's cumulative line total instead.
    result.line_count = lines;
    result.char_count = chars;

    result
}

pub fn print_token_distribution() {
    cprintf!("\n=== Token Distribution ===\n");

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

    ANALYZER.with(|a| {
        let mut an = a.borrow_mut();

        for i in 0..12 {
            if an.token_type_counts[i] > 0 {
                cprintf!("{}: {}\n", TOKEN_NAMES[i], an.token_type_counts[i]);
            }
        }

        cprintf!("\n=== Most Common Words ===\n");

        // Simple bubble sort for display
        let n = an.num_common_words;
        let mut i = 0i32;
        while i < n - 1 {
            let mut j = 0i32;
            while j < n - i - 1 {
                let ju = j as usize;
                if an.common_word_counts[ju] < an.common_word_counts[ju + 1] {
                    // Swap counts
                    an.common_word_counts.swap(ju, ju + 1);
                    // Swap words
                    an.common_words.swap(ju, ju + 1);
                }
                j += 1;
            }
            i += 1;
        }

        // Print top 10
        let limit = if an.num_common_words < 10 {
            an.num_common_words
        } else {
            10
        };
        for i in 0..limit {
            let iu = i as usize;
            let mut line: Vec<u8> = Vec::new();
            let _ = write!(line, "{}. ", i + 1);
            line.extend_from_slice(&an.common_words[iu]);
            let _ = write!(line, ": {} times\n", an.common_word_counts[iu]);
            cio::out_bytes(&line);
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
    ANALYZER.with(|a| {
        let an = a.borrow();
        let mut score: i32 = 0;

        // Base score on keyword density
        score = score.wrapping_add(an.token_type_counts[TOKEN_KEYWORD].wrapping_mul(2));

        // Add points for operators
        score = score.wrapping_add(an.token_type_counts[TOKEN_OPERATOR]);

        // Nesting indicators (braces)
        score = score.wrapping_add(an.token_type_counts[TOKEN_PUNCTUATION] / 10);

        // Comments reduce complexity (good documentation)
        score = score.wrapping_sub(an.token_type_counts[TOKEN_COMMENT]);

        if score < 0 {
            score = 0;
        }

        score
    })
}

pub fn find_patterns(pattern: &[u8]) {
    let ops = ANALYZER.with(|a| {
        let an = a.borrow();
        if !an.initialized {
            None
        } else {
            an.tokenizer_ops
        }
    });

    let ops = match ops {
        Some(ops) => ops,
        None => return,
    };

    let mut header: Vec<u8> = Vec::new();
    header.extend_from_slice(b"\n=== Searching for pattern: '");
    header.extend_from_slice(pattern);
    header.extend_from_slice(b"' ===\n");
    cio::out_bytes(&header);

    // Reset tokenizer using function pointer
    (ops.reset)();

    let mut count: i32 = 0;

    loop {
        let token: Token = (ops.next_token)();
        if token.ttype == TOKEN_EOF {
            break;
        }
        if cio::strstr(&token.value, pattern) {
            let mut line: Vec<u8> = Vec::new();
            let _ = write!(line, "Line {}, Column {}: ", token.line, token.column);
            line.extend_from_slice(&token.value);
            line.push(b'\n');
            cio::out_bytes(&line);
            count += 1;
        }
    }

    cprintf!("Found {} occurrences\n", count);
}

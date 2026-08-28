//! Translation of analyzer.c
//!
//! In C the analyzer holds function pointers to the tokenizer; here the
//! tokenizer is passed in by reference instead, which is equivalent because
//! `get_tokenizer_ops` always returns the same set of functions.

use crate::cio::{contains, err_str, Out};
use crate::tokenizer::{
    Tokenizer, MAX_TOKEN_LENGTH, TOKEN_COMMENT, TOKEN_EOF, TOKEN_IDENTIFIER, TOKEN_KEYWORD,
    TOKEN_NEWLINE, TOKEN_NUMBER, TOKEN_OPERATOR, TOKEN_PUNCTUATION, TOKEN_STRING, TOKEN_WORD,
};

#[derive(Default)]
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

pub struct Analyzer {
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<Vec<u8>>,
    common_word_counts: Vec<i32>,
    num_common_words: usize,
}

impl Analyzer {
    pub fn new() -> Analyzer {
        Analyzer {
            initialized: false,
            token_type_counts: [0; 20],
            common_words: vec![Vec::new(); 100],
            common_word_counts: vec![0; 100],
            num_common_words: 0,
        }
    }

    pub fn init(&mut self) {
        self.initialized = true;

        // Reset tracking arrays
        self.token_type_counts = [0; 20];
        for c in self.common_word_counts.iter_mut() {
            *c = 0;
        }
        self.num_common_words = 0;
    }

    fn track_word(&mut self, word: &[u8]) {
        // Find if word already exists
        for i in 0..self.num_common_words {
            if self.common_words[i] == word {
                self.common_word_counts[i] += 1;
                return;
            }
        }

        // Add new word
        if self.num_common_words < 100 {
            // strncpy(..., MAX_TOKEN_LENGTH - 1) then force-terminate
            let n = if word.len() < MAX_TOKEN_LENGTH - 1 {
                word.len()
            } else {
                MAX_TOKEN_LENGTH - 1
            };
            self.common_words[self.num_common_words] = word[..n].to_vec();
            self.common_word_counts[self.num_common_words] = 1;
            self.num_common_words += 1;
        }
    }

    pub fn analyze_text(&mut self, tk: &mut Tokenizer, text: &[u8]) -> AnalysisResult {
        let mut result = AnalysisResult::default();

        if !self.initialized {
            err_str("Error: Analyzer not initialized\n");
            return result;
        }

        // Load text using function pointer
        if tk.load_text(text) != 0 {
            err_str("Error: Failed to load text\n");
            return result;
        }

        // Process all tokens using function pointers
        loop {
            let token = tk.next_token();
            if token.ttype == TOKEN_EOF {
                break;
            }

            // Update counts
            self.token_type_counts[token.ttype] += 1;

            match token.ttype {
                TOKEN_WORD | TOKEN_IDENTIFIER => {
                    result.word_count += 1;
                    self.track_word(&token.value);
                }
                TOKEN_NUMBER => result.number_count += 1,
                TOKEN_KEYWORD => result.keyword_count += 1,
                TOKEN_OPERATOR => result.operator_count += 1,
                TOKEN_COMMENT => result.comment_count += 1,
                TOKEN_STRING => result.string_count += 1,
                TOKEN_NEWLINE => result.line_count += 1,
                _ => {}
            }
        }

        // Get final statistics using function pointer. Note these are cumulative
        // process-wide totals that are never reset, and they overwrite the
        // per-analysis newline tally above.
        let (lines, _tokens, chars) = tk.get_stats();

        result.line_count = lines;
        result.char_count = chars;

        result
    }

    pub fn print_token_distribution(&mut self, out: &mut Out) {
        out.str("\n=== Token Distribution ===\n");

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

        for i in 0..12usize {
            if self.token_type_counts[i] > 0 {
                out.str(&format!("{}: {}\n", token_names[i], self.token_type_counts[i]));
            }
        }

        out.str("\n=== Most Common Words ===\n");

        // Simple bubble sort for display (mutates the stored arrays, as in C)
        let n = self.num_common_words as i64;
        let mut i: i64 = 0;
        while i < n - 1 {
            let mut j: i64 = 0;
            while j < n - i - 1 {
                let a = j as usize;
                let b = (j + 1) as usize;
                if self.common_word_counts[a] < self.common_word_counts[b] {
                    self.common_word_counts.swap(a, b);
                    self.common_words.swap(a, b);
                }
                j += 1;
            }
            i += 1;
        }

        // Print top 10
        let limit = if self.num_common_words < 10 {
            self.num_common_words
        } else {
            10
        };
        for i in 0..limit {
            out.str(&format!("{}. ", i + 1));
            out.bytes(&self.common_words[i]);
            out.str(&format!(": {} times\n", self.common_word_counts[i]));
        }
    }

    pub fn calculate_complexity_score(&self) -> i32 {
        let mut score: i32 = 0;

        // Base score on keyword density
        score = score.wrapping_add(self.token_type_counts[TOKEN_KEYWORD].wrapping_mul(2));

        // Add points for operators
        score = score.wrapping_add(self.token_type_counts[TOKEN_OPERATOR]);

        // Nesting indicators (braces)
        score = score.wrapping_add(self.token_type_counts[TOKEN_PUNCTUATION] / 10);

        // Comments reduce complexity (good documentation)
        score = score.wrapping_sub(self.token_type_counts[TOKEN_COMMENT]);

        if score < 0 {
            score = 0;
        }

        score
    }

    pub fn find_patterns(&mut self, tk: &mut Tokenizer, out: &mut Out, pattern: &[u8]) {
        if !self.initialized {
            return;
        }

        out.str("\n=== Searching for pattern: '");
        out.bytes(pattern);
        out.str("' ===\n");

        // Reset tokenizer using function pointer
        tk.reset();

        let mut count: i32 = 0;

        loop {
            let token = tk.next_token();
            if token.ttype == TOKEN_EOF {
                break;
            }
            if contains(&token.value, pattern) {
                out.str(&format!("Line {}, Column {}: ", token.line, token.column));
                out.bytes(&token.value);
                out.str("\n");
                count += 1;
            }
        }

        out.str(&format!("Found {} occurrences\n", count));
    }
}

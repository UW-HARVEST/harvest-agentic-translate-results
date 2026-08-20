//! Translation of c_src/src/analyzer.c

use crate::cio::{err, strstr, up_to_nul, Out};
use crate::tokenizer::{TokenType, Tokenizer, MAX_TOKEN_LENGTH};
use std::io::Write;

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

    /// `analyzer_init(ops)`
    pub fn init(&mut self) {
        self.initialized = true;

        for c in self.token_type_counts.iter_mut() {
            *c = 0;
        }
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
            let n = self.num_common_words;
            let src = up_to_nul(word);
            let take = if src.len() < MAX_TOKEN_LENGTH - 1 {
                src.len()
            } else {
                MAX_TOKEN_LENGTH - 1
            };
            self.common_words[n] = src[..take].to_vec();
            self.common_word_counts[n] = 1;
            self.num_common_words += 1;
        }
    }

    pub fn analyze_text(&mut self, tok: &mut Tokenizer, text: &[u8]) -> AnalysisResult {
        let mut result = AnalysisResult::default();

        if !self.initialized {
            err(b"Error: Analyzer not initialized\n");
            return result;
        }

        // Load text using function pointer
        if tok.load_text(text) != 0 {
            err(b"Error: Failed to load text\n");
            return result;
        }

        // Process all tokens using function pointers
        loop {
            let token = tok.next_token();
            if token.ttype == TokenType::Eof {
                break;
            }

            // Update counts
            self.token_type_counts[token.ttype.index()] += 1;

            match token.ttype {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    let value = token.value.clone();
                    self.track_word(&value);
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
        }

        // Get final statistics using function pointer
        let (lines, _tokens, chars) = tok.get_stats();

        result.line_count = lines;
        result.char_count = chars;

        result
    }

    pub fn print_token_distribution(&mut self, out: &mut Out) {
        out.puts("\n=== Token Distribution ===\n");

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
                let _ = write!(out, "{}: {}\n", token_names[i], self.token_type_counts[i]);
            }
        }

        out.puts("\n=== Most Common Words ===\n");

        // Simple bubble sort for display
        let n = self.num_common_words;
        if n > 1 {
            for i in 0..(n - 1) {
                for j in 0..(n - i - 1) {
                    if self.common_word_counts[j] < self.common_word_counts[j + 1] {
                        self.common_word_counts.swap(j, j + 1);
                        self.common_words.swap(j, j + 1);
                    }
                }
            }
        }

        // Print top 10
        let limit = if self.num_common_words < 10 {
            self.num_common_words
        } else {
            10
        };
        for i in 0..limit {
            let _ = write!(out, "{}. ", i + 1);
            let word = self.common_words[i].clone();
            out.put(&word);
            let _ = write!(out, ": {} times\n", self.common_word_counts[i]);
        }
    }

    pub fn calculate_complexity_score(&self) -> i32 {
        let mut score: i32 = 0;

        // Base score on keyword density
        score = score.wrapping_add(
            self.token_type_counts[TokenType::Keyword.index()].wrapping_mul(2),
        );

        // Add points for operators
        score = score.wrapping_add(self.token_type_counts[TokenType::Operator.index()]);

        // Nesting indicators (braces)
        score = score.wrapping_add(self.token_type_counts[TokenType::Punctuation.index()] / 10);

        // Comments reduce complexity (good documentation)
        score = score.wrapping_sub(self.token_type_counts[TokenType::Comment.index()]);

        if score < 0 {
            score = 0;
        }

        score
    }

    pub fn find_patterns(&mut self, out: &mut Out, tok: &mut Tokenizer, pattern: &[u8]) {
        // The C code also checks for a NULL pattern; callers never pass one.
        if !self.initialized {
            return;
        }

        out.puts("\n=== Searching for pattern: '");
        out.put(pattern);
        out.puts("' ===\n");

        // Reset tokenizer using function pointer
        tok.reset();

        let mut count: i32 = 0;

        loop {
            let token = tok.next_token();
            if token.ttype == TokenType::Eof {
                break;
            }
            if strstr(&token.value, pattern) {
                let _ = write!(out, "Line {}, Column {}: ", token.line, token.column);
                out.put(&token.value);
                out.puts("\n");
                count += 1;
            }
        }

        let _ = write!(out, "Found {} occurrences\n", count);
    }
}

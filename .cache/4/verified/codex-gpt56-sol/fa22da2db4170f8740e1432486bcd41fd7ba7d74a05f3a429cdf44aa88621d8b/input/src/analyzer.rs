use std::fmt::Write as _;

use crate::tokenizer::{TokenType, Tokenizer};

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
    tokenizer: Tokenizer,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<(Vec<u8>, i32)>,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            tokenizer: Tokenizer::new(),
            initialized: true,
            token_type_counts: [0; 20],
            common_words: Vec::new(),
        }
    }

    fn track_word(&mut self, word: &[u8]) {
        for (known_word, count) in &mut self.common_words {
            if known_word == word {
                *count = count.wrapping_add(1);
                return;
            }
        }

        if self.common_words.len() < 100 {
            self.common_words.push((word.to_vec(), 1));
        }
    }

    pub fn analyze_text(&mut self, text: &[u8], stderr: &mut Vec<u8>) -> AnalysisResult {
        let mut result = AnalysisResult::default();

        if !self.initialized {
            stderr.extend_from_slice(b"Error: Analyzer not initialized\n");
            return result;
        }

        if !self.tokenizer.load_text(text, stderr) {
            stderr.extend_from_slice(b"Error: Failed to load text\n");
            return result;
        }

        loop {
            let token = self.tokenizer.next_token();
            if token.token_type == TokenType::Eof {
                break;
            }

            let index = token.token_type as usize;
            self.token_type_counts[index] = self.token_type_counts[index].wrapping_add(1);

            match token.token_type {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    self.track_word(&token.value);
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

        let (lines, _tokens, chars) = self.tokenizer.stats();
        result.line_count = lines;
        result.char_count = chars;
        result
    }

    pub fn print_token_distribution(&mut self, stdout: &mut Vec<u8>) {
        stdout.extend_from_slice(b"\n=== Token Distribution ===\n");

        const TOKEN_NAMES: &[&str] = &[
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

        for (index, name) in TOKEN_NAMES.iter().enumerate() {
            if self.token_type_counts[index] > 0 {
                let mut line = String::new();
                writeln!(line, "{}: {}", name, self.token_type_counts[index]).unwrap();
                stdout.extend_from_slice(line.as_bytes());
            }
        }

        stdout.extend_from_slice(b"\n=== Most Common Words ===\n");

        let len = self.common_words.len();
        for i in 0..len.saturating_sub(1) {
            for j in 0..len - i - 1 {
                if self.common_words[j].1 < self.common_words[j + 1].1 {
                    self.common_words.swap(j, j + 1);
                }
            }
        }

        for (index, (word, count)) in self.common_words.iter().take(10).enumerate() {
            let prefix = format!("{}. ", index + 1);
            stdout.extend_from_slice(prefix.as_bytes());
            stdout.extend_from_slice(word);
            let suffix = format!(": {} times\n", count);
            stdout.extend_from_slice(suffix.as_bytes());
        }
    }

    pub fn calculate_complexity_score(&self) -> i32 {
        let mut score = 0i32;
        score =
            score.wrapping_add(self.token_type_counts[TokenType::Keyword as usize].wrapping_mul(2));
        score = score.wrapping_add(self.token_type_counts[TokenType::Operator as usize]);
        score = score.wrapping_add(self.token_type_counts[TokenType::Punctuation as usize] / 10);
        score = score.wrapping_sub(self.token_type_counts[TokenType::Comment as usize]);

        if score < 0 {
            0
        } else {
            score
        }
    }

    pub fn find_patterns(&mut self, pattern: &[u8], stdout: &mut Vec<u8>) {
        if !self.initialized {
            return;
        }

        stdout.extend_from_slice(b"\n=== Searching for pattern: '");
        stdout.extend_from_slice(pattern);
        stdout.extend_from_slice(b"' ===\n");

        self.tokenizer.reset();

        let mut count = 0i32;
        loop {
            let token = self.tokenizer.next_token();
            if token.token_type == TokenType::Eof {
                break;
            }

            if contains(&token.value, pattern) {
                let prefix = format!("Line {}, Column {}: ", token.line, token.column);
                stdout.extend_from_slice(prefix.as_bytes());
                stdout.extend_from_slice(&token.value);
                stdout.push(b'\n');
                count = count.wrapping_add(1);
            }
        }

        let line = format!("Found {} occurrences\n", count);
        stdout.extend_from_slice(line.as_bytes());
    }

    pub fn tokenizer_mut(&mut self) -> &mut Tokenizer {
        &mut self.tokenizer
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

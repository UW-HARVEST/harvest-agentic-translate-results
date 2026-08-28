use std::io::{self, Write};

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
            initialized: false,
            token_type_counts: [0; 20],
            common_words: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        self.initialized = true;
        self.token_type_counts = [0; 20];
        self.common_words.clear();
    }

    fn track_word(&mut self, word: &[u8]) {
        for (existing, count) in &mut self.common_words {
            if existing == word {
                *count = count.wrapping_add(1);
                return;
            }
        }

        if self.common_words.len() < 100 {
            self.common_words.push((word[..word.len().min(255)].to_vec(), 1));
        }
    }

    pub fn analyze_text(&mut self, text: &[u8]) -> AnalysisResult {
        let mut result = AnalysisResult::default();

        if !self.initialized {
            let _ = writeln!(io::stderr().lock(), "Error: Analyzer not initialized");
            return result;
        }

        if self.tokenizer.load_text(text) != 0 {
            let _ = writeln!(io::stderr().lock(), "Error: Failed to load text");
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
                    result.word_count = result.word_count.wrapping_add(1);
                    self.track_word(&token.value);
                }
                TokenType::Number => {
                    result.number_count = result.number_count.wrapping_add(1);
                }
                TokenType::Keyword => {
                    result.keyword_count = result.keyword_count.wrapping_add(1);
                }
                TokenType::Operator => {
                    result.operator_count = result.operator_count.wrapping_add(1);
                }
                TokenType::Comment => {
                    result.comment_count = result.comment_count.wrapping_add(1);
                }
                TokenType::String => {
                    result.string_count = result.string_count.wrapping_add(1);
                }
                TokenType::Newline => {
                    result.line_count = result.line_count.wrapping_add(1);
                }
                _ => {}
            }
        }

        let (lines, _tokens, chars) = self.tokenizer.get_stats();
        result.line_count = lines;
        result.char_count = chars;
        result
    }

    pub fn print_token_distribution<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        writeln!(out, "\n=== Token Distribution ===")?;

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

        for (index, name) in TOKEN_NAMES.iter().enumerate() {
            if self.token_type_counts[index] > 0 {
                writeln!(out, "{}: {}", name, self.token_type_counts[index])?;
            }
        }

        writeln!(out, "\n=== Most Common Words ===")?;

        let count = self.common_words.len();
        for i in 0..count.saturating_sub(1) {
            for j in 0..count - i - 1 {
                if self.common_words[j].1 < self.common_words[j + 1].1 {
                    self.common_words.swap(j, j + 1);
                }
            }
        }

        for (index, (word, occurrences)) in self.common_words.iter().take(10).enumerate() {
            write!(out, "{}. ", index + 1)?;
            out.write_all(word)?;
            writeln!(out, ": {} times", occurrences)?;
        }

        Ok(())
    }

    pub fn calculate_complexity_score(&self) -> i32 {
        let mut score = 0i32;
        score = score.wrapping_add(
            self.token_type_counts[TokenType::Keyword as usize].wrapping_mul(2),
        );
        score = score.wrapping_add(self.token_type_counts[TokenType::Operator as usize]);
        score = score.wrapping_add(
            self.token_type_counts[TokenType::Punctuation as usize].wrapping_div(10),
        );
        score = score.wrapping_sub(self.token_type_counts[TokenType::Comment as usize]);
        score.max(0)
    }

    pub fn find_patterns<W: Write>(&mut self, pattern: &[u8], out: &mut W) -> io::Result<()> {
        if !self.initialized {
            return Ok(());
        }

        write!(out, "\n=== Searching for pattern: '")?;
        out.write_all(pattern)?;
        writeln!(out, "' ===")?;

        self.tokenizer.reset();
        let mut count = 0i32;

        loop {
            let token = self.tokenizer.next_token();
            if token.token_type == TokenType::Eof {
                break;
            }

            if contains(&token.value, pattern) {
                write!(out, "Line {}, Column {}: ", token.line, token.column)?;
                out.write_all(&token.value)?;
                writeln!(out)?;
                count = count.wrapping_add(1);
            }
        }

        writeln!(out, "Found {} occurrences", count)
    }

    pub fn tokenizer_mut(&mut self) -> &mut Tokenizer {
        &mut self.tokenizer
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || (needle.len() <= haystack.len()
            && haystack
                .windows(needle.len())
                .any(|window| window == needle))
}

use crate::tokenizer::{TokenType, Token, TokenizerOps, tokenizer_next_token, tokenizer_peek_token, tokenizer_reset, tokenizer_load_text, tokenizer_get_stats, MAX_TOKEN_LENGTH};
use std::cell::RefCell;

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

impl AnalysisResult {
    pub fn new() -> Self {
        AnalysisResult {
            word_count: 0,
            number_count: 0,
            keyword_count: 0,
            operator_count: 0,
            comment_count: 0,
            string_count: 0,
            line_count: 0,
            char_count: 0,
        }
    }
}

thread_local! {
    static TOKENIZER_OPS: RefCell<Option<TokenizerOps>> = RefCell::new(None);
    static INITIALIZED: RefCell<bool> = RefCell::new(false);
    static TOKEN_TYPE_COUNTS: RefCell<[usize; 20]> = RefCell::new([0; 20]);
    static COMMON_WORDS: RefCell<Vec<(String, usize)>> = RefCell::new(Vec::new());
}

pub fn analyzer_init(ops: TokenizerOps) {
    TOKENIZER_OPS.with(|o| {
        *o.borrow_mut() = Some(ops);
    });
    INITIALIZED.with(|i| {
        *i.borrow_mut() = true;
    });
    TOKEN_TYPE_COUNTS.with(|c| {
        *c.borrow_mut() = [0; 20];
    });
    COMMON_WORDS.with(|w| {
        w.borrow_mut().clear();
    });
}

fn track_word(word: &str) {
    COMMON_WORDS.with(|words| {
        let mut words = words.borrow_mut();
        for (w, count) in words.iter_mut() {
            if w == word {
                *count += 1;
                return;
            }
        }
        if words.len() < 100 {
            let truncated = if word.len() > MAX_TOKEN_LENGTH - 1 {
                &word[..MAX_TOKEN_LENGTH - 1]
            } else {
                word
            };
            words.push((truncated.to_string(), 1));
        }
    });
}

pub fn analyze_text(text: &str) -> AnalysisResult {
    let mut result = AnalysisResult::new();
    
    let initialized = INITIALIZED.with(|i| *i.borrow());
    if !initialized {
        eprintln!("Error: Analyzer not initialized");
        return result;
    }
    
    let load_result = TOKENIZER_OPS.with(|ops| {
        if let Some(ref ops) = *ops.borrow() {
            ops.load_text(text)
        } else {
            -1
        }
    });
    
    if load_result != 0 {
        eprintln!("Error: Failed to load text");
        return result;
    }
    
    loop {
        let token = TOKENIZER_OPS.with(|ops| {
            if let Some(ref ops) = *ops.borrow() {
                ops.next_token()
            } else {
                Token::new(TokenType::Eof, "", 0, 0, 0)
            }
        });
        
        if token.token_type == TokenType::Eof {
            break;
        }
        
        TOKEN_TYPE_COUNTS.with(|counts| {
            let idx = token.token_type as usize;
            if idx < 20 {
                counts.borrow_mut()[idx] += 1;
            }
        });
        
        match token.token_type {
            TokenType::Word | TokenType::Identifier => {
                result.word_count += 1;
                track_word(&token.value);
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
    
    let (lines, _tokens, chars) = TOKENIZER_OPS.with(|ops| {
        if let Some(ref ops) = *ops.borrow() {
            ops.get_stats()
        } else {
            (0, 0, 0)
        }
    });
    
    result.line_count = lines;
    result.char_count = chars;
    
    result
}

pub fn print_token_distribution() {
    println!("\n=== Token Distribution ===");
    
    let token_names = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];
    
    TOKEN_TYPE_COUNTS.with(|counts| {
        let counts = counts.borrow();
        for i in 0..12 {
            if counts[i] > 0 {
                println!("{}: {}", token_names[i], counts[i]);
            }
        }
    });
    
    println!("\n=== Most Common Words ===");
    
    COMMON_WORDS.with(|words| {
        let mut words: Vec<(String, usize)> = words.borrow().clone();
        words.sort_by(|a, b| b.1.cmp(&a.1));
        
        let limit = words.len().min(10);
        for i in 0..limit {
            println!("{}. {}: {} times", i + 1, words[i].0, words[i].1);
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
    TOKEN_TYPE_COUNTS.with(|counts| {
        let counts = counts.borrow();
        let mut score: i32 = 0;
        
        score += (counts[TokenType::Keyword as usize] * 2) as i32;
        score += counts[TokenType::Operator as usize] as i32;
        score += (counts[TokenType::Punctuation as usize] / 10) as i32;
        score -= counts[TokenType::Comment as usize] as i32;
        
        if score < 0 {
            score = 0;
        }
        
        score
    })
}

pub fn find_patterns(pattern: &str) {
    let initialized = INITIALIZED.with(|i| *i.borrow());
    if !initialized || pattern.is_empty() {
        return;
    }
    
    println!("\n=== Searching for pattern: '{}' ===", pattern);
    
    TOKENIZER_OPS.with(|ops| {
        if let Some(ref ops) = *ops.borrow() {
            ops.reset();
        }
    });
    
    let mut count = 0;
    
    loop {
        let token = TOKENIZER_OPS.with(|ops| {
            if let Some(ref ops) = *ops.borrow() {
                ops.next_token()
            } else {
                Token::new(TokenType::Eof, "", 0, 0, 0)
            }
        });
        
        if token.token_type == TokenType::Eof {
            break;
        }
        
        if token.value.contains(pattern) {
            println!("Line {}, Column {}: {}", token.line, token.column, token.value);
            count += 1;
        }
    }
    
    println!("Found {} occurrences", count);
}

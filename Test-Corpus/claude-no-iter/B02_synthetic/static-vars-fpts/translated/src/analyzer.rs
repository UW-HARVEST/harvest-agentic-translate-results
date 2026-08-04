// Translated from c_src/src/analyzer.c

use std::cell::RefCell;

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

struct AnalyzerState {
    ops: Option<TokenizerOps>,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<Vec<u8>>, // each entry is the word's bytes (no NUL)
    common_word_counts: Vec<i32>,
}

impl AnalyzerState {
    const fn new() -> Self {
        AnalyzerState {
            ops: None,
            initialized: false,
            token_type_counts: [0; 20],
            common_words: Vec::new(),
            common_word_counts: Vec::new(),
        }
    }
}

thread_local! {
    static STATE: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::new());
}

pub fn analyzer_init(ops: TokenizerOps) {
    STATE.with(|st| {
        let mut s = st.borrow_mut();
        s.ops = Some(ops);
        s.initialized = true;
        s.token_type_counts = [0; 20];
        s.common_words.clear();
        s.common_word_counts.clear();
    });
}

fn track_word(s: &mut AnalyzerState, word: &[u8]) {
    // Match the C behavior: the common_words slot stores up to
    // MAX_TOKEN_LENGTH-1 bytes (the rest is NUL terminator). The token
    // already came in <= MAX_TOKEN_LENGTH-1 bytes from the tokenizer, so
    // this truncation is largely defensive.
    let truncated: Vec<u8> = word.iter().copied().take(MAX_TOKEN_LENGTH - 1).collect();

    for i in 0..s.common_words.len() {
        if s.common_words[i] == truncated {
            s.common_word_counts[i] += 1;
            return;
        }
    }

    if s.common_words.len() < 100 {
        s.common_words.push(truncated);
        s.common_word_counts.push(1);
    }
}

pub fn analyze_text(text: &[u8]) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let ops = STATE.with(|st| {
        let s = st.borrow();
        if !s.initialized {
            None
        } else {
            s.ops
        }
    });

    let ops = match ops {
        None => {
            eprintln!("Error: Analyzer not initialized");
            return result;
        }
        Some(ops) => ops,
    };

    if (ops.load_text)(text) != 0 {
        eprintln!("Error: Failed to load text");
        return result;
    }

    loop {
        let token = (ops.next_token)();
        if matches!(token.typ, TokenType::Eof) {
            break;
        }
        STATE.with(|st| {
            let mut s = st.borrow_mut();
            let idx = token.typ as usize;
            if idx < s.token_type_counts.len() {
                s.token_type_counts[idx] += 1;
            }
            match token.typ {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    track_word(&mut s, &token.value);
                }
                TokenType::Number => result.number_count += 1,
                TokenType::Keyword => result.keyword_count += 1,
                TokenType::Operator => result.operator_count += 1,
                TokenType::Comment => result.comment_count += 1,
                TokenType::String => result.string_count += 1,
                TokenType::Newline => result.line_count += 1,
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

    let token_names = [
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

    STATE.with(|st| {
        let mut s = st.borrow_mut();
        for i in 0..12 {
            if s.token_type_counts[i] > 0 {
                println!("{}: {}", token_names[i], s.token_type_counts[i]);
            }
        }

        print!("\n=== Most Common Words ===\n");

        // Bubble-sort descending by count, exactly like the C version.
        let n = s.common_words.len();
        if n > 0 {
            for i in 0..(n - 1) {
                for j in 0..(n - i - 1) {
                    if s.common_word_counts[j] < s.common_word_counts[j + 1] {
                        s.common_word_counts.swap(j, j + 1);
                        s.common_words.swap(j, j + 1);
                    }
                }
            }
        }

        let limit = std::cmp::min(s.common_words.len(), 10);
        for i in 0..limit {
            // Print the word as raw bytes; the underlying C `printf("%s", ...)`
            // emits whatever bytes are in the buffer up to a NUL.
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            use std::io::Write;
            let _ = write!(handle, "{}. ", i + 1);
            let _ = handle.write_all(&s.common_words[i]);
            let _ = writeln!(handle, ": {} times", s.common_word_counts[i]);
        }
    });
}

pub fn calculate_complexity_score() -> i32 {
    STATE.with(|st| {
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
    let ops = STATE.with(|st| {
        let s = st.borrow();
        if !s.initialized {
            None
        } else {
            s.ops
        }
    });
    let ops = match ops {
        Some(o) => o,
        None => return,
    };

    use std::io::Write;
    let stdout = std::io::stdout();
    {
        let mut handle = stdout.lock();
        let _ = handle.write_all(b"\n=== Searching for pattern: '");
        let _ = handle.write_all(pattern);
        let _ = handle.write_all(b"' ===\n");
    }

    (ops.reset)();

    let mut count: i32 = 0;
    loop {
        let token = (ops.next_token)();
        if matches!(token.typ, TokenType::Eof) {
            break;
        }
        if find_substring(&token.value, pattern) {
            let mut handle = stdout.lock();
            let _ = write!(handle, "Line {}, Column {}: ", token.line, token.column);
            let _ = handle.write_all(&token.value);
            let _ = handle.write_all(b"\n");
            count += 1;
        }
    }

    println!("Found {} occurrences", count);
}

/// Mimic C's `strstr`. An empty pattern (which would be `pattern[0] == '\0'`)
/// returns true (matches C: strstr of empty needle returns haystack).
fn find_substring(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

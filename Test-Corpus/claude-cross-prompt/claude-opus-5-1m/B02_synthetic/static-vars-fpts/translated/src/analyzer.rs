// Translation of c_src/src/analyzer.c
//
// Mirrors the analyzer's static state and the function pointer-based
// tokenizer interaction from the C version. All public functions are exported
// with their C linker names.

use core::ffi::{c_char, c_int};
use libc::{
    fprintf, memset, printf, size_t, strcmp, strcpy, strncpy, strstr,
};

use crate::tokenizer::{
    Token, TokenizerOps, MAX_TOKEN_LENGTH, TOKEN_COMMENT, TOKEN_EOF, TOKEN_IDENTIFIER,
    TOKEN_KEYWORD, TOKEN_NEWLINE, TOKEN_NUMBER, TOKEN_OPERATOR, TOKEN_PUNCTUATION,
    TOKEN_STRING, TOKEN_WORD,
};

// `analysis_result_t` from analyzer.h
#[repr(C)]
#[derive(Copy, Clone)]
pub struct AnalysisResult {
    pub word_count: size_t,
    pub number_count: size_t,
    pub keyword_count: size_t,
    pub operator_count: size_t,
    pub comment_count: size_t,
    pub string_count: size_t,
    pub line_count: size_t,
    pub char_count: size_t,
}

impl AnalysisResult {
    const fn zero() -> Self {
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

// Static (file-local) state from analyzer.c
const ZERO_OPS_PLACEHOLDER: Option<TokenizerOps> = None;
static mut TOKENIZER_OPS: Option<TokenizerOps> = ZERO_OPS_PLACEHOLDER;
static mut INITIALIZED: c_int = 0;

static mut TOKEN_TYPE_COUNTS: [c_int; 20] = [0; 20];
static mut COMMON_WORDS: [[c_char; MAX_TOKEN_LENGTH]; 100] = [[0; MAX_TOKEN_LENGTH]; 100];
static mut COMMON_WORD_COUNTS: [c_int; 100] = [0; 100];
static mut NUM_COMMON_WORDS: c_int = 0;

extern "C" {
    static stderr: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn analyzer_init(ops: TokenizerOps) {
    unsafe {
        TOKENIZER_OPS = Some(ops);
        INITIALIZED = 1;

        // Reset tracking arrays
        memset(
            TOKEN_TYPE_COUNTS.as_mut_ptr() as *mut libc::c_void,
            0,
            core::mem::size_of_val(&TOKEN_TYPE_COUNTS),
        );
        memset(
            COMMON_WORD_COUNTS.as_mut_ptr() as *mut libc::c_void,
            0,
            core::mem::size_of_val(&COMMON_WORD_COUNTS),
        );
        NUM_COMMON_WORDS = 0;
    }
}

unsafe fn track_word(word: *const c_char) {
    // Find if word already exists
    for i in 0..NUM_COMMON_WORDS {
        if strcmp(COMMON_WORDS[i as usize].as_ptr(), word) == 0 {
            COMMON_WORD_COUNTS[i as usize] += 1;
            return;
        }
    }

    // Add new word
    if NUM_COMMON_WORDS < 100 {
        let idx = NUM_COMMON_WORDS as usize;
        strncpy(
            COMMON_WORDS[idx].as_mut_ptr(),
            word,
            MAX_TOKEN_LENGTH - 1,
        );
        COMMON_WORDS[idx][MAX_TOKEN_LENGTH - 1] = 0;
        COMMON_WORD_COUNTS[idx] = 1;
        NUM_COMMON_WORDS += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn analyze_text(text: *const c_char) -> AnalysisResult {
    unsafe {
        let mut result = AnalysisResult::zero();

        if INITIALIZED == 0 {
            let msg = b"Error: Analyzer not initialized\n\0";
            fprintf(stderr, msg.as_ptr() as *const c_char);
            return result;
        }

        let ops = match TOKENIZER_OPS {
            Some(o) => o,
            None => {
                let msg = b"Error: Analyzer not initialized\n\0";
                fprintf(stderr, msg.as_ptr() as *const c_char);
                return result;
            }
        };

        // Load text using function pointer
        if (ops.load_text)(text) != 0 {
            let msg = b"Error: Failed to load text\n\0";
            fprintf(stderr, msg.as_ptr() as *const c_char);
            return result;
        }

        // Process all tokens using function pointers
        loop {
            let token: Token = (ops.next_token)();
            if token.r#type == TOKEN_EOF {
                break;
            }

            // Update counts
            let type_idx = token.r#type.0 as usize;
            if type_idx < TOKEN_TYPE_COUNTS.len() {
                TOKEN_TYPE_COUNTS[type_idx] += 1;
            }

            if token.r#type == TOKEN_WORD || token.r#type == TOKEN_IDENTIFIER {
                result.word_count += 1;
                track_word(token.value.as_ptr());
            } else if token.r#type == TOKEN_NUMBER {
                result.number_count += 1;
            } else if token.r#type == TOKEN_KEYWORD {
                result.keyword_count += 1;
            } else if token.r#type == TOKEN_OPERATOR {
                result.operator_count += 1;
            } else if token.r#type == TOKEN_COMMENT {
                result.comment_count += 1;
            } else if token.r#type == TOKEN_STRING {
                result.string_count += 1;
            } else if token.r#type == TOKEN_NEWLINE {
                result.line_count += 1;
            }
        }

        // Get final statistics using function pointer
        let mut lines: size_t = 0;
        let mut tokens_n: size_t = 0;
        let mut chars: size_t = 0;
        (ops.get_stats)(&mut lines, &mut tokens_n, &mut chars);

        result.line_count = lines;
        result.char_count = chars;

        result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn print_token_distribution() {
    unsafe {
        let header = b"\n=== Token Distribution ===\n\0";
        printf(header.as_ptr() as *const c_char);

        let token_names: [&[u8]; 12] = [
            b"EOF\0",
            b"WORD\0",
            b"NUMBER\0",
            b"PUNCTUATION\0",
            b"WHITESPACE\0",
            b"NEWLINE\0",
            b"IDENTIFIER\0",
            b"KEYWORD\0",
            b"OPERATOR\0",
            b"STRING\0",
            b"COMMENT\0",
            b"ERROR\0",
        ];

        let fmt = b"%s: %d\n\0";
        for i in 0..12usize {
            if TOKEN_TYPE_COUNTS[i] > 0 {
                printf(
                    fmt.as_ptr() as *const c_char,
                    token_names[i].as_ptr() as *const c_char,
                    TOKEN_TYPE_COUNTS[i],
                );
            }
        }

        let header2 = b"\n=== Most Common Words ===\n\0";
        printf(header2.as_ptr() as *const c_char);

        // Simple bubble sort for display
        if NUM_COMMON_WORDS > 0 {
            for i in 0..(NUM_COMMON_WORDS - 1) {
                for j in 0..(NUM_COMMON_WORDS - i - 1) {
                    let j = j as usize;
                    if COMMON_WORD_COUNTS[j] < COMMON_WORD_COUNTS[j + 1] {
                        // Swap counts
                        let temp_count = COMMON_WORD_COUNTS[j];
                        COMMON_WORD_COUNTS[j] = COMMON_WORD_COUNTS[j + 1];
                        COMMON_WORD_COUNTS[j + 1] = temp_count;

                        // Swap words
                        let mut temp_word: [c_char; MAX_TOKEN_LENGTH] = [0; MAX_TOKEN_LENGTH];
                        strcpy(temp_word.as_mut_ptr(), COMMON_WORDS[j].as_ptr());
                        strcpy(
                            COMMON_WORDS[j].as_mut_ptr(),
                            COMMON_WORDS[j + 1].as_ptr(),
                        );
                        strcpy(
                            COMMON_WORDS[j + 1].as_mut_ptr(),
                            temp_word.as_ptr(),
                        );
                    }
                }
            }
        }

        // Print top 10
        let limit = if NUM_COMMON_WORDS < 10 {
            NUM_COMMON_WORDS
        } else {
            10
        };
        let fmt2 = b"%d. %s: %d times\n\0";
        for i in 0..limit {
            let idx = i as usize;
            printf(
                fmt2.as_ptr() as *const c_char,
                i + 1,
                COMMON_WORDS[idx].as_ptr(),
                COMMON_WORD_COUNTS[idx],
            );
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_complexity_score() -> c_int {
    unsafe {
        let mut score: c_int = 0;

        // Base score on keyword density
        score += TOKEN_TYPE_COUNTS[TOKEN_KEYWORD.0 as usize] * 2;

        // Add points for operators
        score += TOKEN_TYPE_COUNTS[TOKEN_OPERATOR.0 as usize];

        // Nesting indicators (braces)
        score += TOKEN_TYPE_COUNTS[TOKEN_PUNCTUATION.0 as usize] / 10;

        // Comments reduce complexity (good documentation)
        score -= TOKEN_TYPE_COUNTS[TOKEN_COMMENT.0 as usize];

        if score < 0 {
            score = 0;
        }
        score
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn find_patterns(pattern: *const c_char) {
    unsafe {
        if INITIALIZED == 0 || pattern.is_null() {
            return;
        }

        let header_fmt = b"\n=== Searching for pattern: '%s' ===\n\0";
        printf(header_fmt.as_ptr() as *const c_char, pattern);

        let ops = match TOKENIZER_OPS {
            Some(o) => o,
            None => return,
        };

        // Reset tokenizer using function pointer
        (ops.reset)();

        let mut count: c_int = 0;
        loop {
            let token: Token = (ops.next_token)();
            if token.r#type == TOKEN_EOF {
                break;
            }
            if !strstr(token.value.as_ptr(), pattern).is_null() {
                let line_fmt = b"Line %d, Column %d: %s\n\0";
                printf(
                    line_fmt.as_ptr() as *const c_char,
                    token.line,
                    token.column,
                    token.value.as_ptr(),
                );
                count += 1;
            }
        }

        let count_fmt = b"Found %d occurrences\n\0";
        printf(count_fmt.as_ptr() as *const c_char, count);
    }
}

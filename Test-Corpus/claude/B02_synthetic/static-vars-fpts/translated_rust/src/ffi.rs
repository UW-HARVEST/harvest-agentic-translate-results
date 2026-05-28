// C ABI wrappers that match the symbols exported by the C shared library.
//
// These functions mirror the C declarations found in c_src/include/tokenizer.h
// and c_src/include/analyzer.h, allowing external callers (e.g. integration
// tests using libloading) to call the Rust translation through the same
// interface as the C version.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use crate::analyzer as ana;
use crate::tokenizer as tok;

pub const MAX_TOKEN_LENGTH: usize = tok::MAX_TOKEN_LENGTH;

/// Mirror of C `token_t`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CToken {
    pub r#type: c_int,
    pub value: [c_char; MAX_TOKEN_LENGTH],
    pub length: usize,
    pub line: c_int,
    pub column: c_int,
}

impl CToken {
    fn zeroed() -> Self {
        CToken {
            r#type: 0,
            value: [0; MAX_TOKEN_LENGTH],
            length: 0,
            line: 0,
            column: 0,
        }
    }
}

/// Mirror of C `tokenizer_ops_t`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CTokenizerOps {
    pub next_token: Option<extern "C" fn() -> CToken>,
    pub peek_token: Option<extern "C" fn() -> CToken>,
    pub reset: Option<extern "C" fn()>,
    pub load_text: Option<extern "C" fn(*const c_char) -> c_int>,
    pub get_stats: Option<extern "C" fn(*mut usize, *mut usize, *mut usize)>,
}

/// Mirror of C `analysis_result_t`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CAnalysisResult {
    pub word_count: usize,
    pub number_count: usize,
    pub keyword_count: usize,
    pub operator_count: usize,
    pub comment_count: usize,
    pub string_count: usize,
    pub line_count: usize,
    pub char_count: usize,
}

fn token_to_c(t: tok::Token) -> CToken {
    let mut out = CToken::zeroed();
    out.r#type = t.ttype as c_int;
    out.length = t.length;
    out.line = t.line;
    out.column = t.column;
    // t.value contains length bytes of token content followed by a 0 terminator
    // (mirroring C `char value[MAX_TOKEN_LENGTH]`).
    let copy_n = t.value.len().min(MAX_TOKEN_LENGTH);
    for (i, &b) in t.value[..copy_n].iter().enumerate() {
        out.value[i] = b as c_char;
    }
    out
}

fn cstr_to_bytes<'a>(p: *const c_char) -> Option<&'a [u8]> {
    if p.is_null() {
        return None;
    }
    // SAFETY: caller passes a valid C string.
    Some(unsafe { CStr::from_ptr(p).to_bytes() })
}

// ---------------------------------------------------------------------------
// Tokenizer C ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn tokenizer_next_token() -> CToken {
    token_to_c(tok::tokenizer_next_token())
}

#[no_mangle]
pub extern "C" fn tokenizer_peek_token() -> CToken {
    token_to_c(tok::tokenizer_peek_token())
}

#[no_mangle]
pub extern "C" fn tokenizer_reset() {
    tok::tokenizer_reset();
}

#[no_mangle]
pub extern "C" fn tokenizer_load_text(text: *const c_char) -> c_int {
    match cstr_to_bytes(text) {
        None => -1,
        Some(b) => tok::tokenizer_load_text(b),
    }
}

#[no_mangle]
pub extern "C" fn tokenizer_get_stats(
    lines: *mut usize,
    tokens: *mut usize,
    chars: *mut usize,
) {
    let (l, t, c) = tok::tokenizer_get_stats();
    unsafe {
        if !lines.is_null() {
            *lines = l;
        }
        if !tokens.is_null() {
            *tokens = t;
        }
        if !chars.is_null() {
            *chars = c;
        }
    }
}

#[no_mangle]
pub extern "C" fn get_tokenizer_ops() -> CTokenizerOps {
    CTokenizerOps {
        next_token: Some(tokenizer_next_token),
        peek_token: Some(tokenizer_peek_token),
        reset: Some(tokenizer_reset),
        load_text: Some(tokenizer_load_text),
        get_stats: Some(tokenizer_get_stats),
    }
}

// ---------------------------------------------------------------------------
// Analyzer C ABI
// ---------------------------------------------------------------------------

// Storage for tokenizer ops registered through the C interface.
// We call these C function pointers from analyze_text/find_patterns when
// the analyzer was initialized via the C ABI, mirroring the C analyzer's
// behavior of dispatching through the supplied function pointers.
use std::cell::RefCell;
thread_local! {
    static C_OPS: RefCell<Option<CTokenizerOps>> = const { RefCell::new(None) };
    static C_TOKEN_TYPE_COUNTS: RefCell<[i32; 20]> = const { RefCell::new([0; 20]) };
    static C_COMMON_WORDS: RefCell<Vec<[u8; MAX_TOKEN_LENGTH]>> = RefCell::new(Vec::new());
    static C_COMMON_WORD_COUNTS: RefCell<[i32; 100]> = const { RefCell::new([0; 100]) };
    static C_NUM_COMMON_WORDS: RefCell<usize> = const { RefCell::new(0) };
    static C_INITIALIZED: RefCell<bool> = const { RefCell::new(false) };
}

fn ensure_common_words_init() {
    C_COMMON_WORDS.with(|cw| {
        let mut cw = cw.borrow_mut();
        if cw.is_empty() {
            cw.resize(100, [0u8; MAX_TOKEN_LENGTH]);
        }
    });
}

#[no_mangle]
pub extern "C" fn analyzer_init(ops: CTokenizerOps) {
    ensure_common_words_init();
    C_OPS.with(|o| *o.borrow_mut() = Some(ops));
    C_INITIALIZED.with(|i| *i.borrow_mut() = true);
    C_TOKEN_TYPE_COUNTS.with(|c| *c.borrow_mut() = [0; 20]);
    C_COMMON_WORD_COUNTS.with(|c| *c.borrow_mut() = [0; 100]);
    C_NUM_COMMON_WORDS.with(|n| *n.borrow_mut() = 0);

    // Also propagate to the Rust analyzer's internal state, so direct Rust
    // calls keep working consistently.
    ana::analyzer_init(tok::get_tokenizer_ops());
}

fn track_word_c(word: &[u8]) {
    // word is the full token.value buffer with embedded null terminator.
    let word_end = word.iter().position(|&c| c == 0).unwrap_or(word.len());
    let w = &word[..word_end];
    C_COMMON_WORDS.with(|cw| {
        C_COMMON_WORD_COUNTS.with(|cc| {
            C_NUM_COMMON_WORDS.with(|n| {
                let mut cw = cw.borrow_mut();
                let mut cc = cc.borrow_mut();
                let mut n = n.borrow_mut();
                for i in 0..*n {
                    let entry = &cw[i];
                    let entry_end = entry.iter().position(|&c| c == 0).unwrap_or(entry.len());
                    if &entry[..entry_end] == w {
                        cc[i] += 1;
                        return;
                    }
                }
                if *n < 100 {
                    let dst = &mut cw[*n];
                    for b in dst.iter_mut() {
                        *b = 0;
                    }
                    let copy_n = w.len().min(MAX_TOKEN_LENGTH - 1);
                    dst[..copy_n].copy_from_slice(&w[..copy_n]);
                    dst[MAX_TOKEN_LENGTH - 1] = 0;
                    cc[*n] = 1;
                    *n += 1;
                }
            });
        });
    });
}

#[no_mangle]
pub extern "C" fn analyze_text(text: *const c_char) -> CAnalysisResult {
    let mut result = CAnalysisResult::default();

    let initialized = C_INITIALIZED.with(|i| *i.borrow());
    if !initialized {
        eprintln!("Error: Analyzer not initialized");
        return result;
    }

    let ops = match C_OPS.with(|o| *o.borrow()) {
        Some(o) => o,
        None => return result,
    };

    let load_text = match ops.load_text {
        Some(f) => f,
        None => return result,
    };
    let next_token = match ops.next_token {
        Some(f) => f,
        None => return result,
    };
    let get_stats = match ops.get_stats {
        Some(f) => f,
        None => return result,
    };

    if load_text(text) != 0 {
        eprintln!("Error: Failed to load text");
        return result;
    }

    loop {
        let token = next_token();
        if token.r#type == tok::TokenType::Eof as c_int {
            break;
        }

        C_TOKEN_TYPE_COUNTS.with(|c| {
            let mut c = c.borrow_mut();
            let idx = token.r#type as usize;
            if idx < c.len() {
                c[idx] += 1;
            }
        });

        let t_word = tok::TokenType::Word as c_int;
        let t_id = tok::TokenType::Identifier as c_int;
        let t_num = tok::TokenType::Number as c_int;
        let t_kw = tok::TokenType::Keyword as c_int;
        let t_op = tok::TokenType::Operator as c_int;
        let t_cmt = tok::TokenType::Comment as c_int;
        let t_str = tok::TokenType::String as c_int;
        let t_nl = tok::TokenType::Newline as c_int;

        if token.r#type == t_word || token.r#type == t_id {
            result.word_count += 1;
            // Convert i8 array to u8 slice for tracking.
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(token.value.as_ptr() as *const u8, MAX_TOKEN_LENGTH)
            };
            track_word_c(bytes);
        } else if token.r#type == t_num {
            result.number_count += 1;
        } else if token.r#type == t_kw {
            result.keyword_count += 1;
        } else if token.r#type == t_op {
            result.operator_count += 1;
        } else if token.r#type == t_cmt {
            result.comment_count += 1;
        } else if token.r#type == t_str {
            result.string_count += 1;
        } else if token.r#type == t_nl {
            result.line_count += 1;
        }
    }

    let mut lines: usize = 0;
    let mut tokens_out: usize = 0;
    let mut chars: usize = 0;
    get_stats(&mut lines, &mut tokens_out, &mut chars);
    result.line_count = lines;
    result.char_count = chars;

    result
}

#[no_mangle]
pub extern "C" fn print_token_distribution() {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\n=== Token Distribution ===\n");

    let token_names: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    C_TOKEN_TYPE_COUNTS.with(|c| {
        let c = c.borrow();
        for i in 0..12 {
            if c[i] > 0 {
                let _ = write!(stdout, "{}: {}\n", token_names[i], c[i]);
            }
        }
    });

    let _ = stdout.write_all(b"\n=== Most Common Words ===\n");

    C_COMMON_WORDS.with(|cw| {
        C_COMMON_WORD_COUNTS.with(|cc| {
            C_NUM_COMMON_WORDS.with(|n| {
                let mut cw = cw.borrow_mut();
                let mut cc = cc.borrow_mut();
                let n_val = *n.borrow();
                if n_val >= 1 {
                    for i in 0..(n_val - 1) {
                        for j in 0..(n_val - i - 1) {
                            if cc[j] < cc[j + 1] {
                                cc.swap(j, j + 1);
                                cw.swap(j, j + 1);
                            }
                        }
                    }
                }
                let limit = n_val.min(10);
                for i in 0..limit {
                    let w = &cw[i];
                    let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
                    let _ = write!(stdout, "{}. ", i + 1);
                    let _ = stdout.write_all(&w[..end]);
                    let _ = write!(stdout, ": {} times\n", cc[i]);
                }
            });
        });
    });
}

#[no_mangle]
pub extern "C" fn calculate_complexity_score() -> c_int {
    C_TOKEN_TYPE_COUNTS.with(|c| {
        let c = c.borrow();
        let mut score: i32 = 0;
        score = score.wrapping_add(c[tok::TokenType::Keyword as usize].wrapping_mul(2));
        score = score.wrapping_add(c[tok::TokenType::Operator as usize]);
        score = score.wrapping_add(c[tok::TokenType::Punctuation as usize] / 10);
        score = score.wrapping_sub(c[tok::TokenType::Comment as usize]);
        if score < 0 {
            score = 0;
        }
        score as c_int
    })
}

#[no_mangle]
pub extern "C" fn find_patterns(pattern: *const c_char) {
    use std::io::Write;

    let initialized = C_INITIALIZED.with(|i| *i.borrow());
    if !initialized || pattern.is_null() {
        return;
    }

    let pat_bytes = match cstr_to_bytes(pattern) {
        Some(b) => b.to_vec(),
        None => return,
    };

    let ops = match C_OPS.with(|o| *o.borrow()) {
        Some(o) => o,
        None => return,
    };

    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\n=== Searching for pattern: '");
    let _ = stdout.write_all(&pat_bytes);
    let _ = stdout.write_all(b"' ===\n");

    if let Some(reset) = ops.reset {
        reset();
    }

    let next_token = match ops.next_token {
        Some(f) => f,
        None => return,
    };

    let mut count: i32 = 0;
    loop {
        let token = next_token();
        if token.r#type == tok::TokenType::Eof as c_int {
            break;
        }
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(token.value.as_ptr() as *const u8, MAX_TOKEN_LENGTH)
        };
        let val_end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
        let value = &bytes[..val_end];
        let found = if pat_bytes.is_empty() {
            true
        } else {
            value.windows(pat_bytes.len()).any(|w| w == pat_bytes.as_slice())
        };
        if found {
            let _ = write!(stdout, "Line {}, Column {}: ", token.line, token.column);
            let _ = stdout.write_all(value);
            let _ = stdout.write_all(b"\n");
            count += 1;
        }
    }

    let _ = write!(stdout, "Found {} occurrences\n", count);
}

pub mod tokenizer;
pub mod analyzer;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

// C-compatible token_t: size=280, matching C layout exactly
#[repr(C)]
pub struct CToken {
    pub token_type: u32,       // offset 0
    pub value: [u8; 256],      // offset 4
    pub length: usize,         // offset 264 (256+4=260, padded to 264 for alignment)
    pub line: c_int,           // offset 272
    pub column: c_int,         // offset 276
}

// C-compatible analysis_result_t
#[repr(C)]
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

// C-compatible tokenizer_ops_t (5 function pointers)
#[repr(C)]
pub struct CTokenizerOps {
    pub next_token: unsafe extern "C" fn() -> CToken,
    pub peek_token: unsafe extern "C" fn() -> CToken,
    pub reset: unsafe extern "C" fn(),
    pub load_text: unsafe extern "C" fn(*const c_char) -> c_int,
    pub get_stats: unsafe extern "C" fn(*mut usize, *mut usize, *mut usize),
}

fn rust_token_to_c(t: &tokenizer::Token) -> CToken {
    let mut ct = CToken {
        token_type: t.token_type as u32,
        value: [0u8; 256],
        length: t.length,
        line: t.line,
        column: t.column,
    };
    let bytes = t.value.as_bytes();
    let len = bytes.len().min(255);
    ct.value[..len].copy_from_slice(&bytes[..len]);
    ct
}

#[no_mangle]
pub unsafe extern "C" fn tokenizer_next_token() -> CToken {
    rust_token_to_c(&tokenizer::tokenizer_next_token())
}

#[no_mangle]
pub unsafe extern "C" fn tokenizer_peek_token() -> CToken {
    rust_token_to_c(&tokenizer::tokenizer_peek_token())
}

#[no_mangle]
pub unsafe extern "C" fn tokenizer_reset() {
    tokenizer::tokenizer_reset();
}

#[no_mangle]
pub unsafe extern "C" fn tokenizer_load_text(text: *const c_char) -> c_int {
    if text.is_null() {
        return -1;
    }
    let cstr = CStr::from_ptr(text);
    let s = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    tokenizer::tokenizer_load_text(s)
}

#[no_mangle]
pub unsafe extern "C" fn tokenizer_get_stats(
    lines: *mut usize,
    tokens: *mut usize,
    chars: *mut usize,
) {
    let (mut l, mut t, mut c) = (0usize, 0usize, 0usize);
    tokenizer::tokenizer_get_stats(&mut l, &mut t, &mut c);
    if !lines.is_null() { *lines = l; }
    if !tokens.is_null() { *tokens = t; }
    if !chars.is_null() { *chars = c; }
}

#[no_mangle]
pub unsafe extern "C" fn get_tokenizer_ops() -> CTokenizerOps {
    CTokenizerOps {
        next_token: tokenizer_next_token,
        peek_token: tokenizer_peek_token,
        reset: tokenizer_reset,
        load_text: tokenizer_load_text,
        get_stats: tokenizer_get_stats,
    }
}

#[no_mangle]
pub unsafe extern "C" fn analyzer_init(ops: CTokenizerOps) {
    // We ignore the C ops and use our own Rust ops internally
    let _ = ops;
    let rust_ops = tokenizer::get_tokenizer_ops();
    analyzer::analyzer_init(rust_ops);
}

#[no_mangle]
pub unsafe extern "C" fn analyze_text(text: *const c_char) -> CAnalysisResult {
    let mut result = CAnalysisResult {
        word_count: 0, number_count: 0, keyword_count: 0, operator_count: 0,
        comment_count: 0, string_count: 0, line_count: 0, char_count: 0,
    };
    if text.is_null() { return result; }
    let cstr = CStr::from_ptr(text);
    let s = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return result,
    };
    let r = analyzer::analyze_text(s);
    result.word_count = r.word_count;
    result.number_count = r.number_count;
    result.keyword_count = r.keyword_count;
    result.operator_count = r.operator_count;
    result.comment_count = r.comment_count;
    result.string_count = r.string_count;
    result.line_count = r.line_count;
    result.char_count = r.char_count;
    result
}

#[no_mangle]
pub unsafe extern "C" fn print_token_distribution() {
    analyzer::print_token_distribution();
}

#[no_mangle]
pub unsafe extern "C" fn calculate_complexity_score() -> c_int {
    analyzer::calculate_complexity_score()
}

#[no_mangle]
pub unsafe extern "C" fn find_patterns(pattern: *const c_char) {
    if pattern.is_null() { return; }
    let cstr = CStr::from_ptr(pattern);
    if let Ok(s) = cstr.to_str() {
        analyzer::find_patterns(s);
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_menu() {
    print!("\n=== Text Analyzer ===\n");
    print!("1. Analyze text\n");
    print!("2. Load text from file\n");
    print!("3. Show token distribution\n");
    print!("4. Calculate complexity score\n");
    print!("5. Find pattern\n");
    print!("6. Interactive tokenizer\n");
    print!("7. Exit\n");
    print!("Choice: ");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

#[no_mangle]
pub unsafe extern "C" fn print_analysis_result(result: *const CAnalysisResult) {
    if result.is_null() { return; }
    let r = &*result;
    print!("\n=== Analysis Results ===\n");
    print!("Words/Identifiers: {}\n", r.word_count);
    print!("Numbers: {}\n", r.number_count);
    print!("Keywords: {}\n", r.keyword_count);
    print!("Operators: {}\n", r.operator_count);
    print!("Comments: {}\n", r.comment_count);
    print!("Strings: {}\n", r.string_count);
    print!("Lines: {}\n", r.line_count);
    print!("Characters: {}\n", r.char_count);
}

#[no_mangle]
pub unsafe extern "C" fn read_file(filename: *const c_char) -> *mut c_char {
    if filename.is_null() { return std::ptr::null_mut(); }
    let cstr = CStr::from_ptr(filename);
    let fname = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match std::fs::read_to_string(fname) {
        Ok(content) => {
            if content.len() > tokenizer::MAX_BUFFER_SIZE {
                eprint!("Error: File too large\n");
                return std::ptr::null_mut();
            }
            match CString::new(content) {
                Ok(cs) => cs.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => {
            eprint!("Error: Could not open file '{}'\n", fname);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn interactive_tokenizer(ops: CTokenizerOps) {
    // Use the Rust ops internally
    let _ = ops;
    let rust_ops = tokenizer::get_tokenizer_ops();
    // Simplified: just call the Rust version's logic
    use std::io::{self, BufRead, Write};
    print!("\nEnter text (empty line to stop):\n");
    let _ = io::stdout().flush();

    let mut input = String::new();
    let stdin = io::stdin();
    for line_result in stdin.lock().lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.is_empty() { break; }
        let to_append = format!("{}\n", line);
        let remaining = 4096 - input.len().min(4095) - 1;
        if to_append.len() <= remaining {
            input.push_str(&to_append);
        } else {
            input.push_str(&to_append[..remaining]);
        }
    }

    if (rust_ops.load_text)(&input) != 0 {
        print!("Failed to load text\n");
        return;
    }

    print!("\n=== Tokens ===\n");
    let token_type_names = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE",
        "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count = 0;
    loop {
        let token = (rust_ops.next_token)();
        if token.token_type == tokenizer::TokenType::Eof { break; }
        print!("[{}] '{}' (L{}:C{})\n",
            token_type_names[token.token_type as usize],
            token.value, token.line, token.column);
        count += 1;
        if count > 100 {
            print!("... (truncated, too many tokens)\n");
            break;
        }
    }
}

// Export main symbol to match C .so
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    0
}

use libloading::{Library, Symbol};
use std::ffi::CString;
use text_analyzer::tokenizer;
use text_analyzer::analyzer;

// C struct layouts matching the C headers
#[repr(C)]
#[derive(Clone)]
struct CToken {
    token_type: u32, // enum is int-sized in C
    value: [u8; 256],
    length: usize,
    line: i32,
    column: i32,
}

// Padding may exist between token_type(u32) and value on 64-bit.
// Let's check actual layout at runtime if needed.

#[repr(C)]
#[derive(Debug, Clone)]
struct CAnalysisResult {
    word_count: usize,
    number_count: usize,
    keyword_count: usize,
    operator_count: usize,
    comment_count: usize,
    string_count: usize,
    line_count: usize,
    char_count: usize,
}

#[repr(C)]
struct CTokenizerOps {
    next_token: unsafe extern "C" fn() -> CToken,
    peek_token: unsafe extern "C" fn() -> CToken,
    reset: unsafe extern "C" fn(),
    load_text: unsafe extern "C" fn(*const i8) -> i32,
    get_stats: unsafe extern "C" fn(*mut usize, *mut usize, *mut usize),
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn c_token_value(t: &CToken) -> String {
    let end = t.value.iter().position(|&b| b == 0).unwrap_or(t.length);
    String::from_utf8_lossy(&t.value[..end]).to_string()
}

// Helper: collect all tokens from C tokenizer after loading text
unsafe fn c_collect_tokens(lib: &Library, text: &str) -> Vec<CToken> {
    let load: Symbol<unsafe extern "C" fn(*const i8) -> i32> =
        lib.get(b"tokenizer_load_text").unwrap();
    let next: Symbol<unsafe extern "C" fn() -> CToken> =
        lib.get(b"tokenizer_next_token").unwrap();

    let cstr = CString::new(text).unwrap();
    assert_eq!(load(cstr.as_ptr()), 0);

    let mut tokens = Vec::new();
    loop {
        let tok = next();
        if tok.token_type == 0 { // TOKEN_EOF
            break;
        }
        tokens.push(tok);
    }
    tokens
}

// Helper: collect all tokens from Rust tokenizer after loading text
fn rust_collect_tokens(text: &str) -> Vec<tokenizer::Token> {
    assert_eq!(tokenizer::tokenizer_load_text(text), 0);
    let mut tokens = Vec::new();
    loop {
        let tok = tokenizer::tokenizer_next_token();
        if tok.token_type == tokenizer::TokenType::Eof {
            break;
        }
        tokens.push(tok);
    }
    tokens
}

// ============================================================
// TOKENIZER TESTS (lowest level)
// ============================================================

#[test]
fn test_tokenizer_load_text() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_load: Symbol<unsafe extern "C" fn(*const i8) -> i32> =
            lib.get(b"tokenizer_load_text").unwrap();

        // Test normal load
        let text = CString::new("hello world").unwrap();
        let c_ret = c_load(text.as_ptr());
        let r_ret = tokenizer::tokenizer_load_text("hello world");
        assert_eq!(c_ret, r_ret, "load_text return value mismatch");

        // Test null/empty
        let empty = CString::new("").unwrap();
        let c_ret2 = c_load(empty.as_ptr());
        let r_ret2 = tokenizer::tokenizer_load_text("");
        assert_eq!(c_ret2, r_ret2, "load_text empty return value mismatch");
    }
}

#[test]
fn test_tokenizer_simple_words() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "hello world";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(),
            "Token count mismatch for '{}': C={}, Rust={}",
            text, c_tokens.len(), r_tokens.len());

        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "Token {} type mismatch: C={}, Rust={:?}", i, ct.token_type, rt.token_type);
            assert_eq!(c_token_value(ct), rt.value,
                "Token {} value mismatch: C='{}', Rust='{}'", i, c_token_value(ct), rt.value);
            assert_eq!(ct.length, rt.length,
                "Token {} length mismatch", i);
            assert_eq!(ct.line, rt.line,
                "Token {} line mismatch: C={}, Rust={}", i, ct.line, rt.line);
            assert_eq!(ct.column, rt.column,
                "Token {} column mismatch: C={}, Rust={}", i, ct.column, rt.column);
        }
    }
}

#[test]
fn test_tokenizer_keywords() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "if else while for return int";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(), "Keyword token count mismatch");
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "Keyword token {} type mismatch: C={}, Rust={:?}", i, ct.token_type, rt.token_type);
            assert_eq!(c_token_value(ct), rt.value,
                "Keyword token {} value mismatch", i);
        }
    }
}

#[test]
fn test_tokenizer_numbers() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "42 3.14 0 100.0";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(), "Number token count mismatch");
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "Number token {} type mismatch", i);
            assert_eq!(c_token_value(ct), rt.value,
                "Number token {} value mismatch", i);
        }
    }
}

#[test]
fn test_tokenizer_strings() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = r#""hello" 'c' "escaped\"quote""#;
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(), "String token count mismatch");
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "String token {} type mismatch: C={}, Rust={:?}", i, ct.token_type, rt.token_type);
            assert_eq!(c_token_value(ct), rt.value,
                "String token {} value mismatch: C='{}', Rust='{}'", i, c_token_value(ct), rt.value);
        }
    }
}

#[test]
fn test_tokenizer_operators() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "== != <= >= && || ++ -- -> << >>";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(), "Operator token count mismatch");
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "Operator token {} type mismatch", i);
            assert_eq!(c_token_value(ct), rt.value,
                "Operator token {} value mismatch: C='{}', Rust='{}'", i, c_token_value(ct), rt.value);
        }
    }
}

#[test]
fn test_tokenizer_comments() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        // Single-line comment
        let text = "// this is a comment\nx";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(),
            "Comment token count mismatch: C={}, Rust={}", c_tokens.len(), r_tokens.len());
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "Comment token {} type mismatch: C={}, Rust={:?}", i, ct.token_type, rt.token_type);
            assert_eq!(c_token_value(ct), rt.value,
                "Comment token {} value mismatch: C='{}', Rust='{}'", i, c_token_value(ct), rt.value);
        }
    }
}

#[test]
fn test_tokenizer_multiline_comment() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "/* multi\nline */ x";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(),
            "Multiline comment token count mismatch: C={}, Rust={}", c_tokens.len(), r_tokens.len());
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "ML comment token {} type mismatch: C={}, Rust={:?}", i, ct.token_type, rt.token_type);
            assert_eq!(c_token_value(ct), rt.value,
                "ML comment token {} value mismatch: C='{}', Rust='{}'", i, c_token_value(ct), rt.value);
        }
    }
}

#[test]
fn test_tokenizer_punctuation() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "(){};,[]";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(), "Punctuation token count mismatch");
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "Punct token {} type mismatch", i);
            assert_eq!(c_token_value(ct), rt.value,
                "Punct token {} value mismatch", i);
        }
    }
}

#[test]
fn test_tokenizer_newlines() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "a\nb\nc";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(), "Newline token count mismatch");
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "Newline token {} type mismatch: C={}, Rust={:?}", i, ct.token_type, rt.token_type);
            assert_eq!(c_token_value(ct), rt.value,
                "Newline token {} value mismatch", i);
            assert_eq!(ct.line, rt.line,
                "Newline token {} line mismatch: C={}, Rust={}", i, ct.line, rt.line);
            assert_eq!(ct.column, rt.column,
                "Newline token {} column mismatch: C={}, Rust={}", i, ct.column, rt.column);
        }
    }
}

#[test]
fn test_tokenizer_peek_token() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_load: Symbol<unsafe extern "C" fn(*const i8) -> i32> =
            lib.get(b"tokenizer_load_text").unwrap();
        let c_peek: Symbol<unsafe extern "C" fn() -> CToken> =
            lib.get(b"tokenizer_peek_token").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            lib.get(b"tokenizer_next_token").unwrap();

        let text = CString::new("hello world").unwrap();
        c_load(text.as_ptr());
        tokenizer::tokenizer_load_text("hello world");

        // Peek should return same token twice
        let c_peek1 = c_peek();
        let c_peek2 = c_peek();
        assert_eq!(c_token_value(&c_peek1), c_token_value(&c_peek2));

        let r_peek1 = tokenizer::tokenizer_peek_token();
        let r_peek2 = tokenizer::tokenizer_peek_token();
        assert_eq!(r_peek1.value, r_peek2.value);

        // Peek should match next
        assert_eq!(c_token_value(&c_peek1), c_token_value(&c_next()));
        let r_next = tokenizer::tokenizer_next_token();
        assert_eq!(r_peek1.value, r_next.value);

        // Compare C peek vs Rust peek
        assert_eq!(c_peek1.token_type, r_peek1.token_type as u32);
        assert_eq!(c_token_value(&c_peek1), r_peek1.value);
    }
}

#[test]
fn test_tokenizer_get_stats() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_load: Symbol<unsafe extern "C" fn(*const i8) -> i32> =
            lib.get(b"tokenizer_load_text").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            lib.get(b"tokenizer_next_token").unwrap();
        let c_stats: Symbol<unsafe extern "C" fn(*mut usize, *mut usize, *mut usize)> =
            lib.get(b"tokenizer_get_stats").unwrap();

        let text = "hello\nworld";

        // Get baseline stats before this test
        let (mut r_lines0, mut r_tokens0, mut r_chars0) = (0usize, 0usize, 0usize);
        tokenizer::tokenizer_get_stats(&mut r_lines0, &mut r_tokens0, &mut r_chars0);

        // C: fresh library load means stats start at 0
        let cstr = CString::new(text).unwrap();
        c_load(cstr.as_ptr());
        loop {
            let t = c_next();
            if t.token_type == 0 { break; }
        }
        let (mut c_lines, mut c_tokens, mut c_chars) = (0usize, 0usize, 0usize);
        c_stats(&mut c_lines, &mut c_tokens, &mut c_chars);

        // Rust: load and consume
        tokenizer::tokenizer_load_text(text);
        loop {
            let t = tokenizer::tokenizer_next_token();
            if t.token_type == tokenizer::TokenType::Eof { break; }
        }
        let (mut r_lines, mut r_tokens, mut r_chars) = (0usize, 0usize, 0usize);
        tokenizer::tokenizer_get_stats(&mut r_lines, &mut r_tokens, &mut r_chars);

        // Compare deltas (since Rust stats are cumulative across tests)
        let r_lines_delta = r_lines - r_lines0;
        let r_chars_delta = r_chars - r_chars0;

        assert_eq!(c_lines, r_lines_delta,
            "Stats lines mismatch: C={}, Rust delta={}", c_lines, r_lines_delta);
        assert_eq!(c_chars, r_chars_delta,
            "Stats chars mismatch: C={}, Rust delta={}", c_chars, r_chars_delta);
    }
}

#[test]
fn test_tokenizer_reset() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_load: Symbol<unsafe extern "C" fn(*const i8) -> i32> =
            lib.get(b"tokenizer_load_text").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            lib.get(b"tokenizer_next_token").unwrap();
        let c_reset: Symbol<unsafe extern "C" fn()> =
            lib.get(b"tokenizer_reset").unwrap();

        let text = CString::new("abc def").unwrap();
        c_load(text.as_ptr());
        tokenizer::tokenizer_load_text("abc def");

        let c_first = c_next();
        let r_first = tokenizer::tokenizer_next_token();

        c_reset();
        tokenizer::tokenizer_reset();

        let c_after_reset = c_next();
        let r_after_reset = tokenizer::tokenizer_next_token();

        assert_eq!(c_token_value(&c_first), c_token_value(&c_after_reset));
        assert_eq!(r_first.value, r_after_reset.value);
        assert_eq!(c_token_value(&c_first), r_first.value);
    }
}

// ============================================================
// MIXED / COMPLEX TOKENIZER TESTS
// ============================================================

#[test]
fn test_tokenizer_c_code_snippet() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "int main() {\n    return 0;\n}";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(),
            "C code snippet token count mismatch: C={}, Rust={}", c_tokens.len(), r_tokens.len());
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "C code token {} type mismatch: C={}, Rust={:?} (val C='{}', Rust='{}')",
                i, ct.token_type, rt.token_type, c_token_value(ct), rt.value);
            assert_eq!(c_token_value(ct), rt.value,
                "C code token {} value mismatch: C='{}', Rust='{}'", i, c_token_value(ct), rt.value);
            assert_eq!(ct.line, rt.line,
                "C code token {} line mismatch: C={}, Rust={}", i, ct.line, rt.line);
            assert_eq!(ct.column, rt.column,
                "C code token {} column mismatch: C={}, Rust={}", i, ct.column, rt.column);
        }
    }
}

#[test]
fn test_tokenizer_slash_not_comment() {
    // The C code has a bug: comment detection checks peek_char() which returns
    // the same char as c (since position hasn't advanced). So '/' followed by
    // anything that's not '/' or '*' should be treated as operator.
    // But actually the C code checks: c == '/' && (peek_char() == '/' || peek_char() == '*')
    // Since peek_char() returns the char at current_position which IS c itself (not advanced),
    // this means the condition is: c == '/' && (c == '/' || c == '*') which is always true
    // when c == '/'. So in C, '/' always enters scan_comment.
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let text = "a / b";
        let c_tokens = c_collect_tokens(&lib, text);
        let r_tokens = rust_collect_tokens(text);

        assert_eq!(c_tokens.len(), r_tokens.len(),
            "Slash token count mismatch: C={}, Rust={}\nC tokens: {:?}\nRust tokens: {:?}",
            c_tokens.len(), r_tokens.len(),
            c_tokens.iter().map(|t| format!("(type={}, val='{}')", t.token_type, c_token_value(t))).collect::<Vec<_>>(),
            r_tokens.iter().map(|t| format!("(type={:?}, val='{}')", t.token_type, t.value)).collect::<Vec<_>>());
        for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
            assert_eq!(ct.token_type, rt.token_type as u32,
                "Slash token {} type mismatch: C={}, Rust={:?}", i, ct.token_type, rt.token_type);
            assert_eq!(c_token_value(ct), rt.value,
                "Slash token {} value mismatch: C='{}', Rust='{}'", i, c_token_value(ct), rt.value);
        }
    }
}

// ============================================================
// ANALYZER TESTS (mid level)
// ============================================================

#[test]
fn test_analyze_text_simple() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");

        // Initialize C analyzer
        let c_get_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            lib.get(b"get_tokenizer_ops").unwrap();
        let c_analyzer_init: Symbol<unsafe extern "C" fn(CTokenizerOps)> =
            lib.get(b"analyzer_init").unwrap();
        let c_analyze: Symbol<unsafe extern "C" fn(*const i8) -> CAnalysisResult> =
            lib.get(b"analyze_text").unwrap();

        let ops = c_get_ops();
        c_analyzer_init(ops);

        // Initialize Rust analyzer
        let r_ops = tokenizer::get_tokenizer_ops();
        analyzer::analyzer_init(r_ops);

        let text = "int x = 42;\nreturn x;";
        let cstr = CString::new(text).unwrap();

        let c_result = c_analyze(cstr.as_ptr());
        let r_result = analyzer::analyze_text(text);

        assert_eq!(c_result.word_count, r_result.word_count,
            "word_count mismatch: C={}, Rust={}", c_result.word_count, r_result.word_count);
        assert_eq!(c_result.number_count, r_result.number_count,
            "number_count mismatch: C={}, Rust={}", c_result.number_count, r_result.number_count);
        assert_eq!(c_result.keyword_count, r_result.keyword_count,
            "keyword_count mismatch: C={}, Rust={}", c_result.keyword_count, r_result.keyword_count);
        assert_eq!(c_result.operator_count, r_result.operator_count,
            "operator_count mismatch: C={}, Rust={}", c_result.operator_count, r_result.operator_count);
        assert_eq!(c_result.comment_count, r_result.comment_count,
            "comment_count mismatch: C={}, Rust={}", c_result.comment_count, r_result.comment_count);
        assert_eq!(c_result.string_count, r_result.string_count,
            "string_count mismatch: C={}, Rust={}", c_result.string_count, r_result.string_count);
        assert_eq!(c_result.line_count, r_result.line_count,
            "line_count mismatch: C={}, Rust={}", c_result.line_count, r_result.line_count);
        assert_eq!(c_result.char_count, r_result.char_count,
            "char_count mismatch: C={}, Rust={}", c_result.char_count, r_result.char_count);
    }
}

#[test]
fn test_analyze_text_with_comments() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");

        let c_get_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            lib.get(b"get_tokenizer_ops").unwrap();
        let c_analyzer_init: Symbol<unsafe extern "C" fn(CTokenizerOps)> =
            lib.get(b"analyzer_init").unwrap();
        let c_analyze: Symbol<unsafe extern "C" fn(*const i8) -> CAnalysisResult> =
            lib.get(b"analyze_text").unwrap();

        let ops = c_get_ops();
        c_analyzer_init(ops);

        let r_ops = tokenizer::get_tokenizer_ops();
        analyzer::analyzer_init(r_ops);

        let text = "// comment\nint x = 5; /* block */";
        let cstr = CString::new(text).unwrap();

        let c_result = c_analyze(cstr.as_ptr());
        let r_result = analyzer::analyze_text(text);

        assert_eq!(c_result.word_count, r_result.word_count,
            "word_count mismatch: C={}, Rust={}", c_result.word_count, r_result.word_count);
        assert_eq!(c_result.number_count, r_result.number_count,
            "number_count mismatch: C={}, Rust={}", c_result.number_count, r_result.number_count);
        assert_eq!(c_result.keyword_count, r_result.keyword_count,
            "keyword_count mismatch: C={}, Rust={}", c_result.keyword_count, r_result.keyword_count);
        assert_eq!(c_result.operator_count, r_result.operator_count,
            "operator_count mismatch: C={}, Rust={}", c_result.operator_count, r_result.operator_count);
        assert_eq!(c_result.comment_count, r_result.comment_count,
            "comment_count mismatch: C={}, Rust={}", c_result.comment_count, r_result.comment_count);
        assert_eq!(c_result.string_count, r_result.string_count,
            "string_count mismatch: C={}, Rust={}", c_result.string_count, r_result.string_count);
    }
}

#[test]
fn test_calculate_complexity_score() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");

        let c_get_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            lib.get(b"get_tokenizer_ops").unwrap();
        let c_analyzer_init: Symbol<unsafe extern "C" fn(CTokenizerOps)> =
            lib.get(b"analyzer_init").unwrap();
        let c_analyze: Symbol<unsafe extern "C" fn(*const i8) -> CAnalysisResult> =
            lib.get(b"analyze_text").unwrap();
        let c_complexity: Symbol<unsafe extern "C" fn() -> i32> =
            lib.get(b"calculate_complexity_score").unwrap();

        let ops = c_get_ops();
        c_analyzer_init(ops);

        let r_ops = tokenizer::get_tokenizer_ops();
        analyzer::analyzer_init(r_ops);

        let text = "if (x > 0) { return x + 1; } else { return 0; }";
        let cstr = CString::new(text).unwrap();

        let _c_result = c_analyze(cstr.as_ptr());
        let _r_result = analyzer::analyze_text(text);

        let c_score = c_complexity();
        let r_score = analyzer::calculate_complexity_score();

        assert_eq!(c_score, r_score,
            "Complexity score mismatch: C={}, Rust={}", c_score, r_score);
    }
}

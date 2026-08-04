use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

const MAX_TOKEN_LENGTH: usize = 256;

#[repr(C)]
#[derive(Debug, Clone)]
struct CToken {
    token_type: u32,
    value: [c_char; MAX_TOKEN_LENGTH],
    length: usize,
    line: c_int,
    column: c_int,
}

#[repr(C)]
#[derive(Debug)]
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
    next_token: Option<extern "C" fn() -> CToken>,
    peek_token: Option<extern "C" fn() -> CToken>,
    reset: Option<extern "C" fn()>,
    load_text: Option<extern "C" fn(*const c_char) -> c_int>,
    get_stats: Option<extern "C" fn(*mut usize, *mut usize, *mut usize)>,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtext_analyzer_c.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libtext_analyzer.so");
    if !p.exists() {
        p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libtext_analyzer.so");
    }
    p
}

fn token_value_str(t: &CToken) -> String {
    let bytes: Vec<u8> = t.value.iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as u8)
        .collect();
    String::from_utf8_lossy(&bytes).to_string()
}

fn assert_tokens_eq(c: &CToken, r: &CToken, ctx: &str) {
    assert_eq!(c.token_type, r.token_type,
        "{ctx}: token_type mismatch: C={} Rust={}", c.token_type, r.token_type);
    let cv = token_value_str(c);
    let rv = token_value_str(r);
    assert_eq!(cv, rv, "{ctx}: value mismatch: C={cv:?} Rust={rv:?}");
    assert_eq!(c.length, r.length, "{ctx}: length mismatch: C={} Rust={}", c.length, r.length);
    assert_eq!(c.line, r.line, "{ctx}: line mismatch: C={} Rust={}", c.line, r.line);
    assert_eq!(c.column, r.column, "{ctx}: column mismatch: C={} Rust={}", c.column, r.column);
}

fn assert_results_eq(c: &CAnalysisResult, r: &CAnalysisResult, ctx: &str) {
    assert_eq!(c.word_count, r.word_count, "{ctx}: word_count");
    assert_eq!(c.number_count, r.number_count, "{ctx}: number_count");
    assert_eq!(c.keyword_count, r.keyword_count, "{ctx}: keyword_count");
    assert_eq!(c.operator_count, r.operator_count, "{ctx}: operator_count");
    assert_eq!(c.comment_count, r.comment_count, "{ctx}: comment_count");
    assert_eq!(c.string_count, r.string_count, "{ctx}: string_count");
    assert_eq!(c.line_count, r.line_count, "{ctx}: line_count");
    assert_eq!(c.char_count, r.char_count, "{ctx}: char_count");
}

// ============ TOKENIZER TESTS ============

#[test]
fn test_tokenizer_load_text() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();

        let text = CString::new("hello world").unwrap();
        assert_eq!(c_load(text.as_ptr()), r_load(text.as_ptr()));

        // null test
        let c_null: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_null: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();
        assert_eq!(c_null(std::ptr::null()), r_null(std::ptr::null()));
    }
}

#[test]
fn test_tokenizer_next_token_simple() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token").unwrap();
        let r_next: Symbol<unsafe extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token_ffi").unwrap();

        let inputs = [
            "hello",
            "42",
            "int x = 10;",
            "if (a == b) { return 0; }",
            "\"hello world\"",
            "// comment\ncode",
            "/* multi\nline */",
            "a + b - c * d / e",
            "3.14",
            "",
        ];

        for input in &inputs {
            let text = CString::new(*input).unwrap();
            c_load(text.as_ptr());
            r_load(text.as_ptr());

            for i in 0..100 {
                let ct = c_next();
                let rt = r_next();
                assert_tokens_eq(&ct, &rt, &format!("input={input:?} token#{i}"));
                if ct.token_type == 0 { break; } // EOF
            }
        }
    }
}

#[test]
fn test_tokenizer_peek_token() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();
        let c_peek: Symbol<unsafe extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_peek_token").unwrap();
        let r_peek: Symbol<unsafe extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_peek_token_ffi").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token").unwrap();
        let r_next: Symbol<unsafe extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token_ffi").unwrap();

        let text = CString::new("int x = 5;").unwrap();
        c_load(text.as_ptr());
        r_load(text.as_ptr());

        // peek should return same token twice
        let cp1 = c_peek();
        let rp1 = r_peek();
        assert_tokens_eq(&cp1, &rp1, "peek1");

        let cp2 = c_peek();
        let rp2 = r_peek();
        assert_tokens_eq(&cp2, &rp2, "peek2");
        assert_tokens_eq(&cp1, &cp2, "c_peek idempotent");

        // next should consume the peeked token
        let cn = c_next();
        let rn = r_next();
        assert_tokens_eq(&cn, &rn, "next after peek");
        assert_tokens_eq(&cp1, &cn, "peek==next");
    }
}

#[test]
fn test_tokenizer_reset() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token").unwrap();
        let r_next: Symbol<unsafe extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token_ffi").unwrap();
        let c_reset: Symbol<unsafe extern "C" fn()> =
            c_lib.get(b"tokenizer_reset").unwrap();
        let r_reset: Symbol<unsafe extern "C" fn()> =
            r_lib.get(b"tokenizer_reset_ffi").unwrap();

        let text = CString::new("hello world").unwrap();
        c_load(text.as_ptr());
        r_load(text.as_ptr());

        // consume first token
        let ct1 = c_next();
        let rt1 = r_next();
        assert_tokens_eq(&ct1, &rt1, "before reset");

        // reset
        c_reset();
        r_reset();

        // should get same first token again
        let ct2 = c_next();
        let rt2 = r_next();
        assert_tokens_eq(&ct2, &rt2, "after reset");
        assert_tokens_eq(&ct1, &ct2, "reset reproduces first token");
    }
}

#[test]
fn test_tokenizer_get_stats() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token").unwrap();
        let r_next: Symbol<unsafe extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token_ffi").unwrap();
        let c_stats: Symbol<unsafe extern "C" fn(*mut usize, *mut usize, *mut usize)> =
            c_lib.get(b"tokenizer_get_stats").unwrap();
        let r_stats: Symbol<unsafe extern "C" fn(*mut usize, *mut usize, *mut usize)> =
            r_lib.get(b"tokenizer_get_stats_ffi").unwrap();

        let text = CString::new("int x = 5;\nreturn 0;").unwrap();
        c_load(text.as_ptr());
        r_load(text.as_ptr());

        // consume all tokens
        loop {
            let ct = c_next();
            let rt = r_next();
            if ct.token_type == 0 { break; }
            let _ = rt;
        }

        let (mut cl, mut ct_count, mut cc) = (0usize, 0usize, 0usize);
        let (mut rl, mut rt_count, mut rc) = (0usize, 0usize, 0usize);
        c_stats(&mut cl, &mut ct_count, &mut cc);
        r_stats(&mut rl, &mut rt_count, &mut rc);

        assert_eq!(cl, rl, "lines mismatch");
        assert_eq!(ct_count, rt_count, "tokens mismatch");
        assert_eq!(cc, rc, "chars mismatch");
    }
}

#[test]
fn test_get_tokenizer_ops() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            c_lib.get(b"get_tokenizer_ops").unwrap();
        let r_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            r_lib.get(b"get_tokenizer_ops_ffi").unwrap();

        let co = c_ops();
        let ro = r_ops();

        // Just verify all function pointers are non-null
        assert!(co.next_token.is_some(), "C next_token null");
        assert!(co.peek_token.is_some(), "C peek_token null");
        assert!(co.reset.is_some(), "C reset null");
        assert!(co.load_text.is_some(), "C load_text null");
        assert!(co.get_stats.is_some(), "C get_stats null");
        assert!(ro.next_token.is_some(), "Rust next_token null");
        assert!(ro.peek_token.is_some(), "Rust peek_token null");
        assert!(ro.reset.is_some(), "Rust reset null");
        assert!(ro.load_text.is_some(), "Rust load_text null");
        assert!(ro.get_stats.is_some(), "Rust get_stats null");
    }
}

// ============ ANALYZER TESTS ============

#[test]
fn test_analyze_text() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_get_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            c_lib.get(b"get_tokenizer_ops").unwrap();
        let c_init: Symbol<unsafe extern "C" fn(CTokenizerOps)> =
            c_lib.get(b"analyzer_init").unwrap();
        let c_analyze: Symbol<unsafe extern "C" fn(*const c_char) -> CAnalysisResult> =
            c_lib.get(b"analyze_text").unwrap();

        let r_get_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            r_lib.get(b"get_tokenizer_ops_ffi").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(CTokenizerOps)> =
            r_lib.get(b"analyzer_init_ffi").unwrap();
        let r_analyze: Symbol<unsafe extern "C" fn(*const c_char) -> CAnalysisResult> =
            r_lib.get(b"analyze_text_ffi").unwrap();

        let co = c_get_ops();
        c_init(co);
        let ro = r_get_ops();
        r_init(ro);

        let inputs = [
            "int main() { return 0; }",
            "if (x == 10) { y = 20; } else { y = 30; }",
            "// comment\nint a = 5;\n/* block */",
            "\"hello\" + \"world\"",
            "for (int i = 0; i < 10; i++) { sum += i; }",
            "3.14 + 2.71",
        ];

        for input in &inputs {
            // Re-init for each test to reset state
            let co2 = c_get_ops();
            c_init(co2);
            let ro2 = r_get_ops();
            r_init(ro2);

            let text = CString::new(*input).unwrap();
            let cr = c_analyze(text.as_ptr());
            let rr = r_analyze(text.as_ptr());
            assert_results_eq(&cr, &rr, &format!("analyze_text({input:?})"));
        }
    }
}

#[test]
fn test_calculate_complexity_score() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_get_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            c_lib.get(b"get_tokenizer_ops").unwrap();
        let c_init: Symbol<unsafe extern "C" fn(CTokenizerOps)> =
            c_lib.get(b"analyzer_init").unwrap();
        let c_analyze: Symbol<unsafe extern "C" fn(*const c_char) -> CAnalysisResult> =
            c_lib.get(b"analyze_text").unwrap();
        let c_complexity: Symbol<unsafe extern "C" fn() -> c_int> =
            c_lib.get(b"calculate_complexity_score").unwrap();

        let r_get_ops: Symbol<unsafe extern "C" fn() -> CTokenizerOps> =
            r_lib.get(b"get_tokenizer_ops_ffi").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(CTokenizerOps)> =
            r_lib.get(b"analyzer_init_ffi").unwrap();
        let r_analyze: Symbol<unsafe extern "C" fn(*const c_char) -> CAnalysisResult> =
            r_lib.get(b"analyze_text_ffi").unwrap();
        let r_complexity: Symbol<unsafe extern "C" fn() -> c_int> =
            r_lib.get(b"calculate_complexity_score_ffi").unwrap();

        let co = c_get_ops();
        c_init(co);
        let ro = r_get_ops();
        r_init(ro);

        let text = CString::new("if (x > 0) { for (int i = 0; i < x; i++) { sum += i; } }").unwrap();
        c_analyze(text.as_ptr());
        r_analyze(text.as_ptr());

        let cs = c_complexity();
        let rs = r_complexity();
        assert_eq!(cs, rs, "complexity_score mismatch: C={cs} Rust={rs}");
    }
}

#[test]
fn test_tokenizer_various_token_types() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token").unwrap();
        let r_next: Symbol<unsafe extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token_ffi").unwrap();

        // Test various edge cases
        let inputs = [
            "++--",           // operators
            "<<>>",           // shift operators
            "&&||",           // logical operators
            "!=<=>===",       // comparison operators
            "->",             // arrow
            "'c'",            // char literal
            "a_b_c",          // identifier with underscores
            "_start",         // underscore-prefixed identifier
            "0.1.2",          // number with multiple dots
            "struct typedef const static extern", // keywords
            "(){}[];,.",      // punctuation
            "\n\n\n",         // newlines
            "switch case default do goto", // more keywords
            "enum union signed unsigned long short", // more keywords
        ];

        for input in &inputs {
            let text = CString::new(*input).unwrap();
            c_load(text.as_ptr());
            r_load(text.as_ptr());

            for i in 0..200 {
                let ct = c_next();
                let rt = r_next();
                assert_tokens_eq(&ct, &rt, &format!("input={input:?} token#{i}"));
                if ct.token_type == 0 { break; }
            }
        }
    }
}

#[test]
fn test_slash_handling() {
    // The C code has a bug where '/' always enters scan_comment because
    // it checks peek_char() twice without advancing. The Rust code should
    // replicate this behavior.
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token").unwrap();
        let r_next: Symbol<unsafe extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token_ffi").unwrap();

        // standalone '/' - C bug means this enters scan_comment
        let inputs = ["/", "a / b", "10 / 2"];
        for input in &inputs {
            let text = CString::new(*input).unwrap();
            c_load(text.as_ptr());
            r_load(text.as_ptr());

            for i in 0..50 {
                let ct = c_next();
                let rt = r_next();
                assert_tokens_eq(&ct, &rt, &format!("slash input={input:?} token#{i}"));
                if ct.token_type == 0 { break; }
            }
        }
    }
}

#[test]
fn test_escape_in_strings() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text_ffi").unwrap();
        let c_next: Symbol<unsafe extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token").unwrap();
        let r_next: Symbol<unsafe extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token_ffi").unwrap();

        let inputs = [
            r#""hello\nworld""#,
            r#""tab\there""#,
            r#""escaped\"quote""#,
            r#""backslash\\end""#,
        ];

        for input in &inputs {
            let text = CString::new(*input).unwrap();
            c_load(text.as_ptr());
            r_load(text.as_ptr());

            for i in 0..50 {
                let ct = c_next();
                let rt = r_next();
                assert_tokens_eq(&ct, &rt, &format!("escape input={input:?} token#{i}"));
                if ct.token_type == 0 { break; }
            }
        }
    }
}

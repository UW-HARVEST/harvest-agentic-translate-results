// Integration tests that compare the C and Rust shared libraries.
// Each test loads BOTH the C .so and the Rust .so via libloading,
// invokes a function on each, and asserts byte-identical results.
//
// Both .so files use thread_local / static state, so we must take care
// to keep the C and Rust state independent: each library has its own
// process-private data, since they're separate libraries.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

const MAX_TOKEN_LENGTH: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
struct CToken {
    r#type: c_int,
    value: [c_char; MAX_TOKEN_LENGTH],
    length: usize,
    line: c_int,
    column: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CTokenizerOps {
    next_token: Option<extern "C" fn() -> CToken>,
    peek_token: Option<extern "C" fn() -> CToken>,
    reset: Option<extern "C" fn()>,
    load_text: Option<extern "C" fn(*const c_char) -> c_int>,
    get_stats: Option<extern "C" fn(*mut usize, *mut usize, *mut usize)>,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
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

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    // Use the cdylib produced by cargo. The `target` dir is set by
    // cargo at build time; we resolve it relative to the manifest dir.
    let candidates = [
        project_root().join("target/debug/libdriver.so"),
        project_root().join("target/release/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

fn load_libs() -> (Library, Library) {
    let c = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };
    (c, r)
}

fn null_terminated(text: &str) -> Vec<u8> {
    let mut v: Vec<u8> = text.as_bytes().to_vec();
    v.push(0);
    v
}

fn tokens_equal(a: &CToken, b: &CToken) -> bool {
    // Only compare metadata and the meaningful bytes (length bytes + the null
    // terminator). Bytes beyond `length+1` are uninitialized stack memory in
    // the C version, so comparing them would yield false negatives.
    if a.r#type != b.r#type
        || a.length != b.length
        || a.line != b.line
        || a.column != b.column
    {
        return false;
    }
    let n = a.length.min(MAX_TOKEN_LENGTH - 1);
    for i in 0..=n {
        if a.value[i] != b.value[i] {
            return false;
        }
    }
    true
}

fn token_to_string(t: &CToken) -> String {
    let bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(t.value.as_ptr() as *const u8, MAX_TOKEN_LENGTH) };
    let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
    let s = String::from_utf8_lossy(&bytes[..end]).into_owned();
    format!(
        "type={} len={} line={} col={} val={:?}",
        t.r#type, t.length, t.line, t.column, s
    )
}

fn collect_all_tokens(lib: &Library, input: &str) -> Vec<CToken> {
    unsafe {
        let load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            lib.get(b"tokenizer_load_text\0").unwrap();
        let next: Symbol<extern "C" fn() -> CToken> =
            lib.get(b"tokenizer_next_token\0").unwrap();
        let nt = null_terminated(input);
        assert_eq!(load(nt.as_ptr() as *const c_char), 0);
        let mut out = Vec::new();
        loop {
            let t = next();
            let ttype = t.r#type;
            out.push(t);
            if ttype == 0 {
                // EOF
                break;
            }
            if out.len() > 10000 {
                break;
            }
        }
        out
    }
}

fn run_load_and_compare(input: &str) {
    let (c_lib, r_lib) = load_libs();
    let c_tokens = collect_all_tokens(&c_lib, input);
    let r_tokens = collect_all_tokens(&r_lib, input);
    assert_eq!(
        c_tokens.len(),
        r_tokens.len(),
        "token count mismatch for {:?}",
        input
    );
    for (i, (ct, rt)) in c_tokens.iter().zip(r_tokens.iter()).enumerate() {
        assert!(
            tokens_equal(ct, rt),
            "token {} mismatch for input {:?}\n  C: {}\n  R: {}",
            i,
            input,
            token_to_string(ct),
            token_to_string(rt)
        );
    }
}

#[test]
fn test_tokenizer_load_returns_zero_on_valid_input() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text\0").unwrap();
        let r_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text\0").unwrap();
        let nt = null_terminated("hello world");
        assert_eq!(c_load(nt.as_ptr() as *const c_char), 0);
        assert_eq!(r_load(nt.as_ptr() as *const c_char), 0);
    }
}

#[test]
fn test_tokenizer_load_fails_on_null() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text\0").unwrap();
        let r_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text\0").unwrap();
        assert_eq!(c_load(std::ptr::null()), -1);
        assert_eq!(r_load(std::ptr::null()), -1);
    }
}

#[test]
fn test_tokenizer_simple_identifier() {
    run_load_and_compare("hello");
}

#[test]
fn test_tokenizer_keywords() {
    run_load_and_compare("if else while for return int char float");
}

#[test]
fn test_tokenizer_numbers() {
    run_load_and_compare("123 456.789 0 1.0 3.14.15");
}

#[test]
fn test_tokenizer_strings() {
    run_load_and_compare(r#""hello" 'a' "with \"escapes\"" "no_close"#);
}

#[test]
fn test_tokenizer_comments() {
    run_load_and_compare("// single line\n/* multi\nline */ /*");
}

#[test]
fn test_tokenizer_operators() {
    run_load_and_compare("+ - * / % = == != <= >= && || ++ -- -> << >> < > ! & | ^ ~ ? :");
}

#[test]
fn test_tokenizer_punctuation() {
    run_load_and_compare("(){}[];,.");
}

#[test]
fn test_tokenizer_complex_program() {
    run_load_and_compare(
        "int main(void) {\n    int x = 42;\n    if (x > 0) {\n        return x * 2;\n    }\n    return 0;\n}\n",
    );
}

#[test]
fn test_tokenizer_underscore_identifiers() {
    run_load_and_compare("_foo _bar123 __private my_var_2");
}

#[test]
fn test_tokenizer_unknown_chars() {
    run_load_and_compare("@#$");
}

#[test]
fn test_tokenizer_empty() {
    run_load_and_compare("");
}

#[test]
fn test_tokenizer_only_whitespace() {
    run_load_and_compare("   \t  \n  \t\n  ");
}

#[test]
fn test_tokenizer_long_string() {
    let s = "x".repeat(300);
    run_load_and_compare(&format!("\"{}\"", s));
}

#[test]
fn test_tokenizer_peek_then_next() {
    let (c_lib, r_lib) = load_libs();
    let input = "hello world";
    let nt = null_terminated(input);
    unsafe {
        let c_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text\0").unwrap();
        let c_peek: Symbol<extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_peek_token\0").unwrap();
        let c_next: Symbol<extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token\0").unwrap();
        let r_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text\0").unwrap();
        let r_peek: Symbol<extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_peek_token\0").unwrap();
        let r_next: Symbol<extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token\0").unwrap();

        c_load(nt.as_ptr() as *const c_char);
        r_load(nt.as_ptr() as *const c_char);

        // peek twice should return same token
        let cp1 = c_peek();
        let cp2 = c_peek();
        let rp1 = r_peek();
        let rp2 = r_peek();
        assert!(tokens_equal(&cp1, &cp2), "C peek not idempotent");
        assert!(tokens_equal(&rp1, &rp2), "Rust peek not idempotent");
        assert!(tokens_equal(&cp1, &rp1), "C/Rust peek mismatch");

        let cn = c_next();
        let rn = r_next();
        assert!(tokens_equal(&cp1, &cn), "C peek should match next after");
        assert!(tokens_equal(&rp1, &rn), "Rust peek should match next after");

        // next token after peek consumes
        let cn2 = c_next();
        let rn2 = r_next();
        assert!(tokens_equal(&cn2, &rn2), "C/Rust subsequent token mismatch");
    }
}

#[test]
fn test_tokenizer_reset_resets_position() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text\0").unwrap();
        let c_next: Symbol<extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token\0").unwrap();
        let c_reset: Symbol<extern "C" fn()> =
            c_lib.get(b"tokenizer_reset\0").unwrap();
        let r_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text\0").unwrap();
        let r_next: Symbol<extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token\0").unwrap();
        let r_reset: Symbol<extern "C" fn()> =
            r_lib.get(b"tokenizer_reset\0").unwrap();

        let nt = null_terminated("foo bar");
        c_load(nt.as_ptr() as *const c_char);
        r_load(nt.as_ptr() as *const c_char);

        let c1 = c_next();
        let r1 = r_next();
        c_reset();
        r_reset();
        let c2 = c_next();
        let r2 = r_next();
        // After reset, first token type/value should match initial token.
        // Note: line/column are reset and total_chars_processed is not, so
        // the line/column should be the same, but the absolute behavior
        // matches between C and Rust.
        assert!(tokens_equal(&c1, &c2));
        assert!(tokens_equal(&r1, &r2));
        assert!(tokens_equal(&c1, &r1));
    }
}

#[test]
fn test_tokenizer_get_stats_match() {
    let (c_lib, r_lib) = load_libs();
    let input = "int x = 42;\nint y = 100;\n";
    unsafe {
        let c_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"tokenizer_load_text\0").unwrap();
        let c_next: Symbol<extern "C" fn() -> CToken> =
            c_lib.get(b"tokenizer_next_token\0").unwrap();
        let c_stats: Symbol<extern "C" fn(*mut usize, *mut usize, *mut usize)> =
            c_lib.get(b"tokenizer_get_stats\0").unwrap();
        let r_load: Symbol<extern "C" fn(*const c_char) -> c_int> =
            r_lib.get(b"tokenizer_load_text\0").unwrap();
        let r_next: Symbol<extern "C" fn() -> CToken> =
            r_lib.get(b"tokenizer_next_token\0").unwrap();
        let r_stats: Symbol<extern "C" fn(*mut usize, *mut usize, *mut usize)> =
            r_lib.get(b"tokenizer_get_stats\0").unwrap();

        let nt = null_terminated(input);
        c_load(nt.as_ptr() as *const c_char);
        r_load(nt.as_ptr() as *const c_char);

        // Drain
        loop {
            let t = c_next();
            if t.r#type == 0 {
                break;
            }
        }
        loop {
            let t = r_next();
            if t.r#type == 0 {
                break;
            }
        }

        let (mut cl, mut ct, mut cc) = (0usize, 0usize, 0usize);
        let (mut rl, mut rt, mut rc) = (0usize, 0usize, 0usize);
        c_stats(&mut cl, &mut ct, &mut cc);
        r_stats(&mut rl, &mut rt, &mut rc);
        assert_eq!(cl, rl, "lines mismatch");
        assert_eq!(ct, rt, "tokens mismatch");
        assert_eq!(cc, rc, "chars mismatch");
    }
}

#[test]
fn test_get_tokenizer_ops_returns_dispatchable() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            c_lib.get(b"get_tokenizer_ops\0").unwrap();
        let r_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            r_lib.get(b"get_tokenizer_ops\0").unwrap();
        let cops = c_get();
        let rops = r_get();
        // All five function pointers should be set.
        assert!(cops.next_token.is_some());
        assert!(cops.peek_token.is_some());
        assert!(cops.reset.is_some());
        assert!(cops.load_text.is_some());
        assert!(cops.get_stats.is_some());
        assert!(rops.next_token.is_some());
        assert!(rops.peek_token.is_some());
        assert!(rops.reset.is_some());
        assert!(rops.load_text.is_some());
        assert!(rops.get_stats.is_some());
        // Dispatch and compare.
        let nt = null_terminated("foo + bar");
        (cops.load_text.unwrap())(nt.as_ptr() as *const c_char);
        (rops.load_text.unwrap())(nt.as_ptr() as *const c_char);
        loop {
            let ct = (cops.next_token.unwrap())();
            let rt = (rops.next_token.unwrap())();
            assert!(
                tokens_equal(&ct, &rt),
                "ops dispatch token mismatch:\n C: {}\n R: {}",
                token_to_string(&ct),
                token_to_string(&rt)
            );
            if ct.r#type == 0 {
                break;
            }
        }
    }
}

#[test]
fn test_analyze_text_simple() {
    let (c_lib, r_lib) = load_libs();
    let input = "int x = 42;\n";
    unsafe {
        let c_init: Symbol<extern "C" fn(CTokenizerOps)> =
            c_lib.get(b"analyzer_init\0").unwrap();
        let c_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            c_lib.get(b"get_tokenizer_ops\0").unwrap();
        let c_analyze: Symbol<extern "C" fn(*const c_char) -> CAnalysisResult> =
            c_lib.get(b"analyze_text\0").unwrap();

        let r_init: Symbol<extern "C" fn(CTokenizerOps)> =
            r_lib.get(b"analyzer_init\0").unwrap();
        let r_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            r_lib.get(b"get_tokenizer_ops\0").unwrap();
        let r_analyze: Symbol<extern "C" fn(*const c_char) -> CAnalysisResult> =
            r_lib.get(b"analyze_text\0").unwrap();

        c_init(c_get());
        r_init(r_get());

        let nt = null_terminated(input);
        let cr = c_analyze(nt.as_ptr() as *const c_char);
        let rr = r_analyze(nt.as_ptr() as *const c_char);
        assert_eq!(cr, rr, "analyze_text result mismatch");
    }
}

#[test]
fn test_analyze_text_complex_program() {
    let (c_lib, r_lib) = load_libs();
    let input = r#"
// This is a comment
int factorial(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

/* Multi-line
   comment */
int main(void) {
    int x = 5;
    int result = factorial(x);
    char *msg = "result is";
    return result;
}
"#;
    unsafe {
        let c_init: Symbol<extern "C" fn(CTokenizerOps)> =
            c_lib.get(b"analyzer_init\0").unwrap();
        let c_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            c_lib.get(b"get_tokenizer_ops\0").unwrap();
        let c_analyze: Symbol<extern "C" fn(*const c_char) -> CAnalysisResult> =
            c_lib.get(b"analyze_text\0").unwrap();

        let r_init: Symbol<extern "C" fn(CTokenizerOps)> =
            r_lib.get(b"analyzer_init\0").unwrap();
        let r_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            r_lib.get(b"get_tokenizer_ops\0").unwrap();
        let r_analyze: Symbol<extern "C" fn(*const c_char) -> CAnalysisResult> =
            r_lib.get(b"analyze_text\0").unwrap();

        c_init(c_get());
        r_init(r_get());

        let nt = null_terminated(input);
        let cr = c_analyze(nt.as_ptr() as *const c_char);
        let rr = r_analyze(nt.as_ptr() as *const c_char);
        assert_eq!(cr, rr, "analyze_text result mismatch");
    }
}

#[test]
fn test_calculate_complexity_score() {
    let (c_lib, r_lib) = load_libs();
    let input = "if (a && b) { x = y + z; } else if (c) { return; }\n// comment\n";
    unsafe {
        let c_init: Symbol<extern "C" fn(CTokenizerOps)> =
            c_lib.get(b"analyzer_init\0").unwrap();
        let c_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            c_lib.get(b"get_tokenizer_ops\0").unwrap();
        let c_analyze: Symbol<extern "C" fn(*const c_char) -> CAnalysisResult> =
            c_lib.get(b"analyze_text\0").unwrap();
        let c_score: Symbol<extern "C" fn() -> c_int> =
            c_lib.get(b"calculate_complexity_score\0").unwrap();

        let r_init: Symbol<extern "C" fn(CTokenizerOps)> =
            r_lib.get(b"analyzer_init\0").unwrap();
        let r_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            r_lib.get(b"get_tokenizer_ops\0").unwrap();
        let r_analyze: Symbol<extern "C" fn(*const c_char) -> CAnalysisResult> =
            r_lib.get(b"analyze_text\0").unwrap();
        let r_score: Symbol<extern "C" fn() -> c_int> =
            r_lib.get(b"calculate_complexity_score\0").unwrap();

        c_init(c_get());
        r_init(r_get());

        let nt = null_terminated(input);
        let _ = c_analyze(nt.as_ptr() as *const c_char);
        let _ = r_analyze(nt.as_ptr() as *const c_char);

        assert_eq!(c_score(), r_score(), "complexity score mismatch");
    }
}

#[test]
fn test_analyze_text_empty() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_init: Symbol<extern "C" fn(CTokenizerOps)> =
            c_lib.get(b"analyzer_init\0").unwrap();
        let c_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            c_lib.get(b"get_tokenizer_ops\0").unwrap();
        let c_analyze: Symbol<extern "C" fn(*const c_char) -> CAnalysisResult> =
            c_lib.get(b"analyze_text\0").unwrap();

        let r_init: Symbol<extern "C" fn(CTokenizerOps)> =
            r_lib.get(b"analyzer_init\0").unwrap();
        let r_get: Symbol<extern "C" fn() -> CTokenizerOps> =
            r_lib.get(b"get_tokenizer_ops\0").unwrap();
        let r_analyze: Symbol<extern "C" fn(*const c_char) -> CAnalysisResult> =
            r_lib.get(b"analyze_text\0").unwrap();

        c_init(c_get());
        r_init(r_get());

        let nt = null_terminated("");
        let cr = c_analyze(nt.as_ptr() as *const c_char);
        let rr = r_analyze(nt.as_ptr() as *const c_char);
        assert_eq!(cr, rr);
    }
}

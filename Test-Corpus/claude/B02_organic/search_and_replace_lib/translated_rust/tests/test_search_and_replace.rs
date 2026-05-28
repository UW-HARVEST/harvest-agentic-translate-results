use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr, CString};

type SearchAndReplaceFn =
    unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

fn c_lib_path() -> String {
    // CARGO_MANIFEST_DIR points to translated_rust/
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn rust_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // Test runs against the most recently built library. Try both debug and release.
    let release_path = format!("{}/target/release/libdriver.so", manifest);
    let debug_path = format!("{}/target/debug/libdriver.so", manifest);
    if std::path::Path::new(&release_path).exists() {
        release_path
    } else {
        debug_path
    }
}

unsafe fn call_via_lib(
    lib: &Library,
    orig: &str,
    search: &str,
    value: &str,
) -> Option<Vec<u8>> {
    let func: Symbol<SearchAndReplaceFn> = unsafe { lib.get(b"searchAndReplace").unwrap() };
    let c_orig = CString::new(orig).unwrap();
    let c_search = CString::new(search).unwrap();
    let c_value = CString::new(value).unwrap();
    let result = unsafe { func(c_orig.as_ptr(), c_search.as_ptr(), c_value.as_ptr()) };
    if result.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(result).to_bytes().to_vec() };
    // Free the allocated memory using libc::free since both C and Rust use malloc/realloc/strdup.
    unsafe {
        libc::free(result as *mut libc::c_void);
    }
    Some(bytes)
}

fn assert_match(orig: &str, search: &str, value: &str) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("failed to load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("failed to load Rust lib") };

    let c_result = unsafe { call_via_lib(&c_lib, orig, search, value) };
    let rust_result = unsafe { call_via_lib(&rust_lib, orig, search, value) };

    assert_eq!(
        c_result, rust_result,
        "Mismatch for orig={:?}, search={:?}, value={:?}",
        orig, search, value
    );
}

#[test]
fn no_match() {
    assert_match("Hello, world!", "xyz", "ABC");
}

#[test]
fn empty_orig() {
    assert_match("", "xyz", "ABC");
}

#[test]
fn match_at_start() {
    assert_match("Hello, world!", "Hello", "Goodbye");
}

#[test]
fn match_at_end() {
    assert_match("Hello, world!", "world!", "everyone!");
}

#[test]
fn match_in_middle() {
    assert_match("Hello, world!", ", ", " :: ");
}

#[test]
fn multiple_matches() {
    assert_match("ababab", "a", "X");
}

#[test]
fn multiple_matches_overlapping_start() {
    assert_match("aaa", "aa", "b");
}

#[test]
fn replace_with_empty() {
    assert_match("Hello, world!", "world", "");
}

#[test]
fn replace_with_longer() {
    assert_match("foo bar foo", "foo", "longerfoo");
}

#[test]
fn replace_with_shorter() {
    assert_match("longerfoo bar longerfoo", "longerfoo", "foo");
}

#[test]
fn whole_string_match() {
    assert_match("abc", "abc", "xyz");
}

#[test]
fn search_longer_than_orig() {
    assert_match("abc", "abcdef", "xyz");
}

#[test]
fn replace_consecutive() {
    assert_match("abcabcabc", "abc", "X");
}

#[test]
fn match_at_start_only() {
    assert_match("foobar", "foo", "BAZ");
}

#[test]
fn match_at_end_only() {
    assert_match("barfoo", "foo", "BAZ");
}

#[test]
fn many_repetitions() {
    let s: String = "ab".repeat(50);
    assert_match(&s, "ab", "C");
}

#[test]
fn many_repetitions_with_gap() {
    let s: String = "abXY".repeat(50);
    assert_match(&s, "ab", "C");
}

#[test]
fn search_one_char_replace_one_char() {
    assert_match("the quick brown fox jumps over the lazy dog", " ", "_");
}

#[test]
fn unicode_bytes() {
    // Use raw bytes in valid UTF-8; from a byte-level perspective these are just bytes.
    assert_match("héllo wörld héllo", "héllo", "BYE");
}

#[test]
fn search_at_almost_end() {
    assert_match("abcde", "de", "X");
}

#[test]
fn long_string_no_match() {
    let s = "x".repeat(1000);
    assert_match(&s, "y", "z");
}

#[test]
fn long_string_with_matches() {
    let mut s = String::new();
    for _ in 0..50 {
        s.push_str("hello world ");
    }
    assert_match(&s, "world", "WORLD");
}

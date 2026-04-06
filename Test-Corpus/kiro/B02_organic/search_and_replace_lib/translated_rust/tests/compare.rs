use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest)
}

unsafe fn call_c_search_and_replace(lib: &Library, orig: &str, search: &str, value: &str) -> String {
    let func: Symbol<unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char> =
        unsafe { lib.get(b"searchAndReplace\0").unwrap() };
    let o = CString::new(orig).unwrap();
    let s = CString::new(search).unwrap();
    let v = CString::new(value).unwrap();
    let result = unsafe { func(o.as_ptr(), s.as_ptr(), v.as_ptr()) };
    assert!(!result.is_null(), "C returned null for ({:?}, {:?}, {:?})", orig, search, value);
    let out = unsafe { CStr::from_ptr(result) }.to_bytes().to_vec();
    unsafe { libc::free(result as *mut _) };
    String::from_utf8(out).unwrap()
}

fn call_rust_search_and_replace(orig: &str, search: &str, value: &str) -> String {
    let o = CString::new(orig).unwrap();
    let s = CString::new(search).unwrap();
    let v = CString::new(value).unwrap();
    let result = unsafe { driver::searchAndReplace(o.as_ptr(), s.as_ptr(), v.as_ptr()) };
    assert!(!result.is_null(), "Rust returned null for ({:?}, {:?}, {:?})", orig, search, value);
    let out = unsafe { CStr::from_ptr(result) }.to_bytes().to_vec();
    // Rust allocated with std::alloc, we just leak it in tests (small allocations)
    String::from_utf8(out).unwrap()
}

fn check(orig: &str, search: &str, value: &str, lib: &Library) {
    let c_out = unsafe { call_c_search_and_replace(lib, orig, search, value) };
    let r_out = call_rust_search_and_replace(orig, search, value);
    assert_eq!(
        c_out.as_bytes(), r_out.as_bytes(),
        "MISMATCH for orig={:?} search={:?} value={:?}\n  C:    {:?}\n  Rust: {:?}",
        orig, search, value, c_out, r_out
    );
}

#[test]
fn test_search_and_replace_cases() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };

    // No match
    check("hello world", "xyz", "abc", &lib);

    // Simple single replacement
    check("hello world", "world", "rust", &lib);

    // Replacement at start
    check("hello world", "hello", "goodbye", &lib);

    // Multiple occurrences
    check("aaa", "a", "bb", &lib);

    // Replace with empty string
    check("hello world", "world", "", &lib);

    // Replace empty-adjacent: search longer than orig
    check("hi", "hello", "world", &lib);

    // Entire string is the search
    check("abc", "abc", "xyz", &lib);

    // Replacement longer than search, multiple matches
    check("abcabcabc", "abc", "12345", &lib);

    // Replacement shorter than search, multiple matches
    check("abcabcabc", "abc", "x", &lib);

    // Adjacent matches
    check("aaaa", "aa", "b", &lib);

    // Search at end
    check("hello world", "orld", "ORLD", &lib);

    // Single character replacements
    check("abcdef", "c", "C", &lib);

    // Overlapping pattern (non-overlapping match behavior)
    check("aaaa", "aa", "X", &lib);

    // Long string with multiple replacements
    check(
        "the quick brown fox jumps over the lazy dog",
        "the",
        "THE",
        &lib,
    );

    // Replace with longer value containing the search term
    check("abc", "b", "bbb", &lib);
}

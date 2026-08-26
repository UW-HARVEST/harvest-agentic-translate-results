use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;

fn lib_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_lib = manifest.join("c_src/build/libdriver.so");
    let rust_lib = manifest.join("target/debug/libdriver.so");
    (c_lib, rust_lib)
}

unsafe fn call_search_and_replace(
    lib: &Library,
    orig: &str,
    search: &str,
    value: &str,
) -> Option<Vec<u8>> {
    let f: Symbol<unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char> =
        unsafe { lib.get(b"searchAndReplace") }.unwrap();
    let o = CString::new(orig).unwrap();
    let s = CString::new(search).unwrap();
    let v = CString::new(value).unwrap();
    let result = unsafe { f(o.as_ptr(), s.as_ptr(), v.as_ptr()) };
    if result.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(result) }.to_bytes().to_vec();
    // free the C-allocated memory
    extern "C" {
        fn free(ptr: *mut c_char);
    }
    unsafe { free(result) };
    Some(bytes)
}

fn compare(orig: &str, search: &str, value: &str) {
    let (c_path, rust_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).expect("failed to load C lib");
        let rust_lib = Library::new(&rust_path).expect("failed to load Rust lib");
        let c_result = call_search_and_replace(&c_lib, orig, search, value);
        let r_result = call_search_and_replace(&rust_lib, orig, search, value);
        assert_eq!(
            c_result, r_result,
            "Mismatch for orig={:?} search={:?} value={:?}\n  C={:?}\n  Rust={:?}",
            orig, search, value, c_result, r_result
        );
    }
}

#[test]
fn no_match() {
    compare("hello world", "xyz", "abc");
}

#[test]
fn single_match_middle() {
    compare("hello world", "lo wo", "LO WO");
}

#[test]
fn match_at_start() {
    compare("hello world", "hello", "HI");
}

#[test]
fn match_at_end() {
    compare("hello world", "world", "EARTH");
}

#[test]
fn entire_string() {
    compare("abc", "abc", "XYZ");
}

#[test]
fn multiple_matches() {
    compare("abcabcabc", "abc", "X");
}

#[test]
fn replace_longer() {
    compare("aXbXc", "X", "1234");
}

#[test]
fn replace_shorter() {
    compare("aXXXb", "XXX", "Y");
}

#[test]
fn replace_with_empty() {
    compare("hello world", "lo", "");
}

#[test]
fn empty_orig_no_match() {
    compare("", "abc", "xyz");
}

#[test]
fn adjacent_matches() {
    compare("aaaa", "aa", "X");
}

#[test]
fn overlapping_pattern() {
    // C strstr doesn't find overlapping matches; after "aa" at pos 0, it searches from pos 2
    compare("aaa", "aa", "X");
}

#[test]
fn single_char_replace() {
    compare("abcabc", "a", "Z");
}

#[test]
fn replace_with_search_substring() {
    compare("foobarfoo", "foo", "fo");
}

#[test]
fn long_replacement() {
    compare("a.b.c", ".", "---");
}

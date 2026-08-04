use libloading::{Library, Symbol};
use std::ffi::c_char;

extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
    fn strlen(s: *const c_char) -> usize;
}

type FilterFn = unsafe extern "C" fn(*const c_char, bool) -> *mut c_char;
type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn rust_lib_path() -> String {
    // Tests use the dev profile by default; libdriver.so will be in target/debug
    let manifest = env!("CARGO_MANIFEST_DIR");
    // Try both debug and release, prefer whichever exists.
    let debug = format!("{}/target/debug/libdriver.so", manifest);
    let release = format!("{}/target/release/libdriver.so", manifest);
    if std::path::Path::new(&release).exists() {
        release
    } else {
        debug
    }
}

unsafe fn load_libs() -> (Library, Library) {
    let c_lib = Library::new(c_lib_path()).expect("load C lib");
    let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
    (c_lib, r_lib)
}

unsafe fn bytes_from_cstr_ptr(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let len = strlen(p);
    let slice = std::slice::from_raw_parts(p as *const u8, len);
    slice.to_vec()
}

fn compare_filter(input_bytes: &[u8], replacement: bool) {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_filter: Symbol<FilterFn> = c_lib.get(b"w_utf8_filter").unwrap();
        let r_filter: Symbol<FilterFn> = r_lib.get(b"w_utf8_filter").unwrap();

        // Build a C string. The C code reads up to the null terminator, so input must be null-terminated
        // and contain no embedded nulls.
        let mut buf = input_bytes.to_vec();
        buf.push(0);
        let cstr_ptr = buf.as_ptr() as *const c_char;

        let c_out = c_filter(cstr_ptr, replacement);
        let r_out = r_filter(cstr_ptr, replacement);

        assert!(!c_out.is_null(), "C output null");
        assert!(!r_out.is_null(), "Rust output null");

        let c_bytes = bytes_from_cstr_ptr(c_out);
        let r_bytes = bytes_from_cstr_ptr(r_out);

        assert_eq!(
            c_bytes, r_bytes,
            "filter mismatch: input={:?} replacement={}",
            input_bytes, replacement
        );

        free(c_out as *mut _);
        free(r_out as *mut _);
    }
}

fn compare_drop(input_bytes: &[u8]) {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_drop: Symbol<DropFn> = c_lib.get(b"w_utf8_drop").unwrap();
        let r_drop: Symbol<DropFn> = r_lib.get(b"w_utf8_drop").unwrap();

        let mut buf = input_bytes.to_vec();
        buf.push(0);
        let base = buf.as_ptr() as *const c_char;

        let c_res = c_drop(base);
        let r_res = r_drop(base);

        let c_off = c_res as usize - base as usize;
        let r_off = r_res as usize - base as usize;
        assert_eq!(
            c_off, r_off,
            "drop offset mismatch: input={:?}",
            input_bytes
        );
    }
}

#[test]
fn drop_basic_ascii() {
    compare_drop(b"hello world");
    compare_drop(b"");
    compare_drop(b"a");
}

#[test]
fn drop_valid_utf8_multibyte() {
    // 2-byte: U+00E9 (é) = 0xC3 0xA9
    compare_drop(&[0xC3, 0xA9]);
    // 3-byte: U+20AC (€) = 0xE2 0x82 0xAC
    compare_drop(&[0xE2, 0x82, 0xAC]);
    // 4-byte: U+1F600 = 0xF0 0x9F 0x98 0x80
    compare_drop(&[0xF0, 0x9F, 0x98, 0x80]);
    // mixed
    compare_drop(b"hi \xC3\xA9 \xE2\x82\xAC \xF0\x9F\x98\x80 end");
}

#[test]
fn drop_invalid_starts() {
    // lone continuation byte
    compare_drop(&[0x80]);
    // invalid leading byte
    compare_drop(&[0xC0, 0xA0]);
    compare_drop(&[0xC1, 0xA0]);
    // overlong 3-byte
    compare_drop(&[0xE0, 0x80, 0x80]);
    // surrogate range
    compare_drop(&[0xED, 0xA0, 0x80]);
    // invalid 4-byte (>U+10FFFF)
    compare_drop(&[0xF4, 0x90, 0x80, 0x80]);
    compare_drop(&[0xF5, 0x80, 0x80, 0x80]);
    // truncated
    compare_drop(&[0xC3]);
    compare_drop(&[0xE2, 0x82]);
    compare_drop(&[0xF0, 0x9F, 0x98]);
}

#[test]
fn drop_invalid_ef() {
    // 0xEF 0xBF 0xBD is the replacement character itself - should be valid
    compare_drop(&[0xEF, 0xBF, 0xBD]);
    // 0xEF with byte > 0xBF (impossible since continuation is <=0xBF, but test anyway)
    compare_drop(&[0xEF, 0x80, 0x80]);
}

#[test]
fn filter_basic_no_replacement() {
    compare_filter(b"hello", false);
    compare_filter(b"", false);
    compare_filter(b"hi \xC3\xA9 there", false);
}

#[test]
fn filter_invalid_no_replacement() {
    compare_filter(&[0x80, 0x81, b'a'], false);
    compare_filter(b"good\xC0\xA0bad", false);
    compare_filter(b"\xED\xA0\x80x", false);
    compare_filter(b"\xF5\x80\x80\x80end", false);
}

#[test]
fn filter_basic_with_replacement() {
    compare_filter(b"hello", true);
    compare_filter(b"", true);
    compare_filter(b"hi \xC3\xA9 there", true);
}

#[test]
fn filter_invalid_with_replacement() {
    compare_filter(&[0x80, 0x81, b'a'], true);
    compare_filter(b"good\xC0\xA0bad", true);
    compare_filter(b"\xED\xA0\x80x", true);
    compare_filter(b"\xF5\x80\x80\x80end", true);
    compare_filter(b"\xC3", true);
    compare_filter(b"abc\xC3def", true);
}

#[test]
fn filter_lots_of_invalid_bytes() {
    // Many invalid bytes to exercise the realloc path (REPLACEMENT_INC = 4096, repl decrements by 3).
    let mut input = Vec::new();
    input.extend(b"start ");
    for _ in 0..2000 {
        input.push(0x80);
    }
    input.extend(b" end");
    compare_filter(&input, true);
    compare_filter(&input, false);
}

#[test]
fn filter_mixed_complex() {
    let input: &[u8] = b"hi \xC3\xA9 \xE2\x82\xAC \xF0\x9F\x98\x80 \x80\xFF \xED\xA0\x80 end";
    compare_filter(input, true);
    compare_filter(input, false);
}

#[test]
fn filter_only_invalid() {
    compare_filter(&[0x80, 0x81, 0x82, 0x83, 0x84], true);
    compare_filter(&[0x80, 0x81, 0x82, 0x83, 0x84], false);
}

#[test]
fn filter_full_ascii_range() {
    // All valid printable ASCII
    let s: Vec<u8> = (0x20u8..0x7Fu8).collect();
    compare_filter(&s, true);
    compare_filter(&s, false);
}

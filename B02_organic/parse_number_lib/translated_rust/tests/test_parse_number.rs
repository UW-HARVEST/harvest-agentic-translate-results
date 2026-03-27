use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone)]
struct ParseBuffer {
    content: *const u8,
    length: usize,
    offset: usize,
    depth: usize,
}

#[repr(C)]
#[derive(Clone)]
struct CJson {
    type_: c_int,
    valueint: c_int,
    valuedouble: f64,
}

type ParseNumberFn = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> c_int;

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // Find the built Rust cdylib
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_dir = std::path::PathBuf::from(manifest).join("target/debug");
    target_dir.join("libdriver.so")
}

fn call_parse_number(
    func: ParseNumberFn,
    input: &[u8],
    offset: usize,
) -> (c_int, CJson, usize) {
    let mut item = CJson {
        type_: 0,
        valueint: 0,
        valuedouble: 0.0,
    };
    let mut buf = ParseBuffer {
        content: input.as_ptr(),
        length: input.len(),
        offset,
        depth: 0,
    };
    let ret = unsafe { func(&mut item, &mut buf) };
    (ret, item, buf.offset)
}

fn compare(label: &str, input: &[u8], offset: usize, c_fn: ParseNumberFn, rs_fn: ParseNumberFn) {
    let (c_ret, c_item, c_off) = call_parse_number(c_fn, input, offset);
    let (r_ret, r_item, r_off) = call_parse_number(rs_fn, input, offset);

    assert_eq!(c_ret, r_ret, "{label}: return value mismatch");
    if c_ret != 0 {
        assert_eq!(
            c_item.valuedouble.to_bits(),
            r_item.valuedouble.to_bits(),
            "{label}: valuedouble mismatch (C={}, Rust={})",
            c_item.valuedouble,
            r_item.valuedouble
        );
        assert_eq!(c_item.valueint, r_item.valueint, "{label}: valueint mismatch");
        assert_eq!(c_item.type_, r_item.type_, "{label}: type mismatch");
        assert_eq!(c_off, r_off, "{label}: offset mismatch");
    }
}

#[test]
fn test_parse_number_cases() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rs_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_fn: Symbol<ParseNumberFn> = unsafe { c_lib.get(b"parse_number").unwrap() };
    let rs_fn: Symbol<ParseNumberFn> = unsafe { rs_lib.get(b"parse_number").unwrap() };

    let cases: Vec<(&str, &[u8], usize)> = vec![
        // Basic integers
        ("zero", b"0 ", 0),
        ("one", b"1 ", 0),
        ("negative", b"-1 ", 0),
        ("large_int", b"12345 ", 0),
        ("neg_large", b"-99999 ", 0),
        // Floats
        ("float", b"3.14 ", 0),
        ("neg_float", b"-2.718 ", 0),
        ("leading_zero_float", b"0.5 ", 0),
        // Scientific notation
        ("sci", b"1e10 ", 0),
        ("sci_neg", b"1e-5 ", 0),
        ("sci_pos", b"1E+3 ", 0),
        ("sci_float", b"1.5e2 ", 0),
        // Edge cases
        ("int_max_area", b"2147483647 ", 0),
        ("int_min_area", b"-2147483648 ", 0),
        ("overflow_pos", b"1e308 ", 0),
        ("overflow_neg", b"-1e308 ", 0),
        ("huge", b"1e309 ", 0),
        ("tiny", b"1e-400 ", 0),
        // Offset
        ("with_offset", b"abc123.45 ", 3),
        // Null termination boundary
        ("exact_end", b"42", 0),
        // Just a sign (should fail)
        ("just_minus", b"- ", 0),
        ("just_plus", b"+ ", 0),
        // Empty / null
        ("empty", b"", 0),
        // Number followed by non-number
        ("num_then_alpha", b"123abc", 0),
        // Negative zero
        ("neg_zero", b"-0 ", 0),
        ("neg_zero_float", b"-0.0 ", 0),
    ];

    for (label, input, offset) in cases {
        compare(label, input, offset, *c_fn, *rs_fn);
    }
}

#[test]
fn test_null_inputs() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rs_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_fn: Symbol<ParseNumberFn> = unsafe { c_lib.get(b"parse_number").unwrap() };
    let rs_fn: Symbol<ParseNumberFn> = unsafe { rs_lib.get(b"parse_number").unwrap() };

    // null buffer
    let mut item_c = CJson { type_: 0, valueint: 0, valuedouble: 0.0 };
    let mut item_r = CJson { type_: 0, valueint: 0, valuedouble: 0.0 };
    let c_ret = unsafe { c_fn(&mut item_c, std::ptr::null_mut()) };
    let r_ret = unsafe { rs_fn(&mut item_r, std::ptr::null_mut()) };
    assert_eq!(c_ret, r_ret, "null buffer: return mismatch");

    // null content
    let mut buf_c = ParseBuffer { content: std::ptr::null(), length: 10, offset: 0, depth: 0 };
    let mut buf_r = ParseBuffer { content: std::ptr::null(), length: 10, offset: 0, depth: 0 };
    let c_ret = unsafe { c_fn(&mut item_c, &mut buf_c) };
    let r_ret = unsafe { rs_fn(&mut item_r, &mut buf_r) };
    assert_eq!(c_ret, r_ret, "null content: return mismatch");
}

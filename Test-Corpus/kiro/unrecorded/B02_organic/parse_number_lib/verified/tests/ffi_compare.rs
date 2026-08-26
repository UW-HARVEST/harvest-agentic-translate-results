use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Debug)]
struct CJson {
    type_: i32,
    valueint: i32,
    valuedouble: f64,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct ParseBuffer {
    content: *const u8,
    length: usize,
    offset: usize,
    depth: usize,
}

type ParseNumberFn = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> i32;

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libdriver.so");
    p
}

fn call_parse_number(
    lib: &Library,
    input: &[u8],
    offset: usize,
) -> (i32, CJson, usize) {
    unsafe {
        let func: Symbol<ParseNumberFn> = lib.get(b"parse_number").unwrap();
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
        let ret = func(&mut item, &mut buf);
        (ret, item, buf.offset)
    }
}

fn compare(input: &[u8], offset: usize, c_lib: &Library, rs_lib: &Library) {
    let (c_ret, c_item, c_off) = call_parse_number(c_lib, input, offset);
    let (r_ret, r_item, r_off) = call_parse_number(rs_lib, input, offset);

    let label = format!(
        "input={:?} offset={}",
        std::str::from_utf8(input).unwrap_or("<binary>"),
        offset
    );

    assert_eq!(c_ret, r_ret, "return mismatch: {label}");
    if c_ret != 0 {
        assert_eq!(
            c_item.valuedouble.to_bits(),
            r_item.valuedouble.to_bits(),
            "valuedouble mismatch: {label} (C={}, Rust={})",
            c_item.valuedouble,
            r_item.valuedouble
        );
        assert_eq!(c_item.valueint, r_item.valueint, "valueint mismatch: {label}");
        assert_eq!(c_item.type_, r_item.type_, "type mismatch: {label}");
        assert_eq!(c_off, r_off, "offset mismatch: {label}");
    }
}

#[test]
fn test_parse_number_comprehensive() {
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let rs_lib = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };

    let cases: Vec<(&[u8], usize)> = vec![
        // Basic integers
        (b"0", 0),
        (b"1", 0),
        (b"-1", 0),
        (b"42", 0),
        (b"-42", 0),
        (b"123456789", 0),
        (b"-123456789", 0),
        // Floats
        (b"3.14", 0),
        (b"-3.14", 0),
        (b"0.0", 0),
        (b"0.001", 0),
        (b"-0.001", 0),
        (b"1.0", 0),
        // Scientific notation
        (b"1e10", 0),
        (b"1E10", 0),
        (b"1e-10", 0),
        (b"1E-10", 0),
        (b"2.5e3", 0),
        (b"-2.5e3", 0),
        (b"1e+10", 0),
        (b"1.23e45", 0),
        (b"1e308", 0),
        (b"-1e308", 0),
        // Edge: overflow / underflow
        (b"1e309", 0),   // inf
        (b"-1e309", 0),  // -inf
        (b"1e-400", 0),  // underflow to 0
        // INT_MAX / INT_MIN boundary
        (b"2147483647", 0),   // INT_MAX
        (b"2147483648", 0),   // INT_MAX + 1
        (b"-2147483648", 0),  // INT_MIN
        (b"-2147483649", 0),  // INT_MIN - 1
        // Number followed by non-number chars
        (b"42.5xyz", 0),
        (b"100 ", 0),
        (b"3.14,next", 0),
        (b"-7.5]", 0),
        (b"0}", 0),
        // Offset into buffer
        (b"abc123", 3),
        (b"  -99.9end", 2),
        (b"[42]", 1),
        // Leading plus (strtod accepts it)
        (b"+5", 0),
        (b"+3.14", 0),
        // Just a sign (should fail - strtod can't parse)
        (b"-", 0),
        (b"+", 0),
        // Just 'e' (should fail)
        (b"e5", 0),
        // Empty / null-like
        (b"", 0),
        (b"abc", 0),
        // Zero variants
        (b"-0", 0),
        (b"-0.0", 0),
        (b"0e0", 0),
        // Very long number
        (b"1.23456789012345678901234567890", 0),
        // Multiple decimal points (strtod stops at second)
        (b"1.2.3", 0),
    ];

    for (input, offset) in cases {
        compare(input, offset, &c_lib, &rs_lib);
    }
}

#[test]
fn test_null_inputs() {
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let rs_lib = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };

    // null input_buffer
    unsafe {
        let c_fn: Symbol<ParseNumberFn> = c_lib.get(b"parse_number").unwrap();
        let r_fn: Symbol<ParseNumberFn> = rs_lib.get(b"parse_number").unwrap();

        let mut item_c = CJson { type_: 0, valueint: 0, valuedouble: 0.0 };
        let mut item_r = CJson { type_: 0, valueint: 0, valuedouble: 0.0 };

        let c_ret = c_fn(&mut item_c, std::ptr::null_mut());
        let r_ret = r_fn(&mut item_r, std::ptr::null_mut());
        assert_eq!(c_ret, r_ret, "null buffer return mismatch");

        // null content pointer
        let mut buf_c = ParseBuffer { content: std::ptr::null(), length: 10, offset: 0, depth: 0 };
        let mut buf_r = ParseBuffer { content: std::ptr::null(), length: 10, offset: 0, depth: 0 };
        let c_ret = c_fn(&mut item_c, &mut buf_c);
        let r_ret = r_fn(&mut item_r, &mut buf_r);
        assert_eq!(c_ret, r_ret, "null content return mismatch");
    }
}

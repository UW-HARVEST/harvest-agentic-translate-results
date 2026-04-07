use libloading::{Library, Symbol};
use std::path::PathBuf;

type Memchra2Fn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libmemchra2_lib.so");
    p
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

fn call_both(a: i32, b: i32, c: i32, d: i32) -> (i32, i32) {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_fn: Symbol<Memchra2Fn> = c_lib.get(b"memchra2").expect("C memchra2");
        let r_fn: Symbol<Memchra2Fn> = r_lib.get(b"memchra2").expect("Rust memchra2");
        (c_fn(a, b, c, d), r_fn(a, b, c, d))
    }
}

#[test]
fn test_basic_inputs() {
    for &(a, b, c, d) in &[
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (100, 200, 50, 25),
        (255, 128, 64, 32),
        (1000, 2000, 3000, 4000),
        (i32::MAX, 0, 0, 0),
        (i32::MIN, 0, 0, 0),
        (0, i32::MAX, i32::MAX, i32::MAX),
        (0, i32::MIN, i32::MIN, i32::MIN),
        (1, 1, 1, 1),
        (-1, 0, 0, 0),
        (42, 0, 0, 0),
        (0, 42, 0, 0),
        (0, 0, 42, 0),
        (0, 0, 0, 42),
    ] {
        let (c_res, r_res) = call_both(a, b, c, d);
        assert_eq!(c_res, r_res, "mismatch for ({a}, {b}, {c}, {d}): C={c_res}, Rust={r_res}");
    }
}

#[test]
fn test_float_reinterpret_edge_cases() {
    // int_to_float_bits: values that produce floats in (0, 1000) range
    // IEEE 754: 0x3F800000 = 1.0f, 0x44480000 = 800.0f
    for &a in &[0x3F800000u32 as i32, 0x42C80000u32 as i32, 0x447A0000u32 as i32, 0, -1] {
        let (c_res, r_res) = call_both(a, 1, 2, 3);
        assert_eq!(c_res, r_res, "float edge case a={a:#x}: C={c_res}, Rust={r_res}");
    }
}

#[test]
fn test_byte_masking() {
    // Tests the interpret_as_int path with various byte patterns
    for b in [0, 1, 127, 128, 255, 256, -1, -128] {
        for c in [0, 255, -1] {
            let (c_res, r_res) = call_both(0, b, c, 0);
            assert_eq!(c_res, r_res, "byte mask b={b}, c={c}: C={c_res}, Rust={r_res}");
        }
    }
}

#[test]
fn test_large_values() {
    let cases = [
        (999999, 888888, 777777, 666666),
        (-999999, -888888, -777777, -666666),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN, 0, -1),
    ];
    for &(a, b, c, d) in &cases {
        let (c_res, r_res) = call_both(a, b, c, d);
        assert_eq!(c_res, r_res, "large values ({a}, {b}, {c}, {d}): C={c_res}, Rust={r_res}");
    }
}

#[test]
fn test_snprintf_long_format() {
    // Large numbers produce long formatted strings that may approach the 64-byte buffer limit
    let (c_res, r_res) = call_both(2147483647, 2147483647, 2147483647, 2147483647);
    assert_eq!(c_res, r_res, "long format string: C={c_res}, Rust={r_res}");
    let (c_res, r_res) = call_both(-2147483648, -2147483648, -2147483648, -2147483648);
    assert_eq!(c_res, r_res, "long negative format string: C={c_res}, Rust={r_res}");
}

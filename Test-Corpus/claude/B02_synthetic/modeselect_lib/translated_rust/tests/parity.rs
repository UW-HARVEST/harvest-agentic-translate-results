// Integration tests comparing C library output with Rust library output via FFI.
//
// We load both shared libraries with libloading and call the exported symbols,
// then compare the return values byte-for-byte.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_double, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

// Use libc::time_t for portability.
#[allow(non_camel_case_types)]
type time_t = libc::time_t;

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static INSTANCE: OnceLock<Libs> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_so = manifest_dir.join("c_src/build/libtranslated_rust.so");
        // Find Rust .so. cargo test typically builds to target/debug.
        // CARGO_TARGET_DIR may override.
        let rust_so = manifest_dir.join("target/debug/libmodeselect_lib.so");
        let c =
            unsafe { Library::new(&c_so) }.expect("failed to load C .so");
        let rust = unsafe { Library::new(&rust_so) }
            .expect("failed to load Rust .so");
        Libs { c, rust }
    })
}

unsafe fn get_sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib.get(name).expect("symbol missing");
    *s
}

// classify_mode
type FnClassifyMode = unsafe extern "C" fn(*const c_char) -> c_int;
type FnApplyMultiplier = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnConvertTimeFactor = unsafe extern "C" fn(c_double) -> c_int;
type FnConvertNegativeOverflow = unsafe extern "C" fn(c_double) -> c_int;
type FnGetModifiedTime = unsafe extern "C" fn(c_int, c_int) -> time_t;
type FnHashTimeValue = unsafe extern "C" fn(time_t) -> c_int;
type FnModeselect =
    unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[test]
fn test_classify_mode() {
    let libs = libs();
    let c_fn: FnClassifyMode = unsafe { get_sym(&libs.c, b"classify_mode") };
    let r_fn: FnClassifyMode =
        unsafe { get_sym(&libs.rust, b"classify_mode") };

    let cases: &[&[u8]] = &[
        b"standard\0",
        b"enhanced\0",
        b"turbo\0",
        b"extreme\0",
        b"unknown\0",
        b"\0",
        b"Standard\0", // case-sensitive should fail
        b"standar\0",  // shorter
        b"standardx\0", // longer
    ];
    for input in cases {
        let p = input.as_ptr() as *const c_char;
        let c_res = unsafe { c_fn(p) };
        let r_res = unsafe { r_fn(p) };
        assert_eq!(c_res, r_res, "classify_mode mismatch for {:?}", input);
    }
}

#[test]
fn test_apply_multiplier() {
    let libs = libs();
    let c_fn: FnApplyMultiplier =
        unsafe { get_sym(&libs.c, b"apply_multiplier") };
    let r_fn: FnApplyMultiplier =
        unsafe { get_sym(&libs.rust, b"apply_multiplier") };

    let bases = [0i32, 1, -1, 0xA0, 1000, -1000, i32::MAX, i32::MIN];
    let levels = [-1, 0, 1, 2, 3, 4, 5, 100, -100];
    for &b in &bases {
        for &l in &levels {
            let c_res = unsafe { c_fn(b, l) };
            let r_res = unsafe { r_fn(b, l) };
            assert_eq!(c_res, r_res, "apply_multiplier mismatch base={} level={}", b, l);
        }
    }
}

#[test]
fn test_convert_time_factor() {
    let libs = libs();
    let c_fn: FnConvertTimeFactor =
        unsafe { get_sym(&libs.c, b"convert_time_factor") };
    let r_fn: FnConvertTimeFactor =
        unsafe { get_sym(&libs.rust, b"convert_time_factor") };

    // Stay within representable int range after *1e12 to avoid C UB / platform diffs.
    let cases = [
        0.0,
        1e-12,
        -1e-12,
        1e-9,
        -1e-9,
        2.0e-9,
        -2.0e-9,
        1.5e-9,
        0.0021,    // 0.0021 * 1e12 = 2.1e9 — out of i32 range, skip
    ];
    // Use only safe inputs
    let safe = [0.0_f64, 1e-12, -1e-12, 1e-10, -1e-10, 5e-10, -5e-10, 1.5e-12];
    let _ = cases;
    for &v in &safe {
        let c_res = unsafe { c_fn(v) };
        let r_res = unsafe { r_fn(v) };
        assert_eq!(c_res, r_res, "convert_time_factor mismatch v={}", v);
    }
}

#[test]
fn test_convert_negative_overflow() {
    let libs = libs();
    let c_fn: FnConvertNegativeOverflow =
        unsafe { get_sym(&libs.c, b"convert_negative_overflow") };
    let r_fn: FnConvertNegativeOverflow =
        unsafe { get_sym(&libs.rust, b"convert_negative_overflow") };

    // Stay within representable int range after *-1e15 to avoid C UB.
    // |i32::MAX| ~ 2.147e9. So |value| < 2.147e9 / 1e15 = 2.147e-6.
    let safe = [0.0_f64, 1e-15, -1e-15, 1e-12, -1e-12, 1e-9, -1e-9];
    for &v in &safe {
        let c_res = unsafe { c_fn(v) };
        let r_res = unsafe { r_fn(v) };
        assert_eq!(
            c_res, r_res,
            "convert_negative_overflow mismatch v={}",
            v
        );
    }
}

#[test]
fn test_get_modified_time() {
    let libs = libs();
    let c_fn: FnGetModifiedTime =
        unsafe { get_sym(&libs.c, b"get_modified_time") };
    let r_fn: FnGetModifiedTime =
        unsafe { get_sym(&libs.rust, b"get_modified_time") };

    // Both call time(NULL). After >> 29 the result changes ~every 17 years,
    // so calling them in close succession should return identical values
    // unless the boundary is crossed — extremely unlikely.
    let cases = [
        (0i32, 0i32),
        (1, 0),
        (-1, 0),
        (0, 23),
        (10, -5),
        (365, 12),
    ];
    for &(d, h) in &cases {
        let c_res = unsafe { c_fn(d, h) };
        let r_res = unsafe { r_fn(d, h) };
        assert_eq!(c_res, r_res, "get_modified_time mismatch d={} h={}", d, h);
    }
}

#[test]
fn test_hash_time_value() {
    let libs = libs();
    let c_fn: FnHashTimeValue =
        unsafe { get_sym(&libs.c, b"hash_time_value") };
    let r_fn: FnHashTimeValue =
        unsafe { get_sym(&libs.rust, b"hash_time_value") };

    let cases: &[time_t] = &[
        0,
        1,
        -1,
        42,
        0x12345678,
        0x7FFFFFFFFFFFFFFF,
        -0x7FFFFFFFFFFFFFFF,
        0x5A5A5A5A5A5A5A5A,
        0x1234567890ABCDEFu64 as time_t,
    ];
    for &t in cases {
        let c_res = unsafe { c_fn(t) };
        let r_res = unsafe { r_fn(t) };
        assert_eq!(c_res, r_res, "hash_time_value mismatch t={}", t);
    }
}

#[test]
fn test_modeselect_return_value() {
    // modeselect prints a lot to stdout — we only compare its return value.
    let libs = libs();
    let c_fn: FnModeselect = unsafe { get_sym(&libs.c, b"modeselect") };
    let r_fn: FnModeselect = unsafe { get_sym(&libs.rust, b"modeselect") };

    // Choose seed/time_offset values such that the doubles stay within
    // representable i32 range to avoid C undefined behavior.
    // factor1 = seed * 1e8; need |factor1*1e12| < 2.147e9 (after later scale...
    // Actually convert_time_factor multiplies by 1e12 again: so total seed*1e20.
    // For result to be in i32 range we need seed*1e20 < 2.147e9 → impossible.
    // So we'd hit UB in C. We must choose seed=0 to avoid this.
    // Similarly time_offset = 0 keeps factor2 = 0.
    let cases = [
        (0i32, 0i32, 0i32, 0i32),
        (1, 0, 0, 0),
        (2, 0, 1, 0),
        (3, 0, 2, 0),
        (4, 0, 3, 0),
        (5, 0, 4, 0),
        (10, 0, 100, 0),
        // Negative mode_selector in C produces a negative mode_index due to
        // C's truncation semantics for %, leading to out-of-bounds array
        // access (UB). We avoid those cases.
    ];
    for &(a, b, c, d) in &cases {
        let c_res = unsafe { c_fn(a, b, c, d) };
        let r_res = unsafe { r_fn(a, b, c, d) };
        assert_eq!(
            c_res, r_res,
            "modeselect mismatch ({}, {}, {}, {})",
            a, b, c, d
        );
    }
}

#[test]
fn test_exported_symbols_match() {
    use std::process::Command;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = manifest_dir.join("c_src/build/libtranslated_rust.so");
    let rust_so = manifest_dir.join("target/debug/libmodeselect_lib.so");

    let extract = |path: &PathBuf| -> Vec<String> {
        let output = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("nm failed");
        let s = String::from_utf8_lossy(&output.stdout);
        let mut syms = Vec::new();
        for line in s.lines() {
            // Format: "<addr> <type> <name>"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let typ = parts[1];
            let name = parts[2];
            if typ == "T" || typ == "W" {
                // Skip CRT/dynamic linker internals
                if name == "_init"
                    || name == "_fini"
                    || name == "__bss_start"
                    || name == "_edata"
                    || name == "_end"
                {
                    continue;
                }
                syms.push(name.to_string());
            }
        }
        syms.sort();
        syms.dedup();
        syms
    };

    let c_syms = extract(&c_so);
    let rust_syms = extract(&rust_so);

    // Every symbol in C must exist in Rust
    for s in &c_syms {
        assert!(
            rust_syms.iter().any(|r| r == s),
            "Rust .so missing C-exported symbol: {}",
            s
        );
    }
}

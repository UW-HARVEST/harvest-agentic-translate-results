//! High-volume randomized differential fuzzing across the whole public surface.
//!
//! Complements the per-row tests: those pin specific configurations, this one
//! sweeps the value/shape space densely with a fixed seed. Everything goes
//! through the exported symbols of both `.so`s.

mod common;

use common::*;
use std::os::raw::{c_int, c_void};

/// Decimal-length boundary values. `matrix_to_string`'s buffer formula
/// accommodates up to 10 characters per element at any width; 11 characters
/// (i.e. <= -1_000_000_000) overflows the C's own buffer for width >= 2, so
/// those are only used at width 1.
const BOUNDARY: [i32; 21] = [
    0,
    1,
    -1,
    9,
    -9,
    10,
    -10,
    99,
    -99,
    100,
    -100,
    9_999,
    -9_999,
    100_000,
    -100_000,
    99_999_999,
    -99_999_999,
    999_999_999,
    -999_999_999,
    123_456_789,
    -987_654_321,
];

fn to_string_pair(b: &Both, w: c_int, h: c_int, vals: &[c_int]) {
    let run = |api: &Api| unsafe {
        let m = make_matrix(api, w, h, vals);
        let p = (api.matrix_to_string)(m);
        let bytes = cstr_bytes(p);
        if !p.is_null() {
            libc_free(p as *mut c_void);
        }
        (api.free_matrix)(m);
        bytes
    };
    let bc = run(&b.c);
    let br = run(&b.rs);
    assert_eq!(bc, br, "matrix_to_string({w}x{h}) mismatch on {vals:?}");
}

/// Dense sweep of the decimal-length boundary values through
/// `matrix_to_string`, at every width from 1 to 12.
#[test]
fn fuzz_to_string_digit_length_boundary() {
    let b = load_both();
    let mut rng = Rng::new(0xF0F0_0001);
    for w in 1..=12i32 {
        for h in [1i32, 2, 3, 7] {
            for _ in 0..40 {
                let vals: Vec<c_int> = (0..(w * h))
                    .map(|_| *rng.pick(&BOUNDARY))
                    .collect();
                to_string_pair(&b, w, h, &vals);
            }
        }
    }
    // width == 1 is the only width where an 11-character element fits.
    for h in [1i32, 2, 5] {
        let vals: Vec<c_int> = (0..h)
            .map(|i| if i % 2 == 0 { i32::MIN } else { i32::MAX })
            .collect();
        to_string_pair(&b, 1, h, &vals);
    }
}

/// Dense randomized sweep of the parser: random shapes, random token text.
#[test]
fn fuzz_parser_dense() {
    let b = load_both();
    let mut rng = Rng::new(0xF0F0_0002);
    let seps = [" ", "  ", "\t", " \t", "   ", "\t\t"];
    for _ in 0..2000 {
        let w = rng.range(0, 6) as i32;
        let h = rng.range(0, 6) as i32;
        let mut text = String::new();
        let rows = h + rng.range(0, 2) as i32;
        for _ in 0..rows {
            let cols = w + rng.range(0, 2) as i32;
            for j in 0..cols.max(1) {
                if j > 0 {
                    text.push_str(rng.pick(&seps));
                }
                match rng.range(0, 5) {
                    0 => text.push_str(&rng.range(-2_000_000_000, 2_000_000_000).to_string()),
                    1 => text.push_str(&format!("{}", rng.range(-99, 99))),
                    2 => text.push_str(rng.pick(&["abc", "0x1f", "+", "-", "", "..", "1e9"])),
                    3 => text.push_str(&format!("{}q", rng.range(0, 999))),
                    _ => text.push_str(&format!("{:+}", rng.range(-9999, 9999))),
                }
            }
            text.push('\n');
            if rng.range(0, 4) == 0 {
                text.push('\n');
            }
        }
        let s = cs(&text);
        let run = |api: &Api| {
            let (p, err) =
                capture_stderr(|| unsafe { (api.initialize_matrix_from_string)(s.as_ptr(), w, h) });
            let snap = unsafe { snapshot(p) };
            let null = p.is_null();
            if !p.is_null() {
                unsafe { (api.free_matrix)(p) };
            }
            (null, snap, err)
        };
        let (nc, sc, ec) = run(&b.c);
        let (nr, sr, er) = run(&b.rs);
        assert_eq!(nc, nr, "parser NULL-ness mismatch on {text:?} ({w},{h})");
        assert_eq!(sc, sr, "parser value mismatch on {text:?} ({w},{h})");
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "parser stderr mismatch on {text:?} ({w},{h})"
        );
    }
}

/// Dense randomized sweep of `multiply_matrices`, including operand magnitudes
/// that make the accumulator wrap.
#[test]
fn fuzz_multiply_dense() {
    let b = load_both();
    let mut rng = Rng::new(0xF0F0_0003);
    for _ in 0..1200 {
        let ha = rng.range(0, 6) as c_int;
        let inner = rng.range(0, 6) as c_int;
        let wb = rng.range(0, 6) as c_int;
        let magnitude = match rng.range(0, 3) {
            0 => 10i64,
            1 => 100_000i64,
            _ => i32::MAX as i64,
        };
        let va: Vec<c_int> = (0..(ha * inner))
            .map(|_| rng.range(-magnitude, magnitude) as c_int)
            .collect();
        let vb: Vec<c_int> = (0..(inner * wb))
            .map(|_| rng.range(-magnitude, magnitude) as c_int)
            .collect();
        let run = |api: &Api| unsafe {
            let a = make_matrix(api, inner, ha, &va);
            let bb = make_matrix(api, wb, inner, &vb);
            let (res, err) = capture_stderr(|| (api.multiply_matrices)(a, bb));
            let snap = snapshot(res);
            if !res.is_null() {
                (api.free_matrix)(res);
            }
            (api.free_matrix)(a);
            (api.free_matrix)(bb);
            (snap, err)
        };
        let (sc, ec) = run(&b.c);
        let (sr, er) = run(&b.rs);
        assert_eq!(sc, sr, "multiply mismatch ({ha}x{inner}) * ({inner}x{wb})");
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "multiply stderr mismatch"
        );
    }
}

/// Dense randomized sweep of `write_to_file`, comparing the return code and the
/// bytes that landed on disk.
#[test]
fn fuzz_write_dense() {
    let b = load_both();
    let d = std::env::temp_dir().join(format!("difftest-fuzzwrite-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let path = d.join("out.bin");
    let fname = cs(path.to_str().unwrap());
    let mut rng = Rng::new(0xF0F0_0004);
    for _ in 0..500 {
        let len = match rng.range(0, 3) {
            0 => rng.range(0, 16) as usize,
            1 => rng.range(0, 5000) as usize,
            _ => rng.range(4000, 9000) as usize,
        };
        let content: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        let cont = std::ffi::CString::new(content.clone()).unwrap();
        let run = |api: &Api| {
            let _ = std::fs::remove_file(&path);
            let rc = unsafe { (api.write_to_file)(fname.as_ptr(), cont.as_ptr()) };
            let bytes = std::fs::read(&path).ok();
            (rc, bytes)
        };
        let (rc_c, bc) = run(&b.c);
        let (rc_r, br) = run(&b.rs);
        assert_eq!(rc_c, rc_r, "write rc mismatch at len {len}");
        assert_eq!(bc, br, "write bytes mismatch at len {len}");
        assert_eq!(rc_c, 0);
        assert_eq!(bc.as_deref(), Some(content.as_slice()));
    }
    let _ = std::fs::remove_file(&path);
}

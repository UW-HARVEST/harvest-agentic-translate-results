//! Parity tests for the pure (allocation-independent) functions.
//!
//! Every call goes through `libloading` into the corresponding shared object,
//! so the Rust `#[no_mangle]` export wrappers are exercised exactly as an
//! external C caller would exercise them.

mod common;
use common::*;
use std::ffi::c_int;

#[test]
fn apply_bitmask_matches() {
    let l = libs();
    let c = sym!(l.c, "apply_bitmask", Fn2);
    let r = sym!(l.rust, "apply_bitmask", Fn2);

    let values: [c_int; 17] = [
        0,
        1,
        -1,
        2,
        255,
        256,
        -255,
        0x7f,
        0xf0,
        0x0f,
        0xaa,
        0x55,
        1234567,
        -1234567,
        c_int::MIN,
        c_int::MAX,
        -2147483647,
    ];
    for &v in &values {
        for op in -8..=8 {
            let a = unsafe { c(v, op) };
            let b = unsafe { r(v, op) };
            assert_eq!(a, b, "apply_bitmask({v}, {op})");
        }
    }
}

#[test]
fn process_string_matches() {
    let l = libs();
    let c = sym!(l.c, "process_string", FnPtrCInt);
    let r = sym!(l.rust, "process_string", FnPtrCInt);

    let mut cases: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"Hello\0".to_vec(),
        b"Hello, world!\0".to_vec(),
        b" \0".to_vec(),
        b"\x01\0".to_vec(),
        b"\xff\xfe\xfd\0".to_vec(),
        b"tab\there\0".to_vec(),
    ];
    // A long string, and one with high-bit bytes throughout.
    let mut long = vec![b'x'; 1000];
    long.push(0);
    cases.push(long);
    let mut high = vec![0x80u8; 37];
    high.push(0);
    cases.push(high);

    for s in &cases {
        let a = unsafe { c(s.as_ptr() as *const std::ffi::c_char) };
        let b = unsafe { r(s.as_ptr() as *const std::ffi::c_char) };
        assert_eq!(a, b, "process_string({:?})", &s[..s.len() - 1]);
    }
}

#[test]
fn shift_array_matches() {
    let l = libs();
    let c = sym!(l.c, "shift_array", FnShift);
    let r = sym!(l.rust, "shift_array", FnShift);

    // Guarded buffers: canary words on both sides catch out-of-range writes.
    const PAD: usize = 4;
    const CANARY: c_int = -559038737; // 0xDEADBEEF

    let patterns: [&[c_int]; 6] = [
        &[],
        &[7],
        &[1, 2],
        &[1, 2, 3, 4],
        &[10, -20, 30, -40, 50, -60, 70, -80],
        &[c_int::MIN, c_int::MAX, 0, -1, 1, 2, 3, 4],
    ];

    for pat in patterns {
        for size in -2..=(pat.len() as c_int + 2) {
            for positions in -3..=(pat.len() as c_int + 3) {
                let build = || {
                    let mut v = vec![CANARY; PAD];
                    v.extend_from_slice(pat);
                    v.extend(std::iter::repeat(CANARY).take(PAD));
                    v
                };
                let mut ba = build();
                let mut bb = build();
                unsafe {
                    c(ba.as_mut_ptr().add(PAD), size, positions);
                    r(bb.as_mut_ptr().add(PAD), size, positions);
                }
                assert_eq!(
                    ba, bb,
                    "shift_array(pat={pat:?}, size={size}, positions={positions})"
                );
            }
        }
    }
}

#[test]
fn init_matrix_matches() {
    let l = libs();
    let c = sym!(l.c, "init_matrix", FnMatrix);
    let r = sym!(l.rust, "init_matrix", FnMatrix);

    for fill in [0, -1, 0x5a5a5a5a] {
        let mut ba = [[fill as c_int; 4]; 5]; // two extra rows act as canaries
        let mut bb = ba;
        unsafe {
            c(ba.as_mut_ptr());
            r(bb.as_mut_ptr());
        }
        assert_eq!(ba, bb, "init_matrix (prefill {fill:#x})");
        // Rows 3 and 4 must be untouched by both.
        assert_eq!(ba[3], [fill as c_int; 4]);
        assert_eq!(ba[4], [fill as c_int; 4]);
    }
}

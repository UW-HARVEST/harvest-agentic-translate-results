//! Differential tests for the leaf-level API, in call-hierarchy order:
//! validate_uint16_range -> is_string_empty -> find_char_in_buffer ->
//! create_buffer -> counter operations -> apply_operation.
//!
//! Both implementations are reached only through their `.so` exports.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

type IntToInt = unsafe extern "C" fn(c_int) -> c_int;
type StrToInt = unsafe extern "C" fn(*const c_char) -> c_int;
type FindChar = unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char;
type CreateBuf = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type ApplyOp = unsafe extern "C" fn(Option<IntToInt>, c_int) -> c_int;

#[test]
fn validate_uint16_range_matches() {
    let _g = lock();
    let (c, r) = sym::<IntToInt>("validate_uint16_range");

    let mut inputs: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -70000,
        -65536,
        -65535,
        -2,
        -1,
        0,
        1,
        2,
        255,
        256,
        32767,
        32768,
        65534,
        65535,
        65536,
        65537,
        100000,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    // A deterministic spread of additional values.
    let mut x: u32 = 0x1234_5678;
    for _ in 0..2000 {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        inputs.push(x as c_int);
    }

    for v in inputs {
        let (cv, cout) = capture(|| unsafe { c(v) });
        let (rv, rout) = capture(|| unsafe { r(v) });
        assert_eq!(cv, rv, "validate_uint16_range({v}) return mismatch");
        assert_eq!(cout, rout, "validate_uint16_range({v}) stdout mismatch");
    }
}

#[test]
fn is_string_empty_matches() {
    let _g = lock();
    let (c, r) = sym::<StrToInt>("is_string_empty");

    // NULL
    let (cv, _) = capture(|| unsafe { c(std::ptr::null()) });
    let (rv, _) = capture(|| unsafe { r(std::ptr::null()) });
    assert_eq!(cv, rv, "is_string_empty(NULL) mismatch");

    let cases: Vec<&[u8]> = vec![
        b"\0",
        b"a\0",
        b"\0trailing\0",
        b" \0",
        b"\n\0",
        b"Hello, World!\0",
        b"\x80\0",
        b"\xff\0",
        b"\x01\0",
        b"0\0",
    ];
    for case in cases {
        let p = case.as_ptr() as *const c_char;
        let (cv, cout) = capture(|| unsafe { c(p) });
        let (rv, rout) = capture(|| unsafe { r(p) });
        assert_eq!(cv, rv, "is_string_empty({case:?}) return mismatch");
        assert_eq!(cout, rout, "is_string_empty({case:?}) stdout mismatch");
    }
}

#[test]
fn find_char_in_buffer_matches() {
    let _g = lock();
    let (c, r) = sym::<FindChar>("find_char_in_buffer");

    // NULL buffer, any size / target.
    for size in [0usize, 1, 16] {
        for t in [0i8, b'a' as i8, -1i8] {
            let (cv, _) = capture(|| unsafe { c(std::ptr::null(), size, t) });
            let (rv, _) = capture(|| unsafe { r(std::ptr::null(), size, t) });
            assert!(cv.is_null() && rv.is_null(), "NULL buffer must yield NULL");
        }
    }

    let buffers: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"Search for character X in this buffer".to_vec(),
        b"XXXX".to_vec(),
        b"abc\0def".to_vec(),
        vec![0u8; 8],
        (0u8..=255u8).collect(),
        b"\xff\xfe\x80\x7f".to_vec(),
    ];

    let targets: Vec<c_char> = {
        let mut t: Vec<c_char> = vec![0, 1, -1, -128, 127, 65, 88, 97];
        for v in [0u8, 1, 0x7f, 0x80, 0xfe, 0xff] {
            t.push(v as c_char);
        }
        t
    };

    for buf in &buffers {
        // Include sizes beyond nothing but never past the allocation.
        for size in 0..=buf.len() {
            for &t in &targets {
                let base = buf.as_ptr() as *const c_char;
                let (cp, cout) = capture(|| unsafe { c(base, size, t) });
                let (rp, rout) = capture(|| unsafe { r(base, size, t) });
                let coff = if cp.is_null() {
                    -1
                } else {
                    unsafe { cp.offset_from(base as *mut c_char) }
                };
                let roff = if rp.is_null() {
                    -1
                } else {
                    unsafe { rp.offset_from(base as *mut c_char) }
                };
                assert_eq!(
                    coff, roff,
                    "find_char_in_buffer(buf={buf:?}, size={size}, target={t}) offset mismatch"
                );
                assert_eq!(cout, rout, "stdout mismatch");
            }
        }
    }
}

#[test]
fn create_buffer_matches() {
    let _g = lock();
    let (c, r) = sym::<CreateBuf>("create_buffer");

    let (cp, _) = capture(|| unsafe { c(std::ptr::null()) });
    let (rp, _) = capture(|| unsafe { r(std::ptr::null()) });
    assert!(cp.is_null() && rp.is_null(), "create_buffer(NULL) must be NULL");

    let cases: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"Testing malloc and free\0".to_vec(),
        b"Search for character X in this buffer\0".to_vec(),
        b"Hello, World!\0".to_vec(),
        {
            let mut v = vec![b'z'; 1000];
            v.push(0);
            v
        },
        {
            let mut v: Vec<u8> = (1u8..=255u8).collect();
            v.push(0);
            v
        },
        b"tab\there\nnewline\0".to_vec(),
        b"%s %d %n\0".to_vec(),
    ];

    for case in &cases {
        let p = case.as_ptr() as *const c_char;
        let (cp, cout) = capture(|| unsafe { c(p) });
        let (rp, rout) = capture(|| unsafe { r(p) });
        assert!(!cp.is_null(), "C create_buffer returned NULL");
        assert!(!rp.is_null(), "Rust create_buffer returned NULL");
        let cb = unsafe { cstr_bytes(cp) };
        let rb = unsafe { cstr_bytes(rp) };
        assert_eq!(cb, rb, "create_buffer contents mismatch for {case:?}");
        assert_eq!(&cb[..], &case[..case.len() - 1], "C copy is not the input");
        assert_eq!(cout, rout, "stdout mismatch");
        unsafe {
            cfree(cp as *mut c_void);
            cfree(rp as *mut c_void);
        }
    }
}

/// The counter ops mutate file-scope state, so both libraries are driven
/// through the identical sequence and compared after every step.
#[test]
fn counter_operations_match() {
    let _g = lock();
    let (c_inc, r_inc) = sym::<IntToInt>("increment_counter");
    let (c_dec, r_dec) = sym::<IntToInt>("decrement_counter");
    let (c_mul, r_mul) = sym::<IntToInt>("multiply_counter");
    let (c_res, r_res) = sym::<IntToInt>("reset_counter");

    #[derive(Debug, Clone, Copy)]
    enum Op {
        Inc,
        Dec,
        Mul,
        Reset,
    }

    let mut script: Vec<(Op, c_int)> = vec![
        (Op::Reset, 0),
        (Op::Inc, 1),
        (Op::Inc, -1),
        (Op::Dec, 5),
        (Op::Mul, 3),
        (Op::Mul, 0),
        (Op::Reset, 42),
        (Op::Inc, 100),
        (Op::Mul, -7),
        (Op::Dec, -13),
        (Op::Reset, c_int::MAX),
        (Op::Inc, 1),          // signed overflow in C
        (Op::Inc, c_int::MAX), // more overflow
        (Op::Mul, 2),
        (Op::Reset, c_int::MIN),
        (Op::Dec, 1),
        (Op::Mul, -1),
        (Op::Mul, c_int::MIN),
        (Op::Reset, -1),
        (Op::Mul, c_int::MAX),
    ];

    let mut x: u32 = 0xdead_beef;
    for _ in 0..3000 {
        x = x.wrapping_mul(1103515245).wrapping_add(12345);
        let op = match (x >> 28) % 4 {
            0 => Op::Inc,
            1 => Op::Dec,
            2 => Op::Mul,
            _ => Op::Reset,
        };
        script.push((op, x as c_int));
    }

    // Start both from a known state.
    let _ = capture(|| unsafe { c_res(0) });
    let _ = capture(|| unsafe { r_res(0) });

    for (i, (op, v)) in script.iter().enumerate() {
        let (cv, cout) = capture(|| unsafe {
            match op {
                Op::Inc => c_inc(*v),
                Op::Dec => c_dec(*v),
                Op::Mul => c_mul(*v),
                Op::Reset => c_res(*v),
            }
        });
        let (rv, rout) = capture(|| unsafe {
            match op {
                Op::Inc => r_inc(*v),
                Op::Dec => r_dec(*v),
                Op::Mul => r_mul(*v),
                Op::Reset => r_res(*v),
            }
        });
        assert_eq!(cv, rv, "step {i}: {op:?}({v}) return mismatch");
        assert_eq!(cout, rout, "step {i}: {op:?}({v}) stdout mismatch");
    }
}

#[test]
fn apply_operation_matches() {
    let _g = lock();
    let (c_apply, r_apply) = sym::<ApplyOp>("apply_operation");

    // NULL operation.
    for v in [0, 1, -1, c_int::MAX, c_int::MIN] {
        let (cv, cout) = capture(|| unsafe { c_apply(None, v) });
        let (rv, rout) = capture(|| unsafe { r_apply(None, v) });
        assert_eq!(cv, rv, "apply_operation(NULL, {v}) mismatch");
        assert_eq!(cout, rout, "stdout mismatch");
    }

    // Each side gets a pointer to its *own* op so the right counter is hit.
    let names = [
        "reset_counter",
        "increment_counter",
        "decrement_counter",
        "multiply_counter",
    ];
    let (c_res, r_res) = sym::<IntToInt>("reset_counter");
    let _ = capture(|| unsafe { c_res(0) });
    let _ = capture(|| unsafe { r_res(0) });

    let mut x: u32 = 0x0badc0de;
    for i in 0..1200 {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        let name = names[(i % names.len()) as usize];
        let (c_op, r_op) = sym::<IntToInt>(name);
        let cf: IntToInt = *c_op;
        let rf: IntToInt = *r_op;
        let v = x as c_int;
        let (cv, cout) = capture(|| unsafe { c_apply(Some(cf), v) });
        let (rv, rout) = capture(|| unsafe { r_apply(Some(rf), v) });
        assert_eq!(cv, rv, "apply_operation({name}, {v}) return mismatch");
        assert_eq!(cout, rout, "apply_operation({name}, {v}) stdout mismatch");
    }
}

//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! * `cfg_*` tests  -> one per row of CONFIGS.md (Phase B, valid paths)
//! * `err_*` tests  -> one per row of ERRORS.md  (Phase C, error/rejection paths)
//! * `sym_*` tests  -> Phase D symbol parity

mod common;

use common::*;
use std::ffi::c_char;

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// C1: printIntLine over 1000 randomized full-range i32 values.
#[test]
fn cfg_c1_print_int_line_random_full_range() {
    let mut rng = Rng::new(0xC1);
    let values: Vec<i32> = (0..1000).map(|_| rng.next_i32()).collect();
    for &v in &values {
        assert_same(&format!("C1 printIntLine({v})"), |w| {
            let f = f_print_int_line(w);
            unsafe { f(v) };
        });
    }
}

/// C2: printIntLine boundary / output-width shapes.
#[test]
fn cfg_c2_print_int_line_boundaries() {
    let mut values: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        -99,
        -100,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        127,
        128,
        -128,
        255,
        256,
        -256,
        32767,
        32768,
        -32768,
        65535,
        65536,
        1_000_000_000,
        -1_000_000_000,
        2_000_000_000,
        -2_000_000_000,
    ];
    // every power of ten (all decimal widths), both signs
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        values.push(p as i32);
        values.push(-(p as i32));
        values.push((p - 1) as i32);
        values.push(-((p - 1) as i32));
        p *= 10;
    }
    for &v in &values {
        assert_same(&format!("C2 printIntLine({v})"), |w| {
            let f = f_print_int_line(w);
            unsafe { f(v) };
        });
    }
}

/// C3: many printIntLine calls inside a single capture (shared stdout buffer).
#[test]
fn cfg_c3_print_int_line_repeated() {
    let mut rng = Rng::new(0xC3);
    let values: Vec<i32> = (0..500).map(|_| rng.next_i32()).collect();
    assert_same("C3 printIntLine x500", |w| {
        let f = f_print_int_line(w);
        for &v in &values {
            unsafe { f(v) };
        }
    });
}

/// C4: printLine("")
#[test]
fn cfg_c4_print_line_empty() {
    let s = cstr(b"");
    assert_same("C4 printLine(\"\")", |w| {
        let f = f_print_line(w);
        unsafe { f(s.as_ptr()) };
    });
}

/// C5: printLine with a single byte, every value 1..=255.
#[test]
fn cfg_c5_print_line_single_byte() {
    for b in 1u8..=255 {
        let s = cstr(&[b]);
        assert_same(&format!("C5 printLine(byte 0x{b:02x})"), |w| {
            let f = f_print_line(w);
            unsafe { f(s.as_ptr()) };
        });
    }
}

/// C6: printLine with 200 randomized printable-ASCII strings, length 0..64.
#[test]
fn cfg_c6_print_line_random_ascii() {
    let mut rng = Rng::new(0xC6);
    for case in 0..200 {
        let len = rng.below(65);
        let bytes: Vec<u8> = (0..len).map(|_| rng.range_u8(0x20, 0x7e)).collect();
        let s = cstr(&bytes);
        assert_same(&format!("C6 case {case} len {len}"), |w| {
            let f = f_print_line(w);
            unsafe { f(s.as_ptr()) };
        });
    }
}

/// C7: printLine with 200 randomized arbitrary non-NUL byte strings (0x01..0xFF),
/// length 0..256 — includes invalid UTF-8 and control characters.
#[test]
fn cfg_c7_print_line_random_bytes() {
    let mut rng = Rng::new(0xC7);
    for case in 0..200 {
        let len = rng.below(257);
        let bytes: Vec<u8> = (0..len).map(|_| rng.range_u8(1, 255)).collect();
        let s = cstr(&bytes);
        assert_same(&format!("C7 case {case} len {len}"), |w| {
            let f = f_print_line(w);
            unsafe { f(s.as_ptr()) };
        });
    }
}

/// C8: printLine with printf conversion specifiers in the *argument*.
#[test]
fn cfg_c8_print_line_format_specifiers() {
    let cases: [&[u8]; 10] = [
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%1000000d",
        b"%s%s%s%s%s%s%s%s",
        b"100%",
        b"%p %x %o %e %g %c",
        b"a%sb%dc%nd",
        b"%*.*f",
    ];
    for (i, c) in cases.iter().enumerate() {
        let s = cstr(c);
        assert_same(&format!("C8 case {i}"), |w| {
            let f = f_print_line(w);
            unsafe { f(s.as_ptr()) };
        });
    }
}

/// C9: printLine with strings larger than any stdio buffer.
#[test]
fn cfg_c9_print_line_oversized() {
    for &len in &[1023usize, 1024, 4095, 4096, 4097, 8192, 65536] {
        let mut rng = Rng::new(0xC9 ^ len as u64);
        let bytes: Vec<u8> = (0..len).map(|_| rng.range_u8(0x21, 0x7e)).collect();
        let s = cstr(&bytes);
        assert_same(&format!("C9 len {len}"), |w| {
            let f = f_print_line(w);
            unsafe { f(s.as_ptr()) };
        });
    }
}

/// C10: printLine with embedded newlines / whitespace-only / padded strings.
#[test]
fn cfg_c10_print_line_newlines_ws() {
    let cases: [&[u8]; 9] = [
        b"\n",
        b"\n\n\n",
        b"a\nb\nc",
        b"trailing\n",
        b"   ",
        b"\t\t",
        b"  padded  ",
        b"\r\n",
        b"line1\r\nline2\r\n",
    ];
    for (i, c) in cases.iter().enumerate() {
        let s = cstr(c);
        assert_same(&format!("C10 case {i}"), |w| {
            let f = f_print_line(w);
            unsafe { f(s.as_ptr()) };
        });
    }
}

/// C11: printLine(NULL) — the guarded configuration.
#[test]
fn cfg_c11_print_line_null() {
    assert_same("C11 printLine(NULL)", |w| {
        let f = f_print_line(w);
        unsafe { f(core::ptr::null()) };
    });
}

/// C12: 300 mixed NULL / non-NULL printLine calls in one capture.
#[test]
fn cfg_c12_print_line_mixed_repeated() {
    let mut rng = Rng::new(0xC12);
    // Pre-build the whole (owned) sequence so both runs see identical inputs.
    let seq: Vec<Option<Vec<c_char>>> = (0..300)
        .map(|_| {
            if rng.below(4) == 0 {
                None
            } else {
                let len = rng.below(40);
                let bytes: Vec<u8> = (0..len).map(|_| rng.range_u8(1, 255)).collect();
                Some(cstr(&bytes))
            }
        })
        .collect();
    assert_same("C12 mixed x300", |w| {
        let f = f_print_line(w);
        for item in &seq {
            match item {
                None => unsafe { f(core::ptr::null()) },
                Some(s) => unsafe { f(s.as_ptr()) },
            }
        }
    });
}

/// C13: bad() called directly, once.
#[test]
fn cfg_c13_bad_direct() {
    assert_same("C13 bad()", |w| {
        let f = f_bad(w);
        unsafe { f() };
    });
}

/// C14: bad() called directly, 100 times (stack-slack reuse).
#[test]
fn cfg_c14_bad_repeated() {
    assert_same("C14 bad() x100", |w| {
        let f = f_bad(w);
        for _ in 0..100 {
            unsafe { f() };
        }
    });
}

/// C15: good() called directly, once.
#[test]
fn cfg_c15_good_direct() {
    assert_same("C15 good()", |w| {
        let f = f_good(w);
        unsafe { f() };
    });
}

/// C16: good() called directly, 100 times.
#[test]
fn cfg_c16_good_repeated() {
    assert_same("C16 good() x100", |w| {
        let f = f_good(w);
        for _ in 0..100 {
            unsafe { f() };
        }
    });
}

/// C17: driver(0) -> bad()
#[test]
fn cfg_c17_driver_false() {
    assert_same("C17 driver(0)", |w| {
        let f = f_driver(w);
        unsafe { f(0) };
    });
    // and it must equal what bad() prints on its own, in both impls
    for w in BOTH {
        let via_driver = capture(|| unsafe { f_driver(w)(0) });
        let direct = capture(|| unsafe { f_bad(w)() });
        assert_eq!(
            via_driver,
            direct,
            "{}: driver(0) must behave like bad()",
            w.name()
        );
    }
}

/// C18: driver(1) -> good()
#[test]
fn cfg_c18_driver_true() {
    assert_same("C18 driver(1)", |w| {
        let f = f_driver(w);
        unsafe { f(1) };
    });
    for w in BOTH {
        let via_driver = capture(|| unsafe { f_driver(w)(1) });
        let direct = capture(|| unsafe { f_good(w)() });
        assert_eq!(
            via_driver,
            direct,
            "{}: driver(1) must behave like good()",
            w.name()
        );
    }
}

/// C19: driver with 500 randomized non-zero values plus tricky bit patterns.
#[test]
fn cfg_c19_driver_random_nonzero() {
    let mut rng = Rng::new(0xC19);
    let mut values: Vec<i32> = vec![
        1,
        -1,
        2,
        -2,
        i32::MIN,
        i32::MAX,
        0x100,
        0xFFFF,
        0x7F00_0000,
        -0x8000_0000i64 as i32,
        0x0000_FF00,
        1 << 31i32.trailing_zeros(), // 1
    ];
    while values.len() < 500 {
        let v = rng.next_i32();
        if v != 0 {
            values.push(v);
        }
    }
    for &v in &values {
        assert_same(&format!("C19 driver({v})"), |w| {
            let f = f_driver(w);
            unsafe { f(v) };
        });
    }
}

/// C20: randomized sequence of driver() calls in one capture.
#[test]
fn cfg_c20_driver_random_sequence() {
    let mut rng = Rng::new(0xC20);
    let seq: Vec<i32> = (0..300)
        .map(|_| if rng.bool() { 0 } else { rng.next_i32() })
        .collect();
    assert_same("C20 driver sequence x300", |w| {
        let f = f_driver(w);
        for &v in &seq {
            unsafe { f(v) };
        }
    });
}

/// C21: randomized interleaving of ALL five exported entry points in one
/// capture — the composed pipeline against shared stdio state.
#[test]
fn cfg_c21_all_entry_points_interleaved() {
    #[derive(Clone)]
    enum Op {
        Driver(i32),
        Bad,
        Good,
        PrintInt(i32),
        PrintLine(Option<Vec<c_char>>),
    }

    let mut rng = Rng::new(0xC21);
    let ops: Vec<Op> = (0..200)
        .map(|_| match rng.below(5) {
            0 => Op::Driver(if rng.bool() { 0 } else { rng.next_i32() }),
            1 => Op::Bad,
            2 => Op::Good,
            3 => Op::PrintInt(rng.next_i32()),
            _ => {
                if rng.below(5) == 0 {
                    Op::PrintLine(None)
                } else {
                    let len = rng.below(50);
                    let bytes: Vec<u8> = (0..len).map(|_| rng.range_u8(1, 255)).collect();
                    Op::PrintLine(Some(cstr(&bytes)))
                }
            }
        })
        .collect();

    assert_same("C21 interleaved all entry points", |w| {
        let d = f_driver(w);
        let b = f_bad(w);
        let g = f_good(w);
        let pi = f_print_int_line(w);
        let pl = f_print_line(w);
        for op in &ops {
            unsafe {
                match op {
                    Op::Driver(v) => d(*v),
                    Op::Bad => b(),
                    Op::Good => g(),
                    Op::PrintInt(v) => pi(*v),
                    Op::PrintLine(None) => pl(core::ptr::null()),
                    Op::PrintLine(Some(s)) => pl(s.as_ptr()),
                }
            }
        }
    });
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

/// E1: printLine(NULL) -> the `if (line != NULL)` guard rejects; NO output.
#[test]
fn err_e1_print_line_null() {
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_print_line(w)(core::ptr::null()) });
        assert!(
            out.is_empty(),
            "{}: printLine(NULL) must produce no output, got {out:?}",
            w.name()
        );
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1], "E1: C and Rust must reject NULL identically");

    // repeated NULL calls stay silent
    assert_same("E1 printLine(NULL) x50", |w| {
        let f = f_print_line(w);
        for _ in 0..50 {
            unsafe { f(core::ptr::null()) };
        }
    });
}

/// E2: printLine("") — passes the guard, prints just "\n".
#[test]
fn err_e2_print_line_empty() {
    let s = cstr(b"");
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_print_line(w)(s.as_ptr()) });
        assert_eq!(out, b"\n".to_vec(), "{}: printLine(\"\")", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
}

/// E3: printLine with conversion specifiers — must NOT be interpreted.
#[test]
fn err_e3_print_line_percent() {
    let s = cstr(b"%s %d %n");
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_print_line(w)(s.as_ptr()) });
        assert_eq!(out, b"%s %d %n\n".to_vec(), "{}: literal % passthrough", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
}

/// E4: oversized (64 KiB) argument.
#[test]
fn err_e4_print_line_oversized() {
    let bytes = vec![b'A'; 65536];
    let s = cstr(&bytes);
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_print_line(w)(s.as_ptr()) });
        assert_eq!(out.len(), 65537, "{}: oversized length", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
}

/// E5: high / non-ASCII bytes are passed through verbatim.
#[test]
fn err_e5_print_line_high_bytes() {
    let bytes: Vec<u8> = (0x80u8..=0xFFu8).collect();
    let s = cstr(&bytes);
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_print_line(w)(s.as_ptr()) });
        let mut expect = bytes.clone();
        expect.push(b'\n');
        assert_eq!(out, expect, "{}: high-byte passthrough", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
}

/// E6: printIntLine(INT_MIN)
#[test]
fn err_e6_print_int_line_int_min() {
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_print_int_line(w)(i32::MIN) });
        assert_eq!(out, b"-2147483648\n".to_vec(), "{}: INT_MIN", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
}

/// E7: printIntLine(INT_MAX)
#[test]
fn err_e7_print_int_line_int_max() {
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_print_int_line(w)(i32::MAX) });
        assert_eq!(out, b"2147483647\n".to_vec(), "{}: INT_MAX", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
}

/// E8: printIntLine on sentinel-looking values.
#[test]
fn err_e8_print_int_line_sentinels() {
    for (v, expect) in [(0i32, "0\n"), (-1, "-1\n"), (1, "1\n")] {
        let mut outs = Vec::new();
        for w in BOTH {
            let out = capture(|| unsafe { f_print_int_line(w)(v) });
            assert_eq!(out, expect.as_bytes().to_vec(), "{}: printIntLine({v})", w.name());
            outs.push(out);
        }
        assert_eq!(outs[0], outs[1]);
    }
}

/// E9: driver(0) takes the bad() branch.
#[test]
fn err_e9_driver_zero() {
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_driver(w)(0) });
        assert_eq!(out, b"0\n".to_vec(), "{}: driver(0)", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
}

/// E10: out-of-range "enum-like" values crossing the FFI boundary.
/// A C `int` parameter accepts any value; every non-zero one is truthy.
#[test]
fn err_e10_driver_out_of_range_enum() {
    let values: [i32; 14] = [
        2,
        3,
        -1,
        -2,
        i32::MIN,
        i32::MAX,
        0x100,
        0xFFFF,
        0x1_0000,
        0x7FFF_FFFF,
        -0x7FFF_FFFF,
        1 << 16,
        1 << 30,
        (1u32 << 31) as i32,
    ];
    for &v in &values {
        let mut outs = Vec::new();
        for w in BOTH {
            let out = capture(|| unsafe { f_driver(w)(v) });
            assert_eq!(
                out,
                b"0\n".to_vec(),
                "{}: driver({v}) (non-zero => good())",
                w.name()
            );
            outs.push(out);
        }
        assert_eq!(outs[0], outs[1], "E10: driver({v}) diverged");
    }
    // and zero remains the only false value
    assert_same("E10 driver(0) vs nonzero", |w| {
        let f = f_driver(w);
        for &v in &values {
            unsafe { f(v) };
        }
        unsafe { f(0) };
    });
}

/// E11: bad() directly — the intentional CWE-806 under-allocation must still
/// print `0` and must not crash.
#[test]
fn err_e11_bad_direct_overflow() {
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_bad(w)() });
        assert_eq!(out, b"0\n".to_vec(), "{}: bad()", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
    // hammer it to shake out stack corruption
    assert_same("E11 bad() x1000", |w| {
        let f = f_bad(w);
        for _ in 0..1000 {
            unsafe { f() };
        }
    });
}

/// E12: good() directly — the `data = NULL` dead store must not be observable.
#[test]
fn err_e12_good_direct() {
    let mut outs = Vec::new();
    for w in BOTH {
        let out = capture(|| unsafe { f_good(w)() });
        assert_eq!(out, b"0\n".to_vec(), "{}: good()", w.name());
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1]);
    assert_same("E12 good() x1000", |w| {
        let f = f_good(w);
        for _ in 0..1000 {
            unsafe { f() };
        }
    });
}

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

/// Every symbol exported by the C `.so` must be resolvable in the Rust `.so`.
#[test]
fn sym_parity_all_c_exports_present_in_rust() {
    for name in [
        &b"driver\0"[..],
        &b"bad\0"[..],
        &b"good\0"[..],
        &b"printIntLine\0"[..],
        &b"printLine\0"[..],
    ] {
        assert_symbol_in_both(name);
    }
}

/// `bad` and `good` must be *distinguishable* behaviourally even if the
/// optimizer folds their addresses: both print `0\n`, and both are callable.
#[test]
fn sym_bad_and_good_are_independently_callable() {
    for w in BOTH {
        let b = capture(|| unsafe { f_bad(w)() });
        let g = capture(|| unsafe { f_good(w)() });
        assert_eq!(b, b"0\n".to_vec(), "{} bad()", w.name());
        assert_eq!(g, b"0\n".to_vec(), "{} good()", w.name());
    }
}

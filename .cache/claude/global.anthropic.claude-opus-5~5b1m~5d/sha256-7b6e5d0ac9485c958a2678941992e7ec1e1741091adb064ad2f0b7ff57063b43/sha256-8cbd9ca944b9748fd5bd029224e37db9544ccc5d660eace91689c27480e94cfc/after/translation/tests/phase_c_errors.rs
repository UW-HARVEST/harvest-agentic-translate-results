// Phase C -- error / rejection differential tests. One test per ERRORS.md row.

mod common;
use common::*;
use std::ffi::c_int;
use std::ptr;

// --- Row 1 ------------------------------------------------------------------
#[test]
fn e1_call_fma_len_zero_returns_zero() {
    let l = libs();
    let c_fn = l.c.call_fma();
    let r_fn = l.rust.call_fma();
    let mut rng = Rng::new(SEED ^ 101);
    for _ in 0..500 {
        let n = rng.range(0, 16);
        let data = rng.vec_any(n);
        let cv = unsafe { c_fn(data.as_ptr(), 0) };
        let rv = unsafe { r_fn(data.as_ptr(), 0) };
        assert_eq!(cv, rv, "[E1] len==0 diverged");
        assert_eq!(cv, 0, "[E1] C must return the sentinel 0 for len==0");
    }
}

// --- Row 2 ------------------------------------------------------------------
#[test]
fn e2_call_fma_len_zero_with_null_data() {
    // The `if (len == 0) return 0;` guard fires before any dereference, so a
    // NULL `data` must be accepted by both.
    let l = libs();
    let c_fn = l.c.call_fma();
    let r_fn = l.rust.call_fma();
    let cv = unsafe { c_fn(ptr::null(), 0) };
    let rv = unsafe { r_fn(ptr::null(), 0) };
    assert_eq!(cv, rv, "[E2] NULL data + len==0 diverged");
    assert_eq!(cv, 0, "[E2] must return 0");
}

// --- Row 3 ------------------------------------------------------------------
#[test]
fn e3_fma_array_len_zero_writes_nothing() {
    let l = libs();
    let c_fn = l.c.fma_array();
    let r_fn = l.rust.fma_array();
    let mut rng = Rng::new(SEED ^ 103);
    for _ in 0..300 {
        let cap = rng.range(1, 32);
        let m1 = rng.vec_any(cap);
        let m2 = rng.vec_any(cap);
        let a = rng.vec_any(cap);
        let mut c_out = vec![POISON; cap];
        let mut r_out = vec![POISON; cap];
        unsafe {
            c_fn(c_out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), 0);
            r_fn(r_out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), 0);
        }
        assert_eq!(c_out, r_out, "[E3] len==0 diverged");
        assert!(c_out.iter().all(|v| *v == POISON), "[E3] C wrote with len==0");
    }
}

// --- Row 4 ------------------------------------------------------------------
#[test]
fn e4_fma_array_negative_len_writes_nothing() {
    let l = libs();
    let c_fn = l.c.fma_array();
    let r_fn = l.rust.fma_array();
    let cap = 16usize;
    let mut rng = Rng::new(SEED ^ 104);
    let m1 = rng.vec_any(cap);
    let m2 = rng.vec_any(cap);
    let a = rng.vec_any(cap);
    // Every interesting negative, including one step past the valid range.
    let mut lens: Vec<c_int> = vec![-1, -2, -3, -16, -17, -100, -65536, i32::MIN, i32::MIN + 1];
    for _ in 0..100 {
        lens.push(-(rng.range(1, 1 << 20) as i32));
    }
    for len in lens {
        let mut c_out = vec![POISON; cap];
        let mut r_out = vec![POISON; cap];
        unsafe {
            c_fn(c_out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), len);
            r_fn(r_out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), len);
        }
        assert_eq!(c_out, r_out, "[E4] len={len} diverged");
        assert!(
            c_out.iter().all(|v| *v == POISON),
            "[E4] C wrote something with len={len}"
        );
    }
}

// --- Row 5 ------------------------------------------------------------------
#[test]
fn e5_fma_array_all_null_pointers_with_nonpositive_len() {
    let l = libs();
    let c_fn = l.c.fma_array();
    let r_fn = l.rust.fma_array();
    for len in [0, -1, -2, i32::MIN] {
        unsafe {
            c_fn(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), len);
            r_fn(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), len);
        }
    }
    // Reaching here without a segfault in either implementation is the assertion.
}

// --- Rows 6, 6a, 6b, 6c, 14 -------------------------------------------------
#[test]
fn e6_driver_no_parseable_integer_prints_zero() {
    let cases: &[(&[u8], &str)] = &[
        // 14 / 6a: sscanf returns EOF
        (b"", "E14/empty"),
        (b" ", "E6a/space"),
        (b"   ", "E6a/spaces"),
        (b"\n", "E6a/newline"),
        (b"\t\t", "E6a/tabs"),
        (b"\r\n \x0b\x0c", "E6a/all-ws"),
        // 6b: sscanf returns 0 (matching failure on a non-numeric char)
        (b"abc", "E6b/alpha"),
        (b"x1", "E6b/alpha-digit"),
        (b"+z", "E6b/sign-alpha"),
        (b".5", "E6b/dot"),
        (b",", "E6b/comma"),
        (b"e", "E6b/e"),
        (b"  hello world", "E6b/ws-then-alpha"),
        (b"\0", "E6b/nul-first"),
        // 6c: lone sign
        (b"+", "E6c/plus"),
        (b"-", "E6c/minus"),
        (b"- 5", "E6c/minus-space-5"),
        (b"+ 1", "E6c/plus-space-1"),
        (b"++1", "E6c/double-plus"),
        (b"--1", "E6c/double-minus"),
        (b"+-5", "E6c/plus-minus"),
        (b"  -  ", "E6c/ws-minus-ws"),
    ];
    for (input, row) in cases {
        let (c_out, r_out) = run_driver_both(input);
        assert_eq!(
            c_out,
            r_out,
            "[{row}] diverged on {:?}",
            String::from_utf8_lossy(input)
        );
        assert_eq!(c_out, b"0\n".to_vec(), "[{row}] C must print \"0\\n\"");
    }
}

// --- Row 7 ------------------------------------------------------------------
#[test]
fn e7_driver_early_break_returns_last_success() {
    let cases: &[(&[u8], &str)] = &[
        (b"1 2 xyz", "2"),
        (b"1 2 3 abc 9", "3"),
        (b"42 !", "42"),
        (b"-7 q 1000", "-7"),
        (b"5 6 7 +", "7"),
        (b"5 6 7 -", "7"),
        (b"1 2 3.9", "3"), // "3" then ".9" fails
    ];
    for (input, expect) in cases {
        let (c_out, r_out) = run_driver_both(input);
        assert_eq!(c_out, r_out, "[E7] diverged on {:?}", String::from_utf8_lossy(input));
        assert_eq!(
            String::from_utf8_lossy(&c_out).trim_end(),
            *expect,
            "[E7] unexpected C output for {:?}",
            String::from_utf8_lossy(input)
        );
    }
    // Randomized: valid prefix + garbage suffix.
    let mut rng = Rng::new(SEED ^ 107);
    for _ in 0..400 {
        let n = rng.range(1, 30);
        let nums = rng.vec_any(n);
        let mut s: Vec<u8> = nums
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
            .into_bytes();
        s.extend_from_slice(b" ");
        s.push(rng.pick(b"abcXYZ.,;!?*/"));
        s.extend_from_slice(b" 999");
        let (c_out, r_out) = run_driver_both(&s);
        assert_eq!(c_out, r_out, "[E7/random] diverged");
        assert_eq!(
            String::from_utf8_lossy(&c_out).trim_end(),
            nums[n - 1].to_string()
        );
    }
}

// --- Rows 8, 9, 10 ----------------------------------------------------------
#[test]
fn e8_e9_e10_driver_hard_cap_of_100() {
    let mut rng = Rng::new(SEED ^ 108);
    for n in [99usize, 100, 101, 102, 150, 500] {
        for _ in 0..20 {
            let nums = rng.vec_any(n);
            let input = nums
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .into_bytes();
            let (c_out, r_out) = run_driver_both(&input);
            assert_eq!(c_out, r_out, "[E8-10] diverged at n={n}");
            let expect = nums[n.min(100) - 1].to_string();
            assert_eq!(
                String::from_utf8_lossy(&c_out).trim_end(),
                expect,
                "[E8-10] n={n}: C must print element {}",
                n.min(100)
            );
        }
    }
}

// --- Row 11 -----------------------------------------------------------------
#[test]
fn e11_driver_int_overflow_wraps_and_is_not_rejected() {
    // Observed glibc ground truth: `%d` parses into a `long` and stores the
    // low 32 bits into the `int*`; it returns 1, so this is NOT a rejection.
    // (Values below were captured from the C .so itself, not assumed.)
    let cases: &[(&[u8], &str)] = &[
        (b"2147483648", "-2147483648"),
        (b"-2147483649", "2147483647"),
        (b"99999999999999999999", "-1"),
        (b"-99999999999999999999", "0"),
        (b"1 2147483648", "-2147483648"),
        (b"2147483648 5", "5"),
        (b"9223372036854775808", "-1"),
        (b"-9223372036854775809", "0"),
        (b"340282366920938463463374607431768211456", "-1"),
        (b"00000000000000002147483648", "-2147483648"),
        (b"4294967296", "0"),
        (b"4294967297", "1"),
    ];
    for (input, expect) in cases {
        let (c_out, r_out) = run_driver_both(input);
        assert_eq!(c_out, r_out, "[E11] diverged on {:?}", String::from_utf8_lossy(input));
        assert_eq!(
            String::from_utf8_lossy(&c_out).trim_end(),
            *expect,
            "[E11] unexpected C ground truth for {:?}",
            String::from_utf8_lossy(input)
        );
    }
}

// --- Row 12 -----------------------------------------------------------------
#[test]
fn e12_driver_hex_prefix_is_not_hex() {
    for input in [&b"0x10"[..], b"0X10", b"0xff", b"5 0x10", b"0x", b"00x1"] {
        let (c_out, r_out) = run_driver_both(input);
        assert_eq!(c_out, r_out, "[E12] diverged on {:?}", String::from_utf8_lossy(input));
    }
    // "0x10": %d parses 0, next iteration fails on 'x' -> result 0
    let (c_out, _) = run_driver_both(b"0x10");
    assert_eq!(String::from_utf8_lossy(&c_out).trim_end(), "0");
    // "5 0x10": tokens 5, 0 -> last success is 0
    let (c_out, _) = run_driver_both(b"5 0x10");
    assert_eq!(String::from_utf8_lossy(&c_out).trim_end(), "0");
}

// --- Row 13 -----------------------------------------------------------------
#[test]
fn e13_driver_embedded_nul_truncates() {
    // run_driver_both appends the real terminator; interior NULs stay interior.
    let cases: &[(&[u8], &str)] = &[
        (b"1 2\0 3 4", "2"),
        (b"\0 1 2", "0"),
        (b"7\0", "7"),
        (b"42\0999", "42"),
    ];
    for (input, expect) in cases {
        let (c_out, r_out) = run_driver_both(input);
        assert_eq!(c_out, r_out, "[E13] diverged on {input:?}");
        assert_eq!(String::from_utf8_lossy(&c_out).trim_end(), *expect);
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary coverage required by Phase C beyond the table.
// ---------------------------------------------------------------------------

/// `len` values one step past / around every boundary the C code distinguishes,
/// driven through `call_fma` with an oversized buffer so no OOB read occurs.
#[test]
fn generic_call_fma_len_boundaries() {
    let mut rng = Rng::new(SEED ^ 200);
    let buf = rng.vec_any(4096);
    for len in [0i32, 1, 2, 98, 99, 100, 101, 255, 256, 1023, 1024, 4095, 4096] {
        assert_call_fma_same(&buf, len, "generic/len-boundary");
    }
}

/// Out-of-range "enum-like" integers across the FFI boundary. This library has
/// no C `enum`, so the equivalent class of input is an arbitrary `int` in the
/// `len` parameter position -- including values with no meaningful variant.
/// `fma_array` is safe to probe with any `len <= 0`, and with `len` bounded by
/// the buffer for positives.
#[test]
fn generic_fma_array_arbitrary_int_len() {
    let l = libs();
    let c_fn = l.c.fma_array();
    let r_fn = l.rust.fma_array();
    let cap = 128usize;
    let mut rng = Rng::new(SEED ^ 201);
    let m1 = rng.vec_any(cap);
    let m2 = rng.vec_any(cap);
    let a = rng.vec_any(cap);

    let mut lens: Vec<c_int> = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, 127, 128];
    for _ in 0..200 {
        // Arbitrary ints, but clamp positives to `cap` so the C stays in bounds.
        let v = rng.i32_any();
        lens.push(if v > 0 { v % (cap as i32 + 1) } else { v });
    }
    for len in lens {
        let mut c_out = vec![POISON; cap];
        let mut r_out = vec![POISON; cap];
        unsafe {
            c_fn(c_out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), len);
            r_fn(r_out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), len);
        }
        assert_eq!(c_out, r_out, "[generic] fma_array diverged at len={len}");
    }
}

/// `driver` on zero-length and very long inputs (oversized length boundary).
#[test]
fn generic_driver_length_extremes() {
    assert_driver_same(b"", "generic/zero-length");
    // 64 KiB of whitespace: no token at all.
    assert_driver_same(&vec![b' '; 65536], "generic/huge-ws");
    // 64 KiB of digits: one gigantic token; glibc keeps the low 32 bits of the
    // saturated `long`, i.e. -1.
    let mut big = vec![b'9'; 65536];
    let (c_out, r_out) = run_driver_both(&big);
    assert_eq!(c_out, r_out, "[generic] huge digit token diverged");
    assert_eq!(String::from_utf8_lossy(&c_out).trim_end(), "-1");
    // 64 KiB of garbage: immediate rejection.
    big.iter_mut().for_each(|b| *b = b'z');
    assert_driver_same(&big, "generic/huge-garbage");
    // Many tokens, far past the 100 cap.
    let many: Vec<i32> = (1..=5000).collect();
    let input = many
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ")
        .into_bytes();
    let (c_out, r_out) = run_driver_both(&input);
    assert_eq!(c_out, r_out, "[generic] 5000 tokens diverged");
    assert_eq!(String::from_utf8_lossy(&c_out).trim_end(), "100");
}

/// Every single byte value 0..=255 as a one-character input, and as a suffix
/// after a valid integer. Exhaustively covers the `sscanf` accept/reject split.
#[test]
fn generic_driver_every_single_byte() {
    for b in 1u8..=255 {
        assert_driver_same(&[b], "generic/byte");
        assert_driver_same(&[b'4', b'2', b], "generic/42+byte");
        assert_driver_same(&[b'4', b'2', b' ', b], "generic/42_ws_byte");
        assert_driver_same(&[b, b'4', b'2'], "generic/byte+42");
    }
}

/// UB row U1: `call_fma` with a negative `len` declares a VLA of negative size,
/// which is UB in C (it was probed to return nondeterministic stack garbage).
/// The C side is deliberately NOT called; we only pin the Rust side's
/// deterministic, non-crashing behaviour.
#[test]
fn u1_rust_negative_len_is_deterministic_zero() {
    let l = libs();
    let r_fn = l.rust.call_fma();
    let data = [1, 2, 3, 4, 5, 6, 7, 8];
    for len in [-1i32, -2, -100, i32::MIN] {
        let rv = unsafe { r_fn(data.as_ptr(), len) };
        assert_eq!(rv, 0, "[U1] Rust call_fma(len={len}) should return 0");
        // and it must be stable across repeated calls
        let rv2 = unsafe { r_fn(data.as_ptr(), len) };
        assert_eq!(rv, rv2, "[U1] Rust call_fma(len={len}) is not deterministic");
    }
}

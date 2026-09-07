// Phase C — error / rejection-path differential tests.
// One test per row of ERRORS.md. Every call crosses the FFI boundary into
// BOTH `.so`s; each row asserts the SAME sentinel / clamp value, not merely
// "both misbehaved".

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const SEED: u64 = 0xE770_0000_0BAD_0001;

fn c_sdti(d: f64) -> c_int {
    unsafe { (pair().c.safe_double_to_int)(d) }
}
fn rs_sdti(d: f64) -> c_int {
    unsafe { (pair().rs.safe_double_to_int)(d) }
}
fn c_pwf(code: c_int, base: c_int) -> c_int {
    unsafe { (pair().c.process_with_fallthrough)(code, base) }
}
fn rs_pwf(code: c_int, base: c_int) -> c_int {
    unsafe { (pair().rs.process_with_fallthrough)(code, base) }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — d > (double)INT_MAX  =>  INT_MAX
// ---------------------------------------------------------------------------
#[test]
fn err01_sdti_above_int_max_clamps_to_int_max() {
    let imax = i32::MAX as f64;
    let mut cases: Vec<f64> = vec![
        f64::from_bits(imax.to_bits() + 1), // one representable step past INT_MAX
        2147483647.0000002,
        2147483648.0,
        2147483649.0,
        1e15,
        4294967296.0,
        f64::INFINITY,
        f64::MAX,
        1e300,
        9.9e307,
    ];
    let mut r = Rng::new(SEED ^ 1);
    for _ in 0..3000 {
        // uniformly random values strictly greater than INT_MAX
        let v = imax + (r.next_u64() >> 12) as f64 * 1.000_001;
        if v > imax {
            cases.push(v);
        }
    }
    for d in cases {
        let (cv, rv) = (c_sdti(d), rs_sdti(d));
        assert_eq!(cv, i32::MAX, "[err1] C did not clamp for {d:?}");
        assert_eq!(rv, cv, "[err1] safe_double_to_int({d:?}) C={cv} Rust={rv}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — d < (double)INT_MIN  =>  INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn err02_sdti_below_int_min_clamps_to_int_min() {
    let imin = i32::MIN as f64;
    let mut cases: Vec<f64> = vec![
        f64::from_bits(imin.to_bits() + 1), // one step further negative
        -2147483648.0000005,
        -2147483649.0,
        -4294967296.0,
        -1e15,
        f64::NEG_INFINITY,
        f64::MIN,
        -1e300,
        -9.9e307,
    ];
    let mut r = Rng::new(SEED ^ 2);
    for _ in 0..3000 {
        let v = imin - (r.next_u64() >> 12) as f64 * 1.000_001;
        if v < imin {
            cases.push(v);
        }
    }
    for d in cases {
        let (cv, rv) = (c_sdti(d), rs_sdti(d));
        assert_eq!(cv, i32::MIN, "[err2] C did not clamp for {d:?}");
        assert_eq!(rv, cv, "[err2] safe_double_to_int({d:?}) C={cv} Rust={rv}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — isnan(d) => 0  (reached only after both compares fail)
// ---------------------------------------------------------------------------
#[test]
fn err03_sdti_nan_returns_zero() {
    let mut cases: Vec<f64> = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN, min payload
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // all payload bits set
        f64::from_bits(0xFFF7_FFFF_FFFF_FFFF),
        0.0f64 / 0.0,
        f64::INFINITY - f64::INFINITY,
        (-1.0f64).sqrt(),
    ];
    let mut r = Rng::new(SEED ^ 3);
    for _ in 0..3000 {
        // random NaN payloads, both signs
        let payload = r.next_u64() & ((1u64 << 52) - 1);
        if payload == 0 {
            continue;
        }
        let sign = (r.next_u64() & 1) << 63;
        cases.push(f64::from_bits(sign | (0x7FFu64 << 52) | payload));
    }
    for d in cases {
        assert!(d.is_nan(), "test bug: {d:?} is not NaN");
        let (cv, rv) = (c_sdti(d), rs_sdti(d));
        assert_eq!(cv, 0, "[err3] C returned {cv} for NaN bits {:#018x}", d.to_bits());
        assert_eq!(rv, cv, "[err3] NaN bits {:#018x}: C={cv} Rust={rv}", d.to_bits());
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 4 & 5 — exact limits are NOT rejected (`>` / `<`, not `>=`)
// ---------------------------------------------------------------------------
#[test]
fn err04_err05_sdti_exact_limits_fall_through_to_cast() {
    let imax = i32::MAX as f64;
    let imin = i32::MIN as f64;
    let (cv, rv) = (c_sdti(imax), rs_sdti(imax));
    assert_eq!(cv, i32::MAX, "[err4] C changed behaviour at the exact INT_MAX");
    assert_eq!(rv, cv, "[err4] at exact INT_MAX: C={cv} Rust={rv}");
    let (cv, rv) = (c_sdti(imin), rs_sdti(imin));
    assert_eq!(cv, i32::MIN, "[err5] C changed behaviour at the exact INT_MIN");
    assert_eq!(rv, cv, "[err5] at exact INT_MIN: C={cv} Rust={rv}");
    // one representable step INSIDE the range on both ends
    for d in [
        f64::from_bits(imax.to_bits() - 1),
        f64::from_bits(imin.to_bits() - 1),
        2147483646.0,
        -2147483647.0,
        2147483646.9999998,
        -2147483647.9999998,
    ] {
        let (cv, rv) = (c_sdti(d), rs_sdti(d));
        assert_eq!(rv, cv, "[err4/5] safe_double_to_int({d:?}) C={cv} Rust={rv}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 — truncation toward zero, -0.0 maps to 0
// ---------------------------------------------------------------------------
#[test]
fn err06_sdti_truncates_toward_zero() {
    let cases: &[(f64, c_int)] = &[
        (0.0, 0),
        (-0.0, 0),
        (0.9, 0),
        (-0.9, 0),
        (0.999_999_999_999_999_9, 0),
        (-0.999_999_999_999_999_9, 0),
        (1.9, 1),
        (-1.9, -1),
        (f64::MIN_POSITIVE, 0),
        (-f64::MIN_POSITIVE, 0),
        (f64::from_bits(1), 0),
        (f64::from_bits(0x8000_0000_0000_0001), 0),
        (f64::EPSILON, 0),
        (-f64::EPSILON, 0),
    ];
    for &(d, want) in cases {
        let (cv, rv) = (c_sdti(d), rs_sdti(d));
        assert_eq!(cv, want, "[err6] C returned {cv} for {d:?}, expected {want}");
        assert_eq!(rv, cv, "[err6] safe_double_to_int({d:?}) C={cv} Rust={rv}");
        // sign of zero must not leak through
        assert_eq!(
            rv.to_string(),
            cv.to_string(),
            "[err6] textual result differs for {d:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 7 — `code` with no matching case label => default: -1
// This is the "out-of-range enum value across the FFI boundary" case: a C
// `switch` on an `int` that has no valid variant.
// ---------------------------------------------------------------------------
#[test]
fn err07_pwf_out_of_range_code_returns_minus_one() {
    let mut codes: Vec<c_int> = vec![
        -1, -2, -3, -4, -5, -6, -7, -100, 6, 7, 8, 9, 10, 100, 1000,
        i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1,
    ];
    let mut r = Rng::new(SEED ^ 7);
    for _ in 0..4000 {
        let mut c = r.next_i32();
        if (0..=5).contains(&c) {
            c = c.wrapping_add(6);
        }
        codes.push(c);
    }
    for code in codes {
        for base in [0, 1, -1, 12345, i32::MAX, i32::MIN, r.next_i32()] {
            let (cv, rv) = (c_pwf(code, base), rs_pwf(code, base));
            assert_eq!(
                cv, -1,
                "[err7] C returned {cv} for out-of-range code {code} (expected the default: -1)"
            );
            assert_eq!(
                rv, cv,
                "[err7] process_with_fallthrough({code},{base}) C={cv} Rust={rv}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 8 — code == 0 discards base_value entirely
// ---------------------------------------------------------------------------
#[test]
fn err08_pwf_code_zero_discards_base_value() {
    let mut r = Rng::new(SEED ^ 8);
    let mut bases: Vec<c_int> = vec![0, 1, -1, i32::MAX, i32::MIN, 999_999];
    for _ in 0..4000 {
        bases.push(r.next_i32());
    }
    for base in bases {
        let (cv, rv) = (c_pwf(0, base), rs_pwf(0, base));
        assert_eq!(cv, 0, "[err8] C returned {cv} for code=0, base={base}");
        assert_eq!(rv, cv, "[err8] process_with_fallthrough(0,{base}) C={cv} Rust={rv}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 9 — signed overflow of `result += N` in the fall-through chain
// ---------------------------------------------------------------------------
#[test]
fn err09_pwf_accumulator_overflow_matches() {
    for code in 1..=5 {
        for delta in 0..=200i32 {
            for base in [i32::MAX - delta, i32::MIN + delta] {
                let (cv, rv) = (c_pwf(code, base), rs_pwf(code, base));
                assert_eq!(
                    rv, cv,
                    "[err9] process_with_fallthrough({code},{base}) C={cv} Rust={rv}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 10 — sqrt() of a negative wrapped int => NaN => conv4 == 0
// ---------------------------------------------------------------------------
#[test]
fn err10_overunder_sqrt_of_negative_yields_conv4_zero() {
    // Inputs where d*d + a*a wraps negative. Assert the C really does print
    // conv4 == 0 (the 4th "Converted values" field) and that Rust matches
    // return value AND stdout.
    let cases: &[(c_int, c_int)] = &[
        (46341, 46341),
        (46341, 0),
        (0, 46341),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MIN, 0),
        (65536, 46341),
        (1_000_000, 1_000_000),
        (2_000_000_000, 1),
    ];
    for &(a, d) in cases {
        let sq = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
        if sq >= 0 {
            continue;
        }
        let ((cr, cout), (rr, rout)) = overunder_both(a, 3, 5, d);
        let s = String::from_utf8_lossy(&cout);
        let line = s
            .lines()
            .find(|l| l.starts_with("Converted values: "))
            .expect("[err10] no Converted values line");
        let last = line.rsplit(", ").next().unwrap();
        assert_eq!(
            last, "0",
            "[err10] C printed conv4={last} for a={a} d={d} (sqrt of negative should be NaN -> 0)"
        );
        assert_eq!(cr, rr, "[err10] overunder({a},3,5,{d}) return C={cr} Rust={rr}");
        assert_eq!(cout, rout, "[err10] overunder({a},3,5,{d}) stdout mismatch");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 11 — negative `a % 6` reaches the default: branch inside
// overunder, so "Switch fall-through result: -1"
// ---------------------------------------------------------------------------
#[test]
fn err11_overunder_negative_modulo_hits_default_branch() {
    let mut r = Rng::new(SEED ^ 11);
    for _ in 0..300 {
        let m = r.range_i32(1, 5);
        let a = -(r.range_i32(0, 100_000) * 6 + m);
        let ((cr, cout), (rr, rout)) = overunder_both(a, r.range_i32(-1000, 1000), r.range_i32(-1000, 1000), r.range_i32(-1000, 1000));
        let s = String::from_utf8_lossy(&cout);
        assert!(
            s.contains("Switch fall-through result: -1\n"),
            "[err11] C did not take the default branch for a={a}:\n{s}"
        );
        assert_eq!(cr, rr, "[err11] return mismatch for a={a}");
        assert_eq!(cout, rout, "[err11] stdout mismatch for a={a}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 12 — every argument at the signed extremes
// ---------------------------------------------------------------------------
#[test]
fn err12_overunder_extreme_arguments() {
    let ex = [i32::MIN, i32::MIN + 1, i32::MIN / 2, -1, 0, 1, i32::MAX / 2, i32::MAX - 1, i32::MAX];
    for &v in &ex {
        assert_overunder_eq("err12", v, v, v, v);
        for &w in &ex {
            assert_overunder_eq("err12", v, w, 1, 1);
            assert_overunder_eq("err12", 1, 1, v, w);
            assert_overunder_eq("err12", v, 1, w, 1);
            assert_overunder_eq("err12", 1, v, 1, w);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 13 — the strncpy length guard: label is "Source" + NUL fill,
// with label[19] explicitly cleared. Read the bytes back out of the struct
// that `overunder` copies, via the printed "%s" field.
// ---------------------------------------------------------------------------
#[test]
fn err13_label_bounded_and_nul_terminated() {
    let ((_, cout), (_, rout)) = overunder_both(6, 7, 8, 9);
    let s = String::from_utf8_lossy(&cout);
    let line = s.lines().find(|l| l.starts_with("Copied block: ")).unwrap();
    assert!(
        line.ends_with("label=Source"),
        "[err13] C label field is not exactly \"Source\": {line}"
    );
    assert_eq!(cout, rout, "[err13] stdout mismatch");

    // Independently confirm the label bytes survive copy_data_block unchanged,
    // including the zero padding strncpy writes and the explicit label[19]=0.
    let mut src = [0u8; DATABLOCK_SIZE];
    src[0..4].copy_from_slice(&9i32.to_le_bytes());
    src[8..16].copy_from_slice(&1.5f64.to_bits().to_le_bytes());
    src[16..22].copy_from_slice(b"Source");
    // bytes 22..36 stay 0 (strncpy zero-fill + explicit label[19] = '\0')
    assert_copy_eq("err13", &src, 0x77);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 14 — handle_pointer_operations overflow
// ---------------------------------------------------------------------------
#[test]
fn err14_hpo_overflow_matches() {
    let p = pair();
    let mut vals: Vec<c_int> = vec![i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, i32::MAX / 2, i32::MIN / 2];
    let mut r = Rng::new(SEED ^ 14);
    for _ in 0..5000 {
        vals.push(r.next_i32());
    }
    for k in -200..=200i32 {
        vals.push((i32::MAX / 2).wrapping_add(k));
        vals.push((i32::MIN / 2).wrapping_add(k));
    }
    for v in vals {
        let cv = unsafe { (p.c.handle_pointer_operations)(v) };
        let rv = unsafe { (p.rs.handle_pointer_operations)(v) };
        assert_eq!(rv, cv, "[err14] handle_pointer_operations({v}) C={cv} Rust={rv}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 15 — NULL pointers into copy_data_block.
// The C performs an unconditional memcpy, so this faults. We must not kill
// the test process, so each call is made in a forked child and the two
// implementations' wait-statuses are compared: they must fail IDENTICALLY
// (same signal), not merely "both failed somehow".
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Runs `f` in a forked child; returns the raw wait status.
fn status_of<F: FnOnce()>(f: F) -> c_int {
    unsafe {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut st: c_int = -1;
        assert!(waitpid(pid, &mut st, 0) == pid, "waitpid failed");
        st
    }
}

fn decode(st: c_int) -> String {
    if st & 0x7f == 0 {
        format!("exited({})", (st >> 8) & 0xff)
    } else {
        format!("signal({})", st & 0x7f)
    }
}

#[test]
fn err15_null_pointers_fail_identically() {
    let p = pair();
    let mut good = [0u64; BUF_SIZE / 8];
    let gp = good.as_mut_ptr() as *mut c_void;

    let variants: &[(&str, *mut c_void, *const c_void)] = &[
        ("dest=NULL src=NULL", std::ptr::null_mut(), std::ptr::null()),
        ("dest=NULL src=valid", std::ptr::null_mut(), gp as *const c_void),
        ("dest=valid src=NULL", gp, std::ptr::null()),
    ];

    for &(name, dest, src) in variants {
        let cst = status_of(|| unsafe { (p.c.copy_data_block)(dest, src) });
        let rst = status_of(|| unsafe { (p.rs.copy_data_block)(dest, src) });
        assert_eq!(
            decode(cst),
            decode(rst),
            "[err15] {name}: C {} vs Rust {} — the two must fail identically",
            decode(cst),
            decode(rst)
        );
        // Sanity: the C really does dereference unconditionally (no NULL check).
        assert_ne!(
            cst & 0x7f,
            0,
            "[err15] {name}: C unexpectedly returned normally — ERRORS.md row 15 needs revising"
        );
    }

    // Control: a valid call in the same forked-child harness exits cleanly for
    // BOTH, proving the harness itself is not the thing crashing.
    let mut s = [0u64; BUF_SIZE / 8];
    let sp = s.as_mut_ptr() as *const c_void;
    let cst = status_of(|| unsafe { (p.c.copy_data_block)(gp, sp) });
    let rst = status_of(|| unsafe { (p.rs.copy_data_block)(gp, sp) });
    assert_eq!(decode(cst), "exited(0)", "[err15] control: C crashed");
    assert_eq!(decode(rst), "exited(0)", "[err15] control: Rust crashed");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 16 — fully overlapping dest == src.
// ---------------------------------------------------------------------------
#[test]
fn err16_self_copy_is_identity_for_both() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 16);
    for _ in 0..500 {
        let mut cbuf = [0u64; BUF_SIZE / 8];
        for w in cbuf.iter_mut() {
            *w = r.next_u64();
        }
        let mut rbuf = cbuf;
        let before = cbuf;
        unsafe {
            let cp = cbuf.as_mut_ptr() as *mut c_void;
            (p.c.copy_data_block)(cp, cp as *const c_void);
            let rp = rbuf.as_mut_ptr() as *mut c_void;
            (p.rs.copy_data_block)(rp, rp as *const c_void);
        }
        assert_eq!(cbuf, rbuf, "[err16] self-copy diverged");
        assert_eq!(cbuf, before, "[err16] C self-copy was not the identity");
    }
}

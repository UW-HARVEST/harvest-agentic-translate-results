// Phase C -- error/rejection-path differential tests, one test per ERRORS.md row.
//
// Both libraries are exercised through their exported C ABI.  Rows that make the
// C library fault for real (E7, E24) are run in a forked child so the
// *termination signal* itself can be compared rather than merely "both failed".
mod common;

use common::*;
use std::ffi::c_char;

const MODES: [&[u8]; 4] = [b"standard", b"enhanced", b"turbo", b"extreme"];

fn nul(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Differential `classify_mode`, additionally asserting the C sentinel value.
fn classify_both(row: &str, s: &[u8], expect: i32) {
    let l = libs();
    let buf = nul(s);
    let p = buf.as_ptr() as *const c_char;
    let (c, rs) = unsafe { ((l.c.classify_mode)(p), (l.rs.classify_mode)(p)) };
    eq_int(row, show(s), c, rs);
    assert_eq!(
        c, expect,
        "[{row}] C returned {c:#X} for {:?}, expected sentinel {expect:#X}",
        show(s)
    );
}

fn apply_both(row: &str, base: i32, level: i32, expect: Option<i32>) {
    let l = libs();
    let (c, rs) = unsafe {
        (
            (l.c.apply_multiplier)(base, level),
            (l.rs.apply_multiplier)(base, level),
        )
    };
    eq_int(row, (base, level), c, rs);
    if let Some(e) = expect {
        assert_eq!(c, e, "[{row}] C returned {c:#X} for {:?}, expected {e:#X}", (base, level));
    }
}

fn ctf_both(row: &str, f: f64, expect: Option<i32>) {
    let l = libs();
    let (c, rs) = unsafe { ((l.c.convert_time_factor)(f), (l.rs.convert_time_factor)(f)) };
    eq_int(row, (f, f.to_bits()), c, rs);
    if let Some(e) = expect {
        assert_eq!(c, e, "[{row}] C returned {c} for {f:e}, expected {e}");
    }
}

fn cno_both(row: &str, f: f64, expect: Option<i32>) {
    let l = libs();
    let (c, rs) =
        unsafe { ((l.c.convert_negative_overflow)(f), (l.rs.convert_negative_overflow)(f)) };
    eq_int(row, (f, f.to_bits()), c, rs);
    if let Some(e) = expect {
        assert_eq!(c, e, "[{row}] C returned {c} for {f:e}, expected {e}");
    }
}

fn gmt_both(row: &str, d: i32, h: i32) -> i64 {
    let l = libs();
    let (c, rs) = unsafe { ((l.c.get_modified_time)(d, h), (l.rs.get_modified_time)(d, h)) };
    eq_i64(row, (d, h), c, rs);
    c
}

const NAN_PAYLOADS: [u64; 6] = [
    0x7FF8_0000_0000_0001, // quiet NaN
    0xFFF8_0000_0000_0001, // negative quiet NaN
    0x7FF0_0000_0000_0001, // signalling NaN
    0xFFF0_0000_0000_0001,
    0x7FFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
];

// ===========================================================================
// E1..E6 -- classify_mode's `else` fallback sentinel (0x00)
// ===========================================================================

#[test]
fn e1_classify_unknown_strings() {
    for s in [
        &b"unknown"[..],
        b"fast",
        b"STANDARD_MODE",
        b"0",
        b"null",
        b"extremely",
        b"standar",
    ] {
        classify_both("E1", s, 0x00);
    }
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..1500 {
        let len = 1 + rng.below(40) as usize;
        let s: Vec<u8> = (0..len).map(|_| 1 + rng.below(0xFF) as u8).collect();
        if MODES.contains(&s.as_slice()) {
            continue;
        }
        classify_both("E1", &s, 0x00);
    }
}

#[test]
fn e2_classify_empty_string() {
    classify_both("E2", b"", 0x00);
}

#[test]
fn e3_classify_strict_prefixes() {
    for m in MODES {
        for cut in 0..m.len() {
            classify_both("E3", &m[..cut], 0x00);
        }
    }
}

#[test]
fn e4_classify_trailing_bytes() {
    for m in MODES {
        for extra in [&b"X"[..], b" ", b"\t", b"\n", b"0", b"\x7F", b"\xFF", b"aa"] {
            let mut s = m.to_vec();
            s.extend_from_slice(extra);
            classify_both("E4", &s, 0x00);
        }
    }
}

#[test]
fn e5_classify_case_variants() {
    for s in [
        &b"Standard"[..],
        b"STANDARD",
        b"sTANDARD",
        b"Enhanced",
        b"ENHANCED",
        b"Turbo",
        b"TURBO",
        b"Extreme",
        b"EXTREME",
        b"eXtReMe",
    ] {
        classify_both("E5", s, 0x00);
    }
}

#[test]
fn e6_classify_high_bytes() {
    // strcmp compares as unsigned char, so 0x80.. must sort *above* ASCII.
    for m in MODES {
        let mut s = m.to_vec();
        s[0] = 0x80;
        classify_both("E6", &s, 0x00);
        let mut s = m.to_vec();
        s[0] = 0xFF;
        classify_both("E6", &s, 0x00);
        let mut s = m.to_vec();
        *s.last_mut().unwrap() = 0x80;
        classify_both("E6", &s, 0x00);
    }
    for s in [&b"\x80"[..], b"\xFF", b"\xFF\xFF\xFF", b"\x80standard"] {
        classify_both("E6", s, 0x00);
    }
}

// ===========================================================================
// E7 -- classify_mode(NULL): both must die with SIGSEGV
// ===========================================================================

#[test]
fn e7_classify_null_pointer_segfaults_identically() {
    let l = libs();

    // Positive control: the isolation harness reports a clean run correctly.
    let ok_c = run_isolated(|| unsafe {
        (l.c.classify_mode)(b"standard\0".as_ptr() as *const c_char) as i64
    });
    let ok_rs = run_isolated(|| unsafe {
        (l.rs.classify_mode)(b"standard\0".as_ptr() as *const c_char) as i64
    });
    assert_eq!(ok_c.status, 0, "harness: C control run failed: {ok_c:?}");
    assert_eq!(ok_rs.status, 0, "harness: Rust control run failed: {ok_rs:?}");
    assert_eq!(ok_c.ret, 0x10);
    assert_eq!(ok_rs.ret, 0x10);

    let c = run_isolated(|| unsafe { (l.c.classify_mode)(std::ptr::null()) as i64 });
    let rs = run_isolated(|| unsafe { (l.rs.classify_mode)(std::ptr::null()) as i64 });
    assert!(
        c.crashed_with(11),
        "E7: expected C to die with SIGSEGV, got {c:?}"
    );
    assert_eq!(
        c.status, rs.status,
        "E7: C terminated with {} but Rust with {} (C={c:?}, Rust={rs:?})",
        c.status, rs.status
    );
    eq_bytes("E7", "NULL", &c.stdout, &rs.stdout);
}

// ===========================================================================
// E8..E10 -- apply_multiplier's `default:` sentinel and accumulator overflow
// ===========================================================================

#[test]
fn e8_apply_negative_level() {
    let mut rng = Rng::new(SEED ^ 0xE8);
    for level in [-1i32, -2, -4, -5, -100, i32::MIN, i32::MIN + 1] {
        for base in [0i32, 1, -1, 0xA0, i32::MAX, i32::MIN] {
            apply_both("E8", base, level, Some(0xDEAD));
        }
    }
    for _ in 0..1000 {
        let level = -(1 + rng.below(1_000_000) as i32);
        apply_both("E8", rng.next_i32(), level, Some(0xDEAD));
    }
}

#[test]
fn e9_apply_level_above_four() {
    let mut rng = Rng::new(SEED ^ 0xE9);
    for level in [5i32, 6, 100, 0xDEAD, i32::MAX, i32::MAX - 1] {
        for base in [0i32, 1, -1, 0xA0, i32::MAX, i32::MIN] {
            apply_both("E9", base, level, Some(0xDEAD));
        }
    }
    for _ in 0..1000 {
        let level = 5 + rng.below(1_000_000) as i32;
        apply_both("E9", rng.next_i32(), level, Some(0xDEAD));
    }
}

#[test]
fn e10_apply_accumulator_signed_overflow() {
    // Deltas per level: 0->5, 1->0x21, 2->0x9F, 3->0x14A, 4->0x249.
    let deltas = [5i32, 0x21, 0x9F, 0x14A, 0x249];
    for (level, d) in deltas.iter().enumerate() {
        let level = level as i32;
        for base in [
            i32::MAX,
            i32::MAX - 1,
            i32::MAX - d + 1,
            i32::MAX - d,
            i32::MAX - d - 1,
            i32::MIN,
            i32::MIN + 1,
        ] {
            let expect = base.wrapping_add(*d);
            apply_both("E10", base, level, Some(expect));
        }
    }
    let mut rng = Rng::new(SEED ^ 0x10E);
    for _ in 0..1000 {
        let level = rng.below(5) as i32;
        let base = i32::MAX - rng.below(0x400) as i32;
        apply_both("E10", base, level, None);
    }
}

// ===========================================================================
// E11..E14 -- convert_time_factor: out-of-range / non-finite (int) casts
// ===========================================================================

#[test]
fn e11_ctf_overflow_positive() {
    let mut rng = Rng::new(SEED ^ 0xE11);
    for f in [
        2.147483648e-3f64,
        2.2e-3,
        1.0,
        1e6,
        1e100,
        1e300,
        f64::MAX,
        2147483648.0 / 1e12,
    ] {
        ctf_both("E11", f, Some(i32::MIN));
    }
    for _ in 0..1000 {
        let exp = rng.range_i32(-2, 300) as f64;
        let f = (1.0 + rng.below(1000) as f64 / 1000.0) * 10f64.powf(exp);
        if f * 1e12 <= i32::MAX as f64 {
            continue;
        }
        ctf_both("E11", f, Some(i32::MIN));
    }
}

#[test]
fn e12_ctf_overflow_negative() {
    let mut rng = Rng::new(SEED ^ 0xE12);
    for f in [
        -2.147483649e-3f64,
        -2.2e-3,
        -1.0,
        -1e6,
        -1e100,
        -1e300,
        f64::MIN,
        -2147483649.0 / 1e12,
    ] {
        ctf_both("E12", f, Some(i32::MIN));
    }
    for _ in 0..1000 {
        let exp = rng.range_i32(-2, 300) as f64;
        let f = -(1.0 + rng.below(1000) as f64 / 1000.0) * 10f64.powf(exp);
        if f * 1e12 >= i32::MIN as f64 {
            continue;
        }
        ctf_both("E12", f, Some(i32::MIN));
    }
}

#[test]
fn e13_ctf_nan() {
    for bits in NAN_PAYLOADS {
        let f = f64::from_bits(bits);
        assert!(f.is_nan());
        ctf_both("E13", f, Some(i32::MIN));
    }
    ctf_both("E13", f64::NAN, Some(i32::MIN));
    ctf_both("E13", -f64::NAN, Some(i32::MIN));
    ctf_both("E13", 0.0 / 0.0, Some(i32::MIN));
}

#[test]
fn e14_ctf_infinity() {
    ctf_both("E14", f64::INFINITY, Some(i32::MIN));
    ctf_both("E14", f64::NEG_INFINITY, Some(i32::MIN));
    // finite input whose *product* overflows to infinity
    ctf_both("E14", 1e300, Some(i32::MIN));
    ctf_both("E14", -1e300, Some(i32::MIN));
    ctf_both("E14", f64::MAX, Some(i32::MIN));
    ctf_both("E14", f64::MIN, Some(i32::MIN));
}

// ===========================================================================
// E15..E18 -- convert_negative_overflow: out-of-range / non-finite casts
// ===========================================================================

#[test]
fn e15_cno_overflow_via_negative_input() {
    // value * -1e15 > INT_MAX  <=>  value < -2.147483647e-6
    let mut rng = Rng::new(SEED ^ 0xE15);
    for f in [-2.147483648e-6f64, -3e-6, -1e-5, -1.0, -1e100, f64::MIN] {
        cno_both("E15", f, Some(i32::MIN));
    }
    for _ in 0..1000 {
        let exp = rng.range_i32(-5, 300) as f64;
        let f = -(1.0 + rng.below(1000) as f64 / 1000.0) * 10f64.powf(exp);
        if f * -1e15 <= i32::MAX as f64 {
            continue;
        }
        cno_both("E15", f, Some(i32::MIN));
    }
}

#[test]
fn e16_cno_overflow_via_positive_input() {
    let mut rng = Rng::new(SEED ^ 0xE16);
    for f in [2.147483649e-6f64, 3e-6, 1e-5, 1.0, 1e100, f64::MAX] {
        cno_both("E16", f, Some(i32::MIN));
    }
    for _ in 0..1000 {
        let exp = rng.range_i32(-5, 300) as f64;
        let f = (1.0 + rng.below(1000) as f64 / 1000.0) * 10f64.powf(exp);
        if f * -1e15 >= i32::MIN as f64 {
            continue;
        }
        cno_both("E16", f, Some(i32::MIN));
    }
}

#[test]
fn e17_cno_nan() {
    for bits in NAN_PAYLOADS {
        cno_both("E17", f64::from_bits(bits), Some(i32::MIN));
    }
    cno_both("E17", f64::NAN, Some(i32::MIN));
    cno_both("E17", -f64::NAN, Some(i32::MIN));
}

#[test]
fn e18_cno_infinity() {
    cno_both("E18", f64::INFINITY, Some(i32::MIN));
    cno_both("E18", f64::NEG_INFINITY, Some(i32::MIN));
    cno_both("E18", 1e300, Some(i32::MIN));
    cno_both("E18", -1e300, Some(i32::MIN));
}

// ===========================================================================
// E19..E21 -- get_modified_time: signed int overflow in the offset arithmetic
// ===========================================================================

/// `time(NULL) >> 29` -- the base both libraries add the offset to.
fn time_base() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    now >> 29
}

#[test]
fn e19_gmt_days_product_overflow() {
    let mut rng = Rng::new(SEED ^ 0xE19);
    let cases: Vec<i32> = [24856i32, -24856, 25000, -25000, i32::MAX, i32::MIN, 100000, -100000]
        .into_iter()
        .chain((0..1000).map(|_| {
            let mut v = rng.next_i32();
            if v.unsigned_abs() <= 24855 {
                v = 24856 + rng.below(1_000_000) as i32;
            }
            v
        }))
        .collect();
    for d in cases {
        let got = gmt_both("E19", d, 0);
        // wrapping in `int`, then sign-extension to time_t
        let expect = time_base().wrapping_add(d.wrapping_mul(86400) as i64);
        assert_eq!(
            got, expect,
            "E19: get_modified_time({d}, 0) = {got}, expected 32-bit wraparound {expect}"
        );
    }
}

#[test]
fn e20_gmt_hours_product_overflow() {
    let mut rng = Rng::new(SEED ^ 0xE20);
    let cases: Vec<i32> = [596524i32, -596524, 600000, -600000, i32::MAX, i32::MIN]
        .into_iter()
        .chain((0..1000).map(|_| {
            let mut v = rng.next_i32();
            if v.unsigned_abs() <= 596523 {
                v = 596524 + rng.below(1_000_000) as i32;
            }
            v
        }))
        .collect();
    for h in cases {
        let got = gmt_both("E20", 0, h);
        let expect = time_base().wrapping_add(h.wrapping_mul(3600) as i64);
        assert_eq!(
            got, expect,
            "E20: get_modified_time(0, {h}) = {got}, expected 32-bit wraparound {expect}"
        );
    }
}

#[test]
fn e21_gmt_sum_overflow_only() {
    // Neither product overflows, but their sum does.
    let mut rng = Rng::new(SEED ^ 0xE21);
    let mut n = 0;
    while n < 1000 {
        let d = rng.range_i32(-24855, 24855);
        let h = rng.range_i32(-596523, 596523);
        let pd = (d as i64) * 86400;
        let ph = (h as i64) * 3600;
        if pd.abs() > i32::MAX as i64 || ph.abs() > i32::MAX as i64 {
            continue;
        }
        let sum = pd + ph;
        if sum <= i32::MAX as i64 && sum >= i32::MIN as i64 {
            continue; // no sum overflow -- covered by C25
        }
        let got = gmt_both("E21", d, h);
        let expect = time_base()
            .wrapping_add(d.wrapping_mul(86400).wrapping_add(h.wrapping_mul(3600)) as i64);
        assert_eq!(got, expect, "E21: ({d},{h}) = {got}, expected {expect}");
        n += 1;
    }
    // deterministic extremes
    for (d, h) in [
        (24855, 596523),
        (-24855, -596523),
        (24855, 500000),
        (-24855, -500000),
    ] {
        gmt_both("E21", d, h);
    }
}

// ===========================================================================
// E22 / E23 -- hash_time_value: signed shift overflow, always non-negative
// ===========================================================================

#[test]
fn e22_hash_high_bit_in_top_lane() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE22);
    let mut cases: Vec<i64> = Vec::new();
    for b in [0x80u64, 0xFF, 0xC0, 0x81] {
        // lane 3 and lane 7 both map to shift 24 (i % 4 == 3)
        cases.push((b << 24) as i64);
        cases.push((b << 56) as i64);
        cases.push(((b << 24) | (b << 56)) as i64);
    }
    for _ in 0..1000 {
        // force the high bit of both `i % 4 == 3` lanes
        let v = rng.next_u64() | (0x80 << 24) | (0x80 << 56);
        cases.push(v as i64);
    }
    for t in cases {
        let (c, rs) = unsafe { ((l.c.hash_time_value)(t), (l.rs.hash_time_value)(t)) };
        eq_int("E22", t, c, rs);
        assert!(c >= 0, "E22: hash_time_value({t}) returned {c}");
    }
}

#[test]
fn e23_hash_never_negative() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE23);
    let mut cases: Vec<i64> = vec![-1, i64::MIN, i64::MAX, -2, -0x8000_0000_0000_0000];
    for _ in 0..2000 {
        cases.push(-(rng.next_i64().abs().max(1)));
    }
    for t in cases {
        let (c, rs) = unsafe { ((l.c.hash_time_value)(t), (l.rs.hash_time_value)(t)) };
        eq_int("E23", t, c, rs);
        assert!(
            c >= 0 && (c as u32) <= 0x7FFF_FFFF,
            "E23: hash_time_value({t}) = {c} is outside [0, 0x7FFFFFFF]"
        );
    }
}

// ===========================================================================
// E24 -- modeselect with a negative array index.
//
// `modes[mode_index]` for mode_index in {-1,-2,-3} reads UNINITIALISED stack
// memory, so its value is a function of the caller, not of the arguments (see
// the "E24 is not a contract" section of ERRORS.md for the measurements).  The
// three assertions below cover everything that IS well defined.
// ===========================================================================

/// Pre-fills roughly 32 KiB of stack *below* the current frame with `pat` and
/// returns, so that the region becomes the scratch space the next call uses.
#[inline(never)]
fn plant_stack(pat: u64, depth: u32) {
    let mut buf = [0u64; 512];
    for i in 0..512 {
        buf[i] = pat;
    }
    std::hint::black_box(&mut buf);
    if depth > 0 {
        plant_stack(pat, depth - 1);
    }
}

/// `modeselect`'s body with `mode_value` supplied explicitly instead of being
/// loaded out of `modes[mode_index]`; built purely from `lib`'s low-level
/// exports.  This is the *entire* defined content of the negative-index path.
fn pipeline_with_mode_value(
    lib: &Lib,
    mode_value: i32,
    time_offset: i32,
    complexity: i32,
    seed: i32,
) -> i32 {
    unsafe {
        let mut result: i32 = mode_value;
        let multiplier = (lib.apply_multiplier)(0xA0, complexity % 5);
        result = result.wrapping_add(multiplier);
        let modified_time = (lib.get_modified_time)(time_offset, seed % 24);
        let time_hash = (lib.hash_time_value)(modified_time);
        result = result.wrapping_add(time_hash % 0x1000);
        let result1 = (lib.convert_time_factor)((seed as f64) * 1e8);
        let result2 = (lib.convert_negative_overflow)((time_offset as f64) * -1e7);
        result ^= result1 & 0xFF;
        result ^= result2 & 0xFF00;
        result.wrapping_mul(0x10).wrapping_add(0xBEEF)
    }
}

/// (1) With the stack below deterministically zeroed, the out-of-bounds read
/// yields `NULL` in both libraries, so both must die with the SAME signal.
#[test]
fn e24a_negative_index_with_zeroed_stack_faults_identically() {
    let l = libs();
    for ms in [-1i32, -2, -3, -5, -6, -7] {
        assert_ne!(ms % 4, 0, "test bug: {ms} % 4 == 0");
        let c = run_isolated(|| {
            plant_stack(0, 8);
            unsafe { (l.c.modeselect)(ms, 1, 1, 1) as i64 }
        });
        let rs = run_isolated(|| {
            plant_stack(0, 8);
            unsafe { (l.rs.modeselect)(ms, 1, 1, 1) as i64 }
        });
        assert_eq!(
            c.status, rs.status,
            "E24a: mode_selector={ms}: C terminated with {} but Rust with {}\n  C   ={c:?}\n  Rust={rs:?}",
            c.status, rs.status
        );
        if c.ok() {
            // The garbage happened to be readable after all; then the *defined*
            // part must still hold for whichever run completed.
            e24_check_defined_part("E24a/C", &c, ms, 1, 1, 1, &l.c);
            e24_check_defined_part("E24a/Rust", &rs, ms, 1, 1, 1, &l.rs);
        } else {
            assert!(
                c.crashed_with(11),
                "E24a: expected SIGSEGV from the NULL read, got {c:?}"
            );
        }
        eq_bytes("E24a", ms, &c.stdout, &rs.stdout);
    }
}

/// Extracts the `mode_value` a run actually used out of its first printed line
/// (`Selected mode: <name> (0xNN)`), then checks the returned value equals the
/// pipeline formula for that `mode_value`.
fn e24_check_defined_part(
    row: &str,
    o: &Outcome,
    _ms: i32,
    time_offset: i32,
    complexity: i32,
    seed: i32,
    lib: &Lib,
) {
    let text = String::from_utf8_lossy(&o.stdout).to_string();
    let first = text.lines().next().unwrap_or("");
    let hex = first
        .rsplit_once("(0x")
        .and_then(|(_, t)| t.split_once(')'))
        .map(|(h, _)| h.to_string())
        .unwrap_or_else(|| panic!("[{row}] cannot parse mode_value out of {first:?}"));
    let mode_value = i64::from_str_radix(&hex, 16).expect("hex mode_value") as i32;
    let expect = pipeline_with_mode_value(lib, mode_value, time_offset, complexity, seed);
    assert_eq!(
        o.ret as i32, expect,
        "[{row}] returned {} but the pipeline with mode_value={mode_value:#X} gives {expect}",
        o.ret
    );
}

/// (2) Every `mode_value` the garbage load could possibly produce is pushed
/// through the rest of the pipeline with BOTH libraries' low-level exports; the
/// two must agree exactly.  This verifies the negative-index code path apart
/// from the single unspecified load.
#[test]
fn e24b_negative_index_pipeline_agrees_for_every_possible_mode_value() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x24B);
    for mode_value in [0x00i32, 0x10, 0x20, 0x30, 0x40] {
        for _ in 0..400 {
            let to = rng.next_i32();
            let cx = rng.next_i32();
            let sd = rng.next_i32();
            let c = pipeline_with_mode_value(&l.c, mode_value, to, cx, sd);
            let rs = pipeline_with_mode_value(&l.rs, mode_value, to, cx, sd);
            eq_int("E24b", (mode_value, to, cx, sd), c, rs);
        }
        for &(to, cx, sd) in &[
            (0i32, 0i32, 0i32),
            (1, 1, 1),
            (-1, -1, -1),
            (i32::MAX, i32::MAX, i32::MAX),
            (i32::MIN, i32::MIN, i32::MIN),
        ] {
            let c = pipeline_with_mode_value(&l.c, mode_value, to, cx, sd);
            let rs = pipeline_with_mode_value(&l.rs, mode_value, to, cx, sd);
            eq_int("E24b", (mode_value, to, cx, sd), c, rs);
        }
    }
}

/// (3) Characterisation: the C library's own outcome for a negative index is not
/// a function of its arguments.  Recorded (not asserted, because it depends on
/// stack layout) so the exemption in ERRORS.md stays honest and visible.
#[test]
fn e24c_negative_index_is_caller_dependent_in_the_c_library() {
    let l = libs();
    static TURBO: [u8; 6] = *b"turbo\0";
    let mut classes = std::collections::BTreeSet::new();
    for pat in [0u64, TURBO.as_ptr() as u64, 0x4141_4141_4141_4141] {
        for ms in [-1i32, -2, -3] {
            let c = run_isolated(|| {
                plant_stack(pat, 8);
                unsafe { (l.c.modeselect)(ms, 1, 1, 1) as i64 }
            });
            let first = String::from_utf8_lossy(&c.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            eprintln!(
                "E24c: C, stack pattern {pat:#018x}, mode_selector={ms:3} -> status={:4} ret={:8} first_line={:?}",
                c.status, c.ret, first
            );
            classes.insert(c.status);
            if c.ok() {
                e24_check_defined_part("E24c", &c, ms, 1, 1, 1, &l.c);
            }
        }
    }
    eprintln!(
        "E24c: the C library produced {} distinct termination class(es) {:?} for the SAME inputs, \
         differing only in caller stack content",
        classes.len(),
        classes
    );
}

// ===========================================================================
// E25 -- negative mode_selector that is a multiple of 4 is a *valid* input
// ===========================================================================

#[test]
fn e25_modeselect_negative_multiple_of_four() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE25);
    let mut cases: Vec<i32> = vec![-4, -8, -12, -400, i32::MIN, i32::MIN + 4];
    for _ in 0..200 {
        cases.push(-4 * (1 + rng.below(500_000_000) as i32));
    }
    for ms in cases {
        assert_eq!(ms % 4, 0);
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(ms, 7, 3, 5) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(ms, 7, 3, 5) });
        eq_int("E25", ms, cr, rr);
        eq_bytes("E25", ms, &cout, &rout);
        assert!(
            cout.starts_with(b"Selected mode: standard"),
            "E25: expected index 0 for {ms}, got {}",
            show(&cout)
        );
    }
}

// ===========================================================================
// E26 -- negative complexity => apply_multiplier's default arm (0xDEAD)
// ===========================================================================

#[test]
fn e26_modeselect_negative_complexity() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE26);
    for _ in 0..400 {
        let mi = rng.below(4) as i32;
        let ms = mi + 4 * rng.range_i32(0, 100_000);
        let cx = -(1 + rng.below(1_000_000) as i32);
        let to = rng.next_i32();
        let sd = rng.next_i32();
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(ms, to, cx, sd) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(ms, to, cx, sd) });
        eq_int("E26", (ms, to, cx, sd), cr, rr);
        eq_bytes("E26", (ms, to, cx, sd), &cout, &rout);
        if cx % 5 != 0 {
            assert!(
                String::from_utf8_lossy(&cout).contains("Multiplier: 0xDEAD"),
                "E26: expected the 0xDEAD sentinel for complexity={cx}, got {}",
                show(&cout)
            );
        }
    }
    // deterministic: every negative residue class mod 5
    for cx in [-1i32, -2, -3, -4, -5, -6, i32::MIN] {
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(0, 0, cx, 0) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(0, 0, cx, 0) });
        eq_int("E26", cx, cr, rr);
        eq_bytes("E26", cx, &cout, &rout);
    }
}

// ===========================================================================
// E27 / E28 -- the two conversions inside modeselect always saturate to INT_MIN
// ===========================================================================

#[test]
fn e27_modeselect_result1_is_int_min_unless_seed_zero() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE27);
    for i in 0..300 {
        let sd = if i == 0 { 1 } else { rng.next_i32() | 1 };
        let ms = rng.range_i32(0, 1000);
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(ms, 0, 0, sd) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(ms, 0, 0, sd) });
        eq_int("E27", sd, cr, rr);
        eq_bytes("E27", sd, &cout, &rout);
        let text = String::from_utf8_lossy(&cout).to_string();
        assert!(
            text.contains("Result 1: -2147483648 (0x80000000)"),
            "E27: seed={sd} should saturate result1, got {}",
            show(&cout)
        );
    }
    // seed == 0 is the only in-range case
    let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(0, 0, 0, 0) });
    let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(0, 0, 0, 0) });
    eq_int("E27", 0, cr, rr);
    eq_bytes("E27", 0, &cout, &rout);
    assert!(
        String::from_utf8_lossy(&cout).contains("Result 1: 0 (0x0)"),
        "E27: seed=0 should give result1 == 0, got {}",
        show(&cout)
    );
}

#[test]
fn e28_modeselect_result2_is_int_min_unless_time_offset_zero() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE28);
    for i in 0..300 {
        let to = if i == 0 { 1 } else { rng.next_i32() | 1 };
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(0, to, 0, 0) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(0, to, 0, 0) });
        eq_int("E28", to, cr, rr);
        eq_bytes("E28", to, &cout, &rout);
        assert!(
            String::from_utf8_lossy(&cout).contains("Result 2: -2147483648 (0x80000000)"),
            "E28: time_offset={to} should saturate result2, got {}",
            show(&cout)
        );
    }
    let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(0, 0, 0, 0) });
    let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(0, 0, 0, 0) });
    eq_int("E28", 0, cr, rr);
    eq_bytes("E28", 0, &cout, &rout);
    assert!(
        String::from_utf8_lossy(&cout).contains("Result 2: 0 (0x0)"),
        "E28: time_offset=0 should give result2 == 0, got {}",
        show(&cout)
    );
}

// ===========================================================================
// E29 -- the final `result * 0x10 + 0xBEEF` is provably in range; verify that
//        both libraries agree and that no wraparound ever occurs.
// ===========================================================================

#[test]
fn e29_final_multiply_never_overflows() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE29);
    for _ in 0..600 {
        let mut ms = rng.next_i32();
        if ms < 0 && ms % 4 != 0 {
            ms = -ms;
        }
        let to = rng.next_i32();
        let cx = rng.next_i32();
        let sd = rng.next_i32();
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(ms, to, cx, sd) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(ms, to, cx, sd) });
        eq_int("E29", (ms, to, cx, sd), cr, rr);
        eq_bytes("E29", (ms, to, cx, sd), &cout, &rout);
        // result before the final step is at most 0x40 + 0xDEAD + 0xFFF
        let max_pre = 0x40 + 0xDEAD + 0xFFF;
        assert!(
            cr >= 0xBEEF && cr <= max_pre * 0x10 + 0xBEEF,
            "E29: modeselect{:?} returned {cr}, outside the provable range",
            (ms, to, cx, sd)
        );
    }
}

// ===========================================================================
// E30 -- out-of-range discriminants crossing the FFI boundary
// ===========================================================================

#[test]
fn e30_out_of_range_discriminants() {
    let l = libs();
    // `level` is the closest thing to an enum in this API: only 0..=4 are valid
    // variants, and C accepts any int.  One step past each end plus the extremes.
    for level in [-1i32, 5, i32::MIN, i32::MAX, 0x8000_0000u32 as i32, 0x7FFF_FFFF] {
        apply_both("E30/level", 0xA0, level, Some(0xDEAD));
        apply_both("E30/level", 0, level, Some(0xDEAD));
    }
    // Boundary values that ARE valid variants, one step inside each end.
    apply_both("E30/level", 0xA0, 0, Some(0xA5));
    apply_both("E30/level", 0xA0, 4, Some(0x2E9));

    // `mode_selector` discriminant: one step past the 0..=3 index range in the
    // positive direction just wraps via `% 4`, which must match exactly.
    let mut rng = Rng::new(SEED ^ 0xE30);
    for ms in [4i32, 5, 7, 8, i32::MAX, i32::MAX - 1, i32::MAX - 2, i32::MAX - 3] {
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(ms, 0, 0, 0) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(ms, 0, 0, 0) });
        eq_int("E30/mode_selector", ms, cr, rr);
        eq_bytes("E30/mode_selector", ms, &cout, &rout);
    }
    // `complexity` discriminant one step past 0..=4.
    for cx in [5i32, 6, 9, 10, i32::MAX, i32::MIN] {
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(0, 0, cx, 0) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(0, 0, cx, 0) });
        eq_int("E30/complexity", cx, cr, rr);
        eq_bytes("E30/complexity", cx, &cout, &rout);
    }
    for _ in 0..300 {
        apply_both("E30/level", rng.next_i32(), rng.next_i32(), None);
    }
}

// ===========================================================================
// Generic FFI boundaries every C API has (beyond the ERRORS.md rows)
// ===========================================================================

#[test]
fn generic_zero_and_oversized_string_lengths() {
    // zero length
    classify_both("generic/zero-len", b"", 0x00);
    // 1 MiB of non-NUL bytes -- strcmp must bail on the first mismatch
    let big = vec![b'z'; 1 << 20];
    classify_both("generic/oversized", &big, 0x00);
    // 1 MiB that begins with a valid mode
    let mut big2 = b"standard".to_vec();
    big2.resize(1 << 20, b'q');
    classify_both("generic/oversized-prefix", &big2, 0x00);
    // exactly a valid mode inside an otherwise huge buffer (NUL right after)
    let mut buf = vec![0u8; 1 << 16];
    buf[..8].copy_from_slice(b"standard");
    let l = libs();
    let p = buf.as_ptr() as *const c_char;
    let (c, rs) = unsafe { ((l.c.classify_mode)(p), (l.rs.classify_mode)(p)) };
    eq_int("generic/embedded-nul", "standard+padding", c, rs);
    assert_eq!(c, 0x10);
}

#[test]
fn generic_one_past_valid_ranges() {
    // apply_multiplier: 0..=4 valid, so -1 and 5 are one step past each end.
    apply_both("generic/level-1", 0xA0, -1, Some(0xDEAD));
    apply_both("generic/level5", 0xA0, 5, Some(0xDEAD));
    // convert_time_factor: exactly on and one ULP past the representable edge
    let edge_hi = 2147483647.0f64 / 1e12;
    let edge_lo = -2147483648.0f64 / 1e12;
    ctf_both("generic/ctf-edge", edge_hi, None);
    ctf_both("generic/ctf-edge", edge_lo, None);
    for k in 1..=4u64 {
        ctf_both("generic/ctf-edge", f64::from_bits(edge_hi.to_bits() + k), None);
        ctf_both("generic/ctf-edge", f64::from_bits(edge_hi.to_bits() - k), None);
        ctf_both("generic/ctf-edge", f64::from_bits(edge_lo.to_bits() + k), None);
        ctf_both("generic/ctf-edge", f64::from_bits(edge_lo.to_bits() - k), None);
    }
    // get_modified_time: exactly on and one past the non-overflowing edge
    for d in [24855i32, 24856, -24855, -24856] {
        gmt_both("generic/gmt-edge", d, 0);
    }
    for h in [596523i32, 596524, -596523, -596524] {
        gmt_both("generic/gmt-edge", 0, h);
    }
    // hash_time_value: extremes of the time_t domain
    let l = libs();
    for t in [i64::MIN, i64::MIN + 1, i64::MAX, i64::MAX - 1, 0, -1] {
        let (c, rs) = unsafe { ((l.c.hash_time_value)(t), (l.rs.hash_time_value)(t)) };
        eq_int("generic/hash-edge", t, c, rs);
    }
}

#[test]
fn generic_extreme_modeselect_arguments() {
    let l = libs();
    for &(a, b, c, d) in &[
        (0i32, i32::MAX, i32::MAX, i32::MAX),
        (0, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX, i32::MIN, i32::MAX), // i32::MIN % 4 == 0 -> valid
        (i32::MIN, 0, 0, 0),
        (i32::MAX, 0, 0, 0),
    ] {
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(a, b, c, d) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(a, b, c, d) });
        eq_int("generic/extreme", (a, b, c, d), cr, rr);
        eq_bytes("generic/extreme", (a, b, c, d), &cout, &rout);
    }
}

// Phase C — error / rejection-path differential tests.
//
// One test per row of ERRORS.md. Every test constructs the exact invalid input
// or degenerate condition the C code reacts to, runs it through BOTH shared
// libraries, and asserts they produce the SAME sentinel / error / signal — not
// merely "both failed somehow".

mod common;

use common::{Api, Rng, assert_same, assert_same_io, both, run_child};
use std::ffi::CString;

const INT_MIN: i32 = i32::MIN;
const DEAD: i32 = 0xDEAD;

// ===========================================================================
// row 1 — classify_mode: unrecognised mode string -> sentinel 0x00
// ===========================================================================

#[test]
fn err_row01_classify_mode_unrecognized() {
    let (c, r) = both();
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"STANDARD".to_vec(),
        b"standar".to_vec(),
        b"standardx".to_vec(),
        b"turbo\x01".to_vec(),
        b"extremee".to_vec(),
        b" standard".to_vec(),
        b"enhance".to_vec(),
        b"\x01".to_vec(),
        b"\xff\xfe\xfd".to_vec(),
        vec![b'a'; 4096],
    ];
    let mut rng = Rng::with_seed(1);
    for _ in 0..2000 {
        let len = (rng.below(40) + 1) as usize;
        cases.push((0..len).map(|_| (rng.below(255) + 1) as u8).collect());
    }
    for case in cases {
        let cs = CString::new(case.clone()).unwrap();
        let cv = unsafe { (c.classify_mode)(cs.as_ptr()) };
        let rv = unsafe { (r.classify_mode)(cs.as_ptr()) };
        assert_eq!(rv, cv, "classify_mode({:?}) diverged", &case[..case.len().min(16)]);
        assert_eq!(cv, 0x00, "C sentinel changed for {:?}", &case[..case.len().min(16)]);
    }
}

// ===========================================================================
// row 2 — classify_mode(NULL): dereferenced by strcmp, must fault identically
// ===========================================================================

#[test]
#[ignore = "child process only; crashes on purpose"]
fn child_classify_mode_null() {
    if !common::is_child() {
        return;
    }
    let which = std::env::var("DIFFTEST_LIB").unwrap_or_default();
    let api = if which == "c" {
        common::c_api()
    } else {
        common::rust_api()
    };
    let v = unsafe { (api.classify_mode)(std::ptr::null()) };
    // If we get here there was no fault; report it through the exit code so the
    // parent can tell "returned" from "crashed".
    println!("classify_mode(NULL) returned {v}");
    std::process::exit(70);
}

#[test]
fn err_row02_classify_mode_null() {
    let c = run_child("child_classify_mode_null", &[("DIFFTEST_LIB", "c".into())]);
    let r = run_child("child_classify_mode_null", &[("DIFFTEST_LIB", "rust".into())]);
    assert_eq!(
        c, r,
        "classify_mode(NULL): C gave (signal,code)={c:?} but Rust gave {r:?}"
    );
    assert_eq!(
        c.0,
        Some(11),
        "expected SIGSEGV from classify_mode(NULL), got {c:?}"
    );
}

// ===========================================================================
// row 3 — apply_multiplier: level outside 0..=4 -> sentinel 0xDEAD
// ===========================================================================

#[test]
fn err_row03_apply_multiplier_invalid_level() {
    let (c, r) = both();
    let mut levels: Vec<i32> = vec![
        -1,
        5,
        6,
        7,
        -2,
        -5,
        100,
        -100,
        INT_MIN,
        i32::MAX,
        INT_MIN + 1,
        i32::MAX - 1,
        0x10,
        -0x10,
    ];
    let mut rng = Rng::with_seed(3);
    for _ in 0..2000 {
        let l = rng.next_i32();
        if !(0..=4).contains(&l) {
            levels.push(l);
        }
    }
    for lvl in levels {
        for base in [0xA0, 0, 1, -1, i32::MAX, INT_MIN, 0x7FFF_0000] {
            let cv = unsafe { (c.apply_multiplier)(base, lvl) };
            let rv = unsafe { (r.apply_multiplier)(base, lvl) };
            assert_eq!(rv, cv, "apply_multiplier({base},{lvl}) diverged");
            assert_eq!(
                cv, DEAD,
                "C default-case sentinel changed for level {lvl}, base {base}"
            );
        }
    }
}

// ===========================================================================
// row 4 — apply_multiplier: signed overflow of the fall-through additions
// ===========================================================================

#[test]
fn err_row04_apply_multiplier_base_overflow() {
    // total added per level: 5, 33, 179, 350, 605
    let totals = [5i32, 0x1C + 5, 0x7E + 0x1C + 5, 0xAB + 0x7E + 0x1C + 5,
                  0xFF + 0xAB + 0x7E + 0x1C + 5];
    let (c, r) = both();
    for (lvl, total) in totals.iter().enumerate() {
        for delta in -3i32..=3 {
            for base in [
                i32::MAX.wrapping_sub(*total).wrapping_add(delta),
                INT_MIN.wrapping_sub(*total).wrapping_add(delta),
                i32::MAX,
                i32::MAX - 1,
                INT_MIN,
                INT_MIN + 1,
            ] {
                let lvl = lvl as i32;
                let cv = unsafe { (c.apply_multiplier)(base, lvl) };
                let rv = unsafe { (r.apply_multiplier)(base, lvl) };
                assert_eq!(rv, cv, "apply_multiplier({base},{lvl}) overflow diverged");
                assert_eq!(
                    cv,
                    base.wrapping_add(*total),
                    "C did not wrap as expected for ({base},{lvl})"
                );
            }
        }
    }
}

// ===========================================================================
// rows 5..9 — convert_time_factor: out-of-int-range / NaN / inf / zero
// ===========================================================================

#[test]
fn err_row05_09_convert_time_factor_ranges() {
    let (c, r) = both();
    let check = |v: f64, expect: Option<i32>, row: &str| {
        let cv = unsafe { (c.convert_time_factor)(v) };
        let rv = unsafe { (r.convert_time_factor)(v) };
        assert_eq!(rv, cv, "convert_time_factor({v:e}) diverged ({row})");
        if let Some(e) = expect {
            assert_eq!(cv, e, "C result for {v:e} changed ({row})");
        }
    };

    // row 5: product >= 2^31 (note 2.147e-3 * 1e12 is still *inside* the range,
    // the threshold is 2.147483648e-3)
    for v in [1.0f64, 2.1475e-3, 1e-2, 1e3, 1e100, f64::MAX, 0.0021474836480000001] {
        check(v, Some(INT_MIN), "row5");
    }
    // row 6: product < -2^31
    for v in [-1.0f64, -2.1475e-3, -1e-2, -1e3, -1e100, f64::MIN] {
        check(v, Some(INT_MIN), "row6");
    }
    // row 7: NaN
    for v in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001),
        f64::from_bits(0xFFF8_0000_0000_0001),
    ] {
        check(v, Some(INT_MIN), "row7");
    }
    // row 8: infinities
    for v in [f64::INFINITY, f64::NEG_INFINITY] {
        check(v, Some(INT_MIN), "row8");
    }
    // row 9: zero / subnormal -> 0 (boundary of the rows above)
    for v in [
        0.0f64,
        -0.0,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        1e-320,
        -1e-320,
    ] {
        check(v, Some(0), "row9");
    }
    // exact boundary: the smallest |factor| whose product leaves int range
    let up = 2147483648.0f64;
    for k in 0..64u64 {
        let b = (up / 1e12).to_bits();
        check(f64::from_bits(b.wrapping_add(k)), None, "row5-boundary");
        check(f64::from_bits(b.wrapping_sub(k)), None, "row5-boundary");
        let b = (-up / 1e12).to_bits();
        check(f64::from_bits(b.wrapping_add(k)), None, "row6-boundary");
        check(f64::from_bits(b.wrapping_sub(k)), None, "row6-boundary");
    }
}

// ===========================================================================
// rows 10..13 — convert_negative_overflow: same, with the -1e15 factor
// ===========================================================================

#[test]
fn err_row10_13_convert_negative_overflow_ranges() {
    let (c, r) = both();
    let check = |v: f64, expect: Option<i32>, row: &str| {
        let cv = unsafe { (c.convert_negative_overflow)(v) };
        let rv = unsafe { (r.convert_negative_overflow)(v) };
        assert_eq!(rv, cv, "convert_negative_overflow({v:e}) diverged ({row})");
        if let Some(e) = expect {
            assert_eq!(cv, e, "C result for {v:e} changed ({row})");
        }
    };

    // row 10: value * -1e15 >= 2^31  (negative value)
    for v in [-1.0f64, -2.2e-6, -1e-3, -1e100, f64::MIN] {
        check(v, Some(INT_MIN), "row10");
    }
    // row 11: value * -1e15 < -2^31  (positive value)
    for v in [1.0f64, 2.2e-6, 1e-3, 1e100, f64::MAX] {
        check(v, Some(INT_MIN), "row11");
    }
    // row 12: NaN
    for v in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001),
        f64::from_bits(0xFFF8_0000_0000_0001),
    ] {
        check(v, Some(INT_MIN), "row12");
    }
    // row 13: infinities
    for v in [f64::INFINITY, f64::NEG_INFINITY] {
        check(v, Some(INT_MIN), "row13");
    }
    // in-range boundary (kept here so the rejection threshold is pinned)
    for v in [
        0.0f64,
        -0.0,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
    ] {
        check(v, Some(0), "row13-zero");
    }
    let up = 2147483648.0f64;
    for k in 0..64u64 {
        let b = (up / 1e15).to_bits();
        check(f64::from_bits(b.wrapping_add(k)), None, "boundary+");
        check(f64::from_bits(b.wrapping_sub(k)), None, "boundary+");
        let b = (-up / 1e15).to_bits();
        check(f64::from_bits(b.wrapping_add(k)), None, "boundary-");
        check(f64::from_bits(b.wrapping_sub(k)), None, "boundary-");
    }
}

// ===========================================================================
// rows 14..16 — get_modified_time: `int` overflow of the offset arithmetic
// ===========================================================================

#[test]
fn err_row14_16_get_modified_time_int_overflow() {
    let (c, r) = both();
    let base = |d: i32, h: i32| -> i64 {
        // what the C computes: int arithmetic, then sign-extended
        d.wrapping_mul(86400).wrapping_add(h.wrapping_mul(3600)) as i64
    };
    let mut cases: Vec<(i32, i32)> = Vec::new();
    // row 14: days*86400 overflows
    for d in [
        24855i32, 24856, 25000, 100_000, 1_000_000, -24855, -24856, -100_000, INT_MIN,
        i32::MAX,
    ] {
        cases.push((d, 0));
        cases.push((d, 23));
        cases.push((d, -23));
    }
    // row 15: hours*3600 overflows
    for h in [
        596523i32, 596524, 1_000_000, -596523, -596524, -1_000_000, INT_MIN, i32::MAX,
    ] {
        cases.push((0, h));
        cases.push((1, h));
        cases.push((-1, h));
    }
    // row 16: both products in range, their sum overflows
    for pair in [
        (24000i32, 500_000i32),
        (-24000, -500_000),
        (24855, 596523),
        (-24855, -596523),
        (20000, 500_000),
        (-20000, -500_000),
    ] {
        cases.push(pair);
    }
    let mut rng = Rng::with_seed(16);
    for _ in 0..2000 {
        cases.push((rng.next_i32(), rng.next_i32()));
    }

    for (d, h) in cases {
        // the clock term is identical for both, so compare directly
        let cv = unsafe { (c.get_modified_time)(d, h) };
        let rv = unsafe { (r.get_modified_time)(d, h) };
        assert_eq!(rv, cv, "get_modified_time({d},{h}) diverged");
        let clock = common::coarse_now();
        assert!(
            cv == clock.wrapping_add(base(d, h)) || cv == (clock + 1).wrapping_add(base(d, h)),
            "C offset arithmetic is not int-wrapping for ({d},{h}): got {cv}, \
             expected {} (clock {clock})",
            clock.wrapping_add(base(d, h))
        );
    }
}

// ===========================================================================
// rows 17..18 — hash_time_value: shift and multiply overflow, masked result
// ===========================================================================

#[test]
fn err_row17_18_hash_time_value_overflow() {
    let (c, r) = both();
    let mut cases: Vec<i64> = vec![
        0,
        -1,
        i64::MIN,
        i64::MAX,
        0x8080_8080_8080_8080u64 as i64,
        0xFF00_0000_0000_0000u64 as i64,
        0x0000_0000_FF00_0000u64 as i64, // byte 3 (i%4==3) >= 0x80 -> <<24 overflow
        0x0000_0000_8000_0000u64 as i64,
        0x0000_00FF_0000_0000u64 as i64,
    ];
    // one high-bit byte at every position
    for i in 0..8 {
        for v in [0x80u8, 0xFF] {
            let mut b = [0u8; 8];
            b[i] = v;
            cases.push(i64::from_ne_bytes(b));
        }
    }
    let mut rng = Rng::with_seed(17);
    for _ in 0..3000 {
        cases.push(rng.next_i64());
    }
    for t in cases {
        let cv = unsafe { (c.hash_time_value)(t) };
        let rv = unsafe { (r.hash_time_value)(t) };
        assert_eq!(rv, cv, "hash_time_value({t}) diverged");
        assert_eq!(cv & !0x7FFF_FFFFi32, 0, "C result not masked for {t}");
        assert!(cv >= 0, "C result negative for {t}");
    }
}

// ===========================================================================
// row 19 — modeselect: mode_selector % 4 < 0 reads before `modes[]`
// ===========================================================================

#[test]
#[ignore = "child process only; crashes on purpose"]
fn child_modeselect() {
    if !common::is_child() {
        return;
    }
    let which = std::env::var("DIFFTEST_LIB").unwrap_or_default();
    let api = if which == "c" {
        common::c_api()
    } else {
        common::rust_api()
    };
    let args: Vec<i32> = std::env::var("DIFFTEST_ARGS")
        .unwrap()
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    let v = unsafe { (api.modeselect)(args[0], args[1], args[2], args[3]) };
    println!("modeselect returned {v}");
    std::process::exit(70);
}

fn crash_pair(m: i32, t: i32, cx: i32, s: i32) {
    let args = format!("{m},{t},{cx},{s}");
    let c = run_child(
        "child_modeselect",
        &[("DIFFTEST_LIB", "c".into()), ("DIFFTEST_ARGS", args.clone())],
    );
    let r = run_child(
        "child_modeselect",
        &[("DIFFTEST_LIB", "rust".into()), ("DIFFTEST_ARGS", args.clone())],
    );
    assert_eq!(
        c, r,
        "modeselect({args}): C gave (signal,code)={c:?} but Rust gave {r:?}"
    );
    assert_eq!(
        c.0,
        Some(11),
        "expected SIGSEGV for modeselect({args}) (out-of-bounds modes[] read), got {c:?}"
    );
}

#[test]
fn err_row19_modeselect_negative_index_segv() {
    // mode_selector % 4 in {-1,-2,-3}
    for m in [-1i32, -2, -3, -5, -6, -7, -9, -10, -11, INT_MIN + 1, INT_MIN + 2, INT_MIN + 3] {
        assert!(m % 4 < 0, "test setup: {m} % 4 must be negative");
        crash_pair(m, 7, 3, 5);
    }
}

#[test]
fn err_row19b_modeselect_negative_index_multiple_of_four_is_safe() {
    // The complementary case: mode_selector % 4 == 0 must NOT fault, even for
    // negative selectors, and both libraries must agree.
    for m in [-4i32, -8, -100, INT_MIN] {
        assert_eq!(m % 4, 0);
        assert_same_io(&format!("modeselect({m},7,3,5)"), |a: &Api| unsafe {
            (a.modeselect)(m, 7, 3, 5)
        });
    }
}

// ===========================================================================
// row 20 — modeselect: complexity % 5 < 0 -> apply_multiplier default (0xDEAD)
// ===========================================================================

/// Run `modeselect` through BOTH libraries, assert the return value and the
/// stdout bytes agree, and hand back the agreed pair for further inspection.
fn agreed_modeselect(m: i32, t: i32, cx: i32, s: i32) -> (i32, String) {
    let (v, out) = common::same_io(
        &format!("modeselect({m},{t},{cx},{s})"),
        |a: &Api| unsafe { (a.modeselect)(m, t, cx, s) },
    );
    (v, String::from_utf8_lossy(&out).into_owned())
}

#[test]
fn err_row20_modeselect_negative_complexity() {
    let mut rng = Rng::with_seed(20);
    for cx in [-1i32, -2, -3, -4, -5, -9, -100, INT_MIN, INT_MIN + 1] {
        for m in 0..4i32 {
            let t = rng.next_i32();
            let s = rng.next_i32();
            let (_cv, cout) = agreed_modeselect(m, t, cx, s);
            let lvl = cx % 5;
            if lvl != 0 {
                assert!(
                    cout.contains(&format!("Complexity level: {lvl}, Multiplier: 0xDEAD")),
                    "C did not take the default: {cout}"
                );
            }
        }
    }
}

// ===========================================================================
// row 21 — modeselect: seed != 0 always overflows convert_time_factor
// ===========================================================================

#[test]
fn err_row21_modeselect_seed_nonzero_result1() {
    let mut rng = Rng::with_seed(21);
    let mut seeds: Vec<i32> = vec![1, -1, 2, -2, 24, -24, i32::MAX, INT_MIN];
    for _ in 0..40 {
        let s = rng.next_i32();
        if s != 0 {
            seeds.push(s);
        }
    }
    for s in seeds {
        let (_cv, cout) = agreed_modeselect(0, 0, 0, s);
        assert!(
            cout.contains("Result 1: -2147483648 (0x80000000)"),
            "C did not saturate result1 for seed {s}: {cout}"
        );
    }
    // and seed == 0 is the only in-range case
    let (_cv, cout) = agreed_modeselect(0, 0, 0, 0);
    assert!(
        cout.contains("Result 1: 0 (0x0)"),
        "C result1 for seed 0: {cout}"
    );
}

// ===========================================================================
// row 22 — modeselect: time_offset != 0 always overflows the second converter
// ===========================================================================

#[test]
fn err_row22_modeselect_time_offset_nonzero_result2() {
    let mut rng = Rng::with_seed(22);
    let mut ts: Vec<i32> = vec![1, -1, 2, -2, 100, -100, i32::MAX, INT_MIN];
    for _ in 0..40 {
        let t = rng.next_i32();
        if t != 0 {
            ts.push(t);
        }
    }
    for t in ts {
        let (_cv, cout) = agreed_modeselect(0, t, 0, 0);
        assert!(
            cout.contains("Result 2: -2147483648 (0x80000000)"),
            "C did not saturate result2 for time_offset {t}: {cout}"
        );
    }
    let (_cv, cout) = agreed_modeselect(0, 0, 0, 0);
    assert!(
        cout.contains("Result 2: 0 (0x0)"),
        "C result2 for time_offset 0: {cout}"
    );
}

// ===========================================================================
// row 23 — modeselect: the final `result * 0x10 + 0xBEEF`
// ===========================================================================

#[test]
fn err_row23_modeselect_final_overflow() {
    // The reachable range of `result` before the final scaling is small
    // (mode_value <= 0x40, multiplier <= 0xDEAD, time_hash % 0x1000 < 0x1000,
    // then xored with 0xFF / 0xFF00), so `result * 0x10 + 0xBEEF` cannot
    // actually overflow. Assert that both libraries agree on the exact final
    // value and that it is consistent with the printed intermediate values.
    let mut rng = Rng::with_seed(23);
    for _ in 0..200 {
        let m = (rng.next_u32() & 0x7FFF_FFFF) as i32;
        let t = rng.next_i32();
        let cx = rng.next_i32();
        let s = rng.next_i32();
        let (cv, cout) = agreed_modeselect(m, t, cx, s);
        // the printed final value must match the return value
        assert!(
            cout.contains(&format!("Final result: {cv} (0x{:X})", cv)),
            "printed final value != return value: {cout}"
        );
        assert_eq!((cv - 0xBEEF).rem_euclid(0x10), 0, "final scaling broken: {cv}");
    }
}

// ===========================================================================
// row 24 — modeselect with INT_MIN / INT_MAX arguments
// ===========================================================================

#[test]
fn err_row24_modeselect_int_min_args() {
    assert_eq!(INT_MIN % 4, 0, "INT_MIN % 4 must be 0 (no OOB read)");
    assert_eq!(INT_MIN % 5, -3);
    assert_eq!(INT_MIN % 24, -8);
    let vals = [INT_MIN, INT_MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for t in vals {
        for cx in vals {
            for s in vals {
                assert_same_io(
                    &format!("modeselect(INT_MIN,{t},{cx},{s})"),
                    |a: &Api| unsafe { (a.modeselect)(INT_MIN, t, cx, s) },
                );
            }
        }
    }
}

// ===========================================================================
// Generic FFI boundary sweeps (beyond the table)
// ===========================================================================

#[test]
fn generic_out_of_range_discriminants() {
    // C `enum`-like int discriminants: every value one step past the valid
    // window of `apply_multiplier`'s `level`, and of `modeselect`'s mode index.
    for lvl in [-1i32, 5, i32::MIN, i32::MAX] {
        assert_same(&format!("apply_multiplier(0xA0,{lvl})"), |a: &Api| unsafe {
            (a.apply_multiplier)(0xA0, lvl)
        });
    }
    // mode index one step past the top of the table wraps back to 0 (`% 4`)
    for m in [4i32, 5, 6, 7, 8, i32::MAX, i32::MAX - 1, i32::MAX - 2, i32::MAX - 3] {
        assert_same_io(&format!("modeselect({m},0,0,0)"), |a: &Api| unsafe {
            (a.modeselect)(m, 0, 0, 0)
        });
    }
}

#[test]
fn generic_zero_and_extreme_scalars() {
    let xs = [0i32, 1, -1, i32::MAX, i32::MIN];
    for a0 in xs {
        for b0 in xs {
            assert_same(&format!("apply_multiplier({a0},{b0})"), |a: &Api| unsafe {
                (a.apply_multiplier)(a0, b0)
            });
            assert_same(&format!("get_modified_time({a0},{b0})"), |a: &Api| unsafe {
                (a.get_modified_time)(a0, b0)
            });
        }
        assert_same(&format!("hash_time_value({a0})"), |a: &Api| unsafe {
            (a.hash_time_value)(a0 as i64)
        });
        assert_same(&format!("convert_time_factor({a0})"), |a: &Api| unsafe {
            (a.convert_time_factor)(a0 as f64)
        });
        assert_same(
            &format!("convert_negative_overflow({a0})"),
            |a: &Api| unsafe { (a.convert_negative_overflow)(a0 as f64) },
        );
    }
    for t in [i64::MIN, i64::MAX, 0, -1, 1] {
        assert_same(&format!("hash_time_value({t})"), |a: &Api| unsafe {
            (a.hash_time_value)(t)
        });
    }
}

#[test]
fn generic_long_and_empty_strings() {
    let (c, r) = both();
    for len in [0usize, 1, 2, 7, 8, 9, 255, 256, 4095, 65536] {
        let v = vec![b'q'; len];
        let cs = CString::new(v).unwrap();
        let cv = unsafe { (c.classify_mode)(cs.as_ptr()) };
        let rv = unsafe { (r.classify_mode)(cs.as_ptr()) };
        assert_eq!(rv, cv, "classify_mode(len {len}) diverged");
        assert_eq!(cv, 0);
    }
}

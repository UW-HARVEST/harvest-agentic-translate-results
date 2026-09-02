// Phase C -- error/rejection-path differential tests, one per row of ERRORS.md.
// Each test asserts C and Rust return the SAME sentinel/saturated value, and
// additionally pins that value to what the C source says it must be (so "both
// failed somehow" cannot pass).

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const IMAX: i32 = i32::MAX; // 2147483647
const IMIN: i32 = i32::MIN; // -2147483648
const IMAX_F: f64 = 2_147_483_647.0;
const IMIN_F: f64 = -2_147_483_648.0;

fn both_sdti(p: &Pair, d: f64) -> i32 {
    let (cv, rv) = unsafe { ((p.c.safe_double_to_int)(d), (p.r.safe_double_to_int)(d)) };
    assert_eq!(
        cv, rv,
        "safe_double_to_int({}) -> C={cv} RUST={rv}",
        fmt_f64(d)
    );
    cv
}

fn both_pwf(p: &Pair, code: i32, base: i32) -> i32 {
    let (cv, rv) = unsafe {
        (
            (p.c.process_with_fallthrough)(code, base),
            (p.r.process_with_fallthrough)(code, base),
        )
    };
    assert_eq!(
        cv, rv,
        "process_with_fallthrough({code}, {base}) -> C={cv} RUST={rv}"
    );
    cv
}

fn next_up(x: f64) -> f64 {
    if x > 0.0 {
        f64::from_bits(x.to_bits() + 1)
    } else if x < 0.0 {
        f64::from_bits(x.to_bits() - 1)
    } else {
        f64::from_bits(1)
    }
}
fn next_down(x: f64) -> f64 {
    -next_up(-x)
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 -- d > (double)INT_MAX  =>  INT_MAX
// ---------------------------------------------------------------------------
#[test]
fn err01_sdti_above_int_max_saturates() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 101);
    let mut vals = vec![
        2_147_483_648.0f64,
        2_147_483_647.5,
        1e15,
        1e300,
        f64::MAX,
        f64::INFINITY,
        IMAX_F + 1.0,
    ];
    for _ in 0..500 {
        vals.push(IMAX_F + 1.0 + rng.next_f64_unit() * 1e12);
    }
    for d in vals {
        assert!(d > IMAX_F, "precondition: {}", fmt_f64(d));
        let got = both_sdti(&p, d);
        assert_eq!(got, IMAX, "row1: expected INT_MAX for {}", fmt_f64(d));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 -- d < (double)INT_MIN  =>  INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn err02_sdti_below_int_min_saturates() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 102);
    let mut vals = vec![
        -2_147_483_649.0f64,
        -2_147_483_648.5,
        -1e15,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
        IMIN_F - 1.0,
    ];
    for _ in 0..500 {
        vals.push(IMIN_F - 1.0 - rng.next_f64_unit() * 1e12);
    }
    for d in vals {
        assert!(d < IMIN_F, "precondition: {}", fmt_f64(d));
        let got = both_sdti(&p, d);
        assert_eq!(got, IMIN, "row2: expected INT_MIN for {}", fmt_f64(d));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 -- isnan(d) => 0  (the NaN test is AFTER both range compares)
// ---------------------------------------------------------------------------
#[test]
fn err03_sdti_nan_returns_zero() {
    let p = load_pair();
    let mut nans = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0000),
        f64::from_bits(0xfff8_0000_0000_0000),
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling
        f64::from_bits(0xfff0_0000_0000_0001),
        f64::from_bits(0x7fff_ffff_ffff_ffff),
        f64::from_bits(0xffff_ffff_ffff_ffff),
    ];
    let mut rng = Rng::new(SEED ^ 103);
    for _ in 0..500 {
        // random NaN payload, random sign
        let payload = (rng.next_u64() & 0x000f_ffff_ffff_ffff) | 1;
        let sign = (rng.next_u64() & 1) << 63;
        nans.push(f64::from_bits(sign | 0x7ff0_0000_0000_0000 | payload));
    }
    for d in nans {
        assert!(d.is_nan(), "precondition: {}", fmt_f64(d));
        let got = both_sdti(&p, d);
        assert_eq!(got, 0, "row3: expected 0 for NaN {}", fmt_f64(d));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 4 & 5 -- exactly ON the boundary: the guard is NOT taken,
// the value comes from `(int)d`.
// ---------------------------------------------------------------------------
#[test]
fn err04_sdti_exactly_int_max_uses_cast_not_guard() {
    let p = load_pair();
    assert!(!(IMAX_F > IMAX_F));
    assert_eq!(both_sdti(&p, IMAX_F), IMAX, "row4");
    assert_eq!(both_sdti(&p, next_down(IMAX_F)), 2_147_483_646, "row4");
}

#[test]
fn err05_sdti_exactly_int_min_uses_cast_not_guard() {
    let p = load_pair();
    assert!(!(IMIN_F < IMIN_F));
    assert_eq!(both_sdti(&p, IMIN_F), IMIN, "row5");
    assert_eq!(both_sdti(&p, next_up(IMIN_F)), -2_147_483_647, "row5");
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 6 & 7 -- one ulp PAST each guard: guard IS taken.
// ---------------------------------------------------------------------------
#[test]
fn err06_sdti_one_ulp_past_high_guard() {
    let p = load_pair();
    for k in 1..64u64 {
        let d = f64::from_bits(IMAX_F.to_bits() + k);
        assert!(d > IMAX_F);
        assert_eq!(both_sdti(&p, d), IMAX, "row6: {}", fmt_f64(d));
    }
}

#[test]
fn err07_sdti_one_ulp_past_low_guard() {
    let p = load_pair();
    for k in 1..64u64 {
        let d = f64::from_bits(IMIN_F.to_bits() + k); // more negative
        assert!(d < IMIN_F, "{}", fmt_f64(d));
        assert_eq!(both_sdti(&p, d), IMIN, "row7: {}", fmt_f64(d));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 8 -- `default:` sentinel -1 for any code outside 0..=5
// ---------------------------------------------------------------------------
#[test]
fn err08_pwf_default_returns_minus_one() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 108);
    let mut codes: Vec<i32> = vec![-1, -2, -5, -6, 6, 7, 8, 100, -100];
    for _ in 0..1000 {
        let c = rng.next_i32();
        if !(0..=5).contains(&c) {
            codes.push(c);
        }
    }
    for code in codes {
        for base in [0, 1, -1, IMAX, IMIN, 12345] {
            let got = both_pwf(&p, code, base);
            assert_eq!(got, -1, "row8: code={code} base={base}");
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 9 -- out-of-range "enum-like" ints across the FFI boundary.
// C enums/switch selectors accept any int; every non-variant value must land on
// `default:` identically in both implementations.
// ---------------------------------------------------------------------------
#[test]
fn err09_pwf_out_of_range_enum_values() {
    let p = load_pair();
    let hostile: [i32; 16] = [
        IMAX,
        IMIN,
        IMAX - 1,
        IMIN + 1,
        0x7fff_ffff,
        -0x8000_0000i64 as i32,
        i32::from_le_bytes([0xff, 0xff, 0xff, 0xff]), // -1
        6,
        -6,
        255,
        256,
        65_536,
        -65_536,
        1 << 30,
        -(1 << 30),
        0x0000_0006,
    ];
    for &code in &hostile {
        for base in [0, 7, -7, IMAX, IMIN] {
            let got = both_pwf(&p, code, base);
            assert_eq!(got, -1, "row9: code={code} base={base}");
        }
    }
    // exhaustively confirm the ONLY accepted selectors are 0..=5
    for code in -64..=64 {
        let got = both_pwf(&p, code, 1000);
        if (0..=5).contains(&code) {
            assert_ne!(
                (code, got),
                (code, -1),
                "row9: code={code} must not hit default"
            );
        } else {
            assert_eq!(got, -1, "row9: code={code} must hit default");
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 10 -- `case 0:` discards base_value entirely
// ---------------------------------------------------------------------------
#[test]
fn err10_pwf_case_zero_discards_base() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 110);
    for base in [0, 1, -1, IMAX, IMIN, IMAX - 1, IMIN + 1] {
        assert_eq!(both_pwf(&p, 0, base), 0, "row10: base={base}");
    }
    for _ in 0..2000 {
        let base = rng.next_i32();
        assert_eq!(both_pwf(&p, 0, base), 0, "row10: base={base}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 11 -- unchecked `result += N` near the int limits: no range
// check exists, so both must wrap identically.
// ---------------------------------------------------------------------------
#[test]
fn err11_pwf_unchecked_add_wraps_identically() {
    let p = load_pair();
    let increments: [i32; 6] = [0, 10, 30, 60, 100, 150]; // code 0..5 totals
    for code in 1..=5i32 {
        for delta in 0..200i64 {
            for base in [
                (IMAX as i64 - delta) as i32,
                (IMIN as i64 + delta) as i32,
            ] {
                let got = both_pwf(&p, code, base);
                let expect = base.wrapping_add(increments[code as usize]);
                assert_eq!(
                    got, expect,
                    "row11: code={code} base={base} expected wrapped {expect}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 12 -- copy_data_block has NO null check (0 occurrences of NULL
// in the C source). Verified in a forked child so the crash is observed rather
// than assumed: both libraries must die with the SAME signal.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Returns the raw wait status of a child that called `f`.
fn run_in_child<F: FnOnce()>(f: F) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        status
    }
}

fn termsig(status: c_int) -> c_int {
    status & 0x7f
}
fn exited_ok(status: c_int) -> bool {
    termsig(status) == 0 && ((status >> 8) & 0xff) == 0
}

#[test]
fn err12_copy_data_block_has_no_null_check() {
    let p = load_pair();

    // Positive control: valid pointers -> both children exit 0.
    let ok_c = run_in_child(|| {
        let src = Probe::filled(0x42);
        let mut dst = Probe::filled(0);
        unsafe { (p.c.copy_data_block)(dst.as_mut_ptr(), src.as_ptr()) };
    });
    let ok_r = run_in_child(|| {
        let src = Probe::filled(0x42);
        let mut dst = Probe::filled(0);
        unsafe { (p.r.copy_data_block)(dst.as_mut_ptr(), src.as_ptr()) };
    });
    assert!(exited_ok(ok_c) && exited_ok(ok_r), "positive control failed");

    // NULL dest: neither implementation validates, so both must die the same way.
    let null_dest_c = run_in_child(|| {
        let src = Probe::filled(0x42);
        unsafe { (p.c.copy_data_block)(std::ptr::null_mut::<c_void>(), src.as_ptr()) };
    });
    let null_dest_r = run_in_child(|| {
        let src = Probe::filled(0x42);
        unsafe { (p.r.copy_data_block)(std::ptr::null_mut::<c_void>(), src.as_ptr()) };
    });
    assert_eq!(
        termsig(null_dest_c),
        termsig(null_dest_r),
        "row12: NULL dest -> C signal {} vs RUST signal {}",
        termsig(null_dest_c),
        termsig(null_dest_r)
    );
    assert_ne!(
        termsig(null_dest_c),
        0,
        "row12: C is expected to fault on NULL dest (no check exists)"
    );

    // NULL src.
    let null_src_c = run_in_child(|| {
        let mut dst = Probe::filled(0);
        unsafe { (p.c.copy_data_block)(dst.as_mut_ptr(), std::ptr::null::<c_void>()) };
    });
    let null_src_r = run_in_child(|| {
        let mut dst = Probe::filled(0);
        unsafe { (p.r.copy_data_block)(dst.as_mut_ptr(), std::ptr::null::<c_void>()) };
    });
    assert_eq!(
        termsig(null_src_c),
        termsig(null_src_r),
        "row12: NULL src -> C signal {} vs RUST signal {}",
        termsig(null_src_c),
        termsig(null_src_r)
    );
    assert_ne!(termsig(null_src_c), 0, "row12: C is expected to fault on NULL src");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 13 -- overunder: d*d + a*a overflows negative => sqrt yields
// NaN => conv4 == 0 (via row 3). overunder itself still returns a value.
// ---------------------------------------------------------------------------
#[test]
fn err13_overunder_negative_sqrt_operand_yields_zero_conv4() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 113);
    let mut cases: Vec<(i32, i32)> = vec![(0, 46341), (46341, 0), (5, 46349), (46341, 1)];
    let mut tries = 0;
    while cases.len() < 400 && tries < 200_000 {
        tries += 1;
        let a = rng.next_i32();
        let d = rng.next_i32();
        let w = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
        if w < 0 {
            cases.push((a, d));
        }
    }
    for &(a, d) in &cases {
        let w = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
        assert!(w < 0, "precondition a={a} d={d} -> {w}");
        let s = (w as f64).sqrt();
        assert!(s.is_nan(), "sqrt({w}) should be NaN");
        // the NaN path in both libraries returns 0
        assert_eq!(both_sdti(&p, s), 0, "row13: NaN -> 0");
        // and overunder itself agrees end to end
        let (cv, rv) = unsafe { ((p.c.overunder)(a, 3, -5, d), (p.r.overunder)(a, 3, -5, d)) };
        assert_eq!(cv, rv, "row13: overunder({a},3,-5,{d}) C={cv} RUST={rv}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 14 -- negative `a` => negative `a % 6` => default: => -1
// ---------------------------------------------------------------------------
#[test]
fn err14_overunder_negative_a_hits_default_arm() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 114);
    for _ in 0..1000 {
        let a = -(rng.range_i32(1, 2_000_000_000));
        let b = rng.next_i32();
        let r = a % 6;
        assert!(r <= 0, "C % truncates toward zero: {a} % 6 = {r}");
        if r != 0 {
            // must be the default arm in both
            assert_eq!(both_pwf(&p, r, b), -1, "row14: a={a} r={r}");
        } else {
            assert_eq!(both_pwf(&p, r, b), 0, "row14: a={a} r=0 -> case 0");
        }
        let (cv, rv) = unsafe {
            (
                (p.c.overunder)(a, b, 17, 4),
                (p.r.overunder)(a, b, 17, 4),
            )
        };
        assert_eq!(cv, rv, "row14: overunder({a},{b},17,4) C={cv} RUST={rv}");
    }
    // INT_MIN % 6 is well defined (-2) and must reach default:
    assert_eq!(IMIN % 6, -2);
    assert_eq!(both_pwf(&p, IMIN % 6, 999), -1, "row14: INT_MIN residue");
}

// ---------------------------------------------------------------------------
// Generic FFI boundaries not enumerable as ERRORS.md rows: there are no
// length/size/count parameters anywhere in the API (copy_data_block's length is
// the compile-time `sizeof(DataBlock)`), so "zero and oversized lengths" is
// covered by proving both implementations copy exactly sizeof(DataBlock) bytes
// regardless of the surrounding buffer, and by hammering the integer extremes
// of every scalar parameter.
// ---------------------------------------------------------------------------
#[test]
fn err15_generic_scalar_extremes_every_entry_point() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let extremes: [i32; 9] = [IMIN, IMIN + 1, -1, 0, 1, 5, 6, IMAX - 1, IMAX];
    for &v in &extremes {
        let (cv, rv) = unsafe {
            (
                (p.c.handle_pointer_operations)(v),
                (p.r.handle_pointer_operations)(v),
            )
        };
        assert_eq!(cv, rv, "handle_pointer_operations({v})");
        both_sdti(&p, v as f64);
        for &w in &extremes {
            both_pwf(&p, v, w);
            let (cv, rv) = unsafe { ((p.c.overunder)(v, w, w, v), (p.r.overunder)(v, w, w, v)) };
            assert_eq!(cv, rv, "overunder({v},{w},{w},{v}) C={cv} RUST={rv}");
        }
    }
}

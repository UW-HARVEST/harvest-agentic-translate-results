// Phase C — error / rejection-path differential tests.
//
// One test (or one clearly-labelled block) per row of ERRORS.md. Each asserts
// the SAME sentinel value on both sides, not merely "both failed somehow".
mod common;

use common::*;
use std::ffi::c_char;

/// The x86-64 `cvttsd2si` "integer indefinite" result: what C's `(int)double`
/// yields for every out-of-range / NaN / inf conversion.
const INDEFINITE: i32 = i32::MIN;

/// The `apply_multiplier` `default:` sentinel at lib.c:57.
const DEAD: i32 = 0xDEAD;

fn cm(row: &str, bytes: &[u8]) -> i32 {
    let p = pair();
    let buf = cstr(bytes);
    let ptr = buf.as_ptr() as *const c_char;
    // SAFETY: `ptr` is a NUL-terminated buffer alive across both calls.
    let (c, r) = unsafe { ((p.c.classify_mode)(ptr), (p.rs.classify_mode)(ptr)) };
    eq_int(row, format!("classify_mode({:?})", String::from_utf8_lossy(bytes)), c, r);
    c
}

fn am(row: &str, base: i32, level: i32) -> i32 {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe {
        (
            (p.c.apply_multiplier)(base, level),
            (p.rs.apply_multiplier)(base, level),
        )
    };
    eq_int(row, format!("apply_multiplier({base}, {level})"), c, r);
    c
}

fn ctf(row: &str, x: f64) -> i32 {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe { ((p.c.convert_time_factor)(x), (p.rs.convert_time_factor)(x)) };
    eq_int(row, format!("convert_time_factor({x:?})"), c, r);
    c
}

fn cno(row: &str, x: f64) -> i32 {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe {
        (
            (p.c.convert_negative_overflow)(x),
            (p.rs.convert_negative_overflow)(x),
        )
    };
    eq_int(row, format!("convert_negative_overflow({x:?})"), c, r);
    c
}

fn gmt(row: &str, d: i32, h: i32) -> TimeT {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe { ((p.c.get_modified_time)(d, h), (p.rs.get_modified_time)(d, h)) };
    eq_time(row, format!("get_modified_time({d}, {h})"), c, r);
    c
}

fn htv(row: &str, t: TimeT) -> i32 {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe { ((p.c.hash_time_value)(t), (p.rs.hash_time_value)(t)) };
    eq_int(row, format!("hash_time_value({t})"), c, r);
    c
}

// ===========================================================================
// E1..E7 — classify_mode rejections
// ===========================================================================

#[test]
fn e1_classify_mode_unmatched_strings() {
    for s in [
        &b"unknown"[..],
        b"fast",
        b"STANDARD_MODE",
        b"mode",
        b"0",
        b"standard enhanced",
        b"\t",
        b" standard",
    ] {
        let v = cm("E1", s);
        assert_eq!(v, 0x00, "E1: expected 0x00 for {:?}", String::from_utf8_lossy(s));
    }
}

#[test]
fn e2_classify_mode_empty_string() {
    assert_eq!(cm("E2", b""), 0x00);
}

#[test]
fn e3_classify_mode_strict_prefixes() {
    for lit in [&b"standard"[..], b"enhanced", b"turbo", b"extreme"] {
        for keep in 0..lit.len() {
            let v = cm("E3", &lit[..keep]);
            assert_eq!(
                v, 0x00,
                "E3: prefix {:?} must be rejected",
                String::from_utf8_lossy(&lit[..keep])
            );
        }
    }
}

#[test]
fn e4_classify_mode_valid_plus_trailing_bytes() {
    for lit in [&b"standard"[..], b"enhanced", b"turbo", b"extreme"] {
        for tail in [&b"X"[..], b" ", b"\t", b"\n", b"0", b"\xff", b"aaaaaaaa"] {
            let mut s = lit.to_vec();
            s.extend_from_slice(tail);
            let v = cm("E4", &s);
            assert_eq!(v, 0x00, "E4: {:?} must be rejected", String::from_utf8_lossy(&s));
        }
    }
}

#[test]
fn e5_classify_mode_case_variants() {
    for s in [
        &b"Standard"[..],
        b"STANDARD",
        b"Enhanced",
        b"ENHANCED",
        b"Turbo",
        b"TURBO",
        b"Extreme",
        b"EXTREME",
        b"sTaNdArD",
    ] {
        let v = cm("E5", s);
        assert_eq!(v, 0x00, "E5: expected 0x00 for {:?}", String::from_utf8_lossy(s));
    }
}

#[test]
fn e6_classify_mode_embedded_nul_truncates() {
    // `strcmp` stops at the first NUL, so these are ACCEPTED, not rejected.
    // Build the buffer by hand since `cstr` would append a second terminator.
    let p = pair();
    for (payload, want) in [
        (&b"standard\0zzz"[..], 0x10),
        (&b"enhanced\0junk"[..], 0x20),
        (&b"turbo\0zzz"[..], 0x30),
        (&b"extreme\0\xff\xff"[..], 0x40),
        (&b"\0standard"[..], 0x00),
    ] {
        let mut buf: Vec<c_char> = payload.iter().map(|&b| b as c_char).collect();
        buf.push(0);
        let ptr = buf.as_ptr() as *const c_char;
        // SAFETY: buffer contains a NUL and stays alive across both calls.
        let (c, r) = unsafe { ((p.c.classify_mode)(ptr), (p.rs.classify_mode)(ptr)) };
        eq_int("E6", format!("classify_mode({payload:?})"), c, r);
        assert_eq!(c, want, "E6: C ground truth for {payload:?}");
    }
}

#[test]
fn e7_classify_mode_high_bit_bytes() {
    for s in [
        &b"\xff\xfe"[..],
        b"\x80",
        b"\xff",
        b"standar\xff",
        b"\xc3\xa9",
        b"turb\xf6",
    ] {
        let v = cm("E7", s);
        assert_eq!(v, 0x00, "E7: expected 0x00 for {s:?}");
    }
}

// ===========================================================================
// E9..E12 — apply_multiplier rejections
// ===========================================================================

#[test]
fn e9_apply_multiplier_negative_level() {
    for level in [-1i32, -2, -3, -4, -5, -100, -1000, i32::MIN + 1, i32::MIN] {
        for base in [0i32, 1, -1, 0xA0, i32::MAX, i32::MIN] {
            let v = am("E9", base, level);
            assert_eq!(v, DEAD, "E9: level={level} base={base} must yield 0xDEAD");
        }
    }
}

#[test]
fn e10_apply_multiplier_level_above_four() {
    for level in [5i32, 6, 7, 8, 100, 1000, 0x10000, i32::MAX - 1, i32::MAX] {
        for base in [0i32, 1, -1, 0xA0, i32::MAX, i32::MIN] {
            let v = am("E10", base, level);
            assert_eq!(v, DEAD, "E10: level={level} base={base} must yield 0xDEAD");
        }
    }
}

#[test]
fn e11_apply_multiplier_extreme_out_of_range_enum_values() {
    // C enums accept any int across FFI; these have no valid `case` variant.
    for level in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -0x8000_0000i64 as i32] {
        let v = am("E11", 0xA0, level);
        assert_eq!(v, DEAD, "E11: level={level} must yield 0xDEAD");
    }
    // A dense randomized sweep of out-of-range values, still all 0xDEAD.
    let mut rng = Rng::with_seed(SEED ^ 0xE11);
    for _ in 0..4000 {
        let level = rng.next_i32();
        if (0..=4).contains(&level) {
            continue;
        }
        let v = am("E11", rng.next_i32(), level);
        assert_eq!(v, DEAD, "E11: level={level} must yield 0xDEAD");
    }
}

#[test]
fn e12_apply_multiplier_accumulator_overflow() {
    // In-range level, but base chosen so the accumulated += overflows int.
    // Total added for level 4 is 0xFF+0xAB+0x7E+0x1C+0x05 = 585 = 0x249.
    let totals = [0x05i32, 0x21, 0x9F, 0x14A, 0x249]; // levels 0..4
    for level in 0..=4i32 {
        let total = totals[level as usize];
        for base in [
            i32::MAX,
            i32::MAX - 1,
            i32::MAX - total + 1,
            i32::MAX - total,
            i32::MAX - total - 1,
            i32::MIN,
            i32::MIN + 1,
        ] {
            let v = am("E12", base, level);
            assert_eq!(
                v,
                base.wrapping_add(total),
                "E12: C must wrap two's-complement for base={base} level={level}"
            );
        }
    }
}

// ===========================================================================
// E13..E18 — convert_time_factor rejections
// ===========================================================================

#[test]
fn e13_convert_time_factor_above_int_max() {
    for x in [1.0f64, 2.0, 1e3, 1e6, 0.01, 0.0021474836480001] {
        let v = ctf("E13", x);
        assert_eq!(v, INDEFINITE, "E13: x={x} must yield INT_MIN");
    }
}

#[test]
fn e14_convert_time_factor_below_int_min() {
    for x in [-1.0f64, -2.0, -1e3, -1e6, -0.01, -0.0021474836490001] {
        let v = ctf("E14", x);
        assert_eq!(v, INDEFINITE, "E14: x={x} must yield INT_MIN");
    }
}

#[test]
fn e15_convert_time_factor_nan() {
    for x in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001), // quiet NaN, alt payload
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF8_0000_0000_0001), // negative NaN
    ] {
        let v = ctf("E15", x);
        assert_eq!(v, INDEFINITE, "E15: NaN must yield INT_MIN");
    }
}

#[test]
fn e16_convert_time_factor_infinities() {
    for x in [f64::INFINITY, f64::NEG_INFINITY] {
        let v = ctf("E16", x);
        assert_eq!(v, INDEFINITE, "E16: {x} must yield INT_MIN");
    }
}

#[test]
fn e17_convert_time_factor_product_overflows_to_inf() {
    for x in [f64::MAX, f64::MIN, 1e300, -1e300, 1e308, -1e308] {
        let v = ctf("E17", x);
        assert_eq!(v, INDEFINITE, "E17: x={x} must yield INT_MIN");
    }
}

#[test]
fn e18_convert_time_factor_one_step_past_range() {
    // Accepted at exactly the boundary...
    assert_eq!(ctf("E18", 2147483647.0 / 1e12), ctf("E18", 2147483647.0 / 1e12));
    // ...and INT_MIN one step past it. Construct the product exactly.
    for (num, expect_indefinite) in [
        (2147483647.0f64, false),
        (2147483648.0, true),
        (2147483649.0, true),
        (-2147483648.0, false),
        (-2147483649.0, true),
        (-2147483650.0, true),
    ] {
        let x = num / 1e12;
        let v = ctf("E18", x);
        // The division is inexact, so only assert the sentinel where the
        // recomputed product genuinely lands out of range; equality of C and
        // Rust is already asserted by `ctf` for every case.
        let prod = x * 1e12;
        let out_of_range = !(prod.trunc() >= -2147483648.0 && prod.trunc() <= 2147483647.0)
            || prod.is_nan();
        if out_of_range {
            assert_eq!(v, INDEFINITE, "E18: {x} (prod {prod}) must yield INT_MIN");
        } else {
            assert_eq!(v, prod.trunc() as i32, "E18: {x} (prod {prod}) must truncate");
        }
        let _ = expect_indefinite;
    }
    // Dense integer-exact sweep either side of the boundary.
    for k in -64i64..=64 {
        ctf("E18", (2147483647i64 + k) as f64 / 1e12);
        ctf("E18", (-2147483648i64 + k) as f64 / 1e12);
    }
}

// ===========================================================================
// E19..E23 — convert_negative_overflow rejections
// ===========================================================================

#[test]
fn e19_convert_negative_overflow_out_of_range() {
    for x in [1.0f64, -1.0, 2.0, -2.0, 1e-3, -1e-3, 1e6, -1e6] {
        let v = cno("E19", x);
        assert_eq!(v, INDEFINITE, "E19: x={x} must yield INT_MIN");
    }
}

#[test]
fn e20_convert_negative_overflow_nan() {
    for x in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001),
        f64::from_bits(0xFFF8_0000_0000_0001),
    ] {
        let v = cno("E20", x);
        assert_eq!(v, INDEFINITE, "E20: NaN must yield INT_MIN");
    }
}

#[test]
fn e21_convert_negative_overflow_infinities_and_max() {
    for x in [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
        1e300,
        -1e300,
    ] {
        let v = cno("E21", x);
        assert_eq!(v, INDEFINITE, "E21: x={x} must yield INT_MIN");
    }
}

#[test]
fn e22_convert_negative_overflow_signed_zero() {
    // 0.0 * -1e15 == -0.0 and -0.0 * -1e15 == +0.0; both truncate to 0.
    assert_eq!(cno("E22", 0.0), 0);
    assert_eq!(cno("E22", -0.0), 0);
    // Subnormals also truncate to 0 after scaling? 5e-324 * -1e15 is tiny but
    // non-zero; assert only C/Rust agreement (done inside `cno`).
    cno("E22", 5e-324);
    cno("E22", -5e-324);
    cno("E22", f64::MIN_POSITIVE);
    cno("E22", -f64::MIN_POSITIVE);
}

#[test]
fn e23_convert_negative_overflow_one_step_past_range() {
    for num in [
        2147483647.0f64,
        2147483648.0,
        2147483649.0,
        -2147483648.0,
        -2147483649.0,
        -2147483650.0,
    ] {
        let x = num / -1e15;
        let v = cno("E23", x);
        let prod = x * -1e15;
        let out = prod.is_nan() || !(prod.trunc() >= -2147483648.0 && prod.trunc() <= 2147483647.0);
        if out {
            assert_eq!(v, INDEFINITE, "E23: {x} (prod {prod}) must yield INT_MIN");
        } else {
            assert_eq!(v, prod.trunc() as i32, "E23: {x} (prod {prod}) must truncate");
        }
    }
    for k in -64i64..=64 {
        cno("E23", (2147483647i64 + k) as f64 / -1e15);
        cno("E23", (-2147483648i64 + k) as f64 / -1e15);
    }
}

// ===========================================================================
// E24..E26 — get_modified_time int overflow
// ===========================================================================

/// The clock component both libraries observe: `time(NULL) >> 29`.
fn clock_component() -> TimeT {
    let p = pair();
    // SAFETY: plain scalar C ABI call; (0,0) adds no offset.
    unsafe { (p.c.get_modified_time)(0, 0) }
}

#[test]
fn e24_get_modified_time_days_product_overflows() {
    let base = clock_component();
    for d in [
        24855i32, 24856, 100000, 1_000_000, 1 << 20, i32::MAX, i32::MAX - 1, -24856, -100000,
        i32::MIN, i32::MIN + 1,
    ] {
        let got = gmt("E24", d, 0);
        let want = base.wrapping_add(d.wrapping_mul(86400) as TimeT);
        assert_eq!(got, want, "E24: d={d} must wrap in int then sign-extend");
    }
}

#[test]
fn e25_get_modified_time_hours_product_overflows() {
    let base = clock_component();
    for h in [
        596523i32, 596524, 1_000_000, i32::MAX, i32::MAX - 1, -596524, i32::MIN, i32::MIN + 1,
    ] {
        let got = gmt("E25", 0, h);
        let want = base.wrapping_add(h.wrapping_mul(3600) as TimeT);
        assert_eq!(got, want, "E25: h={h} must wrap in int then sign-extend");
    }
}

#[test]
fn e26_get_modified_time_sum_overflows() {
    let base = clock_component();
    let mut rng = Rng::with_seed(SEED ^ 0xE26);
    let mut cases: Vec<(i32, i32)> = vec![
        (24854, 596523),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
        (12427, 298261),
    ];
    for _ in 0..4000 {
        cases.push((rng.next_i32(), rng.next_i32()));
    }
    for (d, h) in cases {
        let got = gmt("E26", d, h);
        let want = base.wrapping_add(
            d.wrapping_mul(86400).wrapping_add(h.wrapping_mul(3600)) as TimeT
        );
        assert_eq!(got, want, "E26: ({d},{h}) must wrap in int then sign-extend");
    }
}

// ===========================================================================
// E27..E28 — hash_time_value overflow
// ===========================================================================

#[test]
fn e27_hash_time_value_high_bit_shift_overflow() {
    // Bytes >= 0x80 at position i%4 == 3 make `bytes[i] << 24` overflow int.
    let mut cases: Vec<i64> = Vec::new();
    for b in 0x80u64..=0xFF {
        cases.push(((b << 24) as i64) | ((b << 56) as i64));
        cases.push((b << 24) as i64);
    }
    cases.push(-1);
    cases.push(i64::MIN);
    for t in cases {
        let v = htv("E27", t);
        assert!(
            (0..=0x7FFF_FFFF).contains(&v),
            "E27: result must be masked non-negative, got {v}"
        );
    }
}

#[test]
fn e28_hash_time_value_multiply_overflow() {
    // hash *= 0x1F overflows on essentially every input; sweep broadly and
    // assert both agree and the mask holds.
    let mut rng = Rng::with_seed(SEED ^ 0xE28);
    for _ in 0..8000 {
        let t = rng.next_i64();
        let v = htv("E28", t);
        assert!(
            (0..=0x7FFF_FFFF).contains(&v),
            "E28: result must be masked non-negative, got {v} for t={t}"
        );
    }
}

// ===========================================================================
// E30..E33 — modeselect rejections that are NOT UB
// ===========================================================================

fn ms_ret(row: &str, a: i32, b: i32, c: i32, d: i32) -> i32 {
    let p = pair();
    // SAFETY: callers pass only non-UB `mode_selector` values (>= 0, or a
    // negative multiple of 4).
    let (rc, _) = capture_forked_i32(|| unsafe { (p.c.modeselect)(a, b, c, d) });
    let (rr, _) = capture_forked_i32(|| unsafe { (p.rs.modeselect)(a, b, c, d) });
    eq_int(row, format!("modeselect({a}, {b}, {c}, {d})"), rc, rr);
    rc
}

fn ms_out(row: &str, a: i32, b: i32, c: i32, d: i32) -> Vec<u8> {
    let p = pair();
    // SAFETY: as `ms_ret`.
    let (rc, oc) = capture_forked_i32(|| unsafe { (p.c.modeselect)(a, b, c, d) });
    let (rr, or) = capture_forked_i32(|| unsafe { (p.rs.modeselect)(a, b, c, d) });
    let ctx = format!("modeselect({a}, {b}, {c}, {d})");
    assert!(!oc.is_empty(), "[{row}] captured no C output for {ctx}");
    eq_bytes(row, &ctx, &oc, &or);
    eq_int(row, &ctx, rc, rr);
    oc
}

#[test]
fn e30_modeselect_negative_complexity_propagates_dead() {
    // Negative complexity NOT divisible by 5 => negative level => default arm.
    for c in [-1i32, -2, -3, -4, -6, -7, -101, i32::MIN, i32::MIN + 1] {
        let out = ms_out("E30", 0, 0, c, 0);
        let s = String::from_utf8_lossy(&out).to_string();
        // C's own output must show the 0xDEAD multiplier for negative levels.
        assert!(
            s.contains("Multiplier: 0xDEAD"),
            "E30: complexity={c} should route to the default arm; got:\n{s}"
        );
    }
    // Negative multiples of 5 reduce to level 0, which is IN range.
    for c in [-5i32, -10, -100, -2147483645] {
        let out = ms_out("E30", 0, 0, c, 0);
        let s = String::from_utf8_lossy(&out).to_string();
        assert!(
            s.contains("Complexity level: 0"),
            "E30: complexity={c} reduces to level 0; got:\n{s}"
        );
    }
}

#[test]
fn e31_modeselect_int_min_selector_is_in_range() {
    // INT_MIN % 4 == 0 in C, so index 0 is valid and this must NOT crash.
    let out = ms_out("E31", i32::MIN, 0, 0, 0);
    let s = String::from_utf8_lossy(&out).to_string();
    assert!(
        s.contains("Selected mode: standard (0x10)"),
        "E31: INT_MIN should select index 0; got:\n{s}"
    );
    // Every negative multiple of 4 likewise.
    for k in 1..=64i32 {
        ms_out("E31", -4 * k, 0, 0, 0);
    }
}

#[test]
fn e32_modeselect_negative_seed_gives_negative_offset_hours() {
    let mut rng = Rng::with_seed(SEED ^ 0xE32);
    for s in [-1i32, -12, -23, -24, -25, -47, i32::MIN, i32::MIN + 1] {
        ms_out("E32", 0, 0, 0, s);
    }
    for _ in 0..200 {
        ms_ret("E32", 0, 0, 0, rng.range_i32(i32::MIN, -1));
    }
}

#[test]
fn e33_modeselect_time_hash_modulo_never_negative() {
    // hash_time_value masks with 0x7FFFFFFF, so `time_hash % 0x1000` >= 0.
    // Verified indirectly: parse C's printed hash and check the invariant.
    let mut rng = Rng::with_seed(SEED ^ 0xE33);
    for _ in 0..400 {
        let out = ms_out(
            "E33",
            (rng.next_u32() >> 1) as i32,
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        let s = String::from_utf8_lossy(&out).to_string();
        let hash_hex = s
            .lines()
            .find(|l| l.starts_with("Modified time: "))
            .and_then(|l| l.rsplit("Hash: 0x").next())
            .expect("hash line present")
            .to_string();
        let h = i64::from_str_radix(hash_hex.trim(), 16).expect("hash parses");
        assert!(
            (0..=0x7FFF_FFFF).contains(&h),
            "E33: printed hash 0x{h:X} violates the 0x7FFFFFFF mask"
        );
    }
}

// ===========================================================================
// E8 / E29 — uncomparable UB, proven by forking
// ===========================================================================

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// Outcome of running `f` in a forked child.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Child returned normally with this exit code.
    Exited(i32),
    /// Child was killed by this signal.
    Signal(i32),
}

const SIGSEGV: i32 = 11;

/// Run `f` in a forked child and report how the child terminated.
///
/// The child does nothing but call `f` and `_exit`, so the usual
/// fork-in-a-threaded-process hazards (allocator locks, at-fork handlers) do not
/// apply: no allocation or locking happens on the child path.
fn run_forked<F: FnOnce() -> i32>(f: F) -> Outcome {
    // Flush first so buffered parent output is not duplicated by the child.
    print!("");
    use std::io::Write;
    std::io::stdout().flush().ok();
    // SAFETY: the child path only calls `f` (a single FFI call) and `_exit`,
    // which is async-signal-safe. The parent only waits.
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let v = f();
            _exit(if v == 0 { 0 } else { 1 });
        }
        let mut status: i32 = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        // WIFSIGNALED / WTERMSIG / WEXITSTATUS
        if status & 0x7F != 0 && status & 0x7F != 0x7F {
            Outcome::Signal(status & 0x7F)
        } else {
            Outcome::Exited((status >> 8) & 0xFF)
        }
    }
}

#[test]
fn e8_classify_mode_null_is_documented_ub() {
    let p = pair();
    // The C dereferences NULL inside `strcmp`. Prove it, rather than assuming.
    let c_out = run_forked(|| unsafe { (p.c.classify_mode)(std::ptr::null()) });
    assert_eq!(
        c_out,
        Outcome::Signal(SIGSEGV),
        "E8: expected the C classify_mode(NULL) to SIGSEGV; got {c_out:?}"
    );
    // The Rust does the same thing (it dereferences the pointer too), so the
    // behaviour MATCHES: both crash on a null pointer.
    let r_out = run_forked(|| unsafe { (p.rs.classify_mode)(std::ptr::null()) });
    assert_eq!(
        r_out, c_out,
        "E8: Rust classify_mode(NULL) must fail the same way the C does"
    );
}

#[test]
fn e29_modeselect_negative_selector_segfaults_in_c() {
    let p = pair();
    // `mode_selector % 4` in {-1,-2,-3} indexes before a 4-element stack array
    // and then `strcmp`s the garbage pointer. Prove the C really does crash, so
    // the "uncomparable UB" classification in ERRORS.md is evidence-backed and
    // not an excuse.
    let mut crashed = 0;
    for sel in [-1i32, -2, -3, -5, -6, -7, -9, -101] {
        let out = run_forked(|| unsafe { (p.c.modeselect)(sel, 0, 0, 0) });
        if out == Outcome::Signal(SIGSEGV) {
            crashed += 1;
        }
    }
    assert!(
        crashed > 0,
        "E29: expected the C to SIGSEGV for negative non-multiple-of-4 selectors; \
         none of 8 attempts crashed, so this row is comparable after all and needs a real test"
    );

    // The Rust must NOT crash for the same inputs: it substitutes an empty mode
    // string, which routes through classify_mode's terminal `return 0x00`.
    for sel in [-1i32, -2, -3, -5, -6, -7, -9, -101, i32::MIN + 1] {
        let out = run_forked(|| unsafe { (p.rs.modeselect)(sel, 0, 0, 0) });
        assert!(
            matches!(out, Outcome::Exited(_)),
            "E29: Rust modeselect({sel}) must not crash; got {out:?}"
        );
    }

    // And the non-UB negative selectors (multiples of 4) must still match the C
    // exactly -- that boundary is the whole reason E29 is narrow.
    for k in 1..=32i32 {
        let sel = -4 * k;
        // SAFETY: index reduces to 0, in range.
        let (rc, _) = capture_stdout(|| unsafe { (p.c.modeselect)(sel, 0, 0, 0) });
        let (rr, _) = capture_stdout(|| unsafe { (p.rs.modeselect)(sel, 0, 0, 0) });
        eq_int("E29", format!("modeselect({sel},0,0,0)"), rc, rr);
    }
}

// ===========================================================================
// Generic FFI boundary cases every C API has
// ===========================================================================

#[test]
fn generic_out_of_range_enum_values_across_ffi() {
    // `apply_multiplier`'s `switch` is the only enum-like dispatch. Feed it the
    // full set of "no valid variant" ints, including ones a Rust `enum` could
    // never hold.
    let mut rng = Rng::with_seed(0xDEAD_BEEF_0000_0001);
    for _ in 0..20000 {
        let level = rng.next_i32();
        let base = rng.next_i32();
        let p = pair();
        // SAFETY: plain scalar C ABI call.
        let (c, r) = unsafe {
            (
                (p.c.apply_multiplier)(base, level),
                (p.rs.apply_multiplier)(base, level),
            )
        };
        eq_int("generic-enum", format!("apply_multiplier({base}, {level})"), c, r);
        if !(0..=4).contains(&level) {
            assert_eq!(c, DEAD, "generic-enum: level={level} should be 0xDEAD");
        }
    }
}

#[test]
fn generic_zero_and_oversized_lengths() {
    // `classify_mode` is the only pointer/length-ish API. Zero length (empty
    // string) and very long strings.
    cm("generic-len", b"");
    let long = vec![b'a'; 1 << 16];
    cm("generic-len", &long);
    let mut long_prefixed = b"standard".to_vec();
    long_prefixed.extend(std::iter::repeat(b'z').take(1 << 16));
    cm("generic-len", &long_prefixed);
    // A string that is exactly a literal followed by nothing but NUL padding.
    let mut padded: Vec<c_char> = b"turbo".iter().map(|&b| b as c_char).collect();
    padded.extend(std::iter::repeat(0 as c_char).take(64));
    let p = pair();
    // SAFETY: NUL-terminated buffer alive across both calls.
    let (c, r) = unsafe {
        let ptr = padded.as_ptr() as *const c_char;
        ((p.c.classify_mode)(ptr), (p.rs.classify_mode)(ptr))
    };
    eq_int("generic-len", "classify_mode(\"turbo\" + NUL padding)", c, r);
    assert_eq!(c, 0x30);
}

#[test]
fn generic_one_step_past_every_documented_range() {
    // apply_multiplier: valid levels are 0..=4
    for level in [-1i32, 0, 4, 5] {
        am("generic-step", 0xA0, level);
    }
    // modeselect mode index: valid 0..=3, reached by selector % 4
    for sel in [0i32, 3, 4, 7, 8] {
        ms_ret("generic-step", sel, 0, 0, 0);
    }
    // modeselect complexity level: valid 0..=4, reached by complexity % 5
    for c in [0i32, 4, 5, 9, 10] {
        ms_ret("generic-step", 0, 0, c, 0);
    }
    // seed % 24 boundary
    for d in [0i32, 23, 24, 47, 48] {
        ms_ret("generic-step", 0, 0, 0, d);
    }
    // int cast boundaries
    ctf("generic-step", 2147483647.0 / 1e12);
    ctf("generic-step", 2147483648.0 / 1e12);
    cno("generic-step", 2147483647.0 / -1e15);
    cno("generic-step", 2147483648.0 / -1e15);
    // time_t extremes into hash_time_value
    htv("generic-step", i64::MIN);
    htv("generic-step", i64::MAX);
}

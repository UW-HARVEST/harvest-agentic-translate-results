//! Phase C — error-path / rejection differential tests.
//! One test (or one clearly-labelled block) per row of ERRORS.md, rows 1..=37.
//!
//! Each test constructs the exact invalid input the C checks for and asserts
//! that C and Rust produce the *same* sentinel/rejection value — not merely
//! that both "failed somehow". Rows 6 and 32 are the two rows where the C
//! crashes instead of returning; those are compared by exit status in a forked
//! child.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn both_classify(bytes: &[u8]) -> (i32, i32) {
    let p = pair();
    let (c, rs) = (p.c.classify_mode(), p.rs.classify_mode());
    let s = cstr(bytes);
    // SAFETY: `s` is NUL-terminated.
    unsafe { (c(s.as_ptr()), rs(s.as_ptr())) }
}

fn both_multiplier(base: i32, level: i32) -> (i32, i32) {
    let p = pair();
    let (c, rs) = (p.c.apply_multiplier(), p.rs.apply_multiplier());
    // SAFETY: plain scalar FFI call.
    unsafe { (c(base, level), rs(base, level)) }
}

fn both_ctf(x: f64) -> (i32, i32) {
    let p = pair();
    let (c, rs) = (p.c.convert_time_factor(), p.rs.convert_time_factor());
    // SAFETY: plain scalar FFI call.
    unsafe { (c(x), rs(x)) }
}

fn both_cno(x: f64) -> (i32, i32) {
    let p = pair();
    let (c, rs) = (
        p.c.convert_negative_overflow(),
        p.rs.convert_negative_overflow(),
    );
    // SAFETY: plain scalar FFI call.
    unsafe { (c(x), rs(x)) }
}

fn both_gmt(d: i32, h: i32) -> (i64, i64) {
    let p = pair();
    let (c, rs) = (p.c.get_modified_time(), p.rs.get_modified_time());
    // SAFETY: plain scalar FFI calls, issued back-to-back.
    unsafe {
        let (a, b) = (c(d, h), rs(d, h));
        if a != b { (c(d, h), rs(d, h)) } else { (a, b) }
    }
}

fn both_hash(t: i64) -> (i32, i32) {
    let p = pair();
    let (c, rs) = (p.c.hash_time_value(), p.rs.hash_time_value());
    // SAFETY: plain scalar FFI call.
    unsafe { (c(t), rs(t)) }
}

/// `modeselect` in both, each in its own child, with stdout captured.
fn both_modeselect(ms: i32, to: i32, cx: i32, seed: i32) -> (Option<i64>, Option<i64>, Vec<u8>, Vec<u8>) {
    let (mut a, mut b) = modeselect_both(ms, to, cx, seed);
    if a.ret != b.ret || a.stdout != b.stdout {
        let (a2, b2) = modeselect_both(ms, to, cx, seed);
        a = a2;
        b = b2;
    }
    (a.ret, b.ret, a.stdout, b.stdout)
}

/// Runs `f` in a forked child and reports how it terminated.
fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    call_in_child(|| {
        f();
        0
    })
    .outcome
}

// ===========================================================================
// Rows 1..=5 — classify_mode's `return 0x00` fallback
// ===========================================================================

#[test]
fn row_01_classify_mode_unrecognised_returns_zero() {
    for s in [
        &b"nope"[..],
        b"fast",
        b"STANDARD_MODE",
        b"\xFF\xFE\x01",
        b"0",
        b"standardenhanced",
    ] {
        let (a, b) = both_classify(s);
        assert_eq!(a, b, "row 1: C/Rust differ for {:?}", show(s));
        assert_eq!(a, 0x00, "row 1: expected sentinel 0x00 for {:?}", show(s));
    }
}

#[test]
fn row_02_classify_mode_empty_string_returns_zero() {
    let (a, b) = both_classify(b"");
    assert_eq!((a, b), (0x00, 0x00), "row 2: empty string must yield 0x00");
}

#[test]
fn row_03_classify_mode_strict_prefix_returns_zero() {
    for lit in [&b"standard"[..], b"enhanced", b"turbo", b"extreme"] {
        for n in 0..lit.len() {
            let (a, b) = both_classify(&lit[..n]);
            assert_eq!(a, b, "row 3: C/Rust differ for prefix {:?}", show(&lit[..n]));
            assert_eq!(a, 0x00, "row 3: prefix {:?} must yield 0x00", show(&lit[..n]));
        }
    }
}

#[test]
fn row_04_classify_mode_literal_plus_suffix_returns_zero() {
    for lit in [&b"standard"[..], b"enhanced", b"turbo", b"extreme"] {
        for suffix in [&b"X"[..], b" ", b"\t", b"0", b"\xFF", b"aaaaaaaaaaaaaaaa"] {
            let mut s = lit.to_vec();
            s.extend_from_slice(suffix);
            let (a, b) = both_classify(&s);
            assert_eq!(a, b, "row 4: C/Rust differ for {:?}", show(&s));
            assert_eq!(a, 0x00, "row 4: {:?} must yield 0x00", show(&s));
        }
    }
}

#[test]
fn row_05_classify_mode_is_case_sensitive() {
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
        let (a, b) = both_classify(s);
        assert_eq!(a, b, "row 5: C/Rust differ for {:?}", show(s));
        assert_eq!(a, 0x00, "row 5: {:?} must yield 0x00 (strcmp is case-sensitive)", show(s));
    }
}

// ===========================================================================
// Row 6 — classify_mode(NULL): both must fault identically
// ===========================================================================

#[test]
fn row_06_classify_mode_null_pointer_faults_in_both() {
    let p = pair();
    let c_out = {
        let c = p.c.classify_mode();
        // SAFETY: deliberately passing NULL; the call happens in a child process.
        run_in_child(|| unsafe {
            std::hint::black_box(c(std::ptr::null()));
        })
    };
    let rs_out = {
        let rs = p.rs.classify_mode();
        // SAFETY: deliberately passing NULL; the call happens in a child process.
        run_in_child(|| unsafe {
            std::hint::black_box(rs(std::ptr::null()));
        })
    };
    assert_eq!(
        c_out, rs_out,
        "row 6: classify_mode(NULL) must terminate the same way in both \
         implementations (C={c_out:?} Rust={rs_out:?})"
    );
    assert_eq!(
        c_out,
        Outcome::Signalled(libc::SIGSEGV),
        "row 6: expected SIGSEGV from dereferencing NULL"
    );
}

// ===========================================================================
// Rows 7..=10 — apply_multiplier's `default:` sentinel 0xDEAD
// ===========================================================================

const DEAD: i32 = 0xDEAD;

#[test]
fn row_07_apply_multiplier_out_of_range_level_returns_dead() {
    let mut rng = Rng::fixed();
    for _ in 0..20000 {
        // Any int outside 0..=4 is an out-of-range "enum" value across the FFI.
        let level = rng.next_i32();
        if (0..=4).contains(&level) {
            continue;
        }
        let base = rng.next_i32();
        let (a, b) = both_multiplier(base, level);
        assert_eq!(a, b, "row 7: C/Rust differ for base={base} level={level}");
        assert_eq!(
            a, DEAD,
            "row 7: level={level} must hit `default:` and return 0xDEAD (base is discarded)"
        );
    }
}

#[test]
fn row_08_apply_multiplier_level_five_returns_dead() {
    for base in [0, 1, -1, 0xA0, i32::MAX, i32::MIN] {
        let (a, b) = both_multiplier(base, 5);
        assert_eq!((a, b), (DEAD, DEAD), "row 8: level=5 must return 0xDEAD (base={base})");
    }
}

#[test]
fn row_09_apply_multiplier_level_minus_one_returns_dead() {
    for base in [0, 1, -1, 0xA0, i32::MAX, i32::MIN] {
        let (a, b) = both_multiplier(base, -1);
        assert_eq!((a, b), (DEAD, DEAD), "row 9: level=-1 must return 0xDEAD (base={base})");
    }
}

#[test]
fn row_10_apply_multiplier_extreme_levels_return_dead() {
    for level in [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -2,
        -3,
        -4,
        -5,
        6,
        7,
        100,
        -100,
        0x7FFF_FFFF,
        1 << 30,
    ] {
        for base in [0, 0xA0, i32::MAX, i32::MIN] {
            let (a, b) = both_multiplier(base, level);
            assert_eq!(a, b, "row 10: C/Rust differ for base={base} level={level}");
            assert_eq!(a, DEAD, "row 10: level={level} must return 0xDEAD");
        }
    }
    // Every valid level must NOT return the sentinel path's value by accident,
    // i.e. the boundary between valid and invalid is exactly 0..=4.
    for level in 0..=4 {
        let (a, b) = both_multiplier(0, level);
        assert_eq!(a, b, "row 10: C/Rust differ for valid level={level}");
        assert_ne!(a, DEAD, "row 10: valid level={level} must not take `default:`");
    }
}

#[test]
fn row_11_apply_multiplier_signed_overflow_wraps_identically() {
    for level in 0..=4 {
        for k in 0..=0x400i64 {
            let base = (i32::MAX as i64 - k) as i32;
            let (a, b) = both_multiplier(base, level);
            assert_eq!(
                a, b,
                "row 11: overflow wrap differs for base={base} (INT_MAX-{k}) level={level}"
            );
        }
    }
    // Confirm at least one case genuinely overflowed (result went negative).
    let (a, b) = both_multiplier(i32::MAX, 4);
    assert_eq!(a, b);
    assert!(a < 0, "row 11: expected wrapped (negative) result, got {a}");
}

// ===========================================================================
// Rows 12..=17 — convert_time_factor out-of-range / NaN / inf
// ===========================================================================

const INDEF: i32 = i32::MIN; // x86-64 cvttsd2si "integer indefinite"

#[test]
fn row_12_13_convert_time_factor_out_of_int_range() {
    // factor*1e12 above INT_MAX
    for x in [1.0f64, 0.003, 2.2e-3, 1e10, f64::MAX, 1e300] {
        let (a, b) = both_ctf(x);
        assert_eq!(a, b, "row 12: C/Rust differ for factor={x:?}");
        assert_eq!(a, INDEF, "row 12: factor={x:?} overflows int, expected INT_MIN");
    }
    // factor*1e12 below INT_MIN
    for x in [-1.0f64, -0.003, -2.2e-3, -1e10, f64::MIN, -1e300] {
        let (a, b) = both_ctf(x);
        assert_eq!(a, b, "row 13: C/Rust differ for factor={x:?}");
        assert_eq!(a, INDEF, "row 13: factor={x:?} underflows int, expected INT_MIN");
    }
}

#[test]
fn row_14_convert_time_factor_nan() {
    for x in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000),
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling
        f64::from_bits(0xFFF0_0000_0000_0001),
    ] {
        let (a, b) = both_ctf(x);
        assert_eq!(a, b, "row 14: C/Rust differ for NaN bits 0x{:016X}", x.to_bits());
        assert_eq!(a, INDEF, "row 14: NaN must convert to INT_MIN");
    }
}

#[test]
fn row_15_convert_time_factor_infinities() {
    for x in [f64::INFINITY, f64::NEG_INFINITY] {
        let (a, b) = both_ctf(x);
        assert_eq!(a, b, "row 15: C/Rust differ for {x:?}");
        assert_eq!(a, INDEF, "row 15: {x:?} must convert to INT_MIN");
    }
}

#[test]
fn row_16_convert_time_factor_zero_and_subnormal_give_zero() {
    for x in [
        0.0f64,
        -0.0,
        5e-324,
        -5e-324,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        1e-300,
        -1e-300,
        1e-13,
        -1e-13,
    ] {
        let (a, b) = both_ctf(x);
        assert_eq!(a, b, "row 16: C/Rust differ for factor={x:?}");
        assert_eq!(a, 0, "row 16: factor={x:?} must truncate to 0");
    }
}

#[test]
fn row_17_convert_time_factor_exact_boundaries() {
    // In range: product <= INT_MAX, product >= INT_MIN.
    for (x, want) in [
        (2_147_483_647.0f64 / 1e12, 2_147_483_647i32),
        (-2_147_483_648.0 / 1e12, -2_147_483_648),
        (2_147_483_646.0 / 1e12, 2_147_483_646),
    ] {
        let (a, b) = both_ctf(x);
        assert_eq!(a, b, "row 17: C/Rust differ at boundary factor={x:?}");
        // The division is inexact, so allow the neighbouring integer, but the
        // point is that C and Rust agree AND it is not the indefinite value.
        assert!(
            (a as i64 - want as i64).abs() <= 1,
            "row 17: factor={x:?} expected ~{want}, got {a}"
        );
    }
    // One step past: product just above INT_MAX / just below INT_MIN.
    for x in [2_147_483_649.0f64 / 1e12, -2_147_483_650.0 / 1e12] {
        let (a, b) = both_ctf(x);
        assert_eq!(a, b, "row 17: C/Rust differ just past boundary factor={x:?}");
        assert_eq!(a, INDEF, "row 17: factor={x:?} is out of range, expected INT_MIN");
    }
}

// ===========================================================================
// Rows 18..=22 — convert_negative_overflow
// ===========================================================================

#[test]
fn row_18_19_convert_negative_overflow_out_of_int_range() {
    // value*-1e15 below INT_MIN (positive value -> large negative product)
    for x in [1.0f64, 1e-5, 2.2e-6, 1e10, f64::MAX] {
        let (a, b) = both_cno(x);
        assert_eq!(a, b, "row 18: C/Rust differ for value={x:?}");
        assert_eq!(a, INDEF, "row 18: value={x:?} underflows int, expected INT_MIN");
    }
    // value*-1e15 above INT_MAX (negative value -> large positive product)
    for x in [-1.0f64, -1e-5, -2.2e-6, -1e10, f64::MIN] {
        let (a, b) = both_cno(x);
        assert_eq!(a, b, "row 19: C/Rust differ for value={x:?}");
        assert_eq!(a, INDEF, "row 19: value={x:?} overflows int, expected INT_MIN");
    }
}

#[test]
fn row_20_convert_negative_overflow_nan() {
    for x in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000),
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF0_0000_0000_0001),
    ] {
        let (a, b) = both_cno(x);
        assert_eq!(a, b, "row 20: C/Rust differ for NaN bits 0x{:016X}", x.to_bits());
        assert_eq!(a, INDEF, "row 20: NaN must convert to INT_MIN");
    }
}

#[test]
fn row_21_convert_negative_overflow_infinities() {
    for x in [f64::INFINITY, f64::NEG_INFINITY] {
        let (a, b) = both_cno(x);
        assert_eq!(a, b, "row 21: C/Rust differ for {x:?}");
        assert_eq!(a, INDEF, "row 21: {x:?} must convert to INT_MIN");
    }
}

#[test]
fn row_22_convert_negative_overflow_signed_zeros() {
    for x in [0.0f64, -0.0, 5e-324, -5e-324, f64::MIN_POSITIVE, 1e-300, -1e-300, 1e-16] {
        let (a, b) = both_cno(x);
        assert_eq!(a, b, "row 22: C/Rust differ for value={x:?}");
        assert_eq!(a, 0, "row 22: value={x:?} must truncate to 0 (sign of zero is lost)");
    }
}

// ===========================================================================
// Rows 23..=26 — get_modified_time int-overflow paths
// ===========================================================================

#[test]
fn row_23_get_modified_time_days_product_overflow_wraps() {
    for d in [24856, 100_000, 1_000_000, i32::MAX, -24856, -100_000, i32::MIN] {
        let (a, b) = both_gmt(d, 0);
        assert_eq!(a, b, "row 23: C/Rust differ for offset_days={d}");
    }
    // Confirm the wrap is real: 100000*86400 = 8_640_000_000 does not fit in an
    // int, so the `int` product differs from the mathematical one.
    let (a, b) = both_gmt(100_000, 0);
    assert_eq!(a, b);
    let wrapped = 100_000i32.wrapping_mul(86400) as i64;
    assert_ne!(
        wrapped,
        100_000i64 * 86400,
        "row 23: sanity — days*86400 must wrap in int, got {wrapped}"
    );
}

#[test]
fn row_24_get_modified_time_hours_product_overflow_wraps() {
    for h in [596_524, 1_000_000, i32::MAX, -596_524, -1_000_000, i32::MIN] {
        let (a, b) = both_gmt(0, h);
        assert_eq!(a, b, "row 24: C/Rust differ for offset_hours={h}");
    }
}

#[test]
fn row_25_get_modified_time_extreme_offsets() {
    for d in [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
        for h in [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 0, -1, 1] {
            let (a, b) = both_gmt(d, h);
            assert_eq!(a, b, "row 25: C/Rust differ for days={d} hours={h}");
        }
    }
}

#[test]
fn row_26_get_modified_time_sum_overflow_wraps() {
    // 24855*86400 = 2_147_472_000 and 596_523*3600 = 2_147_482_800; each fits,
    // the sum (4_294_954_800) does not.
    for (d, h) in [
        (24855, 596_523),
        (-24855, -596_523),
        (24855, 1),
        (1, 596_523),
        (20000, 400_000),
        (-20000, -400_000),
    ] {
        let (a, b) = both_gmt(d, h);
        assert_eq!(a, b, "row 26: C/Rust differ for days={d} hours={h}");
    }
    let s = 24855i32.wrapping_mul(86400).wrapping_add(596_523i32.wrapping_mul(3600));
    assert!(s < 0, "row 26: sanity — the sum must wrap negative, got {s}");
}

// ===========================================================================
// Rows 27..=29 — hash_time_value UB shifts / multiplies
// ===========================================================================

#[test]
fn row_27_hash_time_value_negative_input_shift_into_sign_bit() {
    let mut rng = Rng::fixed();
    for _ in 0..5000 {
        let t = -((rng.next_u64() % (1u64 << 40)) as i64);
        let (a, b) = both_hash(t);
        assert_eq!(a, b, "row 27: C/Rust differ for t={t}");
        assert!(
            (0..=0x7FFF_FFFF).contains(&a),
            "row 27: result {a} outside [0, 0x7FFFFFFF] for t={t}"
        );
    }
    // Bytes >= 0x80 at shift position 3 (i%4==3 -> <<24 touches the sign bit).
    for pos in [3usize, 7] {
        for v in [0x80u8, 0xC0, 0xFF] {
            let mut bs = [0u8; 8];
            bs[pos] = v;
            let t = i64::from_ne_bytes(bs);
            let (a, b) = both_hash(t);
            assert_eq!(a, b, "row 27: C/Rust differ for byte {v:#X} at position {pos}");
        }
    }
}

#[test]
fn row_28_hash_time_value_int64_extremes() {
    for t in [i64::MIN, i64::MAX, i64::MIN + 1, i64::MAX - 1, -1, 0] {
        let (a, b) = both_hash(t);
        assert_eq!(a, b, "row 28: C/Rust differ for t={t}");
        assert!(a >= 0, "row 28: result must be non-negative, got {a} for t={t}");
    }
}

#[test]
fn row_29_hash_time_value_multiply_overflow_never_negative() {
    let mut rng = Rng::fixed();
    for _ in 0..20000 {
        let t = rng.next_i64();
        let (a, b) = both_hash(t);
        assert_eq!(a, b, "row 29: C/Rust differ for t={t}");
        assert!(
            (0..=0x7FFF_FFFF).contains(&a),
            "row 29: `& 0x7FFFFFFF` must keep the result non-negative, got {a}"
        );
    }
}

// ===========================================================================
// Rows 30..=31, 33..=37 — modeselect rejection / overflow paths
// ===========================================================================

#[test]
fn row_30_modeselect_negative_complexity_prints_dead_multiplier() {
    for cx in [-1, -2, -3, -4, -6, -7, -8, -9, -11, -100, i32::MIN, i32::MIN + 1] {
        let (ra, rb, oa, ob) = both_modeselect(0, 0, cx, 0);
        assert_eq!(show(&oa), show(&ob), "row 30: stdout differs for complexity={cx}");
        assert_eq!(ra, rb, "row 30: return differs for complexity={cx}");
        let s = String::from_utf8_lossy(&oa).to_string();
        let level = cx % 5;
        if level != 0 {
            assert!(
                s.contains(&format!("Complexity level: {level}, Multiplier: 0xDEAD\n")),
                "row 30: complexity={cx} (level {level}) must print the 0xDEAD sentinel; got:\n{s}"
            );
        }
    }
}

#[test]
fn row_31_modeselect_negative_seed_mod24() {
    for seed in [-1, -12, -23, -25, -100, i32::MIN, i32::MIN + 1, i32::MAX] {
        let (ra, rb, oa, ob) = both_modeselect(0, 0, 0, seed);
        assert_eq!(show(&oa), show(&ob), "row 31: stdout differs for seed={seed}");
        assert_eq!(ra, rb, "row 31: return differs for seed={seed}");
    }
}

// ---------------------------------------------------------------------------
// Row 32 — mode_selector % 4 in {-1,-2,-3}: the C reads out of bounds
// ---------------------------------------------------------------------------

#[test]
fn row_32_modeselect_negative_index_is_ub() {
    let p = pair();

    // In-bounds negative selectors (index 0) must agree exactly.
    for ms in [-4i32, -8, -1024, i32::MIN] {
        let (ra, rb, oa, ob) = both_modeselect(ms, 0, 0, 0);
        assert_eq!(show(&oa), show(&ob), "row 32: stdout differs for in-bounds ms={ms}");
        assert_eq!(ra, rb, "row 32: return differs for in-bounds ms={ms}");
        assert!(
            String::from_utf8_lossy(&oa).contains("Selected mode: standard (0x10)"),
            "row 32: ms={ms} has ms%4==0 and must select \"standard\""
        );
    }

    // Out-of-bounds selectors: the C reads past the start of its 4-element local
    // `modes` array. From the -O0 disassembly, `modes` sits at -0x70(%rbp) and
    // the spilled arguments sit directly below it, so the "pointer" the C then
    // hands to strcmp is:
    //
    //   mode_index == -1  ->  8 bytes at -0x78(%rbp)
    //                         = (mode_selector as u64) << 32 | time_offset as u32
    //   mode_index == -2  ->  8 bytes at -0x80(%rbp)
    //                         = (complexity as u64) << 32 | seed as u32
    //   mode_index == -3  ->  8 bytes at -0x88(%rbp), i.e. BELOW %rsp: leftover
    //                         stack garbage from whatever ran before
    //
    // This is why the Rust cannot reproduce it: the value is a property of one
    // particular gcc stack layout (and, for -3, of unrelated prior stack
    // contents), not of the C source. See ERRORS.md row 32.
    //
    // The -1 and -2 cases ARE predictable enough to assert: reaching
    // mode_index == -1 requires mode_selector < 0, so the reconstructed address
    // always has its top 32 bits set and is non-canonical -> guaranteed SIGSEGV.
    // For mode_index == -2 we pin complexity and seed to 0, making the address
    // NULL -> guaranteed SIGSEGV.
    for ms in [-1i32, -5, -9, -2_147_483_645] {
        assert_eq!(ms % 4, -1, "test bug: ms={ms} is not mode_index -1");
        let c = p.c.modeselect();
        // SAFETY: the out-of-bounds read happens in a child process.
        let out = run_in_child(|| unsafe {
            std::hint::black_box(c(ms, 0, 0, 0));
        });
        assert_eq!(
            out,
            Outcome::Signalled(libc::SIGSEGV),
            "row 32: mode_index -1 builds a non-canonical address from \
             mode_selector={ms}; expected SIGSEGV in the C"
        );
    }
    for ms in [-2i32, -6, -10, -2_147_483_646] {
        assert_eq!(ms % 4, -2, "test bug: ms={ms} is not mode_index -2");
        let c = p.c.modeselect();
        // complexity = 0 and seed = 0 => the reconstructed address is NULL.
        // SAFETY: the out-of-bounds read happens in a child process.
        let out = run_in_child(|| unsafe {
            std::hint::black_box(c(ms, 0, 0, 0));
        });
        assert_eq!(
            out,
            Outcome::Signalled(libc::SIGSEGV),
            "row 32: mode_index -2 with complexity=seed=0 builds a NULL address \
             from mode_selector={ms}; expected SIGSEGV in the C"
        );
    }

    // mode_index == -3 reads below %rsp. Its outcome depends on unrelated
    // leftover stack contents, so it is deliberately NOT asserted: it has been
    // observed both to SIGSEGV and to return a garbage-derived value in the same
    // build. Assert only that it is one of those two, which documents that the C
    // has no input-determined result here for the Rust to match.
    for ms in [-3i32, -7, -11] {
        assert_eq!(ms % 4, -3, "test bug: ms={ms} is not mode_index -3");
        let c = p.c.modeselect();
        // SAFETY: the out-of-bounds read happens in a child process.
        let out = run_in_child(|| unsafe {
            std::hint::black_box(c(ms, 0, 0, 0));
        });
        assert!(
            matches!(out, Outcome::Signalled(libc::SIGSEGV) | Outcome::Exited(0)),
            "row 32: unexpected outcome {out:?} for mode_selector={ms}"
        );
    }

    // Whatever the C does, the Rust must at least be memory-safe and return its
    // documented fallback (empty mode string -> classify_mode == 0x00).
    for ms in [-1i32, -2, -3, -5, -6, -7] {
        let rs = p.rs.modeselect();
        // SAFETY: plain scalar FFI call in a child process.
        let out = call_in_child(|| unsafe { rs(ms, 0, 0, 0) } as i64);
        assert_eq!(
            out.outcome,
            Outcome::Exited(0),
            "row 32: the Rust must not crash for mode_selector={ms}"
        );
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("Selected mode:  (0x0)"),
            "row 32: the Rust fallback must select the empty mode for ms={ms}; got:\n{}",
            show(&out.stdout)
        );
    }
}

#[test]
fn row_33_modeselect_int_min_selector_is_in_bounds() {
    // INT_MIN % 4 == 0, so this is the "standard" mode and must not crash.
    assert_eq!(i32::MIN % 4, 0);
    let (ra, rb, oa, ob) = both_modeselect(i32::MIN, 0, 0, 0);
    assert_eq!(show(&oa), show(&ob), "row 33: stdout differs for mode_selector=INT_MIN");
    assert_eq!(ra, rb, "row 33: return differs for mode_selector=INT_MIN");
    assert!(
        String::from_utf8_lossy(&oa).contains("Selected mode: standard (0x10)"),
        "row 33: INT_MIN selector must map to \"standard\""
    );
}

#[test]
fn row_34_modeselect_time_offset_overflow_inside_pipeline() {
    for to in [24856, 100_000, i32::MAX, -24856, -100_000, i32::MIN, i32::MAX - 1] {
        let (ra, rb, oa, ob) = both_modeselect(0, to, 0, 0);
        assert_eq!(show(&oa), show(&ob), "row 34: stdout differs for time_offset={to}");
        assert_eq!(ra, rb, "row 34: return differs for time_offset={to}");
    }
}

#[test]
fn row_35_modeselect_nonzero_seed_overflows_result1() {
    // (double)seed * 1e8 * 1e12 exceeds INT_MAX for every seed != 0.
    for seed in [1, -1, 2, 42, -42, i32::MAX, i32::MIN] {
        let (ra, rb, oa, ob) = both_modeselect(0, 0, 0, seed);
        assert_eq!(show(&oa), show(&ob), "row 35: stdout differs for seed={seed}");
        assert_eq!(ra, rb, "row 35: return differs for seed={seed}");
        assert!(
            String::from_utf8_lossy(&oa).contains("Result 1: -2147483648 (0x80000000)"),
            "row 35: seed={seed} must overflow to the indefinite value; got:\n{}",
            show(&oa)
        );
    }
    // seed == 0 is the only case that does not overflow.
    let (_, _, oa, ob) = both_modeselect(0, 0, 0, 0);
    assert_eq!(show(&oa), show(&ob));
    let s = String::from_utf8_lossy(&oa).to_string();
    assert!(s.contains("Result 1: 0 (0x0)"), "row 35: seed=0 must give 0; got:\n{s}");
    assert!(
        s.contains("Converting double 0.00e+00 to int"),
        "row 35: seed=0 must print 0.00e+00; got:\n{s}"
    );
}

#[test]
fn row_36_modeselect_nonzero_time_offset_overflows_result2() {
    for to in [1, -1, 2, 42, -42, i32::MAX, i32::MIN] {
        let (ra, rb, oa, ob) = both_modeselect(0, to, 0, 0);
        assert_eq!(show(&oa), show(&ob), "row 36: stdout differs for time_offset={to}");
        assert_eq!(ra, rb, "row 36: return differs for time_offset={to}");
        assert!(
            String::from_utf8_lossy(&oa).contains("Result 2: -2147483648 (0x80000000)"),
            "row 36: time_offset={to} must overflow to the indefinite value; got:\n{}",
            show(&oa)
        );
    }
    // time_offset == 0 -> factor2 is NEGATIVE zero: "-0.00e+00".
    let (_, _, oa, ob) = both_modeselect(0, 0, 0, 0);
    assert_eq!(show(&oa), show(&ob));
    let s = String::from_utf8_lossy(&oa).to_string();
    assert!(
        s.contains("Converting double -0.00e+00 to int (may underflow)...")
            && s.contains("Result 2: 0 (0x0)"),
        "row 36: time_offset=0 must print negative zero and Result 2: 0; got:\n{s}"
    );
}

#[test]
fn row_37_modeselect_final_arithmetic_overflow_wraps() {
    // result*0x10 + 0xBEEF can overflow int; sweep many in-bounds inputs and
    // require agreement (the return value is the only observable).
    let mut rng = Rng::fixed();
    for _ in 0..400 {
        let ms = (rng.next_u64() >> 33) as i32;
        let (ra, rb, oa, ob) = both_modeselect(ms, rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(show(&oa), show(&ob), "row 37: stdout differs");
        assert_eq!(ra, rb, "row 37: return differs");
    }
}

// ===========================================================================
// Generic FFI boundary coverage required by Phase C, beyond the table
// ===========================================================================

#[test]
fn generic_out_of_range_enum_values_across_ffi() {
    // `level` is the only enum-like parameter. A C enum accepts any int, so
    // sweep dense neighbourhoods around the valid range and both extremes.
    for level in (-64..=64).chain([i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1]) {
        let (a, b) = both_multiplier(0xA0, level);
        assert_eq!(a, b, "generic: C/Rust differ for level={level}");
        if (0..=4).contains(&level) {
            assert_ne!(a, DEAD, "generic: level={level} is valid");
        } else {
            assert_eq!(a, DEAD, "generic: level={level} is out of range -> 0xDEAD");
        }
    }
}

#[test]
fn generic_zero_and_oversized_string_lengths() {
    // classify_mode's only length limits are implicit in strcmp.
    let (a, b) = both_classify(b"");
    assert_eq!((a, b), (0, 0), "generic: zero-length string");

    for len in [1usize, 7, 8, 9, 1024, 65536, 1_000_000] {
        let s = vec![b'x'; len];
        let (a, b) = both_classify(&s);
        assert_eq!(a, b, "generic: C/Rust differ for {len}-byte string");
        assert_eq!(a, 0, "generic: {len}-byte non-matching string must yield 0");
    }
    // A very long string that *starts* with a literal must still be rejected.
    let mut s = b"standard".to_vec();
    s.extend(std::iter::repeat_n(b'x', 1_000_000));
    let (a, b) = both_classify(&s);
    assert_eq!(a, b);
    assert_eq!(a, 0, "generic: literal + 1MB suffix must yield 0");
}

#[test]
fn generic_one_step_past_every_documented_range() {
    // apply_multiplier: valid levels 0..=4, so -1 and 5 are one step past.
    assert_eq!(both_multiplier(0xA0, -1), (DEAD, DEAD));
    assert_eq!(both_multiplier(0xA0, 5), (DEAD, DEAD));
    assert_ne!(both_multiplier(0xA0, 0).0, DEAD);
    assert_ne!(both_multiplier(0xA0, 4).0, DEAD);

    // modeselect: mode_selector % 4 valid range is 0..=3; one step past on the
    // positive side simply wraps to 0, which must agree.
    for ms in [3, 4, 0, 1] {
        let (ra, rb, oa, ob) = both_modeselect(ms, 0, 0, 0);
        assert_eq!(show(&oa), show(&ob), "generic: stdout differs for ms={ms}");
        assert_eq!(ra, rb, "generic: return differs for ms={ms}");
    }

    // convert_*: one step past the int-range boundary in f64 ULPs.
    for scale in [1e12f64, -1e15] {
        for edge in [2_147_483_647.0f64, -2_147_483_648.0] {
            let x = edge / scale;
            for delta in [-2i64, -1, 0, 1, 2] {
                let bits = (x.to_bits() as i64 + delta) as u64;
                let v = f64::from_bits(bits);
                let (a, b) = if scale > 0.0 { both_ctf(v) } else { both_cno(v) };
                assert_eq!(
                    a, b,
                    "generic: C/Rust differ one ULP past boundary, scale={scale} x={v:?}"
                );
            }
        }
    }

    // get_modified_time: one step past the non-overflowing product ranges.
    for d in [24855, 24856, -24855, -24856] {
        let (a, b) = both_gmt(d, 0);
        assert_eq!(a, b, "generic: get_modified_time differs at days={d}");
    }
    for h in [596_523, 596_524, -596_523, -596_524] {
        let (a, b) = both_gmt(0, h);
        assert_eq!(a, b, "generic: get_modified_time differs at hours={h}");
    }
}

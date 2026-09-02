// Phase B -- valid-path differential tests, one test per row of CONFIGS.md.
// Every call goes through a `.so` export loaded with libloading, for BOTH the C
// and the Rust library. Randomized inputs use a fixed seed (common::SEED).

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 2000;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn next_up(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f64::from_bits(1);
    }
    if x > 0.0 {
        f64::from_bits(x.to_bits() + 1)
    } else {
        f64::from_bits(x.to_bits() - 1)
    }
}
fn next_down(x: f64) -> f64 {
    -next_up(-x)
}

/// Differential check for `safe_double_to_int`.
fn chk_sdti(p: &Pair, d: f64, row: &str) {
    let (cv, rv) = unsafe { ((p.c.safe_double_to_int)(d), (p.r.safe_double_to_int)(d)) };
    assert_eq!(
        cv, rv,
        "[{row}] safe_double_to_int({}) -> C={cv} RUST={rv}",
        fmt_f64(d)
    );
}

fn chk_pwf(p: &Pair, code: i32, base: i32, row: &str) {
    let (cv, rv) = unsafe {
        (
            (p.c.process_with_fallthrough)(code, base),
            (p.r.process_with_fallthrough)(code, base),
        )
    };
    assert_eq!(
        cv, rv,
        "[{row}] process_with_fallthrough({code}, {base}) -> C={cv} RUST={rv}"
    );
}

fn chk_hpo(p: &Pair, v: i32, row: &str) {
    let (cv, rv) = unsafe {
        (
            (p.c.handle_pointer_operations)(v),
            (p.r.handle_pointer_operations)(v),
        )
    };
    assert_eq!(
        cv, rv,
        "[{row}] handle_pointer_operations({v}) -> C={cv} RUST={rv}"
    );
}

fn chk_overunder(p: &Pair, a: i32, b: i32, c: i32, d: i32, row: &str) -> i32 {
    let (cv, rv) = unsafe {
        (
            (p.c.overunder)(a, b, c, d),
            (p.r.overunder)(a, b, c, d),
        )
    };
    assert_eq!(
        cv, rv,
        "[{row}] overunder({a}, {b}, {c}, {d}) -> C={cv} RUST={rv}"
    );
    cv
}

/// Copy `src` through both implementations into poisoned destinations and
/// require that the two destinations are byte-identical over the whole probe
/// buffer (so both the copied bytes AND the untouched guard bytes must agree).
fn chk_copy(p: &Pair, src_bytes: &[u8; PROBE], poison: u8, row: &str) {
    let mut src = Probe(*src_bytes);
    let mut dc = Probe::filled(poison);
    let mut dr = Probe::filled(poison);
    unsafe {
        (p.c.copy_data_block)(dc.as_mut_ptr(), src.as_ptr());
        (p.r.copy_data_block)(dr.as_mut_ptr(), src.as_ptr());
    }
    assert_eq!(
        dc.0.as_slice(),
        dr.0.as_slice(),
        "[{row}] copy_data_block mismatch\n C  = {:02x?}\n RUST= {:02x?}",
        &dc.0[..64],
        &dr.0[..64]
    );
    // Both must have copied exactly the same prefix and touched nothing after.
    let changed_c: Vec<usize> = (0..PROBE).filter(|&i| dc.0[i] != poison).collect();
    let changed_r: Vec<usize> = (0..PROBE).filter(|&i| dr.0[i] != poison).collect();
    assert_eq!(changed_c, changed_r, "[{row}] differing written-byte sets");
    let _ = src.as_mut_ptr();
}

// ---------------------------------------------------------------------------
// Rows 1-6: safe_double_to_int
// ---------------------------------------------------------------------------

#[test]
fn row01_sdti_inrange_positive() {
    let p = load_pair();
    let mut rng = Rng::new(SEED);
    for _ in 0..N {
        let d = rng.next_f64_unit() * 2_147_483_647.0;
        chk_sdti(&p, d, "row1");
    }
}

#[test]
fn row02_sdti_inrange_negative() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let d = -rng.next_f64_unit() * 2_147_483_648.0;
        chk_sdti(&p, d, "row2");
    }
}

#[test]
fn row03_sdti_exact_integers_and_signed_zero() {
    let p = load_pair();
    for d in [0.0f64, -0.0, 1.0, -1.0, 2.0, -2.0, 1e9, -1e9] {
        chk_sdti(&p, d, "row3");
    }
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let i = rng.next_i32();
        chk_sdti(&p, i as f64, "row3");
        // ... and the same magnitude with a fractional part in both directions
        chk_sdti(&p, i as f64 + 0.5, "row3");
        chk_sdti(&p, i as f64 - 0.5, "row3");
    }
}

#[test]
fn row04_sdti_guard_boundaries() {
    let p = load_pair();
    let imax = 2_147_483_647.0f64;
    let imin = -2_147_483_648.0f64;
    let mut vals = vec![
        imax,
        imin,
        next_up(imax),
        next_down(imax),
        next_up(imin),
        next_down(imin),
        imax + 1.0,
        imin - 1.0,
        2_147_483_646.5,
        2_147_483_647.5,
        -2_147_483_647.5,
        -2_147_483_648.5,
        -2_147_483_649.0,
    ];
    // one step past the guards in both directions, several ulps out
    for k in 1..8u64 {
        vals.push(f64::from_bits(imax.to_bits() + k));
        vals.push(f64::from_bits(imax.to_bits() - k));
        vals.push(f64::from_bits(imin.to_bits() + k));
        vals.push(f64::from_bits(imin.to_bits() - k));
    }
    for d in vals {
        chk_sdti(&p, d, "row4");
    }
}

#[test]
fn row05_sdti_special_doubles() {
    let p = load_pair();
    let specials = [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001), // quiet NaN, payload 1
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff8_0000_dead_beef), // negative NaN with payload
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),  // smallest subnormal
        f64::from_bits(0x8000_0000_0000_0001), // -smallest subnormal
        0.0,
        -0.0,
        f64::EPSILON,
        -f64::EPSILON,
    ];
    for d in specials {
        chk_sdti(&p, d, "row5");
    }
}

#[test]
fn row06_sdti_random_bit_patterns() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..(N * 10) {
        chk_sdti(&p, rng.next_f64_bits(), "row6");
    }
}

// ---------------------------------------------------------------------------
// Rows 7-14: process_with_fallthrough
// ---------------------------------------------------------------------------

fn pwf_row_for_code(code: i32, row: &str, seed: u64) {
    let p = load_pair();
    let mut rng = Rng::new(seed);
    // hand-picked plus randomized base values
    for base in [0, 1, -1, i32::MAX, i32::MIN, i32::MAX - 25, i32::MIN + 25] {
        chk_pwf(&p, code, base, row);
    }
    for _ in 0..N {
        chk_pwf(&p, code, rng.next_i32(), row);
    }
}

#[test]
fn row07_pwf_code5() {
    pwf_row_for_code(5, "row7", SEED ^ 7);
}
#[test]
fn row08_pwf_code4() {
    pwf_row_for_code(4, "row8", SEED ^ 8);
}
#[test]
fn row09_pwf_code3() {
    pwf_row_for_code(3, "row9", SEED ^ 9);
}
#[test]
fn row10_pwf_code2() {
    pwf_row_for_code(2, "row10", SEED ^ 10);
}
#[test]
fn row11_pwf_code1() {
    pwf_row_for_code(1, "row11", SEED ^ 11);
}
#[test]
fn row12_pwf_code0() {
    pwf_row_for_code(0, "row12", SEED ^ 12);
}

#[test]
fn row13_pwf_wrap_interaction() {
    let p = load_pair();
    for code in 1..=5 {
        for delta in 0..64i64 {
            chk_pwf(&p, code, (i32::MAX as i64 - delta) as i32, "row13");
            chk_pwf(&p, code, (i32::MIN as i64 + delta) as i32, "row13");
        }
    }
}

#[test]
fn row14_pwf_full_random_cross_product() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..(N * 5) {
        chk_pwf(&p, rng.next_i32(), rng.next_i32(), "row14");
    }
    // dense sweep over the interesting code neighbourhood
    for code in -20..=20 {
        for base in [-7, 0, 7, i32::MAX, i32::MIN] {
            chk_pwf(&p, code, base, "row14");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15-18: copy_data_block
// ---------------------------------------------------------------------------

#[test]
fn row15_copy_all_zero() {
    let p = load_pair();
    chk_copy(&p, &[0u8; PROBE], 0xAA, "row15");
    chk_copy(&p, &[0u8; PROBE], 0x00, "row15");
}

#[test]
fn row16_copy_all_ff_unterminated_label() {
    let p = load_pair();
    chk_copy(&p, &[0xFFu8; PROBE], 0x5A, "row16");
    let mut src = [0u8; PROBE];
    // label[16..36] fully populated with no NUL
    for i in 16..36 {
        src[i] = b'Z';
    }
    chk_copy(&p, &src, 0x11, "row16");
}

#[test]
fn row17_copy_random_including_padding_and_special_doubles() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..N {
        let mut src = [0u8; PROBE];
        for b in src.iter_mut() {
            *b = rng.next_u8();
        }
        chk_copy(&p, &src, rng.next_u8(), "row17");
    }
    // value field holding NaN / inf / subnormal, padding poisoned
    for bits in [
        0x7ff8_0000_0000_0001u64,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x8000_0000_0000_0000,
    ] {
        let mut src = [0xCDu8; PROBE];
        src[8..16].copy_from_slice(&bits.to_le_bytes());
        chk_copy(&p, &src, 0x77, "row17");
    }
}

#[test]
fn row18_copy_poisoned_destination_bounds() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..64 {
        let mut src = [0u8; PROBE];
        for b in src.iter_mut() {
            *b = rng.next_u8() | 1; // never equal to the 0x00 poison
        }
        chk_copy(&p, &src, 0x00, "row18");
    }
    // Confirm both wrote exactly DATABLOCK_SIZE bytes and nothing beyond.
    let src = Probe([0xABu8; PROBE]);
    let mut dc = Probe::filled(0x00);
    let mut dr = Probe::filled(0x00);
    unsafe {
        (p.c.copy_data_block)(dc.as_mut_ptr(), src.as_ptr());
        (p.r.copy_data_block)(dr.as_mut_ptr(), src.as_ptr());
    }
    let n_c = (0..PROBE).filter(|&i| dc.0[i] == 0xAB).count();
    let n_r = (0..PROBE).filter(|&i| dr.0[i] == 0xAB).count();
    assert_eq!(n_c, n_r, "row18: differing copy lengths C={n_c} RUST={n_r}");
    assert_eq!(
        n_c, DATABLOCK_SIZE,
        "row18: sizeof(DataBlock) is {n_c}, expected {DATABLOCK_SIZE}"
    );
    assert!(dc.0[DATABLOCK_SIZE..].iter().all(|&b| b == 0),
        "row18: C wrote past sizeof(DataBlock)");
    assert!(dr.0[DATABLOCK_SIZE..].iter().all(|&b| b == 0),
        "row18: RUST wrote past sizeof(DataBlock)");
}

// ---------------------------------------------------------------------------
// Rows 19-20: handle_pointer_operations
// ---------------------------------------------------------------------------

#[test]
fn row19_hpo_small_values() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 19);
    for v in -1000..=1000 {
        chk_hpo(&p, v, "row19");
    }
    for _ in 0..N {
        chk_hpo(&p, rng.range_i32(-1000, 1000), "row19");
    }
}

#[test]
fn row20_hpo_overflowing_values() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 20);
    for delta in 0..256i64 {
        chk_hpo(&p, (i32::MAX as i64 - delta) as i32, "row20");
        chk_hpo(&p, (i32::MIN as i64 + delta) as i32, "row20");
    }
    for _ in 0..(N * 5) {
        chk_hpo(&p, rng.next_i32(), "row20");
    }
}

// ---------------------------------------------------------------------------
// Rows 21-27: overunder, one row per switch arm reached via `a % 6`
// ---------------------------------------------------------------------------

fn overunder_row_for_residue(residue: i32, row: &str, seed: u64) {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(seed);
    for _ in 0..300 {
        // build a non-negative `a` with the requested residue mod 6
        let k = rng.range_i32(0, 300_000_000);
        let a = k.wrapping_mul(6).wrapping_add(residue).abs();
        let a = a - (a % 6) + residue; // guarantee a % 6 == residue, a >= 0
        assert_eq!(a % 6, residue);
        let b = rng.next_i32();
        let c = rng.next_i32();
        let d = rng.range_i32(-50_000, 50_000);
        chk_overunder(&p, a, b, c, d, row);
    }
    // small, easy-to-read values too
    for m in 0..50 {
        let a = m * 6 + residue;
        chk_overunder(&p, a, m, -m, m + 1, row);
    }
}

#[test]
fn row21_overunder_residue0() {
    overunder_row_for_residue(0, "row21", SEED ^ 21);
}
#[test]
fn row22_overunder_residue1() {
    overunder_row_for_residue(1, "row22", SEED ^ 22);
}
#[test]
fn row23_overunder_residue2() {
    overunder_row_for_residue(2, "row23", SEED ^ 23);
}
#[test]
fn row24_overunder_residue3() {
    overunder_row_for_residue(3, "row24", SEED ^ 24);
}
#[test]
fn row25_overunder_residue4() {
    overunder_row_for_residue(4, "row25", SEED ^ 25);
}
#[test]
fn row26_overunder_residue5() {
    overunder_row_for_residue(5, "row26", SEED ^ 26);
}

#[test]
fn row27_overunder_negative_a_default_arm() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 27);
    for residue in 1..6 {
        for m in 0..40 {
            let a = -(m * 6 + residue);
            assert!(a % 6 != 0 || residue == 0);
            chk_overunder(&p, a, m * 7, -m * 3, m + 2, "row27");
        }
    }
    for _ in 0..600 {
        let a = -(rng.range_i32(1, 300_000_000).abs());
        chk_overunder(&p, a, rng.next_i32(), rng.next_i32(), rng.range_i32(-46000, 46000), "row27");
    }
}

// ---------------------------------------------------------------------------
// Rows 28-30: the sqrt operand -- d*d + a*a with/without int overflow
// ---------------------------------------------------------------------------

fn sq_sum(a: i32, d: i32) -> (i32, bool) {
    let wrapped = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
    let exact = (d as i64) * (d as i64) + (a as i64) * (a as i64);
    (wrapped, exact != wrapped as i64)
}

#[test]
fn row28_overunder_sqrt_no_overflow() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 28);
    let mut n = 0;
    while n < 600 {
        let a = rng.range_i32(-32000, 32000);
        let d = rng.range_i32(-32000, 32000);
        let (v, of) = sq_sum(a, d);
        if of || v < 0 {
            continue;
        }
        chk_overunder(&p, a, rng.next_i32(), rng.next_i32(), d, "row28");
        n += 1;
    }
}

#[test]
fn row29_overunder_sqrt_overflow_positive() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 29);
    // deterministic seeds known to wrap to a positive value
    for (a, d) in [(3, 65536), (0, 65536), (1, 131072), (7, 92682)] {
        let (v, of) = sq_sum(a, d);
        assert!(of && v >= 0, "({a},{d}) -> {v} overflow={of}");
        chk_overunder(&p, a, 11, -13, d, "row29");
    }
    let mut n = 0;
    let mut tries = 0;
    while n < 400 && tries < 400_000 {
        tries += 1;
        let a = rng.next_i32();
        let d = rng.next_i32();
        let (v, of) = sq_sum(a, d);
        if !of || v < 0 {
            continue;
        }
        chk_overunder(&p, a, rng.next_i32(), rng.next_i32(), d, "row29");
        n += 1;
    }
    assert!(n > 0, "row29 found no positive-overflow cases");
}

#[test]
fn row30_overunder_sqrt_overflow_negative_nan() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 30);
    for (a, d) in [(0, 46341), (0, -46341), (46341, 0), (5, 46349), (46341, 1)] {
        let (v, of) = sq_sum(a, d);
        assert!(of && v < 0, "({a},{d}) -> {v} overflow={of}");
        // sqrt of a negative double is NaN -> safe_double_to_int yields 0
        assert!(((v as f64).sqrt()).is_nan());
        chk_overunder(&p, a, 3, -5, d, "row30");
    }
    let mut n = 0;
    let mut tries = 0;
    while n < 400 && tries < 400_000 {
        tries += 1;
        let a = rng.next_i32();
        let d = rng.next_i32();
        let (v, of) = sq_sum(a, d);
        if !of || v >= 0 {
            continue;
        }
        chk_overunder(&p, a, rng.next_i32(), rng.next_i32(), d, "row30");
        n += 1;
    }
    assert!(n > 0, "row30 found no negative-overflow cases");
}

// ---------------------------------------------------------------------------
// Rows 31-34: per-argument magnitude interactions inside overunder
// ---------------------------------------------------------------------------

#[test]
fn row31_overunder_b_saturates_high() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 31);
    // b * 2.7 > INT_MAX  <=>  b > 2147483647 / 2.7 = 795364313.7
    const HI: i32 = 795_364_314;
    for b in [HI, 800_000_000, i32::MAX, i32::MAX - 1, 1_000_000_000] {
        assert!(b as f64 * 2.7 > 2_147_483_647.0, "b={b}");
        chk_overunder(&p, 12, b, 7, 3, "row31");
    }
    for _ in 0..300 {
        let b = rng.range_i32(HI, i32::MAX);
        assert!(b as f64 * 2.7 > 2_147_483_647.0, "b={b}");
        chk_overunder(&p, rng.range_i32(0, 1 << 20), b, rng.next_i32(), rng.range_i32(-1000, 1000), "row31");
    }
}

#[test]
fn row32_overunder_b_saturates_low() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 32);
    // b * 2.7 < INT_MIN  <=>  b < -2147483648 / 2.7 = -795364314.07
    const LO: i32 = -795_364_315;
    for b in [LO, -900_000_000, i32::MIN, i32::MIN + 1] {
        assert!(b as f64 * 2.7 < -2_147_483_648.0, "b={b}");
        chk_overunder(&p, 13, b, -7, -3, "row32");
    }
    for _ in 0..300 {
        let b = rng.range_i32(i32::MIN, LO);
        assert!(b as f64 * 2.7 < -2_147_483_648.0, "b={b}");
        chk_overunder(&p, rng.range_i32(0, 1 << 20), b, rng.next_i32(), rng.range_i32(-1000, 1000), "row32");
    }
}

#[test]
fn row33_overunder_c_extremes() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 33);
    for delta in 0..64i64 {
        chk_overunder(&p, 6, 5, (i32::MAX as i64 - delta) as i32, 4, "row33");
        chk_overunder(&p, 7, 5, (i32::MIN as i64 + delta) as i32, 4, "row33");
    }
    for _ in 0..300 {
        let c = if rng.next_u64() & 1 == 0 {
            (i32::MAX as i64 - (rng.next_u64() % 1000) as i64) as i32
        } else {
            (i32::MIN as i64 + (rng.next_u64() % 1000) as i64) as i32
        };
        chk_overunder(&p, rng.range_i32(0, 1 << 16), rng.next_i32(), c, rng.range_i32(-1000, 1000), "row33");
    }
}

#[test]
fn row34_overunder_a_plus_b_overflow() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 34);
    for _ in 0..400 {
        let a = rng.range_i32(1 << 30, i32::MAX);
        let b = rng.range_i32(1 << 30, i32::MAX);
        assert!(a.checked_add(b).is_none());
        chk_overunder(&p, a, b, rng.next_i32(), rng.range_i32(-40000, 40000), "row34");
    }
    for _ in 0..400 {
        let a = rng.range_i32(i32::MIN, -(1 << 30));
        let b = rng.range_i32(i32::MIN, -(1 << 30));
        assert!(a.checked_add(b).is_none());
        chk_overunder(&p, a, b, rng.next_i32(), rng.range_i32(-40000, 40000), "row34");
    }
}

// ---------------------------------------------------------------------------
// Rows 35-36: corner grid and unpruned random cross-product
// ---------------------------------------------------------------------------

const CORNERS: [i32; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];

#[test]
fn row35_overunder_corner_grid() {
    let p = load_pair();
    let _quiet = silence_stdout();
    for &a in &CORNERS {
        for &b in &CORNERS {
            for &c in &CORNERS {
                for &d in &CORNERS {
                    chk_overunder(&p, a, b, c, d, "row35");
                }
            }
        }
    }
}

#[test]
fn row36_overunder_random_cross_product() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 36);
    for _ in 0..(N * 3) {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let c = rng.next_i32();
        let d = rng.next_i32();
        chk_overunder(&p, a, b, c, d, "row36");
    }
    // mixed magnitude classes
    let classes: [fn(&mut Rng) -> i32; 4] = [
        |r| r.range_i32(-10, 10),
        |r| r.range_i32(-46341, 46341),
        |r| r.range_i32(-1 << 30, 1 << 30),
        |r| r.next_i32(),
    ];
    for ca in 0..4 {
        for cb in 0..4 {
            for cc in 0..4 {
                for cd in 0..4 {
                    for _ in 0..8 {
                        let a = classes[ca](&mut rng);
                        let b = classes[cb](&mut rng);
                        let c = classes[cc](&mut rng);
                        let d = classes[cd](&mut rng);
                        chk_overunder(&p, a, b, c, d, "row36");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 38: re-drive overunder's internal pipeline through the low-level exports
// and cross-check against overunder's return value.
// ---------------------------------------------------------------------------

fn pipeline_expected(api: &Api, a: i32, b: i32, c: i32, d: i32) -> i32 {
    unsafe {
        let temp1 = a as f64 * 1.5;
        let temp2 = b as f64 * 2.7;
        let temp3 = c as f64 / 3.3;
        let temp4 = ((d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a))) as f64).sqrt();
        let conv1 = (api.safe_double_to_int)(temp1);
        let conv2 = (api.safe_double_to_int)(temp2);
        let conv3 = (api.safe_double_to_int)(temp3);
        let conv4 = (api.safe_double_to_int)(temp4);
        let switch_result = (api.process_with_fallthrough)(a % 6, b);
        let ptr_result = (api.handle_pointer_operations)(c);

        // DataBlock round-trip through copy_data_block, exactly as overunder does
        let mut src = Probe::filled(0);
        src.0[0..4].copy_from_slice(&a.to_le_bytes());
        src.0[8..16].copy_from_slice(&temp1.to_bits().to_le_bytes());
        src.0[16..22].copy_from_slice(b"Source");
        let mut dst = Probe::filled(0);
        (api.copy_data_block)(dst.as_mut_ptr(), src.as_ptr());
        let dest_id = i32::from_le_bytes([dst.0[0], dst.0[1], dst.0[2], dst.0[3]]);

        let mut total = conv1
            .wrapping_add(conv2)
            .wrapping_add(conv3)
            .wrapping_add(conv4)
            .wrapping_add(switch_result)
            .wrapping_add(ptr_result);
        total = total.wrapping_add(dest_id);
        for v in [a, b, c, d, a.wrapping_add(b)] {
            total = total.wrapping_add(v);
        }
        total
    }
}

#[test]
fn row38_composed_pipeline_matches_low_level_exports() {
    let p = load_pair();
    let _quiet = silence_stdout();
    let mut rng = Rng::new(SEED ^ 38);
    let mut cases: Vec<(i32, i32, i32, i32)> = Vec::new();
    for _ in 0..1500 {
        cases.push((rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()));
    }
    for &a in &CORNERS {
        for &b in &CORNERS {
            cases.push((a, b, b, a));
            cases.push((a, b, a, b));
        }
    }
    for &(a, b, c, d) in &cases {
        let got = chk_overunder(&p, a, b, c, d, "row38");
        let exp_c = pipeline_expected(&p.c, a, b, c, d);
        let exp_r = pipeline_expected(&p.r, a, b, c, d);
        assert_eq!(
            exp_c, exp_r,
            "row38: low-level pipeline diverges for ({a},{b},{c},{d}): C={exp_c} RUST={exp_r}"
        );
        assert_eq!(
            got, exp_c,
            "row38: overunder({a},{b},{c},{d}) = {got} but composing the low-level \
             exports gives {exp_c}"
        );
    }
}

// keep the c_void import used even if a helper is compiled out
const _: Option<*const c_void> = None;

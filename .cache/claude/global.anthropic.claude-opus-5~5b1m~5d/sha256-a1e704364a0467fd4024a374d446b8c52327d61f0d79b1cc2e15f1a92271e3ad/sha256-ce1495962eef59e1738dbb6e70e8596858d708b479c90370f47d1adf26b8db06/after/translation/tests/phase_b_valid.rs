// Phase B -- valid-path differential tests, one test per CONFIGS.md row.
//
// Every call goes through the exported C ABI of BOTH shared objects; the Rust
// crate is loaded with `libloading`, exactly like an external consumer.
mod common;

use common::*;
use std::ffi::c_char;

const MODES: [&[u8]; 4] = [b"standard", b"enhanced", b"turbo", b"extreme"];

fn nul(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

fn check_classify(row: &str, s: &[u8]) {
    let l = libs();
    let buf = nul(s);
    let p = buf.as_ptr() as *const c_char;
    let (c, rs) = unsafe { ((l.c.classify_mode)(p), (l.rs.classify_mode)(p)) };
    eq_int(row, show(s), c, rs);
}

fn check_apply(row: &str, base: i32, level: i32) {
    let l = libs();
    let (c, rs) = unsafe {
        (
            (l.c.apply_multiplier)(base, level),
            (l.rs.apply_multiplier)(base, level),
        )
    };
    eq_int(row, (base, level), c, rs);
}

fn check_ctf(row: &str, f: f64) {
    let l = libs();
    let (c, rs) = unsafe {
        (
            (l.c.convert_time_factor)(f),
            (l.rs.convert_time_factor)(f),
        )
    };
    eq_int(row, (f, f.to_bits()), c, rs);
}

fn check_cno(row: &str, f: f64) {
    let l = libs();
    let (c, rs) = unsafe {
        (
            (l.c.convert_negative_overflow)(f),
            (l.rs.convert_negative_overflow)(f),
        )
    };
    eq_int(row, (f, f.to_bits()), c, rs);
}

fn check_gmt(row: &str, d: i32, h: i32) {
    let l = libs();
    let (c, rs) = unsafe {
        (
            (l.c.get_modified_time)(d, h),
            (l.rs.get_modified_time)(d, h),
        )
    };
    eq_i64(row, (d, h), c, rs);
}

fn check_hash(row: &str, t: i64) {
    let l = libs();
    let (c, rs) = unsafe { ((l.c.hash_time_value)(t), (l.rs.hash_time_value)(t)) };
    eq_int(row, t, c, rs);
    assert!(c >= 0, "[{row}] hash_time_value({t}) returned negative {c}");
}

/// Return value **and** full stdout must match byte-for-byte.
fn check_modeselect(row: &str, a: i32, b: i32, c: i32, d: i32) {
    let l = libs();
    let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(a, b, c, d) });
    let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(a, b, c, d) });
    eq_int(row, (a, b, c, d), cr, rr);
    eq_bytes(row, (a, b, c, d), &cout, &rout);
    assert!(!cout.is_empty(), "[{row}] C produced no stdout for {:?}", (a, b, c, d));
}

// ===========================================================================
// C1..C4 -- the four recognised mode strings
// ===========================================================================

#[test]
fn c1_classify_standard() {
    check_classify("C1", b"standard");
    let l = libs();
    let buf = nul(b"standard");
    let v = unsafe { (l.rs.classify_mode)(buf.as_ptr() as *const c_char) };
    assert_eq!(v, 0x10, "C1: expected 0x10");
}

#[test]
fn c2_classify_enhanced() {
    check_classify("C2", b"enhanced");
    let l = libs();
    let buf = nul(b"enhanced");
    assert_eq!(
        unsafe { (l.rs.classify_mode)(buf.as_ptr() as *const c_char) },
        0x20
    );
}

#[test]
fn c3_classify_turbo() {
    check_classify("C3", b"turbo");
    let l = libs();
    let buf = nul(b"turbo");
    assert_eq!(
        unsafe { (l.rs.classify_mode)(buf.as_ptr() as *const c_char) },
        0x30
    );
}

#[test]
fn c4_classify_extreme() {
    check_classify("C4", b"extreme");
    let l = libs();
    let buf = nul(b"extreme");
    assert_eq!(
        unsafe { (l.rs.classify_mode)(buf.as_ptr() as *const c_char) },
        0x40
    );
}

// ===========================================================================
// C5..C8 -- randomized / adversarial mode strings
// ===========================================================================

#[test]
fn c5_classify_random_ascii() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..2000 {
        let len = 1 + rng.below(64) as usize;
        let s: Vec<u8> = (0..len).map(|_| 1 + (rng.below(0x7F) as u8)).collect();
        check_classify("C5", &s);
    }
}

#[test]
fn c6_classify_random_full_byte_range() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..2000 {
        let len = 1 + rng.below(64) as usize;
        let s: Vec<u8> = (0..len).map(|_| 1 + (rng.below(0xFF) as u8)).collect();
        check_classify("C6", &s);
    }
}

#[test]
fn c7_classify_single_byte_mutations_of_valid_modes() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..2000 {
        let base = MODES[rng.below(4) as usize].to_vec();
        let mut s = base.clone();
        match rng.below(4) {
            0 => {
                // replace one byte
                let i = rng.below(s.len() as u64) as usize;
                s[i] = 1 + rng.below(0xFF) as u8;
            }
            1 => {
                // insert one byte
                let i = rng.below(s.len() as u64 + 1) as usize;
                s.insert(i, 1 + rng.below(0xFF) as u8);
            }
            2 => {
                // truncate
                let i = rng.below(s.len() as u64) as usize;
                s.truncate(i);
            }
            _ => {
                // extend
                s.push(1 + rng.below(0xFF) as u8);
            }
        }
        check_classify("C7", &s);
    }
    // Deterministic case variants and prefixes as well.
    for s in [
        &b"Standard"[..],
        b"STANDARD",
        b"TURBO",
        b"Turbo",
        b"eXtreme",
        b"stand",
        b"turb",
        b"e",
        b"standardX",
        b"turbo ",
        b" standard",
        b"enhanced\t",
    ] {
        check_classify("C7", s);
    }
}

#[test]
fn c8_classify_long_strings() {
    let mut rng = Rng::new(SEED ^ 8);
    for len in [256usize, 1024, 4096] {
        for _ in 0..64 {
            let s: Vec<u8> = (0..len).map(|_| 1 + (rng.below(0xFF) as u8)).collect();
            check_classify("C8", &s);
        }
        // and a long string that starts with a valid mode
        let mut s = b"standard".to_vec();
        s.resize(len, b'x');
        check_classify("C8", &s);
    }
}

// ===========================================================================
// C9..C15 -- apply_multiplier, every switch fall-through arm
// ===========================================================================

fn apply_row_random(row: &str, level: i32, salt: u64) {
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..2000 {
        check_apply(row, rng.next_i32(), level);
    }
}

#[test]
fn c9_apply_level0() {
    apply_row_random("C9", 0, 9);
}
#[test]
fn c10_apply_level1() {
    apply_row_random("C10", 1, 10);
}
#[test]
fn c11_apply_level2() {
    apply_row_random("C11", 2, 11);
}
#[test]
fn c12_apply_level3() {
    apply_row_random("C12", 3, 12);
}
#[test]
fn c13_apply_level4() {
    apply_row_random("C13", 4, 13);
}

#[test]
fn c14_apply_base_a0_all_levels() {
    // The exact call `modeselect` makes.
    let expect = [0xA5, 0xC1, 0x13F, 0x1EA, 0x2E9];
    for level in 0..=4 {
        check_apply("C14", 0xA0, level);
        let l = libs();
        assert_eq!(
            unsafe { (l.rs.apply_multiplier)(0xA0, level) },
            expect[level as usize],
            "C14: fall-through sum wrong for level {level}"
        );
    }
}

#[test]
fn c15_apply_overflow_corners() {
    let bases = [
        0i32,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 0x300,
        i32::MIN + 0x300,
        i32::MAX - 5,
        i32::MAX - 0x2E9,
        i32::MIN + 0x2E9,
    ];
    for &b in &bases {
        for level in 0..=4 {
            check_apply("C15", b, level);
        }
    }
}

// ===========================================================================
// C16..C19 -- convert_time_factor
// ===========================================================================

#[test]
fn c16_ctf_in_range() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..2000 {
        check_ctf("C16", rng.unit_f64() * 2.147e-3);
    }
}

#[test]
fn c17_ctf_boundary_sweep() {
    for k in [
        i32::MAX as i64,
        i32::MAX as i64 + 1,
        i32::MAX as i64 - 1,
        i32::MIN as i64,
        i32::MIN as i64 + 1,
        i32::MIN as i64 - 1,
        0,
        1,
        -1,
    ] {
        let base = k as f64 / 1e12;
        check_ctf("C17", base);
        // nudge the *input* by a few ULPs in both directions
        let mut up = base;
        let mut down = base;
        for _ in 0..8 {
            up = f64::from_bits(up.to_bits() + 1);
            down = f64::from_bits(down.to_bits().wrapping_sub(1));
            check_ctf("C17", up);
            check_ctf("C17", down);
        }
    }
    // also nudge the *product* boundary directly
    for target in [
        2147483646.0f64,
        2147483647.0,
        2147483647.5,
        2147483648.0,
        -2147483647.0,
        -2147483648.0,
        -2147483648.5,
        -2147483649.0,
    ] {
        check_ctf("C17", target / 1e12);
        check_ctf("C17", f64::from_bits((target / 1e12).to_bits() + 1));
        check_ctf("C17", f64::from_bits((target / 1e12).to_bits() - 1));
    }
}

#[test]
fn c18_ctf_zero_and_subnormal() {
    for f in [
        0.0f64,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        1e-300,
        -1e-300,
        1e-13,
        -1e-13,
        f64::EPSILON,
    ] {
        check_ctf("C18", f);
    }
}

#[test]
fn c19_ctf_random_full_range_finite() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..2000 {
        check_ctf("C19", rng.finite_f64());
    }
    // biased sampling around the interesting magnitude band
    let mut rng = Rng::new(SEED ^ 0x19);
    for _ in 0..2000 {
        let exp = rng.range_i32(-30, 30) as f64;
        let mant = rng.unit_f64();
        check_ctf("C19", mant * 10f64.powf(exp));
    }
}

// ===========================================================================
// C20..C23 -- convert_negative_overflow
// ===========================================================================

#[test]
fn c20_cno_in_range() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..2000 {
        check_cno("C20", rng.unit_f64() * 2.147e-6);
    }
}

#[test]
fn c21_cno_boundary_sweep() {
    for k in [
        i32::MAX as i64,
        i32::MAX as i64 + 1,
        i32::MIN as i64,
        i32::MIN as i64 - 1,
        0,
        1,
        -1,
    ] {
        // product is value * -1e15, so the interesting inputs are k / -1e15
        for base in [k as f64 / -1e15, k as f64 / 1e15] {
            check_cno("C21", base);
            let mut up = base;
            let mut down = base;
            for _ in 0..8 {
                up = f64::from_bits(up.to_bits() + 1);
                down = f64::from_bits(down.to_bits().wrapping_sub(1));
                check_cno("C21", up);
                check_cno("C21", down);
            }
        }
    }
}

#[test]
fn c22_cno_zero_and_subnormal() {
    for f in [
        0.0f64,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        1e-300,
        -1e-300,
        1e-16,
        -1e-16,
    ] {
        check_cno("C22", f);
    }
}

#[test]
fn c23_cno_random_full_range_finite() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..2000 {
        check_cno("C23", rng.finite_f64());
    }
    let mut rng = Rng::new(SEED ^ 0x23);
    for _ in 0..2000 {
        let exp = rng.range_i32(-30, 30) as f64;
        let mant = rng.unit_f64();
        check_cno("C23", mant * 10f64.powf(exp));
    }
}

// ===========================================================================
// C24..C30 -- get_modified_time
// ===========================================================================

#[test]
fn c24_gmt_zero() {
    check_gmt("C24", 0, 0);
}

#[test]
fn c25_gmt_non_overflowing_random() {
    let mut rng = Rng::new(SEED ^ 25);
    let mut n = 0;
    while n < 2000 {
        let d = rng.range_i32(-24855, 24855);
        let h = rng.range_i32(-596523, 596523);
        // keep the products and their sum inside i32
        let pd = (d as i64) * 86400;
        let ph = (h as i64) * 3600;
        if pd.abs() > i32::MAX as i64 || ph.abs() > i32::MAX as i64 {
            continue;
        }
        let sum = pd + ph;
        if sum > i32::MAX as i64 || sum < i32::MIN as i64 {
            continue;
        }
        check_gmt("C25", d, h);
        n += 1;
    }
}

#[test]
fn c26_gmt_negative_only() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut n = 0;
    while n < 2000 {
        let d = -(rng.below(24856) as i32);
        let h = -(rng.below(596524) as i32);
        let sum = (d as i64) * 86400 + (h as i64) * 3600;
        if sum < i32::MIN as i64 {
            continue;
        }
        check_gmt("C26", d, h);
        n += 1;
    }
}

#[test]
fn c27_gmt_days_overflow() {
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..2000 {
        check_gmt("C27", rng.next_i32(), 0);
    }
}

#[test]
fn c28_gmt_hours_overflow() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..2000 {
        check_gmt("C28", 0, rng.next_i32());
    }
}

#[test]
fn c29_gmt_both_random_full_range() {
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..4000 {
        check_gmt("C29", rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn c30_gmt_corners() {
    let vals = [
        0i32,
        1,
        -1,
        24855,
        24856,
        -24855,
        -24856,
        596523,
        596524,
        -596523,
        -596524,
        i32::MAX,
        i32::MIN,
    ];
    for &d in &vals {
        for &h in &vals {
            check_gmt("C30", d, h);
        }
    }
}

// ===========================================================================
// C31..C34 -- hash_time_value
// ===========================================================================

#[test]
fn c31_hash_fixed_values() {
    for t in [
        0i64,
        1,
        -1,
        2,
        -2,
        i64::MIN,
        i64::MAX,
        0x5A5A_5A5A_5A5A_5A5Au64 as i64,
        0x7FFF_FFFF,
        -0x8000_0000,
        3,
    ] {
        check_hash("C31", t);
    }
}

#[test]
fn c32_hash_random_full_range() {
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..4000 {
        check_hash("C32", rng.next_i64());
    }
}

#[test]
fn c33_hash_byte_lane_walk() {
    for nn in [0x01u64, 0x7F, 0x80, 0xFF, 0xAB, 0x5A] {
        for k in 0..8u32 {
            check_hash("C33", (nn << (8 * k)) as i64);
            check_hash("C33", !((nn << (8 * k)) as i64));
        }
    }
}

#[test]
fn c34_hash_plausible_time_t() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let mut rng = Rng::new(SEED ^ 34);
    check_hash("C34", now);
    check_hash("C34", now >> 29);
    for k in 0..8i64 {
        check_hash("C34", k << 29);
        check_hash("C34", (k << 29) - 1);
        check_hash("C34", (k << 29) + 1);
        check_hash("C34", k);
    }
    for _ in 0..1000 {
        check_hash("C34", now + (rng.next_i32() as i64));
        check_hash("C34", (now >> 29) + (rng.range_i32(-100000, 100000) as i64));
    }
}

// ===========================================================================
// C35 / C36 -- composed pipelines through the low-level exports
// ===========================================================================

#[test]
fn c35_gmt_then_hash_composed() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 35);
    for _ in 0..2000 {
        let (d, h) = if rng.below(2) == 0 {
            (rng.range_i32(-24855, 24855), rng.range_i32(-596523, 596523))
        } else {
            (rng.next_i32(), rng.next_i32())
        };
        unsafe {
            let tc = (l.c.get_modified_time)(d, h);
            let tr = (l.rs.get_modified_time)(d, h);
            eq_i64("C35/gmt", (d, h), tc, tr);
            let hc = (l.c.hash_time_value)(tc);
            let hr = (l.rs.hash_time_value)(tr);
            eq_int("C35/hash", (d, h, tc), hc, hr);
        }
    }
}

/// Re-implements `modeselect`'s body from the six low-level exports of `lib` and
/// returns the value the composition produces.
fn compose(lib: &Lib, mode_selector: i32, time_offset: i32, complexity: i32, seed: i32) -> i32 {
    unsafe {
        let mut result: i32 = 0;
        let mode_index = mode_selector % 4;
        assert!(mode_index >= 0, "compose() only models the in-bounds path");
        let buf = nul(MODES[mode_index as usize]);
        let mode_value = (lib.classify_mode)(buf.as_ptr() as *const c_char);
        result = result.wrapping_add(mode_value);

        let complexity_level = complexity % 5;
        let multiplier = (lib.apply_multiplier)(0xA0, complexity_level);
        result = result.wrapping_add(multiplier);

        let modified_time = (lib.get_modified_time)(time_offset, seed % 24);
        let time_hash = (lib.hash_time_value)(modified_time);
        result = result.wrapping_add(time_hash % 0x1000);

        let factor1 = (seed as f64) * 1e8;
        let factor2 = (time_offset as f64) * -1e7;
        let result1 = (lib.convert_time_factor)(factor1);
        let result2 = (lib.convert_negative_overflow)(factor2);

        result ^= result1 & 0xFF;
        result ^= result2 & 0xFF00;
        result.wrapping_mul(0x10).wrapping_add(0xBEEF)
    }
}

#[test]
fn c36_low_level_composition_matches_modeselect() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 36);
    for _ in 0..1500 {
        let mi = rng.below(4) as i32;
        let ms = mi + 4 * rng.range_i32(0, 100_000);
        let cl = rng.below(5) as i32;
        let cx = cl + 5 * rng.range_i32(0, 100_000);
        let to = rng.next_i32();
        let sd = rng.next_i32();

        let comp_c = compose(&l.c, ms, to, cx, sd);
        let comp_rs = compose(&l.rs, ms, to, cx, sd);
        eq_int("C36/compose", (ms, to, cx, sd), comp_c, comp_rs);

        let (real_c, _) = capture(|| unsafe { (l.c.modeselect)(ms, to, cx, sd) });
        let (real_rs, _) = capture(|| unsafe { (l.rs.modeselect)(ms, to, cx, sd) });
        eq_int("C36/modeselect", (ms, to, cx, sd), real_c, real_rs);
        // The hand composition must reproduce the one-shot wrapper of *each* lib.
        eq_int("C36/c-vs-composed", (ms, to, cx, sd), real_c, comp_c);
        eq_int("C36/rs-vs-composed", (ms, to, cx, sd), real_rs, comp_rs);
    }
}

// ===========================================================================
// C37..C56 -- modeselect, full cross product of mode_index x complexity_level
// ===========================================================================

fn modeselect_combo(row: &str, mode_index: i32, complexity_level: i32, salt: u64, iters: usize) {
    let mut rng = Rng::new(SEED ^ salt);
    for i in 0..iters {
        let ms = mode_index + 4 * rng.range_i32(0, 500_000_000);
        let cx = complexity_level + 5 * rng.range_i32(0, 400_000_000);
        assert_eq!(ms % 4, mode_index);
        assert_eq!(cx % 5, complexity_level);
        // mix of small, large and extreme time_offset / seed values
        let (to, sd) = match i % 4 {
            0 => (rng.range_i32(-1000, 1000), rng.range_i32(-1000, 1000)),
            1 => (rng.next_i32(), rng.next_i32()),
            2 => (rng.range_i32(-30000, 30000), rng.range_i32(-100, 100)),
            _ => (rng.next_i32(), rng.range_i32(-24, 24)),
        };
        check_modeselect(row, ms, to, cx, sd);
    }
}

macro_rules! ms_combo_tests {
    ($( $name:ident => ($row:literal, $mi:expr, $cl:expr, $salt:expr) ),* $(,)?) => {
        $(
            #[test]
            fn $name() { modeselect_combo($row, $mi, $cl, $salt, 120); }
        )*
    };
}

ms_combo_tests! {
    c37_modeselect_m0_c0 => ("C37", 0, 0, 37),
    c38_modeselect_m0_c1 => ("C38", 0, 1, 38),
    c39_modeselect_m0_c2 => ("C39", 0, 2, 39),
    c40_modeselect_m0_c3 => ("C40", 0, 3, 40),
    c41_modeselect_m0_c4 => ("C41", 0, 4, 41),
    c42_modeselect_m1_c0 => ("C42", 1, 0, 42),
    c43_modeselect_m1_c1 => ("C43", 1, 1, 43),
    c44_modeselect_m1_c2 => ("C44", 1, 2, 44),
    c45_modeselect_m1_c3 => ("C45", 1, 3, 45),
    c46_modeselect_m1_c4 => ("C46", 1, 4, 46),
    c47_modeselect_m2_c0 => ("C47", 2, 0, 47),
    c48_modeselect_m2_c1 => ("C48", 2, 1, 48),
    c49_modeselect_m2_c2 => ("C49", 2, 2, 49),
    c50_modeselect_m2_c3 => ("C50", 2, 3, 50),
    c51_modeselect_m2_c4 => ("C51", 2, 4, 51),
    c52_modeselect_m3_c0 => ("C52", 3, 0, 52),
    c53_modeselect_m3_c1 => ("C53", 3, 1, 53),
    c54_modeselect_m3_c2 => ("C54", 3, 2, 54),
    c55_modeselect_m3_c3 => ("C55", 3, 3, 55),
    c56_modeselect_m3_c4 => ("C56", 3, 4, 56),
}

// ===========================================================================
// C57..C63 -- modeselect input shapes
// ===========================================================================

#[test]
fn c57_modeselect_seed_zero() {
    let mut rng = Rng::new(SEED ^ 57);
    for mi in 0..4 {
        for _ in 0..40 {
            let ms = mi + 4 * rng.range_i32(0, 1_000_000);
            let cx = rng.range_i32(0, 1_000_000);
            let to = rng.next_i32();
            check_modeselect("C57", ms, to, cx, 0);
        }
    }
}

#[test]
fn c58_modeselect_time_offset_zero() {
    let mut rng = Rng::new(SEED ^ 58);
    for mi in 0..4 {
        for _ in 0..40 {
            let ms = mi + 4 * rng.range_i32(0, 1_000_000);
            let cx = rng.range_i32(0, 1_000_000);
            let sd = rng.next_i32();
            check_modeselect("C58", ms, 0, cx, sd);
        }
    }
}

#[test]
fn c59_modeselect_both_conversions_in_range() {
    for mi in 0..4 {
        for cl in 0..5 {
            check_modeselect("C59", mi, 0, cl, 0);
        }
    }
}

#[test]
fn c60_modeselect_negative_seed() {
    let mut rng = Rng::new(SEED ^ 60);
    for _ in 0..400 {
        let mi = rng.below(4) as i32;
        let ms = mi + 4 * rng.range_i32(0, 1_000_000);
        let cx = rng.range_i32(0, 1_000_000);
        let to = rng.range_i32(-100_000, 100_000);
        let sd = -1 - (rng.below(1_000_000) as i32);
        check_modeselect("C60", ms, to, cx, sd);
    }
}

#[test]
fn c61_modeselect_time_offset_overflows_days_product() {
    let mut rng = Rng::new(SEED ^ 61);
    for _ in 0..400 {
        let mi = rng.below(4) as i32;
        let ms = mi + 4 * rng.range_i32(0, 1_000_000);
        let cx = rng.range_i32(0, 1_000_000);
        let mut to = rng.next_i32();
        if to.unsigned_abs() <= 24855 {
            to = 24856 + (rng.below(1_000_000) as i32);
            if rng.below(2) == 0 {
                to = -to;
            }
        }
        let sd = rng.next_i32();
        check_modeselect("C61", ms, to, cx, sd);
    }
}

#[test]
fn c62_modeselect_all_args_random() {
    let mut rng = Rng::new(SEED ^ 62);
    for _ in 0..2500 {
        let mut ms = rng.next_i32();
        if ms < 0 && ms % 4 != 0 {
            ms = -ms; // keep the index in bounds; negative indices are Phase C
        }
        let to = rng.next_i32();
        let cx = rng.next_i32();
        let sd = rng.next_i32();
        // negative complexity is also a valid input; apply_multiplier's default
        // arm handles it (see E26) and both libraries must agree.
        check_modeselect("C62", ms, to, cx, sd);
    }
}

#[test]
fn c63_modeselect_corner_cross_product() {
    let vals = [0i32, 1, -1, 4, -4, 5, 24856, i32::MAX, i32::MIN];
    let mut count = 0usize;
    for &a in &vals {
        if a < 0 && a % 4 != 0 {
            continue; // Phase C (SIGSEGV)
        }
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    check_modeselect("C63", a, b, c, d);
                    count += 1;
                }
            }
        }
    }
    assert!(count > 3000, "C63 only ran {count} combinations");
}

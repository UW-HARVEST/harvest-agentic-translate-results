// Phase B — valid-path differential tests for the LEAF entry points.
//
// Rows C1..C34 and C48..C50 of CONFIGS.md. Every call goes through `dlsym` on
// both the C `.so` and the Rust `.so`.
mod common;

use common::*;
use std::ffi::c_char;

const N: usize = 4000;

// ---------------------------------------------------------------------------
// classify_mode — C1..C8
// ---------------------------------------------------------------------------

fn cm(row: &str, bytes: &[u8]) {
    let p = pair();
    let buf = cstr(bytes);
    let ptr = buf.as_ptr() as *const c_char;
    // SAFETY: `ptr` is a NUL-terminated buffer alive for both calls.
    let (c, r) = unsafe { ((p.c.classify_mode)(ptr), (p.rs.classify_mode)(ptr)) };
    eq_int(row, format!("classify_mode({:?})", String::from_utf8_lossy(bytes)), c, r);
}

#[test]
fn c1_c4_classify_mode_exact_literals() {
    cm("C1", b"standard");
    cm("C2", b"enhanced");
    cm("C3", b"turbo");
    cm("C4", b"extreme");

    // Also pin the absolute values so a "both wrong the same way" pass is ruled out.
    let p = pair();
    for (s, want) in [
        (&b"standard"[..], 0x10),
        (&b"enhanced"[..], 0x20),
        (&b"turbo"[..], 0x30),
        (&b"extreme"[..], 0x40),
    ] {
        let buf = cstr(s);
        // SAFETY: NUL-terminated buffer.
        let got = unsafe { (p.c.classify_mode)(buf.as_ptr() as *const c_char) };
        assert_eq!(got, want, "C ground truth changed for {:?}", String::from_utf8_lossy(s));
    }
}

#[test]
fn c5_classify_mode_random_ascii() {
    let mut rng = Rng::new();
    for _ in 0..N {
        let len = rng.below(17) as usize;
        let s: Vec<u8> = (0..len).map(|_| rng.range_i32(0x20, 0x7E) as u8).collect();
        cm("C5", &s);
    }
}

#[test]
fn c6_classify_mode_random_full_bytes() {
    let mut rng = Rng::with_seed(SEED ^ 0xC6);
    for _ in 0..N {
        let len = rng.below(17) as usize;
        // 0x01..=0xFF: 0x00 would just terminate the string early, which C7/E6 covers.
        let s: Vec<u8> = (0..len).map(|_| rng.range_i32(1, 255) as u8).collect();
        cm("C6", &s);
    }
}

#[test]
fn c7_classify_mode_single_byte_mutations() {
    let mut rng = Rng::with_seed(SEED ^ 0xC7);
    let lits: [&[u8]; 4] = [b"standard", b"enhanced", b"turbo", b"extreme"];
    for _ in 0..N {
        let lit = lits[rng.below(4) as usize];
        let mut s = lit.to_vec();
        match rng.below(3) {
            0 => {
                // flip one byte
                let i = rng.below(s.len() as u64) as usize;
                s[i] = rng.range_i32(1, 255) as u8;
            }
            1 => {
                // drop one byte
                let i = rng.below(s.len() as u64) as usize;
                s.remove(i);
            }
            _ => {
                // append one byte
                s.push(rng.range_i32(1, 255) as u8);
            }
        }
        cm("C7", &s);
    }
}

#[test]
fn c8_classify_mode_long_prefix_strings() {
    let mut rng = Rng::with_seed(SEED ^ 0xC8);
    let lits: [&[u8]; 4] = [b"standard", b"enhanced", b"turbo", b"extreme"];
    for _ in 0..600 {
        let lit = lits[rng.below(4) as usize];
        let keep = rng.below(lit.len() as u64 + 1) as usize;
        let total = 1 + rng.below(4096) as usize;
        let mut s = lit[..keep].to_vec();
        while s.len() < total {
            s.push(rng.range_i32(1, 255) as u8);
        }
        cm("C8", &s);
    }
}

// ---------------------------------------------------------------------------
// apply_multiplier — C9..C15
// ---------------------------------------------------------------------------

fn am(row: &str, base: i32, level: i32) {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe { ((p.c.apply_multiplier)(base, level), (p.rs.apply_multiplier)(base, level)) };
    eq_int(row, format!("apply_multiplier(base={base}, level={level})"), c, r);
}

#[test]
fn c9_c13_apply_multiplier_each_valid_level() {
    let rows = ["C13", "C12", "C11", "C10", "C9"]; // level 0..4
    for level in 0..=4i32 {
        let mut rng = Rng::with_seed(SEED ^ (0xA0 + level as u64));
        am(rows[level as usize], 0xA0, level); // the base modeselect actually uses
        for _ in 0..N {
            am(rows[level as usize], rng.next_i32(), level);
        }
    }
}

#[test]
fn c14_apply_multiplier_random_level_and_base() {
    let mut rng = Rng::with_seed(SEED ^ 0xC14);
    for _ in 0..N {
        // Bias so the valid 0..4 window is hit often, not just `default`.
        let level = match rng.below(3) {
            0 => rng.range_i32(-3, 8),
            1 => rng.range_i32(-1000, 1000),
            _ => rng.next_i32(),
        };
        am("C14", rng.next_i32(), level);
    }
}

#[test]
fn c15_apply_multiplier_boundary_cross_product() {
    let bases = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 0, -1, 0xA0];
    for &base in &bases {
        for level in -2..=6i32 {
            am("C15", base, level);
        }
    }
}

// ---------------------------------------------------------------------------
// convert_time_factor — C16..C20
// ---------------------------------------------------------------------------

fn ctf(row: &str, x: f64) {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe { ((p.c.convert_time_factor)(x), (p.rs.convert_time_factor)(x)) };
    eq_int(row, format!("convert_time_factor({x:?} bits=0x{:016X})", x.to_bits()), c, r);
}

#[test]
fn c16_convert_time_factor_in_range() {
    let mut rng = Rng::with_seed(SEED ^ 0xC16);
    for _ in 0..N {
        // |x| < 2.147e-3 keeps x*1e12 inside int range.
        ctf("C16", rng.unit_f64() * 2.147e-3);
    }
}

#[test]
fn c17_convert_time_factor_unit_range() {
    let mut rng = Rng::with_seed(SEED ^ 0xC17);
    for _ in 0..N {
        ctf("C17", rng.unit_f64());
    }
}

#[test]
fn c18_convert_time_factor_exponent_ladder() {
    let mut rng = Rng::with_seed(SEED ^ 0xC18);
    for _ in 0..N {
        ctf("C18", rng.ladder_f64());
    }
}

#[test]
fn c19_convert_time_factor_arbitrary_bits() {
    let mut rng = Rng::with_seed(SEED ^ 0xC19);
    for _ in 0..N {
        ctf("C19", rng.bits_f64());
    }
}

#[test]
fn c20_convert_time_factor_boundaries() {
    // factor * 1e12 lands exactly on / just past the int boundary.
    for &v in &[
        0.0f64,
        -0.0,
        1e-12,
        -1e-12,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        2147483647.0 / 1e12,
        2147483648.0 / 1e12,
        2147483649.0 / 1e12,
        -2147483648.0 / 1e12,
        -2147483649.0 / 1e12,
        -2147483647.0 / 1e12,
        1.0,
        -1.0,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
    ] {
        ctf("C20", v);
    }
    // Dense sweep across the boundary in ULP-ish steps.
    for k in -40i64..=40 {
        ctf("C20", (2147483647.0 + k as f64) / 1e12);
        ctf("C20", (-2147483648.0 + k as f64) / 1e12);
    }
}

// ---------------------------------------------------------------------------
// convert_negative_overflow — C21..C25
// ---------------------------------------------------------------------------

fn cno(row: &str, x: f64) {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe {
        (
            (p.c.convert_negative_overflow)(x),
            (p.rs.convert_negative_overflow)(x),
        )
    };
    eq_int(
        row,
        format!("convert_negative_overflow({x:?} bits=0x{:016X})", x.to_bits()),
        c,
        r,
    );
}

#[test]
fn c21_convert_negative_overflow_in_range() {
    let mut rng = Rng::with_seed(SEED ^ 0xC21);
    for _ in 0..N {
        // |x| < 2.147e-6 keeps x*-1e15 inside int range.
        cno("C21", rng.unit_f64() * 2.147e-6);
    }
}

#[test]
fn c22_convert_negative_overflow_unit_range() {
    let mut rng = Rng::with_seed(SEED ^ 0xC22);
    for _ in 0..N {
        cno("C22", rng.unit_f64());
    }
}

#[test]
fn c23_convert_negative_overflow_exponent_ladder() {
    let mut rng = Rng::with_seed(SEED ^ 0xC23);
    for _ in 0..N {
        cno("C23", rng.ladder_f64());
    }
}

#[test]
fn c24_convert_negative_overflow_arbitrary_bits() {
    let mut rng = Rng::with_seed(SEED ^ 0xC24);
    for _ in 0..N {
        cno("C24", rng.bits_f64());
    }
}

#[test]
fn c25_convert_negative_overflow_boundaries() {
    for &v in &[
        0.0f64,
        -0.0,
        1e-15,
        -1e-15,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        2147483647.0 / -1e15,
        2147483648.0 / -1e15,
        2147483649.0 / -1e15,
        -2147483648.0 / -1e15,
        -2147483649.0 / -1e15,
        1.0,
        -1.0,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
    ] {
        cno("C25", v);
    }
    for k in -40i64..=40 {
        cno("C25", (2147483647.0 + k as f64) / -1e15);
        cno("C25", (-2147483648.0 + k as f64) / -1e15);
    }
}

// ---------------------------------------------------------------------------
// hash_time_value — C26..C29
// ---------------------------------------------------------------------------

fn htv(row: &str, t: TimeT) {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe { ((p.c.hash_time_value)(t), (p.rs.hash_time_value)(t)) };
    eq_int(row, format!("hash_time_value({t} / 0x{t:016X})"), c, r);
}

#[test]
fn c26_hash_time_value_extremes() {
    for &t in &[0i64, -1, i64::MIN, i64::MAX, 1, -2, 0x7F7F_7F7F_7F7F_7F7F, 0x8080_8080_8080_8080u64 as i64] {
        htv("C26", t);
    }
}

#[test]
fn c27_hash_time_value_random_full_range() {
    let mut rng = Rng::with_seed(SEED ^ 0xC27);
    for _ in 0..N * 4 {
        htv("C27", rng.next_i64());
    }
}

#[test]
fn c28_hash_time_value_realistic_shape() {
    let mut rng = Rng::with_seed(SEED ^ 0xC28);
    for _ in 0..N {
        // time_t >> 29 shape: small non-negative, plus small offsets.
        htv("C28", rng.below(64) as i64);
        htv("C28", rng.range_i32(-100000, 100000) as i64);
    }
}

#[test]
fn c29_hash_time_value_single_bits() {
    for k in 0..64u32 {
        htv("C29", 1i64.wrapping_shl(k));
        htv("C29", !(1i64.wrapping_shl(k)));
    }
    // Every single byte set to every value, at every byte position.
    for pos in 0..8u32 {
        for b in 0..=255u64 {
            htv("C29", ((b << (pos * 8)) as u64) as i64);
        }
    }
}

// ---------------------------------------------------------------------------
// get_modified_time — C30..C34
// ---------------------------------------------------------------------------
//
// `get_modified_time` calls `time(NULL)` then `>> 29`, so the clock-derived part
// only changes once every 2^29 s (~17 years). Both `.so`s read the clock
// microseconds apart, so the shifted value is identical. To make that assumption
// explicit rather than implicit, `c30_..` asserts the shifted clock is stable
// across the whole test.

fn gmt(row: &str, days: i32, hours: i32) {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let (c, r) = unsafe {
        (
            (p.c.get_modified_time)(days, hours),
            (p.rs.get_modified_time)(days, hours),
        )
    };
    eq_time(row, format!("get_modified_time(days={days}, hours={hours})"), c, r);
}

#[test]
fn c30_get_modified_time_zero_and_clock_stability() {
    let p = pair();
    // SAFETY: plain scalar C ABI call.
    let base = unsafe { (p.c.get_modified_time)(0, 0) };
    gmt("C30", 0, 0);
    // The shifted clock must not tick during the run, else every row below is
    // racy. 2^29 s granularity makes this safe, but assert it anyway.
    for _ in 0..50 {
        // SAFETY: plain scalar C ABI call.
        let again = unsafe { (p.c.get_modified_time)(0, 0) };
        assert_eq!(base, again, "time(NULL) >> 29 ticked mid-test; rerun");
    }
}

#[test]
fn c31_get_modified_time_small_positive() {
    let mut rng = Rng::with_seed(SEED ^ 0xC31);
    for _ in 0..N {
        gmt("C31", rng.range_i32(0, 1000), rng.range_i32(0, 23));
    }
}

#[test]
fn c32_get_modified_time_small_negative() {
    let mut rng = Rng::with_seed(SEED ^ 0xC32);
    for _ in 0..N {
        gmt("C32", rng.range_i32(-1000, 0), rng.range_i32(-23, 0));
    }
}

#[test]
fn c33_get_modified_time_random_full_range() {
    let mut rng = Rng::with_seed(SEED ^ 0xC33);
    for _ in 0..N * 2 {
        gmt("C33", rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn c34_get_modified_time_boundary_cross_product() {
    // +-24855 is roughly where days*86400 crosses INT_MAX.
    let vals = [
        i32::MIN,
        i32::MIN + 1,
        -596523,
        -24856,
        -24855,
        -1,
        0,
        1,
        24855,
        24856,
        596523,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &d in &vals {
        for &h in &vals {
            gmt("C34", d, h);
        }
    }
}

// ---------------------------------------------------------------------------
// composed pipelines — C48..C50
// ---------------------------------------------------------------------------

#[test]
fn c48_pipeline_get_modified_time_then_hash() {
    // Exactly the chain at lib.c:114-115.
    let p = pair();
    let mut rng = Rng::with_seed(SEED ^ 0xC48);
    for _ in 0..N {
        let time_offset = rng.next_i32();
        let seed = rng.next_i32();
        let hours = seed % 24;
        // SAFETY: plain scalar C ABI calls.
        unsafe {
            let tc = (p.c.get_modified_time)(time_offset, hours);
            let tr = (p.rs.get_modified_time)(time_offset, hours);
            eq_time("C48", format!("gmt({time_offset},{hours})"), tc, tr);
            let hc = (p.c.hash_time_value)(tc);
            let hr = (p.rs.hash_time_value)(tr);
            eq_int("C48", format!("hash(gmt({time_offset},{hours}))"), hc, hr);
        }
    }
}

#[test]
fn c49_pipeline_classify_mode_over_mode_table() {
    // The exact 4-element table modeselect builds at lib.c:99.
    for s in [&b"standard"[..], b"enhanced", b"turbo", b"extreme"] {
        cm("C49", s);
    }
    // And what the Rust substitutes for the out-of-range index (ERRORS.md E29).
    cm("C49", b"");
}

#[test]
fn c50_pipeline_double_factors_as_modeselect_builds_them() {
    // lib.c:120-121: factor1 = (double)seed * 1e8, factor2 = (double)time_offset * -1e7
    let mut rng = Rng::with_seed(SEED ^ 0xC50);
    let mut cases: Vec<(i32, i32)> = vec![
        (0, 0),
        (1, 1),
        (-1, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (21, 214),
        (-21, -214),
    ];
    for _ in 0..N {
        cases.push((rng.next_i32(), rng.next_i32()));
    }
    for (seed, time_offset) in cases {
        let f1 = (seed as f64) * 1e8;
        let f2 = (time_offset as f64) * -1e7;
        ctf("C50", f1);
        cno("C50", f2);
    }
}

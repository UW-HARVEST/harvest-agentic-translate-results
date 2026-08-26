// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every row runs many randomised inputs with a
// fixed seed, and compares BOTH the return value AND the bytes written to
// stdout by the C and the Rust shared library.

mod common;

use common::{Api, Rng, assert_same, assert_same_io, both};
use std::ffi::CString;

// ===========================================================================
// classify_mode — rows 1..10
// ===========================================================================

fn cm(ctx: &str, s: &[u8]) {
    let cs = CString::new(s.to_vec()).unwrap();
    let p = cs.as_ptr();
    assert_same(&format!("classify_mode({ctx})"), |a: &Api| unsafe {
        (a.classify_mode)(p)
    });
}

/// Also check the exact expected constant so the test is not vacuous.
fn cm_eq(s: &[u8], expect: i32) {
    let cs = CString::new(s.to_vec()).unwrap();
    let (c, r) = both();
    let cv = unsafe { (c.classify_mode)(cs.as_ptr()) };
    let rv = unsafe { (r.classify_mode)(cs.as_ptr()) };
    assert_eq!(cv, expect, "C classify_mode({s:?}) unexpected");
    assert_eq!(rv, cv, "rust classify_mode({s:?}) diverges");
}

#[test]
fn row01_classify_mode_standard() {
    cm("standard", b"standard");
    cm_eq(b"standard", 0x10);
}

#[test]
fn row02_classify_mode_enhanced() {
    cm("enhanced", b"enhanced");
    cm_eq(b"enhanced", 0x20);
}

#[test]
fn row03_classify_mode_turbo() {
    cm("turbo", b"turbo");
    cm_eq(b"turbo", 0x30);
}

#[test]
fn row04_classify_mode_extreme() {
    cm("extreme", b"extreme");
    cm_eq(b"extreme", 0x40);
}

#[test]
fn row05_classify_mode_empty() {
    cm("empty", b"");
    cm_eq(b"", 0x00);
}

#[test]
fn row06_classify_mode_prefixes() {
    for lit in [
        &b"standard"[..],
        &b"enhanced"[..],
        &b"turbo"[..],
        &b"extreme"[..],
    ] {
        for n in 0..lit.len() {
            cm("prefix", &lit[..n]);
            cm_eq(&lit[..n], 0x00);
        }
    }
}

#[test]
fn row07_classify_mode_superstrings() {
    for lit in [
        &b"standard"[..],
        &b"enhanced"[..],
        &b"turbo"[..],
        &b"extreme"[..],
    ] {
        for extra in [&b"X"[..], &b" "[..], &b"\t"[..], &b"standard"[..], &b"\x7f"[..]] {
            let mut v = lit.to_vec();
            v.extend_from_slice(extra);
            cm("superstring", &v);
            cm_eq(&v, 0x00);
        }
    }
}

#[test]
fn row08_classify_mode_case_variants() {
    for s in [
        &b"STANDARD"[..],
        &b"Standard"[..],
        &b"ENHANCED"[..],
        &b"Turbo"[..],
        &b"tURBO"[..],
        &b"EXTREME"[..],
        &b"Extreme"[..],
    ] {
        cm("case", s);
        cm_eq(s, 0x00);
    }
}

#[test]
fn row09_classify_mode_random_strings() {
    let mut rng = Rng::new();
    for _ in 0..2000 {
        let len = (rng.below(32) + 1) as usize;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            // any non-NUL byte
            v.push((rng.below(255) + 1) as u8);
        }
        cm("random", &v);
    }
}

#[test]
fn row10_classify_mode_embedded_literal_after_nul() {
    // CString forbids interior NULs, so build the buffer by hand: the C code
    // must stop at the first NUL and therefore never see the literal.
    let (c, r) = both();
    for lit in [
        &b"standard"[..],
        &b"enhanced"[..],
        &b"turbo"[..],
        &b"extreme"[..],
    ] {
        for head in [&b""[..], &b"x"[..], &b"stand"[..]] {
            let mut buf: Vec<u8> = Vec::new();
            buf.extend_from_slice(head);
            buf.push(0);
            buf.extend_from_slice(lit);
            buf.push(0);
            let p = buf.as_ptr() as *const std::ffi::c_char;
            let cv = unsafe { (c.classify_mode)(p) };
            let rv = unsafe { (r.classify_mode)(p) };
            assert_eq!(cv, rv, "classify_mode(embedded {buf:?})");
        }
    }
}

// ===========================================================================
// apply_multiplier — rows 11..16
// ===========================================================================

fn am(base: i32, level: i32) {
    assert_same(&format!("apply_multiplier({base},{level})"), |a: &Api| unsafe {
        (a.apply_multiplier)(base, level)
    });
}

fn bases(rng: &mut Rng) -> Vec<i32> {
    let mut v = vec![
        0xA0,
        0,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MAX - 0x400,
        i32::MIN + 0x400,
        0xDEAD,
        -0xDEAD,
    ];
    for _ in 0..200 {
        v.push(rng.next_i32());
    }
    v
}

#[test]
fn row11_apply_multiplier_level0() {
    let mut rng = Rng::with_seed(11);
    for b in bases(&mut rng) {
        am(b, 0);
    }
}

#[test]
fn row12_apply_multiplier_level1() {
    let mut rng = Rng::with_seed(12);
    for b in bases(&mut rng) {
        am(b, 1);
    }
}

#[test]
fn row13_apply_multiplier_level2() {
    let mut rng = Rng::with_seed(13);
    for b in bases(&mut rng) {
        am(b, 2);
    }
}

#[test]
fn row14_apply_multiplier_level3() {
    let mut rng = Rng::with_seed(14);
    for b in bases(&mut rng) {
        am(b, 3);
    }
}

#[test]
fn row15_apply_multiplier_level4() {
    let mut rng = Rng::with_seed(15);
    for b in bases(&mut rng) {
        am(b, 4);
    }
}

#[test]
fn row15b_apply_multiplier_fallthrough_constants() {
    // Pin the exact fall-through sums the C switch produces from base 0xA0.
    let (c, _r) = both();
    let expect = [
        0xA0 + 0x05,
        0xA0 + 0x1C + 0x05,
        0xA0 + 0x7E + 0x1C + 0x05,
        0xA0 + 0xAB + 0x7E + 0x1C + 0x05,
        0xA0 + 0xFF + 0xAB + 0x7E + 0x1C + 0x05,
    ];
    for (lvl, e) in expect.iter().enumerate() {
        let got = unsafe { (c.apply_multiplier)(0xA0, lvl as i32) };
        assert_eq!(got, *e, "C fall-through sum changed for level {lvl}");
        am(0xA0, lvl as i32);
    }
}

#[test]
fn row16_apply_multiplier_random_levels() {
    let mut rng = Rng::with_seed(16);
    for lvl in [-1i32, 5, 6, 100, -100, i32::MIN, i32::MAX, 0x10, -0x10] {
        for b in [0xA0, 0, i32::MAX, i32::MIN, 1, -1] {
            am(b, lvl);
        }
    }
    for _ in 0..3000 {
        am(rng.next_i32(), rng.next_i32());
    }
    // biased towards the interesting 0..=4 window plus one step past it
    for _ in 0..2000 {
        let lvl = (rng.below(11) as i32) - 3;
        am(rng.next_i32(), lvl);
    }
}

// ===========================================================================
// convert_time_factor / convert_negative_overflow — rows 17..28
// ===========================================================================

fn ctf(v: f64) {
    assert_same(&format!("convert_time_factor({v:e}/{:#x})", v.to_bits()), |a: &Api| unsafe {
        (a.convert_time_factor)(v)
    });
}

fn cno(v: f64) {
    assert_same(
        &format!("convert_negative_overflow({v:e}/{:#x})", v.to_bits()),
        |a: &Api| unsafe { (a.convert_negative_overflow)(v) },
    );
}

/// Values one ULP either side of `x`, plus `x` itself.
fn ulp_neighbourhood(x: f64) -> Vec<f64> {
    let mut out = vec![x];
    let b = x.to_bits();
    for d in 1..=4i64 {
        out.push(f64::from_bits(b.wrapping_add(d as u64)));
        out.push(f64::from_bits(b.wrapping_sub(d as u64)));
    }
    out
}

#[test]
fn row17_convert_time_factor_in_range() {
    let mut rng = Rng::with_seed(17);
    for _ in 0..3000 {
        // |factor| < 2^31 / 1e12 ~= 2.147e-3
        ctf(rng.scaled_f64(-3) * 2.0);
    }
    for k in -2147..=2147i32 {
        ctf(k as f64 * 1e-3);
    }
}

#[test]
fn row18_convert_time_factor_boundary() {
    let pos = 2147483648.0f64 / 1e12;
    let neg = -2147483648.0f64 / 1e12;
    for x in ulp_neighbourhood(pos)
        .into_iter()
        .chain(ulp_neighbourhood(neg))
        .chain(ulp_neighbourhood(2147483647.0 / 1e12))
        .chain(ulp_neighbourhood(-2147483647.0 / 1e12))
    {
        ctf(x);
    }
    // exact integers around the boundary, reached through the multiply
    for k in [
        2147483646i64,
        2147483647,
        2147483648,
        2147483649,
        -2147483647,
        -2147483648,
        -2147483649,
        -2147483650,
    ] {
        ctf(k as f64 / 1e12);
    }
}

#[test]
fn row19_convert_time_factor_zero_and_subnormal() {
    for x in [
        0.0f64,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        1e-320,
        -1e-320,
        5e-324,
        -5e-324,
    ] {
        ctf(x);
    }
}

#[test]
fn row20_convert_time_factor_overflowing() {
    let mut rng = Rng::with_seed(20);
    for exp in -3..=300i32 {
        for _ in 0..8 {
            ctf(rng.scaled_f64(exp));
        }
    }
    for x in [1.0f64, -1.0, 1e100, -1e100, f64::MAX, f64::MIN, 1e308, -1e308] {
        ctf(x);
    }
}

#[test]
fn row21_convert_time_factor_nan_inf() {
    for x in [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF8_0000_0000_0000),
    ] {
        ctf(x);
    }
}

#[test]
fn row22_convert_time_factor_random_bits() {
    let mut rng = Rng::with_seed(22);
    for _ in 0..5000 {
        ctf(rng.finite_f64());
    }
    for _ in 0..1000 {
        ctf(f64::from_bits(rng.next_u64())); // includes NaN / inf patterns
    }
}

#[test]
fn row23_convert_negative_overflow_in_range() {
    let mut rng = Rng::with_seed(23);
    for _ in 0..3000 {
        // |value| < 2^31 / 1e15 ~= 2.147e-6
        cno(rng.scaled_f64(-6) * 2.0);
    }
    for k in -2147..=2147i32 {
        cno(k as f64 * 1e-6);
    }
}

#[test]
fn row24_convert_negative_overflow_boundary() {
    let pos = 2147483648.0f64 / 1e15;
    let neg = -2147483648.0f64 / 1e15;
    for x in ulp_neighbourhood(pos)
        .into_iter()
        .chain(ulp_neighbourhood(neg))
        .chain(ulp_neighbourhood(2147483647.0 / 1e15))
        .chain(ulp_neighbourhood(-2147483647.0 / 1e15))
    {
        cno(x);
    }
    for k in [
        2147483646i64,
        2147483647,
        2147483648,
        2147483649,
        -2147483647,
        -2147483648,
        -2147483649,
        -2147483650,
    ] {
        cno(-(k as f64) / 1e15);
    }
}

#[test]
fn row25_convert_negative_overflow_zero_and_subnormal() {
    for x in [
        0.0f64,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        1e-320,
        -1e-320,
        5e-324,
        -5e-324,
    ] {
        cno(x);
    }
}

#[test]
fn row26_convert_negative_overflow_overflowing() {
    let mut rng = Rng::with_seed(26);
    for exp in -6..=300i32 {
        for _ in 0..8 {
            cno(rng.scaled_f64(exp));
        }
    }
    for x in [1.0f64, -1.0, 1e100, -1e100, f64::MAX, f64::MIN, 1e308, -1e308] {
        cno(x);
    }
}

#[test]
fn row27_convert_negative_overflow_nan_inf() {
    for x in [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7FF0_0000_0000_0001),
        f64::from_bits(0xFFF8_0000_0000_0000),
    ] {
        cno(x);
    }
}

#[test]
fn row28_convert_negative_overflow_random_bits() {
    let mut rng = Rng::with_seed(28);
    for _ in 0..5000 {
        cno(rng.finite_f64());
    }
    for _ in 0..1000 {
        cno(f64::from_bits(rng.next_u64()));
    }
}

// ===========================================================================
// get_modified_time — rows 29..36
// ===========================================================================

fn gmt(d: i32, h: i32) {
    assert_same(&format!("get_modified_time({d},{h})"), |a: &Api| unsafe {
        (a.get_modified_time)(d, h)
    });
}

#[test]
fn row29_get_modified_time_zero() {
    gmt(0, 0);
}

#[test]
fn row30_get_modified_time_small() {
    for d in -50..=50i32 {
        for h in [-23i32, -1, 0, 1, 12, 23] {
            gmt(d, h);
        }
    }
}

#[test]
fn row31_get_modified_time_seed_mod24_range() {
    let mut rng = Rng::with_seed(31);
    for h in -23..=23i32 {
        for _ in 0..20 {
            gmt(rng.next_i32(), h);
        }
    }
}

#[test]
fn row32_get_modified_time_days_overflow() {
    // 86400 * d overflows int for |d| > 24855
    for d in [
        24855i32, 24856, 25000, 100_000, -24855, -24856, -100_000, i32::MAX, i32::MIN,
        i32::MAX / 86400,
        i32::MAX / 86400 + 1,
        i32::MIN / 86400,
        i32::MIN / 86400 - 1,
    ] {
        for h in [0i32, 1, -1, 23, -23] {
            gmt(d, h);
        }
    }
}

#[test]
fn row33_get_modified_time_hours_overflow() {
    for h in [
        596523i32, 596524, 1_000_000, -596523, -596524, -1_000_000, i32::MAX, i32::MIN,
        i32::MAX / 3600,
        i32::MAX / 3600 + 1,
        i32::MIN / 3600,
        i32::MIN / 3600 - 1,
    ] {
        for d in [0i32, 1, -1, 100, -100] {
            gmt(d, h);
        }
    }
}

#[test]
fn row34_get_modified_time_sum_overflow() {
    // both products in range, sum overflows
    for (d, h) in [
        (24000i32, 500_000i32),
        (-24000, -500_000),
        (24855, 596523),
        (-24855, -596523),
        (24000, 596000),
        (-24000, -596000),
    ] {
        gmt(d, h);
    }
}

#[test]
fn row35_get_modified_time_extremes() {
    let xs = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for d in xs {
        for h in xs {
            gmt(d, h);
        }
    }
}

#[test]
fn row36_get_modified_time_random() {
    let mut rng = Rng::with_seed(36);
    for _ in 0..3000 {
        gmt(rng.next_i32(), rng.next_i32());
    }
}

// ===========================================================================
// hash_time_value — rows 37..43
// ===========================================================================

fn htv(t: i64) {
    assert_same(&format!("hash_time_value({t})"), |a: &Api| unsafe {
        (a.hash_time_value)(t)
    });
}

#[test]
fn row37_hash_time_value_zero() {
    htv(0);
}

#[test]
fn row38_hash_time_value_small_positive() {
    for t in 0..=256i64 {
        htv(t);
    }
    for t in [1i64 << 8, 1 << 16, 1 << 24, 1 << 32, 1 << 40, 1 << 48, 1 << 56] {
        htv(t);
    }
}

#[test]
fn row39_hash_time_value_negative() {
    for t in -256..=0i64 {
        htv(t);
    }
}

#[test]
fn row40_hash_time_value_extremes() {
    for t in [
        i64::MIN,
        i64::MIN + 1,
        i64::MAX,
        i64::MAX - 1,
        i32::MIN as i64,
        i32::MAX as i64,
        1i64 << 29,
        1i64 << 31,
        1i64 << 62,
        -1i64 << 62,
        0x5A5A_5A5A_5A5A_5A5Au64 as i64,
        0xFFFF_FFFF_FFFF_FFFFu64 as i64,
        0x8080_8080_8080_8080u64 as i64,
    ] {
        htv(t);
    }
}

#[test]
fn row41_hash_time_value_high_bit_bytes() {
    let mut rng = Rng::with_seed(41);
    for _ in 0..2000 {
        let mut bytes = [0u8; 8];
        for b in bytes.iter_mut() {
            *b = 0x80 | (rng.below(0x80) as u8);
        }
        htv(i64::from_ne_bytes(bytes));
    }
    // one byte >= 0x80 at each position, rest zero
    for i in 0..8 {
        for v in [0x80u8, 0xFF, 0xC3] {
            let mut bytes = [0u8; 8];
            bytes[i] = v;
            htv(i64::from_ne_bytes(bytes));
        }
    }
}

#[test]
fn row42_hash_time_value_random() {
    let mut rng = Rng::with_seed(42);
    for _ in 0..5000 {
        htv(rng.next_i64());
    }
}

#[test]
fn row43_hash_time_value_composed_with_get_modified_time() {
    // pipeline composition: feed real get_modified_time outputs back in
    let (c, r) = both();
    let mut rng = Rng::with_seed(43);
    for _ in 0..500 {
        let d = rng.next_i32();
        let h = (rng.next_i32() % 24).clamp(-23, 23);
        let t_c = unsafe { (c.get_modified_time)(d, h) };
        let t_r = unsafe { (r.get_modified_time)(d, h) };
        assert_eq!(t_c, t_r, "get_modified_time({d},{h})");
        let hc = unsafe { (c.hash_time_value)(t_c) };
        let hr = unsafe { (r.hash_time_value)(t_r) };
        assert_eq!(hc, hr, "hash_time_value({t_c}) from ({d},{h})");
        assert!(hc >= 0, "hash must be masked non-negative");
    }
}

// ===========================================================================
// modeselect — rows 44..74
// ===========================================================================

fn ms(m: i32, t: i32, cx: i32, s: i32) {
    assert_same_io(
        &format!("modeselect({m},{t},{cx},{s})"),
        |a: &Api| unsafe { (a.modeselect)(m, t, cx, s) },
    );
}

/// `mode_selector` with `mode_selector % 4 == idx` (idx in 0..4), non-negative.
fn sel_for(rng: &mut Rng, idx: i32) -> i32 {
    let k = (rng.next_u32() % 0x1000_0000) as i32; // 0 .. 2^28
    k * 4 + idx
}

/// `complexity` with `complexity % 5 == lvl` (lvl in 0..5), non-negative.
fn cx_for(rng: &mut Rng, lvl: i32) -> i32 {
    let k = (rng.next_u32() % 0x1000_0000) as i32; // 5 * 2^28 < i32::MAX
    k * 5 + lvl
}

fn ms_grid(idx: i32, lvl: i32, seed: u64) {
    let mut rng = Rng::with_seed(seed);
    // the canonical smallest representative first
    ms(idx, 0, lvl, 0);
    for _ in 0..60 {
        let m = sel_for(&mut rng, idx);
        let cx = cx_for(&mut rng, lvl);
        assert_eq!(m % 4, idx);
        assert_eq!(cx % 5, lvl);
        ms(m, rng.next_i32(), cx, rng.next_i32());
    }
}

macro_rules! grid_test {
    ($name:ident, $idx:expr, $lvl:expr, $seed:expr) => {
        #[test]
        fn $name() {
            ms_grid($idx, $lvl, $seed);
        }
    };
}

grid_test!(row44_modeselect_m0_l0, 0, 0, 4400);
grid_test!(row45_modeselect_m0_l1, 0, 1, 4500);
grid_test!(row46_modeselect_m0_l2, 0, 2, 4600);
grid_test!(row47_modeselect_m0_l3, 0, 3, 4700);
grid_test!(row48_modeselect_m0_l4, 0, 4, 4800);
grid_test!(row49_modeselect_m1_l0, 1, 0, 4900);
grid_test!(row50_modeselect_m1_l1, 1, 1, 5000);
grid_test!(row51_modeselect_m1_l2, 1, 2, 5100);
grid_test!(row52_modeselect_m1_l3, 1, 3, 5200);
grid_test!(row53_modeselect_m1_l4, 1, 4, 5300);
grid_test!(row54_modeselect_m2_l0, 2, 0, 5400);
grid_test!(row55_modeselect_m2_l1, 2, 1, 5500);
grid_test!(row56_modeselect_m2_l2, 2, 2, 5600);
grid_test!(row57_modeselect_m2_l3, 2, 3, 5700);
grid_test!(row58_modeselect_m2_l4, 2, 4, 5800);
grid_test!(row59_modeselect_m3_l0, 3, 0, 5900);
grid_test!(row60_modeselect_m3_l1, 3, 1, 6000);
grid_test!(row61_modeselect_m3_l2, 3, 2, 6100);
grid_test!(row62_modeselect_m3_l3, 3, 3, 6200);
grid_test!(row63_modeselect_m3_l4, 3, 4, 6300);

#[test]
fn row64_modeselect_seed_zero() {
    let mut rng = Rng::with_seed(64);
    for idx in 0..4 {
        for _ in 0..20 {
            let m = sel_for(&mut rng, idx);
            ms(m, rng.next_i32(), rng.next_i32().abs(), 0);
        }
    }
}

#[test]
fn row65_modeselect_time_offset_zero() {
    let mut rng = Rng::with_seed(65);
    for idx in 0..4 {
        for _ in 0..20 {
            let m = sel_for(&mut rng, idx);
            ms(m, 0, rng.next_i32().abs(), rng.next_i32());
        }
    }
}

#[test]
fn row66_modeselect_seed_and_offset_zero() {
    for idx in 0..4 {
        for lvl in 0..5 {
            ms(idx, 0, lvl, 0);
        }
    }
}

#[test]
fn row67_modeselect_negative_seed() {
    let mut rng = Rng::with_seed(67);
    for s in [-1i32, -12, -23, -24, -25, -47, -48, i32::MIN, i32::MIN + 1] {
        for idx in 0..4 {
            ms(idx, rng.next_i32(), rng.next_i32().abs(), s);
        }
    }
}

#[test]
fn row68_modeselect_seed_multiple_of_24() {
    let mut rng = Rng::with_seed(68);
    for k in 0..40i32 {
        let s = k * 24;
        ms(sel_for(&mut rng, k % 4), rng.next_i32(), (k * 5) % 5, s);
    }
}

#[test]
fn row69_modeselect_time_offset_overflowing_days() {
    let mut rng = Rng::with_seed(69);
    for t in [
        24855i32, 24856, 100_000, 1_000_000, -24855, -24856, -100_000, -1_000_000,
        i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
    ] {
        for idx in 0..4 {
            ms(idx, t, rng.next_i32().abs(), rng.next_i32());
        }
    }
}

#[test]
fn row70_modeselect_negative_complexity() {
    let mut rng = Rng::with_seed(70);
    for cx in [-1i32, -2, -3, -4, -5, -6, -10, -100, i32::MIN, i32::MIN + 1] {
        for idx in 0..4 {
            ms(idx, rng.next_i32(), cx, rng.next_i32());
        }
    }
}

#[test]
fn row71_modeselect_extreme_cross_product() {
    // ERRORS.md row 19: mode_selector % 4 < 0 faults, so keep the selector in a
    // set whose remainder is >= 0.
    let sels = [0i32, 1, 2, 3, 4, i32::MIN, i32::MAX - 3, -4, -8];
    let others = [i32::MIN, -1, 0, 1, i32::MAX];
    for m in sels {
        assert!(m % 4 >= 0, "selector {m} would fault");
        for t in others {
            for cx in others {
                for s in others {
                    ms(m, t, cx, s);
                }
            }
        }
    }
}

#[test]
fn row72_modeselect_random_nonnegative_selector() {
    let mut rng = Rng::with_seed(72);
    for _ in 0..1200 {
        let m = (rng.next_u32() & 0x7FFF_FFFF) as i32;
        ms(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn row73_modeselect_negative_multiple_of_four_selector() {
    let mut rng = Rng::with_seed(73);
    for k in 1..=40i32 {
        let m = -4 * k;
        assert_eq!(m % 4, 0);
        ms(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    ms(i32::MIN, 0, 0, 0); // INT_MIN % 4 == 0
}

#[test]
fn row74_modeselect_stdout_format_bytes() {
    // Explicitly assert the captured stdout is non-empty and identical, and
    // that it contains the printf-formatted pieces (%.2e, %ld, %X).
    let mut rng = Rng::with_seed(74);
    let (c, r) = both();
    for _ in 0..200 {
        let m = (rng.next_u32() & 0x7FFF_FFFF) as i32;
        let t = rng.next_i32();
        let cx = rng.next_i32();
        let s = rng.next_i32();
        let (cv, cout) = common::capture_stdout(|| unsafe { (c.modeselect)(m, t, cx, s) });
        let (rv, rout) = common::capture_stdout(|| unsafe { (r.modeselect)(m, t, cx, s) });
        assert_eq!(cv, rv, "modeselect({m},{t},{cx},{s})");
        assert_eq!(
            cout,
            rout,
            "stdout for modeselect({m},{t},{cx},{s})\n C   : {}\n RUST: {}",
            common::show(&cout),
            common::show(&rout)
        );
        assert!(!cout.is_empty(), "C produced no stdout");
        let s_out = String::from_utf8_lossy(&cout);
        assert!(s_out.contains("Selected mode: "), "{s_out}");
        assert!(s_out.contains("Complexity level: "), "{s_out}");
        assert!(s_out.contains("Modified time: "), "{s_out}");
        assert!(s_out.contains("e+") || s_out.contains("e-"), "{s_out}");
        assert!(s_out.contains("Final result: "), "{s_out}");
    }
}

// ===========================================================================
// I/O silence of the low-level entry points (justifies the value-only
// comparison used above) + shared-object symbol availability.
// ===========================================================================

#[test]
fn silent_functions_produce_no_output() {
    let (c, r) = both();
    let mut rng = Rng::with_seed(999);
    for _ in 0..200 {
        let lit = CString::new("standard").unwrap();
        let d = rng.next_i32();
        let h = rng.next_i32();
        let f = rng.finite_f64();
        let t = rng.next_i64();
        let base = rng.next_i32();
        let lvl = (rng.below(9) as i32) - 2;

        for api in [&c, &r] {
            let (_, out) = common::capture_stdout(|| unsafe {
                (api.classify_mode)(lit.as_ptr());
                (api.apply_multiplier)(base, lvl);
                (api.convert_time_factor)(f);
                (api.convert_negative_overflow)(f);
                (api.get_modified_time)(d, h);
                (api.hash_time_value)(t);
            });
            assert!(
                out.is_empty(),
                "low-level function wrote to stdout: {}",
                common::show(&out)
            );
        }
    }
}

#[test]
fn all_c_symbols_resolve_in_both_libraries() {
    // Loading both `Api` structs already dlsym()s every symbol from
    // SYMBOLS.md; this test makes the requirement explicit and fails loudly
    // if either library drops one.
    let _ = both();
    for name in common::EXPORTED_SYMBOLS {
        assert!(!name.is_empty());
    }
}

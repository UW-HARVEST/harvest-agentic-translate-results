//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through `dlsym`'d exports
//! of the two shared objects; the Rust crate is never linked directly.
//!
//! The `doubleneg` rows (25-32) live in `tests/doubleneg.rs` because they need
//! process-global fd-1 redirection and must not race with anything else.

mod common;

use std::ffi::c_int;

use common::assert_f64_bits_eq;
use common::both;
use common::Guarded;
use common::Rng;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Interesting `i32` values every integer axis is swept over.
const INT_EDGES: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    i32::MIN + 2,
    -1_000_000,
    -65_537,
    -65_536,
    -257,
    -256,
    -255,
    -128,
    -127,
    -10,
    -9,
    -2,
    -1,
    0,
    1,
    2,
    9,
    10,
    42,
    100,
    127,
    128,
    255,
    256,
    257,
    65_536,
    65_537,
    1_000_000,
    i32::MAX - 2,
    i32::MAX - 1,
    i32::MAX,
];

// ===========================================================================
// process_negation
// ===========================================================================

fn check_negation(v: i32) {
    let (c, r) = both();
    let a = unsafe { (c.process_negation)(v) };
    let b = unsafe { (r.process_negation)(v) };
    assert_eq!(a, b, "process_negation({v}): C {a} vs Rust {b}");
}

/// CONFIGS row 1.
#[test]
fn cfg_01_negation_fixed() {
    for &v in INT_EDGES {
        check_negation(v);
    }
}

/// CONFIGS row 2.
#[test]
fn cfg_02_negation_random() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..20_000 {
        // Mix in plenty of small / masked values so the zero case is hit often.
        let v = match i % 4 {
            0 => rng.next_i32(),
            1 => rng.next_i32() & 0xFF,
            2 => rng.next_i32() & 0x1,
            _ => rng.next_i32() >> (rng.below(32) as u32),
        };
        check_negation(v);
    }
}

// ===========================================================================
// convert_double_to_int
// ===========================================================================

fn check_cvt(v: f64) {
    let (c, r) = both();
    let a = unsafe { (c.convert_double_to_int)(v) };
    let b = unsafe { (r.convert_double_to_int)(v) };
    assert_eq!(
        a, b,
        "convert_double_to_int({v:?} / bits {:#018x}): C {a} vs Rust {b}",
        v.to_bits()
    );
}

/// CONFIGS row 3.
#[test]
fn cfg_03_cvt_zero_subnormal() {
    let vals = [
        0.0_f64,
        -0.0_f64,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        1e-300,
        -1e-300,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
    ];
    for v in vals {
        check_cvt(v);
    }
}

/// CONFIGS row 4.
#[test]
fn cfg_04_cvt_fractional() {
    let mut vals = vec![
        0.5, -0.5, 0.25, -0.25, 0.75, -0.75, 1.5, -1.5, 2.9999, -2.9999, 0.9999999999, -0.9999999999,
        1e9 + 0.5,
        -(1e9 + 0.5),
        123_456.789,
        -123_456.789,
    ];
    // A dense fractional sweep around every small integer, both signs.
    for k in -20..=20 {
        for frac in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 0.999_999] {
            vals.push(k as f64 + frac);
            vals.push(k as f64 - frac);
        }
    }
    for v in vals {
        check_cvt(v);
    }
}

/// CONFIGS row 5.
#[test]
fn cfg_05_cvt_all_int_values() {
    for &v in INT_EDGES {
        check_cvt(v as f64);
        check_cvt(v as f64 + 0.5);
        check_cvt(v as f64 - 0.5);
    }
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..20_000 {
        let v = rng.next_i32();
        check_cvt(v as f64);
    }
}

/// CONFIGS row 6.
#[test]
fn cfg_06_cvt_boundary_sweep() {
    let anchors = [
        2_147_483_647.0_f64,
        -2_147_483_648.0_f64,
        2_147_483_648.0_f64,
        -2_147_483_649.0_f64,
        4_294_967_296.0_f64,
        -4_294_967_296.0_f64,
    ];
    for a in anchors {
        for d in [
            -2.0, -1.5, -1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0,
        ] {
            check_cvt(a + d);
        }
        // Also the neighbouring representable doubles.
        check_cvt(f64::from_bits(a.to_bits().wrapping_sub(1)));
        check_cvt(f64::from_bits(a.to_bits().wrapping_add(1)));
    }
}

/// CONFIGS row 7.
#[test]
fn cfg_07_cvt_random_bits() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..50_000 {
        check_cvt(rng.next_f64_bits());
    }
}

/// CONFIGS row 8.
#[test]
fn cfg_08_cvt_random_scaled() {
    let mut rng = Rng::new(SEED ^ 8);
    for scale in [
        1.0,
        2.0,
        100.0,
        1e6,
        2_147_483_646.0,
        2_147_483_648.0,
        2_147_483_650.0,
        1.0995e12, // 2^40
        1e300,
    ] {
        for _ in 0..5_000 {
            check_cvt(rng.next_f64_scaled(scale));
        }
    }
}

// ===========================================================================
// calculate_with_doubles
// ===========================================================================

fn check_calc(a: i32, b: i32, c: i32) {
    let (cl, rl) = both();
    let x = unsafe { (cl.calculate_with_doubles)(a, b, c) };
    let y = unsafe { (rl.calculate_with_doubles)(a, b, c) };
    assert_f64_bits_eq(x, y, format!("calculate_with_doubles({a},{b},{c})"));
}

const SIGN_PAIRS: &[(i32, i32)] = &[
    (7, 3),
    (7, -3),
    (-7, 3),
    (-7, -3),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
    (0, 5),
    (0, -5),
    (i32::MAX, 3),
    (i32::MIN, 3),
    (i32::MAX, -3),
    (i32::MIN, -3),
    (3, i32::MAX),
    (3, i32::MIN),
    (i32::MAX, i32::MAX),
    (i32::MIN, i32::MIN),
    (i32::MIN, -1),
    (i32::MIN, 1),
];

/// CONFIGS row 9.
#[test]
fn cfg_09_calc_exponent_zero() {
    for &(a, b) in SIGN_PAIRS {
        for c in [0, 10, -10, 20, -20, 100, -100] {
            check_calc(a, b, c);
        }
    }
}

/// CONFIGS row 10.
#[test]
fn cfg_10_calc_positive_exponents() {
    for &(a, b) in SIGN_PAIRS {
        for c in 1..=9 {
            check_calc(a, b, c);
            check_calc(a, b, c + 10);
            check_calc(a, b, c + 1_000_000);
        }
    }
}

/// CONFIGS row 11.
#[test]
fn cfg_11_calc_negative_exponents() {
    for &(a, b) in SIGN_PAIRS {
        for c in -9..=-1 {
            check_calc(a, b, c);
            check_calc(a, b, c - 10);
            check_calc(a, b, c - 1_000_000);
        }
    }
}

/// CONFIGS row 12 — the `b == 0` guard, across every exponent.
#[test]
fn cfg_12_calc_zero_divisor_all_exponents() {
    for c in -25..=25 {
        for a in [0, 1, -1, 42, -42, i32::MIN, i32::MAX] {
            check_calc(a, 0, c);
        }
    }
    check_calc(0, 0, i32::MIN);
    check_calc(0, 0, i32::MAX);
}

/// CONFIGS row 13 — sign of zero must survive bit-exactly.
#[test]
fn cfg_13_calc_signed_zero() {
    for b in [1, -1, 7, -7, i32::MAX, i32::MIN] {
        for c in -12..=12 {
            check_calc(0, b, c);
        }
    }
}

/// CONFIGS row 14.
#[test]
fn cfg_14_calc_extremes_cross() {
    let vals = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let cs = [i32::MIN, i32::MIN + 1, -10, -9, -1, 0, 1, 9, 10, i32::MAX - 1, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &cs {
                check_calc(a, b, c);
            }
        }
    }
}

/// CONFIGS row 15.
#[test]
fn cfg_15_calc_random() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..30_000 {
        let a = rng.next_i32();
        // Every 8th iteration forces the `b == 0` branch.
        let b = if i % 8 == 0 { 0 } else { rng.next_i32() };
        let c = rng.next_i32();
        check_calc(a, b, c);
    }
}

// ===========================================================================
// create_numeric_buffer
// ===========================================================================

/// Runs both implementations into separate canary-guarded buffers and compares
/// the written bytes byte-for-byte.
fn check_create(size: c_int, seed: c_int) {
    let (cl, rl) = both();
    let len = if size > 0 { size as usize } else { 0 };

    let mut cb = Guarded::new(len);
    let mut rb = Guarded::new(len);

    unsafe {
        (cl.create_numeric_buffer)(cb.ptr(), size, seed);
        (rl.create_numeric_buffer)(rb.ptr(), size, seed);
    }

    let ctx = format!("create_numeric_buffer(size={size}, seed={seed})");
    cb.check_canaries(format!("{ctx} [C]"));
    rb.check_canaries(format!("{ctx} [Rust]"));

    if cb.body() != rb.body() {
        let idx = cb
            .body()
            .iter()
            .zip(rb.body())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "{ctx}: first difference at index {idx}: C {} vs Rust {}",
            cb.body()[idx],
            rb.body()[idx]
        );
    }
}

/// CONFIGS row 16.
#[test]
fn cfg_16_create_size_one() {
    check_create(1, 0);
    check_create(1, 1);
    check_create(1, -1);
    check_create(1, i32::MAX);
    check_create(1, i32::MIN);
}

/// CONFIGS row 17.
#[test]
fn cfg_17_create_size_seed_cross() {
    let sizes: &[c_int] = &[1, 2, 7, 8, 36, 37, 63, 64, 127, 128, 255, 256, 257, 1024];
    let seeds: &[c_int] = &[
        0,
        1,
        7,
        42,
        100,
        127,
        128,
        255,
        256,
        257,
        -1,
        -7,
        -42,
        -127,
        -128,
        -255,
        -256,
        -257,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
    ];
    for &size in sizes {
        for &seed in seeds {
            check_create(size, seed);
        }
    }
}

/// CONFIGS row 18.
#[test]
fn cfg_18_create_random() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..5_000 {
        let size = rng.below(2_049) as c_int;
        let seed = rng.next_i32();
        check_create(size, seed);
    }
}

/// CONFIGS row 19 — `seed + i*7` overflows `int` part-way through the loop.
#[test]
fn cfg_19_create_overflow_midloop() {
    for seed in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 3,
        i32::MAX - 6,
        i32::MAX - 7,
        i32::MAX - 700,
        i32::MAX - 7 * 100,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
    ] {
        for size in [1, 2, 7, 8, 256, 300, 1024] {
            check_create(size, seed);
        }
    }
}

// ===========================================================================
// find_value_in_buffer
// ===========================================================================

fn check_find(content: &[i8], size: usize, needle: c_int) {
    let (cl, rl) = both();
    let mut buf = Guarded::new(content.len());
    buf.set_body(content);

    let a = unsafe { (cl.find_value_in_buffer)(buf.const_ptr(), size, needle) };
    let b = unsafe { (rl.find_value_in_buffer)(buf.const_ptr(), size, needle) };
    let ctx = format!(
        "find_value_in_buffer(len={}, size={size}, needle={needle})",
        content.len()
    );
    buf.check_canaries(&ctx);
    assert_eq!(a, b, "{ctx}: C {a} vs Rust {b}");
}

/// CONFIGS row 20 — hit at index 0.
#[test]
fn cfg_20_find_hit_first() {
    for size in [1usize, 2, 7, 256] {
        let mut content = vec![0x11i8; size];
        content[0] = 0x42;
        check_find(&content, size, 0x42);
        // Duplicate the needle later: first occurrence must still win.
        if size > 3 {
            content[size - 1] = 0x42;
            check_find(&content, size, 0x42);
        }
    }
}

/// CONFIGS row 21 — hit position × size cross-product.
#[test]
fn cfg_21_find_position_size_cross() {
    for size in [1usize, 2, 7, 255, 256, 257, 4096] {
        // Miss.
        let content = vec![0x01i8; size];
        check_find(&content, size, 0x42);

        for pos in [0usize, size / 3, size / 2, size.saturating_sub(2), size - 1] {
            let mut content = vec![0x01i8; size];
            content[pos] = 0x42;
            check_find(&content, size, 0x42);
            // Also search a truncated range that excludes / just includes `pos`.
            check_find(&content, pos, 0x42);
            check_find(&content, pos + 1, 0x42);
        }
    }
}

/// CONFIGS row 22 — needle-width truncation against a buffer of all 256 bytes.
#[test]
fn cfg_22_find_needle_width_sweep() {
    let all: Vec<i8> = (0..256).map(|b| b as u8 as i8).collect();
    let needles: &[c_int] = &[
        0, 1, 42, 100, 127, 128, 200, 255, 256, 257, 300, 511, 512, 65_535, 65_536, -1, -2, -127,
        -128, -129, -255, -256, -300, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1,
    ];
    for &n in needles {
        check_find(&all, 256, n);
        check_find(&all, 128, n);
        check_find(&all, 1, n);
    }
    // Reversed, so the needle's index differs.
    let rev: Vec<i8> = all.iter().rev().copied().collect();
    for &n in needles {
        check_find(&rev, 256, n);
    }
    // Every possible single byte, exhaustively, as an in-range needle.
    for b in 0..256i32 {
        check_find(&all, 256, b);
    }
}

/// CONFIGS row 23.
#[test]
fn cfg_23_find_random() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..5_000 {
        let len = rng.below(1_025) as usize;
        let mut content = vec![0i8; len];
        rng.fill_bytes(&mut content);
        let size = if len == 0 { 0 } else { rng.below(len as u64 + 1) as usize };
        let needle = match rng.below(3) {
            // Bias toward needles that are actually present.
            0 if len > 0 => content[rng.below(len as u64) as usize] as c_int,
            1 => rng.next_i32() & 0xFF,
            _ => rng.next_i32(),
        };
        check_find(&content, size, needle);
    }
}

/// CONFIGS row 24 — the composed pipeline `doubleneg` performs, driven through
/// the low-level exports: generate with `create_numeric_buffer`, then search.
#[test]
fn cfg_24_find_over_generated_buffer() {
    let (cl, rl) = both();
    let mut rng = Rng::new(SEED ^ 24);

    for iter in 0..600 {
        let size: c_int = match iter % 5 {
            0 => 256,
            1 => 1,
            2 => 255,
            3 => 257,
            _ => rng.below(1_025) as c_int,
        };
        let seed = if iter % 3 == 0 {
            (iter as i32) - 300
        } else {
            rng.next_i32()
        };

        let len = if size > 0 { size as usize } else { 0 };
        let mut cb = Guarded::new(len);
        let mut rb = Guarded::new(len);
        unsafe {
            (cl.create_numeric_buffer)(cb.ptr(), size, seed);
            (rl.create_numeric_buffer)(rb.ptr(), size, seed);
        }
        let ctx = format!("generated(size={size}, seed={seed})");
        cb.check_canaries(format!("{ctx} [C gen]"));
        rb.check_canaries(format!("{ctx} [Rust gen]"));
        assert_eq!(cb.body(), rb.body(), "{ctx}: generated buffers differ");

        // Search each library's own buffer with its own finder (full pipeline),
        // and cross-search to isolate finder-vs-generator divergence.
        let needles: [c_int; 8] = [
            42,
            100,
            0,
            255,
            -1,
            seed % 256,
            rng.next_i32(),
            rng.next_i32() & 0xFF,
        ];
        for n in needles {
            let a = unsafe { (cl.find_value_in_buffer)(cb.const_ptr(), len, n) };
            let b = unsafe { (rl.find_value_in_buffer)(rb.const_ptr(), len, n) };
            assert_eq!(a, b, "{ctx} pipeline needle={n}: C {a} vs Rust {b}");

            let cross_a = unsafe { (cl.find_value_in_buffer)(rb.const_ptr(), len, n) };
            let cross_b = unsafe { (rl.find_value_in_buffer)(cb.const_ptr(), len, n) };
            assert_eq!(cross_a, cross_b, "{ctx} crossed needle={n}");
            assert_eq!(a, cross_a, "{ctx} crossed vs direct needle={n}");
        }
    }
}

/// CONFIGS row 34 — large buffers, so the generate/search loops run long enough
/// to cross glibc `memchr`'s SIMD blocking and to make `i * 7` grow large.
///
/// (The `i * 7` product itself only overflows `int` for `i > 306783378`, i.e. a
/// buffer above ~307 MB; that is deliberately not allocated here. The wrapping is
/// identical in both languages -- C `-O0` emits a plain `imul` and the
/// translation uses `wrapping_mul` -- and the *sum* `seed + i*7` overflowing is
/// covered by row 19 / `err_11`, which reach the same wrapped code path with a
/// large `seed` instead of a huge `size`.)
#[test]
fn cfg_34_large_buffers() {
    let (cl, rl) = both();

    for &size in &[65_536_i32, 1_048_576, 8_388_608] {
        for &seed in &[0_i32, -1, 12_345, i32::MAX, i32::MIN] {
            let len = size as usize;
            let mut cb = Guarded::new(len);
            let mut rb = Guarded::new(len);
            unsafe {
                (cl.create_numeric_buffer)(cb.ptr(), size, seed);
                (rl.create_numeric_buffer)(rb.ptr(), size, seed);
            }
            let ctx = format!("large create(size={size}, seed={seed})");
            cb.check_canaries(format!("{ctx} [C]"));
            rb.check_canaries(format!("{ctx} [Rust]"));
            assert_eq!(cb.body(), rb.body(), "{ctx}");

            // Search the large buffer: hits (every byte value is present), a
            // deliberate miss is impossible here, so also search sub-ranges.
            for needle in [0, 42, 100, 255, -1, 256, i32::MIN, i32::MAX] {
                let a = unsafe { (cl.find_value_in_buffer)(cb.const_ptr(), len, needle) };
                let b = unsafe { (rl.find_value_in_buffer)(rb.const_ptr(), len, needle) };
                assert_eq!(a, b, "{ctx} find(needle={needle})");
            }
            for sub in [1usize, 3, 15, 31, 63, 127, len / 2, len - 1] {
                let a = unsafe { (cl.find_value_in_buffer)(cb.const_ptr(), sub, 42) };
                let b = unsafe { (rl.find_value_in_buffer)(rb.const_ptr(), sub, 42) };
                assert_eq!(a, b, "{ctx} find(size={sub}, needle=42)");
            }
        }
    }
}

/// CONFIGS row 33 — interleaved script across all six exports, so any hidden
/// per-library state would show up as a divergence.
#[test]
fn cfg_33_interleaved_all_exports() {
    let (cl, rl) = both();
    let mut rng = Rng::new(SEED ^ 33);

    let mut c_acc: i64 = 0;
    let mut r_acc: i64 = 0;

    for _ in 0..20_000 {
        match rng.below(5) {
            0 => {
                let v = rng.next_i32();
                c_acc += unsafe { (cl.process_negation)(v) } as i64;
                r_acc += unsafe { (rl.process_negation)(v) } as i64;
            }
            1 => {
                let v = rng.next_f64_bits();
                c_acc += unsafe { (cl.convert_double_to_int)(v) } as i64;
                r_acc += unsafe { (rl.convert_double_to_int)(v) } as i64;
            }
            2 => {
                let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
                let x = unsafe { (cl.calculate_with_doubles)(a, b, c) };
                let y = unsafe { (rl.calculate_with_doubles)(a, b, c) };
                assert_f64_bits_eq(x, y, format!("interleaved calc({a},{b},{c})"));
                c_acc = c_acc.wrapping_add(x.to_bits() as i64);
                r_acc = r_acc.wrapping_add(y.to_bits() as i64);
            }
            3 => {
                let size = rng.below(300) as c_int;
                let seed = rng.next_i32();
                let len = size.max(0) as usize;
                let mut cb = Guarded::new(len);
                let mut rb = Guarded::new(len);
                unsafe {
                    (cl.create_numeric_buffer)(cb.ptr(), size, seed);
                    (rl.create_numeric_buffer)(rb.ptr(), size, seed);
                }
                assert_eq!(cb.body(), rb.body(), "interleaved create({size},{seed})");
            }
            _ => {
                let len = rng.below(300) as usize;
                let mut content = vec![0i8; len];
                rng.fill_bytes(&mut content);
                let mut buf = Guarded::new(len);
                buf.set_body(&content);
                let n = rng.next_i32();
                let a = unsafe { (cl.find_value_in_buffer)(buf.const_ptr(), len, n) };
                let b = unsafe { (rl.find_value_in_buffer)(buf.const_ptr(), len, n) };
                assert_eq!(a, b, "interleaved find(len={len}, needle={n})");
                c_acc += a as i64;
                r_acc += b as i64;
            }
        }
    }

    assert_eq!(c_acc, r_acc, "interleaved accumulators diverged");
}

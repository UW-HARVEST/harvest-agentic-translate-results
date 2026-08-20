// Phase B — valid-path differential tests for the five low-level entry points.
// One test per row of CONFIGS.md (rows 1..30, 39, 40).  Every row uses many
// randomized inputs from a fixed-seed SplitMix64 generator.
//
// All calls go through `libloading`-resolved symbols of the two `.so` files.

mod common;

use common::{assert_bytes_eq, assert_f64_bits_eq, assert_i32_eq, c, rs, Rng};
use std::ffi::c_char;

const N: usize = 4000;

// ===========================================================================
// convert_double_to_int  (rows 1..5)
// ===========================================================================

fn cmp_convert(v: f64) {
    let cv = unsafe { (c().convert_double_to_int)(v) };
    let rv = unsafe { (rs().convert_double_to_int)(v) };
    assert_i32_eq(
        cv,
        rv,
        &format!("convert_double_to_int({v:?} / bits {:#018x})", v.to_bits()),
    );
}

#[test]
fn cfg01_convert_in_range_exact_integers() {
    let mut rng = Rng::new(0x0101);
    cmp_convert(0.0);
    cmp_convert(-0.0);
    for _ in 0..N {
        cmp_convert(rng.next_i32() as f64);
    }
    for v in [1i64, -1, 2, -2, 1000, -1000, 65536, -65536] {
        cmp_convert(v as f64);
    }
}

#[test]
fn cfg02_convert_in_range_fractional_truncates_toward_zero() {
    let mut rng = Rng::new(0x0202);
    for v in [
        0.9, -0.9, 1.5, -1.5, 2.5, -2.5, 0.5, -0.5, 1.0000001, -1.0000001, 42.99, -42.99,
    ] {
        cmp_convert(v);
    }
    for _ in 0..N {
        let base = rng.range_i32(-2_000_000, 2_000_000) as f64;
        let frac = rng.next_u32() as f64 / u32::MAX as f64;
        cmp_convert(base + frac);
        cmp_convert(base - frac);
    }
}

#[test]
fn cfg03_convert_subnormal_and_tiny() {
    for v in [
        5e-324,
        -5e-324,
        1e-300,
        -1e-300,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0,
        1e-16,
        -1e-16,
        0.4999999999999999,
        -0.4999999999999999,
    ] {
        cmp_convert(v);
    }
    let mut rng = Rng::new(0x0303);
    for _ in 0..N {
        // Random subnormal / very small magnitudes.
        let bits = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        cmp_convert(f64::from_bits(bits));
        cmp_convert(f64::from_bits(bits | (1u64 << 63)));
    }
}

#[test]
fn cfg04_convert_int_boundary_values() {
    let mut vals: Vec<f64> = vec![
        2147483646.0,
        2147483647.0,
        2147483647.5,
        2147483647.9999998,
        2147483648.0,
        2147483649.0,
        -2147483647.0,
        -2147483648.0,
        -2147483648.5,
        -2147483649.0,
        -2147483650.0,
        4294967296.0,
        -4294967296.0,
        1e18,
        -1e18,
        1e300,
        -1e300,
    ];
    // Every representable neighbour of the two range endpoints.
    for anchor in [2147483648.0f64, -2147483648.0f64, 2147483647.0f64] {
        let mut up = anchor;
        let mut down = anchor;
        for _ in 0..8 {
            up = f64::from_bits(up.to_bits() + 1);
            down = f64::from_bits(down.to_bits() - 1);
            vals.push(up);
            vals.push(down);
            vals.push(-up);
            vals.push(-down);
        }
    }
    for v in vals {
        cmp_convert(v);
    }
}

#[test]
fn cfg05_convert_random_full_f64_domain() {
    let mut rng = Rng::new(0x0505);
    for _ in 0..(N * 4) {
        cmp_convert(rng.interesting_f64());
    }
}

// ===========================================================================
// find_value_in_buffer  (rows 6..13)
// ===========================================================================

fn cmp_find(buf: &[i8], size: usize, search_val: i32) {
    let p = buf.as_ptr() as *const c_char;
    let cv = unsafe { (c().find_value_in_buffer)(p, size, search_val) };
    let rv = unsafe { (rs().find_value_in_buffer)(p, size, search_val) };
    assert_i32_eq(
        cv,
        rv,
        &format!(
            "find_value_in_buffer(len={}, size={size}, search_val={search_val})",
            buf.len()
        ),
    );
}

#[test]
fn cfg06_find_size_one() {
    for b in i8::MIN..=i8::MAX {
        let buf = [b];
        for sv in [b as i32, (b as i32) ^ 1, 0, 42, 255, -1] {
            cmp_find(&buf, 1, sv);
        }
    }
}

#[test]
fn cfg07_find_size_two_first_match_precedence() {
    let mut rng = Rng::new(0x0707);
    for _ in 0..N {
        let a = rng.next_u8() as i8;
        let b = rng.next_u8() as i8;
        let buf = [a, b];
        for sv in [a as i32, b as i32, rng.next_i32()] {
            cmp_find(&buf, 2, sv);
        }
        // Identical bytes: must return index 0, not 1.
        let same = [a, a];
        cmp_find(&same, 2, a as i32);
    }
}

#[test]
fn cfg08_find_size_256_first_middle_last() {
    let mut rng = Rng::new(0x0808);
    for _ in 0..N {
        // Buffer of a single filler byte with one distinct needle placed at a
        // chosen index, so the expected position is known to be unique.
        let filler = rng.next_u8();
        let needle = filler.wrapping_add(1 + (rng.next_u8() % 254));
        for idx in [0usize, 1, 127, 128, 254, 255] {
            let mut buf = vec![filler as i8; 256];
            buf[idx] = needle as i8;
            cmp_find(&buf, 256, needle as i32);
            cmp_find(&buf, 256, filler as i32);
        }
    }
}

#[test]
fn cfg09_find_duplicates_returns_lowest_index() {
    let mut rng = Rng::new(0x0909);
    for _ in 0..N {
        let len = 1 + rng.below(256) as usize;
        let needle = rng.next_u8();
        let mut buf = vec![needle.wrapping_add(1) as i8; len];
        let mut first = usize::MAX;
        for _ in 0..4 {
            let idx = rng.below(len as u64) as usize;
            buf[idx] = needle as i8;
            first = first.min(idx);
        }
        cmp_find(&buf, len, needle as i32);
        // Also cross-check the C answer really is the lowest index.
        let got = unsafe { (c().find_value_in_buffer)(buf.as_ptr() as *const c_char, len, needle as i32) };
        assert_eq!(got, first as i32, "C memchr did not return the first match");
    }
}

#[test]
fn cfg10_find_nul_byte_target() {
    let mut rng = Rng::new(0x0A0A);
    for _ in 0..N {
        let len = 1 + rng.below(300) as usize;
        let mut buf: Vec<i8> = (0..len).map(|_| (1 + rng.next_u8() % 255) as i8).collect();
        // Several NULs, including one after other data, so a strlen-style
        // implementation would answer differently from memchr.
        let z1 = rng.below(len as u64) as usize;
        let z2 = rng.below(len as u64) as usize;
        buf[z1] = 0;
        buf[z2] = 0;
        cmp_find(&buf, len, 0);
        // A byte located after the first NUL must still be found.
        let after = z1.min(z2);
        if after + 1 < len {
            buf[after + 1] = 0x7B;
            cmp_find(&buf, len, 0x7B);
        }
    }
}

#[test]
fn cfg11_find_high_bit_targets() {
    let mut rng = Rng::new(0x0B0B);
    for target in [0x80u8, 0x81, 0xFE, 0xFF, 0x7F, 0x00] {
        for _ in 0..300 {
            let len = 1 + rng.below(300) as usize;
            let mut buf: Vec<i8> = (0..len).map(|_| rng.next_u8() as i8).collect();
            let idx = rng.below(len as u64) as usize;
            buf[idx] = target as i8;
            // Both the signed and the unsigned spelling of the same byte.
            cmp_find(&buf, len, target as i8 as i32);
            cmp_find(&buf, len, target as i32);
        }
    }
}

#[test]
fn cfg12_find_match_beyond_size_limit() {
    let mut rng = Rng::new(0x0C0C);
    for _ in 0..N {
        let cap = 8 + rng.below(300) as usize;
        let size = 1 + rng.below(cap as u64) as usize;
        let needle = rng.next_u8();
        let mut buf = vec![needle.wrapping_add(1) as i8; cap];
        if size < cap {
            // Only occurrence lives strictly past `size` -> must not be found.
            let idx = size + rng.below((cap - size) as u64) as usize;
            buf[idx] = needle as i8;
            cmp_find(&buf, size, needle as i32);
            let cv =
                unsafe { (c().find_value_in_buffer)(buf.as_ptr() as *const c_char, size, needle as i32) };
            assert_eq!(cv, -1, "match beyond size must not be reported");
        }
        // Same buffer, full size -> found.
        cmp_find(&buf, cap, needle as i32);
    }
}

#[test]
fn cfg13_find_randomized_buffers_and_search_vals() {
    let mut rng = Rng::new(0x0D0D);
    for _ in 0..(N * 2) {
        let len = 1 + rng.below(512) as usize;
        let buf: Vec<i8> = (0..len).map(|_| rng.next_u8() as i8).collect();
        let size = rng.below(len as u64 + 1) as usize;
        cmp_find(&buf, size, rng.interesting_i32());
        cmp_find(&buf, size, rng.next_i32());
        cmp_find(&buf, len, rng.range_i32(-300, 300));
    }
}

// ===========================================================================
// process_negation  (rows 14, 15)
// ===========================================================================

fn cmp_procneg(v: i32) {
    let cv = unsafe { (c().process_negation)(v) };
    let rv = unsafe { (rs().process_negation)(v) };
    assert_i32_eq(cv, rv, &format!("process_negation({v})"));
}

#[test]
fn cfg14_process_negation_named_shapes() {
    for v in [
        0,
        1,
        -1,
        2,
        -2,
        256,
        -256,
        0x10000,
        -0x10000,
        0x0100_0000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ] {
        cmp_procneg(v);
    }
    // `!!` must not degrade into `& 1`: every input whose low bit is 0 but which
    // is non-zero still yields 1.
    for k in 1..31 {
        cmp_procneg(1i32 << k);
        cmp_procneg(-(1i32 << k));
    }
}

#[test]
fn cfg15_process_negation_random_full_domain() {
    let mut rng = Rng::new(0x0F0F);
    for _ in 0..(N * 4) {
        cmp_procneg(rng.next_i32());
        cmp_procneg(rng.interesting_i32());
    }
}

// ===========================================================================
// create_numeric_buffer  (rows 16..23)
// ===========================================================================

/// Fills two sentinel-initialised buffers of `cap` bytes and compares the whole
/// capacity (so writes past `size` would be caught too).
fn cmp_create(cap: usize, size: i32, seed: i32) {
    const SENTINEL: i8 = 0x5A;
    let mut cbuf = vec![SENTINEL; cap];
    let mut rbuf = vec![SENTINEL; cap];
    unsafe { (c().create_numeric_buffer)(cbuf.as_mut_ptr() as *mut c_char, size, seed) };
    unsafe { (rs().create_numeric_buffer)(rbuf.as_mut_ptr() as *mut c_char, size, seed) };
    let cb: Vec<u8> = cbuf.iter().map(|&b| b as u8).collect();
    let rb: Vec<u8> = rbuf.iter().map(|&b| b as u8).collect();
    assert_bytes_eq(
        &cb,
        &rb,
        &format!("create_numeric_buffer(cap={cap}, size={size}, seed={seed})"),
    );
}

#[test]
fn cfg16_create_size_one() {
    for seed in [0, 1, -1, 7, -7, 255, 256, -256, i32::MAX, i32::MIN] {
        cmp_create(4, 1, seed);
    }
    let mut rng = Rng::new(0x1010);
    for _ in 0..N {
        cmp_create(4, 1, rng.interesting_i32());
    }
}

#[test]
fn cfg17_create_size_seven_stride() {
    let mut rng = Rng::new(0x1111);
    for _ in 0..N {
        cmp_create(16, 7, rng.interesting_i32());
    }
    for seed in -20..=20 {
        cmp_create(16, 7, seed);
    }
}

#[test]
fn cfg18_create_size_256_full_permutation() {
    let mut rng = Rng::new(0x1212);
    for seed in 0..256 {
        cmp_create(256, 256, seed);
    }
    for _ in 0..N {
        cmp_create(256, 256, rng.interesting_i32());
    }
    // Property the whole library depends on: 256 bytes with stride 7 is a
    // permutation of all byte values (gcd(7, 256) == 1).
    let mut buf = vec![0i8; 256];
    unsafe { (c().create_numeric_buffer)(buf.as_mut_ptr() as *mut c_char, 256, 12345) };
    let mut seen = [false; 256];
    for &b in &buf {
        seen[b as u8 as usize] = true;
    }
    assert!(seen.iter().all(|&s| s), "stride-7 fill is not a permutation");
}

#[test]
fn cfg19_create_size_512_two_wraps() {
    let mut rng = Rng::new(0x1313);
    for _ in 0..(N / 4) {
        cmp_create(512, 512, rng.interesting_i32());
    }
    for size in [257, 300, 511, 512, 1000] {
        cmp_create(1024, size, rng.interesting_i32());
    }
}

#[test]
fn cfg20_create_size_below_capacity_leaves_tail() {
    let mut rng = Rng::new(0x1414);
    for _ in 0..N {
        let cap = 1 + rng.below(400) as usize;
        let size = rng.below(cap as u64 + 1) as i32;
        cmp_create(cap, size, rng.interesting_i32());
    }
}

#[test]
fn cfg21_create_negative_seed_signed_char() {
    for seed in -2000..0 {
        cmp_create(64, 40, seed);
    }
    let mut rng = Rng::new(0x1515);
    for _ in 0..N {
        let seed = -(1 + rng.below(1_000_000_000) as i32);
        cmp_create(64, 40, seed);
    }
}

#[test]
fn cfg22_create_seed_overflow_midloop() {
    // seed + i*7 crosses INT_MAX / INT_MIN partway through the loop.
    for delta in 0..40i32 {
        cmp_create(300, 256, i32::MAX - delta);
        cmp_create(300, 256, i32::MIN + delta);
    }
    let mut rng = Rng::new(0x1616);
    for _ in 0..(N / 4) {
        let d = rng.below(2000) as i32;
        cmp_create(300, 256, i32::MAX - d);
        cmp_create(300, 256, i32::MIN + d);
    }
}

#[test]
fn cfg23_create_randomized_size_and_seed() {
    let mut rng = Rng::new(0x1717);
    for _ in 0..(N * 2) {
        let cap = 1 + rng.below(600) as usize;
        let size = rng.below(cap as u64 + 1) as i32;
        cmp_create(cap, size, rng.next_i32());
    }
}

// ===========================================================================
// calculate_with_doubles  (rows 24..30)
// ===========================================================================

fn cmp_calc(a: i32, b: i32, cc: i32) {
    let cv = unsafe { (c().calculate_with_doubles)(a, b, cc) };
    let rv = unsafe { (rs().calculate_with_doubles)(a, b, cc) };
    assert_f64_bits_eq(cv, rv, &format!("calculate_with_doubles({a}, {b}, {cc})"));
}

#[test]
fn cfg24_calc_exact_division_zero_exponent() {
    let mut rng = Rng::new(0x1818);
    for _ in 0..N {
        let b = loop {
            let v = rng.range_i32(-10_000, 10_000);
            if v != 0 {
                break v;
            }
        };
        let k = rng.range_i32(-1000, 1000);
        let a = b.wrapping_mul(k);
        for cc in [0, 10, -10, 20, -20, 100, -100] {
            cmp_calc(a, b, cc);
        }
    }
}

#[test]
fn cfg25_calc_inexact_division_rounding() {
    let mut rng = Rng::new(0x1919);
    for _ in 0..(N * 2) {
        let a = rng.next_i32();
        let b = loop {
            let v = rng.next_i32();
            if v != 0 {
                break v;
            }
        };
        cmp_calc(a, b, rng.interesting_i32());
    }
    // Classic non-terminating binary fractions.
    for (a, b) in [(1, 3), (2, 3), (1, 7), (10, 3), (-1, 3), (1, -3), (-1, -7)] {
        for cc in -9..=9 {
            cmp_calc(a, b, cc);
        }
    }
}

#[test]
fn cfg26_calc_zero_divisor_guard_all_exponents() {
    let mut rng = Rng::new(0x1A1A);
    for cc in -30..=30 {
        cmp_calc(0, 0, cc);
        cmp_calc(1, 0, cc);
        cmp_calc(-1, 0, cc);
        cmp_calc(i32::MAX, 0, cc);
        cmp_calc(i32::MIN, 0, cc);
    }
    for _ in 0..N {
        cmp_calc(rng.next_i32(), 0, rng.next_i32());
    }
}

#[test]
fn cfg27_calc_all_nineteen_exponents() {
    // c % 10 spans -9..=9; pick c values realising each remainder.
    let mut rng = Rng::new(0x1B1B);
    for r in -9i32..=9 {
        for mult in 0..20i32 {
            let cc = r + if r >= 0 { 10 * mult } else { -10 * mult };
            cmp_calc(355, 113, cc);
            cmp_calc(-355, 113, cc);
            cmp_calc(1, 1, cc);
        }
    }
    for _ in 0..N {
        let cc = rng.next_i32();
        cmp_calc(rng.range_i32(-1000, 1000), 7, cc);
    }
}

#[test]
fn cfg28_calc_sign_combinations() {
    let mut rng = Rng::new(0x1C1C);
    for _ in 0..N {
        let m = 1 + rng.below(100_000) as i32;
        let n = 1 + rng.below(100_000) as i32;
        let cc = rng.interesting_i32();
        cmp_calc(m, n, cc);
        cmp_calc(-m, n, cc);
        cmp_calc(m, -n, cc);
        cmp_calc(-m, -n, cc);
        cmp_calc(0, n, cc);
        cmp_calc(0, -n, cc);
    }
}

#[test]
fn cfg29_calc_extreme_operands() {
    let extremes = [i32::MAX, i32::MIN, 1, -1, 0, i32::MAX - 1, i32::MIN + 1, 2, -2];
    let cs = [i32::MAX, i32::MIN, 0, 1, -1, 9, -9, 10, -10, i32::MIN + 1];
    for &a in &extremes {
        for &b in &extremes {
            for &cc in &cs {
                cmp_calc(a, b, cc);
            }
        }
    }
}

#[test]
fn cfg30_calc_randomized_full_domain() {
    let mut rng = Rng::new(0x1D1D);
    for _ in 0..(N * 4) {
        cmp_calc(rng.interesting_i32(), rng.interesting_i32(), rng.interesting_i32());
    }
    for _ in 0..(N * 2) {
        cmp_calc(rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

// ===========================================================================
// Composed low-level pipeline (rows 39, 40)
// ===========================================================================

/// Reproduces `doubleneg`'s internal pipeline by hand out of the low-level
/// exports, comparing C and Rust at every stage rather than only at the end.
#[test]
fn cfg39_low_level_pipeline_stage_by_stage() {
    let mut rng = Rng::new(0x1E1E);
    for _ in 0..(N / 2) {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        let p3 = rng.interesting_i32();
        let p4 = rng.interesting_i32();

        // Stage 1: !! on each parameter.
        for p in [p1, p2, p3, p4] {
            cmp_procneg(p);
        }

        // Stage 2: calculate_with_doubles -> convert_double_to_int.
        let cd = unsafe { (c().calculate_with_doubles)(p1, p2, p3) };
        let rd = unsafe { (rs().calculate_with_doubles)(p1, p2, p3) };
        assert_f64_bits_eq(cd, rd, &format!("pipeline calc({p1},{p2},{p3})"));
        cmp_convert(cd);
        cmp_convert(rd);
        cmp_convert(-1.0 * 1099511627776.0_f64); // -1.0 * pow(2, 40)

        // Stage 3: fill the 256-byte buffer.
        let mut cbuf = vec![0i8; 256];
        let mut rbuf = vec![0i8; 256];
        unsafe { (c().create_numeric_buffer)(cbuf.as_mut_ptr() as *mut c_char, 256, p1) };
        unsafe { (rs().create_numeric_buffer)(rbuf.as_mut_ptr() as *mut c_char, 256, p1) };
        let cb: Vec<u8> = cbuf.iter().map(|&b| b as u8).collect();
        let rb: Vec<u8> = rbuf.iter().map(|&b| b as u8).collect();
        assert_bytes_eq(&cb, &rb, &format!("pipeline buffer(seed={p1})"));

        // Stage 4: the four searches doubleneg performs.
        for sv in [
            p2.wrapping_rem(256),
            p3.wrapping_rem(256),
            p4.wrapping_rem(256),
            42,
        ] {
            cmp_find(&cbuf, 256, sv);
        }

        // Stage 5: the ten combined-feature searches.
        for i in 0..10i32 {
            let sb = p1.wrapping_add(i.wrapping_mul(p2)).wrapping_rem(256);
            cmp_find(&cbuf, 256, sb);
        }

        // Stage 6: the special double values.
        cmp_convert(f64::INFINITY);
        cmp_convert(f64::NAN);
    }
}

/// Cross-library composition: a buffer produced by one implementation must be
/// searchable by the other with identical results, which proves the intermediate
/// byte representation matches (not merely the final accumulator).
#[test]
fn cfg40_cross_library_buffer_and_search() {
    let mut rng = Rng::new(0x1F1F);
    for _ in 0..N {
        let size = 1 + rng.below(400) as i32;
        let seed = rng.interesting_i32();
        let cap = size as usize;

        let mut cbuf = vec![0i8; cap];
        let mut rbuf = vec![0i8; cap];
        unsafe { (c().create_numeric_buffer)(cbuf.as_mut_ptr() as *mut c_char, size, seed) };
        unsafe { (rs().create_numeric_buffer)(rbuf.as_mut_ptr() as *mut c_char, size, seed) };
        assert_eq!(cbuf, rbuf, "buffers differ for (size={size}, seed={seed})");

        for _ in 0..6 {
            let sv = rng.interesting_i32();
            // C buffer searched by both; Rust buffer searched by both.
            let a = unsafe { (c().find_value_in_buffer)(cbuf.as_ptr() as *const c_char, cap, sv) };
            let b = unsafe { (rs().find_value_in_buffer)(cbuf.as_ptr() as *const c_char, cap, sv) };
            let d = unsafe { (c().find_value_in_buffer)(rbuf.as_ptr() as *const c_char, cap, sv) };
            let e = unsafe { (rs().find_value_in_buffer)(rbuf.as_ptr() as *const c_char, cap, sv) };
            assert_i32_eq(a, b, &format!("C-buffer/{sv} C-vs-Rust search"));
            assert_i32_eq(a, d, &format!("C-search on C-buffer vs Rust-buffer /{sv}"));
            assert_i32_eq(a, e, &format!("cross search /{sv}"));
        }
    }
}

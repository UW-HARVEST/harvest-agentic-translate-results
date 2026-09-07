//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares stdout byte-for-byte (plus the
//! mutated `house_t` for `run`).

mod common;

use common::*;

// ===========================================================================
// Row 1 — run, default house, extra = 0
// ===========================================================================
#[test]
fn cfg_01_run_default_house_zero_extra() {
    diff_run(DEFAULT_HOUSE, 0);
}

// ===========================================================================
// Row 2 — run, default house, 512 randomized extra_bedrooms
// ===========================================================================
#[test]
fn cfg_02_run_default_house_random_extra() {
    let mut rng = Rng::new(2);
    for _ in 0..512 {
        diff_run(DEFAULT_HOUSE, rng.next_i32());
    }
}

// ===========================================================================
// Row 3 — run, fully randomized state
// ===========================================================================
#[test]
fn cfg_03_run_fully_random() {
    let mut rng = Rng::new(3);
    for _ in 0..2048 {
        let h = house_t::new(rng.next_i32(), rng.next_i32(), rng.next_bathrooms());
        diff_run(h, rng.next_i32());
    }
}

// ===========================================================================
// Row 4 — run, floors at the `house->floors++` overflow boundary
// ===========================================================================
#[test]
fn cfg_04_run_floors_boundaries() {
    let mut rng = Rng::new(4);
    for &floors in &[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, -1, 0, 1] {
        // fixed companion state first
        diff_run(house_t::new(floors, 5, 2.5), 0);
        // then randomized companions
        for _ in 0..64 {
            let h = house_t::new(floors, rng.next_i32(), rng.next_bathrooms());
            diff_run(h, rng.next_i32());
        }
    }
}

// ===========================================================================
// Row 5 — run, bedrooms x extra_bedrooms full cross-product (`+=` overflow)
// ===========================================================================
#[test]
fn cfg_05_run_bedrooms_extra_cross_product() {
    let vals = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 0, 1, -1];
    for &bedrooms in &vals {
        for &extra in &vals {
            diff_run(house_t::new(2, bedrooms, 2.5), extra);
        }
    }
}

// ===========================================================================
// Row 6 — run, signed zero bathrooms
// ===========================================================================
#[test]
fn cfg_06_run_bathrooms_signed_zero() {
    for &b in &[0.0f64, -0.0f64, -1.0f64, -0.5f64, -0.04f64, -0.06f64] {
        diff_run(house_t::new(2, 5, b), 1);
    }
    // -1.0 + 1.0 == +0.0, and -0.5 + 1.0 == 0.5: check the sign survives.
    diff_run(house_t::new(0, 0, -1.0), 0);
}

// ===========================================================================
// Row 7 — run, %.1f round-to-even half-way points (and one ULP either side)
// ===========================================================================
#[test]
fn cfg_07_run_bathrooms_rounding_halfway() {
    let mut vals: Vec<f64> = Vec::new();
    for i in 0..60 {
        let v = i as f64 * 0.1 + 0.05; // 0.05, 0.15, 0.25, ...
        vals.push(v);
        vals.push(-v);
        vals.push(next_up(v));
        vals.push(next_down(v));
        vals.push(-next_up(v));
        vals.push(-next_down(v));
    }
    // Exactly-representable halves that %.1f must round to even.
    for &v in &[0.25f64, 0.75, 1.25, 1.75, 2.25, 2.75, 0.125, 0.375] {
        vals.push(v);
        vals.push(-v);
    }
    for v in vals {
        diff_run(house_t::new(2, 5, v), 0);
    }
}

fn next_up(v: f64) -> f64 {
    if v.is_nan() {
        return v;
    }
    if v == 0.0 {
        return f64::from_bits(1);
    }
    if v > 0.0 {
        f64::from_bits(v.to_bits() + 1)
    } else {
        f64::from_bits(v.to_bits() - 1)
    }
}

fn next_down(v: f64) -> f64 {
    -next_up(-v)
}

// ===========================================================================
// Row 8 — run, huge bathrooms (`+= 1.0` is a no-op; %.1f emits many digits)
// ===========================================================================
#[test]
fn cfg_08_run_bathrooms_huge() {
    for &v in &[
        1e15f64,
        1e16,
        9_007_199_254_740_992.0, // 2^53
        9_007_199_254_740_993.0,
        1e300,
        f64::MAX,
        1.0 / f64::EPSILON,
    ] {
        diff_run(house_t::new(2, 5, v), 3);
        diff_run(house_t::new(2, 5, -v), 3);
    }
}

// ===========================================================================
// Row 9 — run, tiny / subnormal bathrooms
// ===========================================================================
#[test]
fn cfg_09_run_bathrooms_tiny_subnormal() {
    for &v in &[
        f64::MIN_POSITIVE,
        f64::from_bits(1), // smallest subnormal
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        1e-300f64,
        1e-15f64,
        f64::EPSILON,
    ] {
        diff_run(house_t::new(2, 5, v), -7);
        diff_run(house_t::new(2, 5, -v), -7);
    }
}

// ===========================================================================
// Row 10 — run, non-finite bathrooms
// ===========================================================================
#[test]
fn cfg_10_run_bathrooms_non_finite() {
    let vals = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7FF8_0000_DEAD_BEEF), // NaN with payload
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for &v in &vals {
        diff_run(house_t::new(2, 5, v), 11);
        diff_run(house_t::new(i32::MAX, i32::MAX, v), i32::MIN);
    }
}

// ===========================================================================
// Row 11 — run, 1024 randomized raw 64-bit double patterns
// ===========================================================================
#[test]
fn cfg_11_run_bathrooms_random_bitpatterns() {
    let mut rng = Rng::new(11);
    for _ in 0..1024 {
        let v = f64::from_bits(rng.next_u64());
        diff_run(house_t::new(rng.next_i32(), rng.next_i32(), v), rng.next_i32());
    }
}

// ===========================================================================
// Row 12 — driver, plain positive decimals (randomized)
// ===========================================================================
#[test]
fn cfg_12_driver_random_positive() {
    let mut rng = Rng::new(12);
    for _ in 0..512 {
        let v = (rng.next_u32() >> 1) as i32; // 0..=i32::MAX
        diff_driver_accepts(v.to_string().as_bytes());
    }
}

// ===========================================================================
// Row 13 — driver, plain negative decimals (randomized)
// ===========================================================================
#[test]
fn cfg_13_driver_random_negative() {
    let mut rng = Rng::new(13);
    for _ in 0..512 {
        let v = -((rng.next_u32() >> 1) as i64) - 1; // i32::MIN..=-1
        let v = v.max(i32::MIN as i64);
        diff_driver_accepts(v.to_string().as_bytes());
    }
}

// ===========================================================================
// Row 14 — driver, explicit '+' sign
// ===========================================================================
#[test]
fn cfg_14_driver_explicit_plus() {
    let mut rng = Rng::new(14);
    diff_driver_accepts(b"+0");
    diff_driver_accepts(b"+2147483647");
    for _ in 0..256 {
        let v = (rng.next_u32() >> 1) as i32;
        diff_driver_accepts(format!("+{v}").as_bytes());
    }
}

// ===========================================================================
// Row 15 — driver, leading whitespace
// ===========================================================================
#[test]
fn cfg_15_driver_leading_whitespace() {
    const WS: &[u8] = b" \t\n\r\x0b\x0c";
    let mut rng = Rng::new(15);
    for _ in 0..512 {
        let n = rng.below(6) as usize;
        let mut s: Vec<u8> = (0..n).map(|_| WS[rng.below(6) as usize]).collect();
        let v = rng.next_i32();
        if rng.below(2) == 0 {
            s.push(b'+');
        }
        s.extend_from_slice(v.abs_diff(0).to_string().as_bytes());
        diff_driver(&s);
    }
    diff_driver_accepts(b" \t\r\n\x0b\x0c42");
    diff_driver_accepts(b"   -17");
}

// ===========================================================================
// Row 16 — driver, trailing garbage after a valid number (C accepts it)
// ===========================================================================
#[test]
fn cfg_16_driver_trailing_garbage() {
    let mut rng = Rng::new(16);
    const JUNK: &[u8] = b"abcXYZ_ .,;:/\\+-*()[]{}#!?~@%^&|<>=\"'`\t 0123456789";
    for _ in 0..1024 {
        let v = rng.next_i32();
        let mut s = v.to_string().into_bytes();
        let n = 1 + rng.below(6) as usize;
        for _ in 0..n {
            s.push(JUNK[rng.below(JUNK.len() as u64) as usize]);
        }
        diff_driver(&s);
    }
    diff_driver_accepts(b"12abc");
    diff_driver_accepts(b"0zzz");
    diff_driver_accepts(b"-5 6 7");
}

// ===========================================================================
// Row 17 — driver, leading zeros and base-10 reading of 0x / 0-prefixed text
// ===========================================================================
#[test]
fn cfg_17_driver_leading_zeros_and_base10() {
    let mut rng = Rng::new(17);
    diff_driver_accepts(b"0000042");
    diff_driver_accepts(b"010");
    diff_driver_accepts(b"0x1A"); // base 10 -> 0, stops at 'x'
    diff_driver_accepts(b"0X10");
    diff_driver_accepts(b"0b101");
    diff_driver_accepts(b"-0x10");
    diff_driver_accepts(b"00000000000000000000000000000001");
    for _ in 0..256 {
        let zeros = rng.below(40) as usize;
        let v = rng.next_u32() % 1_000_000;
        let mut s = Vec::new();
        if rng.below(2) == 0 {
            s.push(b'-');
        }
        s.extend(std::iter::repeat(b'0').take(zeros));
        s.extend_from_slice(v.to_string().as_bytes());
        diff_driver(&s);
    }
}

// ===========================================================================
// Row 18 — driver, int boundaries as decimal text
// ===========================================================================
#[test]
fn cfg_18_driver_int_boundaries() {
    for s in [
        "2147483647",
        "-2147483648",
        "2147483646",
        "-2147483647",
        "0",
        "-0",
        "+0",
        "1",
        "-1",
    ] {
        diff_driver_accepts(s.as_bytes());
    }
}

// ===========================================================================
// Row 19 — driver, out-of-int and out-of-long magnitudes (randomized)
// ===========================================================================
#[test]
fn cfg_19_driver_out_of_int_range_random() {
    let mut rng = Rng::new(19);
    for _ in 0..256 {
        // in long, outside int
        let v = (i32::MAX as i64) + 1 + (rng.next_u32() as i64);
        diff_driver_rejects(v.to_string().as_bytes());
        let v = (i32::MIN as i64) - 1 - (rng.next_u32() as i64);
        diff_driver_rejects(v.to_string().as_bytes());
    }
    for _ in 0..256 {
        // beyond long -> ERANGE
        let digits = 20 + rng.below(30) as usize;
        let mut s = Vec::new();
        if rng.below(2) == 0 {
            s.push(b'-');
        }
        s.push(b'1' + (rng.below(9) as u8));
        for _ in 1..digits {
            s.push(b'0' + (rng.below(10) as u8));
        }
        diff_driver_rejects(&s);
    }
}

// ===========================================================================
// Row 20 — driver, 1024 fully randomized byte strings
// ===========================================================================
#[test]
fn cfg_20_driver_random_fuzz_strings() {
    let mut rng = Rng::new(20);
    // Printable ASCII except NUL, weighted toward digits/signs/space.
    let mut alphabet: Vec<u8> = (0x20u8..0x7f).collect();
    alphabet.extend_from_slice(b"0123456789012345678901234567890123456789");
    alphabet.extend_from_slice(b"+-+-+-  \t\n");
    alphabet.extend_from_slice(&[0x80, 0xff, 0xc3, 0xa9]); // non-ASCII bytes
    let alen = alphabet.len() as u64;
    for _ in 0..1024 {
        let n = rng.below(25) as usize;
        let s: Vec<u8> = (0..n)
            .map(|_| alphabet[rng.below(alen) as usize])
            .collect();
        diff_driver(&s);
    }
}

// ===========================================================================
// Row 21 — run twice on the same house (exactly what driver does)
// ===========================================================================
#[test]
fn cfg_21_run_twice_state_carryover() {
    diff_run_n(DEFAULT_HOUSE, 0, 2);
    let mut rng = Rng::new(21);
    for _ in 0..512 {
        let h = house_t::new(rng.next_i32(), rng.next_i32(), rng.next_bathrooms());
        diff_run_n(h, rng.next_i32(), 2);
    }
    for &extra in &[i32::MIN, i32::MAX, 0, -1, 1] {
        diff_run_n(house_t::new(i32::MAX, i32::MAX, 2.5), extra, 2);
    }
}

// ===========================================================================
// Row 22 — run 16 times, accumulating state
// ===========================================================================
#[test]
fn cfg_22_run_many_times_accumulation() {
    diff_run_n(DEFAULT_HOUSE, 1, 16);
    diff_run_n(house_t::new(i32::MAX - 3, i32::MAX - 3, 1e15), 3, 16);
    diff_run_n(house_t::new(i32::MIN, i32::MIN, -1e300), i32::MIN, 16);
    diff_run_n(house_t::new(0, 0, f64::NAN), 0, 16);
    let mut rng = Rng::new(22);
    for _ in 0..128 {
        let h = house_t::new(rng.next_i32(), rng.next_i32(), rng.next_bathrooms());
        diff_run_n(h, rng.next_i32(), 16);
    }
}

// ===========================================================================
// Row 23 — interleaved driver / run in one process (hidden global state)
// ===========================================================================
#[test]
fn cfg_23_interleaved_driver_and_run() {
    let mut rng = Rng::new(23);
    for _ in 0..128 {
        let n = rng.next_i32();
        diff_driver(n.to_string().as_bytes());
        let h = house_t::new(rng.next_i32(), rng.next_i32(), rng.next_bathrooms());
        diff_run(h, rng.next_i32());
        diff_driver(b"not-a-number");
        diff_driver(n.to_string().as_bytes());
        diff_run_n(h, rng.next_i32(), 2);
        // `errno` is dirtied by the rejection path; make sure a following
        // accept still works (parse_val re-zeroes errno).
        diff_driver(b"99999999999999999999999999");
        diff_driver_accepts(b"7");
    }
}

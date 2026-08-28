//! Level 4: `get_time_based_value`.
//!
//! `reference_time` is derived from `current_time`, so `difftime` cancels the
//! wall clock and the result depends only on `seed` — including the wrap of the
//! `seed * 3600` int multiply.

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn get_time_based_value_matches() {
    let libs = load();
    let (f_c, f_r) = libs.pair::<FnUnary>("get_time_based_value");

    let mut seeds: Vec<c_int> = INTS.to_vec();
    // Around the point where `seed * 3600` stops fitting in an int.
    for base in [596_522_i32, 596_523, 596_524, -596_522, -596_523, -596_524] {
        for d in -2..=2 {
            seeds.push(base.wrapping_add(d));
        }
    }
    // Multiples of the wrap period and other awkward magnitudes.
    for k in 1..=64_i32 {
        seeds.push(k.wrapping_mul(1_193_047));
        seeds.push(k.wrapping_mul(-1_193_047));
        seeds.push(i32::MAX / k);
        seeds.push(i32::MIN / k);
        seeds.push(k.wrapping_mul(100));
        seeds.push(k.wrapping_mul(36));
    }
    // Every power of two, both signs.
    for bit in 0..31 {
        let v = 1_i32 << bit;
        seeds.push(v);
        seeds.push(-v);
        seeds.push(v.wrapping_sub(1));
        seeds.push(v.wrapping_add(1));
    }
    // Deterministic pseudo-random sweep over the whole int range.
    let mut state: u32 = 0x1234_5678;
    for _ in 0..20_000 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seeds.push(state as c_int);
    }

    for &seed in &seeds {
        let ec = unsafe { f_c(seed) };
        let er = unsafe { f_r(seed) };
        assert_eq!(ec, er, "get_time_based_value({seed})");
    }
}

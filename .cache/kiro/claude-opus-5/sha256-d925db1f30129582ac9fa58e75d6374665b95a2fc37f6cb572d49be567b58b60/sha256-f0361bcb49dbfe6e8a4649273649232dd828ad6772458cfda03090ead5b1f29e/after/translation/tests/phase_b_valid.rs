//! Phase B — valid-path differential tests.
//! One test (or test group) per row of `CONFIGS.md`. Every call goes through
//! the `.so` exports of BOTH implementations; outputs are compared
//! byte-for-byte.

mod common;

use common::{rand_ascii, Pair, Rng};

const SEED: u64 = 0x5EED_1234;

// ---------------------------------------------------------------------------
// Row 1 / 2 — validate_and_normalize
// ---------------------------------------------------------------------------

#[test]
fn row01_validate_randomized_full_range() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED);
    for i in 0..20_000 {
        let v = rng.interesting_i32();
        assert_eq!(
            p.c_validate(v),
            p.r_validate(v),
            "row1 iter {i}: validate_and_normalize({v})"
        );
    }
}

#[test]
fn row02_validate_exact_boundaries() {
    let p = Pair::fresh();
    for v in [
        i32::MIN,
        i32::MIN + 1,
        -512,
        -1,
        0,
        1,
        62,
        63,
        64,
        65,
        509,
        510,
        511,
        512,
        513,
        i32::MAX - 1,
        i32::MAX,
    ] {
        assert_eq!(
            p.c_validate(v),
            p.r_validate(v),
            "row2: validate_and_normalize({v})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 3 / 4 — process_octal_string
// ---------------------------------------------------------------------------

#[test]
fn row03_octal_string_randomized() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..5_000 {
        let v = rng.interesting_i32();
        let c = p.c_octal(v);
        let r = p.r_octal(v);
        assert_eq!(
            c,
            r,
            "row3 iter {i}: process_octal_string({v})\n c={:?}\n r={:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
    }
}

#[test]
fn row04_octal_string_exact_values() {
    let p = Pair::fresh();
    for v in [
        0,
        1,
        7,
        8,
        9,
        0o123,
        0o777,
        -1,
        -2,
        -8,
        -0o123,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ] {
        assert_eq!(p.c_octal(v), p.r_octal(v), "row4: process_octal_string({v})");
    }
}

// ---------------------------------------------------------------------------
// Rows 5-8 — find_and_replace_char
// ---------------------------------------------------------------------------

#[test]
fn row05_find_replace_single_occurrence_mid() {
    let p = Pair::fresh();
    let s = b"hello world";
    assert_eq!(
        p.c_find_replace(s, b'w' as i32),
        p.r_find_replace(s, b'w' as i32)
    );
}

#[test]
fn row06_find_replace_only_first_of_many() {
    let p = Pair::fresh();
    let s = b"aaaa-aaaa-aaaa";
    let c = p.c_find_replace(s, b'a' as i32);
    let r = p.r_find_replace(s, b'a' as i32);
    assert_eq!(c, r);
    // Sanity: exactly one byte changed.
    assert_eq!(&c[..14], b"Xaaa-aaaa-aaaa");
}

#[test]
fn row07_find_replace_first_and_last_index() {
    let p = Pair::fresh();
    let s = b"abcdefg";
    assert_eq!(
        p.c_find_replace(s, b'a' as i32),
        p.r_find_replace(s, b'a' as i32),
        "needle at index 0"
    );
    assert_eq!(
        p.c_find_replace(s, b'g' as i32),
        p.r_find_replace(s, b'g' as i32),
        "needle at last index"
    );
}

#[test]
fn row08_find_replace_randomized_strings_and_needles() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..10_000 {
        let len = rng.below(40) as usize; // includes 0 (empty string)
        let s = rand_ascii(&mut rng, len);
        // Needle spans well beyond `unsigned char` in both directions.
        let needle = rng.range(-512, 1023) as i32;
        assert_eq!(
            p.c_find_replace(&s, needle),
            p.r_find_replace(&s, needle),
            "row8 iter {i}: find_and_replace_char({:?}, {needle})",
            String::from_utf8_lossy(&s)
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 9-16 — low-level accumulator / multiplier entry points
// ---------------------------------------------------------------------------

#[test]
fn row09_add_single_call_randomized() {
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..2_000 {
        let p = Pair::fresh(); // fresh state per call
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        assert_eq!(
            p.c_add(a, b),
            p.r_add(a, b),
            "row9 iter {i}: add_to_accumulator({a}, {b})"
        );
    }
}

#[test]
fn row10_add_long_sequence() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..20_000 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        assert_eq!(
            p.c_add(a, b),
            p.r_add(a, b),
            "row10 step {i}: add_to_accumulator({a}, {b})"
        );
    }
}

#[test]
fn row11_sub_from_zero_accumulator() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..2_000 {
        let p = Pair::fresh();
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        assert_eq!(
            p.c_sub(a, b),
            p.r_sub(a, b),
            "row11 iter {i}: subtract_from_accumulator({a}, {b})"
        );
    }
}

#[test]
fn row12_sub_after_adds() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..10_000 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        assert_eq!(p.c_add(a, b), p.r_add(a, b), "row12 add step {i}");
        let (c, d) = (rng.interesting_i32(), rng.interesting_i32());
        assert_eq!(
            p.c_sub(c, d),
            p.r_sub(c, d),
            "row12 sub step {i}: subtract_from_accumulator({c}, {d})"
        );
    }
}

#[test]
fn row13_mul_randomized_sequence() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..20_000 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        assert_eq!(
            p.c_mul(a, b),
            p.r_mul(a, b),
            "row13 step {i}: multiply_with_multiplier({a}, {b})"
        );
    }
}

#[test]
fn row14_div_after_seeding_multiplier() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..3_000 {
        let p = Pair::fresh();
        // Seed `multiplier` to a large positive value on both sides.
        let seed_a = rng.range(2, 30_000) as i32;
        let seed_b = rng.range(2, 30_000) as i32;
        assert_eq!(p.c_mul(seed_a, seed_b), p.r_mul(seed_a, seed_b));
        // Any non-zero divisor, positive or negative. `INT_MIN / -1` is UB in
        // C (SIGFPE on x86-64) so -1 is excluded when it could pair with
        // INT_MIN; here `multiplier` is never INT_MIN, but exclude anyway to
        // keep the row deterministic.
        let mut b = rng.interesting_i32();
        if b == 0 {
            b = 3;
        }
        assert_eq!(
            p.c_div(0, b),
            p.r_div(0, b),
            "row14 iter {i}: divide_multiplier(_, {b}) after mul({seed_a},{seed_b})"
        );
    }
}

#[test]
fn row15_div_by_zero_guard_repeated() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..500 {
        let a = rng.interesting_i32();
        assert_eq!(
            p.c_div(a, 0),
            p.r_div(a, 0),
            "row15 step {i}: divide_multiplier({a}, 0)"
        );
        // operation_count keeps rising even though the division is skipped;
        // observe it through findrep's `operation_count * 010` term.
        assert_eq!(
            p.c_findrep(0, 0, 0, 0),
            p.r_findrep(0, 0, 0, 0),
            "row15 step {i}: findrep observing operation_count"
        );
    }
}

#[test]
fn row16_mixed_low_level_interleaving() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 16);
    for step in 0..200 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        let (cv, rv) = match rng.below(4) {
            0 => (p.c_add(a, b), p.r_add(a, b)),
            1 => (p.c_mul(a, b), p.r_mul(a, b)),
            2 => (p.c_sub(a, b), p.r_sub(a, b)),
            _ => {
                let d = if b == 0 { 1 } else { b };
                // Avoid the C UB case INT_MIN / -1.
                let d = if d == -1 { -3 } else { d };
                (p.c_div(a, d), p.r_div(a, d))
            }
        };
        assert_eq!(cv, rv, "row16 step {step}: args ({a}, {b})");
    }
}

// ---------------------------------------------------------------------------
// Rows 17-21 — findrep active_params 0..4, every placement
// ---------------------------------------------------------------------------

fn placements(k: usize) -> Vec<[bool; 4]> {
    (0u8..16)
        .filter(|m| m.count_ones() as usize == k)
        .map(|m| {
            [
                m & 1 != 0,
                m & 2 != 0,
                m & 4 != 0,
                m & 8 != 0,
            ]
        })
        .collect()
}

fn check_active_params(k: usize, row: &str, seed: u64) {
    let mut rng = Rng::new(seed);
    for mask in placements(k) {
        for iter in 0..200 {
            let p = Pair::fresh();
            let mut v = [0i32; 4];
            for i in 0..4 {
                if mask[i] {
                    // Guaranteed non-zero.
                    let mut x = rng.interesting_i32();
                    if x == 0 {
                        x = 1;
                    }
                    v[i] = x;
                }
            }
            assert_eq!(
                p.c_findrep(v[0], v[1], v[2], v[3]),
                p.r_findrep(v[0], v[1], v[2], v[3]),
                "{row} mask {mask:?} iter {iter}: findrep{v:?}"
            );
        }
    }
}

#[test]
fn row17_findrep_active_params_0() {
    let p = Pair::fresh();
    assert_eq!(p.c_findrep(0, 0, 0, 0), p.r_findrep(0, 0, 0, 0));
}

#[test]
fn row18_findrep_active_params_1() {
    check_active_params(1, "row18", SEED ^ 18);
}

#[test]
fn row19_findrep_active_params_2() {
    check_active_params(2, "row19", SEED ^ 19);
}

#[test]
fn row20_findrep_active_params_3() {
    check_active_params(3, "row20", SEED ^ 20);
}

#[test]
fn row21_findrep_active_params_4() {
    check_active_params(4, "row21", SEED ^ 21);
}

// ---------------------------------------------------------------------------
// Rows 22-26 — state-dependent branches inside findrep
// ---------------------------------------------------------------------------

#[test]
fn row22_findrep_accumulator_below_subtract_threshold() {
    // active_params == 1 with a small param keeps accumulator small:
    // add(64, 0) -> accumulator = 64 <= 104, so operations[2] must NOT run.
    let p = Pair::fresh();
    assert_eq!(p.c_findrep(5, 0, 0, 0), p.r_findrep(5, 0, 0, 0));
    // Repeat with several small single-param values.
    for v in [1, 2, 63, 64, -1, -1000, i32::MIN] {
        let p = Pair::fresh();
        assert_eq!(p.c_findrep(v, 0, 0, 0), p.r_findrep(v, 0, 0, 0), "v={v}");
    }
}

#[test]
fn row23_findrep_accumulator_above_subtract_threshold() {
    // add(511, 511) -> accumulator = 1022 > 104, so operations[2] runs.
    let p = Pair::fresh();
    assert_eq!(
        p.c_findrep(1000, 2000, 3000, 4000),
        p.r_findrep(1000, 2000, 3000, 4000)
    );
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..300 {
        let p = Pair::fresh();
        let a = rng.range(200, 100_000) as i32;
        let b = rng.range(200, 100_000) as i32;
        assert_eq!(
            p.c_findrep(a, b, 0, 0),
            p.r_findrep(a, b, 0, 0),
            "row23 iter {i}: ({a},{b})"
        );
    }
}

#[test]
fn row24_findrep_multiplier_above_divide_threshold() {
    // Pre-drive multiplier well past 0100 (=64) via the low-level entry point,
    // then call findrep so the `multiplier > 0100` branch fires.
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..500 {
        let p = Pair::fresh();
        let a = rng.range(2, 1000) as i32;
        let b = rng.range(2, 1000) as i32;
        assert_eq!(p.c_mul(a, b), p.r_mul(a, b), "row24 seed mul {i}");
        let (w, x, y, z) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        assert_eq!(
            p.c_findrep(w, x, y, z),
            p.r_findrep(w, x, y, z),
            "row24 iter {i}: mul({a},{b}) then findrep({w},{x},{y},{z})"
        );
        // Second call sees the divided multiplier.
        assert_eq!(
            p.c_findrep(w, x, y, z),
            p.r_findrep(w, x, y, z),
            "row24 iter {i}: second findrep"
        );
    }
}

#[test]
fn row25_findrep_multiplier_zero_kills_both_active() {
    let p = Pair::fresh();
    // multiply by 0 -> multiplier == 0 forever after.
    assert_eq!(p.c_mul(0, 5), p.r_mul(0, 5));
    for args in [(0, 0, 0, 0), (1, 0, 0, 0), (5, 6, 7, 8), (1000, 2000, 3, 4)] {
        let (a, b, c, d) = args;
        assert_eq!(
            p.c_findrep(a, b, c, d),
            p.r_findrep(a, b, c, d),
            "row25 findrep{args:?}"
        );
    }
}

#[test]
fn row26_findrep_accumulator_zero_kills_both_active() {
    let p = Pair::fresh();
    // add(10,10) then subtract to bring accumulator exactly back to 0:
    // accumulator = 20; subtract_from_accumulator(20, 0) -> 20 - 20 = 0.
    assert_eq!(p.c_add(10, 10), p.r_add(10, 10));
    assert_eq!(p.c_sub(20, 0), p.r_sub(20, 0));
    // accumulator == 0 here on both sides; findrep(0,0,0,0) leaves it at 0 so
    // `both_active` is false.
    assert_eq!(p.c_findrep(0, 0, 0, 0), p.r_findrep(0, 0, 0, 0));
}

// ---------------------------------------------------------------------------
// Rows 27-31 — findrep input value classes
// ---------------------------------------------------------------------------

#[test]
fn row27_findrep_all_negative_params() {
    let mut rng = Rng::new(SEED ^ 27);
    for i in 0..500 {
        let p = Pair::fresh();
        let v: Vec<i32> = (0..4).map(|_| rng.range(i32::MIN as i64, -1) as i32).collect();
        assert_eq!(
            p.c_findrep(v[0], v[1], v[2], v[3]),
            p.r_findrep(v[0], v[1], v[2], v[3]),
            "row27 iter {i}: {v:?}"
        );
    }
}

#[test]
fn row28_findrep_all_params_above_upper_threshold() {
    let mut rng = Rng::new(SEED ^ 28);
    for i in 0..500 {
        let p = Pair::fresh();
        let v: Vec<i32> = (0..4)
            .map(|_| rng.range(512, i32::MAX as i64) as i32)
            .collect();
        assert_eq!(
            p.c_findrep(v[0], v[1], v[2], v[3]),
            p.r_findrep(v[0], v[1], v[2], v[3]),
            "row28 iter {i}: {v:?}"
        );
    }
}

#[test]
fn row29_findrep_all_params_below_lower_threshold() {
    let mut rng = Rng::new(SEED ^ 29);
    for i in 0..500 {
        let p = Pair::fresh();
        let v: Vec<i32> = (0..4).map(|_| rng.range(1, 63) as i32).collect();
        assert_eq!(
            p.c_findrep(v[0], v[1], v[2], v[3]),
            p.r_findrep(v[0], v[1], v[2], v[3]),
            "row29 iter {i}: {v:?}"
        );
    }
}

#[test]
fn row30_findrep_boundary_cross_product() {
    const B: [i32; 8] = [0, 1, 63, 64, 511, 512, i32::MIN, i32::MAX];
    // Full 8^4 = 4096 cross-product, each on a pristine pair.
    for &a in &B {
        for &b in &B {
            for &c in &B {
                for &d in &B {
                    let p = Pair::fresh();
                    assert_eq!(
                        p.c_findrep(a, b, c, d),
                        p.r_findrep(a, b, c, d),
                        "row30: findrep({a}, {b}, {c}, {d})"
                    );
                }
            }
        }
    }
}

#[test]
fn row31_findrep_single_call_fully_randomized() {
    let mut rng = Rng::new(SEED ^ 31);
    for i in 0..3_000 {
        let p = Pair::fresh();
        let (a, b, c, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        assert_eq!(
            p.c_findrep(a, b, c, d),
            p.r_findrep(a, b, c, d),
            "row31 iter {i}: findrep({a}, {b}, {c}, {d})"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 32-35 — long stateful programs across the whole API
// ---------------------------------------------------------------------------

#[test]
fn row32_findrep_repeated_stateful_calls() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 32);
    for step in 0..300 {
        let (a, b, c, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        assert_eq!(
            p.c_findrep(a, b, c, d),
            p.r_findrep(a, b, c, d),
            "row32 step {step}: findrep({a}, {b}, {c}, {d})"
        );
    }
}

#[test]
fn row33_findrep_interleaved_with_low_level_ops() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 33);
    for step in 0..300 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        let (cv, rv) = match rng.below(5) {
            0 => (p.c_add(a, b), p.r_add(a, b)),
            1 => (p.c_mul(a, b), p.r_mul(a, b)),
            2 => (p.c_sub(a, b), p.r_sub(a, b)),
            3 => {
                let mut d = b;
                if d == 0 {
                    d = 7;
                }
                if d == -1 {
                    d = -7;
                }
                (p.c_div(a, d), p.r_div(a, d))
            }
            _ => {
                let (c, d) = (rng.interesting_i32(), rng.interesting_i32());
                (p.c_findrep(a, b, c, d), p.r_findrep(a, b, c, d))
            }
        };
        assert_eq!(cv, rv, "row33 step {step}: args ({a}, {b})");
    }
}

#[test]
fn row34_findrep_internal_string_pipeline() {
    // findrep internally does:
    //   process_octal_string(message, 0123)
    //   memchr over "Function pointer example with static vars" for 'p'
    //   find_and_replace_char(message, 'O')
    // Reproduce those exact steps externally through both `.so`s and compare
    // the resulting buffers byte-for-byte.
    let p = Pair::fresh();
    assert_eq!(p.c_findrep(1, 2, 3, 4), p.r_findrep(1, 2, 3, 4));

    let c_msg = p.c_octal(0o123);
    let r_msg = p.r_octal(0o123);
    assert_eq!(c_msg, r_msg, "row34: process_octal_string(0123)");

    let nul = c_msg.iter().position(|&b| b == 0).expect("NUL terminator");
    let msg = &c_msg[..nul];
    assert_eq!(
        p.c_find_replace(msg, b'O' as i32),
        p.r_find_replace(msg, b'O' as i32),
        "row34: find_and_replace_char(message, 'O')"
    );

    let hay = b"Function pointer example with static vars";
    assert_eq!(
        p.c_find_replace(hay, b'p' as i32),
        p.r_find_replace(hay, b'p' as i32),
        "row34: memchr haystack / 'p'"
    );
}

#[test]
fn row35_whole_api_long_random_program() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 35);
    for step in 0..500 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        match rng.below(8) {
            0 => assert_eq!(p.c_add(a, b), p.r_add(a, b), "row35 step {step} add"),
            1 => assert_eq!(p.c_mul(a, b), p.r_mul(a, b), "row35 step {step} mul"),
            2 => assert_eq!(p.c_sub(a, b), p.r_sub(a, b), "row35 step {step} sub"),
            3 => {
                let mut d = b;
                if d == 0 {
                    d = 11;
                }
                if d == -1 {
                    d = -11;
                }
                assert_eq!(p.c_div(a, d), p.r_div(a, d), "row35 step {step} div");
            }
            4 => assert_eq!(
                p.c_validate(a),
                p.r_validate(a),
                "row35 step {step} validate({a})"
            ),
            5 => assert_eq!(
                p.c_octal(a),
                p.r_octal(a),
                "row35 step {step} octal({a})"
            ),
            6 => {
                let len = rng.below(40) as usize;
                let s = rand_ascii(&mut rng, len);
                let needle = rng.range(-300, 600) as i32;
                assert_eq!(
                    p.c_find_replace(&s, needle),
                    p.r_find_replace(&s, needle),
                    "row35 step {step} find_replace"
                );
            }
            _ => {
                let (c, d) = (rng.interesting_i32(), rng.interesting_i32());
                assert_eq!(
                    p.c_findrep(a, b, c, d),
                    p.r_findrep(a, b, c, d),
                    "row35 step {step} findrep({a},{b},{c},{d})"
                );
            }
        }
    }
}

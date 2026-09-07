// Phase B — valid-path differential tests, one test per CONFIGS.md row.
//
// Every test drives BOTH shared objects through their exported symbols only and
// compares stdout byte-for-byte. Randomized rows use a fixed seed.

mod common;

use common::{Op, Rng, SEED, diff_each_from_pristine, diff_sequence};

// The libtest harness also runs the NULL-pointer worker entry point that lives
// in the error-path file; here it must be a no-op.
fn guard() {
    assert!(
        !common::maybe_act_as_null_worker(),
        "phase B binary should never be used as the null worker"
    );
}

// ---------------------------------------------------------------------- row 1

fn configs_row01_run_zero_from_pristine() {
    guard();
    diff_sequence("row01", &[Op::Run(0)]);
}

// ---------------------------------------------------------------------- row 2

fn configs_row02_run_small_positive() {
    guard();
    let mut rng = Rng::new(SEED ^ 2);
    let ops: Vec<Op> = (0..200)
        .map(|_| Op::Run(rng.range_i64(1, 1000) as i32))
        .collect();
    // Each observed in isolation from pristine state...
    diff_each_from_pristine("row02/pristine", &ops[..40]);
    // ...and accumulated, so state drift is covered too.
    diff_sequence("row02/accumulated", &ops);
}

// ---------------------------------------------------------------------- row 3

fn configs_row03_run_small_negative() {
    guard();
    let mut rng = Rng::new(SEED ^ 3);
    let ops: Vec<Op> = (0..200)
        .map(|_| Op::Run(rng.range_i64(-1000, -1) as i32))
        .collect();
    diff_each_from_pristine("row03/pristine", &ops[..40]);
    diff_sequence("row03/accumulated", &ops);
}

// ---------------------------------------------------------------------- row 4

fn configs_row04_run_full_width_int() {
    guard();
    let mut rng = Rng::new(SEED ^ 4);
    let ops: Vec<Op> = (0..600).map(|_| Op::Run(rng.next_i32())).collect();
    diff_each_from_pristine("row04/pristine", &ops[..60]);
    diff_sequence("row04/accumulated", &ops);
}

// ---------------------------------------------------------------------- row 5

fn configs_row05_run_exact_boundaries() {
    guard();
    let edges = [
        i32::MAX,
        i32::MIN,
        1,
        -1,
        0,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        -5,
        5,
    ];
    let ops: Vec<Op> = edges.iter().copied().map(Op::Run).collect();
    diff_each_from_pristine("row05/pristine", &ops);
    diff_sequence("row05/accumulated", &ops);
    // ...and every ordered pair, to catch order-dependent overflow handling.
    for a in edges {
        for b in edges {
            diff_sequence("row05/pairs", &[Op::Run(a), Op::Run(b)]);
        }
    }
}

// ---------------------------------------------------------------------- row 6

fn configs_row06_run_repeated_counts() {
    guard();
    let mut rng = Rng::new(SEED ^ 6);
    for count in [2usize, 3, 8, 64, 257] {
        let ops: Vec<Op> = (0..count).map(|_| Op::Run(rng.next_i32())).collect();
        diff_sequence(&format!("row06/count={count}"), &ops);
    }
}

// ---------------------------------------------------------------------- row 7

fn configs_row07_run_long_run_accumulation() {
    guard();
    let mut rng = Rng::new(SEED ^ 7);
    let ops: Vec<Op> = (0..1000)
        .map(|_| Op::Run(rng.range_i64(i32::MIN as i64, i32::MAX as i64) as i32))
        .collect();
    diff_sequence("row07", &ops);
}

// ---------------------------------------------------------------------- row 8

fn configs_row08_driver_canonical_decimal() {
    guard();
    let mut rng = Rng::new(SEED ^ 8);
    let ops: Vec<Op> = (0..500)
        .map(|_| {
            let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
            Op::driver(&format!("{v}"))
        })
        .collect();
    diff_each_from_pristine("row08/pristine", &ops[..60]);
    diff_sequence("row08/accumulated", &ops);
}

// ---------------------------------------------------------------------- row 9

fn configs_row09_driver_explicit_plus_sign() {
    guard();
    let mut rng = Rng::new(SEED ^ 9);
    let ops: Vec<Op> = (0..300)
        .map(|_| {
            let v = rng.range_i64(0, i32::MAX as i64);
            Op::driver(&format!("+{v}"))
        })
        .collect();
    diff_each_from_pristine("row09/pristine", &ops[..40]);
    diff_sequence("row09/accumulated", &ops);
}

// --------------------------------------------------------------------- row 10

fn configs_row10_driver_explicit_minus_sign() {
    guard();
    let mut rng = Rng::new(SEED ^ 10);
    let ops: Vec<Op> = (0..300)
        .map(|_| {
            let v = rng.range_i64(0, -(i32::MIN as i64));
            // -(INT_MIN) is representable in i64; "-2147483648" is the extreme.
            let v = v.min(-(i32::MIN as i64));
            Op::driver(&format!("-{v}"))
        })
        .collect();
    diff_each_from_pristine("row10/pristine", &ops[..40]);
    diff_sequence("row10/accumulated", &ops);
}

// --------------------------------------------------------------------- row 11

fn configs_row11_driver_leading_whitespace() {
    guard();
    let mut rng = Rng::new(SEED ^ 11);
    let ws = [' ', '\t', '\n', '\u{b}', '\u{c}', '\r'];
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let n = rng.range_i64(1, 6) as usize;
            let prefix: String = (0..n).map(|_| *rng.pick(&ws)).collect();
            let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
            let sign = if rng.chance(3) && v >= 0 { "+" } else { "" };
            Op::driver(&format!("{prefix}{sign}{v}"))
        })
        .collect();
    diff_each_from_pristine("row11/pristine", &ops[..50]);
    diff_sequence("row11/accumulated", &ops);
}

// --------------------------------------------------------------------- row 12

fn configs_row12_driver_leading_zeros() {
    guard();
    let mut rng = Rng::new(SEED ^ 12);
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let zeros = "0".repeat(rng.range_i64(1, 40) as usize);
            let v = rng.range_i64(0, i32::MAX as i64);
            let sign = *rng.pick(&["", "+", "-"]);
            Op::driver(&format!("{sign}{zeros}{v}"))
        })
        .collect();
    diff_each_from_pristine("row12/pristine", &ops[..50]);
    diff_sequence("row12/accumulated", &ops);
}

// --------------------------------------------------------------------- row 13

fn configs_row13_driver_trailing_garbage_accepted() {
    guard();
    let mut rng = Rng::new(SEED ^ 13);
    let tails = [
        "abc", " ", "\t7", ".9", "e10", "x10", "-", "+", "!!", " 12", ",", "\n\n", "0x1", "L",
    ];
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
            Op::driver(&format!("{v}{}", rng.pick(&tails)))
        })
        .collect();
    diff_each_from_pristine("row13/pristine", &ops[..50]);
    diff_sequence("row13/accumulated", &ops);

    // Explicitly pin the documented quirks (the C accepts these).
    for s in ["12abc", "5 5", "0x10", "1.9", "  -3junk", "007tail"] {
        common::assert_accepted_by_c(s);
        diff_sequence("row13/quirks", &[Op::driver(s), Op::Run(0)]);
    }
}

// --------------------------------------------------------------------- row 14

fn configs_row14_driver_boundary_literals() {
    guard();
    let lits = [
        "2147483647",
        "-2147483648",
        "+2147483647",
        "0",
        "-0",
        "+0",
        "1",
        "-1",
        "2147483646",
        "-2147483647",
    ];
    let ops: Vec<Op> = lits.iter().map(|s| Op::driver(s)).collect();
    for s in lits {
        common::assert_accepted_by_c(s);
    }
    diff_each_from_pristine("row14/pristine", &ops);
    diff_sequence("row14/accumulated", &ops);
}

// --------------------------------------------------------------------- row 15

fn configs_row15_rejection_leaves_state_untouched() {
    guard();
    common::diff_rejection_leaves_state_intact(
        "row15",
        &[
            "",
            "abc",
            " ",
            "+",
            "-",
            "99999999999999999999",
            "-99999999999999999999",
            "2147483648",
            "-2147483649",
        ],
    );
}

// --------------------------------------------------------------------- row 16

fn configs_row16_interleaved_run_and_driver() {
    guard();
    let mut rng = Rng::new(SEED ^ 16);
    let bad = ["", "abc", "+", "99999999999999999999", "2147483648"];
    let ops: Vec<Op> = (0..600)
        .map(|_| match rng.next_u64() % 4 {
            0 => Op::Run(rng.next_i32()),
            1 => Op::driver(&format!(
                "{}",
                rng.range_i64(i32::MIN as i64, i32::MAX as i64)
            )),
            2 => Op::driver(rng.pick(&bad)),
            _ => Op::driver(&format!("  {}xyz", rng.range_i64(-99999, 99999))),
        })
        .collect();
    diff_sequence("row16", &ops);
}

// --------------------------------------------------------------------- row 17

fn configs_row17_driver_repeated_same_input() {
    guard();
    let mut rng = Rng::new(SEED ^ 17);
    for count in [2usize, 5, 20] {
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        let ops: Vec<Op> = (0..count).map(|_| Op::driver(&format!("{v}"))).collect();
        diff_sequence(&format!("row17/count={count}"), &ops);
    }
}

// --------------------------------------------------------------------- row 18

/// Property-style fuzz over the full input grammar the C distinguishes.
fn fuzz_input(rng: &mut Rng) -> String {
    let mut s = String::new();
    // optional whitespace prefix
    if rng.chance(2) {
        let ws = [' ', '\t', '\n', '\u{b}', '\u{c}', '\r'];
        for _ in 0..rng.range_i64(1, 4) {
            s.push(*rng.pick(&ws));
        }
    }
    // optional sign (sometimes doubled, which the C rejects)
    match rng.next_u64() % 8 {
        0 => s.push('+'),
        1 => s.push('-'),
        2 => s.push_str("--"),
        3 => s.push_str("++"),
        _ => {}
    }
    // optional leading zeros
    if rng.chance(4) {
        for _ in 0..rng.range_i64(1, 5) {
            s.push('0');
        }
    }
    // the digit body: sometimes empty (rejected), sometimes huge (ERANGE)
    match rng.next_u64() % 10 {
        0 => {} // no digits at all
        1 => {
            let n = rng.range_i64(20, 60);
            for _ in 0..n {
                s.push((b'0' + (rng.next_u64() % 10) as u8) as char);
            }
        }
        2 => s.push_str(&format!("{}", rng.range_i64(i64::MIN / 2, i64::MAX / 2))),
        3 => s.push_str(*rng.pick(&[
            "2147483647",
            "2147483648",
            "-2147483648",
            "-2147483649",
            "9223372036854775807",
            "-9223372036854775808",
            "9223372036854775808",
            "4294967296",
        ])),
        _ => s.push_str(&format!(
            "{}",
            rng.range_i64(i32::MIN as i64, i32::MAX as i64)
        )),
    }
    // optional trailing garbage
    if rng.chance(3) {
        s.push_str(*rng.pick(&["abc", " ", ".5", "e9", "x", "!", "\t", "-1"]));
    }
    s
}

fn configs_row18_property_fuzz_full_grammar() {
    guard();
    let mut rng = Rng::new(SEED ^ 18);
    let ops: Vec<Op> = (0..4000).map(|_| Op::driver(&fuzz_input(&mut rng))).collect();
    diff_sequence("row18", &ops);
}

// --------------------------------------------------------------------- row 19

fn configs_row19_bathrooms_half_tie_rounding_sweep() {
    guard();
    // `bathrooms` starts at 2.5 and gains exactly 1.0 per `run`, so every call
    // prints a `.5` value with `%.1f` at a growing magnitude; `floors` and
    // `bedrooms` are held boring so the float formatting is what varies.
    let ops: Vec<Op> = (0..1000).map(|_| Op::Run(0)).collect();
    diff_sequence("row19", &ops);
}

// --------------------------------------------------------------------- row 20

fn configs_row20_oversized_inputs() {
    guard();
    let mut rng = Rng::new(SEED ^ 20);
    let mut ops = Vec::new();
    for digits in [1usize, 2, 10, 19, 20, 100, 4096] {
        let mut s = String::new();
        // first digit non-zero so the length is meaningful
        s.push((b'1' + (rng.next_u64() % 9) as u8) as char);
        for _ in 1..digits {
            s.push((b'0' + (rng.next_u64() % 10) as u8) as char);
        }
        ops.push(Op::Driver(s.as_bytes().to_vec()));
        ops.push(Op::Driver(format!("-{s}").into_bytes()));
    }
    // oversized non-digit input
    ops.push(Op::Driver(vec![b'z'; 4096]));
    ops.push(Op::Driver(
        std::iter::repeat(b' ').take(4096).collect::<Vec<u8>>(),
    ));
    diff_each_from_pristine("row20/pristine", &ops);
    diff_sequence("row20/accumulated", &ops);
}

// ------------------------------------------------------------------- runner

fn main() -> ! {
    common::run_suite(
        "phase B (CONFIGS.md rows)",
        &[
            ("configs_row01_run_zero_from_pristine", configs_row01_run_zero_from_pristine as common::TestFn),
            ("configs_row02_run_small_positive", configs_row02_run_small_positive as common::TestFn),
            ("configs_row03_run_small_negative", configs_row03_run_small_negative as common::TestFn),
            ("configs_row04_run_full_width_int", configs_row04_run_full_width_int as common::TestFn),
            ("configs_row05_run_exact_boundaries", configs_row05_run_exact_boundaries as common::TestFn),
            ("configs_row06_run_repeated_counts", configs_row06_run_repeated_counts as common::TestFn),
            ("configs_row07_run_long_run_accumulation", configs_row07_run_long_run_accumulation as common::TestFn),
            ("configs_row08_driver_canonical_decimal", configs_row08_driver_canonical_decimal as common::TestFn),
            ("configs_row09_driver_explicit_plus_sign", configs_row09_driver_explicit_plus_sign as common::TestFn),
            ("configs_row10_driver_explicit_minus_sign", configs_row10_driver_explicit_minus_sign as common::TestFn),
            ("configs_row11_driver_leading_whitespace", configs_row11_driver_leading_whitespace as common::TestFn),
            ("configs_row12_driver_leading_zeros", configs_row12_driver_leading_zeros as common::TestFn),
            ("configs_row13_driver_trailing_garbage_accepted", configs_row13_driver_trailing_garbage_accepted as common::TestFn),
            ("configs_row14_driver_boundary_literals", configs_row14_driver_boundary_literals as common::TestFn),
            ("configs_row15_rejection_leaves_state_untouched", configs_row15_rejection_leaves_state_untouched as common::TestFn),
            ("configs_row16_interleaved_run_and_driver", configs_row16_interleaved_run_and_driver as common::TestFn),
            ("configs_row17_driver_repeated_same_input", configs_row17_driver_repeated_same_input as common::TestFn),
            ("configs_row18_property_fuzz_full_grammar", configs_row18_property_fuzz_full_grammar as common::TestFn),
            ("configs_row19_bathrooms_half_tie_rounding_sweep", configs_row19_bathrooms_half_tie_rounding_sweep as common::TestFn),
            ("configs_row20_oversized_inputs", configs_row20_oversized_inputs as common::TestFn),
        ],
    )
}

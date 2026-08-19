// PHASE B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Every test drives BOTH the C `.so` and the Rust `.so` through their exported
// symbols and asserts byte-identical stdout. Randomized rows use a fixed seed.
//
// Rows 1-11 exercise the LOW-LEVEL entry point `run` directly (it is exported
// even though it is absent from driver.h); rows 12-24 exercise `driver`, which
// composes parse + `run` + `run`, including sequences that interleave both
// entry points against the shared `static the_house`.

mod common;
use common::*;

// Whitespace bytes that `strtol` skips (C `isspace` in the "C" locale).
const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

// ===========================================================================
// Rows 1-11: low-level entry point `run`
// ===========================================================================

#[test]
fn row01_run_pristine_zero() {
    // Identity add: bedrooms += 0.
    assert_same("row01_run_0", &[Op::Run(0)]);
}

#[test]
fn row02_run_pristine_one() {
    assert_same("row02_run_1", &[Op::Run(1)]);
}

#[test]
fn row03_run_small_positive_randomized() {
    let mut rng = Rng::new(0x0301);
    for i in 0..128 {
        let v = rng.range_i32(1, 1000);
        assert_same(&format!("row03_i{}_v{}", i, v), &[Op::Run(v)]);
    }
}

#[test]
fn row04_run_small_negative_randomized() {
    // Drives `bedrooms` negative (5 + (-1000) = -995), exercising `%d` on a
    // negative value.
    let mut rng = Rng::new(0x0401);
    for i in 0..128 {
        let v = rng.range_i32(-1000, -1);
        assert_same(&format!("row04_i{}_v{}", i, v), &[Op::Run(v)]);
    }
}

#[test]
fn row05_run_int_max_signed_overflow() {
    // 5 + INT_MAX overflows; gcc -O0 wraps. The Rust translation must wrap the
    // same way.
    assert_same("row05_run_INT_MAX", &[Op::Run(i32::MAX)]);
}

#[test]
fn row06_run_int_min_signed_underflow() {
    assert_same("row06_run_INT_MIN", &[Op::Run(i32::MIN)]);
}

#[test]
fn row07_run_full_int_domain_randomized() {
    // Any `int` bit pattern is a legal argument across the FFI boundary.
    let mut rng = Rng::new(0x0701);
    for i in 0..512 {
        let v = rng.next_i32();
        assert_same(&format!("row07_i{}", i), &[Op::Run(v)]);
    }
}

#[test]
fn row08_run_long_randomized_sequence_accumulating_state() {
    // 256 chained calls on one evolving `the_house`. Any state drift between the
    // two implementations shows up in the transcript at the op where it starts.
    let mut rng = Rng::new(0x0801);
    let ops: Vec<Op> = (0..256)
        .map(|_| {
            let v = match rng.below(4) {
                0 => rng.range_i32(-50, 50),
                1 => rng.range_i32(-1_000_000, 1_000_000),
                2 => rng.next_i32(),
                _ => *rng.pick(&[0, 1, -1, i32::MAX, i32::MIN]),
            };
            Op::Run(v)
        })
        .collect();
    assert_same("row08_run_seq256", &ops);
}

#[test]
fn row09_run_repeated_overflow() {
    // Repeated wraparound of `bedrooms` in both directions.
    let mut rng = Rng::new(0x0901);
    let ops: Vec<Op> = (0..96)
        .map(|_| Op::Run(*rng.pick(&[i32::MAX, i32::MIN])))
        .collect();
    assert_same("row09_run_repeat_overflow", &ops);
}

#[test]
fn row10_run_bathrooms_double_growth_2000() {
    // bathrooms: 2.5 -> 2002.5 and floors: 2 -> 2002, checking `%.1f` on a
    // growing double and `%d` on a growing int.
    let ops: Vec<Op> = (0..2000).map(|_| Op::Run(0)).collect();
    assert_same("row10_run_2000", &ops);
}

#[test]
fn row11_run_at_scale_20000() {
    // Larger accumulation; ~4 MB of transcript per implementation.
    let mut rng = Rng::new(0x1101);
    let ops: Vec<Op> = (0..20_000)
        .map(|_| Op::Run(rng.range_i32(-3, 3)))
        .collect();
    assert_same("row11_run_20000", &ops);
}

// ===========================================================================
// Rows 12-24: high-level entry point `driver`
// ===========================================================================

#[test]
fn row12_driver_plain_decimal_randomized() {
    let mut rng = Rng::new(0x1201);
    for i in 0..512 {
        let v = rng.next_i32();
        let s = format!("{}", v);
        assert_same(&format!("row12_i{}", i), &[Op::driver(&s)]);
    }
}

#[test]
fn row13_driver_zero_both_signs() {
    assert_same("row13_zero", &[Op::driver("0")]);
    assert_same("row13_negzero", &[Op::driver("-0")]);
    assert_same("row13_poszero", &[Op::driver("+0")]);
}

#[test]
fn row14_driver_explicit_plus_sign_randomized() {
    let mut rng = Rng::new(0x1401);
    for i in 0..128 {
        let v = rng.range_i32(0, i32::MAX);
        let s = format!("+{}", v);
        assert_same(&format!("row14_i{}", i), &[Op::driver(&s)]);
    }
}

#[test]
fn row15_driver_explicit_minus_sign_randomized() {
    let mut rng = Rng::new(0x1501);
    for i in 0..128 {
        let v = rng.range_i32(i32::MIN, 0);
        // format! already emits '-' for negatives; use the magnitude explicitly
        // so INT_MIN is covered too.
        let s = format!("-{}", (v as i64).unsigned_abs());
        assert_same(&format!("row15_i{}", i), &[Op::driver(&s)]);
    }
}

#[test]
fn row16_driver_leading_whitespace_randomized() {
    let mut rng = Rng::new(0x1601);
    for i in 0..128 {
        let n = rng.below(5) as usize + 1;
        let mut s: Vec<u8> = (0..n).map(|_| *rng.pick(&WS)).collect();
        let v = rng.next_i32();
        s.extend_from_slice(format!("{}", v).as_bytes());
        assert_same(&format!("row16_i{}", i), &[Op::driver_bytes(&s)]);
    }
    // Every whitespace byte, all at once.
    assert_same(
        "row16_all_ws",
        &[Op::driver_bytes(b"\t\n\x0b\x0c\r 42")],
    );
}

#[test]
fn row17_driver_leading_zeros_base10_not_octal() {
    // "007" must parse as 7 (base 10), NOT as octal.
    assert_same("row17_007", &[Op::driver("007")]);
    assert_same("row17_010", &[Op::driver("010")]);
    assert_same("row17_long", &[Op::driver("0000000000042")]);
    assert_same("row17_neg", &[Op::driver("-00000123")]);
    let mut rng = Rng::new(0x1701);
    for i in 0..64 {
        let zeros = "0".repeat(rng.below(12) as usize + 1);
        let v = rng.range_i32(0, 100_000);
        let s = format!("{}{}", zeros, v);
        assert_same(&format!("row17_i{}", i), &[Op::driver(&s)]);
    }
}

#[test]
fn row18_driver_trailing_garbage_is_accepted() {
    // The C guard only requires `endp != str`, so partial consumption SUCCEEDS.
    for s in [
        "42abc", "7 8", "1,000", "5-", "0x1F", "12.75", "3e4", "99]", "8\t9", "1_000",
    ] {
        assert_same(&format!("row18_{}", s), &[Op::driver(s)]);
    }
    let mut rng = Rng::new(0x1801);
    let tails = ["abc", "!", " x", ".5", "e9", "-", "+", "\t", "]", "zzz"];
    for i in 0..128 {
        let v = rng.next_i32();
        let s = format!("{}{}", v, rng.pick(&tails));
        assert_same(&format!("row18_i{}", i), &[Op::driver(&s)]);
    }
}

#[test]
fn row19_driver_combined_shapes_randomized() {
    // whitespace + sign + leading zeros + digits + trailing garbage, all mixed.
    let mut rng = Rng::new(0x1901);
    let tails = ["", "abc", "!!", " 7", ".25", "e-3", "]", "\t"];
    for i in 0..256 {
        let mut s: Vec<u8> = Vec::new();
        for _ in 0..rng.below(4) {
            s.push(*rng.pick(&WS));
        }
        match rng.below(3) {
            0 => s.push(b'+'),
            1 => s.push(b'-'),
            _ => {}
        }
        for _ in 0..rng.below(4) {
            s.push(b'0');
        }
        s.extend_from_slice(format!("{}", rng.range_i32(0, 2_147_483_647)).as_bytes());
        s.extend_from_slice(rng.pick(&tails).as_bytes());
        assert_same(&format!("row19_i{}", i), &[Op::driver_bytes(&s)]);
    }
}

#[test]
fn row20_driver_int_boundaries_must_succeed() {
    // Exactly INT_MAX / INT_MIN: inside the range, so `driver` runs twice and
    // `bedrooms` overflows on each pass.
    assert_same("row20_INT_MAX", &[Op::driver("2147483647")]);
    assert_same("row20_INT_MIN", &[Op::driver("-2147483648")]);
    assert_same("row20_INT_MAX_ws", &[Op::driver("  +2147483647")]);
    assert_same("row20_INT_MIN_zeros", &[Op::driver("-0002147483648")]);
}

#[test]
fn row21_driver_near_boundary_randomized() {
    let mut rng = Rng::new(0x2101);
    for i in 0..128 {
        let off = rng.range_i32(0, 64) as i64;
        let v = if rng.below(2) == 0 {
            i32::MAX as i64 - off
        } else {
            i32::MIN as i64 + off
        };
        assert_same(&format!("row21_i{}", i), &[Op::driver(&format!("{}", v))]);
    }
}

#[test]
fn row22_driver_long_sequence_accumulating_state() {
    // 256 `driver` calls = 512 `run` passes on one evolving state.
    let mut rng = Rng::new(0x2201);
    let ops: Vec<Op> = (0..256)
        .map(|_| {
            let v = match rng.below(3) {
                0 => rng.range_i32(-100, 100),
                1 => rng.next_i32(),
                _ => *rng.pick(&[0, 1, -1, i32::MAX, i32::MIN]),
            };
            Op::driver(&format!("{}", v))
        })
        .collect();
    assert_same("row22_driver_seq256", &ops);
}

#[test]
fn row23_driver_and_run_interleaved() {
    // The composed pipeline: both entry points mutating the same `the_house`.
    let mut rng = Rng::new(0x2301);
    let ops: Vec<Op> = (0..512)
        .map(|_| {
            if rng.below(2) == 0 {
                Op::Run(rng.next_i32())
            } else {
                Op::driver(&format!("{}", rng.next_i32()))
            }
        })
        .collect();
    assert_same("row23_interleaved", &ops);
}

#[test]
fn row24_driver_valid_and_invalid_interleaved() {
    // Error and success paths alternate against evolving state. A rejected input
    // must leave `the_house` untouched in BOTH implementations — otherwise the
    // transcripts drift apart from that point on.
    let mut rng = Rng::new(0x2401);
    let bad = [
        "",
        "abc",
        "   ",
        "+",
        "-",
        "2147483648",
        "-2147483649",
        "9223372036854775807",
        "-9223372036854775808",
        "9999999999999999999999",
        "x1",
        ".",
    ];
    let ops: Vec<Op> = (0..512)
        .map(|_| match rng.below(3) {
            0 => Op::driver(*rng.pick(&bad)),
            1 => Op::Run(rng.next_i32()),
            _ => Op::driver(&format!("{}", rng.next_i32())),
        })
        .collect();
    assert_same("row24_mixed_valid_invalid", &ops);
}

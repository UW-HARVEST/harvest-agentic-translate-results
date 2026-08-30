//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1 … C29). Every test drives BOTH the C
//! `.so` and the Rust `.so` through their exported `driver` symbol and compares
//! the emitted byte streams.

mod common;

use common::*;
use std::ffi::c_int;

const IMIN: c_int = i32::MIN;
const IMAX: c_int = i32::MAX;

// --------------------------------------------------------------------------
// C1 – C3 : degenerate / identity shapes
// --------------------------------------------------------------------------

#[test]
fn c1_zero_zero() {
    // 0 | ~0 == 0 | -1 == -1
    diff_one_expect(0, 0, "-1\n");
}

#[test]
fn c2_result_is_zero() {
    // 0 | ~(-1) == 0 | 0 == 0  -- the only shape that prints "0"
    diff_one_expect(0, -1, "0\n");
}

#[test]
fn c3_all_bits_x_absorbing_y() {
    diff_one_expect(-1, 0, "-1\n");
}

// --------------------------------------------------------------------------
// C4 – C7 : one operand pinned to an absorbing / identity value
// --------------------------------------------------------------------------

#[test]
fn c4_y_zero_absorbs() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..256 {
        let x = rng.next_i32();
        diff_one_expect(x, 0, "-1\n");
    }
    for x in [0, 1, -1, IMIN, IMAX, IMIN + 1, IMAX - 1] {
        diff_one_expect(x, 0, "-1\n");
    }
}

#[test]
fn c5_x_zero_is_complement_of_y() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..256 {
        let y = rng.next_i32();
        diff_one_expect(0, y, &format!("{}\n", !y));
    }
}

#[test]
fn c6_y_minus_one_is_identity_on_x() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..256 {
        let x = rng.next_i32();
        diff_one_expect(x, -1, &format!("{x}\n"));
    }
    for x in [0, 1, -1, IMIN, IMAX] {
        diff_one_expect(x, -1, &format!("{x}\n"));
    }
}

#[test]
fn c7_x_all_bits_absorbs() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..256 {
        let y = rng.next_i32();
        diff_one_expect(-1, y, "-1\n");
    }
}

// --------------------------------------------------------------------------
// C8 – C10 : extreme results
// --------------------------------------------------------------------------

#[test]
fn c8_result_int_max_widest_positive() {
    diff_one_expect(IMAX, IMIN, "2147483647\n");
}

#[test]
fn c9_result_int_min_widest_negative() {
    diff_one_expect(IMIN, IMAX, "-2147483648\n");
}

#[test]
fn c10_extremes_paired_with_themselves() {
    diff_one_expect(IMAX, IMAX, "-1\n");
    diff_one_expect(IMIN, IMIN, "-1\n");
}

// --------------------------------------------------------------------------
// C11 – C14 : randomized, per sign quadrant
// --------------------------------------------------------------------------

#[test]
fn c11_random_pos_pos() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..512 {
        let (x, y) = (rng.next_pos(), rng.next_pos());
        assert!(x > 0 && y > 0);
        diff_one_expect(x, y, &expected_text(x, y));
    }
}

#[test]
fn c12_random_pos_neg() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..512 {
        let (x, y) = (rng.next_pos(), rng.next_neg());
        assert!(x > 0 && y < 0);
        diff_one_expect(x, y, &expected_text(x, y));
    }
}

#[test]
fn c13_random_neg_pos() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..512 {
        let (x, y) = (rng.next_neg(), rng.next_pos());
        assert!(x < 0 && y > 0);
        diff_one_expect(x, y, &expected_text(x, y));
    }
}

#[test]
fn c14_random_neg_neg() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..512 {
        let (x, y) = (rng.next_neg(), rng.next_neg());
        assert!(x < 0 && y < 0);
        diff_one_expect(x, y, &expected_text(x, y));
    }
}

// --------------------------------------------------------------------------
// C15 / C16 : every decimal output width, both signs
// --------------------------------------------------------------------------

fn decimal_boundaries() -> Vec<i64> {
    let mut v = vec![0i64];
    let mut p = 1i64;
    for _ in 0..10 {
        v.push(p - 1);
        v.push(p);
        v.push(p + 1);
        p *= 10;
    }
    v.push(i32::MAX as i64);
    v.push(i32::MAX as i64 - 1);
    v
}

#[test]
fn c15_positive_widths_1_to_10() {
    for r in decimal_boundaries() {
        if r < 0 || r > i32::MAX as i64 {
            continue;
        }
        let result = r as i32;
        // x = 0, y = !result  =>  0 | ~(!result) == result
        diff_one_expect(0, !result, &format!("{result}\n"));
    }
}

#[test]
fn c16_negative_widths_1_to_10() {
    let mut targets: Vec<i64> = decimal_boundaries().into_iter().map(|v| -v).collect();
    targets.push(i32::MIN as i64);
    targets.push(i32::MIN as i64 + 1);
    for r in targets {
        if r < i32::MIN as i64 || r > 0 {
            continue;
        }
        let result = r as i32;
        diff_one_expect(0, !result, &format!("{result}\n"));
    }
}

// --------------------------------------------------------------------------
// C17 / C18 : every bit position
// --------------------------------------------------------------------------

#[test]
fn c17_single_bit_x_every_position() {
    for b in 0..32 {
        let x = 1i32.wrapping_shl(b);
        for y in [0, -1, IMIN, IMAX, 1, -2] {
            diff_one_expect(x, y, &expected_text(x, y));
        }
    }
}

#[test]
fn c18_single_bit_y_every_position() {
    for b in 0..32 {
        let y = 1i32.wrapping_shl(b);
        for x in [0, -1, 1, IMIN, IMAX, -2] {
            diff_one_expect(x, y, &expected_text(x, y));
        }
    }
}

// --------------------------------------------------------------------------
// C19 : correlated operand pairs
// --------------------------------------------------------------------------

#[test]
fn c19_correlated_pairs() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..256 {
        let x = rng.next_i32();
        // y == x
        diff_one_expect(x, x, &expected_text(x, x));
        // y == ~x  =>  x | ~~x == x | x == x
        diff_one_expect(x, !x, &format!("{x}\n"));
        // y == -x (wrapping, so INT_MIN is exercised rather than trapping)
        let ny = x.wrapping_neg();
        diff_one_expect(x, ny, &expected_text(x, ny));
    }
}

// --------------------------------------------------------------------------
// C20 / C21 : exhaustive small cross-products
// --------------------------------------------------------------------------

#[test]
fn c20_boundary_cross_product() {
    let vals = [IMIN, IMIN + 1, -2, -1, 0, 1, 2, IMAX - 1, IMAX];
    let mut n = 0;
    for &x in &vals {
        for &y in &vals {
            diff_one_expect(x, y, &expected_text(x, y));
            n += 1;
        }
    }
    assert_eq!(n, 81);
}

#[test]
fn c21_one_past_narrow_width_cross_product() {
    let vals: [c_int; 14] = [
        -32769, -32768, -129, -128, -1, 0, 127, 128, 255, 256, 32767, 32768, 65535, 65536,
    ];
    let mut pairs = Vec::new();
    for &x in &vals {
        for &y in &vals {
            pairs.push((x, y));
        }
    }
    assert_eq!(pairs.len(), 196);
    // Compared both one-shot-per-call and as one accumulated stream.
    for &(x, y) in &pairs {
        diff_one_expect(x, y, &expected_text(x, y));
    }
    diff_batch(&pairs);
}

// --------------------------------------------------------------------------
// C22 : unconstrained randomized sweep
// --------------------------------------------------------------------------

#[test]
fn c22_random_full_domain_sweep() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..4096 {
        let (x, y) = (rng.next_i32(), rng.next_i32());
        diff_one_expect(x, y, &expected_text(x, y));
    }
}

// --------------------------------------------------------------------------
// C23 / C24 : sequencing and stdio buffer accumulation
// --------------------------------------------------------------------------

#[test]
fn c23_two_hundred_calls_single_capture() {
    let mut rng = Rng::new(SEED ^ 23);
    let pairs: Vec<(c_int, c_int)> = (0..200).map(|_| (rng.next_i32(), rng.next_i32())).collect();
    diff_batch(&pairs);

    // and the aggregate text must be the concatenation of the per-call texts
    let f = impls();
    let got = capture(|| unsafe {
        for &(x, y) in &pairs {
            (f.c)(x, y);
        }
    });
    let want: String = pairs.iter().map(|&(x, y)| expected_text(x, y)).collect();
    assert_eq!(show(&got), want);
}

#[test]
fn c24_many_calls_crossing_stdio_buffer() {
    let mut rng = Rng::new(SEED ^ 24);
    let pairs: Vec<(c_int, c_int)> = (0..2000).map(|_| (rng.next_i32(), rng.next_i32())).collect();
    let total: usize = pairs.iter().map(|&(x, y)| expected_text(x, y).len()).sum();
    assert!(total > 4096, "batch must exceed the 4 KiB stdio buffer, got {total}");
    diff_batch(&pairs);

    // Same batch under each explicit stdio buffering mode.
    diff_batch_mode(BufMode::Full, &pairs);
    diff_batch_mode(BufMode::Line, &pairs[..200]);
    diff_batch_mode(BufMode::None, &pairs[..200]);
}

// --------------------------------------------------------------------------
// C25 : C and Rust interleaved into the same stdout FILE
// --------------------------------------------------------------------------

#[test]
fn c25_interleaved_c_and_rust_same_stream() {
    let f = impls();
    let mut rng = Rng::new(SEED ^ 25);
    let pairs: Vec<(c_int, c_int)> = (0..300).map(|_| (rng.next_i32(), rng.next_i32())).collect();

    let all_c = capture(|| unsafe {
        for &(x, y) in &pairs {
            (f.c)(x, y);
        }
    });
    let all_rust = capture(|| unsafe {
        for &(x, y) in &pairs {
            (f.rust)(x, y);
        }
    });
    let c_first = capture(|| unsafe {
        for (i, &(x, y)) in pairs.iter().enumerate() {
            if i % 2 == 0 {
                (f.c)(x, y);
            } else {
                (f.rust)(x, y);
            }
        }
    });
    let rust_first = capture(|| unsafe {
        for (i, &(x, y)) in pairs.iter().enumerate() {
            if i % 2 == 0 {
                (f.rust)(x, y);
            } else {
                (f.c)(x, y);
            }
        }
    });

    assert_eq!(all_c, all_rust, "all-C vs all-Rust stream differ");
    assert_eq!(all_c, c_first, "C/Rust alternating stream differs from all-C");
    assert_eq!(all_c, rust_first, "Rust/C alternating stream differs from all-C");

    let want: String = pairs.iter().map(|&(x, y)| expected_text(x, y)).collect();
    assert_eq!(show(&all_c), want);
}

// --------------------------------------------------------------------------
// C26 – C28 : stdout sink shapes
// --------------------------------------------------------------------------

#[test]
fn c26_sink_regular_file() {
    let mut rng = Rng::new(SEED ^ 26);
    let pairs: Vec<(c_int, c_int)> = (0..300).map(|_| (rng.next_i32(), rng.next_i32())).collect();
    diff_batch_mode(BufMode::Full, &pairs);
}

#[test]
fn c27_sink_pipe_non_seekable() {
    let mut rng = Rng::new(SEED ^ 27);
    // Keep well under the 64 KiB pipe capacity.
    let pairs: Vec<(c_int, c_int)> = (0..1000).map(|_| (rng.next_i32(), rng.next_i32())).collect();
    let total: usize = pairs.iter().map(|&(x, y)| expected_text(x, y).len()).sum();
    assert!(total < 60_000);
    diff_batch_pipe(&pairs);

    let f = impls();
    let got = capture_via_pipe(|| unsafe {
        for &(x, y) in &pairs {
            (f.rust)(x, y);
        }
    });
    let want: String = pairs.iter().map(|&(x, y)| expected_text(x, y)).collect();
    assert_eq!(show(&got), want);
}

#[test]
fn c28_sink_dev_null_character_device() {
    let f = impls();
    let mut rng = Rng::new(SEED ^ 28);
    let pairs: Vec<(c_int, c_int)> = (0..500).map(|_| (rng.next_i32(), rng.next_i32())).collect();

    run_to_dev_null(|| unsafe {
        for &(x, y) in &pairs {
            (f.c)(x, y);
        }
    });
    run_to_dev_null(|| unsafe {
        for &(x, y) in &pairs {
            (f.rust)(x, y);
        }
    });

    // And the stream is still healthy afterwards, identically for both.
    diff_one_expect(7, -9, &expected_text(7, -9));
}

// --------------------------------------------------------------------------
// C29 : statelessness (there is no runtime option to configure)
// --------------------------------------------------------------------------

#[test]
fn c29_stateless_repeated_identical_calls() {
    let f = impls();
    let (x, y) = (0x1234_5678, -0x0765_4321);
    let first_c = capture(|| unsafe { (f.c)(x, y) });
    let first_r = capture(|| unsafe { (f.rust)(x, y) });
    assert_eq!(first_c, first_r);
    for i in 0..100 {
        let c = capture(|| unsafe { (f.c)(x, y) });
        let r = capture(|| unsafe { (f.rust)(x, y) });
        assert_eq!(c, first_c, "C drifted on repeat #{i}");
        assert_eq!(r, first_r, "Rust drifted on repeat #{i}");
    }

    // Structural half of the row: the C library exposes exactly one entry point,
    // so there is no option-setter that could have been missed.
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(c_so_path())
        .output()
        .expect("run nm");
    let names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    assert_eq!(names, vec!["driver".to_string()], "unexpected C export surface: {names:?}");
}

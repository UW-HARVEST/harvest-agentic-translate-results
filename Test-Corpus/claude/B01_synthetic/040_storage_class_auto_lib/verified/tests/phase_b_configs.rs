// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md.  Every test drives BOTH shared objects
// through their exported `driver` symbol and compares stdout byte-for-byte.
// The single public entry point of the C library (`void driver(int)`,
// c_src/include/driver.h) is also its lowest-level entry point, so every test
// exercises it directly.

mod common;

use common::{
    assert_same, assert_same_transcript, assert_same_wide, capture_stdout_via_pipe, impls, Rng,
};

const I32_MIN: i64 = i32::MIN as i64;
const I32_MAX: i64 = i32::MAX as i64;

/// x such that `2*x + 300` lands as close as possible to `target`.
fn x_for(target: i64) -> i32 {
    let x = (target - 300) / 2;
    x.clamp(I32_MIN, I32_MAX) as i32
}

// --- row 1 -----------------------------------------------------------------

fn cfg_01_zero() {
    let out = assert_same(0);
    assert_eq!(out, b"300\n");
}

// --- row 2 -----------------------------------------------------------------

fn cfg_02_small_positive() {
    for x in 1..=32 {
        assert_same(x);
    }
    let mut rng = Rng::new(0x0000_0002_5EED_0001);
    for _ in 0..64 {
        assert_same(rng.in_range(1, 1000));
    }
}

// --- row 3 -----------------------------------------------------------------

fn cfg_03_small_negative() {
    for x in 1..=32 {
        assert_same(-x);
    }
    let mut rng = Rng::new(0x0000_0003_5EED_0001);
    for _ in 0..64 {
        assert_same(rng.in_range(-1000, -1));
    }
}

// --- row 4 -----------------------------------------------------------------

fn cfg_04_sign_crossing_window() {
    for x in -160..=-140 {
        assert_same(x);
    }
    // exact zero: 2*(-150) + 300 == 0 -> printed without a sign
    assert_eq!(assert_same(-150), b"0\n");
    assert_eq!(assert_same(-151), b"-2\n");
    assert_eq!(assert_same(-149), b"2\n");
}

// --- row 5 -----------------------------------------------------------------

fn cfg_05_all_positive_digit_widths() {
    let mut rng = Rng::new(0x0000_0005_5EED_0001);
    let mut pow = 1i64;
    for _w in 1..=10 {
        let lo = pow;
        let hi = (pow * 10 - 1).min(I32_MAX);
        // a couple of exact boundaries plus randomized interior values
        assert_same(x_for(lo));
        assert_same(x_for(hi));
        for _ in 0..16 {
            let t = lo + (rng.next_u64() % ((hi - lo + 1) as u64)) as i64;
            assert_same(x_for(t));
        }
        pow *= 10;
    }
    // and zero-width special case
    assert_same(x_for(0));
}

// --- row 6 -----------------------------------------------------------------

fn cfg_06_all_negative_digit_widths() {
    let mut rng = Rng::new(0x0000_0006_5EED_0001);
    let mut pow = 1i64;
    for _w in 1..=10 {
        let hi = -pow;
        let lo = (-(pow * 10 - 1)).max(I32_MIN);
        assert_same(x_for(hi));
        assert_same(x_for(lo));
        for _ in 0..8 {
            let t = lo + (rng.next_u64() % ((hi - lo + 1) as u64)) as i64;
            assert_same(x_for(t));
        }
        pow *= 10;
    }
}

// --- row 7 -----------------------------------------------------------------

fn cfg_07_random_full_i32_range() {
    let mut rng = Rng::new(0x0000_0007_5EED_0001);
    for _ in 0..64 {
        assert_same(rng.next_i32());
    }
}

// --- row 8 -----------------------------------------------------------------

fn cfg_08_random_multiply_wraps_positive_side() {
    let mut rng = Rng::new(0x0000_0008_5EED_0001);
    for _ in 0..64 {
        let x = rng.in_range(1_073_741_824, I32_MAX);
        // 2*x overflows i32 here
        assert!(x >= 1_073_741_824);
        assert_same(x);
    }
}

// --- row 9 -----------------------------------------------------------------

fn cfg_09_random_multiply_wraps_negative_side() {
    let mut rng = Rng::new(0x0000_0009_5EED_0001);
    for _ in 0..64 {
        let x = rng.in_range(I32_MIN, -1_073_741_825);
        assert!(x <= -1_073_741_825);
        assert_same(x);
    }
}

// --- row 10 ----------------------------------------------------------------

fn cfg_10_random_addition_wraps() {
    // 2*x fits in i32 but 2*x + 300 does not: x in [1073741674, 1073741823]
    for x in 1_073_741_674..=1_073_741_823 {
        let y2 = (x as i64) * 2;
        assert!(y2 <= I32_MAX && y2 + 300 > I32_MAX);
        assert_same(x);
    }
    let mut rng = Rng::new(0x0000_0010_5EED_0001);
    for _ in 0..32 {
        assert_same(rng.in_range(1_073_741_674, 1_073_741_823));
    }
}

// --- row 11 ----------------------------------------------------------------

fn cfg_11_range_endpoints() {
    let xs = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -1_073_741_825,
        -1_073_741_824,
        -1_073_741_823,
        -1,
        0,
        1,
        1_073_741_823,
        1_073_741_824,
        1_073_741_825,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for x in xs {
        assert_same(x);
    }
}

// --- row 12 ----------------------------------------------------------------

fn cfg_12_powers_of_two() {
    for k in 0..32u32 {
        let p = 1i32.wrapping_shl(k);
        assert_same(p);
        assert_same(p.wrapping_neg());
        assert_same(p.wrapping_sub(1));
        assert_same(p.wrapping_add(1));
    }
}

// --- row 13 ----------------------------------------------------------------

fn cfg_13_many_calls_one_transcript() {
    let mut rng = Rng::new(0x0000_0013_5EED_0001);
    let xs: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    let out = assert_same_transcript(&xs);
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 256);
}

// --- row 14 ----------------------------------------------------------------

fn cfg_14_stdout_is_a_pipe() {
    let im = impls();
    let xs = [0, 1, -1, 42, -150, i32::MIN, i32::MAX, 1_073_741_824];
    for x in xs {
        let out_c = capture_stdout_via_pipe(|| (im.c.driver)(x));
        let out_r = capture_stdout_via_pipe(|| (im.rust.driver)(x));
        assert_eq!(
            out_c,
            out_r,
            "pipe-stdout driver({x}) diverged: C={:?} Rust={:?}",
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r)
        );
        assert_eq!(
            String::from_utf8_lossy(&out_c),
            format!("{}\n", x.wrapping_mul(2).wrapping_add(300))
        );
    }
}

// --- row 15 ----------------------------------------------------------------

fn cfg_15_interleaved_c_and_rust_calls() {
    let im = impls();
    let mut rng = Rng::new(0x0000_0015_5EED_0001);
    let xs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();

    // C then Rust, alternating, all inside ONE capture: both libraries write
    // through the same process-wide libc `stdout`.
    let out = common::capture_stdout(|| {
        for &x in &xs {
            (im.c.driver)(x);
            (im.rust.driver)(x);
        }
    });
    let expected: String = xs
        .iter()
        .map(|x| {
            let line = format!("{}\n", x.wrapping_mul(2).wrapping_add(300));
            format!("{line}{line}")
        })
        .collect();
    assert_eq!(String::from_utf8_lossy(&out), expected);

    // Rust first, then C — order must not matter.
    let out2 = common::capture_stdout(|| {
        for &x in &xs {
            (im.rust.driver)(x);
            (im.c.driver)(x);
        }
    });
    assert_eq!(out, out2);
}

// --- row 16 ----------------------------------------------------------------

fn cfg_16_called_via_wide_argument_abi() {
    let mut rng = Rng::new(0x0000_0016_5EED_0001);
    for _ in 0..32 {
        let x = rng.next_i32();
        // sign-extended (what a C caller passing `int` produces)
        let out = assert_same_wide(x as i64);
        assert_eq!(
            String::from_utf8_lossy(&out),
            format!("{}\n", x.wrapping_mul(2).wrapping_add(300))
        );
        // zero-extended
        let out = assert_same_wide((x as u32) as i64);
        assert_eq!(
            String::from_utf8_lossy(&out),
            format!("{}\n", x.wrapping_mul(2).wrapping_add(300))
        );
    }
}

// --- row 17 ----------------------------------------------------------------

fn cfg_17_property_sweep_1024() {
    let mut rng = Rng::new(0x0000_0017_5EED_0001);
    let xs: Vec<i32> = (0..1024).map(|_| rng.next_i32()).collect();
    assert_same_transcript(&xs);

    // also per-call (not just as one transcript) for a subset, so a difference
    // that only shows up on an isolated call cannot hide
    for &x in xs.iter().take(48) {
        assert_same(x);
    }
}

// --- row 18 ----------------------------------------------------------------

/// Strided sweep across the WHOLE `i32` domain (every 2^11-th value, plus a
/// randomized offset inside each stride so no value class is systematically
/// skipped).  Covers all four wrap regimes and every output width in one go.
fn cfg_18_strided_full_range_sweep() {
    const STEP: i64 = 1 << 11;
    let mut rng = Rng::new(0x0000_0018_5EED_0001);
    let mut xs: Vec<i32> = Vec::new();
    let mut v = I32_MIN;
    while v <= I32_MAX {
        xs.push(v as i32);
        let jitter = (rng.next_u64() % STEP as u64) as i64;
        let j = v + jitter;
        if j <= I32_MAX {
            xs.push(j as i32);
        }
        v += STEP;
    }
    assert!(xs.len() > 4_000_000, "sweep too small: {}", xs.len());
    // Compare as chunked transcripts to keep memory bounded.
    for chunk in xs.chunks(65_536) {
        assert_same_transcript(chunk);
    }
}

// --- row 19 ----------------------------------------------------------------

/// Exhaustive over a contiguous window that straddles the interesting
/// arithmetic boundary (`2*x + 300` crossing `INT_MAX`): every single value in
/// `[1073741674 - 4096, 1073741674 + 4096]` is checked, with no sampling.
fn cfg_19_exhaustive_window_around_wrap_boundary() {
    let center: i64 = 1_073_741_674;
    let xs: Vec<i32> = ((center - 4096)..=(center + 4096)).map(|v| v as i32).collect();
    assert_same_transcript(&xs);

    // and exhaustively around zero-crossing of the printed value
    let xs: Vec<i32> = (-4096..=4096).map(|v| v - 150).collect();
    assert_same_transcript(&xs);
}

// ---------------------------------------------------------------------------
// Single entry point.
//
// All CONFIGS.md rows run as sub-cases of ONE `#[test]`: the capture helper
// hijacks the process-wide fd 1, so no other libtest thread may run
// concurrently.  Every row is still attempted and reported individually by the
// RowRunner.
// ---------------------------------------------------------------------------

#[test]
fn phase_b_all_config_rows() {
    let mut r = common::RowRunner::new("CONFIGS.md");
    r.row("cfg_01_zero", cfg_01_zero);
    r.row("cfg_02_small_positive", cfg_02_small_positive);
    r.row("cfg_03_small_negative", cfg_03_small_negative);
    r.row("cfg_04_sign_crossing_window", cfg_04_sign_crossing_window);
    r.row(
        "cfg_05_all_positive_digit_widths",
        cfg_05_all_positive_digit_widths,
    );
    r.row(
        "cfg_06_all_negative_digit_widths",
        cfg_06_all_negative_digit_widths,
    );
    r.row("cfg_07_random_full_i32_range", cfg_07_random_full_i32_range);
    r.row(
        "cfg_08_random_multiply_wraps_positive_side",
        cfg_08_random_multiply_wraps_positive_side,
    );
    r.row(
        "cfg_09_random_multiply_wraps_negative_side",
        cfg_09_random_multiply_wraps_negative_side,
    );
    r.row("cfg_10_random_addition_wraps", cfg_10_random_addition_wraps);
    r.row("cfg_11_range_endpoints", cfg_11_range_endpoints);
    r.row("cfg_12_powers_of_two", cfg_12_powers_of_two);
    r.row(
        "cfg_13_many_calls_one_transcript",
        cfg_13_many_calls_one_transcript,
    );
    r.row("cfg_14_stdout_is_a_pipe", cfg_14_stdout_is_a_pipe);
    r.row(
        "cfg_15_interleaved_c_and_rust_calls",
        cfg_15_interleaved_c_and_rust_calls,
    );
    r.row(
        "cfg_16_called_via_wide_argument_abi",
        cfg_16_called_via_wide_argument_abi,
    );
    r.row("cfg_17_property_sweep_1024", cfg_17_property_sweep_1024);
    r.row(
        "cfg_18_strided_full_range_sweep",
        cfg_18_strided_full_range_sweep,
    );
    r.row(
        "cfg_19_exhaustive_window_around_wrap_boundary",
        cfg_19_exhaustive_window_around_wrap_boundary,
    );
    r.finish();
}

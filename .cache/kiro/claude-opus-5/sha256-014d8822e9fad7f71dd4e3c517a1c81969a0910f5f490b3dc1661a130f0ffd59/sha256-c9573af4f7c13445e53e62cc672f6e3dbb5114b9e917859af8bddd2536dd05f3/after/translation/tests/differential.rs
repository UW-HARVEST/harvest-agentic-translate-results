//! Differential tests: C `.so` vs Rust `.so`, both loaded via `libloading`.
//!
//! Phase B — one test per row of `CONFIGS.md` (valid paths).
//! Phase C — one test per row of `ERRORS.md` (rejection paths).
//! Phase D — symbol parity.
//!
//! Every assertion compares the value returned by the C shared object against
//! the value returned by the Rust shared object for the same arguments. The
//! Rust side is always reached through `dlsym("jumpnode")`, never by a direct
//! Rust call, so the `#[no_mangle]` wrapper is exercised.

mod common;

use common::{Pair, Rng, EXTREMES};

// Octal mode constants, spelled the way `c_src/src/lib.c` spells them.
const MODE_1: i32 = 0o1;
const MODE_2: i32 = 0o2;
const MODE_3: i32 = 0o3;
const MODE_4: i32 = 0o4;

// Expected sentinels, from ERRORS.md.
const ERR_MODE1_NULL: i32 = 0o2 | 0o20; // 18
const ERR_MODE2_NULL: i32 = 0o2 | 0o40; // 34
const ERR_MODE4_NULL: i32 = 0o2 | 0o100; // 66
const ERR_DEFAULT: i32 = 0o2 | 0o200; // 130

// ---------------------------------------------------------------------------
// Phase B — valid-path differential tests, one per CONFIGS.md row
// ---------------------------------------------------------------------------

/// Row 1 — mode 0001, random node_id, depth = 0.
#[test]
fn phase_b_row01_mode1_depth_zero() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0001_0001);
    for _ in 0..20_000 {
        p.assert_same(MODE_1, rng.next_i32(), 0, rng.next_i32());
    }
}

/// Row 2 — mode 0001, positive depth.
#[test]
fn phase_b_row02_mode1_depth_positive() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0001_0002);
    for _ in 0..20_000 {
        p.assert_same(MODE_1, rng.next_i32(), rng.range_i32(1, 64), rng.next_i32());
    }
}

/// Row 3 — mode 0001, negative depth.
#[test]
fn phase_b_row03_mode1_depth_negative() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0001_0003);
    for _ in 0..20_000 {
        p.assert_same(
            MODE_1,
            rng.next_i32(),
            rng.range_i32(i32::MIN, -1),
            rng.next_i32(),
        );
    }
}

/// Row 4 — mode 0001 with the node ids `initialize_test_data` would have
/// created (1..=7). It is never called, so these must behave like any other id.
#[test]
fn phase_b_row04_mode1_would_be_node_ids() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0001_0004);
    for node_id in 0..=8 {
        for depth in [0, 1, 2, 3, 10, -1, i32::MAX, i32::MIN] {
            for _ in 0..50 {
                p.assert_same(MODE_1, node_id, depth, rng.next_i32());
            }
        }
    }
}

/// Row 5 — mode 0001, full extremes cross-product.
#[test]
fn phase_b_row05_mode1_extremes() {
    let p = Pair::load();
    for &n in EXTREMES {
        for &d in EXTREMES {
            for f in [i32::MIN, -1, 0, 1, 0o177, i32::MAX] {
                p.assert_same(MODE_1, n, d, f);
            }
        }
    }
}

/// Row 6 — mode 0002, depth in 0..=15 (in-range `start_offset`).
#[test]
fn phase_b_row06_mode2_depth_in_range() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0002_0006);
    for depth in 0..=15 {
        for _ in 0..2_000 {
            p.assert_same(MODE_2, rng.next_i32(), depth, rng.next_i32());
        }
    }
}

/// Row 7 — mode 0002, depth = 16 (`start == ptr`, empty backward walk).
#[test]
fn phase_b_row07_mode2_depth_equals_size() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0002_0007);
    for _ in 0..20_000 {
        p.assert_same(MODE_2, rng.next_i32(), 16, rng.next_i32());
    }
}

/// Row 8 — mode 0002, depth > 16 (`start > ptr`, loop skipped).
#[test]
fn phase_b_row08_mode2_depth_past_size() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0002_0008);
    for _ in 0..20_000 {
        p.assert_same(MODE_2, rng.next_i32(), rng.range_i32(17, i32::MAX), rng.next_i32());
    }
}

/// Row 9 — mode 0002, negative depth.
#[test]
fn phase_b_row09_mode2_depth_negative() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0002_0009);
    for _ in 0..20_000 {
        p.assert_same(
            MODE_2,
            rng.next_i32(),
            rng.range_i32(i32::MIN, -1),
            rng.next_i32(),
        );
    }
}

/// Row 10 — mode 0002 with `flags` chosen so `(int)array_size * flags`
/// (`16 * flags`) overflows `int`.
#[test]
fn phase_b_row10_mode2_flags_overflow() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0002_0010);
    let pivot = i32::MAX / 16;
    for f in [
        pivot - 1,
        pivot,
        pivot + 1,
        -pivot - 1,
        -pivot,
        i32::MIN,
        i32::MAX,
        i32::MIN / 16,
    ] {
        for _ in 0..2_000 {
            p.assert_same(MODE_2, rng.next_i32(), rng.range_i32(-32, 32), f);
        }
    }
    for _ in 0..20_000 {
        p.assert_same(
            MODE_2,
            rng.next_i32(),
            rng.range_i32(-32, 32),
            rng.range_i32(pivot - 1000, i32::MAX),
        );
    }
}

/// Row 11 — mode 0002, extremes cross-product.
#[test]
fn phase_b_row11_mode2_extremes() {
    let p = Pair::load();
    for &n in EXTREMES {
        for &d in EXTREMES {
            for f in [i32::MIN, -1, 0, 1, 16, 0o177, i32::MAX] {
                p.assert_same(MODE_2, n, d, f);
            }
        }
    }
}

/// Row 12 — mode 0003, shortest formatted string (both args single-digit).
#[test]
fn phase_b_row12_mode3_shortest_string() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0003_0012);
    for n in 0..=9 {
        for d in 0..=9 {
            for _ in 0..40 {
                p.assert_same(MODE_3, n, d, rng.next_i32());
            }
        }
    }
}

/// Row 13 — mode 0003, every positive decimal width 1..=10 for both args.
#[test]
fn phase_b_row13_mode3_decimal_widths() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0003_0013);
    let widths: [i32; 10] = [
        1,
        12,
        123,
        1_234,
        12_345,
        123_456,
        1_234_567,
        12_345_678,
        123_456_789,
        1_234_567_890,
    ];
    for &n in &widths {
        for &d in &widths {
            for _ in 0..20 {
                p.assert_same(MODE_3, n, d, rng.next_i32());
            }
        }
    }
}

/// Row 14 — mode 0003, negative args (the extra `-` byte in `%d`).
#[test]
fn phase_b_row14_mode3_negative_args() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0003_0014);
    let widths: [i32; 10] = [
        -1,
        -12,
        -123,
        -1_234,
        -12_345,
        -123_456,
        -1_234_567,
        -12_345_678,
        -123_456_789,
        -1_234_567_890,
    ];
    for &n in &widths {
        for &d in &widths {
            for _ in 0..20 {
                p.assert_same(MODE_3, n, d, rng.next_i32());
            }
        }
    }
    for _ in 0..20_000 {
        p.assert_same(
            MODE_3,
            rng.range_i32(i32::MIN, -1),
            rng.range_i32(i32::MIN, -1),
            rng.next_i32(),
        );
    }
}

/// Row 15 — mode 0003, longest formatted string (`INT_MIN` twice, 34 bytes).
#[test]
fn phase_b_row15_mode3_longest_string() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0003_0015);
    for _ in 0..20_000 {
        p.assert_same(MODE_3, i32::MIN, i32::MIN, rng.next_i32());
    }
    for &n in &[i32::MIN, i32::MIN + 1, i32::MAX, -2_147_483_647] {
        for &d in &[i32::MIN, i32::MIN + 1, i32::MAX, -2_147_483_647] {
            for &f in EXTREMES {
                p.assert_same(MODE_3, n, d, f);
            }
        }
    }
}

/// Row 16 — mode 0003, zero args (the `magnitude == 0` digit path).
#[test]
fn phase_b_row16_mode3_zero_args() {
    let p = Pair::load();
    for &f in EXTREMES {
        p.assert_same(MODE_3, 0, 0, f);
        p.assert_same(MODE_3, 0, 1, f);
        p.assert_same(MODE_3, 1, 0, f);
        p.assert_same(MODE_3, 0, -1, f);
        p.assert_same(MODE_3, -1, 0, f);
    }
}

/// Row 17 — mode 0003, powers of ten and their ±1 neighbours (digit-count
/// boundaries in `%d`).
#[test]
fn phase_b_row17_mode3_digit_boundaries() {
    let p = Pair::load();
    let mut boundaries: Vec<i32> = Vec::new();
    let mut pow: i64 = 1;
    while pow <= 1_000_000_000 {
        for delta in [-1i64, 0, 1] {
            let v = pow + delta;
            boundaries.push(v as i32);
            boundaries.push(-(v as i32));
        }
        pow *= 10;
    }
    for &n in &boundaries {
        for &d in &boundaries {
            p.assert_same(MODE_3, n, d, 0);
            p.assert_same(MODE_3, n, d, 0o177);
            p.assert_same(MODE_3, n, d, -1);
        }
    }
}

/// Row 18 — mode 0003, random full-range `flags` feeding `flags & 0177`.
#[test]
fn phase_b_row18_mode3_flags_random() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0003_0018);
    for _ in 0..50_000 {
        p.assert_same(MODE_3, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Row 19 — mode 0003, `flags` on the `0177` mask boundaries incl. negatives.
#[test]
fn phase_b_row19_mode3_flags_mask_boundaries() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0003_0019);
    let masks = [
        0,
        1,
        0o77,
        0o100,
        0o176,
        0o177,
        0o200,
        0o201,
        0o377,
        0o400,
        -1,
        -0o177,
        -0o200,
        i32::MIN,
        i32::MAX,
        i32::MIN + 0o177,
        i32::MAX - 0o177,
    ];
    for &f in &masks {
        for _ in 0..2_000 {
            p.assert_same(MODE_3, rng.next_i32(), rng.next_i32(), f);
        }
    }
}

/// Row 20 — mode 0003, full random cross-product of all three data args.
#[test]
fn phase_b_row20_mode3_full_random() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0003_0020);
    for _ in 0..100_000 {
        // Mix wide and narrow magnitudes so both short and long strings appear.
        let n = match rng.next_u64() % 3 {
            0 => rng.next_i32(),
            1 => rng.range_i32(-1000, 1000),
            _ => rng.range_i32(-9, 9),
        };
        let d = match rng.next_u64() % 3 {
            0 => rng.next_i32(),
            1 => rng.range_i32(-1000, 1000),
            _ => rng.range_i32(-9, 9),
        };
        p.assert_same(MODE_3, n, d, rng.next_i32());
    }
}

/// Row 21 — mode 0004, depth = 0.
#[test]
fn phase_b_row21_mode4_depth_zero() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0004_0021);
    for _ in 0..20_000 {
        p.assert_same(MODE_4, rng.next_i32(), 0, rng.next_i32());
    }
}

/// Row 22 — mode 0004, positive depth (`1.0 + depth*0.1` scale-up).
#[test]
fn phase_b_row22_mode4_depth_positive() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0004_0022);
    for _ in 0..20_000 {
        p.assert_same(MODE_4, rng.next_i32(), rng.range_i32(1, 1_000_000), rng.next_i32());
    }
}

/// Row 23 — mode 0004, depth = -10 (`1.0 + depth*0.1` is exactly 0.0).
#[test]
fn phase_b_row23_mode4_depth_minus_ten() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0004_0023);
    for _ in 0..10_000 {
        p.assert_same(MODE_4, rng.next_i32(), -10, rng.next_i32());
    }
    for d in -12..=-8 {
        for _ in 0..500 {
            p.assert_same(MODE_4, rng.next_i32(), d, rng.next_i32());
        }
    }
}

/// Row 24 — mode 0004, depth < -10 (negative scale factor).
#[test]
fn phase_b_row24_mode4_depth_below_minus_ten() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0004_0024);
    for _ in 0..20_000 {
        p.assert_same(MODE_4, rng.next_i32(), rng.range_i32(i32::MIN, -11), rng.next_i32());
    }
}

/// Row 25 — mode 0004, huge depth driving `safe_double_to_int`'s clamps.
#[test]
fn phase_b_row25_mode4_depth_extremes() {
    let p = Pair::load();
    for &d in &[
        i32::MIN,
        i32::MIN + 1,
        -2_000_000_000,
        -1,
        0,
        1,
        2_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        for &n in EXTREMES {
            p.assert_same(MODE_4, n, d, 0);
        }
    }
}

/// Row 26 — mode 0004, extremes cross-product.
#[test]
fn phase_b_row26_mode4_extremes() {
    let p = Pair::load();
    for &n in EXTREMES {
        for &d in EXTREMES {
            for f in [i32::MIN, -1, 0, 1, 0o177, i32::MAX] {
                p.assert_same(MODE_4, n, d, f);
            }
        }
    }
}

/// Row 27 — `default:` arm with a random out-of-range `operation_mode`.
#[test]
fn phase_b_row27_default_random_mode() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0027);
    let mut checked = 0u32;
    while checked < 50_000 {
        let m = rng.next_i32();
        if (1..=4).contains(&m) {
            continue;
        }
        let got = p.assert_same(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(got, ERR_DEFAULT, "default arm should yield 130 for mode {m}");
        checked += 1;
    }
}

/// Row 28 — `default:` arm on the interesting mode values.
#[test]
fn phase_b_row28_default_named_modes() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0028);
    for &m in &[
        0,
        5,
        6,
        7,
        8,
        -1,
        -2,
        -4,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0o177,
        0o200,
        0o377,
        0o400,
        256,
        65_536,
    ] {
        for _ in 0..500 {
            let got = p.assert_same(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
            assert_eq!(got, ERR_DEFAULT, "mode {m} must hit the default arm");
        }
    }
}

/// Row 29 — interleaved calls across every mode in one process, checking that
/// neither library accumulates hidden global state (`node_count`) between calls.
#[test]
fn phase_b_row29_interleaved_state_stability() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0029);

    // Baselines captured up front.
    let base: Vec<(i32, i32)> = (0..=5)
        .map(|m| (p.c(m, 3, 2, 7), p.rust(m, 3, 2, 7)))
        .collect();

    for _ in 0..20_000 {
        let m = rng.range_i32(-2, 8);
        p.assert_same(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }

    // Same calls after 20k intervening calls must give the same answers, and
    // the two libraries must still agree.
    for m in 0..=5 {
        let c_now = p.c(m, 3, 2, 7);
        let r_now = p.rust(m, 3, 2, 7);
        let (c0, r0) = base[m as usize];
        assert_eq!(c_now, c0, "C mode {m} drifted: {c0} -> {c_now}");
        assert_eq!(r_now, r0, "Rust mode {m} drifted: {r0} -> {r_now}");
        assert_eq!(c_now, r_now, "mode {m}: C {c_now} != Rust {r_now}");
    }

    // node_count must remain 0 in both, so modes 1/2/4 keep their sentinels.
    assert_eq!(p.c(MODE_1, 1, 0, 0), ERR_MODE1_NULL);
    assert_eq!(p.rust(MODE_1, 1, 0, 0), ERR_MODE1_NULL);
    assert_eq!(p.c(MODE_2, 1, 0, 0), ERR_MODE2_NULL);
    assert_eq!(p.rust(MODE_2, 1, 0, 0), ERR_MODE2_NULL);
    assert_eq!(p.c(MODE_4, 1, 0, 0), ERR_MODE4_NULL);
    assert_eq!(p.rust(MODE_4, 1, 0, 0), ERR_MODE4_NULL);
}

/// Row 30 — unconstrained fuzz over the whole 4-D `int` space.
#[test]
fn phase_b_row30_unconstrained_fuzz() {
    let p = Pair::load();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0030);
    for _ in 0..200_000 {
        // Bias `operation_mode` so the four real cases are hit often, but keep
        // full-range values in the mix too.
        let m = match rng.next_u64() % 4 {
            0 => rng.range_i32(-4, 9),
            1 => rng.range_i32(1, 4),
            2 => rng.next_i32(),
            _ => rng.range_i32(0, 5),
        };
        p.assert_same(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Phase C — error-path differential tests, one per ERRORS.md row
// ---------------------------------------------------------------------------

/// ERRORS.md row 3 — mode 0001 null-node rejection, `STATUS_ERROR | 0020` = 18.
/// Reachable for *every* node_id because `node_count` is 0.
#[test]
fn phase_c_row3_mode1_null_node() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0C00_0003);
    for _ in 0..30_000 {
        let got = p.assert_same(MODE_1, rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(got, ERR_MODE1_NULL, "mode 1 must reject with 18");
    }
    for &n in EXTREMES {
        for &d in EXTREMES {
            let got = p.assert_same(MODE_1, n, d, 0);
            assert_eq!(got, ERR_MODE1_NULL);
        }
    }
}

/// ERRORS.md row 4 — mode 0002 null-node rejection, `STATUS_ERROR | 0040` = 34.
#[test]
fn phase_c_row4_mode2_null_node() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0C00_0004);
    for _ in 0..30_000 {
        let got = p.assert_same(MODE_2, rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(got, ERR_MODE2_NULL, "mode 2 must reject with 34");
    }
    for &n in EXTREMES {
        for &d in EXTREMES {
            let got = p.assert_same(MODE_2, n, d, 0);
            assert_eq!(got, ERR_MODE2_NULL);
        }
    }
}

/// ERRORS.md row 5 — mode 0004 null-node rejection, `STATUS_ERROR | 0100` = 66.
#[test]
fn phase_c_row5_mode4_null_node() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0C00_0005);
    for _ in 0..30_000 {
        let got = p.assert_same(MODE_4, rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(got, ERR_MODE4_NULL, "mode 4 must reject with 66");
    }
    for &n in EXTREMES {
        for &d in EXTREMES {
            let got = p.assert_same(MODE_4, n, d, 0);
            assert_eq!(got, ERR_MODE4_NULL);
        }
    }
}

/// ERRORS.md row 6 — the `default:` arm, `STATUS_ERROR | 0200` = 130.
#[test]
fn phase_c_row6_default_arm() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0C00_0006);
    for &m in EXTREMES {
        if (1..=4).contains(&m) {
            continue;
        }
        for _ in 0..300 {
            let got = p.assert_same(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
            assert_eq!(got, ERR_DEFAULT, "mode {m} must reject with 130");
        }
    }
}

/// Out-of-range "enum" values crossing the FFI boundary. A C `switch` on an int
/// accepts any bit pattern, so a value with no matching `case` is a real input.
/// Both sides must take the `default:` arm identically.
#[test]
fn phase_c_out_of_range_enum_values() {
    let p = Pair::load();

    // Every mode value one step outside the valid 1..=4 window.
    for m in [0, 5, -1, i32::MIN, i32::MAX] {
        let got = p.assert_same(m, 0, 0, 0);
        assert_eq!(got, ERR_DEFAULT, "mode {m} must be rejected as unknown");
    }

    // The STATUS_* constants themselves, misused as modes.
    for m in [0o0, 0o1, 0o2, 0o377] {
        p.assert_same(m, 42, 7, 9);
    }
    assert_eq!(p.assert_same(0o0, 42, 7, 9), ERR_DEFAULT);
    assert_eq!(p.assert_same(0o377, 42, 7, 9), ERR_DEFAULT);

    // Values that alias 1..=4 only in their low bits must NOT be accepted.
    for shift in 8..31u32 {
        for low in 1..=4i32 {
            let m = low | (1 << shift);
            let got = p.assert_same(m, 1, 1, 1);
            assert_eq!(got, ERR_DEFAULT, "mode {m} (0x{m:x}) must hit default");
        }
    }

    // Exhaustive sweep of a contiguous window around the valid range.
    for m in -4096..=4096 {
        let got = p.assert_same(m, 123, -456, 789);
        if !(1..=4).contains(&m) {
            assert_eq!(got, ERR_DEFAULT, "mode {m} must hit default");
        }
    }
}

/// Generic boundary sweep (ERRORS.md G1..G8): each of the four `int` arguments
/// driven to `INT_MIN` / `INT_MAX` / `0` / `-1` / one-past-valid, in every mode.
#[test]
fn phase_c_generic_boundaries() {
    let p = Pair::load();
    let edges = [i32::MIN, i32::MIN + 1, -1, 0, 1, 15, 16, 17, i32::MAX - 1, i32::MAX];
    for m in [i32::MIN, -1, 0, 1, 2, 3, 4, 5, i32::MAX] {
        for &n in &edges {
            for &d in &edges {
                for &f in &edges {
                    p.assert_same(m, n, d, f);
                }
            }
        }
    }
}

/// ERRORS.md rows 1, 2, 7, 8, 9, 10 — rejections that exist in the C source but
/// are not reachable through the single exported symbol. This test pins down the
/// preemption that makes them unreachable, so the claim stays honest and is
/// re-checked on every run rather than merely asserted in a document.
#[test]
fn phase_c_unreachable_rows() {
    let p = Pair::load();

    // Row 1 (`add_node`: node_count >= MAX_NODES) and row 2's only caller path:
    // `add_node` is called exclusively by `initialize_test_data`, which has no
    // callers. If it were ever reached, node_count would become 7 and mode 0004
    // would take its `node_count > 2` branch, producing something other than 66.
    // Both libraries must show that it does not.
    assert_eq!(p.c(MODE_4, 1, 0, 0), ERR_MODE4_NULL);
    assert_eq!(p.rust(MODE_4, 1, 0, 0), ERR_MODE4_NULL);

    // Rows 7/8 (`safe_double_to_int` clamps) are only called from modes 0001 and
    // 0004, both of which return early on the null node. Confirm the early
    // return by showing the result is the sentinel for every node id, so no
    // clamped arithmetic can leak through.
    for node_id in [i32::MIN, -1, 0, 1, 2, 3, 4, 5, 6, 7, i32::MAX] {
        assert_eq!(p.assert_same(MODE_1, node_id, i32::MAX, 0), ERR_MODE1_NULL);
        assert_eq!(p.assert_same(MODE_4, node_id, i32::MAX, 0), ERR_MODE4_NULL);
        assert_eq!(p.assert_same(MODE_1, node_id, i32::MIN, 0), ERR_MODE1_NULL);
        assert_eq!(p.assert_same(MODE_4, node_id, i32::MIN, 0), ERR_MODE4_NULL);
    }

    // Rows 9/10 (`process_backward` with start_offset >= size, and the
    // out-of-bounds start_offset < 0) sit behind mode 0002's null-node return.
    // Row 10 is UB in C, so it is deliberately never triggered; this asserts the
    // guard that keeps it unreachable.
    for depth in [i32::MIN, -1, 0, 15, 16, 17, i32::MAX] {
        assert_eq!(p.assert_same(MODE_2, 1, depth, 0), ERR_MODE2_NULL);
    }
}

// ---------------------------------------------------------------------------
// Phase D — symbol parity through the loaded objects
// ---------------------------------------------------------------------------

/// The Rust `.so` must export every symbol the C `.so` exports, by exact name.
#[test]
fn phase_d_symbol_parity() {
    let p = Pair::load();

    let c_syms = dynamic_symbols(&p.c_path);
    let rust_syms = dynamic_symbols(&p.rust_path);

    assert!(
        c_syms.contains(&"jumpnode".to_string()),
        "C .so must export jumpnode; got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}"
    );
}

/// `dlsym` must resolve `jumpnode` in both objects — this is what proves the
/// `#[no_mangle] extern "C"` wrapper is really there and callable.
#[test]
fn phase_d_dlsym_resolves_in_both() {
    let p = Pair::load();
    // Loading already resolved the symbol in both; a call through each proves
    // the ABI matches (4 x int in, int out).
    assert_eq!(p.c(MODE_3, 1, 1, 0), p.rust(MODE_3, 1, 1, 0));
    assert_eq!(p.c(999, 1, 1, 0), ERR_DEFAULT);
    assert_eq!(p.rust(999, 1, 1, 0), ERR_DEFAULT);
}

/// Read the dynamic symbol table with `nm -D --defined-only`, keeping only
/// globally-defined names and dropping the toolchain/libc boilerplate that a
/// Rust cdylib and a C .so each add on their own.
fn dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm -D");
    assert!(out.status.success(), "nm failed on {}", path.display());

    const IGNORED: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__cxa_finalize",
        "rust_eh_personality",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__gmon_start__",
    ];

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let _addr = parts.next()?;
            let kind = parts.next()?;
            let name = parts.next()?;
            // Global text/data/bss/weak definitions only.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "i") {
                return None;
            }
            if IGNORED.contains(&name) || name.starts_with("_ZN") || name.starts_with("__rust") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

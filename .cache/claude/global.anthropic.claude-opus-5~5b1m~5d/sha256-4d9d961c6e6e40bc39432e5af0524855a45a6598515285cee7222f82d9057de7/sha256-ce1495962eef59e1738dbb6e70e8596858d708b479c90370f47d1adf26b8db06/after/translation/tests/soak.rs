//! High-volume differential soak tests over the shipped library state
//! (`node_count == 0`). This binary never calls the init hook.
//!
//! The point of these is coverage that the per-row tests cannot give:
//! contiguous exhaustive sweeps rather than sampled points, so a divergence
//! that only shows up at one particular value cannot hide.

mod common;

use common::*;
use std::ffi::c_int;

/// Case 0003's result depends on `strlen("Node_%d_Depth_%d")`, so the decimal
/// width of BOTH integers is the load-bearing quantity. Sweep a contiguous band
/// that covers widths 1..7 for both fields, exhaustively in one dimension.
#[test]
fn soak_mode3_contiguous_node_id_sweep() {
    let p = Pair::shipped();
    for n in -600_000..=600_000 {
        p.assert_same_eq(0o3, n, 7, 0, expect_mode3(n, 7, 0));
    }
}

#[test]
fn soak_mode3_contiguous_depth_sweep() {
    let p = Pair::shipped();
    for d in -600_000..=600_000 {
        p.assert_same_eq(0o3, -12345, d, 0o125, expect_mode3(-12345, d, 0o125));
    }
}

/// Both fields moving together, and around every power-of-ten carry where the
/// decimal width changes.
#[test]
fn soak_mode3_width_transitions() {
    let p = Pair::shipped();
    let mut probes: Vec<c_int> = Vec::new();
    for k in 0..=9u32 {
        let base = 10i64.pow(k);
        for delta in -3i64..=3 {
            for sign in [1i64, -1] {
                let v = sign * (base + delta);
                if (i32::MIN as i64..=i32::MAX as i64).contains(&v) {
                    probes.push(v as c_int);
                }
            }
        }
    }
    for v in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0] {
        probes.push(v);
    }
    probes.sort_unstable();
    probes.dedup();
    println!("width-transition probes: {}", probes.len());
    for &n in &probes {
        for &d in &probes {
            for f in [0, 1, 0o177, 0o200, -1, i32::MIN, i32::MAX] {
                p.assert_same_eq(0o3, n, d, f, expect_mode3(n, d, f));
            }
        }
    }
}

/// Every one of the 128 `flags & 0177` residues against every decimal width
/// combination.
#[test]
fn soak_mode3_full_flag_residues() {
    let p = Pair::shipped();
    // One representative per decimal width 1..11 for each field.
    let reps: [c_int; 11] = [
        7,
        42,
        512,
        4096,
        54321,
        654321,
        7654321,
        87654321,
        987654321,
        i32::MAX,
        i32::MIN,
    ];
    let widths: Vec<usize> = reps.iter().map(|&v| decimal_width(v)).collect();
    assert_eq!(widths, (1..=11).collect::<Vec<_>>(), "one rep per width 1..11");
    for &n in &reps {
        for &d in &reps {
            for f in 0..128 {
                p.assert_same_eq(0o3, n, d, f, expect_mode3(n, d, f));
                // The same residue reached from a negative / high-bit flags.
                let f2 = f | i32::MIN;
                p.assert_same_eq(0o3, n, d, f2, expect_mode3(n, d, f2));
            }
        }
    }
}

/// Contiguous sweep of `operation_mode` far wider than the switch, confirming
/// the `default:` arm for every non-case value and no aliasing.
#[test]
fn soak_mode_contiguous_sweep() {
    let p = Pair::shipped();
    for m in -300_000..=300_000 {
        let got = p.assert_same(m, 3, 4, 5);
        let expected = match m {
            0o1 => ERR_MODE1_NOT_FOUND,
            0o2 => ERR_MODE2_NOT_FOUND,
            0o3 => expect_mode3(3, 4, 5),
            0o4 => ERR_MODE4_NOT_FOUND,
            _ => ERR_UNKNOWN_MODE,
        };
        assert_eq!(got, expected, "mode {m}");
    }
}

/// Large randomized 4-tuple soak across the whole `int` space.
#[test]
fn soak_random_tuples() {
    let p = Pair::shipped();
    let mut rng = Rng::new(0x5040_1234);
    for _ in 0..1_500_000 {
        let m = match rng.next_u64() % 8 {
            0 => 0o1,
            1 => 0o2,
            2 => 0o3,
            3 => 0o3,
            4 => 0o4,
            5 => rng.i32_range(-8, 12),
            _ => rng.i32_any(),
        };
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        let got = p.assert_same(m, n, d, f);
        let expected = match m {
            0o1 => ERR_MODE1_NOT_FOUND,
            0o2 => ERR_MODE2_NOT_FOUND,
            0o3 => expect_mode3(n, d, f),
            0o4 => ERR_MODE4_NOT_FOUND,
            _ => ERR_UNKNOWN_MODE,
        };
        assert_eq!(got, expected, "jumpnode({m},{n},{d},{f})");
    }
}

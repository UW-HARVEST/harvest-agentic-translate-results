//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through their `jumpnode` dynamic symbol and asserts the returned
//! `int`s are identical. Inputs are randomized with a fixed seed.

mod common;

use common::{assert_same, pair, Rng, ARG_BOUNDARIES, DECIMAL_WIDTH_BOUNDARIES};

const N: usize = 4000;

// ---------------------------------------------------------------- row 1
#[test]
fn cfg_row01_mode1_randomized() {
    let mut rng = Rng::new(0x1001);
    for _ in 0..N {
        assert_same(1, rng.shaped_i32(), rng.shaped_i32(), rng.shaped_i32());
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn cfg_row02_mode1_depth_boundaries() {
    let mut rng = Rng::new(0x1002);
    let depths = [i32::MIN, -1, 0, 1, 2, 3, 100, i32::MAX];
    for &d in &depths {
        for _ in 0..300 {
            assert_same(1, rng.shaped_i32(), d, rng.shaped_i32());
        }
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn cfg_row03_mode2_randomized() {
    let mut rng = Rng::new(0x1003);
    for _ in 0..N {
        assert_same(2, rng.shaped_i32(), rng.shaped_i32(), rng.shaped_i32());
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn cfg_row04_mode2_process_backward_offset_boundaries() {
    let mut rng = Rng::new(0x1004);
    let depths = [i32::MIN, -1, 0, 1, 4, 15, 16, 17, i32::MAX];
    for &d in &depths {
        for _ in 0..300 {
            assert_same(2, rng.shaped_i32(), d, rng.shaped_i32());
        }
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn cfg_row05_mode2_flags_multiplier_boundaries() {
    let mut rng = Rng::new(0x1005);
    // 16 * flags overflows i32 for |flags| >= 2^27.
    let flags = [
        0,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        0x7f,
        0x80,
        134_217_728,
        -134_217_728,
        134_217_727,
    ];
    for &f in &flags {
        for _ in 0..300 {
            assert_same(2, rng.shaped_i32(), rng.shaped_i32(), f);
        }
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn cfg_row06_mode3_randomized() {
    let mut rng = Rng::new(0x1006);
    for _ in 0..(N * 4) {
        assert_same(3, rng.shaped_i32(), rng.shaped_i32(), rng.shaped_i32());
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn cfg_row07_mode3_node_id_every_decimal_width() {
    let mut rng = Rng::new(0x1007);
    for &id in &DECIMAL_WIDTH_BOUNDARIES {
        for _ in 0..120 {
            assert_same(3, id, rng.shaped_i32(), rng.shaped_i32());
        }
    }
    // Also sweep every power-of-ten neighbourhood exhaustively.
    let mut p: i64 = 1;
    while p <= i32::MAX as i64 {
        for delta in -2i64..=2 {
            let v = p + delta;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                assert_same(3, v as i32, 7, 0);
                assert_same(3, -(v as i32), 7, 0);
            }
        }
        p *= 10;
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn cfg_row08_mode3_depth_every_decimal_width() {
    let mut rng = Rng::new(0x1008);
    for &d in &DECIMAL_WIDTH_BOUNDARIES {
        for _ in 0..120 {
            assert_same(3, rng.shaped_i32(), d, rng.shaped_i32());
        }
    }
    let mut p: i64 = 1;
    while p <= i32::MAX as i64 {
        for delta in -2i64..=2 {
            let v = p + delta;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                assert_same(3, 42, v as i32, 0);
                assert_same(3, 42, -(v as i32), 0);
            }
        }
        p *= 10;
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn cfg_row09_mode3_node_id_x_depth_cross_product() {
    let mut rng = Rng::new(0x1009);
    for &id in &DECIMAL_WIDTH_BOUNDARIES {
        for &d in &DECIMAL_WIDTH_BOUNDARIES {
            assert_same(3, id, d, rng.shaped_i32());
        }
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn cfg_row10_mode3_flags_mask_residues() {
    let mut rng = Rng::new(0x100a);
    // All 128 residues of `flags & 0177`.
    for low in 0..128i32 {
        assert_same(3, 12345, -678, low);
        // Same low bits, randomized high bits & sign: result must not change.
        for _ in 0..8 {
            let high = rng.i32() & !0x7f;
            assert_same(3, 12345, -678, high | low);
        }
    }
    for &f in &[i32::MIN, i32::MIN + 1, -1, 0, i32::MAX, 0x80, 0xff, -128] {
        for _ in 0..60 {
            assert_same(3, rng.shaped_i32(), rng.shaped_i32(), f);
        }
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn cfg_row11_mode4_randomized() {
    let mut rng = Rng::new(0x100b);
    for _ in 0..N {
        assert_same(4, rng.shaped_i32(), rng.shaped_i32(), rng.shaped_i32());
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn cfg_row12_mode4_depth_scale_boundaries() {
    let mut rng = Rng::new(0x100c);
    let depths = [i32::MIN, -100, -11, -10, -9, -1, 0, 1, 10, i32::MAX];
    for &d in &depths {
        for _ in 0..300 {
            assert_same(4, rng.shaped_i32(), d, rng.shaped_i32());
        }
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn cfg_row13_mode_dispatch_including_out_of_range_enums() {
    let mut rng = Rng::new(0x100d);
    let mut modes: Vec<i32> = (-8..=8).collect();
    modes.extend_from_slice(&[
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        5,
        8,
        0x1_0001,
        0o1000,
        256,
        65536,
        -256,
    ]);
    for &m in &modes {
        for _ in 0..200 {
            assert_same(m, rng.shaped_i32(), rng.shaped_i32(), rng.shaped_i32());
        }
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn cfg_row14_mode_fully_randomized() {
    let mut rng = Rng::new(0x100e);
    for _ in 0..N {
        assert_same(rng.i32(), rng.i32(), rng.i32(), rng.i32());
    }
    for _ in 0..N {
        assert_same(
            rng.shaped_i32(),
            rng.shaped_i32(),
            rng.shaped_i32(),
            rng.shaped_i32(),
        );
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn cfg_row15_full_four_axis_fuzz_biased_to_real_modes() {
    let mut rng = Rng::new(0x100f);
    for _ in 0..(N * 4) {
        let m = 1 + (rng.below(4) as i32);
        assert_same(m, rng.shaped_i32(), rng.shaped_i32(), rng.shaped_i32());
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn cfg_row16_four_axis_boundary_cross_product() {
    // 16^2 boundary pairs for (node_id, depth) x each mode class x boundary flags.
    for &m in &[1i32, 2, 3, 4, 0, 5, i32::MIN, i32::MAX] {
        for &id in &ARG_BOUNDARIES {
            for &d in &ARG_BOUNDARIES {
                for &f in &[i32::MIN, -1, 0, 1, 127, 128, i32::MAX] {
                    assert_same(m, id, d, f);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn cfg_row17_repeated_interleaved_calls_are_stateless() {
    let mut rng = Rng::new(0x1010);
    // Both libraries own `static` mutable node storage; verify no state leaks
    // between calls and that interleaving modes does not perturb results.
    let cases: Vec<(i32, i32, i32, i32)> = (0..64)
        .map(|_| {
            (
                rng.pick(&[1, 2, 3, 4, 0, 7]),
                rng.shaped_i32(),
                rng.shaped_i32(),
                rng.shaped_i32(),
            )
        })
        .collect();

    let first: Vec<i32> = cases
        .iter()
        .map(|&(m, i, d, f)| assert_same(m, i, d, f))
        .collect();

    for _round in 0..25 {
        for (k, &(m, i, d, f)) in cases.iter().enumerate() {
            let v = assert_same(m, i, d, f);
            assert_eq!(
                v, first[k],
                "state leak: jumpnode({m},{i},{d},{f}) changed from {} to {v}",
                first[k]
            );
        }
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn cfg_row18_fresh_dlopen_matches_steady_state() {
    let mut rng = Rng::new(0x1011);
    let cases: Vec<(i32, i32, i32, i32)> = (0..200)
        .map(|_| {
            (
                rng.pick(&[1, 2, 3, 4, 0, 9]),
                rng.shaped_i32(),
                rng.shaped_i32(),
                rng.shaped_i32(),
            )
        })
        .collect();

    // Steady-state results from the shared, already-warm pair.
    let warm: Vec<i32> = cases
        .iter()
        .map(|&(m, i, d, f)| assert_same(m, i, d, f))
        .collect();

    // A brand-new dlopen of both libraries: first-ever call per library must
    // agree with the warm results (static initialisers / lazily-init state).
    let fresh = common::Pair::open();
    let cf = fresh.c();
    let rf = fresh.rust();
    for (k, &(m, i, d, f)) in cases.iter().enumerate() {
        let c_val = unsafe { cf(m, i, d, f) };
        let r_val = unsafe { rf(m, i, d, f) };
        assert_eq!(
            c_val, r_val,
            "fresh-load DIVERGENCE jumpnode({m},{i},{d},{f}): C={c_val} Rust={r_val}"
        );
        assert_eq!(c_val, warm[k], "fresh load differs from warm for ({m},{i},{d},{f})");
    }
    drop(cf);
    drop(rf);
    drop(fresh);

    // Sanity: the shared pair still exports the symbols after the fresh pair
    // was unloaded.
    let p = pair();
    let _ = p.c();
    let _ = p.rust();
}

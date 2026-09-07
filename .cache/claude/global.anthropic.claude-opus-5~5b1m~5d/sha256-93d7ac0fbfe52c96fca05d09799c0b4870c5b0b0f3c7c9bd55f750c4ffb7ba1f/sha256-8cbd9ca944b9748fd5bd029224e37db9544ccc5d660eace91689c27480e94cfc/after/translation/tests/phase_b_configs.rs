//! Phase B — valid-path differential tests.
//! One `#[test]` per row of `CONFIGS.md`, each with many randomized inputs
//! (fixed seed) compared bit-for-bit between the C `.so` and the Rust `.so`.

mod common;

use common::*;

const N: usize = 20_000;

// --- Row 1: s == +0.0 early return -----------------------------------------
#[test]
fn row01_s_plus_zero_early_return() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        let h = rng.range(-1.0e6, 1.0e6);
        let v = rng.range(-1.0e6, 1.0e6);
        let out = check("row01", [h, 0.0, v]);
        // C semantics: dest[0..3] = v verbatim.
        assert_eq!(out.map(f32::to_bits), [v.to_bits(); 3], "row01 shape");
    }
}

// --- Row 2: s == -0.0 early return ------------------------------------------
#[test]
fn row02_s_minus_zero_early_return() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let h = rng.range(-1.0e6, 1.0e6);
        let v = rng.range(-1.0e6, 1.0e6);
        let out = check("row02", [h, -0.0, v]);
        assert_eq!(out.map(f32::to_bits), [v.to_bits(); 3], "row02 shape");
    }
}

// --- Row 3: early return with special v ------------------------------------
#[test]
fn row03_early_return_special_v() {
    let mut rng = Rng::new(SEED ^ 3);
    for &s in &[0.0f32, -0.0f32] {
        for &v in SPECIAL {
            for &h in SPECIAL {
                check("row03/special", [h, s, v]);
            }
            for _ in 0..200 {
                check("row03/rand-h", [rng.any_f32(), s, v]);
            }
        }
    }
}

// --- Rows 4..9: each named switch arm + i == 5 ------------------------------
fn canonical_arm(row: &str, seed: u64, lo: f32, hi: f32) {
    let mut rng = Rng::new(SEED ^ seed);
    for _ in 0..N {
        let h = rng.range(lo, hi);
        // s strictly in (0,1) so the early return is skipped
        let s = rng.range(f32::MIN_POSITIVE, 1.0);
        let v = rng.range(0.0, 1.0);
        check(row, [h, s, v]);
    }
    // deterministic sweep across the arm as well
    let mut x = lo;
    while x < hi {
        check(row, [x, 0.75, 0.5]);
        check(row, [x, 1.0, 1.0]);
        x += (hi - lo) / 512.0;
    }
}

#[test]
fn row04_arm_i0() {
    canonical_arm("row04/i=0", 4, 0.0, 60.0);
}

#[test]
fn row05_arm_i1() {
    canonical_arm("row05/i=1", 5, 60.0, 120.0);
}

#[test]
fn row06_arm_i2() {
    canonical_arm("row06/i=2", 6, 120.0, 180.0);
}

#[test]
fn row07_arm_i3() {
    canonical_arm("row07/i=3", 7, 180.0, 240.0);
}

#[test]
fn row08_arm_i4() {
    canonical_arm("row08/i=4", 8, 240.0, 300.0);
}

#[test]
fn row09_default_arm_i5() {
    canonical_arm("row09/i=5(default)", 9, 300.0, 360.0);
}

// --- Row 10: default via i >= 6 (no hue wrapping in C) ----------------------
#[test]
fn row10_default_large_i() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N {
        let h = rng.range(360.0, 1.0e6);
        let s = rng.range(f32::MIN_POSITIVE, 1.0);
        let v = rng.range(0.0, 1.0);
        check("row10", [h, s, v]);
    }
    for k in 6..2000 {
        check("row10/exact", [60.0 * k as f32, 0.6, 0.9]);
    }
}

// --- Row 11: default via i < 0 ---------------------------------------------
#[test]
fn row11_default_negative_i() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..N {
        let h = rng.range(-1.0e6, -1.0e-6);
        let s = rng.range(f32::MIN_POSITIVE, 1.0);
        let v = rng.range(0.0, 1.0);
        check("row11", [h, s, v]);
    }
    for k in 1..2000 {
        check("row11/exact", [-60.0 * k as f32, 0.6, 0.9]);
        check("row11/frac", [-60.0 * k as f32 + 13.7, 0.6, 0.9]);
    }
}

// --- Row 12: f == 0 exactly (h a multiple of 60) ---------------------------
#[test]
fn row12_zero_fraction() {
    let mut rng = Rng::new(SEED ^ 12);
    for k in -20i32..=20 {
        let h = 60.0 * k as f32;
        for _ in 0..500 {
            let s = rng.range(f32::MIN_POSITIVE, 2.0);
            let v = rng.range(-2.0, 2.0);
            check("row12", [h, s, v]);
        }
        for &s in SPECIAL {
            for &v in SPECIAL {
                check("row12/special", [h, s, v]);
            }
        }
    }
}

// --- Row 13: f just below 1 ------------------------------------------------
#[test]
fn row13_fraction_near_one() {
    let mut rng = Rng::new(SEED ^ 13);
    for k in -20i32..=20 {
        let boundary = 60.0f32 * (k + 1) as f32;
        // Step a few ULPs below the boundary. `next_down`/`next_up` handle
        // ±0.0 and sign crossings correctly (raw `to_bits() - 1` underflows at
        // +0.0, which is exactly the k = -1 case).
        let mut h = boundary;
        for _ in 0..8 {
            h = h.next_down();
            for _ in 0..64 {
                let s = rng.range(f32::MIN_POSITIVE, 1.0);
                let v = rng.range(0.0, 1.0);
                check("row13", [h, s, v]);
            }
        }
        // and a few ULPs above
        let mut h = boundary;
        for _ in 0..8 {
            h = h.next_up();
            check("row13/above", [h, 0.5, 0.5]);
            check("row13/above2", [h, 1.0, 1.0]);
        }
    }
}

// --- Row 14: s == 1.0 exactly (p becomes zero; sign observable) ------------
#[test]
fn row14_s_exactly_one() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..N {
        let h = rng.range(-720.0, 720.0);
        let v = rng.range(-2.0, 2.0);
        check("row14", [h, 1.0, v]);
    }
    for &v in SPECIAL {
        for &h in SPECIAL {
            check("row14/special", [h, 1.0, v]);
        }
    }
}

// --- Row 15: s > 1 --------------------------------------------------------
#[test]
fn row15_s_greater_than_one() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let h = rng.range(-720.0, 720.0);
        let s = rng.range(1.0, 1.0e6);
        let v = rng.range(-2.0, 2.0);
        check("row15", [h, s, v]);
    }
}

// --- Row 16: s < 0 (not -0.0) ---------------------------------------------
#[test]
fn row16_s_negative() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..N {
        let h = rng.range(-720.0, 720.0);
        let s = -rng.range(f32::MIN_POSITIVE, 1.0e6);
        assert!(s != 0.0);
        let v = rng.range(-2.0, 2.0);
        check("row16", [h, s, v]);
    }
}

/// Hue values that land on every distinguished `i` outcome.
const ARM_HUES: &[f32] = &[
    30.0,          // i = 0
    90.0,          // i = 1
    150.0,         // i = 2
    210.0,         // i = 3
    270.0,         // i = 4
    330.0,         // i = 5   -> default
    -30.0,         // i = -1  -> default
    1.0e9,         // huge i  -> default
    f32::NAN,      // INT_MIN -> default
    f32::INFINITY, // INT_MIN -> default
    f32::NEG_INFINITY,
    1e30,          // overflow -> INT_MIN
    0.0,
    60.0,
];

// --- Row 17: special v across all arms -----------------------------------
#[test]
fn row17_special_v_all_arms() {
    let mut rng = Rng::new(SEED ^ 17);
    for &h in ARM_HUES {
        for &v in SPECIAL {
            for _ in 0..64 {
                let s = rng.range(f32::MIN_POSITIVE, 1.0);
                check("row17", [h, s, v]);
            }
            check("row17/s=1", [h, 1.0, v]);
            check("row17/s=2", [h, 2.0, v]);
            check("row17/s=-1", [h, -1.0, v]);
        }
    }
}

// --- Row 18: special s across all arms -----------------------------------
#[test]
fn row18_special_s_all_arms() {
    let mut rng = Rng::new(SEED ^ 18);
    for &h in ARM_HUES {
        for &s in SPECIAL {
            for _ in 0..64 {
                let v = rng.range(-2.0, 2.0);
                check("row18", [h, s, v]);
            }
            for &v in SPECIAL {
                check("row18/special-v", [h, s, v]);
            }
        }
    }
}

// --- Row 19: special h ---------------------------------------------------
#[test]
fn row19_special_h() {
    let mut rng = Rng::new(SEED ^ 19);
    for &h in SPECIAL {
        for _ in 0..2000 {
            let s = rng.range(f32::MIN_POSITIVE, 1.0);
            let v = rng.range(-2.0, 2.0);
            check("row19", [h, s, v]);
        }
    }
    // hues whose /60 straddles the int-conversion boundary
    for &h in &[
        128849018880.0f32, // 2^31 * 60
        128849010000.0,
        -128849018880.0,
        128849011200.0, // just under
        f32::MAX,
        f32::MIN,
    ] {
        for &s in SPECIAL {
            for &v in SPECIAL {
                check("row19/boundary", [h, s, v]);
            }
        }
    }
}

// --- Row 20: fully random bit patterns -----------------------------------
#[test]
fn row20_uniform_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..200_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        check("row20", src);
    }
}

// --- Row 21: exhaustive cross-product of the boundary pool ---------------
#[test]
fn row21_exhaustive_special_cross_product() {
    for &h in SPECIAL {
        for &s in SPECIAL {
            for &v in SPECIAL {
                check("row21", [h, s, v]);
            }
        }
    }
}

// --- Rows 22..26: aliasing / overlap / alignment --------------------------

/// Run both libraries with `dest` and `src` pointing into the same arena at the
/// given element offsets, and compare the whole arena bit-for-bit.
#[track_caller]
fn check_aliased(row: &str, arena: &[f32], dest_off: usize, src_off: usize) {
    let l = libs();
    let mut c_arena: Vec<f32> = arena.to_vec();
    let mut r_arena: Vec<f32> = arena.to_vec();
    unsafe {
        (l.c)(
            c_arena.as_mut_ptr().add(dest_off),
            c_arena.as_ptr().add(src_off),
        );
        (l.rust)(
            r_arena.as_mut_ptr().add(dest_off),
            r_arena.as_ptr().add(src_off),
        );
    }
    let cb: Vec<u32> = c_arena.iter().map(|f| f.to_bits()).collect();
    let rb: Vec<u32> = r_arena.iter().map(|f| f.to_bits()).collect();
    assert_eq!(
        cb, rb,
        "[{row}] aliased mismatch dest_off={dest_off} src_off={src_off} arena={:?}",
        arena.iter().map(|f| fmt(*f)).collect::<Vec<_>>()
    );
}

// --- Row 22: dest == src, full path -------------------------------------
#[test]
fn row22_full_alias() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..20_000 {
        let s = rng.range(f32::MIN_POSITIVE, 1.0);
        let arena = vec![rng.range(-720.0, 720.0), s, rng.range(-2.0, 2.0)];
        check_aliased("row22", &arena, 0, 0);
    }
    for &h in ARM_HUES {
        for &s in SPECIAL {
            for &v in SPECIAL {
                check_aliased("row22/special", &[h, s, v], 0, 0);
            }
        }
    }
}

// --- Row 23: dest == src on the early path ------------------------------
#[test]
fn row23_full_alias_early_path() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..20_000 {
        let z = if rng.next_u32() & 1 == 0 { 0.0 } else { -0.0 };
        let arena = vec![rng.any_f32(), z, rng.any_f32()];
        check_aliased("row23", &arena, 0, 0);
    }
}

// --- Row 24: dest == src + 1 -------------------------------------------
#[test]
fn row24_forward_overlap() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..20_000 {
        let arena: Vec<f32> = (0..5).map(|_| rng.any_f32()).collect();
        check_aliased("row24", &arena, 1, 0);
        let arena2 = vec![
            rng.range(-720.0, 720.0),
            rng.range(f32::MIN_POSITIVE, 1.0),
            rng.range(-2.0, 2.0),
            rng.any_f32(),
            rng.any_f32(),
        ];
        check_aliased("row24/canonical", &arena2, 1, 0);
    }
}

// --- Row 25: dest == src - 1 -------------------------------------------
#[test]
fn row25_backward_overlap() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..20_000 {
        let arena: Vec<f32> = (0..5).map(|_| rng.any_f32()).collect();
        check_aliased("row25", &arena, 0, 1);
        let arena2 = vec![
            rng.any_f32(),
            rng.range(-720.0, 720.0),
            rng.range(f32::MIN_POSITIVE, 1.0),
            rng.range(-2.0, 2.0),
            rng.any_f32(),
        ];
        check_aliased("row25/canonical", &arena2, 0, 1);
        // also offset-2 overlaps
        check_aliased("row25/off2", &arena, 0, 2);
        check_aliased("row25/off2b", &arena, 2, 0);
    }
}

// --- Row 26: unaligned src / dest --------------------------------------
#[test]
fn row26_unaligned_buffers() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 26);

    for _ in 0..20_000 {
        let src_vals = [
            rng.range(-720.0, 720.0),
            rng.range(f32::MIN_POSITIVE, 1.0),
            rng.range(-2.0, 2.0),
        ];
        for src_off in 0..4usize {
            for dst_off in 0..4usize {
                // 32-byte u8 arenas, floats written at arbitrary byte offsets
                let mut src_arena = [0u8; 32];
                for (i, v) in src_vals.iter().enumerate() {
                    src_arena[src_off + i * 4..src_off + i * 4 + 4]
                        .copy_from_slice(&v.to_le_bytes());
                }
                let mut c_dst = [0xAAu8; 32];
                let mut r_dst = [0xAAu8; 32];
                unsafe {
                    (l.c)(
                        c_dst.as_mut_ptr().add(dst_off) as *mut f32,
                        src_arena.as_ptr().add(src_off) as *const f32,
                    );
                    (l.rust)(
                        r_dst.as_mut_ptr().add(dst_off) as *mut f32,
                        src_arena.as_ptr().add(src_off) as *const f32,
                    );
                }
                assert_eq!(
                    c_dst, r_dst,
                    "row26 unaligned mismatch src_off={src_off} dst_off={dst_off} vals={src_vals:?}"
                );
            }
        }
    }
}

// --- Row 27: canary guards (exactly 3 reads / 3 writes) ------------------
#[test]
fn row27_canary_guards() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..20_000 {
        // src padded with poison either side; a 4th-element read would change
        // the result, and a 4th-element write would break the canary.
        let poison = rng.any_f32();
        let src_pad = [
            poison,
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            poison,
        ];
        let mut c_buf = [f32::from_bits(CANARY); 8];
        let mut r_buf = [f32::from_bits(CANARY); 8];
        unsafe {
            (l.c)(c_buf.as_mut_ptr().add(2), src_pad.as_ptr().add(1));
            (l.rust)(r_buf.as_mut_ptr().add(2), src_pad.as_ptr().add(1));
        }
        assert_eq!(
            c_buf.map(f32::to_bits),
            r_buf.map(f32::to_bits),
            "row27 mismatch src={src_pad:?}"
        );
        // canaries outside the 3-float window must be untouched in BOTH
        for i in [0usize, 1, 5, 6, 7] {
            assert_eq!(c_buf[i].to_bits(), CANARY, "row27 C wrote outside at {i}");
            assert_eq!(r_buf[i].to_bits(), CANARY, "row27 Rust wrote outside at {i}");
        }
    }
}

// --- Row 28: repeated invocations, no hidden state ----------------------
#[test]
fn row28_no_hidden_state() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 28);
    let mut c_buf = [0.0f32; 3];
    let mut r_buf = [0.0f32; 3];

    // Interleave many calls reusing the SAME buffers, including alternating
    // between the early-return path and the full path.
    for i in 0..100_000 {
        let src = if i % 3 == 0 {
            [rng.any_f32(), 0.0, rng.any_f32()]
        } else {
            [
                rng.range(-720.0, 720.0),
                rng.range(f32::MIN_POSITIVE, 1.0),
                rng.range(-2.0, 2.0),
            ]
        };
        unsafe {
            (l.c)(c_buf.as_mut_ptr(), src.as_ptr());
            (l.rust)(r_buf.as_mut_ptr(), src.as_ptr());
        }
        assert_eq!(
            c_buf.map(f32::to_bits),
            r_buf.map(f32::to_bits),
            "row28 mismatch at iteration {i} src={}",
            fmt3(&src)
        );
    }

    // And re-running a fixed input after all that noise must still agree.
    for &h in ARM_HUES {
        let src = [h, 0.7, 0.3];
        unsafe {
            (l.c)(c_buf.as_mut_ptr(), src.as_ptr());
            (l.rust)(r_buf.as_mut_ptr(), src.as_ptr());
        }
        assert_eq!(
            c_buf.map(f32::to_bits),
            r_buf.map(f32::to_bits),
            "row28/replay mismatch h={}",
            fmt(h)
        );
    }
}

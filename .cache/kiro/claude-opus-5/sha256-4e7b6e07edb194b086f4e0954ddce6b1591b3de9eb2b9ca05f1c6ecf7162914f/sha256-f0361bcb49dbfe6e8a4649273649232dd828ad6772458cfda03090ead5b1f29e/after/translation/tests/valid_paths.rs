//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the full written region (plus guard slots) bit-for-bit.

mod common;

use common::*;
use std::os::raw::c_int;

/// Randomized-iteration count per row.
const N: usize = 400;

// --------------------------------------------------------------------------
// Row 1-5: each small `size` individually, randomized radius.
// --------------------------------------------------------------------------

fn fixed_size_row(size: c_int, seed: u64, ctx: &str) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let radius = rng.logunif_f32(1e-3, 1e3);
        check(size, radius, ctx);
    }
    // plus a few deterministic anchors
    for &radius in &[1.0f32, 1.6, 2.0, 0.5, 3.0, 10.0, 0.1, 100.0] {
        check(size, radius, ctx);
    }
}

#[test]
fn row01_size_1() {
    fixed_size_row(1, 0x1001, "row01 size=1");
}

#[test]
fn row02_size_2_oob_write() {
    // size=2 -> hsize=1 -> 3 writes for a 2-element request.
    assert_eq!(written_len(2), 3);
    fixed_size_row(2, 0x1002, "row02 size=2");
}

#[test]
fn row03_size_3() {
    assert_eq!(written_len(3), 3);
    fixed_size_row(3, 0x1003, "row03 size=3");
}

#[test]
fn row04_size_4() {
    assert_eq!(written_len(4), 5);
    fixed_size_row(4, 0x1004, "row04 size=4");
}

#[test]
fn row05_size_5() {
    assert_eq!(written_len(5), 5);
    fixed_size_row(5, 0x1005, "row05 size=5");
}

// --------------------------------------------------------------------------
// Row 6/7: randomized odd / even sizes.
// --------------------------------------------------------------------------

#[test]
fn row06_random_odd_sizes() {
    let mut rng = Rng::new(0x1006);
    for _ in 0..N {
        let mut size = rng.int_in(7, 129) as c_int;
        size |= 1; // force odd
        let radius = rng.logunif_f32(1e-3, 1e3);
        check(size, radius, "row06 odd size");
    }
}

#[test]
fn row07_random_even_sizes() {
    let mut rng = Rng::new(0x1007);
    for _ in 0..N {
        let size = (rng.int_in(6, 128) as c_int) & !1; // force even
        assert_eq!(written_len(size), size as usize + 1);
        let radius = rng.logunif_f32(1e-3, 1e3);
        check(size, radius, "row07 even size");
    }
}

// --------------------------------------------------------------------------
// Row 8: large sizes.
// --------------------------------------------------------------------------

#[test]
fn row08_large_sizes() {
    let mut rng = Rng::new(0x1008);
    for &size in &[1023i32, 1024, 4095, 4096] {
        for _ in 0..60 {
            let radius = rng.logunif_f32(1e-3, 1e4);
            check(size, radius, "row08 large size");
        }
        for &radius in &[1.0f32, 50.0, 500.0, 0.01] {
            check(size, radius, "row08 large size anchor");
        }
    }
}

// --------------------------------------------------------------------------
// Rows 9-13: radius scale regimes crossed with randomized sizes.
// --------------------------------------------------------------------------

fn radius_regime_row(lo: f32, hi: f32, seed: u64, ctx: &str) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let size = rng.int_in(0, 96) as c_int;
        let radius = rng.logunif_f32(lo, hi);
        check(size, radius, ctx);
    }
}

#[test]
fn row09_radius_tiny() {
    radius_regime_row(1e-38, 1e-6, 0x1009, "row09 R_TINY");
}

#[test]
fn row10_radius_small() {
    radius_regime_row(0.05, 0.9, 0x100A, "row10 R_SMALL");
}

#[test]
fn row11_radius_mid() {
    radius_regime_row(0.9, 8.0, 0x100B, "row11 R_MID");
}

#[test]
fn row12_radius_large() {
    radius_regime_row(8.0, 1e6, 0x100C, "row12 R_LARGE");
}

#[test]
fn row13_radius_huge() {
    radius_regime_row(1e30, f32::MAX, 0x100D, "row13 R_HUGE");
}

// --------------------------------------------------------------------------
// Row 14: negative radius over every regime.
// --------------------------------------------------------------------------

#[test]
fn row14_negative_radius_all_regimes() {
    let regimes: [(f32, f32); 5] = [
        (1e-38, 1e-6),
        (0.05, 0.9),
        (0.9, 8.0),
        (8.0, 1e6),
        (1e30, f32::MAX),
    ];
    let mut rng = Rng::new(0x100E);
    for (lo, hi) in regimes {
        for _ in 0..120 {
            let size = rng.int_in(0, 96) as c_int;
            let radius = -rng.logunif_f32(lo, hi);
            check(size, radius, "row14 R_NEG");
        }
    }
}

// --------------------------------------------------------------------------
// Row 15: full property fuzz over arbitrary finite radius bit patterns.
// --------------------------------------------------------------------------

#[test]
fn row15_property_fuzz_finite_radius() {
    let mut rng = Rng::new(0x100F);
    for _ in 0..4000 {
        let size = rng.int_in(0, 96) as c_int;
        let radius = rng.finite_f32();
        check(size, radius, "row15 fuzz finite radius");
    }
}

// --------------------------------------------------------------------------
// Row 16: sizes down through the degenerate range, mid radius.
// --------------------------------------------------------------------------

#[test]
fn row16_size_range_including_degenerate() {
    let mut rng = Rng::new(0x1010);
    for _ in 0..N {
        let size = rng.int_in(-4, 96) as c_int;
        let radius = rng.logunif_f32(0.9, 8.0);
        check(size, radius, "row16 size in [-4,96]");
    }
    for size in -4..=8 {
        for &radius in &[1.0f32, 2.5, 0.3, 40.0] {
            check(size, radius, "row16 exhaustive small size");
        }
    }
}

// --------------------------------------------------------------------------
// Row 17: poison pattern -> which slots are written / left alone.
// --------------------------------------------------------------------------

#[test]
fn row17_poison_pattern_untouched_tail() {
    let mut rng = Rng::new(0x1011);
    for _ in 0..N {
        let size = (rng.int_in(2, 64) as c_int) & !1;
        let radius = rng.logunif_f32(1e-2, 1e2);
        let n = buffer_len(size);
        // Distinct poison per slot so a stray write of the "right" value is
        // still detected.
        let fill: Vec<f32> = (0..n)
            .map(|i| f32::from_bits(0xDEAD_0000u32 | (i as u32 & 0xFFFF)))
            .collect();
        let (c, r) = run_both(size, radius, &fill);
        assert_same(size, radius, &c, &r, "row17 poison");

        // Also assert the C really writes exactly written_len(size) slots and
        // Rust agrees about which ones stayed poisoned.
        let w = written_len(size);
        for i in w..n {
            assert_eq!(
                c[i], fill[i].to_bits(),
                "C wrote past written_len at [{i}] for size={size}"
            );
            assert_eq!(
                r[i], fill[i].to_bits(),
                "Rust wrote past written_len at [{i}] for size={size}"
            );
        }
    }
}

// --------------------------------------------------------------------------
// Row 18: repeated in-place invocation (statelessness).
// --------------------------------------------------------------------------

#[test]
fn row18_repeated_inplace_calls() {
    let cf = c_gaussian_kernel();
    let rf = rust_gaussian_kernel();
    let mut rng = Rng::new(0x1012);
    for _ in 0..N {
        let size = rng.int_in(1, 80) as c_int;
        let n = buffer_len(size);
        let r1 = rng.logunif_f32(1e-2, 1e2);
        let r2 = rng.logunif_f32(1e-2, 1e2);
        let r3 = rng.logunif_f32(1e-2, 1e2);

        let mut cbuf = vec![poison(); n];
        let mut rbuf = vec![poison(); n];
        unsafe {
            for radius in [r1, r2, r3] {
                cf(cbuf.as_mut_ptr(), size, radius);
                rf(rbuf.as_mut_ptr(), size, radius);
            }
        }
        let c: Vec<u32> = cbuf.iter().map(|f| f.to_bits()).collect();
        let r: Vec<u32> = rbuf.iter().map(|f| f.to_bits()).collect();
        assert_same(size, r3, &c, &r, "row18 repeated in-place");

        // A fresh single call with r3 must give the same answer (stateless).
        let (c1, r1b) = run_both_poisoned(size, r3);
        assert_same(size, r3, &c, &c1, "row18 C not stateless");
        assert_same(size, r3, &r, &r1b, "row18 Rust not stateless");
    }
}

// --------------------------------------------------------------------------
// Row 19: radius values sitting on the `v > 0` clamp boundary.
// --------------------------------------------------------------------------

/// `v = 1/expf(x*x) - s2` flips sign at `x*x == sigma^2*tetha`, i.e.
/// `|x| == 1.6*1.5 == 2.4`. With `x = r * (1.6/radius)` that is
/// `radius == r * 1.6 / 2.4` for the element at offset `r`.
#[test]
fn row19_clamp_boundary_radii() {
    let mut cases: Vec<f32> = Vec::new();
    for r in 1..=48i32 {
        let base = (r as f32) * 1.6f32 / 2.4f32;
        // walk a few ULPs either side of the exact flip point
        let b = base.to_bits();
        for d in -4i32..=4 {
            let bits = (b as i64 + d as i64) as u32;
            let v = f32::from_bits(bits);
            if v.is_finite() && v > 0.0 {
                cases.push(v);
                cases.push(-v);
            }
        }
    }
    for &radius in &cases {
        for size in [1i32, 2, 3, 4, 5, 32, 33, 64, 65, 96, 97] {
            check(size, radius, "row19 clamp boundary");
        }
    }
}

// --------------------------------------------------------------------------
// Row 20: destination in the middle of a larger allocation.
// --------------------------------------------------------------------------

#[test]
fn row20_offset_destination() {
    let cf = c_gaussian_kernel();
    let rf = rust_gaussian_kernel();
    let mut rng = Rng::new(0x1013);
    for _ in 0..N {
        let size = rng.int_in(0, 64) as c_int;
        let radius = rng.logunif_f32(1e-3, 1e3);
        let off = rng.int_in(0, 7) as usize;
        let n = off + buffer_len(size) + 4;

        let fill: Vec<f32> = (0..n)
            .map(|i| f32::from_bits(0xCAFE_0000u32 | (i as u32 & 0xFFFF)))
            .collect();
        let mut cbuf = fill.clone();
        let mut rbuf = fill.clone();
        unsafe {
            cf(cbuf.as_mut_ptr().add(off), size, radius);
            rf(rbuf.as_mut_ptr().add(off), size, radius);
        }
        let c: Vec<u32> = cbuf.iter().map(|f| f.to_bits()).collect();
        let r: Vec<u32> = rbuf.iter().map(|f| f.to_bits()).collect();
        assert_same(size, radius, &c, &r, "row20 offset dest");
        // prefix must be untouched by both
        for i in 0..off {
            assert_eq!(c[i], fill[i].to_bits(), "C clobbered prefix [{i}]");
            assert_eq!(r[i], fill[i].to_bits(), "Rust clobbered prefix [{i}]");
        }
    }
}

// --------------------------------------------------------------------------
// Sanity: the two libraries really are two distinct files.
// --------------------------------------------------------------------------

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let c = c_so_path();
    let r = rust_so_path();
    assert_ne!(c, r, "C and Rust .so paths must differ");
    assert!(c.exists(), "missing {}", c.display());
    assert!(r.exists(), "missing {}", r.display());
    eprintln!("C   .so: {}", c.display());
    eprintln!("Rust.so: {}", r.display());
}

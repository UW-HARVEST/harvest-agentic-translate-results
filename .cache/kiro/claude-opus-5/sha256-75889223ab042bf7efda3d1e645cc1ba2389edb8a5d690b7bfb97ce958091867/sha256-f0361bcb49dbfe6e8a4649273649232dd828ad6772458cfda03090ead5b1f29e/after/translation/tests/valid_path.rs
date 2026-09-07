//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both libraries are loaded from their `.so` and called only through the
//! exported `normalize` symbol. Every row uses many randomized inputs with a
//! fixed seed; outputs are compared as raw `u32` bit patterns so that signed
//! zero and NaN payloads are compared exactly.

#[path = "harness/mod.rs"]
mod harness;

use harness::*;
use std::ffi::c_int;

const ITERS_SMALL: usize = 300;
const ITERS_MED: usize = 120;
const ITERS_LARGE: usize = 30;

fn mkvec<F: FnMut(&mut Rng) -> f32>(rng: &mut Rng, n: usize, mut f: F) -> Vec<f32> {
    (0..n).map(|_| f(rng)).collect()
}

// --- row 1 / 2: size == 0 -------------------------------------------------

#[test]
fn cfg_row01_size0_disjoint() {
    for it in 0..16 {
        let mut rng = Rng::new(1_000 + it);
        // Non-empty backing storage, but size == 0 is passed.
        let src = mkvec(&mut rng, 8, |r| r.unit());
        let l = libs();
        let s_c = Padded::filled(&src, 0);
        let s_r = Padded::filled(&src, 0);
        let mut d_c = Padded::new(8, 0);
        let mut d_r = Padded::new(8, 0);
        unsafe { (l.c)(d_c.ptr(), s_c.cptr(), 0) };
        unsafe { (l.r)(d_r.ptr(), s_r.cptr(), 0) };
        assert_eq!(bits(d_c.payload()), bits(d_r.payload()), "row01 payload");
        assert_eq!(d_c.canaries(), d_r.canaries(), "row01 canary");
        // Ground truth: a 0-byte memset must leave the destination pristine.
        assert_eq!(bits(d_c.payload()), bits(Padded::new(8, 0).payload()), "row01 C wrote");
    }
}

#[test]
fn cfg_row02_size0_inplace() {
    for it in 0..16 {
        let mut rng = Rng::new(2_000 + it);
        let src = mkvec(&mut rng, 8, |r| r.unit());
        let out = diff_inplace("row02", &src, 0, 0);
        assert_eq!(bits(&out), bits(&src), "row02: size 0 must not modify buffer");
    }
}

// --- rows 3..6: tiny sizes ------------------------------------------------

#[test]
fn cfg_row03_size1_disjoint_uniform() {
    for it in 0..ITERS_SMALL {
        let mut rng = Rng::new(3_000 + it as u64);
        let src = mkvec(&mut rng, 1, |r| r.unit());
        diff_disjoint("row03", &src, 1, 0, 0);
    }
}

#[test]
fn cfg_row04_size1_inplace_uniform() {
    for it in 0..ITERS_SMALL {
        let mut rng = Rng::new(4_000 + it as u64);
        let src = mkvec(&mut rng, 1, |r| r.unit());
        diff_inplace("row04", &src, 1, 0);
    }
}

#[test]
fn cfg_row05_size1_zero() {
    for (i, v) in [0.0f32, -0.0f32].into_iter().enumerate() {
        let src = vec![v];
        diff_disjoint(&format!("row05[{i}]"), &src, 1, 0, 0);
        diff_inplace(&format!("row05ip[{i}]"), &src, 1, 0);
    }
}

#[test]
fn cfg_row06_size2_disjoint_uniform() {
    for it in 0..ITERS_SMALL {
        let mut rng = Rng::new(6_000 + it as u64);
        let src = mkvec(&mut rng, 2, |r| r.unit());
        diff_disjoint("row06", &src, 2, 0, 0);
    }
}

// --- rows 7..12: size buckets, disjoint + in place ------------------------

fn bucket(stream_base: u64, label: &str, lo: usize, hi: usize, iters: usize, inplace: bool) {
    for it in 0..iters {
        let mut rng = Rng::new(stream_base + it as u64);
        let n = lo + rng.below(hi - lo + 1);
        let src = mkvec(&mut rng, n, |r| r.unit());
        if inplace {
            diff_inplace(label, &src, n as c_int, 0);
        } else {
            diff_disjoint(label, &src, n as c_int, 0, 0);
        }
    }
}

#[test]
fn cfg_row07_small_disjoint_uniform() {
    bucket(7_000, "row07", 3, 8, ITERS_SMALL, false);
}

#[test]
fn cfg_row08_small_inplace_uniform() {
    bucket(8_000, "row08", 3, 8, ITERS_SMALL, true);
}

#[test]
fn cfg_row09_medium_disjoint_uniform() {
    bucket(9_000, "row09", 9, 64, ITERS_MED, false);
}

#[test]
fn cfg_row10_medium_inplace_uniform() {
    bucket(10_000, "row10", 9, 64, ITERS_MED, true);
}

#[test]
fn cfg_row11_large_disjoint_uniform() {
    bucket(11_000, "row11", 65, 4096, ITERS_LARGE, false);
}

#[test]
fn cfg_row12_large_inplace_uniform() {
    bucket(12_000, "row12", 65, 4096, ITERS_LARGE, true);
}

// --- rows 13/14: arbitrary bit patterns (NaN, inf, subnormal, huge) -------

#[test]
fn cfg_row13_random_bitpatterns_disjoint() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(13_000 + it as u64);
        let n = 1 + rng.below(4096);
        let src = mkvec(&mut rng, n, |r| r.any_f32());
        diff_disjoint("row13", &src, n as c_int, 0, 0);
    }
}

#[test]
fn cfg_row14_random_bitpatterns_inplace() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(14_000 + it as u64);
        let n = 1 + rng.below(4096);
        let src = mkvec(&mut rng, n, |r| r.any_f32());
        diff_inplace("row14", &src, n as c_int, 0);
    }
}

// --- rows 15..18: overflow / underflow of the accumulator -----------------

#[test]
fn cfg_row15_huge_overflow_disjoint() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(15_000 + it as u64);
        let n = 1 + rng.below(512);
        let src = mkvec(&mut rng, n, |r| r.scaled(64, 126));
        let out = diff_disjoint("row15", &src, n as c_int, 0, 0);
        // Sanity: this row is supposed to drive `sum` to +inf for n >= 2.
        let _ = out;
    }
}

#[test]
fn cfg_row16_huge_overflow_inplace() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(16_000 + it as u64);
        let n = 1 + rng.below(512);
        let src = mkvec(&mut rng, n, |r| r.scaled(64, 126));
        diff_inplace("row16", &src, n as c_int, 0);
    }
}

#[test]
fn cfg_row17_tiny_underflow_disjoint() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(17_000 + it as u64);
        let n = 1 + rng.below(512);
        let src = mkvec(&mut rng, n, |r| r.scaled(-149, -80));
        diff_disjoint("row17", &src, n as c_int, 0, 0);
    }
}

#[test]
fn cfg_row18_tiny_underflow_inplace() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(18_000 + it as u64);
        let n = 1 + rng.below(512);
        let src = mkvec(&mut rng, n, |r| r.scaled(-149, -80));
        diff_inplace("row18", &src, n as c_int, 0);
    }
}

// --- row 19: subnormal but strictly positive accumulator ------------------

#[test]
fn cfg_row19_subnormal_sum_disjoint() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(19_000 + it as u64);
        let n = 1 + rng.below(512);
        // x ~ 2^-70 => x*x ~ 2^-140, subnormal but non-zero.
        let src = mkvec(&mut rng, n, |r| r.scaled(-75, -62));
        diff_disjoint("row19", &src, n as c_int, 0, 0);
        diff_inplace("row19ip", &src, n as c_int, 0);
    }
}

// --- row 20: mixed magnitudes (rounding/accumulation-order sensitive) -----

#[test]
fn cfg_row20_mixed_magnitude_disjoint() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(20_000 + it as u64);
        let n = 1 + rng.below(512);
        let src = mkvec(&mut rng, n, |r| r.scaled(-66, 66));
        diff_disjoint("row20", &src, n as c_int, 0, 0);
        diff_inplace("row20ip", &src, n as c_int, 0);
    }
}

// --- rows 21/22: explicit inf / NaN elements ------------------------------

#[test]
fn cfg_row21_inf_elements_disjoint() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(21_000 + it as u64);
        let n = 1 + rng.below(512);
        let mut src = mkvec(&mut rng, n, |r| r.unit());
        // Sprinkle 1..=3 infinities at random positions.
        let k = 1 + rng.below(3);
        for _ in 0..k {
            let p = rng.below(n);
            src[p] = if rng.next_u64() & 1 == 0 { f32::INFINITY } else { f32::NEG_INFINITY };
        }
        diff_disjoint("row21", &src, n as c_int, 0, 0);
        diff_inplace("row21ip", &src, n as c_int, 0);
    }
}

#[test]
fn cfg_row22_nan_elements_disjoint() {
    // Quiet, signalling and payload-bearing NaNs, both signs.
    let nan_bits: [u32; 6] = [
        0x7FC0_0000, // canonical qNaN
        0xFFC0_0000, // negative qNaN (x86 default NaN)
        0x7F80_0001, // sNaN
        0xFF80_0001, // negative sNaN
        0x7FBF_FFFF, // sNaN max payload
        0x7FFF_FFFF, // qNaN max payload
    ];
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(22_000 + it as u64);
        let n = 1 + rng.below(512);
        let mut src = mkvec(&mut rng, n, |r| r.unit());
        let k = 1 + rng.below(3);
        for _ in 0..k {
            let p = rng.below(n);
            src[p] = f32::from_bits(nan_bits[rng.below(nan_bits.len())]);
        }
        diff_disjoint("row22", &src, n as c_int, 0, 0);
        diff_inplace("row22ip", &src, n as c_int, 0);
    }
}

// --- row 23: all elements identical --------------------------------------

#[test]
fn cfg_row23_identical_elements() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(23_000 + it as u64);
        let n = 1 + rng.below(512);
        let v = if it % 3 == 0 {
            // exact power of two
            (2.0f32).powi(-20 + (rng.below(40) as i32))
        } else {
            rng.unit()
        };
        let src = vec![v; n];
        diff_disjoint("row23", &src, n as c_int, 0, 0);
        diff_inplace("row23ip", &src, n as c_int, 0);
    }
}

// --- row 24: exact small integers ----------------------------------------

#[test]
fn cfg_row24_small_integers() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(24_000 + it as u64);
        let n = 1 + rng.below(512);
        let src = mkvec(&mut rng, n, |r| (r.below(21) as f32) - 10.0);
        diff_disjoint("row24", &src, n as c_int, 0, 0);
        diff_inplace("row24ip", &src, n as c_int, 0);
    }
}

// --- rows 25/26: overlapping windows -------------------------------------

#[test]
fn cfg_row25_forward_overlap() {
    // dest = base + k, src = base  (dest ahead of src)
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(25_000 + it as u64);
        let n = 1 + rng.below(256);
        let k = 1 + rng.below(8);
        let buf = mkvec(&mut rng, n + k, |r| r.unit());
        diff_overlap("row25", &buf, n, k, 0);
    }
}

#[test]
fn cfg_row26_backward_overlap() {
    // dest = base, src = base + k  (src ahead of dest)
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(26_000 + it as u64);
        let n = 1 + rng.below(256);
        let k = 1 + rng.below(8);
        let buf = mkvec(&mut rng, n + k, |r| r.unit());
        diff_overlap("row26", &buf, n, 0, k);
    }
}

// --- row 27: misaligned buffers (all 16 combinations) --------------------

#[test]
fn cfg_row27_misaligned_buffers() {
    for dm in 0..4usize {
        for sm in 0..4usize {
            for it in 0..24 {
                let mut rng = Rng::new(27_000 + (dm * 4 + sm) as u64 * 100 + it);
                let n = 1 + rng.below(256);
                let src = mkvec(&mut rng, n, |r| r.unit());
                diff_disjoint("row27", &src, n as c_int, dm, sm);
                diff_inplace("row27ip", &src, n as c_int, dm);
            }
        }
    }
}

// --- rows 28/29: exact vectorization-width boundaries --------------------

const BOUNDARIES: [usize; 15] = [3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65];

#[test]
fn cfg_row28_vector_width_boundaries() {
    for (bi, &n) in BOUNDARIES.iter().enumerate() {
        for it in 0..40 {
            let mut rng = Rng::new(28_000 + bi as u64 * 1000 + it);
            let src = mkvec(&mut rng, n, |r| r.unit());
            diff_disjoint("row28", &src, n as c_int, 0, 0);
        }
    }
}

#[test]
fn cfg_row29_vector_width_boundaries_inplace() {
    for (bi, &n) in BOUNDARIES.iter().enumerate() {
        for it in 0..40 {
            let mut rng = Rng::new(29_000 + bi as u64 * 1000 + it);
            let src = mkvec(&mut rng, n, |r| r.unit());
            diff_inplace("row29", &src, n as c_int, 0);
        }
    }
}

// --- row 30: sparse single non-zero at every position --------------------

#[test]
fn cfg_row30_sparse_single_nonzero() {
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(30_000 + it as u64);
        let n = 1 + rng.below(64);
        for p in 0..n {
            let mut src = vec![0.0f32; n];
            src[p] = rng.unit();
            diff_disjoint("row30", &src, n as c_int, 0, 0);
            diff_inplace("row30ip", &src, n as c_int, 0);
        }
    }
}

// --- row 31: already-normalized input (sum exactly 1.0) ------------------

#[test]
fn cfg_row31_already_normalized() {
    // n a power of 4 => 1/sqrt(n) is an exact power of two => sum == 1.0f.
    for (ni, &n) in [1usize, 4, 16, 64, 256].iter().enumerate() {
        for it in 0..40 {
            let mut rng = Rng::new(31_000 + ni as u64 * 1000 + it);
            let mag = 1.0f32 / (n as f32).sqrt();
            let src: Vec<f32> =
                (0..n).map(|_| if rng.next_u64() & 1 == 0 { mag } else { -mag }).collect();
            diff_disjoint("row31", &src, n as c_int, 0, 0);
            diff_inplace("row31ip", &src, n as c_int, 0);
        }
    }
    // Sparse unit vectors: exactly one +-1.0, rest zero => sum == 1.0f.
    for n in 1..=64usize {
        let mut rng = Rng::new(31_900 + n as u64);
        let p = rng.below(n);
        let mut src = vec![0.0f32; n];
        src[p] = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        diff_disjoint("row31u", &src, n as c_int, 0, 0);
        diff_inplace("row31uip", &src, n as c_int, 0);
    }
}

// --- row 32: repeated calls, no hidden state ----------------------------

#[test]
fn cfg_row32_repeated_calls_no_state() {
    let l = libs();
    for it in 0..ITERS_MED {
        let mut rng = Rng::new(32_000 + it as u64);
        let n = 1 + rng.below(512);
        let src = mkvec(&mut rng, n, |r| r.unit());

        let s_c = Padded::filled(&src, 0);
        let s_r = Padded::filled(&src, 0);
        let mut d_c = Padded::new(n, 0);
        let mut d_r = Padded::new(n, 0);

        let mut snapshots_c: Vec<Vec<u32>> = Vec::new();
        let mut snapshots_r: Vec<Vec<u32>> = Vec::new();
        for _ in 0..3 {
            unsafe { (l.c)(d_c.ptr(), s_c.cptr(), n as c_int) };
            unsafe { (l.r)(d_r.ptr(), s_r.cptr(), n as c_int) };
            snapshots_c.push(bits(d_c.payload()));
            snapshots_r.push(bits(d_r.payload()));
            assert_eq!(
                snapshots_c.last().unwrap(),
                snapshots_r.last().unwrap(),
                "row32: mismatch on repeated call (n={n})"
            );
        }
        assert_eq!(snapshots_c[0], snapshots_c[2], "row32: C is not stateless?");
        assert_eq!(snapshots_r[0], snapshots_r[2], "row32: Rust is not stateless");

        // Also: repeated in-place normalization (feedback through the buffer).
        let mut b_c = Padded::filled(&src, 0);
        let mut b_r = Padded::filled(&src, 0);
        for round in 0..4 {
            unsafe {
                let p = b_c.ptr();
                (l.c)(p, p as *const f32, n as c_int)
            };
            unsafe {
                let p = b_r.ptr();
                (l.r)(p, p as *const f32, n as c_int)
            };
            assert_eq!(
                bits(b_c.payload()),
                bits(b_r.payload()),
                "row32: in-place round {round} mismatch (n={n})"
            );
        }
    }
}

// --- meta: prove we really loaded two distinct .so files -----------------

#[test]
fn meta_two_distinct_shared_objects_loaded() {
    let l = libs();
    assert_ne!(l.c_path, l.r_path);
    assert_ne!(l.c as usize, l.r as usize, "both symbols resolved to the same address");
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.r_path.display());
}

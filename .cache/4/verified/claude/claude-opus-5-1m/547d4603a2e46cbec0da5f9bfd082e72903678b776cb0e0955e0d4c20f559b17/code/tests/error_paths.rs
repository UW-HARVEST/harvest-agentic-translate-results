//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` that can be observed in-process. The rows
//! whose C behaviour is a fatal signal live in `crash_paths.rs` (they need a
//! sub-process to be observable).
//!
//! Each test asserts BOTH
//!   * the C ground truth documented in `ERRORS.md` (so the row is pinned), and
//!   * bit-for-bit equality between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::ffi::c_int;

fn garbage(len: usize, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let mut v = vec![0.0f32; len];
    fill_garbage(&mut rng, &mut v);
    v
}

// ---------------------------------------------------------------------------
// row 1 : size == 0, dest != src  -> memset(dest, 0, 0), nothing written
// ---------------------------------------------------------------------------
#[test]
fn err_01_size_zero_disjoint() {
    let (c, r) = load_impls();
    let base = garbage(24, 1);
    let cout = run_one(&c, &base, 0, 12, 0);
    assert_eq!(bits_of(&cout), bits_of(&base), "C wrote to dest for size == 0");
    assert!(diff_shared(&c, &r, &base, 0, 12, 0).is_ok());
    // also with two separate allocations
    let dest = garbage(8, 2);
    let src = garbage(8, 3);
    let (cd, cs) = run_one_separate(&c, &dest, &src, 0);
    assert_eq!(bits_of(&cd), bits_of(&dest));
    assert_eq!(bits_of(&cs), bits_of(&src));
    diff_separate(&c, &r, &dest, &src, 0).unwrap();
}

// ---------------------------------------------------------------------------
// row 2 : size == 0, dest == src  -> nothing written
// ---------------------------------------------------------------------------
#[test]
fn err_02_size_zero_inplace() {
    let (c, r) = load_impls();
    let base = garbage(16, 4);
    let cout = run_one(&c, &base, 3, 3, 0);
    assert_eq!(bits_of(&cout), bits_of(&base));
    diff_shared(&c, &r, &base, 3, 3, 0).unwrap();
}

// ---------------------------------------------------------------------------
// row 3 : size < 0, dest == src -> no write, returns normally
// ---------------------------------------------------------------------------
#[test]
fn err_03_negative_size_inplace() {
    let (c, r) = load_impls();
    let base = garbage(32, 5);
    for size in [-1i32, -2, -3, -7, -64, -1000, -65536, i32::MIN + 1] {
        let cout = run_one(&c, &base, 4, 4, size);
        assert_eq!(
            bits_of(&cout),
            bits_of(&base),
            "C wrote something for size == {size} in place"
        );
        diff_shared(&c, &r, &base, 4, 4, size)
            .unwrap_or_else(|e| panic!("size={size}: {e}"));
    }
}

// ---------------------------------------------------------------------------
// row 4 : size == INT_MIN, dest == src -> no write
// ---------------------------------------------------------------------------
#[test]
fn err_04_int_min_size_inplace() {
    let (c, r) = load_impls();
    let base = garbage(16, 6);
    let cout = run_one(&c, &base, 0, 0, i32::MIN);
    assert_eq!(bits_of(&cout), bits_of(&base));
    diff_shared(&c, &r, &base, 0, 0, i32::MIN).unwrap();
    // and at a non-zero offset
    diff_shared(&c, &r, &base, 5, 5, i32::MIN).unwrap();
}

// ---------------------------------------------------------------------------
// rows 7-10 : null pointers with non-positive size -> return normally
// ---------------------------------------------------------------------------

/// Calls both implementations; reaching the end of the function is the
/// assertion (a divergence would abort the test process).
fn both_return(c: &Impl, r: &Impl, dest: *mut f32, src: *const f32, size: c_int) {
    unsafe { (c.normalize)(dest, src, size) };
    unsafe { (r.normalize)(dest, src, size) };
}

#[test]
fn err_07_both_null_size_zero() {
    let (c, r) = load_impls();
    both_return(&c, &r, std::ptr::null_mut(), std::ptr::null(), 0);
}

#[test]
fn err_08_null_dest_size_zero() {
    let (c, r) = load_impls();
    let src = [1.0f32, 2.0, 3.0, 4.0];
    both_return(&c, &r, std::ptr::null_mut(), src.as_ptr(), 0);
    assert_eq!(bits_of(&src), bits_of(&[1.0f32, 2.0, 3.0, 4.0]));
}

#[test]
fn err_09_null_src_size_zero() {
    let (c, r) = load_impls();
    let mut cdest = [1.0f32, -2.0, 3.5, f32::NAN];
    let mut rdest = cdest;
    unsafe { (c.normalize)(cdest.as_mut_ptr(), std::ptr::null(), 0) };
    unsafe { (r.normalize)(rdest.as_mut_ptr(), std::ptr::null(), 0) };
    assert_eq!(bits_of(&cdest), bits_of(&rdest));
    // C leaves dest untouched (memset length 0)
    assert_eq!(cdest[0].to_bits(), 1.0f32.to_bits());
    assert_eq!(cdest[3].to_bits(), f32::NAN.to_bits());
}

#[test]
fn err_10_both_null_negative_size() {
    let (c, r) = load_impls();
    for size in [-1i32, -2, -1024, i32::MIN, i32::MIN + 1] {
        both_return(&c, &r, std::ptr::null_mut(), std::ptr::null(), size);
    }
}

// ---------------------------------------------------------------------------
// row 13 : all-zero input, dest != src -> dest zero filled (+0.0 bits)
// ---------------------------------------------------------------------------
#[test]
fn err_13_all_zero_input_zero_fills() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(13);
    for size_i in 1i32..=40 {
        let size = size_i as usize;
        for trial in 0..8 {
            let mut base = vec![0.0f32; 2 * size + 8];
            fill_garbage(&mut rng, &mut base);
            // src region: mixture of +0.0 / -0.0 => sum == +0.0
            for i in 0..size {
                base[i] = if trial % 2 == 0 && rng.bool() { -0.0 } else { 0.0 };
            }
            let dest_off = size + 4;
            let cout = run_one(&c, &base, dest_off, 0, size_i);
            for i in 0..size {
                assert_eq!(
                    cout[dest_off + i].to_bits(),
                    0u32,
                    "C did not write +0.0 at {i} (size={size_i})"
                );
            }
            diff_shared(&c, &r, &base, dest_off, 0, size_i).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// row 14 : all-zero input, dest == src -> buffer untouched (-0.0 preserved)
// ---------------------------------------------------------------------------
#[test]
fn err_14_all_zero_input_inplace_untouched() {
    let (c, r) = load_impls();
    for size_i in 1i32..=40 {
        let size = size_i as usize;
        let mut base = vec![-0.0f32; size + 4];
        for i in 0..size {
            if i % 3 == 0 {
                base[i] = 0.0;
            }
        }
        let cout = run_one(&c, &base, 0, 0, size_i);
        assert_eq!(
            bits_of(&cout),
            bits_of(&base),
            "C modified the buffer in place for an all-zero input (size={size_i})"
        );
        diff_shared(&c, &r, &base, 0, 0, size_i).unwrap();
    }
}

// ---------------------------------------------------------------------------
// row 15 : sum underflows to 0 although src != 0, dest != src -> zero fill
// ---------------------------------------------------------------------------
#[test]
fn err_15_underflow_to_zero_zero_fills() {
    let (c, r) = load_impls();
    // 1e-25f * 1e-25f == 0.0f in f32
    for &v in &[1.0e-25f32, -1.0e-25, 1.0e-30, f32::MIN_POSITIVE, 1.0e-45] {
        for size_i in 1i32..=16 {
            let size = size_i as usize;
            let mut base = vec![0.0f32; 2 * size + 8];
            for (i, b) in base.iter_mut().enumerate() {
                *b = 100.0 + i as f32;
            }
            for i in 0..size {
                base[i] = v;
            }
            let dest_off = size + 4;
            let cout = run_one(&c, &base, dest_off, 0, size_i);
            let sum: f32 = (0..size).map(|_| v * v).sum();
            if sum == 0.0 {
                for i in 0..size {
                    assert_eq!(
                        cout[dest_off + i].to_bits(),
                        0u32,
                        "C should zero-fill on underflow (v={v:e}, size={size_i})"
                    );
                }
            }
            diff_shared(&c, &r, &base, dest_off, 0, size_i).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// row 16 : sum underflows to 0, dest == src -> untouched
// ---------------------------------------------------------------------------
#[test]
fn err_16_underflow_to_zero_inplace_untouched() {
    let (c, r) = load_impls();
    for &v in &[1.0e-25f32, -1.0e-25, 1.0e-30, 1.0e-45] {
        for size_i in 1i32..=16 {
            let size = size_i as usize;
            let base = vec![v; size + 4];
            let cout = run_one(&c, &base, 0, 0, size_i);
            let sum: f32 = (0..size).map(|_| v * v).sum();
            if sum == 0.0 {
                assert_eq!(bits_of(&cout), bits_of(&base), "v={v:e} size={size_i}");
            }
            diff_shared(&c, &r, &base, 0, 0, size_i).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// row 17 : NaN in src, dest != src -> dest zero filled, NaN not propagated
// ---------------------------------------------------------------------------
#[test]
fn err_17_nan_input_zero_fills() {
    let (c, r) = load_impls();
    let nans = [
        f32::NAN.to_bits(),
        0x7fc0_0000, // canonical quiet
        0xffc0_0000, // negative quiet
        0x7f80_0001, // signalling
        0xff80_0001, // negative signalling
        0x7fff_ffff,
        0x7fbf_ffff,
    ];
    for &nb in &nans {
        for size_i in 1i32..=20 {
            let size = size_i as usize;
            for pos in 0..size {
                let mut base = vec![0.0f32; 2 * size + 8];
                for (i, b) in base.iter_mut().enumerate() {
                    *b = -5.0 - i as f32;
                }
                for i in 0..size {
                    base[i] = 0.5 + i as f32;
                }
                base[pos] = f32::from_bits(nb);
                let dest_off = size + 4;
                let cout = run_one(&c, &base, dest_off, 0, size_i);
                for i in 0..size {
                    assert_eq!(
                        cout[dest_off + i].to_bits(),
                        0u32,
                        "C must zero-fill when sum is NaN (nan=0x{nb:08x} size={size_i} pos={pos})"
                    );
                }
                diff_shared(&c, &r, &base, dest_off, 0, size_i).unwrap();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 18 : NaN in src, dest == src -> untouched, payload preserved
// ---------------------------------------------------------------------------
#[test]
fn err_18_nan_input_inplace_untouched() {
    let (c, r) = load_impls();
    for &nb in &[0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001, 0x7fff_ffff] {
        for size_i in 1i32..=20 {
            let size = size_i as usize;
            for pos in 0..size {
                let mut base = vec![0.0f32; size + 4];
                for (i, b) in base.iter_mut().enumerate() {
                    *b = 1.0 + i as f32;
                }
                base[pos] = f32::from_bits(nb);
                let cout = run_one(&c, &base, 0, 0, size_i);
                assert_eq!(
                    bits_of(&cout),
                    bits_of(&base),
                    "C must leave the buffer untouched (nan=0x{nb:08x} size={size_i} pos={pos})"
                );
                diff_shared(&c, &r, &base, 0, 0, size_i).unwrap();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 19 : sum overflows to +inf -> 1/sqrtf(inf) == 0 -> dest = +/-0.0
// ---------------------------------------------------------------------------
#[test]
fn err_19_sum_overflow_to_inf() {
    let (c, r) = load_impls();
    for &v in &[3.0e38f32, -3.0e38, f32::MAX, -f32::MAX, 1.0e38] {
        for size_i in 1i32..=12 {
            let size = size_i as usize;
            let mut base = vec![0.0f32; 2 * size + 8];
            for (i, b) in base.iter_mut().enumerate() {
                *b = 3.25 + i as f32;
            }
            for i in 0..size {
                base[i] = if i % 2 == 0 { v } else { -v };
            }
            let dest_off = size + 4;
            let cout = run_one(&c, &base, dest_off, 0, size_i);
            let sum: f32 = (0..size).fold(0.0f32, |a, _| a + v * v);
            if sum.is_infinite() {
                for i in 0..size {
                    let got = cout[dest_off + i].to_bits();
                    assert!(
                        got == 0x0000_0000 || got == 0x8000_0000,
                        "C should produce +/-0.0, got 0x{got:08x} (v={v:e} size={size_i})"
                    );
                }
            }
            diff_shared(&c, &r, &base, dest_off, 0, size_i).unwrap();
            diff_shared(&c, &r, &base, 0, 0, size_i).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// row 20 : +/-inf element -> inf * 0.0 == NaN, exact bit pattern must match
// ---------------------------------------------------------------------------
#[test]
fn err_20_inf_element_produces_nan() {
    let (c, r) = load_impls();
    for &inf in &[f32::INFINITY, f32::NEG_INFINITY] {
        for size_i in 1i32..=20 {
            let size = size_i as usize;
            for pos in 0..size {
                let mut base = vec![0.0f32; 2 * size + 8];
                for (i, b) in base.iter_mut().enumerate() {
                    *b = 2.5 + i as f32;
                }
                for i in 0..size {
                    base[i] = 1.0 + i as f32;
                }
                base[pos] = inf;
                let dest_off = size + 4;
                let cout = run_one(&c, &base, dest_off, 0, size_i);
                assert!(
                    cout[dest_off + pos].is_nan(),
                    "C should produce NaN at the inf slot (size={size_i} pos={pos}) got 0x{:08x}",
                    cout[dest_off + pos].to_bits()
                );
                diff_shared(&c, &r, &base, dest_off, 0, size_i).unwrap();
                diff_shared(&c, &r, &base, 0, 0, size_i).unwrap();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 21 : denormal square, sum > 0 -> still normalises
// ---------------------------------------------------------------------------
#[test]
fn err_21_denormal_square_still_normalises() {
    let (c, r) = load_impls();
    for &v in &[1.0e-20f32, -1.0e-20, 1.0e-19, 3.0e-20, 1.0e-21, 1.0e-22] {
        for size_i in 1i32..=8 {
            let size = size_i as usize;
            let mut base = vec![v; 2 * size + 8];
            for i in size..base.len() {
                base[i] = 42.0 + i as f32;
            }
            let dest_off = size + 4;
            let cout = run_one(&c, &base, dest_off, 0, size_i);
            let sum = (0..size).fold(0.0f32, |a, _| a + v * v);
            if sum > 0.0 {
                let k = 1.0f32 / sum.sqrt();
                for i in 0..size {
                    assert_eq!(
                        cout[dest_off + i].to_bits(),
                        (v * k).to_bits(),
                        "C normalisation mismatch (v={v:e} size={size_i})"
                    );
                }
            }
            diff_shared(&c, &r, &base, dest_off, 0, size_i).unwrap();
            diff_shared(&c, &r, &base, 0, 0, size_i).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// row 22 : dest = src - k, sum == 0 -> zero fill overruns into src
// ---------------------------------------------------------------------------
#[test]
fn err_22_zero_fill_overruns_into_src() {
    let (c, r) = load_impls();
    for size_i in 2i32..=24 {
        let size = size_i as usize;
        for k in overlap_ks(size) {
            let mut base = vec![0.0f32; size + k + 8];
            for (i, b) in base.iter_mut().enumerate() {
                *b = 11.0 + i as f32;
            }
            // src region [k, k+size) is all zeros => sum == 0 => memset at dest = src - k
            for i in k..k + size {
                base[i] = 0.0;
            }
            let cout = run_one(&c, &base, 0, k, size_i);
            for i in 0..size {
                assert_eq!(
                    cout[i].to_bits(),
                    0u32,
                    "C should have zeroed [{i}] (size={size_i} k={k})"
                );
            }
            // and elements past the memset range are untouched
            for i in size..base.len() {
                assert_eq!(cout[i].to_bits(), base[i].to_bits(), "overrun past dest+size");
            }
            diff_shared(&c, &r, &base, 0, k, size_i).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// row 23 : the whole `int size` range across the FFI boundary
//          (the API declares no enum/flag parameter, so this is the only
//           non-pointer argument whose out-of-range values must be handled)
// ---------------------------------------------------------------------------
#[test]
fn err_23_int_size_full_range_boundaries() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(23);
    let mut base = vec![0.0f32; 64];
    fill_garbage(&mut rng, &mut base);

    // non-positive sizes are safe in place (both loops skipped, memset skipped)
    for size in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -1_000_000,
        -1024,
        -3,
        -2,
        -1,
        0,
    ] {
        let cout = run_one(&c, &base, 8, 8, size);
        assert_eq!(bits_of(&cout), bits_of(&base), "size={size}");
        diff_shared(&c, &r, &base, 8, 8, size).unwrap_or_else(|e| panic!("size={size}: {e}"));
    }
    // size == 0 with dest != src (memset of length 0)
    diff_shared(&c, &r, &base, 0, 16, 0).unwrap();
    // one step into the valid range
    for size in [1i32, 2, 3] {
        diff_shared(&c, &r, &base, 32, 0, size).unwrap();
        diff_shared(&c, &r, &base, 0, 0, size).unwrap();
    }
    // i32::MAX / large positive sizes dereference out of bounds in C; that
    // fatal-signal behaviour is compared in crash_paths.rs (err_24).
}

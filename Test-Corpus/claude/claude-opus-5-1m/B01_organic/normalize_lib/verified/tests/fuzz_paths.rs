//! Phase B (continued) — high-volume differential fuzzing.
//!
//! `CONFIGS.md` rows 95..98. These deliberately go past the structured rows:
//! they sweep the whole `f32` bit-pattern domain through the `sqrtf` argument,
//! and randomly sample the FULL cross product of (aliasing mode x value class
//! x size x offsets) so that combinations the structured rows only touch once
//! get hit tens of thousands of times.

mod common;

use common::*;
use std::ffi::c_int;

/// Fast path used by the high-volume rows: one allocation, no `Vec` churn in
/// the inner loop, whole-buffer bitwise comparison.
fn diff_raw(
    c: &Impl,
    r: &Impl,
    base: &[f32],
    dest_off: usize,
    src_off: usize,
    size: c_int,
    cbuf: &mut Vec<f32>,
    rbuf: &mut Vec<f32>,
) -> bool {
    cbuf.clear();
    cbuf.extend_from_slice(base);
    rbuf.clear();
    rbuf.extend_from_slice(base);
    unsafe {
        (c.normalize)(
            cbuf.as_mut_ptr().add(dest_off),
            cbuf.as_ptr().add(src_off),
            size,
        );
        (r.normalize)(
            rbuf.as_mut_ptr().add(dest_off),
            rbuf.as_ptr().add(src_off),
            size,
        );
    }
    cbuf.iter()
        .zip(rbuf.iter())
        .all(|(a, b)| a.to_bits() == b.to_bits())
}

// --------------------------------------------------------------------------
// row 95 : systematic stride sweep of the ENTIRE f32 bit-pattern space
// --------------------------------------------------------------------------

#[test]
fn cfg_row_095_full_f32_domain_stride_sweep() {
    let (c, r) = load_impls();
    // A stride that is coprime with 2^32 walks every exponent/mantissa region
    // including subnormals, infinities and every NaN class.
    const STRIDE: u32 = 65_557; // prime
    const STEPS: u32 = 65_536;
    let mut fails = Vec::new();
    let mut cb = Vec::new();
    let mut rb = Vec::new();
    let mut bits: u32 = 0;
    for step in 0..STEPS {
        bits = bits.wrapping_add(STRIDE);
        let x = f32::from_bits(bits);
        // size 1 (sum == x*x, the whole sqrtf domain), disjoint and in place
        let base = [x, -0.5, 7.0, x, 1.0e-3, -3.5];
        for &(d, s) in &[(2usize, 0usize), (0, 0), (1, 0), (0, 3)] {
            if !diff_raw(&c, &r, &base, d, s, 1, &mut cb, &mut rb) {
                fails.push(format!("row 95 size=1 bits=0x{bits:08x} dest={d} src={s}"));
            }
        }
        // size 2/3 so the accumulation has more than one term
        let base3 = [x, 1.0, x, -1.0, x, 0.25, -0.75, 3.0, x];
        for &(d, s, n) in &[
            (0usize, 0usize, 2i32),
            (4, 0, 2),
            (0, 2, 3),
            (2, 0, 3),
            (0, 0, 3),
        ] {
            if !diff_raw(&c, &r, &base3, d, s, n, &mut cb, &mut rb) {
                fails.push(format!(
                    "row 95 size={n} bits=0x{bits:08x} dest={d} src={s}"
                ));
            }
        }
        if fails.len() > 10 {
            break;
        }
        let _ = step;
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 95: {STEPS} bit patterns x 9 layouts swept OK");
}

// --------------------------------------------------------------------------
// row 96 : randomized sampling of the FULL cross product
// --------------------------------------------------------------------------

#[test]
fn cfg_row_096_random_cross_product_fuzz() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(0xF0F0_1234_5678_9ABC);
    let mut fails = Vec::new();
    let mut cb = Vec::new();
    let mut rb = Vec::new();
    const CASES: usize = 120_000;
    let mut vals: Vec<f32> = Vec::new();
    for case in 0..CASES {
        let size = 1 + rng.below(48);
        let class = ALL_CLASSES[rng.below(ALL_CLASSES.len())];
        let mode = rng.below(6);
        let head = rng.below(9);
        let k = 1 + rng.below(size.max(2) - 1);
        let alias = match mode {
            0 => Alias::SameDestFirst,
            1 => Alias::SameSrcFirst,
            2 => Alias::InPlace,
            3 => Alias::OverlapDestAfter(k.min(size.saturating_sub(1)).max(1)),
            4 => Alias::OverlapDestBefore(k.min(size.saturating_sub(1)).max(1)),
            _ => Alias::SameDestFirst,
        };
        let (len, dest_off, src_off) = layout_offsets(alias, size, head);
        vals.clear();
        vals.resize(len, 0.0);
        fill_garbage(&mut rng, &mut vals);
        gen_values(class, &mut rng, &mut vals[src_off..src_off + size]);
        // occasionally poison individual elements with an extreme value
        if rng.below(4) == 0 {
            let idx = src_off + rng.below(size);
            vals[idx] = match rng.below(6) {
                0 => f32::INFINITY,
                1 => f32::NEG_INFINITY,
                2 => f32::from_bits(0x7f80_0001),
                3 => -0.0,
                4 => f32::MAX,
                _ => f32::MIN_POSITIVE,
            };
        }
        if !diff_raw(
            &c,
            &r,
            &vals,
            dest_off,
            src_off,
            size as c_int,
            &mut cb,
            &mut rb,
        ) {
            fails.push(format!(
                "row 96 case {case}: {alias:?} {class:?} size={size} dest={dest_off} src={src_off} vals={:?}",
                &vals[src_off..src_off + size]
            ));
            if fails.len() > 5 {
                break;
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 96: {CASES} random cross-product cases OK");
}

// --------------------------------------------------------------------------
// row 97 : random `size` (including non-positive) with random pointer relation
// --------------------------------------------------------------------------

#[test]
fn cfg_row_097_random_sizes_including_non_positive() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(0x0BAD_C0DE_0000_0097);
    let mut fails = Vec::new();
    let mut cb = Vec::new();
    let mut rb = Vec::new();
    let mut base = vec![0.0f32; 96];
    for case in 0..20_000usize {
        fill_garbage(&mut rng, &mut base);
        // non-positive sizes are only safe when dest == src (see ERRORS.md 3-6)
        let size: c_int = match rng.below(4) {
            0 => 0,
            1 => -(rng.below(1 << 20) as c_int),
            2 => i32::MIN + rng.below(4) as c_int,
            _ => 1 + rng.below(32) as c_int,
        };
        let off = rng.below(32);
        let (dest_off, src_off) = if size > 0 {
            match rng.below(3) {
                0 => (off, off),
                1 => (off, off + 32),
                _ => (off + 32, off),
            }
        } else {
            (off, off) // in place: no memset, no read
        };
        if !diff_raw(
            &c, &r, &base, dest_off, src_off, size, &mut cb, &mut rb,
        ) {
            fails.push(format!(
                "row 97 case {case}: size={size} dest={dest_off} src={src_off}"
            ));
            if fails.len() > 5 {
                break;
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 97: 20000 random size cases OK");
}

// --------------------------------------------------------------------------
// row 98 : long random call SEQUENCES on a shared buffer (composed pipeline)
// --------------------------------------------------------------------------

#[test]
fn cfg_row_098_random_call_sequences() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(0x5E9D_1111_2222_0098);
    let mut fails = Vec::new();
    for seq in 0..2_000usize {
        let len = 8 + rng.below(64);
        let mut base = vec![0.0f32; len];
        fill_garbage(&mut rng, &mut base);
        let class = ALL_CLASSES[rng.below(ALL_CLASSES.len())];
        let n0 = 1 + rng.below(len / 2);
        gen_values(class, &mut rng, &mut base[0..n0]);
        let mut calls: Vec<(usize, usize, c_int)> = Vec::new();
        for _ in 0..(1 + rng.below(8)) {
            let size = 1 + rng.below(len / 2);
            let dest_off = rng.below(len - size + 1);
            let src_off = rng.below(len - size + 1);
            calls.push((dest_off, src_off, size as c_int));
        }
        if let Err(e) = diff_shared_calls(&c, &r, &base, &calls) {
            fails.push(format!("row 98 seq {seq}: {e} calls={calls:?}"));
            if fails.len() > 5 {
                break;
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 98: 2000 random call sequences OK");
}

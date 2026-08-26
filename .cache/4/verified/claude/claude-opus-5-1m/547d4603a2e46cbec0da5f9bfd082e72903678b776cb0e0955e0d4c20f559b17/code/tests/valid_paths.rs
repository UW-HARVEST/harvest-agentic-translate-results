//! Phase B — valid-path differential tests.
//!
//! One test per aliasing mode of `CONFIGS.md` (rows 1..84 = mode x value
//! class), plus the additional shape rows 85..91. Every row sweeps all sizes of
//! axis 2 and runs many randomized trials from a fixed seed; the whole backing
//! allocation is compared bit-for-bit between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::ffi::c_int;

const TRIALS: usize = 8;
const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Runs one `CONFIGS.md` row: `alias` x `class` x all sizes x all overlap `k`
/// x `TRIALS` randomized inputs. Returns the list of failures found.
fn run_row(row: usize, c: &Impl, r: &Impl, alias: Alias, class: VClass) -> Vec<String> {
    let mut fails = Vec::new();
    let mut rng = Rng::new(SEED ^ (row as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut cases = 0usize;

    for &size_i in SIZES.iter() {
        let size = size_i as usize;
        // which overlap distances apply to this (alias, size)?
        let ks: Vec<usize> = match alias {
            Alias::OverlapDestAfter(_) | Alias::OverlapDestBefore(_) => overlap_ks(size),
            _ => vec![0],
        };
        for k in ks {
            let alias = match alias {
                Alias::OverlapDestAfter(_) => Alias::OverlapDestAfter(k),
                Alias::OverlapDestBefore(_) => Alias::OverlapDestBefore(k),
                other => other,
            };
            for trial in 0..TRIALS {
                cases += 1;
                let res = match alias {
                    Alias::Sep => {
                        let tail = 4usize;
                        let mut src = vec![0.0f32; size + tail];
                        let mut dest = vec![0.0f32; size + tail];
                        fill_garbage(&mut rng, &mut src);
                        fill_garbage(&mut rng, &mut dest);
                        gen_values(class, &mut rng, &mut src[..size]);
                        diff_separate(c, r, &dest, &src, size_i)
                    }
                    _ => {
                        let head = 1 + trial % 8; // also varies alignment
                        let (len, dest_off, src_off) = layout_offsets(alias, size, head);
                        let mut base = vec![0.0f32; len];
                        fill_garbage(&mut rng, &mut base);
                        gen_values(class, &mut rng, &mut base[src_off..src_off + size]);
                        diff_shared(c, r, &base, dest_off, src_off, size_i)
                    }
                };
                if let Err(e) = res {
                    fails.push(format!(
                        "row {row} [{alias:?} x {class:?} size={size_i} trial={trial}]: {e}"
                    ));
                    if fails.len() > 20 {
                        return fails;
                    }
                }
            }
        }
    }
    println!("row {row:>2}: {alias:?} x {class:?} -> {cases} cases OK");
    fails
}

fn run_mode(first_row: usize, alias: Alias) {
    let (c, r) = load_impls();
    let mut fails = Vec::new();
    for (i, &class) in ALL_CLASSES.iter().enumerate() {
        fails.extend(run_row(first_row + i, &c, &r, alias, class));
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

// --------------------------------------------------------------------------
// rows 1..84 : aliasing mode x value class
// --------------------------------------------------------------------------

#[test]
fn cfg_rows_001_014_alias_a_disjoint_separate_allocations() {
    run_mode(1, Alias::Sep);
}

#[test]
fn cfg_rows_015_028_alias_b_same_alloc_dest_first() {
    run_mode(15, Alias::SameDestFirst);
}

#[test]
fn cfg_rows_029_042_alias_c_same_alloc_src_first() {
    run_mode(29, Alias::SameSrcFirst);
}

#[test]
fn cfg_rows_043_056_alias_d_in_place() {
    run_mode(43, Alias::InPlace);
}

#[test]
fn cfg_rows_057_070_alias_e_overlap_dest_after_src() {
    run_mode(57, Alias::OverlapDestAfter(1));
}

#[test]
fn cfg_rows_071_084_alias_f_overlap_dest_before_src() {
    run_mode(71, Alias::OverlapDestBefore(1));
}

// --------------------------------------------------------------------------
// row 85 : element offset / alignment sweep
// --------------------------------------------------------------------------

#[test]
fn cfg_row_085_alignment_offset_sweep() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(SEED ^ 85);
    let mut fails = Vec::new();
    for &class in &[VClass::RandomFiniteBits, VClass::RandomAnyBits, VClass::UniformPm1] {
        for &size_i in &[1i32, 2, 3, 4, 7, 8, 15, 16, 17, 32, 33] {
            let size = size_i as usize;
            for dest_off in 0..16usize {
                for src_off in 0..16usize {
                    // keep the two regions disjoint unless offsets are equal
                    let len = dest_off.max(src_off) + size + 20;
                    let mut base = vec![0.0f32; len];
                    fill_garbage(&mut rng, &mut base);
                    gen_values(class, &mut rng, &mut base[src_off..src_off + size]);
                    if let Err(e) = diff_shared(&c, &r, &base, dest_off, src_off, size_i) {
                        fails.push(format!(
                            "row 85 [{class:?} size={size_i} dest_off={dest_off} src_off={src_off}]: {e}"
                        ));
                    }
                }
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 85: alignment sweep OK");
}

// --------------------------------------------------------------------------
// row 86 : repeated / composed invocation on the same buffer
// --------------------------------------------------------------------------

#[test]
fn cfg_row_086_repeated_and_composed_invocations() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(SEED ^ 86);
    let mut fails = Vec::new();
    for &class in ALL_CLASSES.iter() {
        for &size_i in &[1i32, 2, 3, 4, 5, 8, 16, 17, 33, 64, 129] {
            let size = size_i as usize;
            for trial in 0..4 {
                let len = 3 * size + 12;
                let mut base = vec![0.0f32; len];
                fill_garbage(&mut rng, &mut base);
                gen_values(class, &mut rng, &mut base[0..size]);
                // pipeline: in-place, then copy-normalise forward, then again
                let calls: Vec<(usize, usize, c_int)> = vec![
                    (0, 0, size_i),
                    (0, 0, size_i),
                    (size + 4, 0, size_i),
                    (size + 4, size + 4, size_i),
                    (2 * size + 8, size + 4, size_i),
                    (0, 2 * size + 8, size_i),
                ];
                if let Err(e) = diff_shared_calls(&c, &r, &base, &calls) {
                    fails.push(format!("row 86 [{class:?} size={size_i} trial={trial}]: {e}"));
                }
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 86: repeated/composed invocation OK");
}

// --------------------------------------------------------------------------
// row 87 : exactly representable sums / exact reciprocal square roots
// --------------------------------------------------------------------------

#[test]
fn cfg_row_087_exact_sqrt_inputs() {
    let (c, r) = load_impls();
    let cases: Vec<Vec<f32>> = vec![
        vec![1.0],
        vec![-1.0],
        vec![3.0, 4.0],
        vec![4.0, 3.0],
        vec![-3.0, -4.0],
        vec![1.0, 1.0, 1.0, 1.0],
        vec![2.0, 0.0, 0.0, 0.0],
        vec![0.5, 0.5, 0.5, 0.5],
        vec![1.0, 2.0, 2.0],
        vec![6.0, 8.0],
        vec![0.0, 0.0, 5.0],
        vec![1.0; 16],
        vec![1.0; 64],
        vec![0.25; 256],
        vec![16777216.0, 0.0],
        vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
    ];
    let mut fails = Vec::new();
    for (i, vals) in cases.iter().enumerate() {
        let size = vals.len() as c_int;
        // disjoint (separate allocations), disjoint (same alloc), in place
        let mut dest = vec![7.5f32; vals.len() + 4];
        for (j, d) in dest.iter_mut().enumerate() {
            *d = 1.0 + j as f32;
        }
        let mut src = vals.clone();
        src.extend_from_slice(&[9.0, 9.5, -9.0, -9.5]);
        if let Err(e) = diff_separate(&c, &r, &dest, &src, size) {
            fails.push(format!("row 87 case {i} sep: {e}"));
        }
        let mut base = vec![0.0f32; 2 * vals.len() + 8];
        for (j, b) in base.iter_mut().enumerate() {
            *b = -3.5 - j as f32;
        }
        base[..vals.len()].copy_from_slice(vals);
        if let Err(e) = diff_shared(&c, &r, &base, vals.len() + 2, 0, size) {
            fails.push(format!("row 87 case {i} same-alloc: {e}"));
        }
        if let Err(e) = diff_shared(&c, &r, &base, 0, 0, size) {
            fails.push(format!("row 87 case {i} in-place: {e}"));
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 87: exact-sqrt inputs OK");
}

// --------------------------------------------------------------------------
// row 88 : overflow edge of the accumulation
// --------------------------------------------------------------------------

#[test]
fn cfg_row_088_sum_overflow_edge() {
    let (c, r) = load_impls();
    let big = 1.8446743e19f32; // big*big is just under f32::MAX
    let mut fails = Vec::new();
    let cases: Vec<Vec<f32>> = vec![
        vec![big],
        vec![big, 0.0],
        vec![big, 1.0],
        vec![big, big],                       // overflows on the 2nd add
        vec![big, -big],                      // still overflows (squares)
        vec![1.0, big],                       // overflows on the last add
        vec![f32::MAX],
        vec![f32::MAX, f32::MAX],
        vec![f32::MAX, 1.0, -1.0],
        vec![1.0e19, 1.0e19, 1.0e19],
        vec![big; 8],
        vec![3.4028235e38, 1.0e-30],
    ];
    for (i, vals) in cases.iter().enumerate() {
        let size = vals.len() as c_int;
        let mut src = vals.clone();
        src.extend_from_slice(&[1.5, -2.5, 3.5, -4.5]);
        let dest = vec![-1.0f32; src.len()];
        if let Err(e) = diff_separate(&c, &r, &dest, &src, size) {
            fails.push(format!("row 88 case {i} sep: {e}"));
        }
        let mut base = vec![8.0f32; 2 * vals.len() + 12];
        for (j, b) in base.iter_mut().enumerate() {
            *b = 8.0 + j as f32;
        }
        base[..vals.len()].copy_from_slice(vals);
        if let Err(e) = diff_shared(&c, &r, &base, 0, 0, size) {
            fails.push(format!("row 88 case {i} in-place: {e}"));
        }
        if let Err(e) = diff_shared(&c, &r, &base, vals.len() + 4, 0, size) {
            fails.push(format!("row 88 case {i} same-alloc: {e}"));
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 88: sum overflow edge OK");
}

// --------------------------------------------------------------------------
// row 89 : underflow edge (denormal squares, huge 1/sqrt)
// --------------------------------------------------------------------------

#[test]
fn cfg_row_089_sum_underflow_edge() {
    let (c, r) = load_impls();
    let mut fails = Vec::new();
    let cases: Vec<Vec<f32>> = vec![
        vec![1.0e-20],
        vec![1.0e-20, 1.0e-20],
        vec![1.0e-22],
        vec![1.0e-23],
        vec![1.0e-24],
        vec![f32::MIN_POSITIVE],
        vec![f32::MIN_POSITIVE, f32::MIN_POSITIVE],
        vec![1.0e-45],
        vec![1.0e-45, -1.0e-45],
        vec![1.1754942e-38],
        vec![1.0e-20, 1.0e-30],
        vec![1.0e-19; 16],
        vec![1.0e-21; 64],
    ];
    for (i, vals) in cases.iter().enumerate() {
        let size = vals.len() as c_int;
        let mut src = vals.clone();
        src.extend_from_slice(&[0.25, -0.5, 2.0, -4.0]);
        let dest = vec![13.0f32; src.len()];
        if let Err(e) = diff_separate(&c, &r, &dest, &src, size) {
            fails.push(format!("row 89 case {i} sep: {e}"));
        }
        let mut base = vec![-7.0f32; 2 * vals.len() + 12];
        for (j, b) in base.iter_mut().enumerate() {
            *b = -7.0 - j as f32;
        }
        base[..vals.len()].copy_from_slice(vals);
        if let Err(e) = diff_shared(&c, &r, &base, 0, 0, size) {
            fails.push(format!("row 89 case {i} in-place: {e}"));
        }
        if let Err(e) = diff_shared(&c, &r, &base, vals.len() + 4, 0, size) {
            fails.push(format!("row 89 case {i} same-alloc: {e}"));
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 89: sum underflow edge OK");
}

// --------------------------------------------------------------------------
// row 90 : accumulation-order sensitivity (permutations)
// --------------------------------------------------------------------------

#[test]
fn cfg_row_090_accumulation_order_permutations() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(SEED ^ 90);
    let mut fails = Vec::new();
    for &size_i in &[2i32, 3, 5, 8, 17, 33, 64] {
        let size = size_i as usize;
        for &class in &[
            VClass::MixedMagnitudes,
            VClass::UniformPm1,
            VClass::Boundary,
            VClass::Tiny,
        ] {
            let mut vals = vec![0.0f32; size];
            gen_values(class, &mut rng, &mut vals);
            for perm in 0..12 {
                // Fisher-Yates shuffle of the same multiset
                let mut v = vals.clone();
                for i in (1..size).rev() {
                    let j = rng.below(i + 1);
                    v.swap(i, j);
                }
                let mut base = v.clone();
                base.extend_from_slice(&[0.125; 6]);
                let dest = vec![-0.75f32; base.len()];
                if let Err(e) = diff_separate(&c, &r, &dest, &base, size_i) {
                    fails.push(format!("row 90 [{class:?} size={size_i} perm={perm}] sep: {e}"));
                }
                if let Err(e) = diff_shared(&c, &r, &base, 0, 0, size_i) {
                    fails.push(format!(
                        "row 90 [{class:?} size={size_i} perm={perm}] in-place: {e}"
                    ));
                }
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 90: permutation sweep OK");
}

// --------------------------------------------------------------------------
// row 91 : long buffers (compounded rounding)
// --------------------------------------------------------------------------

#[test]
fn cfg_row_091_large_sizes() {
    let (c, r) = load_impls();
    let mut rng = Rng::new(SEED ^ 91);
    let mut fails = Vec::new();
    for &size_i in &[1000i32, 1023, 1024, 1025, 4096, 10_000] {
        let size = size_i as usize;
        for &class in &[
            VClass::UniformPm1,
            VClass::MixedMagnitudes,
            VClass::RandomFiniteBits,
            VClass::RandomAnyBits,
            VClass::Tiny,
            VClass::Huge,
        ] {
            for trial in 0..3 {
                let mut src = vec![0.0f32; size + 8];
                fill_garbage(&mut rng, &mut src);
                gen_values(class, &mut rng, &mut src[..size]);
                let mut dest = vec![0.0f32; size + 8];
                fill_garbage(&mut rng, &mut dest);
                if let Err(e) = diff_separate(&c, &r, &dest, &src, size_i) {
                    fails.push(format!("row 91 [{class:?} size={size_i} t={trial}] sep: {e}"));
                }
                if let Err(e) = diff_shared(&c, &r, &src, 0, 0, size_i) {
                    fails.push(format!(
                        "row 91 [{class:?} size={size_i} t={trial}] in-place: {e}"
                    ));
                }
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
    println!("row 91: large sizes OK");
}

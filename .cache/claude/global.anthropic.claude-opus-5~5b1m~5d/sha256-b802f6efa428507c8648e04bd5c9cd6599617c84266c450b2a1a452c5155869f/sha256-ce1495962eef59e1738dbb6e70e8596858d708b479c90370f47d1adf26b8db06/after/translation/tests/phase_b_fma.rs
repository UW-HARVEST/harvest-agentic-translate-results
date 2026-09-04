//! Phase B -- valid-path differential tests for the LOWEST-LEVEL entry point,
//! `fma_array`, driven directly (not through the `driver` wrapper).
//!
//! Covers rows C1..C27 of CONFIGS.md. Every row runs the full `LENS` list from
//! CONFIGS.md Axis C and, per length, `DRAWS` independent seeded random draws.

mod common;
use common::*;
use std::ffi::c_int;

/// Generic row driver: aliasing configuration `a` x every len x every draw.
fn row(row_id: &str, a: Alias, shape: Shape) {
    let p = pair();
    let (nbufs, _) = alias_layout(a);
    // Seed derived from the row id so each row is independent yet reproducible.
    let mut rng = Rng::new(fnv(row_id));

    for &len in LENS {
        for draw in 0..DRAWS {
            let bufs: Vec<Vec<c_int>> = (0..nbufs).map(|_| gen_vals(shape, len, &mut rng)).collect();
            let ctx = format!("{row_id} shape={shape:?} len={len} draw={draw}");
            diff_fma_alias(p, a, &bufs, len as c_int, &ctx);
        }
    }
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// --- C1..C10: A0 (all four buffers distinct) x every value shape ------------

#[test]
fn c1_distinct_zeros() {
    row("C1", Alias::Distinct, Shape::Zeros);
}

#[test]
fn c2_distinct_ones() {
    row("C2", Alias::Distinct, Shape::Ones);
}

#[test]
fn c3_distinct_small_pos() {
    row("C3", Alias::Distinct, Shape::SmallPos);
}

#[test]
fn c4_distinct_small_neg() {
    row("C4", Alias::Distinct, Shape::SmallNeg);
}

#[test]
fn c5_distinct_mixed_small() {
    row("C5", Alias::Distinct, Shape::MixedSmall);
}

#[test]
fn c6_distinct_safe_magnitudes() {
    row("C6", Alias::Distinct, Shape::SafeMag);
}

#[test]
fn c7_distinct_overflow_boundary() {
    row("C7", Alias::Distinct, Shape::Boundary);
}

#[test]
fn c8_distinct_extremes() {
    row("C8", Alias::Distinct, Shape::Extremes);
}

#[test]
fn c9_distinct_full_range_random() {
    row("C9", Alias::Distinct, Shape::FullRandom);
}

#[test]
fn c10_distinct_extreme_pool() {
    row("C10", Alias::Distinct, Shape::ExtremePool);
}

// --- C11..C24: the aliasing configurations ---------------------------------

#[test]
fn c11_out_is_mul1_full_random() {
    row("C11", Alias::OutIsMul1, Shape::FullRandom);
}

#[test]
fn c12_out_is_mul1_extremes() {
    row("C12", Alias::OutIsMul1, Shape::Extremes);
}

#[test]
fn c13_out_is_mul2_full_random() {
    row("C13", Alias::OutIsMul2, Shape::FullRandom);
}

#[test]
fn c14_out_is_mul2_extreme_pool() {
    row("C14", Alias::OutIsMul2, Shape::ExtremePool);
}

#[test]
fn c15_out_is_add_full_random() {
    row("C15", Alias::OutIsAdd, Shape::FullRandom);
}

#[test]
fn c16_out_is_add_safe_magnitudes() {
    row("C16", Alias::OutIsAdd, Shape::SafeMag);
}

#[test]
fn c17_all_same_full_random() {
    row("C17", Alias::AllSame, Shape::FullRandom);
}

#[test]
fn c18_all_same_extremes() {
    row("C18", Alias::AllSame, Shape::Extremes);
}

#[test]
fn c19_all_same_small_pos() {
    row("C19", Alias::AllSame, Shape::SmallPos);
}

#[test]
fn c20_mul1_is_mul2_full_random() {
    row("C20", Alias::Mul1IsMul2, Shape::FullRandom);
}

#[test]
fn c21_mul1_is_mul2_boundary() {
    row("C21", Alias::Mul1IsMul2, Shape::Boundary);
}

#[test]
fn c22_inputs_all_same_full_random() {
    row("C22", Alias::InputsAllSame, Shape::FullRandom);
}

#[test]
fn c23_out_mul1_mul2_full_random() {
    row("C23", Alias::OutMul1Mul2, Shape::FullRandom);
}

#[test]
fn c24_two_pairs_full_random() {
    row("C24", Alias::TwoPairs, Shape::FullRandom);
}

/// Exhaustive sweep: EVERY aliasing configuration x EVERY value shape. This is
/// the full cross-product of Axes B and D, over a reduced but still
/// representative len list, so no combination is left unexercised.
#[test]
fn c11_24_full_alias_x_shape_cross_product() {
    let p = pair();
    let mut rng = Rng::new(fnv("cross"));
    let lens: &[usize] = &[0, 1, 2, 3, 4, 7, 8, 16, 17, 33, 64, 65];
    for &a in ALL_ALIASES {
        let (nbufs, _) = alias_layout(a);
        for &shape in ALL_SHAPES {
            for &len in lens {
                for draw in 0..3 {
                    let bufs: Vec<Vec<c_int>> =
                        (0..nbufs).map(|_| gen_vals(shape, len, &mut rng)).collect();
                    let ctx = format!("cross a={a:?} shape={shape:?} len={len} draw={draw}");
                    diff_fma_alias(p, a, &bufs, len as c_int, &ctx);
                }
            }
        }
    }
}

// --- C25 / C26: partially overlapping ranges -------------------------------
//
// `out` and the inputs point into the SAME buffer at different offsets. Line 31
// of driver.c reads mul1[i]/mul2[i]/add[i] then writes out[i] in a forward
// element-wise walk, so earlier writes are visible to later reads. Both
// implementations must produce the identical (order-dependent) result.

fn overlap_row(row_id: &str, forward: bool) {
    let p = pair();
    let mut rng = Rng::new(fnv(row_id));

    for &len in LENS {
        for &k in &[1usize, 2, 3, 8] {
            for draw in 0..DRAWS {
                let total = len + k;
                let base = gen_vals(Shape::FullRandom, total, &mut rng);

                let run = |imp: &Impl| -> Vec<c_int> {
                    let mut b = base.clone();
                    let ptr = b.as_mut_ptr();
                    unsafe {
                        let (out, inp) = if forward {
                            // out at the start, inputs shifted forward by k
                            (ptr, ptr.add(k) as *const c_int)
                        } else {
                            // out shifted forward by k, inputs at the start
                            (ptr.add(k), ptr as *const c_int)
                        };
                        (imp.fma_array)(out, inp, inp, inp, len as c_int);
                    }
                    b
                };

                let got_c = run(&p.c);
                let got_rs = run(&p.rs);
                assert_eq!(
                    got_c,
                    got_rs,
                    "DIVERGENCE {row_id} (forward={forward}) len={len} k={k} draw={draw}\n  \
                     base = {}\n  C   = {}\n  Rust= {}",
                    trunc(&base),
                    trunc(&got_c),
                    trunc(&got_rs)
                );
            }
        }
    }
}

#[test]
fn c25_partial_forward_overlap() {
    overlap_row("C25", true);
}

#[test]
fn c26_partial_reverse_overlap() {
    overlap_row("C26", false);
}

// --- C27: len smaller than the allocated buffer ----------------------------

/// `len` is strictly less than the buffer capacity: assert both
/// implementations write exactly `len` elements and leave the tail untouched
/// (i.e. neither over-runs nor under-runs the loop bound).
#[test]
fn c27_len_shorter_than_buffer_tail_untouched() {
    let p = pair();
    let mut rng = Rng::new(fnv("C27"));
    const SENTINEL: c_int = 0x5A5A_5A5A;

    for &cap in &[1usize, 2, 4, 8, 17, 64, 129, 1000] {
        for len in 0..=cap {
            for draw in 0..3 {
                let m1 = gen_vals(Shape::FullRandom, cap, &mut rng);
                let m2 = gen_vals(Shape::FullRandom, cap, &mut rng);
                let ad = gen_vals(Shape::FullRandom, cap, &mut rng);

                let run = |imp: &Impl| -> Vec<c_int> {
                    let mut out = vec![SENTINEL; cap];
                    unsafe {
                        (imp.fma_array)(
                            out.as_mut_ptr(),
                            m1.as_ptr(),
                            m2.as_ptr(),
                            ad.as_ptr(),
                            len as c_int,
                        );
                    }
                    out
                };

                let oc = run(&p.c);
                let or = run(&p.rs);
                assert_eq!(oc, or, "DIVERGENCE C27 cap={cap} len={len} draw={draw}");
                // Tail past `len` must still hold the sentinel in both.
                assert!(
                    oc[len..].iter().all(|&x| x == SENTINEL),
                    "C   wrote past len: cap={cap} len={len} out={}",
                    trunc(&oc)
                );
                assert!(
                    or[len..].iter().all(|&x| x == SENTINEL),
                    "Rust wrote past len: cap={cap} len={len} out={}",
                    trunc(&or)
                );
            }
        }
    }
}

// --- C44: repeated invocations on the same buffer --------------------------

/// Iterate `out = out*out + out` 50 times on the same buffer. Each round feeds
/// the previous (heavily wrapped) values back in, which amplifies any
/// value-dependent divergence in the multiply/add.
#[test]
fn c44_repeated_iteration_amplifier() {
    let p = pair();
    let mut rng = Rng::new(fnv("C44"));

    for &len in &[1usize, 2, 3, 7, 8, 16, 17, 64, 100] {
        for draw in 0..DRAWS {
            let start = gen_vals(Shape::FullRandom, len, &mut rng);

            let run = |imp: &Impl| -> Vec<c_int> {
                let mut b = start.clone();
                for _ in 0..50 {
                    let ptr = b.as_mut_ptr();
                    unsafe {
                        (imp.fma_array)(ptr, ptr, ptr, ptr, len as c_int);
                    }
                }
                b
            };

            let bc = run(&p.c);
            let br = run(&p.rs);
            assert_eq!(
                bc,
                br,
                "DIVERGENCE C44 len={len} draw={draw}\n  start={}\n  C={}\n  Rust={}",
                trunc(&start),
                trunc(&bc),
                trunc(&br)
            );
        }
    }
}

// --- extra: exhaustive small-value sweep -----------------------------------

/// Exhaustive over a small but complete value grid (every combination of a
/// 13-value set for the three inputs, len = 1), rather than random sampling.
/// This pins down the exact wrapping semantics of `a*b + c` at every sign and
/// overflow combination.
#[test]
fn exhaustive_small_grid_len1() {
    let p = pair();
    let vals: &[c_int] = &[
        i32::MIN,
        i32::MIN + 1,
        -65536,
        -46341,
        -46340,
        -2,
        -1,
        0,
        1,
        2,
        46340,
        46341,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in vals {
        for &b in vals {
            for &c in vals {
                let m1 = [a];
                let m2 = [b];
                let ad = [c];
                let mut oc = [0i32];
                let mut or = [0i32];
                unsafe {
                    (p.c.fma_array)(oc.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 1);
                    (p.rs.fma_array)(or.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 1);
                }
                assert_eq!(oc[0], or[0], "DIVERGENCE exhaustive: {a} * {b} + {c}");
                // Cross-check against the two's-complement wrapping model.
                assert_eq!(
                    oc[0],
                    a.wrapping_mul(b).wrapping_add(c),
                    "C is not wrapping two's complement for {a} * {b} + {c}"
                );
            }
        }
    }
}

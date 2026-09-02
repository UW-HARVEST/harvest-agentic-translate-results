//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row is driven with many randomized
//! inputs (fixed seed) through BOTH `.so` exports and compared byte-for-byte,
//! including the guard region past `numElem` and the source allocation.

mod common;

use common::*;

/// Iterations per randomized row.
const ITERS: usize = 400;

// --------------------------------------------------------------------------
// Rows 1-4: degenerate tiny buffers (numElem 1 and 2)
// --------------------------------------------------------------------------

#[test]
fn cfg_01_numelem1_empty_dst_empty_src() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        // Physical allocation larger than numElem so trailing garbage is
        // compared too.
        let phys = rng.range(1, 8);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let case = Case::new(dst, 1, Src::Own(vec![0]));
        assert_same_ret(&case, 0, "cfg_01");
    }
}

#[test]
fn cfg_02_numelem2_empty_dst_empty_src() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let phys = rng_phys(&mut rng, 2);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let case = Case::new(dst, 2, Src::Own(vec![0]));
        assert_same_ret(&case, 0, "cfg_02");
    }
}

#[test]
fn cfg_03_numelem2_empty_dst_len1_src_exact_fit() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let phys = rng_phys(&mut rng, 2);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let src = gen_src(&mut rng, 1, class);
        let case = Case::new(dst, 2, Src::Own(src));
        assert_same_ret(&case, 0, "cfg_03");
    }
}

#[test]
fn cfg_04_numelem2_nul_at_last_elem_empty_src() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let phys = rng_phys(&mut rng, 2);
        let dst = gen_dst(&mut rng, phys, 1, class);
        let case = Case::new(dst, 2, Src::Own(vec![0]));
        assert_same_ret(&case, 0, "cfg_04");
    }
}

// --------------------------------------------------------------------------
// Rows 5-9: successful appends across the fit spectrum
// --------------------------------------------------------------------------

#[test]
fn cfg_05_empty_dst_room_to_spare() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(3, 64);
        let l = rng.range(0, n - 2); // l + 1 < n
        let phys = rng_phys(&mut rng, n);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 0, "cfg_05");
    }
}

#[test]
fn cfg_06_empty_dst_exact_fit() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 64);
        let l = n - 1; // l + 1 == n
        let phys = rng_phys(&mut rng, n);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 0, "cfg_06");
    }
}

#[test]
fn cfg_07_dst_nul_at_k_room_to_spare() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(4, 64);
        let k = rng.range(1, n - 2);
        let l = rng.range(0, n - k - 2); // k + l + 1 < n
        let phys = rng_phys(&mut rng, n);
        let dst = gen_dst(&mut rng, phys, k, class);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 0, "cfg_07");
    }
}

#[test]
fn cfg_08_dst_nul_at_k_exact_fit() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(2, 64);
        let k = rng.range(1, n - 1);
        let l = n - k - 1; // k + l + 1 == n
        let phys = rng_phys(&mut rng, n);
        let dst = gen_dst(&mut rng, phys, k, class);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 0, "cfg_08");
    }
}

#[test]
fn cfg_09_dst_nul_at_last_elem_empty_src() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 64);
        let phys = rng_phys(&mut rng, n);
        let dst = gen_dst(&mut rng, phys, n - 1, class);
        let case = Case::new(dst, n, Src::Own(vec![0]));
        assert_same_ret(&case, 0, "cfg_09");
    }
}

// --------------------------------------------------------------------------
// Rows 10-13: the buffer runs out (return 34 with a partial copy retained)
// --------------------------------------------------------------------------

#[test]
fn cfg_10_dst_nul_at_last_elem_nonempty_src() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 64);
        let l = rng.range(1, 16);
        let phys = rng_phys(&mut rng, n);
        let dst = gen_dst(&mut rng, phys, n - 1, class);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 34, "cfg_10");
    }
}

#[test]
fn cfg_11_empty_dst_one_element_short() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 64);
        let l = n; // l + 1 == n + 1: chars fit exactly, NUL does not
        let phys = rng_phys(&mut rng, n);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 34, "cfg_11");
    }
}

#[test]
fn cfg_12_dst_nul_at_k_one_element_short() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(2, 64);
        let k = rng.range(1, n - 1);
        let l = n - k; // k + l + 1 == n + 1
        let phys = rng_phys(&mut rng, n);
        let dst = gen_dst(&mut rng, phys, k, class);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 34, "cfg_12");
    }
}

#[test]
fn cfg_13_src_grossly_oversized() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 32);
        let k = rng.range(0, n - 1);
        let l = n * 4;
        let phys = rng_phys(&mut rng, n);
        let dst = gen_dst(&mut rng, phys, k, class);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 34, "cfg_13");
    }
}

// --------------------------------------------------------------------------
// Rows 14-15: unterminated destination (seek loop exhausts, src never read)
// --------------------------------------------------------------------------

#[test]
fn cfg_14_unterminated_dst() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 64);
        let phys = rng_phys(&mut rng, n);
        // nul_at == phys => no NUL anywhere in the allocation.
        let dst = gen_dst(&mut rng, phys, phys, class);
        let l = rng.range(0, 8);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src));
        assert_same_ret(&case, 34, "cfg_14");
    }
}

#[test]
fn cfg_15_dst_nul_only_beyond_numelem() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 32);
        let phys = n + rng.range(1, 16);
        let nul_at = rng.range(n, phys - 1); // NUL exists, but at index >= numElem
        let dst = gen_dst(&mut rng, phys, nul_at, class);
        let sl = rng.range(0, 8);
        let src = gen_src(&mut rng, sl, class);
        let case = Case::new(dst, n, Src::Own(src));
        // Must return 34 and must not disturb anything at or past numElem
        // (identity between C and Rust is what assert_same proves).
        assert_same_ret(&case, 34, "cfg_15");
        let out = both(&case);
        assert_eq!(out.dst[0], 0, "cfg_15: dst[0] must be zeroed");
        assert_eq!(
            &out.dst[n..],
            &case.dst_data[n..]
                .iter()
                .copied()
                .chain(std::iter::repeat(GUARD_VAL).take(GUARD))
                .collect::<Vec<_>>()[..],
            "cfg_15: bytes at/after numElem were modified"
        );
    }
}

// --------------------------------------------------------------------------
// Row 16: oversized numElem with an early-terminated dst
// --------------------------------------------------------------------------

#[test]
fn cfg_16_oversized_numelem_early_terminated_dst() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        // Real allocation is comfortably large; numElem lies about it.
        let phys = 96;
        let k = rng.range(0, 8);
        let dst = gen_dst(&mut rng, phys, k, class);
        let l = rng.range(0, 8);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, 1 << 20, Src::Own(src));
        assert_same_ret(&case, 0, "cfg_16");
    }
}

// --------------------------------------------------------------------------
// Rows 17-22: one row per wchar_t value class
// --------------------------------------------------------------------------

fn value_class_row(class: ValClass, seed_salt: u64, label: &str) {
    let mut rng = Rng::new(SEED ^ seed_salt);
    for _ in 0..ITERS {
        let n = rng.range(1, 48);
        let phys = rng_phys(&mut rng, n);
        // Mix terminated and unterminated destinations.
        let nul_at = if rng.range(0, 5) == 0 {
            phys
        } else {
            rng.range(0, n - 1)
        };
        let dst = gen_dst(&mut rng, phys, nul_at, class);
        let l = rng.range(0, n + 2);
        let src = gen_src(&mut rng, l, class);
        assert_same(&Case::new(dst, n, Src::Own(src)), label);
    }
}

#[test]
fn cfg_17_class_ascii() {
    value_class_row(ValClass::Ascii, 17, "cfg_17");
}

#[test]
fn cfg_18_class_non_bmp() {
    value_class_row(ValClass::NonBmp, 18, "cfg_18");
}

#[test]
fn cfg_19_class_surrogates() {
    value_class_row(ValClass::Surrogate, 19, "cfg_19");
}

#[test]
fn cfg_20_class_above_unicode_max() {
    value_class_row(ValClass::AboveUnicodeMax, 20, "cfg_20");
}

#[test]
fn cfg_21_class_negative() {
    value_class_row(ValClass::Negative, 21, "cfg_21");
}

#[test]
fn cfg_22_class_mixed_random() {
    value_class_row(ValClass::MixedRandom, 22, "cfg_22");
}

// --------------------------------------------------------------------------
// Rows 23-24: aliasing between src and dst (the C has no overlap check)
// --------------------------------------------------------------------------

#[test]
fn cfg_23_src_aliases_dst_empty() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 32);
        let phys = rng_phys(&mut rng, n);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let case = Case::new(dst, n, Src::IntoDst(0));
        assert_same_ret(&case, 0, "cfg_23");
    }
}

#[test]
fn cfg_24_src_overlaps_dst_interior() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(2, 40);
        let phys = rng_phys(&mut rng, n);
        let k = rng.range(1, n - 1);
        let mut dst = gen_dst(&mut rng, phys, k, class);
        // `off` must stay inside `numElem` and a NUL must exist at or after
        // `off` within `numElem`; otherwise the C's bounded copy loop reads
        // past the end of the *real* allocation (genuinely undefined, and the
        // C then observes unrelated heap bytes). See CONFIGS.md row 24.
        let off = rng.range(0, n - 1);
        let t = rng.range(off, n - 1);
        dst[t] = 0;
        assert_same(
            &Case::new(dst, n, Src::IntoDst(off)),
            &format!("cfg_24 n={n} phys={phys} k={k} off={off} t={t}"),
        );
    }
}

// --------------------------------------------------------------------------
// Row 25: repeated appends into one buffer (the real consumer pipeline)
// --------------------------------------------------------------------------

#[test]
fn cfg_25_repeated_appends_accumulate_identically() {
    let mut rng = Rng::new(SEED ^ 25);
    let i = impls();
    for _ in 0..(ITERS / 2) {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 40);
        let phys = n + GUARD;
        let mut c_buf: Vec<i32> = vec![GUARD_VAL; phys];
        let mut r_buf: Vec<i32> = c_buf.clone();
        // Start from an empty string in both.
        c_buf[0] = 0;
        r_buf[0] = 0;

        let rounds = rng.range(2, 8);
        for round in 0..rounds {
            let l = rng.range(0, 6);
            let src = gen_src(&mut rng, l, class);
            let rc = unsafe { (i.c)(c_buf.as_mut_ptr(), n, src.as_ptr()) };
            let rr = unsafe { (i.rust)(r_buf.as_mut_ptr(), n, src.as_ptr()) };
            assert_eq!(
                rc, rr,
                "cfg_25 round {round}: ret C={rc} RUST={rr} (n={n}, src={src:?})"
            );
            assert_eq!(
                c_buf, r_buf,
                "cfg_25 round {round}: buffer diverged (n={n}, src={src:?})"
            );
        }
    }
}

// --------------------------------------------------------------------------
// Row 26: exhaustive small sweep
// --------------------------------------------------------------------------

#[test]
fn cfg_26_exhaustive_small_sweep() {
    let mut rng = Rng::new(SEED ^ 26);
    for n in 1usize..=6 {
        for phys in [n, n + 1, n + 3] {
            // nul_at in 0..=phys ; nul_at == phys means "no NUL at all"
            for nul_at in 0..=phys {
                for l in 0usize..=8 {
                    for class in ALL_CLASSES {
                        let dst = gen_dst(&mut rng, phys, nul_at, class);
                        let src = gen_src(&mut rng, l, class);
                        assert_same(
                            &Case::new(dst, n, Src::Own(src)),
                            &format!("cfg_26 n={n} phys={phys} nul_at={nul_at} l={l} {class:?}"),
                        );
                    }
                }
            }
        }
    }
}

// --------------------------------------------------------------------------
// Row 27: very large numElem with an early-terminated dst
// --------------------------------------------------------------------------

#[test]
fn cfg_27_huge_numelem_early_terminated() {
    let mut rng = Rng::new(SEED ^ 27);
    for num_elem in [1usize << 20, 1 << 24, 1 << 30, 1 << 34] {
        for _ in 0..32 {
            let class = *rng.pick(&ALL_CLASSES);
            let phys = 128;
            let k = rng.range(0, 4);
            let dst = gen_dst(&mut rng, phys, k, class);
            let sl = rng.range(0, 4);
            let src = gen_src(&mut rng, sl, class);
            assert_same_ret(
                &Case::new(dst, num_elem, Src::Own(src)),
                0,
                &format!("cfg_27 numElem={num_elem}"),
            );
        }
    }
}

// --------------------------------------------------------------------------
// Row 28: broad randomized fuzz
// --------------------------------------------------------------------------

#[test]
fn cfg_28_randomized_fuzz() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..20_000 {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 80);
        let phys = if rng.range(0, 2) == 0 { n } else { n + rng.range(1, 12) };
        let nul_at = rng.range(0, phys); // == phys never happens here; see below
        let nul_at = if rng.range(0, 6) == 0 { phys } else { nul_at };
        let dst = gen_dst(&mut rng, phys, nul_at, class);
        let l = rng.range(0, n + 4);
        let src_kind = rng.range(0, 9);
        let (dst, src) = if src_kind == 0 {
            // Overlapping src, constrained to stay inside the real allocation
            // (see cfg_24 / CONFIGS.md row 24).
            let mut dst = dst;
            let off = rng.range(0, n - 1);
            let t = rng.range(off, n - 1);
            dst[t] = 0;
            (dst, Src::IntoDst(off))
        } else {
            (dst, Src::Own(gen_src(&mut rng, l, class)))
        };
        assert_same(&Case::new(dst, n, src), "cfg_28");
    }
}

// --------------------------------------------------------------------------

/// Physical allocation size: sometimes exactly `n`, sometimes larger, so that
/// writes past `numElem` are observable.
fn rng_phys(rng: &mut Rng, n: usize) -> usize {
    match rng.range(0, 2) {
        0 => n,
        1 => n + 1,
        _ => n + rng.range(2, 8),
    }
}

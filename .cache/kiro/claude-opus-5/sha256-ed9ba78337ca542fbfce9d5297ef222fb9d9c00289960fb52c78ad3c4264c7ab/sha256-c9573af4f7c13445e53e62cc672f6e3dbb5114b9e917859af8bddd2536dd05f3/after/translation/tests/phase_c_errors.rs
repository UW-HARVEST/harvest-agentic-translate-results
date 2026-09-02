//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input and
//! asserts BOTH `.so`s return the SAME sentinel (22 / 34 / 0), not merely that
//! "both failed", and that the side effects on `dst` are identical.

mod common;

use common::*;

const ITERS: usize = 200;

// --------------------------------------------------------------------------
// Rows 1-3: dst == NULL  (the `!dst` short-circuit fires first)
// --------------------------------------------------------------------------

#[test]
fn err_01_dst_null_valid_src() {
    let mut rng = Rng::new(SEED ^ 101);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 1024);
        let l = rng.range(0, 8);
        let src = gen_src(&mut rng, l, class);
        let case = Case::null_dst(n, Src::Own(src.clone()));
        assert_same_ret(&case, 22, "err_01");
        // src must be untouched.
        let out = both(&case);
        assert_eq!(out.src, src, "err_01: src was modified");
    }
}

#[test]
fn err_02_dst_null_numelem_zero() {
    let mut rng = Rng::new(SEED ^ 102);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let sl = rng.range(0, 8);
        let src = gen_src(&mut rng, sl, class);
        assert_same_ret(&Case::null_dst(0, Src::Own(src)), 22, "err_02");
    }
}

#[test]
fn err_03_dst_null_src_null() {
    // Must NOT dereference dst: the `!dst` test precedes the `!src` branch
    // that performs `dst[0] = 0`.
    for n in [0usize, 1, 2, 7, 1024, usize::MAX] {
        assert_same_ret(&Case::null_dst(n, Src::Null), 22, "err_03");
    }
}

// --------------------------------------------------------------------------
// Rows 4-5: numElem == 0 with a valid dst -> 22 and dst LEFT UNMODIFIED
// --------------------------------------------------------------------------

#[test]
fn err_04_numelem_zero_valid_ptrs() {
    let mut rng = Rng::new(SEED ^ 104);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let phys = rng.range(1, 32);
        let dst = gen_dst(&mut rng, phys, phys, class); // deliberately unterminated
        let sl = rng.range(0, 8);
        let src = gen_src(&mut rng, sl, class);
        let case = Case::new(dst.clone(), 0, Src::Own(src));
        assert_same_ret(&case, 22, "err_04");
        let out = both(&case);
        assert_eq!(
            &out.dst[..phys],
            &dst[..],
            "err_04: dst[0] must NOT be zeroed on the numElem==0 path"
        );
    }
}

#[test]
fn err_05_numelem_zero_src_null() {
    // The `numElem == 0` test lives in the SAME `if` as `!dst`, so it wins over
    // the `!src` branch: dst is not zeroed even though src is NULL.
    let mut rng = Rng::new(SEED ^ 105);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let phys = rng.range(1, 32);
        let dst = gen_dst(&mut rng, phys, phys, class);
        let case = Case::new(dst.clone(), 0, Src::Null);
        assert_same_ret(&case, 22, "err_05");
        let out = both(&case);
        assert_eq!(
            &out.dst[..phys],
            &dst[..],
            "err_05: dst must be untouched when numElem == 0"
        );
    }
}

// --------------------------------------------------------------------------
// Row 6: src == NULL -> 22 AND dst[0] = 0, dst[1..] untouched
// --------------------------------------------------------------------------

#[test]
fn err_06_src_null_truncates_dst() {
    let mut rng = Rng::new(SEED ^ 106);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let phys = rng.range(1, 32);
        let n = rng.range(1, phys);
        let nul_at = if rng.range(0, 2) == 0 { phys } else { rng.range(0, phys - 1) };
        let dst = gen_dst(&mut rng, phys, nul_at, class);
        let case = Case::new(dst.clone(), n, Src::Null);
        assert_same_ret(&case, 22, "err_06");
        let out = both(&case);
        assert_eq!(out.dst[0], 0, "err_06: dst[0] must be zeroed");
        assert_eq!(
            &out.dst[1..phys],
            &dst[1..],
            "err_06: only dst[0] may change"
        );
    }
}

// --------------------------------------------------------------------------
// Rows 7, 11: unterminated dst -> 34, dst[0] = 0, src never read
// --------------------------------------------------------------------------

#[test]
fn err_07_unterminated_dst() {
    let mut rng = Rng::new(SEED ^ 107);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 48);
        let phys = n + rng.range(0, 8);
        let dst = gen_dst(&mut rng, phys, phys, class); // no NUL at all
        let sl = rng.range(0, 8);
        let src = gen_src(&mut rng, sl, class);
        let case = Case::new(dst.clone(), n, Src::Own(src.clone()));
        assert_same_ret(&case, 34, "err_07");
        let out = both(&case);
        assert_eq!(out.dst[0], 0);
        assert_eq!(&out.dst[1..phys], &dst[1..], "err_07: only dst[0] changes");
        assert_eq!(out.src, src, "err_07: src must never be read/written");
    }
}

#[test]
fn err_11_numelem_one_unterminated() {
    let mut rng = Rng::new(SEED ^ 111);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let phys = rng.range(1, 8);
        let dst = gen_dst(&mut rng, phys, phys, class);
        let sl = rng.range(0, 4);
        let src = gen_src(&mut rng, sl, class);
        assert_same_ret(&Case::new(dst, 1, Src::Own(src)), 34, "err_11");
    }
}

// --------------------------------------------------------------------------
// Rows 8-10: the copy loop exhausts the bound -> 34 with a retained partial copy
// --------------------------------------------------------------------------

#[test]
fn err_08_src_too_long() {
    let mut rng = Rng::new(SEED ^ 108);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 48);
        let phys = n + rng.range(0, 6);
        let k = rng.range(0, n - 1);
        let dst = gen_dst(&mut rng, phys, k, class);
        // strlen(dst) + wcslen(src) + 1 > numElem, comfortably.
        let l = (n - k) + rng.range(0, 8);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src.clone()));
        assert_same_ret(&case, 34, "err_08");
        let out = both(&case);
        assert_eq!(out.dst[0], 0, "err_08: dst[0] must be zeroed");
        // The partial copy is NOT rolled back: dst[k+1..n] holds src's prefix
        // (dst[k] too, unless k == 0 in which case it was re-zeroed).
        for i in (k + 1)..n {
            assert_eq!(
                out.dst[i],
                src[i - k],
                "err_08: partial copy at index {i} not retained (n={n} k={k})"
            );
        }
        assert_eq!(out.src, src, "err_08: src must not be written");
    }
}

#[test]
fn err_09_off_by_one_no_room_for_nul() {
    let mut rng = Rng::new(SEED ^ 109);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 48);
        let phys = n + rng.range(0, 6);
        let k = rng.range(0, n - 1);
        let l = n - k; // chars fit exactly; the NUL does not
        let dst = gen_dst(&mut rng, phys, k, class);
        let src = gen_src(&mut rng, l, class);
        let case = Case::new(dst, n, Src::Own(src.clone()));
        assert_same_ret(&case, 34, "err_09");
        let out = both(&case);
        assert_eq!(out.dst[0], 0);
        // Last buffer element holds src's final character; nothing rolled back.
        // (Except when numElem == 1, where the last element *is* dst[0] and the
        // trailing `dst[0] = 0` overwrites it.)
        if n > 1 {
            assert_eq!(
                out.dst[n - 1],
                src[l - 1],
                "err_09: last element should hold the final src char"
            );
        }
    }
}

#[test]
fn err_10_numelem_one_nonempty_src() {
    let mut rng = Rng::new(SEED ^ 110);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let phys = rng.range(1, 8);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let sl = rng.range(1, 6);
        let src = gen_src(&mut rng, sl, class);
        let case = Case::new(dst, 1, Src::Own(src));
        assert_same_ret(&case, 34, "err_10");
        let out = both(&case);
        assert_eq!(out.dst[0], 0, "err_10: dst[0] is written then re-zeroed");
    }
}

// --------------------------------------------------------------------------
// Rows 12-13: oversized numElem is NOT rejected (no upper-bound check in C)
// --------------------------------------------------------------------------

#[test]
fn err_12_oversized_numelem_not_rejected() {
    let mut rng = Rng::new(SEED ^ 112);
    for num_elem in [
        64usize,
        1 << 10,
        1 << 16,
        1 << 20,
        1 << 24,
        1 << 30,
        1usize << 34,
    ] {
        for _ in 0..24 {
            let class = *rng.pick(&ALL_CLASSES);
            // Real allocation is 256 elements; numElem lies about the size but
            // dst terminates early so no out-of-bounds access happens.
            let k = rng.range(0, 4);
            let dst = gen_dst(&mut rng, 256, k, class);
            let sl = rng.range(0, 4);
            let src = gen_src(&mut rng, sl, class);
            assert_same_ret(
                &Case::new(dst, num_elem, Src::Own(src)),
                0,
                &format!("err_12 numElem={num_elem}"),
            );
        }
    }
}

#[test]
fn err_13_numelem_beyond_buffer_last_elem() {
    // dst terminated at the last element of a padded allocation, with numElem
    // claiming far more space. The C writes past the logical string but stays
    // inside the padding we own.
    let mut rng = Rng::new(SEED ^ 113);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let logical = rng.range(1, 16);
        let phys = logical + 64; // padding so the write is inside our memory
        let mut dst = gen_dst(&mut rng, phys, phys, class);
        dst[logical - 1] = 0;
        let sl = rng.range(0, 8);
        let src = gen_src(&mut rng, sl, class);
        assert_same_ret(&Case::new(dst, 1 << 20, Src::Own(src)), 0, "err_13");
    }
}

// --------------------------------------------------------------------------
// Rows 14-15: aliasing is NOT rejected (no overlap check in C)
// --------------------------------------------------------------------------

#[test]
fn err_14_src_aliases_dst_empty() {
    let mut rng = Rng::new(SEED ^ 114);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 32);
        let phys = n + rng.range(0, 8);
        let mut dst = gen_dst(&mut rng, phys, 0, class);
        dst[0] = 0;
        let case = Case::new(dst, n, Src::IntoDst(0));
        assert_same_ret(&case, 0, "err_14");
        let out = both(&case);
        assert_eq!(out.dst[0], 0, "err_14: dst[0] stays the terminator");
    }
}

#[test]
fn err_15_src_overlaps_dst_interior() {
    let mut rng = Rng::new(SEED ^ 115);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(2, 40);
        let phys = n + rng.range(0, 8);
        let k = rng.range(1, n - 1);
        let mut dst = gen_dst(&mut rng, phys, k, class);
        let off = rng.range(0, n - 1);
        // Keep every read inside the real allocation (see CONFIGS.md row 24).
        let t = rng.range(off, n - 1);
        dst[t] = 0;
        assert_same(
            &Case::new(dst, n, Src::IntoDst(off)),
            &format!("err_15 n={n} k={k} off={off} t={t}"),
        );
    }
}

// --------------------------------------------------------------------------
// Rows 16-19: out-of-domain wchar_t scalars (the enum-equivalent for this API)
// --------------------------------------------------------------------------

fn out_of_domain_row(vals: &[i32], salt: u64, label: &str) {
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..ITERS {
        let n = rng.range(1, 32);
        let phys = n + rng.range(0, 8);
        let k = rng.range(0, n - 1);
        // dst prefix from the ordinary ASCII class, src from the exotic class.
        let mut dst = gen_dst(&mut rng, phys, k, ValClass::Ascii);
        if label == "err_19" {
            // Row 19: put the exotic values in dst's prefix instead.
            for i in 0..k {
                dst[i] = *rng.pick(vals);
            }
        }
        let l = rng.range(0, n + 2);
        let mut src: Vec<i32> = (0..l).map(|_| *rng.pick(vals)).collect();
        src.push(0);
        if label == "err_19" {
            src = gen_src(&mut rng, l, ValClass::Ascii);
        }
        assert_same(&Case::new(dst, n, Src::Own(src)), label);
    }
}

#[test]
fn err_16_negative_wchar_values() {
    let vals = [-1i32, -2, -128, -0x8000, i32::MIN, i32::MIN + 1, -0x11_0000];
    out_of_domain_row(&vals, 116, "err_16");
}

#[test]
fn err_17_above_unicode_max() {
    let vals = [
        0x11_0000i32,
        0x20_0000,
        0x7FFF_FFFE,
        i32::MAX,
        0x0100_0000,
        0x4000_0000,
    ];
    out_of_domain_row(&vals, 117, "err_17");
}

#[test]
fn err_18_surrogate_range_values() {
    let vals = [0xD800i32, 0xD801, 0xDBFF, 0xDC00, 0xDFFE, 0xDFFF];
    out_of_domain_row(&vals, 118, "err_18");
}

#[test]
fn err_19_dst_prefix_out_of_domain() {
    let vals = [-1i32, i32::MIN, i32::MAX, 0xD800, 0x11_0000, -0x7FFF_FFFF];
    out_of_domain_row(&vals, 119, "err_19");
}

// --------------------------------------------------------------------------
// Generic boundaries every C API has (required by Phase C even when not in the
// table): null pointers in every combination, zero and oversized lengths, and
// values one step past each documented range.
// --------------------------------------------------------------------------

#[test]
fn generic_null_pointer_matrix() {
    let mut rng = Rng::new(SEED ^ 900);
    for n in [0usize, 1, 2, 3, 16, 1 << 20, usize::MAX - 1, usize::MAX] {
        // (dst=NULL, src=NULL)
        assert_same(&Case::null_dst(n, Src::Null), "null_matrix dst=0 src=0");
        // (dst=NULL, src=valid)
        let src = gen_src(&mut rng, 3, ValClass::Ascii);
        assert_same(
            &Case::null_dst(n, Src::Own(src)),
            "null_matrix dst=0 src=ok",
        );
        // (dst=valid, src=NULL) — safe for any n because the C only ever
        // touches dst[0] on that path (and returns early when n == 0).
        let dst = gen_dst(&mut rng, 16, 4, ValClass::Ascii);
        assert_same(&Case::new(dst, n, Src::Null), "null_matrix dst=ok src=0");
    }
}

#[test]
fn generic_zero_length() {
    let mut rng = Rng::new(SEED ^ 901);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let dst = gen_dst(&mut rng, 8, 3, class);
        let sl = rng.range(0, 4);
        let src = gen_src(&mut rng, sl, class);
        assert_same_ret(&Case::new(dst.clone(), 0, Src::Own(src)), 22, "zero_len");
        assert_same_ret(&Case::new(dst, 0, Src::Null), 22, "zero_len_null_src");
    }
}

/// `numElem` one step past every interesting boundary, with the destination
/// terminated inside the real allocation so no out-of-bounds access occurs.
#[test]
fn generic_numelem_one_step_past_boundaries() {
    let mut rng = Rng::new(SEED ^ 902);
    let phys = 128usize;
    for base in [
        1usize,
        2,
        3,
        4,
        7,
        8,
        15,
        16,
        31,
        32,
        63,
        64,
        127,
        128,
        255,
        256,
        u8::MAX as usize,
        u16::MAX as usize,
        u32::MAX as usize,
    ] {
        for delta in [0i64, 1, -1] {
            let n = (base as i64 + delta).max(0) as usize;
            let class = *rng.pick(&ALL_CLASSES);
            let k = rng.range(0, 4);
            let dst = gen_dst(&mut rng, phys, k, class);
            let sl = rng.range(0, 4);
            let src = gen_src(&mut rng, sl, class);
            // For n <= phys the whole operation is inside the allocation; for
            // larger n the early terminator keeps it inside too.
            assert_same(
                &Case::new(dst, n, Src::Own(src)),
                &format!("boundary numElem={n}"),
            );
        }
    }
}

/// Zero-length `src` allocation is impossible (a wide string is at least a
/// NUL), but a `src` whose very first element is the terminator is the minimal
/// valid input and must be handled identically.
#[test]
fn generic_minimal_src() {
    let mut rng = Rng::new(SEED ^ 903);
    for _ in 0..ITERS {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(1, 32);
        let phys = n + rng.range(0, 4);
        let nul_at = if rng.range(0, 3) == 0 { phys } else { rng.range(0, phys - 1) };
        let dst = gen_dst(&mut rng, phys, nul_at, class);
        assert_same(&Case::new(dst, n, Src::Own(vec![0])), "minimal_src");
    }
}

/// The C returns exactly one of {0, 22, 34}. Confirm no other value can ever
/// escape either implementation across a wide randomized sweep, and that both
/// agree on which one.
#[test]
fn return_code_domain_is_closed() {
    let mut rng = Rng::new(SEED ^ 904);
    for _ in 0..5000 {
        let class = *rng.pick(&ALL_CLASSES);
        let n = rng.range(0, 40);
        let phys = n.max(1) + rng.range(0, 6);
        let nul_at = if rng.range(0, 4) == 0 { phys } else { rng.range(0, phys - 1) };
        let dst = gen_dst(&mut rng, phys, nul_at, class);
        let sl = rng.range(0, n + 3);
        let src = gen_src(&mut rng, sl, class);
        let case = match rng.range(0, 5) {
            0 => Case::null_dst(n, Src::Own(src)),
            1 => Case::new(dst, n, Src::Null),
            2 => Case::null_dst(n, Src::Null),
            _ => Case::new(dst, n, Src::Own(src)),
        };
        let out = both(&case);
        assert!(
            out.ret == 0 || out.ret == 22 || out.ret == 34,
            "unexpected return code {}",
            out.ret
        );
    }
}

/// `numElem` values where the C expression `dst + numElem` overflows the address
/// space and wraps, so `ptr < dst + numElem` is false immediately and both loops
/// fall through. Verified against the built C `.so`: SIZE_MAX, SIZE_MAX-1,
/// SIZE_MAX/2, 2^62 and 2^61 all return 34 with `dst[0] = 0`, while 2^61 (whose
/// byte offset still fits) returns 0. This is the reason the Rust uses
/// `wrapping_add` rather than `add` for the bound.
#[test]
fn generic_numelem_address_space_wraparound() {
    let mut rng = Rng::new(SEED ^ 905);
    let interesting: Vec<usize> = vec![
        usize::MAX,
        usize::MAX - 1,
        usize::MAX - 2,
        usize::MAX / 2,
        usize::MAX / 2 + 1,
        usize::MAX / 4,
        usize::MAX / 4 + 1,
        1usize << 63,
        1usize << 62,
        (1usize << 62) - 1,
        (1usize << 62) + 1,
        1usize << 61,
        (1usize << 61) - 1,
        1usize << 60,
        1usize << 56,
        1usize << 48,
        1usize << 40,
        1usize << 34,
    ];
    for &n in &interesting {
        for _ in 0..8 {
            let class = *rng.pick(&ALL_CLASSES);
            // The destination terminates early inside a real 256-element
            // allocation, so no genuine out-of-bounds access happens for the
            // non-wrapping values.
            let k = rng.range(0, 4);
            let dst = gen_dst(&mut rng, 256, k, class);
            let sl = rng.range(0, 4);
            let src = gen_src(&mut rng, sl, class);
            assert_same(
                &Case::new(dst, n, Src::Own(src)),
                &format!("wraparound numElem={n:#x}"),
            );
        }
        // Same sweep with src == NULL and with dst == NULL.
        let dst = gen_dst(&mut rng, 16, 4, ValClass::Ascii);
        assert_same(
            &Case::new(dst, n, Src::Null),
            &format!("wraparound numElem={n:#x} src=NULL"),
        );
        assert_same(
            &Case::null_dst(n, Src::Null),
            &format!("wraparound numElem={n:#x} dst=NULL"),
        );
    }
}

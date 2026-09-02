//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each test
//!   1. constructs the exact invalid input / rejection condition,
//!   2. calls BOTH shared objects through `dlsym`, and
//!   3. asserts they produce the *same* result **and** that the result is the
//!      specific sentinel the C is documented to produce (not merely "both
//!      failed somehow").
//!
//! `dequantize_granule` has no error return code — it always returns
//! `group_size * 4`. The observable rejection signals are therefore:
//!   * `get_bits` returning its `0` sentinel, which is visible in `grbuf` as
//!     the exact value `0 - half` (linear) or `0 % mod - mod / 2` (grouped),
//!   * `bs->pos` having advanced by exactly `n` per rejected call,
//!   * `grbuf` being left untouched when a branch is skipped,
//!   * the return value itself.

mod common;

use common::*;
use std::ffi::c_int;

/// `half` for the linear path: `(1 << (ba - 1)) - 1`.
fn half(ba: u8) -> i32 {
    (1i32 << (ba - 1)) - 1
}

/// Number of `get_bits` calls the linear path makes: 4 granule halves x
/// `2 * total_bands` bands x `group_size` samples.
fn linear_calls(total_bands: u8, group_size: c_int) -> i64 {
    4 * 2 * total_bands as i64 * group_size.max(0) as i64
}

/// Number of `get_bits` calls the grouped path makes: one per band visit.
fn grouped_calls(total_bands: u8) -> i64 {
    4 * 2 * total_bands as i64
}

fn wrap32(v: i64) -> c_int {
    v as u32 as i32
}

// ===========================================================================
// Row 1 — `bs->limit == 0`, `bs->pos == 0`, `n > 0`
//         => get_bits returns 0, bs->pos still advanced, buffer NOT read.
// ===========================================================================

#[test]
fn row01_limit_zero_rejects_every_read() {
    let (c, r) = load_impls();
    for ba in [1u8, 4, 8, 16] {
        // `buf` is all-ones: if the early-out did NOT fire, the read would
        // yield 2^ba - 1 and the stored value would be `2^ba - 1 - half`,
        // which is provably different from `-half`.
        let case = Case::new(101_000 + ba as u32, 3, 2, BaMode::Const(ba))
            .limit(LimitMode::Abs(0))
            .buf(BufMode::Ones)
            .iters(1);
        let out = run_both(&c, &r, &case, 0);

        assert_eq!(out.ret, 3 * 4);
        // Every written slot must be exactly `-half` (the get_bits-rejected value).
        let expected = -(half(ba) as f32);
        let f = out.floats();
        for &i in &out.written_indices() {
            assert_eq!(
                f[i], expected,
                "ba={ba}: slot {i} = {} but the limit==0 rejection must store {expected}",
                f[i]
            );
        }
        assert!(out.written_slots() > 0, "ba={ba}: nothing was written");
        // bs->pos advanced by n on every rejected call.
        assert_eq!(out.pos, wrap32(linear_calls(2, 3) * ba as i64), "ba={ba}");
    }
}

// ===========================================================================
// Row 2 — `bs->pos > bs->limit` already on entry: every call keeps rejecting.
// ===========================================================================

#[test]
fn row02_pos_already_past_limit() {
    let (c, r) = load_impls();
    let ba = 6u8;
    let case = Case::new(102_000, 4, 2, BaMode::Const(ba))
        .at(100)
        .limit(LimitMode::Abs(50))
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);

    assert_eq!(out.ret, 16);
    let expected = -(half(ba) as f32);
    let f = out.floats();
    for &i in &out.written_indices() {
        assert_eq!(f[i], expected, "slot {i}");
    }
    assert_eq!(out.pos, wrap32(100 + linear_calls(2, 4) * ba as i64));
}

// ===========================================================================
// Row 3 — `bs->pos + n == bs->limit` exactly: NO early-out, the read happens.
// Row 4 — `bs->pos + n == bs->limit + 1`: early-out.
// ===========================================================================

#[test]
fn row03_row04_limit_boundary_is_strictly_greater() {
    let (c, r) = load_impls();
    let ba = 8u8;

    // limit == n: first call is accepted, reads 0xFF => 255 - 127 = 128.
    let inside = Case::new(103_000, 1, 1, BaMode::Const(ba))
        .limit(LimitMode::Abs(8))
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &inside, 0);
    let f = out.floats();
    let first = out.written_indices()[0];
    assert_eq!(
        f[first], 128.0,
        "pos + n == limit must NOT reject: expected 255 - 127 = 128, got {}",
        f[first]
    );

    // limit == n - 1: the very first call is rejected.
    let outside = Case::new(104_000, 1, 1, BaMode::Const(ba))
        .limit(LimitMode::Abs(7))
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &outside, 0);
    let f = out.floats();
    let first = out.written_indices()[0];
    assert_eq!(
        f[first],
        -(half(ba) as f32),
        "pos + n == limit + 1 must reject with the 0 sentinel"
    );
}

// ===========================================================================
// Row 5 — a huge `n` from the grouped path is rejected against a sane limit.
//
// `ba` with `(ba-17)&31 == 24` gives `mod == 0x1000_0001`, `n == 29_360_131`.
// With `limit == 1000` every one of the 16 calls is rejected, and 16 * n still
// fits in `int`, so `bs->pos` is exactly `16 * n` afterwards.
// ===========================================================================

#[test]
fn row05_huge_grouped_n_is_rejected() {
    let (c, r) = load_impls();
    let ba = 17u8 + 24; // 41
    assert_eq!(k_of(ba), 24);
    let m = grouped_mod(24);
    let n = grouped_n(24);
    assert_eq!(m, 0x0200_0001); // 2 << 24 == 2^25
    assert_eq!(n, 29_360_131);

    let case = Case::new(105_000, 2, 1, BaMode::Const(ba))
        .limit(LimitMode::Abs(1000))
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);

    assert_eq!(out.pos, wrap32(grouped_calls(1) * n), "every call must be rejected");
    // code == 0 => 0 % mod - mod/2 == -(mod/2).
    let expected = ((0u32.wrapping_sub(m / 2)) as i32) as f32;
    let f = out.floats();
    for &i in &out.written_indices() {
        assert_eq!(f[i], expected, "slot {i}");
    }
}

// ===========================================================================
// Row 6 — negative `bs->limit`: the very first call is rejected.
// ===========================================================================

#[test]
fn row06_negative_limit() {
    let (c, r) = load_impls();
    for lim in [-1i32, -1000, i32::MIN + 1, i32::MIN] {
        let ba = 5u8;
        let case = Case::new(106_000u32.wrapping_add(lim as u32), 3, 2, BaMode::Const(ba))
            .limit(LimitMode::Abs(lim))
            .buf(BufMode::Ones)
            .iters(1);
        let out = run_both(&c, &r, &case, 0);
        let expected = -(half(ba) as f32);
        let f = out.floats();
        for &i in &out.written_indices() {
            assert_eq!(f[i], expected, "limit={lim} slot {i}");
        }
        assert_eq!(out.pos, wrap32(linear_calls(2, 3) * ba as i64), "limit={lim}");
    }
}

// ===========================================================================
// Row 7 — negative `bs->pos`: `pos >> 3` is arithmetic, so the read happens
//         *before* `bs->buf`.
//
// `BufMode::Split` puts 0x5A in front of `bs->buf` and 0xFF from `bs->buf` on.
// With `pos = -64` (byte -8) and `ba = 8` the first sample must therefore be
// 0x5A - 127 = 90 - 127 = -37 — distinguishable both from reading at `bs->buf`
// (255 - 127 = 128) and from a rejection (-127).
// ===========================================================================

#[test]
fn row07_negative_pos_reads_before_buffer() {
    let (c, r) = load_impls();
    let ba = 8u8;
    let case = Case::new(107_000, 1, 1, BaMode::Const(ba))
        .at(-64)
        .limit(LimitMode::Abs(1_000_000))
        .buf(BufMode::Split)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);
    let f = out.floats();
    let first = out.written_indices()[0];
    assert_eq!(
        f[first], -37.0,
        "negative pos must read 0x5A from before bs->buf: expected 90 - 127 = -37, got {}",
        f[first]
    );

    // `pos = -1` => s = 7, `pos >> 3` = -1.
    let case = Case::new(107_001, 2, 2, BaMode::Const(4))
        .at(-1)
        .limit(LimitMode::Abs(1_000_000))
        .buf(BufMode::Random)
        .iters(8);
    check_case(&c, &r, &case);
}

// ===========================================================================
// Row 8 — `n + s >= 40` makes `cache |= next << shl` an over-wide shift (UB in
// C; a 5-bit-masked `shl` on x86). Verified against a hand-computed value.
//
// ba = 22 => mod = 65, n = 59. With an all-ones buffer and s = 0:
//   shl = 59; iterations at shl = 51, 43, 35, 27, 19, 11, 3 each OR in
//   255 << (shl & 31), then `next >> 5` = 7. The OR of
//   255<<19 | 255<<11 | 255<<3 | 255<<27 | 255<<19 | 255<<11 | 255<<3 | 7
//   is 0xFFFF_FFFF, so code = 0xFFFF_FFFF.
//   0xFFFF_FFFF % 65 = 60, 60 - 32 = 28.
// A non-masking (mathematically correct) shift would give a different code.
// ===========================================================================

#[test]
fn row08_over_wide_shift_is_masked() {
    let (c, r) = load_impls();
    let ba = 22u8;
    assert_eq!(grouped_mod(k_of(ba)), 65);
    assert_eq!(grouped_n(k_of(ba)), 59);

    // Recompute the expectation with the same masked-shift semantics.
    let mut cache: u32 = 0;
    let mut shl: i32 = 59;
    let mut next: u32 = 255;
    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next.wrapping_shl(shl as u32);
        next = 255;
    }
    let code = cache | next.wrapping_shr((-shl) as u32);
    assert_eq!(code, 0xFFFF_FFFF);
    let expected = ((code % 65).wrapping_sub(65 / 2) as i32) as f32;
    assert_eq!(expected, 28.0);

    let case = Case::new(108_000, 1, 1, BaMode::Const(ba))
        .limit(LimitMode::Huge)
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);
    let f = out.floats();
    let first = out.written_indices()[0];
    assert_eq!(f[first], expected, "over-wide shift must be masked to 5 bits");

    // Same axis with randomized buffers across the shift counts that trigger it.
    for ba in [22u8, 23, 24, 25, 26, 27] {
        let case = Case::new(108_100 + ba as u32, 6, 1, BaMode::Const(ba)).iters(6);
        check_case(&c, &r, &case);
    }
}

// ===========================================================================
// Row 9 — `bitalloc[i] == 0`: the band is skipped entirely.
// ===========================================================================

#[test]
fn row09_zero_bitalloc_skips_band() {
    let (c, r) = load_impls();
    for &(g, tb) in &[(1i32, 1u8), (12, 8), (18, 64), (576, 2), (12, 255)] {
        let case = Case::new(109_000 + tb as u32, g, tb, BaMode::Const(0))
            .buf(BufMode::Ones)
            .iters(2);
        let out = run_both(&c, &r, &case, 0);
        assert_eq!(out.ret, g.wrapping_mul(4));
        assert!(
            out.grbuf_untouched(),
            "g={g} tb={tb}: ba==0 must not write to grbuf"
        );
        assert_eq!(out.pos, 0, "g={g} tb={tb}: ba==0 must not consume bits");
        check_case(&c, &r, &case);
    }
}

// ===========================================================================
// Row 10 — `bitalloc[i] == 17`: mod == 3, n == 5.
// ===========================================================================

#[test]
fn row10_ba_17_smallest_grouped() {
    let (c, r) = load_impls();
    assert_eq!(grouped_mod(0), 3);
    assert_eq!(grouped_n(0), 5);

    let case = Case::new(110_000, 2, 1, BaMode::Const(17))
        .buf(BufMode::Ones)
        .limit(LimitMode::Huge)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);
    // n = 5, s = 0 => shl = 5, loop exits immediately, code = 255 >> 3 = 31.
    // 31 % 3 = 1, 1 - 1 = 0.
    let f = out.floats();
    for &i in &out.written_indices() {
        assert!(f[i] == 0.0 || f[i] == 1.0 || f[i] == -1.0, "slot {i} = {}", f[i]);
    }
    assert_eq!(out.pos, wrap32(grouped_calls(1) * 5));
}

// ===========================================================================
// Row 11 — `(ba-17)&31 == 30`: `2 << 30` overflows `int` => mod == 0x80000001.
// ===========================================================================

#[test]
fn row11_signed_overflow_in_mod() {
    let (c, r) = load_impls();
    assert_eq!(grouped_mod(30), 0x8000_0001);
    assert_eq!(grouped_n(30), 0x7000_0003);

    for ba in [47u8, 79, 111, 143, 175, 207, 239] {
        assert_eq!(k_of(ba), 30);
        // NO_READ is mandatory here: two calls of n = 1_879_048_195 overflow
        // `bs->pos`, after which the C would dereference `bs->buf + 234 MB`.
        let case = Case::new(111_000 + ba as u32, 3, 1, BaMode::Const(ba))
            .limit(NO_READ)
            .buf(BufMode::Ones)
            .iters(1);
        let out = run_both(&c, &r, &case, 0);
        let expected = ((0u32.wrapping_sub(0x8000_0001u32 / 2)) as i32) as f32;
        assert_eq!(expected, -1_073_741_824.0);
        let f = out.floats();
        for &i in &out.written_indices() {
            assert_eq!(f[i], expected, "ba={ba} slot {i}");
        }
        // pos wraps: 8 calls x 0x70000003.
        assert_eq!(out.pos, wrap32(grouped_calls(1) * 0x7000_0003), "ba={ba}");
    }
}

// ===========================================================================
// Row 12 — `(ba-17)&31 == 31`: `2 << 31` masks to 0 => mod == 1, n == 3, and
//          every grouped sample becomes exactly 0.0.
// ===========================================================================

#[test]
fn row12_mod_one_yields_zero_samples() {
    let (c, r) = load_impls();
    assert_eq!(grouped_mod(31), 1);
    assert_eq!(grouped_n(31), 3);

    for ba in [48u8, 80, 112, 144, 176, 208, 240] {
        assert_eq!(k_of(ba), 31);
        let case = Case::new(112_000 + ba as u32, 4, 2, BaMode::Const(ba))
            .buf(BufMode::Ones)
            .limit(LimitMode::Huge)
            .iters(2);
        let out = run_both(&c, &r, &case, 0);
        let f = out.floats();
        for &i in &out.written_indices() {
            assert_eq!(f[i], 0.0, "ba={ba} slot {i}: mod==1 must give 0.0");
        }
        assert!(out.written_slots() > 0);
        assert_eq!(out.pos, wrap32(grouped_calls(2) * 3), "ba={ba}");
        check_case(&c, &r, &case);
    }
}

// ===========================================================================
// Row 13 — `bitalloc[i]` in 49..=255: the shift count `ba - 17` exceeds 31, so
//          behaviour is periodic with period 32 in `ba`. No rejection.
// ===========================================================================

#[test]
fn row13_shift_count_wraps_with_period_32() {
    let (c, r) = load_impls();
    // Same seed, same everything except `ba` differing by a multiple of 32:
    // the grouped arithmetic must be identical.
    for k in 0u32..=31 {
        let lo = (17 + k) as u8;
        let mut outs = Vec::new();
        for m in 0u32..=6 {
            let ba = 17 + k + 32 * m;
            if ba > 255 {
                break;
            }
            let ba = ba as u8;
            assert_eq!(k_of(ba), k);
            // Identical case id => identical fixture bytes for every `ba`.
            let case = Case::new(113_000 + k, 4, 2, BaMode::Const(ba))
                .limit(NO_READ)
                .buf(BufMode::Ones)
                .iters(1);
            outs.push((ba, run_both(&c, &r, &case, 0)));
        }
        let (_, ref base) = outs[0];
        for (ba, o) in &outs[1..] {
            assert_eq!(
                o.grbuf, base.grbuf,
                "ba={ba} must behave exactly like ba={lo} (shift count masked to 5 bits)"
            );
            assert_eq!(o.pos, base.pos, "ba={ba} vs ba={lo}: bs->pos differs");
        }
    }
}

// ===========================================================================
// Row 14 — `code % mod - mod / 2` is computed in `unsigned` and cast to `int`.
// ===========================================================================

#[test]
fn row14_unsigned_difference_cast_to_int() {
    let (c, r) = load_impls();
    // mod = 33 (ba = 21) => mod/2 = 16, so ~half of all residues are below it.
    let case = Case::new(114_000, 16, 4, BaMode::Const(21)).buf(BufMode::Random).iters(16);
    let out = run_both(&c, &r, &case, 0);
    let f = out.floats();
    let idx = out.written_indices();
    let negatives = idx.iter().filter(|&&i| f[i] < 0.0).count();
    let positives = idx.iter().filter(|&&i| f[i] > 0.0).count();
    assert!(negatives > 0 && positives > 0, "expected both signs, got {negatives}/{positives}");
    for &i in &idx {
        assert!(
            f[i] >= -16.0 && f[i] <= 16.0,
            "slot {i} = {} outside [-mod/2, mod-1-mod/2]",
            f[i]
        );
    }
    check_case(&c, &r, &case);

    // The same computation where mod/2 is huge (mod = 0x80000001).
    let case = Case::new(114_100, 8, 2, BaMode::Const(47)).limit(NO_READ).iters(2);
    check_case(&c, &r, &case);
}

// ===========================================================================
// Row 15 — `total_bands >= 33` makes `sci->bitalloc[i]` read out of bounds.
// ===========================================================================

#[test]
fn row15_out_of_bounds_bitalloc_read() {
    let (c, r) = load_impls();

    // bitalloc[0..64] == 0, everything from index 64 on == 9.
    // total_bands = 64 => i in 0..127, so bands 64..127 read `scfcod`.
    let case = Case::new(115_000, 4, 64, BaMode::ZeroBelowThenConst(64, 9))
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);
    assert!(
        out.written_slots() > 0,
        "the C must read bitalloc[i] past the 64-byte array (into scfcod)"
    );
    // 64 in-bounds zero bands, then 64 bands of ba = 9 x 4 samples x 4 halves.
    assert_eq!(out.pos, wrap32(4 * 64 * 4 * 9));

    // Read past the END of the struct: zeros for indices < 128 (i.e. all of
    // bitalloc + scfcod + padding), value from index 128 on. offset 770 + 128
    // == 898, and sizeof(L12_scale_info) == 900, so indices >= 130 are strictly
    // past the object.
    let case = Case::new(115_100, 4, 255, BaMode::ZeroBelowThenConst(130, 9))
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);
    assert!(
        out.written_slots() > 0,
        "the C must read bitalloc[i] past the end of the struct"
    );
    // i in 130..509 => 380 active bands per half.
    assert_eq!(out.pos, wrap32(4 * 380 * 4 * 9));

    // Randomized differential coverage of the whole OOB index range.
    for tb in [33u8, 34, 64, 65, 100, 128, 129, 130, 200, 255] {
        check_case(
            &c,
            &r,
            &Case::new(115_200 + tb as u32, 6, tb, BaMode::Range(0, 255)).iters(4),
        );
    }
}

// ===========================================================================
// Row 16 — `total_bands == 0`: nothing happens at all.
// ===========================================================================

#[test]
fn row16_total_bands_zero() {
    let (c, r) = load_impls();
    for g in [-8i32, -1, 0, 1, 12, 576] {
        let case = Case::new(116_000u32.wrapping_add(g as u32), g, 0, BaMode::Range(0, 255))
            .buf(BufMode::Ones)
            .iters(2);
        let out = run_both(&c, &r, &case, 0);
        assert_eq!(out.ret, g.wrapping_mul(4), "g={g}");
        assert!(out.grbuf_untouched(), "g={g}: grbuf must be untouched");
        assert_eq!(out.pos, 0, "g={g}: no bits may be consumed");
        check_case(&c, &r, &case);
    }
}

// ===========================================================================
// Row 17 — `group_size == 0`: no writes, but the grouped path still consumes
//          bits because `get_bits` is called *before* the `k` loop.
// ===========================================================================

#[test]
fn row17_group_size_zero() {
    let (c, r) = load_impls();

    // Linear: get_bits is inside the k loop => nothing is consumed.
    let case = Case::new(117_000, 0, 8, BaMode::Const(7)).buf(BufMode::Ones).iters(2);
    let out = run_both(&c, &r, &case, 0);
    assert_eq!(out.ret, 0);
    assert!(out.grbuf_untouched());
    assert_eq!(out.pos, 0, "linear path must not consume bits when group_size == 0");

    // Grouped: get_bits precedes the k loop => bits ARE consumed.
    let case = Case::new(117_100, 0, 8, BaMode::Const(20)).buf(BufMode::Ones).iters(2);
    let out = run_both(&c, &r, &case, 0);
    assert_eq!(out.ret, 0);
    assert!(out.grbuf_untouched());
    assert_eq!(
        out.pos,
        wrap32(grouped_calls(8) * grouped_n(k_of(20))),
        "grouped path must still consume bits when group_size == 0"
    );
    check_case(&c, &r, &case);
}

// ===========================================================================
// Row 18 — `group_size < 0`: no writes, wild (unused) `dst`, wrapped return.
// ===========================================================================

#[test]
fn row18_negative_group_size() {
    let (c, r) = load_impls();
    for g in [-1i32, -2, -18, -576, -100_000, i32::MIN, i32::MIN + 1, -0x4000_0000] {
        let case = Case::new(118_000u32.wrapping_add(g as u32), g, 4, BaMode::Range(1, 16))
            .buf(BufMode::Ones)
            .iters(1);
        let out = run_both(&c, &r, &case, 0);
        assert_eq!(out.ret, g.wrapping_mul(4), "g={g}");
        assert!(out.grbuf_untouched(), "g={g}");
        assert_eq!(out.pos, 0, "g={g}: linear path consumes nothing");
    }
    // i32::MIN * 4 wraps to 0.
    let case = Case::new(118_900, i32::MIN, 0, BaMode::Const(0)).iters(1);
    let out = run_both(&c, &r, &case, 0);
    assert_eq!(out.ret, 0, "INT_MIN * 4 must wrap to 0");
}

// ===========================================================================
// Row 19 — `grbuf == NULL` in a configuration that never dereferences it.
// ===========================================================================

#[test]
fn row19_null_grbuf_when_never_dereferenced() {
    let (c, r) = load_impls();
    let cases = [
        // total_bands == 0
        Case::new(119_000, 12, 0, BaMode::Range(0, 255)).null_grbuf().iters(2),
        // every ba == 0
        Case::new(119_001, 12, 8, BaMode::Const(0)).null_grbuf().iters(2),
        // group_size == 0, linear
        Case::new(119_002, 0, 8, BaMode::Const(9)).null_grbuf().iters(2),
        // group_size == 0, grouped (get_bits still runs)
        Case::new(119_003, 0, 8, BaMode::Const(20)).null_grbuf().iters(2),
        // group_size < 0
        Case::new(119_004, -4, 8, BaMode::Range(1, 16)).null_grbuf().iters(2),
        // group_size < 0, grouped
        Case::new(119_005, -4, 8, BaMode::Range(17, 32)).null_grbuf().iters(2),
    ];
    for case in &cases {
        let out = run_both(&c, &r, case, 0);
        assert_eq!(out.ret, case.group_size.wrapping_mul(4), "case {}", case.id);
        check_case(&c, &r, case);
    }
}

// ===========================================================================
// Row 20 — `bs == NULL` in a configuration that never dereferences it.
// ===========================================================================

#[test]
fn row20_null_bs_when_never_dereferenced() {
    let (c, r) = load_impls();
    let cases = [
        // total_bands == 0 => get_bits never called
        Case::new(120_000, 12, 0, BaMode::Range(0, 255)).null_bs().iters(2),
        // every ba == 0 => get_bits never called
        Case::new(120_001, 12, 8, BaMode::Const(0)).null_bs().iters(2),
        // linear with group_size == 0 => get_bits never called
        Case::new(120_002, 0, 8, BaMode::Const(9)).null_bs().iters(2),
        // linear with group_size < 0 => get_bits never called
        Case::new(120_003, -3, 8, BaMode::Range(1, 16)).null_bs().iters(2),
        // both pointers null
        Case::new(120_004, 12, 8, BaMode::Const(0)).null_bs().null_grbuf().iters(2),
    ];
    for case in &cases {
        let out = run_both(&c, &r, case, 0);
        assert_eq!(out.ret, case.group_size.wrapping_mul(4), "case {}", case.id);
        assert!(out.grbuf_untouched() || case.null_grbuf, "case {}", case.id);
        check_case(&c, &r, case);
    }
}

// ===========================================================================
// Row 21 — `group_size * 4` overflows `int` (signed overflow; wraps at -O0).
//          Tested with `total_bands == 0` so no 4 GiB buffer is needed.
// ===========================================================================

#[test]
fn row21_return_value_overflow() {
    let (c, r) = load_impls();
    for g in [0x4000_0000i32, 0x2000_0000, 0x7FFF_FFFF, 0x6000_0000, i32::MIN] {
        let case = Case::new(121_000u32.wrapping_add(g as u32), g, 0, BaMode::Const(0))
            .null_grbuf()
            .null_bs()
            .iters(1);
        let out = run_both(&c, &r, &case, 0);
        assert_eq!(
            out.ret,
            g.wrapping_mul(4),
            "group_size={g:#x}: return value must wrap exactly like the C"
        );
    }
    // Explicit expectations for the documented wrap points.
    assert_eq!(0x4000_0000i32.wrapping_mul(4), 0);
    assert_eq!(0x2000_0000i32.wrapping_mul(4), i32::MIN);
}

// ===========================================================================
// Row 22 — `bs->pos & 7 != 0`: the first byte is masked with `255 >> s`.
//
// With an all-ones buffer, ba = 8 and s = 3 the first byte contributes
// 0xFF & (255 >> 3) = 31, then the loop pulls in the next 0xFF:
//   shl = 11 -> cache = 31 << 3 = 248; next = 255; shl = 3 -> cache |= 255 << 3
//   ... verified by recomputation below.
// ===========================================================================

#[test]
fn row22_unaligned_first_byte_is_masked() {
    let (c, r) = load_impls();
    let ba = 8u8;
    for s in 0u32..8 {
        // Recompute get_bits for an all-ones buffer.
        let mut cache: u32 = 0;
        let mut shl: i32 = ba as i32 + s as i32;
        let mut next: u32 = 0xFFu32 & (255u32 >> s);
        loop {
            shl -= 8;
            if shl <= 0 {
                break;
            }
            cache |= next.wrapping_shl(shl as u32);
            next = 255;
        }
        let expected = (cache | next.wrapping_shr((-shl) as u32)) as i32 - half(ba);

        let case = Case::new(122_000 + s, 1, 1, BaMode::Const(ba))
            .at(s as c_int)
            .limit(LimitMode::Huge)
            .buf(BufMode::Ones)
            .iters(1);
        let out = run_both(&c, &r, &case, 0);
        let f = out.floats();
        let first = out.written_indices()[0];
        assert_eq!(
            f[first], expected as f32,
            "s={s}: first sample must be built from a `255 >> s` masked first byte"
        );
    }

    // Randomized sweep over all 8 alignments, both paths.
    for s in 0u32..8 {
        check_case(
            &c,
            &r,
            &Case::new(122_100 + s, 12, 8, BaMode::Range(1, 16)).at(s as c_int).iters(8),
        );
        check_case(
            &c,
            &r,
            &Case::new(122_200 + s, 12, 8, BaMode::Range(17, 32)).at(s as c_int).iters(8),
        );
    }
}

// ===========================================================================
// Generic boundary coverage demanded on top of the table.
// ===========================================================================

/// Zero and oversized lengths, and values one step past every documented range.
#[test]
fn generic_boundaries_one_step_past_ranges() {
    let (c, r) = load_impls();
    let mut cases: Vec<Case> = Vec::new();

    // `ba` around the 0 / 1 / 16 / 17 branch boundaries.
    for (i, ba) in [0u8, 1, 15, 16, 17, 18].into_iter().enumerate() {
        cases.push(Case::new(130_000 + i as u32, 6, 3, BaMode::Const(ba)).iters(6));
    }
    // `total_bands` around 32/33 (bitalloc bound), 64/65, 129/130 (struct end).
    for tb in [0u8, 1, 32, 33, 64, 65, 129, 130, 254, 255] {
        cases.push(Case::new(131_000 + tb as u32, 6, tb, BaMode::Range(1, 16)).iters(4));
    }
    // `group_size`: zero, one, and oversized.
    for (i, g) in [0i32, 1, 2, 575, 576, 577, 1024].into_iter().enumerate() {
        cases.push(Case::new(132_000 + i as u32, g, 2, BaMode::Range(1, 16)).iters(4));
    }
    // `limit` one step either side of the acceptance boundary, for every ba.
    for ba in 1u8..=16 {
        let n = ba as c_int;
        cases.push(
            Case::new(133_000 + ba as u32, 1, 1, BaMode::Const(ba))
                .limit(LimitMode::Abs(n))
                .buf(BufMode::Ones)
                .iters(1),
        );
        cases.push(
            Case::new(133_100 + ba as u32, 1, 1, BaMode::Const(ba))
                .limit(LimitMode::Abs(n - 1))
                .buf(BufMode::Ones)
                .iters(1),
        );
    }
    // Extreme `pos` values (bounded by the 64 KiB of headroom in front of
    // `bs->buf`: `pos = -400_000` reads at most 50 000 bytes before it).
    for (i, p) in [-400_000i32, -100_000, -1, 0, 1, 500_000].into_iter().enumerate() {
        cases.push(
            Case::new(134_000 + i as u32, 6, 4, BaMode::Range(1, 16))
                .at(p)
                .limit(LimitMode::RelPos(100_000))
                .iters(4),
        );
    }
    for case in &cases {
        check_case(&c, &r, case);
    }
}

/// The public API declares no `enum`, so there is no "invalid enum variant" to
/// pass. The closest analogue is the `uint8_t` mode selector `bitalloc[i]`
/// (and `total_bands`), whose *entire* 0..=255 domain — including the values a
/// real MPEG bitstream can never produce — is swept here across the FFI
/// boundary in both directions.
#[test]
fn generic_boundaries_full_uint8_domains() {
    let (c, r) = load_impls();
    for ba in 0u32..=255 {
        // Read-free so the huge-`n` residues cannot fault the C.
        check_case(
            &c,
            &r,
            &Case::new(135_000 + ba, 3, 2, BaMode::Const(ba as u8))
                .limit(NO_READ)
                .iters(2),
        );
    }
    for tb in 0u32..=255 {
        check_case(
            &c,
            &r,
            &Case::new(136_000 + tb, 3, tb as u8, BaMode::Range(0, 255))
                .limit(NO_READ)
                .iters(2),
        );
    }
}

//! Phase B — valid-path differential tests.
//!
//! One test function per group of rows in `CONFIGS.md`. Every row is driven
//! with many randomized inputs (fixed SplitMix64 seeds keyed on the row id and
//! repetition index, so failures are reproducible).
//!
//! Both implementations are reached only through `dlopen` + `dlsym` on their
//! respective `.so`.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Rows 1..10 — linear path (ba 1..16) across group_size x total_bands,
// including the total_bands values that push `bitalloc[i]` out of bounds.
// ---------------------------------------------------------------------------

#[test]
fn rows_01_10_linear_group_size_and_total_bands() {
    let cases = [
        Case::new(1, 1, 1, BaMode::Const(1)),
        Case::new(2, 2, 1, BaMode::Const(16)),
        Case::new(3, 3, 2, BaMode::Range(1, 16)),
        Case::new(4, 12, 8, BaMode::Range(1, 16)),
        Case::new(5, 18, 32, BaMode::Range(1, 16)), // max i = 63, in bounds
        Case::new(6, 32, 33, BaMode::Range(1, 16)), // i reaches 64 -> scfcod
        Case::new(7, 64, 64, BaMode::Range(1, 16)), // max i = 127
        Case::new(8, 12, 65, BaMode::Range(1, 16)), // max i = 129 -> padding
        Case::new(9, 12, 128, BaMode::Range(1, 16)), // max i = 255 -> past struct
        Case::new(10, 12, 255, BaMode::Range(1, 16)), // max i = 509
    ];
    check_cases(&cases);
}

// ---------------------------------------------------------------------------
// Rows 11..15 — `get_bits` bit-alignment axis (`s = bs->pos & 7`) and a
// negative `bs->pos` (arithmetic `>> 3`, read before `bs->buf`).
// ---------------------------------------------------------------------------

#[test]
fn rows_11_15_bit_alignment_and_negative_pos() {
    let cases = [
        Case::new(11, 12, 8, BaMode::Range(1, 16)).at(1),
        Case::new(12, 12, 8, BaMode::Range(1, 16)).at(7),
        Case::new(13, 12, 8, BaMode::Range(1, 16)).pos(PosMode::RandomUpTo(64)).iters(32),
        Case::new(14, 12, 8, BaMode::Range(1, 16)).at(8),
        Case::new(15, 12, 8, BaMode::Range(1, 16)).at(-1),
    ];
    check_cases(&cases);
}

// ---------------------------------------------------------------------------
// Rows 16..30 — grouped path, one row per interesting `bitalloc` value.
// ---------------------------------------------------------------------------

#[test]
fn rows_16_30_grouped_path_individual_values() {
    let cases = [
        Case::new(16, 1, 1, BaMode::Const(17)),  // mod=3,     n=5
        Case::new(17, 2, 1, BaMode::Const(18)),  // mod=5,     n=7
        Case::new(18, 3, 2, BaMode::Const(19)),  // mod=9,     n=10
        Case::new(19, 12, 8, BaMode::Const(20)), // mod=17,    n=17
        Case::new(20, 12, 8, BaMode::Const(21)), // mod=33,    n=31
        Case::new(21, 12, 8, BaMode::Const(22)), // mod=65,    n=59  -> shl >= 32
        Case::new(22, 12, 8, BaMode::Const(23)), // mod=129,   n=115
        Case::new(23, 12, 8, BaMode::Const(24)), // mod=257,   n=227
        Case::new(24, 12, 8, BaMode::Const(25)), // mod=513,   n=451
        Case::new(25, 12, 8, BaMode::Const(30)).iters(8), // mod=16385, n=14339
        // ba=46 (k=29) and ba=47 (k=30) make `bs->pos` overflow after two
        // calls, which segfaults the C. INT_MIN limit keeps get_bits on its
        // early-out path so the arithmetic is still compared.
        Case::new(26, 12, 8, BaMode::Const(46)).limit(NO_READ),
        Case::new(27, 12, 8, BaMode::Const(47)).limit(NO_READ), // 2<<30 overflows int
        Case::new(28, 12, 8, BaMode::Const(48)), // 2<<31 == 0 -> mod=1, n=3
        Case::new(29, 12, 8, BaMode::Const(49)), // (ba-17)&31 == 0 -> same as 17
        Case::new(30, 12, 8, BaMode::Const(255)).iters(8), // k=14, mod=32769, n=28675
    ];
    check_cases(&cases);
}

// ---------------------------------------------------------------------------
// Rows 31..39 — grouped ranges, mixed paths, skip-all, sparse.
// ---------------------------------------------------------------------------

#[test]
fn rows_31_39_ranges_and_mixed_paths() {
    let cases = [
        Case::new(31, 12, 8, BaMode::Range(17, 48)),
        Case::new(32, 12, 8, BaMode::Range(49, 255)),
        Case::new(33, 12, 8, BaMode::Range(17, 255)).at(3),
        Case::new(34, 12, 8, BaMode::Range(0, 255)).iters(32),
        Case::new(35, 12, 32, BaMode::Range(0, 255)).iters(16),
        Case::new(36, 12, 255, BaMode::Range(0, 255)).iters(8),
        Case::new(37, 18, 64, BaMode::BoundaryMix).limit(NO_READ),
        Case::new(38, 12, 8, BaMode::Const(0)),
        Case::new(39, 12, 32, BaMode::Sparse).iters(16),
    ];
    check_cases(&cases);
}

// ---------------------------------------------------------------------------
// Rows 40..47 — bitstream exhaustion and degenerate buffer contents.
// ---------------------------------------------------------------------------

#[test]
fn rows_40_47_exhaustion_and_degenerate_buffers() {
    let cases = [
        Case::new(40, 12, 8, BaMode::Range(1, 16)).limit(LimitMode::RelPos(64)),
        Case::new(41, 12, 8, BaMode::Range(1, 16)).limit(LimitMode::RelPos(1000)),
        Case::new(42, 12, 8, BaMode::Range(17, 32)).limit(LimitMode::RelPos(500)),
        Case::new(43, 12, 255, BaMode::Range(0, 255))
            .limit(LimitMode::RelPos(10_000))
            .iters(8),
        Case::new(44, 12, 8, BaMode::Range(1, 16)).buf(BufMode::Zeros),
        Case::new(45, 12, 8, BaMode::Range(1, 16)).buf(BufMode::Ones),
        Case::new(46, 12, 8, BaMode::Range(17, 48)).buf(BufMode::Zeros),
        Case::new(47, 12, 8, BaMode::Range(17, 48)).buf(BufMode::Ones),
    ];
    check_cases(&cases);
}

// ---------------------------------------------------------------------------
// Rows 48..54 — degenerate group_size / total_bands, large stride.
// ---------------------------------------------------------------------------

#[test]
fn rows_48_54_degenerate_sizes() {
    let cases = [
        Case::new(48, 0, 8, BaMode::Range(1, 16)), // no writes, no bits consumed
        Case::new(49, 0, 8, BaMode::Range(17, 48)), // no writes, bits consumed
        Case::new(50, -1, 8, BaMode::Range(0, 255)),
        Case::new(51, -7, 255, BaMode::Range(0, 255)).iters(8),
        Case::new(52, 12, 0, BaMode::Range(0, 255)),
        Case::new(53, 576, 2, BaMode::Range(1, 16)),
        Case::new(54, 576, 2, BaMode::Range(17, 32)),
    ];
    check_cases(&cases);
}

// ---------------------------------------------------------------------------
// Rows 55..59 — extreme `bs->pos`, every alignment, exact `limit` boundary.
// ---------------------------------------------------------------------------

#[test]
fn rows_55_59_pos_extremes_and_limit_boundary() {
    let cases = [
        Case::new(55, 12, 8, BaMode::Range(1, 16)).at(500_000),
        Case::new(56, 12, 8, BaMode::Range(1, 16)).at(-1000),
        Case::new(57, 12, 8, BaMode::Range(1, 255)).pos(PosMode::RepMod8).iters(24),
        // pos + n == limit exactly -> the read IS performed.
        Case::new(58, 1, 1, BaMode::Const(8)).limit(LimitMode::Abs(8)),
        // pos + n == limit + 1 -> early-out.
        Case::new(59, 1, 1, BaMode::Const(8)).limit(LimitMode::Abs(7)),
    ];
    check_cases(&cases);
}

// ---------------------------------------------------------------------------
// Rows 60..72 — the full `bitalloc` domain with `limit = INT_MIN`.
//
// This is the only way to reach shift residues k = 25..31 (n up to
// 1_879_048_195) without the C overflowing `bs->pos` and faulting. get_bits
// always returns 0, so what is compared here is the `mod` / `mod / 2` /
// `code % mod - mod / 2` arithmetic and the `bs->pos` wraparound.
// ---------------------------------------------------------------------------

#[test]
fn rows_60_72_full_ba_domain_without_reads() {
    let cases = [
        Case::new(60, 12, 8, BaMode::Range(0, 255)).limit(NO_READ).iters(32),
        Case::new(61, 12, 255, BaMode::Range(0, 255)).limit(NO_READ).iters(8),
        Case::new(62, 12, 8, BaMode::BoundaryMix).limit(NO_READ),
        Case::new(63, 12, 8, BaMode::Const(42)).limit(NO_READ), // k=25
        Case::new(64, 12, 8, BaMode::Const(43)).limit(NO_READ), // k=26
        Case::new(65, 12, 8, BaMode::Const(44)).limit(NO_READ), // k=27
        Case::new(66, 12, 8, BaMode::Const(45)).limit(NO_READ), // k=28
        Case::new(67, 12, 8, BaMode::Const(234)).limit(NO_READ), // k=25, high ba
        Case::new(68, 12, 8, BaMode::Const(239)).limit(NO_READ), // k=30, high ba
        Case::new(69, 12, 8, BaMode::Range(1, 16)).limit(NO_READ), // linear, exhausted
        Case::new(70, 576, 8, BaMode::Range(0, 255)).limit(NO_READ),
        Case::new(71, 0, 8, BaMode::Range(0, 255)).limit(NO_READ),
        Case::new(72, -3, 8, BaMode::Range(0, 255)).limit(NO_READ),
    ];
    check_cases(&cases);
}

// ---------------------------------------------------------------------------
// Row 73 — the widest grouped read that fits in the fixture: k = 22 means
// n = 7_340_035, so `shl` starts at ~7.34e6 and every one of the ~917k loop
// iterations does an over-wide `next << shl` (masked to 5 bits on x86).
// ---------------------------------------------------------------------------

#[test]
fn row_73_widest_feasible_grouped_read() {
    // ba = 39 -> k = 22. total_bands = 1 keeps the call count at 8.
    check_cases(&[Case::new(73, 4, 1, BaMode::Const(39)).iters(4)]);
}

// ---------------------------------------------------------------------------
// Row 74 — exhaustive sweep of every linear `bitalloc` value 1..=16 with the
// buffer actually being read.
// ---------------------------------------------------------------------------

#[test]
fn row_74_every_linear_ba_value() {
    let (c, r) = load_impls();
    for ba in 1u8..=16 {
        let case = Case::new(74_000 + ba as u32, 12, 2, BaMode::Const(ba)).iters(8);
        check_case(&c, &r, &case);
    }
}

// ---------------------------------------------------------------------------
// Row 75 — every grouped shift residue k = 0..=22 with the buffer being read
// (k >= 23 needs n > 14.6 Mbit, i.e. a >1.8 MB bitstream and an overflowing
// `bs->pos`; those residues are covered read-free by rows 60..72).
// ---------------------------------------------------------------------------

#[test]
fn row_75_every_readable_shift_residue() {
    let (c, r) = load_impls();
    for k in 0u32..=22 {
        let ba = (17 + k) as u8;
        let iters = if k >= 18 { 2 } else { 6 };
        let case = Case::new(75_000 + k, 6, 1, BaMode::Const(ba)).iters(iters);
        assert_eq!(k_of(ba), k);
        check_case(&c, &r, &case);
    }
}

// ---------------------------------------------------------------------------
// Row 76 — every single `bitalloc` value 0..=255, read-free, so that all 256
// values of this `uint8_t` "mode selector" cross the FFI boundary.
// ---------------------------------------------------------------------------

#[test]
fn row_76_every_ba_value_0_255_without_reads() {
    let (c, r) = load_impls();
    for ba in 0u32..=255 {
        let case = Case::new(76_000 + ba, 5, 2, BaMode::Const(ba as u8))
            .limit(NO_READ)
            .iters(3);
        check_case(&c, &r, &case);
    }
}

// ---------------------------------------------------------------------------
// Row 77 — every `total_bands` value 0..=255 (sweeps the whole out-of-bounds
// `bitalloc[i]` index range 0..509 and the `i`-loop bound).
// ---------------------------------------------------------------------------

#[test]
fn row_77_every_total_bands_value() {
    let (c, r) = load_impls();
    for tb in 0u32..=255 {
        let case = Case::new(77_000 + tb, 4, tb as u8, BaMode::Range(1, 16)).iters(2);
        check_case(&c, &r, &case);
    }
}

// ---------------------------------------------------------------------------
// Row 78 — group_size sweep, both paths.
// ---------------------------------------------------------------------------

#[test]
fn row_78_group_size_sweep() {
    let (c, r) = load_impls();
    let sizes: [c_int; 16] =
        [-64, -18, -3, -2, -1, 0, 1, 2, 3, 4, 6, 12, 18, 32, 64, 576];
    for (i, &g) in sizes.iter().enumerate() {
        check_case(&c, &r, &Case::new(78_000 + i as u32, g, 8, BaMode::Range(1, 16)).iters(4));
        check_case(
            &c,
            &r,
            &Case::new(78_500 + i as u32, g, 8, BaMode::Range(17, 32)).iters(4),
        );
        check_case(
            &c,
            &r,
            &Case::new(78_900 + i as u32, g, 8, BaMode::Range(0, 255))
                .limit(NO_READ)
                .iters(4),
        );
    }
}

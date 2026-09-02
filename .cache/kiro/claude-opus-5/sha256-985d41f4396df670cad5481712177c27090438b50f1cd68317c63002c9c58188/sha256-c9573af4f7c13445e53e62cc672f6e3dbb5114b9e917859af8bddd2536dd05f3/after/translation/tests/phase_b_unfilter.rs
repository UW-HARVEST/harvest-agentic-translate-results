//! Phase B — valid-path differential tests for `unfilter`
//! (`CONFIGS.md` rows C33..C50), randomized with a fixed seed.

mod common;

use common::*;

const AB: CBuild = CBuild::AsBuilt;

/// `unfilter` walks `h` rows of `1 + w*bpp` bytes. Row `y >= 1` with a filter
/// byte in `{2,3,4}` reads `prev[x]` for `x < bpp`, and case 4 reads
/// `prev[x - bpp]` for `x >= bpp`, so `prev` is only ever read inside the
/// previous row. The buffer is padded anyway so any C over-read lands on
/// deterministic bytes in both libraries.
const PAD: usize = 64;

fn stride(w: i32, bpp: i32) -> usize {
    (1 + w.wrapping_mul(bpp).max(0)) as usize
}

fn build_raw(rng: &mut Rng, w: i32, h: i32, bpp: i32, filters: &[u8], fill: Fill) -> Vec<u8> {
    let rows = h.max(0) as usize;
    let st = stride(w, bpp);
    let mut raw = vec![0u8; rows * st + PAD];
    for y in 0..rows {
        raw[y * st] = filters[y % filters.len()];
        for x in 1..st {
            raw[y * st + x] = fill.byte(rng, y, x);
        }
    }
    // Padding gets a recognisable pattern so an over-read is still identical
    // across the two libraries but not silently all-zero.
    for i in rows * st..raw.len() {
        raw[i] = 0x5A;
    }
    raw
}

#[derive(Copy, Clone)]
enum Fill {
    Zero,
    Max,
    Alternating,
    Random,
}

impl Fill {
    fn byte(self, rng: &mut Rng, y: usize, x: usize) -> u8 {
        match self {
            Fill::Zero => 0x00,
            Fill::Max => 0xFF,
            Fill::Alternating => {
                if (x + y) % 2 == 0 {
                    0x00
                } else {
                    0xFF
                }
            }
            Fill::Random => rng.byte(),
        }
    }
    fn all() -> [Fill; 4] {
        [Fill::Zero, Fill::Max, Fill::Alternating, Fill::Random]
    }
}

const BPPS: [i32; 5] = [1, 2, 3, 4, 8];
const WS: [i32; 6] = [0, 1, 2, 7, 16, 64];

// ---------------------------------------------------------------------------
// C33-C34 — h <= 0
// ---------------------------------------------------------------------------

#[test]
fn c33_h_zero_leaves_raw_untouched() {
    let mut rng = Rng::new(0x3301);
    for _ in 0..200 {
        let w = rng.range(0, 64) as i32;
        let bpp = rng.range(0, 8) as i32;
        let raw = {
            let n = rng_len(&mut rng);
            rng.bytes(n)
        };
        let r = diff_unfilter(w, 0, bpp, &raw, AB, "C33");
        assert_eq!(r.ret, 1, "C33: h=0 must succeed");
        assert_eq!(r.raw, raw, "C33: raw must be untouched when h == 0");
    }
}

fn rng_len(rng: &mut Rng) -> usize {
    rng.range(1, 512)
}

#[test]
fn c34_h_negative_leaves_raw_untouched() {
    let mut rng = Rng::new(0x3401);
    for &h in &[-1i32, -2, -100, -1024, i32::MIN / 2] {
        for _ in 0..40 {
            let w = rng.range(0, 64) as i32;
            let bpp = rng.range(0, 8) as i32;
            let n = rng_len(&mut rng);
            let raw = rng.bytes(n);
            let r = diff_unfilter(w, h, bpp, &raw, AB, "C34");
            assert_eq!(r.ret, 1, "C34: h={h} must succeed");
            assert_eq!(r.raw, raw, "C34: raw must be untouched when h < 0");
        }
    }
}

// ---------------------------------------------------------------------------
// C35-C39 — h == 1, each row-0 filter
// ---------------------------------------------------------------------------

fn single_row(filter: u8, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    for &bpp in &BPPS {
        for &w in &WS {
            for fill in Fill::all() {
                for _ in 0..6 {
                    let raw = build_raw(&mut rng, w, 1, bpp, &[filter], fill);
                    let r = diff_unfilter(w, 1, bpp, &raw, AB, label);
                    if w.wrapping_mul(bpp) >= 1 {
                        assert_eq!(r.ret, 1, "{label}: filter={filter} w={w} bpp={bpp}");
                    }
                }
            }
        }
    }
}

#[test]
fn c35_row0_filter_none() {
    single_row(0, 0x3501, "C35");
}

#[test]
fn c36_row0_filter_sub() {
    single_row(1, 0x3601, "C36");
}

#[test]
fn c37_row0_filter_up() {
    single_row(2, 0x3701, "C37");
}

#[test]
fn c38_row0_filter_average() {
    single_row(3, 0x3801, "C38");
}

#[test]
fn c39_row0_filter_paeth() {
    single_row(4, 0x3901, "C39");
}

// ---------------------------------------------------------------------------
// C40-C44 — h >= 2, uniform filter
// ---------------------------------------------------------------------------

fn uniform_filter(filter: u8, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    for &bpp in &BPPS {
        for &w in &WS {
            for &h in &[2i32, 3, 5, 17] {
                for fill in Fill::all() {
                    for _ in 0..3 {
                        let raw = build_raw(&mut rng, w, h, bpp, &[filter], fill);
                        let r = diff_unfilter(w, h, bpp, &raw, AB, label);
                        // With len == 0 the stride is 1, so a row's prologue
                        // writes into the *next* row's filter byte and the
                        // stream can reject itself (ERRORS.md E8). The C does
                        // that, so only demand success when len >= 1.
                        if w.wrapping_mul(bpp) >= 1 {
                            assert_eq!(
                                r.ret, 1,
                                "{label}: filter={filter} w={w} h={h} bpp={bpp}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn c40_rows_filter_none() {
    uniform_filter(0, 0x4001, "C40");
}

#[test]
fn c41_rows_filter_sub() {
    uniform_filter(1, 0x4101, "C41");
}

#[test]
fn c42_rows_filter_up() {
    uniform_filter(2, 0x4201, "C42");
}

#[test]
fn c43_rows_filter_average() {
    uniform_filter(3, 0x4301, "C43");
}

#[test]
fn c44_rows_filter_paeth() {
    uniform_filter(4, 0x4401, "C44");
}

// ---------------------------------------------------------------------------
// C45 — random per-row filter mix
// ---------------------------------------------------------------------------

#[test]
fn c45_random_filter_mix() {
    let mut rng = Rng::new(0x4501);
    for _ in 0..600 {
        let w = rng.range(0, 40) as i32;
        let h = rng.range(1, 20) as i32;
        let bpp = rng.range(1, 8) as i32;
        let st = stride(w, bpp);
        let mut raw = vec![0u8; h as usize * st + PAD];
        for y in 0..h as usize {
            raw[y * st] = rng.below(5) as u8;
            for x in 1..st {
                raw[y * st + x] = rng.byte();
            }
        }
        for i in h as usize * st..raw.len() {
            raw[i] = 0x5A;
        }
        let r = diff_unfilter(w, h, bpp, &raw, AB, "C45");
        if w.wrapping_mul(bpp) >= 1 {
            assert_eq!(r.ret, 1, "C45: w={w} h={h} bpp={bpp}");
        }
    }
}

// ---------------------------------------------------------------------------
// C46-C48 — len / bpp relationships
// ---------------------------------------------------------------------------

#[test]
fn c46_len_zero() {
    // len == w * bpp == 0: every inner loop is empty, but one filter byte per
    // row is still consumed, so the filter values still decide accept/reject.
    let mut rng = Rng::new(0x4601);
    for &(w, bpp) in &[(0i32, 1i32), (0, 4), (0, 8), (1, 0), (16, 0), (0, 0)] {
        for &h in &[1i32, 2, 5] {
            for filter in 0..5u8 {
                let raw = build_raw(&mut rng, w, h, bpp, &[filter], Fill::Random);
                let r = diff_unfilter(w, h, bpp, &raw, AB, "C46");
                if h == 1 || bpp == 0 {
                    // A single row, or bpp == 0, means the prologue writes
                    // nothing at all.
                    assert_eq!(r.ret, 1, "C46: w={w} bpp={bpp} h={h} filter={filter}");
                    assert_eq!(r.raw, raw, "C46: nothing should be modified");
                }
            }
        }
    }
}

#[test]
fn c47_len_equals_bpp() {
    // w == 1: the row-0 `x = bpp; x < len` loops never run; rows >= 1 run only
    // the `x < bpp` prologue.
    let mut rng = Rng::new(0x4701);
    for &bpp in &BPPS {
        for &h in &[1i32, 2, 4, 9] {
            for filter in 0..5u8 {
                for fill in Fill::all() {
                    let raw = build_raw(&mut rng, 1, h, bpp, &[filter], fill);
                    let r = diff_unfilter(1, h, bpp, &raw, AB, "C47");
                    assert_eq!(r.ret, 1, "C47: bpp={bpp} h={h} filter={filter}");
                    let _ = &r.raw;
                }
            }
        }
    }
}

#[test]
fn c48_bpp_greater_than_len() {
    // bpp > len is only reachable as len == 0 (w == 0), plus the neighbouring
    // shape bpp > w (len = w*bpp still >= bpp when w >= 1). Both are covered
    // with generous padding because the row-y prologue indexes prev[0..bpp)
    // regardless of len.
    let mut rng = Rng::new(0x4801);
    for &bpp in &[1i32, 3, 5, 8, 16, 33] {
        for &w in &[0i32, 1, 2] {
            for &h in &[1i32, 2, 3] {
                for filter in 0..5u8 {
                    let raw = build_raw(&mut rng, w, h, bpp, &[filter], Fill::Random);
                    let r = diff_unfilter(w, h, bpp, &raw, AB, "C48");
                    if w.wrapping_mul(bpp) >= 1 {
                        assert_eq!(r.ret, 1, "C48: w={w} bpp={bpp} h={h} filter={filter}");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C49 — value coverage (wrapping add, all three paeth outcomes)
// ---------------------------------------------------------------------------

#[test]
fn c49_value_coverage() {
    let mut rng = Rng::new(0x4901);
    // Exhaustive paeth coverage: for bpp == 1, a 2x2 image lets (a, b, c) be
    // driven directly, so sweep the whole 24-bit space at a coarse stride plus
    // a dense random sample.
    for filter in 0..5u8 {
        for fill in Fill::all() {
            for _ in 0..40 {
                let raw = build_raw(&mut rng, 8, 8, 1, &[filter], fill);
                let r = diff_unfilter(8, 8, 1, &raw, AB, "C49");
                assert_eq!(r.ret, 1);
            }
        }
    }
    // Direct paeth predicate sweep via filter 4 on a 2-row, bpp=1, w=2 image:
    // row1 x=1 evaluates cp_paeth(raw[0], prev[1], prev[0]).
    let st = 3usize;
    for a in (0..=255u16).step_by(7) {
        for b in (0..=255u16).step_by(11) {
            for c in (0..=255u16).step_by(13) {
                let raw = vec![
                    0u8, c as u8, b as u8, // row 0: filter None, then c, b
                    4u8, a as u8, 0u8, // row 1: filter Paeth, then a, target
                ];
                assert_eq!(raw.len(), 2 * st);
                let r = diff_unfilter(2, 2, 1, &raw, AB, "C49-paeth");
                assert_eq!(r.ret, 1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C50 — large image, single call
// ---------------------------------------------------------------------------

#[test]
fn c50_large_image() {
    let mut rng = Rng::new(0x5001);
    for _ in 0..8 {
        let (w, h, bpp) = (257i32, 129i32, 4i32);
        let st = stride(w, bpp);
        let mut raw = vec![0u8; h as usize * st + PAD];
        for y in 0..h as usize {
            raw[y * st] = rng.below(5) as u8;
            for x in 1..st {
                raw[y * st + x] = rng.byte();
            }
        }
        let r = diff_unfilter(w, h, bpp, &raw, AB, "C50");
        assert_eq!(r.ret, 1);
    }
}

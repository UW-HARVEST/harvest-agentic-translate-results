//! Phase B — valid-path differential tests for `unfilter`, the one function
//! declared by `c_src/include/lib.h`.
//!
//! Covers rows 1..22 of `CONFIGS.md`.  Every row is driven with many
//! randomized inputs (fixed seed) rather than one hand-picked value, because
//! all five filters are *value dependent* (`cp_paeth`'s three-way compare and
//! the `/2` truncation in the Average filter).

mod common;

use common::*;

/// How much slack the scratch region needs on either side of `raw`: `unfilter`
/// happily walks backwards when `bpp` or `w*bpp` is negative.
fn span(h: i32, len: i32) -> usize {
    let rows = if h > 0 { h as i64 } else { 0 } + 2;
    let l = (len as i64).abs() + 2;
    (rows * l + 64) as usize
}

struct Shape {
    w: i32,
    h: i32,
    bpp: i32,
}

/// Build a case: the whole scratch is filled with `fill`, then the per-row
/// filter bytes are overwritten with `filters` (row `r`'s filter byte lives at
/// `raw + r*(len+1)`, which is where `unfilter`'s `raw += len` walk puts it).
fn build(s: &Shape, filters: &[u8], fill: &dyn Fn(usize) -> u8) -> Case {
    let len = s.w.wrapping_mul(s.bpp);
    let pad = span(s.h, len) as isize;
    let total = (2 * pad + span(s.h, len) as isize) as usize;
    let mut scratch: Vec<u8> = (0..total).map(fill).collect();
    for (r, &f) in filters.iter().enumerate() {
        let off = pad + (r as isize) * (len as isize + 1);
        if off >= 0 && (off as usize) < total {
            scratch[off as usize] = f;
        }
    }
    Case::unfilter(scratch, s.w, s.h, s.bpp, pad)
}

fn check(s: &Shape, filters: &[u8], rng: &mut Rng, ctx: &str) -> Outcome {
    let r = std::cell::RefCell::new(Rng(rng.next_u64()));
    let case = build(s, filters, &|_| r.borrow_mut().u8());
    diff(&case, ctx)
}

/// Random filter byte, restricted to the five *valid* values.
fn valid_filters(rng: &mut Rng, n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.below(5) as u8).collect()
}

// ---------------------------------------------------------------------------
// rows 1 + 2: h <= 0 -> early out, not a single byte touched
// ---------------------------------------------------------------------------

#[test]
fn cfg01_h_zero() {
    let mut rng = Rng::new(0x01);
    for _ in 0..200 {
        let s = Shape { w: rng.range(-8, 64), h: 0, bpp: rng.range(-8, 8) };
        let o = check(&s, &[], &mut rng, "cfg01 h=0");
        assert_eq!(o.ret, 1);
        // nothing may have been written
        let case = build(&s, &[], &|i| (i & 0xFF) as u8);
        let o = diff(&case, "cfg01 untouched");
        assert_eq!(o.scratch, case.scratch, "h=0 must not touch the buffer");
    }
}

#[test]
fn cfg02_h_negative() {
    let mut rng = Rng::new(0x02);
    for h in [-1i32, -2, -7, -1000, i32::MIN, i32::MIN + 1] {
        for _ in 0..30 {
            let s = Shape { w: rng.range(-8, 64), h, bpp: rng.range(-8, 8) };
            let case = build(&s, &[], &|i| (i as u8).wrapping_mul(37));
            let o = diff(&case, &format!("cfg02 h={h}"));
            assert_eq!(o.ret, 1);
            assert_eq!(o.scratch, case.scratch, "h<0 must not touch the buffer");
        }
    }
}

// ---------------------------------------------------------------------------
// rows 3..8: h == 1, each of the five row-0 filters
// ---------------------------------------------------------------------------

#[test]
fn cfg03to08_h1_each_filter() {
    let mut rng = Rng::new(0x03);
    for f in 0u8..5 {
        for _ in 0..300 {
            let bpp = rng.range(1, 8);
            let w = rng.range(1, 20);
            let s = Shape { w, h: 1, bpp };
            check(&s, &[f], &mut rng, &format!("cfg03..08 h=1 filter={f} w={w} bpp={bpp}"));
        }
    }
}

#[test]
fn cfg05_h1_bpp_ge_len() {
    // `for (x = bpp; x < len; x++)` never runs.
    let mut rng = Rng::new(0x05);
    for f in 0u8..5 {
        for w in [0i32, 1] {
            for bpp in [1i32, 2, 3, 4, 8, 16] {
                let s = Shape { w, h: 1, bpp };
                check(&s, &[f], &mut rng, &format!("cfg05 w={w} bpp={bpp} f={f}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 9: h == 2, the full 5x5 cross product of row-0 x row-1 filters
// ---------------------------------------------------------------------------

#[test]
fn cfg09_h2_filter_cross_product() {
    let mut rng = Rng::new(0x09);
    for f0 in 0u8..5 {
        for f1 in 0u8..5 {
            for _ in 0..40 {
                let bpp = rng.range(1, 8);
                let w = rng.range(1, 24);
                let s = Shape { w, h: 2, bpp };
                check(
                    &s,
                    &[f0, f1],
                    &mut rng,
                    &format!("cfg09 f0={f0} f1={f1} w={w} bpp={bpp}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 10: many rows, an independently random filter for each one
// ---------------------------------------------------------------------------

#[test]
fn cfg10_multi_row_random_filters() {
    let mut rng = Rng::new(0x10);
    for _ in 0..600 {
        let h = rng.range(3, 12);
        let bpp = rng.range(1, 8);
        let w = rng.range(1, 24);
        let filters = valid_filters(&mut rng, h as usize);
        let s = Shape { w, h, bpp };
        check(&s, &filters, &mut rng, &format!("cfg10 h={h} w={w} bpp={bpp} f={filters:?}"));
    }
}

// ---------------------------------------------------------------------------
// rows 11..14: bpp variants
// ---------------------------------------------------------------------------

#[test]
fn cfg11_bpp_one() {
    let mut rng = Rng::new(0x11);
    for _ in 0..400 {
        let h = rng.range(1, 10);
        let w = rng.range(1, 40);
        let filters = valid_filters(&mut rng, h as usize);
        check(&Shape { w, h, bpp: 1 }, &filters, &mut rng, "cfg11 bpp=1");
    }
}

#[test]
fn cfg12_bpp_2_3_4() {
    let mut rng = Rng::new(0x12);
    for bpp in [2i32, 3, 4] {
        for h in 1..9 {
            for _ in 0..60 {
                let w = rng.range(1, 24);
                let filters = valid_filters(&mut rng, h as usize);
                check(
                    &Shape { w, h, bpp },
                    &filters,
                    &mut rng,
                    &format!("cfg12 bpp={bpp} h={h} w={w}"),
                );
            }
        }
    }
}

#[test]
fn cfg13_bpp_equals_len() {
    let mut rng = Rng::new(0x13);
    for bpp in [1i32, 2, 3, 4, 5, 8, 16, 33] {
        for h in 1..6 {
            for _ in 0..25 {
                let filters = valid_filters(&mut rng, h as usize);
                check(
                    &Shape { w: 1, h, bpp },
                    &filters,
                    &mut rng,
                    &format!("cfg13 w=1 bpp={bpp} h={h}"),
                );
            }
        }
    }
}

#[test]
fn cfg14_bpp_greater_than_len() {
    // len == 0 (w == 0) with bpp > 0: the `x < bpp` prologue loops of filters
    // 2/3/4 still run and spill into the following row's bytes.
    let mut rng = Rng::new(0x14);
    for bpp in [1i32, 2, 3, 7, 16] {
        for h in 1..6 {
            for _ in 0..25 {
                let filters = valid_filters(&mut rng, h as usize);
                check(
                    &Shape { w: 0, h, bpp },
                    &filters,
                    &mut rng,
                    &format!("cfg14 w=0 bpp={bpp} h={h}"),
                );
            }
        }
    }
}

#[test]
fn cfg15_bpp_zero() {
    let mut rng = Rng::new(0x15);
    for h in [1i32, 2, 5, 9] {
        for _ in 0..60 {
            let w = rng.range(-8, 40);
            let filters = valid_filters(&mut rng, h as usize);
            check(&Shape { w, h, bpp: 0 }, &filters, &mut rng, &format!("cfg15 bpp=0 h={h} w={w}"));
        }
    }
}

#[test]
fn cfg16_w_zero() {
    let mut rng = Rng::new(0x16);
    for h in [1i32, 2, 3, 8] {
        for bpp in [0i32, 1, 2, 4, 9] {
            for _ in 0..30 {
                let filters = valid_filters(&mut rng, h as usize);
                check(
                    &Shape { w: 0, h, bpp },
                    &filters,
                    &mut rng,
                    &format!("cfg16 w=0 h={h} bpp={bpp}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 17..19: negative strides
// ---------------------------------------------------------------------------

#[test]
fn cfg17_bpp_negative() {
    let mut rng = Rng::new(0x17);
    for bpp in [-1i32, -2, -3, -8] {
        for h in [1i32, 2, 4] {
            for _ in 0..40 {
                let w = rng.range(1, 16);
                let filters = valid_filters(&mut rng, h as usize);
                check(
                    &Shape { w, h, bpp },
                    &filters,
                    &mut rng,
                    &format!("cfg17 bpp={bpp} w={w} h={h}"),
                );
            }
        }
    }
}

#[test]
fn cfg18_w_negative() {
    let mut rng = Rng::new(0x18);
    for w in [-1i32, -2, -5, -16] {
        for h in [1i32, 2, 4] {
            for _ in 0..40 {
                let bpp = rng.range(1, 8);
                let filters = valid_filters(&mut rng, h as usize);
                check(
                    &Shape { w, h, bpp },
                    &filters,
                    &mut rng,
                    &format!("cfg18 w={w} bpp={bpp} h={h}"),
                );
            }
        }
    }
}

#[test]
fn cfg19_w_and_bpp_negative() {
    let mut rng = Rng::new(0x19);
    for w in [-1i32, -3, -12] {
        for bpp in [-1i32, -2, -5] {
            for h in [1i32, 2, 3] {
                for _ in 0..25 {
                    let filters = valid_filters(&mut rng, h as usize);
                    check(
                        &Shape { w, h, bpp },
                        &filters,
                        &mut rng,
                        &format!("cfg19 w={w} bpp={bpp} h={h}"),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 20: the pointer's alignment inside the scratch page must be irrelevant
// ---------------------------------------------------------------------------

#[test]
fn cfg20_pointer_offsets() {
    let mut rng = Rng::new(0x20);
    for skew in 0..8usize {
        for _ in 0..60 {
            let h = rng.range(1, 6);
            let bpp = rng.range(1, 8);
            let w = rng.range(1, 20);
            let len = w.wrapping_mul(bpp);
            let pad = span(h, len) as isize + skew as isize;
            let total = (2 * pad + span(h, len) as isize) as usize;
            let mut r = Rng(rng.next_u64());
            let mut scratch: Vec<u8> = (0..total).map(|_| r.u8()).collect();
            let filters = valid_filters(&mut rng, h as usize);
            for (i, &f) in filters.iter().enumerate() {
                let off = pad + (i as isize) * (len as isize + 1);
                scratch[off as usize] = f;
            }
            let case = Case::unfilter(scratch, w, h, bpp, pad);
            diff(&case, &format!("cfg20 skew={skew}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 21: bigger shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg21_large_sweep() {
    let mut rng = Rng::new(0x21);
    for _ in 0..250 {
        let h = rng.range(1, 48);
        let bpp = rng.range(1, 8);
        let w = rng.range(1, 64);
        let filters = valid_filters(&mut rng, h as usize);
        check(&Shape { w, h, bpp }, &filters, &mut rng, &format!("cfg21 w={w} h={h} bpp={bpp}"));
    }
}

// ---------------------------------------------------------------------------
// row 22: data patterns that walk every branch of cp_paeth and the /2 rounding
// ---------------------------------------------------------------------------

#[test]
fn cfg22_paeth_and_average_patterns() {
    let patterns: Vec<(&str, Box<dyn Fn(usize) -> u8>)> = vec![
        ("zeros", Box::new(|_| 0)),
        ("ones", Box::new(|_| 0xFF)),
        ("half", Box::new(|_| 0x80)),
        ("ramp", Box::new(|i| i as u8)),
        ("revramp", Box::new(|i| !(i as u8))),
        ("alt", Box::new(|i| if i % 2 == 0 { 0 } else { 0xFF })),
        ("lowbits", Box::new(|i| (i % 3) as u8)),
        ("edge", Box::new(|i| [0u8, 1, 0x7F, 0x80, 0x81, 0xFE, 0xFF][i % 7])),
    ];
    let mut rng = Rng::new(0x22);
    for (name, f) in &patterns {
        for f0 in 0u8..5 {
            for f1 in 0u8..5 {
                for bpp in [1i32, 2, 3, 4] {
                    let w = 9;
                    let h = 4;
                    // filter bytes are part of the pattern too, so overwrite
                    // them explicitly per row
                    let s = Shape { w, h, bpp };
                    let filters = [f0, f1, f0, f1];
                    let case = build(&s, &filters, f.as_ref());
                    diff(&case, &format!("cfg22 {name} f0={f0} f1={f1} bpp={bpp}"));
                }
            }
        }
    }
    // and fully random values on top, to cover the tie-break cases the fixed
    // patterns miss
    for _ in 0..400 {
        let h = rng.range(2, 6);
        let bpp = rng.range(1, 5);
        let w = rng.range(2, 12);
        let filters: Vec<u8> = (0..h).map(|_| rng.pick(&[3u8, 4])).collect();
        check(&Shape { w, h, bpp }, &filters, &mut rng, "cfg22 random paeth/avg");
    }
}

// ---------------------------------------------------------------------------
// exhaustive small-shape sweep: every (w, h, bpp) in a small box x every
// filter combination.  This is the strongest single check for `unfilter`.
// ---------------------------------------------------------------------------

#[test]
fn cfg23_exhaustive_small_shapes() {
    let mut rng = Rng::new(0x23);
    for w in -2i32..=5 {
        for h in -1i32..=3 {
            for bpp in -2i32..=4 {
                let nrows = h.max(0) as usize;
                // every filter combination for up to 3 rows (5^3 = 125)
                let combos: Vec<Vec<u8>> = match nrows {
                    0 => vec![vec![]],
                    1 => (0u8..5).map(|a| vec![a]).collect(),
                    2 => (0u8..5).flat_map(|a| (0u8..5).map(move |b| vec![a, b])).collect(),
                    _ => (0u8..5)
                        .flat_map(|a| {
                            (0u8..5).flat_map(move |b| (0u8..5).map(move |c| vec![a, b, c]))
                        })
                        .collect(),
                };
                for filters in combos {
                    check(
                        &Shape { w, h, bpp },
                        &filters,
                        &mut rng,
                        &format!("cfg23 w={w} h={h} bpp={bpp} f={filters:?}"),
                    );
                }
            }
        }
    }
}

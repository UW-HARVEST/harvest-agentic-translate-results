//! Phase B — valid-path differential tests for `unfilter`.
//! Covers CONFIGS.md rows C1..C12 and C36.

mod harness;

use harness::{diff_unfilter, pair, unfilter_buf_len, Pair, Rng};

const PAD: usize = 4096; // slack before `raw` so negative indices stay mapped
const TAIL: usize = 4096; // slack after, so over-writes are visible

/// Build `h` rows of `1 + w*bpp` bytes with the given per-row filter bytes.
fn rows(rng: &mut Rng, w: i32, h: i32, bpp: i32, filters: &[u8], alphabet: Option<&[u8]>) -> Vec<u8> {
    let len = (w as i64 * bpp as i64).max(0) as usize;
    let mut v = Vec::with_capacity(h.max(0) as usize * (len + 1) + 8);
    for y in 0..h.max(0) as usize {
        v.push(filters[y % filters.len()]);
        for _ in 0..len {
            match alphabet {
                None => v.push(rng.u8()),
                Some(a) => v.push(rng.pick(a)),
            }
        }
    }
    if v.is_empty() {
        v.push(rng.u8());
    }
    v
}

fn case(p: &Pair, tag: &str, rng: &mut Rng, w: i32, h: i32, bpp: i32, filters: &[u8], alpha: Option<&[u8]>) {
    let data = rows(rng, w, h, bpp, filters, alpha);
    diff_unfilter(p, tag, w, h, bpp, &data, PAD, TAIL);
}

// ---------------------------------------------------------------------------
// C1..C5 — single row (h == 1): only the row-0 switch runs, whose formulas
// differ from the row->=1 switch for filters 2, 3 and 4.
// ---------------------------------------------------------------------------

/// C1 / C2 / C3 / C4 / C5 — one row per row-0 filter byte 0..=4.
#[test]
fn c1_c2_c3_c4_c5_single_row_all_filters() {
    let p = pair();
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    let row_name = ["C1", "C2", "C3", "C4", "C5"];
    for f in 0u8..=4 {
        for w in [1i32, 7, 64] {
            for bpp in [1i32, 2, 3, 4] {
                for rep in 0..24 {
                    case(
                        &p,
                        &format!("{}/f={f} rep={rep}", row_name[f as usize]),
                        &mut rng,
                        w,
                        1,
                        bpp,
                        &[f],
                        None,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C6 — row-0 filter x row-1 filter cross product (25 combinations).
// ---------------------------------------------------------------------------

#[test]
fn c6_row0_x_row1_filter_cross_product() {
    let p = pair();
    let mut rng = Rng::new(0x0BAD_C0DE_1234_5678);
    for f0 in 0u8..=4 {
        for f1 in 0u8..=4 {
            for w in [1i32, 7] {
                for bpp in [1i32, 3, 4] {
                    for rep in 0..12 {
                        case(
                            &p,
                            &format!("C6/f0={f0},f1={f1} rep={rep}"),
                            &mut rng,
                            w,
                            2,
                            bpp,
                            &[f0, f1],
                            None,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C7 — the composed multi-row pipeline with random per-row filters.
// ---------------------------------------------------------------------------

#[test]
fn c7_multirow_random_filters() {
    let p = pair();
    let mut rng = Rng::new(0xFEED_FACE_CAFE_0001);
    for h in [2i32, 3, 8, 17] {
        for w in [1i32, 2, 5, 16, 64] {
            for bpp in [1i32, 2, 3, 4, 8] {
                for rep in 0..8 {
                    let filters: Vec<u8> = (0..h).map(|_| rng.below(5) as u8).collect();
                    case(
                        &p,
                        &format!("C7/h={h},w={w},bpp={bpp} rep={rep}"),
                        &mut rng,
                        w,
                        h,
                        bpp,
                        &filters,
                        None,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C8 — bpp > len. The row->=1 `for (x = 0; x < bpp; x++)` pre-loop writes `bpp`
// bytes even when `len < bpp`, i.e. past the nominal row.
// ---------------------------------------------------------------------------

#[test]
fn c8_bpp_greater_than_len() {
    let p = pair();
    let mut rng = Rng::new(0x5151_5151_5151_5151);
    let shapes: &[(i32, i32)] = &[
        (1, 1),
        (1, 2),
        (1, 3),
        (1, 4),
        (1, 5),
        (1, 6),
        (1, 7),
        (1, 8),
        (2, 5),
        (2, 9),
        (3, 7),
    ];
    for &(w, bpp) in shapes {
        for h in [1i32, 2, 4] {
            for f in 0u8..=4 {
                for rep in 0..4 {
                    // deliberately size the buffer generously: the C writes past
                    // `len` here, and the snapshot must cover it in both libs.
                    let len = (w * bpp) as usize;
                    let mut data = rows(&mut rng, w, h, bpp, &[f], None);
                    data.resize(data.len() + bpp as usize + len + 8, 0x33);
                    diff_unfilter(
                        &p,
                        &format!("C8/w={w},bpp={bpp},h={h},f={f},rep={rep}"),
                        w,
                        h,
                        bpp,
                        &data,
                        PAD,
                        TAIL,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C9 — len == 0 (bpp == 0 or w == 0).
// ---------------------------------------------------------------------------

#[test]
fn c9_zero_len() {
    let p = pair();
    let mut rng = Rng::new(0x9999_0000_9999_0000);
    for (w, bpp) in [(0i32, 4i32), (0, 0), (4, 0), (1, 0), (0, 1), (64, 0)] {
        for h in [0i32, 1, 5] {
            for f in 0u8..=4 {
                let mut data = rng.bytes(64);
                for y in 0..h.max(1) as usize {
                    data[y] = f;
                }
                diff_unfilter(
                    &p,
                    &format!("C9/w={w},bpp={bpp},h={h},f={f}"),
                    w,
                    h,
                    bpp,
                    &data,
                    PAD,
                    TAIL,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C10 — h <= 0: both switch statements are skipped entirely.
// ---------------------------------------------------------------------------

#[test]
fn c10_nonpositive_h() {
    let p = pair();
    let mut rng = Rng::new(0xA0A0_B0B0_C0C0_D0D0);
    for h in [0i32, -1, -7, -1000, i32::MIN + 1] {
        for w in [0i32, 1, 8] {
            for bpp in [0i32, 1, 4] {
                let data = rng.bytes(128);
                diff_unfilter(
                    &p,
                    &format!("C10/h={h},w={w},bpp={bpp}"),
                    w,
                    h,
                    bpp,
                    &data,
                    PAD,
                    TAIL,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C11 — negative bpp: `len = w*bpp` is negative and the row-0 loops index from
// a negative offset. PAD keeps those accesses inside the shared mapping.
// ---------------------------------------------------------------------------

#[test]
fn c11_negative_bpp() {
    let p = pair();
    let mut rng = Rng::new(0xDEAD_BEEF_0F0F_0F0F);
    for bpp in [-1i32, -2, -3] {
        for w in [1i32, 4] {
            for h in [1i32, 3] {
                for f in 0u8..=4 {
                    let mut data = rng.bytes(512);
                    data[0] = f;
                    diff_unfilter(
                        &p,
                        &format!("C11/bpp={bpp},w={w},h={h},f={f}"),
                        w,
                        h,
                        bpp,
                        &data,
                        PAD,
                        TAIL,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — paeth tie-break saturation. Restricting the data alphabet to extremes
// makes `pa == pb`, `pa == pc` and `pb == pc` all occur, exercising every
// branch of `(pa <= pb && pa <= pc) ? a : (pb <= pc) ? b : c`.
// ---------------------------------------------------------------------------

#[test]
fn c12_paeth_tie_breaks() {
    let p = pair();
    let mut rng = Rng::new(0x0C0C_0C0C_1234_ABCD);
    let alpha: &[u8] = &[0, 1, 2, 126, 127, 128, 129, 253, 254, 255];
    for bpp in [1i32, 3, 4] {
        for h in [2i32, 5] {
            for w in [1i32, 9, 33] {
                for rep in 0..10 {
                    case(
                        &p,
                        &format!("C12/bpp={bpp},h={h},w={w},rep={rep}"),
                        &mut rng,
                        w,
                        h,
                        bpp,
                        &[4],
                        Some(alpha),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C36 — fully random (w, h, bpp) plus random data.
// ---------------------------------------------------------------------------

#[test]
fn c36_random_shapes() {
    let p = pair();
    let mut rng = Rng::new(0x1357_9BDF_2468_ACE0);
    for i in 0..8000 {
        let w = rng.range(0, 40) as i32;
        let h = rng.range(0, 12) as i32;
        let bpp = rng.range(0, 9) as i32;
        let mut data = vec![0u8; unfilter_buf_len(w, h, bpp) + bpp as usize + 16];
        for b in data.iter_mut() {
            *b = rng.u8();
        }
        // keep every filter byte valid so this row stays on the happy path
        let len = (w * bpp) as usize;
        for y in 0..h.max(0) as usize {
            data[y * (len + 1)] = rng.below(5) as u8;
        }
        diff_unfilter(&p, &format!("C36/#{i}"), w, h, bpp, &data, PAD, TAIL);
    }
}

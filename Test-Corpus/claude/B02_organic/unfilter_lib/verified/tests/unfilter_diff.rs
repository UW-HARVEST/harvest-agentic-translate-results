//! Phase B — valid-path differential tests for `unfilter` (rows U1…U17 of
//! `CONFIGS.md`).
//!
//! Every row runs many randomized inputs through *both* `.so`s and compares the
//! return value and the whole scanline buffer (including 32 trailing guard bytes
//! that must stay untouched). A third, independent reference model of the C code
//! is checked as well, so a row cannot "pass" because both libraries are
//! equally wrong about what the C source says.

mod common;

use common::{check_unfilter, Rng};

const PAD: usize = 32;
const GUARD: u8 = 0x5A;

/// Independent transcription of `lib.c:417-478`, used as a cross-check.
fn reference(w: i32, h: i32, bpp: i32, raw: &mut [u8]) -> i32 {
    fn paeth(a: u8, b: u8, c: u8) -> u8 {
        let p = a as i32 + b as i32 - c as i32;
        let pa = (p - a as i32).abs();
        let pb = (p - b as i32).abs();
        let pc = (p - c as i32).abs();
        if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        }
    }
    let len = (w as isize) * (bpp as isize);
    let bpp = bpp as isize;
    let mut r: isize = 0;
    if h > 0 {
        let f = raw[r as usize];
        r += 1;
        match f {
            0 | 2 => {}
            1 => {
                let mut x = bpp;
                while x < len {
                    let v = raw[(r + x - bpp) as usize];
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
            }
            3 => {
                let mut x = bpp;
                while x < len {
                    let v = raw[(r + x - bpp) as usize] / 2;
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
            }
            4 => {
                let mut x = bpp;
                while x < len {
                    let v = paeth(raw[(r + x - bpp) as usize], 0, 0);
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    let mut prev = r;
    r += len;
    let mut y = 1;
    while y < h {
        let f = raw[r as usize];
        r += 1;
        match f {
            0 => {}
            1 => {
                let mut x = 0;
                while x < bpp {
                    x += 1; // `raw[x] += 0`
                }
                while x < len {
                    let v = raw[(r + x - bpp) as usize];
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
            }
            2 => {
                // NB: two separate loops in the C source; the first is bounded
                // by `bpp`, *not* by `len`, so for `bpp > len` it writes past
                // the nominal end of the scanline.
                let mut x = 0;
                while x < bpp {
                    let v = raw[(prev + x) as usize];
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let v = raw[(prev + x) as usize];
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
            }
            3 => {
                let mut x = 0;
                while x < bpp {
                    let v = raw[(prev + x) as usize] / 2;
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let v = (raw[(r + x - bpp) as usize] as i32 + raw[(prev + x) as usize] as i32)
                        / 2;
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v as u8);
                    x += 1;
                }
            }
            4 => {
                let mut x = 0;
                while x < bpp {
                    let v = raw[(prev + x) as usize];
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let v = paeth(
                        raw[(r + x - bpp) as usize],
                        raw[(prev + x) as usize],
                        raw[(prev + x - bpp) as usize],
                    );
                    let i = (r + x) as usize;
                    raw[i] = raw[i].wrapping_add(v);
                    x += 1;
                }
            }
            _ => return 0,
        }
        y += 1;
        prev = r;
        r += len;
    }
    1
}

/// Build a scanline buffer: `h` rows of `[filter, w*bpp data bytes]` plus PAD
/// guard bytes.
fn build(
    rng: &mut Rng,
    w: i32,
    h: i32,
    bpp: i32,
    filt: &mut dyn FnMut(usize, &mut Rng) -> u8,
    data: &mut dyn FnMut(&mut Rng) -> u8,
) -> Vec<u8> {
    let len = (w.max(0) as usize) * (bpp.max(0) as usize);
    let mut v = Vec::new();
    for y in 0..h.max(0) as usize {
        v.push(filt(y, rng));
        for _ in 0..len {
            v.push(data(rng));
        }
    }
    v.extend(std::iter::repeat(GUARD).take(PAD));
    v
}

#[track_caller]
fn one(ctx: &str, w: i32, h: i32, bpp: i32, buf: &[u8]) {
    let (ret, got) = check_unfilter(ctx, w, h, bpp, buf);
    let mut refbuf = buf.to_vec();
    let rref = reference(w, h, bpp, &mut refbuf);
    assert_eq!(
        ret, rref,
        "[{ctx}] reference model disagrees with the libraries about the return value \
         (w={w},h={h},bpp={bpp})"
    );
    assert_eq!(
        got, refbuf,
        "[{ctx}] reference model disagrees with the libraries about the output \
         (w={w},h={h},bpp={bpp})"
    );
    // Highest byte the C code can touch. Rows >= 1 with filter 2/3/4 run their
    // first loop up to `bpp` even when `bpp > len`, so the span is
    // max(len, bpp) for those rows.
    let len = (w.max(0) as usize) * (bpp.max(0) as usize);
    let touched = if h <= 0 {
        0
    } else if h == 1 {
        1 + len
    } else {
        (h as usize - 1) * (len + 1) + 1 + len.max(bpp.max(0) as usize)
    };
    assert!(
        got.len() >= touched + 4,
        "[{ctx}] test buffer too small (w={w},h={h},bpp={bpp})"
    );
    assert!(
        got[touched..].iter().all(|&b| b == GUARD),
        "[{ctx}] guard bytes beyond offset {touched} were modified (w={w},h={h},bpp={bpp}): {:02x?}",
        &got[touched..]
    );
}

/// Common driver for the "all rows use filter `f`" rows.
fn sweep_single_filter(ctx: &str, seed: u64, f: u8, hlo: i32, hhi: i32, iters: usize) {
    let mut rng = Rng::new(seed);
    for it in 0..iters {
        let w = rng.range(1, 17);
        let bpp = rng.range(1, 8);
        let h = rng.range(hlo, hhi);
        let buf = build(&mut rng, w, h, bpp, &mut |_, _| f, &mut |r| r.u8());
        one(&format!("{ctx}#{it}"), w, h, bpp, &buf);
    }
}

#[test]
fn u1_h1_filter0() {
    sweep_single_filter("U1", 0x1001, 0, 1, 1, 300);
}
#[test]
fn u2_h1_filter1() {
    sweep_single_filter("U2", 0x1002, 1, 1, 1, 300);
}
#[test]
fn u3_h1_filter2() {
    sweep_single_filter("U3", 0x1003, 2, 1, 1, 300);
}
#[test]
fn u4_h1_filter3() {
    sweep_single_filter("U4", 0x1004, 3, 1, 1, 300);
}
#[test]
fn u5_h1_filter4() {
    sweep_single_filter("U5", 0x1005, 4, 1, 1, 300);
}

#[test]
fn u6_multirow_filter0() {
    sweep_single_filter("U6", 0x1006, 0, 2, 9, 300);
}
#[test]
fn u7_multirow_filter1() {
    sweep_single_filter("U7", 0x1007, 1, 2, 9, 300);
}
#[test]
fn u8_multirow_filter2() {
    sweep_single_filter("U8", 0x1008, 2, 2, 9, 300);
}
#[test]
fn u9_multirow_filter3() {
    sweep_single_filter("U9", 0x1009, 3, 2, 9, 300);
}
#[test]
fn u10_multirow_filter4() {
    sweep_single_filter("U10", 0x100A, 4, 2, 9, 300);
}

#[test]
fn u11_multirow_mixed_filters() {
    let mut rng = Rng::new(0x100B);
    for it in 0..600 {
        let w = rng.range(1, 20);
        let bpp = rng.range(1, 8);
        let h = rng.range(2, 12);
        let buf = build(
            &mut rng,
            w,
            h,
            bpp,
            &mut |_, r| r.below(5) as u8,
            &mut |r| r.u8(),
        );
        one(&format!("U11#{it}"), w, h, bpp, &buf);
    }
}

#[test]
fn u12_w1_len_eq_bpp() {
    // w == 1  =>  len == bpp: only the `x < bpp` prologues run.
    let mut rng = Rng::new(0x100C);
    for bpp in 1..=8 {
        for f in 0..=4u8 {
            for h in 1..=6 {
                for it in 0..10 {
                    let buf = build(&mut rng, 1, h, bpp, &mut |_, _| f, &mut |r| r.u8());
                    one(&format!("U12 bpp={bpp} f={f} h={h} #{it}"), 1, h, bpp, &buf);
                }
            }
        }
    }
}

#[test]
fn u13_zero_len() {
    let mut rng = Rng::new(0x100D);
    for &(w, bpp) in &[(0i32, 4i32), (4, 0), (0, 0)] {
        for f in 0..=4u8 {
            for h in 0..=6 {
                let buf = build(&mut rng, w, h, bpp, &mut |_, _| f, &mut |r| r.u8());
                one(&format!("U13 w={w} bpp={bpp} f={f} h={h}"), w, h, bpp, &buf);
            }
        }
    }
}

#[test]
fn u14_extreme_byte_patterns() {
    let pats: [&dyn Fn(usize) -> u8; 6] = [
        &|_| 0x00,
        &|_| 0xFF,
        &|_| 0x80,
        &|i| if i % 2 == 0 { 0x00 } else { 0xFF },
        &|i| if i % 3 == 0 { 0x01 } else { 0xFE },
        &|i| (i as u8).wrapping_mul(37),
    ];
    let mut rng = Rng::new(0x100E);
    for (pi, pat) in pats.iter().enumerate() {
        for f in 0..=4u8 {
            for bpp in 1..=8 {
                for h in 1..=5 {
                    let w = 7;
                    let mut i = 0usize;
                    let buf = build(
                        &mut rng,
                        w,
                        h,
                        bpp,
                        &mut |_, _| f,
                        &mut |_| {
                            let v = pat(i);
                            i += 1;
                            v
                        },
                    );
                    one(
                        &format!("U14 pat={pi} f={f} bpp={bpp} h={h}"),
                        w,
                        h,
                        bpp,
                        &buf,
                    );
                }
            }
        }
    }
}

#[test]
fn u15_large_image() {
    let mut rng = Rng::new(0x100F);
    for bpp in [1, 2, 3, 4, 6, 8] {
        for it in 0..6 {
            let w = 64;
            let h = 48;
            let buf = build(
                &mut rng,
                w,
                h,
                bpp,
                &mut |_, r| r.below(5) as u8,
                &mut |r| r.u8(),
            );
            one(&format!("U15 bpp={bpp} #{it}"), w, h, bpp, &buf);
        }
    }
}

#[test]
fn u16_bpp_vs_len_boundaries() {
    // bpp == len (w == 1), bpp just below len (w == 2), and bpp == 1 with a wide
    // row: the three shapes the `x < bpp` / `x < len` loop pair distinguishes.
    let mut rng = Rng::new(0x1010);
    for &(w, bpp) in &[
        (1i32, 8i32),
        (2, 8),
        (1, 1),
        (2, 1),
        (64, 1),
        (3, 5),
        (5, 3),
        (17, 7),
    ] {
        for f in 0..=4u8 {
            for h in 1..=4 {
                for it in 0..8 {
                    let buf = build(&mut rng, w, h, bpp, &mut |_, _| f, &mut |r| r.u8());
                    one(
                        &format!("U16 w={w} bpp={bpp} f={f} h={h} #{it}"),
                        w,
                        h,
                        bpp,
                        &buf,
                    );
                }
            }
        }
    }
}

/// U17 — a wide randomized sweep: `w`, `h`, `bpp` and every per-row filter byte
/// (including invalid ones) drawn at random, 4000 cases.
#[test]
fn u17_wide_random_sweep() {
    let mut rng = Rng::new(0x1011);
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    for it in 0..4000 {
        let w = rng.range(0, 24);
        let bpp = rng.range(0, 8);
        let h = rng.range(0, 10);
        // 1 in 6 filter bytes is out of range
        let buf = build(
            &mut rng,
            w,
            h,
            bpp,
            &mut |_, r| {
                if r.below(6) == 0 {
                    r.range(5, 255) as u8
                } else {
                    r.below(5) as u8
                }
            },
            &mut |r| r.u8(),
        );
        // bpp > len lets filters 2/3/4 of rows >= 1 write `bpp` bytes past the
        // scanline, so give the buffer room for that.
        let mut buf = buf;
        buf.extend(std::iter::repeat(GUARD).take(16));
        let (ret, _) = one_ret(&format!("U17#{it}"), w, h, bpp, &buf);
        if ret == 0 {
            rejected += 1;
        } else {
            accepted += 1;
        }
    }
    assert!(
        rejected > 200 && accepted > 200,
        "sweep is one-sided: {accepted} accepted, {rejected} rejected"
    );
}

/// Like [`one`] but returns the (identical) return value.
#[track_caller]
fn one_ret(ctx: &str, w: i32, h: i32, bpp: i32, buf: &[u8]) -> (i32, Vec<u8>) {
    let (ret, got) = check_unfilter(ctx, w, h, bpp, buf);
    let mut refbuf = buf.to_vec();
    let rref = reference(w, h, bpp, &mut refbuf);
    assert_eq!(ret, rref, "[{ctx}] reference model return value (w={w},h={h},bpp={bpp})");
    assert_eq!(got, refbuf, "[{ctx}] reference model output (w={w},h={h},bpp={bpp})");
    (ret, got)
}

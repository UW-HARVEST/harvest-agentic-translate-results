//! CONFIGS.md rows 8–13 and ERRORS.md rows 22–23: `convert_pix`.

mod common;

use common::*;

/// Bytes `convert_pix` reads: one filter byte per row plus `w * bpp` samples.
fn src_len(bpp: i32, w: i32, h: i32) -> usize {
    if bpp <= 0 || w <= 0 || h <= 0 {
        return 64;
    }
    (h as usize) * (1 + (w as usize) * (bpp as usize)) + 64
}

fn dst_len(w: i32, h: i32) -> usize {
    if w <= 0 || h <= 0 {
        0
    } else {
        (w as usize) * (h as usize)
    }
}

const WS: [i32; 6] = [0, 1, 2, 3, 7, 16];
const HS: [i32; 5] = [0, 1, 2, 3, 5];

fn grid(bpp: i32, tag: &str) {
    let p = pair();
    let mut rng = Rng::new(SEED ^ (bpp as u64) << 32);
    for &w in WS.iter() {
        for &h in HS.iter() {
            for rep in 0..8 {
                let src = rng.bytes(src_len(bpp, w, h));
                diff_convert_pix(
                    &p,
                    bpp,
                    w,
                    h,
                    &src,
                    dst_len(w, h),
                    &format!("{tag}/w{w}h{h}r{rep}"),
                );
            }
        }
    }
}

// --- CONFIGS row 8 ---------------------------------------------------------
#[test]
fn c01_bpp1_grid() {
    grid(1, "c01");
}

// --- CONFIGS row 9 ---------------------------------------------------------
#[test]
fn c02_bpp2_grid() {
    grid(2, "c02");
}

// --- CONFIGS row 10 --------------------------------------------------------
#[test]
fn c03_bpp3_grid() {
    grid(3, "c03");
}

// --- CONFIGS row 11 --------------------------------------------------------
#[test]
fn c04_bpp4_grid() {
    grid(4, "c04");
}

// --- CONFIGS row 12 --------------------------------------------------------
#[test]
fn c05_boundary_bytes() {
    let p = pair();
    for bpp in 1..=4i32 {
        for fill in [0x00u8, 0xFF, 0x01, 0x80, 0x7F] {
            for &w in WS.iter() {
                for &h in HS.iter() {
                    let src = vec![fill; src_len(bpp, w, h)];
                    diff_convert_pix(
                        &p,
                        bpp,
                        w,
                        h,
                        &src,
                        dst_len(w, h),
                        &format!("c05/bpp{bpp}/fill{fill:02x}/w{w}h{h}"),
                    );
                }
            }
        }
    }
}

// --- CONFIGS row 13 --------------------------------------------------------
#[test]
fn c06_random_property() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xC06);
    for i in 0..1000 {
        let bpp = rng.range(1, 4) as i32;
        let w = rng.below(65) as i32;
        let h = rng.below(65) as i32;
        let src = rng.bytes(src_len(bpp, w, h));
        diff_convert_pix(&p, bpp, w, h, &src, dst_len(w, h), &format!("c06/{i}"));
    }
}

// --- ERRORS row 22 ---------------------------------------------------------
/// `bpp` outside `{1,2,3,4}`: the C `switch` has no `default`, so `dst` is never
/// written or advanced, but `src` still advances by `bpp` per pixel.
#[test]
fn err22_convert_pix_bad_bpp() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x22);
    for bpp in [0i32, 5, 6, 7, 8, 16, 255, 256, -1, -2, -8, i32::MAX, i32::MIN] {
        for &w in WS.iter() {
            for &h in HS.iter() {
                let src = rng.bytes(4096);
                // dst is untouched by these bpp values, but pass a real buffer
                diff_convert_pix(&p, bpp, w, h, &src, dst_len(w, h), &format!("err22/bpp{bpp}/w{w}h{h}"));
            }
        }
    }
}

// --- ERRORS row 23 ---------------------------------------------------------
/// Non-positive `w`/`h` (loops never run) and NULL `src`/`dst` (never
/// dereferenced when `h <= 0`).
#[test]
fn err23_convert_pix_nonpositive_and_null() {
    let p = pair();
    let f_c = p.c.convert_pix();
    let f_rs = p.rs.convert_pix();

    for bpp in [1i32, 2, 3, 4, 0, -1, i32::MAX, i32::MIN] {
        for (w, h) in [
            (0i32, 0i32),
            (0, 1),
            (1, 0),
            (-1, 5),
            (5, -1),
            (-1, -1),
            (i32::MIN, 4),
            (4, i32::MIN),
            (i32::MIN, i32::MIN),
        ] {
            // NULL src / NULL dst: with h <= 0 or w <= 0 nothing is dereferenced.
            if h <= 0 || w <= 0 {
                unsafe {
                    f_c(bpp, w, h, std::ptr::null_mut(), std::ptr::null_mut());
                    f_rs(bpp, w, h, std::ptr::null_mut(), std::ptr::null_mut());
                }
            }
            let src = vec![0x5Au8; 4096];
            diff_convert_pix(&p, bpp, w, h, &src, 16, &format!("err23/bpp{bpp}/w{w}h{h}"));
        }
    }
}

/// Boundary: `w`/`h` = 1 with every valid `bpp`, one pixel only — the smallest
/// non-empty shape, where the per-row `src++` dominates.
#[test]
fn c07_single_pixel() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x07);
    for bpp in 1..=4i32 {
        for _ in 0..64 {
            let src = rng.bytes(64);
            diff_convert_pix(&p, bpp, 1, 1, &src, 1, &format!("c07/bpp{bpp}"));
        }
    }
}

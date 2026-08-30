//! Level 1: the exported data tables and `convert_pix`.

mod common;

use common::{libs, CpPixel, Lib, Rng};
use std::ffi::c_int;

fn cmp_u8(name: &str, c: *mut u8, rs: *mut u8, n: usize) {
    let a = unsafe { std::slice::from_raw_parts(c, n) };
    let b = unsafe { std::slice::from_raw_parts(rs, n) };
    assert_eq!(a, b, "table {name} differs");
}

fn cmp_u32(name: &str, c: *mut u32, rs: *mut u32, n: usize) {
    let a = unsafe { std::slice::from_raw_parts(c, n) };
    let b = unsafe { std::slice::from_raw_parts(rs, n) };
    assert_eq!(a, b, "table {name} differs");
}

#[test]
fn exported_tables_match() {
    let l = libs();
    cmp_u8(
        "cp_fixed_table",
        l.c.cp_fixed_table,
        l.rs.cp_fixed_table,
        288 + 32,
    );
    cmp_u8(
        "cp_permutation_order",
        l.c.cp_permutation_order,
        l.rs.cp_permutation_order,
        19,
    );
    cmp_u8(
        "cp_len_extra_bits",
        l.c.cp_len_extra_bits,
        l.rs.cp_len_extra_bits,
        29 + 2,
    );
    cmp_u32("cp_len_base", l.c.cp_len_base, l.rs.cp_len_base, 29 + 2);
    cmp_u8(
        "cp_dist_extra_bits",
        l.c.cp_dist_extra_bits,
        l.rs.cp_dist_extra_bits,
        30 + 2,
    );
    cmp_u32("cp_dist_base", l.c.cp_dist_base, l.rs.cp_dist_base, 30 + 2);
}

#[test]
fn error_reason_initially_null() {
    let l = libs();
    // Freshly loaded libraries must both report a NULL reason.  (Other tests
    // hold the same mutex, so this cannot race, but they may have run first;
    // only assert equality of the two libraries in that case.)
    assert_eq!(l.c.error_reason(), l.rs.error_reason());
}

// ---------------------------------------------------------------------------
// convert_pix
// ---------------------------------------------------------------------------

const FILL: u8 = 0xCD;

fn run_convert(lib: &Lib, bpp: c_int, w: c_int, h: c_int, src: &[u8], pixels: usize) -> Vec<u8> {
    let mut src = src.to_vec();
    let mut dst = vec![CpPixel::default(); pixels + 8];
    for p in dst.iter_mut() {
        *p = CpPixel {
            r: FILL,
            g: FILL,
            b: FILL,
            a: FILL,
        };
    }
    unsafe {
        (lib.convert_pix)(bpp, w, h, src.as_mut_ptr(), dst.as_mut_ptr());
    }
    // Flatten to bytes so the comparison is literally byte-for-byte.
    let bytes = unsafe {
        std::slice::from_raw_parts(dst.as_ptr() as *const u8, std::mem::size_of_val(&dst[..]))
    };
    bytes.to_vec()
}

fn check_convert(bpp: c_int, w: c_int, h: c_int, seed: u64) {
    let l = libs();
    // The C loop reads `1 + w * bpp` bytes per row for bpp in 1..=4 and only
    // advances the cursor otherwise; size the source for the worst case.
    let per_row = 1usize + (w.max(0) as usize) * (bpp.max(0) as usize);
    let src_len = per_row * (h.max(0) as usize) + 64;
    let mut rng = Rng::new(seed);
    let src: Vec<u8> = (0..src_len).map(|_| rng.byte()).collect();
    let pixels = (w.max(0) as usize) * (h.max(0) as usize);

    let a = run_convert(&l.c, bpp, w, h, &src, pixels);
    let b = run_convert(&l.rs, bpp, w, h, &src, pixels);
    assert_eq!(
        a.len(),
        b.len(),
        "convert_pix bpp={bpp} w={w} h={h}: length"
    );
    if a != b {
        let i = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap();
        panic!(
            "convert_pix bpp={bpp} w={w} h={h}: first diff at byte {i}: C={:#04x} Rust={:#04x}",
            a[i], b[i]
        );
    }
}

#[test]
fn convert_pix_all_bpp() {
    for bpp in [1, 2, 3, 4] {
        for (w, h) in [
            (0, 0),
            (1, 1),
            (1, 5),
            (5, 1),
            (3, 3),
            (7, 4),
            (16, 16),
            (17, 9),
            (64, 3),
            (2, 64),
            (100, 7),
        ] {
            check_convert(bpp, w, h, 0x1000 + bpp as u64 * 97 + w as u64 * 13 + h as u64);
        }
    }
}

#[test]
fn convert_pix_unhandled_bpp() {
    // bpp outside 1..=4 hits the empty `default:` arm: nothing is written and
    // the source cursor still advances.
    for bpp in [0, 5, 6, 8, 16, -1, -3] {
        for (w, h) in [(0, 0), (1, 1), (4, 4), (9, 2)] {
            check_convert(bpp, w, h, 0x2000u64 ^ (bpp as i64 as u64) ^ ((w as u64) << 8) ^ h as u64);
        }
    }
}

#[test]
fn convert_pix_zero_dims() {
    for bpp in [1, 2, 3, 4] {
        check_convert(bpp, 0, 5, 0x3000 + bpp as u64);
        check_convert(bpp, 5, 0, 0x3100 + bpp as u64);
        check_convert(bpp, 0, 0, 0x3200 + bpp as u64);
        // negative extents: both loops are `< h` / `< w`, so they do nothing
        check_convert(bpp, -1, 3, 0x3300 + bpp as u64);
        check_convert(bpp, 3, -1, 0x3400 + bpp as u64);
    }
}

#[test]
fn convert_pix_grayscale_edges() {
    // exercise every byte value through the bpp==1 / bpp==2 paths
    let l = libs();
    for bpp in [1, 2] {
        let w = 256;
        let h = 2;
        let per_row = 1 + w * bpp as usize;
        let mut src = vec![0u8; per_row * h + 16];
        for y in 0..h {
            src[y * per_row] = 0; // filter byte, skipped
            for x in 0..w {
                for b in 0..bpp as usize {
                    src[y * per_row + 1 + x * bpp as usize + b] = ((x + b * 128 + y) % 256) as u8;
                }
            }
        }
        let a = run_convert(&l.c, bpp, w as c_int, h as c_int, &src, w * h);
        let b = run_convert(&l.rs, bpp, w as c_int, h as c_int, &src, w * h);
        assert_eq!(a, b, "convert_pix grayscale bpp={bpp}");
    }
}

//! Tier 2: row-level transforms and filter reconstruction.  These take only a
//! `png_row_info` (a public struct) plus raw row buffers, so they can be driven
//! directly through the exported symbols of both libraries.

mod common;
use common::*;
use std::ffi::c_int;

type FnRow = unsafe extern "C-unwind" fn(*mut png_row_info, *mut u8);

fn rowbytes(pixel_depth: u32, width: u32) -> usize {
    // PNG_ROWBYTES
    if pixel_depth >= 8 {
        (width as usize) * ((pixel_depth as usize) >> 3)
    } else {
        ((width as usize) * (pixel_depth as usize) + 7) >> 3
    }
}

/// Every (color_type, bit_depth) combination PNG allows, plus the channel and
/// pixel-depth bookkeeping libpng derives from them.
fn valid_formats() -> Vec<(u8, u8, u8)> {
    let mut v = Vec::new();
    for &(ct, chans) in &[
        (PNG_COLOR_TYPE_GRAY, 1u8),
        (PNG_COLOR_TYPE_PALETTE, 1),
        (PNG_COLOR_TYPE_RGB, 3),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 2),
        (PNG_COLOR_TYPE_RGB_ALPHA, 4),
    ] {
        for &bd in &[1u8, 2, 4, 8, 16] {
            let ok = match ct {
                PNG_COLOR_TYPE_GRAY => true,
                PNG_COLOR_TYPE_PALETTE => bd <= 8,
                _ => bd == 8 || bd == 16,
            };
            if ok {
                v.push((ct, bd, chans));
            }
        }
    }
    v
}

fn make_row(len: usize, seed: u32) -> Vec<u8> {
    let mut s = seed | 1;
    (0..len)
        .map(|_| {
            s = s.wrapping_mul(1103515245).wrapping_add(12345);
            (s >> 16) as u8
        })
        .collect()
}

fn run_pair(name: &str, f: &FnRow, g: &FnRow, ri: png_row_info, row: &[u8]) {
    // generous slack so an over-write by either side is visible
    let pad = 64;
    let mut a = row.to_vec();
    a.extend(std::iter::repeat(0x5a).take(pad));
    let mut b = a.clone();
    let mut ria = ri;
    let mut rib = ri;
    unsafe { f(&mut ria, a.as_mut_ptr()) };
    unsafe { g(&mut rib, b.as_mut_ptr()) };
    assert_eq!(ria, rib, "{name}: row_info differs for {ri:?}");
    assert_eq!(
        a,
        b,
        "{name}: row differs for {ri:?}\n C: {}\n R: {}",
        hex(&a),
        hex(&b)
    );
}

fn simple_transform(name: &str) {
    let l = libs();
    let f: libloading::Symbol<FnRow> = l.c.sym(name);
    let g: libloading::Symbol<FnRow> = l.r.sym(name);
    let mut n = 0u32;
    for (ct, bd, ch) in valid_formats() {
        let pixel_depth = (bd as u32) * (ch as u32);
        for width in [0u32, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 33] {
            let rb = rowbytes(pixel_depth, width);
            n += 1;
            let row = make_row(rb, n * 7919);
            let ri = png_row_info {
                width,
                rowbytes: rb,
                color_type: ct,
                bit_depth: bd,
                channels: ch,
                pixel_depth: pixel_depth as u8,
            };
            run_pair(name, &f, &g, ri, &row);
        }
    }
}

#[test]
fn do_bgr() {
    simple_transform("png_do_bgr");
}

#[test]
fn do_invert() {
    simple_transform("png_do_invert");
}

#[test]
fn do_packswap() {
    simple_transform("png_do_packswap");
}

#[test]
fn do_swap() {
    simple_transform("png_do_swap");
}

#[test]
fn do_strip_channel() {
    let l = libs();
    type F = unsafe extern "C-unwind" fn(*mut png_row_info, *mut u8, c_int);
    let f: libloading::Symbol<F> = l.c.sym("png_do_strip_channel");
    let g: libloading::Symbol<F> = l.r.sym("png_do_strip_channel");
    let mut n = 0u32;
    for (ct, bd, ch) in valid_formats() {
        let pixel_depth = (bd as u32) * (ch as u32);
        for width in [0u32, 1, 2, 3, 5, 8, 9, 16, 17] {
            for at_start in [0, 1] {
                let rb = rowbytes(pixel_depth, width);
                n += 1;
                let row = make_row(rb, n * 104729);
                let ri = png_row_info {
                    width,
                    rowbytes: rb,
                    color_type: ct,
                    bit_depth: bd,
                    channels: ch,
                    pixel_depth: pixel_depth as u8,
                };
                let pad = 64;
                let mut a = row.to_vec();
                a.extend(std::iter::repeat(0x5au8).take(pad));
                let mut b = a.clone();
                let (mut ria, mut rib) = (ri, ri);
                unsafe { f(&mut ria, a.as_mut_ptr(), at_start) };
                unsafe { g(&mut rib, b.as_mut_ptr(), at_start) };
                assert_eq!(ria, rib, "strip_channel info {ri:?} at_start={at_start}");
                assert_eq!(a, b, "strip_channel row {ri:?} at_start={at_start}");
            }
        }
    }
}

#[test]
fn do_write_interlace() {
    let l = libs();
    type F = unsafe extern "C-unwind" fn(*mut png_row_info, *mut u8, c_int);
    let f: libloading::Symbol<F> = l.c.sym("png_do_write_interlace");
    let g: libloading::Symbol<F> = l.r.sym("png_do_write_interlace");
    let mut n = 0u32;
    for (ct, bd, ch) in valid_formats() {
        let pixel_depth = (bd as u32) * (ch as u32);
        for width in [1u32, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 32, 33] {
            for pass in 0..7 {
                let rb = rowbytes(pixel_depth, width);
                n += 1;
                let row = make_row(rb, n * 65537);
                let ri = png_row_info {
                    width,
                    rowbytes: rb,
                    color_type: ct,
                    bit_depth: bd,
                    channels: ch,
                    pixel_depth: pixel_depth as u8,
                };
                let pad = 64;
                let mut a = row.to_vec();
                a.extend(std::iter::repeat(0x5au8).take(pad));
                let mut b = a.clone();
                let (mut ria, mut rib) = (ri, ri);
                unsafe { f(&mut ria, a.as_mut_ptr(), pass) };
                unsafe { g(&mut rib, b.as_mut_ptr(), pass) };
                assert_eq!(ria, rib, "write_interlace info {ri:?} pass={pass}");
                assert_eq!(a, b, "write_interlace row {ri:?} pass={pass}");
            }
        }
    }
}

#[test]
fn do_read_interlace() {
    let l = libs();
    type F = unsafe extern "C-unwind" fn(*mut png_row_info, *mut u8, c_int, u32);
    let f: libloading::Symbol<F> = l.c.sym("png_do_read_interlace");
    let g: libloading::Symbol<F> = l.r.sym("png_do_read_interlace");
    // PNG_FLAG_ROW_INIT etc are irrelevant; the only flag read is
    // PNG_INTERLACE (0x0100 in the transformations word) - exercise both.
    const PNG_INTERLACE: u32 = 0x0100;
    let mut n = 0u32;
    for (ct, bd, ch) in valid_formats() {
        let pixel_depth = (bd as u32) * (ch as u32);
        // row_info.width is the number of pixels actually present in the pass
        for final_width in [1u32, 2, 3, 4, 5, 8, 9, 16, 17, 32] {
            for pass in 0..7 {
                // number of pixels in this pass for a row of final_width
                let png_pass_start = [0u32, 4, 0, 2, 0, 1, 0];
                let png_pass_inc = [8u32, 8, 4, 4, 2, 2, 1];
                let start = png_pass_start[pass as usize];
                let inc = png_pass_inc[pass as usize];
                if final_width <= start {
                    continue;
                }
                let pass_width = (final_width - start + inc - 1) / inc;
                // the buffer must be large enough for the expanded row
                let out_rb = rowbytes(pixel_depth, final_width);
                let in_rb = rowbytes(pixel_depth, pass_width);
                n += 1;
                let mut row = make_row(out_rb.max(in_rb) + 16, n * 2654435761);
                row.truncate(out_rb.max(in_rb) + 16);
                let ri = png_row_info {
                    width: pass_width,
                    rowbytes: in_rb,
                    color_type: ct,
                    bit_depth: bd,
                    channels: ch,
                    pixel_depth: pixel_depth as u8,
                };
                for transformations in [0u32, PNG_INTERLACE] {
                    let mut a = row.clone();
                    a.extend(std::iter::repeat(0x5au8).take(64));
                    let mut b = a.clone();
                    let (mut ria, mut rib) = (ri, ri);
                    unsafe { f(&mut ria, a.as_mut_ptr(), pass, transformations) };
                    unsafe { g(&mut rib, b.as_mut_ptr(), pass, transformations) };
                    assert_eq!(
                        ria, rib,
                        "read_interlace info {ri:?} pass={pass} tr={transformations:#x}"
                    );
                    assert_eq!(
                        a, b,
                        "read_interlace row {ri:?} pass={pass} tr={transformations:#x}"
                    );
                }
            }
        }
    }
}

//! Write-pipeline differential tests (CONFIGS.md rows W1..W12).
//!
//! Every test drives both the C reference `libpng.so` and the translated Rust
//! `liblibpng.so` through a complete write cycle and compares the produced PNG
//! byte stream (`Trace::out`) together with the whole event trace.
//!
//! All pixel data is produced by a deterministic PRNG seeded with a literal and
//! is generated *outside* the `diff` closure, so both libraries see byte
//! identical input.
mod support;

use std::ffi::c_int;
use support::core::*;
use support::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// The 15 legal (colour_type, bit_depth) combinations.
const ALL_COMBOS: &[(c_int, c_int)] = &[
    (PNG_COLOR_TYPE_GRAY, 1),
    (PNG_COLOR_TYPE_GRAY, 2),
    (PNG_COLOR_TYPE_GRAY, 4),
    (PNG_COLOR_TYPE_GRAY, 8),
    (PNG_COLOR_TYPE_GRAY, 16),
    (PNG_COLOR_TYPE_RGB, 8),
    (PNG_COLOR_TYPE_RGB, 16),
    (PNG_COLOR_TYPE_PALETTE, 1),
    (PNG_COLOR_TYPE_PALETTE, 2),
    (PNG_COLOR_TYPE_PALETTE, 4),
    (PNG_COLOR_TYPE_PALETTE, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
];

fn rb(ct: c_int, bd: c_int, w: u32) -> usize {
    pngbuild::rowbytes(ct as u8, bd as u8, w)
}

/// Store a (possibly sub-byte) palette index, MSB-first as PNG requires.
fn set_index(row: &mut [u8], x: usize, bd: c_int, idx: u8) {
    if bd == 8 {
        row[x] = idx;
        return;
    }
    let per = (8 / bd) as usize; // pixels per byte
    let byte = x / per;
    let shift = 8 - bd as usize * (x % per + 1);
    let mask = ((1u16 << bd) - 1) as u8;
    row[byte] = (row[byte] & !(mask << shift)) | ((idx & mask) << shift);
}

/// `h` rows of pseudo-random content for the given image shape.  Palette rows
/// only ever contain indices `< npal`.  Each row has 8 bytes of slack.
fn make_rows(rng: &mut Rng, ct: c_int, bd: c_int, w: u32, h: u32, npal: u32) -> Vec<Vec<u8>> {
    let n = rb(ct, bd, w);
    (0..h)
        .map(|_| {
            let mut row = vec![0u8; n + 8];
            if ct == PNG_COLOR_TYPE_PALETTE {
                for x in 0..w as usize {
                    let idx = rng.below(npal) as u8;
                    set_index(&mut row, x, bd, idx);
                }
            } else {
                for i in 0..n {
                    row[i] = rng.byte();
                }
            }
            row
        })
        .collect()
}

/// Rows of a fixed byte length (used where a write transform changes the number
/// of user-supplied channels or the user bit depth).
fn make_flat_rows(rng: &mut Rng, bytes: usize, h: u32) -> Vec<Vec<u8>> {
    (0..h)
        .map(|_| {
            let mut row = vec![0u8; bytes + 8];
            for i in 0..bytes {
                row[i] = rng.byte();
            }
            row
        })
        .collect()
}

/// A compressible 8-bit RGB image: linear gradients plus a little noise.
fn gradient_rows(rng: &mut Rng, w: u32, h: u32) -> Vec<Vec<u8>> {
    let n = 3 * w as usize;
    (0..h)
        .map(|y| {
            let mut row = vec![0u8; n + 8];
            for x in 0..w as usize {
                let base = (x * 9 + y as usize * 5) as u8;
                row[3 * x] = base.wrapping_add(rng.byte() & 0x0f);
                row[3 * x + 1] = base.wrapping_mul(2).wrapping_add(rng.byte() & 0x07);
                row[3 * x + 2] = base.wrapping_add(0x40) ^ (rng.byte() & 0x03);
            }
            row
        })
        .collect()
}

fn ptr_vec(rows: &mut [Vec<u8>]) -> Vec<*mut u8> {
    rows.iter_mut().map(|r| r.as_mut_ptr()).collect()
}

/// CRC over every row buffer: libpng must not modify the caller's rows.
fn rowsum(rows: &[Vec<u8>]) -> u32 {
    let mut all = Vec::new();
    for r in rows {
        all.extend_from_slice(r);
    }
    pngbuild::crc32(&all)
}

/// Sanity guard so a configuration can never silently do nothing: the driver
/// must have run to completion and emitted a PNG datastream.
fn checked(t: Trace) -> Trace {
    assert_eq!(t.rc, 0, "unexpected longjmp out of the write driver");
    assert!(
        t.out.starts_with(&pngbuild::SIG) && t.out.len() > 8,
        "no PNG datastream produced (out.len={})",
        t.out.len()
    );
    t
}

/// `with_write` plus the sanity guard.
fn wwrite(lib: &Lib, body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    checked(with_write(lib, body))
}

unsafe fn log_hdr(c: &Core, png: Png, info: Info) {
    log(format!(
        "rowbytes={} channels={} cbuf={}",
        (c.get_rowbytes)(png, info),
        (c.get_channels)(png, info),
        (c.get_compression_buffer_size)(png)
    ));
}

unsafe fn maybe_plte(c: &Core, png: Png, info: Info, ct: c_int, pal: &[u8], npal: u32) {
    if ct == PNG_COLOR_TYPE_PALETTE {
        (c.set_PLTE)(png, info, pal.as_ptr(), npal as c_int);
    }
}

/// A 256-entry random palette; only the first `3*npal` bytes are ever used.
fn full_palette(rng: &mut Rng) -> Vec<u8> {
    rng.bytes(3 * 256)
}

/// One complete `png_write_row` cycle per repetition, compared between the two
/// libraries.  Handles Adam7 (`png_set_interlace_handling` + `height` rows per
/// pass) and the palette chunk, and optionally sets a row filter.
#[allow(clippy::too_many_arguments)]
fn row_case(
    tag: &str,
    ct: c_int,
    bd: c_int,
    w: u32,
    h: u32,
    il: c_int,
    filter: Option<(&str, c_int)>,
    reps: u32,
    rng: &mut Rng,
) {
    let npal = 1u32 << bd;
    for rep in 0..reps {
        let pal = full_palette(rng);
        let rows = make_rows(rng, ct, bd, w, h, npal);
        let fdesc = filter.map(|(n, _)| n).unwrap_or("-");
        let label = format!("{tag} ct={ct} bd={bd} w={w} h={h} il={il} f={fdesc} rep={rep}");
        diff(&label, |lib| {
            wwrite(lib, &mut |c, png, info| unsafe {
                (c.set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
                maybe_plte(c, png, info, ct, &pal, npal);
                if let Some((_, f)) = filter {
                    (c.set_filter)(png, PNG_FILTER_TYPE_BASE, f);
                }
                log_hdr(c, png, info);
                (c.write_info)(png, info);
                let passes = if il == PNG_INTERLACE_ADAM7 {
                    let p = (c.set_interlace_handling)(png);
                    log(format!("passes={p}"));
                    p
                } else {
                    1
                };
                for _pass in 0..passes {
                    for r in &rows {
                        (c.write_row)(png, r.as_ptr());
                    }
                }
                (c.write_end)(png, info);
                log(format!("rowsum={:08x}", rowsum(&rows)));
            })
        });
    }
}

// ---------------------------------------------------------------------------
// W1 — png_write_row, GRAY 1/2/4/8/16, interlace NONE
// ---------------------------------------------------------------------------

#[test]
fn w1_write_row_gray() {
    let mut rng = Rng::new(0x1101);
    let mut widths: Vec<u32> = (1..=17).collect();
    widths.push(33);
    for &bd in &[1, 2, 4, 8, 16] {
        for &w in &widths {
            for h in 1..=4u32 {
                row_case(
                    "W1",
                    PNG_COLOR_TYPE_GRAY,
                    bd,
                    w,
                    h,
                    PNG_INTERLACE_NONE,
                    None,
                    2,
                    &mut rng,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W2 — png_write_row, RGB 8/16
// ---------------------------------------------------------------------------

#[test]
fn w2_write_row_rgb() {
    let mut rng = Rng::new(0x1202);
    for &bd in &[8, 16] {
        for w in 1..=9u32 {
            for h in 1..=3u32 {
                row_case(
                    "W2",
                    PNG_COLOR_TYPE_RGB,
                    bd,
                    w,
                    h,
                    PNG_INTERLACE_NONE,
                    None,
                    3,
                    &mut rng,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W3 — png_write_row, PALETTE 1/2/4/8 with random palette (+ optional tRNS)
// ---------------------------------------------------------------------------

#[test]
fn w3_write_row_palette() {
    let mut rng = Rng::new(0x1303);
    for &bd in &[1, 2, 4, 8] {
        let full = 1u32 << bd;
        // Full palette (2**bd entries) and a shorter one; indices always stay
        // below num_palette (out-of-range indices are a Phase C row).
        for &npal in &[full, std::cmp::max(1, full * 3 / 4)] {
            for &w in &[1u32, 2, 3, 7, 17] {
                for &h in &[1u32, 3] {
                    for tv in 0..3 {
                        for rep in 0..2 {
                            let pal = full_palette(&mut rng);
                            let trns: Vec<u8> = match tv {
                                0 => Vec::new(),
                                1 => rng.bytes(npal as usize),
                                _ => rng.bytes(std::cmp::max(1, npal / 2) as usize),
                            };
                            let rows =
                                make_rows(&mut rng, PNG_COLOR_TYPE_PALETTE, bd, w, h, npal);
                            let label = format!(
                                "W3 bd={bd} npal={npal} w={w} h={h} ntrns={} rep={rep}",
                                trns.len()
                            );
                            diff(&label, |lib| {
                                wwrite(lib, &mut |c, png, info| unsafe {
                                    (c.set_IHDR)(
                                        png,
                                        info,
                                        w,
                                        h,
                                        bd,
                                        PNG_COLOR_TYPE_PALETTE,
                                        PNG_INTERLACE_NONE,
                                        0,
                                        0,
                                    );
                                    (c.set_PLTE)(png, info, pal.as_ptr(), npal as c_int);
                                    if !trns.is_empty() {
                                        (c.set_tRNS)(
                                            png,
                                            info,
                                            trns.as_ptr(),
                                            trns.len() as c_int,
                                            std::ptr::null(),
                                        );
                                    }
                                    log_hdr(c, png, info);
                                    log(format!(
                                        "valid.PLTE={} valid.tRNS={}",
                                        (c.get_valid)(png, info, PNG_INFO_PLTE),
                                        (c.get_valid)(png, info, PNG_INFO_tRNS)
                                    ));
                                    (c.write_info)(png, info);
                                    for r in &rows {
                                        (c.write_row)(png, r.as_ptr());
                                    }
                                    (c.write_end)(png, info);
                                    log(format!("rowsum={:08x}", rowsum(&rows)));
                                })
                            });
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W4 — png_write_row, GRAY_ALPHA 8/16
// ---------------------------------------------------------------------------

#[test]
fn w4_write_row_gray_alpha() {
    let mut rng = Rng::new(0x1404);
    for &bd in &[8, 16] {
        for w in 1..=9u32 {
            for h in 1..=3u32 {
                row_case(
                    "W4",
                    PNG_COLOR_TYPE_GRAY_ALPHA,
                    bd,
                    w,
                    h,
                    PNG_INTERLACE_NONE,
                    None,
                    3,
                    &mut rng,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W5 — png_write_row, RGB_ALPHA 8/16
// ---------------------------------------------------------------------------

#[test]
fn w5_write_row_rgb_alpha() {
    let mut rng = Rng::new(0x1505);
    for &bd in &[8, 16] {
        for w in 1..=9u32 {
            for h in 1..=3u32 {
                row_case(
                    "W5",
                    PNG_COLOR_TYPE_RGB_ALPHA,
                    bd,
                    w,
                    h,
                    PNG_INTERLACE_NONE,
                    None,
                    3,
                    &mut rng,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W6 — png_write_row, all 15 combos, interlace ADAM7
// ---------------------------------------------------------------------------

#[test]
fn w6_write_row_adam7() {
    let mut rng = Rng::new(0x1606);
    for &(ct, bd) in ALL_COMBOS {
        for &w in &[1u32, 2, 5, 9, 17] {
            for &h in &[1u32, 2, 5, 9] {
                row_case(
                    "W6",
                    ct,
                    bd,
                    w,
                    h,
                    PNG_INTERLACE_ADAM7,
                    None,
                    2,
                    &mut rng,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W7 — png_write_rows with num_rows = 1 / height / mixed, interlace 0 and 1
// ---------------------------------------------------------------------------

#[test]
fn w7_write_rows() {
    let mut rng = Rng::new(0x1707);
    let w = 7u32;
    let h = 5u32;
    for &(ct, bd) in ALL_COMBOS {
        let npal = 1u32 << bd;
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for mode in 0..3 {
                for rep in 0..2 {
                    let pal = full_palette(&mut rng);
                    let mut rows = make_rows(&mut rng, ct, bd, w, h, npal);
                    let mut ptrs = ptr_vec(&mut rows);
                    let label = format!("W7 ct={ct} bd={bd} il={il} mode={mode} rep={rep}");
                    diff(&label, |lib| {
                        wwrite(lib, &mut |c, png, info| unsafe {
                            (c.set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
                            maybe_plte(c, png, info, ct, &pal, npal);
                            log_hdr(c, png, info);
                            (c.write_info)(png, info);
                            let passes = if il == PNG_INTERLACE_ADAM7 {
                                let p = (c.set_interlace_handling)(png);
                                log(format!("passes={p}"));
                                p
                            } else {
                                1
                            };
                            let base = ptrs.as_mut_ptr();
                            for _pass in 0..passes {
                                match mode {
                                    // one row per call
                                    0 => {
                                        for i in 0..h {
                                            (c.write_rows)(png, base.add(i as usize), 1);
                                        }
                                    }
                                    // the whole image in one call
                                    1 => (c.write_rows)(png, base, h),
                                    // 2 rows, then the rest
                                    _ => {
                                        (c.write_rows)(png, base, 2);
                                        (c.write_rows)(png, base.add(2), h - 2);
                                    }
                                }
                            }
                            (c.write_end)(png, info);
                            log(format!("rowsum={:08x}", rowsum(&rows)));
                        })
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W8 — png_write_image, all combos × interlace 0/1
// ---------------------------------------------------------------------------

#[test]
fn w8_write_image() {
    let mut rng = Rng::new(0x1808);
    for &(ct, bd) in ALL_COMBOS {
        let npal = 1u32 << bd;
        for &(w, h) in &[(1u32, 1u32), (2, 3), (9, 6), (17, 9)] {
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                for rep in 0..2 {
                    let pal = full_palette(&mut rng);
                    let mut rows = make_rows(&mut rng, ct, bd, w, h, npal);
                    let mut ptrs = ptr_vec(&mut rows);
                    let label = format!("W8 ct={ct} bd={bd} w={w} h={h} il={il} rep={rep}");
                    diff(&label, |lib| {
                        wwrite(lib, &mut |c, png, info| unsafe {
                            (c.set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
                            maybe_plte(c, png, info, ct, &pal, npal);
                            log_hdr(c, png, info);
                            (c.write_info)(png, info);
                            (c.write_image)(png, ptrs.as_mut_ptr());
                            (c.write_end)(png, info);
                            log(format!("rowsum={:08x}", rowsum(&rows)));
                        })
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W9 — png_write_png with png_set_rows and the write transform bits
// ---------------------------------------------------------------------------

/// The colour types each bit that `png_write_png` acts on is legal for on the
/// write side (see `png_write_png` in `c_src/src/pngwrite.c` and the
/// `png_set_*` implementations in `c_src/src/pngtrans.c`).
fn w9_bit_combos(bit: c_int) -> Vec<(c_int, c_int)> {
    let gray_low = vec![
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 2),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_PALETTE, 1),
        (PNG_COLOR_TYPE_PALETTE, 2),
        (PNG_COLOR_TYPE_PALETTE, 4),
    ];
    let alpha = vec![
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ];
    let colour = vec![
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ];
    // png_set_filler on write only accepts RGB (usr_channels 4) and GRAY with
    // bit depth >= 8 (usr_channels 2); anything else is an app error.
    let filler = vec![
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
    ];
    let d16 = vec![
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ];
    match bit {
        PNG_TRANSFORM_IDENTITY => ALL_COMBOS.to_vec(),
        PNG_TRANSFORM_PACKING | PNG_TRANSFORM_PACKSWAP => gray_low,
        PNG_TRANSFORM_INVERT_MONO => vec![
            (PNG_COLOR_TYPE_GRAY, 1),
            (PNG_COLOR_TYPE_GRAY, 2),
            (PNG_COLOR_TYPE_GRAY, 4),
            (PNG_COLOR_TYPE_GRAY, 8),
            (PNG_COLOR_TYPE_GRAY, 16),
            (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
            (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
        ],
        PNG_TRANSFORM_SHIFT => ALL_COMBOS.to_vec(),
        PNG_TRANSFORM_BGR => colour,
        PNG_TRANSFORM_SWAP_ALPHA | PNG_TRANSFORM_INVERT_ALPHA => alpha,
        PNG_TRANSFORM_SWAP_ENDIAN => d16,
        PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER => filler,
        _ => Vec::new(),
    }
}

const W9_BITS: &[(&str, c_int)] = &[
    ("IDENTITY", PNG_TRANSFORM_IDENTITY),
    ("PACKING", PNG_TRANSFORM_PACKING),
    ("PACKSWAP", PNG_TRANSFORM_PACKSWAP),
    ("INVERT_MONO", PNG_TRANSFORM_INVERT_MONO),
    ("SHIFT", PNG_TRANSFORM_SHIFT),
    ("BGR", PNG_TRANSFORM_BGR),
    ("SWAP_ALPHA", PNG_TRANSFORM_SWAP_ALPHA),
    ("SWAP_ENDIAN", PNG_TRANSFORM_SWAP_ENDIAN),
    ("INVERT_ALPHA", PNG_TRANSFORM_INVERT_ALPHA),
    ("STRIP_FILLER_BEFORE", PNG_TRANSFORM_STRIP_FILLER_BEFORE),
    ("STRIP_FILLER_AFTER", PNG_TRANSFORM_STRIP_FILLER_AFTER),
];

/// Every write transform bit that is legal for `(ct, bd)`.
fn w9_legal_bits(ct: c_int, bd: c_int) -> Vec<c_int> {
    W9_BITS
        .iter()
        .filter(|(_, b)| *b != PNG_TRANSFORM_IDENTITY)
        .filter(|(_, b)| w9_bit_combos(*b).contains(&(ct, bd)))
        .map(|(_, b)| *b)
        .collect()
}

fn w9_run(label: &str, ct: c_int, bd: c_int, il: c_int, transforms: c_int, rng: &mut Rng) {
    let w = 9u32;
    let h = 6u32;
    let npal = 1u32 << bd;
    let pal = full_palette(rng);
    // Widest row any write transform can ask for: 4 channels x 16 bit.
    let mut rows = make_flat_rows(rng, w as usize * 8, h);
    if ct == PNG_COLOR_TYPE_PALETTE {
        for row in rows.iter_mut() {
            // Packed indices (no PNG_TRANSFORM_PACKING) ...
            for x in 0..w as usize {
                let idx = rng.below(npal) as u8;
                set_index(row, x, bd, idx);
            }
            // ... and, for the low bit depths, one legal index per byte so the
            // same buffer is also valid under PNG_TRANSFORM_PACKING.
            if bd < 8 && (transforms & PNG_TRANSFORM_PACKING) != 0 {
                for x in 0..w as usize {
                    row[x] = rng.below(npal) as u8;
                }
            }
        }
    }
    let sb = std::cmp::max(1, bd - 1) as u8;
    let sbit = PngColor8 {
        red: sb,
        green: sb,
        blue: sb,
        gray: sb,
        alpha: sb,
    };
    let mut ptrs = ptr_vec(&mut rows);
    diff(label, |lib| {
        wwrite(lib, &mut |c, png, info| unsafe {
            (c.set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
            maybe_plte(c, png, info, ct, &pal, npal);
            // png_write_png only honours PNG_TRANSFORM_SHIFT when sBIT is valid.
            if transforms & PNG_TRANSFORM_SHIFT != 0 {
                (c.set_sBIT)(png, info, &sbit as *const PngColor8 as *const u8);
            }
            (c.set_rows)(png, info, ptrs.as_mut_ptr());
            log(format!(
                "valid.IDAT={} rows_set={} valid.sBIT={}",
                (c.get_valid)(png, info, PNG_INFO_IDAT),
                if (c.get_rows)(png, info).is_null() { 0 } else { 1 },
                (c.get_valid)(png, info, PNG_INFO_sBIT)
            ));
            log_hdr(c, png, info);
            (c.write_png)(png, info, transforms, std::ptr::null_mut());
            log_hdr(c, png, info);
            log(format!("rowsum={:08x}", rowsum(&rows)));
        })
    });
}

#[test]
fn w9_write_png_transforms() {
    let mut rng = Rng::new(0x1909);

    // (a) every honoured bit on its own, on every colour type it applies to.
    for &(name, bit) in W9_BITS {
        for (ct, bd) in w9_bit_combos(bit) {
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                for rep in 0..2 {
                    let label = format!("W9 {name} ct={ct} bd={bd} il={il} rep={rep}");
                    w9_run(&label, ct, bd, il, bit, &mut rng);
                }
            }
        }
    }

    // (b) six seeded random combinations of the legal bits per colour type.
    for &(ct, bd) in ALL_COMBOS {
        let legal = w9_legal_bits(ct, bd);
        for k in 0..6 {
            let mut mask = 0;
            for &b in &legal {
                if rng.next_u32() & 1 != 0 {
                    mask |= b;
                }
            }
            // BEFORE+AFTER together is an app error (a Phase C row).
            if mask & PNG_TRANSFORM_STRIP_FILLER_AFTER != 0 {
                mask &= !PNG_TRANSFORM_STRIP_FILLER_BEFORE;
            }
            let il = if rng.next_u32() & 1 != 0 {
                PNG_INTERLACE_ADAM7
            } else {
                PNG_INTERLACE_NONE
            };
            let label = format!("W9 rand{k} ct={ct} bd={bd} il={il} mask={mask:#06x}");
            w9_run(&label, ct, bd, il, mask, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// W10 — png_set_filter, exercising png_write_find_filter's heuristic
// ---------------------------------------------------------------------------

#[test]
fn w10_set_filter() {
    let mut rng = Rng::new(0x2010);
    let filters: &[(&str, c_int)] = &[
        ("NO_FILTERS", PNG_NO_FILTERS),
        ("NONE", PNG_FILTER_NONE),
        ("SUB", PNG_FILTER_SUB),
        ("UP", PNG_FILTER_UP),
        ("AVG", PNG_FILTER_AVG),
        ("PAETH", PNG_FILTER_PAETH),
        ("ALL", PNG_ALL_FILTERS),
        ("NONE|SUB", PNG_FILTER_NONE | PNG_FILTER_SUB),
        ("SUB|PAETH", PNG_FILTER_SUB | PNG_FILTER_PAETH),
        (
            "UP|AVG|PAETH",
            PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH,
        ),
    ];
    let combos: &[(c_int, c_int)] = &[
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ];
    // 13 x 9 gives the heuristic >= 8 rows to choose different filters from.
    for &(ct, bd) in combos {
        for &f in filters {
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                row_case("W10", ct, bd, 13, 9, il, Some(f), 2, &mut rng);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W11 — compression level × strategy
// ---------------------------------------------------------------------------

#[test]
fn w11_compression_level_strategy() {
    let mut rng = Rng::new(0x2111);
    let w = 16u32;
    let h = 20u32;
    let images: Vec<Vec<Vec<u8>>> = (0..2).map(|_| gradient_rows(&mut rng, w, h)).collect();
    for (n, rows) in images.iter().enumerate() {
        for &level in &[0, 1, 6, 9] {
            for strategy in 0..=4 {
                let label = format!("W11 img={n} level={level} strategy={strategy}");
                diff(&label, |lib| {
                    wwrite(lib, &mut |c, png, info| unsafe {
                        (c.set_IHDR)(
                            png,
                            info,
                            w,
                            h,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            PNG_INTERLACE_NONE,
                            0,
                            0,
                        );
                        (c.set_compression_level)(png, level);
                        (c.set_compression_strategy)(png, strategy);
                        log_hdr(c, png, info);
                        (c.write_info)(png, info);
                        for r in rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                        (c.write_end)(png, info);
                        log(format!("rowsum={:08x}", rowsum(rows)));
                    })
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W12 — compression window bits × mem level
// ---------------------------------------------------------------------------

#[test]
fn w12_compression_window_mem() {
    let mut rng = Rng::new(0x2212);
    let w = 16u32;
    let h = 20u32;
    let images: Vec<Vec<Vec<u8>>> = (0..2).map(|_| gradient_rows(&mut rng, w, h)).collect();
    for (n, rows) in images.iter().enumerate() {
        // 7 and 16 are out of range: the C clamps them and emits a warning that
        // must be reproduced verbatim.
        for &wb in &[8, 9, 10, 15, 7, 16] {
            for &ml in &[1, 4, 8, 9] {
                let label = format!("W12 img={n} window_bits={wb} mem_level={ml}");
                diff(&label, |lib| {
                    let t = wwrite(lib, &mut |c, png, info| unsafe {
                        (c.set_IHDR)(
                            png,
                            info,
                            w,
                            h,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            PNG_INTERLACE_NONE,
                            0,
                            0,
                        );
                        (c.set_compression_window_bits)(png, wb);
                        (c.set_compression_mem_level)(png, ml);
                        log_hdr(c, png, info);
                        (c.write_info)(png, info);
                        for r in rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                        (c.write_end)(png, info);
                        log(format!("rowsum={:08x}", rowsum(rows)));
                    });
                    // The clamp warning must be produced by both libraries.
                    let warned = t.lines.iter().any(|l| l.starts_with("WARNING("));
                    assert_eq!(
                        warned,
                        !(8..=15).contains(&wb),
                        "[{}] window_bits={wb}: warned={warned}",
                        lib.tag
                    );
                    t
                });
            }
        }
    }
}

//! Phase B, group W: the write path driven row-at-a-time through the exported
//! symbols of both `.so`s, over the full cross-product of image shapes and
//! write options that the C code branches on.
mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

const SEED: u64 = 0x77ee_0011_2233_4455;

/// The 15 legal (colour type, bit depth) pairs.
const LEGAL: &[(c_int, c_int)] = &[
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (2, 8),
    (2, 16),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

fn pal_for(bd: c_int) -> usize {
    match bd {
        1 => 2,
        2 => 4,
        4 => 16,
        _ => 256,
    }
}

// ---------------------------------------------------------------------------
// W1..W16: every legal (colour type, bit depth) × interlace × several widths
// ---------------------------------------------------------------------------
#[test]
fn w1_w16_all_legal_shapes() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        for interlace in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for (wi, &(w, h)) in [(1u32, 1u32), (7, 3), (8, 8), (9, 5), (33, 4), (17, 17)]
                .iter()
                .enumerate()
            {
                let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                    make_palette(pal_for(bd), SEED ^ 0x501)
                } else {
                    vec![]
                };
                let rb = rowbytes(w, bd, ct);
                let seed = SEED ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ (w as u64) ^ ((interlace as u64) << 20);
                let mut run = |l: &Lib| -> Report {
                    write_full(
                        l,
                        w,
                        h,
                        ct,
                        bd,
                        interlace,
                        PNG_FILTER_TYPE_BASE,
                        &pal,
                        rb,
                        seed,
                        &mut no_setup,
                    )
                };
                diff(
                    &format!("W1-W16 ct={ct} bd={bd} il={interlace} {w}x{h} (#{wi})"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W17 png_write_rows / W18 png_write_image
// ---------------------------------------------------------------------------
#[test]
fn w17_w18_write_rows_and_image() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        for interlace in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let (w, h) = (13u32, 6u32);
            let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                make_palette(pal_for(bd), SEED ^ 0x502)
            } else {
                vec![]
            };
            let rb = rowbytes(w, bd, ct);
            let seed = SEED ^ 0x1717 ^ ((ct as u64) << 8) ^ (bd as u64) ^ ((interlace as u64) << 16);
            let rows = make_rows(h as usize, rb, seed);

            // png_write_image
            let mut run = |l: &Lib| -> Report {
                let rows = rows.clone();
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, w, h, bd, ct, interlace, PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    if !pal.is_empty() {
                        (l.api.png_set_PLTE)(png, info, pal.as_ptr(), pal.len() as c_int);
                    }
                    (l.api.png_write_info)(png, info);
                    let mut ptrs: Vec<*mut u8> =
                        rows.iter().map(|v| v.as_ptr() as *mut u8).collect();
                    (l.api.png_write_image)(png, ptrs.as_mut_ptr());
                    (l.api.png_write_end)(png, info);
                })
            };
            diff(
                &format!("W18 png_write_image ct={ct} bd={bd} il={interlace}"),
                &c,
                &r,
                &mut run,
            );

            // png_write_rows in batches
            for batch in [1usize, 2, 6] {
                let mut run = |l: &Lib| -> Report {
                    let rows = rows.clone();
                    write_session(l, &mut |l, png, info| unsafe {
                        (l.api.png_set_IHDR)(
                            png, info, w, h, bd, ct, interlace, PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        if !pal.is_empty() {
                            (l.api.png_set_PLTE)(png, info, pal.as_ptr(), pal.len() as c_int);
                        }
                        (l.api.png_write_info)(png, info);
                        let passes = if interlace == PNG_INTERLACE_ADAM7 {
                            (l.api.png_set_interlace_handling)(png)
                        } else {
                            1
                        };
                        let mut ptrs: Vec<*mut u8> =
                            rows.iter().map(|v| v.as_ptr() as *mut u8).collect();
                        for _ in 0..passes {
                            let mut i = 0usize;
                            while i < ptrs.len() {
                                let n = batch.min(ptrs.len() - i);
                                (l.api.png_write_rows)(png, ptrs[i..].as_mut_ptr(), n as u32);
                                i += n;
                            }
                        }
                        (l.api.png_write_end)(png, info);
                    })
                };
                diff(
                    &format!("W17 png_write_rows ct={ct} bd={bd} il={interlace} batch={batch}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W19 filter selection
// ---------------------------------------------------------------------------
#[test]
fn w19_filters() {
    let (c, r) = libs();
    let masks: &[(&str, c_int)] = &[
        ("NO_FILTERS", PNG_NO_FILTERS),
        ("NONE", PNG_FILTER_NONE),
        ("SUB", PNG_FILTER_SUB),
        ("UP", PNG_FILTER_UP),
        ("AVG", PNG_FILTER_AVG),
        ("PAETH", PNG_FILTER_PAETH),
        ("FAST", PNG_FILTER_NONE | PNG_FILTER_SUB | PNG_FILTER_UP),
        ("ALL", PNG_ALL_FILTERS),
        ("SUB|PAETH", PNG_FILTER_SUB | PNG_FILTER_PAETH),
        ("AVG|UP", PNG_FILTER_AVG | PNG_FILTER_UP),
    ];
    for &(ct, bd) in LEGAL {
        for &(name, mask) in masks {
            let (w, h) = (23u32, 9u32);
            let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                make_palette(pal_for(bd), SEED ^ 0x503)
            } else {
                vec![]
            };
            let rb = rowbytes(w, bd, ct);
            let seed = SEED ^ 0x1900 ^ (mask as u64) ^ ((ct as u64) << 12) ^ ((bd as u64) << 20);
            let mut run = |l: &Lib| -> Report {
                write_full(
                    l, w, h, ct, bd, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &pal, rb, seed,
                    &mut |l, png, _info| unsafe {
                        (l.api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, mask);
                    },
                )
            };
            diff(
                &format!("W19 png_set_filter {name} ct={ct} bd={bd}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// W20..W25 zlib parameters
// ---------------------------------------------------------------------------
#[test]
fn w20_w25_zlib_parameters() {
    let (c, r) = libs();
    let (w, h) = (41u32, 12u32);
    let rb = rowbytes(w, 8, PNG_COLOR_TYPE_RGB);

    for level in [0i32, 1, 2, 3, 6, 9] {
        let mut run = |l: &Lib| -> Report {
            write_full(
                l, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb,
                SEED ^ 0x2000, &mut |l, png, _info| unsafe {
                    (l.api.png_set_compression_level)(png, level);
                },
            )
        };
        diff(&format!("W20 compression_level={level}"), &c, &r, &mut run);
    }
    for strategy in [0i32, 1, 2, 3, 4] {
        let mut run = |l: &Lib| -> Report {
            write_full(
                l, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb,
                SEED ^ 0x2100, &mut |l, png, _info| unsafe {
                    (l.api.png_set_compression_strategy)(png, strategy);
                },
            )
        };
        diff(&format!("W21 compression_strategy={strategy}"), &c, &r, &mut run);
    }
    for wb in [8i32, 9, 10, 11, 12, 13, 14, 15] {
        let mut run = |l: &Lib| -> Report {
            write_full(
                l, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb,
                SEED ^ 0x2200, &mut |l, png, _info| unsafe {
                    (l.api.png_set_compression_window_bits)(png, wb);
                },
            )
        };
        diff(&format!("W22 compression_window_bits={wb}"), &c, &r, &mut run);
    }
    for ml in [1i32, 3, 5, 8, 9] {
        let mut run = |l: &Lib| -> Report {
            write_full(
                l, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb,
                SEED ^ 0x2300, &mut |l, png, _info| unsafe {
                    (l.api.png_set_compression_mem_level)(png, ml);
                },
            )
        };
        diff(&format!("W23 compression_mem_level={ml}"), &c, &r, &mut run);
    }
    for m in [8i32] {
        let mut run = |l: &Lib| -> Report {
            write_full(
                l, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb,
                SEED ^ 0x2400, &mut |l, png, _info| unsafe {
                    (l.api.png_set_compression_method)(png, m);
                },
            )
        };
        diff(&format!("W24 compression_method={m}"), &c, &r, &mut run);
    }
    for bs in [1usize, 2, 8, 128, 1024, 8192, 65536] {
        let mut run = |l: &Lib| -> Report {
            write_full(
                l, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb,
                SEED ^ 0x2500, &mut |l, png, _info| unsafe {
                    (l.api.png_set_compression_buffer_size)(png, bs);
                    log(format!(
                        "buffer_size now {}",
                        (l.api.png_get_compression_buffer_size)(png)
                    ));
                },
            )
        };
        diff(&format!("W25 compression_buffer_size={bs}"), &c, &r, &mut run);
    }
    // Randomized combinations of all of the above
    let mut rng = Rng::new(SEED ^ 0x2600);
    for i in 0..48 {
        let level = (rng.below(10)) as c_int;
        let strategy = (rng.below(5)) as c_int;
        let wb = 8 + (rng.below(8)) as c_int;
        let ml = 1 + (rng.below(9)) as c_int;
        let bs = 1usize << (rng.below(14) + 1);
        let mask = [
            PNG_NO_FILTERS,
            PNG_FILTER_NONE,
            PNG_FILTER_SUB,
            PNG_FILTER_UP,
            PNG_FILTER_AVG,
            PNG_FILTER_PAETH,
            PNG_ALL_FILTERS,
        ][(rng.below(7)) as usize];
        let mut run = |l: &Lib| -> Report {
            write_full(
                l, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb,
                SEED ^ 0x2600 ^ i, &mut |l, png, _info| unsafe {
                    (l.api.png_set_compression_level)(png, level);
                    (l.api.png_set_compression_strategy)(png, strategy);
                    (l.api.png_set_compression_window_bits)(png, wb);
                    (l.api.png_set_compression_mem_level)(png, ml);
                    (l.api.png_set_compression_buffer_size)(png, bs);
                    (l.api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, mask);
                },
            )
        };
        diff(
            &format!("W20-W25 combo #{i} lvl={level} st={strategy} wb={wb} ml={ml} bs={bs} f={mask:#x}"),
            &c,
            &r,
            &mut run,
        );
    }
}

// ---------------------------------------------------------------------------
// W26 text compression parameters
// ---------------------------------------------------------------------------
#[test]
fn w26_text_compression_parameters() {
    let (c, r) = libs();
    let (w, h) = (8u32, 4u32);
    let rb = rowbytes(w, 8, PNG_COLOR_TYPE_RGB);
    let long_text: String = "The quick brown fox jumps over the lazy dog. ".repeat(40);
    for (i, (level, strategy, wb, ml, method)) in [
        (0i32, 0i32, 15i32, 8i32, 8i32),
        (1, 0, 15, 8, 8),
        (9, 0, 15, 8, 8),
        (6, 1, 15, 8, 8),
        (6, 2, 15, 8, 8),
        (6, 3, 15, 8, 8),
        (6, 4, 15, 8, 8),
        (6, 0, 9, 8, 8),
        (6, 0, 15, 1, 8),
        (6, 0, 15, 9, 8),
    ]
    .into_iter()
    .enumerate()
    {
        let lt = long_text.clone();
        let mut run = |l: &Lib| -> Report {
            let lt = lt.clone();
            write_full(
                l, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb,
                SEED ^ 0x2600 ^ (i as u64), &mut |l, png, info| unsafe {
                    (l.api.png_set_text_compression_level)(png, level);
                    (l.api.png_set_text_compression_strategy)(png, strategy);
                    (l.api.png_set_text_compression_window_bits)(png, wb);
                    (l.api.png_set_text_compression_mem_level)(png, ml);
                    (l.api.png_set_text_compression_method)(png, method);
                    let key = std::ffi::CString::new("Comment").unwrap();
                    let txt = std::ffi::CString::new(lt.as_str()).unwrap();
                    let t = PngText {
                        compression: 0, // zTXt
                        key: key.as_ptr() as *mut _,
                        text: txt.as_ptr() as *mut _,
                        text_length: lt.len(),
                        ..Default::default()
                    };
                    (l.api.png_set_text)(png, info, &t, 1);
                },
            )
        };
        diff(
            &format!("W26 text compression #{i} lvl={level} st={strategy} wb={wb} ml={ml}"),
            &c,
            &r,
            &mut run,
        );
    }
}

// ---------------------------------------------------------------------------
// W27 png_set_flush / png_write_flush
// ---------------------------------------------------------------------------
#[test]
fn w27_flush() {
    let (c, r) = libs();
    let (w, h) = (16u32, 8u32);
    let rb = rowbytes(w, 8, PNG_COLOR_TYPE_RGB);
    for nrows in [0i32, 1, 2, 3, 5, 8, 100] {
        let mut run = |l: &Lib| -> Report {
            let rows = make_rows(h as usize, rb, SEED ^ 0x2700);
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_IHDR)(
                    png, info, w, h, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                (l.api.png_set_flush)(png, nrows);
                (l.api.png_write_info)(png, info);
                for row in &rows {
                    (l.api.png_write_row)(png, row.as_ptr());
                }
                (l.api.png_write_flush)(png);
                (l.api.png_write_end)(png, info);
            })
        };
        diff(&format!("W27 png_set_flush({nrows})"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// W28..W37 write transforms
// ---------------------------------------------------------------------------
#[test]
fn w28_w37_write_transforms() {
    let (c, r) = libs();
    let (w, h) = (11u32, 5u32);

    // W28 bgr, W29 swap, W32 invert_mono, W34 swap_alpha, W35 invert_alpha
    for &(ct, bd) in LEGAL {
        let rb = rowbytes(w, bd, ct);
        let pal = if ct == PNG_COLOR_TYPE_PALETTE {
            make_palette(pal_for(bd), SEED ^ 0x2800)
        } else {
            vec![]
        };
        for (name, apply) in [
            ("bgr", 0u8),
            ("swap", 1),
            ("invert_mono", 2),
            ("swap_alpha", 3),
            ("invert_alpha", 4),
            ("packswap", 5),
        ] {
            let seed = SEED ^ 0x2800 ^ ((ct as u64) << 16) ^ ((bd as u64) << 8) ^ apply as u64;
            let mut run = |l: &Lib| -> Report {
                write_full(
                    l, w, h, ct, bd, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &pal, rb, seed,
                    &mut |l, png, _info| unsafe {
                        match apply {
                            0 => (l.api.png_set_bgr)(png),
                            1 => (l.api.png_set_swap)(png),
                            2 => (l.api.png_set_invert_mono)(png),
                            3 => (l.api.png_set_swap_alpha)(png),
                            4 => (l.api.png_set_invert_alpha)(png),
                            _ => (l.api.png_set_packswap)(png),
                        }
                    },
                )
            };
            diff(
                &format!("W28-W35 {name} ct={ct} bd={bd}"),
                &c,
                &r,
                &mut run,
            );
        }
    }

    // W30 packing: input is one byte per pixel for sub-8-bit output
    for &(ct, bd) in &[(0i32, 1i32), (0, 2), (0, 4), (3, 1), (3, 2), (3, 4)] {
        let in_rb = w as usize; // one byte per pixel
        let pal = if ct == PNG_COLOR_TYPE_PALETTE {
            make_palette(pal_for(bd), SEED ^ 0x3000)
        } else {
            vec![]
        };
        for also_packswap in [false, true] {
            let seed = SEED ^ 0x3000 ^ ((ct as u64) << 8) ^ (bd as u64) ^ (also_packswap as u64) << 20;
            let mut run = |l: &Lib| -> Report {
                write_full(
                    l, w, h, ct, bd, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &pal, in_rb, seed,
                    &mut |l, png, _info| unsafe {
                        (l.api.png_set_packing)(png);
                        if also_packswap {
                            (l.api.png_set_packswap)(png);
                        }
                    },
                )
            };
            diff(
                &format!("W30-W31 packing ct={ct} bd={bd} packswap={also_packswap}"),
                &c,
                &r,
                &mut run,
            );
        }
    }

    // W33 shift (needs sBIT)
    for &(ct, bd) in &[(0i32, 8i32), (0, 16), (2, 8), (2, 16), (6, 8), (6, 16), (4, 8), (4, 16)] {
        let rb = rowbytes(w, bd, ct);
        for sb in [1u8, 3, 5, 7] {
            let sig = PngColor8 {
                red: sb,
                green: sb,
                blue: sb,
                gray: sb,
                alpha: sb,
            };
            let seed = SEED ^ 0x3300 ^ ((ct as u64) << 8) ^ (bd as u64) ^ ((sb as u64) << 24);
            let mut run = |l: &Lib| -> Report {
                write_full(
                    l, w, h, ct, bd, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], rb, seed,
                    &mut |l, png, info| unsafe {
                        (l.api.png_set_sBIT)(png, info, &sig);
                        (l.api.png_set_shift)(png, &sig);
                    },
                )
            };
            diff(
                &format!("W33 shift ct={ct} bd={bd} sbit={sb}"),
                &c,
                &r,
                &mut run,
            );
        }
    }

    // W36/W37 filler + add_alpha on write (= strip the extra channel)
    for &(ct, bd) in &[(0i32, 8i32), (0, 16), (2, 8), (2, 16)] {
        let out_ch = channels(ct);
        let in_rb = w as usize * (out_ch + 1) * (bd as usize / 8);
        for flags in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
            for use_add_alpha in [false, true] {
                let seed = SEED
                    ^ 0x3600
                    ^ ((ct as u64) << 8)
                    ^ (bd as u64)
                    ^ ((flags as u64) << 16)
                    ^ ((use_add_alpha as u64) << 24);
                let mut run = |l: &Lib| -> Report {
                    write_full(
                        l, w, h, ct, bd, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE, &[], in_rb,
                        seed, &mut |l, png, _info| unsafe {
                            if use_add_alpha {
                                (l.api.png_set_add_alpha)(png, 0xffff, flags);
                            } else {
                                (l.api.png_set_filler)(png, 0xffff, flags);
                            }
                        },
                    )
                };
                diff(
                    &format!("W36-W37 filler ct={ct} bd={bd} flags={flags} add_alpha={use_add_alpha}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W38 user transform + W39 write status callback
// ---------------------------------------------------------------------------
unsafe extern "C" fn user_write_transform(
    _png: *mut c_void,
    row_info: *mut PngRowInfo,
    data: *mut u8,
) {
    let ri = *row_info;
    log(format!("uwt ri={ri:?}"));
    if !data.is_null() && ri.rowbytes > 0 {
        let s = std::slice::from_raw_parts_mut(data, ri.rowbytes);
        for (i, b) in s.iter_mut().enumerate() {
            *b = b.wrapping_add(i as u8).rotate_left(3);
        }
    }
}

#[test]
fn w38_w39_user_transform_and_status() {
    let (c, r) = libs();
    let (w, h) = (13u32, 7u32);
    for &(ct, bd) in LEGAL {
        for interlace in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let rb = rowbytes(w, bd, ct);
            let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                make_palette(pal_for(bd), SEED ^ 0x3800)
            } else {
                vec![]
            };
            let seed = SEED ^ 0x3800 ^ ((ct as u64) << 8) ^ (bd as u64) ^ ((interlace as u64) << 30);
            let mut run = |l: &Lib| -> Report {
                write_full(
                    l, w, h, ct, bd, interlace, PNG_FILTER_TYPE_BASE, &pal, rb, seed,
                    &mut |l, png, _info| unsafe {
                        (l.api.png_set_write_user_transform_fn)(
                            png,
                            user_write_transform as *mut c_void,
                        );
                        (l.api.png_set_user_transform_info)(png, ptr::null_mut(), bd, channels(ct) as c_int);
                        (l.api.png_set_write_status_fn)(png, cb_row as *mut c_void);
                    },
                )
            };
            diff(
                &format!("W38-W39 user transform ct={ct} bd={bd} il={interlace}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// W40 MNG features / intrapixel differencing
// ---------------------------------------------------------------------------
#[test]
fn w40_mng_features() {
    let (c, r) = libs();
    let (w, h) = (9u32, 4u32);
    for &(ct, bd) in &[(2i32, 8i32), (2, 16), (6, 8), (6, 16)] {
        for features in [0u32, 1, 4, 5] {
            let rb = rowbytes(w, bd, ct);
            let seed = SEED ^ 0x4000 ^ ((ct as u64) << 8) ^ (bd as u64) ^ ((features as u64) << 16);
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    let got = (l.api.png_permit_mng_features)(png, features);
                    log(format!("mng_features({features})={got}"));
                    (l.api.png_set_IHDR)(
                        png, info, w, h, bd, ct, PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                        PNG_INTRAPIXEL_DIFFERENCING,
                    );
                    (l.api.png_write_info)(png, info);
                    for row in &make_rows(h as usize, rb, seed) {
                        (l.api.png_write_row)(png, row.as_ptr());
                    }
                    (l.api.png_write_end)(png, info);
                })
            };
            diff(
                &format!("W40 mng intrapixel ct={ct} bd={bd} features={features}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// W41 raw chunk writing
// ---------------------------------------------------------------------------
#[test]
fn w41_raw_chunk_api() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            let mut rng = Rng::new(SEED ^ 0x4100);
            (l.api.png_write_sig)(png);
            for name in [b"gAMA", b"tEXt", b"prVt", b"XXXX"] {
                for n in [0usize, 1, 7, 300] {
                    let data = rng.bytes(n);
                    (l.api.png_write_chunk)(png, name.as_ptr(), data.as_ptr(), n);
                }
            }
            // split writes
            for n in [0usize, 5, 100] {
                let data = rng.bytes(n);
                (l.api.png_write_chunk_start)(png, b"spLt".as_ptr(), n as u32);
                let mut i = 0;
                while i < n {
                    let k = (n - i).min(3);
                    (l.api.png_write_chunk_data)(png, data[i..].as_ptr(), k);
                    i += k;
                }
                (l.api.png_write_chunk_end)(png);
            }
            // zero-length data with a non-NULL pointer, and NULL data
            (l.api.png_write_chunk)(png, b"zeRo".as_ptr(), ptr::null(), 0);
            (l.api.png_write_chunk_start)(png, b"emTy".as_ptr(), 0);
            (l.api.png_write_chunk_data)(png, ptr::null(), 0);
            (l.api.png_write_chunk_end)(png);
        })
    };
    diff("W41 raw chunk API", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// W42 palette index checking
// ---------------------------------------------------------------------------
#[test]
fn w42_invalid_index_check() {
    let (c, r) = libs();
    let (w, h) = (16u32, 4u32);
    for npal in [1usize, 2, 17, 256] {
        for allowed in [-1i32, 0, 1] {
            let pal = make_palette(npal, SEED ^ 0x4200);
            let seed = SEED ^ 0x4200 ^ (npal as u64) ^ ((allowed as u64 as u64) << 32);
            let mut run = |l: &Lib| -> Report {
                write_full(
                    l, w, h, PNG_COLOR_TYPE_PALETTE, 8, PNG_INTERLACE_NONE, PNG_FILTER_TYPE_BASE,
                    &pal, w as usize, seed, &mut |l, png, _info| unsafe {
                        (l.api.png_set_check_for_invalid_index)(png, allowed);
                    },
                )
            };
            diff(
                &format!("W42 invalid index npal={npal} allowed={allowed}"),
                &c,
                &r,
                &mut run,
            );
            // read back png_get_palette_max after the write
            let mut run2 = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, w, h, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                    );
                    (l.api.png_set_PLTE)(png, info, pal.as_ptr(), pal.len() as c_int);
                    (l.api.png_set_check_for_invalid_index)(png, allowed);
                    (l.api.png_write_info)(png, info);
                    for row in &make_rows(h as usize, w as usize, seed) {
                        (l.api.png_write_row)(png, row.as_ptr());
                    }
                    (l.api.png_write_end)(png, info);
                    log(format!(
                        "palette_max={}",
                        (l.api.png_get_palette_max)(png, info)
                    ));
                })
            };
            diff(
                &format!("W42 png_get_palette_max npal={npal} allowed={allowed}"),
                &c,
                &r,
                &mut run2,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// W43 png_set_option on the write struct
// ---------------------------------------------------------------------------
#[test]
fn w43_set_option() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            for opt in [-2i32, -1, 0, 1, 2, 3, 4, 5, 6, 8, 10, 12, 14, 16, 17, 100] {
                for onoff in [0i32, 1, 2, 99] {
                    log(format!(
                        "set_option({opt},{onoff})={}",
                        (l.api.png_set_option)(png, opt, onoff)
                    ));
                }
            }
        })
    };
    diff("W43 png_set_option (write)", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// W44 split info write
// ---------------------------------------------------------------------------
#[test]
fn w44_write_info_before_plte() {
    let (c, r) = libs();
    let (w, h) = (8u32, 4u32);
    for &(ct, bd) in &[(3i32, 8i32), (2, 8), (0, 8)] {
        let pal = if ct == PNG_COLOR_TYPE_PALETTE {
            make_palette(64, SEED ^ 0x4400)
        } else {
            vec![]
        };
        let rb = rowbytes(w, bd, ct);
        let seed = SEED ^ 0x4400 ^ ((ct as u64) << 8) ^ bd as u64;
        let mut run = |l: &Lib| -> Report {
            let rows = make_rows(h as usize, rb, seed);
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_IHDR)(
                    png, info, w, h, bd, ct, PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (l.api.png_set_gAMA_fixed)(png, info, 45455);
                if !pal.is_empty() {
                    (l.api.png_set_PLTE)(png, info, pal.as_ptr(), pal.len() as c_int);
                }
                (l.api.png_write_info_before_PLTE)(png, info);
                (l.api.png_write_info)(png, info);
                for row in &rows {
                    (l.api.png_write_row)(png, row.as_ptr());
                }
                (l.api.png_write_end)(png, info);
            })
        };
        diff(&format!("W44 write_info_before_PLTE ct={ct}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// W45 png_write_png with every write transform
// ---------------------------------------------------------------------------
#[test]
fn w45_write_png_transforms() {
    let (c, r) = libs();
    let (w, h) = (12u32, 6u32);
    let transforms: &[(&str, c_int)] = &[
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
        (
            "BGR|SWAP_ALPHA|INVERT_ALPHA",
            PNG_TRANSFORM_BGR | PNG_TRANSFORM_SWAP_ALPHA | PNG_TRANSFORM_INVERT_ALPHA,
        ),
    ];
    for &(ct, bd) in LEGAL {
        for &(name, tr) in transforms {
            // Allocate the input row large enough for ANY of these transforms:
            // PACKING wants one byte per sample, STRIP_FILLER wants one extra
            // channel.  libpng only reads what it needs, and the bytes it reads
            // are the same in both implementations because the seed is fixed.
            let bytes_per_sample = ((bd as usize) / 8).max(1);
            let rb = rowbytes(w, bd, ct)
                .max(w as usize * (channels(ct) + 1) * bytes_per_sample);
            let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                make_palette(pal_for(bd), SEED ^ 0x4500)
            } else {
                vec![]
            };
            let seed = SEED ^ 0x4500 ^ ((ct as u64) << 8) ^ (bd as u64) ^ ((tr as u64) << 16);
            let mut run = |l: &Lib| -> Report {
                let rows = make_rows(h as usize, rb, seed);
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, w, h, bd, ct, PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    if !pal.is_empty() {
                        (l.api.png_set_PLTE)(png, info, pal.as_ptr(), pal.len() as c_int);
                    }
                    let sig = PngColor8 { red: 5, green: 5, blue: 5, gray: 5, alpha: 5 };
                    (l.api.png_set_sBIT)(png, info, &sig);
                    let mut ptrs: Vec<*mut u8> =
                        rows.iter().map(|v| v.as_ptr() as *mut u8).collect();
                    (l.api.png_set_rows)(png, info, ptrs.as_mut_ptr());
                    (l.api.png_write_png)(png, info, tr, ptr::null_mut());
                })
            };
            diff(
                &format!("W45 png_write_png {name} ct={ct} bd={bd}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

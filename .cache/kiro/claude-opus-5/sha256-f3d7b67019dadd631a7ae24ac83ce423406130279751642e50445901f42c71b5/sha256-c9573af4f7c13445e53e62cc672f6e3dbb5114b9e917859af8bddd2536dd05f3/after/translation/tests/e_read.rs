//! Phase B, group R: the sequential read path.  Streams are produced ONCE (by
//! the C writer) and then fed byte-identically to both readers, so any
//! divergence is a decoding difference rather than an encoding one.
mod common;
use common::*;
use std::ffi::{c_int, c_void, CString};
use std::ptr;

const SEED: u64 = 0x8ead_0f0f_1111_2222;

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

/// Build a PNG datastream with the C reference writer.
fn gen(
    cl: &Lib,
    w: u32,
    h: u32,
    ct: c_int,
    bd: c_int,
    il: c_int,
    setup: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void),
) -> Vec<u8> {
    let pal = if ct == PNG_COLOR_TYPE_PALETTE {
        make_palette(pal_for(bd), SEED ^ 0xaa)
    } else {
        vec![]
    };
    let rep = write_full(
        cl,
        w,
        h,
        ct,
        bd,
        il,
        PNG_FILTER_TYPE_BASE,
        &pal,
        rowbytes(w, bd, ct),
        SEED ^ ((ct as u64) << 24) ^ ((bd as u64) << 16) ^ (w as u64) ^ ((il as u64) << 8),
        setup,
    );
    assert!(rep.error.is_none(), "stream generation failed: {:?}", rep.error);
    rep.out
}

fn gen_plain(cl: &Lib, w: u32, h: u32, ct: c_int, bd: c_int, il: c_int) -> Vec<u8> {
    gen(cl, w, h, ct, bd, il, &mut no_setup)
}

/// Read a whole stream row-at-a-time, logging every decoded row plus the
/// post-`png_read_info` metadata.
fn read_rows_session(
    l: &Lib,
    stream: Vec<u8>,
    setup: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void),
) -> Report {
    read_session(l, stream, &mut |l, png, info| unsafe {
        (l.api.png_read_info)(png, info);
        log(format!(
            "info: {}x{} bd={} ct={} il={} ch={} rb={}",
            (l.api.png_get_image_width)(png, info),
            (l.api.png_get_image_height)(png, info),
            (l.api.png_get_bit_depth)(png, info),
            (l.api.png_get_color_type)(png, info),
            (l.api.png_get_interlace_type)(png, info),
            (l.api.png_get_channels)(png, info),
            (l.api.png_get_rowbytes)(png, info)
        ));
        setup(l, png, info);
        let passes = if (l.api.png_get_interlace_type)(png, info) == 1 {
            (l.api.png_set_interlace_handling)(png)
        } else {
            1
        };
        (l.api.png_read_update_info)(png, info);
        let h = (l.api.png_get_image_height)(png, info);
        let rb = (l.api.png_get_rowbytes)(png, info);
        log(format!("after update: rb={rb} passes={passes} ch={} bd={}",
            (l.api.png_get_channels)(png, info),
            (l.api.png_get_bit_depth)(png, info)));
        let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb + 16]).collect();
        for p in 0..passes {
            for (i, row) in rows.iter_mut().enumerate() {
                (l.api.png_read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                if p == passes - 1 {
                    log(format!("row{i}={:02x?}", &row[..rb]));
                }
            }
        }
        (l.api.png_read_end)(png, info);
        log(format!(
            "end: io_state={} chunk_type={:#x}",
            (l.api.png_get_io_state)(png),
            (l.api.png_get_io_chunk_type)(png)
        ));
    })
}

// ---------------------------------------------------------------------------
// R1/R2 every legal shape, non-interlaced and Adam7
// ---------------------------------------------------------------------------
#[test]
fn r1_r2_all_legal_shapes() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &[(1u32, 1u32), (7, 3), (9, 5), (33, 4), (17, 17)] {
                let stream = gen_plain(&c, w, h, ct, bd, il);
                let mut run = |l: &Lib| -> Report {
                    read_rows_session(l, stream.clone(), &mut no_setup)
                };
                diff(
                    &format!("R1-R2 read ct={ct} bd={bd} il={il} {w}x{h}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R3 Adam7 stream read WITHOUT png_set_interlace_handling
// ---------------------------------------------------------------------------
#[test]
fn r3_manual_interlace() {
    let (c, r) = libs();
    for &(ct, bd) in &[(2i32, 8i32), (0, 1), (6, 16)] {
        let (w, h) = (17u32, 9u32);
        let stream = gen_plain(&c, w, h, ct, bd, PNG_INTERLACE_ADAM7);
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_read_info)(png, info);
                (l.api.png_read_update_info)(png, info);
                let rb = (l.api.png_get_rowbytes)(png, info);
                // The app drives all 7 passes itself.
                for pass in 0..7u32 {
                    let rows = pass_rows(h, pass);
                    for i in 0..rows {
                        let mut buf = vec![0u8; rb + 16];
                        (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                        log(format!("p{pass} r{i}={:02x?}", &buf[..rb.min(buf.len())]));
                    }
                }
                (l.api.png_read_end)(png, info);
            })
        };
        diff(&format!("R3 manual Adam7 ct={ct} bd={bd}"), &c, &r, &mut run);
    }
}

fn pass_row_shift(pass: u32) -> u32 {
    if pass > 2 {
        (8 - pass) >> 1
    } else {
        3
    }
}
fn pass_start_row(pass: u32) -> u32 {
    ((1 & !pass) << (3 - (pass >> 1))) & 7
}
fn pass_rows(height: u32, pass: u32) -> u32 {
    let sh = pass_row_shift(pass);
    (height + ((1u32 << sh) - 1 - pass_start_row(pass))) >> sh
}

// ---------------------------------------------------------------------------
// R4 png_read_image / R5 png_read_rows
// ---------------------------------------------------------------------------
#[test]
fn r4_r5_read_image_and_rows() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let (w, h) = (13u32, 6u32);
            let stream = gen_plain(&c, w, h, ct, bd, il);
            let mut run = |l: &Lib| -> Report {
                read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_read_info)(png, info);
                    (l.api.png_read_update_info)(png, info);
                    let rb = (l.api.png_get_rowbytes)(png, info);
                    let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb + 8]).collect();
                    let mut ptrs: Vec<*mut u8> =
                        rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                    (l.api.png_read_image)(png, ptrs.as_mut_ptr());
                    for (i, row) in rows.iter().enumerate() {
                        log(format!("img row{i}={:02x?}", &row[..rb]));
                    }
                    (l.api.png_read_end)(png, info);
                })
            };
            diff(
                &format!("R4 png_read_image ct={ct} bd={bd} il={il}"),
                &c,
                &r,
                &mut run,
            );

            // png_read_rows with row only, display only, and both
            for mode in 0..3 {
                let mut run = |l: &Lib| -> Report {
                    read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                        (l.api.png_read_info)(png, info);
                        let passes = (l.api.png_set_interlace_handling)(png);
                        (l.api.png_read_update_info)(png, info);
                        let rb = (l.api.png_get_rowbytes)(png, info);
                        let mut a: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb + 8]).collect();
                        let mut b: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb + 8]).collect();
                        for _ in 0..passes {
                            let mut ap: Vec<*mut u8> =
                                a.iter_mut().map(|v| v.as_mut_ptr()).collect();
                            let mut bp: Vec<*mut u8> =
                                b.iter_mut().map(|v| v.as_mut_ptr()).collect();
                            match mode {
                                0 => (l.api.png_read_rows)(
                                    png,
                                    ap.as_mut_ptr(),
                                    ptr::null_mut(),
                                    h,
                                ),
                                1 => (l.api.png_read_rows)(
                                    png,
                                    ptr::null_mut(),
                                    bp.as_mut_ptr(),
                                    h,
                                ),
                                _ => (l.api.png_read_rows)(
                                    png,
                                    ap.as_mut_ptr(),
                                    bp.as_mut_ptr(),
                                    h,
                                ),
                            }
                        }
                        for i in 0..h as usize {
                            log(format!("a{i}={:02x?}", &a[i][..rb]));
                            log(format!("b{i}={:02x?}", &b[i][..rb]));
                        }
                        (l.api.png_read_end)(png, info);
                    })
                };
                diff(
                    &format!("R5 png_read_rows mode={mode} ct={ct} bd={bd} il={il}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R7 png_start_read_image, R8 png_set_sig_bytes
// ---------------------------------------------------------------------------
#[test]
fn r7_r8_start_read_and_sig_bytes() {
    let (c, r) = libs();
    let (w, h) = (11u32, 5u32);
    let stream = gen(&c, w, h, PNG_COLOR_TYPE_PALETTE, 8, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
        let alpha: Vec<u8> = (0..256u32).map(|i| (i as u8) ^ 0x5a).collect();
        (l.api.png_set_tRNS)(png, info, alpha.as_ptr(), 256, ptr::null());
    });
    let mut run = |l: &Lib| -> Report {
        read_session(l, stream.clone(), &mut |l, png, info| unsafe {
            (l.api.png_read_info)(png, info);
            (l.api.png_start_read_image)(png);
            let rb = (l.api.png_get_rowbytes)(png, info);
            for i in 0..h {
                let mut buf = vec![0u8; rb + 8];
                (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                log(format!("row{i}={:02x?}", &buf[..rb]));
            }
            (l.api.png_read_end)(png, info);
        })
    };
    diff("R7 png_start_read_image", &c, &r, &mut run);

    for nsig in [0usize, 1, 4, 8] {
        let mut run = |l: &Lib| -> Report {
            let s = stream[nsig..].to_vec();
            read_session(l, s, &mut |l, png, info| unsafe {
                (l.api.png_set_sig_bytes)(png, nsig as c_int);
                (l.api.png_read_info)(png, info);
                log(format!("w={}", (l.api.png_get_image_width)(png, info)));
                (l.api.png_read_end)(png, info);
            })
        };
        diff(&format!("R8 png_set_sig_bytes({nsig})"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// R9 CRC actions
// ---------------------------------------------------------------------------
#[test]
fn r9_crc_actions() {
    let (c, r) = libs();
    let (w, h) = (9u32, 4u32);
    let good = gen(&c, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
        (l.api.png_set_gAMA_fixed)(png, info, 45455);
    });
    // A stream whose gAMA CRC has been corrupted, and one whose IDAT CRC has.
    let mut bad_anc = good.clone();
    corrupt_chunk_crc(&mut bad_anc, b"gAMA");
    let mut bad_crit = good.clone();
    corrupt_chunk_crc(&mut bad_crit, b"IDAT");
    for (name, stream) in [("good", &good), ("bad-gAMA", &bad_anc), ("bad-IDAT", &bad_crit)] {
        for crit in 0..6i32 {
            for anc in 0..6i32 {
                let mut run = |l: &Lib| -> Report {
                    read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                        (l.api.png_set_crc_action)(png, crit, anc);
                    })
                };
                diff(
                    &format!("R9 crc_action {name} crit={crit} anc={anc}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

/// Flip one bit of the CRC of the first chunk with the given type.
fn corrupt_chunk_crc(s: &mut [u8], want: &[u8; 4]) {
    let mut i = 8usize;
    while i + 12 <= s.len() {
        let len = u32::from_be_bytes([s[i], s[i + 1], s[i + 2], s[i + 3]]) as usize;
        let ty = &s[i + 4..i + 8];
        if ty == want {
            let crc = i + 8 + len;
            s[crc] ^= 0x01;
            return;
        }
        i += 12 + len;
    }
    panic!("chunk {:?} not found", String::from_utf8_lossy(want));
}

// ---------------------------------------------------------------------------
// R10 user limits
// ---------------------------------------------------------------------------
#[test]
fn r10_user_limits() {
    let (c, r) = libs();
    let (w, h) = (16u32, 8u32);
    let stream = gen_plain(&c, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    for &(uw, uh) in &[
        (0u32, 0u32),
        (15, 8),
        (16, 7),
        (16, 8),
        (17, 9),
        (0x7fff_ffff, 0x7fff_ffff),
        (1_000_000, 1_000_000),
    ] {
        let mut run = |l: &Lib| -> Report {
            read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                (l.api.png_set_user_limits)(png, uw, uh);
                log(format!(
                    "limits {} {}",
                    (l.api.png_get_user_width_max)(png),
                    (l.api.png_get_user_height_max)(png)
                ));
            })
        };
        diff(&format!("R10 user_limits {uw}x{uh}"), &c, &r, &mut run);
    }
    for cache in [0u32, 1, 2, 1000] {
        for mal in [0usize, 1, 100, 1 << 24] {
            let mut run = |l: &Lib| -> Report {
                read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                    (l.api.png_set_chunk_cache_max)(png, cache);
                    (l.api.png_set_chunk_malloc_max)(png, mal);
                    log(format!(
                        "cache={} mal={}",
                        (l.api.png_get_chunk_cache_max)(png),
                        (l.api.png_get_chunk_malloc_max)(png)
                    ));
                })
            };
            diff(&format!("R10 chunk limits cache={cache} mal={mal}"), &c, &r, &mut run);
        }
    }
}

// ---------------------------------------------------------------------------
// R11..R29 read transforms, one per (transform, shape)
// ---------------------------------------------------------------------------
#[derive(Copy, Clone)]
enum Tr {
    Expand,
    PaletteToRgb,
    ExpandGray124,
    TrnsToAlpha,
    Expand16,
    GrayToRgb,
    StripAlpha,
    Strip16,
    Scale16,
    Packing,
    Packswap,
    Swap,
    Bgr,
    SwapAlpha,
    InvertAlpha,
    InvertMono,
    FillerBefore,
    FillerAfter,
    AddAlphaBefore,
    AddAlphaAfter,
    Shift,
}

const TRS: &[(&str, Tr)] = &[
    ("expand", Tr::Expand),
    ("palette_to_rgb", Tr::PaletteToRgb),
    ("expand_gray_1_2_4_to_8", Tr::ExpandGray124),
    ("tRNS_to_alpha", Tr::TrnsToAlpha),
    ("expand_16", Tr::Expand16),
    ("gray_to_rgb", Tr::GrayToRgb),
    ("strip_alpha", Tr::StripAlpha),
    ("strip_16", Tr::Strip16),
    ("scale_16", Tr::Scale16),
    ("packing", Tr::Packing),
    ("packswap", Tr::Packswap),
    ("swap", Tr::Swap),
    ("bgr", Tr::Bgr),
    ("swap_alpha", Tr::SwapAlpha),
    ("invert_alpha", Tr::InvertAlpha),
    ("invert_mono", Tr::InvertMono),
    ("filler_before", Tr::FillerBefore),
    ("filler_after", Tr::FillerAfter),
    ("add_alpha_before", Tr::AddAlphaBefore),
    ("add_alpha_after", Tr::AddAlphaAfter),
    ("shift", Tr::Shift),
];

unsafe fn apply_tr(l: &Lib, png: *mut c_void, info: *mut c_void, t: Tr) {
    match t {
        Tr::Expand => (l.api.png_set_expand)(png),
        Tr::PaletteToRgb => (l.api.png_set_palette_to_rgb)(png),
        Tr::ExpandGray124 => (l.api.png_set_expand_gray_1_2_4_to_8)(png),
        Tr::TrnsToAlpha => (l.api.png_set_tRNS_to_alpha)(png),
        Tr::Expand16 => (l.api.png_set_expand_16)(png),
        Tr::GrayToRgb => (l.api.png_set_gray_to_rgb)(png),
        Tr::StripAlpha => (l.api.png_set_strip_alpha)(png),
        Tr::Strip16 => (l.api.png_set_strip_16)(png),
        Tr::Scale16 => (l.api.png_set_scale_16)(png),
        Tr::Packing => (l.api.png_set_packing)(png),
        Tr::Packswap => (l.api.png_set_packswap)(png),
        Tr::Swap => (l.api.png_set_swap)(png),
        Tr::Bgr => (l.api.png_set_bgr)(png),
        Tr::SwapAlpha => (l.api.png_set_swap_alpha)(png),
        Tr::InvertAlpha => (l.api.png_set_invert_alpha)(png),
        Tr::InvertMono => (l.api.png_set_invert_mono)(png),
        Tr::FillerBefore => (l.api.png_set_filler)(png, 0x1234, PNG_FILLER_BEFORE),
        Tr::FillerAfter => (l.api.png_set_filler)(png, 0x1234, PNG_FILLER_AFTER),
        Tr::AddAlphaBefore => (l.api.png_set_add_alpha)(png, 0x1234, PNG_FILLER_BEFORE),
        Tr::AddAlphaAfter => (l.api.png_set_add_alpha)(png, 0x1234, PNG_FILLER_AFTER),
        Tr::Shift => {
            let sig = PngColor8 { red: 5, green: 5, blue: 5, gray: 5, alpha: 5 };
            (l.api.png_set_shift)(png, &sig);
            let _ = info;
        }
    }
}

#[test]
fn r11_r29_read_transforms() {
    let (c, r) = libs();
    let (w, h) = (13u32, 5u32);
    for &(ct, bd) in LEGAL {
        // Two stream flavours: plain, and carrying tRNS + sBIT so the
        // tRNS/shift paths are reachable.
        let plain = gen_plain(&c, w, h, ct, bd, PNG_INTERLACE_NONE);
        let rich = gen(&c, w, h, ct, bd, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
            let sig = PngColor8 {
                red: bd.min(8) as u8,
                green: bd.min(8) as u8,
                blue: bd.min(8) as u8,
                gray: bd.min(8) as u8,
                alpha: bd.min(8) as u8,
            };
            (l.api.png_set_sBIT)(png, info, &sig);
            (l.api.png_set_gAMA_fixed)(png, info, 45455);
            if ct == PNG_COLOR_TYPE_PALETTE {
                let n = pal_for(bd);
                let alpha: Vec<u8> = (0..n).map(|i| (i as u8) ^ 0x33).collect();
                (l.api.png_set_tRNS)(png, info, alpha.as_ptr(), n as c_int, ptr::null());
            } else if ct == PNG_COLOR_TYPE_GRAY || ct == PNG_COLOR_TYPE_RGB {
                let maxv = ((1u32 << bd) - 1) as u16;
                let tc = PngColor16 {
                    index: 0,
                    red: maxv / 2,
                    green: maxv / 3,
                    blue: maxv / 4,
                    gray: maxv / 2,
                };
                (l.api.png_set_tRNS)(png, info, ptr::null(), 0, &tc);
            }
        });
        for (sname, stream) in [("plain", &plain), ("rich", &rich)] {
            for &(tname, t) in TRS {
                let mut run = |l: &Lib| -> Report {
                    read_rows_session(l, stream.clone(), &mut |l, png, info| unsafe {
                        apply_tr(l, png, info, t);
                    })
                };
                diff(
                    &format!("R11-R29 {tname} ct={ct} bd={bd} {sname}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
            // randomized combinations of two and three transforms
            let mut rng = Rng::new(SEED ^ (ct as u64) ^ ((bd as u64) << 8));
            for k in 0..12 {
                let n = 2 + (k % 2);
                let picks: Vec<usize> = (0..n)
                    .map(|_| (rng.below(TRS.len() as u32)) as usize)
                    .collect();
                let names: Vec<&str> = picks.iter().map(|&i| TRS[i].0).collect();
                let mut run = |l: &Lib| -> Report {
                    read_rows_session(l, stream.clone(), &mut |l, png, info| unsafe {
                        for &i in &picks {
                            apply_tr(l, png, info, TRS[i].1);
                        }
                    })
                };
                diff(
                    &format!("R11-R29 combo {names:?} ct={ct} bd={bd} {sname}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R17 rgb_to_gray
// ---------------------------------------------------------------------------
#[test]
fn r17_rgb_to_gray() {
    let (c, r) = libs();
    let (w, h) = (11u32, 4u32);
    for &(ct, bd) in &[(2i32, 8i32), (2, 16), (6, 8), (6, 16), (3, 8)] {
        let stream = gen_plain(&c, w, h, ct, bd, PNG_INTERLACE_NONE);
        for action in [-1i32, 0, 1, 2, 3, 4] {
            for &(red, green) in &[
                (-1i32, -1i32),
                (0, 0),
                (6968, 23434),
                (100000, 0),
                (0, 100000),
                (50000, 50000),
                (70000, 70000),
            ] {
                let mut run = |l: &Lib| -> Report {
                    read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                        (l.api.png_set_rgb_to_gray_fixed)(png, action, red, green);
                        log(format!(
                            "status={}",
                            (l.api.png_get_rgb_to_gray_status)(png)
                        ));
                    })
                };
                diff(
                    &format!("R17 rgb_to_gray ct={ct} bd={bd} action={action} {red}/{green}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R30 gamma
// ---------------------------------------------------------------------------
#[test]
fn r30_gamma() {
    let (c, r) = libs();
    let (w, h) = (13u32, 4u32);
    for &(ct, bd) in LEGAL {
        for with_gama in [false, true] {
            let stream = gen(&c, w, h, ct, bd, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
                if with_gama {
                    (l.api.png_set_gAMA_fixed)(png, info, 45455);
                }
            });
            for &screen in &[100000i32, 220000, 45455, -1, -2, 0, 1, 2_147_483_647] {
                for &file in &[0i32, 45455, 100000, -1, -2] {
                    let mut run = |l: &Lib| -> Report {
                        read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                            (l.api.png_set_gamma_fixed)(png, screen, file);
                        })
                    };
                    diff(
                        &format!("R30 gamma ct={ct} bd={bd} gama={with_gama} s={screen} f={file}"),
                        &c,
                        &r,
                        &mut run,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R31 background
// ---------------------------------------------------------------------------
#[test]
fn r31_background() {
    let (c, r) = libs();
    let (w, h) = (11u32, 4u32);
    for &(ct, bd) in LEGAL {
        let stream = gen(&c, w, h, ct, bd, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
            (l.api.png_set_gAMA_fixed)(png, info, 45455);
            if ct == PNG_COLOR_TYPE_PALETTE {
                let n = pal_for(bd);
                let alpha: Vec<u8> = (0..n).map(|i| (i as u8) ^ 0x33).collect();
                (l.api.png_set_tRNS)(png, info, alpha.as_ptr(), n as c_int, ptr::null());
            }
        });
        let bg = PngColor16 {
            index: 3,
            red: 0x1234,
            green: 0x5678,
            blue: 0x9abc,
            gray: 0x0f0f,
        };
        for code in [-1i32, 0, 1, 2, 3, 4] {
            for need_expand in [0i32, 1] {
                for &bgg in &[100000i32, 45455, 0, -1] {
                    let mut run = |l: &Lib| -> Report {
                        read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                            (l.api.png_set_background_fixed)(png, &bg, code, need_expand, bgg);
                        })
                    };
                    diff(
                        &format!("R31 background ct={ct} bd={bd} code={code} exp={need_expand} g={bgg}"),
                        &c,
                        &r,
                        &mut run,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R32 alpha mode
// ---------------------------------------------------------------------------
#[test]
fn r32_alpha_mode() {
    let (c, r) = libs();
    let (w, h) = (11u32, 4u32);
    for &(ct, bd) in LEGAL {
        let stream = gen_plain(&c, w, h, ct, bd, PNG_INTERLACE_NONE);
        for mode in [-1i32, 0, 1, 2, 3, 4] {
            for &g in &[100000i32, 45455, 220000, -1, -2, 0] {
                let mut run = |l: &Lib| -> Report {
                    read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                        (l.api.png_set_alpha_mode_fixed)(png, mode, g);
                    })
                };
                diff(
                    &format!("R32 alpha_mode ct={ct} bd={bd} mode={mode} g={g}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R33 quantize
// ---------------------------------------------------------------------------
#[test]
fn r33_quantize() {
    let (c, r) = libs();
    let (w, h) = (16u32, 6u32);
    for &(ct, bd) in &[(2i32, 8i32), (2, 16), (6, 8), (3, 8), (3, 4)] {
        let stream = gen_plain(&c, w, h, ct, bd, PNG_INTERLACE_NONE);
        for maxcol in [1i32, 2, 16, 255, 256, 300] {
            for full in [0i32, 1] {
                for with_hist in [false, true] {
                    let npal = 256usize;
                    let pal0 = make_palette(npal, SEED ^ 0x3300);
                    let mut rng = Rng::new(SEED ^ 0x3301);
                    let hist: Vec<u16> =
                        (0..npal).map(|_| (rng.u32() & 0xffff) as u16).collect();
                    let mut run = |l: &Lib| -> Report {
                        // png_set_quantize MUTATES the caller's palette, so each
                        // implementation must get its own pristine copy.
                        let mut pal = pal0.clone();
                        read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                            (l.api.png_set_quantize)(
                                png,
                                pal.as_mut_ptr(),
                                npal as c_int,
                                maxcol,
                                if with_hist {
                                    hist.as_ptr()
                                } else {
                                    ptr::null()
                                },
                                full,
                            );
                            log(format!("palette after quantize={:?}", &pal[..8]));
                        })
                    };
                    diff(
                        &format!("R33 quantize ct={ct} bd={bd} max={maxcol} full={full} hist={with_hist}"),
                        &c,
                        &r,
                        &mut run,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R34 read user transform / R35 read status callback
// ---------------------------------------------------------------------------
unsafe extern "C" fn user_read_transform(
    png: *mut c_void,
    row_info: *mut PngRowInfo,
    data: *mut u8,
) {
    let ri = *row_info;
    log(format!("urt ri={ri:?} png_null={}", png.is_null()));
    if !data.is_null() && ri.rowbytes > 0 {
        let s = std::slice::from_raw_parts_mut(data, ri.rowbytes);
        for (i, b) in s.iter_mut().enumerate() {
            *b ^= (i as u8).wrapping_mul(31);
        }
    }
}

#[test]
fn r34_r35_read_callbacks() {
    let (c, r) = libs();
    let (w, h) = (13u32, 7u32);
    for &(ct, bd) in LEGAL {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let stream = gen_plain(&c, w, h, ct, bd, il);
            let mut run = |l: &Lib| -> Report {
                read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                    (l.api.png_set_read_user_transform_fn)(
                        png,
                        user_read_transform as *mut c_void,
                    );
                    (l.api.png_set_user_transform_info)(
                        png,
                        0x1234 as *mut c_void,
                        bd,
                        channels(ct) as c_int,
                    );
                    (l.api.png_set_read_status_fn)(png, cb_row as *mut c_void);
                    log(format!(
                        "user_transform_ptr={:?}",
                        (l.api.png_get_user_transform_ptr)(png)
                    ));
                })
            };
            diff(
                &format!("R34-R35 read callbacks ct={ct} bd={bd} il={il}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// R36 user chunk callback
// ---------------------------------------------------------------------------
static mut UCHUNK_RET: c_int = 0;

unsafe extern "C" fn user_chunk_cb(_png: *mut c_void, chunk: *mut PngUnknownChunk) -> c_int {
    if !chunk.is_null() {
        let ch = &*chunk;
        log(format!(
            "uchunk name={:?} size={}",
            String::from_utf8_lossy(&ch.name[..4]),
            ch.size
        ));
    }
    UCHUNK_RET
}

#[test]
fn r36_user_chunk_callback() {
    let (c, r) = libs();
    let (w, h) = (8u32, 4u32);
    let stream = gen(&c, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
        let payload = [1u8, 2, 3, 4, 5];
        let unk = [
            PngUnknownChunk {
                name: *b"prVt\0",
                data: payload.as_ptr() as *mut u8,
                size: payload.len(),
                location: PNG_HAVE_IHDR as u8,
            },
            PngUnknownChunk {
                name: *b"PrVt\0",
                data: payload.as_ptr() as *mut u8,
                size: payload.len(),
                location: PNG_HAVE_IHDR as u8,
            },
        ];
        (l.api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
        (l.api.png_set_unknown_chunks)(png, info, unk.as_ptr(), 2);
        for i in 0..2 {
            (l.api.png_set_unknown_chunk_location)(png, info, i, PNG_HAVE_IHDR);
        }
    });
    for ret in [-1i32, 0, 1] {
        let mut run = |l: &Lib| -> Report {
            unsafe { UCHUNK_RET = ret };
            read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                (l.api.png_set_read_user_chunk_fn)(
                    png,
                    0x4321 as *mut c_void,
                    user_chunk_cb as *mut c_void,
                );
                log(format!(
                    "user_chunk_ptr={:?}",
                    (l.api.png_get_user_chunk_ptr)(png)
                ));
            })
        };
        diff(&format!("R36 user chunk callback ret={ret}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// R37 keep unknown chunks on read
// ---------------------------------------------------------------------------
#[test]
fn r37_keep_unknown_on_read() {
    let (c, r) = libs();
    let (w, h) = (8u32, 4u32);
    let stream = gen(&c, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
        let payload = [9u8, 8, 7, 6];
        let unk = [
            PngUnknownChunk {
                name: *b"prVt\0",
                data: payload.as_ptr() as *mut u8,
                size: payload.len(),
                location: PNG_HAVE_IHDR as u8,
            },
            PngUnknownChunk {
                name: *b"PRVt\0",
                data: payload.as_ptr() as *mut u8,
                size: payload.len(),
                location: PNG_HAVE_IHDR as u8,
            },
        ];
        (l.api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
        (l.api.png_set_unknown_chunks)(png, info, unk.as_ptr(), 2);
        for i in 0..2 {
            (l.api.png_set_unknown_chunk_location)(png, info, i, PNG_HAVE_IHDR);
        }
    });
    for keep in [0i32, 1, 2, 3] {
        for num in [0i32, 2, -1] {
            let mut run = |l: &Lib| -> Report {
                read_rows_session(l, stream.clone(), &mut |l, png, info| unsafe {
                    let mut list: Vec<u8> = Vec::new();
                    list.extend_from_slice(b"prVt\0");
                    list.extend_from_slice(b"PRVt\0");
                    (l.api.png_set_keep_unknown_chunks)(png, keep, list.as_ptr(), num);
                    let mut e: *mut PngUnknownChunk = ptr::null_mut();
                    log(format!(
                        "unknown_chunks={}",
                        (l.api.png_get_unknown_chunks)(png, info, &mut e)
                    ));
                })
            };
            diff(&format!("R37 keep_unknown read keep={keep} num={num}"), &c, &r, &mut run);
        }
    }
}

// ---------------------------------------------------------------------------
// R38 benign errors / R39 png_set_option
// ---------------------------------------------------------------------------
#[test]
fn r38_r39_benign_and_options() {
    let (c, r) = libs();
    let (w, h) = (8u32, 4u32);
    let good = gen(&c, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
        (l.api.png_set_gAMA_fixed)(png, info, 45455);
    });
    let mut bad = good.clone();
    corrupt_chunk_crc(&mut bad, b"gAMA");
    for allowed in [-1i32, 0, 1, 2] {
        for (name, stream) in [("good", &good), ("bad", &bad)] {
            let mut run = |l: &Lib| -> Report {
                read_rows_session(l, stream.clone(), &mut |l, png, _info| unsafe {
                    (l.api.png_set_benign_errors)(png, allowed);
                })
            };
            diff(&format!("R38 benign_errors={allowed} {name}"), &c, &r, &mut run);
        }
    }
    for opt in [-1i32, 0, 2, 4, 8, 16, 100] {
        for onoff in [0i32, 1, 2] {
            let mut run = |l: &Lib| -> Report {
                read_rows_session(l, good.clone(), &mut |l, png, _info| unsafe {
                    log(format!(
                        "set_option({opt},{onoff})={}",
                        (l.api.png_set_option)(png, opt, onoff)
                    ));
                })
            };
            diff(&format!("R39 set_option {opt}/{onoff}"), &c, &r, &mut run);
        }
    }
}

// ---------------------------------------------------------------------------
// R40 png_read_png with every read transform
// ---------------------------------------------------------------------------
#[test]
fn r40_read_png_transforms() {
    let (c, r) = libs();
    let (w, h) = (12u32, 6u32);
    let transforms: &[(&str, c_int)] = &[
        ("IDENTITY", PNG_TRANSFORM_IDENTITY),
        ("STRIP_16", PNG_TRANSFORM_STRIP_16),
        ("STRIP_ALPHA", PNG_TRANSFORM_STRIP_ALPHA),
        ("PACKING", PNG_TRANSFORM_PACKING),
        ("PACKSWAP", PNG_TRANSFORM_PACKSWAP),
        ("EXPAND", PNG_TRANSFORM_EXPAND),
        ("INVERT_MONO", PNG_TRANSFORM_INVERT_MONO),
        ("SHIFT", PNG_TRANSFORM_SHIFT),
        ("BGR", PNG_TRANSFORM_BGR),
        ("SWAP_ALPHA", PNG_TRANSFORM_SWAP_ALPHA),
        ("SWAP_ENDIAN", PNG_TRANSFORM_SWAP_ENDIAN),
        ("INVERT_ALPHA", PNG_TRANSFORM_INVERT_ALPHA),
        ("GRAY_TO_RGB", PNG_TRANSFORM_GRAY_TO_RGB),
        ("EXPAND_16", PNG_TRANSFORM_EXPAND_16),
        ("SCALE_16", PNG_TRANSFORM_SCALE_16),
        (
            "EXPAND|GRAY_TO_RGB|EXPAND_16",
            PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_GRAY_TO_RGB | PNG_TRANSFORM_EXPAND_16,
        ),
    ];
    for &(ct, bd) in LEGAL {
        let stream = gen(&c, w, h, ct, bd, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
            let sb = bd.min(8) as u8;
            let sig = PngColor8 { red: sb, green: sb, blue: sb, gray: sb, alpha: sb };
            (l.api.png_set_sBIT)(png, info, &sig);
        });
        for &(tname, tr) in transforms {
            let mut run = |l: &Lib| -> Report {
                // Supply our own zeroed row buffers.  png_combine_row
                // deliberately PRESERVES the unused bits of the final byte of a
                // sub-8-bit row ("Preserve the last byte in cases where only
                // part of it will be overwritten"), so the rows png_read_png
                // allocates itself with png_malloc would expose uninitialised
                // heap in those padding bits.
                let cap = w as usize * 16 + 32;
                let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; cap]).collect();
                let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_rows)(png, info, ptrs.as_mut_ptr());
                    (l.api.png_read_png)(png, info, tr, ptr::null_mut());
                    let rb = (l.api.png_get_rowbytes)(png, info);
                    log(format!(
                        "rb={rb} ct={} bd={} ch={}",
                        (l.api.png_get_color_type)(png, info),
                        (l.api.png_get_bit_depth)(png, info),
                        (l.api.png_get_channels)(png, info)
                    ));
                    let got = (l.api.png_get_rows)(png, info);
                    if !got.is_null() {
                        for i in 0..h as usize {
                            let p = *got.add(i);
                            if !p.is_null() {
                                log(format!(
                                    "row{i}={:02x?}",
                                    std::slice::from_raw_parts(p, rb)
                                ));
                            }
                        }
                    }
                })
            };
            diff(
                &format!("R40 png_read_png {tname} ct={ct} bd={bd}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// R41 io state observed from the read callback
// ---------------------------------------------------------------------------
#[test]
fn r41_io_state() {
    let (c, r) = libs();
    let (w, h) = (8u32, 4u32);
    let stream = gen(&c, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
        (l.api.png_set_gAMA_fixed)(png, info, 45455);
        (l.api.png_set_pHYs)(png, info, 300, 300, 1);
        let t = PngTime { year: 2020, month: 2, day: 3, hour: 4, minute: 5, second: 6 };
        (l.api.png_set_tIME)(png, info, &t);
    });
    let mut run = |l: &Lib| -> Report {
        read_session(l, stream.clone(), &mut |l, png, info| unsafe {
            (l.api.png_read_info)(png, info);
            log(format!(
                "io_state={:#x} chunk={:#x}",
                (l.api.png_get_io_state)(png),
                (l.api.png_get_io_chunk_type)(png)
            ));
            let rb = (l.api.png_get_rowbytes)(png, info);
            for _ in 0..h {
                let mut buf = vec![0u8; rb + 8];
                (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                log(format!(
                    "io_state={:#x} chunk={:#x} row={:02x?}",
                    (l.api.png_get_io_state)(png),
                    (l.api.png_get_io_chunk_type)(png),
                    &buf[..rb]
                ));
            }
            (l.api.png_read_end)(png, info);
            log(format!(
                "final io_state={:#x} chunk={:#x}",
                (l.api.png_get_io_state)(png),
                (l.api.png_get_io_chunk_type)(png)
            ));
        })
    };
    diff("R41 io state", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// R43 all ancillary chunks round-tripped and read back
// ---------------------------------------------------------------------------
#[test]
fn r43_all_ancillary_chunks() {
    let (c, r) = libs();
    let (w, h) = (8u32, 4u32);
    let purpose = CString::new("purpose").unwrap();
    let units = CString::new("m").unwrap();
    let iccname = CString::new("profile").unwrap();
    let key = CString::new("Title").unwrap();
    let txt = CString::new("A title value").unwrap();
    let sw = CString::new("1.5").unwrap();
    let sh = CString::new("2.5").unwrap();
    let prof = {
        let mut p = vec![0u8; 132];
        p[0..4].copy_from_slice(&132u32.to_be_bytes());
        p[4..8].copy_from_slice(b"ADBE");
        p[8..12].copy_from_slice(&0x0200_0000u32.to_be_bytes());
        p[12..16].copy_from_slice(b"mntr");
        p[16..20].copy_from_slice(b"RGB ");
        p[20..24].copy_from_slice(b"XYZ ");
        p[36..40].copy_from_slice(b"acsp");
        p[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
        p[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        p[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
        p
    };
    let stream = gen(&c, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
        (l.api.png_set_gAMA_fixed)(png, info, 45455);
        (l.api.png_set_cHRM_fixed)(png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
        let sig = PngColor8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
        (l.api.png_set_sBIT)(png, info, &sig);
        let bg = PngColor16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
        (l.api.png_set_bKGD)(png, info, &bg);
        let tc = PngColor16 { index: 0, red: 10, green: 20, blue: 30, gray: 40 };
        (l.api.png_set_tRNS)(png, info, ptr::null(), 0, &tc);
        (l.api.png_set_pHYs)(png, info, 400, 500, 1);
        (l.api.png_set_oFFs)(png, info, -7, 9, 1);
        (l.api.png_set_pCAL)(png, info, purpose.as_ptr(), 0, 255, 0, 0, units.as_ptr(), ptr::null_mut());
        (l.api.png_set_sCAL_s)(png, info, 1, sw.as_ptr(), sh.as_ptr());
        let t = PngTime { year: 2021, month: 3, day: 4, hour: 5, minute: 6, second: 7 };
        (l.api.png_set_tIME)(png, info, &t);
        (l.api.png_set_iCCP)(png, info, iccname.as_ptr(), 0, prof.as_ptr(), prof.len() as u32);
        let tt = PngText {
            compression: -1,
            key: key.as_ptr() as *mut i8,
            text: txt.as_ptr() as *mut i8,
            text_length: 13,
            ..Default::default()
        };
        (l.api.png_set_text)(png, info, &tt, 1);
        (l.api.png_set_cICP)(png, info, 9, 16, 0, 1);
        (l.api.png_set_cLLI_fixed)(png, info, 10_000_000, 4_000_000);
        (l.api.png_set_mDCV_fixed)(
            png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10_000_000, 500,
        );
        let exif = b"II*\0\x08\0\0\0";
        (l.api.png_set_eXIf_1)(png, info, exif.len() as u32, exif.as_ptr() as *mut u8);
    });
    let mut run = |l: &Lib| -> Report {
        read_session(l, stream.clone(), &mut |l, png, info| unsafe {
            (l.api.png_read_info)(png, info);
            let masks: &[(&str, u32)] = &[
                ("gAMA", PNG_INFO_gAMA),
                ("sBIT", PNG_INFO_sBIT),
                ("cHRM", PNG_INFO_cHRM),
                ("PLTE", PNG_INFO_PLTE),
                ("tRNS", PNG_INFO_tRNS),
                ("bKGD", PNG_INFO_bKGD),
                ("hIST", PNG_INFO_hIST),
                ("pHYs", PNG_INFO_pHYs),
                ("oFFs", PNG_INFO_oFFs),
                ("tIME", PNG_INFO_tIME),
                ("pCAL", PNG_INFO_pCAL),
                ("sRGB", PNG_INFO_sRGB),
                ("iCCP", PNG_INFO_iCCP),
                ("sPLT", PNG_INFO_sPLT),
                ("sCAL", PNG_INFO_sCAL),
                ("IDAT", PNG_INFO_IDAT),
                ("eXIf", PNG_INFO_eXIf),
                ("cICP", PNG_INFO_cICP),
                ("cLLI", PNG_INFO_cLLI),
                ("mDCV", PNG_INFO_mDCV),
            ];
            for (n, m) in masks {
                log(format!("valid {n}={}", (l.api.png_get_valid)(png, info, *m)));
            }
            let mut g = 0i32;
            log(format!("gAMA={} {g}", (l.api.png_get_gAMA_fixed)(png, info, &mut g)));
            let mut a = [0i32; 8];
            log(format!(
                "cHRM={} {a:?}",
                (l.api.png_get_cHRM_fixed)(
                    png, info, &mut a[0], &mut a[1], &mut a[2], &mut a[3], &mut a[4], &mut a[5],
                    &mut a[6], &mut a[7]
                )
            ));
            let mut sb: *mut PngColor8 = ptr::null_mut();
            log(format!("sBIT={} {:?}", (l.api.png_get_sBIT)(png, info, &mut sb),
                if sb.is_null() { None } else { Some(*sb) }));
            let mut bgp: *mut PngColor16 = ptr::null_mut();
            log(format!("bKGD={} {:?}", (l.api.png_get_bKGD)(png, info, &mut bgp),
                if bgp.is_null() { None } else { Some(*bgp) }));
            let mut ta: *mut u8 = ptr::null_mut();
            let mut nt = 0;
            let mut tcp: *mut PngColor16 = ptr::null_mut();
            log(format!("tRNS={} n={nt} {:?}",
                (l.api.png_get_tRNS)(png, info, &mut ta, &mut nt, &mut tcp),
                if tcp.is_null() { None } else { Some(*tcp) }));
            let mut rx = 0u32;
            let mut ry = 0u32;
            let mut ru = 0;
            log(format!("pHYs={} {rx} {ry} {ru}",
                (l.api.png_get_pHYs)(png, info, &mut rx, &mut ry, &mut ru)));
            let mut ox = 0i32;
            let mut oy = 0i32;
            let mut ou = 0;
            log(format!("oFFs={} {ox} {oy} {ou}",
                (l.api.png_get_oFFs)(png, info, &mut ox, &mut oy, &mut ou)));
            let mut tp: *mut PngTime = ptr::null_mut();
            log(format!("tIME={} {:?}", (l.api.png_get_tIME)(png, info, &mut tp),
                if tp.is_null() { None } else { Some(*tp) }));
            let mut nm: *mut i8 = ptr::null_mut();
            let mut comp = 0;
            let mut pp: *mut u8 = ptr::null_mut();
            let mut plen = 0u32;
            log(format!("iCCP={} comp={comp} plen={plen}",
                (l.api.png_get_iCCP)(png, info, &mut nm, &mut comp, &mut pp, &mut plen)));
            let mut txp: *mut PngText = ptr::null_mut();
            let mut ntx = 0;
            log(format!("text={} n={ntx}", (l.api.png_get_text)(png, info, &mut txp, &mut ntx)));
            let mut cp = [0u8; 4];
            log(format!("cICP={} {cp:?}",
                (l.api.png_get_cICP)(png, info, &mut cp[0], &mut cp[1], &mut cp[2], &mut cp[3])));
            let mut c1 = 0u32;
            let mut c2 = 0u32;
            log(format!("cLLI={} {c1} {c2}",
                (l.api.png_get_cLLI_fixed)(png, info, &mut c1, &mut c2)));
            let mut m = [0i32; 8];
            let mut ml = 0u32;
            let mut mn = 0u32;
            log(format!("mDCV={} {m:?} {ml} {mn}",
                (l.api.png_get_mDCV_fixed)(png, info, &mut m[0], &mut m[1], &mut m[2], &mut m[3],
                    &mut m[4], &mut m[5], &mut m[6], &mut m[7], &mut ml, &mut mn)));
            let mut ne = 0u32;
            let mut ep: *mut u8 = ptr::null_mut();
            log(format!("eXIf={} n={ne}", (l.api.png_get_eXIf_1)(png, info, &mut ne, &mut ep)));
            let mut su = 0;
            let mut ssw: *mut i8 = ptr::null_mut();
            let mut ssh: *mut i8 = ptr::null_mut();
            log(format!("sCAL_s={} unit={su}",
                (l.api.png_get_sCAL_s)(png, info, &mut su, &mut ssw, &mut ssh)));
            log(format!("signature={:?}",
                {
                    let p = (l.api.png_get_signature)(png, info);
                    if p.is_null() { None } else { Some(std::slice::from_raw_parts(p, 8).to_vec()) }
                }));
            let rb = (l.api.png_get_rowbytes)(png, info);
            for i in 0..h {
                let mut buf = vec![0u8; rb + 8];
                (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                log(format!("row{i}={:02x?}", &buf[..rb]));
            }
            (l.api.png_read_end)(png, info);
            log(format!("after end: text={}", {
                let mut txp: *mut PngText = ptr::null_mut();
                let mut ntx = 0;
                (l.api.png_get_text)(png, info, &mut txp, &mut ntx)
            }));
            log(format!("reset_zstream={}", (l.api.png_reset_zstream)(png)));
        })
    };
    diff("R43 all ancillary chunks", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// R45/R46/R47 chunk ordering, multiple IDATs, zero-length chunks
// ---------------------------------------------------------------------------
#[test]
fn r45_r47_stream_shapes() {
    let (c, r) = libs();
    let (w, h) = (32u32, 8u32);
    // R46: force many small IDATs
    for bs in [1usize, 2, 8, 64, 8192] {
        let stream = gen(&c, w, h, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, &mut |l, png, _info| unsafe {
            (l.api.png_set_compression_buffer_size)(png, bs);
        });
        let mut run = |l: &Lib| -> Report { read_rows_session(l, stream.clone(), &mut no_setup) };
        diff(&format!("R46 multi-IDAT buffer_size={bs}"), &c, &r, &mut run);
    }
    // R45: ancillary chunks before PLTE, after PLTE, after IDAT
    let stream = gen(&c, 8, 4, PNG_COLOR_TYPE_PALETTE, 8, PNG_INTERLACE_NONE, &mut |l, png, info| unsafe {
        (l.api.png_set_gAMA_fixed)(png, info, 45455);
        let bg = PngColor16 { index: 2, red: 0, green: 0, blue: 0, gray: 0 };
        (l.api.png_set_bKGD)(png, info, &bg);
        let hist: Vec<u16> = (0..256u32).map(|i| i as u16).collect();
        (l.api.png_set_hIST)(png, info, hist.as_ptr());
        let t = PngTime { year: 2022, month: 1, day: 2, hour: 3, minute: 4, second: 5 };
        (l.api.png_set_tIME)(png, info, &t);
        let key = CString::new("After").unwrap();
        let txt = CString::new("idat text").unwrap();
        let tt = PngText {
            compression: -1,
            key: key.as_ptr() as *mut i8,
            text: txt.as_ptr() as *mut i8,
            text_length: 9,
            ..Default::default()
        };
        (l.api.png_set_text)(png, info, &tt, 1);
        std::mem::forget(key);
        std::mem::forget(txt);
    });
    let mut run = |l: &Lib| -> Report { read_rows_session(l, stream.clone(), &mut no_setup) };
    diff("R45 chunk ordering (PLTE + hIST + bKGD + tIME + tEXt)", &c, &r, &mut run);

    // R47: hand-crafted zero-length IDAT and zero-length ancillary chunk
    let base = gen_plain(&c, 4, 2, PNG_COLOR_TYPE_GRAY, 8, PNG_INTERLACE_NONE);
    let with_empty = insert_chunk_before(&base, b"IDAT", b"zeRo", &[]);
    let mut run = |l: &Lib| -> Report { read_rows_session(l, with_empty.clone(), &mut no_setup) };
    diff("R47 zero-length ancillary chunk", &c, &r, &mut run);
    let with_empty_idat = insert_chunk_before(&base, b"IDAT", b"IDAT", &[]);
    let mut run = |l: &Lib| -> Report {
        read_rows_session(l, with_empty_idat.clone(), &mut no_setup)
    };
    diff("R47 zero-length IDAT", &c, &r, &mut run);
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

fn make_chunk(ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(ty);
    v.extend_from_slice(data);
    let mut crc_in = ty.to_vec();
    crc_in.extend_from_slice(data);
    v.extend_from_slice(&crc32(&crc_in).to_be_bytes());
    v
}

fn insert_chunk_before(s: &[u8], before: &[u8; 4], ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = s[..8].to_vec();
    let mut i = 8usize;
    let mut inserted = false;
    while i + 12 <= s.len() {
        let len = u32::from_be_bytes([s[i], s[i + 1], s[i + 2], s[i + 3]]) as usize;
        let cty = &s[i + 4..i + 8];
        if !inserted && cty == before {
            out.extend_from_slice(&make_chunk(ty, data));
            inserted = true;
        }
        out.extend_from_slice(&s[i..i + 12 + len]);
        i += 12 + len;
    }
    assert!(inserted, "anchor chunk not found");
    out
}

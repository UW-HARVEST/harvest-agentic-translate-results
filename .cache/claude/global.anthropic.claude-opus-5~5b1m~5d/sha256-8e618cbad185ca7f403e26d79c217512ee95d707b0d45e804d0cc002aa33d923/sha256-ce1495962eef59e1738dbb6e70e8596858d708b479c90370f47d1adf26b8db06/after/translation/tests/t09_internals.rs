//! Phase B — CONFIGS.md section E, lowest level: the exported *internal*
//! pipeline entry points, driven directly (not through the convenience
//! wrappers) so that the composed pipeline is not the only thing tested.
mod common;
use common::*;
use std::ffi::CString;

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

const PNG_IHDR: u32 = 0x4948_4452;
const PNG_PLTE: u32 = 0x504c_5445;
const PNG_IDAT: u32 = 0x4944_4154;
const PNG_IEND: u32 = 0x4945_4e44;

// ---------------------------------------------------------------------------

unsafe fn make_png(
    rng: &mut Rng,
    ct: c_int,
    bd: c_int,
    w: u32,
    h: u32,
    il: c_int,
    with_extras: bool,
) -> Vec<u8> {
    let api = c_api();
    set_current_api(api);
    diag_reset();
    let mut sess = WriteSess::new(api);
    let (png, info) = (sess.png, sess.info);
    let pd = channels_of(ct) * bd as u32;
    let rows: Vec<Vec<u8>> = (0..h).map(|_| rng.bytes(rowbytes(pd, w))).collect();
    let npal = if ct == PNG_COLOR_TYPE_PALETTE {
        1usize << bd
    } else {
        0
    };
    let palette: Vec<png_color> = (0..npal)
        .map(|_| png_color {
            red: rng.u8(),
            green: rng.u8(),
            blue: rng.u8(),
        })
        .collect();
    let mut keep: Vec<CString> = Vec::new();
    let mut texts: Vec<png_text> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
    guard(|| {
        (api.png_set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
        if !palette.is_empty() {
            (api.png_set_PLTE)(png, info, palette.as_ptr(), palette.len() as c_int);
        }
        if with_extras {
            (api.png_set_gAMA_fixed)(png, info, 45455);
            (api.png_set_pHYs)(png, info, 300, 300, 1);
            (api.png_set_oFFs)(png, info, -5, 7, 0);
            (api.png_set_tIME)(
                png,
                info,
                &png_time {
                    year: 2024,
                    month: 6,
                    day: 1,
                    hour: 12,
                    minute: 0,
                    second: 0,
                } as *const _ as png_const_timep,
            );
            keep.push(cs("Title"));
            let k = keep.last().unwrap().as_ptr() as png_charp;
            keep.push(cs("low level"));
            let t = keep.last().unwrap().as_ptr() as png_charp;
            texts.push(png_text {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: k,
                text: t,
                text_length: 9,
                itxt_length: 0,
                lang: std::ptr::null_mut(),
                lang_key: std::ptr::null_mut(),
            });
            (api.png_set_text)(png, info, texts.last().unwrap(), 1);
            if ct == PNG_COLOR_TYPE_PALETTE {
                trns = (0..npal).map(|i| (i as u8) ^ 0x3c).collect();
                (api.png_set_tRNS)(
                    png,
                    info,
                    trns.as_mut_ptr(),
                    trns.len() as c_int,
                    std::ptr::null_mut(),
                );
                let hist: Vec<u16> = (0..npal).map(|i| (i * 13) as u16).collect();
                (api.png_set_hIST)(png, info, hist.as_ptr());
            }
        }
        (api.png_write_info)(png, info);
        let mut rp: Vec<png_bytep> = rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
        (api.png_write_image)(png, rp.as_mut_ptr());
        (api.png_write_end)(png, info);
    });
    let _ = diag_take();
    std::mem::take(&mut sess.sink.buf)
}

// ---------------------------------------------------------------------------

#[test]
fn internal_getters_and_defaults() {
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let s = ReadSess::new(api, &[]);
            let d = (
                (api.png_get_chunk_cache_max)(s.png),
                (api.png_get_chunk_malloc_max)(s.png),
                (api.png_get_compression_buffer_size)(s.png),
                (api.png_get_user_width_max)(s.png),
                (api.png_get_user_height_max)(s.png),
            );
            let w = WriteSess::new(api);
            set_current_api(api);
            let dw = (api.png_get_compression_buffer_size)(w.png);
            // NULL guards
            let n = (
                (api.png_get_chunk_cache_max)(std::ptr::null()),
                (api.png_get_chunk_malloc_max)(std::ptr::null()),
                (api.png_get_compression_buffer_size)(std::ptr::null()),
                (api.png_get_user_width_max)(std::ptr::null()),
                (api.png_get_user_height_max)(std::ptr::null()),
            );
            // after setting
            (api.png_set_chunk_cache_max)(s.png, 17);
            (api.png_set_chunk_malloc_max)(s.png, 4096);
            (api.png_set_user_limits)(s.png, 123, 456);
            (api.png_set_compression_buffer_size)(w.png, 777);
            let a = (
                (api.png_get_chunk_cache_max)(s.png),
                (api.png_get_chunk_malloc_max)(s.png),
                (api.png_get_user_width_max)(s.png),
                (api.png_get_user_height_max)(s.png),
                (api.png_get_compression_buffer_size)(w.png),
            );
            let diag = diag_take();
            if api.name == "C" {
                C_SNAP.with(|c| *c.borrow_mut() = Some((d, dw, n, a, diag)));
            } else {
                C_SNAP.with(|c| {
                    let want = c.borrow().clone().unwrap();
                    assert_eq!(want.0, d, "internal getter defaults (read)");
                    assert_eq!(want.1, dw, "compression buffer size default (write)");
                    assert_eq!(want.2, n, "internal getter NULL guards");
                    assert_eq!(want.3, a, "internal getters after set");
                    assert_eq!(want.4, diag, "internal getter diagnostics");
                });
            }
        }
    }
}

type Snap = (
    (u32, usize, usize, u32, u32),
    usize,
    (u32, usize, usize, u32, u32),
    (u32, usize, u32, u32, usize),
    Diag,
);
thread_local! {
    static C_SNAP: std::cell::RefCell<Option<Snap>> = const { std::cell::RefCell::new(None) };
}

#[test]
fn set_invalid_and_valid_mask() {
    let mut rng = Rng::new(0x1a2b_3c4d_5e6f_7001);
    let bytes = unsafe {
        make_png(
            &mut rng,
            PNG_COLOR_TYPE_PALETTE,
            4,
            9,
            3,
            PNG_INTERLACE_NONE,
            true,
        )
    };
    let flags = [
        PNG_INFO_gAMA,
        PNG_INFO_sBIT,
        PNG_INFO_cHRM,
        PNG_INFO_PLTE,
        PNG_INFO_tRNS,
        PNG_INFO_bKGD,
        PNG_INFO_hIST,
        PNG_INFO_pHYs,
        PNG_INFO_oFFs,
        PNG_INFO_tIME,
        PNG_INFO_pCAL,
        PNG_INFO_sRGB,
        PNG_INFO_iCCP,
        PNG_INFO_sPLT,
        PNG_INFO_sCAL,
        PNG_INFO_IDAT,
        PNG_INFO_eXIf,
        PNG_INFO_cICP,
        PNG_INFO_cLLI,
        PNG_INFO_mDCV,
        0,
        0xffff_ffff,
        0x8000_0000,
    ];
    for &f in &flags {
        let mut outs = Vec::new();
        for api in both() {
            unsafe {
                set_current_api(api);
                diag_reset();
                let s = ReadSess::new(api, &bytes);
                let mut before = 0u32;
                let mut after = 0u32;
                let ok = guard(|| {
                    (api.png_read_info)(s.png, s.info);
                    before = (api.png_get_valid)(s.png, s.info, 0xffff_ffff);
                    (api.png_set_invalid)(s.png, s.info, f as c_int);
                    after = (api.png_get_valid)(s.png, s.info, 0xffff_ffff);
                })
                .is_some();
                // NULL guards
                (api.png_set_invalid)(std::ptr::null(), s.info, f as c_int);
                (api.png_set_invalid)(s.png, std::ptr::null_mut(), f as c_int);
                outs.push((ok, diag_take(), before, after));
            }
        }
        assert_eq!(outs[0], outs[1], "png_set_invalid({:#x})", f);
    }
}

#[test]
fn gamma_table_lifecycle() {
    let mut rng = Rng::new(0x2b3c_4d5e_6f70_8002);
    for (ct, bd) in legal_ihdr() {
        let bytes = unsafe { make_png(&mut rng, ct, bd, 7, 3, PNG_INTERLACE_NONE, true) };
        for bit_depth_arg in [1i32, 2, 4, 8, 16, 0, -1, 32] {
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    let s = ReadSess::new(api, &bytes);
                    let mut resolved = 0i32;
                    let mut corrected: Vec<u16> = Vec::new();
                    let ok = guard(|| {
                        (api.png_read_info)(s.png, s.info);
                        (api.png_set_gamma_fixed)(s.png, 100000, 45455);
                        resolved = (api.png_resolve_file_gamma)(s.png);
                        (api.png_build_gamma_table)(s.png, bit_depth_arg);
                        for v in [0u32, 1, 5, 63, 127, 128, 200, 255] {
                            corrected.push((api.png_gamma_correct)(s.png, v, 45455));
                            corrected.push((api.png_gamma_correct)(s.png, v, 100000));
                            corrected.push((api.png_gamma_correct)(s.png, v, 220000));
                        }
                        (api.png_destroy_gamma_table)(s.png);
                        // building and destroying twice must be safe
                        (api.png_build_gamma_table)(s.png, bit_depth_arg);
                        (api.png_destroy_gamma_table)(s.png);
                        (api.png_destroy_gamma_table)(s.png);
                    })
                    .is_some();
                    outs.push((ok, diag_take(), resolved, corrected));
                }
            }
            assert_eq!(
                outs[0], outs[1],
                "gamma table ct={} bd={} arg={}",
                ct, bd, bit_depth_arg
            );
        }
    }
}

#[test]
fn read_transform_primitives_direct() {
    let mut rng = Rng::new(0x3c4d_5e6f_7081_9003);
    for (ct, bd) in legal_ihdr() {
        let bytes = unsafe { make_png(&mut rng, ct, bd, 11, 4, PNG_INTERLACE_NONE, true) };
        let mut outs = Vec::new();
        for api in both() {
            unsafe {
                set_current_api(api);
                diag_reset();
                let s = ReadSess::new(api, &bytes);
                let mut ri_after = png_row_info::default();
                let mut row_out: Vec<u8> = Vec::new();
                let mut combined: Vec<u8> = Vec::new();
                let mut rbz = 0usize;
                let ok = guard(|| {
                    (api.png_read_info)(s.png, s.info);
                    (api.png_set_expand)(s.png);
                    (api.png_set_gray_to_rgb)(s.png);
                    (api.png_set_expand_16)(s.png);
                    // The two halves of what png_read_update_info does, called
                    // separately: first the transformation-initialisation,
                    // then the info_ptr update.
                    (api.png_init_read_transformations)(s.png);
                    (api.png_read_transform_info)(s.png, s.info);
                    rbz = (api.png_get_rowbytes)(s.png, s.info);
                    // Read one row through the normal path so that row_buf and
                    // transformed_pixel_depth are valid ...
                    let mut buf = vec![0u8; rbz + 32];
                    (api.png_read_row)(s.png, buf.as_mut_ptr(), std::ptr::null_mut());
                    row_out = buf.clone();
                    // ... then re-combine the same internal row buffer into a
                    // fresh destination with png_combine_row directly.
                    let mut dst = vec![0xa5u8; rbz + 32];
                    (api.png_combine_row)(s.png, dst.as_mut_ptr(), -1);
                    (api.png_combine_row)(s.png, dst.as_mut_ptr(), 1);
                    (api.png_combine_row)(s.png, dst.as_mut_ptr(), 0);
                    combined = dst;
                    // png_do_read_transformations on a synthetic row_info: the
                    // transformations have already been applied to row_buf, so
                    // this exercises the dispatcher with the real png_struct.
                    let mut ri = png_row_info {
                        width: 11,
                        rowbytes: rowbytes(channels_of(ct) * bd as u32, 11),
                        color_type: ct as u8,
                        bit_depth: bd as u8,
                        channels: channels_of(ct) as u8,
                        pixel_depth: (channels_of(ct) * bd as u32) as u8,
                    };
                    (api.png_do_check_palette_indexes)(s.png, &mut ri);
                    ri_after = ri;
                })
                .is_some();
                outs.push((ok, diag_take(), rbz, ri_after, row_out, combined));
            }
        }
        assert_eq!(outs[0].0, outs[1].0, "read prim parity ct={} bd={}", ct, bd);
        assert_eq!(outs[0].1, outs[1].1, "read prim diag ct={} bd={}", ct, bd);
        assert_eq!(outs[0].2, outs[1].2, "read prim rowbytes");
        assert_eq!(outs[0].3, outs[1].3, "read prim row_info");
        assert_eq!(outs[0].4, outs[1].4, "read prim row");
        assert_eq!(outs[0].5, outs[1].5, "read prim combined row");
    }
}

#[test]
fn write_transform_primitives_direct() {
    // NOTE: png_do_write_transformations / png_do_check_palette_indexes /
    // png_write_find_filter all operate on png_ptr->row_buf, which only exists
    // after png_write_start_row has run (pngwtran.c:520, pngwutil.c).  They are
    // therefore driven here *after* one ordinary png_write_row, which is the
    // only state in which the C can be called at all.
    let mut rng = Rng::new(0x4d5e_6f70_8192_a004);
    for (ct, bd) in legal_ihdr() {
        let pd = channels_of(ct) * bd as u32;
        let rb = rowbytes(pd, 11);
        let rows: Vec<Vec<u8>> = (0..4).map(|_| rng.bytes(rb + 32)).collect();
        let mut outs = Vec::new();
        for api in both() {
            unsafe {
                set_current_api(api);
                diag_reset();
                let mut s = WriteSess::new(api);
                let npal = if ct == PNG_COLOR_TYPE_PALETTE {
                    1usize << bd
                } else {
                    0
                };
                let palette: Vec<png_color> = (0..npal)
                    .map(|i| png_color {
                        red: i as u8,
                        green: (i * 2) as u8,
                        blue: (i * 3) as u8,
                    })
                    .collect();
                let mut ri = png_row_info {
                    width: 11,
                    rowbytes: rb,
                    color_type: ct as u8,
                    bit_depth: bd as u8,
                    channels: channels_of(ct) as u8,
                    pixel_depth: pd as u8,
                };
                let mut ri2 = ri;
                let ok = guard(|| {
                    (api.png_set_IHDR)(s.png, s.info, 11, 4, bd, ct, 0, 0, 0);
                    if !palette.is_empty() {
                        (api.png_set_PLTE)(
                            s.png,
                            s.info,
                            palette.as_ptr(),
                            palette.len() as c_int,
                        );
                    }
                    (api.png_set_filter)(s.png, PNG_FILTER_TYPE_BASE, PNG_ALL_FILTERS);
                    (api.png_write_info)(s.png, s.info);
                    // allocates row_buf / prev_row / try_row / tst_row
                    (api.png_write_row)(s.png, rows[0].as_ptr());
                    (api.png_set_bgr)(s.png);
                    (api.png_set_swap)(s.png);
                    (api.png_set_invert_mono)(s.png);
                    (api.png_do_write_transformations)(s.png, &mut ri);
                    (api.png_do_check_palette_indexes)(s.png, &mut ri2);
                    (api.png_write_find_filter)(s.png, &mut ri2);
                    (api.png_write_finish_row)(s.png);
                })
                .is_some();
                outs.push((ok, diag_take(), ri, ri2, std::mem::take(&mut s.sink.buf)));
            }
        }
        assert_eq!(outs[0].0, outs[1].0, "write prim parity ct={} bd={}", ct, bd);
        assert_eq!(outs[0].1, outs[1].1, "write prim diag ct={} bd={}", ct, bd);
        assert_eq!(outs[0].2, outs[1].2, "write prim row_info");
        assert_eq!(outs[0].3, outs[1].3, "write prim row_info 2");
        assert_bytes_eq(
            &format!("write prim bytes ct={} bd={}", ct, bd),
            &outs[0].4,
            &outs[1].4,
        );
    }
}

/// Drive the chunk layer by hand: `png_read_sig`, `png_read_chunk_header`,
/// `png_chunk_unknown_handling`, `png_handle_chunk` / `png_handle_unknown`,
/// and (for the ancillary chunks) `png_crc_read` + `png_crc_finish` directly.
#[test]
fn low_level_chunk_read_driver() {
    let mut rng = Rng::new(0x5e6f_7081_92a3_b005);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let bytes = unsafe { make_png(&mut rng, ct, bd, 13, 5, il, true) };
            for manual_crc in [false, true] {
                let mut outs = Vec::new();
                for api in both() {
                    unsafe {
                        set_current_api(api);
                        diag_reset();
                        let s = ReadSess::new(api, &bytes);
                        let mut log: Vec<(u32, u32, c_int, c_int, Vec<u8>)> = Vec::new();
                        let mut ihdr = (0u32, 0u32, 0i32, 0i32, 0i32);
                        let ok = guard(|| {
                            (api.png_read_sig)(s.png, s.info);
                            loop {
                                let len = (api.png_read_chunk_header)(s.png);
                                let name = (api.png_get_io_chunk_type)(s.png);
                                if name == PNG_IDAT {
                                    log.push((name, len, 0, 0, Vec::new()));
                                    break;
                                }
                                let keep = (api.png_chunk_unknown_handling)(s.png, name);
                                let mut data = Vec::new();
                                let mut crcres = 0;
                                let mut handled = 0;
                                if name == PNG_IHDR || name == PNG_PLTE || name == PNG_IEND {
                                    handled = (api.png_handle_chunk)(s.png, s.info, len);
                                } else if keep != 0 {
                                    handled =
                                        (api.png_handle_unknown)(s.png, s.info, len, keep);
                                } else if manual_crc {
                                    data = vec![0u8; len as usize];
                                    if len > 0 {
                                        (api.png_crc_read)(s.png, data.as_mut_ptr(), len);
                                    }
                                    crcres = (api.png_crc_finish)(s.png, 0);
                                } else {
                                    handled = (api.png_handle_chunk)(s.png, s.info, len);
                                }
                                log.push((name, len, keep, handled + crcres * 16, data));
                                if name == PNG_IEND {
                                    break;
                                }
                            }
                            let mut w = 0;
                            let mut h = 0;
                            let mut b = 0;
                            let mut c = 0;
                            let mut i = 0;
                            let mut cm = 0;
                            let mut fm = 0;
                            (api.png_get_IHDR)(
                                s.png, s.info, &mut w, &mut h, &mut b, &mut c, &mut i,
                                &mut cm, &mut fm,
                            );
                            ihdr = (w, h, b, c, i);
                        })
                        .is_some();
                        outs.push((ok, diag_take(), log, ihdr));
                    }
                }
                assert_eq!(
                    outs[0], outs[1],
                    "low level chunk read ct={} bd={} il={} manual_crc={}",
                    ct, bd, il, manual_crc
                );
            }
        }
    }
}

/// Write the chunk layer by hand with the exported low-level writers.
#[test]
fn low_level_chunk_write_driver() {
    let mut rng = Rng::new(0x6f70_8192_a3b4_c006);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let npal = if ct == PNG_COLOR_TYPE_PALETTE {
                1usize << bd
            } else {
                0
            };
            let palette: Vec<png_color> = (0..npal)
                .map(|_| png_color {
                    red: rng.u8(),
                    green: rng.u8(),
                    blue: rng.u8(),
                })
                .collect();
            let extra = rng.bytes(7);
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    let mut s = WriteSess::new(api);
                    let ok = guard(|| {
                        (api.png_write_sig)(s.png);
                        (api.png_write_IHDR)(s.png, 13, 5, bd, ct, 0, 0, il);
                        if !palette.is_empty() {
                            (api.png_write_PLTE)(
                                s.png,
                                palette.as_ptr(),
                                palette.len() as png_uint_32,
                            );
                        }
                        (api.png_write_gAMA_fixed)(s.png, 45455);
                        (api.png_write_sRGB)(s.png, 0);
                        (api.png_write_pHYs)(s.png, 300, 300, 1);
                        (api.png_write_oFFs)(s.png, -5, 7, 0);
                        (api.png_write_tIME)(
                            s.png,
                            &png_time {
                                year: 2024,
                                month: 6,
                                day: 1,
                                hour: 12,
                                minute: 0,
                                second: 0,
                            } as *const _ as png_const_timep,
                        );
                        // a raw chunk written in one call and in three
                        let nm = cs("prVt");
                        (api.png_write_chunk)(
                            s.png,
                            nm.as_ptr() as png_const_bytep,
                            extra.as_ptr(),
                            extra.len(),
                        );
                        (api.png_write_chunk_start)(
                            s.png,
                            nm.as_ptr() as png_const_bytep,
                            extra.len() as png_uint_32,
                        );
                        (api.png_write_chunk_data)(s.png, extra.as_ptr(), extra.len());
                        (api.png_write_chunk_end)(s.png);
                        (api.png_write_IEND)(s.png);
                    })
                    .is_some();
                    outs.push((ok, diag_take(), std::mem::take(&mut s.sink.buf)));
                }
            }
            assert_eq!(outs[0].0, outs[1].0, "low level write parity");
            assert_eq!(outs[0].1, outs[1].1, "low level write diag");
            assert_bytes_eq(
                &format!("low level chunk write ct={} bd={} il={}", ct, bd, il),
                &outs[0].2,
                &outs[1].2,
            );
        }
    }
}

#[test]
fn write_info_before_plte_and_find_filter() {
    let mut rng = Rng::new(0x7081_92a3_b4c5_d007);
    for (ct, bd) in legal_ihdr() {
        for order in 0..3 {
            let npal = if ct == PNG_COLOR_TYPE_PALETTE {
                1usize << bd
            } else {
                0
            };
            let palette: Vec<png_color> = (0..npal)
                .map(|_| png_color {
                    red: rng.u8(),
                    green: rng.u8(),
                    blue: rng.u8(),
                })
                .collect();
            let pd = channels_of(ct) * bd as u32;
            let rows: Vec<Vec<u8>> = (0..4).map(|_| rng.bytes(rowbytes(pd, 13) + 16)).collect();
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    let mut s = WriteSess::new(api);
                    let ok = guard(|| {
                        (api.png_set_IHDR)(s.png, s.info, 13, 4, bd, ct, 0, 0, 0);
                        if !palette.is_empty() {
                            (api.png_set_PLTE)(
                                s.png,
                                s.info,
                                palette.as_ptr(),
                                palette.len() as c_int,
                            );
                        }
                        (api.png_set_gAMA_fixed)(s.png, s.info, 45455);
                        match order {
                            0 => {
                                (api.png_write_info_before_PLTE)(s.png, s.info);
                                (api.png_write_info)(s.png, s.info);
                            }
                            1 => {
                                // calling it twice must be idempotent
                                (api.png_write_info_before_PLTE)(s.png, s.info);
                                (api.png_write_info_before_PLTE)(s.png, s.info);
                                (api.png_write_info)(s.png, s.info);
                            }
                            _ => {
                                (api.png_write_info)(s.png, s.info);
                            }
                        }
                        (api.png_set_filter)(s.png, PNG_FILTER_TYPE_BASE, PNG_ALL_FILTERS);
                        let mut rp: Vec<png_bytep> =
                            rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                        (api.png_write_image)(s.png, rp.as_mut_ptr());
                        (api.png_write_end)(s.png, s.info);
                    })
                    .is_some();
                    outs.push((ok, diag_take(), std::mem::take(&mut s.sink.buf)));
                }
            }
            assert_eq!(outs[0].0, outs[1].0, "before_PLTE parity order={}", order);
            assert_eq!(outs[0].1, outs[1].1, "before_PLTE diag order={}", order);
            assert_bytes_eq(
                &format!("before_PLTE ct={} bd={} order={}", ct, bd, order),
                &outs[0].2,
                &outs[1].2,
            );
        }
    }
}

#[test]
fn create_and_destroy_png_struct_directly() {
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let v = ver();
            let p = (api.png_create_png_struct)(
                v.as_ptr(),
                std::ptr::null_mut(),
                Some(cb_error),
                Some(cb_warning),
                std::ptr::null_mut(),
                None,
                None,
            );
            assert!(!p.is_null(), "{}: png_create_png_struct", api.name);
            let i = (api.png_create_info_struct)(p);
            assert!(!i.is_null());
            let mut ii = i;
            (api.png_destroy_info_struct)(p, &mut ii);
            (api.png_destroy_png_struct)(p);
            // a bad version string must be rejected identically
            let bad = cs("0.0.0");
            let q = (api.png_create_png_struct)(
                bad.as_ptr(),
                std::ptr::null_mut(),
                Some(cb_error),
                Some(cb_warning),
                std::ptr::null_mut(),
                None,
                None,
            );
            let d = diag_take();
            if !q.is_null() {
                (api.png_destroy_png_struct)(q);
            }
            if api.name == "C" {
                CS2.with(|c| *c.borrow_mut() = Some((q.is_null(), d)));
            } else {
                CS2.with(|c| {
                    assert_eq!(
                        c.borrow().clone().unwrap(),
                        (q.is_null(), d),
                        "png_create_png_struct version rejection"
                    )
                });
            }
            // NULL guard
            (api.png_destroy_png_struct)(std::ptr::null_mut());
        }
    }
}

thread_local! {
    static CS2: std::cell::RefCell<Option<(bool, Diag)>> =
        const { std::cell::RefCell::new(None) };
}

#[test]
fn safe_execute_and_image_error() {
    unsafe extern "C-unwind" fn ok_fn(arg: png_voidp) -> c_int {
        (arg as *mut c_int).write(11);
        1
    }
    unsafe extern "C-unwind" fn fail_fn(arg: png_voidp) -> c_int {
        (arg as *mut c_int).write(22);
        0
    }
    // NOTE: png_safe_execute dereferences image->opaque->error_buf with no NULL
    // check (pngerror.c:817), so it can only be called on an image whose opaque
    // control block already exists -- i.e. after png_image_begin_read_*.
    let mut rng = Rng::new(0xa3b4_c5d6_e7f8_0910);
    let good = unsafe {
        make_png(
            &mut rng,
            PNG_COLOR_TYPE_RGB,
            8,
            5,
            3,
            PNG_INTERLACE_NONE,
            false,
        )
    };
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let mut img = png_image::default();
            let begun = (api.png_image_begin_read_from_memory)(
                &mut img,
                good.as_ptr() as png_const_voidp,
                good.len(),
            );
            assert_ne!(begun, 0, "{}: begin_read_from_memory", api.name);
            let mut cell: c_int = 0;
            let a = (api.png_safe_execute)(
                &mut img,
                Some(ok_fn),
                &mut cell as *mut c_int as png_voidp,
            );
            let v1 = (a, cell, img.warning_or_error, img.opaque.is_null());
            let b = (api.png_safe_execute)(
                &mut img,
                Some(fail_fn),
                &mut cell as *mut c_int as png_voidp,
            );
            // a false return frees the opaque state
            let v2 = (b, cell, img.warning_or_error, img.opaque.is_null());
            // png_image_error records the message and frees the opaque state
            let msg = cs("recorded failure");
            let c = (api.png_image_error)(&mut img, msg.as_ptr());
            let recorded = (
                c,
                img.warning_or_error,
                CStrLike(img.message),
                img.opaque.is_null(),
            );
            (api.png_image_free)(&mut img);
            // png_image_free is idempotent and NULL-safe
            (api.png_image_free)(&mut img);
            (api.png_image_free)(std::ptr::null_mut());
            let d = diag_take();
            if api.name == "C" {
                CS3.with(|x| *x.borrow_mut() = Some((v1, v2, recorded, d)));
            } else {
                CS3.with(|x| {
                    assert_eq!(
                        x.borrow().clone().unwrap(),
                        (v1, v2, recorded, d),
                        "png_safe_execute / png_image_error"
                    )
                });
            }
        }
    }
}

#[derive(Clone)]
struct CStrLike([c_char; 64]);
impl PartialEq for CStrLike {
    fn eq(&self, o: &Self) -> bool {
        self.0[..] == o.0[..]
    }
}
impl std::fmt::Debug for CStrLike {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let b: Vec<u8> = self.0.iter().map(|&c| c as u8).collect();
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        write!(f, "{:?}", String::from_utf8_lossy(&b[..end]))
    }
}

type Snap3 = (
    (c_int, c_int, u32, bool),
    (c_int, c_int, u32, bool),
    (c_int, u32, CStrLike, bool),
    Diag,
);
thread_local! {
    static CS3: std::cell::RefCell<Option<Snap3>> = const { std::cell::RefCell::new(None) };
}

/// `png_default_read_data` / `png_default_write_data` / `png_default_flush`
/// need a real `FILE*`; drive them through `png_init_io`.
#[test]
fn default_stdio_callbacks() {
    let mut rng = Rng::new(0x8192_a3b4_c5d6_e008);
    let bytes = unsafe {
        make_png(
            &mut rng,
            PNG_COLOR_TYPE_RGB_ALPHA,
            8,
            11,
            5,
            PNG_INTERLACE_NONE,
            true,
        )
    };
    let dir = std::env::temp_dir();
    // --- write to a real file with the default stdio writer ---
    let mut wouts = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let path = dir.join(format!("pngdiff_{}_{}.png", api.name, std::process::id()));
            let cpath = CString::new(path.to_str().unwrap()).unwrap();
            let mode = cs("wb");
            let fp = fopen(cpath.as_ptr(), mode.as_ptr());
            assert!(!fp.is_null(), "fopen for write");
            let v = ver();
            let png = (api.png_create_write_struct)(
                v.as_ptr(),
                std::ptr::null_mut(),
                Some(cb_error),
                Some(cb_warning),
            );
            let info = (api.png_create_info_struct)(png);
            let mut r2 = Rng::new(0x4242_0000_0000_0001);
            let rows: Vec<Vec<u8>> = (0..5).map(|_| r2.bytes(11 * 4)).collect();
            let ok = guard(|| {
                (api.png_init_io)(png, fp);
                (api.png_set_IHDR)(
                    png,
                    info,
                    11,
                    5,
                    8,
                    PNG_COLOR_TYPE_RGB_ALPHA,
                    PNG_INTERLACE_NONE,
                    0,
                    0,
                );
                (api.png_set_flush)(png, 2);
                (api.png_write_info)(png, info);
                let mut rp: Vec<png_bytep> =
                    rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                for r in rp.iter() {
                    (api.png_write_row)(png, *r);
                }
                (api.png_write_flush)(png);
                (api.png_write_end)(png, info);
            })
            .is_some();
            let mut p = png;
            let mut i = info;
            (api.png_destroy_write_struct)(&mut p, &mut i);
            fclose(fp);
            let written = std::fs::read(&path).unwrap_or_default();
            let _ = std::fs::remove_file(&path);
            wouts.push((ok, diag_take(), written));
        }
    }
    assert_eq!(wouts[0].0, wouts[1].0, "stdio write parity");
    assert_eq!(wouts[0].1, wouts[1].1, "stdio write diag");
    assert_bytes_eq("stdio write bytes", &wouts[0].2, &wouts[1].2);

    // --- read the same file back with the default stdio reader ---
    let src = dir.join(format!("pngdiff_src_{}.png", std::process::id()));
    std::fs::write(&src, &bytes).unwrap();
    let mut routs = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let cpath = CString::new(src.to_str().unwrap()).unwrap();
            let mode = cs("rb");
            let fp = fopen(cpath.as_ptr(), mode.as_ptr());
            assert!(!fp.is_null(), "fopen for read");
            let v = ver();
            let png = (api.png_create_read_struct)(
                v.as_ptr(),
                std::ptr::null_mut(),
                Some(cb_error),
                Some(cb_warning),
            );
            let info = (api.png_create_info_struct)(png);
            let mut rows: Vec<Vec<u8>> = Vec::new();
            let ok = guard(|| {
                (api.png_init_io)(png, fp);
                (api.png_read_info)(png, info);
                let rbz = (api.png_get_rowbytes)(png, info);
                let h = (api.png_get_image_height)(png, info);
                let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                let mut ptrs: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                (api.png_read_image)(png, ptrs.as_mut_ptr());
                (api.png_read_end)(png, std::ptr::null_mut());
                rows = buf;
            })
            .is_some();
            let mut p = png;
            let mut i = info;
            (api.png_destroy_read_struct)(&mut p, &mut i, std::ptr::null_mut());
            fclose(fp);
            routs.push((ok, diag_take(), rows));
        }
    }
    let _ = std::fs::remove_file(&src);
    assert_eq!(routs[0], routs[1], "stdio read");
}

#[test]
fn longjmp_and_fixed_error() {
    unsafe extern "C-unwind" fn panic_longjmp(_e: *mut jmp_buf, v: c_int) -> ! {
        std::panic::panic_any(LongJmp(v))
    }
    struct LongJmp(c_int);

    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let s = ReadSess::new(api, &[]);
            let jb = (api.png_set_longjmp_fn)(s.png, Some(panic_longjmp), 200);
            assert!(!jb.is_null(), "{}: set_longjmp_fn", api.name);
            for v in [1i32, 2, -1, 0, 12345] {
                let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    (api.png_longjmp)(s.png, v)
                }));
                match caught {
                    Err(e) => {
                        let got = e.downcast_ref::<LongJmp>().map(|l| l.0);
                        assert_eq!(got, Some(v), "{}: png_longjmp value", api.name);
                    }
                    Ok(_) => unreachable!(),
                }
            }
            (api.png_free_jmpbuf)(s.png);
            let name = cs("gamma value");
            let r = guard(|| (api.png_fixed_error)(s.png, name.as_ptr()));
            assert!(r.is_none(), "{}: png_fixed_error must not return", api.name);
            let d = diag_take();
            if api.name == "C" {
                CS4.with(|x| *x.borrow_mut() = Some(d));
            } else {
                CS4.with(|x| {
                    assert_eq!(x.borrow().clone().unwrap(), d, "png_fixed_error message")
                });
            }
        }
    }
}

thread_local! {
    static CS4: std::cell::RefCell<Option<Diag>> = const { std::cell::RefCell::new(None) };
}

#[test]
fn free_buffer_list_direct() {
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let s = WriteSess::new(api);
            let mut list: png_compression_bufferp = std::ptr::null_mut();
            (api.png_free_buffer_list)(s.png, &mut list);
            assert!(list.is_null(), "{}", api.name);
            // and again, on an already-empty list
            (api.png_free_buffer_list)(s.png, &mut list);
            assert!(list.is_null(), "{}", api.name);
            let _ = diag_take();
        }
    }
}

#[test]
fn safe_error_and_warning_hooks() {
    // png_safe_error / png_safe_warning are the error handlers the simplified
    // API installs; they require png_ptr->error_ptr to be a png_control.  They
    // are reached through png_image_finish_read on a broken stream, which is
    // what this drives (calling them with a hand-made png_struct would be UB).
    let mut rng = Rng::new(0x92a3_b4c5_d6e7_f009);
    let good = unsafe {
        make_png(
            &mut rng,
            PNG_COLOR_TYPE_RGB,
            8,
            9,
            4,
            PNG_INTERLACE_NONE,
            true,
        )
    };
    for cut in [8usize, 20, 33, 45, good.len() - 1] {
        let mut outs = Vec::new();
        for api in both() {
            unsafe {
                set_current_api(api);
                diag_reset();
                let mut img = png_image::default();
                let data = &good[..cut.min(good.len())];
                let r = (api.png_image_begin_read_from_memory)(
                    &mut img,
                    data.as_ptr() as png_const_voidp,
                    data.len(),
                );
                let mut r2 = 0;
                let mut buf: Vec<u8> = Vec::new();
                if r != 0 {
                    img.format = PNG_FORMAT_RGBA;
                    buf = vec![0u8; (img.width as usize) * (img.height as usize) * 4 + 64];
                    r2 = (api.png_image_finish_read)(
                        &mut img,
                        std::ptr::null(),
                        buf.as_mut_ptr() as png_voidp,
                        0,
                        std::ptr::null_mut(),
                    );
                }
                (api.png_image_free)(&mut img);
                outs.push((
                    r,
                    r2,
                    img.warning_or_error,
                    CStrLike(img.message),
                    buf,
                    diag_take(),
                ));
            }
        }
        assert_eq!(outs[0], outs[1], "simplified read truncated at {}", cut);
    }
}

/// `png_read_start_row` is what `png_start_read_image` calls; drive it directly
/// (the wrapper only adds the duplicate-call check).  `png_do_read_transformations`
/// operates on `png_ptr->row_buf`, so like its write twin it is only callable
/// after at least one row has been read.
#[test]
fn read_start_row_and_do_read_transformations() {
    let mut rng = Rng::new(0xb4c5_d6e7_f809_1a0b);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let bytes = unsafe { make_png(&mut rng, ct, bd, 11, 4, il, true) };
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    let s = ReadSess::new(api, &bytes);
                    let mut rows: Vec<Vec<u8>> = Vec::new();
                    let mut ri_after = png_row_info::default();
                    let ok = guard(|| {
                        (api.png_read_info)(s.png, s.info);
                        (api.png_set_expand)(s.png);
                        // png_read_start_row IN PLACE OF png_start_read_image /
                        // png_read_update_info (the wrapper only adds the
                        // duplicate-call check).  info_ptr->rowbytes is
                        // therefore NOT updated, so size the buffers for the
                        // largest possible transformed pixel depth.
                        (api.png_read_start_row)(s.png);
                        let w = (api.png_get_image_width)(s.png, s.info);
                        let rbz = rowbytes(64, w) + 32;
                        let h = (api.png_get_image_height)(s.png, s.info);
                        let np = if il == PNG_INTERLACE_ADAM7 {
                            (api.png_set_interlace_handling)(s.png)
                        } else {
                            1
                        };
                        let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz]).collect();
                        for _ in 0..np {
                            for y in 0..h as usize {
                                (api.png_read_row)(
                                    s.png,
                                    buf[y].as_mut_ptr(),
                                    std::ptr::null_mut(),
                                );
                            }
                        }
                        rows = buf;
                        let mut ri = png_row_info {
                            width: 11,
                            rowbytes: rbz,
                            color_type: (api.png_get_color_type)(s.png, s.info),
                            bit_depth: (api.png_get_bit_depth)(s.png, s.info),
                            channels: (api.png_get_channels)(s.png, s.info),
                            pixel_depth: ((api.png_get_channels)(s.png, s.info) as u32
                                * (api.png_get_bit_depth)(s.png, s.info) as u32)
                                as u8,
                        };
                        (api.png_do_read_transformations)(s.png, &mut ri);
                        ri_after = ri;
                    })
                    .is_some();
                    outs.push((ok, diag_take(), rows, ri_after));
                }
            }
            assert_eq!(
                outs[0], outs[1],
                "read_start_row / do_read_transformations ct={} bd={} il={}",
                ct, bd, il
            );
        }
    }
    // NOTE: png_read_start_row is a private entry point with no NULL check
    // (pngrutil.c), so a NULL png_ptr there is C undefined behaviour.
}

/// `png_read_data` / `png_write_data` and the stdio defaults behind them,
/// called directly with a real `FILE*` installed by `png_init_io`.
#[test]
fn raw_io_entry_points() {
    let dir = std::env::temp_dir();
    let payload: Vec<u8> = (0u16..600).map(|i| (i * 7 % 251) as u8).collect();

    // --- png_write_data / png_default_write_data / png_default_flush ---
    let mut wouts = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let path = dir.join(format!("pngraw_w_{}_{}.bin", api.name, std::process::id()));
            let cpath = CString::new(path.to_str().unwrap()).unwrap();
            let mode = cs("wb");
            let fp = fopen(cpath.as_ptr(), mode.as_ptr());
            assert!(!fp.is_null());
            let v = ver();
            let png = (api.png_create_write_struct)(
                v.as_ptr(),
                std::ptr::null_mut(),
                Some(cb_error),
                Some(cb_warning),
            );
            let ok = guard(|| {
                (api.png_init_io)(png, fp);
                // through the dispatcher ...
                (api.png_write_data)(png, payload.as_ptr() as png_bytep, 100);
                // ... and straight into the stdio default
                (api.png_default_write_data)(
                    png,
                    payload.as_ptr().add(100) as png_bytep,
                    200,
                );
                (api.png_default_flush)(png);
                (api.png_write_data)(png, payload.as_ptr().add(300) as png_bytep, 0);
                (api.png_flush)(png);
            })
            .is_some();
            let mut p = png;
            (api.png_destroy_write_struct)(&mut p, std::ptr::null_mut());
            fclose(fp);
            let got = std::fs::read(&path).unwrap_or_default();
            let _ = std::fs::remove_file(&path);
            wouts.push((ok, diag_take(), got));
        }
    }
    assert_eq!(wouts[0].0, wouts[1].0, "raw write parity");
    assert_eq!(wouts[0].1, wouts[1].1, "raw write diag");
    assert_bytes_eq("raw write bytes", &wouts[0].2, &wouts[1].2);
    assert_eq!(wouts[0].2, payload[..300], "raw write content");

    // --- png_read_data / png_default_read_data ---
    let src = dir.join(format!("pngraw_r_{}.bin", std::process::id()));
    std::fs::write(&src, &payload).unwrap();
    let mut routs = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let cpath = CString::new(src.to_str().unwrap()).unwrap();
            let mode = cs("rb");
            let fp = fopen(cpath.as_ptr(), mode.as_ptr());
            assert!(!fp.is_null());
            let v = ver();
            let png = (api.png_create_read_struct)(
                v.as_ptr(),
                std::ptr::null_mut(),
                Some(cb_error),
                Some(cb_warning),
            );
            let mut buf = vec![0u8; 700];
            let ok = guard(|| {
                (api.png_init_io)(png, fp);
                (api.png_read_data)(png, buf.as_mut_ptr(), 100);
                (api.png_default_read_data)(png, buf.as_mut_ptr().add(100), 200);
                (api.png_read_data)(png, buf.as_mut_ptr().add(300), 0);
            })
            .is_some();
            // reading past the end of the file must fail identically
            let ok2 = guard(|| {
                (api.png_read_data)(png, buf.as_mut_ptr().add(300), 400);
            })
            .is_some();
            let mut p = png;
            (api.png_destroy_read_struct)(
                &mut p,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            fclose(fp);
            routs.push((ok, ok2, diag_take(), buf));
        }
    }
    let _ = std::fs::remove_file(&src);
    assert_eq!(routs[0].0, routs[1].0, "raw read parity");
    assert_eq!(routs[0].1, routs[1].1, "raw read overrun parity");
    assert_eq!(routs[0].2, routs[1].2, "raw read diag");
    assert_eq!(routs[0].3, routs[1].3, "raw read buffer");
    assert_eq!(&routs[0].3[..300], &payload[..300], "raw read content");
}

// The remaining exported internals are only reachable from inside the composed
// pipeline, where they are exercised by the other test files:
//   png_read_IDAT_data, png_read_finish_IDAT, png_read_finish_row,
//   png_compress_IDAT, png_zlib_inflate  -- need a live, mid-IDAT zstream;
//   png_process_IDAT_data, png_push_have_info/_end/_row, png_push_process_row,
//   png_read_push_finish_row            -- driven by tests/t06_progressive.rs;
//   png_write_start_row                 -- png_write_row calls it on the first
//                                          row; calling it again double-allocates;
//   png_safe_error, png_safe_warning    -- dereference png_ptr->error_ptr as a
//                                          png_control, so they are only valid
//                                          for a simplified-API png_struct
//                                          (tests/t07_simplified.rs and
//                                          safe_error_and_warning_hooks above).

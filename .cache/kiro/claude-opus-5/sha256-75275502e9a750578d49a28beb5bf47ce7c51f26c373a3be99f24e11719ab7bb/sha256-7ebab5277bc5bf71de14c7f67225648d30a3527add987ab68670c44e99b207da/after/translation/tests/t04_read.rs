//! Tier 4: whole-file reads.  Both libraries decode the same byte stream; the
//! decoded rows, every `png_get_*` value and every warning must agree.

mod common;
use common::*;
use std::ffi::{c_char, c_int, CString};

/* ------------------------------------------------------- corpus building */

fn rowbytes(pixel_depth: u32, width: u32) -> usize {
    if pixel_depth >= 8 {
        (width as usize) * ((pixel_depth as usize) >> 3)
    } else {
        ((width as usize) * (pixel_depth as usize) + 7) >> 3
    }
}

fn channels(ct: u8) -> u32 {
    match ct {
        PNG_COLOR_TYPE_GRAY | PNG_COLOR_TYPE_PALETTE => 1,
        PNG_COLOR_TYPE_GRAY_ALPHA => 2,
        PNG_COLOR_TYPE_RGB => 3,
        PNG_COLOR_TYPE_RGB_ALPHA => 4,
        _ => unreachable!(),
    }
}

fn formats() -> Vec<(u8, u8)> {
    let mut v = Vec::new();
    for &ct in &[
        PNG_COLOR_TYPE_GRAY,
        PNG_COLOR_TYPE_PALETTE,
        PNG_COLOR_TYPE_RGB,
        PNG_COLOR_TYPE_GRAY_ALPHA,
        PNG_COLOR_TYPE_RGB_ALPHA,
    ] {
        for &bd in &[1u8, 2, 4, 8, 16] {
            let ok = match ct {
                PNG_COLOR_TYPE_GRAY => true,
                PNG_COLOR_TYPE_PALETTE => bd <= 8,
                _ => bd == 8 || bd == 16,
            };
            if ok {
                v.push((ct, bd));
            }
        }
    }
    v
}

#[derive(Clone, Copy, Debug)]
struct Img {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    interlace: c_int,
    /// include the full set of ancillary chunks
    rich: bool,
    seed: u32,
}

impl Default for Img {
    fn default() -> Self {
        Img {
            width: 11,
            height: 7,
            bit_depth: 8,
            color_type: PNG_COLOR_TYPE_RGB,
            interlace: PNG_INTERLACE_NONE,
            rich: false,
            seed: 3,
        }
    }
}

/// Encode an image with the *C* library so that both readers see identical
/// input bytes.
fn encode(img: &Img) -> Vec<u8> {
    let pd = channels(img.color_type) * img.bit_depth as u32;
    let rb = rowbytes(pd, img.width);
    let mut s = img.seed | 1;
    let rows: Vec<Vec<u8>> = (0..img.height)
        .map(|_| {
            (0..rb)
                .map(|_| {
                    s = s.wrapping_mul(1103515245).wrapping_add(12345);
                    (s >> 16) as u8
                })
                .collect()
        })
        .collect();

    let out = write_with(&libs().c, |c, _| {
        let png = c.png;
        let info = c.info;
        let mut keep: Vec<CString> = Vec::new();
        type Fihdr = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            u32,
            u32,
            c_int,
            c_int,
            c_int,
            c_int,
            c_int,
        );
        let f: libloading::Symbol<Fihdr> = c.sym("png_set_IHDR");
        unsafe {
            f(
                png,
                info,
                img.width,
                img.height,
                img.bit_depth as c_int,
                img.color_type as c_int,
                img.interlace,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            )
        };

        let npal = 1usize << img.bit_depth.min(8);
        if img.color_type == PNG_COLOR_TYPE_PALETTE {
            let pal: Vec<png_color> = (0..npal)
                .map(|i| png_color {
                    red: (i * 7 % 256) as u8,
                    green: (i * 13 % 256) as u8,
                    blue: (i * 29 % 256) as u8,
                })
                .collect();
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color, c_int),
            > = c.sym("png_set_PLTE");
            unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
        }

        if img.rich {
            {
                let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, i32)> =
                    c.sym("png_set_gAMA_fixed");
                unsafe { g(png, info, 45455) };
            }
            {
                type F = unsafe extern "C-unwind" fn(
                    png_structp, png_infop, i32, i32, i32, i32, i32, i32, i32, i32,
                );
                let g: libloading::Symbol<F> = c.sym("png_set_cHRM_fixed");
                unsafe { g(png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000) };
            }
            {
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color_8),
                > = c.sym("png_set_sBIT");
                let bd = img.bit_depth.min(8);
                let sb = match img.color_type {
                    PNG_COLOR_TYPE_GRAY => png_color_8 { gray: bd, ..Default::default() },
                    PNG_COLOR_TYPE_PALETTE => {
                        png_color_8 { red: 8, green: 8, blue: 8, ..Default::default() }
                    }
                    PNG_COLOR_TYPE_RGB => {
                        png_color_8 { red: bd, green: bd, blue: bd, ..Default::default() }
                    }
                    PNG_COLOR_TYPE_GRAY_ALPHA => {
                        png_color_8 { gray: bd, alpha: bd, ..Default::default() }
                    }
                    _ => png_color_8 { red: bd, green: bd, blue: bd, alpha: bd, ..Default::default() },
                };
                unsafe { g(png, info, &sb) };
            }
            {
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, u32, u32, c_int),
                > = c.sym("png_set_pHYs");
                unsafe { g(png, info, 3000, 2500, 1) };
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, i32, i32, c_int),
                > = c.sym("png_set_oFFs");
                unsafe { g(png, info, -17, 42, 0) };
            }
            {
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_time),
                > = c.sym("png_set_tIME");
                let t = png_time { year: 2024, month: 2, day: 29, hour: 13, minute: 45, second: 7 };
                unsafe { g(png, info, &t) };
            }
            {
                let w = CString::new("3.14159").unwrap();
                let h = CString::new("2.71828e2").unwrap();
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(
                        png_structp,
                        png_infop,
                        c_int,
                        *const c_char,
                        *const c_char,
                    ),
                > = c.sym("png_set_sCAL_s");
                unsafe { g(png, info, 1, w.as_ptr(), h.as_ptr()) };
                keep.push(w);
                keep.push(h);
            }
            {
                let purpose = CString::new("calibration").unwrap();
                let units = CString::new("metres").unwrap();
                let p0 = CString::new("1.5").unwrap();
                let p1 = CString::new("-2.25e-3").unwrap();
                let mut params: Vec<*mut c_char> =
                    vec![p0.as_ptr() as *mut c_char, p1.as_ptr() as *mut c_char];
                type F = unsafe extern "C-unwind" fn(
                    png_structp,
                    png_infop,
                    *const c_char,
                    i32,
                    i32,
                    c_int,
                    c_int,
                    *const c_char,
                    *mut *mut c_char,
                );
                let g: libloading::Symbol<F> = c.sym("png_set_pCAL");
                unsafe {
                    g(png, info, purpose.as_ptr(), -100, 1000, 2, 2, units.as_ptr(), params.as_mut_ptr())
                };
                keep.push(purpose);
                keep.push(units);
                keep.push(p0);
                keep.push(p1);
            }
            {
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, u8, u8, u8, u8),
                > = c.sym("png_set_cICP");
                unsafe { g(png, info, 9, 16, 0, 1) };
                let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, u32, u32)> =
                    c.sym("png_set_cLLI_fixed");
                unsafe { g(png, info, 10_000_000, 4_000_000) };
                type Fm = unsafe extern "C-unwind" fn(
                    png_structp, png_infop, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32,
                );
                let g: libloading::Symbol<Fm> = c.sym("png_set_mDCV_fixed");
                unsafe {
                    g(png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10000000, 50)
                };
            }
            {
                let exif: Vec<u8> = b"II\x2a\x00\x08\x00\x00\x00\x00\x00".to_vec();
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, u32, *const u8),
                > = c.sym("png_set_eXIf_1");
                unsafe { g(png, info, exif.len() as u32, exif.as_ptr()) };
            }
            {
                let prof = make_icc_profile();
                let name = CString::new("ICC profile").unwrap();
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(
                        png_structp,
                        png_infop,
                        *const c_char,
                        c_int,
                        *const u8,
                        u32,
                    ),
                > = c.sym("png_set_iCCP");
                unsafe { g(png, info, name.as_ptr(), 0, prof.as_ptr(), prof.len() as u32) };
                keep.push(name);
            }
            {
                let name = CString::new("suggested").unwrap();
                let entries: Vec<png_sPLT_entry> = (0..8u16)
                    .map(|i| png_sPLT_entry {
                        red: i * 1000,
                        green: i * 2000,
                        blue: i * 3000,
                        alpha: i * 4000,
                        frequency: i,
                    })
                    .collect();
                let sp = png_sPLT_t {
                    name: name.as_ptr() as *mut c_char,
                    depth: 16,
                    entries: entries.as_ptr() as *mut png_sPLT_entry,
                    nentries: entries.len() as i32,
                };
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_sPLT_t, c_int),
                > = c.sym("png_set_sPLT");
                unsafe { g(png, info, &sp, 1) };
                keep.push(name);
            }
            // bKGD and tRNS
            {
                let maxv = if img.bit_depth == 16 { 65535u16 } else { (1u16 << img.bit_depth) - 1 };
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color_16),
                > = c.sym("png_set_bKGD");
                let bg = if img.color_type == PNG_COLOR_TYPE_PALETTE {
                    png_color_16 { index: (npal as u16 - 1).min(3) as u8, ..Default::default() }
                } else if (img.color_type & PNG_COLOR_MASK_COLOR) != 0 {
                    png_color_16 { red: maxv / 2, green: maxv / 3, blue: maxv / 4, ..Default::default() }
                } else {
                    png_color_16 { gray: maxv / 2, ..Default::default() }
                };
                unsafe { g(png, info, &bg) };

                if (img.color_type & PNG_COLOR_MASK_ALPHA) == 0 {
                    type F = unsafe extern "C-unwind" fn(
                        png_structp,
                        png_infop,
                        *const u8,
                        c_int,
                        *const png_color_16,
                    );
                    let g: libloading::Symbol<F> = c.sym("png_set_tRNS");
                    if img.color_type == PNG_COLOR_TYPE_PALETTE {
                        let trans: Vec<u8> = (0..npal).map(|i| (i * 17 % 256) as u8).collect();
                        unsafe { g(png, info, trans.as_ptr(), npal as c_int, std::ptr::null()) };
                    } else {
                        let tc = if (img.color_type & PNG_COLOR_MASK_COLOR) != 0 {
                            png_color_16 { red: 1, green: 2, blue: 3, ..Default::default() }
                        } else {
                            png_color_16 { gray: maxv / 5, ..Default::default() }
                        };
                        unsafe { g(png, info, std::ptr::null(), 0, &tc) };
                    }
                }
            }
            if img.color_type == PNG_COLOR_TYPE_PALETTE {
                let hist: Vec<u16> = (0..npal).map(|i| (i * 3 % 65536) as u16).collect();
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, *const u16),
                > = c.sym("png_set_hIST");
                unsafe { g(png, info, hist.as_ptr()) };
            }
            // text chunks, all four flavours
            {
                let key0 = CString::new("Title").unwrap();
                let txt0 = CString::new("plain text").unwrap();
                let key1 = CString::new("Description").unwrap();
                let txt1 = CString::new(
                    "a longer description repeated to be worth compressing, \
                     a longer description repeated to be worth compressing",
                )
                .unwrap();
                let key2 = CString::new("Comment").unwrap();
                let txt2 = CString::new("international").unwrap();
                let lang2 = CString::new("en-GB").unwrap();
                let lk2 = CString::new("Comment").unwrap();
                let texts = [
                    png_text {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: key0.as_ptr() as *mut c_char,
                        text: txt0.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: std::ptr::null_mut(),
                        lang_key: std::ptr::null_mut(),
                    },
                    png_text {
                        compression: PNG_TEXT_COMPRESSION_zTXt,
                        key: key1.as_ptr() as *mut c_char,
                        text: txt1.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: std::ptr::null_mut(),
                        lang_key: std::ptr::null_mut(),
                    },
                    png_text {
                        compression: PNG_ITXT_COMPRESSION_NONE,
                        key: key2.as_ptr() as *mut c_char,
                        text: txt2.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: lang2.as_ptr() as *mut c_char,
                        lang_key: lk2.as_ptr() as *mut c_char,
                    },
                ];
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_text, c_int),
                > = c.sym("png_set_text");
                unsafe { g(png, info, texts.as_ptr(), texts.len() as c_int) };
                keep.push(key0);
                keep.push(txt0);
                keep.push(key1);
                keep.push(txt1);
                keep.push(key2);
                keep.push(txt2);
                keep.push(lang2);
                keep.push(lk2);
            }
            // unknown chunks in all three positions
            {
                const PNG_HAVE_IHDR: u8 = 0x01;
                const PNG_HAVE_PLTE: u8 = 0x02;
                const PNG_AFTER_IDAT: u8 = 0x08;
                let d0: Vec<u8> = (0u8..24).collect();
                let d1: Vec<u8> = vec![0xaa; 5];
                let chunks = [
                    png_unknown_chunk {
                        name: *b"prVt\0",
                        data: d0.as_ptr() as *mut u8,
                        size: d0.len(),
                        location: PNG_HAVE_IHDR,
                    },
                    png_unknown_chunk {
                        name: *b"seCd\0",
                        data: d1.as_ptr() as *mut u8,
                        size: d1.len(),
                        location: PNG_HAVE_PLTE,
                    },
                    png_unknown_chunk {
                        name: *b"aftR\0",
                        data: d1.as_ptr() as *mut u8,
                        size: d1.len(),
                        location: PNG_AFTER_IDAT,
                    },
                ];
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(
                        png_structp,
                        png_infop,
                        *const png_unknown_chunk,
                        c_int,
                    ),
                > = c.sym("png_set_unknown_chunks");
                unsafe { g(png, info, chunks.as_ptr(), chunks.len() as c_int) };
            }
        }

        c.call2("png_write_info");
        let mut ptrs: Vec<*mut u8> = rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
        if img.interlace == PNG_INTERLACE_ADAM7 {
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
                c.sym("png_write_image");
            unsafe { g(png, ptrs.as_mut_ptr(), img.height) };
        } else {
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
                c.sym("png_write_image");
            unsafe { g(png, ptrs.as_mut_ptr(), img.height) };
        }
        c.call2("png_write_end");
        drop(keep);
    });
    assert!(!out.errored, "encoding failed: {:?} for {img:?}", out.diag);
    out.bytes
}

fn make_icc_profile() -> Vec<u8> {
    let tag_count: u32 = 0;
    let len: u32 = 132 + 12 * tag_count;
    let mut p = vec![0u8; len as usize];
    p[0..4].copy_from_slice(&len.to_be_bytes());
    p[4..8].copy_from_slice(b"ADBE");
    p[8..12].copy_from_slice(&0x0200_0000u32.to_be_bytes());
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(b"RGB ");
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
    p[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    p[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
    p[128..132].copy_from_slice(&tag_count.to_be_bytes());
    p
}

/* ---------------------------------------------------------- read options */

#[derive(Clone, Copy, Debug, Default)]
struct ReadOpts {
    expand: bool,
    palette_to_rgb: bool,
    gray_1_2_4_to_8: bool,
    trns_to_alpha: bool,
    expand_16: bool,
    gray_to_rgb: bool,
    strip_16: bool,
    scale_16: bool,
    strip_alpha: bool,
    swap_alpha: bool,
    invert_alpha: bool,
    packing: bool,
    packswap: bool,
    bgr: bool,
    swap: bool,
    invert_mono: bool,
    shift: bool,
    filler: Option<(u32, c_int)>,
    gamma: Option<(i32, i32)>,
    background: Option<(c_int, i32)>,
    alpha_mode: Option<(c_int, i32)>,
    rgb_to_gray: Option<(c_int, i32, i32)>,
    quantize: Option<c_int>,
    interlace_handling: bool,
    /// call png_read_update_info before reading rows
    update_info: bool,
    /// read whole image with png_read_image instead of row-by-row
    whole_image: bool,
    crc_action: Option<(c_int, c_int)>,
    user_limits: Option<(u32, u32)>,
    keep_unknown: Option<c_int>,
    benign: bool,
}

fn apply_read_opts(c: &Ctx, o: &ReadOpts) {
    let png = c.png;
    if o.expand {
        c.call1("png_set_expand");
    }
    if o.palette_to_rgb {
        c.call1("png_set_palette_to_rgb");
    }
    if o.gray_1_2_4_to_8 {
        c.call1("png_set_expand_gray_1_2_4_to_8");
    }
    if o.trns_to_alpha {
        c.call1("png_set_tRNS_to_alpha");
    }
    if o.expand_16 {
        c.call1("png_set_expand_16");
    }
    if o.gray_to_rgb {
        c.call1("png_set_gray_to_rgb");
    }
    if o.strip_16 {
        c.call1("png_set_strip_16");
    }
    if o.scale_16 {
        c.call1("png_set_scale_16");
    }
    if o.strip_alpha {
        c.call1("png_set_strip_alpha");
    }
    if o.swap_alpha {
        c.call1("png_set_swap_alpha");
    }
    if o.invert_alpha {
        c.call1("png_set_invert_alpha");
    }
    if o.packing {
        c.call1("png_set_packing");
    }
    if o.packswap {
        c.call1("png_set_packswap");
    }
    if o.bgr {
        c.call1("png_set_bgr");
    }
    if o.swap {
        c.call1("png_set_swap");
    }
    if o.invert_mono {
        c.call1("png_set_invert_mono");
    }
    if o.shift {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *const png_color_8)> =
            c.sym("png_set_shift");
        let sb = png_color_8 { red: 4, green: 4, blue: 4, gray: 4, alpha: 4 };
        unsafe { f(png, &sb) };
    }
    if let Some((v, loc)) = o.filler {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, u32, c_int)> =
            c.sym("png_set_filler");
        unsafe { f(png, v, loc) };
    }
    if let Some((screen, file)) = o.gamma {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, i32, i32)> =
            c.sym("png_set_gamma_fixed");
        unsafe { f(png, screen, file) };
    }
    if let Some((kind, g)) = o.background {
        type F = unsafe extern "C-unwind" fn(png_structp, *const png_color_16, c_int, c_int, i32);
        let f: libloading::Symbol<F> = c.sym("png_set_background_fixed");
        // keep every component within the 8-bit range: libpng indexes its
        // 256-entry gamma tables with these values
        let bg = png_color_16 { index: 1, red: 100, green: 200, blue: 250, gray: 150 };
        unsafe { f(png, &bg, kind, 0, g) };
    }
    if let Some((mode, g)) = o.alpha_mode {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int, i32)> =
            c.sym("png_set_alpha_mode_fixed");
        unsafe { f(png, mode, g) };
    }
    if let Some((err, r, g)) = o.rgb_to_gray {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int, i32, i32)> =
            c.sym("png_set_rgb_to_gray_fixed");
        unsafe { f(png, err, r, g) };
    }
    if let Some(ndither) = o.quantize {
        // quantize needs a palette and (optionally) a histogram
        let pal: Vec<png_color> = (0..216u32)
            .map(|i| png_color {
                red: ((i / 36) * 51) as u8,
                green: (((i / 6) % 6) * 51) as u8,
                blue: ((i % 6) * 51) as u8,
            })
            .collect();
        let hist: Vec<u16> = (0..216u16).map(|i| i * 7).collect();
        type F = unsafe extern "C-unwind" fn(
            png_structp,
            *mut png_color,
            c_int,
            c_int,
            *const u16,
            c_int,
        );
        let f: libloading::Symbol<F> = c.sym("png_set_quantize");
        unsafe {
            f(
                png,
                pal.as_ptr() as *mut png_color,
                pal.len() as c_int,
                ndither,
                hist.as_ptr(),
                1,
            )
        };
    }
    if let Some((action, crit)) = o.crc_action {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int, c_int)> =
            c.sym("png_set_crc_action");
        unsafe { f(png, action, crit) };
    }
    if let Some((w, h)) = o.user_limits {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, u32, u32)> =
            c.sym("png_set_user_limits");
        unsafe { f(png, w, h) };
    }
    if let Some(keep) = o.keep_unknown {
        let f: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, c_int, *const u8, c_int),
        > = c.sym("png_set_keep_unknown_chunks");
        unsafe { f(png, keep, std::ptr::null(), 0) };
    }
    if o.benign {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
            c.sym("png_set_benign_errors");
        unsafe { f(png, 1) };
    }
}

fn decode(lib: &Lib, data: &[u8], o: &ReadOpts) -> ReadOutcome {
    read_with(lib, data, |c, out| {
        let png = c.png;
        c.call2("png_read_info");
        out.notes.push("--- after read_info ---".to_string());
        out.notes.extend(snapshot_info(c));

        apply_read_opts(c, o);

        let passes = if o.interlace_handling {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> c_int> =
                c.sym("png_set_interlace_handling");
            unsafe { f(png) }
        } else {
            1
        };

        if o.update_info {
            c.call2("png_read_update_info");
            out.notes.push("--- after update_info ---".to_string());
            out.notes.extend(snapshot_info(c));
        } else {
            c.call1("png_start_read_image");
        }

        let rb: usize = {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize> =
                c.sym("png_get_rowbytes");
            unsafe { f(png, c.info) }
        };
        let height: u32 = {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> u32> =
                c.sym("png_get_image_height");
            unsafe { f(png, c.info) }
        };
        let width: u32 = {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> u32> =
                c.sym("png_get_image_width");
            unsafe { f(png, c.info) }
        };
        out.notes.push(format!("rowbytes_for_read={rb} height={height} passes={passes}"));

        // Without png_read_update_info the reported rowbytes is the
        // untransformed size; the transformed row can never exceed 8 bytes per
        // pixel, so bound the buffer by that.
        let cap = rb.max(width as usize * 8 + 8) + 64;
        if o.whole_image {
            let mut bufs: Vec<Vec<u8>> = (0..height).map(|_| vec![0x5au8; cap]).collect();
            let mut ptrs: Vec<*mut u8> = bufs.iter_mut().map(|b| b.as_mut_ptr()).collect();
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8)> =
                c.sym("png_read_image");
            unsafe { f(png, ptrs.as_mut_ptr()) };
            out.rows = bufs;
        } else {
            let mut bufs: Vec<Vec<u8>> = (0..height).map(|_| vec![0x5au8; cap]).collect();
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut u8, *mut u8)> =
                c.sym("png_read_row");
            for _ in 0..passes {
                for b in bufs.iter_mut() {
                    unsafe { f(png, b.as_mut_ptr(), std::ptr::null_mut()) };
                }
            }
            out.rows = bufs;
        }

        c.call2("png_read_end");
        out.notes.push("--- after read_end ---".to_string());
        out.notes.extend(snapshot_info(c));
    })
}

fn compare_read(label: &str, data: &[u8], o: &ReadOpts, ctx: &str) {
    let l = libs();
    let a = decode(&l.c, data, o);
    let b = decode(&l.r, data, o);
    assert_eq!(
        a.errored, b.errored,
        "{label}/{ctx}: error flag differs\n C {:?}\n R {:?}",
        a.diag, b.diag
    );
    assert_eq!(a.diag, b.diag, "{label}/{ctx}: diagnostics differ");
    assert_snapshots_eq(&format!("{label}/{ctx}"), &a.notes, &b.notes);
    assert_eq!(a.rows.len(), b.rows.len(), "{label}/{ctx}: row count differs");
    for (i, (x, y)) in a.rows.iter().zip(b.rows.iter()).enumerate() {
        assert_eq!(
            x, y,
            "{label}/{ctx}: row {i} differs\n C: {}\n R: {}",
            hex(x),
            hex(y)
        );
    }
}

/* ------------------------------------------------------------------ tests */

#[test]
fn plain_reads() {
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &[(1u32, 1u32), (9, 5), (17, 9)] {
                let img = Img {
                    width: w,
                    height: h,
                    bit_depth: bd,
                    color_type: ct,
                    interlace: il,
                    rich: false,
                    seed: w * 7 + h * 3 + bd as u32 + ct as u32,
                };
                let data = encode(&img);
                for &(update, whole, ih) in &[
                    (false, false, false),
                    (true, false, false),
                    (true, true, false),
                    (true, false, true),
                ] {
                    let o = ReadOpts {
                        update_info: update,
                        whole_image: whole,
                        interlace_handling: ih,
                        ..Default::default()
                    };
                    compare_read("plain", &data, &o, &format!("{img:?} u={update} w={whole} ih={ih}"));
                }
            }
        }
    }
}

#[test]
fn rich_reads() {
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let img = Img {
                width: 13,
                height: 6,
                bit_depth: bd,
                color_type: ct,
                interlace: il,
                rich: true,
                seed: 909 + bd as u32 * 5 + ct as u32,
            };
            let data = encode(&img);
            let o = ReadOpts { update_info: true, ..Default::default() };
            compare_read("rich", &data, &o, &format!("{img:?}"));
            let o = ReadOpts {
                update_info: true,
                keep_unknown: Some(PNG_HANDLE_CHUNK_ALWAYS),
                ..Default::default()
            };
            compare_read("rich-keep", &data, &o, &format!("{img:?}"));
        }
    }
}

#[test]
fn read_transforms_basic() {
    for (ct, bd) in formats() {
        let img = Img {
            width: 15,
            height: 5,
            bit_depth: bd,
            color_type: ct,
            interlace: PNG_INTERLACE_NONE,
            rich: true,
            seed: 55 + bd as u32 + ct as u32 * 11,
        };
        let data = encode(&img);
        let mut opts: Vec<(&str, ReadOpts)> = Vec::new();
        opts.push(("expand", ReadOpts { expand: true, update_info: true, ..Default::default() }));
        opts.push((
            "palette_to_rgb",
            ReadOpts { palette_to_rgb: true, update_info: true, ..Default::default() },
        ));
        opts.push((
            "gray124to8",
            ReadOpts { gray_1_2_4_to_8: true, update_info: true, ..Default::default() },
        ));
        opts.push((
            "trns_to_alpha",
            ReadOpts { trns_to_alpha: true, update_info: true, ..Default::default() },
        ));
        opts.push((
            "expand16",
            ReadOpts { expand: true, expand_16: true, update_info: true, ..Default::default() },
        ));
        opts.push((
            "gray_to_rgb",
            ReadOpts { gray_to_rgb: true, update_info: true, ..Default::default() },
        ));
        opts.push(("strip16", ReadOpts { strip_16: true, update_info: true, ..Default::default() }));
        opts.push(("scale16", ReadOpts { scale_16: true, update_info: true, ..Default::default() }));
        opts.push((
            "strip_alpha",
            ReadOpts { strip_alpha: true, update_info: true, ..Default::default() },
        ));
        opts.push((
            "swap_alpha",
            ReadOpts { swap_alpha: true, update_info: true, ..Default::default() },
        ));
        opts.push((
            "invert_alpha",
            ReadOpts { invert_alpha: true, update_info: true, ..Default::default() },
        ));
        opts.push(("packing", ReadOpts { packing: true, update_info: true, ..Default::default() }));
        opts.push(("packswap", ReadOpts { packswap: true, update_info: true, ..Default::default() }));
        opts.push(("bgr", ReadOpts { bgr: true, update_info: true, ..Default::default() }));
        opts.push(("swap", ReadOpts { swap: true, update_info: true, ..Default::default() }));
        opts.push((
            "invert_mono",
            ReadOpts { invert_mono: true, update_info: true, ..Default::default() },
        ));
        opts.push(("shift", ReadOpts { shift: true, update_info: true, ..Default::default() }));
        opts.push((
            "filler_before",
            ReadOpts {
                filler: Some((0xabcd, PNG_FILLER_BEFORE)),
                update_info: true,
                ..Default::default()
            },
        ));
        opts.push((
            "filler_after",
            ReadOpts {
                filler: Some((0xabcd, PNG_FILLER_AFTER)),
                update_info: true,
                ..Default::default()
            },
        ));
        opts.push((
            "everything",
            ReadOpts {
                expand: true,
                gray_to_rgb: true,
                expand_16: true,
                swap: true,
                bgr: true,
                invert_alpha: true,
                update_info: true,
                ..Default::default()
            },
        ));
        for (name, o) in opts {
            compare_read(name, &data, &o, &format!("{img:?}"));
        }
    }
}

#[test]
fn read_transforms_gamma_background_alpha() {
    for (ct, bd) in formats() {
        let img = Img {
            width: 12,
            height: 4,
            bit_depth: bd,
            color_type: ct,
            interlace: PNG_INTERLACE_NONE,
            rich: true,
            seed: 4321 + bd as u32 * 3 + ct as u32,
        };
        let data = encode(&img);
        let mut opts: Vec<(String, ReadOpts)> = Vec::new();
        for &(screen, file) in &[
            (100000i32, 45455i32),
            (220000, 100000),
            (45455, 220000),
            (100000, 100000),
        ] {
            opts.push((
                format!("gamma {screen}/{file}"),
                ReadOpts { gamma: Some((screen, file)), update_info: true, ..Default::default() },
            ));
            opts.push((
                format!("gamma+expand {screen}/{file}"),
                ReadOpts {
                    gamma: Some((screen, file)),
                    expand: true,
                    update_info: true,
                    ..Default::default()
                },
            ));
        }
        for &kind in &[
            PNG_BACKGROUND_GAMMA_SCREEN,
            PNG_BACKGROUND_GAMMA_FILE,
            PNG_BACKGROUND_GAMMA_UNIQUE,
        ] {
            opts.push((
                format!("background {kind}"),
                ReadOpts {
                    background: Some((kind, 100000)),
                    gamma: Some((100000, 45455)),
                    expand: true,
                    update_info: true,
                    ..Default::default()
                },
            ));
        }
        for &mode in &[PNG_ALPHA_PNG, PNG_ALPHA_STANDARD, PNG_ALPHA_BROKEN, PNG_ALPHA_OPTIMIZED] {
            opts.push((
                format!("alpha_mode {mode}"),
                ReadOpts { alpha_mode: Some((mode, 100000)), update_info: true, ..Default::default() },
            ));
        }
        opts.push((
            "rgb_to_gray".to_string(),
            ReadOpts {
                rgb_to_gray: Some((1, 21260, 71520)),
                expand: true,
                update_info: true,
                ..Default::default()
            },
        ));
        opts.push((
            "rgb_to_gray default coeffs".to_string(),
            ReadOpts {
                rgb_to_gray: Some((2, -1, -1)),
                expand: true,
                update_info: true,
                ..Default::default()
            },
        ));
        for (name, o) in opts {
            compare_read(&name, &data, &o, &format!("{img:?}"));
        }
    }
}

#[test]
fn read_quantize() {
    for (ct, bd) in [
        (PNG_COLOR_TYPE_RGB, 8u8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ] {
        let img = Img {
            width: 20,
            height: 6,
            bit_depth: bd,
            color_type: ct,
            interlace: PNG_INTERLACE_NONE,
            rich: true,
            seed: 31337 + bd as u32,
        };
        let data = encode(&img);
        for &n in &[2i32, 16, 64, 216, 256] {
            let o = ReadOpts {
                quantize: Some(n),
                strip_16: bd == 16,
                update_info: true,
                ..Default::default()
            };
            compare_read("quantize", &data, &o, &format!("{img:?} n={n}"));
        }
    }
}

#[test]
fn read_png_high_level() {
    let l = libs();
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            // png_read_png allocates the row buffers itself and libpng
            // deliberately leaves the bits beyond the image width in the
            // destination row untouched, so use a width that is a whole
            // number of bytes at every bit depth.
            let img = Img {
                width: 16,
                height: 6,
                bit_depth: bd,
                color_type: ct,
                interlace: il,
                rich: true,
                seed: 60 + bd as u32 + ct as u32 * 7,
            };
            let data = encode(&img);
            for &tr in &[
                PNG_TRANSFORM_IDENTITY,
                PNG_TRANSFORM_STRIP_16,
                PNG_TRANSFORM_STRIP_ALPHA,
                PNG_TRANSFORM_PACKING,
                PNG_TRANSFORM_PACKSWAP,
                PNG_TRANSFORM_EXPAND,
                PNG_TRANSFORM_INVERT_MONO,
                PNG_TRANSFORM_SHIFT,
                PNG_TRANSFORM_BGR,
                PNG_TRANSFORM_SWAP_ALPHA,
                PNG_TRANSFORM_SWAP_ENDIAN,
                PNG_TRANSFORM_INVERT_ALPHA,
                PNG_TRANSFORM_GRAY_TO_RGB,
                PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_GRAY_TO_RGB | PNG_TRANSFORM_BGR,
            ] {
                let run = |lib: &Lib| {
                    read_with(lib, &data, |c, out| {
                        let f: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, png_infop, c_int, *mut c_void),
                        > = c.sym("png_read_png");
                        unsafe { f(c.png, c.info, tr, std::ptr::null_mut()) };
                        out.notes.extend(snapshot_info(c));
                        // copy the decoded rows out
                        let g: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, png_infop) -> *mut *mut u8,
                        > = c.sym("png_get_rows");
                        let rows = unsafe { g(c.png, c.info) };
                        let h: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize,
                        > = c.sym("png_get_rowbytes");
                        let rb = unsafe { h(c.png, c.info) };
                        let hh: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, png_infop) -> u32,
                        > = c.sym("png_get_image_height");
                        let height = unsafe { hh(c.png, c.info) };
                        if !rows.is_null() {
                            for i in 0..height as usize {
                                let p = unsafe { *rows.add(i) };
                                out.rows.push(if p.is_null() {
                                    Vec::new()
                                } else {
                                    unsafe { std::slice::from_raw_parts(p, rb) }.to_vec()
                                });
                            }
                        }
                    })
                };
                let a = run(&l.c);
                let b = run(&l.r);
                let ctx = format!("{img:?} tr={tr:#x}");
                assert_eq!(a.errored, b.errored, "read_png/{ctx}: error differs {:?} {:?}", a.diag, b.diag);
                assert_eq!(a.diag, b.diag, "read_png/{ctx}: diag differs");
                assert_snapshots_eq(&format!("read_png/{ctx}"), &a.notes, &b.notes);
                assert_eq!(a.rows, b.rows, "read_png/{ctx}: rows differ");
            }
        }
    }
}

use std::ffi::c_void;

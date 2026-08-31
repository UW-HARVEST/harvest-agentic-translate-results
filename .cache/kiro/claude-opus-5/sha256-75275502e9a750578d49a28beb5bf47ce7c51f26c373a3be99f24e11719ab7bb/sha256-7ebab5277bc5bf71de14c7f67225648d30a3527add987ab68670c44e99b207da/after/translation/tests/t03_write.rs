//! Tier 3: whole-file writes.  The complete encoded byte stream produced by
//! the C library and by the Rust library must be identical, including the
//! zlib-compressed IDAT payload and every ancillary chunk.

mod common;
use common::*;
use std::ffi::{c_char, c_int, CString};

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

#[derive(Clone, Debug)]
struct Scenario {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    interlace: c_int,
    filters: Option<c_int>,
    level: Option<c_int>,
    strategy: Option<c_int>,
    window_bits: Option<c_int>,
    mem_level: Option<c_int>,
    /// write row-by-row via png_write_row instead of png_write_image
    row_by_row: bool,
    /// number of png_write_flush calls interleaved (0 = none)
    flush_every: u32,
    chunks: bool,
    text: bool,
    unknown: bool,
    transforms: u32,
    seed: u32,
}

impl Default for Scenario {
    fn default() -> Self {
        Scenario {
            width: 13,
            height: 7,
            bit_depth: 8,
            color_type: PNG_COLOR_TYPE_RGB,
            interlace: PNG_INTERLACE_NONE,
            filters: None,
            level: None,
            strategy: None,
            window_bits: None,
            mem_level: None,
            row_by_row: false,
            flush_every: 0,
            chunks: false,
            text: false,
            unknown: false,
            transforms: 0,
            seed: 1,
        }
    }
}

const TR_BGR: u32 = 1 << 0;
const TR_SWAP: u32 = 1 << 1;
const TR_PACKING: u32 = 1 << 2;
const TR_PACKSWAP: u32 = 1 << 3;
const TR_INVERT_MONO: u32 = 1 << 4;
const TR_SHIFT: u32 = 1 << 5;
const TR_SWAP_ALPHA: u32 = 1 << 6;
const TR_INVERT_ALPHA: u32 = 1 << 7;
const TR_FILLER_AFTER: u32 = 1 << 8;
const TR_INTERLACE_HANDLING: u32 = 1 << 9;

fn pixel_data(sc: &Scenario) -> Vec<Vec<u8>> {
    let pd = channels(sc.color_type) * sc.bit_depth as u32;
    // when PNG_TRANSFORM packing/filler is in use the caller supplies wider rows
    let rb = rowbytes(pd, sc.width);
    let mut s = sc.seed | 1;
    (0..sc.height)
        .map(|_| {
            (0..rb)
                .map(|_| {
                    s = s.wrapping_mul(1103515245).wrapping_add(12345);
                    (s >> 16) as u8
                })
                .collect()
        })
        .collect()
}

fn palette() -> Vec<png_color> {
    (0..256u32)
        .map(|i| png_color {
            red: (i * 7 % 256) as u8,
            green: (i * 13 % 256) as u8,
            blue: (i * 29 % 256) as u8,
        })
        .collect()
}

fn set_common_chunks(c: &Ctx, sc: &Scenario, keep: &mut Vec<CString>) {
    let png = c.png;
    let info = c.info;

    // gAMA / sRGB-free (sRGB would suppress gAMA/cHRM), cHRM, sBIT ...
    {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, i32)> =
            c.sym("png_set_gAMA_fixed");
        unsafe { f(png, info, 45455) };
    }
    {
        type F = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
        );
        let f: libloading::Symbol<F> = c.sym("png_set_cHRM_fixed");
        unsafe { f(png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000) };
    }
    {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color_8)> =
            c.sym("png_set_sBIT");
        let bd = sc.bit_depth.min(8);
        let sb = match sc.color_type {
            PNG_COLOR_TYPE_GRAY => png_color_8 { gray: bd, ..Default::default() },
            PNG_COLOR_TYPE_PALETTE => png_color_8 { red: 8, green: 8, blue: 8, ..Default::default() },
            PNG_COLOR_TYPE_RGB => png_color_8 { red: bd, green: bd, blue: bd, ..Default::default() },
            PNG_COLOR_TYPE_GRAY_ALPHA => png_color_8 { gray: bd, alpha: bd, ..Default::default() },
            _ => png_color_8 { red: bd, green: bd, blue: bd, alpha: bd, ..Default::default() },
        };
        unsafe { f(png, info, &sb) };
    }
    {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, u32, u32, c_int)> =
            c.sym("png_set_pHYs");
        unsafe { f(png, info, 3000, 2500, 1) };
    }
    {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, i32, i32, c_int)> =
            c.sym("png_set_oFFs");
        unsafe { f(png, info, -17, 42, 0) };
    }
    {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_time)> =
            c.sym("png_set_tIME");
        let t = png_time { year: 2024, month: 2, day: 29, hour: 13, minute: 45, second: 7 };
        unsafe { f(png, info, &t) };
    }
    {
        // sCAL, string form
        let w = CString::new("3.14159").unwrap();
        let h = CString::new("2.71828e2").unwrap();
        let f: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, c_int, *const c_char, *const c_char),
        > = c.sym("png_set_sCAL_s");
        unsafe { f(png, info, 1, w.as_ptr(), h.as_ptr()) };
        keep.push(w);
        keep.push(h);
    }
    {
        // pCAL
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
        let f: libloading::Symbol<F> = c.sym("png_set_pCAL");
        unsafe {
            f(
                png,
                info,
                purpose.as_ptr(),
                -100,
                1000,
                2,
                2,
                units.as_ptr(),
                params.as_mut_ptr(),
            )
        };
        keep.push(purpose);
        keep.push(units);
        keep.push(p0);
        keep.push(p1);
    }
    {
        // cICP, cLLI, mDCV
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, u8, u8, u8, u8)> =
            c.sym("png_set_cICP");
        unsafe { f(png, info, 9, 16, 0, 1) };
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, u32, u32)> =
            c.sym("png_set_cLLI_fixed");
        unsafe { f(png, info, 1000 * 10000, 400 * 10000) };
        type Fm = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
        );
        let f: libloading::Symbol<Fm> = c.sym("png_set_mDCV_fixed");
        unsafe {
            f(
                png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10000000,
                50,
            )
        };
    }
    {
        // eXIf
        let exif: Vec<u8> = b"II\x2a\x00\x08\x00\x00\x00\x00\x00".to_vec();
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, u32, *const u8)> =
            c.sym("png_set_eXIf_1");
        unsafe { f(png, info, exif.len() as u32, exif.as_ptr()) };
    }
    {
        // iCCP: a syntactically valid but minimal profile is rejected; use a
        // handcrafted 132-byte header that passes libpng's checks.
        let prof = make_icc_profile();
        let name = CString::new("ICC profile").unwrap();
        let f: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *const c_char, c_int, *const u8, u32),
        > = c.sym("png_set_iCCP");
        unsafe { f(png, info, name.as_ptr(), 0, prof.as_ptr(), prof.len() as u32) };
        keep.push(name);
    }
    {
        // sPLT
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
        let mut s = png_sPLT_t {
            name: name.as_ptr() as *mut c_char,
            depth: 16,
            entries: entries.as_ptr() as *mut png_sPLT_entry,
            nentries: entries.len() as i32,
        };
        let f: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_sPLT_t, c_int),
        > = c.sym("png_set_sPLT");
        unsafe { f(png, info, &s, 1) };
        keep.push(name);
        let _ = &mut s;
    }
}

/// A minimal ICC v2 profile skeleton that satisfies libpng's header checks.
fn make_icc_profile() -> Vec<u8> {
    let tag_count: u32 = 0;
    let len: u32 = 132 + 12 * tag_count;
    let mut p = vec![0u8; len as usize];
    p[0..4].copy_from_slice(&len.to_be_bytes());
    p[4..8].copy_from_slice(b"ADBE"); // preferred CMM
    p[8..12].copy_from_slice(&0x0200_0000u32.to_be_bytes()); // version 2.0
    p[12..16].copy_from_slice(b"mntr"); // device class
    p[16..20].copy_from_slice(b"RGB "); // colour space
    p[20..24].copy_from_slice(b"XYZ "); // PCS
    p[36..40].copy_from_slice(b"acsp"); // signature
    // rendering intent 0, illuminant D50 (0xF6D6, 0x10000, 0xD32D in 16.16)
    p[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
    p[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    p[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
    p[128..132].copy_from_slice(&tag_count.to_be_bytes());
    p
}

fn add_text(c: &Ctx, keep: &mut Vec<CString>) {
    let png = c.png;
    let info = c.info;
    let mk = |k: &str, t: &str, comp: c_int, lang: Option<&str>, lk: Option<&str>| {
        let key = CString::new(k).unwrap();
        let text = CString::new(t).unwrap();
        let l = lang.map(|x| CString::new(x).unwrap());
        let lkk = lk.map(|x| CString::new(x).unwrap());
        (key, text, l, lkk, comp)
    };
    let specs = vec![
        mk("Title", "A short title", PNG_TEXT_COMPRESSION_NONE, None, None),
        mk(
            "Description",
            "A much longer description, repeated to be worth compressing. \
             A much longer description, repeated to be worth compressing. \
             A much longer description, repeated to be worth compressing.",
            PNG_TEXT_COMPRESSION_zTXt,
            None,
            None,
        ),
        mk(
            "Comment",
            "international text",
            PNG_ITXT_COMPRESSION_NONE,
            Some("en-GB"),
            Some("Comment"),
        ),
        mk(
            "Author",
            "compressed international text, long enough to matter, \
             compressed international text, long enough to matter",
            PNG_ITXT_COMPRESSION_zTXt,
            Some("de"),
            Some("Autor"),
        ),
    ];
    let mut texts: Vec<png_text> = Vec::new();
    for (key, text, lang, lk, comp) in &specs {
        texts.push(png_text {
            compression: *comp,
            key: key.as_ptr() as *mut c_char,
            text: text.as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: lang.as_ref().map_or(std::ptr::null_mut(), |x| x.as_ptr() as *mut c_char),
            lang_key: lk.as_ref().map_or(std::ptr::null_mut(), |x| x.as_ptr() as *mut c_char),
        });
    }
    let f: libloading::Symbol<
        unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_text, c_int),
    > = c.sym("png_set_text");
    unsafe { f(png, info, texts.as_ptr(), texts.len() as c_int) };
    for (key, text, lang, lk, _) in specs {
        keep.push(key);
        keep.push(text);
        if let Some(x) = lang {
            keep.push(x);
        }
        if let Some(x) = lk {
            keep.push(x);
        }
    }
}

fn add_unknown(c: &Ctx) {
    let png = c.png;
    let info = c.info;
    let d0: Vec<u8> = (0u8..32).collect();
    let d1: Vec<u8> = vec![0xaa; 5];
    let d2: Vec<u8> = Vec::new();
    // On write libpng insists on a concrete location: before PLTE, before IDAT
    // or after IDAT.
    const PNG_HAVE_IHDR: u8 = 0x01;
    const PNG_HAVE_PLTE: u8 = 0x02;
    const PNG_AFTER_IDAT: u8 = 0x08;
    let chunks = [
        png_unknown_chunk { name: *b"prVt\0", data: d0.as_ptr() as *mut u8, size: d0.len(), location: PNG_HAVE_IHDR },
        png_unknown_chunk { name: *b"seCd\0", data: d1.as_ptr() as *mut u8, size: d1.len(), location: PNG_HAVE_PLTE },
        png_unknown_chunk { name: *b"emPt\0", data: d2.as_ptr() as *mut u8, size: 0, location: PNG_AFTER_IDAT },
    ];
    let f: libloading::Symbol<
        unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_unknown_chunk, c_int),
    > = c.sym("png_set_unknown_chunks");
    unsafe { f(png, info, chunks.as_ptr(), chunks.len() as c_int) };
    let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, c_int, c_int)> =
        c.sym("png_set_unknown_chunk_location");
    unsafe { g(png, info, 1, PNG_HAVE_PLTE as c_int) };
}

fn write_scenario(lib: &Lib, sc: &Scenario, rows: &[Vec<u8>]) -> WriteOutcome {
    write_with(lib, |c, _notes| {
        let png = c.png;
        let info = c.info;
        let mut keep: Vec<CString> = Vec::new();

        if let Some(l) = sc.level {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_compression_level");
            unsafe { f(png, l) };
        }
        if let Some(s) = sc.strategy {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_compression_strategy");
            unsafe { f(png, s) };
        }
        if let Some(w) = sc.window_bits {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_compression_window_bits");
            unsafe { f(png, w) };
        }
        if let Some(m) = sc.mem_level {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_compression_mem_level");
            unsafe { f(png, m) };
        }
        if let Some(fl) = sc.filters {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int, c_int)> =
                c.sym("png_set_filter");
            unsafe { f(png, 0, fl) };
        }

        {
            type F = unsafe extern "C-unwind" fn(
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
            let f: libloading::Symbol<F> = c.sym("png_set_IHDR");
            unsafe {
                f(
                    png,
                    info,
                    sc.width,
                    sc.height,
                    sc.bit_depth as c_int,
                    sc.color_type as c_int,
                    sc.interlace,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                )
            };
        }

        if sc.color_type == PNG_COLOR_TYPE_PALETTE {
            let pal = palette();
            let n = 1usize << sc.bit_depth;
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color, c_int),
            > = c.sym("png_set_PLTE");
            unsafe { f(png, info, pal.as_ptr(), n as c_int) };
            // hIST needs a PLTE
            let hist: Vec<u16> = (0..n).map(|i| (i * 3 % 65536) as u16).collect();
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *const u16)> =
                c.sym("png_set_hIST");
            unsafe { f(png, info, hist.as_ptr()) };
        }

        if sc.chunks {
            set_common_chunks(c, sc, &mut keep);
            // bKGD / tRNS depend on the colour type
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color_16),
            > = c.sym("png_set_bKGD");
            let maxv = if sc.bit_depth == 16 { 65535u16 } else { (1u16 << sc.bit_depth) - 1 };
            let bg = if sc.color_type == PNG_COLOR_TYPE_PALETTE {
                png_color_16 {
                    index: (((1u16 << sc.bit_depth) - 1).min(3)) as u8,
                    ..Default::default()
                }
            } else if (sc.color_type & PNG_COLOR_MASK_COLOR) != 0 {
                png_color_16 { red: maxv / 2, green: maxv / 3, blue: maxv / 4, ..Default::default() }
            } else {
                png_color_16 { gray: maxv / 2, ..Default::default() }
            };
            unsafe { f(png, info, &bg) };

            if (sc.color_type & PNG_COLOR_MASK_ALPHA) == 0 {
                type F = unsafe extern "C-unwind" fn(
                    png_structp,
                    png_infop,
                    *const u8,
                    c_int,
                    *const png_color_16,
                );
                let f: libloading::Symbol<F> = c.sym("png_set_tRNS");
                if sc.color_type == PNG_COLOR_TYPE_PALETTE {
                    let n = 1usize << sc.bit_depth;
                    let trans: Vec<u8> = (0..n).map(|i| (i * 17 % 256) as u8).collect();
                    unsafe {
                        f(png, info, trans.as_ptr(), n as c_int, std::ptr::null())
                    };
                } else {
                    let tc = if (sc.color_type & PNG_COLOR_MASK_COLOR) != 0 {
                        png_color_16 { red: 1, green: 2, blue: 3, ..Default::default() }
                    } else {
                        png_color_16 { gray: maxv / 5, ..Default::default() }
                    };
                    unsafe { f(png, info, std::ptr::null(), 0, &tc) };
                }
            }
        }

        if sc.text {
            add_text(c, &mut keep);
        }
        if sc.unknown {
            add_unknown(c);
        }

        c.call2("png_write_info");

        // write-side transforms
        if sc.transforms & TR_BGR != 0 {
            c.call1("png_set_bgr");
        }
        if sc.transforms & TR_SWAP != 0 {
            c.call1("png_set_swap");
        }
        if sc.transforms & TR_PACKING != 0 {
            c.call1("png_set_packing");
        }
        if sc.transforms & TR_PACKSWAP != 0 {
            c.call1("png_set_packswap");
        }
        if sc.transforms & TR_INVERT_MONO != 0 {
            c.call1("png_set_invert_mono");
        }
        if sc.transforms & TR_SWAP_ALPHA != 0 {
            c.call1("png_set_swap_alpha");
        }
        if sc.transforms & TR_INVERT_ALPHA != 0 {
            c.call1("png_set_invert_alpha");
        }
        if sc.transforms & TR_SHIFT != 0 {
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, *const png_color_8),
            > = c.sym("png_set_shift");
            let bd = sc.bit_depth.min(8);
            let sb = png_color_8 {
                red: bd.saturating_sub(1).max(1),
                green: bd.saturating_sub(1).max(1),
                blue: bd.saturating_sub(1).max(1),
                gray: bd.saturating_sub(1).max(1),
                alpha: bd.saturating_sub(1).max(1),
            };
            unsafe { f(png, &sb) };
        }
        if sc.transforms & TR_FILLER_AFTER != 0 {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, u32, c_int)> =
                c.sym("png_set_filler");
            unsafe { f(png, 0, PNG_FILLER_AFTER) };
        }
        // Writing an interlaced image row by row only produces IDAT data if
        // libpng is asked to do the pass decomposition.
        let passes = if sc.transforms & TR_INTERLACE_HANDLING != 0
            || (sc.row_by_row && sc.interlace == PNG_INTERLACE_ADAM7)
        {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> c_int> =
                c.sym("png_set_interlace_handling");
            unsafe { f(png) }
        } else {
            1
        };

        if sc.flush_every > 0 {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_flush");
            unsafe { f(png, sc.flush_every as c_int) };
        }

        if sc.row_by_row || passes > 1 {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *const u8)> =
                c.sym("png_write_row");
            for _ in 0..passes {
                for r in rows {
                    unsafe { f(png, r.as_ptr()) };
                }
            }
        } else {
            let mut ptrs: Vec<*mut u8> = rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
                c.sym("png_write_image");
            unsafe { f(png, ptrs.as_mut_ptr(), sc.height) };
        }

        c.call2("png_write_end");
        drop(keep);
    })
}

fn compare(label: &str, sc: &Scenario, rows: &[Vec<u8>]) {
    let l = libs();
    let a = write_scenario(&l.c, sc, rows);
    let b = write_scenario(&l.r, sc, rows);
    assert!(
        !a.errored,
        "{label}: C library failed: {:?} ({sc:?}) rows={} rowlen={}",
        a.diag,
        rows.len(),
        rows.first().map_or(0, |r| r.len())
    );
    assert_eq!(a.errored, b.errored, "{label}: error flag differs ({sc:?})\nC diag {:?}\nR diag {:?}", a.diag, b.diag);
    assert_eq!(a.diag, b.diag, "{label}: diagnostics differ ({sc:?})");
    assert_eq!(a.flushes, b.flushes, "{label}: flush count differs ({sc:?})");
    if a.bytes != b.bytes {
        let n = a
            .bytes
            .iter()
            .zip(b.bytes.iter())
            .position(|(x, y)| x != y)
            .unwrap_or(a.bytes.len().min(b.bytes.len()));
        panic!(
            "{label}: byte stream differs at offset {n} ({sc:?})\n C len {} R len {}\n C: {}\n R: {}",
            a.bytes.len(),
            b.bytes.len(),
            hex(&a.bytes[n.saturating_sub(16)..(n + 16).min(a.bytes.len())]),
            hex(&b.bytes[n.saturating_sub(16)..(n + 16).min(b.bytes.len())]),
        );
    }
}

#[test]
fn plain_all_formats() {
    for (ct, bd) in formats() {
        for &(w, h) in &[(1u32, 1u32), (7, 3), (16, 16), (33, 5)] {
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                let sc = Scenario {
                    width: w,
                    height: h,
                    bit_depth: bd,
                    color_type: ct,
                    interlace: il,
                    seed: w * 31 + h * 7 + bd as u32 + ct as u32,
                    ..Default::default()
                };
                let rows = pixel_data(&sc);
                compare("plain", &sc, &rows);
            }
        }
    }
}

#[test]
fn filters_and_compression() {
    let filter_sets = [
        PNG_NO_FILTERS,
        PNG_FILTER_NONE,
        PNG_FILTER_SUB,
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_ALL_FILTERS,
        PNG_FILTER_SUB | PNG_FILTER_PAETH,
    ];
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1u8),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
    ] {
        for &fl in &filter_sets {
            for &level in &[0i32, 1, 6, 9] {
                let sc = Scenario {
                    width: 23,
                    height: 11,
                    bit_depth: bd,
                    color_type: ct,
                    filters: Some(fl),
                    level: Some(level),
                    seed: fl as u32 * 7 + level as u32 * 13 + bd as u32,
                    ..Default::default()
                };
                let rows = pixel_data(&sc);
                compare("filters", &sc, &rows);
            }
        }
    }
    // strategies, window bits, mem levels
    for &strategy in &[0i32, 1, 2, 3, 4] {
        for &wb in &[8i32, 9, 12, 15] {
            for &ml in &[1i32, 8, 9] {
                let sc = Scenario {
                    width: 40,
                    height: 9,
                    bit_depth: 8,
                    color_type: PNG_COLOR_TYPE_RGB,
                    strategy: Some(strategy),
                    window_bits: Some(wb),
                    mem_level: Some(ml),
                    filters: Some(PNG_ALL_FILTERS),
                    seed: strategy as u32 * 101 + wb as u32 * 3 + ml as u32,
                    ..Default::default()
                };
                let rows = pixel_data(&sc);
                compare("zlib-params", &sc, &rows);
            }
        }
    }
}

#[test]
fn ancillary_chunks() {
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let sc = Scenario {
                width: 9,
                height: 6,
                bit_depth: bd,
                color_type: ct,
                interlace: il,
                chunks: true,
                text: true,
                unknown: true,
                seed: 4242 + bd as u32 + ct as u32 * 3,
                ..Default::default()
            };
            let rows = pixel_data(&sc);
            compare("chunks", &sc, &rows);
            // make sure the scenario really does emit the chunks it claims to
            let bytes = write_scenario(&libs().c, &sc, &rows).bytes;
            for tag in [
                &b"gAMA"[..], b"cHRM", b"sBIT", b"pHYs", b"oFFs", b"tIME", b"sCAL", b"pCAL",
                b"cICP", b"cLLI", b"mDCV", b"eXIf", b"iCCP", b"sPLT", b"bKGD", b"tEXt",
                b"zTXt", b"iTXt", b"prVt", b"seCd", b"emPt",
            ] {
                assert!(
                    bytes.windows(4).any(|w| w == tag),
                    "chunk {} missing from output ({sc:?})",
                    String::from_utf8_lossy(tag)
                );
            }
        }
    }
}

#[test]
fn write_transforms() {
    // transforms that do not change the number of input bytes per row
    for (ct, bd) in formats() {
        let mut set = vec![0u32];
        if (ct & PNG_COLOR_MASK_COLOR) != 0 && ct != PNG_COLOR_TYPE_PALETTE {
            set.push(TR_BGR);
        }
        if bd == 16 {
            set.push(TR_SWAP);
        }
        if bd < 8 {
            set.push(TR_PACKSWAP);
        }
        if ct == PNG_COLOR_TYPE_GRAY && bd == 1 {
            set.push(TR_INVERT_MONO);
        }
        if (ct & PNG_COLOR_MASK_ALPHA) != 0 {
            set.push(TR_SWAP_ALPHA);
            set.push(TR_INVERT_ALPHA);
        }
        if ct != PNG_COLOR_TYPE_PALETTE {
            set.push(TR_SHIFT);
        }
        for &tr in &set {
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                let sc = Scenario {
                    width: 17,
                    height: 5,
                    bit_depth: bd,
                    color_type: ct,
                    interlace: il,
                    transforms: tr,
                    row_by_row: true,
                    seed: tr * 31 + bd as u32 * 7 + ct as u32,
                    ..Default::default()
                };
                let rows = pixel_data(&sc);
                compare("write-transform", &sc, &rows);
            }
        }
    }
}

#[test]
fn interlace_handling_and_flush() {
    for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
        for &flush in &[0u32, 1, 3] {
            let sc = Scenario {
                width: 21,
                height: 13,
                bit_depth: 8,
                color_type: PNG_COLOR_TYPE_RGB_ALPHA,
                interlace: il,
                transforms: TR_INTERLACE_HANDLING,
                flush_every: flush,
                seed: 777 + flush,
                ..Default::default()
            };
            let rows = pixel_data(&sc);
            compare("interlace-handling", &sc, &rows);
        }
    }
}

#[test]
fn packing_and_filler() {
    // png_set_packing widens sub-8-bit input to one byte per pixel
    for bd in [1u8, 2, 4] {
        let sc = Scenario {
            width: 11,
            height: 4,
            bit_depth: bd,
            color_type: PNG_COLOR_TYPE_GRAY,
            transforms: TR_PACKING,
            row_by_row: true,
            seed: bd as u32 * 101,
            ..Default::default()
        };
        // one byte per pixel on input
        let mut s = sc.seed | 1;
        let rows: Vec<Vec<u8>> = (0..sc.height)
            .map(|_| {
                (0..sc.width)
                    .map(|_| {
                        s = s.wrapping_mul(1103515245).wrapping_add(12345);
                        ((s >> 16) as u8) & ((1u16 << bd) - 1) as u8
                    })
                    .collect()
            })
            .collect();
        compare("packing", &sc, &rows);
    }
    // png_set_filler(AFTER) strips a 4th channel from RGBX input
    for &(ct, in_ch) in &[(PNG_COLOR_TYPE_RGB, 4usize), (PNG_COLOR_TYPE_GRAY, 2usize)] {
        for bd in [8u8, 16] {
            let sc = Scenario {
                width: 9,
                height: 4,
                bit_depth: bd,
                color_type: ct,
                transforms: TR_FILLER_AFTER,
                row_by_row: true,
                seed: 31 * bd as u32 + in_ch as u32,
                ..Default::default()
            };
            let bytes_per_row = sc.width as usize * in_ch * (bd as usize / 8);
            let mut s = sc.seed | 1;
            let rows: Vec<Vec<u8>> = (0..sc.height)
                .map(|_| {
                    (0..bytes_per_row)
                        .map(|_| {
                            s = s.wrapping_mul(1103515245).wrapping_add(12345);
                            (s >> 16) as u8
                        })
                        .collect()
                })
                .collect();
            compare("filler", &sc, &rows);
        }
    }
}

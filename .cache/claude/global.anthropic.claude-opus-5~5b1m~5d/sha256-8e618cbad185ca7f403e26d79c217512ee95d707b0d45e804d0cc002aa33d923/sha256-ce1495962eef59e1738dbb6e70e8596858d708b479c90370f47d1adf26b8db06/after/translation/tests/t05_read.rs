//! Phase B — CONFIGS.md section B: the low-level sequential READ pipeline
//! (`png_create_read_struct` .. `png_read_end`) with every read transform,
//! driven end to end and compared row-for-row between the two `.so`s.
mod common;
use common::*;

// ---------------------------------------------------------------------------
// Building the input PNGs: produced by the (already verified) write path of
// the *C* library, so both readers get byte-identical input.
// ---------------------------------------------------------------------------

pub struct Src {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub bit_depth: c_int,
    pub color_type: c_int,
    pub interlace: c_int,
}

unsafe fn build(
    rng: &mut Rng,
    color_type: c_int,
    bit_depth: c_int,
    w: u32,
    h: u32,
    interlace: c_int,
    extras: &[Extra],
) -> Src {
    let api = c_api();
    set_current_api(api);
    diag_reset();
    let mut sess = WriteSess::new(api);
    let png = sess.png;
    let info = sess.info;
    let pd = channels_of(color_type) * bit_depth as u32;
    let rb = rowbytes(pd, w);
    let rows: Vec<Vec<u8>> = (0..h).map(|_| rng.bytes(rb)).collect();
    let npal = if color_type == PNG_COLOR_TYPE_PALETTE {
        1usize << bit_depth
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
    let mut trns_alpha: Vec<u8> = Vec::new();
    let mut keep_c: Vec<std::ffi::CString> = Vec::new();
    let mut keep_t: Vec<png_text> = Vec::new();
    let ok = guard(|| {
        (api.png_set_IHDR)(
            png,
            info,
            w,
            h,
            bit_depth,
            color_type,
            interlace,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if !palette.is_empty() {
            (api.png_set_PLTE)(png, info, palette.as_ptr(), palette.len() as c_int);
        }
        for e in extras {
            match e {
                Extra::Gama(g) => (api.png_set_gAMA_fixed)(png, info, *g),
                Extra::Srgb(i) => (api.png_set_sRGB)(png, info, *i),
                Extra::Trns => {
                    if color_type == PNG_COLOR_TYPE_PALETTE {
                        trns_alpha = (0..palette.len()).map(|i| (i as u8) ^ 0x5a).collect();
                        (api.png_set_tRNS)(
                            png,
                            info,
                            trns_alpha.as_mut_ptr(),
                            trns_alpha.len() as c_int,
                            std::ptr::null_mut(),
                        );
                    } else if color_type == PNG_COLOR_TYPE_GRAY
                        || color_type == PNG_COLOR_TYPE_RGB
                    {
                        let mx = if bit_depth == 16 {
                            0xffffu16
                        } else {
                            ((1u32 << bit_depth) - 1) as u16
                        };
                        let col = png_color_16 {
                            index: 0,
                            red: mx / 3,
                            green: mx / 5,
                            blue: mx / 7,
                            gray: mx / 2,
                        };
                        (api.png_set_tRNS)(
                            png,
                            info,
                            std::ptr::null_mut(),
                            0,
                            &col as *const _ as png_color_16p,
                        );
                    }
                }
                Extra::Bkgd => {
                    let mx = if color_type == PNG_COLOR_TYPE_PALETTE {
                        (palette.len() - 1) as u16
                    } else if bit_depth == 16 {
                        0xffff
                    } else {
                        ((1u32 << bit_depth) - 1) as u16
                    };
                    let col = png_color_16 {
                        index: (mx & 0xff) as u8,
                        red: mx / 2,
                        green: mx / 3,
                        blue: mx / 4,
                        gray: mx / 5,
                    };
                    (api.png_set_bKGD)(png, info, &col as *const _ as png_const_color_16p);
                }
                Extra::SigBit => {
                    let b = if bit_depth == 16 { 12 } else { bit_depth as u8 };
                    let s = png_color_8 {
                        red: b,
                        green: b,
                        blue: b,
                        gray: b,
                        alpha: b,
                    };
                    (api.png_set_sBIT)(png, info, &s);
                }
                Extra::Text => {
                    keep_c.push(cs("Comment"));
                    let k = keep_c.last().unwrap().as_ptr() as png_charp;
                    keep_c.push(cs("hello progressive world"));
                    let t = keep_c.last().unwrap().as_ptr() as png_charp;
                    keep_t.push(png_text {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: k,
                        text: t,
                        text_length: 23,
                        itxt_length: 0,
                        lang: std::ptr::null_mut(),
                        lang_key: std::ptr::null_mut(),
                    });
                    (api.png_set_text)(png, info, keep_t.last().unwrap(), 1);
                }
                Extra::Hist => {
                    if color_type == PNG_COLOR_TYPE_PALETTE {
                        let h: Vec<u16> = (0..palette.len()).map(|i| (i * 7) as u16).collect();
                        (api.png_set_hIST)(png, info, h.as_ptr());
                    }
                }
            }
        }
        (api.png_write_info)(png, info);
        let mut rowps: Vec<png_bytep> = rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
        (api.png_write_image)(png, rowps.as_mut_ptr());
        (api.png_write_end)(png, info);
    })
    .is_some();
    let _ = diag_take();
    assert!(ok, "reference PNG construction failed");
    Src {
        bytes: std::mem::take(&mut sess.sink.buf),
        width: w,
        height: h,
        bit_depth,
        color_type,
        interlace,
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Extra {
    Gama(i32),
    Srgb(c_int),
    Trns,
    Bkgd,
    SigBit,
    Text,
    Hist,
}

// ---------------------------------------------------------------------------
// Read transforms
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum R {
    Expand,
    Expand16,
    PaletteToRgb,
    ExpandGray124To8,
    TrnsToAlpha,
    GrayToRgb,
    Strip16,
    Scale16,
    StripAlpha,
    AddAlpha(u32, c_int),
    Filler(u32, c_int),
    Background(png_color_16, c_int, c_int, f64),
    BackgroundFixed(png_color_16, c_int, c_int, i32),
    Gamma(f64, f64),
    GammaFixed(i32, i32),
    AlphaMode(c_int, f64),
    AlphaModeFixed(c_int, i32),
    RgbToGray(c_int, f64, f64),
    RgbToGrayFixed(c_int, i32, i32),
    RgbCoefficients,
    Quantize(Vec<png_color>, c_int, Vec<u16>, c_int),
    Shift(png_color_8),
    Packing,
    PackSwap,
    Swap,
    SwapAlpha,
    InvertAlpha,
    InvertMono,
    Bgr,
    CrcAction(c_int, c_int),
    UserLimits(u32, u32),
    ChunkCacheMax(u32),
    ChunkMallocMax(usize),
    Option(c_int, c_int),
    MngFeatures(u32),
    Benign(c_int),
    CheckInvalidIndex(c_int),
    KeepUnknown(c_int),
}

/// `png_set_quantize` REWRITES the caller's palette in place, so each library
/// must get its own copy; the rewritten copy is returned so it can be compared.
unsafe fn apply_r(
    api: &'static Api,
    png: png_structp,
    info: png_infop,
    ts: &[R],
    out_pal: &mut Vec<png_color>,
) {
    let mut owned: Vec<Vec<png_color>> = Vec::new();
    for t in ts {
        match t {
            R::Expand => (api.png_set_expand)(png),
            R::Expand16 => (api.png_set_expand_16)(png),
            R::PaletteToRgb => (api.png_set_palette_to_rgb)(png),
            R::ExpandGray124To8 => (api.png_set_expand_gray_1_2_4_to_8)(png),
            R::TrnsToAlpha => (api.png_set_tRNS_to_alpha)(png),
            R::GrayToRgb => (api.png_set_gray_to_rgb)(png),
            R::Strip16 => (api.png_set_strip_16)(png),
            R::Scale16 => (api.png_set_scale_16)(png),
            R::StripAlpha => (api.png_set_strip_alpha)(png),
            R::AddAlpha(v, loc) => (api.png_set_add_alpha)(png, *v, *loc),
            R::Filler(v, loc) => (api.png_set_filler)(png, *v, *loc),
            R::Background(c, code, need, g) => {
                (api.png_set_background)(png, c as *const _ as png_const_color_16p, *code, *need, *g)
            }
            R::BackgroundFixed(c, code, need, g) => (api.png_set_background_fixed)(
                png,
                c as *const _ as png_const_color_16p,
                *code,
                *need,
                *g,
            ),
            R::Gamma(s, f) => (api.png_set_gamma)(png, *s, *f),
            R::GammaFixed(s, f) => (api.png_set_gamma_fixed)(png, *s, *f),
            R::AlphaMode(m, g) => (api.png_set_alpha_mode)(png, *m, *g),
            R::AlphaModeFixed(m, g) => (api.png_set_alpha_mode_fixed)(png, *m, *g),
            R::RgbToGray(a, r, g) => (api.png_set_rgb_to_gray)(png, *a, *r, *g),
            R::RgbToGrayFixed(a, r, g) => (api.png_set_rgb_to_gray_fixed)(png, *a, *r, *g),
            // png_set_rgb_coefficients(png_ptr) -- installs the default
            // (or colorspace-derived) rgb_to_gray coefficients (png.c:1886).
            R::RgbCoefficients => (api.png_set_rgb_coefficients)(png),
            R::Quantize(pal, npal, hist, max) => {
                owned.push(pal.clone());
                let p = owned.last_mut().unwrap();
                (api.png_set_quantize)(
                    png,
                    p.as_mut_ptr(),
                    *npal,
                    *max,
                    if hist.is_empty() {
                        std::ptr::null()
                    } else {
                        hist.as_ptr()
                    },
                    1,
                );
            }
            R::Shift(s) => (api.png_set_shift)(png, s),
            R::Packing => (api.png_set_packing)(png),
            R::PackSwap => (api.png_set_packswap)(png),
            R::Swap => (api.png_set_swap)(png),
            R::SwapAlpha => (api.png_set_swap_alpha)(png),
            R::InvertAlpha => (api.png_set_invert_alpha)(png),
            R::InvertMono => (api.png_set_invert_mono)(png),
            R::Bgr => (api.png_set_bgr)(png),
            R::CrcAction(a, b) => (api.png_set_crc_action)(png, *a, *b),
            R::UserLimits(w, h) => (api.png_set_user_limits)(png, *w, *h),
            R::ChunkCacheMax(v) => (api.png_set_chunk_cache_max)(png, *v),
            R::ChunkMallocMax(v) => (api.png_set_chunk_malloc_max)(png, *v),
            R::Option(o, v) => {
                (api.png_set_option)(png, *o, *v);
            }
            R::MngFeatures(f) => {
                (api.png_permit_mng_features)(png, *f);
            }
            R::Benign(v) => (api.png_set_benign_errors)(png, *v),
            R::CheckInvalidIndex(v) => (api.png_set_check_for_invalid_index)(png, *v),
            R::KeepUnknown(k) => {
                (api.png_set_keep_unknown_chunks)(png, *k, std::ptr::null(), 0)
            }
        }
    }
    for v in owned {
        out_pal.extend_from_slice(&v);
    }
}

/// Everything observable after a full read.
#[derive(Debug, PartialEq)]
pub struct ReadOut {
    pub ok: bool,
    pub diag: Diag,
    pub width: u32,
    pub height: u32,
    pub bit_depth: c_int,
    pub color_type: c_int,
    pub interlace: c_int,
    pub channels: u8,
    pub rowbytes: usize,
    pub valid: u32,
    pub rows: Vec<Vec<u8>>,
    pub rgb_to_gray_status: u8,
    pub palette_max: c_int,
    pub current_pass: u8,
    pub current_row: u32,
    pub quantize_palette: Vec<png_color>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReadMode {
    /// png_read_row per row (with a display row too)
    Row { display: bool },
    /// png_read_rows for the whole image
    Rows,
    /// png_read_image with a row-pointer array
    Image,
    /// png_read_png one-shot
    OneShot(c_int),
}

pub unsafe fn read_png(api: &'static Api, src: &Src, ts: &[R], mode: ReadMode) -> ReadOut {
    set_current_api(api);
    diag_reset();
    let sess = ReadSess::new(api, &src.bytes);
    let png = sess.png;
    let info = sess.info;
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let mut w = 0u32;
    let mut h = 0u32;
    let mut bd = 0i32;
    let mut ct = 0i32;
    let mut il = 0i32;
    let mut ch = 0u8;
    let mut rbz = 0usize;
    let mut valid = 0u32;
    let mut r2g = 0u8;
    let mut pmax = -1i32;
    let mut cpass = 0u8;
    let mut crow = 0u32;
    let mut qpal: Vec<png_color> = Vec::new();
    let ok = guard(|| {
        match mode {
            ReadMode::OneShot(t) => {
                apply_r(api, png, info, ts, &mut qpal);
                (api.png_read_png)(png, info, t, std::ptr::null_mut());
                let rp = (api.png_get_rows)(png, info);
                w = (api.png_get_image_width)(png, info);
                h = (api.png_get_image_height)(png, info);
                rbz = (api.png_get_rowbytes)(png, info);
                if !rp.is_null() {
                    for y in 0..h {
                        let p = *rp.add(y as usize);
                        rows.push(std::slice::from_raw_parts(p, rbz).to_vec());
                    }
                }
            }
            _ => {
                (api.png_read_info)(png, info);
                apply_r(api, png, info, ts, &mut qpal);
                (api.png_read_update_info)(png, info);
                w = (api.png_get_image_width)(png, info);
                h = (api.png_get_image_height)(png, info);
                rbz = (api.png_get_rowbytes)(png, info);
                let npasses = if src.interlace == PNG_INTERLACE_ADAM7 {
                    (api.png_set_interlace_handling)(png)
                } else {
                    1
                };
                match mode {
                    ReadMode::Row { display } => {
                        let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                        let mut disp: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                        for _ in 0..npasses {
                            for y in 0..h as usize {
                                if display {
                                    (api.png_read_row)(
                                        png,
                                        buf[y].as_mut_ptr(),
                                        disp[y].as_mut_ptr(),
                                    );
                                } else {
                                    (api.png_read_row)(
                                        png,
                                        buf[y].as_mut_ptr(),
                                        std::ptr::null_mut(),
                                    );
                                }
                            }
                        }
                        for y in 0..h as usize {
                            rows.push(buf[y].clone());
                            if display {
                                rows.push(disp[y].clone());
                            }
                        }
                    }
                    ReadMode::Rows => {
                        let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                        let mut ptrs: Vec<png_bytep> =
                            buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        for _ in 0..npasses {
                            (api.png_read_rows)(
                                png,
                                ptrs.as_mut_ptr(),
                                std::ptr::null_mut(),
                                h,
                            );
                        }
                        rows = buf;
                    }
                    ReadMode::Image => {
                        let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                        let mut ptrs: Vec<png_bytep> =
                            buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        (api.png_read_image)(png, ptrs.as_mut_ptr());
                        rows = buf;
                    }
                    ReadMode::OneShot(_) => unreachable!(),
                }
                (api.png_read_end)(png, sess.end);
            }
        }
        bd = (api.png_get_bit_depth)(png, info) as c_int;
        ct = (api.png_get_color_type)(png, info) as c_int;
        il = (api.png_get_interlace_type)(png, info) as c_int;
        ch = (api.png_get_channels)(png, info);
        valid = (api.png_get_valid)(png, info, 0xffff_ffff);
        r2g = (api.png_get_rgb_to_gray_status)(png);
        pmax = (api.png_get_palette_max)(png, info);
        cpass = (api.png_get_current_pass_number)(png);
        crow = (api.png_get_current_row_number)(png);
    })
    .is_some();
    ReadOut {
        ok,
        diag: diag_take(),
        width: w,
        height: h,
        bit_depth: bd,
        color_type: ct,
        interlace: il,
        channels: ch,
        rowbytes: rbz,
        valid,
        rows,
        rgb_to_gray_status: r2g,
        palette_max: pmax,
        current_pass: cpass,
        current_row: crow,
        quantize_palette: qpal,
    }
}

/// Zero the padding bits of the last byte of a sub-byte-depth row.
///
/// `png_combine_row` deliberately PRESERVES the destination row's bits beyond
/// the last pixel (`end_byte & end_mask`).  For `png_read_png` the destination
/// rows are allocated by libpng with `png_malloc` and are therefore
/// uninitialised, so those bits are indeterminate in BOTH libraries and must
/// not be compared.  Every other read mode uses zeroed buffers, where the bits
/// are well defined and ARE compared.
fn strip_padding(rows: &mut [Vec<u8>], pixel_depth: u32, width: u32, packswap: bool) {
    if pixel_depth == 0 || pixel_depth >= 8 {
        return;
    }
    let bits = pixel_depth as u64 * width as u64;
    let m = (bits & 7) as u32;
    if m == 0 {
        return;
    }
    let last = (bits / 8) as usize; // index of the partial byte
    let keep: u8 = if packswap {
        // little-endian byte: the *high* bits are padding
        0xffu8 >> (8 - m)
    } else {
        !(0xffu8 >> m)
    };
    for r in rows.iter_mut() {
        if last < r.len() {
            r[last] &= keep;
        }
    }
}

fn rdiff(label: &str, src: &Src, ts: &[R], mode: ReadMode) {
    unsafe {
        let mut co = read_png(c_api(), src, ts, mode);
        let mut ro = read_png(rs_api(), src, ts, mode);
        if let ReadMode::OneShot(t) = mode {
            let pd = co.channels as u32 * co.bit_depth as u32;
            let ps = (t & PNG_TRANSFORM_PACKSWAP) != 0;
            strip_padding(&mut co.rows, pd, co.width, ps);
            let pd2 = ro.channels as u32 * ro.bit_depth as u32;
            strip_padding(&mut ro.rows, pd2, ro.width, ps);
        }
        assert_eq!(
            co.ok, ro.ok,
            "{}: error parity (C={} RS={})\n C diag {:?}\n RS diag {:?}",
            label, co.ok, ro.ok, co.diag, ro.diag
        );
        assert_eq!(co.diag, ro.diag, "{}: diagnostics", label);
        assert_eq!(
            (
                co.width,
                co.height,
                co.bit_depth,
                co.color_type,
                co.interlace,
                co.channels,
                co.rowbytes,
                co.valid,
                co.rgb_to_gray_status,
                co.palette_max,
                co.current_pass,
                co.current_row
            ),
            (
                ro.width,
                ro.height,
                ro.bit_depth,
                ro.color_type,
                ro.interlace,
                ro.channels,
                ro.rowbytes,
                ro.valid,
                ro.rgb_to_gray_status,
                ro.palette_max,
                ro.current_pass,
                ro.current_row
            ),
            "{}: info fields",
            label
        );
        assert_eq!(
            co.quantize_palette, ro.quantize_palette,
            "{}: palette rewritten by png_set_quantize",
            label
        );
        assert_eq!(co.rows.len(), ro.rows.len(), "{}: row count", label);
        for (i, (a, b)) in co.rows.iter().zip(ro.rows.iter()).enumerate() {
            assert_bytes_eq(&format!("{} row {}", label, i), a, b);
        }
    }
}

const SIZES: [(u32, u32); 5] = [(1, 1), (1, 9), (9, 1), (13, 7), (32, 5)];

#[test]
fn plain_read_all_formats_all_modes() {
    let mut rng = Rng::new(0x2244_6688_aacc_ee01);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &SIZES {
                unsafe {
                    let src = build(&mut rng, ct, bd, w, h, il, &[]);
                    for mode in [
                        ReadMode::Row { display: false },
                        ReadMode::Row { display: true },
                        ReadMode::Rows,
                        ReadMode::Image,
                    ] {
                        rdiff(
                            &format!("plain ct={} bd={} il={} {}x{} {:?}", ct, bd, il, w, h, mode),
                            &src,
                            &[],
                            mode,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn read_png_one_shot_transforms() {
    let mut rng = Rng::new(0x3355_7799_bbdd_ff01);
    let masks = [
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
        PNG_TRANSFORM_EXPAND_16,
        PNG_TRANSFORM_SCALE_16,
        PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_GRAY_TO_RGB,
        PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_STRIP_ALPHA,
        PNG_TRANSFORM_STRIP_16 | PNG_TRANSFORM_BGR | PNG_TRANSFORM_INVERT_ALPHA,
    ];
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            unsafe {
                let src = build(
                    &mut rng,
                    ct,
                    bd,
                    11,
                    5,
                    il,
                    &[Extra::SigBit, Extra::Trns, Extra::Bkgd],
                );
                for &m in &masks {
                    rdiff(
                        &format!("read_png ct={} bd={} il={} m={:#x}", ct, bd, il, m),
                        &src,
                        &[],
                        ReadMode::OneShot(m),
                    );
                }
            }
        }
    }
}

#[test]
fn expansion_transforms() {
    let mut rng = Rng::new(0x4466_88aa_ccee_0011);
    let sets: Vec<Vec<R>> = vec![
        vec![R::Expand],
        vec![R::Expand, R::Expand16],
        vec![R::PaletteToRgb],
        vec![R::ExpandGray124To8],
        vec![R::TrnsToAlpha],
        vec![R::GrayToRgb],
        vec![R::Expand, R::GrayToRgb],
        vec![R::Expand16],
        vec![R::Strip16],
        vec![R::Scale16],
        vec![R::Strip16, R::Expand],
        vec![R::Scale16, R::Expand],
        vec![R::StripAlpha],
        vec![R::AddAlpha(0xffff, PNG_FILLER_AFTER)],
        vec![R::AddAlpha(0x1234, PNG_FILLER_BEFORE)],
        vec![R::Filler(0x55aa, PNG_FILLER_AFTER)],
        vec![R::Filler(0x55aa, PNG_FILLER_BEFORE)],
        vec![R::Expand, R::TrnsToAlpha, R::GrayToRgb, R::Expand16],
        vec![R::PaletteToRgb, R::TrnsToAlpha, R::StripAlpha],
    ];
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            unsafe {
                let src = build(&mut rng, ct, bd, 13, 4, il, &[Extra::Trns, Extra::SigBit]);
                for ts in &sets {
                    for mode in [ReadMode::Image, ReadMode::Row { display: true }] {
                        rdiff(
                            &format!(
                                "expand ct={} bd={} il={} {:?} {:?}",
                                ct, bd, il, ts, mode
                            ),
                            &src,
                            ts,
                            mode,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn gamma_background_alpha_mode() {
    let mut rng = Rng::new(0x5577_99bb_ddff_1122);
    for (ct, bd) in legal_ihdr() {
        unsafe {
            let src = build(
                &mut rng,
                ct,
                bd,
                17,
                3,
                PNG_INTERLACE_NONE,
                &[Extra::Gama(45455), Extra::Trns, Extra::Bkgd],
            );
            // IMPORTANT: the background components must be IN RANGE.
            //
            //  * need_expand == 0 -> the value is already in the *output*
            //    format, i.e. <= 255 for 8-bit output.  libpng indexes
            //    `gamma_table[background.red]` (a 256-entry table) directly,
            //    so a 16-bit value here is an out-of-bounds read in the C
            //    (pngrtran.c:1712) -- C undefined behaviour, not testable.
            //  * need_expand != 0 -> the value is in the *file* format: a
            //    palette index for colour type 3 (must be < num_palette,
            //    otherwise `palette[background.index]` is out of bounds), or a
            //    sample in 0 ..= (1<<bit_depth)-1 otherwise.
            let bg_noexpand = png_color_16 {
                index: 1,
                red: 0x3f,
                green: 0x7f,
                blue: 0xbf,
                gray: 0x55,
            };
            let filemax: u16 = if bd == 16 {
                0xffff
            } else {
                ((1u32 << bd) - 1) as u16
            };
            let bg_expand = if ct == PNG_COLOR_TYPE_PALETTE {
                png_color_16 {
                    index: (filemax as u8).min(1),
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                }
            } else {
                png_color_16 {
                    index: 0,
                    red: filemax / 3,
                    green: filemax / 5,
                    blue: filemax / 7,
                    gray: filemax / 2,
                }
            };
            let mut sets: Vec<Vec<R>> = vec![
                vec![R::GammaFixed(45455, 100000)],
                vec![R::GammaFixed(100000, 45455)],
                vec![R::Gamma(2.2, 0.45455)],
                vec![R::GammaFixed(220000, 45455), R::Expand],
                vec![R::GammaFixed(0, 100000)],
                vec![R::GammaFixed(100000, 100000)],
            ];
            for code in [
                PNG_BACKGROUND_GAMMA_SCREEN,
                PNG_BACKGROUND_GAMMA_FILE,
                PNG_BACKGROUND_GAMMA_UNIQUE,
                PNG_BACKGROUND_GAMMA_UNKNOWN,
            ] {
                for &(need, bg) in &[(0i32, bg_noexpand), (1i32, bg_expand)] {
                    sets.push(vec![
                        R::Expand,
                        R::Background(bg, code, need, 1.0),
                        R::GammaFixed(45455, 100000),
                    ]);
                    sets.push(vec![R::Expand, R::BackgroundFixed(bg, code, need, 100000)]);
                    sets.push(vec![
                        R::Expand,
                        R::BackgroundFixed(bg, code, need, 45455),
                        R::GammaFixed(45455, 220000),
                    ]);
                    sets.push(vec![
                        R::Expand,
                        R::Expand16,
                        R::BackgroundFixed(bg, code, need, 100000),
                    ]);
                }
            }
            for m in [
                PNG_ALPHA_PNG,
                PNG_ALPHA_STANDARD,
                PNG_ALPHA_OPTIMIZED,
                PNG_ALPHA_BROKEN,
            ] {
                sets.push(vec![R::AlphaMode(m, 2.2)]);
                sets.push(vec![R::AlphaModeFixed(m, 220000)]);
                sets.push(vec![R::AlphaModeFixed(m, PNG_DEFAULT_sRGB)]);
                sets.push(vec![R::AlphaModeFixed(m, PNG_GAMMA_MAC_18)]);
                sets.push(vec![R::Expand, R::AlphaModeFixed(m, 220000)]);
            }
            for ts in &sets {
                rdiff(
                    &format!("gamma ct={} bd={} {:?}", ct, bd, ts),
                    &src,
                    ts,
                    ReadMode::Image,
                );
            }
        }
    }
}

#[test]
fn rgb_to_gray_and_coefficients() {
    let mut rng = Rng::new(0x6688_aacc_ee00_2233);
    let mut sets: Vec<Vec<R>> = Vec::new();
    for action in [-1i32, 0, 1, 2, 3] {
        sets.push(vec![R::RgbToGray(action, -1.0, -1.0)]);
        sets.push(vec![R::RgbToGrayFixed(action, -1, -1)]);
        sets.push(vec![R::RgbToGrayFixed(action, 21260, 71520)]);
        sets.push(vec![R::RgbToGray(action, 0.3, 0.6)]);
        sets.push(vec![
            R::RgbCoefficients,
            R::RgbToGrayFixed(action, -1, -1),
        ]);
        sets.push(vec![R::Expand, R::RgbToGrayFixed(action, -1, -1)]);
    }
    for (ct, bd) in legal_ihdr() {
        unsafe {
            let src = build(
                &mut rng,
                ct,
                bd,
                15,
                4,
                PNG_INTERLACE_NONE,
                &[Extra::Gama(45455)],
            );
            for ts in &sets {
                rdiff(
                    &format!("r2g ct={} bd={} {:?}", ct, bd, ts),
                    &src,
                    ts,
                    ReadMode::Image,
                );
            }
        }
    }
}

#[test]
fn quantize() {
    let mut rng = Rng::new(0x7799_bbdd_ff11_3344);
    let pal: Vec<png_color> = (0..64)
        .map(|i| png_color {
            red: (i * 4) as u8,
            green: (255 - i * 3) as u8,
            blue: (i * 7 % 256) as u8,
        })
        .collect();
    let hist: Vec<u16> = (0..64).map(|i| (i * 11 % 1000) as u16).collect();
    for (ct, bd) in legal_ihdr() {
        for maxc in [1i32, 2, 16, 64, 256] {
            unsafe {
                let src = build(&mut rng, ct, bd, 21, 5, PNG_INTERLACE_NONE, &[Extra::Hist]);
                for withhist in [false, true] {
                    let ts = vec![R::Quantize(
                        pal.clone(),
                        pal.len() as c_int,
                        if withhist { hist.clone() } else { Vec::new() },
                        maxc,
                    )];
                    rdiff(
                        &format!("quantize ct={} bd={} max={} hist={}", ct, bd, maxc, withhist),
                        &src,
                        &ts,
                        ReadMode::Image,
                    );
                }
            }
        }
    }
}

#[test]
fn bit_level_transforms() {
    let mut rng = Rng::new(0x88aa_ccee_0022_4455);
    let sets: Vec<Vec<R>> = vec![
        vec![R::Packing],
        vec![R::PackSwap],
        vec![R::Packing, R::PackSwap],
        vec![R::Swap],
        vec![R::SwapAlpha],
        vec![R::InvertAlpha],
        vec![R::InvertMono],
        vec![R::Bgr],
        vec![R::Bgr, R::Swap],
        vec![R::Shift(png_color_8 {
            red: 4,
            green: 4,
            blue: 4,
            gray: 4,
            alpha: 4,
        })],
        vec![R::Packing, R::InvertMono, R::PackSwap],
        vec![R::Expand, R::Bgr, R::InvertAlpha, R::Swap],
    ];
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            unsafe {
                let src = build(&mut rng, ct, bd, 19, 3, il, &[Extra::SigBit]);
                for ts in &sets {
                    rdiff(
                        &format!("bits ct={} bd={} il={} {:?}", ct, bd, il, ts),
                        &src,
                        ts,
                        ReadMode::Image,
                    );
                }
            }
        }
    }
}

#[test]
fn read_options_and_limits() {
    let mut rng = Rng::new(0x99bb_ddff_1133_5566);
    let mut sets: Vec<Vec<R>> = Vec::new();
    for crit in [
        PNG_CRC_DEFAULT,
        PNG_CRC_ERROR_QUIT,
        PNG_CRC_WARN_USE,
        PNG_CRC_QUIET_USE,
        PNG_CRC_NO_CHANGE,
        PNG_CRC_WARN_DISCARD,
    ] {
        for ancil in [
            PNG_CRC_DEFAULT,
            PNG_CRC_ERROR_QUIT,
            PNG_CRC_WARN_DISCARD,
            PNG_CRC_WARN_USE,
            PNG_CRC_QUIET_USE,
            PNG_CRC_NO_CHANGE,
        ] {
            sets.push(vec![R::CrcAction(crit, ancil)]);
        }
    }
    for o in [
        PNG_MAXIMUM_INFLATE_WINDOW,
        PNG_SKIP_sRGB_CHECK_PROFILE,
        PNG_IGNORE_ADLER32,
        0,
        10,
    ] {
        for v in [PNG_OPTION_OFF, PNG_OPTION_ON] {
            sets.push(vec![R::Option(o, v)]);
        }
    }
    sets.push(vec![R::UserLimits(1_000_000, 1_000_000)]);
    sets.push(vec![R::UserLimits(1, 1)]);
    sets.push(vec![R::ChunkCacheMax(0)]);
    sets.push(vec![R::ChunkCacheMax(1)]);
    sets.push(vec![R::ChunkMallocMax(0)]);
    sets.push(vec![R::ChunkMallocMax(1)]);
    sets.push(vec![R::ChunkMallocMax(1 << 20)]);
    sets.push(vec![R::MngFeatures(PNG_ALL_MNG_FEATURES)]);
    sets.push(vec![R::MngFeatures(0)]);
    sets.push(vec![R::Benign(0)]);
    sets.push(vec![R::Benign(1)]);
    sets.push(vec![R::CheckInvalidIndex(0)]);
    sets.push(vec![R::CheckInvalidIndex(1)]);
    for k in [
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
    ] {
        sets.push(vec![R::KeepUnknown(k)]);
    }
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        unsafe {
            let src = build(
                &mut rng,
                ct,
                bd,
                12,
                6,
                PNG_INTERLACE_NONE,
                &[Extra::Gama(45455), Extra::Text, Extra::Srgb(0), Extra::Bkgd],
            );
            for ts in &sets {
                rdiff(
                    &format!("opt ct={} bd={} {:?}", ct, bd, ts),
                    &src,
                    ts,
                    ReadMode::Image,
                );
            }
        }
    }
}

#[test]
fn start_read_image_and_update_info_orderings() {
    let mut rng = Rng::new(0xaacc_ee00_2244_6677)
        ;
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            unsafe {
                let src = build(&mut rng, ct, bd, 9, 5, il, &[]);
                // png_start_read_image instead of png_read_update_info
                let mut outs = Vec::new();
                for api in both() {
                    set_current_api(api);
                    diag_reset();
                    let sess = ReadSess::new(api, &src.bytes);
                    let png = sess.png;
                    let info = sess.info;
                    let mut rows: Vec<Vec<u8>> = Vec::new();
                    let mut rbz = 0usize;
                    let ok = guard(|| {
                        (api.png_read_info)(png, info);
                        (api.png_set_expand)(png);
                        (api.png_start_read_image)(png);
                        // NB: png_start_read_image does NOT update
                        // info_ptr->rowbytes (only png_read_update_info does),
                        // so allocate for the largest possible transformed
                        // pixel depth (RGBA @ 16 bits = 64) instead.
                        rbz = (api.png_get_rowbytes)(png, info);
                        let w = (api.png_get_image_width)(png, info);
                        let alloc = rowbytes(64, w) + 16;
                        let h = (api.png_get_image_height)(png, info);
                        let np = if il == PNG_INTERLACE_ADAM7 {
                            (api.png_set_interlace_handling)(png)
                        } else {
                            1
                        };
                        let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; alloc]).collect();
                        for _ in 0..np {
                            for y in 0..h as usize {
                                (api.png_read_row)(
                                    png,
                                    buf[y].as_mut_ptr(),
                                    std::ptr::null_mut(),
                                );
                            }
                        }
                        rows = buf;
                        (api.png_read_end)(png, sess.end);
                    })
                    .is_some();
                    outs.push((ok, diag_take(), rbz, rows));
                }
                assert_eq!(
                    outs[0].0, outs[1].0,
                    "start_read_image parity ct={} bd={} il={}",
                    ct, bd, il
                );
                assert_eq!(outs[0].1, outs[1].1, "start_read_image diag");
                assert_eq!(outs[0].2, outs[1].2, "start_read_image rowbytes");
                assert_eq!(outs[0].3, outs[1].3, "start_read_image rows");
            }
        }
    }
}

#[test]
fn read_with_signature_already_consumed() {
    let mut rng = Rng::new(0xbbdd_ff11_3355_7788);
    for (ct, bd) in [(PNG_COLOR_TYPE_RGB, 8), (PNG_COLOR_TYPE_GRAY, 1)] {
        unsafe {
            let src = build(&mut rng, ct, bd, 7, 3, PNG_INTERLACE_NONE, &[]);
            for skip in [0usize, 1, 4, 8] {
                let mut outs = Vec::new();
                for api in both() {
                    set_current_api(api);
                    diag_reset();
                    let sess = ReadSess::new(api, &src.bytes[skip..]);
                    let png = sess.png;
                    let info = sess.info;
                    let mut rows: Vec<Vec<u8>> = Vec::new();
                    let ok = guard(|| {
                        (api.png_set_sig_bytes)(png, skip as c_int);
                        (api.png_read_info)(png, info);
                        let rbz = (api.png_get_rowbytes)(png, info);
                        let h = (api.png_get_image_height)(png, info);
                        let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                        let mut ptrs: Vec<png_bytep> =
                            buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        (api.png_read_image)(png, ptrs.as_mut_ptr());
                        (api.png_read_end)(png, sess.end);
                        rows = buf;
                    })
                    .is_some();
                    outs.push((ok, diag_take(), rows));
                }
                assert_eq!(
                    outs[0].0, outs[1].0,
                    "sig_bytes={} parity ct={} bd={}",
                    skip, ct, bd
                );
                assert_eq!(outs[0].1, outs[1].1, "sig_bytes={} diag", skip);
                assert_eq!(outs[0].2, outs[1].2, "sig_bytes={} rows", skip);
            }
        }
    }
}

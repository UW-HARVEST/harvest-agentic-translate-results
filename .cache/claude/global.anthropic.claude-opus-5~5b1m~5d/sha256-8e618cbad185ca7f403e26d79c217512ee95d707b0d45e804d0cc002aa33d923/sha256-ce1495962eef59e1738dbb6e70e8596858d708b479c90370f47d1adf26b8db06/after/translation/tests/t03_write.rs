//! Phase B — CONFIGS.md section A: the low-level sequential WRITE pipeline
//! (`png_create_write_struct` .. `png_write_end`) driven end to end, with the
//! produced PNG byte stream compared byte-for-byte between the two `.so`s.
mod common;
use common::*;
use std::ffi::CString;

#[derive(Clone, Debug)]
pub enum W {
    Bgr,
    Swap,
    SwapAlpha,
    InvertAlpha,
    InvertMono,
    Packing,
    PackSwap,
    Shift(png_color_8),
    Filler(u32, c_int),
    Filter(c_int),
    FilterHeuristics(c_int, Vec<f64>, Vec<f64>),
    FilterHeuristicsFixed(c_int, Vec<i32>, Vec<i32>),
    Level(c_int),
    MemLevel(c_int),
    Strategy(c_int),
    WindowBits(c_int),
    Method(c_int),
    BufferSize(usize),
    TextLevel(c_int),
    TextMemLevel(c_int),
    TextStrategy(c_int),
    TextWindowBits(c_int),
    TextMethod(c_int),
    FlushEvery(c_int),
    SigBit(png_color_8),
    Gamma(f64),
    GammaFixed(i32),
    Srgb(c_int),
    SrgbGamaChrm(c_int),
    Chrm([f64; 8]),
    ChrmFixed([i32; 8]),
    ChrmXyz([f64; 9]),
    ChrmXyzFixed([i32; 9]),
    Iccp(String, Vec<u8>),
    Trns(Vec<u8>, png_color_16, c_int),
    Bkgd(png_color_16),
    Hist(Vec<u16>),
    Phys(u32, u32, c_int),
    Offs(i32, i32, c_int),
    Scal(c_int, f64, f64),
    ScalFixed(c_int, i32, i32),
    ScalS(c_int, String, String),
    PCal(String, i32, i32, c_int, Vec<String>, String),
    Time(png_time),
    Text(Vec<(c_int, String, String, Option<(String, String)>)>),
    Splt(String, u8, Vec<png_sPLT_entry>),
    Exif(Vec<u8>),
    Cicp(u8, u8, u8, u8),
    Clli(u32, u32),
    ClliFixed(u32, u32),
    Mdcv([f64; 8], f64, f64),
    MdcvFixed([i32; 8], u32, u32),
    Unknown(Vec<png_unknown_chunk>, Vec<Vec<u8>>, c_int),
    MngFeatures(u32),
    CheckInvalidIndex(c_int),
    Benign(c_int),
    Option(c_int, c_int),
}

pub struct Img {
    pub width: u32,
    pub height: u32,
    pub bit_depth: c_int,
    pub color_type: c_int,
    pub interlace: c_int,
    pub palette: Vec<png_color>,
    pub rows: Vec<Vec<u8>>,
}

impl Img {
    fn gen(rng: &mut Rng, color_type: c_int, bit_depth: c_int, w: u32, h: u32, interlace: c_int) -> Img {
        let pd = channels_of(color_type) * bit_depth as u32;
        // Rows are over-allocated: some write transforms (png_set_filler /
        // png_set_packing) make libpng consume MORE application bytes per row
        // than PNG_ROWBYTES of the stored format.  Over-allocating keeps the
        // bytes libpng reads deterministic and identical for both libraries.
        let rb = rowbytes(pd, w)
            .max(rowbytes(4 * bit_depth as u32, w))
            .max(w as usize * 4)
            + 16;
        let rows = (0..h).map(|_| rng.bytes(rb)).collect();
        let npal = if color_type == PNG_COLOR_TYPE_PALETTE {
            1usize << bit_depth
        } else {
            0
        };
        let palette = (0..npal)
            .map(|_| png_color {
                red: rng.u8(),
                green: rng.u8(),
                blue: rng.u8(),
            })
            .collect();
        Img {
            width: w,
            height: h,
            bit_depth,
            color_type,
            interlace,
            palette,
            rows,
        }
    }
}

/// How the rows are handed to libpng.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RowMode {
    /// png_write_row, one row at a time (interlace handled by libpng passes)
    Row,
    /// png_write_rows with all rows at once
    Rows,
    /// png_write_image with a row-pointer array
    Image,
    /// png_write_png one-shot with the given transform mask
    OneShot(c_int),
}

pub struct Out {
    pub bytes: Vec<u8>,
    pub diag: Diag,
    pub ok: bool,
    pub flushes: u32,
}

impl PartialEq for Out {
    fn eq(&self, o: &Out) -> bool {
        self.bytes == o.bytes && self.diag == o.diag && self.ok == o.ok && self.flushes == o.flushes
    }
}

/// Keeps the CStrings/vectors used by the setters alive for the whole call.
#[derive(Default)]
struct Keep {
    cstrings: Vec<CString>,
    bytes: Vec<Vec<u8>>,
    texts: Vec<png_text>,
    splts: Vec<png_sPLT_t>,
    entries: Vec<Vec<png_sPLT_entry>>,
    chunks: Vec<png_unknown_chunk>,
    charps: Vec<Vec<png_charp>>,
    u16s: Vec<Vec<u16>>,
}

unsafe fn apply(api: &'static Api, png: png_structp, info: png_infop, opts: &[W], keep: &mut Keep) {
    for o in opts {
        match o {
            W::Bgr => (api.png_set_bgr)(png),
            W::Swap => (api.png_set_swap)(png),
            W::SwapAlpha => (api.png_set_swap_alpha)(png),
            W::InvertAlpha => (api.png_set_invert_alpha)(png),
            W::InvertMono => (api.png_set_invert_mono)(png),
            W::Packing => (api.png_set_packing)(png),
            W::PackSwap => (api.png_set_packswap)(png),
            W::Shift(s) => (api.png_set_shift)(png, s),
            W::Filler(v, loc) => (api.png_set_filler)(png, *v, *loc),
            W::Filter(f) => (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, *f),
            W::FilterHeuristics(m, w, c) => {
                (api.png_set_filter_heuristics)(
                    png,
                    *m,
                    w.len() as c_int,
                    w.as_ptr() as png_const_doublep,
                    c.as_ptr() as png_const_doublep,
                );
            }
            W::FilterHeuristicsFixed(m, w, c) => {
                (api.png_set_filter_heuristics_fixed)(
                    png,
                    *m,
                    w.len() as c_int,
                    w.as_ptr(),
                    c.as_ptr(),
                );
            }
            W::Level(v) => (api.png_set_compression_level)(png, *v),
            W::MemLevel(v) => (api.png_set_compression_mem_level)(png, *v),
            W::Strategy(v) => (api.png_set_compression_strategy)(png, *v),
            W::WindowBits(v) => (api.png_set_compression_window_bits)(png, *v),
            W::Method(v) => (api.png_set_compression_method)(png, *v),
            W::BufferSize(v) => (api.png_set_compression_buffer_size)(png, *v),
            W::TextLevel(v) => (api.png_set_text_compression_level)(png, *v),
            W::TextMemLevel(v) => (api.png_set_text_compression_mem_level)(png, *v),
            W::TextStrategy(v) => (api.png_set_text_compression_strategy)(png, *v),
            W::TextWindowBits(v) => (api.png_set_text_compression_window_bits)(png, *v),
            W::TextMethod(v) => (api.png_set_text_compression_method)(png, *v),
            W::FlushEvery(n) => (api.png_set_flush)(png, *n),
            W::SigBit(s) => (api.png_set_sBIT)(png, info, s),
            W::Gamma(g) => (api.png_set_gAMA)(png, info, *g),
            W::GammaFixed(g) => (api.png_set_gAMA_fixed)(png, info, *g),
            W::Srgb(i) => (api.png_set_sRGB)(png, info, *i),
            W::SrgbGamaChrm(i) => (api.png_set_sRGB_gAMA_and_cHRM)(png, info, *i),
            W::Chrm(v) => (api.png_set_cHRM)(
                png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
            ),
            W::ChrmFixed(v) => (api.png_set_cHRM_fixed)(
                png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
            ),
            W::ChrmXyz(v) => (api.png_set_cHRM_XYZ)(
                png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
            ),
            W::ChrmXyzFixed(v) => (api.png_set_cHRM_XYZ_fixed)(
                png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
            ),
            W::Iccp(name, prof) => {
                let n = cs(name);
                keep.bytes.push(prof.clone());
                let p = keep.bytes.last().unwrap().as_ptr();
                let len = prof.len() as png_uint_32;
                keep.cstrings.push(n);
                (api.png_set_iCCP)(
                    png,
                    info,
                    keep.cstrings.last().unwrap().as_ptr(),
                    0,
                    p,
                    len,
                );
            }
            W::Trns(alpha, col, num) => {
                keep.bytes.push(alpha.clone());
                let a = if alpha.is_empty() {
                    std::ptr::null_mut()
                } else {
                    keep.bytes.last().unwrap().as_ptr() as png_bytep
                };
                (api.png_set_tRNS)(png, info, a, *num, col as *const _ as png_color_16p);
            }
            W::Bkgd(b) => (api.png_set_bKGD)(png, info, b as *const _ as png_const_color_16p),
            W::Hist(h) => {
                keep.u16s.push(h.clone());
                (api.png_set_hIST)(png, info, keep.u16s.last().unwrap().as_ptr());
            }
            W::Phys(x, y, u) => (api.png_set_pHYs)(png, info, *x, *y, *u),
            W::Offs(x, y, u) => (api.png_set_oFFs)(png, info, *x, *y, *u),
            W::Scal(u, w, h) => (api.png_set_sCAL)(png, info, *u, *w, *h),
            W::ScalFixed(u, w, h) => (api.png_set_sCAL_fixed)(png, info, *u, *w, *h),
            W::ScalS(u, w, h) => {
                keep.cstrings.push(cs(w));
                let wp = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                keep.cstrings.push(cs(h));
                let hp = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                (api.png_set_sCAL_s)(png, info, *u, wp, hp);
            }
            W::PCal(purpose, x0, x1, typ, params, units) => {
                keep.cstrings.push(cs(purpose));
                let pp = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                keep.cstrings.push(cs(units));
                let up = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                let mut ptrs: Vec<png_charp> = Vec::new();
                for p in params {
                    keep.cstrings.push(cs(p));
                    ptrs.push(keep.cstrings.last().unwrap().as_ptr() as png_charp);
                }
                keep.charps.push(ptrs);
                let pa = keep.charps.last().unwrap();
                (api.png_set_pCAL)(
                    png,
                    info,
                    pp,
                    *x0,
                    *x1,
                    *typ,
                    pa.len() as c_int,
                    up,
                    if pa.is_empty() {
                        std::ptr::null_mut()
                    } else {
                        pa.as_ptr() as png_charpp
                    },
                );
            }
            W::Time(t) => (api.png_set_tIME)(png, info, t as *const _ as png_const_timep),
            W::Text(items) => {
                let base = keep.texts.len();
                for (comp, key, text, lang) in items {
                    keep.cstrings.push(cs(key));
                    let k = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                    keep.cstrings.push(cs(text));
                    let t = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                    let (l, lk) = match lang {
                        Some((l, lk)) => {
                            keep.cstrings.push(cs(l));
                            let a = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                            keep.cstrings.push(cs(lk));
                            let b = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                            (a, b)
                        }
                        None => (std::ptr::null_mut(), std::ptr::null_mut()),
                    };
                    keep.texts.push(png_text {
                        compression: *comp,
                        key: k,
                        text: t,
                        text_length: text.len(),
                        itxt_length: 0,
                        lang: l,
                        lang_key: lk,
                    });
                }
                let n = keep.texts.len() - base;
                (api.png_set_text)(png, info, keep.texts.as_ptr().add(base), n as c_int);
            }
            W::Splt(name, depth, entries) => {
                keep.cstrings.push(cs(name));
                let n = keep.cstrings.last().unwrap().as_ptr() as png_charp;
                keep.entries.push(entries.clone());
                let e = keep.entries.last().unwrap();
                keep.splts.push(png_sPLT_t {
                    name: n,
                    depth: *depth,
                    entries: e.as_ptr() as png_sPLT_entryp,
                    nentries: e.len() as png_int_32,
                });
                (api.png_set_sPLT)(png, info, keep.splts.last().unwrap(), 1);
            }
            W::Exif(e) => {
                keep.bytes.push(e.clone());
                (api.png_set_eXIf_1)(
                    png,
                    info,
                    e.len() as png_uint_32,
                    keep.bytes.last().unwrap().as_ptr() as png_bytep,
                );
            }
            W::Cicp(a, b, c_, d) => (api.png_set_cICP)(png, info, *a, *b, *c_, *d),
            W::Clli(a, b) => (api.png_set_cLLI)(png, info, *a as f64 / 10000.0, *b as f64 / 10000.0),
            W::ClliFixed(a, b) => (api.png_set_cLLI_fixed)(png, info, *a, *b),
            W::Mdcv(xy, a, b) => (api.png_set_mDCV)(
                png, info, xy[0], xy[1], xy[2], xy[3], xy[4], xy[5], xy[6], xy[7], *a, *b,
            ),
            W::MdcvFixed(xy, a, b) => (api.png_set_mDCV_fixed)(
                png, info, xy[0], xy[1], xy[2], xy[3], xy[4], xy[5], xy[6], xy[7], *a, *b,
            ),
            W::Unknown(chunks, datas, loc) => {
                let base = keep.chunks.len();
                for (i, ch) in chunks.iter().enumerate() {
                    keep.bytes.push(datas[i].clone());
                    let d = keep.bytes.last().unwrap().as_ptr() as *mut png_byte;
                    keep.chunks.push(png_unknown_chunk {
                        name: ch.name,
                        data: d,
                        size: datas[i].len(),
                        location: ch.location,
                    });
                }
                let n = keep.chunks.len() - base;
                (api.png_set_unknown_chunks)(
                    png,
                    info,
                    keep.chunks.as_ptr().add(base),
                    n as c_int,
                );
                for i in 0..n {
                    (api.png_set_unknown_chunk_location)(png, info, i as c_int, *loc);
                }
            }
            W::MngFeatures(f) => {
                (api.png_permit_mng_features)(png, *f);
            }
            W::CheckInvalidIndex(v) => (api.png_set_check_for_invalid_index)(png, *v),
            W::Benign(v) => (api.png_set_benign_errors)(png, *v),
            W::Option(o, v) => {
                (api.png_set_option)(png, *o, *v);
            }
        }
    }
}

/// Which options must be applied *before* png_write_info (they set info fields
/// serialised into the header) and which after (pure row transforms).
fn is_pre_info(o: &W) -> bool {
    !matches!(
        o,
        W::Bgr
            | W::Swap
            | W::SwapAlpha
            | W::InvertAlpha
            | W::InvertMono
            | W::Packing
            | W::PackSwap
            | W::Shift(_)
            | W::Filler(..)
    )
}

pub unsafe fn write_png(api: &'static Api, img: &Img, opts: &[W], mode: RowMode) -> Out {
    set_current_api(api);
    diag_reset();
    let mut sess = WriteSess::new(api);
    let png = sess.png;
    let info = sess.info;
    let mut keep = Keep::default();
    let ok = guard(|| {
        (api.png_set_IHDR)(
            png,
            info,
            img.width,
            img.height,
            img.bit_depth,
            img.color_type,
            img.interlace,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if !img.palette.is_empty() {
            (api.png_set_PLTE)(
                png,
                info,
                img.palette.as_ptr(),
                img.palette.len() as c_int,
            );
        }
        let pre: Vec<W> = opts.iter().filter(|o| is_pre_info(o)).cloned().collect();
        let post: Vec<W> = opts.iter().filter(|o| !is_pre_info(o)).cloned().collect();
        apply(api, png, info, &pre, &mut keep);

        match mode {
            RowMode::OneShot(t) => {
                let mut rowps: Vec<png_bytep> =
                    img.rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                (api.png_set_rows)(png, info, rowps.as_mut_ptr());
                apply(api, png, info, &post, &mut keep);
                (api.png_write_png)(png, info, t, std::ptr::null_mut());
            }
            _ => {
                (api.png_write_info)(png, info);
                apply(api, png, info, &post, &mut keep);
                let npasses = if img.interlace == PNG_INTERLACE_ADAM7 {
                    (api.png_set_interlace_handling)(png)
                } else {
                    1
                };
                match mode {
                    RowMode::Row => {
                        for _ in 0..npasses {
                            for r in &img.rows {
                                (api.png_write_row)(png, r.as_ptr());
                            }
                        }
                    }
                    RowMode::Rows => {
                        let mut rowps: Vec<png_bytep> =
                            img.rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                        for _ in 0..npasses {
                            (api.png_write_rows)(
                                png,
                                rowps.as_mut_ptr(),
                                img.rows.len() as png_uint_32,
                            );
                        }
                    }
                    RowMode::Image => {
                        let mut rowps: Vec<png_bytep> =
                            img.rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                        (api.png_write_image)(png, rowps.as_mut_ptr());
                    }
                    RowMode::OneShot(_) => unreachable!(),
                }
                (api.png_write_end)(png, info);
            }
        }
    })
    .is_some();
    let diag = diag_take();
    let bytes = std::mem::take(&mut sess.sink.buf);
    let flushes = sess.sink.flushes;
    drop(keep);
    Out {
        bytes,
        diag,
        ok,
        flushes,
    }
}

fn diff(label: &str, img: &Img, opts: &[W], mode: RowMode) {
    unsafe {
        let co = write_png(c_api(), img, opts, mode);
        let ro = write_png(rs_api(), img, opts, mode);
        assert_eq!(
            co.ok, ro.ok,
            "{}: error parity (C ok={} RS ok={})\n C diag {:?}\n RS diag {:?}",
            label, co.ok, ro.ok, co.diag, ro.diag
        );
        assert_eq!(co.diag, ro.diag, "{}: diagnostics", label);
        assert_eq!(co.flushes, ro.flushes, "{}: flush count", label);
        assert_bytes_eq(label, &co.bytes, &ro.bytes);
    }
}

// ---------------------------------------------------------------------------

#[test]
fn all_color_types_and_depths() {
    let mut rng = Rng::new(0xa1b2_c3d4_e5f6_0701);
    for (ct, bd) in legal_ihdr() {
        for interlace in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &[(1u32, 1u32), (1, 7), (7, 1), (5, 5), (17, 3), (32, 9)] {
                let img = Img::gen(&mut rng, ct, bd, w, h, interlace);
                for mode in [RowMode::Row, RowMode::Rows, RowMode::Image] {
                    if interlace == PNG_INTERLACE_ADAM7 && mode == RowMode::Image {
                        // png_write_image handles the passes itself
                    }
                    diff(
                        &format!(
                            "ct={} bd={} il={} {}x{} {:?}",
                            ct, bd, interlace, w, h, mode
                        ),
                        &img,
                        &[],
                        mode,
                    );
                }
            }
        }
    }
}

#[test]
fn filter_selection() {
    let mut rng = Rng::new(0xb2c3_d4e5_f607_1801);
    let filters = [
        PNG_NO_FILTERS,
        PNG_FILTER_NONE,
        PNG_FILTER_SUB,
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_FAST_FILTERS,
        PNG_ALL_FILTERS,
        PNG_FILTER_SUB | PNG_FILTER_PAETH,
        PNG_FILTER_NONE | PNG_FILTER_AVG,
    ];
    for (ct, bd) in legal_ihdr() {
        for &f in &filters {
            for interlace in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                let img = Img::gen(&mut rng, ct, bd, 23, 11, interlace);
                diff(
                    &format!("filter ct={} bd={} f={:#x} il={}", ct, bd, f, interlace),
                    &img,
                    &[W::Filter(f)],
                    RowMode::Image,
                );
            }
        }
    }
}

#[test]
fn weighted_filter_heuristics() {
    let mut rng = Rng::new(0xc3d4_e5f6_0718_2901);
    for (ct, bd) in [
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
        (PNG_COLOR_TYPE_PALETTE, 4),
    ] {
        for method in [
            PNG_FILTER_HEURISTIC_DEFAULT,
            PNG_FILTER_HEURISTIC_UNWEIGHTED,
            PNG_FILTER_HEURISTIC_WEIGHTED,
        ] {
            for nw in [0usize, 1, 3] {
                let w: Vec<f64> = (0..nw).map(|i| 1.0 + i as f64 * 0.5).collect();
                let c: Vec<f64> = (0..5).map(|i| 1.0 + i as f64 * 0.25).collect();
                let img = Img::gen(&mut rng, ct, bd, 29, 13, PNG_INTERLACE_NONE);
                diff(
                    &format!("heur ct={} bd={} m={} nw={}", ct, bd, method, nw),
                    &img,
                    &[
                        W::Filter(PNG_ALL_FILTERS),
                        W::FilterHeuristics(method, w.clone(), c.clone()),
                    ],
                    RowMode::Image,
                );
                let wf: Vec<i32> = w.iter().map(|x| (x * 100000.0) as i32).collect();
                let cf: Vec<i32> = c.iter().map(|x| (x * 100000.0) as i32).collect();
                diff(
                    &format!("heurfx ct={} bd={} m={} nw={}", ct, bd, method, nw),
                    &img,
                    &[
                        W::Filter(PNG_ALL_FILTERS),
                        W::FilterHeuristicsFixed(method, wf, cf),
                    ],
                    RowMode::Image,
                );
            }
        }
    }
}

#[test]
fn compression_settings() {
    let mut rng = Rng::new(0xd4e5_f607_1829_3a01);
    let img = Img::gen(&mut rng, PNG_COLOR_TYPE_RGB, 8, 40, 20, PNG_INTERLACE_NONE);
    for lvl in [-1i32, 0, 1, 5, 9] {
        diff(&format!("level {}", lvl), &img, &[W::Level(lvl)], RowMode::Image);
    }
    for ml in [1i32, 8, 9] {
        diff(&format!("memlevel {}", ml), &img, &[W::MemLevel(ml)], RowMode::Image);
    }
    for st in [0i32, 1, 2, 3, 4] {
        diff(&format!("strategy {}", st), &img, &[W::Strategy(st)], RowMode::Image);
    }
    for wb in [8i32, 9, 12, 15] {
        diff(&format!("windowbits {}", wb), &img, &[W::WindowBits(wb)], RowMode::Image);
    }
    diff("method 8", &img, &[W::Method(8)], RowMode::Image);
    for bs in [1usize, 2, 3, 64, 1024, 8192, 100_000] {
        diff(&format!("bufsize {}", bs), &img, &[W::BufferSize(bs)], RowMode::Image);
    }
    // combinations
    let mut rng2 = Rng::new(0x1122_3344_5566_7781);
    for _ in 0..40 {
        let opts = vec![
            W::Level(rng2.range(-1, 9) as i32),
            W::MemLevel(rng2.range(1, 9) as i32),
            W::Strategy(rng2.range(0, 4) as i32),
            W::WindowBits(rng2.range(8, 15) as i32),
            W::BufferSize(1 + rng2.below(4096) as usize),
            W::Filter(PNG_ALL_FILTERS),
        ];
        diff(&format!("combo {:?}", opts), &img, &opts, RowMode::Image);
    }
}

#[test]
fn write_transforms() {
    let mut rng = Rng::new(0xe5f6_0718_293a_4b01);
    let cases: Vec<(c_int, c_int, Vec<W>)> = vec![
        (PNG_COLOR_TYPE_RGB, 8, vec![W::Bgr]),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8, vec![W::Bgr]),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8, vec![W::SwapAlpha]),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8, vec![W::InvertAlpha]),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8, vec![W::SwapAlpha, W::InvertAlpha]),
        (PNG_COLOR_TYPE_RGB, 16, vec![W::Swap]),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16, vec![W::Swap, W::Bgr]),
        (PNG_COLOR_TYPE_GRAY, 16, vec![W::Swap]),
        (PNG_COLOR_TYPE_GRAY, 1, vec![W::InvertMono]),
        (PNG_COLOR_TYPE_GRAY, 2, vec![W::PackSwap]),
        (PNG_COLOR_TYPE_GRAY, 4, vec![W::PackSwap, W::InvertMono]),
        (PNG_COLOR_TYPE_PALETTE, 4, vec![W::PackSwap]),
        (
            PNG_COLOR_TYPE_GRAY,
            8,
            vec![W::Shift(png_color_8 {
                red: 0,
                green: 0,
                blue: 0,
                gray: 5,
                alpha: 0,
            })],
        ),
        (
            PNG_COLOR_TYPE_RGB,
            8,
            vec![W::Shift(png_color_8 {
                red: 5,
                green: 6,
                blue: 4,
                gray: 0,
                alpha: 0,
            })],
        ),
        (
            PNG_COLOR_TYPE_RGB_ALPHA,
            16,
            vec![W::Shift(png_color_8 {
                red: 12,
                green: 13,
                blue: 11,
                gray: 0,
                alpha: 10,
            })],
        ),
    ];
    for (ct, bd, opts) in cases {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let img = Img::gen(&mut rng, ct, bd, 19, 7, il);
            diff(
                &format!("xform ct={} bd={} il={} {:?}", ct, bd, il, opts),
                &img,
                &opts,
                RowMode::Image,
            );
        }
    }
}

#[test]
fn packing_and_filler() {
    let mut rng = Rng::new(0xf607_1829_3a4b_5c01);
    // png_set_packing: rows are supplied one sample per byte for depths < 8
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 2),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_PALETTE, 1),
        (PNG_COLOR_TYPE_PALETTE, 2),
        (PNG_COLOR_TYPE_PALETTE, 4),
    ] {
        for &w in &[1u32, 3, 8, 15] {
            let h = 4;
            let mut img = Img::gen(&mut rng, ct, bd, w, h, PNG_INTERLACE_NONE);
            // one byte per sample
            img.rows = (0..h).map(|_| rng.bytes(w as usize)).collect();
            diff(
                &format!("packing ct={} bd={} w={}", ct, bd, w),
                &img,
                &[W::Packing],
                RowMode::Image,
            );
        }
    }
    // png_set_filler on write strips the filler channel
    for (ct, bd, chan) in [
        (PNG_COLOR_TYPE_RGB, 8, 4usize),
        (PNG_COLOR_TYPE_GRAY, 8, 2),
        (PNG_COLOR_TYPE_RGB, 16, 4),
        (PNG_COLOR_TYPE_GRAY, 16, 2),
    ] {
        for loc in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
            let (w, h) = (13u32, 5u32);
            let mut img = Img::gen(&mut rng, ct, bd, w, h, PNG_INTERLACE_NONE);
            let per = chan * (bd as usize / 8);
            img.rows = (0..h).map(|_| rng.bytes(w as usize * per)).collect();
            diff(
                &format!("filler ct={} bd={} loc={}", ct, bd, loc),
                &img,
                &[W::Filler(0xabcd, loc)],
                RowMode::Image,
            );
        }
    }
}

#[test]
fn flush_behaviour() {
    let mut rng = Rng::new(0x0718_293a_4b5c_6d01);
    let img = Img::gen(&mut rng, PNG_COLOR_TYPE_RGB, 8, 30, 30, PNG_INTERLACE_NONE);
    for n in [0i32, 1, 2, 5, 1000] {
        diff(
            &format!("flush {}", n),
            &img,
            &[W::FlushEvery(n), W::BufferSize(64)],
            RowMode::Row,
        );
    }
}

#[test]
fn one_shot_write_png_transforms() {
    let mut rng = Rng::new(0x1829_3a4b_5c6d_7e01);
    let masks = [
        PNG_TRANSFORM_IDENTITY,
        PNG_TRANSFORM_PACKING,
        PNG_TRANSFORM_PACKSWAP,
        PNG_TRANSFORM_INVERT_MONO,
        PNG_TRANSFORM_SHIFT,
        PNG_TRANSFORM_BGR,
        PNG_TRANSFORM_SWAP_ALPHA,
        PNG_TRANSFORM_SWAP_ENDIAN,
        PNG_TRANSFORM_INVERT_ALPHA,
        PNG_TRANSFORM_STRIP_FILLER_BEFORE,
        PNG_TRANSFORM_STRIP_FILLER_AFTER,
        PNG_TRANSFORM_STRIP_16,
        PNG_TRANSFORM_STRIP_ALPHA,
        PNG_TRANSFORM_EXPAND,
        PNG_TRANSFORM_GRAY_TO_RGB,
        PNG_TRANSFORM_EXPAND_16,
        PNG_TRANSFORM_SCALE_16,
        PNG_TRANSFORM_BGR | PNG_TRANSFORM_SWAP_ENDIAN,
    ];
    for (ct, bd) in legal_ihdr() {
        for &m in &masks {
            let img = Img::gen(&mut rng, ct, bd, 11, 5, PNG_INTERLACE_NONE);
            diff(
                &format!("write_png ct={} bd={} m={:#x}", ct, bd, m),
                &img,
                &[W::SigBit(png_color_8 {
                    red: 4,
                    green: 4,
                    blue: 4,
                    gray: 4,
                    alpha: 4,
                })],
                RowMode::OneShot(m),
            );
        }
    }
}

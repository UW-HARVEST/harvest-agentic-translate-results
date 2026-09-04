//! Phase B — the write pipeline, driven through the LOW-LEVEL entry points
//! (`png_write_info` / `png_write_row` / `png_write_rows` / `png_write_image` /
//! `png_write_end`) as well as the one-shot `png_write_png`.
//!
//! Both libraries are driven with identical inputs and the complete output byte
//! stream, plus the ordered warning/flush/status transcript, must match.
mod common;

use common::api::{apis, Api};
use common::harness::*;
use common::pngbuild as pb;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// configuration description
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct WCfg {
    pub width: u32,
    pub height: u32,
    pub bit_depth: c_int,
    pub color_type: c_int,
    pub interlace: c_int,
    /// `png_set_filter` argument, `None` to leave the default alone.
    pub filters: Option<c_int>,
    pub level: Option<c_int>,
    pub mem_level: Option<c_int>,
    pub strategy: Option<c_int>,
    pub window_bits: Option<c_int>,
    pub method: Option<c_int>,
    pub buffer_size: Option<usize>,
    pub flush_rows: Option<c_int>,
    /// write transforms applied to the caller's rows
    pub bgr: bool,
    pub swap: bool,
    pub swap_alpha: bool,
    pub invert_alpha: bool,
    pub invert_mono: bool,
    pub packing: bool,
    pub packswap: bool,
    pub shift: Option<png_color_8>,
    pub filler: Option<(u32, c_int)>,
    /// ancillary chunks
    pub gamma: Option<i32>,
    pub srgb: Option<c_int>,
    pub sbit: Option<png_color_8>,
    pub bkgd: Option<png_color_16>,
    pub trns: bool,
    pub hist: bool,
    pub phys: Option<(u32, u32, c_int)>,
    pub offs: Option<(i32, i32, c_int)>,
    pub time: Option<png_time>,
    pub text: Vec<(Vec<u8>, Vec<u8>, c_int)>,
    pub scal: Option<(c_int, i32, i32)>,
    pub chrm: bool,
    pub cicp: Option<(u8, u8, u8, u8)>,
    pub clli: Option<(u32, u32)>,
    pub exif: Option<Vec<u8>>,
    pub unknown: Vec<([u8; 5], Vec<u8>, c_int)>,
    pub mng: Option<u32>,
    /// which row-writing entry point to use
    pub mode: WMode,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WMode {
    /// one `png_write_row` call per row
    Row,
    /// `png_write_rows` in chunks of n
    Rows(u32),
    /// `png_write_image` with a full row-pointer array
    Image,
    /// `png_write_png` with the given transform mask
    WholePng(c_int),
}

impl WCfg {
    pub fn new(width: u32, height: u32, bit_depth: c_int, color_type: c_int, interlace: c_int) -> Self {
        WCfg {
            width,
            height,
            bit_depth,
            color_type,
            interlace,
            filters: None,
            level: None,
            mem_level: None,
            strategy: None,
            window_bits: None,
            method: None,
            buffer_size: None,
            flush_rows: None,
            bgr: false,
            swap: false,
            swap_alpha: false,
            invert_alpha: false,
            invert_mono: false,
            packing: false,
            packswap: false,
            shift: None,
            filler: None,
            gamma: None,
            srgb: None,
            sbit: None,
            bkgd: None,
            trns: false,
            hist: false,
            phys: None,
            offs: None,
            time: None,
            text: Vec::new(),
            scal: None,
            chrm: false,
            cicp: None,
            clli: None,
            exif: None,
            unknown: Vec::new(),
            mng: None,
            mode: WMode::Row,
            seed: 1,
        }
    }

    fn channels(&self) -> u32 {
        pb::channels_of(self.color_type as u8)
    }

    /// Bytes per caller-supplied row (before any write transform that changes
    /// the layout).  `png_set_packing` means the caller supplies one byte per
    /// sample; `png_set_filler` means one extra channel.
    fn user_rowbytes(&self) -> usize {
        // `png_write_png` applies the same transforms through the transform mask
        let tmask = match self.mode {
            WMode::WholePng(m) => m,
            _ => 0,
        };
        let mut ch = self.channels();
        let want_filler = self.filler.is_some()
            || (tmask & (PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER))
                != 0;
        // png_set_filler on write strips one channel; pngwrite.c documents the
        // input colour type must be G or RGB (no alpha, not palette).
        if want_filler && (self.color_type == 0 || self.color_type == 2) {
            ch += 1;
        }
        let packing = self.packing || (tmask & PNG_TRANSFORM_PACKING) != 0;
        let depth = if packing && self.bit_depth < 8 {
            8
        } else {
            self.bit_depth as u32
        };
        (((self.width as u64) * (ch as u64) * (depth as u64) + 7) / 8) as usize
    }

    fn palette(&self) -> Vec<png_color> {
        if self.color_type != 3 {
            return Vec::new();
        }
        let mut rng = Rng::new(self.seed ^ 0x9999);
        let n = (1usize << self.bit_depth.min(8)).min(256);
        (0..n)
            .map(|_| png_color {
                red: rng.next_u8(),
                green: rng.next_u8(),
                blue: rng.next_u8(),
            })
            .collect()
    }

    /// The caller-visible rows.  For colour type 3 the samples are palette
    /// indices, clamped to the palette size so no invalid-index warning fires.
    fn rows(&self) -> Vec<Vec<u8>> {
        let rb = self.user_rowbytes();
        let mut rng = Rng::new(self.seed);
        let npal = self.palette().len().max(1);
        let nrows = self.height as usize;
        (0..nrows)
            .map(|_| {
                let mut r: Vec<u8> = (0..rb).map(|_| rng.next_u8()).collect();
                if self.color_type == 3 && npal < 256 {
                    // keep every index inside the palette so that no
                    // invalid-index warning fires on a valid-path test
                    for b in r.iter_mut() {
                        *b %= npal as u8;
                    }
                }
                r
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// the driver
// ---------------------------------------------------------------------------

unsafe fn set_ancillary(a: &Api, p: png_structp, info: png_infop, cfg: &WCfg, keep: &mut Vec<Vec<u8>>) {
    if let Some(g) = cfg.gamma {
        (a.png_set_gAMA_fixed)(p, info, g);
    }
    if let Some(i) = cfg.srgb {
        (a.png_set_sRGB)(p, info, i);
    }
    if cfg.chrm {
        (a.png_set_cHRM_fixed)(p, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
    }
    if let Some(s) = cfg.sbit {
        (a.png_set_sBIT)(p, info, &s);
    }
    if let Some(b) = cfg.bkgd {
        (a.png_set_bKGD)(p, info, &b);
    }
    if let Some((x, y, u)) = cfg.phys {
        (a.png_set_pHYs)(p, info, x, y, u);
    }
    if let Some((x, y, u)) = cfg.offs {
        (a.png_set_oFFs)(p, info, x, y, u);
    }
    if let Some(t) = cfg.time {
        (a.png_set_tIME)(p, info, &t);
    }
    if let Some((u, w, h)) = cfg.scal {
        (a.png_set_sCAL_fixed)(p, info, u, w, h);
    }
    if let Some((cp, tf, mc, vf)) = cfg.cicp {
        (a.png_set_cICP)(p, info, cp, tf, mc, vf);
    }
    if let Some((m, f)) = cfg.clli {
        (a.png_set_cLLI_fixed)(p, info, m, f);
    }
    if let Some(e) = &cfg.exif {
        keep.push(e.clone());
        let b = keep.last_mut().unwrap();
        (a.png_set_eXIf_1)(p, info, b.len() as u32, b.as_mut_ptr());
    }
    if !cfg.text.is_empty() {
        // keep the C strings alive for the duration of the call
        let mut texts: Vec<png_text> = Vec::new();
        for (k, v, comp) in &cfg.text {
            let mut key = k.clone();
            key.push(0);
            let mut val = v.clone();
            val.push(0);
            keep.push(key);
            let kp = keep.last_mut().unwrap().as_mut_ptr() as *mut c_char;
            keep.push(val);
            let vp = keep.last_mut().unwrap().as_mut_ptr() as *mut c_char;
            texts.push(png_text {
                compression: *comp,
                key: kp,
                text: vp,
                text_length: 0,
                itxt_length: 0,
                lang: std::ptr::null_mut(),
                lang_key: std::ptr::null_mut(),
            });
        }
        (a.png_set_text)(p, info, texts.as_ptr(), texts.len() as c_int);
    }
    if !cfg.unknown.is_empty() {
        let mut chunks: Vec<png_unknown_chunk> = Vec::new();
        for (name, data, loc) in &cfg.unknown {
            keep.push(data.clone());
            let dp = keep.last_mut().unwrap();
            chunks.push(png_unknown_chunk {
                name: *name,
                data: if dp.is_empty() {
                    std::ptr::null_mut()
                } else {
                    dp.as_mut_ptr()
                },
                size: dp.len(),
                location: *loc as u8,
            });
        }
        (a.png_set_keep_unknown_chunks)(
            p,
            PNG_HANDLE_CHUNK_ALWAYS,
            std::ptr::null(),
            0,
        );
        (a.png_set_unknown_chunks)(p, info, chunks.as_ptr(), chunks.len() as c_int);
        for i in 0..chunks.len() {
            (a.png_set_unknown_chunk_location)(p, info, i as c_int, chunks[i].location as c_int);
        }
    }
}

unsafe fn set_transforms(a: &Api, p: png_structp, cfg: &WCfg) {
    if cfg.bgr {
        (a.png_set_bgr)(p);
    }
    if cfg.swap {
        (a.png_set_swap)(p);
    }
    if cfg.swap_alpha {
        (a.png_set_swap_alpha)(p);
    }
    if cfg.invert_alpha {
        (a.png_set_invert_alpha)(p);
    }
    if cfg.invert_mono {
        (a.png_set_invert_mono)(p);
    }
    if cfg.packing {
        (a.png_set_packing)(p);
    }
    if cfg.packswap {
        (a.png_set_packswap)(p);
    }
    if let Some(s) = cfg.shift {
        (a.png_set_shift)(p, &s);
    }
    if let Some((f, fl)) = cfg.filler {
        (a.png_set_filler)(p, f, fl);
    }
}

/// Run one write configuration against one library; returns the produced PNG
/// bytes and the callback transcript.
unsafe fn run_write(a: &Api, is_c: bool, cfg: &WCfg) -> (Vec<u8>, Vec<String>) {
    set_cur_is_c(is_c);
    reset_all();

    let p = (a.png_create_write_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null(), "png_create_write_struct returned NULL");
    let mut p = p;
    let mut info = (a.png_create_info_struct)(p);
    assert!(!info.is_null());

    (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
    (a.png_set_write_status_fn)(p, Some(write_status_cb));

    if let Some(m) = cfg.mng {
        let r = (a.png_permit_mng_features)(p, m);
        log_push(format!("MNG:{r}"));
    }
    if let Some(sz) = cfg.buffer_size {
        (a.png_set_compression_buffer_size)(p, sz);
    }
    if let Some(l) = cfg.level {
        (a.png_set_compression_level)(p, l);
    }
    if let Some(l) = cfg.mem_level {
        (a.png_set_compression_mem_level)(p, l);
    }
    if let Some(s) = cfg.strategy {
        (a.png_set_compression_strategy)(p, s);
    }
    if let Some(w) = cfg.window_bits {
        (a.png_set_compression_window_bits)(p, w);
    }
    if let Some(m) = cfg.method {
        (a.png_set_compression_method)(p, m);
    }
    if let Some(f) = cfg.flush_rows {
        (a.png_set_flush)(p, f);
    }

    (a.png_set_IHDR)(
        p,
        info,
        cfg.width,
        cfg.height,
        cfg.bit_depth,
        cfg.color_type,
        cfg.interlace,
        PNG_COMPRESSION_TYPE_BASE,
        if cfg.mng.is_some() && cfg.color_type == 2 {
            PNG_FILTER_TYPE_BASE
        } else {
            PNG_FILTER_TYPE_BASE
        },
    );

    let pal = cfg.palette();
    if !pal.is_empty() {
        (a.png_set_PLTE)(p, info, pal.as_ptr(), pal.len() as c_int);
    }

    let mut keep: Vec<Vec<u8>> = Vec::new();
    // tRNS / hIST need the palette to exist first
    let mut trns_alpha: Vec<u8> = Vec::new();
    let mut trns_col = png_color_16::default();
    if cfg.trns {
        match cfg.color_type {
            3 => {
                let mut rng = Rng::new(cfg.seed ^ 0x77);
                trns_alpha = (0..pal.len()).map(|_| rng.next_u8()).collect();
                (a.png_set_tRNS)(
                    p,
                    info,
                    trns_alpha.as_ptr(),
                    trns_alpha.len() as c_int,
                    std::ptr::null(),
                );
            }
            0 => {
                trns_col.gray = 1;
                (a.png_set_tRNS)(p, info, std::ptr::null(), 0, &trns_col);
            }
            2 => {
                trns_col.red = 1;
                trns_col.green = 2;
                trns_col.blue = 3;
                (a.png_set_tRNS)(p, info, std::ptr::null(), 0, &trns_col);
            }
            _ => {}
        }
    }
    let mut hist: Vec<u16> = Vec::new();
    if cfg.hist && cfg.color_type == 3 {
        let mut rng = Rng::new(cfg.seed ^ 0x55);
        hist = (0..pal.len()).map(|_| rng.next_u16()).collect();
        (a.png_set_hIST)(p, info, hist.as_ptr());
    }

    set_ancillary(a, p, info, cfg, &mut keep);

    if let Some(f) = cfg.filters {
        (a.png_set_filter)(p, PNG_FILTER_TYPE_BASE, f);
    }

    let rows = cfg.rows();

    if let WMode::WholePng(transforms) = cfg.mode {
        let mut rowptrs: Vec<*mut png_byte> = Vec::new();
        let mut owned: Vec<Vec<u8>> = rows.clone();
        for r in owned.iter_mut() {
            rowptrs.push(r.as_mut_ptr());
        }
        (a.png_set_rows)(p, info, rowptrs.as_mut_ptr());
        (a.png_write_png)(p, info, transforms, std::ptr::null_mut());
        (a.png_destroy_write_struct)(&mut p, &mut info);
        return (out_take(), log_take());
    }

    (a.png_write_info)(p, info);
    set_transforms(a, p, cfg);

    let passes = if cfg.interlace == PNG_INTERLACE_ADAM7 {
        (a.png_set_interlace_handling)(p)
    } else {
        1
    };
    log_push(format!("PASSES:{passes}"));

    let mut owned: Vec<Vec<u8>> = rows.clone();
    match cfg.mode {
        WMode::Row => {
            for _ in 0..passes {
                for r in owned.iter() {
                    (a.png_write_row)(p, r.as_ptr());
                }
            }
        }
        WMode::Rows(n) => {
            let mut ptrs: Vec<*mut png_byte> = owned.iter_mut().map(|r| r.as_mut_ptr()).collect();
            for _ in 0..passes {
                let mut i = 0usize;
                while i < ptrs.len() {
                    let k = (n as usize).min(ptrs.len() - i);
                    (a.png_write_rows)(p, ptrs[i..].as_mut_ptr(), k as u32);
                    i += k;
                }
            }
        }
        WMode::Image => {
            let mut ptrs: Vec<*mut png_byte> = owned.iter_mut().map(|r| r.as_mut_ptr()).collect();
            (a.png_write_image)(p, ptrs.as_mut_ptr());
        }
        WMode::WholePng(_) => unreachable!(),
    }

    (a.png_write_end)(p, info);
    (a.png_destroy_write_struct)(&mut p, &mut info);
    (out_take(), log_take())
}

/// Assert that both libraries produce identical output for `cfg`.
#[track_caller]
fn diff_write(cfg: &WCfg) {
    let b = apis();
    let (co, cl) = unsafe { run_write(&b.c, true, cfg) };
    let (ro, rl) = unsafe { run_write(&b.rs, false, cfg) };
    eq_bytes(&format!("write output for {cfg:?}"), &co, &ro);
    eq_dbg(&format!("write transcript for {cfg:?}"), cl, rl);
    assert!(!co.is_empty(), "no output produced for {cfg:?}");
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows
// ---------------------------------------------------------------------------

/// The legal (bit_depth, color_type) pairs.
pub const DEPTH_TYPE: [(c_int, c_int); 15] = [
    (1, 0),
    (2, 0),
    (4, 0),
    (8, 0),
    (16, 0),
    (8, 2),
    (16, 2),
    (1, 3),
    (2, 3),
    (4, 3),
    (8, 3),
    (8, 4),
    (16, 4),
    (8, 6),
    (16, 6),
];

#[test]
fn all_depth_colour_interlace() {
    // every legal depth/colour combination x interlaced/not x a set of widths
    // that straddle the sub-byte packing boundaries, x heights 1..
    let widths = [1u32, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 33];
    let heights = [1u32, 2, 3, 8, 9];
    let mut seed = 0x2000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &w in &widths {
                for &h in &heights {
                    seed += 1;
                    let mut cfg = WCfg::new(w, h, bd, ct, il);
                    cfg.seed = seed;
                    diff_write(&cfg);
                }
            }
        }
    }
}

#[test]
fn all_filter_combinations() {
    // png_set_filter with every subset of the 5 filters, on the colour types
    // whose pixel depth selects different filter code paths
    let mut seed = 0x3000u64;
    for &(bd, ct) in &[(1i32, 0i32), (8, 0), (16, 0), (8, 2), (16, 2), (8, 3), (8, 4), (16, 6)] {
        for mask in 0..32u32 {
            let filters = ((mask & 1) as c_int) * PNG_FILTER_NONE
                | (((mask >> 1) & 1) as c_int) * PNG_FILTER_SUB
                | (((mask >> 2) & 1) as c_int) * PNG_FILTER_UP
                | (((mask >> 3) & 1) as c_int) * PNG_FILTER_AVG
                | (((mask >> 4) & 1) as c_int) * PNG_FILTER_PAETH;
            if filters == 0 {
                // PNG_NO_FILTERS is legal and means "None only"
            }
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                seed += 1;
                let mut cfg = WCfg::new(13, 7, bd, ct, il);
                cfg.filters = Some(filters);
                cfg.seed = seed;
                diff_write(&cfg);
            }
        }
    }
    // the named single-filter values too
    for f in [PNG_NO_FILTERS, PNG_FILTER_NONE, PNG_FILTER_SUB, PNG_FILTER_UP,
              PNG_FILTER_AVG, PNG_FILTER_PAETH, PNG_ALL_FILTERS] {
        seed += 1;
        let mut cfg = WCfg::new(23, 11, 8, 6, PNG_INTERLACE_NONE);
        cfg.filters = Some(f);
        cfg.seed = seed;
        diff_write(&cfg);
    }
}

#[test]
fn compression_options() {
    let mut seed = 0x4000u64;
    for level in [-1i32, 0, 1, 5, 9] {
        for strategy in [0i32, 1, 2, 3, 4] {
            seed += 1;
            let mut cfg = WCfg::new(31, 9, 8, 2, PNG_INTERLACE_NONE);
            cfg.level = Some(level);
            cfg.strategy = Some(strategy);
            cfg.seed = seed;
            diff_write(&cfg);
        }
    }
    for mem_level in [1i32, 4, 8, 9] {
        seed += 1;
        let mut cfg = WCfg::new(31, 9, 8, 2, PNG_INTERLACE_NONE);
        cfg.mem_level = Some(mem_level);
        cfg.seed = seed;
        diff_write(&cfg);
    }
    for wb in [8i32, 9, 10, 12, 15] {
        seed += 1;
        let mut cfg = WCfg::new(64, 20, 8, 2, PNG_INTERLACE_NONE);
        cfg.window_bits = Some(wb);
        cfg.seed = seed;
        diff_write(&cfg);
    }
    for bs in [1usize, 2, 15, 100, 1024, 8192, 65536] {
        seed += 1;
        let mut cfg = WCfg::new(64, 20, 8, 6, PNG_INTERLACE_NONE);
        cfg.buffer_size = Some(bs);
        cfg.seed = seed;
        diff_write(&cfg);
    }
    // method 8 is the only legal one
    seed += 1;
    let mut cfg = WCfg::new(20, 5, 8, 2, PNG_INTERLACE_NONE);
    cfg.method = Some(8);
    cfg.seed = seed;
    diff_write(&cfg);
}

#[test]
fn write_entry_points() {
    let mut seed = 0x5000u64;
    for &(bd, ct) in &[(1i32, 0i32), (8, 3), (8, 2), (16, 6)] {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for mode in [
                WMode::Row,
                WMode::Rows(1),
                WMode::Rows(2),
                WMode::Rows(3),
                WMode::Rows(100),
                WMode::Image,
            ] {
                seed += 1;
                let mut cfg = WCfg::new(19, 6, bd, ct, il);
                cfg.mode = mode;
                cfg.seed = seed;
                diff_write(&cfg);
            }
        }
    }
}

#[test]
fn write_png_one_shot() {
    let mut seed = 0x6000u64;
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
        PNG_TRANSFORM_BGR | PNG_TRANSFORM_SWAP_ALPHA,
        PNG_TRANSFORM_BGR | PNG_TRANSFORM_INVERT_ALPHA | PNG_TRANSFORM_SWAP_ENDIAN,
    ];
    for &(bd, ct) in &[(1i32, 0i32), (2, 0), (4, 0), (8, 0), (16, 0), (8, 2), (16, 2),
                       (8, 3), (8, 4), (16, 4), (8, 6), (16, 6)] {
        for &m in &masks {
            // pngwrite.c:1461 -- STRIP_FILLER expects the input colour type to
            // be G or RGB with no alpha channel (and not palette).
            // `png_set_filler` additionally rejects low-bit-depth grey with
            // "png_set_filler is invalid for low bit depth gray output"
            // (pngtrans.c) -- that rejection is a Phase C row, not a valid path.
            if (m & (PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER)) != 0
                && !((ct == PNG_COLOR_TYPE_GRAY && bd >= 8) || ct == PNG_COLOR_TYPE_RGB)
            {
                continue;
            }
            // SHIFT requires sBIT
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                seed += 1;
                let mut cfg = WCfg::new(11, 5, bd, ct, il);
                cfg.mode = WMode::WholePng(m);
                if m & PNG_TRANSFORM_SHIFT != 0 {
                    let d = bd.min(8) as u8;
                    cfg.sbit = Some(png_color_8 {
                        red: d,
                        green: d,
                        blue: d,
                        gray: d,
                        alpha: d,
                    });
                }
                cfg.seed = seed;
                diff_write(&cfg);
            }
        }
    }
}

#[test]
fn write_transforms() {
    let mut seed = 0x7000u64;
    // bgr / swap / swap_alpha / invert_alpha / invert_mono / packswap
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for k in 0..6 {
            seed += 1;
            let mut cfg = WCfg::new(17, 4, bd, ct, PNG_INTERLACE_NONE);
            match k {
                0 => cfg.bgr = true,
                1 => cfg.swap = true,
                2 => cfg.swap_alpha = true,
                3 => cfg.invert_alpha = true,
                4 => cfg.invert_mono = true,
                _ => cfg.packswap = true,
            }
            cfg.seed = seed;
            diff_write(&cfg);
        }
        // all at once
        seed += 1;
        let mut cfg = WCfg::new(17, 4, bd, ct, PNG_INTERLACE_ADAM7);
        cfg.bgr = true;
        cfg.swap = true;
        cfg.swap_alpha = true;
        cfg.invert_alpha = true;
        cfg.invert_mono = true;
        cfg.packswap = true;
        cfg.seed = seed;
        diff_write(&cfg);
    }
    // png_set_packing: caller supplies 1 byte/sample for depths < 8
    for &(bd, ct) in &[(1i32, 0i32), (2, 0), (4, 0), (1, 3), (2, 3), (4, 3)] {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for w in [1u32, 2, 3, 7, 8, 9, 16, 17] {
                seed += 1;
                let mut cfg = WCfg::new(w, 3, bd, ct, il);
                cfg.packing = true;
                cfg.seed = seed;
                diff_write(&cfg);
                seed += 1;
                let mut cfg = WCfg::new(w, 3, bd, ct, il);
                cfg.packing = true;
                cfg.packswap = true;
                cfg.seed = seed;
                diff_write(&cfg);
            }
        }
    }
    // png_set_shift with every legal sBIT
    for &(bd, ct) in &[(8i32, 0i32), (16, 0), (8, 2), (16, 2), (8, 4), (16, 6), (4, 0)] {
        let maxd = bd.min(8) as u8;
        for d in 1..=maxd {
            seed += 1;
            let mut cfg = WCfg::new(9, 3, bd, ct, PNG_INTERLACE_NONE);
            let s = png_color_8 {
                red: d,
                green: d,
                blue: d,
                gray: d,
                alpha: d,
            };
            cfg.sbit = Some(s);
            cfg.shift = Some(s);
            cfg.seed = seed;
            diff_write(&cfg);
        }
    }
    // png_set_filler on write == strip the extra channel
    for &(bd, ct) in &[(8i32, 0i32), (16, 0), (8, 2), (16, 2)] {
        for fl in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
            seed += 1;
            let mut cfg = WCfg::new(9, 3, bd, ct, PNG_INTERLACE_NONE);
            cfg.filler = Some((0xffff, fl));
            cfg.seed = seed;
            diff_write(&cfg);
        }
    }
}

#[test]
fn ancillary_chunks() {
    let mut seed = 0x8000u64;
    let base = |seed: u64, bd: c_int, ct: c_int| {
        let mut c = WCfg::new(12, 4, bd, ct, PNG_INTERLACE_NONE);
        c.seed = seed;
        c
    };
    // gAMA over a wide range of legal fixed-point values
    for g in [1i32, 100, 45455, 100000, 220000, 1_000_000, 2_147_483] {
        seed += 1;
        let mut c = base(seed, 8, 2);
        c.gamma = Some(g);
        diff_write(&c);
    }
    // sRGB intents
    for i in 0..4 {
        seed += 1;
        let mut c = base(seed, 8, 2);
        c.srgb = Some(i);
        diff_write(&c);
    }
    // cHRM
    seed += 1;
    let mut c = base(seed, 8, 2);
    c.chrm = true;
    diff_write(&c);
    // sBIT for every colour type / depth
    for &(bd, ct) in DEPTH_TYPE.iter() {
        let maxd = if ct == 3 { 8u8 } else { bd as u8 };
        for d in 1..=maxd {
            seed += 1;
            let mut c = base(seed, bd, ct);
            c.sbit = Some(png_color_8 {
                red: d,
                green: d,
                blue: d,
                gray: d,
                alpha: d,
            });
            diff_write(&c);
        }
    }
    // bKGD for every colour type
    for &(bd, ct) in DEPTH_TYPE.iter() {
        seed += 1;
        let mut c = base(seed, bd, ct);
        let maxv: u16 = if ct == 3 {
            0
        } else if bd == 16 {
            0xffff
        } else {
            ((1u32 << bd) - 1) as u16
        };
        c.bkgd = Some(png_color_16 {
            index: 0,
            red: maxv,
            green: maxv / 2,
            blue: maxv / 3,
            gray: maxv / 4,
        });
        diff_write(&c);
    }
    // tRNS for grey / RGB / palette
    for &(bd, ct) in &[(1i32, 0i32), (8, 0), (16, 0), (8, 2), (16, 2), (1, 3), (4, 3), (8, 3)] {
        seed += 1;
        let mut c = base(seed, bd, ct);
        c.trns = true;
        diff_write(&c);
    }
    // hIST (palette only)
    for bd in [1i32, 2, 4, 8] {
        seed += 1;
        let mut c = base(seed, bd, 3);
        c.hist = true;
        diff_write(&c);
    }
    // pHYs / oFFs with both unit types
    for u in [0i32, 1] {
        seed += 1;
        let mut c = base(seed, 8, 2);
        c.phys = Some((300, 400, u));
        diff_write(&c);
        seed += 1;
        let mut c = base(seed, 8, 2);
        c.offs = Some((-5, 7, u));
        diff_write(&c);
    }
    // tIME
    seed += 1;
    let mut c = base(seed, 8, 2);
    c.time = Some(png_time {
        year: 2024,
        month: 2,
        day: 29,
        hour: 23,
        minute: 59,
        second: 60,
    });
    diff_write(&c);
    // sCAL with all three unit types
    for u in [1i32, 2] {
        seed += 1;
        let mut c = base(seed, 8, 2);
        c.scal = Some((u, 100000, 250000));
        diff_write(&c);
    }
    // cICP / cLLI
    seed += 1;
    let mut c = base(seed, 8, 2);
    c.cicp = Some((9, 16, 0, 1));
    diff_write(&c);
    seed += 1;
    let mut c = base(seed, 8, 2);
    c.clli = Some((10_000_000, 4_000_000));
    diff_write(&c);
    // eXIf
    for n in [1usize, 2, 6, 40] {
        seed += 1;
        let mut c = base(seed, 8, 2);
        let mut rng = Rng::new(seed);
        c.exif = Some(rng.bytes(n));
        diff_write(&c);
    }
}

#[test]
fn text_chunks() {
    let mut seed = 0x9000u64;
    // tEXt / zTXt / iTXt, empty / one / many, short and long payloads
    let comps = [
        PNG_TEXT_COMPRESSION_NONE,
        PNG_TEXT_COMPRESSION_zTXt,
        PNG_ITXT_COMPRESSION_NONE,
        PNG_ITXT_COMPRESSION_zTXt,
    ];
    for &comp in &comps {
        for nlen in [0usize, 1, 10, 500, 5000] {
            seed += 1;
            let mut c = WCfg::new(8, 3, 8, 2, PNG_INTERLACE_NONE);
            c.seed = seed;
            let mut rng = Rng::new(seed);
            let text: Vec<u8> = (0..nlen).map(|_| rng.range(32, 126) as u8).collect();
            c.text = vec![(b"Title".to_vec(), text, comp)];
            diff_write(&c);
        }
    }
    // several text chunks of mixed kinds at once
    seed += 1;
    let mut c = WCfg::new(8, 3, 8, 2, PNG_INTERLACE_NONE);
    c.seed = seed;
    c.text = vec![
        (b"Title".to_vec(), b"a".to_vec(), PNG_TEXT_COMPRESSION_NONE),
        (b"Author".to_vec(), b"bb".to_vec(), PNG_TEXT_COMPRESSION_zTXt),
        (b"Comment".to_vec(), vec![b'c'; 3000], PNG_ITXT_COMPRESSION_zTXt),
        (b"Software".to_vec(), b"".to_vec(), PNG_ITXT_COMPRESSION_NONE),
    ];
    diff_write(&c);
    // text compression parameters
    for level in [-1i32, 0, 1, 9] {
        for strategy in [0i32, 1, 2, 3, 4] {
            seed += 1;
            let b = apis();
            // exercised through the dedicated setters: build the cfg then
            // apply them by hand for both libraries
            let mut cfg = WCfg::new(8, 3, 8, 2, PNG_INTERLACE_NONE);
            cfg.seed = seed;
            cfg.text = vec![(b"Comment".to_vec(), vec![b'z'; 2000], PNG_TEXT_COMPRESSION_zTXt)];
            let run = |a: &Api, is_c: bool| unsafe {
                set_cur_is_c(is_c);
                reset_all();
                let mut p = (a.png_create_write_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                let mut info = (a.png_create_info_struct)(p);
                (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
                (a.png_set_text_compression_level)(p, level);
                (a.png_set_text_compression_strategy)(p, strategy);
                (a.png_set_text_compression_mem_level)(p, 8);
                (a.png_set_text_compression_window_bits)(p, 15);
                (a.png_set_text_compression_method)(p, 8);
                (a.png_set_IHDR)(p, info, 8, 3, 8, 2, 0, 0, 0);
                let mut key = b"Comment\0".to_vec();
                let mut val = vec![b'z'; 2000];
                val.push(0);
                let t = png_text {
                    compression: PNG_TEXT_COMPRESSION_zTXt,
                    key: key.as_mut_ptr() as *mut c_char,
                    text: val.as_mut_ptr() as *mut c_char,
                    text_length: 0,
                    itxt_length: 0,
                    lang: std::ptr::null_mut(),
                    lang_key: std::ptr::null_mut(),
                };
                (a.png_set_text)(p, info, &t, 1);
                (a.png_write_info)(p, info);
                let rb = (a.png_get_rowbytes)(p, info);
                let row = vec![0x5au8; rb];
                for _ in 0..3 {
                    (a.png_write_row)(p, row.as_ptr());
                }
                (a.png_write_end)(p, info);
                (a.png_destroy_write_struct)(&mut p, &mut info);
                (out_take(), log_take())
            };
            let (co, cl) = run(&b.c, true);
            let (ro, rl) = run(&b.rs, false);
            eq_bytes(&format!("text compression level={level} strategy={strategy}"), &co, &ro);
            eq_dbg("text compression transcript", cl, rl);
        }
    }
}

#[test]
fn unknown_chunks_and_flush() {
    let mut seed = 0xa000u64;
    // unknown chunks in all three locations, empty / small / large
    for loc in [1i32 /* HAVE_IHDR */, 2 /* HAVE_PLTE */, 8 /* AFTER_IDAT */] {
        for n in [0usize, 1, 7, 300] {
            seed += 1;
            let mut c = WCfg::new(9, 3, 8, 3, PNG_INTERLACE_NONE);
            c.seed = seed;
            let mut rng = Rng::new(seed);
            c.unknown = vec![(*b"prVt\0", rng.bytes(n), loc)];
            diff_write(&c);
        }
    }
    // several unknown chunks at once, safe and unsafe to copy
    seed += 1;
    let mut c = WCfg::new(9, 3, 8, 2, PNG_INTERLACE_NONE);
    c.seed = seed;
    c.unknown = vec![
        (*b"prVt\0", vec![1, 2, 3], 1),
        (*b"orNG\0", vec![4], 1),
        (*b"blUE\0", vec![], 8),
    ];
    diff_write(&c);

    // png_set_flush / png_write_flush
    for nrows in [0i32, 1, 2, 5, 1000] {
        seed += 1;
        let mut c = WCfg::new(20, 10, 8, 2, PNG_INTERLACE_NONE);
        c.seed = seed;
        c.flush_rows = Some(nrows);
        diff_write(&c);
    }
    // explicit png_write_flush between rows
    let b = apis();
    let run = |a: &Api, is_c: bool| unsafe {
        set_cur_is_c(is_c);
        reset_all();
        let mut p = (a.png_create_write_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
        );
        let mut info = (a.png_create_info_struct)(p);
        (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
        (a.png_set_IHDR)(p, info, 16, 8, 8, 2, 0, 0, 0);
        (a.png_write_info)(p, info);
        let rb = (a.png_get_rowbytes)(p, info);
        for y in 0..8u32 {
            let row = vec![(y * 7) as u8; rb];
            (a.png_write_row)(p, row.as_ptr());
            (a.png_write_flush)(p);
        }
        (a.png_write_end)(p, info);
        (a.png_destroy_write_struct)(&mut p, &mut info);
        (out_take(), log_take())
    };
    let (co, cl) = run(&b.c, true);
    let (ro, rl) = run(&b.rs, false);
    eq_bytes("explicit png_write_flush", &co, &ro);
    eq_dbg("explicit png_write_flush transcript", cl, rl);
}

#[test]
fn mng_features_and_raw_chunk_api() {
    // png_permit_mng_features + the intrapixel filter method
    let b = apis();
    for &m in &[0u32, 1, 4, 5, 0xff] {
        let run = |a: &Api, is_c: bool| unsafe {
            set_cur_is_c(is_c);
            reset_all();
            let mut p = (a.png_create_write_struct)(
                PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                Some(error_cb),
                Some(warn_cb),
            );
            let mut info = (a.png_create_info_struct)(p);
            (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let got = (a.png_permit_mng_features)(p, m);
            log_push(format!("MNG:{got}"));
            (a.png_set_IHDR)(p, info, 8, 4, 8, 2, 0, 0,
                             if got & 4 != 0 { PNG_INTRAPIXEL_DIFFERENCING } else { 0 });
            (a.png_write_info)(p, info);
            let rb = (a.png_get_rowbytes)(p, info);
            let mut rng = Rng::new(m as u64 + 1);
            for _ in 0..4 {
                let row: Vec<u8> = (0..rb).map(|_| rng.next_u8()).collect();
                (a.png_write_row)(p, row.as_ptr());
            }
            (a.png_write_end)(p, info);
            (a.png_destroy_write_struct)(&mut p, &mut info);
            (out_take(), log_take())
        };
        let (co, cl) = run(&b.c, true);
        let (ro, rl) = run(&b.rs, false);
        eq_bytes(&format!("mng features {m}"), &co, &ro);
        eq_dbg(&format!("mng features {m} transcript"), cl, rl);
    }

    // the raw chunk-writing API: png_write_sig / png_write_chunk /
    // png_write_chunk_start + _data + _end
    let run = |a: &Api, is_c: bool| unsafe {
        set_cur_is_c(is_c);
        reset_all();
        let mut p = (a.png_create_write_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
        );
        let mut info = (a.png_create_info_struct)(p);
        (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
        (a.png_write_sig)(p);
        let ihdr = pb::ihdr_data(4, 4, 8, 2, 0, 0, 0);
        (a.png_write_chunk)(p, b"IHDR".as_ptr(), ihdr.as_ptr(), ihdr.len());
        // split-write form
        (a.png_write_chunk_start)(p, b"prVt".as_ptr(), 6);
        (a.png_write_chunk_data)(p, b"abc".as_ptr(), 3);
        (a.png_write_chunk_data)(p, b"def".as_ptr(), 3);
        (a.png_write_chunk_end)(p);
        // zero-length data
        (a.png_write_chunk)(p, b"zzZz".as_ptr(), std::ptr::null(), 0);
        (a.png_write_chunk_start)(p, b"yyYy".as_ptr(), 0);
        (a.png_write_chunk_end)(p);
        (a.png_write_chunk)(p, b"IEND".as_ptr(), std::ptr::null(), 0);
        (a.png_destroy_write_struct)(&mut p, &mut info);
        (out_take(), log_take())
    };
    let (co, cl) = run(&b.c, true);
    let (ro, rl) = run(&b.rs, false);
    eq_bytes("raw chunk API", &co, &ro);
    eq_dbg("raw chunk API transcript", cl, rl);
}

#[test]
fn randomised_write_configs() {
    // property-style: many random but legal combinations, fixed seed
    let mut rng = Rng::new(0xb000);
    for i in 0..3000 {
        let (bd, ct) = DEPTH_TYPE[rng.below(DEPTH_TYPE.len() as u32) as usize];
        let w = rng.range(1, 40);
        let h = rng.range(1, 12);
        let il = if rng.bool() { PNG_INTERLACE_ADAM7 } else { PNG_INTERLACE_NONE };
        let mut cfg = WCfg::new(w, h, bd, ct, il);
        cfg.seed = 0xb000 + i;
        if rng.bool() {
            cfg.filters = Some((rng.below(32) as c_int) << 3);
        }
        if rng.bool() {
            cfg.level = Some(rng.range(0, 9) as c_int);
        }
        if rng.bool() {
            cfg.strategy = Some(rng.below(5) as c_int);
        }
        if rng.bool() {
            cfg.mem_level = Some(rng.range(1, 9) as c_int);
        }
        if rng.bool() {
            cfg.window_bits = Some(rng.range(8, 15) as c_int);
        }
        if rng.bool() {
            cfg.buffer_size = Some(rng.range(1, 4096) as usize);
        }
        if rng.bool() {
            cfg.bgr = true;
        }
        if rng.bool() {
            cfg.swap = true;
        }
        if rng.bool() {
            cfg.swap_alpha = true;
        }
        if rng.bool() {
            cfg.invert_alpha = true;
        }
        if rng.bool() {
            cfg.invert_mono = true;
        }
        if rng.bool() {
            cfg.packswap = true;
        }
        if bd < 8 && rng.bool() {
            cfg.packing = true;
        }
        if rng.bool() {
            cfg.gamma = Some(rng.range(1, 2_000_000) as i32);
        }
        if rng.bool() {
            cfg.phys = Some((rng.next_u32() % 100000, rng.next_u32() % 100000, rng.below(2) as c_int));
        }
        if rng.bool() {
            cfg.trns = true;
        }
        if ct == 3 && rng.bool() {
            cfg.hist = true;
        }
        cfg.mode = match rng.below(4) {
            0 => WMode::Row,
            1 => WMode::Rows(rng.range(1, 5)),
            2 => WMode::Image,
            _ => WMode::Row,
        };
        diff_write(&cfg);
    }
}

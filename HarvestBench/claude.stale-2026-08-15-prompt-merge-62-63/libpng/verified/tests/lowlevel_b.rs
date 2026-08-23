//! Internal (`PNG_INTERNAL_FUNCTION`) row-level / utility entry points.
//!
//! CONFIGS.md rows **L19 .. L29**.  Every test drives both the reference C
//! `libpng.so` and the translated Rust `liblibpng.so` through an identical call
//! sequence and requires byte-identical traces.
//!
//! All signatures are taken from `c_src/include/pngpriv.h` / `png.h`.
#![allow(clippy::too_many_arguments)]
// Test names mirror the C entry points (png_check_IHDR, ...).
#![allow(non_snake_case)]

mod support;

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use support::core::*;
use support::pngbuild::{self, Builder, Chunk};
use support::*;

// ---------------------------------------------------------------------------
// `target/cbuild/libpng.so` has no DT_NEEDED entry for libm (its CMakeLists
// only links zlib), and on glibc >= 2.34 `floor`/`pow`/`frexp` still live in
// libm.so.6.  Because both libraries are opened with RTLD_LOCAL, the C library
// can only resolve those from the *global* scope, i.e. from this test
// executable.  Referencing them here puts libm.so.6 in the test binary's
// DT_NEEDED list, which is what the gamma / fixed-point tests below need.
// ---------------------------------------------------------------------------
#[link(name = "m")]
extern "C" {
    fn floor(x: f64) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    fn modf(x: f64, i: *mut f64) -> f64;
    fn frexp(x: f64, e: *mut c_int) -> f64;
}

#[test]
fn l00_libm_is_linked() {
    let mut i = 0f64;
    let mut e: c_int = 0;
    let v = unsafe { floor(1.5) + pow(2.0, 3.0) + modf(1.25, &mut i) + frexp(8.0, &mut e) };
    assert!(v > 0.0 && i == 1.0 && e == 4);
}

// ---------------------------------------------------------------------------
// constants (taken from pngpriv.h / png.h, NOT from support::core which
// carries a couple of stale pre-1.6 PNG_FREE_* values)
// ---------------------------------------------------------------------------

/// `PNG_PACKSWAP` transformation bit — pngpriv.h: `#define PNG_PACKSWAP 0x10000U`
const T_PACKSWAP: u32 = 0x0001_0000;

/// `png_pass_inc` / `png_pass_start` (png.c)
const PASS_INC: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];

// png.h PNG_FREE_*
const F_HIST: u32 = 0x0008;
const F_ICCP: u32 = 0x0010;
const F_SPLT: u32 = 0x0020;
const F_ROWS: u32 = 0x0040;
const F_PCAL: u32 = 0x0080;
const F_SCAL: u32 = 0x0100;
const F_UNKN: u32 = 0x0200;
const F_PLTE: u32 = 0x1000;
const F_TRNS: u32 = 0x2000;
const F_TEXT: u32 = 0x4000;
const F_EXIF: u32 = 0x8000;
const F_ALL: u32 = 0xffff;

const FREE_MASKS: &[(&str, u32)] = &[
    ("HIST", F_HIST),
    ("ICCP", F_ICCP),
    ("SPLT", F_SPLT),
    ("ROWS", F_ROWS),
    ("PCAL", F_PCAL),
    ("SCAL", F_SCAL),
    ("UNKN", F_UNKN),
    ("PLTE", F_PLTE),
    ("TRNS", F_TRNS),
    ("TEXT", F_TEXT),
    ("EXIF", F_EXIF),
    ("ALL", F_ALL),
    ("NONE", 0),
    ("0x0400(removed)", 0x0400),
];

/// The 15 legal (colour type, bit depth) pairs.
const COMBOS: &[(u8, u8)] = &[
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (2, 8),
    (2, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

fn chans(ct: u8) -> u8 {
    match ct {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

fn rb_pd(pd: u8, w: u32) -> usize {
    ((pd as usize) * (w as usize) + 7) >> 3
}

/// Fill a `png_row_info` exactly the way libpng does.
fn mk_ri(ct: u8, bd: u8, w: u32) -> PngRowInfo {
    let c = chans(ct);
    let pd = c * bd; // max 4*16 = 64
    PngRowInfo {
        width: w,
        rowbytes: rb_pd(pd, w),
        color_type: ct,
        bit_depth: bd,
        channels: c,
        pixel_depth: pd,
    }
}

fn ri_str(ri: &PngRowInfo) -> String {
    format!(
        "[w={} rb={} ct={} bd={} ch={} pd={}]",
        ri.width, ri.rowbytes, ri.color_type, ri.bit_depth, ri.channels, ri.pixel_depth
    )
}

/// Trace for tests that need no png_struct at all.
fn tr(f: impl FnOnce()) -> Trace {
    session_reset(Vec::new());
    f();
    Trace {
        lines: take_log(),
        out: take_out(),
        rc: 0,
    }
}

/// A palette (`3*n` bytes) with deterministic content.
fn pal_bytes(n: usize, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    (0..n * 3).map(|_| r.byte()).collect()
}

/// Pack palette indices at `bd` bits per pixel, most significant bits first.
fn pack_indices(idx: &[u32], bd: u8) -> Vec<u8> {
    let per = 8 / bd as usize;
    let mask = ((1u16 << bd) - 1) as u8;
    let mut out = vec![0u8; (idx.len() + per - 1) / per];
    for (i, &v) in idx.iter().enumerate() {
        let byte = i / per;
        let slot = i % per;
        let shift = 8 - bd as usize * (slot + 1);
        out[byte] |= ((v as u8) & mask) << shift;
    }
    out
}

/// A complete palette PNG whose indices are all in `0 ..= maxidx`.
fn palette_png(w: u32, h: u32, bd: u8, npal: usize, maxidx: u32, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    let mut raw = Vec::new();
    for _ in 0..h {
        raw.push(0u8); // filter NONE
        let idx: Vec<u32> = (0..w).map(|_| r.below(maxidx + 1)).collect();
        raw.extend_from_slice(&pack_indices(&idx, bd));
    }
    Builder::new(w, h, bd, 3)
        .add(b"PLTE", pal_bytes(npal, seed ^ 0x5555))
        .build(&raw, 0)
}

/// A complete PNG for `(ct, bd)`, adding a full PLTE when the colour type needs
/// one.
fn image_png(w: u32, h: u32, ct: u8, bd: u8, interlace: u8, seed: u64) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct).interlace(interlace);
    if ct == 3 {
        b = b.add(b"PLTE", pal_bytes(1usize << bd, seed ^ 0xa5a5));
    }
    b.build_valid(seed)
}

// ===========================================================================
// L19 — png_do_bgr / swap / invert / packswap / strip_channel /
//        check_palette_indexes
// ===========================================================================

type RowFn2 = unsafe extern "C" fn(*mut PngRowInfo, *mut u8);

/// One pure row transform over every legal colour/depth pair × widths 1..17.
fn l19_pure(sym: &'static str, seed: u64) {
    // Pre-computed so that both libraries see byte-identical input rows.
    let mut rng = Rng::new(seed);
    let mut cases: Vec<(u8, u8, u32, Vec<u8>)> = Vec::new();
    for &(ct, bd) in COMBOS {
        for w in 1..=17u32 {
            let n = rb_pd(chans(ct) * bd, w) + 8;
            cases.push((ct, bd, w, rng.bytes(n)));
        }
    }
    diff(&format!("L19 {sym}"), |lib| {
        let f: RowFn2 = lib.f(sym);
        tr(|| {
            for (ct, bd, w, src) in &cases {
                let mut buf = src.clone();
                let mut ri = mk_ri(*ct, *bd, *w);
                unsafe { f(&mut ri, buf.as_mut_ptr()) };
                log(format!(
                    "{sym} ct={ct} bd={bd} w={w} in={} out={} ri={}",
                    hex(src),
                    hex(&buf),
                    ri_str(&ri)
                ));
            }
        })
    });
}

#[test]
fn l19_do_bgr() {
    l19_pure("png_do_bgr", 0x1901);
}

#[test]
fn l19_do_swap() {
    l19_pure("png_do_swap", 0x1902);
}

#[test]
fn l19_do_invert() {
    l19_pure("png_do_invert", 0x1903);
}

#[test]
fn l19_do_packswap() {
    l19_pure("png_do_packswap", 0x1904);
}

#[test]
fn l19_do_strip_channel() {
    // The legal combos plus a few deliberately "impossible" row_infos that hit
    // the `bad bit depth` early returns in png_do_strip_channel.
    let extra: [(u8, u8); 4] = [(4, 4), (4, 1), (6, 2), (6, 4)];
    let all: Vec<(u8, u8)> = COMBOS.iter().copied().chain(extra).collect();

    let mut rng = Rng::new(0x1905);
    let mut cases: Vec<(u8, u8, c_int, u32, Vec<u8>)> = Vec::new();
    for &(ct, bd) in &all {
        for at_start in [0 as c_int, 1] {
            for w in 1..=17u32 {
                let n = rb_pd(chans(ct) * bd, w) + 8;
                cases.push((ct, bd, at_start, w, rng.bytes(n)));
            }
        }
    }

    diff("L19 png_do_strip_channel", |lib| {
        let f: unsafe extern "C" fn(*mut PngRowInfo, *mut u8, c_int) = lib.f("png_do_strip_channel");
        tr(|| {
            for (ct, bd, at_start, w, src) in &cases {
                let mut buf = src.clone();
                let mut ri = mk_ri(*ct, *bd, *w);
                unsafe { f(&mut ri, buf.as_mut_ptr(), *at_start) };
                log(format!(
                    "strip ct={ct} bd={bd} at_start={at_start} w={w} in={} out={} ri={}",
                    hex(src),
                    hex(&buf),
                    ri_str(&ri)
                ));
            }
        })
    });
}

/// `png_do_check_palette_indexes` reads `png_ptr->num_palette` /
/// `num_palette_max`, so it is driven through a real read of palette images
/// with in-range and out-of-range indices.  The observable is the benign error
/// text from `png_read_end` plus `png_get_palette_max`.
#[test]
fn l19_do_check_palette_indexes() {
    // (bit depth, num_palette, max index actually used)
    let shapes: &[(u8, usize, u32)] = &[
        (1, 2, 1),   // num_palette == 1<<bd -> check skipped
        (1, 1, 0),   // in range
        (1, 1, 1),   // out of range
        (2, 4, 3),   // check skipped
        (2, 2, 1),   // in range
        (2, 2, 3),   // out of range
        (4, 16, 15), // check skipped
        (4, 5, 4),   // in range
        (4, 5, 15),  // out of range
        (8, 256, 255),
        (8, 100, 99),
        (8, 100, 255),
    ];
    let widths: [u32; 8] = [1, 2, 3, 7, 8, 9, 16, 17];

    for &(bd, npal, maxidx) in shapes {
        for &w in &widths {
            for allowed in [1 as c_int, 0] {
                for benign in [1 as c_int, 0] {
                    let png = palette_png(w, 3, bd, npal, maxidx, 0x1906 ^ (w as u64) << 8);
                    let label = format!(
                        "L19 palette-index bd={bd} npal={npal} max={maxidx} w={w} allowed={allowed} benign={benign}"
                    );
                    diff(&label, |lib| {
                        let mut row = vec![0u8; 512];
                        with_read(lib, &png, &mut |c, png, info| unsafe {
                            (c.set_benign_errors)(png, benign);
                            (c.set_check_for_invalid_index)(png, allowed);
                            (c.read_info)(png, info);
                            let rb = (c.get_rowbytes)(png, info);
                            log(format!(
                                "rowbytes={rb} palette_max={}",
                                (c.get_palette_max)(png, info)
                            ));
                            for r in 0..3 {
                                (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                                log(format!(
                                    "row{r}={} palette_max={}",
                                    hex(&row[..rb]),
                                    (c.get_palette_max)(png, info)
                                ));
                            }
                            (c.read_end)(png, ptr::null_mut());
                            log(format!("after_end palette_max={}", (c.get_palette_max)(png, info)));
                        })
                    });
                }
            }
        }
    }
}

// ===========================================================================
// L20 — png_read_filter_row
// ===========================================================================

type FilterFn = unsafe extern "C" fn(Png, *mut PngRowInfo, *mut u8, *const u8, c_int);

struct FCase {
    pd: u8,
    filter: c_int,
    w: u32,
    row: Vec<u8>,
    prev: Vec<u8>,
}

fn filter_cases(depths: &[u8], seed: u64) -> Vec<FCase> {
    let mut rng = Rng::new(seed);
    let mut v = Vec::new();
    for &pd in depths {
        for filter in 0..=4i32 {
            for w in 1..=13u32 {
                let n = rb_pd(pd, w) + 8;
                v.push(FCase {
                    pd,
                    filter,
                    w,
                    row: rng.bytes(n),
                    prev: rng.bytes(n),
                });
            }
        }
    }
    v
}

/// Plausible (colour type, bit depth) for a given transformed pixel depth.
fn pd_image(pd: u8) -> (u8, u8) {
    match pd {
        1 => (0, 1),
        2 => (0, 2),
        4 => (0, 4),
        8 => (0, 8),
        16 => (0, 16),
        24 => (2, 8),
        32 => (6, 8),
        48 => (2, 16),
        _ => (6, 16),
    }
}

const PDS: [u8; 9] = [1, 2, 4, 8, 16, 24, 32, 48, 64];

#[test]
fn l20_read_filter_row_fresh_struct() {
    // A fresh read struct has pixel_depth == 0, so png_init_filter_functions
    // installs the multi-byte paeth implementation; this is exactly what the C
    // does when png_read_filter_row is called directly.
    let cases = filter_cases(&PDS, 0x2001);
    diff("L20 png_read_filter_row (fresh struct)", |lib| {
        let f: FilterFn = lib.f("png_read_filter_row");
        with_read(lib, &[], &mut |_c, png, _info| unsafe {
            for k in &cases {
                let mut row = k.row.clone();
                let prev = k.prev.clone();
                let mut ri = PngRowInfo {
                    width: k.w,
                    rowbytes: rb_pd(k.pd, k.w),
                    color_type: 0,
                    bit_depth: if k.pd < 8 { k.pd } else { 8 },
                    channels: if k.pd < 8 { 1 } else { k.pd / 8 },
                    pixel_depth: k.pd,
                };
                f(png, &mut ri, row.as_mut_ptr(), prev.as_ptr(), k.filter);
                log(format!(
                    "pd={} filter={} w={} in={} prev={} out={} ri={}",
                    k.pd,
                    k.filter,
                    k.w,
                    hex(&k.row),
                    hex(&k.prev),
                    hex(&row),
                    ri_str(&ri)
                ));
            }
        })
    });
}

#[test]
fn l20_read_filter_row_after_read_info() {
    // png_read_info sets png_ptr->pixel_depth, so png_init_filter_functions
    // selects the 1-byte-pixel paeth implementation for pixel_depth <= 8.
    for &pd in &PDS {
        let (ct, bd) = pd_image(pd);
        let png = image_png(4, 1, ct, bd, 0, 0x2002 ^ pd as u64);
        let cases = filter_cases(&[pd], 0x2003 ^ (pd as u64) << 16);
        diff(&format!("L20 png_read_filter_row (pixel_depth={pd})"), |lib| {
            let f: FilterFn = lib.f("png_read_filter_row");
            with_read(lib, &png, &mut |c, png, info| unsafe {
                (c.read_info)(png, info);
                log(format!(
                    "info depth={} channels={} rowbytes={}",
                    (c.get_bit_depth)(png, info),
                    (c.get_channels)(png, info),
                    (c.get_rowbytes)(png, info)
                ));
                for k in &cases {
                    let mut row = k.row.clone();
                    let prev = k.prev.clone();
                    let mut ri = PngRowInfo {
                        width: k.w,
                        rowbytes: rb_pd(k.pd, k.w),
                        color_type: ct,
                        bit_depth: bd,
                        channels: chans(ct),
                        pixel_depth: k.pd,
                    };
                    f(png, &mut ri, row.as_mut_ptr(), prev.as_ptr(), k.filter);
                    log(format!(
                        "pd={} filter={} w={} in={} prev={} out={} ri={}",
                        k.pd,
                        k.filter,
                        k.w,
                        hex(&k.row),
                        hex(&k.prev),
                        hex(&row),
                        ri_str(&ri)
                    ));
                }
            })
        });
    }
}

// ===========================================================================
// L21 — png_do_read_interlace / png_do_write_interlace
// ===========================================================================

#[test]
fn l21_do_read_interlace() {
    let mut rng = Rng::new(0x2101);
    // (ct, bd, pass, w, transformations, buffer)
    let mut cases: Vec<(u8, u8, c_int, u32, u32, Vec<u8>)> = Vec::new();
    for pass in 0..7usize {
        for &(ct, bd) in COMBOS {
            for w in 1..=7u32 {
                let pd = chans(ct) * bd;
                let final_w = w * PASS_INC[pass];
                let n = rb_pd(pd, final_w) + 8;
                for tf in [0u32, T_PACKSWAP] {
                    cases.push((ct, bd, pass as c_int, w, tf, rng.bytes(n)));
                }
            }
        }
    }
    diff("L21 png_do_read_interlace", |lib| {
        let f: unsafe extern "C" fn(*mut PngRowInfo, *mut u8, c_int, u32) =
            lib.f("png_do_read_interlace");
        tr(|| {
            for (ct, bd, pass, w, tf, src) in &cases {
                let mut buf = src.clone();
                let mut ri = mk_ri(*ct, *bd, *w);
                unsafe { f(&mut ri, buf.as_mut_ptr(), *pass, *tf) };
                log(format!(
                    "ct={ct} bd={bd} pass={pass} w={w} tf={tf:#x} in={} out={} ri={}",
                    hex(src),
                    hex(&buf),
                    ri_str(&ri)
                ));
            }
        })
    });
}

#[test]
fn l21_do_write_interlace() {
    let mut rng = Rng::new(0x2102);
    let mut cases: Vec<(u8, u8, c_int, u32, Vec<u8>)> = Vec::new();
    for pass in 0..7usize {
        for &(ct, bd) in COMBOS {
            for w in 1..=13u32 {
                let n = rb_pd(chans(ct) * bd, w) + 8;
                cases.push((ct, bd, pass as c_int, w, rng.bytes(n)));
            }
        }
    }
    diff("L21 png_do_write_interlace", |lib| {
        let f: unsafe extern "C" fn(*mut PngRowInfo, *mut u8, c_int) =
            lib.f("png_do_write_interlace");
        tr(|| {
            for (ct, bd, pass, w, src) in &cases {
                let mut buf = src.clone();
                let mut ri = mk_ri(*ct, *bd, *w);
                unsafe { f(&mut ri, buf.as_mut_ptr(), *pass) };
                log(format!(
                    "ct={ct} bd={bd} pass={pass} w={w} in={} out={} ri={}",
                    hex(src),
                    hex(&buf),
                    ri_str(&ri)
                ));
            }
        })
    });
}

// ===========================================================================
// L22 — png_combine_row (sequential and progressive)
// ===========================================================================

#[test]
fn l22_combine_row_sequential() {
    for &(ct, bd) in COMBOS {
        for w in 1..=17u32 {
            for interlace in [1u8, 0] {
                let png = image_png(w, 3, ct, bd, interlace, 0x2201 ^ (w as u64) << 8);
                let label = format!("L22 combine ct={ct} bd={bd} w={w} il={interlace}");
                diff(&label, |lib| {
                    let mut row = vec![0xA5u8; 1024];
                    let mut disp = vec![0x5Au8; 1024];
                    with_read(lib, &png, &mut |c, png, info| unsafe {
                        (c.read_info)(png, info);
                        let passes = (c.set_interlace_handling)(png);
                        let rb = (c.get_rowbytes)(png, info);
                        log(format!("passes={passes} rowbytes={rb}"));
                        for p in 0..passes {
                            for r in 0..3 {
                                (c.read_row)(png, row.as_mut_ptr(), disp.as_mut_ptr());
                                log(format!(
                                    "p{p}r{r} row={} disp={}",
                                    hex(&row[..rb + 2]),
                                    hex(&disp[..rb + 2])
                                ));
                            }
                        }
                        (c.read_end)(png, ptr::null_mut());
                    })
                });
            }
        }
    }
}

// --- progressive -----------------------------------------------------------

#[derive(Clone, Copy)]
struct ProgCtx {
    base: *mut u8,
    stride: usize,
    rowbytes: usize,
    combine: Option<unsafe extern "C" fn(Png, *mut u8, *const u8)>,
    interlace: Option<unsafe extern "C" fn(Png) -> c_int>,
    prog_ptr: Option<unsafe extern "C" fn(Png) -> *mut c_void>,
    update_info: Option<unsafe extern "C" fn(Png, Info)>,
    get_rowbytes: Option<unsafe extern "C" fn(Png, Info) -> usize>,
}

impl ProgCtx {
    const EMPTY: ProgCtx = ProgCtx {
        base: ptr::null_mut(),
        stride: 0,
        rowbytes: 0,
        combine: None,
        interlace: None,
        prog_ptr: None,
        update_info: None,
        get_rowbytes: None,
    };
}

thread_local! {
    static PROG: Cell<ProgCtx> = const { Cell::new(ProgCtx::EMPTY) };
}

unsafe extern "C" fn prog_info(png: Png, info: Info) {
    let ctx = PROG.get();
    let passes = (ctx.interlace.unwrap())(png);
    let p = (ctx.prog_ptr.unwrap())(png);
    // The progressive reader requires the info callback to call
    // png_read_update_info() (or png_start_read_image()) so that row_buf and
    // the pass geometry exist before the first row arrives.
    (ctx.update_info.unwrap())(png, info);
    log(format!(
        "prog info passes={passes} prog_ptr_null={} rowbytes={}",
        u8::from(p.is_null()),
        (ctx.get_rowbytes.unwrap())(png, info)
    ));
}

unsafe extern "C" fn prog_row(png: Png, new_row: *mut u8, row_num: u32, pass: c_int) {
    let ctx = PROG.get();
    let dst = ctx.base.add(row_num as usize * ctx.stride);
    (ctx.combine.unwrap())(png, dst, new_row);
    log(format!(
        "prog row={row_num} pass={pass} new_null={} dst={}",
        u8::from(new_row.is_null()),
        hex(std::slice::from_raw_parts(dst, ctx.rowbytes))
    ));
}

unsafe extern "C" fn prog_end(_png: Png, _info: Info) {
    log("prog end".to_string());
}

#[test]
fn l22_progressive_combine_row() {
    let widths: [u32; 6] = [1, 2, 3, 7, 8, 17];
    for &(ct, bd) in COMBOS {
        for &w in &widths {
            for interlace in [1u8, 0] {
                let h = 3u32;
                let png = image_png(w, h, ct, bd, interlace, 0x2202 ^ (w as u64) << 8);
                let stride = rb_pd(chans(ct) * bd, w) + 8;
                let label = format!("L22 progressive ct={ct} bd={bd} w={w} il={interlace}");
                diff(&label, |lib| {
                    let mut base = vec![0xC3u8; stride * h as usize];
                    let mut input = png.clone();
                    PROG.set(ProgCtx {
                        base: base.as_mut_ptr(),
                        stride,
                        rowbytes: rb_pd(chans(ct) * bd, w),
                        combine: Some(lib.f("png_progressive_combine_row")),
                        interlace: Some(lib.f("png_set_interlace_handling")),
                        prog_ptr: Some(lib.f("png_get_progressive_ptr")),
                        update_info: Some(lib.f("png_read_update_info")),
                        get_rowbytes: Some(lib.f("png_get_rowbytes")),
                    });
                    let t = with_read(lib, &[], &mut |c, png, info| unsafe {
                        (c.set_progressive_read_fn)(
                            png,
                            0x1234 as *mut c_void,
                            prog_info as Cb,
                            prog_row as Cb,
                            prog_end as Cb,
                        );
                        (c.process_data)(png, info, input.as_mut_ptr(), input.len());
                    });
                    for (i, chunk) in base.chunks(stride).enumerate() {
                        log(format!("final row{i}={}", hex(chunk)));
                    }
                    PROG.set(ProgCtx::EMPTY);
                    Trace {
                        lines: t.lines.into_iter().chain(take_log()).collect(),
                        out: t.out,
                        rc: t.rc,
                    }
                });
            }
        }
    }
}

// ===========================================================================
// L23 — version / copyright queries
// ===========================================================================

#[test]
fn l23_version_queries() {
    diff("L23 version queries", |lib| {
        let ver: unsafe extern "C" fn() -> u32 = lib.f("png_access_version_number");
        let copyright: unsafe extern "C" fn(Png) -> *const c_char = lib.f("png_get_copyright");
        let header_ver: unsafe extern "C" fn(Png) -> *const c_char = lib.f("png_get_header_ver");
        let libpng_ver: unsafe extern "C" fn(Png) -> *const c_char = lib.f("png_get_libpng_ver");
        let header_version: unsafe extern "C" fn(Png) -> *const c_char =
            lib.f("png_get_header_version");
        session_reset(Vec::new());
        let rc = protected(|| unsafe {
            log(format!("access_version_number={}", ver()));
            // NULL png_ptr
            log(format!("copyright(NULL)={}", cstr(copyright(ptr::null_mut()))));
            log(format!("header_ver(NULL)={}", cstr(header_ver(ptr::null_mut()))));
            log(format!("libpng_ver(NULL)={}", cstr(libpng_ver(ptr::null_mut()))));
            log(format!(
                "header_version(NULL)={}",
                cstr(header_version(ptr::null_mut()))
            ));
        });
        let mut t = Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        };
        // ... and with real read / write structs.
        let a = with_read(lib, &[], &mut |_c, png, _i| unsafe {
            log(format!("read copyright={}", cstr(copyright(png))));
            log(format!("read header_ver={}", cstr(header_ver(png))));
            log(format!("read libpng_ver={}", cstr(libpng_ver(png))));
            log(format!("read header_version={}", cstr(header_version(png))));
        });
        let b = with_write(lib, &mut |_c, png, _i| unsafe {
            log(format!("write copyright={}", cstr(copyright(png))));
            log(format!("write header_ver={}", cstr(header_ver(png))));
            log(format!("write libpng_ver={}", cstr(libpng_ver(png))));
            log(format!("write header_version={}", cstr(header_version(png))));
        });
        t.lines.extend(a.lines);
        t.lines.extend(b.lines);
        t.rc += a.rc + b.rc;
        t
    });
}

// ===========================================================================
// L24 — png_permit_mng_features / png_set_option
// ===========================================================================

#[test]
fn l24_permit_mng_features() {
    let masks: Vec<u32> = (0u32..=0x07).chain([0xffff_ffff, 0x04, 0x05]).collect();
    diff("L24 png_permit_mng_features", |lib| {
        let mut t = with_read(lib, &[], &mut |c, png, _i| unsafe {
            for &m in &masks {
                log(format!(
                    "read permit_mng({m:#x})={:#x}",
                    (c.permit_mng_features)(png, m)
                ));
            }
            // NULL png_ptr
            log(format!(
                "permit_mng(NULL,0xffffffff)={:#x}",
                (c.permit_mng_features)(ptr::null_mut(), 0xffff_ffff)
            ));
        });
        let w = with_write(lib, &mut |c, png, _i| unsafe {
            for &m in &masks {
                log(format!(
                    "write permit_mng({m:#x})={:#x}",
                    (c.permit_mng_features)(png, m)
                ));
            }
        });
        t.lines.extend(w.lines);
        t.rc += w.rc;
        t
    });
}

#[test]
fn l24_set_option() {
    let options: Vec<c_int> = (-1..=13).chain([99, 15, 16]).collect();
    let onoffs: [c_int; 4] = [0, 1, 2, 3]; // OFF/ON/UNSET semantics + invalid 3
    diff("L24 png_set_option", |lib| {
        let mut t = with_read(lib, &[], &mut |c, png, _i| unsafe {
            for &o in &options {
                for &v in &onoffs {
                    log(format!(
                        "read set_option({o},{v})={}",
                        (c.set_option)(png, o, v)
                    ));
                }
            }
            log(format!(
                "set_option(NULL,2,1)={}",
                (c.set_option)(ptr::null_mut(), 2, 1)
            ));
        });
        let w = with_write(lib, &mut |c, png, _i| unsafe {
            for &o in &options {
                for &v in &onoffs {
                    log(format!(
                        "write set_option({o},{v})={}",
                        (c.set_option)(png, o, v)
                    ));
                }
            }
        });
        t.lines.extend(w.lines);
        t.rc += w.rc;
        t
    });
}

// ===========================================================================
// L25 — png_icc_check_header / _length / _tag_table
// ===========================================================================

const D50: [u8; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];

fn be32(b: &mut [u8], off: usize, v: u32) {
    b[off..off + 4].copy_from_slice(&v.to_be_bytes());
}

#[derive(Clone)]
struct Icc {
    ver: u32,
    class: u32,
    space: u32,
    pcs: u32,
    sig: u32,
    intent: u32,
    illum: [u8; 12],
    tags: Vec<(u32, u32, u32)>,
    count_override: Option<u32>,
    len_override: Option<u32>,
    pad: usize,
}

impl Icc {
    /// A minimal *valid* profile: 128-byte header, 4-byte tag count, one
    /// 12-byte tag table entry, 12 bytes of tag data.  Total 156 bytes.
    fn valid() -> Icc {
        Icc {
            ver: 0x0210_0000,
            class: 0x6d6e_7472, // 'mntr'
            space: 0x5247_4220, // 'RGB '
            pcs: 0x5859_5a20,   // 'XYZ '
            sig: 0x6163_7370,   // 'acsp'
            intent: 0,
            illum: D50,
            tags: vec![(0x6465_7363, 144, 8)], // 'desc'
            count_override: None,
            len_override: None,
            pad: 12,
        }
    }
    fn build(&self) -> Vec<u8> {
        let mut p = vec![0u8; 132 + 12 * self.tags.len() + self.pad];
        be32(&mut p, 8, self.ver);
        be32(&mut p, 12, self.class);
        be32(&mut p, 16, self.space);
        be32(&mut p, 20, self.pcs);
        be32(&mut p, 36, self.sig);
        be32(&mut p, 64, self.intent);
        p[68..80].copy_from_slice(&self.illum);
        be32(&mut p, 128, self.count_override.unwrap_or(self.tags.len() as u32));
        for (i, &(id, st, ln)) in self.tags.iter().enumerate() {
            let o = 132 + 12 * i;
            be32(&mut p, o, id);
            be32(&mut p, o + 4, st);
            be32(&mut p, o + 8, ln);
        }
        let n = p.len() as u32;
        be32(&mut p, 0, self.len_override.unwrap_or(n));
        p
    }
}

/// (label, profile bytes, colour type)
fn icc_cases() -> Vec<(String, Vec<u8>, c_int)> {
    let mut v: Vec<(String, Vec<u8>, c_int)> = Vec::new();
    fn add(v: &mut Vec<(String, Vec<u8>, c_int)>, name: &str, i: &Icc, ct: c_int) {
        v.push((name.to_string(), i.build(), ct));
    }
    macro_rules! push {
        ($name:expr, $i:expr, $ct:expr) => {
            add(&mut v, $name, $i, $ct)
        };
    }

    // -- valid ------------------------------------------------------------
    let base = Icc::valid();
    push!("valid RGB on ct=2", &base, 2);
    push!("valid RGB on ct=3", &base, 3);
    push!("valid RGB on ct=6", &base, 6);
    let mut gray = base.clone();
    gray.space = 0x4752_4159; // 'GRAY'
    push!("valid GRAY on ct=0", &gray, 0);
    push!("valid GRAY on ct=4", &gray, 4);
    // colour space / colour type mismatches
    push!("RGB on ct=0", &base, 0);
    push!("RGB on ct=4", &base, 4);
    push!("GRAY on ct=2", &gray, 2);
    push!("GRAY on ct=6", &gray, 6);
    let mut cmyk = base.clone();
    cmyk.space = 0x434d_594b; // 'CMYK'
    push!("space CMYK", &cmyk, 2);
    let mut spacebad = base.clone();
    spacebad.space = 0x0001_0203; // not an ICC signature -> hex in the message
    push!("space non-signature", &spacebad, 2);

    // -- header length ----------------------------------------------------
    let mut m = base.clone();
    m.len_override = Some(999);
    push!("length field mismatch", &m, 2);
    let mut m = base.clone();
    m.len_override = Some(0);
    push!("length field zero", &m, 2);
    let mut m = base.clone();
    m.ver = 0x0400_0000; // major version 4
    m.pad = 13; // total 157, not a multiple of 4
    push!("version 4 unaligned length", &m, 2);
    let mut m = base.clone();
    m.ver = 0x0400_0000;
    push!("version 4 aligned length", &m, 2);

    // -- tag count --------------------------------------------------------
    let mut m = base.clone();
    m.count_override = Some(0xffff_ffff);
    push!("tag count 0xffffffff", &m, 2);
    let mut m = base.clone();
    m.count_override = Some(357_913_931);
    push!("tag count 357913931", &m, 2);
    let mut m = base.clone();
    m.count_override = Some(100);
    push!("tag count 100 (truncated table)", &m, 2);
    let mut m = base.clone();
    m.tags = Vec::new();
    m.pad = 12;
    push!("tag count 0", &m, 2);
    let mut m = base.clone();
    m.tags = vec![
        (0x6465_7363, 168, 8),
        (0x7758_595a, 176, 8),
        (0x6258_595a, 176, 8), // overlapping with the previous tag: legal
    ];
    m.pad = 20;
    push!("three tags, two overlapping", &m, 2);

    // -- rendering intent -------------------------------------------------
    for i in [1u32, 2, 3] {
        let mut m = base.clone();
        m.intent = i;
        v.push((format!("intent {i}"), m.build(), 2));
    }
    for i in [4u32, 0xfffe] {
        let mut m = base.clone();
        m.intent = i;
        v.push((format!("intent {i} (outside range)"), m.build(), 2));
    }
    for i in [0xffffu32, 0x1_0000, 0xffff_ffff] {
        let mut m = base.clone();
        m.intent = i;
        v.push((format!("intent {i:#x} (invalid)"), m.build(), 2));
    }

    // -- signature --------------------------------------------------------
    let mut m = base.clone();
    m.sig = 0x6163_7371; // 'acsq'
    push!("signature acsq", &m, 2);
    let mut m = base.clone();
    m.sig = 0;
    push!("signature zero", &m, 2);

    // -- PCS illuminant ---------------------------------------------------
    let mut m = base.clone();
    m.illum = [0xff; 12];
    push!("PCS illuminant not D50", &m, 2);

    // -- profile class ----------------------------------------------------
    for (nm, cl) in [
        ("scnr", 0x7363_6e72u32),
        ("prtr", 0x7072_7472),
        ("spac", 0x7370_6163),
        ("abst", 0x6162_7374),
        ("link", 0x6c69_6e6b),
        ("nmcl", 0x6e6d_636c),
        ("zzzz", 0x7a7a_7a7a),
    ] {
        let mut m = base.clone();
        m.class = cl;
        v.push((format!("class {nm}"), m.build(), 2));
    }
    let mut m = base.clone();
    m.class = 0x0001_0203;
    push!("class non-signature", &m, 2);

    // -- PCS encoding -----------------------------------------------------
    let mut m = base.clone();
    m.pcs = 0x4c61_6220; // 'Lab '
    push!("pcs Lab", &m, 2);
    let mut m = base.clone();
    m.pcs = 0x7879_7a7a; // 'xyzz'
    push!("pcs xyzz", &m, 2);

    // -- tag table --------------------------------------------------------
    let mut m = base.clone();
    m.tags = vec![(0x6465_7363, 1000, 0)];
    push!("tag start beyond profile", &m, 2);
    let mut m = base.clone();
    m.tags = vec![(0x6465_7363, 144, 1000)];
    push!("tag length beyond profile", &m, 2);
    let mut m = base.clone();
    m.tags = vec![(0x6465_7363, 145, 8)];
    push!("tag start misaligned", &m, 2);
    let mut m = base.clone();
    m.tags = vec![(0x6465_7363, 156, 0)];
    push!("zero length tag at end", &m, 2);
    let mut m = base.clone();
    m.tags = vec![(0x0001_0203, 144, 8)];
    push!("tag id non-signature, misaligned", &m, 2);
    let mut m = base.clone();
    m.tags = vec![(0x0001_0203, 145, 8)];
    push!("tag id non-signature start misaligned", &m, 2);

    v
}

const ICC_NAME: &[u8] = b"ICC Profile\0";
// 100 characters: png_icc_profile_error truncates the name to 79.
const ICC_LONG: &[u8] =
    b"0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\0";

unsafe fn icc_run(lib: &Lib, png: Png, cases: &[(String, Vec<u8>, c_int)], name: *const c_char) {
    let cl: unsafe extern "C" fn(Png, *const c_char, u32) -> c_int = lib.f("png_icc_check_length");
    let ch: unsafe extern "C" fn(Png, *const c_char, u32, *const u8, c_int) -> c_int =
        lib.f("png_icc_check_header");
    let ct_: unsafe extern "C" fn(Png, *const c_char, u32, *const u8) -> c_int =
        lib.f("png_icc_check_tag_table");

    for len in [0u32, 1, 131, 132, 144, 156, 8_000_000, 8_000_001, 0xffff_ffff] {
        log(format!("check_length({len})={}", cl(png, name, len)));
    }

    for (label, prof, color_type) in cases {
        let n = prof.len() as u32;
        log(format!("--- {label} (len={n} ct={color_type})"));
        let rl = cl(png, name, n);
        log(format!("check_length={rl}"));
        let rh = ch(png, name, n, prof.as_ptr(), *color_type);
        log(format!("check_header={rh}"));
        if rh != 0 {
            // Only safe once the header check passed: it guarantees that the
            // tag table lies inside the profile.
            log(format!("check_tag_table={}", ct_(png, name, n, prof.as_ptr())));
        }
    }
}

#[test]
fn l25_icc_checks_benign() {
    let cases = icc_cases();
    for (nm, name) in [("short", ICC_NAME), ("long", ICC_LONG)] {
        diff(&format!("L25 icc benign=1 name={nm}"), |lib| {
            with_read(lib, &[], &mut |c, png, _i| unsafe {
                (c.set_benign_errors)(png, 1);
                icc_run(lib, png, &cases, name.as_ptr() as *const c_char);
            })
        });
    }
}

#[test]
fn l25_icc_checks_hard_errors() {
    // With benign errors switched off every failing check is a png_error, so
    // each case needs its own run to keep later cases from being hidden.
    let cases = icc_cases();
    for (i, case) in cases.iter().enumerate() {
        let one = vec![case.clone()];
        diff(&format!("L25 icc benign=0 #{i} {}", case.0), |lib| {
            with_read(lib, &[], &mut |c, png, _i| unsafe {
                (c.set_benign_errors)(png, 0);
                icc_run(lib, png, &one, ICC_NAME.as_ptr() as *const c_char);
            })
        });
    }
}

// ===========================================================================
// L26 — png_build_gamma_table / png_destroy_gamma_table
// ===========================================================================

fn gama_png(w: u32, h: u32, ct: u8, bd: u8, gamma: u32, seed: u64) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct).add(b"gAMA", gamma.to_be_bytes().to_vec());
    if ct == 3 {
        b = b.add(b"PLTE", pal_bytes(1usize << bd, seed ^ 0x33));
    }
    b.build_valid(seed)
}

#[test]
fn l26_gamma_via_set_gamma() {
    // (screen, file) as fixed-point values; 100000 == 1.0
    let fixed_pairs: &[(i32, i32)] = &[
        (100_000, 100_000),
        (220_000, 45_455),
        (45_455, 220_000),
        (100_000, 45_455),
        (220_000, 100_000),
        (1, 100_000),
        (100_000, 1),
        (2_147_483_647, 100_000),
        (100_000, 2_147_483_647),
        (0, 100_000),
        (100_000, 0),
        (-1, -1),
    ];
    let float_pairs: &[(f64, f64)] = &[
        (1.0, 1.0),
        (2.2, 0.45455),
        (0.45455, 2.2),
        (1.0, 0.45455),
        (2.2, 1.0),
    ];

    for &(ct, bd) in COMBOS {
        for &gama in &[45455u32, 100_000, 220_000] {
            let png = gama_png(6, 3, ct, bd, gama, 0x2601 ^ gama as u64);
            for &(s, f) in fixed_pairs {
                let label = format!("L26 fixed ct={ct} bd={bd} gAMA={gama} screen={s} file={f}");
                diff(&label, |lib| {
                    let mut row = vec![0u8; 512];
                    with_read(lib, &png, &mut |c, png, info| unsafe {
                        (c.set_benign_errors)(png, 1);
                        (c.read_info)(png, info);
                        (c.set_gamma_fixed)(png, s, f);
                        (c.read_update_info)(png, info);
                        let rb = (c.get_rowbytes)(png, info);
                        log(format!("rowbytes={rb}"));
                        for r in 0..3 {
                            (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                            log(format!("row{r}={}", hex(&row[..rb])));
                        }
                        (c.read_end)(png, ptr::null_mut());
                    })
                });
            }
            for &(s, f) in float_pairs {
                let label = format!("L26 float ct={ct} bd={bd} gAMA={gama} screen={s} file={f}");
                diff(&label, |lib| {
                    let mut row = vec![0u8; 512];
                    with_read(lib, &png, &mut |c, png, info| unsafe {
                        (c.set_benign_errors)(png, 1);
                        (c.read_info)(png, info);
                        (c.set_gamma)(png, s, f);
                        (c.read_update_info)(png, info);
                        let rb = (c.get_rowbytes)(png, info);
                        log(format!("rowbytes={rb}"));
                        for r in 0..3 {
                            (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                            log(format!("row{r}={}", hex(&row[..rb])));
                        }
                        (c.read_end)(png, ptr::null_mut());
                    })
                });
            }
        }
    }
}

#[test]
fn l26_build_gamma_table_direct() {
    // 0 = baseline, 1 = build(8), 2 = build(16), 3 = build(8) twice,
    // 4 = build(8)+destroy, 5 = build(16)+destroy, 6 = build(8)+build(16),
    // 7 = build(bit_depth) *after* png_read_update_info, i.e. after libpng has
    //     already built the table itself: the rebuilt table must produce
    //     byte-identical rows, which compares the table *contents*, not just
    //     the "gamma table being rebuilt" warning.
    for &(ct, bd) in &[(0u8, 8u8), (0, 16), (2, 8), (2, 16), (3, 8), (4, 8), (6, 16)] {
        let png = gama_png(6, 3, ct, bd, 45455, 0x2602);
        for variant in 0..8 {
            // The first two pairs are gamma-*significant* (so libpng builds the
            // tables itself and the direct call is observable through the
            // "gamma table being rebuilt" warning and the transformed rows);
            // the last two are not (2.2 * 0.45455 == 1.0).
            for &(s, f) in &[
                (220_000i32, 100_000i32),
                (100_000, 45_455),
                (220_000, 45_455),
                (100_000, 100_000),
            ] {
                let label =
                    format!("L26 direct ct={ct} bd={bd} variant={variant} screen={s} file={f}");
                diff(&label, |lib| {
                    let build: unsafe extern "C" fn(Png, c_int) = lib.f("png_build_gamma_table");
                    let destroy: unsafe extern "C" fn(Png) = lib.f("png_destroy_gamma_table");
                    let mut row = vec![0u8; 512];
                    with_read(lib, &png, &mut |c, png, info| unsafe {
                        (c.set_benign_errors)(png, 1);
                        (c.read_info)(png, info);
                        (c.set_gamma_fixed)(png, s, f);
                        match variant {
                            0 => {}
                            1 => build(png, 8),
                            2 => build(png, 16),
                            3 => {
                                build(png, 8);
                                build(png, 8);
                            }
                            4 => {
                                build(png, 8);
                                destroy(png);
                            }
                            5 => {
                                build(png, 16);
                                destroy(png);
                            }
                            6 => {
                                build(png, 8);
                                build(png, 16);
                            }
                            _ => {}
                        }
                        log(format!("variant {variant} done"));
                        (c.read_update_info)(png, info);
                        if variant == 7 {
                            build(png, bd as c_int);
                            log("post-update build done".to_string());
                        }
                        let rb = (c.get_rowbytes)(png, info);
                        log(format!("rowbytes={rb}"));
                        for r in 0..3 {
                            (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                            log(format!("row{r}={}", hex(&row[..rb])));
                        }
                        (c.read_end)(png, ptr::null_mut());
                        destroy(png);
                        log("final destroy done".to_string());
                    })
                });
            }
        }
    }
}

// ===========================================================================
// L27 — info struct lifecycle: png_info_init_3 / png_create_info_struct /
//        png_destroy_info_struct / png_data_freer / png_free_data
// ===========================================================================

/// Fill `info` with every chunk that owns memory, using `png` for allocation.
unsafe fn populate_info(c: &Core, png: Png, info: Info, seed: u64) {
    let mut r = Rng::new(seed);
    (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);

    let pal = pal_bytes(8, seed);
    (c.set_PLTE)(png, info, pal.as_ptr(), 8);

    let trans: Vec<u8> = (0..8).map(|_| r.byte()).collect();
    (c.set_tRNS)(png, info, trans.as_ptr(), 8, ptr::null());

    let hist: Vec<u16> = (0..8).map(|_| r.next_u32() as u16).collect();
    (c.set_hIST)(png, info, hist.as_ptr());

    let prof = Icc::valid().build();
    (c.set_iCCP)(
        png,
        info,
        b"icc\0".as_ptr() as *const c_char,
        0,
        prof.as_ptr(),
        prof.len() as u32,
    );

    (c.set_sCAL_s)(
        png,
        info,
        1,
        b"1.5\0".as_ptr() as *const c_char,
        b"2.5\0".as_ptr() as *const c_char,
    );

    let mut params: [*mut c_char; 2] = [
        b"1.0\0".as_ptr() as *mut c_char,
        b"2.0\0".as_ptr() as *mut c_char,
    ];
    (c.set_pCAL)(
        png,
        info,
        b"purpose\0".as_ptr() as *const c_char,
        0,
        100,
        PNG_EQUATION_LINEAR,
        2,
        b"units\0".as_ptr() as *const c_char,
        params.as_mut_ptr(),
    );

    let exif: Vec<u8> = b"II\x2a\x00abcd".to_vec();
    (c.set_eXIf_1)(png, info, exif.len() as u32, exif.as_ptr());

    let texts = [
        PngText {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: b"KeyOne\0".as_ptr() as *mut c_char,
            text: b"text one\0".as_ptr() as *mut c_char,
            text_length: 8,
            ..Default::default()
        },
        PngText {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: b"KeyTwo\0".as_ptr() as *mut c_char,
            text: b"text two\0".as_ptr() as *mut c_char,
            text_length: 8,
            ..Default::default()
        },
    ];
    (c.set_text)(png, info, texts.as_ptr() as *const c_void, 2);

    let mut sentries = [
        PngSpltEntry {
            red: 1,
            green: 2,
            blue: 3,
            alpha: 4,
            frequency: 5,
        },
        PngSpltEntry {
            red: 6,
            green: 7,
            blue: 8,
            alpha: 9,
            frequency: 10,
        },
    ];
    let splt = [PngSpltT {
        name: b"splt name\0".as_ptr() as *mut c_char,
        depth: 8,
        entries: sentries.as_mut_ptr(),
        nentries: 2,
    }];
    (c.set_sPLT)(png, info, splt.as_ptr() as *const c_void, 1);

    let mut ud1: Vec<u8> = vec![1, 2, 3, 4];
    let mut ud2: Vec<u8> = vec![9, 9];
    let unk = [
        PngUnknownChunk {
            name: *b"uNk1\0",
            data: ud1.as_mut_ptr(),
            size: ud1.len(),
            location: 1, // PNG_HAVE_IHDR
        },
        PngUnknownChunk {
            name: *b"uNk2\0",
            data: ud2.as_mut_ptr(),
            size: ud2.len(),
            location: 8, // PNG_AFTER_IDAT
        },
    ];
    (c.set_unknown_chunks)(png, info, unk.as_ptr() as *const c_void, 2);

    // Rows are allocated with png_malloc so that PNG_FREE_ROWS can legitimately
    // free them (exactly what png_read_png does).
    let h = 3usize;
    let rowbytes = 4usize;
    let rows = (c.malloc)(png, (h * std::mem::size_of::<*mut u8>()) as u64) as *mut *mut u8;
    for i in 0..h {
        let p = (c.malloc)(png, rowbytes as u64) as *mut u8;
        for k in 0..rowbytes {
            *p.add(k) = (i * 16 + k) as u8;
        }
        *rows.add(i) = p;
    }
    (c.set_rows)(png, info, rows);
    // png_set_rows does not take ownership, so say so explicitly.
    (c.data_freer)(png, info, PNG_DESTROY_WILL_FREE_DATA, F_ROWS);
}

/// `png_set_text_2` allocates `key`, `text`, `lang` and `lang_key` as ONE block
/// and points `text` into it, but `png_free_data(PNG_FREE_TEXT, num != -1)`
/// frees only `key`.  The remaining pointers are therefore dangling and their
/// *contents* are freed heap, i.e. exactly the kind of thing that legitimately
/// differs between two processes.  Log the structure instead.
unsafe fn log_text_struct(c: &Core, png: Png, info: Info) {
    let mut tptr: *mut c_void = ptr::null_mut();
    let mut ntext: c_int = 0;
    let n = (c.get_text)(png, info, &mut tptr, &mut ntext);
    log(format!("text n={n} num={ntext} arr_null={}", u8::from(tptr.is_null())));
    if n > 0 && !tptr.is_null() {
        let arr = std::slice::from_raw_parts(tptr as *const PngText, n as usize);
        for (i, t) in arr.iter().enumerate() {
            log(format!(
                "text[{i}] comp={} key_null={} text_null={} tlen={} ilen={} lang_null={} langkey_null={}",
                t.compression,
                u8::from(t.key.is_null()),
                u8::from(t.text.is_null()),
                t.text_length,
                t.itxt_length,
                u8::from(t.lang.is_null()),
                u8::from(t.lang_key.is_null())
            ));
        }
    }
}

/// Everything `log_all_info` logs except the text strings (see above).
unsafe fn log_info_state_notext(c: &Core, png: Png, info: Info, tag: &str) {
    log(format!("== state {tag} (text structural)"));
    let mut w = 0u32;
    let mut h = 0u32;
    let (mut bd, mut ct, mut il, mut cm, mut fm) = (0, 0, 0, 0, 0);
    let r = (c.get_IHDR)(png, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm);
    log(format!(
        "IHDR rc={r} w={w} h={h} depth={bd} color={ct} interlace={il} comp={cm} filter={fm}"
    ));
    log(format!(
        "rowbytes={} channels={} palette_max={}",
        (c.get_rowbytes)(png, info),
        (c.get_channels)(png, info),
        (c.get_palette_max)(png, info)
    ));
    for (name, flag) in [
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
    ] {
        log(format!("valid.{name}={}", (c.get_valid)(png, info, flag)));
    }
    let mut pal: *mut u8 = ptr::null_mut();
    let mut npal: c_int = -1;
    let r = (c.get_PLTE)(png, info, &mut pal, &mut npal);
    log(format!("PLTE rc={r} n={npal}"));
    if r != 0 && !pal.is_null() && npal > 0 {
        log(format!(
            "PLTE data={}",
            hex(std::slice::from_raw_parts(pal, npal as usize * 3))
        ));
    }
    let mut ta: *mut u8 = ptr::null_mut();
    let mut nt: c_int = -1;
    let mut tc: *mut u8 = ptr::null_mut();
    let r = (c.get_tRNS)(png, info, &mut ta, &mut nt, &mut tc);
    log(format!("tRNS rc={r} n={nt}"));
    if r != 0 && !ta.is_null() && nt > 0 {
        log(format!(
            "tRNS alpha={}",
            hex(std::slice::from_raw_parts(ta, nt as usize))
        ));
    }
    let mut hi: *mut u16 = ptr::null_mut();
    let r = (c.get_hIST)(png, info, &mut hi);
    log(format!("hIST rc={r} null={}", u8::from(hi.is_null())));
    if r != 0 && !hi.is_null() && npal > 0 {
        log(format!(
            "hIST v={:?}",
            std::slice::from_raw_parts(hi, npal as usize)
        ));
    }
    let mut nm: *mut c_char = ptr::null_mut();
    let mut comp: c_int = -1;
    let mut prof: *mut u8 = ptr::null_mut();
    let mut plen: u32 = 0;
    let r = (c.get_iCCP)(png, info, &mut nm, &mut comp, &mut prof, &mut plen);
    log(format!(
        "iCCP rc={r} name={} comp={comp} len={plen}",
        cstr(nm)
    ));
    let mut sunit: c_int = -1;
    let mut sw: *mut c_char = ptr::null_mut();
    let mut sh: *mut c_char = ptr::null_mut();
    let r = (c.get_sCAL_s)(png, info, &mut sunit, &mut sw, &mut sh);
    log(format!(
        "sCAL rc={r} unit={sunit} w={} h={}",
        cstr(sw),
        cstr(sh)
    ));
    let mut purpose: *mut c_char = ptr::null_mut();
    let (mut x0, mut x1) = (0i32, 0i32);
    let (mut etype, mut nparams) = (0, 0);
    let mut units: *mut c_char = ptr::null_mut();
    let mut params: *mut *mut c_char = ptr::null_mut();
    let r = (c.get_pCAL)(
        png,
        info,
        &mut purpose,
        &mut x0,
        &mut x1,
        &mut etype,
        &mut nparams,
        &mut units,
        &mut params,
    );
    log(format!(
        "pCAL rc={r} purpose={} x0={x0} x1={x1} type={etype} nparams={nparams} units={} params_null={}",
        cstr(purpose),
        cstr(units),
        u8::from(params.is_null())
    ));
    let mut exif: *mut u8 = ptr::null_mut();
    let mut elen: u32 = 0;
    let r = (c.get_eXIf_1)(png, info, &mut elen, &mut exif);
    log(format!("eXIf rc={r} len={elen}"));
    let mut splt: *mut c_void = ptr::null_mut();
    let n = (c.get_sPLT)(png, info, &mut splt);
    log(format!("sPLT n={n}"));
    if n > 0 && !splt.is_null() {
        let arr = std::slice::from_raw_parts(splt as *const PngSpltT, n as usize);
        for (i, e) in arr.iter().enumerate() {
            log(format!(
                "sPLT[{i}] name={} depth={} nentries={} entries_null={}",
                cstr(e.name),
                e.depth,
                e.nentries,
                u8::from(e.entries.is_null())
            ));
        }
    }
    log_text_struct(c, png, info);
    let mut uptr: *mut c_void = ptr::null_mut();
    let n = (c.get_unknown_chunks)(png, info, &mut uptr);
    log(format!("unknown n={n}"));
    if n > 0 && !uptr.is_null() {
        let arr = std::slice::from_raw_parts(uptr as *const PngUnknownChunk, n as usize);
        for (i, u) in arr.iter().enumerate() {
            log(format!(
                "unknown[{i}] name={} size={} loc={} data={}",
                String::from_utf8_lossy(&u.name[..4]),
                u.size,
                u.location,
                if u.data.is_null() {
                    "<null>".to_string()
                } else {
                    hex(std::slice::from_raw_parts(u.data, u.size))
                }
            ));
        }
    }
    log_rows(c, png, info);
}

unsafe fn log_rows(c: &Core, png: Png, info: Info) {
    let rows = (c.get_rows)(png, info);
    log(format!("rows_null={}", u8::from(rows.is_null())));
    if !rows.is_null() {
        for i in 0..3usize {
            let p = *rows.add(i);
            log(format!(
                "row[{i}]={}",
                if p.is_null() {
                    "<null>".to_string()
                } else {
                    hex(std::slice::from_raw_parts(p, 4))
                }
            ));
        }
    }
}

unsafe fn log_info_state(c: &Core, png: Png, info: Info, tag: &str) {
    log(format!("== state {tag}"));
    log_all_info(c, png, info);
    let rows = (c.get_rows)(png, info);
    log(format!("rows_null={}", u8::from(rows.is_null())));
    if !rows.is_null() {
        for i in 0..3usize {
            let p = *rows.add(i);
            log(format!(
                "row[{i}]={}",
                if p.is_null() {
                    "<null>".to_string()
                } else {
                    hex(std::slice::from_raw_parts(p, 4))
                }
            ));
        }
    }
}

#[test]
fn l27_free_data_masks() {
    for &(mname, mask) in FREE_MASKS {
        for num in [-1 as c_int, 0] {
            // See log_text_struct: freeing a single text item leaves
            // text[num].text dangling, so its contents must not be logged.
            let dangling_text = num != -1 && (mask & F_TEXT) != 0;
            let show = move |c: &Core, png: Png, info: Info, tag: &str| unsafe {
                if dangling_text {
                    log_info_state_notext(c, png, info, tag)
                } else {
                    log_info_state(c, png, info, tag)
                }
            };
            diff(&format!("L27 free_data mask={mname} num={num}"), |lib| {
                with_write(lib, &mut |c, png, _info| unsafe {
                    let info2 = (c.create_info)(png);
                    log(format!("info2={}", u8::from(!info2.is_null())));
                    populate_info(c, png, info2, 0x2701);
                    show(c, png, info2, "populated");
                    (c.free_data)(png, info2, mask, num);
                    show(c, png, info2, "after free_data");
                    // A second identical call must be a no-op (free_me cleared).
                    (c.free_data)(png, info2, mask, num);
                    show(c, png, info2, "after second free_data");
                    let mut i2 = info2;
                    (c.destroy_info)(png, &mut i2);
                    log(format!("destroyed_info null={}", u8::from(i2.is_null())));
                })
            });
        }
    }
}

#[test]
fn l27_data_freer_destroy_and_user() {
    for &(mname, mask) in FREE_MASKS {
        for freer in [PNG_DESTROY_WILL_FREE_DATA, PNG_USER_WILL_FREE_DATA] {
            diff(
                &format!("L27 data_freer freer={freer} mask={mname}"),
                |lib| {
                    with_write(lib, &mut |c, png, _info| unsafe {
                        let info2 = (c.create_info)(png);
                        populate_info(c, png, info2, 0x2702);
                        (c.data_freer)(png, info2, freer, mask);
                        log(format!("data_freer({freer},{mask:#x}) ok"));
                        log_info_state(c, png, info2, "after data_freer");
                        (c.free_data)(png, info2, F_ALL, -1);
                        log_info_state(c, png, info2, "after free_data(ALL)");
                        // Anything the user still owns has to be re-claimed or
                        // png_destroy_info_struct would leak it; that is fine
                        // for a test.
                        (c.data_freer)(png, info2, PNG_DESTROY_WILL_FREE_DATA, F_ALL);
                        let mut i2 = info2;
                        (c.destroy_info)(png, &mut i2);
                        log("destroyed_info".to_string());
                    })
                },
            );
        }
    }
}

#[test]
fn l27_data_freer_invalid_freer() {
    // PNG_SET_WILL_FREE_DATA (and anything else) is rejected with png_error.
    for freer in [0 as c_int, PNG_SET_WILL_FREE_DATA, 4, -1, 99] {
        diff(&format!("L27 data_freer invalid freer={freer}"), |lib| {
            with_write(lib, &mut |c, png, _info| unsafe {
                let info2 = (c.create_info)(png);
                populate_info(c, png, info2, 0x2703);
                log(format!("calling data_freer({freer})"));
                (c.data_freer)(png, info2, freer, F_ALL);
                log("data_freer returned".to_string());
                let mut i2 = info2;
                (c.destroy_info)(png, &mut i2);
            })
        });
    }
    // NULL arguments.
    diff("L27 data_freer/free_data NULL", |lib| {
        with_write(lib, &mut |c, png, _info| unsafe {
            (c.data_freer)(ptr::null_mut(), ptr::null_mut(), PNG_DESTROY_WILL_FREE_DATA, F_ALL);
            (c.free_data)(ptr::null_mut(), ptr::null_mut(), F_ALL, -1);
            (c.data_freer)(png, ptr::null_mut(), PNG_DESTROY_WILL_FREE_DATA, F_ALL);
            (c.free_data)(png, ptr::null_mut(), F_ALL, -1);
            log("null calls survived".to_string());
        })
    });
}

#[test]
fn l27_info_init_3() {
    // png_info_init_3 replaces the block when the caller's idea of
    // sizeof(png_info) is smaller than the library's.  Whether that happened is
    // observable (as a boolean) from the caller.
    let sizes: [usize; 14] = [
        0, 1, 4, 8, 64, 128, 256, 384, 512, 768, 1024, 2048, 8192, 65536,
    ];
    diff("L27 png_info_init_3", |lib| {
        with_write(lib, &mut |c, png, _info| unsafe {
            for &sz in &sizes {
                let info2 = (c.create_info)(png);
                populate_info(c, png, info2, 0x2704);
                let before = info2;
                let mut p = info2;
                (c.info_init_3)(&mut p, sz);
                log(format!(
                    "info_init_3(size={sz}) null={} changed={}",
                    u8::from(p.is_null()),
                    u8::from(p != before)
                ));
                if !p.is_null() {
                    log_info_state(c, png, p, &format!("after info_init_3({sz})"));
                    let mut q = p;
                    (c.destroy_info)(png, &mut q);
                }
            }
            // *ptr_ptr == NULL must be a no-op.
            let mut nul: Info = ptr::null_mut();
            (c.info_init_3)(&mut nul, 65536);
            log(format!("info_init_3(NULL) still_null={}", u8::from(nul.is_null())));
            // create/destroy round trip
            let a = (c.create_info)(png);
            let b = (c.create_info)(png);
            log(format!(
                "two infos non-null={} distinct={}",
                u8::from(!a.is_null() && !b.is_null()),
                u8::from(a != b)
            ));
            let mut x = a;
            (c.destroy_info)(png, &mut x);
            log(format!("destroy nulls out={}", u8::from(x.is_null())));
            let mut y = b;
            (c.destroy_info)(png, &mut y);
            // destroying a NULL info is a no-op
            let mut z: Info = ptr::null_mut();
            (c.destroy_info)(png, &mut z);
            (c.destroy_info)(ptr::null_mut(), &mut z);
            log("destroy_info NULL survived".to_string());
        })
    });
}

// ===========================================================================
// L28 — png_check_IHDR
// ===========================================================================

type CheckIhdr = unsafe extern "C" fn(Png, u32, u32, c_int, c_int, c_int, c_int, c_int);

#[allow(clippy::type_complexity)]
fn ihdr_combos() -> Vec<(u32, u32, c_int, c_int, c_int, c_int, c_int)> {
    let widths: [u32; 6] = [0, 1, 7, 8, 0x7fff_ffff, 0x8000_0000];
    let heights: [u32; 3] = [0, 1, 8];
    let depths: [c_int; 9] = [0, 1, 2, 3, 4, 7, 8, 16, 32];
    let cts: [c_int; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    let ils: [c_int; 3] = [0, 1, 2];
    let comps: [c_int; 2] = [0, 1];
    let filts: [c_int; 3] = [0, 1, 64];

    let mut v = Vec::new();
    // All 15 legal (colour, depth) pairs, fully legal otherwise.
    for &(ct, bd) in COMBOS {
        for &il in &ils[..2] {
            v.push((8u32, 8u32, bd as c_int, ct as c_int, il, 0, 0));
        }
    }
    // A deterministic random sample of the full cross product.
    let mut r = Rng::new(0x2801);
    for _ in 0..300 {
        v.push((
            widths[r.below(widths.len() as u32) as usize],
            heights[r.below(heights.len() as u32) as usize],
            depths[r.below(depths.len() as u32) as usize],
            cts[r.below(cts.len() as u32) as usize],
            ils[r.below(ils.len() as u32) as usize],
            comps[r.below(comps.len() as u32) as usize],
            filts[r.below(filts.len() as u32) as usize],
        ));
    }
    v
}

#[test]
fn l28_check_IHDR() {
    let combos = ihdr_combos();
    for benign in [1 as c_int, 0] {
        for (i, &(w, h, bd, ct, il, cm, fm)) in combos.iter().enumerate() {
            // png_error longjmps, so every combination gets its own run.
            let label = format!(
                "L28 #{i} benign={benign} w={w} h={h} bd={bd} ct={ct} il={il} cm={cm} fm={fm}"
            );
            diff(&label, |lib| {
                let f: CheckIhdr = lib.f("png_check_IHDR");
                with_read(lib, &[], &mut |c, png, _i| unsafe {
                    (c.set_benign_errors)(png, benign);
                    log("calling check_IHDR".to_string());
                    f(png, w, h, bd, ct, il, cm, fm);
                    log("check_IHDR returned".to_string());
                })
            });
        }
    }
}

#[test]
fn l28_check_IHDR_mng_and_limits() {
    // filter_type 64 is only accepted with the MNG filter feature permitted and
    // no PNG signature seen; png_check_IHDR is called directly here so no
    // signature has been read.
    for mng in [0u32, 0x01, 0x04, 0x05] {
        for ct in [0 as c_int, 2, 3, 6] {
            for fm in [0 as c_int, 64, 1] {
                let label = format!("L28 mng={mng:#x} ct={ct} filter={fm}");
                diff(&label, |lib| {
                    let f: CheckIhdr = lib.f("png_check_IHDR");
                    with_read(lib, &[], &mut |c, png, _i| unsafe {
                        log(format!(
                            "permit_mng={:#x}",
                            (c.permit_mng_features)(png, mng)
                        ));
                        f(png, 8, 8, 8, ct, 0, 0, fm);
                        log("returned".to_string());
                    })
                });
            }
        }
    }
    // user limits below the image size
    for (uw, uh) in [(4u32, 4u32), (1_000_000, 1_000_000), (0, 0)] {
        let label = format!("L28 user_limits {uw}x{uh}");
        diff(&label, |lib| {
            let f: CheckIhdr = lib.f("png_check_IHDR");
            with_read(lib, &[], &mut |c, png, _i| unsafe {
                (c.set_user_limits)(png, uw, uh);
                log(format!(
                    "limits w={} h={}",
                    (c.get_user_width_max)(png),
                    (c.get_user_height_max)(png)
                ));
                f(png, 8, 8, 8, 0, 0, 0, 0);
                log("returned".to_string());
            })
        });
    }
}

// ===========================================================================
// L29 — zlib plumbing: png_zstream_error / png_reset_zstream /
//        png_inflate_claim / png_zlib_inflate / png_inflate
// ===========================================================================
// png_inflate_claim and png_inflate are `static` in pngrutil.c (verified with
// `nm -D` on both .so files: neither library exports them), so they are driven
// through real inflate work on crafted zlib streams.

/// A valid zlib stream carrying `data` in stored blocks of at most `block`
/// bytes (so the "multi-block" path is taken).
fn zlib_blocks(data: &[u8], block: usize) -> Vec<u8> {
    let mut out = vec![0x78u8, 0x01];
    let b = block.max(1);
    let mut off = 0usize;
    loop {
        let n = std::cmp::min(b, data.len() - off);
        let last = off + n >= data.len();
        out.push(u8::from(last));
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!(n as u16)).to_le_bytes());
        out.extend_from_slice(&data[off..off + n]);
        off += n;
        if last {
            break;
        }
    }
    out.extend_from_slice(&pngbuild::adler32(data).to_be_bytes());
    out
}

#[derive(Clone, Copy, PartialEq)]
enum ZKind {
    Valid,
    MultiBlock,
    Truncated,
    BadAdler,
    BadCmfWindow,
    BadCmfMethod,
    BadHeaderCheck,
    BadStoredLen,
    NeedDict,
}

fn z_variants() -> &'static [(&'static str, ZKind)] {
    &[
        ("valid-stored", ZKind::Valid),
        ("multi-block", ZKind::MultiBlock),
        ("truncated", ZKind::Truncated),
        ("bad-adler32", ZKind::BadAdler),
        ("bad-cmf-window", ZKind::BadCmfWindow),
        ("bad-cmf-method", ZKind::BadCmfMethod),
        ("bad-header-check", ZKind::BadHeaderCheck),
        ("bad-stored-length", ZKind::BadStoredLen),
        ("need-dict", ZKind::NeedDict),
    ]
}

fn make_zstream(raw: &[u8], kind: ZKind) -> Vec<u8> {
    let mut z = pngbuild::zlib_stored(raw);
    match kind {
        ZKind::Valid => z,
        ZKind::MultiBlock => zlib_blocks(raw, 7),
        ZKind::Truncated => {
            let keep = z.len().saturating_sub(6).max(3);
            z.truncate(keep);
            z
        }
        ZKind::BadAdler => {
            let n = z.len();
            z[n - 1] ^= 0xff;
            z
        }
        // CINFO > 7 -> png_zlib_inflate rejects it ("invalid window size (libpng)")
        ZKind::BadCmfWindow => {
            z[0] = 0x88;
            z[1] = 0x1c; // 0x881c % 31 == 0
            z
        }
        // CM != 8 -> zlib "unknown compression method"
        ZKind::BadCmfMethod => {
            z[0] = 0x79;
            z[1] = 0x18; // 0x7918 % 31 == 0
            z
        }
        ZKind::BadHeaderCheck => {
            z[1] ^= 0x01; // breaks (CMF<<8|FLG) % 31 == 0
            z
        }
        ZKind::BadStoredLen => {
            if z.len() > 5 {
                z[4] ^= 0xff; // corrupt the stored block length complement
            }
            z
        }
        // FDICT set -> zlib returns Z_NEED_DICT, msg left NULL
        ZKind::NeedDict => {
            z[1] = 0x20; // 0x7820 % 31 == 0, FDICT set
            z
        }
    }
}

/// A PNG whose IDAT zlib stream is built by `make_zstream`.
fn idat_png(w: u32, h: u32, ct: u8, bd: u8, kind: ZKind, seed: u64) -> Vec<u8> {
    let b = Builder::new(w, h, bd, ct);
    let raw = b.raw_rows(seed);
    let z = make_zstream(&raw, kind);
    let mut chunks = vec![Chunk::new(b"IHDR", b.ihdr_bytes())];
    if ct == 3 {
        chunks.push(Chunk::new(b"PLTE", pal_bytes(1usize << bd, seed ^ 7)));
    }
    chunks.push(Chunk::new(b"IDAT", z));
    chunks.push(Chunk::new(b"IEND", Vec::new()));
    pngbuild::join(&chunks)
}

/// A PNG carrying a zTXt (and, optionally, an iCCP) whose zlib stream is built
/// by `make_zstream`.
fn text_png(kind: ZKind, with_iccp: bool, seed: u64) -> Vec<u8> {
    let b = Builder::new(4, 2, 8, 2);
    let raw = b.raw_rows(seed);

    let mut ztxt = b"Comment\0\0".to_vec(); // keyword, NUL, compression method 0
    ztxt.extend_from_slice(&make_zstream(b"a compressed comment, quite long", kind));

    let mut chunks = vec![Chunk::new(b"IHDR", b.ihdr_bytes())];
    if with_iccp {
        let prof = Icc::valid().build();
        let mut iccp = b"icc name\0\0".to_vec();
        iccp.extend_from_slice(&make_zstream(&prof, kind));
        chunks.push(Chunk::new(b"iCCP", iccp));
    }
    chunks.push(Chunk::new(b"zTXt", ztxt));
    chunks.push(Chunk::new(b"IDAT", pngbuild::zlib_stored(&raw)));
    chunks.push(Chunk::new(b"IEND", Vec::new()));
    pngbuild::join(&chunks)
}

#[test]
fn l29_idat_zlib_variants() {
    for &(name, kind) in z_variants() {
        for &(ct, bd) in &[(0u8, 8u8), (2, 8), (3, 4), (6, 16)] {
            for benign in [1 as c_int, 0] {
                let png = idat_png(5, 3, ct, bd, kind, 0x2901);
                let label = format!("L29 IDAT {name} ct={ct} bd={bd} benign={benign}");
                diff(&label, |lib| {
                    let mut row = vec![0u8; 256];
                    with_read(lib, &png, &mut |c, png, info| unsafe {
                        (c.set_benign_errors)(png, benign);
                        (c.read_info)(png, info);
                        let rb = (c.get_rowbytes)(png, info);
                        log(format!("rowbytes={rb}"));
                        for r in 0..3 {
                            (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                            log(format!("row{r}={}", hex(&row[..rb])));
                        }
                        (c.read_end)(png, ptr::null_mut());
                        log("read_end done".to_string());
                    })
                });
            }
        }
    }
}

#[test]
fn l29_text_zlib_variants() {
    for &(name, kind) in z_variants() {
        for with_iccp in [false, true] {
            for benign in [1 as c_int, 0] {
                let png = text_png(kind, with_iccp, 0x2902);
                let label = format!("L29 zTXt {name} iccp={with_iccp} benign={benign}");
                diff(&label, |lib| {
                    let mut row = vec![0u8; 64];
                    with_read(lib, &png, &mut |c, png, info| unsafe {
                        (c.set_benign_errors)(png, benign);
                        (c.read_info)(png, info);
                        log_all_info(c, png, info);
                        let rb = (c.get_rowbytes)(png, info);
                        for r in 0..2 {
                            (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                            log(format!("row{r}={}", hex(&row[..rb])));
                        }
                        (c.read_end)(png, ptr::null_mut());
                        log("read_end done".to_string());
                    })
                });
            }
        }
    }
}

#[test]
fn l29_reset_zstream_and_zlib_inflate() {
    // Directly on a fresh read struct (the z_stream has never been inflateInit'ed).
    diff("L29 reset_zstream fresh", |lib| {
        with_read(lib, &[], &mut |c, png, _i| unsafe {
            log(format!("reset_zstream(fresh)={}", (c.reset_zstream)(png)));
            log(format!("reset_zstream(fresh again)={}", (c.reset_zstream)(png)));
            log(format!(
                "reset_zstream(NULL)={}",
                (c.reset_zstream)(ptr::null_mut())
            ));
        })
    });

    let png_bytes = image_png(5, 3, 2, 8, 0, 0x2903);

    // After png_start_read_image the stream has been claimed for IDAT.
    diff("L29 reset_zstream after start_read_image", |lib| {
        let mut row = vec![0u8; 256];
        with_read(lib, &png_bytes, &mut |c, png, info| unsafe {
            (c.set_benign_errors)(png, 1);
            (c.read_info)(png, info);
            (c.start_read_image)(png);
            log(format!("reset_zstream(claimed)={}", (c.reset_zstream)(png)));
            let rb = (c.get_rowbytes)(png, info);
            for r in 0..3 {
                (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                log(format!("row{r}={}", hex(&row[..rb])));
            }
            (c.read_end)(png, ptr::null_mut());
        })
    });

    // png_zlib_inflate called directly.
    for flush in [0 as c_int, 1, 2, 4] {
        diff(&format!("L29 zlib_inflate fresh flush={flush}"), |lib| {
            let inflate: unsafe extern "C" fn(Png, c_int) -> c_int = lib.f("png_zlib_inflate");
            with_read(lib, &[], &mut |_c, png, _i| unsafe {
                log(format!("zlib_inflate(fresh,{flush})={}", inflate(png, flush)));
            })
        });
        diff(&format!("L29 zlib_inflate claimed flush={flush}"), |lib| {
            let inflate: unsafe extern "C" fn(Png, c_int) -> c_int = lib.f("png_zlib_inflate");
            with_read(lib, &png_bytes, &mut |c, png, info| unsafe {
                (c.set_benign_errors)(png, 1);
                (c.read_info)(png, info);
                (c.start_read_image)(png);
                log(format!("zlib_inflate(claimed,{flush})={}", inflate(png, flush)));
            })
        });
    }
}

#[test]
fn l29_zstream_error_messages() {
    // png_zstream_error only assigns z_stream::msg when it is still NULL, so a
    // direct call between png_start_read_image (which claims and therefore
    // resets the stream) and a failing inflate is fully observable: the message
    // reported for the failure is the pre-set one.
    let png = idat_png(5, 3, 2, 8, ZKind::NeedDict, 0x2904);
    let rets: [c_int; 12] = [0, 1, 2, -1, -2, -3, -4, -5, -6, -7, 99, -99];

    // Baseline: no direct call -> the message comes from the Z_NEED_DICT branch.
    diff("L29 zstream_error baseline", |lib| {
        let mut row = vec![0u8; 256];
        with_read(lib, &png, &mut |c, png, info| unsafe {
            (c.set_benign_errors)(png, 1);
            (c.read_info)(png, info);
            (c.start_read_image)(png);
            let rb = (c.get_rowbytes)(png, info);
            (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
            log(format!("row0={}", hex(&row[..rb])));
        })
    });

    for r in rets {
        diff(&format!("L29 zstream_error ret={r}"), |lib| {
            let zerr: unsafe extern "C" fn(Png, c_int) = lib.f("png_zstream_error");
            let mut row = vec![0u8; 256];
            with_read(lib, &png, &mut |c, png, info| unsafe {
                (c.set_benign_errors)(png, 1);
                (c.read_info)(png, info);
                (c.start_read_image)(png);
                zerr(png, r);
                log(format!("zstream_error({r}) returned"));
                let rb = (c.get_rowbytes)(png, info);
                (c.read_row)(png, row.as_mut_ptr(), ptr::null_mut());
                log(format!("row0={}", hex(&row[..rb])));
            })
        });
    }

    // Also exercise it on a write struct and on a fresh read struct where
    // nothing observes the message; it must simply not crash.
    diff("L29 zstream_error direct only", |lib| {
        let mut t = with_read(lib, &[], &mut |_c, png, _i| unsafe {
            let zerr: unsafe extern "C" fn(Png, c_int) = lib.f("png_zstream_error");
            for r in rets {
                zerr(png, r);
            }
            log("read zstream_error sweep survived".to_string());
        });
        let w = with_write(lib, &mut |_c, png, _i| unsafe {
            let zerr: unsafe extern "C" fn(Png, c_int) = lib.f("png_zstream_error");
            for r in rets {
                zerr(png, r);
            }
            log("write zstream_error sweep survived".to_string());
        });
        t.lines.extend(w.lines);
        t.rc += w.rc;
        t
    });
}

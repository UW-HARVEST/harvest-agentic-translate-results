//! Phase F — the simplified (`png_image_*`) API.
//!
//! Covers CONFIGS.md rows
//!   * C-138 `simplified::read_formats`  — `png_image_begin_read_from_memory`
//!   * C-139 `simplified::read_stdio`    — `..._from_stdio` / `..._from_file`
//!   * C-140 `simplified::write_formats` — `png_image_write_to_{memory,stdio,file}`
//!   * C-141 `simplified::round_trip`    — simplified write → simplified read
//!
//! i.e. configuration row **S. simplified formats**: the 8+1 8-bit formats ×
//! {plain, `_COLORMAP`}, the 4 `LINEAR` formats, every `PNG_IMAGE_FLAG_*`,
//! background supplied or not and `row_stride` positive/negative/zero.
//!
//! The simplified API never `longjmp`s out to the caller — it traps `png_error`
//! internally with `png_safe_execute` and reports through
//! `png_image::warning_or_error` + `png_image::message` — so `guarded` is not
//! used here.  Instead *every* observable field of the `png_image` (including
//! all 64 bytes of `message`), the return value of every call, every byte of
//! the output buffer and every byte of the colormap are recorded.
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_char, c_int, c_void};
use std::sync::atomic::Ordering::Relaxed;

/* There is no `libc` crate available offline, so declare what we need. */
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
}

const RB: *const c_char = b"rb\0".as_ptr() as *const c_char;
const WB: *const c_char = b"wb\0".as_ptr() as *const c_char;

/* ------------------------------------------------------------------ */
/* the PNG_IMAGE_* macros from png.h, transcribed exactly              */
/* ------------------------------------------------------------------ */

/// `PNG_IMAGE_SAMPLE_CHANNELS`
fn sample_channels(fmt: u32) -> u32 {
    (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}
/// `PNG_IMAGE_SAMPLE_COMPONENT_SIZE`
fn sample_component_size(fmt: u32) -> u32 {
    ((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
}
/// `PNG_IMAGE_SAMPLE_SIZE`
fn sample_size(fmt: u32) -> u32 {
    sample_channels(fmt) * sample_component_size(fmt)
}
/// `PNG_IMAGE_MAXIMUM_COLORMAP_COMPONENTS` expressed in bytes.
fn colormap_alloc(fmt: u32) -> usize {
    sample_channels(fmt) as usize * 256 * sample_component_size(fmt) as usize
}
/// `PNG_IMAGE_PIXEL_CHANNELS`
fn pixel_channels(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_channels(fmt)
    }
}
/// `PNG_IMAGE_PIXEL_COMPONENT_SIZE`
fn pixel_component_size(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_component_size(fmt)
    }
}
/// `PNG_IMAGE_ROW_STRIDE`
fn row_stride_natural(fmt: u32, width: u32) -> u32 {
    pixel_channels(fmt) * width
}
/// `PNG_IMAGE_BUFFER_SIZE(image, row_stride)`
fn buffer_size(fmt: u32, height: u32, row_stride: u32) -> usize {
    pixel_component_size(fmt) as usize * height as usize * row_stride as usize
}
/// `PNG_IMAGE_SIZE`
fn image_size(fmt: u32, width: u32, height: u32) -> usize {
    buffer_size(fmt, height, row_stride_natural(fmt, width))
}
/// `PNG_IMAGE_COLORMAP_SIZE`
fn colormap_size(fmt: u32, entries: u32) -> usize {
    sample_size(fmt) as usize * entries as usize
}

const PNG_ZBUF_SIZE: usize = 8192; // from c_src/include/pnglibconf.h

/// `PNG_ZLIB_MAX_SIZE`
fn zlib_max_size(b: usize) -> usize {
    b + ((b + 7) >> 3) + ((b + 63) >> 6) + 11
}

/// `PNG_IMAGE_PNG_SIZE_MAX`
fn png_size_max(fmt: u32, width: u32, height: u32, entries: u32) -> usize {
    // PNG_IMAGE_DATA_SIZE
    let data = image_size(fmt, width, height) + height as usize;
    let image_size_max = zlib_max_size(data); // PNG_IMAGE_COMPRESSED_SIZE_MAX
    let mut n = 8usize /*sig*/ + 25 /*IHDR*/ + 16 /*gAMA*/ + 44 /*cHRM*/ + 12 /*IEND*/;
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        n += 12 + 3 * entries as usize;
        if fmt & PNG_FORMAT_FLAG_ALPHA != 0 {
            n += 12 + entries as usize;
        }
    }
    n += 12;
    n + 12 * (image_size_max / PNG_ZBUF_SIZE) + image_size_max
}

/* ------------------------------------------------------------------ */
/* format tables                                                       */
/* ------------------------------------------------------------------ */

const BASE8: [(&str, u32); 9] = [
    ("GRAY", PNG_FORMAT_GRAY),
    ("GA", PNG_FORMAT_GA),
    ("AG", PNG_FORMAT_AG),
    ("RGB", PNG_FORMAT_RGB),
    ("BGR", PNG_FORMAT_BGR),
    ("RGBA", PNG_FORMAT_RGBA),
    ("ARGB", PNG_FORMAT_ARGB),
    ("BGRA", PNG_FORMAT_BGRA),
    ("ABGR", PNG_FORMAT_ABGR),
];

const LINEARS: [(&str, u32); 4] = [
    ("LINEAR_Y", PNG_FORMAT_LINEAR_Y),
    ("LINEAR_Y_ALPHA", PNG_FORMAT_LINEAR_Y_ALPHA),
    ("LINEAR_RGB", PNG_FORMAT_LINEAR_RGB),
    ("LINEAR_RGB_ALPHA", PNG_FORMAT_LINEAR_RGB_ALPHA),
];

/// The 22 output formats `png_image_finish_read` is asked for.
fn read_format_list() -> Vec<(String, u32)> {
    let mut v = Vec::new();
    for (n, f) in BASE8 {
        v.push((n.to_string(), f));
    }
    for (n, f) in BASE8 {
        v.push((format!("{}|CMAP", n), f | PNG_FORMAT_FLAG_COLORMAP));
    }
    for (n, f) in LINEARS {
        v.push((n.to_string(), f));
    }
    v
}

/// The 26 input formats `png_image_write_*` is given: the 22 above plus the four
/// 16-bit linear colour-mapped forms.
fn write_formats_list() -> Vec<(String, u32)> {
    let mut v = read_format_list();
    for (n, f) in LINEARS {
        v.push((format!("{}|CMAP", n), f | PNG_FORMAT_FLAG_COLORMAP));
    }
    v
}

/// Only the colour-mapped output formats (used for the `colormap_entries`
/// sweep, which drives the per-colour-type "too few entries" rejections in
/// `png_image_read_colormap`).
fn colormap_formats() -> Vec<(String, u32)> {
    write_formats_list()
        .into_iter()
        .filter(|(_, f)| f & PNG_FORMAT_FLAG_COLORMAP != 0)
        .collect()
}

const ALL_FLAGS: [u32; 4] = [
    0,
    PNG_IMAGE_FLAG_FAST,
    PNG_IMAGE_FLAG_16BIT_sRGB,
    PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB,
];

/* ------------------------------------------------------------------ */
/* recording                                                           */
/* ------------------------------------------------------------------ */

fn msg_bytes(im: &png_image) -> Vec<u8> {
    im.message.iter().map(|&c| c as u8).collect()
}

fn msg_text(im: &png_image) -> String {
    let b = msg_bytes(im);
    let n = b.iter().position(|&c| c == 0).unwrap_or(64);
    String::from_utf8_lossy(&b[..n]).into_owned()
}

/// Everything about a `png_image` that is comparable between the two libraries
/// (the `opaque` pointer value itself of course is not, only its nullness).
fn state(im: &png_image) -> String {
    format!(
        "opaque_null={} version={} {}x{} fmt=0x{:x} flags=0x{:x} entries={} woe={} msg={:?}",
        im.opaque.is_null(),
        im.version,
        im.width,
        im.height,
        im.format,
        im.flags,
        im.colormap_entries,
        im.warning_or_error,
        msg_text(im)
    )
}

/// The same, plus all 64 raw message bytes — used by the forked cases where the
/// only channel back to the parent is this string.
fn state_raw(im: &png_image) -> String {
    let b = msg_bytes(im);
    let mut hex = String::with_capacity(128);
    for &x in &b {
        hex.push_str(&format!("{:02x}", x));
    }
    note(im);
    format!("{} raw={}", state(im), hex)
}

/// Record a call's return value and the resulting `png_image`; all 64 bytes of
/// `message` go into `Outcome::output` so they are compared byte for byte.
fn snap(o: &mut Outcome, tag: &str, im: &png_image, ret: c_int) {
    o.push(format!("{}: ret={} {}", tag, ret, state(im)));
    o.output.extend_from_slice(&msg_bytes(im));
    note(im);
}

/// The simplified API traps `png_error` internally (`png_safe_error` copies the
/// text into `png_image::message`), so those diagnostics never reach the
/// harness' error/warning callbacks.  Feed them to `observe()` by hand so that
/// `tools/error_coverage.py` sees the simplified API's rejection sites.
fn note(im: &png_image) {
    if im.warning_or_error != 0 {
        let m = msg_text(im);
        if !m.is_empty() {
            observe(&m);
        }
    }
}

/// A deterministic, non-zero fill so that bytes libpng does *not* write are
/// still part of the comparison.
fn fill_pattern(b: &mut [u8], seed: u32) {
    let mut x = seed | 1;
    for v in b.iter_mut() {
        x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
        *v = (x >> 16) as u8;
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/simplified");
    let _ = std::fs::create_dir_all(&d);
    d.join(name)
}

fn cases_now() -> usize {
    CASES.load(Relaxed) + FORKED_CASES.load(Relaxed)
}

/// Sanity bookkeeping: how many `png_image_finish_read` / `png_image_write_*`
/// calls actually *succeeded*.  A test that only ever hits error paths would
/// still be "identical" in both libraries but would not test anything.
static OK_READS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static OK_WRITES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static NONTRIVIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/* ------------------------------------------------------------------ */
/* source PNGs, built with the low-level C writer                      */
/* ------------------------------------------------------------------ */

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Extra {
    None,
    Trns,
    Bkgd,
    Gama,
    Srgb,
    Chrm,
    /// cICP with non-sRGB primaries — highest priority input to
    /// `png_image_is_not_sRGB` (pngread.c:1247).
    Cicp,
    /// mDCV with non-sRGB primaries — likewise.
    Mdcv,
    Sbit,
    All,
}

const EXTRAS: [Extra; 10] = [
    Extra::None,
    Extra::Trns,
    Extra::Bkgd,
    Extra::Gama,
    Extra::Srgb,
    Extra::Chrm,
    Extra::Cicp,
    Extra::Mdcv,
    Extra::Sbit,
    Extra::All,
];

unsafe fn set_trns(api: &Api, png: *mut PngStruct, info: *mut PngInfo, img: &Img) {
    let maxval: u16 = if img.bit_depth >= 16 {
        0xffff
    } else {
        ((1u32 << img.bit_depth) - 1) as u16
    };
    match img.color_type {
        PNG_COLOR_TYPE_PALETTE => {
            let n = img.palette.len();
            let alpha: Vec<u8> = (0..n).map(|i| ((i * 37 + 5) & 0xff) as u8).collect();
            (api.png_set_tRNS)(png, info, alpha.as_ptr(), n as c_int, core::ptr::null());
        }
        PNG_COLOR_TYPE_GRAY => {
            let c = png_color_16 {
                index: 0,
                red: 0,
                green: 0,
                blue: 0,
                gray: maxval / 2,
            };
            (api.png_set_tRNS)(png, info, core::ptr::null(), 0, &c);
        }
        PNG_COLOR_TYPE_RGB => {
            let c = png_color_16 {
                index: 0,
                red: maxval / 3,
                green: maxval / 2,
                blue: maxval / 5,
                gray: 0,
            };
            (api.png_set_tRNS)(png, info, core::ptr::null(), 0, &c);
        }
        // tRNS is illegal for the two colour types that already have alpha.
        _ => {}
    }
}

unsafe fn set_bkgd(api: &Api, png: *mut PngStruct, info: *mut PngInfo, img: &Img) {
    let maxval: u16 = if img.bit_depth >= 16 {
        0xffff
    } else {
        ((1u32 << img.bit_depth) - 1) as u16
    };
    let c = if img.color_type == PNG_COLOR_TYPE_PALETTE {
        png_color_16 {
            index: 1,
            red: 0,
            green: 0,
            blue: 0,
            gray: 0,
        }
    } else if img.color_type & PNG_COLOR_MASK_COLOR != 0 {
        png_color_16 {
            index: 0,
            red: maxval / 4,
            green: maxval / 2,
            blue: maxval,
            gray: 0,
        }
    } else {
        png_color_16 {
            index: 0,
            red: 0,
            green: 0,
            blue: 0,
            gray: maxval / 4,
        }
    };
    (api.png_set_bKGD)(png, info, &c);
}

unsafe fn apply_extra(api: &Api, png: *mut PngStruct, info: *mut PngInfo, img: &Img, ex: Extra) {
    match ex {
        Extra::None => {}
        Extra::Trns => set_trns(api, png, info, img),
        Extra::Bkgd => set_bkgd(api, png, info, img),
        Extra::Gama => (api.png_set_gAMA_fixed)(png, info, 100_000),
        Extra::Srgb => (api.png_set_sRGB)(png, info, PNG_sRGB_INTENT_PERCEPTUAL),
        Extra::Chrm => (api.png_set_cHRM_fixed)(
            png, info, 31270, 32900, /* white */
            64000, 33000, /* red   */
            25000, 60000, /* green -- deliberately NOT sRGB */
            15000, 6000,  /* blue  */
        ),
        // BT.2020 primaries + BT.2020 transfer: definitely not sRGB.
        Extra::Cicp => (api.png_set_cICP)(png, info, 9, 14, 0, 1),
        Extra::Mdcv => (api.png_set_mDCV_fixed)(
            png, info, 31270, 32900, /* white */
            70800, 29200, /* red   */
            17000, 79700, /* green */
            13100, 4600,  /* blue  */
            10_000_000, 500,
        ),
        Extra::Sbit => {
            let d = img.bit_depth as u8;
            let sb = png_color_8 {
                red: d,
                green: d,
                blue: d,
                gray: d,
                alpha: d,
            };
            (api.png_set_sBIT)(png, info, &sb);
        }
        Extra::All => {
            set_trns(api, png, info, img);
            set_bkgd(api, png, info, img);
            (api.png_set_gAMA_fixed)(png, info, 45455);
            (api.png_set_cHRM_fixed)(
                png, info, 31270, 32900, 64000, 33000, 25000, 60000, 15000, 6000,
            );
        }
    }
}

struct Src {
    name: String,
    bytes: Vec<u8>,
}

/// Build one source PNG with the low-level writer, comparing the two libraries
/// while doing so (so a broken writer cannot silently poison the read tests).
fn build_src(tag: &str, img: &Img, ex: Extra) -> Src {
    let mut file = Vec::new();
    let name = format!(
        "{} ct={} bd={} il={} {}x{} {:?}",
        tag, img.color_type, img.bit_depth, img.interlace, img.w, img.h, ex
    );
    assert_same(&format!("build {}", name), |api| unsafe {
        let mut o = Outcome::default();
        let wr = write_image(api, img, &WriteOpts::default(), &mut |a, p, i| {
            apply_extra(a, p, i, img, ex)
        });
        o.push(format!("guard={:?}", wr.guard));
        o.output = wr.bytes.clone();
        if api.which == "C" {
            file = wr.bytes.clone();
        }
        o
    });
    // A truncated source would silently turn every read case below into an
    // error-path case, so insist the writer really produced a complete PNG.
    assert!(
        file.len() > 8 && file.ends_with(&chunk(b"IEND", &[])),
        "{}: incomplete PNG ({} bytes)",
        name,
        file.len()
    );
    Src { name, bytes: file }
}

/// All 15 shapes × interlace × the seven chunk sets.
fn all_sources(seed: u64, w: u32, h: u32) -> Vec<Src> {
    let mut out = Vec::new();
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for ex in EXTRAS {
                let mut rng = Rng::new(
                    seed ^ ((ct as u64) << 40)
                        ^ ((bd as u64) << 32)
                        ^ ((il as u64) << 24)
                        ^ ((EXTRAS.iter().position(|&e| e == ex).unwrap() as u64) << 16),
                );
                let mut img = Img::random(&mut rng, w, h, ct, bd);
                img.interlace = il;
                out.push(build_src("src", &img, ex));
            }
        }
    }
    out
}

/* ------------------------------------------------------------------ */
/* the read driver                                                     */
/* ------------------------------------------------------------------ */

/// `row_stride` selector: 0 → "natural", 1 → natural, 2 → natural+extra,
/// 3 → -natural, 4 → -(natural+extra).
const STRIDES: [usize; 5] = [0, 1, 2, 3, 4];

fn stride_for(sel: usize, nat: u32, extra: u32) -> i32 {
    match sel {
        0 => 0,
        1 => nat as i32,
        2 => (nat + extra) as i32,
        3 => -(nat as i32),
        _ => -((nat + extra) as i32),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Source<'a> {
    Memory(&'a [u8]),
    /// A real `FILE*` obtained with `fopen`.
    Stdio(&'a str),
    /// A real path handed to `png_image_begin_read_from_file`.
    File(&'a str),
}

/// One complete simplified read: begin → set format/flags → finish → free.
unsafe fn read_one(
    api: &Api,
    src: Source,
    fmt: u32,
    flags: u32,
    bg: Option<png_color>,
    stride_sel: usize,
    stride_extra: u32,
) -> Outcome {
    read_one_ex(api, src, fmt, flags, bg, stride_sel, stride_extra, None)
}

/// As `read_one`, but `entries_override` replaces the `colormap_entries` value
/// `png_image_begin_read_*` reported (which is how an application can starve
/// `png_image_read_colormap` of colour-map space).
#[allow(clippy::too_many_arguments)]
unsafe fn read_one_ex(
    api: &Api,
    src: Source,
    fmt: u32,
    flags: u32,
    bg: Option<png_color>,
    stride_sel: usize,
    stride_extra: u32,
    entries_override: Option<u32>,
) -> Outcome {
    let mut o = Outcome::default();
    let mut im = png_image::default();
    im.version = PNG_IMAGE_VERSION;

    let mut fp: *mut c_void = core::ptr::null_mut();
    let r0 = match src {
        Source::Memory(b) => {
            (api.png_image_begin_read_from_memory)(&mut im, b.as_ptr() as *const c_void, b.len())
        }
        Source::Stdio(p) => {
            let cp = cs(p);
            fp = fopen(cp.as_ptr(), RB);
            assert!(!fp.is_null(), "fopen({}) failed", p);
            (api.png_image_begin_read_from_stdio)(&mut im, fp)
        }
        Source::File(p) => {
            let cp = cs(p);
            (api.png_image_begin_read_from_file)(&mut im, cp.as_ptr())
        }
    };
    snap(&mut o, "begin", &im, r0);

    if r0 != 0 {
        im.format = fmt;
        im.flags |= flags;
        if let Some(e) = entries_override {
            im.colormap_entries = e;
        }
        o.push(format!(
            "request fmt=0x{:x} flags=0x{:x} entries={}",
            im.format, im.flags, im.colormap_entries
        ));

        let nat = row_stride_natural(fmt, im.width);
        let stride = stride_for(stride_sel, nat, stride_extra);
        let abs = if stride == 0 { nat } else { stride.unsigned_abs() };
        let bufsz = buffer_size(fmt, im.height, abs).max(1);
        let mut buf = vec![0u8; bufsz];
        fill_pattern(&mut buf, 0x5a5a_0001);
        let cmsz = colormap_alloc(fmt);
        let mut cmap = vec![0u8; cmsz];
        fill_pattern(&mut cmap, 0xa5a5_0002);
        o.push(format!(
            "bufsz={} stride={} cmap_alloc={} cmap_size={}",
            bufsz,
            stride,
            cmsz,
            colormap_size(fmt, im.colormap_entries)
        ));

        let bgp: *const png_color = match &bg {
            Some(c) => c as *const png_color,
            None => core::ptr::null(),
        };
        let r1 = (api.png_image_finish_read)(
            &mut im,
            bgp,
            buf.as_mut_ptr() as *mut c_void,
            stride,
            cmap.as_mut_ptr() as *mut c_void,
        );
        snap(&mut o, "finish", &im, r1);
        if r1 != 0 {
            OK_READS.fetch_add(1, Relaxed);
            let mut fresh = vec![0u8; buf.len()];
            fill_pattern(&mut fresh, 0x5a5a_0001);
            if fresh != buf {
                NONTRIVIAL.fetch_add(1, Relaxed);
            }
        }
        o.output.extend_from_slice(&buf);
        o.output.extend_from_slice(&cmap);
    }

    (api.png_image_free)(&mut im);
    snap(&mut o, "after free", &im, 0);
    // A second free must be a harmless no-op.
    (api.png_image_free)(&mut im);
    snap(&mut o, "after free x2", &im, 0);
    if !fp.is_null() {
        // `begin_read_from_stdio` does not take ownership of the FILE*.
        fclose(fp);
    }
    o
}

/* ------------------------------------------------------------------ */
/* C-138: png_image_begin_read_from_memory × every output format       */
/* ------------------------------------------------------------------ */

#[test]
fn read_formats() {
    let t0 = cases_now();
    let fmts = read_format_list();
    let bgc = {
        let mut rng = Rng::new(0xbac6);
        png_color {
            red: rng.u8(),
            green: rng.u8(),
            blue: rng.u8(),
        }
    };

    let backgrounds = [
        None,
        Some(bgc),
        Some(png_color { red: 0, green: 0, blue: 0 }),
        Some(png_color {
            red: 255,
            green: 255,
            blue: 255,
        }),
    ];

    /* ---- 1. every source shape × every output format × every stride -- */
    let srcs = all_sources(0x513f, 13, 11);
    let mut rng = Rng::new(0x1234_5678);
    for s in &srcs {
        for (fname, fmt) in &fmts {
            for sel in STRIDES {
                let flags = rng.pick(&ALL_FLAGS);
                let bg = rng.pick(&backgrounds);
                assert_same(
                    &format!(
                        "mem {} -> {} flags=0x{:x} bg={:?} stride#{}",
                        s.name, fname, flags, bg, sel
                    ),
                    |api| unsafe {
                        read_one(api, Source::Memory(&s.bytes), *fmt, flags, bg, sel, 5)
                    },
                );
            }
        }
    }

    /* ---- 2. the full flag × background × row_stride matrix ---------- */
    // Four representative sources: 1-bit gray, 8-bit palette with tRNS,
    // 16-bit RGBA and 8-bit gray+alpha interlaced.
    let mut axis: Vec<Src> = Vec::new();
    for (ct, bd, il, ex) in [
        (PNG_COLOR_TYPE_GRAY, 1, PNG_INTERLACE_NONE, Extra::None),
        (PNG_COLOR_TYPE_GRAY, 8, PNG_INTERLACE_NONE, Extra::Trns),
        (PNG_COLOR_TYPE_GRAY, 16, PNG_INTERLACE_NONE, Extra::Trns),
        (PNG_COLOR_TYPE_PALETTE, 4, PNG_INTERLACE_ADAM7, Extra::Bkgd),
        (PNG_COLOR_TYPE_PALETTE, 8, PNG_INTERLACE_NONE, Extra::Trns),
        (PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, Extra::Trns),
        (PNG_COLOR_TYPE_RGB, 16, PNG_INTERLACE_NONE, Extra::Mdcv),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8, PNG_INTERLACE_NONE, Extra::Srgb),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16, PNG_INTERLACE_NONE, Extra::Gama),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8, PNG_INTERLACE_ADAM7, Extra::Chrm),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16, PNG_INTERLACE_NONE, Extra::None),
    ] {
        let mut rng = Rng::new(0xa715 ^ ((ct as u64) << 8) ^ bd as u64);
        let mut img = Img::random(&mut rng, 5, 4, ct, bd);
        img.interlace = il;
        axis.push(build_src("axis", &img, ex));
    }
    for s in &axis {
        for (fname, fmt) in &fmts {
            for flags in ALL_FLAGS {
                for bg in backgrounds {
                    for sel in STRIDES {
                        assert_same(
                            &format!(
                                "axis {} -> {} flags=0x{:x} bg={:?} stride#{}",
                                s.name, fname, flags, bg, sel
                            ),
                            |api| unsafe {
                                read_one(api, Source::Memory(&s.bytes), *fmt, flags, bg, sel, 3)
                            },
                        );
                    }
                }
            }
        }
    }

    /* ---- 3. row_stride values that are much larger than natural ----- */
    for s in axis.iter() {
        for (fname, fmt) in &fmts {
            for extra in [1u32, 7, 64, 1000] {
                for sel in [2usize, 4] {
                    assert_same(
                        &format!("stride {} -> {} +{} #{}", s.name, fname, extra, sel),
                        |api| unsafe {
                            read_one(api, Source::Memory(&s.bytes), *fmt, 0, None, sel, extra)
                        },
                    );
                }
            }
        }
    }

    /* ---- 3b. image geometries, including 1-pixel rows/columns ------- */
    for (w, h) in [(1u32, 1u32), (1, 9), (9, 1), (2, 2), (3, 5), (16, 9), (31, 17)] {
        for (ct, bd) in VALID_SHAPES.iter() {
            for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                let mut r = Rng::new(0x5123 ^ ((w as u64) << 20) ^ ((h as u64) << 8) ^ (*bd as u64));
                let mut img = Img::random(&mut r, w, h, *ct, *bd);
                img.interlace = il;
                let src = build_src("geom", &img, Extra::Trns);
                for (fname, fmt) in &fmts {
                    for sel in [0usize, 3] {
                        assert_same(
                            &format!("geom {} -> {} #{}", src.name, fname, sel),
                            |api| unsafe {
                                read_one(api, Source::Memory(&src.bytes), *fmt, 0, None, sel, 1)
                            },
                        );
                    }
                }
            }
        }
    }

    /* ---- 3c. colour-map output with a starved `colormap_entries` ----
     * This is what drives every per-colour-type "... color-map: too few
     * entries" rejection in `png_image_read_colormap` (pngread.c:2056 …
     * 2695) as well as the successful paths just above each threshold. */
    let cmfmts = colormap_formats();
    for (ct, bd) in VALID_SHAPES {
        for ex in [Extra::None, Extra::Trns] {
            let mut r = Rng::new(0xcc11 ^ ((ct as u64) << 8) ^ bd as u64);
            let img = Img::random(&mut r, 5, 4, ct, bd);
            let src = build_src("cmap-entries", &img, ex);
            for (fname, fmt) in &cmfmts {
                for e in [
                    0u32, 1, 2, 3, 4, 5, 16, 17, 100, 200, 215, 216, 217, 231, 243, 244, 255, 256,
                    257, 0xffff_ffff,
                ] {
                    for bg in [None, Some(bgc)] {
                        assert_same(
                            &format!(
                                "cmap-entries {} -> {} entries={} bg={}",
                                src.name,
                                fname,
                                e,
                                bg.is_some()
                            ),
                            |api| unsafe {
                                read_one_ex(
                                    api,
                                    Source::Memory(&src.bytes),
                                    *fmt,
                                    0,
                                    bg,
                                    0,
                                    0,
                                    Some(e),
                                )
                            },
                        );
                    }
                }
            }
        }
    }

    /* ---- 4. error paths of the read entry points -------------------- */
    let good = &srcs[0].bytes;

    for v in [0u32, 2, 0xffff_ffff] {
        assert_same_forked(&format!("begin_read_from_memory version={}", v), |api| unsafe {
            let mut im = png_image::default();
            im.version = v;
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                good.as_ptr() as *const c_void,
                good.len(),
            );
            format!("ret={} {}", r, state_raw(&im))
        });
        assert_same_forked(&format!("finish_read version={}", v), |api| unsafe {
            let mut im = png_image::default();
            im.version = v;
            im.width = 4;
            im.height = 4;
            im.format = PNG_FORMAT_RGB;
            let mut buf = vec![0u8; 64];
            let r = (api.png_image_finish_read)(
                &mut im,
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0,
                core::ptr::null_mut(),
            );
            format!("ret={} {} buf={:02x?}", r, state_raw(&im), &buf[..8])
        });
    }

    // `opaque` not NULL on entry: point it at a zeroed png_control-sized block
    // so that the ensuing png_image_free() finds png_ptr == NULL and bails out.
    assert_same_forked("begin_read_from_memory opaque != NULL", |api| unsafe {
        let fake = vec![0usize; 32];
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        im.opaque = fake.as_ptr() as *mut c_void;
        let r = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as *const c_void,
            good.len(),
        );
        format!("ret={} {}", r, state_raw(&im))
    });

    // NULL image / NULL memory / zero size.
    assert_same_forked("begin_read_from_memory NULL image", |api| unsafe {
        let r = (api.png_image_begin_read_from_memory)(
            core::ptr::null_mut(),
            good.as_ptr() as *const c_void,
            good.len(),
        );
        format!("ret={}", r)
    });
    assert_same_forked("finish_read NULL image", |api| unsafe {
        let mut buf = vec![0u8; 16];
        let r = (api.png_image_finish_read)(
            core::ptr::null_mut(),
            core::ptr::null(),
            buf.as_mut_ptr() as *mut c_void,
            0,
            core::ptr::null_mut(),
        );
        format!("ret={}", r)
    });
    assert_same_forked("png_image_free NULL image", |api| unsafe {
        (api.png_image_free)(core::ptr::null_mut());
        "survived".to_string()
    });
    assert_same_forked("begin_read_from_memory NULL memory", |api| unsafe {
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        let r = (api.png_image_begin_read_from_memory)(&mut im, core::ptr::null(), 100);
        format!("ret={} {}", r, state_raw(&im))
    });
    assert_same_forked("begin_read_from_memory size=0", |api| unsafe {
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        let r = (api.png_image_begin_read_from_memory)(&mut im, good.as_ptr() as *const c_void, 0);
        format!("ret={} {}", r, state_raw(&im))
    });

    // Truncated / corrupt data.
    for cut in [1usize, 8, 20, 33] {
        let short = good[..cut.min(good.len())].to_vec();
        assert_same(&format!("begin_read_from_memory truncated to {}", cut), |api| unsafe {
            read_one(api, Source::Memory(&short), PNG_FORMAT_RGBA, 0, None, 0, 0)
        });
    }

    // finish_read without a preceding begin, including width/height 0 and huge
    // (image->opaque == NULL short-circuits before the division in the C, so
    // these are all well defined).
    for (tag, w, h, fmt, stride) in [
        ("3x2", 3u32, 2u32, PNG_FORMAT_RGB, 0i32),
        ("0x0", 0, 0, PNG_FORMAT_RGB, 0),
        ("0x4", 0, 4, PNG_FORMAT_RGBA, 0),
        ("4x0", 4, 0, PNG_FORMAT_GRAY, 0),
        ("huge w", 0xffff_ffff, 1, PNG_FORMAT_RGBA, 0),
        ("huge h", 4, 0xffff_ffff, PNG_FORMAT_GRAY, 0),
        ("huge stride", 4, 4, PNG_FORMAT_GRAY, 0x7fff_ffff),
        ("neg stride", 4, 4, PNG_FORMAT_GRAY, -4),
        ("small stride", 4, 4, PNG_FORMAT_RGB, 4),
    ] {
        assert_same_forked(&format!("finish_read without begin {}", tag), |api| unsafe {
            let mut im = png_image::default();
            im.version = PNG_IMAGE_VERSION;
            im.width = w;
            im.height = h;
            im.format = fmt;
            let mut buf = vec![0u8; 4 * 16 * 16];
            fill_pattern(&mut buf, 0x77);
            let r = (api.png_image_finish_read)(
                &mut im,
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                stride,
                core::ptr::null_mut(),
            );
            format!("ret={} {} buf={:02x?}", r, state_raw(&im), &buf[..16])
        });
    }

    // NULL buffer, colour-mapped format with a NULL colormap, colormap_entries
    // tampering, width/height/row_stride overflow — all after a good begin.
    let bad_after_begin: [(&str, fn(&mut png_image) -> (i32, bool, bool)); 8] = [
        ("null buffer", |_im| (0, false, true)),
        ("cmap fmt, null colormap", |im| {
            im.format |= PNG_FORMAT_FLAG_COLORMAP;
            (0, true, false)
        }),
        ("colormap_entries=0", |im| {
            im.format |= PNG_FORMAT_FLAG_COLORMAP;
            im.colormap_entries = 0;
            (0, true, true)
        }),
        ("colormap_entries=1", |im| {
            im.format |= PNG_FORMAT_FLAG_COLORMAP;
            im.colormap_entries = 1;
            (0, true, true)
        }),
        ("colormap_entries=257", |im| {
            im.format |= PNG_FORMAT_FLAG_COLORMAP;
            im.colormap_entries = 257;
            (0, true, true)
        }),
        ("colormap_entries=0xffffffff", |im| {
            im.format |= PNG_FORMAT_FLAG_COLORMAP;
            im.colormap_entries = 0xffff_ffff;
            (0, true, true)
        }),
        ("width huge", |im| {
            im.format = PNG_FORMAT_RGBA;
            im.width = 0x7fff_ffff;
            (0, false, true)
        }),
        ("row_stride huge", |im| {
            im.format = PNG_FORMAT_RGBA;
            (0x7fff_ffff, false, true)
        }),
    ];
    for (tag, tamper) in bad_after_begin {
        assert_same_forked(&format!("finish_read {}", tag), |api| unsafe {
            let mut im = png_image::default();
            im.version = PNG_IMAGE_VERSION;
            let r0 = (api.png_image_begin_read_from_memory)(
                &mut im,
                good.as_ptr() as *const c_void,
                good.len(),
            );
            let mut out = format!("begin ret={} {}\n", r0, state_raw(&im));
            if r0 != 0 {
                im.format = PNG_FORMAT_RGBA;
                let (stride, want_cmap, want_buf) = tamper(&mut im);
                let mut buf = vec![0u8; 4 * 64 * 64];
                fill_pattern(&mut buf, 0x11);
                let mut cmap = vec![0u8; 4 * 256];
                fill_pattern(&mut cmap, 0x22);
                let bp: *mut c_void = if want_buf {
                    buf.as_mut_ptr() as *mut c_void
                } else {
                    core::ptr::null_mut()
                };
                let cp: *mut c_void = if want_cmap {
                    cmap.as_mut_ptr() as *mut c_void
                } else {
                    core::ptr::null_mut()
                };
                let r1 = (api.png_image_finish_read)(&mut im, core::ptr::null(), bp, stride, cp);
                out += &format!("finish ret={} {}\n", r1, state_raw(&im));
                out += &format!("cmap={:02x?}\n", &cmap[..32]);
            }
            (api.png_image_free)(&mut im);
            out += &format!("free {}", state_raw(&im));
            out
        });
    }

    // Lifecycle abuse: two begins without a free, free between begin and
    // finish, and finish called twice.
    assert_same_forked("begin_read_from_memory twice", |api| unsafe {
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        let r0 = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as *const c_void,
            good.len(),
        );
        let mut out = format!("begin1 ret={} {}\n", r0, state_raw(&im));
        let r1 = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as *const c_void,
            good.len(),
        );
        out += &format!("begin2 ret={} {}\n", r1, state_raw(&im));
        (api.png_image_free)(&mut im);
        out += &format!("free {}", state_raw(&im));
        out
    });
    assert_same_forked("free between begin and finish", |api| unsafe {
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        let r0 = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as *const c_void,
            good.len(),
        );
        let mut out = format!("begin ret={} {}\n", r0, state_raw(&im));
        (api.png_image_free)(&mut im);
        out += &format!("free {}\n", state_raw(&im));
        im.format = PNG_FORMAT_RGBA;
        let mut buf = vec![0u8; 4 * 64 * 64];
        fill_pattern(&mut buf, 0x99);
        let r1 = (api.png_image_finish_read)(
            &mut im,
            core::ptr::null(),
            buf.as_mut_ptr() as *mut c_void,
            0,
            core::ptr::null_mut(),
        );
        out += &format!("finish ret={} {} buf={:02x?}", r1, state_raw(&im), &buf[..16]);
        out
    });
    assert_same_forked("finish_read twice", |api| unsafe {
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        let r0 = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as *const c_void,
            good.len(),
        );
        let mut out = format!("begin ret={} {}\n", r0, state_raw(&im));
        im.format = PNG_FORMAT_RGBA;
        let n = buffer_size(PNG_FORMAT_RGBA, im.height, row_stride_natural(PNG_FORMAT_RGBA, im.width));
        let mut buf = vec![0u8; n.max(1)];
        fill_pattern(&mut buf, 0xaa);
        let r1 = (api.png_image_finish_read)(
            &mut im,
            core::ptr::null(),
            buf.as_mut_ptr() as *mut c_void,
            0,
            core::ptr::null_mut(),
        );
        out += &format!("finish1 ret={} {}\n", r1, state_raw(&im));
        let r2 = (api.png_image_finish_read)(
            &mut im,
            core::ptr::null(),
            buf.as_mut_ptr() as *mut c_void,
            0,
            core::ptr::null_mut(),
        );
        out += &format!("finish2 ret={} {}\n", r2, state_raw(&im));
        out += &format!("buf={:02x?}", &buf[..16.min(buf.len())]);
        out
    });
    // png_image_free called twice, and on a never-initialised image.
    assert_same_forked("png_image_free twice / uninitialised", |api| unsafe {
        let mut im = png_image::default();
        (api.png_image_free)(&mut im);
        let mut out = format!("free-of-zeroed {}\n", state_raw(&im));
        im.version = PNG_IMAGE_VERSION;
        let r0 = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as *const c_void,
            good.len(),
        );
        out += &format!("begin ret={} {}\n", r0, state_raw(&im));
        (api.png_image_free)(&mut im);
        out += &format!("free1 {}\n", state_raw(&im));
        (api.png_image_free)(&mut im);
        out += &format!("free2 {}\n", state_raw(&im));
        (api.png_image_free)(&mut im);
        out += &format!("free3 {}", state_raw(&im));
        out
    });

    // Formats with bits libpng does not define.
    for f in [
        PNG_FORMAT_FLAG_ASSOCIATED_ALPHA,
        PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_ASSOCIATED_ALPHA,
        PNG_FORMAT_RGBA_COLORMAP | PNG_FORMAT_FLAG_ASSOCIATED_ALPHA,
        0x80,
        0x100,
        0xffff_ffff,
    ] {
        assert_same_forked(&format!("finish_read undefined format 0x{:x}", f), |api| unsafe {
            let mut im = png_image::default();
            im.version = PNG_IMAGE_VERSION;
            let r0 = (api.png_image_begin_read_from_memory)(
                &mut im,
                good.as_ptr() as *const c_void,
                good.len(),
            );
            let mut out = format!("begin ret={} {}\n", r0, state_raw(&im));
            if r0 != 0 {
                im.format = f;
                // Generously over-allocate: an undefined format may make libpng
                // compute a bigger row than PNG_IMAGE_BUFFER_SIZE would.
                let mut buf = vec![0u8; 8 * 64 * 64];
                fill_pattern(&mut buf, 0x33);
                let mut cmap = vec![0u8; 8 * 256];
                fill_pattern(&mut cmap, 0x44);
                let r1 = (api.png_image_finish_read)(
                    &mut im,
                    core::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    row_stride_natural(f, im.width) as i32,
                    cmap.as_mut_ptr() as *mut c_void,
                );
                out += &format!("finish ret={} {}\n", r1, state_raw(&im));
                out += &format!("buf={:02x?} cmap={:02x?}", &buf[..48], &cmap[..48]);
            }
            (api.png_image_free)(&mut im);
            out
        });
    }

    eprintln!(
        "simplified::read_formats: {} comparisons, {} successful reads ({} \
         with data), {} successful writes",
        cases_now() - t0,
        OK_READS.load(Relaxed),
        NONTRIVIAL.load(Relaxed),
        OK_WRITES.load(Relaxed)
    );
}

/* ------------------------------------------------------------------ */
/* C-139: the stdio / file read entry points                           */
/* ------------------------------------------------------------------ */

#[test]
fn read_stdio() {
    let t0 = cases_now();
    let fmts = read_format_list();

    /* ---- 1. every shape × interlace × a few chunk sets ------------- */
    let mut rng = Rng::new(0xf11e_0001);
    let mut n = 0usize;
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for ex in [Extra::None, Extra::Trns, Extra::Gama] {
                let mut r = Rng::new(0x57d1 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ (il as u64));
                let mut img = Img::random(&mut r, 6, 5, ct, bd);
                img.interlace = il;
                let src = build_src("stdio-src", &img, ex);
                n += 1;
                let path = scratch(&format!("read{}.png", n));
                std::fs::write(&path, &src.bytes).expect("write source png");
                let ps = path.to_str().unwrap().to_string();

                for (fname, fmt) in fmts.iter() {
                    let flags = rng.pick(&ALL_FLAGS);
                    let sel = rng.pick(&STRIDES);
                    let bg = if rng.bool() {
                        Some(png_color {
                            red: rng.u8(),
                            green: rng.u8(),
                            blue: rng.u8(),
                        })
                    } else {
                        None
                    };
                    assert_same(
                        &format!("stdio {} -> {} flags=0x{:x} #{}", src.name, fname, flags, sel),
                        |api| unsafe { read_one(api, Source::Stdio(&ps), *fmt, flags, bg, sel, 2) },
                    );
                    assert_same(
                        &format!("file {} -> {} flags=0x{:x} #{}", src.name, fname, flags, sel),
                        |api| unsafe { read_one(api, Source::File(&ps), *fmt, flags, bg, sel, 2) },
                    );
                }
            }
        }
    }

    /* ---- 2. the complete format list through both entry points ------ */
    for (ct, bd, ex) in [
        (PNG_COLOR_TYPE_GRAY, 1, Extra::None),
        (PNG_COLOR_TYPE_PALETTE, 4, Extra::Trns),
        (PNG_COLOR_TYPE_PALETTE, 8, Extra::Bkgd),
        (PNG_COLOR_TYPE_RGB, 16, Extra::Chrm),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8, Extra::None),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16, Extra::Gama),
    ] {
        let mut r = Rng::new(0x9911 ^ ((ct as u64) << 8) ^ bd as u64);
        let img = Img::random(&mut r, 7, 3, ct, bd);
        let src = build_src("stdio-full", &img, ex);
        let path = scratch(&format!("full-{}-{}.png", ct, bd));
        std::fs::write(&path, &src.bytes).expect("write source png");
        let ps = path.to_str().unwrap().to_string();
        for (fname, fmt) in &fmts {
            for flags in ALL_FLAGS {
                for sel in STRIDES {
                    assert_same(
                        &format!("stdio-full {} -> {} flags=0x{:x} #{}", src.name, fname, flags, sel),
                        |api| unsafe { read_one(api, Source::Stdio(&ps), *fmt, flags, None, sel, 0) },
                    );
                    assert_same(
                        &format!("file-full {} -> {} flags=0x{:x} #{}", src.name, fname, flags, sel),
                        |api| unsafe { read_one(api, Source::File(&ps), *fmt, flags, None, sel, 0) },
                    );
                }
            }
        }
    }

    /* ---- 3. error paths -------------------------------------------- */
    let ok_path = scratch("read1.png");
    let ok = ok_path.to_str().unwrap().to_string();

    assert_same_forked("begin_read_from_stdio NULL file", |api| unsafe {
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        let r = (api.png_image_begin_read_from_stdio)(&mut im, core::ptr::null_mut());
        format!("ret={} {}", r, state_raw(&im))
    });
    assert_same_forked("begin_read_from_stdio NULL image", |api| unsafe {
        let cp = cs(&ok);
        let fp = fopen(cp.as_ptr(), RB);
        let r = (api.png_image_begin_read_from_stdio)(core::ptr::null_mut(), fp);
        fclose(fp);
        format!("ret={}", r)
    });
    assert_same_forked("begin_read_from_file NULL name", |api| unsafe {
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        let r = (api.png_image_begin_read_from_file)(&mut im, core::ptr::null());
        format!("ret={} {}", r, state_raw(&im))
    });
    assert_same_forked("begin_read_from_file NULL image", |api| unsafe {
        let cp = cs(&ok);
        let r = (api.png_image_begin_read_from_file)(core::ptr::null_mut(), cp.as_ptr());
        format!("ret={}", r)
    });
    assert_same_forked("begin_read_from_file missing", |api| unsafe {
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        let cp = cs("/nonexistent/directory/does-not-exist.png");
        let r = (api.png_image_begin_read_from_file)(&mut im, cp.as_ptr());
        format!("ret={} {}", r, state_raw(&im))
    });
    for v in [0u32, 2, 0xffff_ffff] {
        assert_same_forked(&format!("begin_read_from_file version={}", v), |api| unsafe {
            let mut im = png_image::default();
            im.version = v;
            let cp = cs(&ok);
            let r = (api.png_image_begin_read_from_file)(&mut im, cp.as_ptr());
            format!("ret={} {}", r, state_raw(&im))
        });
        assert_same_forked(&format!("begin_read_from_stdio version={}", v), |api| unsafe {
            let mut im = png_image::default();
            im.version = v;
            let cp = cs(&ok);
            let fp = fopen(cp.as_ptr(), RB);
            let r = (api.png_image_begin_read_from_stdio)(&mut im, fp);
            fclose(fp);
            format!("ret={} {}", r, state_raw(&im))
        });
    }
    // opaque non-NULL on entry, through the file entry point.
    assert_same_forked("begin_read_from_file opaque != NULL", |api| unsafe {
        let fake = vec![0usize; 32];
        let mut im = png_image::default();
        im.version = PNG_IMAGE_VERSION;
        im.opaque = fake.as_ptr() as *mut c_void;
        let cp = cs(&ok);
        let r = (api.png_image_begin_read_from_file)(&mut im, cp.as_ptr());
        format!("ret={} {}", r, state_raw(&im))
    });
    // A file that is not a PNG at all.
    let junk_path = scratch("junk.bin");
    std::fs::write(&junk_path, Rng::new(0xdead).bytes(300)).unwrap();
    let junk = junk_path.to_str().unwrap().to_string();
    assert_same("begin_read_from_file junk", |api| unsafe {
        read_one(api, Source::File(&junk), PNG_FORMAT_RGBA, 0, None, 0, 0)
    });
    assert_same("begin_read_from_stdio junk", |api| unsafe {
        read_one(api, Source::Stdio(&junk), PNG_FORMAT_RGBA, 0, None, 0, 0)
    });
    // An empty file.
    let empty_path = scratch("empty.bin");
    std::fs::write(&empty_path, []).unwrap();
    let empty = empty_path.to_str().unwrap().to_string();
    assert_same("begin_read_from_file empty", |api| unsafe {
        read_one(api, Source::File(&empty), PNG_FORMAT_GRAY, 0, None, 0, 0)
    });
    assert_same("begin_read_from_stdio empty", |api| unsafe {
        read_one(api, Source::Stdio(&empty), PNG_FORMAT_GRAY, 0, None, 0, 0)
    });

    eprintln!(
        "simplified::read_stdio: {} comparisons, {} successful reads ({} \
         with data), {} successful writes",
        cases_now() - t0,
        OK_READS.load(Relaxed),
        NONTRIVIAL.load(Relaxed),
        OK_WRITES.load(Relaxed)
    );
}

/* ------------------------------------------------------------------ */
/* the write driver                                                    */
/* ------------------------------------------------------------------ */

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dest {
    /// `png_image_write_to_memory(.., memory = NULL, ..)` — a size query.
    MemQuery,
    /// A buffer of exactly the queried size.
    MemExact,
    /// One byte too small.
    MemShort,
    /// A zero-length buffer (`*memory_bytes == 0` with `memory != NULL`).
    MemZero,
    /// `PNG_IMAGE_PNG_SIZE_MAX` bytes.
    MemBig,
    Stdio,
    File,
}

const DESTS: [Dest; 7] = [
    Dest::MemQuery,
    Dest::MemExact,
    Dest::MemShort,
    Dest::MemZero,
    Dest::MemBig,
    Dest::Stdio,
    Dest::File,
];

struct WImg {
    w: u32,
    h: u32,
    fmt: u32,
    flags: u32,
    entries: u32,
    /// `PNG_IMAGE_BUFFER_SIZE(image, |row_stride|)` bytes of pixel data.
    buf: Vec<u8>,
    /// `PNG_IMAGE_COLORMAP_SIZE` bytes, empty when the format has no colormap.
    cmap: Vec<u8>,
    stride: i32,
}

/// Build the input for one simplified write.  Colour-map indices are always
/// `< colormap_entries` so that the packed low-bit-depth palette written by
/// `png_image_write_main` stays valid.
fn make_winput(seed: u64, w: u32, h: u32, fmt: u32, flags: u32, entries: u32, stride_sel: usize, extra: u32) -> WImg {
    let nat = row_stride_natural(fmt, w);
    let stride = stride_for(stride_sel, nat, extra);
    let abs = if stride == 0 { nat } else { stride.unsigned_abs() };
    let mut buf = vec![0u8; buffer_size(fmt, h, abs).max(1)];
    fill_pattern(&mut buf, (seed as u32) | 0x9000_0000);
    let cmap = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        // Keep the colour-map indices inside the palette libpng will write.
        let n = entries.clamp(1, 256);
        for b in buf.iter_mut() {
            *b = (*b as u32 % n) as u8;
        }
        let mut c = vec![0u8; colormap_alloc(fmt)];
        fill_pattern(&mut c, (seed as u32) | 0x4000_0000);
        c
    } else {
        Vec::new()
    };
    WImg {
        w,
        h,
        fmt,
        flags,
        entries,
        buf,
        cmap,
        stride,
    }
}

fn fresh_wimage(wi: &WImg) -> png_image {
    let mut im = png_image::default();
    im.version = PNG_IMAGE_VERSION;
    im.width = wi.w;
    im.height = wi.h;
    im.format = wi.fmt;
    im.flags = wi.flags;
    im.colormap_entries = wi.entries;
    im
}

/// One complete simplified write.  Returns the produced PNG bytes (for the
/// round-trip test) in `Outcome::output`.
unsafe fn write_one(api: &Api, wi: &WImg, conv8: c_int, dest: Dest, path: &str) -> Outcome {
    let mut o = Outcome::default();
    let bufp = wi.buf.as_ptr() as *const c_void;
    let cmapp: *const c_void = if wi.cmap.is_empty() {
        core::ptr::null()
    } else {
        wi.cmap.as_ptr() as *const c_void
    };
    o.push(format!(
        "input {}x{} fmt=0x{:x} flags=0x{:x} entries={} stride={} bufsz={} cmapsz={} conv8={} dest={:?}",
        wi.w, wi.h, wi.fmt, wi.flags, wi.entries, wi.stride, wi.buf.len(), wi.cmap.len(), conv8, dest
    ));

    match dest {
        Dest::MemQuery | Dest::MemExact | Dest::MemShort | Dest::MemZero | Dest::MemBig => {
            // Size query first; `size` need not be initialised, use a sentinel
            // so it is visible whether libpng wrote to it at all.
            let mut im = fresh_wimage(wi);
            let mut sz: usize = 0xdead_beef;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                core::ptr::null_mut(),
                &mut sz,
                conv8,
                bufp,
                wi.stride,
                cmapp,
            );
            o.push(format!("query ret={} size={}", r, sz));
            snap(&mut o, "query", &im, r);
            (api.png_image_free)(&mut im);

            if dest == Dest::MemQuery {
                return o;
            }
            if r == 0 || sz == 0 || sz > (1 << 22) {
                o.push("skip real write".to_string());
                return o;
            }

            let want = match dest {
                Dest::MemExact => sz,
                Dest::MemShort => sz - 1,
                Dest::MemZero => 0,
                _ => png_size_max(wi.fmt, wi.w, wi.h, wi.entries).max(sz + 64),
            };
            let mut mem = vec![0u8; want.max(1)];
            fill_pattern(&mut mem, 0x3333_0001);
            let mut im2 = fresh_wimage(wi);
            let mut n = want;
            let r2 = (api.png_image_write_to_memory)(
                &mut im2,
                mem.as_mut_ptr() as *mut c_void,
                &mut n,
                conv8,
                bufp,
                wi.stride,
                cmapp,
            );
            o.push(format!("write ret={} n={} cap={}", r2, n, want));
            if r2 != 0 {
                OK_WRITES.fetch_add(1, Relaxed);
            }
            snap(&mut o, "write", &im2, r2);
            (api.png_image_free)(&mut im2);
            o.output.extend_from_slice(&mem);
        }

        Dest::Stdio => {
            let mut im = fresh_wimage(wi);
            let cp = cs(path);
            let fp = fopen(cp.as_ptr(), WB);
            assert!(!fp.is_null(), "fopen({}) for writing", path);
            let r = (api.png_image_write_to_stdio)(&mut im, fp, conv8, bufp, wi.stride, cmapp);
            if r != 0 {
                OK_WRITES.fetch_add(1, Relaxed);
            }
            o.push(format!("stdio ret={}", r));
            snap(&mut o, "stdio", &im, r);
            fflush(fp);
            fclose(fp);
            (api.png_image_free)(&mut im);
            let bytes = std::fs::read(path).unwrap_or_default();
            o.push(format!("file bytes={}", bytes.len()));
            o.output.extend_from_slice(&bytes);
        }

        Dest::File => {
            let _ = std::fs::remove_file(path);
            let mut im = fresh_wimage(wi);
            let cp = cs(path);
            let r =
                (api.png_image_write_to_file)(&mut im, cp.as_ptr(), conv8, bufp, wi.stride, cmapp);
            if r != 0 {
                OK_WRITES.fetch_add(1, Relaxed);
            }
            o.push(format!("file ret={}", r));
            snap(&mut o, "file", &im, r);
            (api.png_image_free)(&mut im);
            let bytes = std::fs::read(path).unwrap_or_default();
            o.push(format!("file exists={} bytes={}", bytes.is_empty(), bytes.len()));
            o.output.extend_from_slice(&bytes);
        }
    }
    o
}

/* ------------------------------------------------------------------ */
/* C-140: png_image_write_to_{memory,stdio,file}                        */
/* ------------------------------------------------------------------ */

#[test]
fn write_formats() {
    let t0 = cases_now();
    let fmts = write_formats_list();
    let path = scratch("write.png").to_str().unwrap().to_string();

    /* ---- 1. format × convert_to_8bit × row_stride × destination ----- */
    // The third geometry is deliberately large: the 16-bit linear write path
    // (`png_write_image_16bit`) only rounds differently for one component value
    // in 32768, so it needs a lot of pixels to be exercised meaningfully.
    for (gi, (gw, gh)) in [(9u32, 7u32), (4, 4), (40, 24)].iter().enumerate() {
        for (i, (fname, fmt)) in fmts.iter().enumerate() {
            for conv8 in [0, 1] {
                for sel in STRIDES {
                    let entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 17 } else { 0 };
                    let wi = make_winput(
                        0x7000 + i as u64 + ((gi as u64) << 20),
                        *gw,
                        *gh,
                        *fmt,
                        0,
                        entries,
                        sel,
                        4,
                    );
                    for dest in DESTS {
                        assert_same(
                            &format!(
                                "write {} {}x{} conv8={} stride#{} {:?}",
                                fname, gw, gh, conv8, sel, dest
                            ),
                            |api| unsafe { write_one(api, &wi, conv8, dest, &path) },
                        );
                    }
                }
            }
        }
    }

    /* ---- 1b. a large 16-bit linear image ---------------------------
     * `png_write_image_16bit` / `png_write_image_8bit` un-premultiply with
     * `(component * reciprocal + 16384) >> 15`; a rounding error there only
     * shows up for about one component value in 32768, so this needs a big
     * image with plenty of alpha values to be a meaningful check. */
    for (i, (fname, fmt)) in LINEARS.iter().enumerate() {
        for conv8 in [0, 1] {
            for sel in [1usize, 3] {
                let wi = make_winput(0x6000 + i as u64, 256, 200, *fmt, 0, 0, sel, 0);
                assert_same(
                    &format!("write big {} conv8={} stride#{}", fname, conv8, sel),
                    |api| unsafe { write_one(api, &wi, conv8, Dest::MemExact, &path) },
                );
            }
        }
    }

    /* ---- 2. colormap_entries across the palette bit-depth thresholds - */
    for (i, (fname, fmt)) in fmts.iter().enumerate() {
        if fmt & PNG_FORMAT_FLAG_COLORMAP == 0 {
            continue;
        }
        for entries in [1u32, 2, 3, 4, 5, 16, 17, 255, 256, 257, 1000] {
            for conv8 in [0, 1] {
                let wi = make_winput(0x8000 + i as u64 + (entries as u64) * 31, 8, 5, *fmt, 0, entries, 1, 0);
                for dest in [Dest::MemExact, Dest::MemShort, Dest::File] {
                    assert_same(
                        &format!("write {} entries={} conv8={} {:?}", fname, entries, conv8, dest),
                        |api| unsafe { write_one(api, &wi, conv8, dest, &path) },
                    );
                }
            }
        }
    }

    /* ---- 3. every PNG_IMAGE_FLAG_* -------------------------------- */
    for (i, (fname, fmt)) in fmts.iter().enumerate() {
        for flags in ALL_FLAGS {
            for conv8 in [0, 1] {
                let entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 5 } else { 0 };
                let wi = make_winput(0x9000 + i as u64, 11, 4, *fmt, flags, entries, 0, 0);
                for dest in [Dest::MemExact, Dest::Stdio, Dest::File] {
                    assert_same(
                        &format!("write {} flags=0x{:x} conv8={} {:?}", fname, flags, conv8, dest),
                        |api| unsafe { write_one(api, &wi, conv8, dest, &path) },
                    );
                }
            }
        }
    }

    /* ---- 4. a few different image geometries ---------------------- */
    for (w, h) in [(1u32, 1u32), (1, 9), (9, 1), (2, 3), (33, 2), (17, 13), (64, 3)] {
        for (fname, fmt) in fmts.iter() {
            let entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 200 } else { 0 };
            for sel in [0usize, 2, 3, 4] {
                let wi = make_winput(
                    0xa000 ^ ((w as u64) << 8) ^ (h as u64),
                    w,
                    h,
                    *fmt,
                    0,
                    entries,
                    sel,
                    2,
                );
                assert_same(
                    &format!("write {} {}x{} #{}", fname, w, h, sel),
                    |api| unsafe { write_one(api, &wi, 0, Dest::MemExact, &path) },
                );
            }
        }
    }

    /* ---- 5. error paths ------------------------------------------- */
    let plain = make_winput(0xb000, 4, 4, PNG_FORMAT_RGB, 0, 0, 0, 0);
    let bufp = plain.buf.as_ptr() as *const c_void;

    for v in [0u32, 2, 0xffff_ffff] {
        assert_same_forked(&format!("write_to_memory version={}", v), |api| unsafe {
            let mut im = fresh_wimage(&plain);
            im.version = v;
            let mut sz = 0usize;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                core::ptr::null_mut(),
                &mut sz,
                0,
                bufp,
                0,
                core::ptr::null(),
            );
            format!("ret={} size={} {}", r, sz, state_raw(&im))
        });
        assert_same_forked(&format!("write_to_file version={}", v), |api| unsafe {
            let mut im = fresh_wimage(&plain);
            im.version = v;
            let p = scratch("err.png");
            let cp = cs(p.to_str().unwrap());
            let r = (api.png_image_write_to_file)(&mut im, cp.as_ptr(), 0, bufp, 0, core::ptr::null());
            format!("ret={} {}", r, state_raw(&im))
        });
        assert_same_forked(&format!("write_to_stdio version={}", v), |api| unsafe {
            let mut im = fresh_wimage(&plain);
            im.version = v;
            let p = scratch("err2.png");
            let cp = cs(p.to_str().unwrap());
            let fp = fopen(cp.as_ptr(), WB);
            let r = (api.png_image_write_to_stdio)(&mut im, fp, 0, bufp, 0, core::ptr::null());
            fclose(fp);
            format!("ret={} {}", r, state_raw(&im))
        });
    }

    assert_same_forked("write_to_memory opaque != NULL", |api| unsafe {
        let fake = vec![0usize; 32];
        let mut im = fresh_wimage(&plain);
        im.opaque = fake.as_ptr() as *mut c_void;
        let mut sz = 0usize;
        let r = (api.png_image_write_to_memory)(
            &mut im,
            core::ptr::null_mut(),
            &mut sz,
            0,
            bufp,
            0,
            core::ptr::null(),
        );
        format!("ret={} size={} {}", r, sz, state_raw(&im))
    });

    assert_same_forked("write_to_memory NULL image", |api| unsafe {
        let mut sz = 0usize;
        let r = (api.png_image_write_to_memory)(
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut sz,
            0,
            bufp,
            0,
            core::ptr::null(),
        );
        format!("ret={} size={}", r, sz)
    });
    assert_same_forked("write_to_memory NULL memory_bytes", |api| unsafe {
        let mut im = fresh_wimage(&plain);
        let r = (api.png_image_write_to_memory)(
            &mut im,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            0,
            bufp,
            0,
            core::ptr::null(),
        );
        format!("ret={} {}", r, state_raw(&im))
    });
    assert_same_forked("write_to_memory NULL buffer", |api| unsafe {
        let mut im = fresh_wimage(&plain);
        let mut sz = 0usize;
        let r = (api.png_image_write_to_memory)(
            &mut im,
            core::ptr::null_mut(),
            &mut sz,
            0,
            core::ptr::null(),
            0,
            core::ptr::null(),
        );
        format!("ret={} size={} {}", r, sz, state_raw(&im))
    });
    assert_same_forked("write_to_stdio NULL file", |api| unsafe {
        let mut im = fresh_wimage(&plain);
        let r = (api.png_image_write_to_stdio)(
            &mut im,
            core::ptr::null_mut(),
            0,
            bufp,
            0,
            core::ptr::null(),
        );
        format!("ret={} {}", r, state_raw(&im))
    });
    assert_same_forked("write_to_stdio NULL image", |api| unsafe {
        let p = scratch("err3.png");
        let cp = cs(p.to_str().unwrap());
        let fp = fopen(cp.as_ptr(), WB);
        let r = (api.png_image_write_to_stdio)(
            core::ptr::null_mut(),
            fp,
            0,
            bufp,
            0,
            core::ptr::null(),
        );
        fclose(fp);
        format!("ret={}", r)
    });
    assert_same_forked("write_to_file NULL name", |api| unsafe {
        let mut im = fresh_wimage(&plain);
        let r = (api.png_image_write_to_file)(&mut im, core::ptr::null(), 0, bufp, 0, core::ptr::null());
        format!("ret={} {}", r, state_raw(&im))
    });
    assert_same_forked("write_to_file NULL image", |api| unsafe {
        let p = scratch("err4.png");
        let cp = cs(p.to_str().unwrap());
        let r = (api.png_image_write_to_file)(
            core::ptr::null_mut(),
            cp.as_ptr(),
            0,
            bufp,
            0,
            core::ptr::null(),
        );
        format!("ret={}", r)
    });
    assert_same_forked("write_to_file unopenable", |api| unsafe {
        let mut im = fresh_wimage(&plain);
        let cp = cs("/nonexistent/directory/out.png");
        let r = (api.png_image_write_to_file)(&mut im, cp.as_ptr(), 0, bufp, 0, core::ptr::null());
        format!("ret={} {}", r, state_raw(&im))
    });

    // width / height 0 or huge, undefined format bits, colormap format with no
    // colormap, colormap_entries = 0.
    //
    // NOTE `width == 0`: png_image_write_main computes
    // `0xffffffffU/png_row_stride` with png_row_stride == 0 (pngwrite.c:2045),
    // i.e. a division by zero, and the C library dies from SIGFPE.  The Rust
    // translation performs that division with `util::c_div_u32`, which issues the
    // same `div` instruction and therefore raises the same hardware trap (a plain
    // Rust `/` would panic and abort with SIGABRT instead).  Both `width == 0`
    // cases are included below and must die from signal 8.
    let bad: [(&str, u32, u32, u32, u32, i32, bool); 13] = [
        /* tag, w, h, fmt, entries, stride, give_cmap */
        ("width=0", 0, 4, PNG_FORMAT_RGB, 0, 0, false),
        ("width=0 gray", 0, 1, PNG_FORMAT_GRAY, 0, 0, false),
        ("height=0", 4, 0, PNG_FORMAT_RGB, 0, 0, false),
        ("height=0 gray", 1, 0, PNG_FORMAT_GRAY, 0, 0, false),
        ("width=0x7fffffff", 0x7fff_ffff, 1, PNG_FORMAT_RGB, 0, 0, false),
        ("width=0xffffffff", 0xffff_ffff, 1, PNG_FORMAT_GRAY, 0, 0, false),
        ("height huge", 0x10000, 0x10000, PNG_FORMAT_RGBA, 0, 0, false),
        ("stride too small", 8, 4, PNG_FORMAT_RGB, 0, 8, false),
        ("stride -1", 8, 4, PNG_FORMAT_GRAY, 0, -1, false),
        ("cmap fmt, no colormap", 4, 4, PNG_FORMAT_RGB_COLORMAP, 8, 0, false),
        ("cmap fmt, entries=0", 4, 4, PNG_FORMAT_RGB_COLORMAP, 0, 0, true),
        ("undefined format 0x80", 4, 4, 0x80, 0, 4, false),
        ("undefined format 0x40", 4, 4, PNG_FORMAT_RGBA | 0x40, 0, 16, false),
    ];
    for (tag, w, h, fmt, entries, stride, give_cmap) in bad {
        assert_same_forked(&format!("write bad: {}", tag), |api| unsafe {
            let mut im = png_image::default();
            im.version = PNG_IMAGE_VERSION;
            im.width = w;
            im.height = h;
            im.format = fmt;
            im.colormap_entries = entries;
            let mut b = vec![0u8; 8 * 64 * 64];
            fill_pattern(&mut b, 0x55);
            let mut c = vec![0u8; 8 * 256];
            fill_pattern(&mut c, 0x66);
            let cp: *const c_void = if give_cmap {
                c.as_ptr() as *const c_void
            } else {
                core::ptr::null()
            };
            let mut sz = 0usize;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                core::ptr::null_mut(),
                &mut sz,
                0,
                b.as_ptr() as *const c_void,
                stride,
                cp,
            );
            format!("ret={} size={} {}", r, sz, state_raw(&im))
        });
    }

    eprintln!(
        "simplified::write_formats: {} comparisons, {} successful reads ({} \
         with data), {} successful writes",
        cases_now() - t0,
        OK_READS.load(Relaxed),
        NONTRIVIAL.load(Relaxed),
        OK_WRITES.load(Relaxed)
    );
}

/* ------------------------------------------------------------------ */
/* C-141: simplified write → simplified read                           */
/* ------------------------------------------------------------------ */

#[test]
fn round_trip() {
    let t0 = cases_now();
    let wfmts = write_formats_list();
    let rfmts = read_format_list();
    let path = scratch("rt.png").to_str().unwrap().to_string();

    for (i, (wname, wfmt)) in wfmts.iter().enumerate() {
        for conv8 in [0, 1] {
            let entries = if wfmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 13 } else { 0 };
            let wi = make_winput(0xc000 + i as u64, 7, 5, *wfmt, 0, entries, 0, 0);

            // 1. write it (and compare the two writers byte for byte).  The
            //    File destination gives the exact PNG with no trailing padding.
            let mut file = Vec::new();
            assert_same(&format!("rt write {} conv8={}", wname, conv8), |api| unsafe {
                let mut o = write_one(api, &wi, conv8, Dest::MemBig, &path);
                let o2 = write_one(api, &wi, conv8, Dest::File, &path);
                o.trace.extend(o2.trace);
                o.output.extend_from_slice(&o2.output);
                if api.which == "C" {
                    file = o2.output.clone();
                }
                o
            });

            if file.is_empty() {
                continue;
            }

            // 2. read it back into every output format
            for (rname, rfmt) in &rfmts {
                for sel in STRIDES {
                    assert_same(
                        &format!("rt {} conv8={} -> {} #{}", wname, conv8, rname, sel),
                        |api| unsafe {
                            read_one(api, Source::Memory(&file), *rfmt, 0, None, sel, 3)
                        },
                    );
                }
            }
        }
    }

    /* A second pass through the whole cycle: simplified write, simplified
     * read, simplified write again — the second PNG must be identical for
     * both libraries. */
    for (i, (wname, wfmt)) in wfmts.iter().enumerate().step_by(2) {
        let entries = if wfmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 9 } else { 0 };
        let wi = make_winput(0xd000 + i as u64, 6, 4, *wfmt, 0, entries, 1, 0);
        let mut file = Vec::new();
        assert_same(&format!("rt2 write {}", wname), |api| unsafe {
            let o = write_one(api, &wi, 0, Dest::File, &path);
            let bytes = std::fs::read(&path).unwrap_or_default();
            if api.which == "C" {
                file = bytes;
            }
            o
        });
        if file.is_empty() {
            continue;
        }
        for (rname, rfmt) in rfmts.iter() {
            assert_same(&format!("rt2 {} -> {} -> write", wname, rname), |api| unsafe {
                let mut o = Outcome::default();
                let mut im = png_image::default();
                im.version = PNG_IMAGE_VERSION;
                let r0 = (api.png_image_begin_read_from_memory)(
                    &mut im,
                    file.as_ptr() as *const c_void,
                    file.len(),
                );
                snap(&mut o, "begin", &im, r0);
                if r0 == 0 {
                    return o;
                }
                im.format = *rfmt;
                let w = im.width;
                let h = im.height;
                let nat = row_stride_natural(*rfmt, w);
                let mut buf = vec![0u8; buffer_size(*rfmt, h, nat).max(1)];
                fill_pattern(&mut buf, 0x7777_0001);
                let mut cmap = vec![0u8; colormap_alloc(*rfmt)];
                fill_pattern(&mut cmap, 0x8888_0001);
                let r1 = (api.png_image_finish_read)(
                    &mut im,
                    core::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    0,
                    cmap.as_mut_ptr() as *mut c_void,
                );
                snap(&mut o, "finish", &im, r1);
                o.output.extend_from_slice(&buf);
                o.output.extend_from_slice(&cmap);
                if r1 == 0 {
                    (api.png_image_free)(&mut im);
                    return o;
                }
                // Write what we just read straight back out.
                let entries2 = im.colormap_entries;
                let mut im2 = png_image::default();
                im2.version = PNG_IMAGE_VERSION;
                im2.width = w;
                im2.height = h;
                im2.format = *rfmt;
                im2.colormap_entries = entries2;
                let mut sz = 0usize;
                let cmp: *const c_void = if rfmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                    cmap.as_ptr() as *const c_void
                } else {
                    core::ptr::null()
                };
                let rq = (api.png_image_write_to_memory)(
                    &mut im2,
                    core::ptr::null_mut(),
                    &mut sz,
                    0,
                    buf.as_ptr() as *const c_void,
                    0,
                    cmp,
                );
                o.push(format!("requery ret={} size={}", rq, sz));
                snap(&mut o, "requery", &im2, rq);
                if rq != 0 && sz > 0 && sz < (1 << 22) {
                    let mut mem = vec![0u8; sz];
                    fill_pattern(&mut mem, 0x9999_0001);
                    let mut im3 = png_image::default();
                    im3.version = PNG_IMAGE_VERSION;
                    im3.width = w;
                    im3.height = h;
                    im3.format = *rfmt;
                    im3.colormap_entries = entries2;
                    let mut n = sz;
                    let r3 = (api.png_image_write_to_memory)(
                        &mut im3,
                        mem.as_mut_ptr() as *mut c_void,
                        &mut n,
                        0,
                        buf.as_ptr() as *const c_void,
                        0,
                        cmp,
                    );
                    o.push(format!("rewrite ret={} n={}", r3, n));
                    snap(&mut o, "rewrite", &im3, r3);
                    o.output.extend_from_slice(&mem);
                }
                (api.png_image_free)(&mut im);
                o
            });
        }
    }

    eprintln!(
        "simplified::round_trip: {} comparisons, {} successful reads ({} \
         with data), {} successful writes",
        cases_now() - t0,
        OK_READS.load(Relaxed),
        NONTRIVIAL.load(Relaxed),
        OK_WRITES.load(Relaxed)
    );
}

//! Simplified-API differential tests, CONFIGS.md rows S1..S7.
//!
//! The simplified API (`png_image_*`) brings its own `png_safe_execute`
//! (setjmp) landing pad and its own error/warning callbacks, so these tests
//! call the entry points directly: no `png_struct`, no harness error callback
//! and no longjmp pad of ours is involved.  Everything observable is written
//! into the caller's `png_image` and the caller's buffers, and all of that is
//! logged so the C and the Rust trace can be compared byte for byte.
#![allow(non_upper_case_globals)]
mod support;

use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr::{null, null_mut};
use support::core::*;
use support::pngbuild::Builder;
use support::*;

// ---------------------------------------------------------------------------
// entry points (signatures taken verbatim from c_src/include/png.h)
// ---------------------------------------------------------------------------

/// `png_color { png_byte red, green, blue; }` — the `background` argument of
/// `png_image_finish_read` (`png_const_colorp`).
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct PngColor {
    red: u8,
    green: u8,
    blue: u8,
}

/// `PNG_IMAGE_FLAG_*` (png.h); not part of `support::core`.
const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: u32 = 0x01;
const PNG_IMAGE_FLAG_FAST: u32 = 0x02;
const PNG_IMAGE_FLAG_16BIT_sRGB: u32 = 0x04;

struct Simp {
    /// `int png_image_begin_read_from_memory(png_imagep, png_const_voidp, size_t)`
    begin_mem: unsafe extern "C" fn(*mut PngImage, *const c_void, usize) -> c_int,
    /// `int png_image_begin_read_from_file(png_imagep, const char*)`
    begin_file: unsafe extern "C" fn(*mut PngImage, *const c_char) -> c_int,
    /// `int png_image_begin_read_from_stdio(png_imagep, FILE*)`
    begin_stdio: unsafe extern "C" fn(*mut PngImage, *mut c_void) -> c_int,
    /// `int png_image_finish_read(png_imagep, png_const_colorp, void*, png_int_32, void*)`
    finish: unsafe extern "C" fn(
        *mut PngImage,
        *const PngColor,
        *mut c_void,
        i32,
        *mut c_void,
    ) -> c_int,
    /// `void png_image_free(png_imagep)`
    free: unsafe extern "C" fn(*mut PngImage),
    /// `int png_image_write_to_memory(png_imagep, void*, png_alloc_size_t* restrict,
    ///  int, const void*, png_int_32, const void*)`
    wr_mem: unsafe extern "C" fn(
        *mut PngImage,
        *mut c_void,
        *mut usize,
        c_int,
        *const c_void,
        i32,
        *const c_void,
    ) -> c_int,
    /// `int png_image_write_to_file(png_imagep, const char*, int, const void*,
    ///  png_int_32, const void*)`
    wr_file: unsafe extern "C" fn(
        *mut PngImage,
        *const c_char,
        c_int,
        *const c_void,
        i32,
        *const c_void,
    ) -> c_int,
    /// `int png_image_write_to_stdio(png_imagep, FILE*, int, const void*,
    ///  png_int_32, const void*)`
    wr_stdio: unsafe extern "C" fn(
        *mut PngImage,
        *mut c_void,
        c_int,
        *const c_void,
        i32,
        *const c_void,
    ) -> c_int,
}

impl Simp {
    fn new(lib: &Lib) -> Simp {
        Simp {
            begin_mem: lib.f("png_image_begin_read_from_memory"),
            begin_file: lib.f("png_image_begin_read_from_file"),
            begin_stdio: lib.f("png_image_begin_read_from_stdio"),
            finish: lib.f("png_image_finish_read"),
            free: lib.f("png_image_free"),
            wr_mem: lib.f("png_image_write_to_memory"),
            wr_file: lib.f("png_image_write_to_file"),
            wr_stdio: lib.f("png_image_write_to_stdio"),
        }
    }
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
}

/// The reference C `libpng.so` is linked without `-lm`, so `floor`/`pow` stay
/// unresolved in it; the simplified read path calls them through the gamma
/// code.  Loading libm into the *global* symbol scope (RTLD_GLOBAL) makes the
/// lazy binding resolvable, identically for both libraries.
fn ensure_libm() {
    use std::sync::OnceLock;
    static LIBM: OnceLock<libloading::os::unix::Library> = OnceLock::new();
    LIBM.get_or_init(|| unsafe {
        libloading::os::unix::Library::open(Some("libm.so.6"), 0x2 | 0x100)
            .expect("dlopen libm.so.6")
    });
}

// ---------------------------------------------------------------------------
// the PNG_IMAGE_* macros of png.h, as Rust functions
// ---------------------------------------------------------------------------

/// `PNG_IMAGE_SAMPLE_CHANNELS(fmt)`
fn sample_channels(fmt: u32) -> usize {
    ((fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1) as usize
}

/// `PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt)`
fn sample_component_size(fmt: u32) -> usize {
    (((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1) as usize
}

/// `PNG_IMAGE_SAMPLE_SIZE(fmt)`
fn sample_size(fmt: u32) -> usize {
    sample_channels(fmt) * sample_component_size(fmt)
}

/// `PNG_IMAGE_PIXEL_CHANNELS(fmt)`
fn pixel_channels(fmt: u32) -> usize {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_channels(fmt)
    }
}

/// `PNG_IMAGE_PIXEL_COMPONENT_SIZE(fmt)`
fn pixel_component_size(fmt: u32) -> usize {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_component_size(fmt)
    }
}

/// `PNG_IMAGE_PIXEL_SIZE(fmt)`
#[allow(dead_code)]
fn pixel_size(fmt: u32) -> usize {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_size(fmt)
    }
}

/// `PNG_IMAGE_ROW_STRIDE(image)` — in *components*, not bytes.
fn row_stride(im: &PngImage) -> usize {
    pixel_channels(im.format) * im.width as usize
}

/// `PNG_IMAGE_BUFFER_SIZE(image, row_stride)` — in bytes.
fn buffer_size(im: &PngImage, stride: usize) -> usize {
    pixel_component_size(im.format) * im.height as usize * stride
}

/// `PNG_IMAGE_SIZE(image)`
#[allow(dead_code)]
fn image_size(im: &PngImage) -> usize {
    buffer_size(im, row_stride(im))
}

/// `PNG_IMAGE_COLORMAP_SIZE(image)`
fn colormap_size(im: &PngImage) -> usize {
    sample_size(im.format) * im.colormap_entries as usize
}

/// Bytes of slack allocated (and logged separately) after every buffer handed
/// to libpng, so that an overrun in either library shows up in the trace.
const SLACK: usize = 64;

// ---------------------------------------------------------------------------
// formats
// ---------------------------------------------------------------------------

/// The nine 8-bit-per-component output formats (S1).
const FMT_SRGB: &[(u32, &str)] = &[
    (PNG_FORMAT_GRAY, "GRAY"),
    (PNG_FORMAT_GA, "GA"),
    (PNG_FORMAT_AG, "AG"),
    (PNG_FORMAT_RGB, "RGB"),
    (PNG_FORMAT_BGR, "BGR"),
    (PNG_FORMAT_RGBA, "RGBA"),
    (PNG_FORMAT_ARGB, "ARGB"),
    (PNG_FORMAT_BGRA, "BGRA"),
    (PNG_FORMAT_ABGR, "ABGR"),
];

/// The four linear (2-byte component) output formats (S2).
const FMT_LINEAR: &[(u32, &str)] = &[
    (PNG_FORMAT_LINEAR_Y, "LINEAR_Y"),
    (PNG_FORMAT_LINEAR_Y_ALPHA, "LINEAR_Y_ALPHA"),
    (PNG_FORMAT_LINEAR_RGB, "LINEAR_RGB"),
    (PNG_FORMAT_LINEAR_RGB_ALPHA, "LINEAR_RGB_ALPHA"),
];

/// The six colour-mapped output formats (S3).
const FMT_CMAP: &[(u32, &str)] = &[
    (PNG_FORMAT_RGB_COLORMAP, "RGB_COLORMAP"),
    (PNG_FORMAT_BGR_COLORMAP, "BGR_COLORMAP"),
    (PNG_FORMAT_RGBA_COLORMAP, "RGBA_COLORMAP"),
    (PNG_FORMAT_ARGB_COLORMAP, "ARGB_COLORMAP"),
    (PNG_FORMAT_BGRA_COLORMAP, "BGRA_COLORMAP"),
    (PNG_FORMAT_ABGR_COLORMAP, "ABGR_COLORMAP"),
];

/// Output formats without an alpha channel: these are the ones for which the
/// `background` argument of `png_image_finish_read` is actually used (S4).
const FMT_NOALPHA: &[(u32, &str)] = &[
    (PNG_FORMAT_GRAY, "GRAY"),
    (PNG_FORMAT_RGB, "RGB"),
    (PNG_FORMAT_BGR, "BGR"),
    (PNG_FORMAT_LINEAR_Y, "LINEAR_Y"),
    (PNG_FORMAT_LINEAR_RGB, "LINEAR_RGB"),
    (PNG_FORMAT_RGB_COLORMAP, "RGB_COLORMAP"),
    (PNG_FORMAT_BGR_COLORMAP, "BGR_COLORMAP"),
];

// ---------------------------------------------------------------------------
// input construction (independent of both libraries)
// ---------------------------------------------------------------------------

/// The 15 legal (colour_type, bit_depth) combinations.
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

fn palette_for(bd: u8, seed: u64) -> Vec<u8> {
    let n = 1usize << bd; // every index of a `bd`-bit image is in range
    let mut r = Rng::new(seed);
    (0..3 * n).map(|_| r.byte()).collect()
}

fn trns_for(ct: u8, bd: u8, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    match ct {
        0 => {
            let m: u32 = if bd >= 16 { 0xffff } else { (1u32 << bd) - 1 };
            ((r.next_u32() % (m + 1)) as u16).to_be_bytes().to_vec()
        }
        2 => {
            let m: u32 = if bd >= 16 { 0xffff } else { 0xff };
            let mut v = Vec::new();
            for _ in 0..3 {
                v.extend_from_slice(&((r.next_u32() % (m + 1)) as u16).to_be_bytes());
            }
            v
        }
        3 => {
            let n = 1usize << bd;
            r.bytes(n)
        }
        // Illegal for GRAY_ALPHA/RGB_ALPHA: libpng reports a benign error,
        // which the simplified API turns into a warning.
        _ => vec![0x12, 0x34],
    }
}

/// A valid PNG datastream.  `trns` adds a tRNS chunk, `gama` a gAMA chunk
/// (0.45455 = the usual file gamma, i.e. *not* the sRGB default).
fn mk(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64, trns: bool, gama: bool) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct).interlace(il);
    if gama {
        b = b.add(b"gAMA", 45455u32.to_be_bytes().to_vec());
    }
    if ct == 3 {
        b = b.add(b"PLTE", palette_for(bd, seed ^ 0x5eed_1234));
    }
    if trns {
        b = b.add(b"tRNS", trns_for(ct, bd, seed ^ 0x7a17_9999));
    }
    b.build_valid(seed)
}

/// Does the *input* PNG (as reported by `png_image_format`) carry an alpha
/// channel?  A tRNS chunk counts, but only for the colour types where it is
/// legal.
fn input_has_alpha(ct: u8, trns: bool) -> bool {
    ct == 4 || ct == 6 || (trns && (ct == 0 || ct == 2 || ct == 3))
}

/// png.h, `png_image_finish_read`: "background must be supplied when an alpha
/// channel must be removed from a single byte color-mapped output format".
fn needs_bg(fmt: u32, ct: u8, trns: bool) -> bool {
    fmt & PNG_FORMAT_FLAG_COLORMAP != 0
        && fmt & PNG_FORMAT_FLAG_LINEAR == 0
        && fmt & PNG_FORMAT_FLAG_ALPHA == 0
        && input_has_alpha(ct, trns)
}

/// A fixed background colour used wherever the contract demands one.
const BG: PngColor = PngColor {
    red: 0x20,
    green: 0x40,
    blue: 0x60,
};

// --- colour-space chunks: the `png_image_is_not_sRGB` decision -------------

/// cHRM payload order: white x,y; red x,y; green x,y; blue x,y (x100000).
const CHRM_SRGB: [u32; 8] = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];
const CHRM_ODD: [u32; 8] = [20000, 25000, 50000, 20000, 20000, 50000, 10000, 3000];

fn chrm_bytes(v: &[u32; 8]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_be_bytes()).collect()
}

/// mDCV payload order: red x,y; green x,y; blue x,y; white x,y (x50000, 2
/// bytes each), then peak and minimum luminance (4 bytes each).
const MDCV_SRGB: [u16; 8] = [32000, 16500, 15000, 30000, 7500, 3000, 15635, 16450];
const MDCV_ODD: [u16; 8] = [25000, 10000, 10000, 25000, 5000, 1500, 10000, 12500];

fn mdcv_bytes(v: &[u16; 8]) -> Vec<u8> {
    let mut d: Vec<u8> = v.iter().flat_map(|x| x.to_be_bytes()).collect();
    d.extend_from_slice(&1_000_000u32.to_be_bytes());
    d.extend_from_slice(&500u32.to_be_bytes());
    d
}

/// The colour-space chunk variants exercised on the read side.
const CS: &[(&str, u8)] = &[
    ("none", 0),
    ("sRGB", 1),
    ("cHRM_sRGB", 2),
    ("cHRM_odd", 3),
    ("cICP", 4),
    ("mDCV_sRGB", 5),
    ("mDCV_odd", 6),
    ("gAMA_linear", 7),
];

/// A valid PNG carrying one colour-space chunk (before PLTE, as required).
fn mk_cs(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64, cs: u8) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct).interlace(il);
    b = match cs {
        1 => b.add(b"sRGB", vec![0]),
        2 => b.add(b"cHRM", chrm_bytes(&CHRM_SRGB)),
        3 => b.add(b"cHRM", chrm_bytes(&CHRM_ODD)),
        4 => b.add(b"cICP", vec![1, 13, 0, 1]),
        5 => b.add(b"mDCV", mdcv_bytes(&MDCV_SRGB)),
        6 => b.add(b"mDCV", mdcv_bytes(&MDCV_ODD)),
        7 => b.add(b"gAMA", 100_000u32.to_be_bytes().to_vec()),
        _ => b,
    };
    if ct == 3 {
        b = b.add(b"PLTE", palette_for(bd, seed ^ 0x5eed_1234));
    }
    b.build_valid(seed)
}

// ---------------------------------------------------------------------------
// logging
// ---------------------------------------------------------------------------

/// Log everything observable in a `png_image`.  The `opaque` pointer is only
/// reported as NULL / non-NULL: its value legitimately differs between the two
/// libraries.
fn log_img(tag: &str, im: &PngImage) {
    log(format!(
        "{tag} opaque_null={} version={} w={} h={} fmt={:#x} flags={:#x} cmap_entries={} woe={} msg={:?}",
        im.opaque.is_null() as u8,
        im.version,
        im.width,
        im.height,
        im.format,
        im.flags,
        im.colormap_entries,
        im.warning_or_error,
        im.msg()
    ));
}

fn trace() -> Trace {
    Trace {
        lines: take_log(),
        out: take_out(),
        rc: 0,
    }
}

// ---------------------------------------------------------------------------
// read driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Rd {
    /// output format requested by the application
    fmt: u32,
    /// `background` argument of `png_image_finish_read`
    bg: Option<PngColor>,
    /// use a negative `row_stride` (bottom-up image)
    negative: bool,
    /// extra components of row stride (row padding)
    pad: usize,
    /// OR these bits into `image.flags` after `begin_read`
    set_flags: u32,
}

impl Default for Rd {
    fn default() -> Self {
        Rd {
            fmt: PNG_FORMAT_RGBA,
            bg: None,
            negative: false,
            pad: 0,
            set_flags: 0,
        }
    }
}

fn read_case(label: &str, png: &[u8], o: Rd) {
    diff(label, |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        // `image` must keep a stable address for as long as libpng holds it
        // (image.opaque points at internal state that points back here).
        let mut image = PngImage::default();
        let bg = o.bg;
        unsafe {
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_mem)(ip, png.as_ptr() as *const c_void, png.len());
            log(format!("begin rc={rc}"));
            log_img("begin", &image);
            if rc != 0 {
                image.flags |= o.set_flags;
                image.format = o.fmt;
                let stride = row_stride(&image) + o.pad;
                let need = buffer_size(&image, stride);
                let cmneed = if o.fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                    colormap_size(&image)
                } else {
                    0
                };
                let mut buf = vec![0xa5u8; need + SLACK];
                let mut cmap = vec![0x5au8; cmneed + SLACK];
                let cmptr = if o.fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                    cmap.as_mut_ptr() as *mut c_void
                } else {
                    null_mut()
                };
                let rs: i32 = if o.negative {
                    -(stride as i32)
                } else {
                    stride as i32
                };
                let bgp: *const PngColor = match &bg {
                    Some(c) => c as *const PngColor,
                    None => null(),
                };
                log(format!(
                    "finish_args stride={rs} buf_bytes={need} cmap_bytes={cmneed} bg={}",
                    match &bg {
                        Some(c) => format!("{},{},{}", c.red, c.green, c.blue),
                        None => "<null>".to_string(),
                    }
                ));
                let rc2 = (s.finish)(ip, bgp, buf.as_mut_ptr() as *mut c_void, rs, cmptr);
                log(format!("finish rc={rc2}"));
                log_img("finish", &image);
                log(format!("buf={}", hex(&buf[..need])));
                log(format!("buf_slack={}", hex(&buf[need..])));
                if cmneed > 0 {
                    log(format!("cmap={}", hex(&cmap[..cmneed])));
                    log(format!("cmap_slack={}", hex(&cmap[cmneed..])));
                }
            }
            (s.free)(ip);
            log(format!("freed opaque_null={}", image.opaque.is_null() as u8));
        }
        trace()
    });
}

// ---------------------------------------------------------------------------
// S1 — begin_read_from_memory + finish_read, the sRGB output formats
// ---------------------------------------------------------------------------

/// Image shapes used throughout: byte-aligned and non-aligned widths at every
/// bit depth, plus the degenerate 1x1 case.
const SHAPES: &[(u32, u32)] = &[(1, 1), (3, 2), (7, 4), (8, 5), (17, 3)];

#[test]
fn s1_read_srgb_formats() {
    ensure_libm();
    // Every legal input (colour type, bit depth) x interlace x shape x output
    // format.
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for &(w, h) in SHAPES {
                let png = mk(
                    w,
                    h,
                    ct,
                    bd,
                    il,
                    0x5100 + (ct as u64) * 97 + (bd as u64) * 13 + w as u64 * 3 + h as u64,
                    false,
                    false,
                );
                for &(fmt, fname) in FMT_SRGB {
                    read_case(
                        &format!("S1 ct={ct} bd={bd} il={il} {w}x{h} out={fname}"),
                        &png,
                        Rd {
                            fmt,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
    // Inputs carrying tRNS and/or gAMA, both interlace types, two seeds.
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for &sx in &[0u64, 0x9e37_79b9] {
                for &(vname, trns, gama) in VARIANTS {
                    let png = mk(
                        5,
                        3,
                        ct,
                        bd,
                        il,
                        0x5200 + (ct as u64) * 31 + bd as u64 + sx,
                        trns,
                        gama,
                    );
                    for &(fmt, fname) in FMT_SRGB {
                        read_case(
                            &format!("S1b ct={ct} bd={bd} il={il} s={sx} {vname} out={fname}"),
                            &png,
                            Rd {
                                fmt,
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
    }
    // Row-padded strides.
    for &(ct, bd) in COMBOS {
        let png = mk(3, 2, ct, bd, 0, 0x5400 + ct as u64, true, true);
        for &pad in &[1usize, 5, 13] {
            for &(fmt, fname) in FMT_SRGB {
                read_case(
                    &format!("S1c ct={ct} bd={bd} out={fname} pad={pad}"),
                    &png,
                    Rd {
                        fmt,
                        pad,
                        ..Default::default()
                    },
                );
            }
        }
    }
    // Colour-space chunks: these drive png_image_is_not_sRGB and therefore
    // PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB plus the gamma/rgb-to-gray setup.
    for &(csname, cs) in CS {
        for &(ct, bd) in COMBOS {
            let png = mk_cs(5, 3, ct, bd, 0, 0x5500 + (ct as u64) * 71 + bd as u64, cs);
            for &(fmt, fname) in FMT_SRGB {
                read_case(
                    &format!("S1d cs={csname} ct={ct} bd={bd} out={fname}"),
                    &png,
                    Rd {
                        fmt,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// S2 — the LINEAR output formats
// ---------------------------------------------------------------------------

#[test]
fn s2_read_linear_formats() {
    ensure_libm();
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for &(w, h) in SHAPES {
                let seed =
                    0x5500 + (ct as u64) * 41 + bd as u64 + il as u64 + w as u64 * 5 + h as u64;
                let png = mk(w, h, ct, bd, il, seed, false, false);
                for &(fmt, fname) in FMT_LINEAR {
                    read_case(
                        &format!("S2 ct={ct} bd={bd} il={il} {w}x{h} out={fname}"),
                        &png,
                        Rd {
                            fmt,
                            ..Default::default()
                        },
                    );
                }
            }
            for &(vname, trns, gama) in VARIANTS {
                let png = mk(
                    5,
                    3,
                    ct,
                    bd,
                    il,
                    0x5580 + (ct as u64) * 43 + bd as u64 + trns as u64,
                    trns,
                    gama,
                );
                for &(fmt, fname) in FMT_LINEAR {
                    read_case(
                        &format!("S2v ct={ct} bd={bd} il={il} {vname} out={fname}"),
                        &png,
                        Rd {
                            fmt,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
    // PNG_IMAGE_FLAG_16BIT_sRGB changes the interpretation of 16-bit input.
    for &(ct, bd) in COMBOS {
        let png = mk(4, 3, ct, bd, 0, 0x5600 + (ct as u64) * 7 + bd as u64, false, false);
        for &(fmt, fname) in FMT_LINEAR {
            read_case(
                &format!("S2f ct={ct} bd={bd} out={fname} flag16"),
                &png,
                Rd {
                    fmt,
                    set_flags: PNG_IMAGE_FLAG_16BIT_sRGB,
                    ..Default::default()
                },
            );
        }
        for &(fmt, fname) in FMT_SRGB {
            read_case(
                &format!("S2f ct={ct} bd={bd} out={fname} flag16"),
                &png,
                Rd {
                    fmt,
                    set_flags: PNG_IMAGE_FLAG_16BIT_sRGB,
                    ..Default::default()
                },
            );
        }
    }
    // Colour-space chunks with the linear output formats.
    for &(csname, cs) in CS {
        for &(ct, bd) in COMBOS {
            let png = mk_cs(5, 3, ct, bd, 0, 0x5680 + (ct as u64) * 73 + bd as u64, cs);
            for &(fmt, fname) in FMT_LINEAR {
                read_case(
                    &format!("S2c cs={csname} ct={ct} bd={bd} out={fname}"),
                    &png,
                    Rd {
                        fmt,
                        ..Default::default()
                    },
                );
                read_case(
                    &format!("S2c cs={csname} ct={ct} bd={bd} out={fname} flag16"),
                    &png,
                    Rd {
                        fmt,
                        set_flags: PNG_IMAGE_FLAG_16BIT_sRGB,
                        ..Default::default()
                    },
                );
            }
        }
    }
    // padded linear reads (the stride is counted in 2-byte components)
    for &(ct, bd) in COMBOS {
        let png = mk(3, 2, ct, bd, 0, 0x5700 + (ct as u64) * 13 + bd as u64, true, false);
        for &pad in &[1usize, 3, 8] {
            for &(fmt, fname) in FMT_LINEAR {
                read_case(
                    &format!("S2p ct={ct} bd={bd} out={fname} pad={pad}"),
                    &png,
                    Rd {
                        fmt,
                        pad,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// S3 — the COLORMAP output formats
// ---------------------------------------------------------------------------

/// The four input variants used by S1..S4: plain, tRNS, gAMA, both.
const VARIANTS: &[(&str, bool, bool)] = &[
    ("plain", false, false),
    ("tRNS", true, false),
    ("gAMA", false, true),
    ("tRNS+gAMA", true, true),
];

#[test]
fn s3_read_colormap_formats() {
    ensure_libm();
    // --- the documented contract: a background is supplied whenever an alpha
    // channel has to be dropped for a single-byte colour-mapped format -----
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for &(w, h) in SHAPES {
                let seed = 0x5800 + (ct as u64) * 53 + bd as u64 + il as u64 + w as u64 * 3;
                let png = mk(w, h, ct, bd, il, seed, false, false);
                for &(fmt, fname) in FMT_CMAP {
                    read_case(
                        &format!("S3 ct={ct} bd={bd} il={il} {w}x{h} out={fname}"),
                        &png,
                        Rd {
                            fmt,
                            bg: if needs_bg(fmt, ct, false) { Some(BG) } else { None },
                            ..Default::default()
                        },
                    );
                }
            }
            for &(vname, trns, gama) in VARIANTS {
                let seed = 0x5880 + (ct as u64) * 59 + bd as u64 + il as u64 + trns as u64 * 3;
                let png = mk(7, 4, ct, bd, il, seed, trns, gama);
                for &(fmt, fname) in FMT_CMAP {
                    let bg = if needs_bg(fmt, ct, trns) { Some(BG) } else { None };
                    read_case(
                        &format!("S3v ct={ct} bd={bd} il={il} {vname} out={fname}"),
                        &png,
                        Rd {
                            fmt,
                            bg,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
    // 1x1, a padded stride and an explicit background even where the contract
    // does not require one.
    for &(ct, bd) in COMBOS {
        let one = mk(1, 1, ct, bd, 0, 0x5900 + ct as u64, false, false);
        let png = mk(4, 3, ct, bd, 0, 0x5a00 + (ct as u64) * 11 + bd as u64, true, false);
        for &(fmt, fname) in FMT_CMAP {
            read_case(
                &format!("S3b ct={ct} bd={bd} out={fname} 1x1"),
                &one,
                Rd {
                    fmt,
                    bg: if needs_bg(fmt, ct, false) { Some(BG) } else { None },
                    ..Default::default()
                },
            );
            read_case(
                &format!("S3b ct={ct} bd={bd} out={fname} pad=4"),
                &png,
                Rd {
                    fmt,
                    pad: 4,
                    bg: if needs_bg(fmt, ct, true) { Some(BG) } else { None },
                    ..Default::default()
                },
            );
            read_case(
                &format!("S3b ct={ct} bd={bd} out={fname} bg"),
                &png,
                Rd {
                    fmt,
                    bg: Some(BG),
                    ..Default::default()
                },
            );
        }
    }
    // Colour-space chunks with the colour-mapped output formats.
    for &(csname, cs) in CS {
        for &(ct, bd) in COMBOS {
            let png = mk_cs(5, 3, ct, bd, 0, 0x5980 + (ct as u64) * 79 + bd as u64, cs);
            for &(fmt, fname) in FMT_CMAP {
                read_case(
                    &format!("S3c cs={csname} ct={ct} bd={bd} out={fname}"),
                    &png,
                    Rd {
                        fmt,
                        bg: if needs_bg(fmt, ct, false) { Some(BG) } else { None },
                        ..Default::default()
                    },
                );
            }
        }
    }
    // --- error path, deliberately last: the *required* background withheld.
    // png.h mandates a background for a single-byte colour-mapped output
    // format when the input carries alpha, so libpng must fail here.
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for &(vname, trns, gama) in VARIANTS {
                if !input_has_alpha(ct, trns) {
                    continue;
                }
                let png = mk(5, 3, ct, bd, il, 0x5b80 + (ct as u64) * 17 + bd as u64, trns, gama);
                for &(fmt, fname) in FMT_CMAP {
                    if fmt & PNG_FORMAT_FLAG_ALPHA != 0 {
                        continue;
                    }
                    read_case(
                        &format!("S3e ct={ct} bd={bd} il={il} {vname} out={fname} nobg"),
                        &png,
                        Rd {
                            fmt,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// S4 — background compositing and negative (bottom-up) row strides
// ---------------------------------------------------------------------------

#[test]
fn s4_background_and_negative_stride() {
    ensure_libm();
    let backgrounds: &[PngColor] = &[
        PngColor {
            red: 0,
            green: 0,
            blue: 0,
        },
        PngColor {
            red: 255,
            green: 255,
            blue: 255,
        },
        PngColor {
            red: 0x12,
            green: 0x34,
            blue: 0x56,
        },
        PngColor {
            red: 0xff,
            green: 0x00,
            blue: 0x80,
        },
    ];
    // alpha-bearing inputs and tRNS-bearing inputs: both make the simplified
    // reader composite when the requested output format has no alpha channel.
    let inputs: &[(u8, u8, bool, &str)] = &[
        (4, 8, false, "GA8"),
        (4, 16, false, "GA16"),
        (6, 8, false, "RGBA8"),
        (6, 16, false, "RGBA16"),
        (0, 8, true, "G8+tRNS"),
        (0, 16, true, "G16+tRNS"),
        (2, 8, true, "RGB8+tRNS"),
        (2, 16, true, "RGB16+tRNS"),
        (3, 4, true, "P4+tRNS"),
        (3, 8, true, "P8+tRNS"),
    ];
    for &(ct, bd, trns, iname) in inputs {
        let png = mk(5, 3, ct, bd, 0, 0x5b00 + (ct as u64) * 61 + bd as u64, trns, false);
        for &(fmt, fname) in FMT_NOALPHA {
            for (bi, bg) in backgrounds.iter().enumerate() {
                read_case(
                    &format!("S4 in={iname} out={fname} bg{bi}"),
                    &png,
                    Rd {
                        fmt,
                        bg: Some(*bg),
                        ..Default::default()
                    },
                );
            }
        }
        // A background is also accepted (and ignored) for formats that keep
        // the alpha channel.
        for &(fmt, fname) in &[
            (PNG_FORMAT_RGBA, "RGBA"),
            (PNG_FORMAT_ARGB, "ARGB"),
            (PNG_FORMAT_LINEAR_RGB_ALPHA, "LINEAR_RGB_ALPHA"),
            (PNG_FORMAT_RGBA_COLORMAP, "RGBA_COLORMAP"),
        ] {
            read_case(
                &format!("S4a in={iname} out={fname} bg2"),
                &png,
                Rd {
                    fmt,
                    bg: Some(backgrounds[2]),
                    ..Default::default()
                },
            );
        }
    }
    // Negative row_stride (bottom-up image) for every kind of output format.
    let neg_fmts: &[(u32, &str)] = &[
        (PNG_FORMAT_GRAY, "GRAY"),
        (PNG_FORMAT_GA, "GA"),
        (PNG_FORMAT_AG, "AG"),
        (PNG_FORMAT_RGB, "RGB"),
        (PNG_FORMAT_BGR, "BGR"),
        (PNG_FORMAT_RGBA, "RGBA"),
        (PNG_FORMAT_ABGR, "ABGR"),
        (PNG_FORMAT_LINEAR_Y, "LINEAR_Y"),
        (PNG_FORMAT_LINEAR_RGB, "LINEAR_RGB"),
        (PNG_FORMAT_LINEAR_RGB_ALPHA, "LINEAR_RGB_ALPHA"),
        (PNG_FORMAT_RGB_COLORMAP, "RGB_COLORMAP"),
        (PNG_FORMAT_RGBA_COLORMAP, "RGBA_COLORMAP"),
    ];
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for &(vname, trns, gama) in VARIANTS {
                let png = mk(
                    4,
                    3,
                    ct,
                    bd,
                    il,
                    0x5c00 + (ct as u64) * 67 + bd as u64 + trns as u64,
                    trns,
                    gama,
                );
                for &(fmt, fname) in neg_fmts {
                    read_case(
                        &format!("S4n ct={ct} bd={bd} il={il} {vname} out={fname} neg"),
                        &png,
                        Rd {
                            fmt,
                            negative: true,
                            bg: if needs_bg(fmt, ct, trns) { Some(BG) } else { None },
                            ..Default::default()
                        },
                    );
                    read_case(
                        &format!("S4n ct={ct} bd={bd} il={il} {vname} out={fname} neg+bg"),
                        &png,
                        Rd {
                            fmt,
                            negative: true,
                            bg: Some(backgrounds[2]),
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
    // negative stride with row padding
    for &(fmt, fname) in neg_fmts {
        let png = mk(3, 4, 6, 8, 0, 0x5d00, false, false);
        read_case(
            &format!("S4np out={fname} neg pad=6"),
            &png,
            Rd {
                fmt,
                negative: true,
                pad: 6,
                bg: Some(backgrounds[1]),
                ..Default::default()
            },
        );
    }
    // --- error path, deliberately last: a colour-mapped 8-bit output format
    // that must drop an alpha channel *requires* a background; without one the
    // reader must fail.
    for &(ct, bd) in COMBOS {
        for trns in [false, true] {
            if !input_has_alpha(ct, trns) {
                continue;
            }
            let png = mk(4, 3, ct, bd, 0, 0x5e00 + (ct as u64) * 19 + bd as u64, trns, false);
            for &(fmt, fname) in &[
                (PNG_FORMAT_RGB_COLORMAP, "RGB_COLORMAP"),
                (PNG_FORMAT_BGR_COLORMAP, "BGR_COLORMAP"),
            ] {
                read_case(
                    &format!("S4e ct={ct} bd={bd} trns={trns} out={fname} nobg"),
                    &png,
                    Rd {
                        fmt,
                        ..Default::default()
                    },
                );
                read_case(
                    &format!("S4e ct={ct} bd={bd} trns={trns} out={fname} nobg neg"),
                    &png,
                    Rd {
                        fmt,
                        negative: true,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// write driver (S5)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Wr {
    fmt: u32,
    w: u32,
    h: u32,
    entries: u32,
    convert8: c_int,
    /// `row_stride` argument; 0 means "let libpng work it out"
    stride: i32,
    flags: u32,
    seed: u64,
}

impl Default for Wr {
    fn default() -> Self {
        Wr {
            fmt: PNG_FORMAT_RGB,
            w: 6,
            h: 4,
            entries: 0,
            convert8: 0,
            stride: 0,
            flags: 0,
            seed: 0x9000,
        }
    }
}

impl Wr {
    fn mk_image(&self) -> PngImage {
        PngImage {
            width: self.w,
            height: self.h,
            format: self.fmt,
            flags: self.flags,
            colormap_entries: self.entries,
            ..Default::default()
        }
    }

    /// Number of components between rows, as libpng will compute it.
    fn stride_components(&self) -> usize {
        if self.stride == 0 {
            pixel_channels(self.fmt) * self.w as usize
        } else {
            self.stride.unsigned_abs() as usize
        }
    }

    /// The caller's image buffer, built here with a fixed seed so both
    /// libraries get identical input.
    fn pixels(&self) -> Vec<u8> {
        let n = pixel_component_size(self.fmt) * self.h as usize * self.stride_components();
        let mut r = Rng::new(self.seed);
        let mut v = vec![0u8; n];
        if self.fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
            let e = self.entries.max(1);
            for b in v.iter_mut() {
                *b = (r.byte() as u32 % e) as u8;
            }
        } else {
            for b in v.iter_mut() {
                *b = r.byte();
            }
        }
        v
    }

    /// The caller's colour-map, `colormap_entries` samples in `fmt`.
    fn colormap(&self) -> Vec<u8> {
        if self.fmt & PNG_FORMAT_FLAG_COLORMAP == 0 {
            return Vec::new();
        }
        let n = sample_size(self.fmt) * self.entries as usize;
        let mut r = Rng::new(self.seed ^ 0xc0ffee);
        (0..n).map(|_| r.byte()).collect()
    }
}

/// `png_image_write_to_memory`: size query, real write, deliberately
/// too-small buffer.
fn write_mem_case(label: &str, o: Wr) {
    let pixels = o.pixels();
    let cmap = o.colormap();
    diff(label, |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        let mut image = o.mk_image();
        let bufp = pixels.as_ptr() as *const c_void;
        let cmp: *const c_void = if cmap.is_empty() {
            null()
        } else {
            cmap.as_ptr() as *const c_void
        };
        unsafe {
            let ip: *mut PngImage = &mut image;
            log(format!(
                "in fmt={:#x} w={} h={} entries={} convert8={} stride={} pixels={} cmap={}",
                o.fmt,
                o.w,
                o.h,
                o.entries,
                o.convert8,
                o.stride,
                pixels.len(),
                cmap.len()
            ));
            // --- pass 1: memory == NULL, just ask for the size -------------
            let mut sz: usize = 0;
            let rc1 = (s.wr_mem)(ip, null_mut(), &mut sz, o.convert8, bufp, o.stride, cmp);
            log(format!("query rc={rc1} memory_bytes={sz}"));
            log_img("query", &image);
            if rc1 == 0 {
                (s.free)(ip);
                log(format!("freed opaque_null={}", image.opaque.is_null() as u8));
                return trace();
            }
            // --- pass 2: exactly the queried size -------------------------
            let mut mem = vec![0xc3u8; sz + SLACK];
            let mut sz2 = sz;
            let rc2 = (s.wr_mem)(
                ip,
                mem.as_mut_ptr() as *mut c_void,
                &mut sz2,
                o.convert8,
                bufp,
                o.stride,
                cmp,
            );
            log(format!("write rc={rc2} memory_bytes={sz2}"));
            log_img("write", &image);
            log(format!("png={}", hex(&mem[..sz2.min(sz)])));
            log(format!("png_slack={}", hex(&mem[sz..])));
            // --- pass 3: a deliberately too-small buffer -------------------
            let small = sz / 2;
            let mut mem2 = vec![0x7eu8; small + SLACK];
            let mut sz3 = small;
            let rc3 = (s.wr_mem)(
                ip,
                mem2.as_mut_ptr() as *mut c_void,
                &mut sz3,
                o.convert8,
                bufp,
                o.stride,
                cmp,
            );
            log(format!("small rc={rc3} given={small} memory_bytes={sz3}"));
            log_img("small", &image);
            log(format!("small_buf={}", hex(&mem2[..small])));
            log(format!("small_slack={}", hex(&mem2[small..])));
            // --- pass 4: zero-sized buffer --------------------------------
            let mut mem3 = vec![0x11u8; SLACK];
            let mut sz4: usize = 0;
            let rc4 = (s.wr_mem)(
                ip,
                mem3.as_mut_ptr() as *mut c_void,
                &mut sz4,
                o.convert8,
                bufp,
                o.stride,
                cmp,
            );
            log(format!("zero rc={rc4} memory_bytes={sz4}"));
            log_img("zero", &image);
            log(format!("zero_slack={}", hex(&mem3)));
            (s.free)(ip);
            log(format!("freed opaque_null={}", image.opaque.is_null() as u8));
        }
        trace()
    });
}

// ---------------------------------------------------------------------------
// S5 — png_image_write_to_memory
// ---------------------------------------------------------------------------

#[test]
fn s5_write_to_memory() {
    ensure_libm();
    // every non-colour-mapped format x convert_to_8_bit 0/1 x shape
    for &(fmt, fname) in FMT_SRGB.iter().chain(FMT_LINEAR.iter()) {
        for c8 in [0 as c_int, 1] {
            for &(w, h) in &[(6u32, 4u32), (1, 1), (3, 2), (16, 8)] {
                write_mem_case(
                    &format!("S5 fmt={fname} convert8={c8} {w}x{h}"),
                    Wr {
                        fmt,
                        w,
                        h,
                        convert8: c8,
                        seed: 0x9100 + fmt as u64 + w as u64 * 7 + h as u64,
                        ..Default::default()
                    },
                );
            }
        }
    }
    // colour-mapped formats: the written PNG bit depth follows the entry count
    // (>16 -> 8, >4 -> 4, >2 -> 2, else 1).
    for &(fmt, fname) in FMT_CMAP {
        for entries in [1u32, 2, 3, 4, 5, 16, 17, 200, 256] {
            for c8 in [0 as c_int, 1] {
                for &(w, h) in &[(6u32, 4u32), (1, 1)] {
                    write_mem_case(
                        &format!("S5c fmt={fname} entries={entries} convert8={c8} {w}x{h}"),
                        Wr {
                            fmt,
                            w,
                            h,
                            entries,
                            convert8: c8,
                            seed: 0x9300 + fmt as u64 + entries as u64 * 7 + w as u64,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
    // explicit / padded / negative row strides, for every format
    for &(fmt, fname) in FMT_SRGB
        .iter()
        .chain(FMT_LINEAR.iter())
        .chain(FMT_CMAP.iter())
    {
        let entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 9 } else { 0 };
        let exact = (pixel_channels(fmt) * 6) as i32;
        for &st in &[exact, exact + 5, -exact, -(exact + 3)] {
            write_mem_case(
                &format!("S5s fmt={fname} stride={st}"),
                Wr {
                    fmt,
                    entries,
                    stride: st,
                    seed: 0x9400 + fmt as u64 + st.unsigned_abs() as u64,
                    ..Default::default()
                },
            );
        }
    }
    // PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB and PNG_IMAGE_FLAG_FAST
    for &(fmt, fname) in &[
        (PNG_FORMAT_RGB, "RGB"),
        (PNG_FORMAT_RGBA, "RGBA"),
        (PNG_FORMAT_LINEAR_RGB, "LINEAR_RGB"),
    ] {
        for &(fl, flname) in &[
            (PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB, "NOT_sRGB"),
            (PNG_IMAGE_FLAG_FAST, "FAST"),
            (
                PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB | PNG_IMAGE_FLAG_FAST,
                "NOT_sRGB|FAST",
            ),
        ] {
            for c8 in [0 as c_int, 1] {
                write_mem_case(
                    &format!("S5f fmt={fname} flags={flname} convert8={c8}"),
                    Wr {
                        fmt,
                        flags: fl,
                        convert8: c8,
                        seed: 0x9600 + fmt as u64 + fl as u64,
                        ..Default::default()
                    },
                );
            }
        }
    }
    // Argument validation: buffer == NULL, memory_bytes == NULL, bad version.
    diff("S5arg", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        let px = [0u8; 64];
        unsafe {
            let mut image = PngImage {
                width: 4,
                height: 2,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let mut sz: usize = 0;
            let rc = (s.wr_mem)(ip, null_mut(), &mut sz, 0, null(), 0, null());
            log(format!("null_buffer rc={rc} sz={sz}"));
            log_img("null_buffer", &image);

            let mut image = PngImage {
                width: 4,
                height: 2,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let rc = (s.wr_mem)(
                ip,
                null_mut(),
                null_mut(),
                0,
                px.as_ptr() as *const c_void,
                0,
                null(),
            );
            log(format!("null_size rc={rc}"));
            log_img("null_size", &image);

            let mut image = PngImage {
                version: 7,
                width: 4,
                height: 2,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let mut sz: usize = 0;
            let rc = (s.wr_mem)(
                ip,
                null_mut(),
                &mut sz,
                0,
                px.as_ptr() as *const c_void,
                0,
                null(),
            );
            log(format!("bad_version rc={rc} sz={sz}"));
            log_img("bad_version", &image);
        }
        trace()
    });

    // --- error paths, deliberately last -------------------------------------
    // A row stride smaller than the image row: png_error("supplied row stride
    // too small").
    for &(fmt, fname) in &[
        (PNG_FORMAT_RGB, "RGB"),
        (PNG_FORMAT_BGRA, "BGRA"),
        (PNG_FORMAT_LINEAR_RGB_ALPHA, "LINEAR_RGB_ALPHA"),
    ] {
        let exact = (pixel_channels(fmt) * 6) as i32;
        for &st in &[exact - 1, 1, -1] {
            write_mem_case(
                &format!("S5e fmt={fname} stride_too_small={st}"),
                Wr {
                    fmt,
                    stride: st,
                    seed: 0x9500 + fmt as u64,
                    ..Default::default()
                },
            );
        }
    }
    // A colour-mapped format with no colour-map at all, and a format carrying
    // a flag the writer cannot handle.
    let bad: &[(u32, u32, &str)] = &[
        (PNG_FORMAT_RGB_COLORMAP, 0, "cmap_no_entries"),
        (PNG_FORMAT_RGBA_COLORMAP, 0, "cmapa_no_entries"),
        (
            PNG_FORMAT_RGB | PNG_FORMAT_FLAG_ASSOCIATED_ALPHA,
            0,
            "assoc_alpha",
        ),
        (
            PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_ASSOCIATED_ALPHA,
            0,
            "rgba_assoc_alpha",
        ),
    ];
    for &(fmt, entries, name) in bad {
        write_mem_case(
            &format!("S5e {name}"),
            Wr {
                fmt,
                entries,
                seed: 0x9700 + fmt as u64,
                ..Default::default()
            },
        );
    }
}

// ---------------------------------------------------------------------------
// S6 — the stdio / file entry points
// ---------------------------------------------------------------------------

fn tmp_path(lib: &Lib, tag: &str) -> std::path::PathBuf {
    // Distinct file names per library run so the two runs cannot interfere.
    std::env::temp_dir().join(format!("png_s6_{}_{}_{}.png", lib.tag, tag, std::process::id()))
}

/// A fresh id per test case, so a leftover file can never be mistaken for the
/// output of the case currently running.
fn next_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

/// write_to_file -> read the file back -> begin_read_from_file + finish_read.
fn file_roundtrip(label: &str, o: Wr, out_fmt: u32) {
    let pixels = o.pixels();
    let cmap = o.colormap();
    let id = next_id();
    diff(label, |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        let path = tmp_path(lib, &format!("file{id}"));
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let bufp = pixels.as_ptr() as *const c_void;
        let cmp: *const c_void = if cmap.is_empty() {
            null()
        } else {
            cmap.as_ptr() as *const c_void
        };
        unsafe {
            let mut image = o.mk_image();
            let ip: *mut PngImage = &mut image;
            let rc = (s.wr_file)(ip, cpath.as_ptr(), o.convert8, bufp, o.stride, cmp);
            log(format!("write_to_file rc={rc}"));
            log_img("write_to_file", &image);
            let bytes = std::fs::read(&path).unwrap_or_default();
            log(format!("file_len={} file={}", bytes.len(), hex(&bytes)));

            // read it straight back with the simplified reader
            let mut im2 = PngImage::default();
            let ip2: *mut PngImage = &mut im2;
            let rc = (s.begin_file)(ip2, cpath.as_ptr());
            log(format!("begin_read_from_file rc={rc}"));
            log_img("begin_file", &im2);
            if rc != 0 {
                im2.format = out_fmt;
                let stride = row_stride(&im2);
                let need = buffer_size(&im2, stride);
                let cmneed = if out_fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                    colormap_size(&im2)
                } else {
                    0
                };
                let mut rbuf = vec![0xa5u8; need + SLACK];
                let mut rcmap = vec![0x5au8; cmneed + SLACK];
                let cmptr = if cmneed > 0 {
                    rcmap.as_mut_ptr() as *mut c_void
                } else {
                    null_mut()
                };
                let rc2 = (s.finish)(
                    ip2,
                    null(),
                    rbuf.as_mut_ptr() as *mut c_void,
                    stride as i32,
                    cmptr,
                );
                log(format!("finish rc={rc2} stride={stride} bytes={need}"));
                log_img("finish_file", &im2);
                log(format!("buf={}", hex(&rbuf[..need])));
                log(format!("buf_slack={}", hex(&rbuf[need..])));
                if cmneed > 0 {
                    log(format!("cmap={}", hex(&rcmap[..cmneed])));
                    log(format!("cmap_slack={}", hex(&rcmap[cmneed..])));
                }
            }
            (s.free)(ip2);
            log(format!("freed opaque_null={}", im2.opaque.is_null() as u8));
        }
        let _ = std::fs::remove_file(&path);
        trace()
    });
}

/// write_to_stdio -> read the file back -> begin_read_from_stdio + finish_read.
fn stdio_roundtrip(label: &str, o: Wr, out_fmt: u32) {
    let pixels = o.pixels();
    let cmap = o.colormap();
    let id = next_id();
    diff(label, |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        let path = tmp_path(lib, &format!("stdio{id}"));
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let wmode = CString::new("wb").unwrap();
        let rmode = CString::new("rb").unwrap();
        let bufp = pixels.as_ptr() as *const c_void;
        let cmp: *const c_void = if cmap.is_empty() {
            null()
        } else {
            cmap.as_ptr() as *const c_void
        };
        unsafe {
            let fp = fopen(cpath.as_ptr(), wmode.as_ptr());
            log(format!("fopen_w_null={}", fp.is_null() as u8));
            let mut image = o.mk_image();
            let ip: *mut PngImage = &mut image;
            let rc = (s.wr_stdio)(ip, fp, o.convert8, bufp, o.stride, cmp);
            log(format!("write_to_stdio rc={rc}"));
            log_img("write_to_stdio", &image);
            log(format!("fflush={} fclose={}", fflush(fp), fclose(fp)));
            let bytes = std::fs::read(&path).unwrap_or_default();
            log(format!("file_len={} file={}", bytes.len(), hex(&bytes)));

            let fp = fopen(cpath.as_ptr(), rmode.as_ptr());
            log(format!("fopen_r_null={}", fp.is_null() as u8));
            let mut im2 = PngImage::default();
            let ip2: *mut PngImage = &mut im2;
            let rc = (s.begin_stdio)(ip2, fp);
            log(format!("begin_read_from_stdio rc={rc}"));
            log_img("begin_stdio", &im2);
            if rc != 0 {
                im2.format = out_fmt;
                let stride = row_stride(&im2);
                let need = buffer_size(&im2, stride);
                let cmneed = if out_fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                    colormap_size(&im2)
                } else {
                    0
                };
                let mut rbuf = vec![0xa5u8; need + SLACK];
                let mut rcmap = vec![0x5au8; cmneed + SLACK];
                let cmptr = if cmneed > 0 {
                    rcmap.as_mut_ptr() as *mut c_void
                } else {
                    null_mut()
                };
                let rc2 = (s.finish)(
                    ip2,
                    null(),
                    rbuf.as_mut_ptr() as *mut c_void,
                    stride as i32,
                    cmptr,
                );
                log(format!("finish rc={rc2} stride={stride} bytes={need}"));
                log_img("finish_stdio", &im2);
                log(format!("buf={}", hex(&rbuf[..need])));
                log(format!("buf_slack={}", hex(&rbuf[need..])));
                if cmneed > 0 {
                    log(format!("cmap={}", hex(&rcmap[..cmneed])));
                    log(format!("cmap_slack={}", hex(&rcmap[cmneed..])));
                }
            }
            (s.free)(ip2);
            log(format!("freed opaque_null={}", im2.opaque.is_null() as u8));
            // begin_read_from_stdio does not take ownership of the FILE.
            log(format!("fclose_r={}", fclose(fp)));
        }
        let _ = std::fs::remove_file(&path);
        trace()
    });
}

#[test]
fn s6_file_and_stdio() {
    ensure_libm();
    let cases: &[(u32, &str, u32, u32)] = &[
        (PNG_FORMAT_GRAY, "GRAY", 0, PNG_FORMAT_GRAY),
        (PNG_FORMAT_GA, "GA", 0, PNG_FORMAT_GA),
        (PNG_FORMAT_RGB, "RGB", 0, PNG_FORMAT_RGB),
        (PNG_FORMAT_RGBA, "RGBA", 0, PNG_FORMAT_RGBA),
        (PNG_FORMAT_BGRA, "BGRA", 0, PNG_FORMAT_BGR),
        (PNG_FORMAT_LINEAR_Y, "LINEAR_Y", 0, PNG_FORMAT_LINEAR_Y),
        (
            PNG_FORMAT_LINEAR_RGB_ALPHA,
            "LINEAR_RGB_ALPHA",
            0,
            PNG_FORMAT_LINEAR_RGB_ALPHA,
        ),
        (PNG_FORMAT_RGB_COLORMAP, "RGB_COLORMAP", 6, PNG_FORMAT_RGB),
        (
            PNG_FORMAT_RGBA_COLORMAP,
            "RGBA_COLORMAP",
            17,
            PNG_FORMAT_RGBA_COLORMAP,
        ),
    ];
    for &(fmt, fname, entries, out_fmt) in cases {
        for c8 in [0 as c_int, 1] {
            for &(w, h) in &[(5u32, 3u32), (1, 1), (9, 2)] {
                let o = Wr {
                    fmt,
                    entries,
                    convert8: c8,
                    w,
                    h,
                    seed: 0xa100 + fmt as u64 + c8 as u64 + w as u64 * 5,
                    ..Default::default()
                };
                file_roundtrip(
                    &format!("S6 file fmt={fname} convert8={c8} {w}x{h}"),
                    o,
                    out_fmt,
                );
                stdio_roundtrip(
                    &format!("S6 stdio fmt={fname} convert8={c8} {w}x{h}"),
                    o,
                    out_fmt,
                );
            }
        }
    }

    // Error paths: a non-existent file, a non-existent directory, NULL
    // arguments and a damaged version.  The messages come from strerror(),
    // which is the same libc for both libraries.
    diff("S6err", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        let px = [0u8; 128];
        let missing = std::env::temp_dir().join(format!(
            "png_s6_missing_{}_{}.png",
            lib.tag,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&missing);
        let cmissing = CString::new(missing.to_str().unwrap()).unwrap();
        let baddir = std::env::temp_dir().join(format!(
            "png_s6_nodir_{}_{}/x.png",
            lib.tag,
            std::process::id()
        ));
        let cbaddir = CString::new(baddir.to_str().unwrap()).unwrap();
        unsafe {
            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_file)(ip, cmissing.as_ptr());
            log(format!("begin_read_from_file missing rc={rc}"));
            log_img("missing", &image);
            (s.free)(ip);

            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_file)(ip, null());
            log(format!("begin_read_from_file null rc={rc}"));
            log_img("null_name", &image);
            (s.free)(ip);

            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_stdio)(ip, null_mut());
            log(format!("begin_read_from_stdio null rc={rc}"));
            log_img("null_file", &image);
            (s.free)(ip);

            let mut image = PngImage {
                version: 3,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_stdio)(ip, null_mut());
            log(format!("begin_read_from_stdio badver rc={rc}"));
            log_img("badver_stdio", &image);
            let rc = (s.begin_file)(ip, cmissing.as_ptr());
            log(format!("begin_read_from_file badver rc={rc}"));
            log_img("badver_file", &image);

            let mut image = PngImage {
                width: 4,
                height: 2,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let rc = (s.wr_file)(
                ip,
                cbaddir.as_ptr(),
                0,
                px.as_ptr() as *const c_void,
                0,
                null(),
            );
            log(format!("write_to_file baddir rc={rc}"));
            log_img("write_baddir", &image);

            let mut image = PngImage {
                width: 4,
                height: 2,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let rc = (s.wr_file)(ip, null(), 0, px.as_ptr() as *const c_void, 0, null());
            log(format!("write_to_file nullname rc={rc}"));
            log_img("write_nullname", &image);

            let mut image = PngImage {
                width: 4,
                height: 2,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let rc = (s.wr_stdio)(ip, null_mut(), 0, px.as_ptr() as *const c_void, 0, null());
            log(format!("write_to_stdio nullfile rc={rc}"));
            log_img("write_nullfile", &image);

            let mut image = PngImage {
                version: 9,
                width: 4,
                height: 2,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let rc = (s.wr_stdio)(ip, null_mut(), 0, px.as_ptr() as *const c_void, 0, null());
            log(format!("write_to_stdio badver rc={rc}"));
            log_img("write_badver", &image);
        }
        trace()
    });

    // A file that exists but is not a PNG.
    diff("S6notpng", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        let path = tmp_path(lib, "notpng");
        std::fs::write(&path, b"not a PNG file at all, just 40 bytes...").unwrap();
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let rmode = CString::new("rb").unwrap();
        unsafe {
            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_file)(ip, cpath.as_ptr());
            log(format!("begin_read_from_file notpng rc={rc}"));
            log_img("notpng_file", &image);
            (s.free)(ip);

            let fp = fopen(cpath.as_ptr(), rmode.as_ptr());
            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_stdio)(ip, fp);
            log(format!("begin_read_from_stdio notpng rc={rc}"));
            log_img("notpng_stdio", &image);
            (s.free)(ip);
            log(format!("fclose={}", fclose(fp)));
        }
        let _ = std::fs::remove_file(&path);
        trace()
    });
}

// ---------------------------------------------------------------------------
// S7 — png_image_free
// ---------------------------------------------------------------------------

#[test]
fn s7_image_free() {
    ensure_libm();
    let good = mk(5, 3, 6, 8, 0, 0xb100, false, false);
    // IEND dropped: libpng does not need it for the simplified read, so this
    // still succeeds.
    let no_iend = good[..good.len() - 12].to_vec();
    // truncated: keep the signature + IHDR + the IDAT chunk header (so the
    // header read succeeds) but cut the IDAT data short, so finish_read fails
    // with "read beyond end of data".
    let truncated = good[..good.len() - 24].to_vec();
    // header itself truncated: begin_read already fails
    let header_cut = good[..20].to_vec();

    diff("S7 free_paths", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        unsafe {
            // --- (a) after a successful read -------------------------------
            {
                let mut image = PngImage::default();
                let ip: *mut PngImage = &mut image;
                let rc = (s.begin_mem)(ip, good.as_ptr() as *const c_void, good.len());
                log(format!("a begin rc={rc}"));
                log_img("a begin", &image);
                image.format = PNG_FORMAT_RGBA;
                let stride = row_stride(&image);
                let need = buffer_size(&image, stride);
                let mut buf = vec![0xa5u8; need + SLACK];
                let rc2 = (s.finish)(
                    ip,
                    null(),
                    buf.as_mut_ptr() as *mut c_void,
                    stride as i32,
                    null_mut(),
                );
                log(format!("a finish rc={rc2}"));
                log_img("a finish", &image);
                log(format!("a buf={}", hex(&buf[..need])));
                log(format!("a slack={}", hex(&buf[need..])));
                (s.free)(ip);
                log_img("a free1", &image);
                (s.free)(ip);
                log_img("a free2", &image);
                (s.free)(ip);
                log_img("a free3", &image);
            }
            // --- (d) begin_read succeeded, freed without finish_read -------
            {
                let mut image = PngImage::default();
                let ip: *mut PngImage = &mut image;
                let rc = (s.begin_mem)(ip, good.as_ptr() as *const c_void, good.len());
                log(format!("d begin rc={rc}"));
                log_img("d begin", &image);
                (s.free)(ip);
                log_img("d free1", &image);
                (s.free)(ip);
                log_img("d free2", &image);
            }
            // --- (e) a zeroed png_image (version 0) ------------------------
            {
                let mut image = PngImage {
                    version: 0,
                    ..Default::default()
                };
                let ip: *mut PngImage = &mut image;
                log_img("e before", &image);
                (s.free)(ip);
                log_img("e free1", &image);
                (s.free)(ip);
                log_img("e free2", &image);
            }
            // --- (f) only `version` set ------------------------------------
            {
                let mut image = PngImage::default();
                let ip: *mut PngImage = &mut image;
                log_img("f before", &image);
                (s.free)(ip);
                log_img("f free1", &image);
                (s.free)(ip);
                log_img("f free2", &image);
            }
            // --- (h) after a successful write -----------------------------
            {
                let px = [0x5au8; 4 * 2 * 3];
                let mut image = PngImage {
                    width: 4,
                    height: 2,
                    format: PNG_FORMAT_RGB,
                    ..Default::default()
                };
                let ip: *mut PngImage = &mut image;
                let mut sz: usize = 0;
                let rc = (s.wr_mem)(
                    ip,
                    null_mut(),
                    &mut sz,
                    0,
                    px.as_ptr() as *const c_void,
                    0,
                    null(),
                );
                log(format!("h query rc={rc} sz={sz}"));
                let mut mem = vec![0u8; sz + SLACK];
                let mut sz2 = sz;
                let rc2 = (s.wr_mem)(
                    ip,
                    mem.as_mut_ptr() as *mut c_void,
                    &mut sz2,
                    0,
                    px.as_ptr() as *const c_void,
                    0,
                    null(),
                );
                log(format!("h write rc={rc2} sz={sz2}"));
                log_img("h write", &image);
                log(format!("h png={}", hex(&mem[..sz2.min(sz)])));
                (s.free)(ip);
                log_img("h free1", &image);
                (s.free)(ip);
                log_img("h free2", &image);
            }
            // --- (i) NULL image ------------------------------------------
            {
                (s.free)(null_mut());
                log("i free(NULL) returned".to_string());
            }
        }
        trace()
    });

    // begin_read_from_memory argument validation, then free.
    diff("S7 args", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        unsafe {
            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_mem)(ip, null(), 10);
            log(format!("null_memory rc={rc}"));
            log_img("null_memory", &image);
            (s.free)(ip);
            log_img("null_memory freed", &image);

            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_mem)(ip, good.as_ptr() as *const c_void, 0);
            log(format!("zero_size rc={rc}"));
            log_img("zero_size", &image);
            (s.free)(ip);
            log_img("zero_size freed", &image);

            let mut image = PngImage {
                version: 2,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_mem)(ip, good.as_ptr() as *const c_void, good.len());
            log(format!("bad_version rc={rc}"));
            log_img("bad_version", &image);
            (s.free)(ip);
            log_img("bad_version freed", &image);

            // finish_read on an image whose opaque is NULL / version damaged
            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            image.width = 2;
            image.height = 2;
            image.format = PNG_FORMAT_RGB;
            let mut buf = [0u8; 64];
            let rc = (s.finish)(ip, null(), buf.as_mut_ptr() as *mut c_void, 6, null_mut());
            log(format!("finish_no_opaque rc={rc}"));
            log_img("finish_no_opaque", &image);

            let mut image = PngImage {
                version: 5,
                width: 2,
                height: 2,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let rc = (s.finish)(ip, null(), buf.as_mut_ptr() as *mut c_void, 6, null_mut());
            log(format!("finish_bad_version rc={rc}"));
            log_img("finish_bad_version", &image);
        }
        trace()
    });

    // A datastream without IEND: the simplified reader does not need it, so
    // this must succeed and free cleanly.
    diff("S7j read_without_IEND", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        unsafe {
            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_mem)(ip, no_iend.as_ptr() as *const c_void, no_iend.len());
            log(format!("j begin rc={rc}"));
            log_img("j begin", &image);
            image.format = PNG_FORMAT_RGBA;
            let stride = row_stride(&image);
            let need = buffer_size(&image, stride);
            let mut buf = vec![0xa5u8; need + SLACK];
            let rc2 = (s.finish)(
                ip,
                null(),
                buf.as_mut_ptr() as *mut c_void,
                stride as i32,
                null_mut(),
            );
            log(format!("j finish rc={rc2}"));
            log_img("j finish", &image);
            log(format!("j buf={}", hex(&buf[..need])));
            log(format!("j slack={}", hex(&buf[need..])));
            (s.free)(ip);
            log_img("j free1", &image);
            (s.free)(ip);
            log_img("j free2", &image);
        }
        trace()
    });

    // --- error paths, deliberately last -------------------------------------
    // (b) png_image_free after a *failed* read: the IDAT data is cut short, so
    // finish_read must fail and must still leave image.opaque == NULL.
    diff("S7b free_after_failed_read", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        unsafe {
            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_mem)(ip, truncated.as_ptr() as *const c_void, truncated.len());
            log(format!("b begin rc={rc}"));
            log_img("b begin", &image);
            image.format = PNG_FORMAT_RGBA;
            let stride = row_stride(&image);
            let need = buffer_size(&image, stride);
            let mut buf = vec![0xa5u8; need + SLACK];
            let rc2 = (s.finish)(
                ip,
                null(),
                buf.as_mut_ptr() as *mut c_void,
                stride as i32,
                null_mut(),
            );
            log(format!("b finish rc={rc2}"));
            log_img("b finish", &image);
            log(format!("b buf={}", hex(&buf[..need])));
            log(format!("b slack={}", hex(&buf[need..])));
            (s.free)(ip);
            log_img("b free1", &image);
            (s.free)(ip);
            log_img("b free2", &image);
        }
        trace()
    });

    // (c) png_image_free after begin_read itself failed (header cut in two).
    diff("S7c free_after_failed_begin", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        unsafe {
            let mut image = PngImage::default();
            let ip: *mut PngImage = &mut image;
            let rc = (s.begin_mem)(ip, header_cut.as_ptr() as *const c_void, header_cut.len());
            log(format!("c begin rc={rc}"));
            log_img("c begin", &image);
            (s.free)(ip);
            log_img("c free1", &image);
            (s.free)(ip);
            log_img("c free2", &image);
        }
        trace()
    });

    // (g) png_image_free after a failed write (colour-mapped, no colour-map).
    diff("S7g free_after_failed_write", |lib| {
        session_reset(Vec::new());
        let s = Simp::new(lib);
        let px = [0u8; 128];
        unsafe {
            let mut image = PngImage {
                width: 4,
                height: 2,
                format: PNG_FORMAT_RGB_COLORMAP,
                ..Default::default()
            };
            let ip: *mut PngImage = &mut image;
            let mut sz: usize = 0;
            let rc = (s.wr_mem)(
                ip,
                null_mut(),
                &mut sz,
                0,
                px.as_ptr() as *const c_void,
                0,
                null(),
            );
            log(format!("g write rc={rc} sz={sz}"));
            log_img("g write", &image);
            (s.free)(ip);
            log_img("g free1", &image);
            (s.free)(ip);
            log_img("g free2", &image);
        }
        trace()
    });

    // Several truncation lengths: every one must leave a clean png_image.
    for cut in [1usize, 8, 12, 20, 30, 33, 40, 45, 50] {
        if cut >= good.len() {
            continue;
        }
        let part = good[..cut].to_vec();
        diff(&format!("S7 cut={cut}"), |lib| {
            session_reset(Vec::new());
            let s = Simp::new(lib);
            unsafe {
                let mut image = PngImage::default();
                let ip: *mut PngImage = &mut image;
                let rc = (s.begin_mem)(ip, part.as_ptr() as *const c_void, part.len());
                log(format!("begin rc={rc}"));
                log_img("begin", &image);
                if rc != 0 {
                    image.format = PNG_FORMAT_RGBA;
                    let stride = row_stride(&image);
                    let need = buffer_size(&image, stride);
                    let mut buf = vec![0xa5u8; need + SLACK];
                    let rc2 = (s.finish)(
                        ip,
                        null(),
                        buf.as_mut_ptr() as *mut c_void,
                        stride as i32,
                        null_mut(),
                    );
                    log(format!("finish rc={rc2} bytes={need}"));
                    log_img("finish", &image);
                    log(format!("buf={}", hex(&buf[..need])));
                    log(format!("slack={}", hex(&buf[need..])));
                }
                (s.free)(ip);
                log_img("free1", &image);
                (s.free)(ip);
                log_img("free2", &image);
            }
            trace()
        });
    }
}

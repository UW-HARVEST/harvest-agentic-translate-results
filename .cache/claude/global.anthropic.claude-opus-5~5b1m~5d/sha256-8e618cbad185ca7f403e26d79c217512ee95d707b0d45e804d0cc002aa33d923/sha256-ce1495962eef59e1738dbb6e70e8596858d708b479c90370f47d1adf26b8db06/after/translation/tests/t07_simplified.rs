//! Phase B — the SIMPLIFIED API:
//! `png_image_begin_read_from_memory` / `_from_stdio` / `_from_file`,
//! `png_image_finish_read`, `png_image_free`, `png_image_error`,
//! `png_image_write_to_memory` / `_to_file` / `_to_stdio`.
//!
//! Every `PNG_FORMAT_*` value (including the `_COLORMAP` variants), direct and
//! color-mapped output, a NULL and a non-NULL `background`, positive, negative
//! and over-sized `row_stride`, and `convert_to_8bit` both ways are driven
//! through both shared objects and the produced buffers, the returned
//! `png_image` fields, `image.warning_or_error` and the `image.message` text
//! are compared byte for byte / character for character.
//!
//! Source PNGs are produced with the sequential write path (already verified
//! byte-for-byte in `t03_write`).
mod common;
use common::*;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

#[allow(non_upper_case_globals)]
const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: png_uint_32 = 0x01;
const PNG_IMAGE_FLAG_FAST: png_uint_32 = 0x02;

// ---------------------------------------------------------------------------
// PNG_IMAGE_* macros, mirrored from png.h
// ---------------------------------------------------------------------------

fn f_sample_channels(f: png_uint_32) -> u32 {
    (f & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}
fn f_sample_comp(f: png_uint_32) -> u32 {
    ((f & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
}
fn f_is_cmap(f: png_uint_32) -> bool {
    f & PNG_FORMAT_FLAG_COLORMAP != 0
}
fn f_pixel_channels(f: png_uint_32) -> u32 {
    if f_is_cmap(f) {
        1
    } else {
        f_sample_channels(f)
    }
}
fn f_pixel_comp(f: png_uint_32) -> u32 {
    if f_is_cmap(f) {
        1
    } else {
        f_sample_comp(f)
    }
}
/// PNG_IMAGE_MAXIMUM_COLORMAP_COMPONENTS * PNG_IMAGE_SAMPLE_COMPONENT_SIZE
fn f_cmap_bytes(f: png_uint_32) -> usize {
    f_sample_channels(f) as usize * 256 * f_sample_comp(f) as usize
}

const FORMATS: &[(&str, png_uint_32)] = &[
    ("GRAY", PNG_FORMAT_GRAY),
    ("GA", PNG_FORMAT_GA),
    ("AG", PNG_FORMAT_AG),
    ("RGB", PNG_FORMAT_RGB),
    ("BGR", PNG_FORMAT_BGR),
    ("RGBA", PNG_FORMAT_RGBA),
    ("ARGB", PNG_FORMAT_ARGB),
    ("BGRA", PNG_FORMAT_BGRA),
    ("ABGR", PNG_FORMAT_ABGR),
    ("LINEAR_Y", PNG_FORMAT_LINEAR_Y),
    ("LINEAR_Y_ALPHA", PNG_FORMAT_LINEAR_Y_ALPHA),
    ("LINEAR_RGB", PNG_FORMAT_LINEAR_RGB),
    ("LINEAR_RGB_ALPHA", PNG_FORMAT_LINEAR_RGB_ALPHA),
    ("GRAY_COLORMAP", PNG_FORMAT_FLAG_COLORMAP),
    ("GA_COLORMAP", PNG_FORMAT_GA | PNG_FORMAT_FLAG_COLORMAP),
    ("AG_COLORMAP", PNG_FORMAT_AG | PNG_FORMAT_FLAG_COLORMAP),
    ("RGB_COLORMAP", PNG_FORMAT_RGB_COLORMAP),
    ("BGR_COLORMAP", PNG_FORMAT_BGR_COLORMAP),
    ("RGBA_COLORMAP", PNG_FORMAT_RGBA_COLORMAP),
    ("ARGB_COLORMAP", PNG_FORMAT_ARGB_COLORMAP),
    ("BGRA_COLORMAP", PNG_FORMAT_BGRA_COLORMAP),
    ("ABGR_COLORMAP", PNG_FORMAT_ABGR_COLORMAP),
    ("LINEAR_Y_COLORMAP", PNG_FORMAT_LINEAR_Y | PNG_FORMAT_FLAG_COLORMAP),
    (
        "LINEAR_RGB_COLORMAP",
        PNG_FORMAT_LINEAR_RGB | PNG_FORMAT_FLAG_COLORMAP,
    ),
];

// ---------------------------------------------------------------------------
// Source PNGs, produced with the sequential write path
// ---------------------------------------------------------------------------

struct Src {
    w: u32,
    h: u32,
    bd: c_int,
    ct: c_int,
    il: c_int,
    palette: Vec<png_color>,
    rows: Vec<Vec<u8>>,
    anc: bool,
}

impl Src {
    fn gen(rng: &mut Rng, ct: c_int, bd: c_int, w: u32, h: u32, il: c_int, anc: bool) -> Src {
        let pd = channels_of(ct) * bd as u32;
        let rb = rowbytes(pd, w);
        let rows = (0..h).map(|_| rng.bytes(rb)).collect();
        let npal = if ct == PNG_COLOR_TYPE_PALETTE {
            1usize << bd
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
        Src {
            w,
            h,
            bd,
            ct,
            il,
            palette,
            rows,
            anc,
        }
    }
}

unsafe fn encode(api: &'static Api, s: &Src) -> Vec<u8> {
    set_current_api(api);
    diag_reset();
    let mut sess = WriteSess::new(api);
    let png = sess.png;
    let info = sess.info;

    let key = cs("Title");
    let txt = cs("simplified test");
    let text = [png_text {
        compression: PNG_TEXT_COMPRESSION_NONE,
        key: key.as_ptr() as png_charp,
        text: txt.as_ptr() as png_charp,
        text_length: 15,
        itxt_length: 0,
        lang: null_mut(),
        lang_key: null_mut(),
    }];
    let trns_alpha: Vec<u8> = if s.ct == PNG_COLOR_TYPE_PALETTE {
        (0..s.palette.len())
            .map(|i| (i as u8).wrapping_mul(53).wrapping_add(7))
            .collect()
    } else {
        Vec::new()
    };
    let trns_col = png_color_16 {
        index: 0,
        red: 1,
        green: 1,
        blue: 1,
        gray: 1,
    };
    let bkgd = png_color_16 {
        index: 0,
        red: 1,
        green: 1,
        blue: 1,
        gray: 1,
    };

    let ok = guard(|| {
        (api.png_set_IHDR)(
            png,
            info,
            s.w,
            s.h,
            s.bd,
            s.ct,
            s.il,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if !s.palette.is_empty() {
            (api.png_set_PLTE)(png, info, s.palette.as_ptr(), s.palette.len() as c_int);
        }
        if s.anc {
            (api.png_set_gAMA)(png, info, 0.45455);
            (api.png_set_text)(png, info, text.as_ptr(), 1);
            (api.png_set_bKGD)(png, info, &bkgd as *const png_color_16);
            match s.ct {
                PNG_COLOR_TYPE_PALETTE => (api.png_set_tRNS)(
                    png,
                    info,
                    trns_alpha.as_ptr() as png_bytep,
                    trns_alpha.len() as c_int,
                    null_mut(),
                ),
                PNG_COLOR_TYPE_GRAY | PNG_COLOR_TYPE_RGB => (api.png_set_tRNS)(
                    png,
                    info,
                    null_mut(),
                    0,
                    &trns_col as *const png_color_16 as png_color_16p,
                ),
                _ => {}
            }
        }
        (api.png_write_info)(png, info);
        let mut rowps: Vec<png_bytep> = s.rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
        (api.png_write_image)(png, rowps.as_mut_ptr());
        (api.png_write_end)(png, info);
    });
    let d = diag_take();
    assert!(ok.is_some(), "encode failed: {:?}", d);
    assert!(d.errors.is_empty(), "encode errors: {:?}", d);
    std::mem::take(&mut sess.sink.buf)
}

// ---------------------------------------------------------------------------
// png_image snapshots
// ---------------------------------------------------------------------------

fn msg_of(im: &png_image) -> String {
    let b: Vec<u8> = im
        .message
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&b).into_owned()
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
struct Snap {
    ret: c_int,
    woe: png_uint_32,
    msg: String,
    version: png_uint_32,
    w: u32,
    h: u32,
    fmt: png_uint_32,
    flags: png_uint_32,
    cme: png_uint_32,
    opaque_null: bool,
}

fn snap(ret: c_int, im: &png_image) -> Snap {
    Snap {
        ret,
        woe: im.warning_or_error,
        msg: msg_of(im),
        version: im.version,
        w: im.width,
        h: im.height,
        fmt: im.format,
        flags: im.flags,
        cme: im.colormap_entries,
        opaque_null: im.opaque.is_null(),
    }
}

// ---------------------------------------------------------------------------
// Read driver
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Sourc {
    Memory,
    File,
    Stdio,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Stride {
    /// pass 0 -> libpng defaults to PNG_IMAGE_ROW_STRIDE
    Default,
    Min,
    MinNeg,
    Pad(i32),
    PadNeg(i32),
    /// deliberately one component too small
    TooSmall,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
struct RRes {
    begin: Snap,
    finish: Snap,
    after_free: Snap,
    stride_used: png_int_32,
    buffer: Vec<u8>,
    colormap: Vec<u8>,
    diag: Diag,
    ok: bool,
}

#[allow(clippy::too_many_arguments)]
unsafe fn read_simplified(
    api: &'static Api,
    bytes: &[u8],
    src: Sourc,
    path: &Path,
    fmt: png_uint_32,
    bg: Option<png_color>,
    stride: Stride,
    give_colormap: bool,
) -> RRes {
    set_current_api(api);
    diag_reset();
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    let mode_r = cs("rb");
    let mut im = png_image::default();
    let mut fh: *mut c_void = null_mut();
    let mut res = RRes::default();
    let ok = guard(|| {
        let br = match src {
            Sourc::Memory => (api.png_image_begin_read_from_memory)(
                &mut im,
                bytes.as_ptr() as png_const_voidp,
                bytes.len(),
            ),
            Sourc::File => (api.png_image_begin_read_from_file)(&mut im, cpath.as_ptr()),
            Sourc::Stdio => {
                fh = fopen(cpath.as_ptr(), mode_r.as_ptr());
                assert!(!fh.is_null(), "fopen({}) failed", path.display());
                (api.png_image_begin_read_from_stdio)(&mut im, fh)
            }
        };
        res.begin = snap(br, &im);
        if br != 0 {
            im.format = fmt;
            let chans = f_pixel_channels(fmt);
            let comp = f_pixel_comp(fmt);
            let min = (im.width * chans) as i32;
            let sv = match stride {
                Stride::Default => 0,
                Stride::Min => min,
                Stride::MinNeg => -min,
                Stride::Pad(p) => min + p,
                Stride::PadNeg(p) => -(min + p),
                Stride::TooSmall => min - 1,
            };
            res.stride_used = sv;
            let a = if sv == 0 { min } else { sv.abs() };
            // Always allocate for at least the minimum stride so that the
            // TooSmall case still points at a valid buffer.
            let bufsize = comp as usize * im.height as usize * (a.max(min) as usize);
            res.buffer = vec![0xA5u8; bufsize.max(1)];
            res.colormap = vec![0x5Au8; f_cmap_bytes(fmt)];
            let cmp = if give_colormap && f_is_cmap(fmt) {
                res.colormap.as_mut_ptr() as *mut c_void
            } else {
                null_mut()
            };
            let bgp = match &bg {
                Some(c) => c as *const png_color,
                None => null(),
            };
            let fr = (api.png_image_finish_read)(
                &mut im,
                bgp,
                res.buffer.as_mut_ptr() as *mut c_void,
                sv,
                cmp,
            );
            res.finish = snap(fr, &im);
        }
        (api.png_image_free)(&mut im);
        res.after_free = snap(0, &im);
    })
    .is_some();
    if !fh.is_null() {
        fclose(fh);
    }
    res.diag = diag_take();
    res.ok = ok;
    res
}

#[allow(clippy::too_many_arguments)]
fn read_diff(
    label: &str,
    bytes: &[u8],
    src: Sourc,
    path: &Path,
    fmt: png_uint_32,
    bg: Option<png_color>,
    stride: Stride,
    give_colormap: bool,
) -> RRes {
    unsafe {
        let c = read_simplified(c_api(), bytes, src, path, fmt, bg, stride, give_colormap);
        let r = read_simplified(rs_api(), bytes, src, path, fmt, bg, stride, give_colormap);
        assert_eq!(c.ok, r.ok, "{label}: unwind parity");
        assert_eq!(c.diag, r.diag, "{label}: diagnostics");
        assert_eq!(c.begin, r.begin, "{label}: begin_read");
        assert_eq!(c.finish, r.finish, "{label}: finish_read");
        assert_eq!(c.after_free, r.after_free, "{label}: after png_image_free");
        assert_eq!(c.stride_used, r.stride_used, "{label}: stride");
        assert_bytes_eq(&format!("{label}: pixel buffer"), &c.buffer, &r.buffer);
        assert_bytes_eq(&format!("{label}: colormap"), &c.colormap, &r.colormap);
        c
    }
}

// ---------------------------------------------------------------------------
// Write driver
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Dest {
    Memory,
    File,
    Stdio,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
struct WRes {
    query: Snap,
    query_size: usize,
    write: Snap,
    written: usize,
    bytes: Vec<u8>,
    diag: Diag,
    ok: bool,
}

struct WIn {
    w: u32,
    h: u32,
    fmt: png_uint_32,
    flags: png_uint_32,
    cme: png_uint_32,
    stride: png_int_32,
    convert8: c_int,
    buffer: Vec<u8>,
    colormap: Vec<u8>,
    pass_colormap: bool,
}

impl WIn {
    fn image(&self) -> png_image {
        png_image {
            width: self.w,
            height: self.h,
            format: self.fmt,
            flags: self.flags,
            colormap_entries: self.cme,
            ..png_image::default()
        }
    }
}

unsafe fn write_simplified(api: &'static Api, w: &WIn, dest: Dest, path: &Path) -> WRes {
    set_current_api(api);
    diag_reset();
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    let mode_w = cs("wb");
    let mut res = WRes::default();
    let cmap: *const c_void = if w.pass_colormap {
        w.colormap.as_ptr() as *const c_void
    } else {
        null()
    };
    let buf: *const c_void = w.buffer.as_ptr() as *const c_void;
    let mut fh: *mut c_void = null_mut();
    let ok = guard(|| match dest {
        Dest::Memory => {
            // 1. size query (memory == NULL)
            let mut im = w.image();
            let mut mb: usize = 12345; // must be overwritten with 0 then the size
            let q = (api.png_image_write_to_memory)(
                &mut im,
                null_mut(),
                &mut mb,
                w.convert8,
                buf,
                w.stride,
                cmap,
            );
            res.query = snap(q, &im);
            res.query_size = mb;

            // 2. the real thing
            let mut im2 = w.image();
            let mut out = vec![0u8; mb.max(1)];
            let mut mb2: usize = out.len();
            let r = (api.png_image_write_to_memory)(
                &mut im2,
                out.as_mut_ptr() as *mut c_void,
                &mut mb2,
                w.convert8,
                buf,
                w.stride,
                cmap,
            );
            res.write = snap(r, &im2);
            res.written = mb2;
            out.truncate(mb2.min(out.len()));
            res.bytes = out;
        }
        Dest::File => {
            let mut im = w.image();
            let r = (api.png_image_write_to_file)(
                &mut im,
                cpath.as_ptr(),
                w.convert8,
                buf,
                w.stride,
                cmap,
            );
            res.write = snap(r, &im);
            res.bytes = std::fs::read(path).unwrap_or_default();
            res.written = res.bytes.len();
        }
        Dest::Stdio => {
            let mut im = w.image();
            fh = fopen(cpath.as_ptr(), mode_w.as_ptr());
            assert!(!fh.is_null(), "fopen({}) failed", path.display());
            let r =
                (api.png_image_write_to_stdio)(&mut im, fh, w.convert8, buf, w.stride, cmap);
            res.write = snap(r, &im);
            fclose(fh);
            fh = null_mut();
            res.bytes = std::fs::read(path).unwrap_or_default();
            res.written = res.bytes.len();
        }
    })
    .is_some();
    if !fh.is_null() {
        fclose(fh);
    }
    res.diag = diag_take();
    res.ok = ok;
    res
}

fn write_diff(label: &str, w: &WIn, dest: Dest, path: &Path) -> WRes {
    unsafe {
        let c = write_simplified(c_api(), w, dest, path);
        let r = write_simplified(rs_api(), w, dest, path);
        assert_eq!(c.ok, r.ok, "{label}: unwind parity");
        assert_eq!(c.diag, r.diag, "{label}: diagnostics");
        assert_eq!(c.query, r.query, "{label}: size-query call");
        assert_eq!(c.query_size, r.query_size, "{label}: queried size");
        assert_eq!(c.write, r.write, "{label}: write call");
        assert_eq!(c.written, r.written, "{label}: written size");
        assert_bytes_eq(&format!("{label}: PNG bytes"), &c.bytes, &r.bytes);
        c
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn tmp(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("t07_simplified_{}_{}.png", tag, std::process::id()))
}

fn gen_wbuf(rng: &mut Rng, h: u32, fmt: png_uint_32, stride: i32, cme: png_uint_32) -> Vec<u8> {
    let comp = f_pixel_comp(fmt) as usize;
    let n = comp * h as usize * stride.unsigned_abs() as usize;
    if f_is_cmap(fmt) {
        (0..n).map(|_| rng.below(cme.max(1)) as u8).collect()
    } else {
        rng.bytes(n)
    }
}

const IHDRS: &[(c_int, c_int)] = &[
    (PNG_COLOR_TYPE_GRAY, 1),
    (PNG_COLOR_TYPE_GRAY, 8),
    (PNG_COLOR_TYPE_GRAY, 16),
    (PNG_COLOR_TYPE_PALETTE, 4),
    (PNG_COLOR_TYPE_RGB, 8),
    (PNG_COLOR_TYPE_RGB, 16),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
];

// ---------------------------------------------------------------------------
// Read tests
// ---------------------------------------------------------------------------

/// Every output format, from memory, for every legal IHDR, several sizes, both
/// interlace types.  The source PNGs carry no ancillary chunks so the
/// "background must be supplied" path is not the only thing exercised.
#[test]
fn read_every_format_from_memory() {
    let mut rng = Rng::new(0x1357_9bdf_0246_8ace);
    let p = tmp("unused_mem");
    // Guards against a vacuously-passing sweep.
    let mut total = 0u32;
    let mut decoded = 0u32;
    let mut touched = 0u32;
    for &(ct, bd) in IHDRS {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &[(1u32, 1u32), (7, 5), (13, 9)] {
                let src = Src::gen(&mut rng, ct, bd, w, h, il, false);
                let bytes = unsafe { encode(c_api(), &src) };
                for &(name, fmt) in FORMATS {
                    let label =
                        format!("mem ct={ct} bd={bd} il={il} {w}x{h} fmt={name}");
                    let res = read_diff(
                        &label,
                        &bytes,
                        Sourc::Memory,
                        &p,
                        fmt,
                        None,
                        Stride::Default,
                        true,
                    );
                    assert_eq!(res.begin.ret, 1, "{label}: begin_read failed");
                    assert_eq!(res.begin.w, w, "{label}: width");
                    assert_eq!(res.begin.h, h, "{label}: height");
                    total += 1;
                    if res.finish.ret != 0 {
                        decoded += 1;
                        assert_eq!(res.finish.woe & PNG_IMAGE_ERROR, 0, "{label}");
                        if res.buffer.iter().any(|b| *b != 0xA5) {
                            touched += 1;
                        }
                        if f_is_cmap(fmt) {
                            assert!(
                                res.finish.cme > 0 && res.finish.cme <= 256,
                                "{label}: colormap_entries {}",
                                res.finish.cme
                            );
                            assert!(
                                res.colormap.iter().any(|b| *b != 0x5A),
                                "{label}: colormap untouched"
                            );
                        }
                    } else {
                        // The only legitimate failure for these (alpha bearing)
                        // sources is the missing background colour.
                        assert!(
                            res.finish
                                .msg
                                .contains("background color must be supplied"),
                            "{label}: unexpected failure {:?}",
                            res.finish.msg
                        );
                    }
                }
            }
        }
    }
    assert!(total >= 1000, "sweep too small: {total}");
    assert!(
        decoded * 20 >= total * 19,
        "too many failures: {decoded}/{total}"
    );
    // (a 1x1 gray image can legitimately decode to the fill byte itself)
    assert!(
        touched * 50 >= decoded * 49,
        "decodes that wrote nothing: {}/{decoded}",
        decoded - touched
    );
}

/// The same sweep with the ancillary chunks (and tRNS) present, with and
/// without a `background` colour.  Removing an alpha channel or transparency
/// for an sRGB output needs a background, so both the success and the
/// "background color must be supplied" paths are covered.
#[test]
fn read_every_format_with_background() {
    let mut rng = Rng::new(0x2468_ace0_1357_9bdf);
    let p = tmp("unused_bg");
    let bg = png_color {
        red: 0x20,
        green: 0x80,
        blue: 0xf0,
    };
    for &(ct, bd) in IHDRS {
        for &(w, h) in &[(1u32, 1u32), (11, 6)] {
            let src = Src::gen(&mut rng, ct, bd, w, h, PNG_INTERLACE_NONE, true);
            let bytes = unsafe { encode(c_api(), &src) };
            for &(name, fmt) in FORMATS {
                for b in [None, Some(bg)] {
                    let label = format!(
                        "bg ct={ct} bd={bd} {w}x{h} fmt={name} bg={}",
                        b.is_some()
                    );
                    read_diff(
                        &label,
                        &bytes,
                        Sourc::Memory,
                        &p,
                        fmt,
                        b,
                        Stride::Default,
                        true,
                    );
                }
            }
        }
    }
}

/// Positive, negative, default and padded `row_stride`, plus a stride that is
/// one component too small (a documented `invalid argument` error).
#[test]
fn read_row_strides() {
    let mut rng = Rng::new(0x3579_bdf0_2468_ace1);
    let p = tmp("unused_stride");
    let bg = png_color {
        red: 9,
        green: 99,
        blue: 199,
    };
    let strides = [
        Stride::Default,
        Stride::Min,
        Stride::MinNeg,
        Stride::Pad(3),
        Stride::PadNeg(3),
        Stride::Pad(16),
        Stride::TooSmall,
    ];
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let src = Src::gen(&mut rng, ct, bd, 9, 7, il, false);
            let bytes = unsafe { encode(c_api(), &src) };
            for &(name, fmt) in FORMATS {
                for st in strides {
                    let label = format!("stride ct={ct} bd={bd} il={il} fmt={name} {st:?}");
                    let res = read_diff(
                        &label,
                        &bytes,
                        Sourc::Memory,
                        &p,
                        fmt,
                        Some(bg),
                        st,
                        true,
                    );
                    if st == Stride::TooSmall {
                        assert_eq!(res.finish.ret, 0, "{label}: must fail");
                        assert!(
                            res.finish.msg.contains("invalid argument"),
                            "{label}: {:?}",
                            res.finish.msg
                        );
                    }
                }
            }
        }
    }
}

/// A negative stride really is a bottom-up write of the same pixels.
#[test]
fn read_negative_stride_is_flipped() {
    let mut rng = Rng::new(0x468a_ce02_468a_ce03);
    let p = tmp("unused_flip");
    for &(ct, bd) in &[(PNG_COLOR_TYPE_RGB, 8), (PNG_COLOR_TYPE_GRAY, 8)] {
        let (w, h) = (5u32, 4u32);
        let src = Src::gen(&mut rng, ct, bd, w, h, PNG_INTERLACE_NONE, false);
        let bytes = unsafe { encode(c_api(), &src) };
        for &(name, fmt) in FORMATS {
            if f_is_cmap(fmt) {
                continue;
            }
            let up = read_diff(
                &format!("flip+ {name}"),
                &bytes,
                Sourc::Memory,
                &p,
                fmt,
                Some(png_color {
                    red: 1,
                    green: 2,
                    blue: 3,
                }),
                Stride::Min,
                false,
            );
            let down = read_diff(
                &format!("flip- {name}"),
                &bytes,
                Sourc::Memory,
                &p,
                fmt,
                Some(png_color {
                    red: 1,
                    green: 2,
                    blue: 3,
                }),
                Stride::MinNeg,
                false,
            );
            if up.finish.ret == 0 || down.finish.ret == 0 {
                continue;
            }
            let rb = up.buffer.len() / h as usize;
            for y in 0..h as usize {
                assert_bytes_eq(
                    &format!("flip {name} row {y}"),
                    &up.buffer[y * rb..(y + 1) * rb],
                    &down.buffer[(h as usize - 1 - y) * rb..(h as usize - y) * rb],
                );
            }
        }
    }
}

/// `png_image_begin_read_from_file` and `png_image_begin_read_from_stdio`
/// produce exactly the same result as `..._from_memory`.
#[test]
fn read_from_file_and_stdio() {
    let mut rng = Rng::new(0x579b_df02_468a_ce05);
    let p = tmp("infile");
    for &(ct, bd) in IHDRS {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let src = Src::gen(&mut rng, ct, bd, 12, 8, il, true);
            let bytes = unsafe { encode(c_api(), &src) };
            std::fs::write(&p, &bytes).expect("write temp png");
            for &(name, fmt) in FORMATS {
                let bg = Some(png_color {
                    red: 3,
                    green: 4,
                    blue: 5,
                });
                let m = read_diff(
                    &format!("src=mem ct={ct} bd={bd} il={il} fmt={name}"),
                    &bytes,
                    Sourc::Memory,
                    &p,
                    fmt,
                    bg,
                    Stride::Default,
                    true,
                );
                let f = read_diff(
                    &format!("src=file ct={ct} bd={bd} il={il} fmt={name}"),
                    &bytes,
                    Sourc::File,
                    &p,
                    fmt,
                    bg,
                    Stride::Default,
                    true,
                );
                let s = read_diff(
                    &format!("src=stdio ct={ct} bd={bd} il={il} fmt={name}"),
                    &bytes,
                    Sourc::Stdio,
                    &p,
                    fmt,
                    bg,
                    Stride::Default,
                    true,
                );
                let lbl = format!("entrypoints ct={ct} bd={bd} il={il} fmt={name}");
                assert_eq!(m.begin, f.begin, "{lbl}: memory vs file begin");
                assert_eq!(m.begin, s.begin, "{lbl}: memory vs stdio begin");
                assert_eq!(m.finish, f.finish, "{lbl}: memory vs file finish");
                assert_eq!(m.finish, s.finish, "{lbl}: memory vs stdio finish");
                assert_bytes_eq(&format!("{lbl}: memory vs file"), &m.buffer, &f.buffer);
                assert_bytes_eq(&format!("{lbl}: memory vs stdio"), &m.buffer, &s.buffer);
                assert_bytes_eq(&format!("{lbl}: cmap file"), &m.colormap, &f.colormap);
                assert_bytes_eq(&format!("{lbl}: cmap stdio"), &m.colormap, &s.colormap);
            }
        }
    }
    let _ = std::fs::remove_file(&p);
}

/// Color-mapped output without a colormap buffer is a documented error.
#[test]
fn read_colormap_without_buffer() {
    let mut rng = Rng::new(0x68ac_e024_68ac_e007);
    let p = tmp("unused_nocmap");
    let src = Src::gen(&mut rng, PNG_COLOR_TYPE_RGB, 8, 6, 4, PNG_INTERLACE_NONE, false);
    let bytes = unsafe { encode(c_api(), &src) };
    for &(name, fmt) in FORMATS {
        if !f_is_cmap(fmt) {
            continue;
        }
        let res = read_diff(
            &format!("nocmap {name}"),
            &bytes,
            Sourc::Memory,
            &p,
            fmt,
            None,
            Stride::Default,
            false,
        );
        assert_eq!(res.finish.ret, 0, "{name}: must fail without a colormap");
        assert!(
            res.finish.msg.contains("no color-map"),
            "{name}: {:?}",
            res.finish.msg
        );
    }
}

// ---------------------------------------------------------------------------
// Write tests
// ---------------------------------------------------------------------------

/// `png_image_write_to_memory`: the size-query form (`memory == NULL`) and the
/// real call, every input format, `convert_to_8bit` both ways.
#[test]
fn write_every_format_to_memory() {
    let mut rng = Rng::new(0x79bd_f024_68ac_e009);
    let p = tmp("unused_wmem");
    for &(w, h) in &[(1u32, 1u32), (7, 5), (16, 3)] {
        for &(name, fmt) in FORMATS {
            for cme in [1u32, 2, 5, 17, 256] {
                if !f_is_cmap(fmt) && cme != 1 {
                    continue;
                }
                for convert8 in [0i32, 1] {
                    for flags in [0u32, PNG_IMAGE_FLAG_FAST, PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB] {
                        let stride = (w * f_pixel_channels(fmt)) as i32;
                        let win = WIn {
                            w,
                            h,
                            fmt,
                            flags,
                            cme: if f_is_cmap(fmt) { cme } else { 0 },
                            stride,
                            convert8,
                            buffer: gen_wbuf(&mut rng, h, fmt, stride, cme),
                            colormap: rng.bytes(f_cmap_bytes(fmt)),
                            pass_colormap: f_is_cmap(fmt),
                        };
                        let label = format!(
                            "wmem {w}x{h} fmt={name} cme={cme} c8={convert8} flags={flags:#x}"
                        );
                        let res = write_diff(&label, &win, Dest::Memory, &p);
                        assert_eq!(res.query.ret, 1, "{label}: size query failed");
                        assert!(res.query_size > 0, "{label}: zero size");
                        assert_eq!(res.write.ret, 1, "{label}: write failed");
                        assert_eq!(
                            res.written, res.query_size,
                            "{label}: size query != written"
                        );
                        assert_eq!(&res.bytes[..8], &PNG_SIG[..], "{label}: signature");
                    }
                }
            }
        }
    }
}

/// `png_image_write_to_file` / `png_image_write_to_stdio` produce exactly the
/// same bytes as `png_image_write_to_memory`.
#[test]
fn write_to_file_and_stdio() {
    let mut rng = Rng::new(0x8ace_0246_8ace_000b);
    let p = tmp("outfile");
    for &(w, h) in &[(1u32, 1u32), (9, 6)] {
        for &(name, fmt) in FORMATS {
            for convert8 in [0i32, 1] {
                let cme = 13u32;
                let stride = (w * f_pixel_channels(fmt)) as i32;
                let win = WIn {
                    w,
                    h,
                    fmt,
                    flags: 0,
                    cme: if f_is_cmap(fmt) { cme } else { 0 },
                    stride,
                    convert8,
                    buffer: gen_wbuf(&mut rng, h, fmt, stride, cme),
                    colormap: rng.bytes(f_cmap_bytes(fmt)),
                    pass_colormap: f_is_cmap(fmt),
                };
                let m = write_diff(
                    &format!("wmem {w}x{h} {name} c8={convert8}"),
                    &win,
                    Dest::Memory,
                    &p,
                );
                let f = write_diff(
                    &format!("wfile {w}x{h} {name} c8={convert8}"),
                    &win,
                    Dest::File,
                    &p,
                );
                let s = write_diff(
                    &format!("wstdio {w}x{h} {name} c8={convert8}"),
                    &win,
                    Dest::Stdio,
                    &p,
                );
                let lbl = format!("wdest {w}x{h} {name} c8={convert8}");
                assert_eq!(m.write.ret, f.write.ret, "{lbl}: file return");
                assert_eq!(m.write.ret, s.write.ret, "{lbl}: stdio return");
                assert_bytes_eq(&format!("{lbl}: memory vs file"), &m.bytes, &f.bytes);
                assert_bytes_eq(&format!("{lbl}: memory vs stdio"), &m.bytes, &s.bytes);
            }
        }
    }
    let _ = std::fs::remove_file(&p);
}

/// Negative, default and padded strides on the write side.
#[test]
fn write_row_strides() {
    let mut rng = Rng::new(0x9bdf_0246_8ace_000d);
    let p = tmp("unused_wstride");
    for &(name, fmt) in FORMATS {
        let (w, h) = (7u32, 5u32);
        let min = (w * f_pixel_channels(fmt)) as i32;
        for &st in &[0i32, min, -min, min + 4, -(min + 4), min - 1] {
            let abs = if st == 0 { min } else { st.abs() };
            let cme = 9u32;
            let win = WIn {
                w,
                h,
                fmt,
                flags: 0,
                cme: if f_is_cmap(fmt) { cme } else { 0 },
                stride: st,
                convert8: 0,
                buffer: gen_wbuf(&mut rng, h, fmt, abs.max(min), cme),
                colormap: rng.bytes(f_cmap_bytes(fmt)),
                pass_colormap: f_is_cmap(fmt),
            };
            let label = format!("wstride {name} stride={st}");
            let res = write_diff(&label, &win, Dest::Memory, &p);
            if st == min - 1 && min > 1 {
                assert_eq!(res.query.ret, 0, "{label}: must fail");
                assert!(
                    res.query.msg.contains("row stride too small"),
                    "{label}: {:?}",
                    res.query.msg
                );
            } else {
                assert_eq!(res.query.ret, 1, "{label}: {:?}", res.query.msg);
            }
        }
    }
}

/// `png_image_write_to_memory` with a buffer that is too small: the call
/// returns 0 but still reports the size that would have been required.
#[test]
fn write_to_memory_short_buffer() {
    let mut rng = Rng::new(0xace0_2468_ace0_000f);
    let mut out: Vec<(c_int, usize, String, png_uint_32)> = Vec::new();
    let (w, h) = (12u32, 9u32);
    let fmt = PNG_FORMAT_RGB;
    let stride = (w * 3) as i32;
    let buffer = rng.bytes(h as usize * stride as usize);
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let mut im = png_image {
                width: w,
                height: h,
                format: fmt,
                ..png_image::default()
            };
            let mut mb: usize = 4; // far too small
            let mut small = vec![0u8; 4];
            let r = (api.png_image_write_to_memory)(
                &mut im,
                small.as_mut_ptr() as *mut c_void,
                &mut mb,
                0,
                buffer.as_ptr() as *const c_void,
                stride,
                null(),
            );
            out.push((r, mb, msg_of(&im), im.warning_or_error));
            (api.png_image_free)(&mut im);
        }
    }
    assert_eq!(out[0], out[1], "short buffer: C vs RS");
    assert_eq!(out[0].0, 0, "short buffer must return 0");
    assert!(out[0].1 > 4, "short buffer must report the real size");
}

/// Read into a format, then write it straight back out and compare the PNG.
#[test]
fn round_trip_read_then_write() {
    let mut rng = Rng::new(0xbdf0_2468_ace0_0011);
    let p = tmp("unused_rt");
    let bg = png_color {
        red: 0x11,
        green: 0x22,
        blue: 0x33,
    };
    for &(ct, bd) in IHDRS {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let src = Src::gen(&mut rng, ct, bd, 10, 6, il, false);
            let bytes = unsafe { encode(c_api(), &src) };
            for &(name, fmt) in FORMATS {
                let label = format!("rt ct={ct} bd={bd} il={il} fmt={name}");
                let rd = read_diff(
                    &label,
                    &bytes,
                    Sourc::Memory,
                    &p,
                    fmt,
                    Some(bg),
                    Stride::Default,
                    true,
                );
                if rd.finish.ret == 0 {
                    continue;
                }
                let win = WIn {
                    w: rd.finish.w,
                    h: rd.finish.h,
                    fmt: rd.finish.fmt,
                    flags: rd.finish.flags,
                    cme: rd.finish.cme,
                    stride: (rd.finish.w * f_pixel_channels(rd.finish.fmt)) as png_int_32,
                    convert8: 0,
                    buffer: rd.buffer.clone(),
                    colormap: rd.colormap.clone(),
                    pass_colormap: f_is_cmap(rd.finish.fmt),
                };
                write_diff(&format!("{label} (write back)"), &win, Dest::Memory, &p);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Error / edge paths
// ---------------------------------------------------------------------------

/// All the documented argument checks of the simplified API.
#[test]
fn simplified_error_paths() {
    let mut rng = Rng::new(0xdf02_468a_ce00_0013);
    let src = Src::gen(&mut rng, PNG_COLOR_TYPE_RGB, 8, 6, 4, PNG_INTERLACE_NONE, false);
    let bytes = unsafe { encode(c_api(), &src) };
    let missing = tmp("does_not_exist_ever");
    let _ = std::fs::remove_file(&missing);
    let cmissing = CString::new(missing.to_str().unwrap()).unwrap();

    let mut all: Vec<Vec<(String, c_int, Snap)>> = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let mut log: Vec<(String, c_int, Snap)> = Vec::new();
            let mut rec = |name: &str, ret: c_int, im: &png_image| {
                log.push((name.to_string(), ret, snap(ret, im)));
            };

            // NULL image: every entry point returns 0 without touching memory.
            assert_eq!(
                (api.png_image_begin_read_from_memory)(
                    null_mut(),
                    bytes.as_ptr() as png_const_voidp,
                    bytes.len()
                ),
                0
            );
            assert_eq!(
                (api.png_image_finish_read)(null_mut(), null(), null_mut(), 0, null_mut()),
                0
            );
            (api.png_image_free)(null_mut());

            // begin_read_from_memory with no data.
            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_memory)(&mut im, null(), 0);
            rec("mem-null", r, &im);
            (api.png_image_free)(&mut im);

            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                bytes.as_ptr() as png_const_voidp,
                0,
            );
            rec("mem-zero", r, &im);
            (api.png_image_free)(&mut im);

            // Wrong version.
            let mut im = png_image {
                version: 99,
                ..png_image::default()
            };
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                bytes.as_ptr() as png_const_voidp,
                bytes.len(),
            );
            rec("mem-badversion", r, &im);
            let r = (api.png_image_finish_read)(&mut im, null(), null_mut(), 0, null_mut());
            rec("finish-badversion", r, &im);
            let mut mb: usize = 0;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                null_mut(),
                &mut mb,
                0,
                bytes.as_ptr() as *const c_void,
                0,
                null(),
            );
            rec("write-badversion", r, &im);
            (api.png_image_free)(&mut im);

            // Not a PNG at all.
            let junk = [0u8; 64];
            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                junk.as_ptr() as png_const_voidp,
                junk.len(),
            );
            rec("mem-junk", r, &im);
            (api.png_image_free)(&mut im);

            // Truncated PNG (header only).
            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                bytes.as_ptr() as png_const_voidp,
                30,
            );
            rec("mem-truncated", r, &im);
            (api.png_image_free)(&mut im);

            // finish_read with a NULL buffer.
            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                bytes.as_ptr() as png_const_voidp,
                bytes.len(),
            );
            rec("begin-ok", r, &im);
            let r = (api.png_image_finish_read)(&mut im, null(), null_mut(), 0, null_mut());
            rec("finish-nullbuf", r, &im);
            (api.png_image_free)(&mut im);

            // A file that does not exist.
            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_file)(&mut im, cmissing.as_ptr());
            rec("file-missing", r, &im);
            (api.png_image_free)(&mut im);

            // NULL file name / NULL FILE*.
            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_file)(&mut im, null());
            rec("file-nullname", r, &im);
            (api.png_image_free)(&mut im);

            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_stdio)(&mut im, null_mut());
            rec("stdio-nullfile", r, &im);
            (api.png_image_free)(&mut im);

            // write_to_memory argument checks.
            let mut im = png_image {
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGB,
                ..png_image::default()
            };
            let r = (api.png_image_write_to_memory)(
                &mut im,
                null_mut(),
                null_mut(),
                0,
                bytes.as_ptr() as *const c_void,
                0,
                null(),
            );
            rec("write-null-sizeptr", r, &im);
            let mut mb: usize = 0;
            let r = (api.png_image_write_to_memory)(
                &mut im, null_mut(), &mut mb, 0, null(), 0, null(),
            );
            rec("write-null-buffer", r, &im);
            (api.png_image_free)(&mut im);

            // A color-mapped write without a color-map.
            let cbuf = [0u8; 64];
            let mut im = png_image {
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGB_COLORMAP,
                colormap_entries: 4,
                ..png_image::default()
            };
            let mut mb: usize = 0;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                null_mut(),
                &mut mb,
                0,
                cbuf.as_ptr() as *const c_void,
                0,
                null(),
            );
            rec("write-cmap-nocmap", r, &im);
            (api.png_image_free)(&mut im);

            // ... and with zero entries.
            let cmap = [0u8; 3 * 256];
            let mut im = png_image {
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGB_COLORMAP,
                colormap_entries: 0,
                ..png_image::default()
            };
            let mut mb: usize = 0;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                null_mut(),
                &mut mb,
                0,
                cbuf.as_ptr() as *const c_void,
                0,
                cmap.as_ptr() as *const c_void,
            );
            rec("write-cmap-zeroentries", r, &im);
            (api.png_image_free)(&mut im);

            // png_image_error itself.
            let mut im = png_image::default();
            let m = cs("deliberate");
            let r = (api.png_image_error)(&mut im, m.as_ptr());
            rec("png_image_error", r, &im);

            let d = diag_take();
            assert_eq!(d, Diag::default(), "{}: unexpected diag {:?}", api.name, d);
            all.push(log);
        }
    }
    assert_eq!(all[0].len(), all[1].len());
    for (c, r) in all[0].iter().zip(all[1].iter()) {
        assert_eq!(c.0, r.0);
        assert_eq!(c.1, r.1, "{}: return value", c.0);
        assert_eq!(c.2, r.2, "{}: png_image state", c.0);
    }
    // Spot checks against the documented behaviour.
    let by = |n: &str| all[0].iter().find(|e| e.0 == n).unwrap();
    assert_eq!(by("mem-null").1, 0);
    assert!(by("mem-null").2.msg.contains("invalid argument"));
    assert_eq!(by("mem-badversion").1, 0);
    assert_eq!(by("begin-ok").1, 1);
    assert_eq!(by("finish-nullbuf").1, 0);
    assert!(by("finish-nullbuf").2.msg.contains("invalid argument"));
    assert_eq!(by("file-missing").1, 0);
    assert_eq!(by("write-cmap-nocmap").1, 0);
    assert_eq!(by("png_image_error").1, 0);
    assert_eq!(by("png_image_error").2.msg, "deliberate");
    assert_eq!(by("png_image_error").2.woe, PNG_IMAGE_ERROR);
}

//! CONFIGS.md rows 59-63: the libpng *simplified* API, C vs Rust.
//!
//! Every call goes through `dlsym` on both shared objects; the Rust crate is
//! never linked directly.  For each configuration we compare
//!
//!   * the return value of every simplified-API call,
//!   * the whole `png_image` struct except `opaque` (`cmp_key()`),
//!   * the ENTIRE output buffer (pre-filled with a non-zero pattern so that
//!     bytes libpng must not touch are compared too),
//!   * the colormap buffer (also pre-filled),
//!   * the produced PNG bytes for the write side.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod common;

use common::api;
use common::*;
use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::sync::OnceLock;

// `png_image_error` is a documented-internal but exported entry point; row 63
// drives it directly.  Declared locally so that tests/common/api.rs is
// untouched.
crate::decl_api! {
    fn png_image_error(image: *mut png_image, msg: png_const_charp) -> c_int;
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

// --------------------------------------------------------------------------
// png.h PNG_IMAGE_* macros, ported verbatim.
// --------------------------------------------------------------------------

fn sample_channels(fmt: u32) -> u32 {
    (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}
fn sample_component_size(fmt: u32) -> u32 {
    ((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
}
fn sample_size(fmt: u32) -> u32 {
    sample_channels(fmt) * sample_component_size(fmt)
}
fn pixel_channels(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_channels(fmt)
    }
}
fn pixel_component_size(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_component_size(fmt)
    }
}
/// PNG_IMAGE_ROW_STRIDE
fn row_stride_of(fmt: u32, width: u32) -> u32 {
    pixel_channels(fmt) * width
}
/// PNG_IMAGE_BUFFER_SIZE
fn buffer_size(fmt: u32, height: u32, row_stride: u32) -> usize {
    pixel_component_size(fmt) as usize * height as usize * row_stride as usize
}
/// PNG_IMAGE_SIZE
fn image_size(fmt: u32, width: u32, height: u32) -> usize {
    buffer_size(fmt, height, row_stride_of(fmt, width))
}
/// PNG_IMAGE_COLORMAP_SIZE
fn colormap_size(fmt: u32, colormap_entries: u32) -> usize {
    sample_size(fmt) as usize * colormap_entries as usize
}
/// PNG_IMAGE_MAXIMUM_COLORMAP_COMPONENTS, in bytes
fn colormap_max_bytes(fmt: u32) -> usize {
    sample_channels(fmt) as usize * 256 * sample_component_size(fmt) as usize
}

/// The reference C `libpng.so` in this tree was linked without `-lm`, so its
/// `pow`/`floor`/`frexp`/`modf` references are undefined.  `libloading` uses
/// `RTLD_LOCAL`, so the libm that the Rust `.so` pulls in is not visible to it
/// and the first gamma computation would abort the process with a symbol
/// lookup error.  Loading libm with `RTLD_GLOBAL` puts those symbols in the
/// global lookup scope, i.e. makes the C library behave exactly as a normally
/// linked build.  This changes no libpng behaviour.
fn L() -> &'static Libs {
    static M: OnceLock<()> = OnceLock::new();
    M.get_or_init(|| unsafe {
        use libloading::os::unix::{Library as OsLibrary, RTLD_GLOBAL, RTLD_NOW};
        let lm = OsLibrary::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL)
            .expect("dlopen libm.so.6 with RTLD_GLOBAL");
        std::mem::forget(lm);
    });
    libs()
}

const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: u32 = 0x01;
const PNG_IMAGE_FLAG_FAST: u32 = 0x02;
const PNG_IMAGE_FLAG_16BIT_sRGB: u32 = 0x04;

// --------------------------------------------------------------------------
// A 2-byte(+) aligned buffer pre-filled with a fixed non-zero pattern.
// --------------------------------------------------------------------------

fn pat(i: usize) -> u8 {
    let b = 0x5Bu8 ^ (i as u8).wrapping_mul(197);
    if b == 0 {
        0xFF
    } else {
        b
    }
}

/// True if any byte of `b` deviates from the pristine fill pattern, i.e. the
/// library actually produced output.  Used to prove the tests do real work.
fn touched(b: &[u8]) -> bool {
    b.iter().enumerate().any(|(i, &x)| x != pat(i))
}

struct Buf {
    v: Vec<u64>,
    alloc: usize,
}

impl Buf {
    /// `n` usable bytes plus at least one guard byte, the whole allocation
    /// filled with the fixed pattern.
    fn pattern(n: usize) -> Buf {
        let words = n / 8 + 1;
        let alloc = words * 8;
        let mut v = vec![0u64; words];
        unsafe {
            let p = v.as_mut_ptr() as *mut u8;
            for i in 0..alloc {
                *p.add(i) = pat(i);
            }
        }
        Buf { v, alloc }
    }
    fn from_bytes(b: &[u8]) -> Buf {
        let mut buf = Buf::pattern(b.len());
        unsafe {
            std::ptr::copy_nonoverlapping(b.as_ptr(), buf.v.as_mut_ptr() as *mut u8, b.len());
        }
        buf
    }
    fn ptr(&mut self) -> *mut c_void {
        self.v.as_mut_ptr() as *mut c_void
    }
    fn cptr(&self) -> *const c_void {
        self.v.as_ptr() as *const c_void
    }
    /// The complete allocation, guard bytes included.
    fn all(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.v.as_ptr() as *const u8, self.alloc).to_vec() }
    }
}

fn cmp_bytes(c: &[u8], r: &[u8], label: &str) {
    assert_eq!(
        c.len(),
        r.len(),
        "{label}: buffer length differs C={} RS={}",
        c.len(),
        r.len()
    );
    if let Some(i) = c.iter().zip(r.iter()).position(|(a, b)| a != b) {
        let lo = i.saturating_sub(4);
        let hi = (i + 12).min(c.len());
        panic!(
            "{label}: first differing byte at offset {i} of {}\n  C  [{lo}..{hi}] = {}\n  RS [{lo}..{hi}] = {}",
            c.len(),
            hex(&c[lo..hi]),
            hex(&r[lo..hi])
        );
    }
}

// --------------------------------------------------------------------------
// Building PNG inputs with the low-level C writer.
// --------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum Extra {
    None,
    SRgb,
    Gama(i32),
}

#[derive(Clone)]
struct Input {
    name: String,
    data: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
fn gen_png(
    color_type: c_int,
    bit_depth: c_int,
    w: u32,
    h: u32,
    interlace: c_int,
    trns: bool,
    extra: Extra,
    seed: u64,
) -> Vec<u8> {
    let l = &L().c;
    let mut rng = Rng::new(seed);
    sink_reset();
    log_clear();
    unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        assert!(!ip.is_null());
        api::png_set_IHDR(
            l,
            pp,
            ip,
            w,
            h,
            bit_depth,
            color_type,
            interlace,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );

        let nent: usize = 1usize << bit_depth;
        if color_type == PNG_COLOR_TYPE_PALETTE {
            let pal: Vec<png_color> = (0..nent)
                .map(|_| png_color {
                    red: rng.u8(),
                    green: rng.u8(),
                    blue: rng.u8(),
                })
                .collect();
            api::png_set_PLTE(l, pp, ip, pal.as_ptr(), nent as c_int);
            if trns {
                let nt = nent.min(5);
                let alphas: Vec<u8> = (0..nt).map(|i| (i as u8) * 60).collect();
                api::png_set_tRNS(
                    l,
                    pp,
                    ip,
                    alphas.as_ptr(),
                    nt as c_int,
                    std::ptr::null(),
                );
            }
        } else if trns {
            let mut tc = png_color_16::default();
            let maxv: u32 = (1u32 << bit_depth) - 1;
            if color_type == PNG_COLOR_TYPE_GRAY {
                tc.gray = (maxv / 2) as u16;
            } else {
                tc.red = (maxv / 3) as u16;
                tc.green = (maxv / 2) as u16;
                tc.blue = (maxv / 5) as u16;
            }
            api::png_set_tRNS(l, pp, ip, std::ptr::null(), 0, &tc);
        }

        match extra {
            Extra::None => {}
            Extra::SRgb => api::png_set_sRGB(l, pp, ip, 0),
            Extra::Gama(g) => api::png_set_gAMA_fixed(l, pp, ip, g),
        }

        api::png_write_info(l, pp, ip);
        let rowbytes = api::png_get_rowbytes(l, pp, ip);
        let mut rows: Vec<Vec<u8>> = (0..h as usize).map(|_| rng.bytes(rowbytes)).collect();
        let mut rp: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
        api::png_write_image(l, pp, rp.as_mut_ptr());
        api::png_write_end(l, pp, ip);
        let mut p = pp;
        let mut i = ip;
        api::png_destroy_write_struct(l, &mut p, &mut i);
    }
    let msgs = log_take();
    let (data, _) = sink_take();
    assert!(
        msgs.is_empty(),
        "input generation ct={color_type} bd={bit_depth} produced {msgs:?}"
    );
    assert!(!data.is_empty());
    data
}

const CTS: [(c_int, &[c_int], &str); 5] = [
    (PNG_COLOR_TYPE_GRAY, &[1, 2, 4, 8, 16], "G"),
    (PNG_COLOR_TYPE_RGB, &[8, 16], "RGB"),
    (PNG_COLOR_TYPE_PALETTE, &[1, 2, 4, 8], "P"),
    (PNG_COLOR_TYPE_GRAY_ALPHA, &[8, 16], "GA"),
    (PNG_COLOR_TYPE_RGB_ALPHA, &[8, 16], "RGBA"),
];

fn build_inputs() -> Vec<Input> {
    let mut v: Vec<Input> = Vec::new();
    let mut seed: u64 = 0x1234_5678_9abc_def1;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed | 1
    };

    for (ct, depths, cn) in CTS {
        for &bd in depths {
            for &(w, h, il) in &[(33u32, 5u32, 0i32), (33, 5, 1), (1, 1, 0)] {
                let data = gen_png(ct, bd, w, h, il, false, Extra::None, next());
                v.push(Input {
                    name: format!("{cn}{bd}_{w}x{h}_il{il}"),
                    data,
                });
            }
        }
    }
    // transparency (tRNS) variants -- these drive the background handling.
    for &(ct, bd, cn) in &[
        (PNG_COLOR_TYPE_PALETTE, 8, "P"),
        (PNG_COLOR_TYPE_PALETTE, 4, "P"),
        (PNG_COLOR_TYPE_GRAY, 8, "G"),
        (PNG_COLOR_TYPE_GRAY, 2, "G"),
        (PNG_COLOR_TYPE_RGB, 8, "RGB"),
        (PNG_COLOR_TYPE_RGB, 16, "RGB"),
    ] {
        for il in [0, 1] {
            let data = gen_png(ct, bd, 7, 4, il, true, Extra::None, next());
            v.push(Input {
                name: format!("{cn}{bd}_trns_il{il}"),
                data,
            });
        }
    }
    // Widths that leave a partially used final byte for sub-byte bit depths.
    for &(ct, bd, cn) in &[
        (PNG_COLOR_TYPE_GRAY, 1, "G"),
        (PNG_COLOR_TYPE_GRAY, 2, "G"),
        (PNG_COLOR_TYPE_GRAY, 4, "G"),
        (PNG_COLOR_TYPE_PALETTE, 1, "P"),
        (PNG_COLOR_TYPE_PALETTE, 2, "P"),
        (PNG_COLOR_TYPE_PALETTE, 4, "P"),
    ] {
        for w in [2u32, 3, 5, 7, 9] {
            let data = gen_png(ct, bd, w, 3, 0, false, Extra::None, next());
            v.push(Input {
                name: format!("{cn}{bd}_{w}x3_il0"),
                data,
            });
        }
    }
    // Tiny interlaced images: several Adam7 passes are completely empty.
    for &(ct, bd, cn) in &[
        (PNG_COLOR_TYPE_GRAY, 1, "G"),
        (PNG_COLOR_TYPE_PALETTE, 2, "P"),
        (PNG_COLOR_TYPE_RGB, 8, "RGB"),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16, "RGBA"),
    ] {
        for &(w, h) in &[(1u32, 1u32), (2, 3), (5, 1), (1, 7), (4, 4)] {
            let data = gen_png(ct, bd, w, h, 1, false, Extra::None, next());
            v.push(Input {
                name: format!("{cn}{bd}_{w}x{h}_il1"),
                data,
            });
        }
    }
    // colour-space variants -- drive the gamma / sRGB decision paths.
    for &(ct, bd, cn) in &[
        (PNG_COLOR_TYPE_RGB, 8, "RGB"),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16, "RGBA"),
        (PNG_COLOR_TYPE_GRAY, 8, "G"),
    ] {
        for (e, en) in [
            (Extra::SRgb, "srgb"),
            (Extra::Gama(45455), "g45455"),
            (Extra::Gama(100000), "glinear"),
            (Extra::Gama(20000), "g20000"),
            (Extra::Gama(1), "gtiny"),
            (Extra::Gama(1000000), "ghuge"),
            (Extra::Gama(500000), "g5"),
        ] {
            let data = gen_png(ct, bd, 9, 3, 0, false, e, next());
            v.push(Input {
                name: format!("{cn}{bd}_{en}"),
                data,
            });
        }
    }
    v
}

fn inputs() -> &'static Vec<Input> {
    static I: OnceLock<Vec<Input>> = OnceLock::new();
    I.get_or_init(build_inputs)
}

fn input(name: &str) -> &'static Input {
    inputs()
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no input named {name}"))
}

// --------------------------------------------------------------------------
// The output-format cross-product.
// --------------------------------------------------------------------------

const BASES: [(&str, u32); 9] = [
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

/// {GRAY,GA,AG,RGB,BGR,RGBA,ARGB,BGRA,ABGR} x {8-bit,LINEAR} x {direct,COLORMAP}
fn all_formats() -> Vec<(String, u32)> {
    let mut v = Vec::new();
    for (n, f) in BASES {
        for lin in [0u32, PNG_FORMAT_FLAG_LINEAR] {
            for cm in [0u32, PNG_FORMAT_FLAG_COLORMAP] {
                v.push((
                    format!(
                        "{n}{}{}",
                        if lin != 0 { "+LIN" } else { "" },
                        if cm != 0 { "+CMAP" } else { "" }
                    ),
                    f | lin | cm,
                ));
            }
        }
    }
    v
}

const BG: png_color = png_color {
    red: 0x2A,
    green: 0x93,
    blue: 0xC7,
};

const SMODES: [(i32, &str); 3] = [(0, "s0"), (1, "s+"), (-1, "s-")];

const BGS: [(&str, Option<png_color>); 4] = [
    ("bgNULL", None),
    ("bgMID", Some(BG)),
    ("bgBLACK", Some(png_color { red: 0, green: 0, blue: 0 })),
    ("bgWHITE", Some(png_color { red: 255, green: 255, blue: 255 })),
];


// --------------------------------------------------------------------------
// One differential read.
// --------------------------------------------------------------------------

#[derive(Debug)]
struct ReadOut {
    r_begin: c_int,
    key_begin: (u32, u32, u32, u32, u32, u32, u32, String),
    r_finish: c_int,
    key_finish: (u32, u32, u32, u32, u32, u32, u32, String),
    buf: Vec<u8>,
    cmap: Vec<u8>,
    /// what the harness computed for PNG_IMAGE_SIZE / _COLORMAP_SIZE
    sizes: (usize, usize),
    /// the output buffer was written to (not still the pristine pattern)
    buf_touched: bool,
}

#[allow(clippy::too_many_arguments)]
unsafe fn read_with<F>(
    l: &Library,
    begin: F,
    fmt: u32,
    smode: i32,
    stride_pad: u32,
    bg: Option<png_color>,
    flags: u32,
) -> ReadOut
where
    F: FnOnce(*mut png_image) -> c_int,
{
    let mut img = png_image {
        version: PNG_IMAGE_VERSION,
        ..Default::default()
    };
    let r_begin = begin(&mut img);
    let key_begin = img.cmp_key();
    if r_begin == 0 {
        return ReadOut {
            r_begin,
            key_begin: key_begin.clone(),
            r_finish: i32::MIN,
            key_finish: key_begin,
            buf: Vec::new(),
            cmap: Vec::new(),
            sizes: (0, 0),
            buf_touched: false,
        };
    }

    img.format = fmt;
    img.flags |= flags;

    let nat = row_stride_of(fmt, img.width) + stride_pad;
    // smode 0 exercises the `row_stride == 0` default (libpng then uses
    // PNG_IMAGE_ROW_STRIDE itself); the buffer is still sized for `nat`.
    let stride: i32 = match smode {
        0 => 0,
        m if m > 0 => nat as i32,
        _ => -(nat as i32),
    };
    let nbytes = buffer_size(fmt, img.height, nat);
    let cmap_bytes = colormap_max_bytes(fmt);

    let mut buf = Buf::pattern(nbytes);
    let mut cmap = Buf::pattern(cmap_bytes);
    let bgp: *const png_color = match &bg {
        Some(c) => c as *const png_color,
        None => std::ptr::null(),
    };

    let r_finish = api::png_image_finish_read(l, &mut img, bgp, buf.ptr(), stride, cmap.ptr());
    let key_finish = img.cmp_key();
    let sizes = (
        image_size(img.format, img.width, img.height),
        colormap_size(img.format, img.colormap_entries),
    );
    // Free again: after finish_read the image is already freed, this must be a
    // no-op in both libraries.
    api::png_image_free(l, &mut img);

    let bytes = buf.all();
    ReadOut {
        r_begin,
        key_begin,
        r_finish,
        key_finish,
        buf_touched: touched(&bytes),
        buf: bytes,
        cmap: cmap.all(),
        sizes,
    }
}

unsafe fn read_mem(
    l: &Library,
    data: &[u8],
    fmt: u32,
    smode: i32,
    stride_pad: u32,
    bg: Option<png_color>,
    flags: u32,
) -> ReadOut {
    read_with(
        l,
        |img| {
            api::png_image_begin_read_from_memory(
                l,
                img,
                data.as_ptr() as *const c_void,
                data.len(),
            )
        },
        fmt,
        smode,
        stride_pad,
        bg,
        flags,
    )
}

/// (begin_read succeeded, finish_read succeeded, output buffer written)
#[derive(Default, Debug)]
struct Tally {
    total: u32,
    begin_ok: u32,
    finish_ok: u32,
    wrote: u32,
}

impl Tally {
    fn add(&mut self, c: &Run<ReadOut>) {
        self.total += 1;
        if let Some(o) = &c.out {
            if o.r_begin != 0 {
                self.begin_ok += 1;
            }
            if o.r_finish == 1 {
                self.finish_ok += 1;
            }
            if o.buf_touched {
                self.wrote += 1;
            }
        }
    }
    /// Guard against a test that silently exercises nothing.
    fn check(&self, what: &str, min_finish_pct: u32) {
        eprintln!("[{what}] {self:?}");
        assert!(self.total > 0, "{what}: no configurations were exercised");
        assert!(
            self.finish_ok * 100 >= self.total * min_finish_pct,
            "{what}: only {}/{} configurations decoded successfully -- the test is not exercising the decoder",
            self.finish_ok,
            self.total
        );
        assert!(
            self.wrote <= self.finish_ok,
            "{what}: a failed decode wrote to the output buffer"
        );
        assert!(
            self.wrote * 100 >= self.finish_ok * 90,
            "{what}: {} successful decodes but only {} touched the output buffer",
            self.finish_ok,
            self.wrote
        );
    }
}

fn cmp_read(c: &Run<ReadOut>, r: &Run<ReadOut>, cfg: &str) {
    assert_eq!(
        c.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        r.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        "{cfg}: harness message log differs (C first)"
    );
    assert_eq!(
        c.out.is_some(),
        r.out.is_some(),
        "{cfg}: exactly one library unwound (C unwound = {})",
        c.out.is_none()
    );
    let (a, b) = match (&c.out, &r.out) {
        (Some(a), Some(b)) => (a, b),
        _ => return,
    };
    assert_eq!(a.r_begin, b.r_begin, "{cfg}: begin_read return value");
    assert_eq!(
        a.key_begin, b.key_begin,
        "{cfg}: png_image after begin_read (version,w,h,format,flags,cmap_entries,warn_or_err,msg)"
    );
    assert_eq!(
        a.r_finish, b.r_finish,
        "{cfg}: finish_read return value\n  C  image = {:?}\n  RS image = {:?}",
        a.key_finish, b.key_finish
    );
    assert_eq!(
        a.key_finish, b.key_finish,
        "{cfg}: png_image after finish_read"
    );
    assert_eq!(a.sizes, b.sizes, "{cfg}: PNG_IMAGE_SIZE/COLORMAP_SIZE");
    assert_eq!(
        a.buf_touched, b.buf_touched,
        "{cfg}: one library wrote to the output buffer and the other did not"
    );
    cmp_bytes(&a.buf, &b.buf, &format!("{cfg}: output buffer"));
    cmp_bytes(&a.cmap, &b.cmap, &format!("{cfg}: colormap"));
}

// ==========================================================================
// Row 59 -- simplified read from memory.
// ==========================================================================

/// Bigger images: more rows, wider rows, all seven Adam7 passes populated.
fn large_inputs() -> &'static Vec<Input> {
    static I: OnceLock<Vec<Input>> = OnceLock::new();
    I.get_or_init(|| {
        let mut seed: u64 = 0xDEAD_BEEF_CAFE_0001;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            seed | 1
        };
        let mut v = Vec::new();
        for &(ct, bd, cn, w, h, il) in &[
            (PNG_COLOR_TYPE_GRAY, 8, "G", 64u32, 48u32, 0i32),
            (PNG_COLOR_TYPE_GRAY, 16, "G", 47, 33, 1),
            (PNG_COLOR_TYPE_PALETTE, 8, "P", 64, 48, 1),
            (PNG_COLOR_TYPE_RGB, 8, "RGB", 64, 48, 0),
            (PNG_COLOR_TYPE_RGB_ALPHA, 8, "RGBA", 47, 33, 1),
            (PNG_COLOR_TYPE_RGB_ALPHA, 16, "RGBA", 64, 48, 0),
        ] {
            v.push(Input {
                name: format!("{cn}{bd}_{w}x{h}_il{il}_big"),
                data: gen_png(ct, bd, w, h, il, false, Extra::None, next()),
            });
        }
        v
    })
}

#[test]
fn cfg59_read_from_memory_full_matrix() {
    // Every generated PNG shape x the complete output cross-product:
    //   {GRAY,GA,AG,RGB,BGR,RGBA,ARGB,BGRA,ABGR}
    //     x {8-bit, LINEAR} x {direct, COLORMAP}
    //     x row_stride {natural +, natural -} x background {NULL, set}
    let fmts = all_formats();
    let mut t = Tally::default();
    for inp in inputs() {
        for (fname, fmt) in &fmts {
            for (sm, smn) in SMODES {
                for (bgn, bg) in BGS {
                    let cfg =
                        format!("cfg59 mem in={} out={fname} {smn} {bgn}", inp.name);
                    let c =
                        capture(|| unsafe { read_mem(&L().c, &inp.data, *fmt, sm, 0, bg, 0) });
                    let r =
                        capture(|| unsafe { read_mem(&L().rs, &inp.data, *fmt, sm, 0, bg, 0) });
                    cmp_read(&c, &r, &cfg);
                    t.add(&c);
                }
            }
        }
    }
    assert_eq!(
        t.total,
        inputs().len() as u32 * 36 * 3 * 4,
        "cfg59: configuration count"
    );
    assert_eq!(t.begin_ok, t.total, "cfg59: every input must open");
    t.check("cfg59 full matrix", 70);
}

#[test]
fn cfg59_read_from_memory_large_images() {
    let fmts = all_formats();
    let mut t = Tally::default();
    for inp in large_inputs() {
        for (fname, fmt) in &fmts {
            for (sm, smn) in SMODES {
                for (bgn, bg) in BGS {
                    let cfg = format!("cfg59 big in={} out={fname} {smn} {bgn}", inp.name);
                    let c =
                        capture(|| unsafe { read_mem(&L().c, &inp.data, *fmt, sm, 0, bg, 0) });
                    let r =
                        capture(|| unsafe { read_mem(&L().rs, &inp.data, *fmt, sm, 0, bg, 0) });
                    cmp_read(&c, &r, &cfg);
                    t.add(&c);
                }
            }
        }
    }
    assert_eq!(
        t.total,
        large_inputs().len() as u32 * 36 * 3 * 4,
        "cfg59 big: configuration count"
    );
    assert_eq!(t.begin_ok, t.total, "cfg59 big: every input must open");
    t.check("cfg59 large images", 70);
}

#[test]
fn cfg59_read_from_memory_padded_stride_and_flags() {
    let fmts = all_formats();
    let mut t = Tally::default();
    for iname in ["G8_33x5_il0", "P8_33x5_il0", "RGBA16_33x5_il0", "RGB8_srgb"] {
        let inp = input(iname);
        for (fname, fmt) in &fmts {
            for pad in [1u32, 7] {
                for sm in [1i32, -1] {
                    for flags in [0u32, PNG_IMAGE_FLAG_16BIT_sRGB] {
                        let cfg = format!(
                            "cfg59 pad in={iname} out={fname} pad={pad} smode={sm} flags={flags:#x}"
                        );
                        let c = capture(|| unsafe {
                            read_mem(&L().c, &inp.data, *fmt, sm, pad, Some(BG), flags)
                        });
                        let r = capture(|| unsafe {
                            read_mem(&L().rs, &inp.data, *fmt, sm, pad, Some(BG), flags)
                        });
                        cmp_read(&c, &r, &cfg);
                        t.add(&c);
                    }
                }
            }
        }
    }
    assert_eq!(t.total, 4 * 36 * 2 * 2 * 2, "cfg59 pad: configuration count");
    t.check("cfg59 padded stride", 95);
}

#[test]
fn cfg59_read_from_memory_truncated_and_corrupt() {
    // Broken inputs: both libraries must fail identically.
    let base = &input("RGBA8_33x5_il0").data;
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("empty".into(), Vec::new()));
    cases.push(("sig_only".into(), base[..8].to_vec()));
    for cut in [10usize, 20, 33, 40, 60] {
        if cut < base.len() {
            cases.push((format!("trunc{cut}"), base[..cut].to_vec()));
        }
    }
    let mut rng = Rng::new(0xBADF00D);
    for k in 0..8 {
        let mut d = base.clone();
        let i = 8 + rng.below((d.len() - 8) as u32) as usize;
        d[i] ^= 0x40;
        cases.push((format!("flip{k}@{i}"), d));
    }
    cases.push(("not_png".into(), b"hello world, not a PNG at all".to_vec()));

    let (mut n, mut ok) = (0u32, 0u32);
    for (name, data) in &cases {
        for (fname, fmt) in [
            ("RGBA", PNG_FORMAT_RGBA),
            ("GRAY", PNG_FORMAT_GRAY),
            ("RGB+CMAP", PNG_FORMAT_RGB_COLORMAP),
            ("LINEAR_RGBA", PNG_FORMAT_LINEAR_RGB_ALPHA),
        ] {
            let cfg = format!("cfg59 bad in={name} out={fname}");
            let c = capture(|| unsafe { read_mem(&L().c, data, fmt, 1, 0, Some(BG), 0) });
            let r = capture(|| unsafe { read_mem(&L().rs, data, fmt, 1, 0, Some(BG), 0) });
            cmp_read(&c, &r, &cfg);
            n += 1;
            if let Some(o) = &c.out {
                if o.r_finish == 1 {
                    ok += 1;
                }
            }
        }
    }
    eprintln!("[cfg59 corrupt] {n} cases, {ok} still decoded");
    assert!(n >= 60, "cfg59 corrupt: too few cases ({n})");
    assert!(ok < n, "cfg59 corrupt: no case actually failed");
}

// ==========================================================================
// Row 60 -- simplified read from file and from stdio.
// ==========================================================================

fn tmp_path(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("t_simple_{}_{tag}.png", std::process::id()))
}

unsafe fn read_file(
    l: &Library,
    path: &std::path::Path,
    fmt: u32,
    smode: i32,
    bg: Option<png_color>,
) -> ReadOut {
    let cp = CString::new(path.to_str().unwrap()).unwrap();
    read_with(
        l,
        |img| api::png_image_begin_read_from_file(l, img, cp.as_ptr()),
        fmt,
        smode,
        0,
        bg,
        0,
    )
}

unsafe fn read_stdio(
    l: &Library,
    path: &std::path::Path,
    fmt: u32,
    smode: i32,
    bg: Option<png_color>,
) -> ReadOut {
    let cp = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("rb").unwrap();
    let f = fopen(cp.as_ptr(), mode.as_ptr());
    assert!(!f.is_null(), "fopen {} failed", path.display());
    let out = read_with(
        l,
        |img| api::png_image_begin_read_from_stdio(l, img, f),
        fmt,
        smode,
        0,
        bg,
        0,
    );
    fclose(f);
    out
}

#[test]
fn cfg60_read_from_file_and_stdio() {
    let names = [
        "G8_33x5_il0",
        "G16_33x5_il1",
        "P8_33x5_il0",
        "P4_trns_il0",
        "RGB8_33x5_il0",
        "RGBA16_33x5_il0",
        "GA8_1x1_il0",
    ];
    let fmts: Vec<(String, u32)> = all_formats();
    let mut tf = Tally::default();
    let mut ts = Tally::default();

    for (k, iname) in names.iter().enumerate() {
        let inp = input(iname);
        let path = tmp_path(&format!("r60_{k}"));
        std::fs::write(&path, &inp.data).unwrap();

        for (fname, fmt) in &fmts {
            for (sm, smn) in SMODES {
                for (bgn, bg) in BGS {
                    let tag = format!("in={iname} out={fname} {smn} {bgn}");

                    let cf = capture(|| unsafe { read_file(&L().c, &path, *fmt, sm, bg) });
                    let rf = capture(|| unsafe { read_file(&L().rs, &path, *fmt, sm, bg) });
                    cmp_read(&cf, &rf, &format!("cfg60 file {tag}"));
                    tf.add(&cf);

                    let cs = capture(|| unsafe { read_stdio(&L().c, &path, *fmt, sm, bg) });
                    let rs = capture(|| unsafe { read_stdio(&L().rs, &path, *fmt, sm, bg) });
                    cmp_read(&cs, &rs, &format!("cfg60 stdio {tag}"));
                    ts.add(&cs);

                    // file / stdio / memory must all agree within each library.
                    let cm =
                        capture(|| unsafe { read_mem(&L().c, &inp.data, *fmt, sm, 0, bg, 0) });
                    cmp_read(&cf, &cm, &format!("cfg60 C file-vs-memory {tag}"));
                    cmp_read(&cs, &cm, &format!("cfg60 C stdio-vs-memory {tag}"));
                    let rm =
                        capture(|| unsafe { read_mem(&L().rs, &inp.data, *fmt, sm, 0, bg, 0) });
                    cmp_read(&rf, &rm, &format!("cfg60 RS file-vs-memory {tag}"));
                    cmp_read(&rs, &rm, &format!("cfg60 RS stdio-vs-memory {tag}"));
                }
            }
        }
        let _ = std::fs::remove_file(&path);
    }
    assert_eq!(tf.total, names.len() as u32 * 36 * 3 * 4, "cfg60: config count");
    assert_eq!(tf.begin_ok, tf.total, "cfg60: every file must open");
    assert_eq!(ts.begin_ok, ts.total, "cfg60: every stdio stream must open");
    tf.check("cfg60 from_file", 70);
    ts.check("cfg60 from_stdio", 70);
}

#[test]
fn cfg60_read_from_missing_file() {
    let path = tmp_path("r60_missing_does_not_exist");
    let _ = std::fs::remove_file(&path);
    let c = capture(|| unsafe { read_file(&L().c, &path, PNG_FORMAT_RGBA, 1, None) });
    let r = capture(|| unsafe { read_file(&L().rs, &path, PNG_FORMAT_RGBA, 1, None) });
    cmp_read(&c, &r, "cfg60 missing file");
    assert_eq!(c.out.as_ref().unwrap().r_begin, 0, "expected failure");

    // A file that exists but is not a PNG.
    let path2 = tmp_path("r60_notpng");
    std::fs::write(&path2, b"not a png").unwrap();
    let c = capture(|| unsafe { read_file(&L().c, &path2, PNG_FORMAT_RGBA, 1, None) });
    let r = capture(|| unsafe { read_file(&L().rs, &path2, PNG_FORMAT_RGBA, 1, None) });
    cmp_read(&c, &r, "cfg60 non-png file");
    let cs = capture(|| unsafe { read_stdio(&L().c, &path2, PNG_FORMAT_RGBA, 1, None) });
    let rs = capture(|| unsafe { read_stdio(&L().rs, &path2, PNG_FORMAT_RGBA, 1, None) });
    cmp_read(&cs, &rs, "cfg60 non-png stdio");
    let _ = std::fs::remove_file(&path2);
}

// ==========================================================================
// Row 61 -- simplified write to memory.
// ==========================================================================

#[derive(Clone, Copy, Debug)]
struct WCfg {
    fmt: u32,
    w: u32,
    h: u32,
    entries: u32,
    /// -1 = negative row_stride, 0 = pass 0 (libpng defaults it), 1 = positive
    smode: i32,
    pad: u32,
    conv8: c_int,
    flags: u32,
}

#[derive(Debug)]
struct WriteOut {
    r: c_int,
    mb: usize,
    png: Vec<u8>,
    key: (u32, u32, u32, u32, u32, u32, u32, String),
}

/// mode: None = size query (memory == NULL); Some(n) = buffer of n bytes.
unsafe fn write_mem(
    l: &Library,
    cfg: &WCfg,
    data: &[u8],
    cmap: &[u8],
    mode: Option<usize>,
) -> WriteOut {
    let mut img = png_image {
        version: PNG_IMAGE_VERSION,
        width: cfg.w,
        height: cfg.h,
        format: cfg.fmt,
        flags: cfg.flags,
        colormap_entries: cfg.entries,
        ..Default::default()
    };
    let stride_abs = row_stride_of(cfg.fmt, cfg.w) + cfg.pad;
    let stride: i32 = match cfg.smode {
        0 => 0,
        m if m > 0 => stride_abs as i32,
        _ => -(stride_abs as i32),
    };
    let inbuf = Buf::from_bytes(data);
    let cmbuf = Buf::from_bytes(cmap);

    let mut out = mode.map(Buf::pattern);
    let mut mb: usize = mode.unwrap_or(0);
    let memp: *mut c_void = match &mut out {
        Some(b) => b.ptr(),
        None => std::ptr::null_mut(),
    };
    let r = api::png_image_write_to_memory(
        l,
        &mut img,
        memp,
        &mut mb,
        cfg.conv8,
        inbuf.cptr(),
        stride,
        cmbuf.cptr(),
    );
    let key = img.cmp_key();
    api::png_image_free(l, &mut img);
    WriteOut {
        r,
        mb,
        png: out.map(|b| b.all()).unwrap_or_default(),
        key,
    }
}

fn cmp_write(c: &Run<WriteOut>, r: &Run<WriteOut>, cfg: &str) {
    assert_eq!(
        c.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        r.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        "{cfg}: harness message log differs (C first)"
    );
    assert_eq!(
        c.out.is_some(),
        r.out.is_some(),
        "{cfg}: exactly one library unwound (C unwound = {})",
        c.out.is_none()
    );
    let (a, b) = match (&c.out, &r.out) {
        (Some(a), Some(b)) => (a, b),
        _ => return,
    };
    assert_eq!(
        a.r, b.r,
        "{cfg}: return value\n  C  image = {:?}\n  RS image = {:?}",
        a.key, b.key
    );
    assert_eq!(
        a.mb, b.mb,
        "{cfg}: *memory_bytes\n  C  image = {:?}\n  RS image = {:?}",
        a.key, b.key
    );
    assert_eq!(a.key, b.key, "{cfg}: png_image after write");
    cmp_bytes(&a.png, &b.png, &format!("{cfg}: PNG bytes"));
}

fn wcfg_data(cfg: &WCfg, rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let stride_abs = row_stride_of(cfg.fmt, cfg.w) + cfg.pad;
    let n = buffer_size(cfg.fmt, cfg.h, stride_abs);
    let data: Vec<u8> = if cfg.fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        (0..n).map(|_| rng.below(cfg.entries.max(1)) as u8).collect()
    } else {
        rng.bytes(n)
    };
    let cmap = rng.bytes(colormap_max_bytes(cfg.fmt));
    (data, cmap)
}

fn run_write_cfg(cfg: &WCfg, tag: &str, rng: &mut Rng) -> Option<usize> {
    let (data, cmap) = wcfg_data(cfg, rng);

    // (a) size query
    let cq = capture(|| unsafe { write_mem(&L().c, cfg, &data, &cmap, None) });
    let rq = capture(|| unsafe { write_mem(&L().rs, cfg, &data, &cmap, None) });
    cmp_write(&cq, &rq, &format!("{tag} query"));

    let size = match &cq.out {
        Some(o) if o.r != 0 && o.mb > 0 => o.mb,
        // The configuration is rejected by both libraries (already compared).
        _ => return None,
    };

    // (b) exact-size buffer
    let ce = capture(|| unsafe { write_mem(&L().c, cfg, &data, &cmap, Some(size)) });
    let re = capture(|| unsafe { write_mem(&L().rs, cfg, &data, &cmap, Some(size)) });
    cmp_write(&ce, &re, &format!("{tag} exact({size})"));
    if let Some(o) = &ce.out {
        assert_eq!(o.r, 1, "{tag}: exact-size write should succeed");
        assert_eq!(o.mb, size, "{tag}: exact-size *memory_bytes");
        assert_eq!(
            &o.png[..8],
            b"\x89PNG\r\n\x1a\n",
            "{tag}: output does not start with the PNG signature"
        );
    }

    // (c) one byte too small
    let cs = capture(|| unsafe { write_mem(&L().c, cfg, &data, &cmap, Some(size - 1)) });
    let rs = capture(|| unsafe { write_mem(&L().rs, cfg, &data, &cmap, Some(size - 1)) });
    cmp_write(&cs, &rs, &format!("{tag} short({})", size - 1));
    if let Some(o) = &cs.out {
        assert_eq!(o.r, 0, "{tag}: one-byte-short write must report failure");
        assert_eq!(o.mb, size, "{tag}: short write must still report the full size");
    }
    Some(size)
}

#[test]
fn cfg61_write_to_memory_matrix() {
    let fmts = all_formats();
    let mut rng = Rng::new(0x11223344);
    let entries_cycle = [256u32, 17, 5, 2, 1, 3];
    let mut ei = 0usize;
    let (mut total, mut ok) = (0u32, 0u32);

    for (fname, fmt) in &fmts {
        let linear = fmt & PNG_FORMAT_FLAG_LINEAR != 0 && fmt & PNG_FORMAT_FLAG_COLORMAP == 0;
        for &(w, h) in &[(1u32, 1u32), (7, 3), (32, 32), (64, 40)] {
            for &conv8 in if fmt & PNG_FORMAT_FLAG_LINEAR != 0 {
                &[0 as c_int, 1][..]
            } else {
                &[0 as c_int][..]
            } {
                for (sm, smn) in SMODES {
                    let entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                        ei += 1;
                        entries_cycle[ei % entries_cycle.len()]
                    } else {
                        0
                    };
                    let cfg = WCfg {
                        fmt: *fmt,
                        w,
                        h,
                        entries,
                        smode: sm,
                        pad: 0,
                        conv8,
                        flags: 0,
                    };
                    let tag = format!(
                        "cfg61 mem fmt={fname} {w}x{h} conv8={conv8} {smn} entries={entries} linear={linear}"
                    );
                    total += 1;
                    if run_write_cfg(&cfg, &tag, &mut rng).is_some() {
                        ok += 1;
                    }
                }
            }
        }
    }
    eprintln!("[cfg61 matrix] {total} configurations, {ok} produced a PNG");
    // 4 sizes x 3 stride modes x (18 LINEAR formats x 2 convert_to_8bit
    // + 18 non-linear formats x 1)
    assert_eq!(total, 4 * 3 * (18 * 2 + 18), "cfg61: configuration count");
    assert_eq!(ok, total, "cfg61: every configuration must produce a PNG");
}

#[test]
fn cfg61_write_to_memory_padded_stride_and_flags() {
    let fmts = all_formats();
    let mut rng = Rng::new(0x55667788);
    let (mut total, mut ok) = (0u32, 0u32);
    for (fname, fmt) in &fmts {
        for &pad in &[3u32, 8] {
            for sm in [1i32, -1] {
                for &flags in &[
                    0u32,
                    PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB,
                    PNG_IMAGE_FLAG_FAST,
                    PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB | PNG_IMAGE_FLAG_FAST,
                ] {
                    let entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 200 } else { 0 };
                    let cfg = WCfg {
                        fmt: *fmt,
                        w: 9,
                        h: 4,
                        entries,
                        smode: sm,
                        pad,
                        conv8: (pad & 1) as c_int,
                        flags,
                    };
                    let tag =
                        format!("cfg61 pad fmt={fname} pad={pad} smode={sm} flags={flags:#x}");
                    total += 1;
                    if run_write_cfg(&cfg, &tag, &mut rng).is_some() {
                        ok += 1;
                    }
                }
            }
        }
    }
    eprintln!("[cfg61 padded] {total} configurations, {ok} produced a PNG");
    assert_eq!(total, 36 * 2 * 2 * 4, "cfg61 pad: configuration count");
    assert_eq!(ok, total, "cfg61 pad: every configuration must produce a PNG");
}

#[test]
fn cfg61_write_to_memory_colormap_entry_counts() {
    let mut rng = Rng::new(0x99AABBCC);
    let (mut total, mut ok) = (0u32, 0u32);
    for (fname, base) in BASES {
        for lin in [0u32, PNG_FORMAT_FLAG_LINEAR] {
            let fmt = base | lin | PNG_FORMAT_FLAG_COLORMAP;
            for entries in [1u32, 2, 3, 4, 5, 16, 17, 255, 256] {
                for (sm, smn) in SMODES {
                    let cfg = WCfg {
                        fmt,
                        w: 11,
                        h: 3,
                        entries,
                        smode: sm,
                        pad: 0,
                        conv8: 0,
                        flags: 0,
                    };
                    let tag = format!(
                        "cfg61 cmap fmt={fname}{}+CMAP entries={entries} {smn}",
                        if lin != 0 { "+LIN" } else { "" }
                    );
                    total += 1;
                    if run_write_cfg(&cfg, &tag, &mut rng).is_some() {
                        ok += 1;
                    }
                }
            }
        }
    }
    eprintln!("[cfg61 cmap] {total} configurations, {ok} produced a PNG");
    assert_eq!(total, 9 * 2 * 9 * 3, "cfg61 cmap: configuration count");
    assert_eq!(ok, total, "cfg61 cmap: every configuration must produce a PNG");
}

// ==========================================================================
// Row 62 -- simplified write to file and to stdio.
// ==========================================================================

unsafe fn write_file(
    l: &Library,
    cfg: &WCfg,
    data: &[u8],
    cmap: &[u8],
    path: &std::path::Path,
) -> (c_int, (u32, u32, u32, u32, u32, u32, u32, String)) {
    let mut img = png_image {
        version: PNG_IMAGE_VERSION,
        width: cfg.w,
        height: cfg.h,
        format: cfg.fmt,
        flags: cfg.flags,
        colormap_entries: cfg.entries,
        ..Default::default()
    };
    let stride_abs = row_stride_of(cfg.fmt, cfg.w) + cfg.pad;
    let stride: i32 = match cfg.smode {
        0 => 0,
        m if m > 0 => stride_abs as i32,
        _ => -(stride_abs as i32),
    };
    let inbuf = Buf::from_bytes(data);
    let cmbuf = Buf::from_bytes(cmap);
    let cp = CString::new(path.to_str().unwrap()).unwrap();
    let r = api::png_image_write_to_file(
        l,
        &mut img,
        cp.as_ptr(),
        cfg.conv8,
        inbuf.cptr(),
        stride,
        cmbuf.cptr(),
    );
    let key = img.cmp_key();
    api::png_image_free(l, &mut img);
    (r, key)
}

unsafe fn write_stdio(
    l: &Library,
    cfg: &WCfg,
    data: &[u8],
    cmap: &[u8],
    path: &std::path::Path,
) -> (c_int, (u32, u32, u32, u32, u32, u32, u32, String)) {
    let mut img = png_image {
        version: PNG_IMAGE_VERSION,
        width: cfg.w,
        height: cfg.h,
        format: cfg.fmt,
        flags: cfg.flags,
        colormap_entries: cfg.entries,
        ..Default::default()
    };
    let stride_abs = row_stride_of(cfg.fmt, cfg.w) + cfg.pad;
    let stride: i32 = match cfg.smode {
        0 => 0,
        m if m > 0 => stride_abs as i32,
        _ => -(stride_abs as i32),
    };
    let inbuf = Buf::from_bytes(data);
    let cmbuf = Buf::from_bytes(cmap);
    let cp = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("wb").unwrap();
    let f = fopen(cp.as_ptr(), mode.as_ptr());
    assert!(!f.is_null(), "fopen(w) {} failed", path.display());
    let r = api::png_image_write_to_stdio(
        l,
        &mut img,
        f,
        cfg.conv8,
        inbuf.cptr(),
        stride,
        cmbuf.cptr(),
    );
    let key = img.cmp_key();
    api::png_image_free(l, &mut img);
    fclose(f);
    (r, key)
}

#[test]
fn cfg62_write_to_file_and_stdio() {
    let mut rng = Rng::new(0xFEDCBA98);
    let mut cfgs: Vec<(String, WCfg)> = Vec::new();
    for &(w, h) in &[(13u32, 6u32), (1, 1)] {
        for (fname, base) in BASES {
            for lin in [0u32, PNG_FORMAT_FLAG_LINEAR] {
                for cm in [0u32, PNG_FORMAT_FLAG_COLORMAP] {
                    let fmt = base | lin | cm;
                    let sm = ((fname.len() + lin as usize + cm as usize + w as usize) % 3) as i32
                        - 1;
                    cfgs.push((
                        format!(
                            "{fname}{}{} {w}x{h}",
                            if lin != 0 { "+LIN" } else { "" },
                            if cm != 0 { "+CMAP" } else { "" }
                        ),
                        WCfg {
                            fmt,
                            w,
                            h,
                            entries: if cm != 0 { 37 } else { 0 },
                            smode: sm,
                            pad: 0,
                            conv8: if lin != 0 { 1 } else { 0 },
                            flags: 0,
                        },
                    ));
                }
            }
        }
    }

    let mut nonempty = 0u32;
    for (i, (fname, cfg)) in cfgs.iter().enumerate() {
        let (data, cmap) = wcfg_data(cfg, &mut rng);
        let pc = tmp_path(&format!("w62_c_{i}"));
        let pr = tmp_path(&format!("w62_r_{i}"));
        let pcs = tmp_path(&format!("w62_cs_{i}"));
        let prs = tmp_path(&format!("w62_rs_{i}"));

        let c = capture(|| unsafe { write_file(&L().c, cfg, &data, &cmap, &pc) });
        let r = capture(|| unsafe { write_file(&L().rs, cfg, &data, &cmap, &pr) });
        assert_eq!(
            c.out.is_some(),
            r.out.is_some(),
            "cfg62 file fmt={fname}: exactly one library unwound"
        );
        if let (Some(a), Some(b)) = (&c.out, &r.out) {
            assert_eq!(
                a.0, b.0,
                "cfg62 file fmt={fname}: write_to_file return\n  C  {:?}\n  RS {:?}",
                a.1, b.1
            );
            assert_eq!(a.1, b.1, "cfg62 file fmt={fname}: png_image after write");
        }

        let cs = capture(|| unsafe { write_stdio(&L().c, cfg, &data, &cmap, &pcs) });
        let rs = capture(|| unsafe { write_stdio(&L().rs, cfg, &data, &cmap, &prs) });
        assert_eq!(
            cs.out.is_some(),
            rs.out.is_some(),
            "cfg62 stdio fmt={fname}: exactly one library unwound"
        );
        if let (Some(a), Some(b)) = (&cs.out, &rs.out) {
            assert_eq!(
                a.0, b.0,
                "cfg62 stdio fmt={fname}: write_to_stdio return\n  C  {:?}\n  RS {:?}",
                a.1, b.1
            );
            assert_eq!(a.1, b.1, "cfg62 stdio fmt={fname}: png_image after write");
        }

        let rd = |p: &std::path::Path| std::fs::read(p).unwrap_or_default();
        let (bc, br, bcs, brs) = (rd(&pc), rd(&pr), rd(&pcs), rd(&prs));
        if !bc.is_empty() {
            nonempty += 1;
            assert_eq!(
                &bc[..8],
                b"\x89PNG\r\n\x1a\n",
                "cfg62 fmt={fname}: C file is not a PNG"
            );
        }
        cmp_bytes(&bc, &br, &format!("cfg62 fmt={fname}: file C vs Rust"));
        cmp_bytes(&bcs, &brs, &format!("cfg62 fmt={fname}: stdio C vs Rust"));
        cmp_bytes(&bc, &bcs, &format!("cfg62 fmt={fname}: C file vs C stdio"));
        cmp_bytes(&br, &brs, &format!("cfg62 fmt={fname}: RS file vs RS stdio"));

        // Cross-check against write_to_memory for both libraries.
        let cm = capture(|| unsafe {
            let q = write_mem(&L().c, cfg, &data, &cmap, None);
            let n = q.mb;
            write_mem(&L().c, cfg, &data, &cmap, Some(n))
        });
        if let Some(o) = &cm.out {
            if o.r != 0 {
                cmp_bytes(
                    &bc,
                    &o.png[..o.mb.min(o.png.len())],
                    &format!("cfg62 fmt={fname}: C file vs C memory"),
                );
            }
        }

        for p in [&pc, &pr, &pcs, &prs] {
            let _ = std::fs::remove_file(p);
        }
    }
    eprintln!("[cfg62] {} configurations, {nonempty} wrote a PNG", cfgs.len());
    assert_eq!(cfgs.len(), 72, "cfg62: configuration count");
    assert_eq!(nonempty, 72, "cfg62: every configuration must write a PNG file");
}

#[test]
fn cfg62_write_to_unwritable_path() {
    let cfg = WCfg {
        fmt: PNG_FORMAT_RGBA,
        w: 4,
        h: 4,
        entries: 0,
        smode: 1,
        pad: 0,
        conv8: 0,
        flags: 0,
    };
    let mut rng = Rng::new(0x2468);
    let (data, cmap) = wcfg_data(&cfg, &mut rng);
    let bad = std::path::Path::new("/definitely/not/a/writable/directory/x.png");
    let c = capture(|| unsafe { write_file(&L().c, &cfg, &data, &cmap, bad) });
    let r = capture(|| unsafe { write_file(&L().rs, &cfg, &data, &cmap, bad) });
    assert_eq!(c.out.is_some(), r.out.is_some(), "cfg62 unwritable: unwind");
    if let (Some(a), Some(b)) = (&c.out, &r.out) {
        assert_eq!(a.0, b.0, "cfg62 unwritable: return value");
        assert_eq!(a.1, b.1, "cfg62 unwritable: png_image (message must match)");
        assert_eq!(a.0, 0, "cfg62 unwritable: expected failure");
    }
}

// ==========================================================================
// Row 63 -- png_image_free / png_image_error driven paths.
// ==========================================================================

/// A whole scenario boiled down to a comparable value: the sequence of return
/// codes plus the sequence of `png_image` states.
type Trace = (Vec<c_int>, Vec<(u32, u32, u32, u32, u32, u32, u32, String)>);

fn trace_eq(c: &Run<Trace>, r: &Run<Trace>, what: &str) {
    assert_eq!(
        c.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        r.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        "{what}: harness message log differs (C first)"
    );
    assert_eq!(
        c.out.is_some(),
        r.out.is_some(),
        "{what}: exactly one library unwound (C unwound = {})",
        c.out.is_none()
    );
    if let (Some(a), Some(b)) = (&c.out, &r.out) {
        assert_eq!(a.0, b.0, "{what}: return codes\n  C  {:?}\n  RS {:?}", a, b);
        assert_eq!(a.1, b.1, "{what}: png_image states");
    }
}

fn both_trace<F>(f: F, what: &str)
where
    F: Fn(&'static Library) -> Trace,
{
    let c = capture(|| f(&L().c));
    let r = capture(|| f(&L().rs));
    trace_eq(&c, &r, what);
}

#[test]
fn cfg63_free_after_successful_read() {
    let data = input("RGBA8_33x5_il0").data.clone();
    both_trace(
        move |l| unsafe {
            let mut rc = Vec::new();
            let mut ks = Vec::new();
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            rc.push(api::png_image_begin_read_from_memory(
                l,
                &mut img,
                data.as_ptr() as *const c_void,
                data.len(),
            ));
            ks.push(img.cmp_key());
            img.format = PNG_FORMAT_RGBA;
            let n = image_size(img.format, img.width, img.height);
            let mut buf = Buf::pattern(n);
            let stride = row_stride_of(img.format, img.width) as i32;
            rc.push(api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.ptr(),
                stride,
                std::ptr::null_mut(),
            ));
            ks.push(img.cmp_key());
            rc.push(if img.opaque.is_null() { 1 } else { 0 });
            // finish_read again -- the image is already freed
            rc.push(api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.ptr(),
                stride,
                std::ptr::null_mut(),
            ));
            ks.push(img.cmp_key());
            // free, twice
            api::png_image_free(l, &mut img);
            ks.push(img.cmp_key());
            api::png_image_free(l, &mut img);
            ks.push(img.cmp_key());
            rc.push(if img.opaque.is_null() { 1 } else { 0 });
            (rc, ks)
        },
        "cfg63 free after successful read",
    );
}

#[test]
fn cfg63_free_after_failed_read() {
    for (name, data) in [
        ("empty", Vec::new()),
        ("garbage", b"0123456789abcdef".to_vec()),
        ("sig_only", input("G8_33x5_il0").data[..8].to_vec()),
        ("trunc_ihdr", input("G8_33x5_il0").data[..20].to_vec()),
    ] {
        let d = data.clone();
        both_trace(
            move |l| unsafe {
                let mut rc = Vec::new();
                let mut ks = Vec::new();
                let mut img = png_image {
                    version: PNG_IMAGE_VERSION,
                    ..Default::default()
                };
                rc.push(api::png_image_begin_read_from_memory(
                    l,
                    &mut img,
                    if d.is_empty() {
                        d.as_ptr() as *const c_void
                    } else {
                        d.as_ptr() as *const c_void
                    },
                    d.len(),
                ));
                ks.push(img.cmp_key());
                rc.push(if img.opaque.is_null() { 1 } else { 0 });
                // finish_read on the failed image
                let mut buf = Buf::pattern(64);
                rc.push(api::png_image_finish_read(
                    l,
                    &mut img,
                    std::ptr::null(),
                    buf.ptr(),
                    0,
                    std::ptr::null_mut(),
                ));
                ks.push(img.cmp_key());
                api::png_image_free(l, &mut img);
                ks.push(img.cmp_key());
                api::png_image_free(l, &mut img);
                ks.push(img.cmp_key());
                (rc, ks)
            },
            &format!("cfg63 free after failed read ({name})"),
        );
    }
}

#[test]
fn cfg63_free_on_zeroed_and_bad_version_image() {
    both_trace(
        |l| unsafe {
            let mut rc = Vec::new();
            let mut ks = Vec::new();
            // all-zero png_image: version 0, opaque NULL
            let mut img = png_image {
                version: 0,
                ..Default::default()
            };
            api::png_image_free(l, &mut img);
            ks.push(img.cmp_key());
            api::png_image_free(l, &mut img);
            ks.push(img.cmp_key());
            // begin_read on a damaged version
            let d = b"\x89PNG\r\n\x1a\n";
            rc.push(api::png_image_begin_read_from_memory(
                l,
                &mut img,
                d.as_ptr() as *const c_void,
                d.len(),
            ));
            ks.push(img.cmp_key());
            // finish_read on a damaged version
            let mut buf = Buf::pattern(16);
            rc.push(api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.ptr(),
                0,
                std::ptr::null_mut(),
            ));
            ks.push(img.cmp_key());
            // write_to_memory on a damaged version
            let mut mb: usize = 0;
            let inb = Buf::pattern(64);
            rc.push(api::png_image_write_to_memory(
                l,
                &mut img,
                std::ptr::null_mut(),
                &mut mb,
                0,
                inb.cptr(),
                0,
                std::ptr::null(),
            ));
            ks.push(img.cmp_key());
            rc.push(mb as c_int);
            api::png_image_free(l, &mut img);
            ks.push(img.cmp_key());
            (rc, ks)
        },
        "cfg63 zeroed / damaged-version image",
    );
}

#[test]
fn cfg63_invalid_argument_paths() {
    let data = input("RGBA8_33x5_il0").data.clone();

    // begin_read_from_memory(NULL, ...) and size 0
    let d0 = data.clone();
    both_trace(
        move |l| unsafe {
            let mut rc = Vec::new();
            let mut ks = Vec::new();
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            rc.push(api::png_image_begin_read_from_memory(
                l,
                &mut img,
                std::ptr::null(),
                10,
            ));
            ks.push(img.cmp_key());
            let mut img2 = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            rc.push(api::png_image_begin_read_from_memory(
                l,
                &mut img2,
                d0.as_ptr() as *const c_void,
                0,
            ));
            ks.push(img2.cmp_key());
            api::png_image_free(l, &mut img2);
            ks.push(img2.cmp_key());
            // begin_read_from_stdio(NULL)
            let mut img3 = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            rc.push(api::png_image_begin_read_from_stdio(
                l,
                &mut img3,
                std::ptr::null_mut(),
            ));
            ks.push(img3.cmp_key());
            // begin_read_from_file(NULL)
            let mut img4 = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            rc.push(api::png_image_begin_read_from_file(
                l,
                &mut img4,
                std::ptr::null(),
            ));
            ks.push(img4.cmp_key());
            (rc, ks)
        },
        "cfg63 begin_read invalid arguments",
    );

    // finish_read argument errors
    for (name, fmt, buf_null, stride, cmap_null) in [
        ("buffer NULL", PNG_FORMAT_RGBA, true, 0i32, true),
        ("stride too small", PNG_FORMAT_RGBA, false, 1i32, true),
        ("stride too small neg", PNG_FORMAT_RGBA, false, -1i32, true),
        ("no colormap", PNG_FORMAT_RGB_COLORMAP, false, 0i32, true),
        (
            "no colormap linear",
            PNG_FORMAT_LINEAR_RGB | PNG_FORMAT_FLAG_COLORMAP,
            false,
            0i32,
            true,
        ),
    ] {
        let d = data.clone();
        both_trace(
            move |l| unsafe {
                let mut rc = Vec::new();
                let mut ks = Vec::new();
                let mut img = png_image {
                    version: PNG_IMAGE_VERSION,
                    ..Default::default()
                };
                rc.push(api::png_image_begin_read_from_memory(
                    l,
                    &mut img,
                    d.as_ptr() as *const c_void,
                    d.len(),
                ));
                ks.push(img.cmp_key());
                img.format = fmt;
                let mut buf = Buf::pattern(image_size(fmt, img.width, img.height).max(16));
                let bp = if buf_null {
                    std::ptr::null_mut()
                } else {
                    buf.ptr()
                };
                let mut cm = Buf::pattern(colormap_max_bytes(fmt));
                let cp = if cmap_null {
                    std::ptr::null_mut()
                } else {
                    cm.ptr()
                };
                rc.push(api::png_image_finish_read(
                    l, &mut img, std::ptr::null(), bp, stride, cp,
                ));
                ks.push(img.cmp_key());
                api::png_image_free(l, &mut img);
                ks.push(img.cmp_key());
                (rc, ks)
            },
            &format!("cfg63 finish_read ({name})"),
        );
    }

    // row_stride too large / image too large: hand-doctored png_image
    for (name, w, h, fmt) in [
        ("row_stride too large", 0x4000_0000u32, 1u32, PNG_FORMAT_RGBA),
        ("image too large", 1u32, 0xffff_ffffu32, PNG_FORMAT_LINEAR_Y),
    ] {
        let d = data.clone();
        both_trace(
            move |l| unsafe {
                let mut rc = Vec::new();
                let mut ks = Vec::new();
                let mut img = png_image {
                    version: PNG_IMAGE_VERSION,
                    ..Default::default()
                };
                rc.push(api::png_image_begin_read_from_memory(
                    l,
                    &mut img,
                    d.as_ptr() as *const c_void,
                    d.len(),
                ));
                img.format = fmt;
                img.width = w;
                img.height = h;
                let mut buf = Buf::pattern(64);
                rc.push(api::png_image_finish_read(
                    l,
                    &mut img,
                    std::ptr::null(),
                    buf.ptr(),
                    0,
                    std::ptr::null_mut(),
                ));
                ks.push(img.cmp_key());
                api::png_image_free(l, &mut img);
                ks.push(img.cmp_key());
                (rc, ks)
            },
            &format!("cfg63 finish_read ({name})"),
        );
    }

    // write_to_memory argument errors
    both_trace(
        |l| unsafe {
            let mut rc = Vec::new();
            let mut ks = Vec::new();
            let inb = Buf::pattern(64);
            // memory_bytes == NULL
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGBA,
                ..Default::default()
            };
            rc.push(api::png_image_write_to_memory(
                l,
                &mut img,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                inb.cptr(),
                0,
                std::ptr::null(),
            ));
            ks.push(img.cmp_key());
            // buffer == NULL
            let mut mb: usize = 0;
            let mut img2 = png_image {
                version: PNG_IMAGE_VERSION,
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGBA,
                ..Default::default()
            };
            rc.push(api::png_image_write_to_memory(
                l,
                &mut img2,
                std::ptr::null_mut(),
                &mut mb,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
            ));
            ks.push(img2.cmp_key());
            rc.push(mb as c_int);
            // colormap format with no colormap
            let mut img3 = png_image {
                version: PNG_IMAGE_VERSION,
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGB_COLORMAP,
                colormap_entries: 8,
                ..Default::default()
            };
            rc.push(api::png_image_write_to_memory(
                l,
                &mut img3,
                std::ptr::null_mut(),
                &mut mb,
                0,
                inb.cptr(),
                0,
                std::ptr::null(),
            ));
            ks.push(img3.cmp_key());
            rc.push(mb as c_int);
            // colormap format with zero entries
            let mut img4 = png_image {
                version: PNG_IMAGE_VERSION,
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGB_COLORMAP,
                colormap_entries: 0,
                ..Default::default()
            };
            rc.push(api::png_image_write_to_memory(
                l,
                &mut img4,
                std::ptr::null_mut(),
                &mut mb,
                0,
                inb.cptr(),
                0,
                inb.cptr(),
            ));
            ks.push(img4.cmp_key());
            // row stride too small
            let mut img5 = png_image {
                version: PNG_IMAGE_VERSION,
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGBA,
                ..Default::default()
            };
            rc.push(api::png_image_write_to_memory(
                l,
                &mut img5,
                std::ptr::null_mut(),
                &mut mb,
                0,
                inb.cptr(),
                4,
                std::ptr::null(),
            ));
            ks.push(img5.cmp_key());
            // write_to_file / write_to_stdio invalid arguments
            let mut img6 = png_image {
                version: PNG_IMAGE_VERSION,
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGBA,
                ..Default::default()
            };
            rc.push(api::png_image_write_to_file(
                l,
                &mut img6,
                std::ptr::null(),
                0,
                inb.cptr(),
                0,
                std::ptr::null(),
            ));
            ks.push(img6.cmp_key());
            let mut img7 = png_image {
                version: PNG_IMAGE_VERSION,
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGBA,
                ..Default::default()
            };
            rc.push(api::png_image_write_to_stdio(
                l,
                &mut img7,
                std::ptr::null_mut(),
                0,
                inb.cptr(),
                0,
                std::ptr::null(),
            ));
            ks.push(img7.cmp_key());
            (rc, ks)
        },
        "cfg63 write invalid arguments",
    );
}

#[test]
fn cfg63_png_image_error_direct() {
    both_trace(
        |l| unsafe {
            let mut rc = Vec::new();
            let mut ks = Vec::new();
            let short = cs("short message");
            let long = cs(
                "a very long error message that is definitely longer than the \
                 sixty-four byte png_image::message buffer and must be truncated",
            );
            let empty = cs("");

            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            rc.push(png_image_error(l, &mut img, short.as_ptr()));
            ks.push(img.cmp_key());
            rc.push(png_image_error(l, &mut img, long.as_ptr()));
            ks.push(img.cmp_key());
            rc.push(png_image_error(l, &mut img, empty.as_ptr()));
            ks.push(img.cmp_key());
            rc.push(png_image_error(l, &mut img, short.as_ptr()));
            ks.push(img.cmp_key());
            api::png_image_free(l, &mut img);
            ks.push(img.cmp_key());
            (rc, ks)
        },
        "cfg63 png_image_error direct",
    );

    // png_image_error on a live (successfully opened) image must free it.
    let data = input("G8_33x5_il0").data.clone();
    both_trace(
        move |l| unsafe {
            let mut rc = Vec::new();
            let mut ks = Vec::new();
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            rc.push(api::png_image_begin_read_from_memory(
                l,
                &mut img,
                data.as_ptr() as *const c_void,
                data.len(),
            ));
            ks.push(img.cmp_key());
            let msg = cs("harness induced failure");
            rc.push(png_image_error(l, &mut img, msg.as_ptr()));
            ks.push(img.cmp_key());
            rc.push(if img.opaque.is_null() { 1 } else { 0 });
            api::png_image_free(l, &mut img);
            ks.push(img.cmp_key());
            (rc, ks)
        },
        "cfg63 png_image_error on live image",
    );
}

#[test]
fn cfg63_warning_paths_preserve_state() {
    // Inputs that produce libpng warnings inside the simplified reader; the
    // warning text ends up in png_image::message and warning_or_error must be
    // identical in both libraries.
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    let base = input("RGB8_33x5_il0").data.clone();
    // corrupt the CRC of the IHDR chunk (bytes 29..33) -> benign error/warning
    let mut d = base.clone();
    let n = d.len();
    for i in 29..33.min(n) {
        d[i] ^= 0xFF;
    }
    cases.push(("ihdr_crc".into(), d));
    // corrupt the IDAT CRC: find the last 4 bytes before IEND
    let mut d = base.clone();
    if d.len() > 16 {
        let l = d.len();
        d[l - 16] ^= 0x01;
    }
    cases.push(("tail_flip".into(), d));
    // append trailing garbage after IEND
    let mut d = base.clone();
    d.extend_from_slice(b"trailing garbage bytes");
    cases.push(("trailing".into(), d));

    for (name, data) in cases {
        let d = data.clone();
        both_trace(
            move |l| unsafe {
                let mut rc = Vec::new();
                let mut ks = Vec::new();
                let mut img = png_image {
                    version: PNG_IMAGE_VERSION,
                    ..Default::default()
                };
                rc.push(api::png_image_begin_read_from_memory(
                    l,
                    &mut img,
                    d.as_ptr() as *const c_void,
                    d.len(),
                ));
                ks.push(img.cmp_key());
                if img.opaque.is_null() {
                    return (rc, ks);
                }
                img.format = PNG_FORMAT_RGBA;
                let mut buf = Buf::pattern(image_size(img.format, img.width, img.height).max(16));
                let stride = row_stride_of(img.format, img.width) as i32;
                rc.push(api::png_image_finish_read(
                    l,
                    &mut img,
                    std::ptr::null(),
                    buf.ptr(),
                    stride,
                    std::ptr::null_mut(),
                ));
                ks.push(img.cmp_key());
                api::png_image_free(l, &mut img);
                ks.push(img.cmp_key());
                (rc, ks)
            },
            &format!("cfg63 warning path ({name})"),
        );
    }
}

// ==========================================================================
// Rows 59 + 61 together: write with the simplified API, read the result back.
// This drives the simplified reader over PNGs that carry the sRGB / gAMA /
// cHRM chunks the simplified writer emits -- shapes no other input covers.
// ==========================================================================

#[test]
fn cfg61_write_then_read_roundtrip() {
    let fmts = all_formats();
    let mut rng = Rng::new(0xABCDEF01);
    let mut t = Tally::default();
    let mut produced = 0u32;

    for (wname, wfmt) in &fmts {
        for conv8 in [0 as c_int, 1] {
            let cfg = WCfg {
                fmt: *wfmt,
                w: 9,
                h: 5,
                entries: if wfmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                    200
                } else {
                    0
                },
                smode: 1,
                pad: 0,
                conv8,
                flags: 0,
            };
            let (data, cmap) = wcfg_data(&cfg, &mut rng);

            let cq = capture(|| unsafe { write_mem(&L().c, &cfg, &data, &cmap, None) });
            let rq = capture(|| unsafe { write_mem(&L().rs, &cfg, &data, &cmap, None) });
            cmp_write(&cq, &rq, &format!("cfg61 rt query fmt={wname} conv8={conv8}"));
            let size = match &cq.out {
                Some(o) if o.r != 0 && o.mb > 0 => o.mb,
                _ => continue,
            };
            let ce = capture(|| unsafe { write_mem(&L().c, &cfg, &data, &cmap, Some(size)) });
            let re = capture(|| unsafe { write_mem(&L().rs, &cfg, &data, &cmap, Some(size)) });
            cmp_write(&ce, &re, &format!("cfg61 rt write fmt={wname} conv8={conv8}"));
            let png = match &ce.out {
                Some(o) if o.r != 0 => o.png[..size].to_vec(),
                _ => continue,
            };
            produced += 1;

            for (rname, rfmt) in &fmts {
                for (bgn, bg) in [("bgNULL", BGS[0].1), ("bgMID", BGS[1].1)] {
                    let sm = if produced % 2 == 0 { 1 } else { -1 };
                    let tag = format!(
                        "cfg61 roundtrip write={wname} conv8={conv8} read={rname} {bgn} smode={sm}"
                    );
                    let c = capture(|| unsafe { read_mem(&L().c, &png, *rfmt, sm, 0, bg, 0) });
                    let r = capture(|| unsafe { read_mem(&L().rs, &png, *rfmt, sm, 0, bg, 0) });
                    cmp_read(&c, &r, &tag);
                    t.add(&c);
                }
            }
        }
    }
    eprintln!("[cfg61 roundtrip] {produced} PNGs written");
    assert_eq!(produced, 72, "cfg61 roundtrip: every format must write");
    assert_eq!(t.total, 72 * 36 * 2, "cfg61 roundtrip: config count");
    assert_eq!(t.begin_ok, t.total, "cfg61 roundtrip: every PNG must re-open");
    t.check("cfg61 roundtrip", 70);
}

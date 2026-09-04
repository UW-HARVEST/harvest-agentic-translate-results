//! Phase B — the SIMPLIFIED API (`png_image_*`).
//!
//! Every entry point of the simplified API is driven, in both shared libraries,
//! with a large number of deterministic (fixed-seed) inputs:
//!
//!   * `png_image_begin_read_from_memory` / `_from_file` / `_from_stdio`
//!   * `png_image_finish_read` for every output format, every
//!     `PNG_IMAGE_FLAG_*` combination, NULL and non-NULL `background`, and
//!     zero / natural / padded / negative `row_stride`
//!   * `png_image_write_to_memory` (size query, exact, generous and too-small
//!     buffers), `png_image_write_to_file`, `png_image_write_to_stdio`
//!   * write→read round trips
//!   * `png_image_free` idempotency / version handling
//!
//! Because the simplified API wraps everything in `png_safe_execute` no error
//! ever escapes as a `longjmp`; the functions return 0 and fill in
//! `png_image::warning_or_error` and `png_image::message`.  Those are therefore
//! compared exactly too, together with the return value, the whole
//! `png_image` (minus the `opaque` allocation address), the whole output buffer
//! and the whole colour-map buffer.
#![allow(dead_code)]

mod common;

use common::api::{apis, Api};
use common::pngbuild as pb;
use common::*;
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// png.h PNG_IMAGE_* macros, re-implemented here
// ---------------------------------------------------------------------------

fn sample_channels(f: u32) -> u32 {
    (f & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}
fn sample_component_size(f: u32) -> u32 {
    ((f & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
}
fn sample_size(f: u32) -> u32 {
    sample_channels(f) * sample_component_size(f)
}
fn is_colormap(f: u32) -> bool {
    f & PNG_FORMAT_FLAG_COLORMAP != 0
}
fn pixel_channels(f: u32) -> u32 {
    if is_colormap(f) {
        1
    } else {
        sample_channels(f)
    }
}
fn pixel_component_size(f: u32) -> u32 {
    if is_colormap(f) {
        1
    } else {
        sample_component_size(f)
    }
}
/// PNG_IMAGE_ROW_STRIDE — in *components*, not bytes.
fn natural_stride(f: u32, width: u32) -> u32 {
    pixel_channels(f) * width
}
/// PNG_IMAGE_BUFFER_SIZE
fn buffer_size(f: u32, height: u32, abs_stride: u32) -> usize {
    pixel_component_size(f) as usize * height as usize * abs_stride as usize
}

/// Big enough for PNG_IMAGE_COLORMAP_SIZE of *any* format
/// (4 channels * 2 bytes * 256 entries).
const CMAP_ALLOC: usize = 8 * 256;

const BUF_FILL: u8 = 0xa5;
const CMAP_FILL: u8 = 0x5c;

/// The 20 output formats required by the simplified read API.
const ALL_FORMATS: [(&str, u32); 20] = [
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
    ("RGB_COLORMAP", PNG_FORMAT_RGB_COLORMAP),
    ("BGR_COLORMAP", PNG_FORMAT_BGR_COLORMAP),
    ("RGBA_COLORMAP", PNG_FORMAT_RGBA_COLORMAP),
    ("ARGB_COLORMAP", PNG_FORMAT_ARGB_COLORMAP),
    ("BGRA_COLORMAP", PNG_FORMAT_BGRA_COLORMAP),
    ("ABGR_COLORMAP", PNG_FORMAT_ABGR_COLORMAP),
    ("GRAY_COLORMAP", PNG_FORMAT_GRAY | PNG_FORMAT_FLAG_COLORMAP),
];

/// Every legal (bit_depth, colour_type) pair.
const SHAPES: [(u8, u8); 15] = [
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

const FLAG_SETS: [(&str, u32); 6] = [
    ("0", 0),
    ("FAST", PNG_IMAGE_FLAG_FAST),
    ("16BIT_sRGB", PNG_IMAGE_FLAG_16BIT_sRGB),
    ("NOT_sRGB", PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB),
    (
        "FAST|16BIT_sRGB",
        PNG_IMAGE_FLAG_FAST | PNG_IMAGE_FLAG_16BIT_sRGB,
    ),
    (
        "ALL",
        PNG_IMAGE_FLAG_FAST | PNG_IMAGE_FLAG_16BIT_sRGB | PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB,
    ),
];

// ---------------------------------------------------------------------------
// 8-byte aligned byte buffer
//
// The simplified API casts the caller's buffer to `png_uint_16*` for LINEAR
// formats, so a plain `Vec<u8>` (1-byte aligned) would be a (benign on x86 but
// still real) alignment violation.  A `Vec<u64>` backing store is always
// 8-byte aligned.
// ---------------------------------------------------------------------------

struct ABuf {
    words: Vec<u64>,
    len: usize,
}

impl ABuf {
    fn new(len: usize, fill: u8) -> Self {
        let words = len / 8 + 2;
        ABuf {
            words: vec![u64::from_ne_bytes([fill; 8]); words],
            len,
        }
    }
    fn from_bytes(src: &[u8]) -> Self {
        let mut b = ABuf::new(src.len(), 0);
        b.bytes_mut().copy_from_slice(src);
        b
    }
    fn ptr(&mut self) -> *mut c_void {
        self.words.as_mut_ptr() as *mut c_void
    }
    fn cptr(&self) -> *const c_void {
        self.words.as_ptr() as *const c_void
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.words.as_ptr() as *const u8, self.len) }
    }
    fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.words.as_mut_ptr() as *mut u8, self.len) }
    }
}

// ---------------------------------------------------------------------------
// libc stdio, resolved from the main program handle (both `.so`s share it)
// ---------------------------------------------------------------------------

type FopenFn = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void;
type FcloseFn = unsafe extern "C" fn(*mut c_void) -> c_int;

fn stdio() -> (FopenFn, FcloseFn) {
    static S: OnceLock<(FopenFn, FcloseFn)> = OnceLock::new();
    *S.get_or_init(|| unsafe {
        use libloading::os::unix as u;
        let main = u::Library::open(None::<&std::ffi::OsStr>, u::RTLD_NOW | u::RTLD_GLOBAL)
            .expect("dlopen(NULL) failed");
        let fo: u::Symbol<FopenFn> = main.get(b"fopen\0").expect("fopen not found");
        let fc: u::Symbol<FcloseFn> = main.get(b"fclose\0").expect("fclose not found");
        let r = (*fo, *fc);
        std::mem::forget(main); // keep the handle (and the symbols) alive forever
        r
    })
}

fn tmp_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("t04_simplified_{}_{}_{}.tmp", tag, std::process::id(), n))
}

fn cpath(p: &Path) -> CString {
    CString::new(p.to_str().expect("utf-8 path")).unwrap()
}

// ---------------------------------------------------------------------------
// read driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stride {
    /// pass 0 => "natural"
    Zero,
    /// pass exactly the natural stride
    Exact,
    /// pass natural + n (padding)
    Pad(u32),
    NegExact,
    NegPad(u32),
}

impl Stride {
    fn value(self, nat: u32) -> i32 {
        match self {
            Stride::Zero => 0,
            Stride::Exact => nat as i32,
            Stride::Pad(n) => (nat + n) as i32,
            Stride::NegExact => -(nat as i32),
            Stride::NegPad(n) => -((nat + n) as i32),
        }
    }
    fn abs(self, nat: u32) -> u32 {
        let v = self.value(nat);
        if v == 0 {
            nat
        } else {
            v.unsigned_abs()
        }
    }
}

#[derive(Clone, Copy)]
enum RSrc<'a> {
    Mem(&'a [u8]),
    File(&'a Path),
    Stdio(&'a Path),
}

struct ReadRes {
    begin: c_int,
    finish: c_int,
    /// png_image state after begin_read
    after_begin: (u32, u32, u32, u32, u32, u32, u32, String),
    /// png_image state after finish_read
    after_finish: (u32, u32, u32, u32, u32, u32, u32, String),
    opaque_null_at_end: bool,
    buf: Vec<u8>,
    cmap: Vec<u8>,
}

struct ReadCfg {
    format: Option<u32>,
    flags: Option<u32>,
    background: Option<png_color>,
    stride: Stride,
    /// pass a colour-map buffer at all?
    give_colormap: bool,
}

impl Default for ReadCfg {
    fn default() -> Self {
        ReadCfg {
            format: None,
            flags: None,
            background: None,
            stride: Stride::Zero,
            give_colormap: true,
        }
    }
}

fn do_read(a: &Api, src: RSrc<'_>, cfg: &ReadCfg) -> ReadRes {
    unsafe {
        let mut img = png_image::default();
        img.version = PNG_IMAGE_VERSION;

        // `_from_stdio` needs the FILE* to stay open until finish_read is done.
        let mut fp: *mut c_void = ptr::null_mut();
        let (fopen, fclose) = stdio();

        let begin = match src {
            RSrc::Mem(b) => (a.png_image_begin_read_from_memory)(
                &mut img,
                b.as_ptr() as *const c_void,
                b.len(),
            ),
            RSrc::File(p) => {
                let c = cpath(p);
                (a.png_image_begin_read_from_file)(&mut img, c.as_ptr())
            }
            RSrc::Stdio(p) => {
                let c = cpath(p);
                fp = fopen(c.as_ptr(), c"rb".as_ptr());
                assert!(!fp.is_null(), "fopen({}) failed", p.display());
                (a.png_image_begin_read_from_stdio)(&mut img, fp)
            }
        };

        let after_begin = img.cmp_tuple();

        if begin == 0 {
            let r = ReadRes {
                begin,
                finish: -1,
                after_begin: after_begin.clone(),
                after_finish: after_begin,
                opaque_null_at_end: img.opaque.is_null(),
                buf: Vec::new(),
                cmap: Vec::new(),
            };
            (a.png_image_free)(&mut img);
            if !fp.is_null() {
                fclose(fp);
            }
            return r;
        }

        if let Some(f) = cfg.format {
            img.format = f;
        }
        if let Some(fl) = cfg.flags {
            img.flags = fl;
        }

        let nat = natural_stride(img.format, img.width);
        let rs = cfg.stride.value(nat);
        let absr = cfg.stride.abs(nat);
        let mut buf = ABuf::new(buffer_size(img.format, img.height, absr).max(8), BUF_FILL);
        let mut cmap = ABuf::new(CMAP_ALLOC, CMAP_FILL);

        let bg = cfg.background.unwrap_or_default();
        let bgp: *const png_color = match cfg.background {
            Some(_) => &bg,
            None => ptr::null(),
        };
        let cmp: *mut c_void = if cfg.give_colormap {
            cmap.ptr()
        } else {
            ptr::null_mut()
        };

        let finish = (a.png_image_finish_read)(&mut img, bgp, buf.ptr(), rs, cmp);
        let after_finish = img.cmp_tuple();
        (a.png_image_free)(&mut img);
        let opaque_null_at_end = img.opaque.is_null();
        if !fp.is_null() {
            fclose(fp);
        }

        ReadRes {
            begin,
            finish,
            after_begin,
            after_finish,
            opaque_null_at_end,
            buf: buf.bytes().to_vec(),
            cmap: cmap.bytes().to_vec(),
        }
    }
}

#[track_caller]
fn cmp_read(what: &str, c: &ReadRes, r: &ReadRes) {
    eq_dbg(&format!("{what}: begin_read return"), c.begin, r.begin);
    eq_dbg(
        &format!("{what}: png_image after begin_read"),
        c.after_begin.clone(),
        r.after_begin.clone(),
    );
    eq_dbg(&format!("{what}: finish_read return"), c.finish, r.finish);
    eq_dbg(
        &format!("{what}: png_image after finish_read"),
        c.after_finish.clone(),
        r.after_finish.clone(),
    );
    eq_dbg(
        &format!("{what}: colormap_entries"),
        c.after_finish.5,
        r.after_finish.5,
    );
    eq_dbg(
        &format!("{what}: opaque cleared"),
        c.opaque_null_at_end,
        r.opaque_null_at_end,
    );
    eq_bytes(&format!("{what}: output buffer"), &c.buf, &r.buf);
    eq_bytes(&format!("{what}: colormap buffer"), &c.cmap, &r.cmap);
}

/// Runs one case against both libraries, compares everything, and returns the
/// (identical) return value of `png_image_finish_read` so the callers can prove
/// that they are not silently testing nothing but failures.
#[track_caller]
fn diff_read(what: &str, src: RSrc<'_>, cfg: &ReadCfg) -> (c_int, String) {
    let b = apis();
    let c = do_read(&b.c, src, cfg);
    let r = do_read(&b.rs, src, cfg);
    cmp_read(what, &c, &r);
    if c.finish == 0 && std::env::var("T04_DEBUG").is_ok() {
        eprintln!(
            "@FAIL {} :: warn_or_err={} msg={:?}",
            what, c.after_finish.6, c.after_finish.7
        );
    }
    (c.finish, c.after_finish.7.clone())
}

/// Counts how many cases in a test actually reached a *successful* decode /
/// encode, and which messages the unsuccessful ones produced, so that
///
///   * a run in which both libraries merely failed everywhere cannot be
///     mistaken for a passing differential test, and
///   * a *new* kind of failure (which would mean the test inputs are wrong, or
///     that both libraries regressed together) is caught immediately.
#[derive(Default)]
struct Tally {
    total: u64,
    ok: u64,
    msgs: std::collections::BTreeMap<String, u64>,
}

impl Tally {
    fn add(&mut self, r: &(c_int, String)) {
        self.total += 1;
        if r.0 == 1 {
            self.ok += 1;
        } else {
            *self.msgs.entry(r.1.clone()).or_insert(0) += 1;
        }
    }
    fn add_ret(&mut self, ret: c_int, msg: &str) {
        self.add(&(ret, msg.to_string()));
    }
    /// `allowed` lists the messages that legitimately occur for the inputs the
    /// test uses (documented "returns 0 and sets a message" situations).
    #[track_caller]
    fn require(&self, what: &str, allowed: &[&str]) {
        assert!(self.total > 0, "{what}: no cases were run at all");
        assert!(
            self.ok * 2 > self.total,
            "{what}: only {}/{} cases succeeded - the test is not exercising \
             the happy path",
            self.ok,
            self.total
        );
        for (m, n) in self.msgs.iter() {
            assert!(
                allowed.contains(&m.as_str()),
                "{what}: {n} case(s) failed with an unexpected message {m:?} \
                 ({}/{} succeeded)",
                self.ok,
                self.total
            );
        }
    }
}

/// The only failure the valid-input read rows can legitimately produce: an
/// input with an alpha channel, an output format without one and a NULL
/// `background` argument.
const NEED_BACKGROUND: &str = "background color must be supplied to remove alpha/transparency";

// ---------------------------------------------------------------------------
// write driver
// ---------------------------------------------------------------------------

struct WCase {
    width: u32,
    height: u32,
    format: u32,
    flags: u32,
    entries: u32,
    stride: Stride,
    buf: ABuf,
    cmap: ABuf,
    label: String,
}

impl WCase {
    fn image(&self) -> png_image {
        let mut i = png_image::default();
        i.version = PNG_IMAGE_VERSION;
        i.width = self.width;
        i.height = self.height;
        i.format = self.format;
        i.flags = self.flags;
        i.colormap_entries = self.entries;
        i
    }
    fn rs(&self) -> i32 {
        self.stride.value(natural_stride(self.format, self.width))
    }
    fn cmap_ptr(&self) -> *const c_void {
        if is_colormap(self.format) {
            self.cmap.cptr()
        } else {
            ptr::null()
        }
    }
}

fn make_wcase(seed: u64, width: u32, height: u32, format: u32, flags: u32, stride: Stride) -> WCase {
    let mut rng = Rng::new(seed);
    let cm = is_colormap(format);
    let entries: u32 = if cm {
        *rng.pick(&[1u32, 2, 3, 4, 5, 15, 16, 17, 100, 256])
    } else {
        0
    };
    let nat = natural_stride(format, width);
    let absr = stride.abs(nat);
    let n = buffer_size(format, height, absr).max(8);
    let mut buf = ABuf::new(n, 0);
    {
        let bytes = buf.bytes_mut();
        for b in bytes.iter_mut() {
            *b = rng.next_u8();
        }
        if cm {
            // palette indices must be inside the colour-map, otherwise libpng
            // (correctly) reports "Wrote palette index exceeding num_palette"
            for b in bytes.iter_mut() {
                *b = (*b as u32 % entries) as u8;
            }
        }
    }
    let mut cmap = ABuf::new(CMAP_ALLOC, 0);
    for b in cmap.bytes_mut().iter_mut() {
        *b = rng.next_u8();
    }
    let label = format!(
        "{width}x{height} fmt=0x{format:02x} flags={flags} entries={entries} stride={stride:?}"
    );
    WCase {
        width,
        height,
        format,
        flags,
        entries,
        stride,
        buf,
        cmap,
        label,
    }
}

#[derive(Clone, Copy, Debug)]
enum MemMode {
    /// memory == NULL: size query only
    Query,
    /// buffer of exactly the queried size
    Exact,
    /// queried size + n
    Generous(usize),
    /// one byte short — documented to return 0 but still update *memory_bytes
    Short,
}

struct WriteRes {
    ret: c_int,
    size: usize,
    tup: (u32, u32, u32, u32, u32, u32, u32, String),
    opaque_null: bool,
    bytes: Vec<u8>,
    /// for an over-sized buffer: were the bytes past the reported size left
    /// alone?
    tail_intact: bool,
}

fn write_mem(a: &Api, w: &WCase, convert8: c_int, mode: MemMode, avail: usize) -> WriteRes {
    unsafe {
        let mut img = w.image();
        let mut size: usize = avail;
        let mut out = ABuf::new(avail.max(8), 0xcd);
        let memp: *mut c_void = match mode {
            MemMode::Query => ptr::null_mut(),
            _ => out.ptr(),
        };
        let ret = (a.png_image_write_to_memory)(
            &mut img,
            memp,
            &mut size,
            convert8,
            w.buf.cptr(),
            w.rs(),
            w.cmap_ptr(),
        );
        let tup = img.cmp_tuple();
        let opaque_null = img.opaque.is_null();
        (a.png_image_free)(&mut img);
        let n = size.min(avail);
        let bytes = match mode {
            MemMode::Query => Vec::new(),
            _ => out.bytes()[..n].to_vec(),
        };
        let tail_intact = match mode {
            MemMode::Query => true,
            _ => out.bytes()[n..].iter().all(|&x| x == 0xcd),
        };
        WriteRes {
            ret,
            size,
            tup,
            opaque_null,
            bytes,
            tail_intact,
        }
    }
}

#[track_caller]
fn cmp_write(what: &str, c: &WriteRes, r: &WriteRes) {
    eq_dbg(&format!("{what}: return"), c.ret, r.ret);
    eq_dbg(&format!("{what}: memory_bytes"), c.size, r.size);
    eq_dbg(&format!("{what}: png_image"), c.tup.clone(), r.tup.clone());
    eq_dbg(
        &format!("{what}: opaque cleared"),
        c.opaque_null,
        r.opaque_null,
    );
    eq_bytes(&format!("{what}: PNG bytes"), &c.bytes, &r.bytes);
    eq_dbg(
        &format!("{what}: bytes past memory_bytes untouched"),
        c.tail_intact,
        r.tail_intact,
    );
    assert!(
        c.tail_intact,
        "{what}: the C library wrote past *memory_bytes - the test is wrong"
    );
    assert!(
        r.tail_intact,
        "{what}: the Rust library wrote past *memory_bytes"
    );
}

// ===========================================================================
// 1. every legal (bit_depth, colour_type) x interlace x width x height
// ===========================================================================

#[test]
fn simplified_read_memory_all_shapes() {
    const WIDTHS: [u32; 11] = [1, 2, 3, 5, 7, 8, 9, 15, 16, 17, 33];
    const HEIGHTS: [u32; 4] = [1, 2, 3, 9];
    let mut n = 0u64;
    for &(bd, ct) in SHAPES.iter() {
        for interlace in 0..2u8 {
            for &w in WIDTHS.iter() {
                for &h in HEIGHTS.iter() {
                    n += 1;
                    let seed = 0x0401_0000 ^ (n * 0x9E37_79B9);
                    let png = pb::make_png(seed, w, h, bd, ct, interlace);
                    let what = format!(
                        "shape bd={bd} ct={ct} il={interlace} {w}x{h}"
                    );
                    // Every one of these inputs is a fully valid PNG, so the
                    // simplified API must succeed on all of them.
                    let ok = diff_read(&what, RSrc::Mem(&png), &ReadCfg::default());
                    assert_eq!(
                        ok,
                        (1, String::new()),
                        "{what}: finish_read failed on a valid PNG"
                    );
                }
            }
        }
    }
    assert_eq!(n, 15 * 2 * 11 * 4);
}

// ===========================================================================
// 2. every output format
// ===========================================================================

#[test]
fn simplified_read_every_output_format() {
    // A representative spread of input images (all colour types, both
    // interlaces, both bit-depth classes, odd and even widths).
    let inputs: Vec<(String, Vec<u8>)> = {
        let mut v = Vec::new();
        let mut k = 0u64;
        for &(bd, ct) in SHAPES.iter() {
            for &(w, h, il) in [
                (7u32, 3u32, 0u8),
                (16, 2, 1),
                (1, 1, 1),
                (33, 9, 0),
                (23, 7, 1),
            ]
            .iter()
            {
                k += 1;
                let seed = 0x0402_0000 ^ (k * 0x2545_F491);
                v.push((
                    format!("in(bd={bd},ct={ct},il={il},{w}x{h})"),
                    pb::make_png(seed, w, h, bd, ct, il),
                ));
            }
        }
        v
    };

    let mut tally = Tally::default();
    let mut rng = Rng::new(0x4020_1234);
    for (iname, png) in inputs.iter() {
        for &(fname, fmt) in ALL_FORMATS.iter() {
            // NULL and non-NULL background, alternating deterministically.
            let bg = if rng.bool() {
                Some(png_color {
                    red: rng.next_u8(),
                    green: rng.next_u8(),
                    blue: rng.next_u8(),
                })
            } else {
                None
            };
            let cfg = ReadCfg {
                format: Some(fmt),
                background: bg,
                ..Default::default()
            };
            let ok = diff_read(&format!("{iname} -> {fname}"), RSrc::Mem(png), &cfg);
            tally.add(&ok);
        }
    }
    tally.require("read_every_output_format", &[NEED_BACKGROUND]);
}

// ===========================================================================
// 3. PNG_IMAGE_FLAG_* combinations x every format
// ===========================================================================

#[test]
fn simplified_read_image_flags() {
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, &(bd, ct)) in SHAPES.iter().enumerate() {
        for il in 0..2u8 {
            let seed = 0x0403_0000 ^ ((i as u64 * 2 + il as u64 + 1) * 0x1234_5677);
            inputs.push((
                format!("in(bd={bd},ct={ct},il={il})"),
                pb::make_png(seed, 9, 4, bd, ct, il),
            ));
        }
    }

    let mut tally = Tally::default();
    for (iname, png) in inputs.iter() {
        for &(fname, fmt) in ALL_FORMATS.iter() {
            for &(flname, fl) in FLAG_SETS.iter() {
                let cfg = ReadCfg {
                    format: Some(fmt),
                    flags: Some(fl),
                    ..Default::default()
                };
                let ok = diff_read(
                    &format!("{iname} -> {fname} flags={flname}"),
                    RSrc::Mem(png),
                    &cfg,
                );
                tally.add(&ok);
            }
        }
    }
    tally.require("read_image_flags", &[NEED_BACKGROUND]);
}

// ===========================================================================
// 4. the `background` argument
// ===========================================================================

#[test]
fn simplified_read_background_argument() {
    // The background is only used when the input has alpha (or tRNS) and the
    // output format does not; cover both the meaningful and the ignored case.
    let alpha_inputs: Vec<(String, Vec<u8>)> = [(8u8, 4u8), (16, 4), (8, 6), (16, 6)]
        .iter()
        .enumerate()
        .map(|(i, &(bd, ct))| {
            let seed = 0x0404_0000 ^ ((i as u64 + 1) * 0xdead_beef);
            (
                format!("alpha(bd={bd},ct={ct})"),
                pb::make_png(seed, 11, 5, bd, ct, (i % 2) as u8),
            )
        })
        .collect();

    // A palette image with tRNS also gains an alpha channel.
    let trns_png = {
        let mut rng = Rng::new(0x0404_9999);
        let mut spec = pb::PngSpec::new(9, 4, 8, 3, 0);
        spec.palette = (0..256 * 3).map(|_| rng.next_u8()).collect();
        spec.trns = Some((0..16).map(|_| rng.next_u8()).collect());
        let mut r2 = Rng::new(0x0404_8888);
        spec.raw = pb::raw_rows_none(9, 4, 8, 3, &mut |_y, rb| {
            (0..rb).map(|_| r2.next_u8()).collect()
        });
        spec.build()
    };

    let backgrounds = [
        None,
        Some(png_color {
            red: 0,
            green: 0,
            blue: 0,
        }),
        Some(png_color {
            red: 255,
            green: 255,
            blue: 255,
        }),
        Some(png_color {
            red: 0x12,
            green: 0x9a,
            blue: 0x5e,
        }),
        Some(png_color {
            red: 0xff,
            green: 0x00,
            blue: 0x80,
        }),
    ];

    let mut all: Vec<(String, Vec<u8>)> = alpha_inputs;
    all.push(("palette+tRNS".to_string(), trns_png));

    let mut tally = Tally::default();
    for (iname, png) in all.iter() {
        for &(fname, fmt) in ALL_FORMATS.iter() {
            for (bi, bg) in backgrounds.iter().enumerate() {
                let cfg = ReadCfg {
                    format: Some(fmt),
                    background: *bg,
                    ..Default::default()
                };
                let ok = diff_read(
                    &format!("{iname} -> {fname} bg#{bi}={bg:?}"),
                    RSrc::Mem(png),
                    &cfg,
                );
                tally.add(&ok);
            }
        }
    }
    tally.require("read_background_argument", &[NEED_BACKGROUND]);
}

// ===========================================================================
// 5. row_stride: 0 / natural / padded / negative
// ===========================================================================

#[test]
fn simplified_read_row_stride_variants() {
    let strides = [
        Stride::Zero,
        Stride::Exact,
        Stride::Pad(1),
        Stride::Pad(7),
        Stride::Pad(32),
        Stride::NegExact,
        Stride::NegPad(1),
        Stride::NegPad(7),
        Stride::NegPad(32),
    ];

    let inputs: Vec<(String, Vec<u8>)> = [
        (1u8, 0u8, 0u8, 5u32, 3u32),
        (8, 0, 0, 7, 4),
        (16, 0, 1, 6, 5),
        (8, 2, 0, 5, 4),
        (16, 2, 1, 9, 3),
        (4, 3, 0, 7, 6),
        (8, 3, 1, 8, 2),
        (8, 4, 0, 5, 5),
        (16, 6, 0, 4, 4),
        (8, 6, 1, 11, 3),
        (8, 6, 0, 40, 17),
        (16, 2, 1, 33, 9),
        (2, 3, 0, 31, 8),
    ]
    .iter()
    .enumerate()
    .map(|(i, &(bd, ct, il, w, h))| {
        let seed = 0x0405_0000 ^ ((i as u64 + 1) * 0xcafe_0f1e);
        (
            format!("in(bd={bd},ct={ct},il={il},{w}x{h})"),
            pb::make_png(seed, w, h, bd, ct, il),
        )
    })
    .collect();

    // A subset of formats (all channel counts, LINEAR and COLORMAP variants).
    let fmts = [
        ("GRAY", PNG_FORMAT_GRAY),
        ("GA", PNG_FORMAT_GA),
        ("RGB", PNG_FORMAT_RGB),
        ("BGRA", PNG_FORMAT_BGRA),
        ("ARGB", PNG_FORMAT_ARGB),
        ("LINEAR_Y", PNG_FORMAT_LINEAR_Y),
        ("LINEAR_RGB_ALPHA", PNG_FORMAT_LINEAR_RGB_ALPHA),
        ("RGB_COLORMAP", PNG_FORMAT_RGB_COLORMAP),
        ("RGBA_COLORMAP", PNG_FORMAT_RGBA_COLORMAP),
    ];

    let mut tally = Tally::default();
    for (iname, png) in inputs.iter() {
        for &(fname, fmt) in fmts.iter() {
            for st in strides.iter() {
                let cfg = ReadCfg {
                    format: Some(fmt),
                    stride: *st,
                    ..Default::default()
                };
                let ok = diff_read(
                    &format!("{iname} -> {fname} stride={st:?}"),
                    RSrc::Mem(png),
                    &cfg,
                );
                tally.add(&ok);
            }
        }
        // ... and with the image's own format, unchanged.
        for st in strides.iter() {
            let cfg = ReadCfg {
                stride: *st,
                ..Default::default()
            };
            let ok = diff_read(
                &format!("{iname} -> native stride={st:?}"),
                RSrc::Mem(png),
                &cfg,
            );
            tally.add(&ok);
        }
    }
    tally.require("read_row_stride_variants", &[NEED_BACKGROUND]);
}

// ===========================================================================
// 6. begin_read_from_file / begin_read_from_stdio
// ===========================================================================

#[test]
fn simplified_read_from_file_and_stdio() {
    let mut tally = Tally::default();
    let mut k = 0u64;
    for &(bd, ct) in SHAPES.iter() {
        for &(w, h, il) in [(7u32, 3u32, 0u8), (13, 4, 1)].iter() {
            k += 1;
            let seed = 0x0406_0000 ^ (k * 0x51ed_2701);
            let png = pb::make_png(seed, w, h, bd, ct, il);
            let path = tmp_path("read");
            std::fs::write(&path, &png).expect("write temp png");

            let base = format!("file bd={bd} ct={ct} il={il} {w}x{h}");

            // native format, natural stride
            tally.add(&diff_read(
                &format!("{base} [from_file]"),
                RSrc::File(&path),
                &ReadCfg::default(),
            ));
            tally.add(&diff_read(
                &format!("{base} [from_stdio]"),
                RSrc::Stdio(&path),
                &ReadCfg::default(),
            ));

            // a couple of explicit formats/strides through the file paths too
            for &(fname, fmt) in [
                ("RGBA", PNG_FORMAT_RGBA),
                ("LINEAR_RGB", PNG_FORMAT_LINEAR_RGB),
                ("BGRA_COLORMAP", PNG_FORMAT_BGRA_COLORMAP),
            ]
            .iter()
            {
                for st in [Stride::Zero, Stride::Pad(3), Stride::NegExact].iter() {
                    let cfg = ReadCfg {
                        format: Some(fmt),
                        stride: *st,
                        ..Default::default()
                    };
                    tally.add(&diff_read(
                        &format!("{base} [from_file] {fname} {st:?}"),
                        RSrc::File(&path),
                        &cfg,
                    ));
                    tally.add(&diff_read(
                        &format!("{base} [from_stdio] {fname} {st:?}"),
                        RSrc::Stdio(&path),
                        &cfg,
                    ));
                }
            }

            let _ = std::fs::remove_file(&path);
        }
    }
    tally.require("read_from_file_and_stdio", &[NEED_BACKGROUND]);

    // Both libraries must reject a non-existent file the same way (errno
    // string) — a documented "return 0 with a message" case.
    {
        let missing = tmp_path("missing");
        let cs = cpath(&missing);
        let b = apis();
        let mut ic = png_image::default();
        let mut ir = png_image::default();
        unsafe {
            let rc = (b.c.png_image_begin_read_from_file)(&mut ic, cs.as_ptr());
            let rr = (b.rs.png_image_begin_read_from_file)(&mut ir, cs.as_ptr());
            eq_dbg("missing file: return", rc, rr);
            eq_dbg("missing file: png_image", ic.cmp_tuple(), ir.cmp_tuple());
            (b.c.png_image_free)(&mut ic);
            (b.rs.png_image_free)(&mut ir);
        }
    }
}

// ===========================================================================
// 7a. png_image_write_to_memory
// ===========================================================================

#[test]
fn simplified_write_to_memory() {
    let b = apis();
    let shapes = [(1u32, 1u32), (5, 3), (9, 4), (17, 2), (3, 9), (32, 17), (63, 2)];
    let strides = [
        Stride::Zero,
        Stride::Exact,
        Stride::Pad(4),
        Stride::NegExact,
        Stride::NegPad(4),
    ];
    let mut k = 0u64;
    let mut tally = Tally::default();

    for &(fname, fmt) in ALL_FORMATS.iter() {
        for &(w, h) in shapes.iter() {
            for convert8 in 0..2 as c_int {
                for st in strides.iter() {
                    k += 1;
                    let seed = 0x0407_0000 ^ (k * 0x9E37_79B9_7F4A_7C15);
                    let flags = FLAG_SETS[(k % FLAG_SETS.len() as u64) as usize].1;
                    let wc = make_wcase(seed, w, h, fmt, flags, *st);
                    let what = format!("write_to_memory {fname} {} c8={convert8}", wc.label);

                    // (a) size query: memory == NULL
                    let qc = write_mem(&b.c, &wc, convert8, MemMode::Query, 0);
                    let qr = write_mem(&b.rs, &wc, convert8, MemMode::Query, 0);
                    cmp_write(&format!("{what} [query]"), &qc, &qr);
                    tally.add_ret(qc.ret, &qc.tup.7);

                    if qc.ret == 0 {
                        // Both libraries rejected the case identically; there is
                        // no size to allocate from.
                        continue;
                    }
                    let need = qc.size;

                    // (b) exactly the right size
                    let ec = write_mem(&b.c, &wc, convert8, MemMode::Exact, need);
                    let er = write_mem(&b.rs, &wc, convert8, MemMode::Exact, need);
                    cmp_write(&format!("{what} [exact {need}]"), &ec, &er);
                    eq_dbg(&format!("{what} [exact] size==query"), ec.size, need);

                    // (c) a generous buffer
                    let gc = write_mem(&b.c, &wc, convert8, MemMode::Generous(64), need + 64);
                    let gr = write_mem(&b.rs, &wc, convert8, MemMode::Generous(64), need + 64);
                    cmp_write(&format!("{what} [generous]"), &gc, &gr);

                    // (d) one byte short: documented to return 0 but still to
                    //     report the required size.
                    if need > 0 {
                        let sc = write_mem(&b.c, &wc, convert8, MemMode::Short, need - 1);
                        let sr = write_mem(&b.rs, &wc, convert8, MemMode::Short, need - 1);
                        cmp_write(&format!("{what} [short]"), &sc, &sr);
                        eq_dbg(&format!("{what} [short] return"), sc.ret, 0);
                    }
                }
            }
        }
    }
    tally.require("write_to_memory", &[]);
}

// ===========================================================================
// 7b. png_image_write_to_file / _to_stdio
// ===========================================================================

#[test]
fn simplified_write_to_file_and_stdio() {
    let b = apis();
    let (fopen, fclose) = stdio();
    let shapes = [(1u32, 1u32), (7, 3), (16, 5), (33, 9)];
    let strides = [Stride::Zero, Stride::Pad(3), Stride::NegExact];
    let mut k = 0u64;
    let mut tally = Tally::default();

    for &(fname, fmt) in ALL_FORMATS.iter() {
        for &(w, h) in shapes.iter() {
            for convert8 in 0..2 as c_int {
                for st in strides.iter() {
                    k += 1;
                    let seed = 0x0408_0000 ^ (k * 0x2545_F491_4F6C_DD1D);
                    let flags = FLAG_SETS[(k % FLAG_SETS.len() as u64) as usize].1;
                    let wc = make_wcase(seed, w, h, fmt, flags, *st);
                    let what = format!("write_to_file {fname} {} c8={convert8}", wc.label);

                    // --- png_image_write_to_file ---
                    let mut got: Vec<(c_int, _, Option<Vec<u8>>)> = Vec::new();
                    for a in [&b.c, &b.rs] {
                        let path = tmp_path("wfile");
                        let cs = cpath(&path);
                        let mut img = wc.image();
                        let ret = unsafe {
                            (a.png_image_write_to_file)(
                                &mut img,
                                cs.as_ptr(),
                                convert8,
                                wc.buf.cptr(),
                                wc.rs(),
                                wc.cmap_ptr(),
                            )
                        };
                        let tup = img.cmp_tuple();
                        unsafe { (a.png_image_free)(&mut img) };
                        let content = std::fs::read(&path).ok();
                        let _ = std::fs::remove_file(&path);
                        got.push((ret, tup, content));
                    }
                    tally.add_ret(got[0].0, &got[0].1 .7);
                    eq_dbg(&format!("{what}: return"), got[0].0, got[1].0);
                    eq_dbg(
                        &format!("{what}: png_image"),
                        got[0].1.clone(),
                        got[1].1.clone(),
                    );
                    eq_dbg(
                        &format!("{what}: file exists"),
                        got[0].2.is_some(),
                        got[1].2.is_some(),
                    );
                    if let (Some(c), Some(r)) = (&got[0].2, &got[1].2) {
                        eq_bytes(&format!("{what}: file contents"), c, r);
                    }

                    // --- png_image_write_to_stdio ---
                    let mut got2: Vec<(c_int, _, Vec<u8>)> = Vec::new();
                    for a in [&b.c, &b.rs] {
                        let path = tmp_path("wstdio");
                        let cs = cpath(&path);
                        let fp = unsafe { fopen(cs.as_ptr(), c"wb".as_ptr()) };
                        assert!(!fp.is_null(), "fopen for write failed");
                        let mut img = wc.image();
                        let ret = unsafe {
                            (a.png_image_write_to_stdio)(
                                &mut img,
                                fp,
                                convert8,
                                wc.buf.cptr(),
                                wc.rs(),
                                wc.cmap_ptr(),
                            )
                        };
                        let tup = img.cmp_tuple();
                        unsafe { (a.png_image_free)(&mut img) };
                        unsafe { fclose(fp) };
                        let content = std::fs::read(&path).unwrap_or_default();
                        let _ = std::fs::remove_file(&path);
                        got2.push((ret, tup, content));
                    }
                    let what = format!("write_to_stdio {fname} {} c8={convert8}", wc.label);
                    eq_dbg(&format!("{what}: return"), got2[0].0, got2[1].0);
                    eq_dbg(
                        &format!("{what}: png_image"),
                        got2[0].1.clone(),
                        got2[1].1.clone(),
                    );
                    eq_bytes(&format!("{what}: file contents"), &got2[0].2, &got2[1].2);
                }
            }
        }
    }
    tally.require("write_to_file_and_stdio", &[]);
}

// ===========================================================================
// 8. round trip: write to memory, read it back
// ===========================================================================

#[test]
fn simplified_round_trip() {
    let b = apis();
    let shapes = [(1u32, 1u32), (5, 4), (13, 3), (8, 8), (32, 9)];
    let strides = [Stride::Zero, Stride::Pad(2), Stride::NegExact];
    let read_formats = [
        ("native", None),
        ("GRAY", Some(PNG_FORMAT_GRAY)),
        ("RGBA", Some(PNG_FORMAT_RGBA)),
        ("ABGR", Some(PNG_FORMAT_ABGR)),
        ("LINEAR_RGB_ALPHA", Some(PNG_FORMAT_LINEAR_RGB_ALPHA)),
        ("RGB_COLORMAP", Some(PNG_FORMAT_RGB_COLORMAP)),
    ];
    let mut k = 0u64;
    let mut tally = Tally::default();

    for &(fname, fmt) in ALL_FORMATS.iter() {
        for &(w, h) in shapes.iter() {
            for convert8 in 0..2 as c_int {
                for st in strides.iter() {
                    k += 1;
                    let seed = 0x0409_0000 ^ (k * 0x1234_5678_9abc_def1);
                    let flags = FLAG_SETS[(k % FLAG_SETS.len() as u64) as usize].1;
                    let wc = make_wcase(seed, w, h, fmt, flags, *st);
                    let what = format!("round_trip {fname} {} c8={convert8}", wc.label);

                    // 1) produce the PNG with each library
                    let qc = write_mem(&b.c, &wc, convert8, MemMode::Query, 0);
                    let qr = write_mem(&b.rs, &wc, convert8, MemMode::Query, 0);
                    cmp_write(&format!("{what} [query]"), &qc, &qr);
                    if qc.ret == 0 {
                        continue;
                    }
                    let need = qc.size;
                    let ec = write_mem(&b.c, &wc, convert8, MemMode::Exact, need);
                    let er = write_mem(&b.rs, &wc, convert8, MemMode::Exact, need);
                    cmp_write(&format!("{what} [exact]"), &ec, &er);
                    if ec.ret == 0 {
                        continue;
                    }

                    // 2) read the C-produced bytes back with both libraries
                    for &(rname, rfmt) in read_formats.iter() {
                        for rst in [Stride::Zero, Stride::NegExact].iter() {
                            let cfg = ReadCfg {
                                format: rfmt,
                                stride: *rst,
                                ..Default::default()
                            };
                            let ok = diff_read(
                                &format!("{what} -> read {rname} {rst:?}"),
                                RSrc::Mem(&ec.bytes),
                                &cfg,
                            );
                            tally.add(&ok);
                        }
                    }
                }
            }
        }
    }
    tally.require("round_trip", &[NEED_BACKGROUND]);
}

// ===========================================================================
// 9. png_image_free
// ===========================================================================

#[test]
fn simplified_image_free() {
    let b = apis();
    let png = pb::make_png(0x040a_0001, 9, 4, 8, 6, 0);

    // (a) a fresh, zeroed struct
    for tag in ["fresh", "fresh-again"] {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            unsafe { (a.png_image_free)(&mut img) };
            unsafe { (a.png_image_free)(&mut img) }; // idempotent
            res.push((img.cmp_tuple(), img.opaque.is_null()));
        }
        eq_dbg(&format!("free({tag})"), res[0].clone(), res[1].clone());
    }

    // (b) a struct whose `version` is wrong (opaque still NULL)
    {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            img.version = 0xdead_beef;
            unsafe { (a.png_image_free)(&mut img) };
            res.push((img.cmp_tuple(), img.opaque.is_null()));
        }
        eq_dbg("free(bad version)", res[0].clone(), res[1].clone());
    }

    // (c) after begin_read only, then twice in a row
    {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            let r = unsafe {
                (a.png_image_begin_read_from_memory)(
                    &mut img,
                    png.as_ptr() as *const c_void,
                    png.len(),
                )
            };
            assert_eq!(r, 1, "begin_read must succeed on a valid PNG");
            assert!(!img.opaque.is_null());
            unsafe { (a.png_image_free)(&mut img) };
            let n1 = img.opaque.is_null();
            unsafe { (a.png_image_free)(&mut img) };
            let n2 = img.opaque.is_null();
            res.push((img.cmp_tuple(), n1, n2));
        }
        eq_dbg("free(after begin_read, twice)", res[0].clone(), res[1].clone());
    }

    // (d) after begin_read with the version then corrupted
    {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            let r = unsafe {
                (a.png_image_begin_read_from_memory)(
                    &mut img,
                    png.as_ptr() as *const c_void,
                    png.len(),
                )
            };
            assert_eq!(r, 1);
            img.version = 99;
            unsafe { (a.png_image_free)(&mut img) };
            res.push((img.cmp_tuple(), img.opaque.is_null()));
        }
        eq_dbg("free(after begin_read, bad version)", res[0].clone(), res[1].clone());
    }

    // (e) after a *successful* finish_read (which frees internally) and after a
    //     *failed* finish_read, plus an extra explicit free each time.
    {
        for (tag, buffer_ok) in [("success", true), ("failure", false)] {
            let mut res = Vec::new();
            for a in [&b.c, &b.rs] {
                let mut img = png_image::default();
                let r = unsafe {
                    (a.png_image_begin_read_from_memory)(
                        &mut img,
                        png.as_ptr() as *const c_void,
                        png.len(),
                    )
                };
                assert_eq!(r, 1);
                let nat = natural_stride(img.format, img.width);
                let mut buf = ABuf::new(buffer_size(img.format, img.height, nat).max(8), BUF_FILL);
                let bp = if buffer_ok {
                    buf.ptr()
                } else {
                    ptr::null_mut() // "invalid argument"
                };
                let fr = unsafe {
                    (a.png_image_finish_read)(&mut img, ptr::null(), bp, 0, ptr::null_mut())
                };
                let n0 = img.opaque.is_null();
                unsafe { (a.png_image_free)(&mut img) };
                let n1 = img.opaque.is_null();
                unsafe { (a.png_image_free)(&mut img) };
                res.push((fr, img.cmp_tuple(), n0, n1));
            }
            eq_dbg(
                &format!("free(after finish_read {tag})"),
                res[0].clone(),
                res[1].clone(),
            );
        }
    }

    // (f) after a write (write_to_memory frees internally too)
    {
        let wc = make_wcase(0x040a_0002, 7, 3, PNG_FORMAT_RGBA, 0, Stride::Zero);
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = wc.image();
            let mut size: usize = 0;
            let r = unsafe {
                (a.png_image_write_to_memory)(
                    &mut img,
                    ptr::null_mut(),
                    &mut size,
                    0,
                    wc.buf.cptr(),
                    wc.rs(),
                    ptr::null(),
                )
            };
            let n0 = img.opaque.is_null();
            unsafe { (a.png_image_free)(&mut img) };
            unsafe { (a.png_image_free)(&mut img) };
            res.push((r, size, img.cmp_tuple(), n0, img.opaque.is_null()));
        }
        eq_dbg("free(after write_to_memory)", res[0].clone(), res[1].clone());
    }
}

// ===========================================================================
// 10. the documented "return 0 and set message" argument checks
// ===========================================================================

#[test]
fn simplified_documented_error_returns() {
    let b = apis();
    let png = pb::make_png(0x040b_0001, 8, 4, 8, 6, 0);
    let pal = pb::make_png(0x040b_0002, 8, 4, 8, 3, 0);

    // --- begin_read_from_memory with NULL / zero size ---
    for (tag, ptr_null, len) in [
        ("null-memory", true, 16usize),
        ("zero-size", false, 0usize),
        ("both", true, 0usize),
    ] {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            let p: *const c_void = if ptr_null {
                ptr::null()
            } else {
                png.as_ptr() as *const c_void
            };
            let r = unsafe { (a.png_image_begin_read_from_memory)(&mut img, p, len) };
            let t = img.cmp_tuple();
            unsafe { (a.png_image_free)(&mut img) };
            res.push((r, t, img.opaque.is_null()));
        }
        eq_dbg(&format!("begin_read_from_memory[{tag}]"), res[0].clone(), res[1].clone());
    }

    // --- wrong version on every entry point ---
    for bad in [0u32, 2u32, 0xffff_ffff] {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut v: Vec<(u32, u32, u32, u32, u32, u32, u32, String)> = Vec::new();
            let mut rets: Vec<c_int> = Vec::new();

            let mut img = png_image::default();
            img.version = bad;
            rets.push(unsafe {
                (a.png_image_begin_read_from_memory)(
                    &mut img,
                    png.as_ptr() as *const c_void,
                    png.len(),
                )
            });
            v.push(img.cmp_tuple());

            let mut img = png_image::default();
            img.version = bad;
            let cs = cpath(Path::new("/nonexistent/t04"));
            rets.push(unsafe { (a.png_image_begin_read_from_file)(&mut img, cs.as_ptr()) });
            v.push(img.cmp_tuple());

            let mut img = png_image::default();
            img.version = bad;
            let mut b8 = ABuf::new(64, BUF_FILL);
            rets.push(unsafe {
                (a.png_image_finish_read)(&mut img, ptr::null(), b8.ptr(), 0, ptr::null_mut())
            });
            v.push(img.cmp_tuple());

            let mut img = png_image::default();
            img.version = bad;
            img.width = 2;
            img.height = 2;
            img.format = PNG_FORMAT_RGB;
            let mut size: usize = 0;
            rets.push(unsafe {
                (a.png_image_write_to_memory)(
                    &mut img,
                    ptr::null_mut(),
                    &mut size,
                    0,
                    b8.cptr(),
                    0,
                    ptr::null(),
                )
            });
            v.push(img.cmp_tuple());

            let mut img = png_image::default();
            img.version = bad;
            img.width = 2;
            img.height = 2;
            img.format = PNG_FORMAT_RGB;
            let cs = cpath(Path::new("/nonexistent/t04b"));
            rets.push(unsafe {
                (a.png_image_write_to_file)(&mut img, cs.as_ptr(), 0, b8.cptr(), 0, ptr::null())
            });
            v.push(img.cmp_tuple());

            res.push((rets, v));
        }
        eq_dbg(&format!("bad version {bad}"), res[0].clone(), res[1].clone());
    }

    // --- finish_read with a NULL buffer -> "invalid argument" ---
    // --- finish_read on a colour-mapped format with no colour-map ---
    for (tag, give_buffer, fmt) in [
        ("null-buffer", false, None),
        ("no-colormap", true, Some(PNG_FORMAT_RGB_COLORMAP)),
        ("no-colormap-native", true, None),
    ] {
        let src: &[u8] = if tag == "no-colormap-native" { &pal } else { &png };
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            let r = unsafe {
                (a.png_image_begin_read_from_memory)(
                    &mut img,
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            assert_eq!(r, 1);
            if let Some(f) = fmt {
                img.format = f;
            }
            let nat = natural_stride(img.format, img.width);
            let mut buf = ABuf::new(buffer_size(img.format, img.height, nat).max(8), BUF_FILL);
            let bp = if give_buffer { buf.ptr() } else { ptr::null_mut() };
            let fr =
                unsafe { (a.png_image_finish_read)(&mut img, ptr::null(), bp, 0, ptr::null_mut()) };
            let t = img.cmp_tuple();
            unsafe { (a.png_image_free)(&mut img) };
            res.push((fr, t, buf.bytes().to_vec()));
        }
        eq_dbg(&format!("finish_read[{tag}] ret"), res[0].0, res[1].0);
        eq_dbg(
            &format!("finish_read[{tag}] png_image"),
            res[0].1.clone(),
            res[1].1.clone(),
        );
        eq_bytes(&format!("finish_read[{tag}] buffer"), &res[0].2, &res[1].2);
    }

    // --- finish_read with a row_stride that is too small ---
    for st in [1u32, 2, 3] {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            let r = unsafe {
                (a.png_image_begin_read_from_memory)(
                    &mut img,
                    png.as_ptr() as *const c_void,
                    png.len(),
                )
            };
            assert_eq!(r, 1);
            img.format = PNG_FORMAT_RGBA;
            let nat = natural_stride(img.format, img.width);
            assert!(st < nat);
            let mut buf = ABuf::new(buffer_size(img.format, img.height, nat).max(8), BUF_FILL);
            let fr = unsafe {
                (a.png_image_finish_read)(&mut img, ptr::null(), buf.ptr(), st as i32, ptr::null_mut())
            };
            let t = img.cmp_tuple();
            unsafe { (a.png_image_free)(&mut img) };
            res.push((fr, t));
        }
        eq_dbg(
            &format!("finish_read[stride too small {st}]"),
            res[0].clone(),
            res[1].clone(),
        );
    }

    // --- write_to_memory with NULL memory_bytes / NULL buffer ---
    {
        let wc = make_wcase(0x040b_0003, 5, 3, PNG_FORMAT_RGB, 0, Stride::Zero);
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut out: Vec<(c_int, _)> = Vec::new();

            let mut img = wc.image();
            let mut size: usize = 0;
            out.push((
                unsafe {
                    (a.png_image_write_to_memory)(
                        &mut img,
                        ptr::null_mut(),
                        &mut size,
                        0,
                        ptr::null(),
                        0,
                        ptr::null(),
                    )
                },
                img.cmp_tuple(),
            ));
            unsafe { (a.png_image_free)(&mut img) };

            let mut img = wc.image();
            out.push((
                unsafe {
                    (a.png_image_write_to_memory)(
                        &mut img,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        0,
                        wc.buf.cptr(),
                        0,
                        ptr::null(),
                    )
                },
                img.cmp_tuple(),
            ));
            unsafe { (a.png_image_free)(&mut img) };

            // colour-mapped output with no colour-map
            let cwc = make_wcase(0x040b_0004, 5, 3, PNG_FORMAT_RGB_COLORMAP, 0, Stride::Zero);
            let mut img = cwc.image();
            let mut size: usize = 0;
            out.push((
                unsafe {
                    (a.png_image_write_to_memory)(
                        &mut img,
                        ptr::null_mut(),
                        &mut size,
                        0,
                        cwc.buf.cptr(),
                        0,
                        ptr::null(),
                    )
                },
                img.cmp_tuple(),
            ));
            unsafe { (a.png_image_free)(&mut img) };

            // row_stride smaller than the natural stride
            let mut img = wc.image();
            let mut size: usize = 0;
            out.push((
                unsafe {
                    (a.png_image_write_to_memory)(
                        &mut img,
                        ptr::null_mut(),
                        &mut size,
                        0,
                        wc.buf.cptr(),
                        1,
                        ptr::null(),
                    )
                },
                img.cmp_tuple(),
            ));
            unsafe { (a.png_image_free)(&mut img) };

            res.push(out);
        }
        eq_dbg("write_to_memory arg checks", res[0].clone(), res[1].clone());
    }

    // --- write_to_file / _to_stdio with a NULL name / file ---
    {
        let wc = make_wcase(0x040b_0005, 5, 3, PNG_FORMAT_RGB, 0, Stride::Zero);
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut out: Vec<(c_int, _)> = Vec::new();

            let mut img = wc.image();
            out.push((
                unsafe {
                    (a.png_image_write_to_file)(
                        &mut img,
                        ptr::null(),
                        0,
                        wc.buf.cptr(),
                        0,
                        ptr::null(),
                    )
                },
                img.cmp_tuple(),
            ));
            unsafe { (a.png_image_free)(&mut img) };

            let mut img = wc.image();
            out.push((
                unsafe {
                    (a.png_image_write_to_stdio)(
                        &mut img,
                        ptr::null_mut(),
                        0,
                        wc.buf.cptr(),
                        0,
                        ptr::null(),
                    )
                },
                img.cmp_tuple(),
            ));
            unsafe { (a.png_image_free)(&mut img) };

            let mut img = wc.image();
            out.push((
                unsafe {
                    (a.png_image_write_to_stdio)(
                        &mut img,
                        ptr::null_mut(),
                        0,
                        ptr::null(),
                        0,
                        ptr::null(),
                    )
                },
                img.cmp_tuple(),
            ));
            unsafe { (a.png_image_free)(&mut img) };

            res.push(out);
        }
        eq_dbg("write_to_file/stdio arg checks", res[0].clone(), res[1].clone());
    }

    // --- begin_read_from_stdio with a NULL FILE* ---
    {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            let r = unsafe { (a.png_image_begin_read_from_stdio)(&mut img, ptr::null_mut()) };
            let t = img.cmp_tuple();
            unsafe { (a.png_image_free)(&mut img) };
            res.push((r, t));
        }
        eq_dbg("begin_read_from_stdio(NULL)", res[0].clone(), res[1].clone());
    }

    // --- begin_read_from_file with a NULL name ---
    {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            let mut img = png_image::default();
            let r = unsafe { (a.png_image_begin_read_from_file)(&mut img, ptr::null()) };
            let t = img.cmp_tuple();
            unsafe { (a.png_image_free)(&mut img) };
            res.push((r, t));
        }
        eq_dbg("begin_read_from_file(NULL)", res[0].clone(), res[1].clone());
    }
}

// ===========================================================================
// 11. randomised read matrix (fixed seed): shape x format x flags x
//     background x row_stride x input source, all mixed
// ===========================================================================

const ALL_STRIDES: [Stride; 9] = [
    Stride::Zero,
    Stride::Exact,
    Stride::Pad(1),
    Stride::Pad(5),
    Stride::Pad(64),
    Stride::NegExact,
    Stride::NegPad(1),
    Stride::NegPad(5),
    Stride::NegPad(64),
];

#[test]
fn simplified_read_randomised_matrix() {
    let mut rng = Rng::new(0x040c_2024);
    let mut tally = Tally::default();
    for i in 0..30000u64 {
        let (bd, ct) = *rng.pick(&SHAPES);
        let il = rng.bool() as u8;
        let w = rng.range(1, 64);
        let h = rng.range(1, 24);
        let png = pb::make_png(0x040c_0000 ^ i.wrapping_mul(0x9E37_79B9), w, h, bd, ct, il);

        let fmt = if rng.bool() {
            Some(rng.pick(&ALL_FORMATS).1)
        } else {
            None
        };
        let flags = if rng.bool() {
            Some(rng.pick(&FLAG_SETS).1)
        } else {
            None
        };
        let bg = if rng.bool() {
            Some(png_color {
                red: rng.next_u8(),
                green: rng.next_u8(),
                blue: rng.next_u8(),
            })
        } else {
            None
        };
        let stride = *rng.pick(&ALL_STRIDES);
        let cfg = ReadCfg {
            format: fmt,
            flags,
            background: bg,
            stride,
            give_colormap: true,
        };
        let what = format!(
            "rnd#{i} bd={bd} ct={ct} il={il} {w}x{h} fmt={fmt:?} flags={flags:?} \
             bg={bg:?} stride={stride:?}"
        );

        // Every eighth case goes through a temporary file instead of memory.
        let via = rng.below(8);
        if via < 6 {
            tally.add(&diff_read(&what, RSrc::Mem(&png), &cfg));
        } else {
            let path = tmp_path("rnd");
            std::fs::write(&path, &png).expect("write temp png");
            if via == 6 {
                tally.add(&diff_read(&format!("{what} [file]"), RSrc::File(&path), &cfg));
            } else {
                tally.add(&diff_read(
                    &format!("{what} [stdio]"),
                    RSrc::Stdio(&path),
                    &cfg,
                ));
            }
            let _ = std::fs::remove_file(&path);
        }
    }
    tally.require("read_randomised_matrix", &[NEED_BACKGROUND]);
}

// ===========================================================================
// 12. randomised write matrix + read-back (fixed seed)
// ===========================================================================

#[test]
fn simplified_write_randomised_matrix() {
    let b = apis();
    let mut rng = Rng::new(0x040d_2024);
    let mut tally = Tally::default();
    let mut rtally = Tally::default();

    for i in 0..12000u64 {
        let fmt = rng.pick(&ALL_FORMATS).1;
        let w = rng.range(1, 64);
        let h = rng.range(1, 20);
        let flags = rng.pick(&FLAG_SETS).1;
        let stride = *rng.pick(&ALL_STRIDES);
        let convert8 = rng.below(2) as c_int;
        let seed = 0x040d_0000 ^ i.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let wc = make_wcase(seed, w, h, fmt, flags, stride);
        let what = format!("wrnd#{i} {} c8={convert8}", wc.label);

        let qc = write_mem(&b.c, &wc, convert8, MemMode::Query, 0);
        let qr = write_mem(&b.rs, &wc, convert8, MemMode::Query, 0);
        cmp_write(&format!("{what} [query]"), &qc, &qr);
        tally.add_ret(qc.ret, &qc.tup.7);
        if qc.ret == 0 {
            continue;
        }
        let need = qc.size;

        let ec = write_mem(&b.c, &wc, convert8, MemMode::Exact, need);
        let er = write_mem(&b.rs, &wc, convert8, MemMode::Exact, need);
        cmp_write(&format!("{what} [exact]"), &ec, &er);

        let extra = 1 + rng.below(97) as usize;
        let gc = write_mem(&b.c, &wc, convert8, MemMode::Generous(extra), need + extra);
        let gr = write_mem(&b.rs, &wc, convert8, MemMode::Generous(extra), need + extra);
        cmp_write(&format!("{what} [generous {extra}]"), &gc, &gr);

        if need > 1 {
            let avail = need - 1 - rng.below(need as u32 / 2) as usize;
            let sc = write_mem(&b.c, &wc, convert8, MemMode::Short, avail);
            let sr = write_mem(&b.rs, &wc, convert8, MemMode::Short, avail);
            cmp_write(&format!("{what} [short {avail}]"), &sc, &sr);
            eq_dbg(&format!("{what} [short] return"), sc.ret, 0);
            eq_dbg(&format!("{what} [short] size"), sc.size, need);
        }

        // Every fourth case also goes to a file and to a FILE*.
        if i % 4 == 0 {
            let (fopen, fclose) = stdio();
            let mut file_out: Vec<(c_int, Vec<u8>)> = Vec::new();
            let mut stdio_out: Vec<(c_int, Vec<u8>)> = Vec::new();
            for a in [&b.c, &b.rs] {
                let path = tmp_path("wrndf");
                let cs = cpath(&path);
                let mut img = wc.image();
                let ret = unsafe {
                    (a.png_image_write_to_file)(
                        &mut img,
                        cs.as_ptr(),
                        convert8,
                        wc.buf.cptr(),
                        wc.rs(),
                        wc.cmap_ptr(),
                    )
                };
                unsafe { (a.png_image_free)(&mut img) };
                file_out.push((ret, std::fs::read(&path).unwrap_or_default()));
                let _ = std::fs::remove_file(&path);

                let path = tmp_path("wrnds");
                let cs = cpath(&path);
                let fp = unsafe { fopen(cs.as_ptr(), c"wb".as_ptr()) };
                assert!(!fp.is_null());
                let mut img = wc.image();
                let ret = unsafe {
                    (a.png_image_write_to_stdio)(
                        &mut img,
                        fp,
                        convert8,
                        wc.buf.cptr(),
                        wc.rs(),
                        wc.cmap_ptr(),
                    )
                };
                unsafe { (a.png_image_free)(&mut img) };
                unsafe { fclose(fp) };
                stdio_out.push((ret, std::fs::read(&path).unwrap_or_default()));
                let _ = std::fs::remove_file(&path);
            }
            eq_dbg(&format!("{what} [to_file] return"), file_out[0].0, file_out[1].0);
            eq_bytes(&format!("{what} [to_file] bytes"), &file_out[0].1, &file_out[1].1);
            eq_dbg(
                &format!("{what} [to_stdio] return"),
                stdio_out[0].0,
                stdio_out[1].0,
            );
            eq_bytes(
                &format!("{what} [to_stdio] bytes"),
                &stdio_out[0].1,
                &stdio_out[1].1,
            );
            // memory / file / stdio must all produce the very same PNG
            eq_bytes(&format!("{what} memory==file"), &ec.bytes, &file_out[0].1);
            eq_bytes(&format!("{what} memory==stdio"), &ec.bytes, &stdio_out[0].1);
        }

        // ... and read the result back with a random output format.
        if ec.ret == 1 {
            let rfmt = if rng.bool() {
                Some(rng.pick(&ALL_FORMATS).1)
            } else {
                None
            };
            let rst = *rng.pick(&ALL_STRIDES);
            let rbg = if rng.bool() {
                Some(png_color {
                    red: rng.next_u8(),
                    green: rng.next_u8(),
                    blue: rng.next_u8(),
                })
            } else {
                None
            };
            let cfg = ReadCfg {
                format: rfmt,
                flags: Some(rng.pick(&FLAG_SETS).1),
                background: rbg,
                stride: rst,
                give_colormap: true,
            };
            rtally.add(&diff_read(
                &format!("{what} -> read back {rfmt:?} {rst:?}"),
                RSrc::Mem(&ec.bytes),
                &cfg,
            ));
        }
    }
    tally.require("write_randomised_matrix", &[]);
    rtally.require("write_randomised_matrix[read-back]", &[NEED_BACKGROUND]);
}

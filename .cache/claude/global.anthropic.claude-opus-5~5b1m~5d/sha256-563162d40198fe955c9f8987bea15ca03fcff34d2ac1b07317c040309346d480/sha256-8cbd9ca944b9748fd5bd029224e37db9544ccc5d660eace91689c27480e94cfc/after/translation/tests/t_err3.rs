//! Differential verification of the *error paths* of `pngread.c` and
//! `pngrtran.c` (rows 88..185 of `translation/ERRORS.md`, see
//! `.verify/E3.md`).
//!
//! Every assertion drives the reference C `libpng.so` and the translated Rust
//! `liblibpng.so` through `dlsym` with exactly the same invalid input and
//! requires bit-identical message logs (order included) plus the same
//! return values / error sentinels.
//!
//! Row numbers in the `// row N` comments refer to `.verify/E3.md`.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod common;

use common::api;
use common::*;
use libloading::Library;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// extra entry points not present in tests/common/api.rs
// ---------------------------------------------------------------------------

crate::decl_api! {
    fn png_read_transform_info(pp: png_structp, ip: png_infop);
    fn png_do_read_transformations(pp: png_structp, ri: *mut png_row_info);
    fn png_init_read_transformations(pp: png_structp);
    fn png_resolve_file_gamma(pp: png_structp) -> png_fixed_point;
}

// ---------------------------------------------------------------------------
// differential driver
// ---------------------------------------------------------------------------

/// The reference `libpng.so` references `floor` from libm but is not linked
/// against it (lazy binding), and the Rust test executable does not pull libm
/// in either.  Make libm part of the global symbol scope *before* either
/// library is dlopen()ed so that the lazy `floor` relocation can be resolved.
fn ensure_libm() {
    static ONCE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        use libloading::os::unix as u;
        for name in ["libm.so.6", "libm.so"] {
            if let Ok(lib) = u::Library::open(Some(name), u::RTLD_NOW | u::RTLD_GLOBAL) {
                std::mem::forget(lib);
                return true;
            }
        }
        false
    });
}

/// Run `f` against both libraries inside `capture` and require identical
/// message logs and identical results.  Returns the reference-C run so that
/// the caller can additionally pin down *which* rejection site fired.
fn dual<T, F>(label: &str, f: F) -> Run<T>
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&'static Library) -> T,
{
    ensure_libm();
    if std::env::var_os("T_ERR3_TRACE").is_some() {
        eprintln!("[case] {label}");
    }
    let l = libs();
    let c = capture(|| f(&l.c));
    let r = capture(|| f(&l.rs));
    c.assert_eq(&r, label);
    c
}

fn msgs<T>(run: &Run<T>) -> Vec<String> {
    run.log.iter().map(|m| m.to_string()).collect()
}

/// Assert that the site under test really was reached (guards against a test
/// that silently exercises nothing).
fn want<T>(run: &Run<T>, label: &str, needle: &str) {
    let hit = run.log.iter().any(|m| {
        let b = match m {
            Msg::Err(b) => b,
            Msg::Warn(b) => b,
        };
        String::from_utf8_lossy(b).contains(needle)
    });
    assert!(
        hit,
        "{label}: expected a libpng message containing {needle:?}, C log = {:?}",
        msgs(run)
    );
}

/// Assert that nothing at all was reported.
fn want_silent<T>(run: &Run<T>, label: &str) {
    assert!(
        run.log.is_empty(),
        "{label}: expected no messages, C log = {:?}",
        msgs(run)
    );
}

// ---------------------------------------------------------------------------
// PNG stream construction
// ---------------------------------------------------------------------------

const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let mut c = name.to_vec();
    c.extend_from_slice(data);
    v.extend_from_slice(&crc32(&c).to_be_bytes());
    v
}

/// zlib stream using only *stored* deflate blocks (no compressor needed).
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut v = vec![0x78u8, 0x01];
    if raw.is_empty() {
        v.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let mut off = 0usize;
        while off < raw.len() {
            let n = core::cmp::min(65535, raw.len() - off);
            v.push(if off + n == raw.len() { 1 } else { 0 });
            v.extend_from_slice(&(n as u16).to_le_bytes());
            v.extend_from_slice(&(!(n as u16)).to_le_bytes());
            v.extend_from_slice(&raw[off..off + n]);
            off += n;
        }
    }
    v.extend_from_slice(&adler32(raw).to_be_bytes());
    v
}

fn ihdr_raw(w: u32, h: u32, bd: u8, ct: u8, il: u8, cm: u8, ft: u8) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&w.to_be_bytes());
    d.extend_from_slice(&h.to_be_bytes());
    d.push(bd);
    d.push(ct);
    d.push(cm);
    d.push(ft);
    d.push(il);
    chunk(b"IHDR", &d)
}

fn ihdr(w: u32, h: u32, bd: u8, ct: u8, il: u8) -> Vec<u8> {
    ihdr_raw(w, h, bd, ct, il, 0, 0)
}

fn nchannels(ct: u8) -> usize {
    match ct {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

fn rowbytes(w: u32, bd: u8, ct: u8) -> usize {
    (w as usize * nchannels(ct) * bd as usize + 7) / 8
}

/// Filter-byte-0 rows filled with a deterministic pattern.
fn raw_rows(w: u32, h: u32, bd: u8, ct: u8) -> Vec<u8> {
    let rb = rowbytes(w, bd, ct);
    let mut v = Vec::new();
    let mut n: u8 = 1;
    for _ in 0..h {
        v.push(0);
        for _ in 0..rb {
            v.push(n);
            n = n.wrapping_add(17);
        }
    }
    v
}

const A7_XSTART: [u32; 7] = [0, 4, 0, 2, 0, 1, 0];
const A7_YSTART: [u32; 7] = [0, 0, 4, 0, 2, 0, 1];
const A7_XINC: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];
const A7_YINC: [u32; 7] = [8, 8, 8, 4, 4, 2, 2];

fn raw_rows_adam7(w: u32, h: u32, bd: u8, ct: u8) -> Vec<u8> {
    let mut v = Vec::new();
    let mut n: u8 = 1;
    for p in 0..7 {
        let pw = if w > A7_XSTART[p] {
            (w - A7_XSTART[p] + A7_XINC[p] - 1) / A7_XINC[p]
        } else {
            0
        };
        let ph = if h > A7_YSTART[p] {
            (h - A7_YSTART[p] + A7_YINC[p] - 1) / A7_YINC[p]
        } else {
            0
        };
        if pw == 0 || ph == 0 {
            continue;
        }
        let rb = rowbytes(pw, bd, ct);
        for _ in 0..ph {
            v.push(0);
            for _ in 0..rb {
                v.push(n);
                n = n.wrapping_add(17);
            }
        }
    }
    v
}

/// A whole PNG: signature, IHDR, optional PLTE/tRNS, one IDAT, IEND.
fn build(
    w: u32,
    h: u32,
    bd: u8,
    ct: u8,
    il: u8,
    plte: Option<&[u8]>,
    trns: Option<&[u8]>,
    raw: &[u8],
) -> Vec<u8> {
    let mut v = SIG.to_vec();
    v.extend(ihdr(w, h, bd, ct, il));
    if let Some(p) = plte {
        v.extend(chunk(b"PLTE", p));
    }
    if let Some(t) = trns {
        v.extend(chunk(b"tRNS", t));
    }
    v.extend(chunk(b"IDAT", &zlib_stored(raw)));
    v.extend(chunk(b"IEND", &[]));
    v
}

fn simple(w: u32, h: u32, bd: u8, ct: u8) -> Vec<u8> {
    build(w, h, bd, ct, 0, None, None, &raw_rows(w, h, bd, ct))
}

fn palette_png(w: u32, h: u32, nentries: usize, pixel: u8) -> Vec<u8> {
    let mut plte = Vec::new();
    for i in 0..nentries {
        plte.push((i * 7) as u8);
        plte.push((i * 11) as u8);
        plte.push((i * 13) as u8);
    }
    let mut raw = Vec::new();
    for _ in 0..h {
        raw.push(0);
        for _ in 0..w {
            raw.push(pixel);
        }
    }
    build(w, h, 8, 3, 0, Some(&plte), None, &raw)
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

unsafe fn reader(l: &'static Library, bytes: &[u8]) -> (png_structp, png_infop) {
    src_set(bytes);
    let pp = api::new_reader(l);
    let ip = api::png_create_info_struct(l, pp);
    assert!(!ip.is_null());
    (pp, ip)
}

/// A row-pointer array for `png_read_image` / `png_read_png`.
struct Rows {
    _data: Vec<Vec<u8>>,
    ptrs: Vec<png_bytep>,
}

impl Rows {
    fn new(h: usize, stride: usize) -> Rows {
        let mut data: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; stride]).collect();
        let ptrs = data.iter_mut().map(|r| r.as_mut_ptr()).collect();
        Rows { _data: data, ptrs }
    }
    fn as_ptr(&mut self) -> *mut png_bytep {
        self.ptrs.as_mut_ptr()
    }
}

// ===========================================================================
// pngread.c: png_read_info
// ===========================================================================

#[test]
fn read_info_missing_ihdr_row88() {
    // row 88: an IDAT before any IHDR
    let mut b = SIG.to_vec();
    b.extend(chunk(b"IDAT", &zlib_stored(&[0u8, 1])));
    b.extend(chunk(b"IEND", &[]));
    let r = dual("row88 missing IHDR before IDAT", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        src_pos()
    });
    want(&r, "row88", "Missing IHDR before IDAT"); // row 88
    assert!(r.out.is_none(), "row88: expected png_chunk_error to unwind");
}

#[test]
fn read_info_missing_plte_row89() {
    // row 89: colour type 3 with no PLTE before IDAT
    let mut b = SIG.to_vec();
    b.extend(ihdr(1, 1, 8, 3, 0));
    b.extend(chunk(b"IDAT", &zlib_stored(&[0u8, 0])));
    b.extend(chunk(b"IEND", &[]));
    let r = dual("row89 missing PLTE before IDAT", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        src_pos()
    });
    want(&r, "row89", "Missing PLTE before IDAT"); // row 89
    assert!(r.out.is_none(), "row89: expected png_chunk_error to unwind");
}

// ===========================================================================
// pngread.c: png_read_update_info / png_start_read_image duplicate calls
// ===========================================================================

#[test]
fn update_info_and_start_read_image_duplicate_rows90_91() {
    let b = simple(4, 2, 8, 0);

    const M90: &str = "png_read_update_info/png_start_read_image: duplicate call";
    const M91: &str = "png_start_read_image/png_read_update_info: duplicate call";

    // row 90: png_read_update_info called twice
    {
        let b = b.clone();
        let r = dual("row90 update_info twice", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_read_update_info(l, pp, ip);
            api::png_read_update_info(l, pp, ip);
            0u32
        });
        want(&r, "row90", M90); // row 90
    }
    // row 90: png_start_read_image then png_read_update_info
    {
        let b = b.clone();
        let r = dual("row90 start_read_image then update_info", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_start_read_image(l, pp);
            api::png_read_update_info(l, pp, ip);
            0u32
        });
        want(&r, "row90 (b)", M90); // row 90
    }
    // row 91: png_start_read_image called twice
    {
        let b = b.clone();
        let r = dual("row91 start_read_image twice", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_start_read_image(l, pp);
            api::png_start_read_image(l, pp);
            0u32
        });
        want(&r, "row91", M91); // row 91
    }
    // row 91: png_read_update_info then png_start_read_image
    {
        let b = b.clone();
        let r = dual("row91 update_info then start_read_image", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_read_update_info(l, pp, ip);
            api::png_start_read_image(l, pp);
            0u32
        });
        want(&r, "row91 (b)", M91); // row 91
    }
    // a single call of either must be silent
    {
        let b = b.clone();
        let r = dual("update_info once is silent", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_read_update_info(l, pp, ip);
            0u32
        });
        want_silent(&r, "update_info once");
    }
}

// ===========================================================================
// pngread.c: png_read_row
// ===========================================================================

#[test]
fn read_row_without_idat_row99() {
    // row 99: png_read_row on a struct that has never seen an IDAT
    let r = dual("row99 invalid attempt to read row data", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        let mut buf = vec![0u8; 64];
        api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
        0u32
    });
    want(&r, "row99", "Invalid attempt to read row data"); // row 99
}

#[test]
fn read_row_bad_adaptive_filter_row100() {
    // row 100: filter byte 5 (>= PNG_FILTER_VALUE_LAST)
    let raw = vec![5u8, 0x80];
    let b = build(1, 1, 8, 0, 0, None, None, &raw);
    let r = dual("row100 bad adaptive filter value", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        let mut buf = vec![0u8; 64];
        api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
        0u32
    });
    want(&r, "row100", "bad adaptive filter value"); // row 100
    // every invalid filter byte value
    for f in [5u8, 6, 100, 255] {
        let raw = vec![f, 0x80];
        let b = build(1, 1, 8, 0, 0, None, None, &raw);
        let label = format!("row100 filter byte {f}");
        let r = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            let mut buf = vec![0u8; 64];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            0u32
        });
        want(&r, &label, "bad adaptive filter value"); // row 100
    }
    // the four valid filter bytes must be accepted
    for f in [0u8, 1, 2, 3, 4] {
        let raw = vec![f, 0x80];
        let b = build(1, 1, 8, 0, 0, None, None, &raw);
        let label = format!("valid filter byte {f}");
        let r = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            let mut buf = vec![0u8; 64];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            buf[0]
        });
        want_silent(&r, &label);
    }
}

thread_local! {
    static UT_CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Blows the transformed pixel depth up past `maximum_pixel_depth`.
unsafe extern "C-unwind" fn ut_grow(_pp: png_structp, ri: *mut png_row_info, _row: png_bytep) {
    UT_CALLS.with(|c| c.set(c.get() + 1));
    (*ri).bit_depth = 16;
    (*ri).channels = 4;
}

/// Leaves row 0 alone, changes the depth on row 1.
unsafe extern "C-unwind" fn ut_grow_second(
    _pp: png_structp,
    ri: *mut png_row_info,
    _row: png_bytep,
) {
    let n = UT_CALLS.with(|c| {
        let v = c.get();
        c.set(v + 1);
        v
    });
    if n >= 1 {
        (*ri).channels = 2;
    }
}

#[test]
fn read_row_transformed_depth_rows101_102() {
    // row 101: "sequential row overflow" - the user transform reports a pixel
    // depth larger than the maximum computed by png_read_start_row.
    {
        let b = simple(4, 1, 8, 0);
        let r = dual("row101 sequential row overflow", move |l| unsafe {
            UT_CALLS.with(|c| c.set(0));
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_read_user_transform_fn(l, pp, Some(ut_grow));
            api::png_read_update_info(l, pp, ip);
            let mut buf = vec![0u8; 256];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            0u32
        });
        want(&r, "row101", "sequential row overflow"); // row 101
    }
    // row 102: "internal sequential row size calculation error" - the depth
    // reported for row 1 differs from the depth recorded for row 0.
    {
        let b = simple(4, 2, 8, 0);
        let r = dual(
            "row102 internal sequential row size calculation error",
            move |l| unsafe {
                UT_CALLS.with(|c| c.set(0));
                let (pp, ip) = reader(l, &b);
                api::png_read_info(l, pp, ip);
                api::png_set_read_user_transform_fn(l, pp, Some(ut_grow_second));
                api::png_read_update_info(l, pp, ip);
                let mut buf = vec![0u8; 256];
                api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
                api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
                0u32
            },
        );
        want(&r, "row102", "internal sequential row size calculation error"); // row 102
    }
}

#[test]
fn read_row_past_last_row() {
    // png_read_row called more times than there are rows in the image.
    let b = simple(4, 2, 8, 0);
    dual("read_row past the last row", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        let mut buf = vec![0u8; 64];
        for _ in 0..4 {
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
        }
        0u32
    });
}

// ===========================================================================
// pngread.c: png_read_image
// ===========================================================================

#[test]
fn read_image_interlaced_without_handler_row103() {
    // row 103: png_read_update_info called before png_set_interlace_handling
    let w = 8u32;
    let h = 8u32;
    let raw = raw_rows_adam7(w, h, 8, 0);
    let b = build(w, h, 8, 0, 1, None, None, &raw);
    let r = dual("row103 interlace handling should be turned on", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        api::png_read_update_info(l, pp, ip);
        let mut rows = Rows::new(h as usize, 64);
        api::png_read_image(l, pp, rows.as_ptr());
        api::png_get_current_pass_number(l, pp)
    });
    // row 103
    want(
        &r,
        "row103",
        "Interlace handling should be turned on when using png_read_image",
    );
}

#[test]
fn read_image_interlaced_with_handler_is_quiet() {
    let w = 8u32;
    let h = 8u32;
    let raw = raw_rows_adam7(w, h, 8, 0);
    let b = build(w, h, 8, 0, 1, None, None, &raw);
    let r = dual("interlaced read_image with handler", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        let passes = api::png_set_interlace_handling(l, pp);
        api::png_read_update_info(l, pp, ip);
        let mut rows = Rows::new(h as usize, 64);
        api::png_read_image(l, pp, rows.as_ptr());
        passes
    });
    want_silent(&r, "interlaced read_image with handler");
}

// ===========================================================================
// pngread.c: png_read_end
// ===========================================================================

#[test]
fn read_end_palette_index_out_of_range_row105() {
    // row 105: a palette index >= num_palette was seen while reading rows
    let b = palette_png(4, 1, 2, 3);
    let r = dual("row105 read palette index exceeding num_palette", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_set_check_for_invalid_index(l, pp, 1);
        api::png_read_info(l, pp, ip);
        let mut buf = vec![0u8; 64];
        api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
        api::png_read_end(l, pp, ip);
        api::png_get_palette_max(l, pp, ip)
    });
    want(&r, "row105", "Read palette index exceeding num_palette"); // row 105
    // in-range indices must stay quiet
    let b = palette_png(4, 1, 4, 3);
    let r = dual("row105 in-range palette index", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_set_check_for_invalid_index(l, pp, 1);
        api::png_read_info(l, pp, ip);
        let mut buf = vec![0u8; 64];
        api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
        api::png_read_end(l, pp, ip);
        api::png_get_palette_max(l, pp, ip)
    });
    want_silent(&r, "row105 in-range palette index");
}

/// gray 1x1 stream with a tEXt and then a second, empty IDAT after the image
/// data: `HAVE_CHUNK_AFTER_IDAT` is set when the second IDAT is reached.
fn png_with_trailing_idat() -> Vec<u8> {
    let mut v = SIG.to_vec();
    v.extend(ihdr(1, 1, 8, 0, 0));
    v.extend(chunk(b"IDAT", &zlib_stored(&[0u8, 0x80])));
    v.extend(chunk(b"tEXt", b"K\0v"));
    v.extend(chunk(b"IDAT", &[]));
    v.extend(chunk(b"IEND", &[]));
    v
}

#[test]
fn read_end_too_many_idats_rows106_107() {
    // row 107: the ordinary IDAT branch of png_read_end
    {
        let b = png_with_trailing_idat();
        let r = dual("row107 ..Too many IDATs found", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            let mut buf = vec![0u8; 64];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            api::png_read_end(l, pp, ip);
            0u32
        });
        want(&r, "row107", "..Too many IDATs found"); // row 107
    }
    // row 106: the same check inside the "handle as unknown" branch, reached by
    // registering IDAT itself in the unknown-chunk list.  PNG_HANDLE_CHUNK_
    // ALWAYS is required: with PNG_HANDLE_CHUNK_NEVER png_read_info would
    // reject the first IDAT with "unhandled critical chunk" before png_read_end
    // ever runs.  With ALWAYS png_read_info swallows the first IDAT as an
    // unknown chunk (idat_size = 0), so no rows are read here.
    {
        let b = png_with_trailing_idat();
        let r = dual("row106 .Too many IDATs found", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_set_keep_unknown_chunks(
                l,
                pp,
                PNG_HANDLE_CHUNK_ALWAYS,
                b"IDAT\0".as_ptr(),
                1,
            );
            api::png_read_info(l, pp, ip);
            api::png_read_end(l, pp, ip);
            0u32
        });
        // row 106 - the exact same text but with a single leading '.'
        let hit106 = r.log.iter().any(|m| {
            let b = match m {
                Msg::Err(b) => b,
                Msg::Warn(b) => b,
            };
            let s = String::from_utf8_lossy(b);
            s.contains(".Too many IDATs found") && !s.contains("..Too many IDATs found")
        });
        assert!(
            hit106,
            "row106: expected \".Too many IDATs found\", C log = {:?}",
            msgs(&r)
        );
    }
}

// ===========================================================================
// pngread.c: png_read_png
// ===========================================================================

#[test]
fn read_png_image_too_high_row108() {
    // row 108: info_ptr->height > PNG_UINT_32_MAX/sizeof(png_bytep)
    let mut b = SIG.to_vec();
    b.extend(ihdr(1, 0x2000_0000, 8, 0, 0));
    b.extend(chunk(b"IDAT", &zlib_stored(&[0u8, 0x80])));
    b.extend(chunk(b"IEND", &[]));
    let r = dual("row108 image is too high for png_read_png", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_set_user_limits(l, pp, PNG_UINT_31_MAX, PNG_UINT_31_MAX);
        api::png_read_png(l, pp, ip, 0, std::ptr::null_mut());
        0u32
    });
    // row 108
    want(&r, "row108", "Image is too high to process with png_read_png()");
}

#[test]
fn read_png_conflicting_transform_bits() {
    // png_read_png with mutually conflicting `transforms` bits; all of the
    // PNG_TRANSFORM_* handling in this build is compiled in, so the interesting
    // part is that both libraries agree on the (possibly silent) outcome.
    let cases: Vec<(&str, Vec<u8>, c_int)> = vec![
        (
            "STRIP_16|SCALE_16 on 16-bit",
            simple(4, 2, 16, 0),
            PNG_TRANSFORM_STRIP_16 | PNG_TRANSFORM_SCALE_16,
        ),
        (
            "EXPAND_16|STRIP_16 on 8-bit",
            simple(4, 2, 8, 0),
            PNG_TRANSFORM_EXPAND_16 | PNG_TRANSFORM_STRIP_16,
        ),
        (
            "EXPAND_16|SCALE_16 on 16-bit",
            simple(4, 2, 16, 2),
            PNG_TRANSFORM_EXPAND_16 | PNG_TRANSFORM_SCALE_16,
        ),
        (
            "STRIP_FILLER_BEFORE|STRIP_FILLER_AFTER",
            simple(4, 2, 8, 6),
            PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER,
        ),
        (
            "PACKING on 16-bit",
            simple(4, 2, 16, 0),
            PNG_TRANSFORM_PACKING,
        ),
        (
            "INVERT_MONO on colour",
            simple(4, 2, 8, 2),
            PNG_TRANSFORM_INVERT_MONO,
        ),
        (
            "SHIFT without sBIT",
            simple(4, 2, 8, 2),
            PNG_TRANSFORM_SHIFT,
        ),
        (
            "SWAP_ALPHA|INVERT_ALPHA without alpha",
            simple(4, 2, 8, 2),
            PNG_TRANSFORM_SWAP_ALPHA | PNG_TRANSFORM_INVERT_ALPHA,
        ),
        (
            "STRIP_ALPHA|GRAY_TO_RGB|EXPAND",
            simple(4, 2, 8, 4),
            PNG_TRANSFORM_STRIP_ALPHA | PNG_TRANSFORM_GRAY_TO_RGB | PNG_TRANSFORM_EXPAND,
        ),
        ("IDENTITY", simple(4, 2, 8, 0), PNG_TRANSFORM_IDENTITY),
    ];
    for (name, bytes, tr) in cases {
        let label = format!("png_read_png transforms: {name}");
        dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &bytes);
            api::png_read_png(l, pp, ip, tr, std::ptr::null_mut());
            (
                api::png_get_rowbytes(l, pp, ip),
                api::png_get_color_type(l, pp, ip),
                api::png_get_bit_depth(l, pp, ip),
                api::png_get_channels(l, pp, ip),
            )
        });
    }
}

#[test]
fn read_png_null_info_and_null_struct() {
    let b = simple(2, 2, 8, 0);
    // NULL info_ptr: documented no-op
    {
        let b = b.clone();
        dual("png_read_png NULL info_ptr", move |l| unsafe {
            let (pp, _ip) = reader(l, &b);
            api::png_read_png(l, pp, std::ptr::null_mut(), 0, std::ptr::null_mut());
            src_pos()
        });
    }
    // NULL png_ptr: documented no-op
    dual("png_read_png NULL png_ptr", |l| unsafe {
        api::png_read_png(
            l,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );
        0u32
    });
}

// ===========================================================================
// pngread.c: simplified read API
// ===========================================================================

type ImgKey = (u32, u32, u32, u32, u32, u32, u32, String);
type ImgOut = (c_int, c_int, ImgKey);

/// The simplified API traps errors internally and reports them through
/// `png_image::message`, so the expectation has to be checked there.
fn want_img(run: &Run<ImgOut>, label: &str, needle: &str) {
    let out = run
        .out
        .as_ref()
        .unwrap_or_else(|| panic!("{label}: the simplified API must not unwind"));
    assert!(
        out.2 .7.contains(needle),
        "{label}: expected png_image::message to contain {needle:?}, got {:?} (begin={}, finish={})",
        out.2 .7,
        out.0,
        out.1
    );
    assert_eq!(out.1, 0, "{label}: png_image_finish_read should have failed");
}

/// Drive `png_image_begin_read_from_memory` + `png_image_finish_read` with a
/// background colour supplied.
fn img_case<F>(label: &str, bytes: Vec<u8>, tweak: F) -> Run<ImgOut>
where
    F: Fn(&mut png_image) + Copy,
{
    img_case_bg(label, bytes, tweak, true)
}

/// As [`img_case`] but `background` may be NULL.
fn img_case_bg<F>(label: &str, bytes: Vec<u8>, tweak: F, use_bg: bool) -> Run<ImgOut>
where
    F: Fn(&mut png_image) + Copy,
{
    dual(label, move |l| unsafe {
        let mut img = png_image {
            version: PNG_IMAGE_VERSION,
            ..Default::default()
        };
        let r1 = api::png_image_begin_read_from_memory(
            l,
            &mut img,
            bytes.as_ptr() as *const c_void,
            bytes.len(),
        );
        if r1 == 0 {
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            return (r1, 0, k);
        }
        tweak(&mut img);
        let mut buf = vec![0u8; 1 << 16];
        let mut cmap = vec![0u8; 1 << 13];
        let bg = png_color {
            red: 1,
            green: 2,
            blue: 3,
        };
        let bgp: *const png_color = if use_bg { &bg } else { std::ptr::null() };
        let r2 = api::png_image_finish_read(
            l,
            &mut img,
            bgp,
            buf.as_mut_ptr() as *mut c_void,
            0,
            cmap.as_mut_ptr() as *mut c_void,
        );
        let k = img.cmp_key();
        api::png_image_free(l, &mut img);
        (r1, r2, k)
    })
}

#[test]
fn image_memory_read_beyond_end_row123() {
    // row 123: "read beyond end of data" for every truncation point of a valid
    // PNG that is reached while reading the header.
    let full = simple(2, 2, 8, 2);
    for n in [1usize, 2, 8, 9, 16, 20, 24, 30] {
        let b = full[..core::cmp::min(n, full.len())].to_vec();
        let label = format!("row123 truncated to {n} bytes");
        let r = dual(&label, move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            let r = api::png_image_begin_read_from_memory(
                l,
                &mut img,
                b.as_ptr() as *const c_void,
                b.len(),
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r, k)
        });
        let out = r.out.as_ref().expect("no unwind expected");
        assert_eq!(out.0, 0, "{label}: begin_read should have failed");
        // row 123 (or the signature check for the shortest prefixes)
        assert!(
            out.1 .7.contains("read beyond end of data")
                || out.1 .7.contains("not a PNG file"),
            "{label}: unexpected message {:?}",
            out.1 .7
        );
    }
    // the truncation points that reach png_image_memory_read
    for n in [9usize, 16, 20, 24, 30] {
        let b = full[..core::cmp::min(n, full.len())].to_vec();
        let label = format!("row123 memory read beyond end at {n} bytes");
        let r = dual(&label, move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            let r = api::png_image_begin_read_from_memory(
                l,
                &mut img,
                b.as_ptr() as *const c_void,
                b.len(),
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r, k)
        });
        let out = r.out.as_ref().expect("no unwind expected");
        assert!(
            out.1 .7.contains("read beyond end of data"),
            "{label}: expected \"read beyond end of data\", got {:?}",
            out.1 .7
        );
    }
    // truncation inside the image data (the trailing IEND chunk is 12 bytes,
    // so the cut has to be larger than that): the error surfaces from
    // png_image_finish_read.
    for cut in [13usize, 20, 30, 37] {
        let b = full[..full.len() - cut].to_vec();
        let label = format!("row123 body truncated by {cut} bytes");
        let r = img_case(&label, b, |_| {});
        want_img(&r, &label, "read beyond end of data"); // row 123
    }
}

#[test]
fn image_colormap_needs_background_row129() {
    // row 129: the input has alpha/transparency, the requested colour-mapped
    // output has no alpha channel and is not linear, and no background colour
    // was supplied.
    const M129: &str = "background color must be supplied to remove alpha/transparency";
    let cases: Vec<(&str, Vec<u8>, png_uint_32)> = vec![
        ("gray+alpha", simple(1, 1, 8, 4), PNG_FORMAT_GRAY_COLORMAP),
        ("rgb+alpha", simple(1, 1, 8, 6), PNG_FORMAT_RGB_COLORMAP),
        (
            "gray+alpha to rgb colour-map",
            simple(1, 1, 8, 4),
            PNG_FORMAT_RGB_COLORMAP,
        ),
        (
            "palette with tRNS",
            {
                let plte: Vec<u8> = (0..4u8).flat_map(|i| [i * 7, i * 11, i * 13]).collect();
                build(
                    1,
                    1,
                    8,
                    3,
                    0,
                    Some(&plte),
                    Some(&[0u8, 128, 255, 255]),
                    &[0u8, 1],
                )
            },
            PNG_FORMAT_RGB_COLORMAP,
        ),
        (
            "gray with tRNS",
            build(1, 1, 8, 0, 0, None, Some(&[0u8, 0x80]), &[0u8, 0x80]),
            PNG_FORMAT_GRAY_COLORMAP,
        ),
    ];
    for (name, bytes, fmt) in cases {
        let label = format!("row129 {name}");
        let r = img_case_bg(
            &label,
            bytes,
            move |img| {
                img.format = fmt;
            },
            false, // background == NULL
        );
        want_img(&r, &label, M129); // row 129
    }
    // supplying a background makes exactly the same call succeed
    let r = img_case(
        "row129 with a background",
        simple(1, 1, 8, 4),
        |img| {
            img.format = PNG_FORMAT_GRAY_COLORMAP;
        },
    );
    let out = r.out.as_ref().expect("no unwind");
    assert_eq!(
        (out.0, out.1),
        (1, 1),
        "row129 control case should succeed, message = {:?}",
        out.2 .7
    );
    // ... and a linear output composes on black instead of erroring
    let r = img_case_bg(
        "row129 linear output needs no background",
        simple(1, 1, 8, 4),
        |img| {
            img.format = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLORMAP;
        },
        false,
    );
    let out = r.out.as_ref().expect("no unwind");
    assert!(
        !out.2 .7.contains(M129),
        "row129: a linear colour-mapped output must not require a background, got {:?}",
        out.2 .7
    );
}

#[test]
fn image_colormap_too_few_entries_rows130_140() {
    // Every "<...> color-map: too few entries" site is selected by the PNG
    // colour type plus the output `format` the application asks for; setting
    // image->colormap_entries to 1 makes each of them fire.
    struct C(&'static str, Vec<u8>, png_uint_32, &'static str);
    let cases = vec![
        // row 130
        C(
            "row130 gray[8]",
            simple(1, 1, 8, 0),
            PNG_FORMAT_GRAY_COLORMAP,
            "gray[8] color-map: too few entries",
        ),
        // row 131
        C(
            "row131 gray[16]",
            simple(1, 1, 16, 0),
            PNG_FORMAT_GRAY_COLORMAP,
            "gray[16] color-map: too few entries",
        ),
        // row 132
        C(
            "row132 gray+alpha",
            simple(1, 1, 8, 4),
            PNG_FORMAT_GA | PNG_FORMAT_FLAG_COLORMAP,
            "gray+alpha color-map: too few entries",
        ),
        // row 133 (output has no COLOR flag)
        C(
            "row133 gray-alpha",
            simple(1, 1, 8, 4),
            PNG_FORMAT_GRAY_COLORMAP,
            "gray-alpha color-map: too few entries",
        ),
        // row 134 (output is colour, background is not gray)
        C(
            "row134 ga-alpha",
            simple(1, 1, 8, 4),
            PNG_FORMAT_RGB_COLORMAP,
            "ga-alpha color-map: too few entries",
        ),
        // row 135
        C(
            "row135 rgb[ga]",
            simple(1, 1, 8, 6),
            PNG_FORMAT_GA | PNG_FORMAT_FLAG_COLORMAP,
            "rgb[ga] color-map: too few entries",
        ),
        // row 136
        C(
            "row136 rgb[gray]",
            simple(1, 1, 8, 2),
            PNG_FORMAT_GRAY_COLORMAP,
            "rgb[gray] color-map: too few entries",
        ),
        // row 137
        C(
            "row137 rgb+alpha",
            simple(1, 1, 8, 6),
            PNG_FORMAT_RGBA_COLORMAP,
            "rgb+alpha color-map: too few entries",
        ),
        // row 138
        C(
            "row138 rgb-alpha",
            simple(1, 1, 8, 6),
            PNG_FORMAT_RGB_COLORMAP,
            "rgb-alpha color-map: too few entries",
        ),
        // row 139
        C(
            "row139 rgb",
            simple(1, 1, 8, 2),
            PNG_FORMAT_RGB_COLORMAP,
            "rgb color-map: too few entries",
        ),
        // row 140
        C(
            "row140 palette",
            palette_png(1, 1, 4, 0),
            PNG_FORMAT_RGB_COLORMAP,
            "palette color-map: too few entries",
        ),
    ];
    for C(label, bytes, fmt, expect) in cases {
        let r = img_case(label, bytes, move |img| {
            img.format = fmt;
            img.colormap_entries = 1;
        });
        want_img(&r, label, expect);
    }
}

/// Exhaustive differential sweep of the colour-map decision tree in
/// `png_image_read_colormap`: every source colour type crossed with every
/// colour-mapped output format and the interesting `colormap_entries` values,
/// with and without a background colour.  This is the only public route to the
/// remaining internal checks in that function (rows 141..145).
#[test]
fn image_colormap_decision_tree_sweep() {
    let sources: Vec<(&str, Vec<u8>)> = vec![
        ("gray1", simple(2, 2, 1, 0)),
        ("gray2", simple(2, 2, 2, 0)),
        ("gray4", simple(2, 2, 4, 0)),
        ("gray8", simple(2, 2, 8, 0)),
        ("gray16", simple(2, 2, 16, 0)),
        (
            "gray8+tRNS",
            build(2, 2, 8, 0, 0, None, Some(&[0u8, 0x80]), &raw_rows(2, 2, 8, 0)),
        ),
        (
            "gray16+tRNS",
            build(
                2,
                2,
                16,
                0,
                0,
                None,
                Some(&[0u8, 0x80]),
                &raw_rows(2, 2, 16, 0),
            ),
        ),
        ("ga8", simple(2, 2, 8, 4)),
        ("ga16", simple(2, 2, 16, 4)),
        ("rgb8", simple(2, 2, 8, 2)),
        ("rgb16", simple(2, 2, 16, 2)),
        (
            "rgb8+tRNS",
            build(
                2,
                2,
                8,
                2,
                0,
                None,
                Some(&[0u8, 0x11, 0u8, 0x22, 0u8, 0x33]),
                &raw_rows(2, 2, 8, 2),
            ),
        ),
        ("rgba8", simple(2, 2, 8, 6)),
        ("rgba16", simple(2, 2, 16, 6)),
        ("palette8", palette_png(2, 2, 4, 1)),
        ("palette8-256", palette_png(2, 2, 256, 1)),
        ("palette2", {
            let plte: Vec<u8> = (0..4u8).flat_map(|i| [i * 7, i * 11, i * 13]).collect();
            build(4, 2, 2, 3, 0, Some(&plte), None, &raw_rows(4, 2, 2, 3))
        }),
        ("palette8+tRNS", {
            let plte: Vec<u8> = (0..4u8).flat_map(|i| [i * 7, i * 11, i * 13]).collect();
            build(
                2,
                2,
                8,
                3,
                0,
                Some(&plte),
                Some(&[0u8, 128, 255, 255]),
                &raw_rows(2, 2, 8, 3),
            )
        }),
    ];
    for (src, src_bytes) in sources {
        for fmt in 0u32..0x80 {
            if fmt & PNG_FORMAT_FLAG_COLORMAP == 0 {
                continue;
            }
            for entries in [1u32, 2, 216, 217, 231, 232, 244, 254, 255, 256] {
                for use_bg in [false, true] {
                    let label =
                        format!("cmap sweep {src} fmt 0x{fmt:02x} entries {entries} bg {use_bg}");
                    let r = img_case_bg(
                        &label,
                        src_bytes.clone(),
                        move |img| {
                            img.format = fmt;
                            img.colormap_entries = entries;
                        },
                        use_bg,
                    );
                    if std::env::var_os("T_ERR3_TRACE").is_some() {
                        if let Some(o) = r.out.as_ref() {
                            if !o.2 .7.is_empty() {
                                eprintln!("[cmap] {label}: {}", o.2 .7);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn image_read_background_unexpected_compose_row151() {
    // row 151: png_image_read_background's "unexpected compose".
    //
    // Reached with an RGBA source and an output format that asks for
    // ASSOCIATED_ALPHA but neither COLOR nor ALPHA: png_image_read_direct then
    //   * turns on rgb-to-gray (colour -> gray) and notes do_local_background,
    //   * upgrades the alpha mode to PNG_ALPHA_OPTIMIZED (which composes),
    //   * promotes do_local_background to 2 when removing the alpha channel,
    // so png_image_read_background runs with PNG_COMPOSE still set.
    for (name, bytes) in [
        ("rgba8", simple(2, 2, 8, 6)),
        ("rgba16", simple(2, 2, 16, 6)),
    ] {
        for fmt in [0x40u32, 0x50, 0x60, 0x70] {
            let label = format!("row151 {name} format 0x{fmt:02x}");
            let r = img_case(&label, bytes.clone(), move |img| {
                img.format = fmt;
            });
            want_img(&r, &label, "unexpected compose"); // row 151
        }
    }
}

#[test]
fn image_read_direct_unsupported_transformation_row156() {
    // row 156: a format bit that no transform can implement (0x80 is not a
    // defined PNG_FORMAT_FLAG_*).
    let r = img_case(
        "row156 unsupported transformation",
        simple(2, 2, 8, 0),
        |img| {
            img.format |= 0x80;
        },
    );
    // row 156
    want_img(
        &r,
        "row156",
        "png_read_image: unsupported transformation",
    );
    let r = img_case(
        "row156 unsupported transformation (colour)",
        simple(2, 2, 8, 2),
        |img| {
            img.format |= 0x100;
        },
    );
    // row 156
    want_img(
        &r,
        "row156 (b)",
        "png_read_image: unsupported transformation",
    );
}

#[test]
fn image_finish_read_argument_errors() {
    let bytes = simple(2, 2, 8, 2);

    // NULL image
    dual("png_image_finish_read NULL image", |l| unsafe {
        api::png_image_finish_read(
            l,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        )
    });
    dual("png_image_begin_read_from_memory NULL image", |l| unsafe {
        let m = [0u8; 8];
        api::png_image_begin_read_from_memory(
            l,
            std::ptr::null_mut(),
            m.as_ptr() as *const c_void,
            8,
        )
    });

    // bad version on begin_read
    dual("begin_read bad version", |l| unsafe {
        let mut img = png_image {
            version: 99,
            ..Default::default()
        };
        let m = [0u8; 8];
        let r = api::png_image_begin_read_from_memory(
            l,
            &mut img,
            m.as_ptr() as *const c_void,
            8,
        );
        (r, img.cmp_key())
    });
    // NULL memory / zero size
    dual("begin_read NULL memory", |l| unsafe {
        let mut img = png_image {
            version: PNG_IMAGE_VERSION,
            ..Default::default()
        };
        let r =
            api::png_image_begin_read_from_memory(l, &mut img, std::ptr::null(), 8);
        (r, img.cmp_key())
    });
    dual("begin_read zero size", |l| unsafe {
        let mut img = png_image {
            version: PNG_IMAGE_VERSION,
            ..Default::default()
        };
        let m = [0u8; 8];
        let r = api::png_image_begin_read_from_memory(
            l,
            &mut img,
            m.as_ptr() as *const c_void,
            0,
        );
        (r, img.cmp_key())
    });

    // finish_read: damaged version
    {
        let bytes = bytes.clone();
        dual("finish_read damaged version", move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            api::png_image_begin_read_from_memory(
                l,
                &mut img,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
            );
            img.version = 77;
            let mut buf = vec![0u8; 4096];
            let r = api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0,
                std::ptr::null_mut(),
            );
            (r, img.cmp_key())
        });
    }
    // finish_read: NULL buffer
    {
        let bytes = bytes.clone();
        dual("finish_read NULL buffer", move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            api::png_image_begin_read_from_memory(
                l,
                &mut img,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
            );
            let r = api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r, k)
        });
    }
    // finish_read: colour-mapped format but no colormap
    {
        let bytes = palette_png(2, 2, 4, 1);
        dual("finish_read colour-map without colormap", move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            api::png_image_begin_read_from_memory(
                l,
                &mut img,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
            );
            let mut buf = vec![0u8; 4096];
            let r = api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0,
                std::ptr::null_mut(),
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r, k)
        });
    }
    // finish_read: colour-mapped format with colormap_entries == 0
    {
        let bytes = palette_png(2, 2, 4, 1);
        dual("finish_read colour-map with 0 entries", move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            api::png_image_begin_read_from_memory(
                l,
                &mut img,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
            );
            img.colormap_entries = 0;
            let mut buf = vec![0u8; 4096];
            let mut cmap = vec![0u8; 4096];
            let r = api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0,
                cmap.as_mut_ptr() as *mut c_void,
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r, k)
        });
    }
    // finish_read: row_stride too small (in both signs)
    for stride in [1i32, 2, 5, -1, -5] {
        let bytes = bytes.clone();
        let label = format!("finish_read row_stride {stride}");
        dual(&label, move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            api::png_image_begin_read_from_memory(
                l,
                &mut img,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
            );
            let mut buf = vec![0u8; 4096];
            let r = api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                stride,
                std::ptr::null_mut(),
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r, k)
        });
    }
    // finish_read: image too large / row_stride too large
    {
        let bytes = bytes.clone();
        dual("finish_read image too large", move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            api::png_image_begin_read_from_memory(
                l,
                &mut img,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
            );
            img.height = 0x4000_0000;
            let mut buf = vec![0u8; 4096];
            let r = api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0x4000_0000,
                std::ptr::null_mut(),
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r, k)
        });
    }
    {
        let bytes = bytes.clone();
        dual("finish_read row_stride too large", move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            api::png_image_begin_read_from_memory(
                l,
                &mut img,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
            );
            img.width = 0x4000_0000;
            let mut buf = vec![0u8; 4096];
            let r = api::png_image_finish_read(
                l,
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0,
                std::ptr::null_mut(),
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r, k)
        });
    }
    // finish_read with every "format" bit pattern in 0..0x80 for every source
    // colour type / bit depth.  This is the only public route to the internal
    // consistency checks in png_image_read_direct (rows 156..159).
    let sources: Vec<(&str, Vec<u8>)> = vec![
        ("gray8", simple(2, 2, 8, 0)),
        ("gray16", simple(2, 2, 16, 0)),
        ("ga8", simple(2, 2, 8, 4)),
        ("ga16", simple(2, 2, 16, 4)),
        ("rgb8", simple(2, 2, 8, 2)),
        ("rgb16", simple(2, 2, 16, 2)),
        ("rgba8", simple(2, 2, 8, 6)),
        ("rgba16", simple(2, 2, 16, 6)),
        ("palette8", palette_png(2, 2, 4, 1)),
    ];
    for (src, src_bytes) in sources {
        for fmt in 0u32..0x80 {
            let bytes = src_bytes.clone();
            let label = format!("finish_read {src} format 0x{fmt:02x}");
            let r = dual(&label, move |l| unsafe {
                let mut img = png_image {
                    version: PNG_IMAGE_VERSION,
                    ..Default::default()
                };
                let r1 = api::png_image_begin_read_from_memory(
                    l,
                    &mut img,
                    bytes.as_ptr() as *const c_void,
                    bytes.len(),
                );
                img.format = fmt;
                let mut buf = vec![0u8; 1 << 16];
                let mut cmap = vec![0u8; 1 << 13];
                let bg = png_color {
                    red: 9,
                    green: 40,
                    blue: 200,
                };
                let r2 = api::png_image_finish_read(
                    l,
                    &mut img,
                    &bg,
                    buf.as_mut_ptr() as *mut c_void,
                    0,
                    cmap.as_mut_ptr() as *mut c_void,
                );
                let k = img.cmp_key();
                api::png_image_free(l, &mut img);
                (r1, r2, k)
            });
            if std::env::var_os("T_ERR3_TRACE").is_some() {
                if let Some(o) = r.out.as_ref() {
                    if !o.2 .7.is_empty() {
                        eprintln!("[msg] {label}: {}", o.2 .7);
                    }
                }
            }
        }
    }
    // finish_read with every "format" bit pattern in 0..0x40 on an RGB source
    for fmt in 0u32..0x40 {
        let bytes = bytes.clone();
        let label = format!("finish_read format 0x{fmt:02x}");
        dual(&label, move |l| unsafe {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                ..Default::default()
            };
            let r1 = api::png_image_begin_read_from_memory(
                l,
                &mut img,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
            );
            img.format = fmt;
            let mut buf = vec![0u8; 1 << 16];
            let mut cmap = vec![0u8; 1 << 13];
            let bg = png_color {
                red: 9,
                green: 9,
                blue: 9,
            };
            let r2 = api::png_image_finish_read(
                l,
                &mut img,
                &bg,
                buf.as_mut_ptr() as *mut c_void,
                0,
                cmap.as_mut_ptr() as *mut c_void,
            );
            let k = img.cmp_key();
            api::png_image_free(l, &mut img);
            (r1, r2, k)
        });
    }
}

#[test]
fn image_begin_read_from_file_bad_arguments() {
    dual("begin_read_from_file NULL name", |l| unsafe {
        let mut img = png_image {
            version: PNG_IMAGE_VERSION,
            ..Default::default()
        };
        let r =
            api::png_image_begin_read_from_file(l, &mut img, std::ptr::null());
        (r, img.cmp_key())
    });
    dual("begin_read_from_file bad version", |l| unsafe {
        let mut img = png_image {
            version: 3,
            ..Default::default()
        };
        let r =
            api::png_image_begin_read_from_file(l, &mut img, std::ptr::null());
        (r, img.cmp_key())
    });
    dual("begin_read_from_file NULL image", |l| unsafe {
        api::png_image_begin_read_from_file(l, std::ptr::null_mut(), std::ptr::null())
    });
}

// ===========================================================================
// truncated and corrupt read streams
// ===========================================================================

/// Read a stream all the way through and report what happened.
fn read_all(label: &str, bytes: Vec<u8>) {
    dual(label, move |l| unsafe {
        let (pp, ip) = reader(l, &bytes);
        api::png_read_info(l, pp, ip);
        let h = api::png_get_image_height(l, pp, ip);
        let mut buf = vec![0u8; 1 << 12];
        for _ in 0..core::cmp::min(h, 16) {
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
        }
        api::png_read_end(l, pp, ip);
        (
            api::png_get_image_width(l, pp, ip),
            h,
            api::png_get_bit_depth(l, pp, ip),
            api::png_get_color_type(l, pp, ip),
            src_pos(),
        )
    });
}

#[test]
fn truncated_streams() {
    read_all("empty stream", Vec::new());
    read_all("one byte", vec![0x89]);
    read_all("signature only", SIG.to_vec());
    read_all("bad signature", vec![0u8; 8]);
    // signature + a truncated IHDR chunk
    let full = ihdr(4, 4, 8, 2, 0);
    for n in 1..=full.len() {
        let mut b = SIG.to_vec();
        b.extend_from_slice(&full[..n]);
        read_all(&format!("truncated IHDR ({n} of {})", full.len()), b);
    }
}

#[test]
fn invalid_ihdr_fields() {
    // Each IHDR field individually out of range.
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("width 0", ihdr_raw(0, 4, 8, 0, 0, 0, 0)),
        ("height 0", ihdr_raw(4, 0, 8, 0, 0, 0, 0)),
        ("width 2^31", ihdr_raw(0x8000_0000, 4, 8, 0, 0, 0, 0)),
        ("height 2^31", ihdr_raw(4, 0x8000_0000, 8, 0, 0, 0, 0)),
        ("width over user limit", ihdr_raw(2_000_000, 4, 8, 0, 0, 0, 0)),
        ("height over user limit", ihdr_raw(4, 2_000_000, 8, 0, 0, 0, 0)),
        ("bit depth 0", ihdr_raw(4, 4, 0, 0, 0, 0, 0)),
        ("bit depth 3", ihdr_raw(4, 4, 3, 0, 0, 0, 0)),
        ("bit depth 32", ihdr_raw(4, 4, 32, 0, 0, 0, 0)),
        ("colour type 1", ihdr_raw(4, 4, 8, 1, 0, 0, 0)),
        ("colour type 5", ihdr_raw(4, 4, 8, 5, 0, 0, 0)),
        ("colour type 7", ihdr_raw(4, 4, 8, 7, 0, 0, 0)),
        ("palette bit depth 16", ihdr_raw(4, 4, 16, 3, 0, 0, 0)),
        ("rgb bit depth 4", ihdr_raw(4, 4, 4, 2, 0, 0, 0)),
        ("interlace 2", ihdr_raw(4, 4, 8, 0, 2, 0, 0)),
        ("interlace 255", ihdr_raw(4, 4, 8, 0, 255, 0, 0)),
        ("compression 1", ihdr_raw(4, 4, 8, 0, 0, 1, 0)),
        ("filter 1", ihdr_raw(4, 4, 8, 0, 0, 0, 1)),
    ];
    for (name, ih) in cases {
        let mut b = SIG.to_vec();
        b.extend(ih);
        b.extend(chunk(b"IDAT", &zlib_stored(&[0u8, 0, 0, 0, 0])));
        b.extend(chunk(b"IEND", &[]));
        read_all(&format!("invalid IHDR: {name}"), b);
    }
}

#[test]
fn missing_and_corrupt_chunks() {
    // no IDAT at all
    {
        let mut b = SIG.to_vec();
        b.extend(ihdr(2, 2, 8, 0, 0));
        b.extend(chunk(b"IEND", &[]));
        read_all("missing IDAT", b);
    }
    // no IEND
    {
        let mut b = SIG.to_vec();
        b.extend(ihdr(2, 2, 8, 0, 0));
        b.extend(chunk(b"IDAT", &zlib_stored(&raw_rows(2, 2, 8, 0))));
        read_all("missing IEND", b);
    }
    // IDAT with garbage instead of a zlib stream
    {
        let mut b = SIG.to_vec();
        b.extend(ihdr(2, 2, 8, 0, 0));
        b.extend(chunk(b"IDAT", &[0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff]));
        b.extend(chunk(b"IEND", &[]));
        read_all("corrupt zlib data", b);
    }
    // IDAT with a valid zlib header but a corrupt deflate body
    {
        let mut z = zlib_stored(&raw_rows(2, 2, 8, 0));
        z[2] ^= 0x06; // break the block type
        let mut b = SIG.to_vec();
        b.extend(ihdr(2, 2, 8, 0, 0));
        b.extend(chunk(b"IDAT", &z));
        b.extend(chunk(b"IEND", &[]));
        read_all("corrupt deflate block", b);
    }
    // zlib stream that stops in the middle of a row
    {
        let raw = raw_rows(8, 4, 8, 2);
        let short = &raw[..raw.len() / 2];
        let mut b = SIG.to_vec();
        b.extend(ihdr(8, 4, 8, 2, 0));
        b.extend(chunk(b"IDAT", &zlib_stored(short)));
        b.extend(chunk(b"IEND", &[]));
        read_all("stream ends mid-row", b);
    }
    // bad CRC on IHDR
    {
        let mut ih = ihdr(2, 2, 8, 0, 0);
        let n = ih.len();
        ih[n - 1] ^= 0xff;
        let mut b = SIG.to_vec();
        b.extend(ih);
        b.extend(chunk(b"IDAT", &zlib_stored(&raw_rows(2, 2, 8, 0))));
        b.extend(chunk(b"IEND", &[]));
        read_all("bad IHDR CRC", b);
    }
    // chunk header with the high length bit set / invalid chunk name
    {
        let mut b = SIG.to_vec();
        b.extend(ihdr(2, 2, 8, 0, 0));
        b.extend_from_slice(&[0x80, 0, 0, 0]);
        b.extend_from_slice(b"IDAT");
        read_all("bad header invalid length", b);
    }
    {
        let mut b = SIG.to_vec();
        b.extend(ihdr(2, 2, 8, 0, 0));
        b.extend_from_slice(&[0, 0, 0, 0]);
        b.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);
        b.extend_from_slice(&[0, 0, 0, 0]);
        read_all("bad header invalid type", b);
    }
}

// ===========================================================================
// pngrtran.c: png_set_crc_action
// ===========================================================================

#[test]
fn crc_action_row164() {
    // row 164: PNG_CRC_WARN_DISCARD is not valid for critical chunks
    let vals: [c_int; 10] = [-1, 0, 1, 2, 3, 4, 5, 6, 99, 0x7fff_ffff];
    for crit in vals {
        for ancil in vals {
            let label = format!("row164 png_set_crc_action({crit}, {ancil})");
            let r = dual(&label, move |l| unsafe {
                let pp = api::new_reader(l);
                api::png_set_crc_action(l, pp, crit, ancil);
                0u32
            });
            if crit == PNG_CRC_WARN_DISCARD {
                // row 164
                want(&r, &label, "Can't discard critical data on CRC error");
            } else {
                want_silent(&r, &label);
            }
        }
    }
    // NULL png_ptr
    dual("png_set_crc_action NULL", |l| unsafe {
        api::png_set_crc_action(l, std::ptr::null_mut(), PNG_CRC_WARN_DISCARD, 99);
        0u32
    });
}

// ===========================================================================
// pngrtran.c: png_rtran_ok
// ===========================================================================

/// Every pngrtran.c read transform setter, applied to `pp`.
unsafe fn call_all_rtran_setters(l: &'static Library, pp: png_structp, which: usize) {
    let bg = png_color_16 {
        index: 0,
        red: 1,
        green: 2,
        blue: 3,
        gray: 4,
    };
    let sig = png_color_8 {
        red: 4,
        green: 4,
        blue: 4,
        gray: 4,
        alpha: 4,
    };
    let mut pal = [png_color::default(); 4];
    match which {
        0 => api::png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0),
        1 => api::png_set_scale_16(l, pp),
        2 => api::png_set_strip_16(l, pp),
        3 => api::png_set_strip_alpha(l, pp),
        4 => api::png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_PNG, PNG_FP_1),
        5 => api::png_set_quantize(l, pp, pal.as_mut_ptr(), 4, 4, std::ptr::null(), 0),
        6 => api::png_set_gamma_fixed(l, pp, PNG_FP_1, PNG_FP_1),
        7 => api::png_set_expand(l, pp),
        8 => api::png_set_palette_to_rgb(l, pp),
        9 => api::png_set_expand_gray_1_2_4_to_8(l, pp),
        10 => api::png_set_tRNS_to_alpha(l, pp),
        11 => api::png_set_expand_16(l, pp),
        12 => api::png_set_gray_to_rgb(l, pp),
        13 => api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_NONE, -1, -1),
        14 => api::png_set_shift(l, pp, &sig),
        15 => api::png_set_packing(l, pp),
        16 => api::png_set_packswap(l, pp),
        17 => api::png_set_invert_mono(l, pp),
        18 => api::png_set_bgr(l, pp),
        19 => api::png_set_swap(l, pp),
        20 => api::png_set_swap_alpha(l, pp),
        21 => api::png_set_invert_alpha(l, pp),
        22 => api::png_set_filler(l, pp, 0, PNG_FILLER_AFTER),
        23 => api::png_set_add_alpha(l, pp, 0, PNG_FILLER_AFTER),
        _ => unreachable!(),
    }
}

const N_RTRAN_SETTERS: usize = 24;

#[test]
fn rtran_ok_after_row_init_row165() {
    // row 165: "invalid after png_start_read_image or png_read_update_info"
    let b = simple(4, 2, 8, 6);
    const M165: &str = "invalid after png_start_read_image or png_read_update_info";
    let mut hits = 0usize;
    for which in 0..N_RTRAN_SETTERS {
        let b = b.clone();
        let label = format!("row165 setter #{which} after png_read_update_info");
        let r = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_read_update_info(l, pp, ip);
            call_all_rtran_setters(l, pp, which);
            0u32
        });
        if msgs(&r).iter().any(|m| m.contains(M165)) {
            hits += 1; // row 165
        }
    }
    assert!(hits >= 14, "row165: only {hits} setters reported the error");
    for which in 0..N_RTRAN_SETTERS {
        let b = b.clone();
        let label = format!("row165 setter #{which} after png_start_read_image");
        dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_start_read_image(l, pp);
            call_all_rtran_setters(l, pp, which);
            0u32
        });
    }
    // one explicit, unambiguous instance
    let b2 = simple(4, 2, 8, 6);
    let r = dual("row165 png_set_scale_16 after update_info", move |l| unsafe {
        let (pp, ip) = reader(l, &b2);
        api::png_read_info(l, pp, ip);
        api::png_read_update_info(l, pp, ip);
        api::png_set_scale_16(l, pp);
        0u32
    });
    want(&r, "row165", M165); // row 165
}

#[test]
fn rtran_ok_before_ihdr_row166() {
    // row 166: "invalid before the PNG header has been read"; only
    // png_set_rgb_to_gray_fixed passes need_IHDR = 1.
    for which in 0..N_RTRAN_SETTERS {
        let label = format!("row166 setter #{which} before png_read_info");
        dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            call_all_rtran_setters(l, pp, which);
            0u32
        });
    }
    // and the float wrapper
    let r = dual("row166 png_set_rgb_to_gray before header", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        api::png_set_rgb_to_gray(l, pp, PNG_ERROR_ACTION_NONE, -1.0, -1.0);
        0u32
    });
    // row 166
    want(&r, "row166", "invalid before the PNG header has been read");
    let r = dual("row166 png_set_rgb_to_gray_fixed before header", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_NONE, -1, -1);
        0u32
    });
    // row 166
    want(&r, "row166 (b)", "invalid before the PNG header has been read");
}

#[test]
fn rtran_setters_on_a_write_struct() {
    // The read transform setters called on a write struct: png_rtran_ok only
    // looks at the flags, so this documents whatever libpng does.
    for which in 0..N_RTRAN_SETTERS {
        let label = format!("write-struct setter #{which}");
        dual(&label, move |l| unsafe {
            sink_reset();
            let pp = api::new_writer(l);
            call_all_rtran_setters(l, pp, which);
            0u32
        });
    }
}

#[test]
fn rtran_setters_null_png_ptr() {
    for which in 0..N_RTRAN_SETTERS {
        let label = format!("NULL png_ptr setter #{which}");
        dual(&label, move |l| unsafe {
            call_all_rtran_setters(l, std::ptr::null_mut(), which);
            0u32
        });
    }
    // the floating point wrappers too
    dual("NULL png_ptr float wrappers", |l| unsafe {
        let n: png_structp = std::ptr::null_mut();
        let bg = png_color_16::default();
        api::png_set_background(l, n, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 1.0);
        api::png_set_alpha_mode(l, n, PNG_ALPHA_PNG, 1.0);
        api::png_set_gamma(l, n, 1.0, 1.0);
        api::png_set_rgb_to_gray(l, n, PNG_ERROR_ACTION_NONE, 0.2, 0.7);
        // NOTE: png_set_read_user_transform_fn has no NULL guard in the
        // reference C (pngrtran.c:1133 dereferences png_ptr unconditionally),
        // so passing NULL there is undefined behaviour, not a testable
        // boundary.
        0u32
    });
}

// ===========================================================================
// pngrtran.c: png_set_background_fixed
// ===========================================================================

#[test]
fn background_unknown_gamma_row167() {
    // row 167: PNG_BACKGROUND_GAMMA_UNKNOWN
    let r = dual("row167 unknown background gamma", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        let bg = png_color_16 {
            index: 0,
            red: 1,
            green: 2,
            blue: 3,
            gray: 4,
        };
        api::png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_UNKNOWN, 0, 0);
        0u32
    });
    // row 167
    want(&r, "row167", "Application must supply a known background gamma");
    // NULL background: silent return
    let r = dual("png_set_background_fixed NULL colour", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        api::png_set_background_fixed(
            l,
            pp,
            std::ptr::null(),
            PNG_BACKGROUND_GAMMA_SCREEN,
            0,
            0,
        );
        0u32
    });
    want_silent(&r, "png_set_background_fixed NULL colour");
    // every background_gamma_code, accepted or not
    for code in [-1i32, 0, 1, 2, 3, 4, 5, 99, 0x7fff_ffff] {
        let label = format!("row167 bg_gamma_code {code}");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            let bg = png_color_16 {
                index: 0,
                red: 1,
                green: 2,
                blue: 3,
                gray: 4,
            };
            api::png_set_background_fixed(l, pp, &bg, code, 0, PNG_FP_1);
            0u32
        });
        if code == PNG_BACKGROUND_GAMMA_UNKNOWN {
            // row 167
            want(&r, &label, "Application must supply a known background gamma");
        } else {
            want_silent(&r, &label);
        }
    }
}

// ===========================================================================
// pngrtran.c: convert_gamma_value / unsupported_gamma
// ===========================================================================

#[test]
fn convert_gamma_value_overflow_row168() {
    // row 168: png_fixed_error("gamma value")
    const M168: &str = "fixed point overflow in gamma value";
    for g in [1e10f64, 1e30, -1e10, -1e30, 2.2e9] {
        {
            let label = format!("row168 png_set_alpha_mode({g})");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                api::png_set_alpha_mode(l, pp, PNG_ALPHA_PNG, g);
                0u32
            });
            want(&r, &label, M168); // row 168
        }
        {
            let label = format!("row168 png_set_gamma(scrn={g})");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                api::png_set_gamma(l, pp, g, 1.0);
                0u32
            });
            want(&r, &label, M168); // row 168
        }
        {
            let label = format!("row168 png_set_gamma(file={g})");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                api::png_set_gamma(l, pp, 1.0, g);
                0u32
            });
            want(&r, &label, M168); // row 168
        }
        {
            // png_set_background goes through png_fixed, not
            // convert_gamma_value, so the text differs.
            let label = format!("png_set_background({g}) fixed point overflow");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                let bg = png_color_16::default();
                api::png_set_background(
                    l,
                    pp,
                    &bg,
                    PNG_BACKGROUND_GAMMA_UNIQUE,
                    0,
                    g,
                );
                0u32
            });
            want(&r, &label, "fixed point overflow in ");
        }
    }
    // values that must NOT overflow
    for g in [1.0f64, 0.45455, 2.2, 45455.0, 100000.0, 1e7] {
        let label = format!("gamma {g} does not overflow");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            api::png_set_alpha_mode(l, pp, PNG_ALPHA_PNG, g);
            0u32
        });
        want_silent(&r, &label);
    }
}

#[test]
fn unsupported_gamma_rows169_170() {
    // PNG_LIB_GAMMA_MIN = 1000, PNG_LIB_GAMMA_MAX = 10000000
    const RANGE: &str = "gamma out of supported range";
    let bad: [png_fixed_point; 6] = [1, 999, 10_000_001, 100_000_000, 0x7fff_ffff, 1000 - 1];
    for g in bad {
        // row 169: png_app_warning from png_set_gamma_fixed (warn = 1)
        {
            let label = format!("row169 png_set_gamma_fixed file={g}");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                api::png_set_gamma_fixed(l, pp, PNG_FP_1, g);
                0u32
            });
            want(&r, &label, RANGE); // row 169
        }
        {
            let label = format!("row169 png_set_gamma_fixed scrn={g}");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                api::png_set_gamma_fixed(l, pp, g, PNG_FP_1);
                0u32
            });
            want(&r, &label, RANGE); // row 169
        }
        // row 169 as a benign (warning) error, so that the caller continues
        {
            let label = format!("row169 benign png_set_gamma_fixed file={g}");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                api::png_set_benign_errors(l, pp, 1);
                api::png_set_gamma_fixed(l, pp, PNG_FP_1, g);
                0u32
            });
            want(&r, &label, RANGE); // row 169
            assert_eq!(
                r.log,
                vec![Msg::Warn(RANGE.as_bytes().to_vec())],
                "{label}: expected exactly one warning"
            );
        }
        // row 170: png_app_error from png_set_alpha_mode_fixed (warn = 0)
        {
            let label = format!("row170 png_set_alpha_mode_fixed g={g}");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                api::png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_PNG, g);
                0u32
            });
            want(&r, &label, RANGE); // row 170
            assert_eq!(
                r.log,
                vec![Msg::Err(RANGE.as_bytes().to_vec())],
                "{label}: png_app_error must be an error by default"
            );
        }
        {
            let label = format!("row170 benign png_set_alpha_mode_fixed g={g}");
            let r = dual(&label, move |l| unsafe {
                src_set(&[]);
                let pp = api::new_reader(l);
                api::png_set_benign_errors(l, pp, 1);
                api::png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_PNG, g);
                0u32
            });
            want(&r, &label, RANGE); // row 170
        }
    }
    // 0 and plain negative values are rejected by the <= 0 checks in
    // png_set_gamma_fixed first (rows 173/174) but reach unsupported_gamma in
    // png_set_alpha_mode_fixed.
    for g in [0i32, -5, -3, -7] {
        let label = format!("row170 png_set_alpha_mode_fixed g={g}");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            api::png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_PNG, g);
            0u32
        });
        want(&r, &label, RANGE); // row 170
    }
    // the reserved flag values must NOT be rejected
    for g in [PNG_DEFAULT_sRGB, PNG_GAMMA_MAC_18, -100_000, -50_000] {
        let label = format!("gamma flag value {g} accepted");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            api::png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_PNG, g);
            api::png_set_gamma_fixed(l, pp, g, g);
            0u32
        });
        want_silent(&r, &label);
    }
}

// ===========================================================================
// pngrtran.c: png_set_alpha_mode_fixed
// ===========================================================================

#[test]
fn alpha_mode_invalid_row171() {
    // row 171: "invalid alpha mode"
    for mode in [-1i32, 4, 5, 99, 0x7fff_ffff, i32::MIN] {
        let label = format!("row171 alpha mode {mode}");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            api::png_set_alpha_mode_fixed(l, pp, mode, PNG_FP_1);
            0u32
        });
        want(&r, &label, "invalid alpha mode"); // row 171
        let label = format!("row171 alpha mode {mode} (float)");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            api::png_set_alpha_mode(l, pp, mode, 1.0);
            0u32
        });
        want(&r, &label, "invalid alpha mode"); // row 171
    }
    // valid modes stay quiet
    for mode in [PNG_ALPHA_PNG, PNG_ALPHA_STANDARD, PNG_ALPHA_OPTIMIZED, PNG_ALPHA_BROKEN] {
        let label = format!("valid alpha mode {mode}");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            api::png_set_alpha_mode_fixed(l, pp, mode, PNG_FP_1);
            0u32
        });
        want_silent(&r, &label);
    }
}

#[test]
fn alpha_mode_conflicts_with_background_row172() {
    // row 172: "conflicting calls to set alpha mode and background"
    for mode in [PNG_ALPHA_STANDARD, PNG_ALPHA_OPTIMIZED, PNG_ALPHA_BROKEN] {
        let label = format!("row172 background then alpha mode {mode}");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            let bg = png_color_16 {
                index: 0,
                red: 1,
                green: 2,
                blue: 3,
                gray: 4,
            };
            api::png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);
            api::png_set_alpha_mode_fixed(l, pp, mode, PNG_FP_1);
            0u32
        });
        // row 172
        want(
            &r,
            &label,
            "conflicting calls to set alpha mode and background",
        );
    }
    // PNG_ALPHA_PNG does not compose, so it must be accepted
    let r = dual("row172 background then PNG_ALPHA_PNG", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        let bg = png_color_16::default();
        api::png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);
        api::png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_PNG, PNG_FP_1);
        0u32
    });
    want_silent(&r, "row172 background then PNG_ALPHA_PNG");
    // two composing alpha modes in a row
    let r = dual("row172 two composing alpha modes", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        api::png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_STANDARD, PNG_FP_1);
        api::png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_STANDARD, PNG_FP_1);
        0u32
    });
    // row 172
    want(
        &r,
        "row172 two composing alpha modes",
        "conflicting calls to set alpha mode and background",
    );
}

// ===========================================================================
// pngrtran.c: png_set_gamma_fixed
// ===========================================================================

#[test]
fn set_gamma_non_positive_rows173_174() {
    const M173: &str = "invalid file gamma in png_set_gamma";
    const M174: &str = "invalid screen gamma in png_set_gamma";
    // row 173
    for g in [0i32, -3, -5, -7, -99999, i32::MIN + 1] {
        let label = format!("row173 file gamma {g}");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            api::png_set_gamma_fixed(l, pp, PNG_FP_1, g);
            0u32
        });
        want(&r, &label, M173); // row 173
    }
    // row 174
    for g in [0i32, -3, -5, -7, -99999, i32::MIN + 1] {
        let label = format!("row174 screen gamma {g}");
        let r = dual(&label, move |l| unsafe {
            src_set(&[]);
            let pp = api::new_reader(l);
            api::png_set_gamma_fixed(l, pp, g, PNG_FP_1);
            0u32
        });
        want(&r, &label, M174); // row 174
    }
    // both bad, benign: rows 173 and 174 must both be reported, in order
    let r = dual("rows173+174 benign both bad", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        api::png_set_benign_errors(l, pp, 1);
        api::png_set_gamma_fixed(l, pp, 0, 0);
        0u32
    });
    assert_eq!(
        msgs(&r),
        vec![
            format!("WARN({M173})"),
            format!("WARN({M174})"),
            "WARN(gamma out of supported range)".to_string(),
        ],
        "rows173+174: message order"
    );
    let r = dual("rows173+174 benign both negative", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        api::png_set_benign_errors(l, pp, 1);
        api::png_set_gamma_fixed(l, pp, -3, -7);
        0u32
    });
    want(&r, "rows173+174 negative", M173); // row 173
    want(&r, "rows173+174 negative", M174); // row 174
    // valid gammas stay quiet
    let r = dual("valid gammas quiet", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        api::png_set_gamma_fixed(l, pp, 220_000, 45_455);
        0u32
    });
    want_silent(&r, "valid gammas quiet");
}

// ===========================================================================
// pngrtran.c: png_set_rgb_to_gray_fixed
// ===========================================================================

#[test]
fn rgb_to_gray_error_action_row175() {
    // row 175: "invalid error action to rgb_to_gray"
    let b = simple(2, 2, 8, 2);
    for action in [0i32, -1, 4, 5, 99, 0x7fff_ffff] {
        let b = b.clone();
        let label = format!("row175 error_action {action}");
        let r = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, action, -1, -1);
            0u32
        });
        want(&r, &label, "invalid error action to rgb_to_gray"); // row 175
    }
    // and via the floating point wrapper
    for action in [0i32, 4, 99] {
        let b = b.clone();
        let label = format!("row175 error_action {action} (float)");
        let r = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray(l, pp, action, -1.0, -1.0);
            0u32
        });
        want(&r, &label, "invalid error action to rgb_to_gray"); // row 175
    }
    // the three valid actions are accepted
    for action in [
        PNG_ERROR_ACTION_NONE,
        PNG_ERROR_ACTION_WARN,
        PNG_ERROR_ACTION_ERROR,
    ] {
        let b = b.clone();
        let label = format!("valid error_action {action}");
        let r = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, action, -1, -1);
            0u32
        });
        want_silent(&r, &label);
    }
}

#[test]
fn rgb_to_gray_coefficients_row177() {
    // row 177: "ignoring out of range rgb_to_gray coefficients"
    let b = simple(2, 2, 8, 2);
    // NOTE: `red + green` is computed in `int` in the C, so values whose sum
    // overflows INT_MAX (e.g. PNG_FP_MAX) are signed-overflow UB, not a
    // testable boundary.
    let cases: [(png_fixed_point, png_fixed_point); 7] = [
        (60000, 60000),
        (100000, 1),
        (50000, 50001),
        (0, 100001),
        (100001, 0),
        (2_000_000, 3_000_000),
        (1_000_000_000, 1_000_000_000),
    ];
    for (r, g) in cases {
        let b = b.clone();
        let label = format!("row177 coefficients ({r}, {g})");
        let run = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_NONE, r, g);
            0u32
        });
        // row 177
        want(&run, &label, "ignoring out of range rgb_to_gray coefficients");
    }
    // negative coefficients: the documented "use the default" case, silent
    for (r, g) in [(-1i32, -1i32), (-1, 50000), (50000, -1), (i32::MIN, i32::MIN)] {
        let b = b.clone();
        let label = format!("row177 negative coefficients ({r}, {g})");
        let run = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_NONE, r, g);
            api::png_get_rgb_to_gray_status(l, pp)
        });
        want_silent(&run, &label);
    }
    // in-range coefficients are accepted silently
    for (r, g) in [(0i32, 0i32), (50000, 50000), (21260, 71520)] {
        let b = b.clone();
        let label = format!("row177 in-range coefficients ({r}, {g})");
        let run = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_NONE, r, g);
            0u32
        });
        want_silent(&run, &label);
    }
}

const PNG_FP_MAX_LOCAL: png_fixed_point = 0x7fff_ffff;

// ===========================================================================
// pngrtran.c: png_init_read_transformations
// ===========================================================================

#[test]
fn gamma_background_rgb_to_gray_row178() {
    // row 178: "libpng does not support gamma+background+rgb_to_gray".
    // NOTE: png_init_rgb_transformations cancels PNG_COMPOSE outright when the
    // input has neither an alpha channel nor a tRNS chunk, so the source must
    // have alpha.
    let b = simple(4, 2, 8, 6);
    let r = dual("row178 gamma+background+rgb_to_gray", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        api::png_set_gamma_fixed(l, pp, PNG_GAMMA_sRGB_LOCAL, PNG_GAMMA_sRGB_INV_LOCAL);
        let bg = png_color_16 {
            index: 0,
            red: 10,
            green: 20,
            blue: 30,
            gray: 20,
        };
        api::png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);
        api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_NONE, -1, -1);
        api::png_read_update_info(l, pp, ip);
        (
            api::png_get_rowbytes(l, pp, ip),
            api::png_get_color_type(l, pp, ip),
            api::png_get_bit_depth(l, pp, ip),
        )
    });
    // row 178
    want(
        &r,
        "row178",
        "libpng does not support gamma+background+rgb_to_gray",
    );
    // the same with a palette source (which needs a non-opaque tRNS entry to
    // keep PNG_COMPOSE alive); this takes the palette branch below the warning
    let b = {
        let mut plte = Vec::new();
        for i in 0..4u8 {
            plte.push(i * 7);
            plte.push(i * 11);
            plte.push(i * 13);
        }
        let raw = vec![0u8, 1, 1, 1, 1, 0u8, 2, 2, 2, 2];
        build(4, 2, 8, 3, 0, Some(&plte), Some(&[0u8, 128, 255, 255]), &raw)
    };
    let r = dual("row178 palette variant", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        api::png_set_gamma_fixed(l, pp, PNG_GAMMA_sRGB_LOCAL, PNG_GAMMA_sRGB_INV_LOCAL);
        let bg = png_color_16 {
            index: 0,
            red: 10,
            green: 20,
            blue: 30,
            gray: 20,
        };
        api::png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);
        api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_NONE, -1, -1);
        api::png_read_update_info(l, pp, ip);
        (
            api::png_get_rowbytes(l, pp, ip),
            api::png_get_color_type(l, pp, ip),
            api::png_get_bit_depth(l, pp, ip),
        )
    });
    // row 178
    want(
        &r,
        "row178 palette",
        "libpng does not support gamma+background+rgb_to_gray",
    );
}

const PNG_GAMMA_sRGB_LOCAL: png_fixed_point = 220_000;
const PNG_GAMMA_sRGB_INV_LOCAL: png_fixed_point = 45_455;

#[test]
fn invalid_background_gamma_type_row179() {
    // row 179: "invalid background gamma type" (the source needs alpha so that
    // PNG_COMPOSE survives png_init_rgb_transformations)
    let b = simple(4, 2, 8, 6);
    for code in [4i32, 5, 99, 0x7fff_ffff] {
        let b = b.clone();
        let label = format!("row179 background_gamma_code {code}");
        let r = dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_gamma_fixed(l, pp, PNG_GAMMA_sRGB_LOCAL, PNG_GAMMA_sRGB_INV_LOCAL);
            let bg = png_color_16 {
                index: 0,
                red: 10,
                green: 20,
                blue: 30,
                gray: 20,
            };
            api::png_set_background_fixed(l, pp, &bg, code, 0, PNG_FP_1);
            api::png_read_update_info(l, pp, ip);
            0u32
        });
        want(&r, &label, "invalid background gamma type"); // row 179
    }
    // the valid codes must not error
    for code in [
        PNG_BACKGROUND_GAMMA_SCREEN,
        PNG_BACKGROUND_GAMMA_FILE,
        PNG_BACKGROUND_GAMMA_UNIQUE,
    ] {
        let b = b.clone();
        let label = format!("row179 valid background_gamma_code {code}");
        dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_gamma_fixed(l, pp, PNG_GAMMA_sRGB_LOCAL, PNG_GAMMA_sRGB_INV_LOCAL);
            let bg = png_color_16 {
                index: 0,
                red: 10,
                green: 20,
                blue: 30,
                gray: 20,
            };
            api::png_set_background_fixed(l, pp, &bg, code, 0, PNG_FP_1);
            api::png_read_update_info(l, pp, ip);
            0u32
        });
    }
}

// ===========================================================================
// pngrtran.c: png_read_transform_info
// ===========================================================================

#[test]
fn read_transform_info_null_palette_row180() {
    // row 180: "Palette is NULL in indexed image"
    let r = dual("row180 palette is NULL in indexed image", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        let ip = api::png_create_info_struct(l, pp);
        api::png_set_IHDR(
            l,
            pp,
            ip,
            1,
            1,
            8,
            PNG_COLOR_TYPE_PALETTE,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        api::png_set_expand(l, pp);
        png_read_transform_info(l, pp, ip);
        api::png_get_color_type(l, pp, ip)
    });
    want(&r, "row180", "Palette is NULL in indexed image"); // row 180
    // the same without EXPAND: no error
    let r = dual("row180 without EXPAND", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        let ip = api::png_create_info_struct(l, pp);
        api::png_set_IHDR(
            l,
            pp,
            ip,
            1,
            1,
            8,
            PNG_COLOR_TYPE_PALETTE,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        png_read_transform_info(l, pp, ip);
        api::png_get_color_type(l, pp, ip)
    });
    want_silent(&r, "row180 without EXPAND");
}

// ===========================================================================
// pngrtran.c: png_do_read_transformations
// ===========================================================================

#[test]
fn do_read_transformations_null_row_buffer_row182() {
    // row 182: "NULL row buffer"
    let r = dual("row182 NULL row buffer", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        let mut ri = png_row_info {
            width: 4,
            rowbytes: 4,
            color_type: 0,
            bit_depth: 8,
            channels: 1,
            pixel_depth: 8,
        };
        png_do_read_transformations(l, pp, &mut ri);
        ri
    });
    want(&r, "row182", "NULL row buffer"); // row 182
    // also with transforms requested (DETECT_UNINITIALIZED set)
    let r = dual("row182 NULL row buffer after png_set_expand", |l| unsafe {
        src_set(&[]);
        let pp = api::new_reader(l);
        api::png_set_expand(l, pp);
        let mut ri = png_row_info {
            width: 4,
            rowbytes: 4,
            color_type: 0,
            bit_depth: 8,
            channels: 1,
            pixel_depth: 8,
        };
        png_do_read_transformations(l, pp, &mut ri);
        ri
    });
    want(&r, "row182 (b)", "NULL row buffer"); // row 182
}

#[test]
fn rgb_to_gray_nongray_pixel_rows184_185() {
    // A 2x1 RGB image whose pixels are not gray.
    let raw = vec![0u8, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60];
    let b = build(2, 1, 8, 2, 0, None, None, &raw);

    // row 184: PNG_ERROR_ACTION_WARN
    {
        let b = b.clone();
        let r = dual("row184 rgb_to_gray warn", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_WARN, -1, -1);
            api::png_read_update_info(l, pp, ip);
            let mut buf = vec![0u8; 64];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            (api::png_get_rgb_to_gray_status(l, pp), buf[..8].to_vec())
        });
        // row 184
        want(&r, "row184", "png_do_rgb_to_gray found nongray pixel");
        assert!(
            r.out.is_some(),
            "row184: PNG_ERROR_ACTION_WARN must only warn"
        );
    }
    // row 185: PNG_ERROR_ACTION_ERROR
    {
        let b = b.clone();
        let r = dual("row185 rgb_to_gray error", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_ERROR, -1, -1);
            api::png_read_update_info(l, pp, ip);
            let mut buf = vec![0u8; 64];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            (api::png_get_rgb_to_gray_status(l, pp), buf[..8].to_vec())
        });
        // row 185
        want(&r, "row185", "png_do_rgb_to_gray found nongray pixel");
        assert!(
            r.out.is_none(),
            "row185: PNG_ERROR_ACTION_ERROR must be fatal"
        );
    }
    // PNG_ERROR_ACTION_NONE: silent
    {
        let b = b.clone();
        let r = dual("rgb_to_gray none is silent", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_NONE, -1, -1);
            api::png_read_update_info(l, pp, ip);
            let mut buf = vec![0u8; 64];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            (api::png_get_rgb_to_gray_status(l, pp), buf[..8].to_vec())
        });
        want_silent(&r, "rgb_to_gray none");
    }
    // A genuinely gray RGB image must not trigger the report at all.
    {
        let raw = vec![0u8, 0x33, 0x33, 0x33, 0x44, 0x44, 0x44];
        let b = build(2, 1, 8, 2, 0, None, None, &raw);
        let r = dual("rgb_to_gray warn with gray pixels", move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_WARN, -1, -1);
            api::png_read_update_info(l, pp, ip);
            let mut buf = vec![0u8; 64];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            (api::png_get_rgb_to_gray_status(l, pp), buf[..8].to_vec())
        });
        want_silent(&r, "rgb_to_gray warn with gray pixels");
    }
}

// ===========================================================================
// generic boundaries: NULL pointers for pngread.c entry points
// ===========================================================================

#[test]
fn pngread_entry_points_null_png_ptr() {
    let r = dual("pngread NULL png_ptr", |l| unsafe {
        let n: png_structp = std::ptr::null_mut();
        let mut buf = [0u8; 16];
        let mut rp: [png_bytep; 2] = [buf.as_mut_ptr(), buf.as_mut_ptr()];
        api::png_read_info(l, n, std::ptr::null_mut());
        api::png_read_update_info(l, n, std::ptr::null_mut());
        api::png_start_read_image(l, n);
        api::png_read_row(l, n, buf.as_mut_ptr(), std::ptr::null_mut());
        api::png_read_rows(l, n, rp.as_mut_ptr(), std::ptr::null_mut(), 2);
        api::png_read_image(l, n, rp.as_mut_ptr());
        api::png_read_end(l, n, std::ptr::null_mut());
        api::png_read_png(l, n, std::ptr::null_mut(), 0, std::ptr::null_mut());
        api::png_set_read_status_fn(l, n, None);
        api::png_image_free(l, std::ptr::null_mut());
        0u32
    });
    want_silent(&r, "pngread NULL png_ptr");
    assert!(r.out.is_some(), "NULL png_ptr must be a silent no-op");
}

#[test]
fn pngread_entry_points_null_info_ptr() {
    let b = simple(2, 2, 8, 0);
    dual("png_read_info NULL info_ptr", move |l| unsafe {
        let (pp, _ip) = reader(l, &b);
        api::png_read_info(l, pp, std::ptr::null_mut());
        src_pos()
    });
    // png_read_end(pp, NULL) is documented as legal
    let b = simple(2, 2, 8, 0);
    dual("png_read_end NULL info_ptr", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        let mut buf = vec![0u8; 64];
        api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
        api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
        api::png_read_end(l, pp, std::ptr::null_mut());
        src_pos()
    });
}

#[test]
fn read_rows_and_read_image_null_row_arrays() {
    let b = simple(2, 2, 8, 0);
    dual("png_read_rows NULL arrays", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        api::png_read_rows(l, pp, std::ptr::null_mut(), std::ptr::null_mut(), 2);
        src_pos()
    });
}

// ===========================================================================
// generic boundaries: out-of-range int parameters
// ===========================================================================

#[test]
fn filler_and_add_alpha_flags_out_of_range() {
    let b = simple(2, 2, 8, 2);
    for flags in [-1i32, 0, 1, 2, 99, 0x7fff_ffff] {
        let b = b.clone();
        let label = format!("png_set_filler flags {flags}");
        dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            api::png_set_filler(l, pp, 0xffff, flags);
            0u32
        });
        let b2 = simple(2, 2, 8, 2);
        let label = format!("png_set_add_alpha flags {flags}");
        dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b2);
            api::png_read_info(l, pp, ip);
            api::png_set_add_alpha(l, pp, 0xffff, flags);
            0u32
        });
    }
}

#[test]
fn quantize_bad_arguments() {
    // png_set_quantize is reached through png_rtran_ok; feed it degenerate
    // palette / colour counts and a NULL histogram.
    let b = simple(4, 2, 8, 2);
    // NOTE: png_set_quantize performs no validation of num_palette /
    // maximum_colors; negative values or values above
    // PNG_MAX_PALETTE_LENGTH (256) make the *reference C* index out of bounds
    // (png_malloc((size_t)-1), quantize_index[256], ...), so those argument
    // ranges are undefined behaviour rather than a testable boundary.
    let combos: [(c_int, c_int, bool, c_int); 13] = [
        (0, 0, false, 1),
        (0, 0, false, 0),
        (0, 256, false, 1),
        (4, 4, false, 1),
        (4, 4, true, 1),
        (4, 2, true, 1),
        (4, 2, true, 0),
        (4, 2, false, 1),
        (4, 2, false, 0),
        (2, 4, false, 1),
        (256, 8, true, 1),
        (256, 8, true, 0),
        (256, 256, false, 1),
    ];
    for (num, maxc, hist, full) in combos {
        let b = b.clone();
        let label =
            format!("png_set_quantize(num={num}, max={maxc}, hist={hist}, full={full})");
        dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            let mut pal = vec![png_color::default(); 300];
            for (i, p) in pal.iter_mut().enumerate() {
                p.red = (i * 3) as u8;
                p.green = (i * 5) as u8;
                p.blue = (i * 7) as u8;
            }
            let h: Vec<png_uint_16> = (0..300u16).map(|i| 300 - i).collect();
            let hp = if hist {
                h.as_ptr()
            } else {
                std::ptr::null()
            };
            api::png_set_quantize(l, pp, pal.as_mut_ptr(), num, maxc, hp, full);
            0u32
        });
    }
    // NULL palette: documented silent return
    let b = simple(4, 2, 8, 2);
    dual("png_set_quantize NULL palette", move |l| unsafe {
        let (pp, ip) = reader(l, &b);
        api::png_read_info(l, pp, ip);
        api::png_set_quantize(l, pp, std::ptr::null_mut(), 4, 4, std::ptr::null(), 1);
        0u32
    });
}

#[test]
fn conflicting_16_bit_transforms() {
    // scale_16 / strip_16 / expand_16 in every order and combination
    let b = simple(4, 2, 16, 2);
    for mask in 0u32..8 {
        let b = b.clone();
        let label = format!("16-bit transform mask {mask}");
        dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            if mask & 1 != 0 {
                api::png_set_scale_16(l, pp);
            }
            if mask & 2 != 0 {
                api::png_set_strip_16(l, pp);
            }
            if mask & 4 != 0 {
                api::png_set_expand_16(l, pp);
            }
            api::png_read_update_info(l, pp, ip);
            let mut buf = vec![0u8; 256];
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            api::png_read_row(l, pp, buf.as_mut_ptr(), std::ptr::null_mut());
            (
                api::png_get_rowbytes(l, pp, ip),
                api::png_get_bit_depth(l, pp, ip),
                api::png_get_color_type(l, pp, ip),
                buf[..16].to_vec(),
            )
        });
    }
}

#[test]
fn init_read_transformations_and_resolve_gamma_agree() {
    // png_init_read_transformations is reached from png_read_start_row; also
    // exercise it and png_resolve_file_gamma directly on a header-only struct.
    let b = simple(4, 2, 8, 2);
    for g in [0i32, PNG_FP_1, 45_455, 220_000] {
        let b = b.clone();
        let label = format!("init_read_transformations with screen gamma {g}");
        dual(&label, move |l| unsafe {
            let (pp, ip) = reader(l, &b);
            api::png_read_info(l, pp, ip);
            if g != 0 {
                api::png_set_gamma_fixed(l, pp, g, PNG_FP_1);
            }
            let before = png_resolve_file_gamma(l, pp);
            png_init_read_transformations(l, pp);
            (before, png_resolve_file_gamma(l, pp))
        });
    }
}

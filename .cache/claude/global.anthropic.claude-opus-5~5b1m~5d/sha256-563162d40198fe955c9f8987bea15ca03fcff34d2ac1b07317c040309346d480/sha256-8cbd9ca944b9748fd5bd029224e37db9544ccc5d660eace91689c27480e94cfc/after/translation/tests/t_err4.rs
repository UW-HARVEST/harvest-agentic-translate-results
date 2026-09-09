//! E4: differential tests for the **error paths** in `pngrutil.c`,
//! `pngpread.c`, `pngrio.c` and `pngwio.c`.
//!
//! Every assertion drives BOTH the reference C `libpng.so` and the translated
//! Rust `liblibpng.so` through `dlsym` only, feeds them the identical (usually
//! deliberately corrupt) input, and requires the identical message log *and*
//! the identical observable result.
//!
//! Row numbers in the `// row N` comments refer to `translation/ERRORS.md`
//! (== `.verify/E4.md`).
#![allow(clippy::too_many_arguments)]

mod common;

use common::api::*;
use common::*;
use libloading::Library;
use std::cell::Cell;
use std::ffi::{c_int, c_void};

// ===========================================================================
// generic driver
// ===========================================================================

/// Run the same closure against the C and the Rust library, capturing the
/// error-callback unwind and the message log of each.
fn run2<T, F>(f: F) -> (Run<T>, Run<T>)
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&'static Library) -> T,
{
    let l = libs();
    let c = capture(|| f(&l.c));
    let r = capture(|| f(&l.rs));
    (c, r)
}

/// `run2` + assert the two runs agree.  Also dumps the C log when
/// `T_ERR4_VERBOSE` is set in the environment (used while developing the
/// tests to confirm that the intended rejection site really is reached).
fn diff<T, F>(label: &str, f: F)
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&'static Library) -> T,
{
    let (c, r) = run2(f);
    if std::env::var_os("T_ERR4_VERBOSE").is_some() {
        eprintln!(
            "[{label}] C out={:?} log={}",
            c.out,
            c.log
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    c.assert_eq(&r, label);
}

// ===========================================================================
// PNG stream construction helpers
// ===========================================================================

const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Standard zlib/PNG CRC-32 (reflected, polynomial 0xEDB88320).
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
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

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// `be_u32(len) || name || data || be_u32(crc32(name||data))`
fn ck(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    ck_full(name, data, data.len() as u32, false)
}

/// Same but with a deliberately wrong CRC.
fn ck_bad(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    ck_full(name, data, data.len() as u32, true)
}

/// Same but with a declared length that does not match `data.len()`.
fn ck_len(name: &[u8; 4], data: &[u8], declared: u32) -> Vec<u8> {
    ck_full(name, data, declared, false)
}

fn ck_full(name: &[u8; 4], data: &[u8], declared: u32, bad_crc: bool) -> Vec<u8> {
    let mut v = Vec::with_capacity(12 + data.len());
    v.extend_from_slice(&be32(declared));
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let mut c = {
        let mut t = name.to_vec();
        t.extend_from_slice(data);
        crc32(&t)
    };
    if bad_crc {
        c ^= 0xdead_beef;
    }
    v.extend_from_slice(&be32(c));
    v
}

fn ihdr(w: u32, h: u32, bd: u8, ct: u8, il: u8) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&be32(w));
    v.extend_from_slice(&be32(h));
    v.push(bd);
    v.push(ct);
    v.push(0); // compression
    v.push(0); // filter
    v.push(il);
    v
}

/// One raw ("stored") deflate block, `fin` == BFINAL.
fn stored(data: &[u8], fin: bool) -> Vec<u8> {
    let mut v = Vec::new();
    v.push(if fin { 1 } else { 0 });
    let n = data.len() as u16;
    v.extend_from_slice(&n.to_le_bytes());
    v.extend_from_slice(&(!n).to_le_bytes());
    v.extend_from_slice(data);
    v
}

/// Complete zlib stream (0x78 0x01 header, one final stored block, adler32).
fn zl(data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x78, 0x01];
    v.extend_from_slice(&stored(data, true));
    v.extend_from_slice(&be32(adler32(data)));
    v
}

/// Truncated zlib stream: valid header, one NON-final stored block, no adler.
fn zl_open(data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x78, 0x01];
    v.extend_from_slice(&stored(data, false));
    v
}

/// A deflate block with the reserved BTYPE (3) -> zlib `Z_DATA_ERROR`.
const BAD_BLOCK: [u8; 4] = [0x06, 0x00, 0x00, 0x00];

/// A 1 x `h` 8-bit grayscale image: one filter byte + one sample per row.
fn gray_rows(h: u32) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..h {
        v.push(0); // filter: none
        v.push((i & 0xff) as u8);
    }
    v
}

/// A minimal, complete, valid 1 x `h` grayscale PNG.
fn tiny_png(h: u32) -> Vec<u8> {
    let mut v = SIG.to_vec();
    v.extend_from_slice(&ck(b"IHDR", &ihdr(1, h, 8, 0, 0)));
    v.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(h))));
    v.extend_from_slice(&ck(b"IEND", &[]));
    v
}

// ===========================================================================
// failing allocator (for the OOM branches)
// ===========================================================================

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn fopen(path: *const i8, mode: *const i8) -> *mut c_void;
}

thread_local! {
    static FAIL_LO: Cell<usize> = const { Cell::new(usize::MAX) };
    static FAIL_HI: Cell<usize> = const { Cell::new(0) };
}

fn fail_range(lo: usize, hi: usize) {
    FAIL_LO.with(|c| c.set(lo));
    FAIL_HI.with(|c| c.set(hi));
}
fn fail_none() {
    fail_range(usize::MAX, 0);
}

unsafe extern "C-unwind" fn my_malloc(_pp: png_structp, size: usize) -> png_voidp {
    let lo = FAIL_LO.with(|c| c.get());
    let hi = FAIL_HI.with(|c| c.get());
    if size >= lo && size <= hi {
        return std::ptr::null_mut();
    }
    malloc(size)
}

unsafe extern "C-unwind" fn my_free(_pp: png_structp, p: png_voidp) {
    free(p)
}

/// Read struct with the recording handlers, the harness read source and the
/// controllable allocator above.
unsafe fn reader_mem(l: &Library) -> png_structp {
    let pp = png_create_read_struct_2(
        l,
        PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
        1usize as png_voidp,
        Some(my_malloc),
        Some(my_free),
    );
    assert!(!pp.is_null());
    png_set_read_fn(l, pp, 1usize as png_voidp, Some(read_cb));
    pp
}

// ===========================================================================
// progressive-reader plumbing
// ===========================================================================

thread_local! {
    /// address of `png_read_update_info` in the library currently under test
    static UPD: Cell<usize> = const { Cell::new(0) };
    /// address of `png_set_read_user_transform_fn` in that library
    static SUTF: Cell<usize> = const { Cell::new(0) };
    static N_INFO: Cell<u32> = const { Cell::new(0) };
    static N_ROW: Cell<u32> = const { Cell::new(0) };
    static N_END: Cell<u32> = const { Cell::new(0) };
    /// install the user transform function once this many rows have been seen
    static TF_AFTER_ROW: Cell<u32> = const { Cell::new(0) };
}

type FnUpdateInfo = unsafe extern "C-unwind" fn(png_structp, png_infop);
type FnSetUserTf = unsafe extern "C-unwind" fn(png_structp, Option<PngUserTransformCb>);

unsafe extern "C-unwind" fn info_cb(pp: png_structp, ip: png_infop) {
    N_INFO.with(|c| c.set(c.get() + 1));
    // png_read_update_info must be called exactly once (a second call is a
    // "duplicate call" application error).
    if N_INFO.with(|c| c.get()) == 1 {
        let a = UPD.with(|c| c.get());
        if a != 0 {
            let f: FnUpdateInfo = std::mem::transmute(a);
            f(pp, ip);
        }
    }
}

unsafe extern "C-unwind" fn row_cb(pp: png_structp, _row: png_bytep, _n: png_uint_32, _p: c_int) {
    N_ROW.with(|c| c.set(c.get() + 1));
    let want = TF_AFTER_ROW.with(|c| c.get());
    if want != 0 && N_ROW.with(|c| c.get()) == want {
        let a = SUTF.with(|c| c.get());
        if a != 0 {
            let f: FnSetUserTf = std::mem::transmute(a);
            f(pp, Some(user_tf_cb));
        }
    }
}

unsafe extern "C-unwind" fn end_cb(_pp: png_structp, _ip: png_infop) {
    N_END.with(|c| c.set(c.get() + 1));
}

/// A user transform that does nothing at all; the interesting part is the
/// pixel depth libpng derives from `png_set_user_transform_info`.
unsafe extern "C-unwind" fn user_tf_cb(
    _pp: png_structp,
    _ri: *mut png_row_info,
    _row: png_bytep,
) {
}

fn prog_reset(l: &Library, with_update: bool, tf_after_row: u32) {
    N_INFO.with(|c| c.set(0));
    N_ROW.with(|c| c.set(0));
    N_END.with(|c| c.set(0));
    TF_AFTER_ROW.with(|c| c.set(tf_after_row));
    UPD.with(|c| {
        c.set(if with_update {
            *sym::<FnUpdateInfo>(l, "png_read_update_info") as usize
        } else {
            0
        })
    });
    SUTF.with(|c| {
        c.set(*sym::<FnSetUserTf>(l, "png_set_read_user_transform_fn") as usize)
    });
}

fn counters() -> (u32, u32, u32) {
    (
        N_INFO.with(|c| c.get()),
        N_ROW.with(|c| c.get()),
        N_END.with(|c| c.get()),
    )
}

/// Create a progressive reader with all three callbacks installed.
unsafe fn prog_reader(l: &Library) -> (png_structp, png_infop) {
    let pp = png_create_read_struct(
        l,
        PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
    );
    assert!(!pp.is_null());
    let ip = png_create_info_struct(l, pp);
    assert!(!ip.is_null());
    png_set_progressive_read_fn(
        l,
        pp,
        1usize as png_voidp,
        Some(info_cb),
        Some(row_cb),
        Some(end_cb),
    );
    (pp, ip)
}

/// Feed `data` to a fresh progressive reader in one call.
unsafe fn feed(l: &Library, data: &[u8], with_update: bool) -> (u32, u32, u32) {
    prog_reset(l, with_update, 0);
    let (pp, ip) = prog_reader(l);
    let mut buf = data.to_vec();
    png_process_data(l, pp, ip, buf.as_mut_ptr(), buf.len());
    counters()
}

// ===========================================================================
// pngrio.c / pngwio.c
// ===========================================================================

/// row 160 `png_read_data`  "Call to NULL read function"
/// row 163 `png_set_read_fn` "Can't set both read_data_fn and write_data_fn..."
/// row 286 `png_write_data` "Call to NULL write function"
/// row 289 `png_set_write_fn` "Can't set both read_data_fn and write_data_fn..."
#[test]
fn t_rows_160_163_286_289_io_function_mixups() {
    // row 289 + row 160: png_set_write_fn() on a READ struct warns and NULLs
    // read_data_fn; the next read then hits "Call to NULL read function".
    diff("row289+160: set_write_fn on read struct", |l| unsafe {
        src_set(&tiny_png(1));
        let pp = new_reader(l);
        png_set_write_fn(l, pp, 1usize as png_voidp, Some(write_cb), Some(flush_cb));
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip); // row 160
        0u32
    });

    // row 163 + row 286: png_set_read_fn() on a WRITE struct warns and NULLs
    // write_data_fn; the next write then hits "Call to NULL write function".
    diff("row163+286: set_read_fn on write struct", |l| unsafe {
        sink_reset();
        let pp = new_writer(l);
        png_set_read_fn(l, pp, 1usize as png_voidp, Some(read_cb)); // row 163
        let ip = png_create_info_struct(l, pp);
        png_set_IHDR(
            l,
            pp,
            ip,
            1,
            1,
            8,
            PNG_COLOR_TYPE_GRAY,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip); // row 286
        sink_len() as u32
    });

    // png_set_read_fn(pp, NULL, None) on a read struct installs the *default*
    // (stdio) reader rather than NULL - assert both libraries agree on that.
    diff("set_read_fn(NULL,None) keeps a non-NULL fn", |l| unsafe {
        let pp = new_reader(l);
        png_set_read_fn(l, pp, std::ptr::null_mut(), None);
        png_get_io_ptr(l, pp).is_null()
    });
}

/// row 161 `png_default_read_data` "Read Error"
/// row 287 `png_default_write_data` "Write Error"
#[test]
fn t_rows_161_287_stdio_default_io_failures() {
    let dir = std::env::temp_dir();
    let empty = dir.join("t_err4_empty.bin");
    std::fs::write(&empty, b"").unwrap();
    let ro = dir.join("t_err4_ro.bin");
    std::fs::write(&ro, b"x").unwrap();

    let empty_c = std::ffi::CString::new(empty.to_str().unwrap()).unwrap();
    let ro_c = std::ffi::CString::new(ro.to_str().unwrap()).unwrap();
    let mode_r = std::ffi::CString::new("rb").unwrap();

    // row 161: the default stdio reader on an empty file - fread() returns
    // short, so png_default_read_data calls png_error("Read Error").
    diff("row161: png_default_read_data short read", |l| unsafe {
        let f = fopen(empty_c.as_ptr(), mode_r.as_ptr());
        assert!(!f.is_null());
        let pp = png_create_read_struct(
            l,
            PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
            1usize as png_voidp,
            Some(rec_error),
            Some(rec_warning),
        );
        let ip = png_create_info_struct(l, pp);
        png_init_io(l, pp, f);
        png_read_info(l, pp, ip);
        0u32
    });

    // row 287: the default stdio writer on a read-only stream - fwrite()
    // returns short, so png_default_write_data calls png_error("Write Error").
    diff("row287: png_default_write_data short write", |l| unsafe {
        let f = fopen(ro_c.as_ptr(), mode_r.as_ptr());
        assert!(!f.is_null());
        let pp = png_create_write_struct(
            l,
            PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
            1usize as png_voidp,
            Some(rec_error),
            Some(rec_warning),
        );
        let ip = png_create_info_struct(l, pp);
        png_init_io(l, pp, f);
        png_set_IHDR(
            l,
            pp,
            ip,
            1,
            1,
            8,
            PNG_COLOR_TYPE_GRAY,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
        0u32
    });
}

// ===========================================================================
// pngrutil.c: png_get_uint_31 / png_read_sig / png_read_chunk_header
// ===========================================================================

/// row 186 `png_get_uint_31` "PNG unsigned integer out of range"
#[test]
fn t_row_186_get_uint_31_out_of_range() {
    for (label, bytes) in [
        ("0x80000000", [0x80u8, 0, 0, 0]),
        ("0xffffffff", [0xff, 0xff, 0xff, 0xff]),
        ("0x7fffffff", [0x7f, 0xff, 0xff, 0xff]), // in range: no error
        ("0x00000000", [0, 0, 0, 0]),
    ] {
        diff(&format!("row186: png_get_uint_31 {label}"), move |l| unsafe {
            let pp = new_reader(l);
            png_get_uint_31(l, pp, bytes.as_ptr()) // row 186
        });
    }
}

/// row 187 `png_read_sig` "Not a PNG file"
/// row 188 `png_read_sig` "PNG file corrupted by ASCII conversion"
#[test]
fn t_rows_187_188_read_sig_sequential() {
    // completely wrong first byte -> "Not a PNG file"
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("row187 garbage", b"not a PNG file at all".to_vec()),
        // EBCDIC mangling: 0x89 -> 0x8b etc.  Byte 0 differs, so the
        // "first four bytes" test fails too -> "Not a PNG file".
        (
            "row187 ebcdic",
            vec![0x8b, 0xd7, 0xd5, 0xc7, 0x0d, 0x25, 0x1a, 0x25],
        ),
        // ASCII/CRLF mangling: "\x89PNG\r\n\x1a\n" with \r\n -> \n.  The
        // first four bytes still match -> "corrupted by ASCII conversion".
        ("row188 crlf->lf", vec![137, 80, 78, 71, 10, 26, 10, 0]),
        // lower-case mangling of the trailing bytes only
        ("row188 tail", vec![137, 80, 78, 71, 13, 10, 26, 11]),
    ];
    for (label, data) in cases {
        diff(label, move |l| unsafe {
            src_set(&data);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip); // rows 187 / 188
            src_pos() as u32
        });
    }

    // png_set_sig_bytes(4) then a mangled tail: num_checked == 4, so the
    // "num_checked < 4" guard is false and the ASCII branch is taken.
    diff("row188 with sig_bytes=4", |l| unsafe {
        src_set(&[0u8, 0, 0, 0]);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_set_sig_bytes(l, pp, 4);
        png_read_info(l, pp, ip);
        src_pos() as u32
    });
}

/// row 189 `png_read_chunk_header` "bad header (invalid length)"  [shadowed]
/// row 190 `png_read_chunk_header` "bad header (invalid type)"
#[test]
fn t_rows_189_190_chunk_header() {
    // row 189: a chunk length with bit 31 set.  png_get_uint_31 is called
    // *before* the buf[0] >= 0x80 test, so the reachable message is
    // "PNG unsigned integer out of range"; the "bad header (invalid length)"
    // site is unreachable.  Assert the two libraries agree on what happens.
    diff("row189: length > PNG_UINT_31_MAX", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&[0x80, 0, 0, 0]); // length
        s.extend_from_slice(b"IHDR");
        s.extend_from_slice(&[0u8; 32]);
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip); // row 189 (shadowed by row 186)
        src_pos() as u32
    });

    // row 190: a chunk name containing a non-alphabetic byte.
    for name in [b"IH1R", b"\0\0\0\0", b"IHD*", b"iHdR"] {
        let n = *name;
        diff("row190: bad chunk name", move |l| unsafe {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(&n, &ihdr(1, 1, 8, 0, 0)));
            src_set(&s);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip); // row 190
            src_pos() as u32
        });
    }

    // A chunk whose declared length runs off the end of the stream: the
    // harness read callback reports a short read exactly like a real app.
    diff("chunk length past end of stream", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck_len(b"IHDR", &ihdr(1, 1, 8, 0, 0), 4096));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        src_pos() as u32
    });
}

/// row 191 `png_crc_finish_critical` `png_chunk_warning` "CRC error"
/// row 192 `png_crc_finish_critical` `png_chunk_error`   "CRC error"
#[test]
fn t_rows_191_192_crc_error_all_actions() {
    let actions = [
        PNG_CRC_DEFAULT,
        PNG_CRC_ERROR_QUIT,
        PNG_CRC_WARN_DISCARD,
        PNG_CRC_WARN_USE,
        PNG_CRC_QUIET_USE,
        PNG_CRC_NO_CHANGE,
    ];

    // Critical chunk (IHDR) with a wrong CRC.  Default -> row 192
    // (png_chunk_error); PNG_CRC_WARN_USE -> row 191 (png_chunk_warning).
    for crit in actions {
        for anc in actions {
            diff(
                &format!("rows191/192 critical crc crit={crit} anc={anc}"),
                move |l| unsafe {
                    let mut s = SIG.to_vec();
                    s.extend_from_slice(&ck_bad(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
                    s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
                    s.extend_from_slice(&ck(b"IEND", &[]));
                    src_set(&s);
                    let pp = new_reader(l);
                    png_set_crc_action(l, pp, crit, anc);
                    let ip = png_create_info_struct(l, pp);
                    png_read_info(l, pp, ip); // rows 191 / 192
                    (
                        png_get_image_width(l, pp, ip),
                        png_get_image_height(l, pp, ip),
                        src_pos() as u32,
                    )
                },
            );
        }
    }

    // Ancillary chunk (gAMA) with a wrong CRC.
    for crit in actions {
        for anc in actions {
            diff(
                &format!("rows191/192 ancillary crc crit={crit} anc={anc}"),
                move |l| unsafe {
                    let mut s = SIG.to_vec();
                    s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
                    s.extend_from_slice(&ck_bad(b"gAMA", &be32(45455)));
                    s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
                    s.extend_from_slice(&ck(b"IEND", &[]));
                    src_set(&s);
                    let pp = new_reader(l);
                    png_set_crc_action(l, pp, crit, anc);
                    let ip = png_create_info_struct(l, pp);
                    png_read_info(l, pp, ip); // rows 191 / 192
                    let mut g: png_fixed_point = 0;
                    let valid = png_get_gAMA_fixed(l, pp, ip, &mut g);
                    (valid, g, src_pos() as u32)
                },
            );
        }
    }

    // PLTE with a wrong CRC in a non-colour-mapped image drives the
    // "handle_as_ancillary" variant of png_crc_finish_critical.
    diff("rows191/192 PLTE-as-ancillary crc", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 2, 0)));
        s.extend_from_slice(&ck_bad(b"PLTE", &[1, 2, 3]));
        s.extend_from_slice(&ck(b"IDAT", &zl(&[0, 0, 0, 0])));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        src_pos() as u32
    });

    // IEND with a wrong CRC (also handled as ancillary).
    diff("rows191/192 IEND crc", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck_bad(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 4];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut());
        png_read_end(l, pp, ip);
        src_pos() as u32
    });
}

/// row 194 `png_inflate_claim` `png_chunk_error` "<chunk> using zstream"
#[test]
fn t_row_194_inflate_claim_zstream_in_use() {
    // png_read_end() skips png_read_finish_IDAT (and hence the release of the
    // zstream) when the application asked for IDAT to be handled as an
    // unknown chunk.  A compressed chunk after IDAT then finds zowner != 0.
    diff("row194: zTXt while IDAT owns the zstream", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        let mut ztxt = b"k\0\0".to_vec();
        ztxt.extend_from_slice(&zl(b""));
        s.extend_from_slice(&ck(b"zTXt", &ztxt));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, b"IDAT\0".as_ptr(), 1);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        png_start_read_image(l, pp);
        png_read_end(l, pp, ip); // row 194
        src_pos() as u32
    });
}

/// row 195 `png_handle_PLTE` `png_chunk_error`
#[test]
fn t_row_195_plte_chunk_error() {
    let pal: Vec<u8> = (0u8..3 * 4).collect();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        // duplicate PLTE in a colour-mapped image -> critical error
        ("row195 duplicate", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 3, 0)));
            s.extend_from_slice(&ck(b"PLTE", &pal));
            s.extend_from_slice(&ck(b"PLTE", &pal));
            s
        }),
        // length not a multiple of 3
        ("row195 invalid length", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 3, 0)));
            s.extend_from_slice(&ck(b"PLTE", &[1, 2, 3, 4]));
            s
        }),
        // too many entries
        ("row195 too long", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 3, 0)));
            s.extend_from_slice(&ck(b"PLTE", &vec![7u8; 3 * 257]));
            s
        }),
        // PLTE after IDAT in a colour-mapped image
        ("row195 out of place", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 3, 0)));
            s.extend_from_slice(&ck(b"PLTE", &pal));
            s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
            s.extend_from_slice(&ck(b"PLTE", &pal));
            s.extend_from_slice(&ck(b"IEND", &[]));
            s
        }),
        // PLTE in a grayscale image: benign, not critical
        ("PLTE in grayscale PNG", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s.extend_from_slice(&ck(b"PLTE", &pal));
            s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
            s.extend_from_slice(&ck(b"IEND", &[]));
            s
        }),
    ];
    for (label, data) in cases {
        diff(label, move |l| unsafe {
            src_set(&data);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip); // row 195
            let mut p: *mut png_color = std::ptr::null_mut();
            let mut n: c_int = -1;
            let v = png_get_PLTE(l, pp, ip, &mut p, &mut n);
            (v, n, p.is_null(), src_pos() as u32)
        });
    }
}

/// row 196 `png_handle_iCCP` `png_chunk_warning` "extra compressed data"
#[test]
fn t_row_196_iccp_extra_compressed_data() {
    // A minimal but *valid* 132-byte ICC profile with an empty tag table.
    let mut prof = vec![0u8; 132];
    prof[0..4].copy_from_slice(&be32(132)); // profile length
    prof[12..16].copy_from_slice(b"mntr"); // device class
    prof[16..20].copy_from_slice(b"GRAY"); // data colour space
    prof[20..24].copy_from_slice(b"XYZ "); // PCS
    prof[36..40].copy_from_slice(b"acsp"); // signature
    prof[68..80].copy_from_slice(&[
        0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
    ]); // D50
    prof[128..132].copy_from_slice(&be32(0)); // tag count

    // keyword + separator + compression method + deflate stream + a lot of
    // trailing garbage (more than PNG_INFLATE_BUF_SIZE so that the chunk is
    // not fully consumed while inflating).
    let mut payload = b"i\0\0".to_vec();
    payload.extend_from_slice(&zl(&prof));
    payload.extend_from_slice(&vec![0x5au8; 3000]);

    let mut s = SIG.to_vec();
    s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
    s.extend_from_slice(&ck(b"iCCP", &payload));
    s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
    s.extend_from_slice(&ck(b"IEND", &[]));

    diff("row196: iCCP extra compressed data", move |l| unsafe {
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip); // row 196
        let mut name: png_charp = std::ptr::null_mut();
        let mut ctype: c_int = -1;
        let mut profile: png_bytep = std::ptr::null_mut();
        let mut len: png_uint_32 = 0;
        let v = png_get_iCCP(l, pp, ip, &mut name, &mut ctype, &mut profile, &mut len);
        (v, ctype, len, profile.is_null(), src_pos() as u32)
    });

    // The same chunk but with benign errors turned into hard errors: the
    // "extra compressed data" becomes a png_chunk_benign_error instead.
    let mut prof2 = vec![0u8; 132];
    prof2.copy_from_slice(&{
        let mut p = vec![0u8; 132];
        p[0..4].copy_from_slice(&be32(132));
        p[12..16].copy_from_slice(b"mntr");
        p[16..20].copy_from_slice(b"GRAY");
        p[20..24].copy_from_slice(b"XYZ ");
        p[36..40].copy_from_slice(b"acsp");
        p[68..80].copy_from_slice(&[
            0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
        ]);
        p
    });
    let mut payload2 = b"i\0\0".to_vec();
    payload2.extend_from_slice(&zl(&prof2));
    payload2.extend_from_slice(&vec![0x5au8; 3000]);
    let mut s2 = SIG.to_vec();
    s2.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
    s2.extend_from_slice(&ck(b"iCCP", &payload2));
    s2.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
    s2.extend_from_slice(&ck(b"IEND", &[]));

    diff("row196: iCCP extra data, benign=off", move |l| unsafe {
        src_set(&s2);
        let pp = new_reader(l);
        png_set_benign_errors(l, pp, 0);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        src_pos() as u32
    });

    // Truncated / short iCCP chunks exercise the neighbouring branches.
    // NOTE: png_handle_iCCP reads the first 81 bytes into its keyword buffer
    // and then requires at least LZ77Min (11) more, so these payloads have to
    // be > 92 bytes to get past the "too short" test.
    let filler = [0u8; 120];
    for (label, payload) in [
        ("iCCP too short", b"i\0\0short".to_vec()),
        ("iCCP bad compression method", {
            let mut v = b"i\0\x01".to_vec();
            v.extend_from_slice(&zl(&filler));
            v
        }),
        ("iCCP empty keyword", {
            let mut v = b"\0\0".to_vec();
            v.extend_from_slice(&zl(&filler));
            v
        }),
        ("iCCP corrupt deflate", {
            let mut v = b"i\0\0".to_vec();
            v.extend_from_slice(&[0x78, 0x01]);
            v.extend_from_slice(&BAD_BLOCK);
            v.extend_from_slice(&[0u8; 120]);
            v
        }),
        ("iCCP bad window bits", {
            let mut v = b"i\0\0".to_vec();
            v.extend_from_slice(&[0x88, 0x1d]);
            v.extend_from_slice(&stored(&filler, true));
            v.extend_from_slice(&be32(adler32(&filler)));
            v
        }),
        ("iCCP profile too short", {
            let mut v = b"i\0\0".to_vec();
            v.extend_from_slice(&zl(&[7u8; 100]));
            v
        }),
    ] {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"iCCP", &payload));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        diff(label, move |l| unsafe {
            src_set(&s);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip);
            src_pos() as u32
        });
    }
}

// ===========================================================================
// png_handle_sPLT
// ===========================================================================

fn splt_stream(payload: &[u8]) -> Vec<u8> {
    let mut s = SIG.to_vec();
    s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
    s.extend_from_slice(&ck(b"sPLT", payload));
    s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
    s.extend_from_slice(&ck(b"IEND", &[]));
    s
}

/// row 197 `png_handle_sPLT` "No space in chunk cache for sPLT"
/// row 198 `png_handle_sPLT` "malformed sPLT chunk"
/// row 199 `png_handle_sPLT` "sPLT chunk has bad length"
#[test]
fn t_rows_197_198_199_splt() {
    // row 197: user_chunk_cache_max == 2 -> the pre-decrement hits 1.
    for cache in [0u32, 1, 2, 3] {
        let mut good = b"p\0".to_vec();
        good.push(8); // depth 8 -> 6 bytes per entry
        good.extend_from_slice(&[1, 2, 3, 4, 0, 1]);
        let s = splt_stream(&good);
        diff(
            &format!("row197: sPLT chunk_cache_max={cache}"),
            move |l| unsafe {
                src_set(&s);
                let pp = new_reader(l);
                png_set_chunk_cache_max(l, pp, cache);
                let ip = png_create_info_struct(l, pp);
                png_read_info(l, pp, ip); // row 197
                let mut e: *mut png_sPLT_t = std::ptr::null_mut();
                let n = png_get_sPLT(l, pp, ip, &mut e);
                (n, e.is_null(), png_get_chunk_cache_max(l, pp))
            },
        );
    }

    // row 198: "malformed sPLT chunk" (length < 2, or the name eats the chunk)
    for payload in [
        vec![0u8],           // just the name terminator, length 1
        vec![0u8, 0u8],      // length 2, no depth byte available
        b"abc".to_vec(),     // unterminated name
        b"ab\0".to_vec(),    // name + terminator, no depth
    ] {
        let s = splt_stream(&payload);
        diff("row198: malformed sPLT", move |l| unsafe {
            src_set(&s);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip); // row 198
            let mut e: *mut png_sPLT_t = std::ptr::null_mut();
            (png_get_sPLT(l, pp, ip, &mut e), e.is_null())
        });
    }

    // row 199: "sPLT chunk has bad length" (data length not a multiple of the
    // entry size)
    for (depth, extra) in [(8u8, 1usize), (8, 5), (16, 1), (16, 9), (4, 3)] {
        let mut payload = b"q\0".to_vec();
        payload.push(depth);
        payload.extend(std::iter::repeat(0xa5u8).take(extra));
        let s = splt_stream(&payload);
        diff("row199: sPLT bad length", move |l| unsafe {
            src_set(&s);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip); // row 199
            let mut e: *mut png_sPLT_t = std::ptr::null_mut();
            (png_get_sPLT(l, pp, ip, &mut e), e.is_null())
        });
    }
}

/// row 201 `png_handle_sPLT` "sPLT chunk requires too much memory"
#[test]
fn t_row_201_splt_out_of_memory() {
    // depth != 8 -> 10 bytes per entry; 7 entries -> exactly 70 bytes.
    let mut payload = b"r\0".to_vec();
    payload.push(16);
    payload.extend(std::iter::repeat(0u8).take(70));
    let s = splt_stream(&payload);

    diff("row201: sPLT entry array OOM", move |l| unsafe {
        fail_none();
        src_set(&s);
        let pp = reader_mem(l);
        let ip = png_create_info_struct(l, pp);
        fail_range(70, 70);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            png_read_info(l, pp, ip); // row 201
        }));
        fail_none();
        if r.is_err() {
            std::panic::resume_unwind(r.unwrap_err());
        }
        let mut e: *mut png_sPLT_t = std::ptr::null_mut();
        (png_get_sPLT(l, pp, ip, &mut e), e.is_null())
    });
}

// ===========================================================================
// png_handle_unknown
// ===========================================================================

thread_local! {
    static USER_CHUNK_RET: Cell<c_int> = const { Cell::new(0) };
    static USER_CHUNK_SEEN: Cell<u32> = const { Cell::new(0) };
}

unsafe extern "C-unwind" fn user_chunk_cb(
    _pp: png_structp,
    _c: *mut png_unknown_chunk,
) -> c_int {
    USER_CHUNK_SEEN.with(|c| c.set(c.get() + 1));
    USER_CHUNK_RET.with(|c| c.get())
}

/// row 202 `png_handle_unknown` "error in user chunk"
/// row 203 `png_handle_unknown` "Saving unknown chunk:"
/// row 204 `png_handle_unknown` "forcing save of an unhandled chunk; ..."
/// row 206 `png_handle_unknown` "unhandled critical chunk"
#[test]
fn t_rows_202_203_204_206_handle_unknown() {
    // rows 202 (ret < 0), 203 + 204 (ret == 0), and the ret > 0 control case.
    for ret in [-1i32, 0, 1] {
        for benign in [0i32, 1] {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s.extend_from_slice(&ck(b"uNKn", b"payload"));
            s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
            s.extend_from_slice(&ck(b"IEND", &[]));
            diff(
                &format!("rows202/203/204: user chunk ret={ret} benign={benign}"),
                move |l| unsafe {
                    USER_CHUNK_RET.with(|c| c.set(ret));
                    USER_CHUNK_SEEN.with(|c| c.set(0));
                    src_set(&s);
                    let pp = new_reader(l);
                    png_set_benign_errors(l, pp, benign);
                    png_set_read_user_chunk_fn(l, pp, 1usize as png_voidp, Some(user_chunk_cb));
                    let ip = png_create_info_struct(l, pp);
                    png_read_info(l, pp, ip); // rows 202 / 203 / 204
                    let mut u: *mut png_unknown_chunk = std::ptr::null_mut();
                    (
                        png_get_unknown_chunks(l, pp, ip, &mut u),
                        USER_CHUNK_SEEN.with(|c| c.get()),
                        src_pos() as u32,
                    )
                },
            );
        }
    }

    // row 206: an unknown *critical* chunk that nothing handles or saves.
    for keep in [
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
    ] {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"CrIT", b"zz"));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        diff(
            &format!("row206: unhandled critical chunk keep={keep}"),
            move |l| unsafe {
                src_set(&s);
                let pp = new_reader(l);
                png_set_keep_unknown_chunks(l, pp, keep, std::ptr::null(), 0);
                let ip = png_create_info_struct(l, pp);
                png_read_info(l, pp, ip); // row 206
                let mut u: *mut png_unknown_chunk = std::ptr::null_mut();
                (png_get_unknown_chunks(l, pp, ip, &mut u), src_pos() as u32)
            },
        );
    }

    // An unknown chunk larger than the chunk-malloc limit ("unknown chunk
    // exceeds memory limits") - the branch immediately above row 202.
    diff("unknown chunk exceeds memory limits", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"uNKn", &vec![3u8; 600]));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
        png_set_chunk_malloc_max(l, pp, 64);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut u: *mut png_unknown_chunk = std::ptr::null_mut();
        (png_get_unknown_chunks(l, pp, ip, &mut u), src_pos() as u32)
    });
}

// ===========================================================================
// png_handle_chunk
// ===========================================================================

/// row 207 `png_handle_chunk` "missing IHDR"
#[test]
fn t_row_207_missing_ihdr() {
    for name in [b"gAMA", b"sRGB", b"pHYs", b"tIME", b"PLTE", b"IEND"] {
        let n = *name;
        diff("row207: known chunk before IHDR", move |l| unsafe {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(&n, &[0u8; 4]));
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
            s.extend_from_slice(&ck(b"IEND", &[]));
            src_set(&s);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip); // row 207
            src_pos() as u32
        });
    }

    // A zero-length IDAT before IHDR takes the dedicated png_read_info path.
    diff("IDAT before IHDR", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IDAT", &[]));
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        src_pos() as u32
    });
}

/// row 208 `png_handle_chunk` `png_chunk_error` for a critical chunk
#[test]
fn t_row_208_critical_chunk_position_and_length() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        // duplicate IHDR -> "duplicate"
        ("row208 duplicate IHDR", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s
        }),
        // IHDR too short -> "too short"
        ("row208 short IHDR", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &[0u8; 12]));
            s
        }),
        // IHDR too long -> "too long"
        ("row208 long IHDR", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &[0u8; 14]));
            s
        }),
        // IEND before any IDAT -> "out of place"
        ("row208 early IEND", {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s.extend_from_slice(&ck(b"IEND", &[]));
            s
        }),
    ];
    for (label, data) in cases {
        diff(label, move |l| unsafe {
            src_set(&data);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip); // row 208
            src_pos() as u32
        });
    }

    // Ancillary counterparts: benign errors, so reading continues.
    let anc: Vec<(&str, Vec<u8>)> = vec![
        ("gAMA too short", ck(b"gAMA", &[0u8; 3])),
        ("gAMA too long", ck(b"gAMA", &[0u8; 5])),
        ("duplicate gAMA", {
            let mut v = ck(b"gAMA", &be32(45455));
            v.extend_from_slice(&ck(b"gAMA", &be32(45455)));
            v
        }),
        ("cHRM bad length", ck(b"cHRM", &[0u8; 31])),
        ("sRGB bad length", ck(b"sRGB", &[0u8; 2])),
        ("cICP bad length", ck(b"cICP", &[0u8; 3])),
        ("cLLI bad length", ck(b"cLLI", &[0u8; 7])),
        ("mDCV bad length", ck(b"mDCV", &[0u8; 23])),
        ("eXIf too short", ck(b"eXIf", &[0u8; 3])),
        ("sBIT bad length", ck(b"sBIT", &[0u8; 5])),
        ("tIME bad length", ck(b"tIME", &[0u8; 6])),
        ("pHYs bad length", ck(b"pHYs", &[0u8; 8])),
        ("oFFs bad length", ck(b"oFFs", &[0u8; 8])),
        ("pCAL too short", ck(b"pCAL", &[0u8; 13])),
        ("sCAL too short", ck(b"sCAL", &[0u8; 3])),
        ("hIST too long", ck(b"hIST", &[0u8; 1026])),
        ("bKGD too long", ck(b"bKGD", &[0u8; 7])),
        ("tRNS too long", ck(b"tRNS", &[0u8; 257])),
        ("tEXt too short", ck(b"tEXt", &[0u8; 1])),
        ("iTXt too short", ck(b"iTXt", &[0u8; 5])),
        ("zTXt too short", ck(b"zTXt", &[0u8; 13])),
        ("sPLT too short", ck(b"sPLT", &[0u8; 2])),
        ("bKGD for grayscale", ck(b"bKGD", &[0u8; 6])),
        ("hIST without PLTE", ck(b"hIST", &[0u8; 4])),
    ];
    for (label, chunk) in anc {
        diff(label, move |l| unsafe {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s.extend_from_slice(&chunk);
            s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
            s.extend_from_slice(&ck(b"IEND", &[]));
            src_set(&s);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip);
            (png_get_valid(l, pp, ip, 0xffff_ffff), src_pos() as u32)
        });
    }

    // Ancillary chunks that appear after IDAT ("out of place").
    for name in [b"tRNS", b"bKGD", b"hIST", b"pHYs", b"oFFs", b"sPLT", b"cHRM"] {
        let n = *name;
        diff("ancillary after IDAT", move |l| unsafe {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
            s.extend_from_slice(&ck(&n, &[0u8; 4]));
            s.extend_from_slice(&ck(b"IEND", &[]));
            src_set(&s);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip);
            let mut row = [0u8; 8];
            png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut());
            png_read_end(l, pp, ip);
            (png_get_valid(l, pp, ip, 0xffff_ffff), src_pos() as u32)
        });
    }

    // IHDR with each field invalid, plus a non-zero-length IEND and a
    // missing IEND.
    let ihdrs: Vec<(&str, Vec<u8>)> = vec![
        ("IHDR width 0", ihdr(0, 1, 8, 0, 0)),
        ("IHDR height 0", ihdr(1, 0, 8, 0, 0)),
        ("IHDR bit depth 3", ihdr(1, 1, 3, 0, 0)),
        ("IHDR bit depth 0", ihdr(1, 1, 0, 0, 0)),
        ("IHDR colour type 1", ihdr(1, 1, 8, 1, 0)),
        ("IHDR colour type 5", ihdr(1, 1, 8, 5, 0)),
        ("IHDR palette 16-bit", ihdr(1, 1, 16, 3, 0)),
        ("IHDR gray+alpha 1-bit", ihdr(1, 1, 1, 4, 0)),
        ("IHDR interlace 2", ihdr(1, 1, 8, 0, 2)),
        ("IHDR compression 1", {
            let mut v = ihdr(1, 1, 8, 0, 0);
            v[10] = 1;
            v
        }),
        ("IHDR filter 1", {
            let mut v = ihdr(1, 1, 8, 0, 0);
            v[11] = 1;
            v
        }),
        ("IHDR width > 2^31", {
            let mut v = ihdr(1, 1, 8, 0, 0);
            v[0..4].copy_from_slice(&[0x80, 0, 0, 0]);
            v
        }),
    ];
    for (label, payload) in ihdrs {
        diff(label, move |l| unsafe {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &payload));
            s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
            s.extend_from_slice(&ck(b"IEND", &[]));
            src_set(&s);
            let pp = new_reader(l);
            let ip = png_create_info_struct(l, pp);
            png_read_info(l, pp, ip);
            src_pos() as u32
        });
    }

    diff("IEND with non-zero length", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", b"junk"));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 8];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut());
        png_read_end(l, pp, ip);
        src_pos() as u32
    });

    diff("missing IEND", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 8];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut());
        png_read_end(l, pp, ip);
        src_pos() as u32
    });

    diff("zero-length IDAT only", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &[]));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 8];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut());
        0u32
    });
}

// ===========================================================================
// png_combine_row
// ===========================================================================

/// row 209 `png_combine_row` "internal row logic error"
#[test]
fn t_row_209_combine_row_logic_error() {
    diff("row209: combine_row on a fresh struct", |l| unsafe {
        let pp = new_reader(l);
        let mut dst = [0u8; 32];
        png_combine_row(l, pp, dst.as_mut_ptr(), 0); // row 209
        0u32
    });
    diff("row209: combine_row display=1", |l| unsafe {
        let pp = new_reader(l);
        let mut dst = [0u8; 32];
        png_combine_row(l, pp, dst.as_mut_ptr(), 1); // row 209
        0u32
    });
}

/// row 210 `png_combine_row` "internal row size calculation error"
#[test]
fn t_row_210_combine_row_size_calculation_error() {
    // png_read_update_info() caches info_ptr->rowbytes in
    // png_struct::info_rowbytes.  Feeding it an info struct whose width has
    // been overwritten makes that cache disagree with png_struct::width.
    diff("row210: info_rowbytes mismatch", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(8, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&[0u8, 1, 2, 3, 4, 5, 6, 7, 8])));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        // Lie about the width in info_ptr only.
        png_set_IHDR(
            l,
            pp,
            ip,
            16,
            1,
            8,
            PNG_COLOR_TYPE_GRAY,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        png_read_update_info(l, pp, ip);
        let mut row = [0u8; 64];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut()); // row 210
        0u32
    });
}

/// row 212 `png_combine_row` "invalid user transform pixel depth"
#[test]
fn t_row_212_combine_row_invalid_user_transform_depth() {
    // 8x8 interlaced grayscale; a user transform declaring 3 bits x 3
    // channels gives a 9-bit pixel depth, which is >= 8 but not a multiple
    // of 8, so the de-interlacing copy in png_combine_row rejects it.
    let mut idat = Vec::new();
    // Adam7 pass 1 of an 8x8 image is a single 1-pixel row.
    for _ in 0..64 {
        idat.push(0u8);
    }
    let mut s = SIG.to_vec();
    s.extend_from_slice(&ck(b"IHDR", &ihdr(8, 8, 8, 0, 1)));
    s.extend_from_slice(&ck(b"IDAT", &zl(&idat)));
    s.extend_from_slice(&ck(b"IEND", &[]));

    diff("row212: 9-bit user transform depth", move |l| unsafe {
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        png_set_interlace_handling(l, pp);
        png_set_user_transform_info(l, pp, std::ptr::null_mut(), 3, 3);
        png_set_read_user_transform_fn(l, pp, Some(user_tf_cb));
        // png_start_read_image (not png_read_update_info) leaves
        // png_struct::info_rowbytes at 0, so the row-size check above is
        // skipped and the pixel-depth check is the one that fires.
        png_start_read_image(l, pp);
        let mut row = [0u8; 256];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut()); // row 212
        0u32
    });
}

// ===========================================================================
// png_read_IDAT_data / png_read_start_row
// ===========================================================================

/// row 213 `png_read_IDAT_data` "Not enough image data" (non-IDAT header)
/// row 216 `png_read_IDAT_data` "Not enough image data" (stream ended early)
#[test]
fn t_rows_213_216_not_enough_image_data() {
    // row 213: the IDAT list ends (IEND follows) while zlib still wants data.
    diff("row213: IDAT list too short", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 4, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl_open(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 8];
        for _ in 0..4 {
            png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut()); // row 213
        }
        0u32
    });

    // row 216: a *complete* zlib stream that ends part way through a row, so
    // png_read_IDAT_data breaks out of its loop with avail_out > 0.
    diff("row216: LZ stream ends before the image", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 4, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&[0u8, 0, 0])));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 8];
        for _ in 0..4 {
            png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut()); // row 216
        }
        0u32
    });

    // "Extra compressed data" / "Too much image data": the mirror branches.
    diff("extra/too much image data", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(4))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 8];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut());
        png_read_end(l, pp, ip);
        0u32
    });
}

/// row 214 `png_read_IDAT_data` `png_chunk_error` "out of memory"
#[test]
fn t_row_214_idat_read_buffer_out_of_memory() {
    let mut s = SIG.to_vec();
    s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
    let mut big = zl(&gray_rows(1));
    big.resize(5000, 0);
    s.extend_from_slice(&ck(b"IDAT", &big));
    s.extend_from_slice(&ck(b"IEND", &[]));

    diff("row214: IDAT read buffer OOM", move |l| unsafe {
        fail_none();
        src_set(&s);
        let pp = reader_mem(l);
        let ip = png_create_info_struct(l, pp);
        png_set_compression_buffer_size(l, pp, 4242);
        png_read_info(l, pp, ip);
        png_start_read_image(l, pp);
        fail_range(4242, 4242);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut row = [0u8; 8];
            png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut()); // row 214
        }));
        fail_none();
        if r.is_err() {
            std::panic::resume_unwind(r.unwrap_err());
        }
        0u32
    });
}

/// row 215 `png_read_IDAT_data` `png_chunk_error` (zlib message)
#[test]
fn t_row_215_idat_decompression_error() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("row215 reserved block type", {
            let mut v = vec![0x78u8, 0x01];
            v.extend_from_slice(&BAD_BLOCK);
            v
        }),
        ("row215 bad window bits", {
            let mut v = vec![0x88u8, 0x1d];
            v.extend_from_slice(&stored(&gray_rows(1), true));
            v.extend_from_slice(&be32(adler32(&gray_rows(1))));
            v
        }),
        ("row215 bad zlib header check", {
            let mut v = vec![0x78u8, 0x02];
            v.extend_from_slice(&stored(&gray_rows(1), true));
            v
        }),
        ("row215 adler mismatch", {
            let mut v = vec![0x78u8, 0x01];
            v.extend_from_slice(&stored(&gray_rows(1), true));
            v.extend_from_slice(&be32(adler32(&gray_rows(1)) ^ 0xffff));
            v
        }),
        ("row215 corrupt huffman block", {
            let mut v = vec![0x78u8, 0x01];
            v.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
            v
        }),
    ];
    for (label, idat) in cases {
        for ignore_adler in [false, true] {
            let idat = idat.clone();
            diff(
                &format!("{label} ignore_adler={ignore_adler}"),
                move |l| unsafe {
                    let mut s = SIG.to_vec();
                    s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
                    s.extend_from_slice(&ck(b"IDAT", &idat));
                    s.extend_from_slice(&ck(b"IEND", &[]));
                    src_set(&s);
                    let pp = new_reader(l);
                    let opt = if ignore_adler {
                        png_set_option(l, pp, PNG_IGNORE_ADLER32, PNG_OPTION_ON)
                    } else {
                        -1
                    };
                    let ip = png_create_info_struct(l, pp);
                    png_read_info(l, pp, ip);
                    let mut row = [0u8; 8];
                    png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut()); // row 215
                    png_read_end(l, pp, ip);
                    (opt, src_pos() as u32)
                },
            );
        }
    }

    // A chunk-malloc limit smaller than the inflated size.
    diff("chunk_malloc_max below the inflated size", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        png_set_chunk_malloc_max(l, pp, 1);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 8];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut());
        png_read_end(l, pp, ip);
        src_pos() as u32
    });
}

/// row 220 `png_read_start_row` `png_error(png_ptr->zstream.msg)`
#[test]
fn t_row_220_inflate_claim_failure() {
    let s = tiny_png(1);
    diff("row220: inflateInit2 out of memory", move |l| unsafe {
        fail_none();
        src_set(&s);
        let pp = reader_mem(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        // Everything png_read_start_row needs for a 1x1 image is tiny; the
        // only large allocation left is zlib's inflate state.
        fail_range(1024, usize::MAX);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            png_start_read_image(l, pp); // row 220
        }));
        fail_none();
        if r.is_err() {
            std::panic::resume_unwind(r.unwrap_err());
        }
        0u32
    });
}

// ===========================================================================
// pngpread.c
// ===========================================================================

/// row 68 `png_process_data_skip` "png_process_data_skip is not implemented..."
#[test]
fn t_row_68_process_data_skip() {
    for benign in [0i32, 1] {
        diff(
            &format!("row68: png_process_data_skip benign={benign}"),
            move |l| unsafe {
                let pp = new_reader(l);
                png_set_benign_errors(l, pp, benign);
                png_process_data_skip(l, pp) // row 68
            },
        );
    }
}

/// row 69 `png_push_read_sig` "Not a PNG file"
/// row 70 `png_push_read_sig` "PNG file corrupted by ASCII conversion"
#[test]
fn t_rows_69_70_push_read_sig() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("row69 garbage", b"nope, not a PNG".to_vec()),
        (
            "row69 ebcdic",
            vec![0x8b, 0xd7, 0xd5, 0xc7, 0x0d, 0x25, 0x1a, 0x25],
        ),
        ("row70 crlf->lf", vec![137, 80, 78, 71, 10, 26, 10, 0]),
        ("row70 tail", vec![137, 80, 78, 71, 13, 10, 26, 11]),
    ];
    for (label, data) in cases {
        diff(label, move |l| unsafe { feed(l, &data, true) });
    }

    // The signature split across two png_process_data calls exercises the
    // partially-checked path (num_to_check < 8).
    diff("row69/70: signature fed one byte at a time", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        let bad = [137u8, 80, 78, 71, 13, 10, 26, 11];
        for b in bad {
            let mut one = [b];
            png_process_data(l, pp, ip, one.as_mut_ptr(), 1);
        }
        counters()
    });
}

/// row 71 `png_push_read_chunk` "Missing IHDR before IDAT"
/// row 72 `png_push_read_chunk` "Missing PLTE before IDAT"
/// row 74 `png_push_read_chunk` "Invalid IHDR length"
#[test]
fn t_rows_71_72_74_push_read_chunk() {
    // row 71
    diff("row71: IDAT before IHDR (progressive)", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true)
    });

    // row 72
    diff("row72: palette image without PLTE", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 3, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true)
    });

    // row 74
    for len in [0usize, 12, 14, 32] {
        diff(&format!("row74: IHDR length {len}"), move |l| unsafe {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &vec![0u8; len]));
            s.extend_from_slice(&ck(b"IEND", &[]));
            feed(l, &s, true)
        });
    }
}

/// row 73 `png_push_read_chunk` "Too many IDATs found"
/// row 84 `png_process_IDAT_data` "Extra compression data in IDAT"
#[test]
fn t_rows_73_84_too_many_idats() {
    // row 73: IDAT, a non-IDAT chunk, then another IDAT.
    diff("row73: IDAT after a non-IDAT chunk", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"tEXt", b"k\0v"));
        s.extend_from_slice(&ck(b"IDAT", &[1, 2, 3]));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true) // rows 73 + 84
    });

    // row 84 on its own: trailing bytes after the zlib end code, same IDAT.
    diff("row84: bytes after the LZ end code", |l| unsafe {
        let mut idat = zl(&gray_rows(1));
        idat.extend_from_slice(&[9, 9, 9]);
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &idat));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true) // row 84
    });

    // Benign errors disabled turns row 73 into a hard error.
    diff("row73 with benign errors off", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        png_set_benign_errors(l, pp, 0);
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"tEXt", b"k\0v"));
        s.extend_from_slice(&ck(b"IDAT", &[1, 2, 3]));
        s.extend_from_slice(&ck(b"IEND", &[]));
        png_process_data(l, pp, ip, s.as_mut_ptr(), s.len());
        counters()
    });
}

/// row 76 `png_push_save_buffer` "Insufficient memory for save_buffer"
#[test]
fn t_row_76_save_buffer_out_of_memory() {
    diff("row76: save_buffer allocation fails", |l| unsafe {
        fail_none();
        prog_reset(l, true, 0);
        let pp = png_create_read_struct_2(
            l,
            PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
            1usize as png_voidp,
            Some(rec_error),
            Some(rec_warning),
            1usize as png_voidp,
            Some(my_malloc),
            Some(my_free),
        );
        assert!(!pp.is_null());
        let ip = png_create_info_struct(l, pp);
        png_set_progressive_read_fn(
            l,
            pp,
            1usize as png_voidp,
            Some(info_cb),
            Some(row_cb),
            Some(end_cb),
        );
        let mut sig = SIG.to_vec();
        png_process_data(l, pp, ip, sig.as_mut_ptr(), sig.len());
        // 3 bytes < the 8 needed for a chunk header -> png_push_save_buffer
        // wants save_buffer_size(0) + current(3) + 256 == 259 bytes.
        fail_range(259, 259);
        let mut three = [0u8, 0, 13];
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            png_process_data(l, pp, ip, three.as_mut_ptr(), 3); // row 76
        }));
        fail_none();
        if r.is_err() {
            std::panic::resume_unwind(r.unwrap_err());
        }
        counters()
    });

    // The same feed without the injected failure must succeed (the save
    // buffer path itself is exercised).
    diff("save_buffer path, allocation succeeds", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        let full = tiny_png(1);
        for b in full {
            let mut one = [b];
            png_process_data(l, pp, ip, one.as_mut_ptr(), 1);
        }
        counters()
    });
}

/// row 78 `png_push_read_IDAT` "Not enough compressed data"
#[test]
fn t_row_78_not_enough_compressed_data() {
    diff("row78: non-IDAT header with the LZ stream open", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 2, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl_open(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true) // row 78
    });
}

/// row 80 `png_process_IDAT_data` "Truncated compressed data in IDAT"
/// row 83 `png_process_IDAT_data` "Extra compressed data in IDAT"
#[test]
fn t_rows_80_83_idat_truncated_and_extra() {
    // row 80: the only row of the image is completed by the first IDAT, then
    // a second IDAT damages the LZ stream, so row_number >= num_rows when the
    // zlib error is reported.
    diff("row80: damage after the last row", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl_open(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IDAT", &BAD_BLOCK));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true) // row 80
    });

    // Same shape but with benign errors turned off: still a warning, because
    // this site is png_warning, not png_benign_error.
    diff("row80: benign errors off", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        png_set_benign_errors(l, pp, 0);
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl_open(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IDAT", &BAD_BLOCK));
        s.extend_from_slice(&ck(b"IEND", &[]));
        png_process_data(l, pp, ip, s.as_mut_ptr(), s.len()); // row 80
        counters()
    });

    // row 83: more uncompressed output than the image needs.
    diff("row83: extra rows in the LZ stream", |l| unsafe {
        let mut idat = zl_open(&[0u8, 0x11, 0u8, 0x22]);
        idat.extend_from_slice(&stored(&[0u8, 0x33], true));
        idat.extend_from_slice(&be32(1));
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &idat));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true) // row 83
    });
}

/// row 81 `png_process_IDAT_data` "IDAT: ADLER32 checksum mismatch"
/// row 82 `png_process_IDAT_data` "Decompression error in IDAT"
#[test]
fn t_rows_81_82_idat_data_and_decompression_errors() {
    // row 81: Z_DATA_ERROR before the last row of the image.
    for (label, idat) in [
        ("row81 reserved block type", {
            let mut v = zl_open(&gray_rows(1));
            v.extend_from_slice(&BAD_BLOCK);
            v
        }),
        ("row81 bad window bits", {
            let mut v = vec![0x88u8, 0x1d];
            v.extend_from_slice(&stored(&gray_rows(1), true));
            v
        }),
        ("row81 adler mismatch", {
            let mut v = vec![0x78u8, 0x01];
            v.extend_from_slice(&stored(&gray_rows(2), true));
            v.extend_from_slice(&be32(adler32(&gray_rows(2)) ^ 0x5a5a));
            v
        }),
    ] {
        for benign in [0i32, 1] {
            let idat = idat.clone();
            diff(&format!("{label} benign={benign}"), move |l| unsafe {
                prog_reset(l, true, 0);
                let (pp, ip) = prog_reader(l);
                png_set_benign_errors(l, pp, benign);
                let mut s = SIG.to_vec();
                s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 3, 8, 0, 0)));
                s.extend_from_slice(&ck(b"IDAT", &idat));
                s.extend_from_slice(&ck(b"IEND", &[]));
                png_process_data(l, pp, ip, s.as_mut_ptr(), s.len()); // row 81
                counters()
            });
        }
    }

    // row 82: a zlib error that is *not* Z_DATA_ERROR.  A header with FDICT
    // set makes inflate() return Z_NEED_DICT.
    diff("row82: Z_NEED_DICT in IDAT", |l| unsafe {
        let idat = vec![0x78u8, 0x20, 0x11, 0x22, 0x33, 0x44, 0x55];
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 3, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &idat));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true) // row 82
    });
}

/// row 85 `png_push_process_row` "bad adaptive filter value"
#[test]
fn t_row_85_bad_adaptive_filter_value() {
    for f in [5u8, 6, 200, 255] {
        diff(&format!("row85: filter byte {f}"), move |l| unsafe {
            let mut s = SIG.to_vec();
            s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
            s.extend_from_slice(&ck(b"IDAT", &zl(&[f, 0x40])));
            s.extend_from_slice(&ck(b"IEND", &[]));
            feed(l, &s, true) // row 85
        });
    }

    // Same site in the sequential reader for comparison.
    diff("sequential bad adaptive filter value", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&[9u8, 0x40])));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut row = [0u8; 8];
        png_read_row(l, pp, row.as_mut_ptr(), std::ptr::null_mut());
        0u32
    });
}

/// row 86 `png_push_process_row` "progressive row overflow"
/// row 87 `png_push_process_row` "internal progressive row size calculation
///        error"
#[test]
fn t_rows_86_87_progressive_row_depth_checks() {
    // row 86: the user transform is installed *after* png_read_update_info
    // has sized the row buffer, so the transformed depth exceeds
    // png_struct::maximum_pixel_depth on the very first row.
    diff("row86: progressive row overflow", |l| unsafe {
        N_INFO.with(|c| c.set(0));
        N_ROW.with(|c| c.set(0));
        N_END.with(|c| c.set(0));
        TF_AFTER_ROW.with(|c| c.set(0));
        UPD.with(|c| c.set(*sym::<FnUpdateInfo>(l, "png_read_update_info") as usize));
        SUTF.with(|c| c.set(*sym::<FnSetUserTf>(l, "png_set_read_user_transform_fn") as usize));
        let (pp, ip) = prog_reader(l);
        // Declare a 16 x 4 == 64 bit transformed pixel depth but do not set
        // PNG_USER_TRANSFORM yet.
        png_set_user_transform_info(l, pp, std::ptr::null_mut(), 16, 4);
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        // Feed the signature, IHDR and the 8-byte IDAT header only.  That is
        // exactly enough for info_cb (and hence png_read_update_info, which
        // sizes the row buffer for an 8-bit pixel depth) to run.
        let hdr = 8 + 25 + 8;
        png_process_data(l, pp, ip, s.as_mut_ptr(), hdr);
        assert_eq!(N_INFO.with(|c| c.get()), 1);
        png_set_read_user_transform_fn(l, pp, Some(user_tf_cb));
        let rest = s.len() - hdr;
        png_process_data(l, pp, ip, s[hdr..].as_mut_ptr(), rest); // row 86
        counters()
    });

    // row 87: the first row fixes transformed_pixel_depth at 8, then the row
    // callback installs a user transform declaring 8 x 2 == 16 bits.
    diff("row87: transformed depth changes mid-image", |l| unsafe {
        prog_reset(l, true, 1); // install the transform after row 1
        let (pp, ip) = prog_reader(l);
        png_set_user_transform_info(l, pp, std::ptr::null_mut(), 8, 2);
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 4, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(4))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        png_process_data(l, pp, ip, s.as_mut_ptr(), s.len()); // row 87
        counters()
    });
}

// ===========================================================================
// generic boundaries
// ===========================================================================

#[test]
fn t_generic_process_data_boundaries() {
    // png_process_data with size 0
    diff("png_process_data size 0", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        let mut b = [0u8; 1];
        png_process_data(l, pp, ip, b.as_mut_ptr(), 0);
        counters()
    });

    // png_process_data with a NULL buffer and size 0
    diff("png_process_data NULL buffer", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        png_process_data(l, pp, ip, std::ptr::null_mut(), 0);
        counters()
    });

    // png_process_data called again after the end of the stream
    diff("png_process_data after the end", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        let mut s = tiny_png(1);
        png_process_data(l, pp, ip, s.as_mut_ptr(), s.len());
        let mut extra = [1u8, 2, 3, 4, 5, 6, 7, 8];
        png_process_data(l, pp, ip, extra.as_mut_ptr(), extra.len());
        png_process_data(l, pp, ip, extra.as_mut_ptr(), 0);
        counters()
    });

    // png_process_data_pause with and without saving
    diff("png_process_data_pause", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        let mut s = tiny_png(1);
        png_process_data(l, pp, ip, s.as_mut_ptr(), 12);
        let a = png_process_data_pause(l, pp, 1);
        let b = png_process_data_pause(l, pp, 0);
        (a as u32, b as u32, counters())
    });

    // png_process_data_skip on a struct that has consumed nothing
    diff("png_process_data_skip with nothing to skip", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, _ip) = prog_reader(l);
        png_set_benign_errors(l, pp, 1);
        png_process_data_skip(l, pp)
    });

    // png_read_row with both row and display_row NULL
    diff("png_read_row(NULL, NULL)", |l| unsafe {
        src_set(&tiny_png(2));
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        png_read_row(l, pp, std::ptr::null_mut(), std::ptr::null_mut());
        png_read_row(l, pp, std::ptr::null_mut(), std::ptr::null_mut());
        (png_get_current_row_number(l, pp), src_pos() as u32)
    });

    // png_read_rows with both arrays NULL
    diff("png_read_rows(NULL, NULL)", |l| unsafe {
        src_set(&tiny_png(2));
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        png_read_rows(l, pp, std::ptr::null_mut(), std::ptr::null_mut(), 2);
        (png_get_current_row_number(l, pp), src_pos() as u32)
    });
}

#[test]
fn t_generic_chunk_limit_boundaries() {
    for n in [0usize, 1, 2, 65535, 65536, usize::MAX] {
        diff(&format!("png_set_chunk_malloc_max {n}"), move |l| unsafe {
            let pp = new_reader(l);
            png_set_chunk_malloc_max(l, pp, n);
            png_get_chunk_malloc_max(l, pp)
        });
    }
    for n in [0u32, 1, 2, 3, 1000, 0xffff_ffff] {
        diff(&format!("png_set_chunk_cache_max {n}"), move |l| unsafe {
            let pp = new_reader(l);
            png_set_chunk_cache_max(l, pp, n);
            png_get_chunk_cache_max(l, pp)
        });
    }

    // A chunk exactly at, and one past, the chunk cache limit.
    for (cache, count) in [
        (0u32, 3usize),
        (1, 1),
        (2, 1),
        (2, 2),
        (3, 2),
        (3, 3),
        (4, 3),
    ] {
        diff(
            &format!("chunk cache max={cache} with {count} unknown chunks"),
            move |l| unsafe {
                let mut s = SIG.to_vec();
                s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
                for i in 0..count {
                    let name = [b'u', b'N', b'K', b'a' + i as u8];
                    s.extend_from_slice(&ck(&name, b"x"));
                }
                s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
                s.extend_from_slice(&ck(b"IEND", &[]));
                src_set(&s);
                let pp = new_reader(l);
                png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
                png_set_chunk_cache_max(l, pp, cache);
                let ip = png_create_info_struct(l, pp);
                png_read_info(l, pp, ip);
                let mut u: *mut png_unknown_chunk = std::ptr::null_mut();
                (
                    png_get_unknown_chunks(l, pp, ip, &mut u),
                    png_get_chunk_cache_max(l, pp),
                    src_pos() as u32,
                )
            },
        );
    }
}

/// Sites in the assigned range that this build cannot reach; the assertions
/// here pin down what happens instead, identically in both libraries.
///
/// * row 162 (`pngrio.c:81`) and row 288 (`pngwio.c:102`) are the string
///   `"Error msg"` inside the *doc comments* of `png_set_read_fn` /
///   `png_set_write_fn`; there is no such call site in the code, and
///   `png_default_flush` has no error path at all.
/// * row 193 (`pngrutil.c:427`) is the `png_chunk_warning` half of
///   `png_inflate_claim`'s `#if PNG_RELEASE_BUILD` / `#else`.  This build has
///   `PNG_LIBPNG_BUILD_BASE_TYPE == PNG_LIBPNG_BUILD_BETA`, so
///   `PNG_RELEASE_BUILD == 0` and the compiled branch is row 194.
/// * rows 217 and 218 sit inside `#ifdef PNG_MAX_MALLOC_64K`, which
///   `pnglibconf.h` does not define.
#[test]
fn t_unreachable_sites_documented() {
    // row 288: png_default_flush is installed by png_set_write_fn(.., None)
    // and by png_init_io; it can only return normally.
    diff("row288: png_default_flush has no error path", |l| unsafe {
        sink_reset();
        let pp = new_writer(l);
        png_set_write_fn(l, pp, 1usize as png_voidp, Some(write_cb), None);
        png_write_flush(l, pp);
        png_set_flush(l, pp, 1);
        png_write_flush(l, pp);
        sink_take().1 as u32
    });

    // row 75 (save_buffer overflow) needs save_buffer_size close to
    // PNG_SIZE_MAX; row 77 ("save_buffer error") needs save_buffer == NULL
    // with save_buffer_size != 0.  Feeding a long stream one byte at a time
    // exercises the surrounding code without reaching either.
    diff("rows75/77: save_buffer growth", |l| unsafe {
        prog_reset(l, true, 0);
        let (pp, ip) = prog_reader(l);
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"tEXt", &vec![b'k'; 1000]));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IEND", &[]));
        for b in s {
            let mut one = [b];
            png_process_data(l, pp, ip, one.as_mut_ptr(), 1);
        }
        counters()
    });

    // row 79 ("No IDAT data (internal error)") is guarded by its callers;
    // the closest reachable state is a zero-length IDAT.
    diff("row79: zero-length IDAT (progressive)", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(1, 1, 8, 0, 0)));
        s.extend_from_slice(&ck(b"IDAT", &[]));
        s.extend_from_slice(&ck(b"IDAT", &zl(&gray_rows(1))));
        s.extend_from_slice(&ck(b"IDAT", &[]));
        s.extend_from_slice(&ck(b"IEND", &[]));
        feed(l, &s, true)
    });

    // rows 217/218 (PNG_MAX_MALLOC_64K) and row 219 (rowbytes > SIZE_MAX-1)
    // are compiled out / impossible on a 64-bit host.  A very wide row still
    // has to behave identically.
    diff("rows217-219: very wide row", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(100_000, 1, 16, 6, 0)));
        s.extend_from_slice(&ck(b"IDAT", &zl_open(&[0u8; 16])));
        s.extend_from_slice(&ck(b"IEND", &[]));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let ok = png_get_rowbytes(l, pp, ip);
        png_start_read_image(l, pp);
        ok as u64
    });

    // row 200 ("sPLT chunk too long") needs data_length/entry_size to exceed
    // PNG_SIZE_MAX/sizeof(png_sPLT_entry), impossible for a <2^31 chunk.
    diff("row200: largest practical sPLT", |l| unsafe {
        let mut payload = b"s\0".to_vec();
        payload.push(8);
        payload.extend(std::iter::repeat(0u8).take(6 * 1000));
        let s = splt_stream(&payload);
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        let mut e: *mut png_sPLT_t = std::ptr::null_mut();
        let n = png_get_sPLT(l, pp, ip, &mut e);
        let nent = if e.is_null() { -1 } else { (*e).nentries };
        (n, nent)
    });

    // row 205 ("no unknown chunk support available") is compiled out because
    // this build has PNG_SAVE_UNKNOWN_CHUNKS_SUPPORTED.
    diff("row205: keep > NEVER with save support", |l| unsafe {
        let pp = new_reader(l);
        png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
        png_handle_as_unknown(l, pp, b"uNKn\0".as_ptr())
    });

    // row 211 ("internal row width error") needs png_struct::width == 0,
    // which png_set_IHDR rejects.
    diff("row211: IHDR with width 0", |l| unsafe {
        let mut s = SIG.to_vec();
        s.extend_from_slice(&ck(b"IHDR", &ihdr(0, 1, 8, 0, 0)));
        src_set(&s);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_read_info(l, pp, ip);
        0u32
    });
}

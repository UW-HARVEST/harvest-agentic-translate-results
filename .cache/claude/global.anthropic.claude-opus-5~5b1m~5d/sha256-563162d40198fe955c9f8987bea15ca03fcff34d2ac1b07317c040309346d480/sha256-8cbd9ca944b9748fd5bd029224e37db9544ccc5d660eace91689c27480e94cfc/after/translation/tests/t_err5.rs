//! Differential verification of the **error paths** of `pngwrite.c` and
//! `pngwutil.c` (assignment E5, `ERRORS.md` rows 290..392).
//!
//! Every assertion drives BOTH the reference C `libpng.so` and the translated
//! Rust `liblibpng.so` through `dlsym` with byte-identical inputs and requires
//! * the identical sequence of recorded warning/error message bytes,
//! * the identical "did it longjmp out" outcome,
//! * the identical bytes handed to the write callback (and flush count),
//! * the identical return value.
//!
//! Build/run:  `cargo test --offline --release --test t_err5`
//!
//! Notes on the configuration of this build (they decide which rows are live
//! code at all):
//!   * `PNG_LIBPNG_BUILD_BASE_TYPE == PNG_LIBPNG_BUILD_BETA` (2) so
//!     `PNG_RELEASE_BUILD == 0`.  Therefore `png_create_write_struct` does NOT
//!     set `PNG_FLAG_APP_WARNINGS_WARN`, so on a *write* struct
//!     `png_app_warning`, `png_app_error` and `png_benign_error` are all FATAL
//!     (`png_error`).  It also means the `#if PNG_RELEASE_BUILD` half of
//!     `png_deflate_claim` is compiled out.
//!   * every `PNG_WRITE_*_SUPPORTED` transform macro is defined, so all the
//!     `#else  png_warning("... is not defined")` arms are dead code.
//!
//! ## Rows of `E5.md` that are NOT reachable in this configuration
//!
//! * 293, 294, 295 (`png_write_info`) and 298, 299, 300 (`png_write_end`):
//!   the `#else` arms of `#ifdef PNG_WRITE_{iTXt,zTXt,tEXt}_SUPPORTED`; all
//!   three macros are defined, so these `png_warning`s are not compiled.
//! * 302..=308 (`png_write_row`): guarded by
//!   `#if !defined(PNG_WRITE_<X>_SUPPORTED)` for INVERT / FILLER / PACKSWAP /
//!   PACK / SHIFT / BGR / SWAP -- all of which *are* defined.
//! * 311 (`png_set_filter`): the `#else` arm of
//!   `#ifdef PNG_WRITE_FILTER_SUPPORTED`, which is defined.
//! * 321, 322, 323, 324, 326, 327, 328, 329, 330 (`png_write_png`): the
//!   `#else` arms of the corresponding `PNG_WRITE_*_SUPPORTED` guards.
//!   (Row 325, which lives *inside* `#ifdef PNG_WRITE_FILLER_SUPPORTED`, IS
//!   covered -- see `t_rows320_325_write_png`.)
//! * 331 (`png_write_image_16bit`): the `png_error` fires when the format has
//!   no ALPHA flag, but `png_image_write_main` only dispatches to this function
//!   when `linear && !convert_to_8bit && alpha`, i.e. when ALPHA *is* set.
//!   Dead code.
//! * 337 (`image_memory_write`): needs `size > SIZE_MAX - output_bytes`, i.e. a
//!   PNG whose byte count is within `size` of 2^64.
//! * 339 (`png_deflate_claim`): inside `#if PNG_RELEASE_BUILD`, which is 0 in
//!   this build; the `#else` half (row 340) is what runs.
//! * 341 (`png_deflate_claim`, "deflateEnd failed (ignored)"): requires
//!   `deflateEnd` to return non-`Z_OK`, which zlib only does for a stream in
//!   `BUSY_STATE`.  libpng always drives the stream to `Z_STREAM_END` (or
//!   abandons the struct) before a parameter change can trigger `deflateEnd`,
//!   and while `zowner == png_IDAT` the function returns early.  Only reachable
//!   by re-entering libpng from a custom `malloc_fn` during `png_text_compress`.
//! * 356 (`png_compress_IDAT`, "Z_OK on Z_FINISH with output space"): needs
//!   `deflate` to report `Z_OK` on `Z_FINISH` with all input consumed *and*
//!   output space left; the `avail_out == 0` case is handled by the `continue`
//!   above it, so this is an internal-consistency check only.
//! * 363 (`png_write_iCCP`, "Profile length does not match profile"): the
//!   predicate `profile_len != png_get_uint_32(profile)` is *identical* to the
//!   one on row 361, which already png_error()ed.  Dead code.
//! * 379 (`png_write_tEXt`, "tEXt: text too long") and 386 (`png_write_iTXt`,
//!   "iTXt: uncompressed text too long"): both need a NUL-terminated string
//!   longer than `PNG_UINT_31_MAX` (> 2 GiB), which is not practical to
//!   allocate in a unit test.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod common;

use common::api::*;
use common::*;
use libloading::Library;
use std::cell::Cell;
use std::ffi::{c_void, CString};

// ---------------------------------------------------------------------------
// Local API declarations.
//
// `tests/common/api.rs` declares `png_write_iCCP` and `png_write_zTXt` with
// signatures that do not match the C ABI (iCCP takes a 4th `profile_len`
// argument, zTXt takes `(key, text, compression)` and no `len`).  Re-declare
// them here rather than editing the shared file.
// ---------------------------------------------------------------------------
mod loc {
    use super::common::*;
    use core::ffi::c_int;

    crate::decl_api! {
        fn png_write_iCCP(pp: png_structp, name: png_const_charp,
                          profile: png_const_bytep, proflen: png_uint_32);
        fn png_write_zTXt(pp: png_structp, key: png_const_charp,
                          text: png_const_charp, compression: c_int);
    }
}

// ---------------------------------------------------------------------------
// differential driver
// ---------------------------------------------------------------------------

fn fmt(log: &[Msg]) -> Vec<String> {
    log.iter().map(|m| m.to_string()).collect()
}

#[derive(Debug)]
struct Res<T> {
    out: Option<T>,
    log: Vec<Msg>,
    data: Vec<u8>,
    flushes: usize,
}

fn run_one<T, F: Fn(&'static Library) -> T>(l: &'static Library, f: &F) -> Res<T> {
    common::sink_reset();
    let r = common::capture(|| f(l));
    let (data, flushes) = common::sink_take();
    Res { out: r.out, log: r.log, data, flushes }
}

/// Run `f` against the C library and the Rust library and require identical
/// observable behaviour.  Returns the C-side observation so that the caller can
/// additionally pin down *which* rejection site was reached.
fn diff<T, F>(label: &str, f: F) -> Res<T>
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&'static Library) -> T,
{
    let ls = libs();
    let a = run_one(&ls.c, &f);
    let b = run_one(&ls.rs, &f);
    assert_eq!(fmt(&a.log), fmt(&b.log), "{label}: message log differs (C first)");
    assert_eq!(
        a.out.is_none(),
        b.out.is_none(),
        "{label}: C errored={} but Rust errored={}",
        a.out.is_none(),
        b.out.is_none()
    );
    assert_eq!(a.out, b.out, "{label}: return value differs (C first)");
    assert_eq!(hex(&a.data), hex(&b.data), "{label}: emitted bytes differ (C first)");
    assert_eq!(a.flushes, b.flushes, "{label}: flush count differs");
    a
}

/// The call must have been aborted by a fatal error whose *last* message is
/// exactly `msg`.
fn exp_err<T: std::fmt::Debug>(r: &Res<T>, label: &str, msg: &[u8]) {
    assert!(
        r.out.is_none(),
        "{label}: expected a FATAL error, but the call returned; log = {:?}",
        fmt(&r.log)
    );
    assert_eq!(
        r.log.last().map(|m| m.to_string()),
        Some(Msg::Err(msg.to_vec()).to_string()),
        "{label}: wrong final message; full log = {:?}",
        fmt(&r.log)
    );
}

/// The call must have returned normally and have emitted `msg` as a warning.
fn exp_warn<T: std::fmt::Debug>(r: &Res<T>, label: &str, msg: &[u8]) {
    assert!(
        r.out.is_some(),
        "{label}: expected a non-fatal warning, but the call errored out; log = {:?}",
        fmt(&r.log)
    );
    let want = Msg::Warn(msg.to_vec()).to_string();
    assert!(
        fmt(&r.log).contains(&want),
        "{label}: expected warning {want}; full log = {:?}",
        fmt(&r.log)
    );
}

/// The call must have returned normally with an empty message log.
fn exp_clean<T: std::fmt::Debug>(r: &Res<T>, label: &str) {
    assert!(
        r.out.is_some(),
        "{label}: expected success, but the call errored out; log = {:?}",
        fmt(&r.log)
    );
    assert!(r.log.is_empty(), "{label}: expected no messages, got {:?}", fmt(&r.log));
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

unsafe fn wr(l: &Library) -> (png_structp, png_infop) {
    let pp = new_writer(l);
    let ip = png_create_info_struct(l, pp);
    assert!(!ip.is_null(), "png_create_info_struct returned NULL");
    (pp, ip)
}

const NUL: png_const_charp = std::ptr::null();

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// A minimal but *valid* ICC profile blob for the iCCP tests: `len` bytes whose
/// first 4 bytes are the big-endian length and whose byte 8 (the "major
/// version") is `major`.
fn icc(len: u32, major: u8) -> Vec<u8> {
    let mut v = vec![0u8; len as usize];
    v[0] = (len >> 24) as u8;
    v[1] = (len >> 16) as u8;
    v[2] = (len >> 8) as u8;
    v[3] = len as u8;
    v[8] = major;
    v
}

unsafe extern "C-unwind" fn bad_pixel_depth(
    _pp: png_structp,
    ri: *mut png_row_info,
    _row: png_bytep,
) {
    // Lie about the transformed pixel depth; png_write_row must catch this.
    (*ri).pixel_depth = 7;
}

// ===========================================================================
// pngwrite.c
// ===========================================================================

// row 290
#[test]
fn t_row290_zero_length_unknown_chunk() {
    let r = diff("row 290", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 1, 1, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        // 'meTa': ancillary, safe-to-copy (name[3] has 0x20 set) so
        // write_unknown_chunks will actually emit it.
        let unk = png_unknown_chunk {
            name: [b'm', b'e', b'T', b'a', 0],
            data: std::ptr::null_mut(),
            size: 0,
            location: PNG_HAVE_IHDR as png_byte,
        };
        png_set_unknown_chunks(l, pp, ip, &unk, 1);
        png_write_info(l, pp, ip);
    });
    // row 290
    exp_warn(&r, "row 290", b"Writing zero-length unknown chunk");
}

// row 291
#[test]
fn t_row291_mng_features_not_allowed_in_png() {
    let r = diff("row 291", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 1, 1, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        let got = png_permit_mng_features(l, pp, PNG_ALL_MNG_FEATURES as png_uint_32);
        png_write_info_before_PLTE(l, pp, ip);
        got
    });
    // row 291
    exp_warn(&r, "row 291", b"MNG features are not allowed in a PNG datastream");
    assert_eq!(r.out, Some(PNG_ALL_MNG_FEATURES as png_uint_32));
}

// row 292
#[test]
fn t_row292_valid_palette_required() {
    let r = diff("row 292", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
    });
    // row 292
    exp_err(&r, "row 292", b"Valid palette required for paletted images");
}

// rows 296, 297
#[test]
fn t_rows296_297_write_end() {
    // row 296
    let r = diff("row 296 (end before any IDAT)", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
        png_write_end(l, pp, ip);
    });
    exp_err(&r, "row 296", b"No IDATs written into file");

    // row 296 again: png_write_end straight after creation (no info at all)
    let r = diff("row 296 (end on a virgin struct)", |l| unsafe {
        let (pp, ip) = wr(l);
        png_write_end(l, pp, ip);
    });
    exp_err(&r, "row 296/virgin", b"No IDATs written into file");

    // row 297: a palette image whose IDAT references index 1 while the PLTE
    // only has 1 entry.  png_benign_error is fatal on a write struct here.
    let r = diff("row 297", |l| unsafe {
        let (pp, ip) = wr(l);
        let pal = [png_color { red: 1, green: 2, blue: 3 }];
        png_set_IHDR(
            l, pp, ip, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_set_PLTE(l, pp, ip, pal.as_ptr(), 1);
        png_write_info(l, pp, ip);
        let row = [1u8];
        png_write_row(l, pp, row.as_ptr());
        png_write_end(l, pp, ip);
    });
    exp_err(&r, "row 297", b"Wrote palette index exceeding num_palette");
}

// row 301
#[test]
fn t_row301_row_before_info() {
    // row 301
    let r = diff("row 301", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        let row = [0u8; 4];
        png_write_row(l, pp, row.as_ptr());
    });
    exp_err(&r, "row 301", b"png_write_info was never called before png_write_row");

    // row 301 again, reached through png_write_rows
    let r = diff("row 301 via png_write_rows", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        let mut buf = [[0u8; 4]; 4];
        let mut rows: [png_bytep; 4] = std::array::from_fn(|i| buf[i].as_mut_ptr());
        png_write_rows(l, pp, rows.as_mut_ptr(), 4);
    });
    exp_err(&r, "row 301/rows", b"png_write_info was never called before png_write_row");

    // png_write_image, by contrast, loops over png_ptr->height, which is still
    // 0 because png_write_IHDR has not run: it writes nothing and says nothing.
    let r = diff("png_write_image before info", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        let mut buf = [[0u8; 4]; 4];
        let mut rows: [png_bytep; 4] = std::array::from_fn(|i| buf[i].as_mut_ptr());
        png_write_image(l, pp, rows.as_mut_ptr());
    });
    exp_clean(&r, "png_write_image before info");
    assert_eq!(r.data.len(), 0);

    // ...and on a READ struct, which likewise never wrote the info.
    let r = diff("row 301 on a read struct", |l| unsafe {
        src_set(&[]);
        let pp = new_reader(l);
        let row = [0u8; 4];
        png_write_row(l, pp, row.as_ptr());
    });
    exp_err(&r, "row 301/read struct", b"png_write_info was never called before png_write_row");
}

// row 309
#[test]
fn t_row309_internal_write_transform_logic_error() {
    let r = diff("row 309", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_set_write_user_transform_fn(l, pp, Some(bad_pixel_depth));
        png_write_info(l, pp, ip);
        let row = [0u8; 4];
        png_write_row(l, pp, row.as_ptr());
    });
    // row 309
    exp_err(&r, "row 309", b"internal write transform logic error");
}

// rows 310, 312, 313
#[test]
fn t_rows310_312_313_set_filter() {
    // row 310: `filters & 0xff` lands on case 5/6/7.
    for f in [5i32, 6, 7] {
        let r = diff(&format!("row 310 filters={f}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_filter(l, pp, PNG_FILTER_TYPE_BASE, f);
        });
        // row 310
        exp_err(&r, &format!("row 310 filters={f}"), b"Unknown row filter for method 0");
    }
    // ...and the *legal* neighbours must stay silent.
    for f in [0i32, 1, 2, 3, 4, -1, 0x08, 0xF8, 0x7fff_ffff] {
        let r = diff(&format!("png_set_filter legal filters={f}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_filter(l, pp, PNG_FILTER_TYPE_BASE, f);
        });
        exp_clean(&r, &format!("png_set_filter filters={f}"));
    }

    // row 312: adding a prev_row filter once writing has started.
    let r = diff("row 312", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_set_filter(l, pp, PNG_FILTER_TYPE_BASE, PNG_FILTER_NONE);
        png_write_info(l, pp, ip);
        let row = [0u8; 4];
        png_write_row(l, pp, row.as_ptr());
        png_set_filter(l, pp, PNG_FILTER_TYPE_BASE, PNG_FILTER_UP);
    });
    // row 312 (png_app_warning is fatal in this build)
    exp_err(&r, "row 312", b"png_set_filter: UP/AVG/PAETH cannot be added after start");

    // row 313: any method other than PNG_FILTER_TYPE_BASE.
    for m in [1i32, 2, 64, 99, -1, 0x7fff_ffff] {
        let r = diff(&format!("row 313 method={m}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_filter(l, pp, m, PNG_ALL_FILTERS);
        });
        // row 313
        exp_err(&r, &format!("row 313 method={m}"), b"Unknown custom filter method");
    }

    // With MNG intrapixel differencing permitted, method 64 is silently
    // rewritten to 0 and must NOT reach row 313.
    let r = diff("png_set_filter method=64 with MNG", |l| unsafe {
        let pp = new_writer(l);
        png_permit_mng_features(l, pp, PNG_ALL_MNG_FEATURES as png_uint_32);
        png_set_filter(l, pp, PNG_INTRAPIXEL_DIFFERENCING, PNG_ALL_FILTERS);
    });
    exp_clean(&r, "png_set_filter method=64 with MNG");
}

// deprecated no-op heuristics APIs
#[test]
fn t_set_filter_heuristics_are_noops() {
    let r = diff("png_set_filter_heuristics", |l| unsafe {
        let pp = new_writer(l);
        let mut w = [1.0f64, 2.0, 3.0];
        let mut c = [1.0f64, 2.0, 3.0];
        png_set_filter_heuristics(l, pp, 99, 3, w.as_mut_ptr(), c.as_mut_ptr());
        png_set_filter_heuristics(l, pp, -1, 0, std::ptr::null_mut(), std::ptr::null_mut());
        let mut wf = [1i32, 2, 3];
        let mut cf = [1i32, 2, 3];
        png_set_filter_heuristics_fixed(l, pp, 99, 3, wf.as_mut_ptr(), cf.as_mut_ptr());
        png_set_filter_heuristics_fixed(l, pp, 0x7fff_ffff, -1, std::ptr::null_mut(),
                                        std::ptr::null_mut());
        // and with a NULL png_ptr
        png_set_filter_heuristics(l, std::ptr::null_mut(), 0, 0, std::ptr::null_mut(),
                                  std::ptr::null_mut());
        png_set_filter_heuristics_fixed(l, std::ptr::null_mut(), 0, 0,
                                        std::ptr::null_mut(), std::ptr::null_mut());
    });
    exp_clean(&r, "png_set_filter_heuristics");
}

// rows 314, 315, 316, 317, 318, 319
#[test]
fn t_rows314_319_compression_setters() {
    // rows 314 / 315
    for (b, msg) in [
        (16i32, &b"Only compression windows <= 32k supported by PNG"[..]),
        (99, b"Only compression windows <= 32k supported by PNG"),
        (0x7fff_ffff, b"Only compression windows <= 32k supported by PNG"),
        (7, b"Only compression windows >= 256 supported by PNG"),
        (0, b"Only compression windows >= 256 supported by PNG"),
        (-1, b"Only compression windows >= 256 supported by PNG"),
    ] {
        let r = diff(&format!("rows 314/315 window_bits={b}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_compression_window_bits(l, pp, b);
        });
        // row 314 (b > 15) / row 315 (b < 8)
        exp_warn(&r, &format!("rows 314/315 window_bits={b}"), msg);
        assert_eq!(r.log.len(), 1);
    }
    for b in [8i32, 9, 15] {
        let r = diff(&format!("window_bits={b} legal"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_compression_window_bits(l, pp, b);
        });
        exp_clean(&r, &format!("window_bits={b}"));
    }

    // row 316
    for m in [0i32, 7, 9, 99, -1, 0x7fff_ffff] {
        let r = diff(&format!("row 316 method={m}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_compression_method(l, pp, m);
        });
        // row 316
        exp_warn(&r, &format!("row 316 method={m}"),
                 b"Only compression method 8 is supported by PNG");
    }
    let r = diff("method=8 legal", |l| unsafe {
        let pp = new_writer(l);
        png_set_compression_method(l, pp, 8);
    });
    exp_clean(&r, "method=8");

    // rows 317 / 318
    for (b, msg) in [
        (16i32, &b"Only compression windows <= 32k supported by PNG"[..]),
        (99, b"Only compression windows <= 32k supported by PNG"),
        (7, b"Only compression windows >= 256 supported by PNG"),
        (-1, b"Only compression windows >= 256 supported by PNG"),
    ] {
        let r = diff(&format!("rows 317/318 text window_bits={b}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_text_compression_window_bits(l, pp, b);
        });
        // row 317 (b > 15) / row 318 (b < 8)
        exp_warn(&r, &format!("rows 317/318 text window_bits={b}"), msg);
    }

    // row 319
    for m in [0i32, 9, 99, -1] {
        let r = diff(&format!("row 319 text method={m}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_text_compression_method(l, pp, m);
        });
        // row 319
        exp_warn(&r, &format!("row 319 text method={m}"),
                 b"Only compression method 8 is supported by PNG");
    }

    // The un-validated setters must stay silent for every value.
    let r = diff("unvalidated zlib setters", |l| unsafe {
        let pp = new_writer(l);
        for v in [-2i32, -1, 0, 5, 10, 99, 0x7fff_ffff] {
            png_set_compression_level(l, pp, v);
            png_set_compression_mem_level(l, pp, v);
            png_set_compression_strategy(l, pp, v);
            png_set_text_compression_level(l, pp, v);
            png_set_text_compression_mem_level(l, pp, v);
            png_set_text_compression_strategy(l, pp, v);
        }
    });
    exp_clean(&r, "unvalidated zlib setters");
}

// png_set_compression_buffer_size boundaries
#[test]
fn t_compression_buffer_size_boundaries() {
    let r = diff("buffer size 0", |l| unsafe {
        let pp = new_writer(l);
        png_set_compression_buffer_size(l, pp, 0);
        png_get_compression_buffer_size(l, pp)
    });
    exp_err(&r, "buffer size 0", b"invalid compression buffer size");

    let r = diff("buffer size usize::MAX", |l| unsafe {
        let pp = new_writer(l);
        png_set_compression_buffer_size(l, pp, usize::MAX);
        png_get_compression_buffer_size(l, pp)
    });
    exp_err(&r, "buffer size usize::MAX", b"invalid compression buffer size");

    let r = diff("buffer size 0x80000000", |l| unsafe {
        let pp = new_writer(l);
        png_set_compression_buffer_size(l, pp, 0x8000_0000);
        png_get_compression_buffer_size(l, pp)
    });
    exp_err(&r, "buffer size 0x80000000", b"invalid compression buffer size");

    for n in [1usize, 2, 5] {
        let r = diff(&format!("buffer size {n}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_compression_buffer_size(l, pp, n);
            png_get_compression_buffer_size(l, pp)
        });
        exp_warn(&r, &format!("buffer size {n}"),
                 b"Compression buffer size cannot be reduced below 6");
        assert_eq!(r.out, Some(8192), "buffer size must be unchanged");
    }

    let r = diff("buffer size 6", |l| unsafe {
        let pp = new_writer(l);
        png_set_compression_buffer_size(l, pp, 6);
        png_get_compression_buffer_size(l, pp)
    });
    exp_clean(&r, "buffer size 6");
    assert_eq!(r.out, Some(6));

    // in use by IDAT -> refused with a warning
    let r = diff("buffer size while zstream in use", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
        let row = [0u8; 4];
        png_write_row(l, pp, row.as_ptr());
        png_set_compression_buffer_size(l, pp, 4096);
        png_get_compression_buffer_size(l, pp)
    });
    exp_warn(&r, "buffer size while zstream in use",
             b"Compression buffer size cannot be changed because it is in use");
    assert_eq!(r.out, Some(8192));
}

// rows 320, 325
#[test]
fn t_rows320_325_write_png() {
    // row 320
    let r = diff("row 320", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_png(l, pp, ip, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut());
    });
    // row 320
    exp_err(&r, "row 320", b"no rows for png_write_image to write");

    // row 320 again with a pile of transform bits set: the IDAT check comes
    // first, so the message must be identical.
    let r = diff("row 320 with transforms", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_png(l, pp, ip, -1, std::ptr::null_mut());
    });
    exp_err(&r, "row 320/transforms", b"no rows for png_write_image to write");

    // row 325: STRIP_FILLER_BEFORE and STRIP_FILLER_AFTER together.
    let r = diff("row 325", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        // GA input data, 2 bytes per pixel
        let mut buf = [[0u8; 4]; 2];
        let mut rows: [png_bytep; 2] = std::array::from_fn(|i| buf[i].as_mut_ptr());
        png_set_rows(l, pp, ip, rows.as_mut_ptr());
        png_write_png(
            l, pp, ip,
            PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER,
            std::ptr::null_mut(),
        );
    });
    // row 325
    exp_err(&r, "row 325", b"PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported");
}

// rows 332, 333, 334, 335, 336 -- the simplified writer.
//
// NOTE: the simplified API installs png_safe_error/png_safe_warning, so these
// diagnostics never reach the harness recorder; they surface as a 0 return with
// `png_image::warning_or_error` / `png_image::message` set.  That whole tuple is
// compared here.
#[test]
fn t_rows332_336_image_write_main() {
    // row 332: height * row_stride overflows 32 bits.
    let r = diff("row 332", |l| unsafe {
        let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
        img.width = 65536;
        img.height = 16384;
        img.format = PNG_FORMAT_RGBA;
        let dummy = [0u8; 16];
        let mut n: usize = 0;
        let ret = png_image_write_to_memory(
            l, &mut img, std::ptr::null_mut(), &mut n, 0,
            dummy.as_ptr() as *const c_void, 0, std::ptr::null(),
        );
        (ret, img.cmp_key(), n)
    });
    // row 332
    let (ret, key, _) = r.out.clone().unwrap();
    assert_eq!(ret, 0, "row 332: expected failure");
    assert_eq!(key.7, "memory image too large", "row 332: message");

    // row 333: row_stride smaller than one row.
    for stride in [1i32, 15, -1, -15] {
        let r = diff(&format!("row 333 stride={stride}"), move |l| unsafe {
            let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
            img.width = 4;
            img.height = 1;
            img.format = PNG_FORMAT_RGBA;
            let dummy = [0u8; 64];
            let mut n: usize = 0;
            let ret = png_image_write_to_memory(
                l, &mut img, std::ptr::null_mut(), &mut n, 0,
                dummy.as_ptr() as *const c_void, stride, std::ptr::null(),
            );
            (ret, img.cmp_key(), n)
        });
        // row 333
        let (ret, key, _) = r.out.clone().unwrap();
        assert_eq!(ret, 0, "row 333 stride={stride}");
        assert_eq!(key.7, "supplied row stride too small", "row 333 stride={stride}");
    }

    // row 334: width * channels overflows 31 bits.
    let r = diff("row 334", |l| unsafe {
        let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
        img.width = 0x2000_0000;
        img.height = 1;
        img.format = PNG_FORMAT_RGBA;
        let dummy = [0u8; 16];
        let mut n: usize = 0;
        let ret = png_image_write_to_memory(
            l, &mut img, std::ptr::null_mut(), &mut n, 0,
            dummy.as_ptr() as *const c_void, 0, std::ptr::null(),
        );
        (ret, img.cmp_key(), n)
    });
    // row 334
    let (ret, key, _) = r.out.clone().unwrap();
    assert_eq!(ret, 0, "row 334");
    assert_eq!(key.7, "image row stride too large", "row 334: message");

    // row 335: a colour-mapped format with no colour map.
    for (cmap_entries, cmap_null) in [(0u32, false), (4, true), (0, true)] {
        let label = format!("row 335 entries={cmap_entries} null={cmap_null}");
        let r = diff(&label, move |l| unsafe {
            let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
            img.width = 1;
            img.height = 1;
            img.format = PNG_FORMAT_RGB_COLORMAP;
            img.colormap_entries = cmap_entries;
            let dummy = [0u8; 16];
            let cmap = [0u8; 1024];
            let mut n: usize = 0;
            let ret = png_image_write_to_memory(
                l, &mut img, std::ptr::null_mut(), &mut n, 0,
                dummy.as_ptr() as *const c_void, 0,
                if cmap_null { std::ptr::null() } else { cmap.as_ptr() as *const c_void },
            );
            (ret, img.cmp_key(), n)
        });
        // row 335
        let (ret, key, _) = r.out.clone().unwrap();
        assert_eq!(ret, 0, "{label}");
        assert_eq!(key.7, "no color-map for color-mapped image", "{label}");
    }

    // row 336: a format bit that no transform can account for.
    for f in [0x40u32, 0x80, 0x4000_0000] {
        let label = format!("row 336 format=0x{f:x}");
        let r = diff(&label, move |l| unsafe {
            let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
            img.width = 1;
            img.height = 1;
            img.format = f;
            let dummy = [0u8; 16];
            let mut n: usize = 0;
            let ret = png_image_write_to_memory(
                l, &mut img, std::ptr::null_mut(), &mut n, 0,
                dummy.as_ptr() as *const c_void, 0, std::ptr::null(),
            );
            (ret, img.cmp_key(), n)
        });
        // row 336
        let (ret, key, _) = r.out.clone().unwrap();
        assert_eq!(ret, 0, "{label}");
        assert_eq!(key.7, "png_write_image: unsupported transformation", "{label}");
    }
}

// Generic simplified-writer boundaries (no specific E5 row, but required by the
// assignment): NULL image / buffer / memory_bytes, bad version, zero height,
// a `memory_bytes` that is too small.
#[test]
fn t_image_write_to_memory_boundaries() {
    let r = diff("write_to_memory NULL image", |l| unsafe {
        png_image_write_to_memory(
            l, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 0,
            std::ptr::null(), 0, std::ptr::null(),
        )
    });
    exp_clean(&r, "write_to_memory NULL image");
    assert_eq!(r.out, Some(0));

    let r = diff("write_to_memory NULL buffer", |l| unsafe {
        let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
        img.width = 1;
        img.height = 1;
        img.format = PNG_FORMAT_GRAY;
        let mut n: usize = 0;
        let ret = png_image_write_to_memory(
            l, &mut img, std::ptr::null_mut(), &mut n, 0, std::ptr::null(), 0,
            std::ptr::null(),
        );
        (ret, img.cmp_key(), n)
    });
    let (ret, key, _) = r.out.clone().unwrap();
    assert_eq!(ret, 0);
    assert_eq!(key.7, "png_image_write_to_memory: invalid argument");

    let r = diff("write_to_memory NULL memory_bytes", |l| unsafe {
        let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
        img.width = 1;
        img.height = 1;
        let dummy = [0u8; 4];
        let ret = png_image_write_to_memory(
            l, &mut img, std::ptr::null_mut(), std::ptr::null_mut(), 0,
            dummy.as_ptr() as *const c_void, 0, std::ptr::null(),
        );
        (ret, img.cmp_key())
    });
    let (ret, key) = r.out.clone().unwrap();
    assert_eq!(ret, 0);
    assert_eq!(key.7, "png_image_write_to_memory: invalid argument");

    let r = diff("write_to_memory bad version", |l| unsafe {
        let mut img = png_image { version: 99, ..Default::default() };
        img.width = 1;
        img.height = 1;
        let dummy = [0u8; 4];
        let mut n: usize = 0;
        let ret = png_image_write_to_memory(
            l, &mut img, std::ptr::null_mut(), &mut n, 0,
            dummy.as_ptr() as *const c_void, 0, std::ptr::null(),
        );
        (ret, img.cmp_key(), n)
    });
    let (ret, key, _) = r.out.clone().unwrap();
    assert_eq!(ret, 0);
    assert_eq!(key.7, "png_image_write_to_memory: incorrect PNG_IMAGE_VERSION");

    // height == 0 -> png_set_IHDR rejects it.
    // (width == 0 is deliberately NOT tested: png_image_write_main divides by
    // `png_row_stride`, which is 0 in that case -- a division by zero in the C
    // reference, i.e. undefined behaviour, not a rejection site.)
    let r = diff("write_to_memory height=0", |l| unsafe {
        let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
        img.width = 1;
        img.height = 0;
        img.format = PNG_FORMAT_GRAY;
        let dummy = [0u8; 4];
        let mut n: usize = 0;
        let ret = png_image_write_to_memory(
            l, &mut img, std::ptr::null_mut(), &mut n, 0,
            dummy.as_ptr() as *const c_void, 0, std::ptr::null(),
        );
        (ret, img.cmp_key(), n)
    });
    let (ret, _key, _) = r.out.clone().unwrap();
    assert_eq!(ret, 0, "height=0 must be rejected");

    // memory_bytes too small: returns 0 but reports the real size.
    let r = diff("write_to_memory buffer too small", |l| unsafe {
        let mut img = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
        img.width = 4;
        img.height = 4;
        img.format = PNG_FORMAT_GRAY;
        let src = [0x5au8; 16];
        let mut mem = [0u8; 10];
        let mut n: usize = mem.len();
        let ret = png_image_write_to_memory(
            l, &mut img, mem.as_mut_ptr() as *mut c_void, &mut n, 0,
            src.as_ptr() as *const c_void, 0, std::ptr::null(),
        );
        (ret, img.cmp_key(), n, mem.to_vec())
    });
    let (ret, _k, n, _mem) = r.out.clone().unwrap();
    assert_eq!(ret, 0, "an undersized buffer must fail");
    assert!(n > 10, "the real size must be reported back, got {n}");
}

// ===========================================================================
// pngwutil.c
// ===========================================================================

// row 338
#[test]
fn t_row338_chunk_length() {
    for len in [0x8000_0000usize, usize::MAX] {
        let r = diff(&format!("row 338 len={len:#x}"), move |l| unsafe {
            let pp = new_writer(l);
            let data = [0u8; 16];
            png_write_chunk(l, pp, b"teSt".as_ptr(), data.as_ptr(), len);
        });
        // row 338
        exp_err(&r, &format!("row 338 len={len:#x}"), b"length exceeds PNG maximum");
    }

    // len 0 (and a NULL data pointer) is legal and writes an empty chunk.
    let r = diff("png_write_chunk len=0", |l| unsafe {
        let pp = new_writer(l);
        png_write_chunk(l, pp, b"teSt".as_ptr(), std::ptr::null(), 0);
    });
    exp_clean(&r, "png_write_chunk len=0");
    assert_eq!(r.data.len(), 12, "8-byte header + 4-byte CRC");

    // Exactly PNG_UINT_31_MAX is accepted by the length check (the write
    // callback then just receives the caller's buffer, so use a real one).
    let r = diff("png_write_chunk_start huge length", |l| unsafe {
        let pp = new_writer(l);
        // png_write_chunk_start does NOT range-check; it just emits the header.
        png_write_chunk_start(l, pp, b"teSt".as_ptr(), 0x8000_0000);
        png_write_chunk_data(l, pp, std::ptr::null(), 0);
        png_write_chunk_data(l, pp, b"ab".as_ptr(), 2);
        png_write_chunk_end(l, pp);
    });
    exp_clean(&r, "png_write_chunk_start huge length");

    // png_write_chunk_end with no matching start: just emits the current CRC.
    let r = diff("png_write_chunk_end unmatched", |l| unsafe {
        let pp = new_writer(l);
        png_write_chunk_end(l, pp);
        png_write_chunk_end(l, pp);
    });
    exp_clean(&r, "png_write_chunk_end unmatched");
    assert_eq!(r.data.len(), 8);

    // Every non-alphabetic "chunk name" is accepted verbatim.
    for name in [&b"\x00\x00\x00\x00"[..], b"1234", b"    ", b"\xff\xff\xff\xff"] {
        let nm = name.to_vec();
        let r = diff("png_write_chunk odd name", move |l| unsafe {
            let pp = new_writer(l);
            png_write_chunk(l, pp, nm.as_ptr(), b"x".as_ptr(), 1);
        });
        exp_clean(&r, "png_write_chunk odd name");
    }
}

// row 340 (and row 357, row 382)
#[test]
fn t_row340_zstream_in_use_by_idat() {
    let r = diff("row 340", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
        let row = [0x11u8, 0x22, 0x33, 0x44];
        png_write_row(l, pp, row.as_ptr()); // claims the zstream for IDAT
        let key = cstr("Comment");
        let txt = cstr("hello");
        loc::png_write_zTXt(l, pp, key.as_ptr(), txt.as_ptr(), PNG_TEXT_COMPRESSION_zTXt);
    });
    // row 340 -- PNG_RELEASE_BUILD == 0, so png_deflate_claim png_error()s here
    exp_err(&r, "row 340", b"zTXt: IDAT using zstream");
}

// row 357
#[test]
fn t_row357_compress_idat_zlib_error() {
    // Two consecutive Z_SYNC_FLUSHes with no new input make zlib return
    // Z_BUF_ERROR, which png_compress_IDAT turns into a fatal "truncated".
    let r = diff("row 357", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
        let row = [0x11u8, 0x22, 0x33, 0x44];
        png_write_row(l, pp, row.as_ptr());
        png_write_flush(l, pp);
        png_write_flush(l, pp);
    });
    // row 357 -- zlib itself sets zstream.msg, so png_zstream_error keeps it.
    exp_err(&r, "row 357", b"buffer error");
}

// row 355
#[test]
fn t_row355_deflate_claim_failure_for_idat() {
    // Every zlib configuration that deflateInit2() rejects.  (window_bits
    // cannot be made invalid: png_set_compression_window_bits clamps it.)
    let cases: [(&str, fn(&Library, png_structp)); 8] = [
        ("level=10", |l, pp| unsafe { png_set_compression_level(l, pp, 10) }),
        ("level=-2", |l, pp| unsafe { png_set_compression_level(l, pp, -2) }),
        ("level=99", |l, pp| unsafe { png_set_compression_level(l, pp, 99) }),
        ("method=9", |l, pp| unsafe { png_set_compression_method(l, pp, 9) }),
        ("mem_level=0", |l, pp| unsafe { png_set_compression_mem_level(l, pp, 0) }),
        ("mem_level=10", |l, pp| unsafe { png_set_compression_mem_level(l, pp, 10) }),
        ("strategy=5", |l, pp| unsafe { png_set_compression_strategy(l, pp, 5) }),
        ("strategy=-1", |l, pp| unsafe { png_set_compression_strategy(l, pp, -1) }),
    ];
    for (name, setup) in cases {
        let r = diff(&format!("row 355 {name}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            png_set_IHDR(
                l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            );
            setup(l, pp);
            png_write_info(l, pp, ip);
            let row = [0x11u8, 0x22, 0x33, 0x44];
            png_write_row(l, pp, row.as_ptr());
        });
        // row 355 (method=9 also emits the row-316 warning first)
        exp_err(&r, &format!("row 355 {name}"), b"bad parameters to zlib");
    }
}

// row 342
//
// png_write_compressed_data_out only reports "error writing ancillary chunked
// compressed data" when the buffer chain it has to walk is shorter than the
// compressed data it must emit.  The one way an application can bring that
// about is to free the chain from inside its own write callback (which libpng
// calls from png_write_chunk_header, after the compression has already run).
#[test]
fn t_row342_compressed_data_out_short_chain() {
    thread_local! {
        static ARMED: Cell<u32> = const { Cell::new(0) };
        static LIB: Cell<usize> = const { Cell::new(0) };
    }

    unsafe extern "C-unwind" fn wcb(pp: png_structp, data: png_bytep, len: usize) {
        common::write_cb(pp, data, len);
        if ARMED.with(|a| a.get()) == 1 {
            ARMED.with(|a| a.set(2));
            let l = &*(LIB.with(|c| c.get()) as *const Library);
            // Frees png_ptr->zbuffer_list (the size differs from the default).
            png_set_compression_buffer_size(l, pp, 4096);
        }
    }

    let r = diff("row 342", |l| unsafe {
        LIB.with(|c| c.set(l as *const Library as usize));
        ARMED.with(|a| a.set(0));
        let pp = new_writer(l);
        png_set_write_fn(l, pp, 1usize as png_voidp, Some(wcb), Some(common::flush_cb));

        // Incompressible text so that the deflate output overflows the 1024-byte
        // `comp.output` block and spills into png_ptr->zbuffer_list.
        let mut rng = Rng::new(0xE5E5_0342);
        let mut txt: Vec<u8> = (0..8000).map(|_| rng.u8() | 1).collect();
        txt.push(0);
        let key = cstr("Comment");
        ARMED.with(|a| a.set(1));
        loc::png_write_zTXt(
            l, pp, key.as_ptr(), txt.as_ptr() as png_const_charp,
            PNG_TEXT_COMPRESSION_zTXt,
        );
    });
    // row 342
    exp_err(&r, "row 342", b"error writing ancillary chunked compressed data");
}

// rows 343, 344, 345, 346, 347, 348, 349, 350, 351
#[test]
fn t_rows343_351_write_IHDR() {
    // row 343: grayscale accepts 1/2/4/8/16 only.
    for bd in [0i32, 3, 5, 6, 7, 9, 12, 15, 17, 32, 64, -1, 99, 0x7fff_ffff] {
        let r = diff(&format!("row 343 gray bd={bd}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        });
        // row 343
        exp_err(&r, &format!("row 343 gray bd={bd}"), b"Invalid bit depth for grayscale image");
    }
    for bd in [1i32, 2, 4, 8, 16] {
        let r = diff(&format!("gray bd={bd} legal"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        });
        exp_clean(&r, &format!("gray bd={bd}"));
    }

    // row 344: RGB accepts 8/16 only.
    for bd in [0i32, 1, 2, 4, 12, 32, -1, 99] {
        let r = diff(&format!("row 344 rgb bd={bd}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        });
        // row 344
        exp_err(&r, &format!("row 344 rgb bd={bd}"), b"Invalid bit depth for RGB image");
    }

    // row 345: palette accepts 1/2/4/8 only (16 is rejected).
    for bd in [0i32, 3, 16, 32, -1, 99] {
        let r = diff(&format!("row 345 palette bd={bd}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        });
        // row 345
        exp_err(&r, &format!("row 345 palette bd={bd}"),
                b"Invalid bit depth for paletted image");
    }

    // row 346: gray+alpha accepts 8/16 only.
    for bd in [0i32, 1, 2, 4, 12, -1, 99] {
        let r = diff(&format!("row 346 ga bd={bd}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_GRAY_ALPHA, 0, 0, 0);
        });
        // row 346
        exp_err(&r, &format!("row 346 ga bd={bd}"),
                b"Invalid bit depth for grayscale+alpha image");
    }

    // row 347: RGBA accepts 8/16 only.
    for bd in [0i32, 1, 2, 4, 12, -1, 99] {
        let r = diff(&format!("row 347 rgba bd={bd}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_RGB_ALPHA, 0, 0, 0);
        });
        // row 347
        exp_err(&r, &format!("row 347 rgba bd={bd}"), b"Invalid bit depth for RGBA image");
    }

    // row 348: unknown colour types.
    for ct in [1i32, 5, 7, 8, 99, -1, 0x7fff_ffff] {
        let r = diff(&format!("row 348 ct={ct}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, ct, 0, 0, 0);
        });
        // row 348
        exp_err(&r, &format!("row 348 ct={ct}"), b"Invalid image color type specified");
    }

    // row 349: any compression_type other than 0 warns and is forced to 0.
    for cm in [1i32, 8, 99, -1, 0x7fff_ffff] {
        let r = diff(&format!("row 349 cm={cm}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY, cm, 0, 0);
        });
        // row 349
        exp_warn(&r, &format!("row 349 cm={cm}"), b"Invalid compression type specified");
        assert_eq!(r.data[8 + 10], 0, "compression_type must be forced to 0");
    }

    // row 350: filter_type must be 0 (64 is only allowed for MNG-embedded
    // RGB/RGBA before the signature has been written).
    for ft in [1i32, 2, 64, 99, -1, 0x7fff_ffff] {
        let r = diff(&format!("row 350 ft={ft}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY, 0, ft, 0);
        });
        // row 350
        exp_warn(&r, &format!("row 350 ft={ft}"), b"Invalid filter type specified");
        assert_eq!(r.data[8 + 11], 0, "filter_type must be forced to 0");
    }
    // ...and the MNG escape hatch must not warn.
    let r = diff("filter_type=64 with MNG on RGB", |l| unsafe {
        let pp = new_writer(l);
        png_permit_mng_features(l, pp, PNG_ALL_MNG_FEATURES as png_uint_32);
        png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB, 0,
                       PNG_INTRAPIXEL_DIFFERENCING, 0);
    });
    exp_clean(&r, "filter_type=64 with MNG on RGB");
    assert_eq!(r.data[8 + 11], 64);
    // but not for grayscale
    let r = diff("filter_type=64 with MNG on gray", |l| unsafe {
        let pp = new_writer(l);
        png_permit_mng_features(l, pp, PNG_ALL_MNG_FEATURES as png_uint_32);
        png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY, 0,
                       PNG_INTRAPIXEL_DIFFERENCING, 0);
    });
    exp_warn(&r, "filter_type=64 with MNG on gray", b"Invalid filter type specified");

    // row 351: interlace_type must be 0 or 1; anything else warns and becomes
    // ADAM7.
    for il in [2i32, 3, 99, -1, 0x7fff_ffff] {
        let r = diff(&format!("row 351 il={il}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY, 0, 0, il);
        });
        // row 351
        exp_warn(&r, &format!("row 351 il={il}"), b"Invalid interlace type specified");
        assert_eq!(r.data[8 + 12], 1, "interlace_type must be forced to ADAM7");
    }

    // Widths/heights are NOT validated by png_write_IHDR; the exact 13 IHDR
    // data bytes must nevertheless match.
    for (w, h) in [(0u32, 0u32), (0, 1), (1, 0), (PNG_UINT_31_MAX, PNG_UINT_31_MAX),
                   (0x8000_0000, 0x8000_0000), (0xffff_ffff, 0xffff_ffff)] {
        let r = diff(&format!("IHDR w={w} h={h}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, w, h, 16, PNG_COLOR_TYPE_RGB_ALPHA, 0, 0, 0);
        });
        exp_clean(&r, &format!("IHDR w={w} h={h}"));
        assert_eq!(r.data.len(), 25);
    }
}

// rows 352, 353, 354
#[test]
fn t_rows352_354_write_PLTE() {
    let pal: Vec<png_color> = (0..300u32)
        .map(|i| png_color { red: i as u8, green: (i >> 1) as u8, blue: (i >> 2) as u8 })
        .collect();

    // row 352: paletted image, bad palette size -> fatal.
    for (bd, n) in [(8i32, 0u32), (8, 257), (8, 0xffff_ffff), (1, 3), (2, 5), (4, 17)] {
        let p = pal.clone();
        let r = diff(&format!("row 352 bd={bd} n={n}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
            png_write_PLTE(l, pp, p.as_ptr(), n);
        });
        // row 352
        exp_err(&r, &format!("row 352 bd={bd} n={n}"),
                b"Invalid number of colors in palette");
    }

    // row 353: non-paletted image, bad palette size -> warning + no chunk.
    for (ct, n) in [(PNG_COLOR_TYPE_RGB, 0u32), (PNG_COLOR_TYPE_RGB, 257),
                    (PNG_COLOR_TYPE_RGB_ALPHA, 300), (PNG_COLOR_TYPE_GRAY, 0)] {
        let p = pal.clone();
        let r = diff(&format!("row 353 ct={ct} n={n}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, ct, 0, 0, 0);
            let before = sink_len();
            png_write_PLTE(l, pp, p.as_ptr(), n);
            sink_len() - before
        });
        // row 353
        exp_warn(&r, &format!("row 353 ct={ct} n={n}"),
                 b"Invalid number of colors in palette");
        assert_eq!(r.out, Some(0), "no PLTE chunk must be written");
    }

    // row 354: a legal palette size on a grayscale image -> warning, no chunk.
    for ct in [PNG_COLOR_TYPE_GRAY, PNG_COLOR_TYPE_GRAY_ALPHA] {
        let p = pal.clone();
        let r = diff(&format!("row 354 ct={ct}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, ct, 0, 0, 0);
            let before = sink_len();
            png_write_PLTE(l, pp, p.as_ptr(), 4);
            sink_len() - before
        });
        // row 354
        exp_warn(&r, &format!("row 354 ct={ct}"),
                 b"Ignoring request to write a PLTE chunk in grayscale PNG");
        assert_eq!(r.out, Some(0));
    }

    // A zero-length palette IS legal when MNG empty-PLTE is permitted.
    let p = pal.clone();
    let r = diff("PLTE n=0 with MNG empty PLTE", move |l| unsafe {
        let pp = new_writer(l);
        png_permit_mng_features(l, pp, PNG_FLAG_MNG_EMPTY_PLTE as png_uint_32);
        png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        png_write_PLTE(l, pp, p.as_ptr(), 0);
    });
    exp_clean(&r, "PLTE n=0 with MNG empty PLTE");

    // Writing PLTE twice is not diagnosed; both chunks must be emitted
    // identically by both libraries.
    let p = pal.clone();
    let r = diff("PLTE twice", move |l| unsafe {
        let pp = new_writer(l);
        png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        png_write_PLTE(l, pp, p.as_ptr(), 3);
        png_write_PLTE(l, pp, p.as_ptr(), 5);
    });
    exp_clean(&r, "PLTE twice");
}

// row 358
#[test]
fn t_row358_write_sRGB() {
    for intent in [4i32, 5, 99, 0x7fff_ffff] {
        let r = diff(&format!("row 358 intent={intent}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_sRGB(l, pp, intent);
        });
        // row 358
        exp_warn(&r, &format!("row 358 intent={intent}"),
                 b"Invalid sRGB rendering intent specified");
        assert_eq!(r.data.len(), 13, "the chunk is still written");
    }
    // Negative intents slip past the `>=` test unnoticed (C behaviour).
    for intent in [0i32, 1, 2, 3, -1, -99] {
        let r = diff(&format!("sRGB intent={intent}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_sRGB(l, pp, intent);
        });
        exp_clean(&r, &format!("sRGB intent={intent}"));
    }
}

// rows 359, 360, 361, 362, 364, 365
#[test]
fn t_rows359_365_write_iCCP() {
    // row 359
    let r = diff("row 359", |l| unsafe {
        let pp = new_writer(l);
        let n = cstr("ICC");
        loc::png_write_iCCP(l, pp, n.as_ptr(), std::ptr::null(), 132);
    });
    exp_err(&r, "row 359", b"No profile for iCCP chunk");

    // row 360
    for len in [0u32, 1, 4, 131] {
        let r = diff(&format!("row 360 len={len}"), move |l| unsafe {
            let pp = new_writer(l);
            let n = cstr("ICC");
            let prof = vec![0u8; 200];
            loc::png_write_iCCP(l, pp, n.as_ptr(), prof.as_ptr(), len);
        });
        // row 360
        exp_err(&r, &format!("row 360 len={len}"), b"ICC profile too short");
    }

    // row 361: the embedded length disagrees with profile_len.
    for (embedded, len) in [(999u32, 132u32), (0, 132), (132, 200), (0xffff_ffff, 132)] {
        let r = diff(&format!("row 361 emb={embedded} len={len}"), move |l| unsafe {
            let pp = new_writer(l);
            let n = cstr("ICC");
            let mut prof = vec![0u8; 400];
            prof[0] = (embedded >> 24) as u8;
            prof[1] = (embedded >> 16) as u8;
            prof[2] = (embedded >> 8) as u8;
            prof[3] = embedded as u8;
            loc::png_write_iCCP(l, pp, n.as_ptr(), prof.as_ptr(), len);
        });
        // row 361 (row 363 is the *same* predicate and is therefore dead code)
        exp_err(&r, &format!("row 361 emb={embedded} len={len}"),
                b"Incorrect data in iCCP");
    }

    // row 362: profile version > 3 and a length that is not a multiple of 4.
    for len in [133u32, 134, 135] {
        let r = diff(&format!("row 362 len={len}"), move |l| unsafe {
            let pp = new_writer(l);
            let n = cstr("ICC");
            let mut prof = icc(len, 4);
            prof.resize(400, 0);
            loc::png_write_iCCP(l, pp, n.as_ptr(), prof.as_ptr(), len);
        });
        // row 362
        exp_err(&r, &format!("row 362 len={len}"),
                b"ICC profile length invalid (not a multiple of 4)");
    }
    // major <= 3 makes the same length acceptable
    let r = diff("iCCP len=133 major=3", |l| unsafe {
        let pp = new_writer(l);
        let n = cstr("ICC");
        let mut prof = icc(133, 3);
        prof.resize(400, 0);
        loc::png_write_iCCP(l, pp, n.as_ptr(), prof.as_ptr(), 133);
    });
    exp_clean(&r, "iCCP len=133 major=3");

    // row 364: an unusable keyword.
    for key in ["", " ", "\x01\x02", "\x7f", "  \t  "] {
        let k = key.to_string();
        let r = diff(&format!("row 364 key={key:?}"), move |l| unsafe {
            let pp = new_writer(l);
            let n = cstr(&k);
            let prof = icc(132, 0);
            loc::png_write_iCCP(l, pp, n.as_ptr(), prof.as_ptr(), 132);
        });
        // row 364
        exp_err(&r, &format!("row 364 key={key:?}"), b"iCCP: invalid keyword");
    }
    // ...and a NULL keyword
    let r = diff("row 364 key=NULL", |l| unsafe {
        let pp = new_writer(l);
        let prof = icc(132, 0);
        loc::png_write_iCCP(l, pp, NUL, prof.as_ptr(), 132);
    });
    // row 364
    exp_err(&r, "row 364 key=NULL", b"iCCP: invalid keyword");

    // row 365: the text compressor cannot be initialised.
    let r = diff("row 365", |l| unsafe {
        let pp = new_writer(l);
        let n = cstr("ICC");
        png_set_text_compression_mem_level(l, pp, 0);
        let prof = icc(132, 0);
        loc::png_write_iCCP(l, pp, n.as_ptr(), prof.as_ptr(), 132);
    });
    // row 365
    exp_err(&r, "row 365", b"bad parameters to zlib");
}

// row 366
#[test]
fn t_row366_write_sPLT() {
    for key in ["", " ", "\x01"] {
        let k = key.to_string();
        let r = diff(&format!("row 366 key={key:?}"), move |l| unsafe {
            let pp = new_writer(l);
            let n = cstr(&k);
            let sp = png_sPLT_t {
                name: n.as_ptr() as png_charp,
                depth: 8,
                entries: std::ptr::null_mut(),
                nentries: 0,
            };
            png_write_sPLT(l, pp, &sp);
        });
        // row 366
        exp_err(&r, &format!("row 366 key={key:?}"), b"sPLT: invalid keyword");
    }
    let r = diff("row 366 key=NULL", |l| unsafe {
        let pp = new_writer(l);
        let sp = png_sPLT_t {
            name: std::ptr::null_mut(),
            depth: 16,
            entries: std::ptr::null_mut(),
            nentries: 0,
        };
        png_write_sPLT(l, pp, &sp);
    });
    // row 366
    exp_err(&r, "row 366 key=NULL", b"sPLT: invalid keyword");
}

// rows 367, 368, 369
#[test]
fn t_rows367_369_write_sBIT() {
    // row 367: colour images check red/green/blue against usr_bit_depth.
    let bad_rgb = [
        png_color_8 { red: 0, green: 8, blue: 8, gray: 0, alpha: 0 },
        png_color_8 { red: 8, green: 0, blue: 8, gray: 0, alpha: 0 },
        png_color_8 { red: 8, green: 8, blue: 0, gray: 0, alpha: 0 },
        png_color_8 { red: 9, green: 8, blue: 8, gray: 0, alpha: 0 },
        png_color_8 { red: 8, green: 255, blue: 8, gray: 0, alpha: 0 },
    ];
    for (i, sb) in bad_rgb.into_iter().enumerate() {
        let r = diff(&format!("row 367 #{i}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            let before = sink_len();
            png_write_sBIT(l, pp, &sb, PNG_COLOR_TYPE_RGB);
            sink_len() - before
        });
        // row 367
        exp_warn(&r, &format!("row 367 #{i}"), b"Invalid sBIT depth specified");
        assert_eq!(r.out, Some(0));
    }
    // Palette images clamp maxbits to 8 regardless of the bit depth.
    let r = diff("sBIT palette maxbits=8", |l| unsafe {
        let pp = new_writer(l);
        png_write_IHDR(l, pp, 1, 1, 1, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        let sb = png_color_8 { red: 8, green: 8, blue: 8, gray: 0, alpha: 0 };
        png_write_sBIT(l, pp, &sb, PNG_COLOR_TYPE_PALETTE);
    });
    exp_clean(&r, "sBIT palette maxbits=8");
    let r = diff("row 367 palette maxbits=8 exceeded", |l| unsafe {
        let pp = new_writer(l);
        png_write_IHDR(l, pp, 1, 1, 1, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        let sb = png_color_8 { red: 9, green: 8, blue: 8, gray: 0, alpha: 0 };
        png_write_sBIT(l, pp, &sb, PNG_COLOR_TYPE_PALETTE);
    });
    // row 367
    exp_warn(&r, "row 367 palette", b"Invalid sBIT depth specified");

    // row 368: grayscale checks `gray`.
    for g in [0u8, 9, 17, 255] {
        let r = diff(&format!("row 368 gray={g}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            let sb = png_color_8 { red: 0, green: 0, blue: 0, gray: g, alpha: 0 };
            let before = sink_len();
            png_write_sBIT(l, pp, &sb, PNG_COLOR_TYPE_GRAY);
            sink_len() - before
        });
        // row 368
        exp_warn(&r, &format!("row 368 gray={g}"), b"Invalid sBIT depth specified");
        assert_eq!(r.out, Some(0));
    }

    // row 369: the alpha channel is checked last.
    for a in [0u8, 9, 255] {
        let r = diff(&format!("row 369 ga alpha={a}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY_ALPHA, 0, 0, 0);
            let sb = png_color_8 { red: 0, green: 0, blue: 0, gray: 8, alpha: a };
            let before = sink_len();
            png_write_sBIT(l, pp, &sb, PNG_COLOR_TYPE_GRAY_ALPHA);
            sink_len() - before
        });
        // row 369
        exp_warn(&r, &format!("row 369 ga alpha={a}"), b"Invalid sBIT depth specified");
        assert_eq!(r.out, Some(0));
    }
    for a in [0u8, 9] {
        let r = diff(&format!("row 369 rgba alpha={a}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB_ALPHA, 0, 0, 0);
            let sb = png_color_8 { red: 8, green: 8, blue: 8, gray: 0, alpha: a };
            png_write_sBIT(l, pp, &sb, PNG_COLOR_TYPE_RGB_ALPHA);
        });
        // row 369
        exp_warn(&r, &format!("row 369 rgba alpha={a}"), b"Invalid sBIT depth specified");
    }
}

// rows 370, 371, 372, 373
#[test]
fn t_rows370_373_write_tRNS() {
    // row 370: paletted, num_trans out of range (num_palette is still 0 here).
    for n in [0i32, -1, 1, 2, 257, 0x7fff_ffff] {
        let r = diff(&format!("row 370 n={n}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
            let trans = [0u8; 300];
            let c = png_color_16::default();
            png_write_tRNS(l, pp, trans.as_ptr(), &c, n, PNG_COLOR_TYPE_PALETTE);
        });
        // row 370
        exp_err(&r, &format!("row 370 n={n}"),
                b"Invalid number of transparent colors specified");
    }
    // With a 2-entry PLTE, 1 and 2 become legal but 3 does not.
    let r = diff("row 370 n=3 with 2-entry PLTE", |l| unsafe {
        let pp = new_writer(l);
        png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        let pal = [png_color::default(); 2];
        png_write_PLTE(l, pp, pal.as_ptr(), 2);
        let trans = [0u8; 4];
        let c = png_color_16::default();
        png_write_tRNS(l, pp, trans.as_ptr(), &c, 3, PNG_COLOR_TYPE_PALETTE);
    });
    // row 370
    exp_err(&r, "row 370 n=3", b"Invalid number of transparent colors specified");

    // row 371: grayscale, tran->gray out of range for the bit depth.
    for (bd, gray) in [(8i32, 256u16), (8, 65535), (1, 2), (2, 4), (4, 16), (16, 0)] {
        if bd == 16 {
            continue; // 1<<16 overflows nothing: any u16 gray is in range
        }
        let r = diff(&format!("row 371 bd={bd} gray={gray}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            let mut c = png_color_16::default();
            c.gray = gray;
            png_write_tRNS(l, pp, std::ptr::null(), &c, 1, PNG_COLOR_TYPE_GRAY);
        });
        // row 371
        exp_err(&r, &format!("row 371 bd={bd} gray={gray}"),
                b"Ignoring attempt to write tRNS chunk out-of-range for bit_depth");
    }

    // row 372: RGB with 8-bit depth but 16-bit tRNS values.
    for (rd, gr, bl) in [(0x1234u16, 0u16, 0u16), (0, 0x0100, 0), (0, 0, 0xff00),
                         (0x0100, 0x0100, 0x0100)] {
        let r = diff(&format!("row 372 {rd:#x},{gr:#x},{bl:#x}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            let mut c = png_color_16::default();
            c.red = rd;
            c.green = gr;
            c.blue = bl;
            png_write_tRNS(l, pp, std::ptr::null(), &c, 1, PNG_COLOR_TYPE_RGB);
        });
        // row 372
        exp_err(&r, &format!("row 372 {rd:#x}"),
                b"Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8");
    }
    // 8-bit-representable values are fine
    let r = diff("tRNS RGB 8-bit values", |l| unsafe {
        let pp = new_writer(l);
        png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        let mut c = png_color_16::default();
        c.red = 0x00ff;
        c.green = 0x0012;
        c.blue = 0x0034;
        png_write_tRNS(l, pp, std::ptr::null(), &c, 1, PNG_COLOR_TYPE_RGB);
    });
    exp_clean(&r, "tRNS RGB 8-bit values");

    // row 373: colour types that already have an alpha channel.
    for ct in [PNG_COLOR_TYPE_GRAY_ALPHA, PNG_COLOR_TYPE_RGB_ALPHA] {
        let r = diff(&format!("row 373 ct={ct}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, ct, 0, 0, 0);
            let c = png_color_16::default();
            png_write_tRNS(l, pp, std::ptr::null(), &c, 1, ct);
        });
        // row 373
        exp_err(&r, &format!("row 373 ct={ct}"), b"Can't write tRNS with an alpha channel");
    }
    // ...and for a colour type png_write_tRNS does not know at all.
    for ct in [1i32, 5, 7, 99, -1] {
        let r = diff(&format!("row 373 unknown ct={ct}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            let c = png_color_16::default();
            png_write_tRNS(l, pp, std::ptr::null(), &c, 1, ct);
        });
        // row 373
        exp_err(&r, &format!("row 373 unknown ct={ct}"),
                b"Can't write tRNS with an alpha channel");
    }
}

// rows 374, 375, 376
#[test]
fn t_rows374_376_write_bKGD() {
    // row 374: palette index beyond num_palette.
    for idx in [0u8, 1, 255] {
        let r = diff(&format!("row 374 index={idx}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
            let mut b = png_color_16::default();
            b.index = idx;
            let before = sink_len();
            png_write_bKGD(l, pp, &b, PNG_COLOR_TYPE_PALETTE);
            sink_len() - before
        });
        // row 374
        exp_warn(&r, &format!("row 374 index={idx}"), b"Invalid background palette index");
        assert_eq!(r.out, Some(0));
    }
    // Within a written PLTE it is accepted.
    let r = diff("bKGD index inside PLTE", |l| unsafe {
        let pp = new_writer(l);
        png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        let pal = [png_color::default(); 4];
        png_write_PLTE(l, pp, pal.as_ptr(), 4);
        let mut b = png_color_16::default();
        b.index = 3;
        png_write_bKGD(l, pp, &b, PNG_COLOR_TYPE_PALETTE);
    });
    exp_clean(&r, "bKGD index inside PLTE");

    // row 375: 16-bit values on an 8-bit colour image.
    for (rd, gr, bl) in [(0x1234u16, 0u16, 0u16), (0, 0x0100, 0), (0, 0, 0x0100)] {
        let r = diff(&format!("row 375 {rd:#x},{gr:#x},{bl:#x}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            let mut b = png_color_16::default();
            b.red = rd;
            b.green = gr;
            b.blue = bl;
            let before = sink_len();
            png_write_bKGD(l, pp, &b, PNG_COLOR_TYPE_RGB);
            sink_len() - before
        });
        // row 375
        exp_warn(&r, &format!("row 375 {rd:#x}"),
                 b"Ignoring attempt to write 16-bit bKGD chunk when bit_depth is 8");
        assert_eq!(r.out, Some(0));
    }

    // row 376: grayscale value out of range for the bit depth.
    for (bd, gray) in [(8i32, 256u16), (1, 2), (2, 4), (4, 16), (8, 65535)] {
        let r = diff(&format!("row 376 bd={bd} gray={gray}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, bd, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            let mut b = png_color_16::default();
            b.gray = gray;
            let before = sink_len();
            png_write_bKGD(l, pp, &b, PNG_COLOR_TYPE_GRAY);
            sink_len() - before
        });
        // row 376
        exp_warn(&r, &format!("row 376 bd={bd} gray={gray}"),
                 b"Ignoring attempt to write bKGD chunk out-of-range for bit_depth");
        assert_eq!(r.out, Some(0));
    }
}

// row 377
#[test]
fn t_row377_write_hIST() {
    for n in [1i32, 2, 257, 0x7fff] {
        let r = diff(&format!("row 377 n={n}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
            let hist = [0u16; 300];
            let before = sink_len();
            png_write_hIST(l, pp, hist.as_ptr(), n);
            sink_len() - before
        });
        // row 377
        exp_warn(&r, &format!("row 377 n={n}"),
                 b"Invalid number of histogram entries specified");
        assert_eq!(r.out, Some(0));
    }
    // n <= num_palette is accepted (n == 0 and num_palette == 0).
    let r = diff("hIST n=0", |l| unsafe {
        let pp = new_writer(l);
        png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        let hist = [0u16; 4];
        png_write_hIST(l, pp, hist.as_ptr(), 0);
    });
    exp_clean(&r, "hIST n=0");
}

// row 378
#[test]
fn t_row378_write_tEXt() {
    for key in ["", " ", "  ", "\x01", "\x7f"] {
        let k = key.to_string();
        let r = diff(&format!("row 378 key={key:?}"), move |l| unsafe {
            let pp = new_writer(l);
            let ks = cstr(&k);
            let t = cstr("value");
            png_write_tEXt(l, pp, ks.as_ptr(), t.as_ptr(), 0);
        });
        // row 378
        exp_err(&r, &format!("row 378 key={key:?}"), b"tEXt: invalid keyword");
    }
    let r = diff("row 378 key=NULL", |l| unsafe {
        let pp = new_writer(l);
        let t = cstr("value");
        png_write_tEXt(l, pp, NUL, t.as_ptr(), 0);
    });
    // row 378
    exp_err(&r, "row 378 key=NULL", b"tEXt: invalid keyword");

    // A NULL text pointer is legal: the chunk is written with no text.
    let r = diff("tEXt NULL text", |l| unsafe {
        let pp = new_writer(l);
        let k = cstr("Comment");
        png_write_tEXt(l, pp, k.as_ptr(), NUL, 12345);
    });
    exp_clean(&r, "tEXt NULL text");
    assert_eq!(r.data.len(), 8 + 8 + 4, "'Comment' + NUL");

    // An over-long keyword is truncated with a warning, not rejected.
    let r = diff("tEXt long keyword", |l| unsafe {
        let pp = new_writer(l);
        let k = cstr(&"K".repeat(200));
        let t = cstr("v");
        png_write_tEXt(l, pp, k.as_ptr(), t.as_ptr(), 0);
    });
    exp_warn(&r, "tEXt long keyword", b"keyword truncated");

    // An embedded bad character produces the formatted warning.
    let r = diff("tEXt bad char in keyword", |l| unsafe {
        let pp = new_writer(l);
        let k = cstr("Ke\x01y");
        let t = cstr("v");
        png_write_tEXt(l, pp, k.as_ptr(), t.as_ptr(), 0);
    });
    assert!(r.out.is_some());
    assert_eq!(r.log.len(), 1, "expected exactly one warning, got {:?}", fmt(&r.log));
}

// rows 380, 381, 382
#[test]
fn t_rows380_382_write_zTXt() {
    // row 380: any compression other than NONE(-1) / zTXt(0).
    for c in [1i32, 2, 3, 5, 99, -2, -3, 0x7fff_ffff] {
        let r = diff(&format!("row 380 compression={c}"), move |l| unsafe {
            let pp = new_writer(l);
            let k = cstr("Comment");
            let t = cstr("value");
            loc::png_write_zTXt(l, pp, k.as_ptr(), t.as_ptr(), c);
        });
        // row 380
        exp_err(&r, &format!("row 380 compression={c}"), b"zTXt: invalid compression type");
    }
    // compression == PNG_TEXT_COMPRESSION_NONE delegates to png_write_tEXt.
    let r = diff("zTXt compression=NONE -> tEXt", |l| unsafe {
        let pp = new_writer(l);
        let k = cstr("Comment");
        let t = cstr("value");
        loc::png_write_zTXt(l, pp, k.as_ptr(), t.as_ptr(), PNG_TEXT_COMPRESSION_NONE);
    });
    exp_clean(&r, "zTXt compression=NONE");
    assert_eq!(&r.data[4..8], b"tEXt");

    // row 381
    for key in ["", " ", "\x01"] {
        let k = key.to_string();
        let r = diff(&format!("row 381 key={key:?}"), move |l| unsafe {
            let pp = new_writer(l);
            let ks = cstr(&k);
            let t = cstr("value");
            loc::png_write_zTXt(l, pp, ks.as_ptr(), t.as_ptr(), PNG_TEXT_COMPRESSION_zTXt);
        });
        // row 381
        exp_err(&r, &format!("row 381 key={key:?}"), b"zTXt: invalid keyword");
    }
    let r = diff("row 381 key=NULL", |l| unsafe {
        let pp = new_writer(l);
        let t = cstr("value");
        loc::png_write_zTXt(l, pp, NUL, t.as_ptr(), PNG_TEXT_COMPRESSION_zTXt);
    });
    // row 381
    exp_err(&r, "row 381 key=NULL", b"zTXt: invalid keyword");

    // row 382: the compressor cannot be initialised.
    let setters: [(&str, fn(&Library, png_structp)); 5] = [
        ("text level=10", |l, pp| unsafe { png_set_text_compression_level(l, pp, 10) }),
        ("text level=-2", |l, pp| unsafe { png_set_text_compression_level(l, pp, -2) }),
        ("text method=9", |l, pp| unsafe { png_set_text_compression_method(l, pp, 9) }),
        ("text mem_level=0", |l, pp| unsafe { png_set_text_compression_mem_level(l, pp, 0) }),
        ("text strategy=5", |l, pp| unsafe { png_set_text_compression_strategy(l, pp, 5) }),
    ];
    for (name, setup) in setters {
        let r = diff(&format!("row 382 {name}"), move |l| unsafe {
            let pp = new_writer(l);
            setup(l, pp);
            let k = cstr("Comment");
            let t = cstr("value");
            loc::png_write_zTXt(l, pp, k.as_ptr(), t.as_ptr(), PNG_TEXT_COMPRESSION_zTXt);
        });
        // row 382
        exp_err(&r, &format!("row 382 {name}"), b"bad parameters to zlib");
    }

    // A NULL text is legal (an empty deflate stream is written).
    let r = diff("zTXt NULL text", |l| unsafe {
        let pp = new_writer(l);
        let k = cstr("Comment");
        loc::png_write_zTXt(l, pp, k.as_ptr(), NUL, PNG_TEXT_COMPRESSION_zTXt);
    });
    exp_clean(&r, "zTXt NULL text");
}

// rows 383, 384, 385
#[test]
fn t_rows383_385_write_iTXt() {
    // row 383
    for key in ["", " ", "\x01"] {
        let k = key.to_string();
        let r = diff(&format!("row 383 key={key:?}"), move |l| unsafe {
            let pp = new_writer(l);
            let ks = cstr(&k);
            let lang = cstr("en");
            let lk = cstr("Comment");
            let t = cstr("value");
            png_write_iTXt(l, pp, PNG_ITXT_COMPRESSION_NONE, ks.as_ptr(), lang.as_ptr(),
                           lk.as_ptr(), t.as_ptr());
        });
        // row 383
        exp_err(&r, &format!("row 383 key={key:?}"), b"iTXt: invalid keyword");
    }
    let r = diff("row 383 key=NULL", |l| unsafe {
        let pp = new_writer(l);
        png_write_iTXt(l, pp, PNG_ITXT_COMPRESSION_NONE, NUL, NUL, NUL, NUL);
    });
    // row 383
    exp_err(&r, "row 383 key=NULL", b"iTXt: invalid keyword");

    // row 384
    for c in [3i32, 4, 5, 99, -2, -3, 0x7fff_ffff] {
        let r = diff(&format!("row 384 compression={c}"), move |l| unsafe {
            let pp = new_writer(l);
            let k = cstr("Comment");
            png_write_iTXt(l, pp, c, k.as_ptr(), NUL, NUL, NUL);
        });
        // row 384
        exp_err(&r, &format!("row 384 compression={c}"), b"iTXt: invalid compression");
    }
    // the four legal codes
    for c in [PNG_TEXT_COMPRESSION_NONE, PNG_TEXT_COMPRESSION_zTXt,
              PNG_ITXT_COMPRESSION_NONE, PNG_ITXT_COMPRESSION_zTXt] {
        let r = diff(&format!("iTXt compression={c} legal"), move |l| unsafe {
            let pp = new_writer(l);
            let k = cstr("Comment");
            let t = cstr("value");
            png_write_iTXt(l, pp, c, k.as_ptr(), NUL, NUL, t.as_ptr());
        });
        exp_clean(&r, &format!("iTXt compression={c}"));
    }

    // row 385: the compressor cannot be initialised (only on the compressed
    // code paths).
    for c in [PNG_TEXT_COMPRESSION_zTXt, PNG_ITXT_COMPRESSION_zTXt] {
        let r = diff(&format!("row 385 compression={c}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_text_compression_mem_level(l, pp, 0);
            let k = cstr("Comment");
            let t = cstr("value");
            png_write_iTXt(l, pp, c, k.as_ptr(), NUL, NUL, t.as_ptr());
        });
        // row 385
        exp_err(&r, &format!("row 385 compression={c}"), b"bad parameters to zlib");
    }
    // ...and the uncompressed codes are unaffected by the broken zlib config.
    for c in [PNG_TEXT_COMPRESSION_NONE, PNG_ITXT_COMPRESSION_NONE] {
        let r = diff(&format!("iTXt uncompressed compression={c}"), move |l| unsafe {
            let pp = new_writer(l);
            png_set_text_compression_mem_level(l, pp, 0);
            let k = cstr("Comment");
            let t = cstr("value");
            png_write_iTXt(l, pp, c, k.as_ptr(), NUL, NUL, t.as_ptr());
        });
        exp_clean(&r, &format!("iTXt uncompressed compression={c}"));
    }
}

// row 387
#[test]
fn t_row387_write_oFFs() {
    for unit in [2i32, 3, 99, 0x7fff_ffff] {
        let r = diff(&format!("row 387 unit={unit}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_oFFs(l, pp, -1, 1, unit);
        });
        // row 387
        exp_warn(&r, &format!("row 387 unit={unit}"),
                 b"Unrecognized unit type for oFFs chunk");
        assert_eq!(r.data.len(), 9 + 12, "the chunk is still written");
    }
    for unit in [0i32, 1, -1, -99] {
        let r = diff(&format!("oFFs unit={unit}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_oFFs(l, pp, i32::MIN, i32::MAX, unit);
        });
        exp_clean(&r, &format!("oFFs unit={unit}"));
    }
}

// rows 388, 389
#[test]
fn t_rows388_389_write_pCAL() {
    // row 388
    for ty in [4i32, 5, 99, 0x7fff_ffff] {
        let r = diff(&format!("row 388 type={ty}"), move |l| unsafe {
            let pp = new_writer(l);
            let purpose = cstr("purpose");
            let units = cstr("units");
            png_write_pCAL(l, pp, purpose.as_ptr() as png_charp, 0, 100, ty, 0,
                           units.as_ptr(), std::ptr::null_mut());
        });
        // row 388
        exp_err(&r, &format!("row 388 type={ty}"),
                b"Unrecognized equation type for pCAL chunk");
    }

    // row 389
    for key in ["", " ", "\x01"] {
        let k = key.to_string();
        let r = diff(&format!("row 389 purpose={key:?}"), move |l| unsafe {
            let pp = new_writer(l);
            let purpose = cstr(&k);
            let units = cstr("units");
            png_write_pCAL(l, pp, purpose.as_ptr() as png_charp, 0, 100, 0, 0,
                           units.as_ptr(), std::ptr::null_mut());
        });
        // row 389
        exp_err(&r, &format!("row 389 purpose={key:?}"), b"pCAL: invalid keyword");
    }
    let r = diff("row 389 purpose=NULL", |l| unsafe {
        let pp = new_writer(l);
        let units = cstr("units");
        png_write_pCAL(l, pp, std::ptr::null_mut(), 0, 100, 0, 0, units.as_ptr(),
                       std::ptr::null_mut());
    });
    // row 389
    exp_err(&r, "row 389 purpose=NULL", b"pCAL: invalid keyword");
}

// row 390
#[test]
fn t_row390_write_sCAL_s() {
    // total_len = strlen(w) + strlen(h) + 2 must be <= 64
    for (wn, hn) in [(63usize, 0usize), (32, 31), (64, 64), (100, 1)] {
        let r = diff(&format!("row 390 {wn}+{hn}"), move |l| unsafe {
            let pp = new_writer(l);
            let w = cstr(&"1".repeat(wn.max(1)));
            let h = cstr(&"2".repeat(hn.max(1)));
            png_write_sCAL_s(l, pp, 1, w.as_ptr(), h.as_ptr());
        });
        // row 390
        exp_warn(&r, &format!("row 390 {wn}+{hn}"), b"Can't write sCAL (buffer too small)");
        assert_eq!(r.data.len(), 0, "no chunk must be written");
    }
    // exactly 64 is accepted
    let r = diff("sCAL total_len=64", |l| unsafe {
        let pp = new_writer(l);
        let w = cstr(&"1".repeat(31));
        let h = cstr(&"2".repeat(31));
        png_write_sCAL_s(l, pp, 1, w.as_ptr(), h.as_ptr());
    });
    exp_clean(&r, "sCAL total_len=64");
    // out-of-range unit values are not diagnosed at all
    for unit in [0i32, 3, 99, -1] {
        let r = diff(&format!("sCAL unit={unit}"), move |l| unsafe {
            let pp = new_writer(l);
            let w = cstr("1.0");
            let h = cstr("2.0");
            png_write_sCAL_s(l, pp, unit, w.as_ptr(), h.as_ptr());
        });
        exp_clean(&r, &format!("sCAL unit={unit}"));
    }
}

// row 391
#[test]
fn t_row391_write_pHYs() {
    for unit in [2i32, 3, 99, 0x7fff_ffff] {
        let r = diff(&format!("row 391 unit={unit}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_pHYs(l, pp, 1, 2, unit);
        });
        // row 391
        exp_warn(&r, &format!("row 391 unit={unit}"),
                 b"Unrecognized unit type for pHYs chunk");
        assert_eq!(r.data.len(), 9 + 12);
    }
    for unit in [0i32, 1, -1] {
        let r = diff(&format!("pHYs unit={unit}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_pHYs(l, pp, 0xffff_ffff, 0, unit);
        });
        exp_clean(&r, &format!("pHYs unit={unit}"));
    }
}

// row 392
#[test]
fn t_row392_write_tIME() {
    let bad: [png_time; 10] = [
        png_time { year: 2020, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2020, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2020, month: 255, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2020, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 2020, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        png_time { year: 2020, month: 1, day: 255, hour: 0, minute: 0, second: 0 },
        png_time { year: 2020, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        png_time { year: 2020, month: 1, day: 1, hour: 255, minute: 0, second: 0 },
        png_time { year: 2020, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
        png_time { year: 2020, month: 1, day: 1, hour: 0, minute: 0, second: 255 },
    ];
    for (i, t) in bad.into_iter().enumerate() {
        let r = diff(&format!("row 392 #{i}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_tIME(l, pp, &t);
        });
        // row 392
        exp_warn(&r, &format!("row 392 #{i}"), b"Invalid time specified for tIME chunk");
        assert_eq!(r.data.len(), 0, "no chunk must be written");
    }
    // `minute` and `year` are NOT validated, and second == 60 (leap) is legal.
    let ok: [png_time; 3] = [
        png_time { year: 0, month: 1, day: 1, hour: 0, minute: 99, second: 60 },
        png_time { year: 0xffff, month: 12, day: 31, hour: 23, minute: 255, second: 0 },
        png_time { year: 1995, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
    ];
    for (i, t) in ok.into_iter().enumerate() {
        let r = diff(&format!("tIME legal #{i}"), move |l| unsafe {
            let pp = new_writer(l);
            png_write_tIME(l, pp, &t);
        });
        exp_clean(&r, &format!("tIME legal #{i}"));
        assert_eq!(r.data.len(), 7 + 12);
    }
}

// ===========================================================================
// generic boundaries: NULL png_ptr / NULL info_ptr, out-of-order calls
// ===========================================================================

#[test]
fn t_null_pointer_boundaries() {
    // Every *public* pngwrite.c / pngwutil.c entry point that documents a NULL
    // check must be a silent no-op.  (The PNG_INTERNAL chunk writers -- e.g.
    // png_write_IHDR -- dereference png_ptr unconditionally and are therefore
    // not exercised with NULL.)
    let r = diff("NULL png_ptr", |l| unsafe {
        let n: png_structp = std::ptr::null_mut();
        png_write_info_before_PLTE(l, n, std::ptr::null_mut());
        png_write_info(l, n, std::ptr::null_mut());
        png_write_row(l, n, std::ptr::null());
        png_write_rows(l, n, std::ptr::null_mut(), 4);
        png_write_image(l, n, std::ptr::null_mut());
        png_write_end(l, n, std::ptr::null_mut());
        png_write_png(l, n, std::ptr::null_mut(), 0, std::ptr::null_mut());
        png_set_filter(l, n, 99, 99);
        png_set_compression_level(l, n, 99);
        png_set_compression_mem_level(l, n, 99);
        png_set_compression_strategy(l, n, 99);
        png_set_compression_window_bits(l, n, 99);
        png_set_compression_method(l, n, 99);
        png_set_compression_buffer_size(l, n, 0);
        png_set_text_compression_level(l, n, 99);
        png_set_text_compression_mem_level(l, n, 99);
        png_set_text_compression_strategy(l, n, 99);
        png_set_text_compression_window_bits(l, n, 99);
        png_set_text_compression_method(l, n, 99);
        png_set_write_status_fn(l, n, None);
        png_set_write_user_transform_fn(l, n, None);
        png_set_flush(l, n, -5);
        png_write_flush(l, n);
        png_write_chunk(l, n, b"teSt".as_ptr(), std::ptr::null(), usize::MAX);
        png_write_chunk_start(l, n, b"teSt".as_ptr(), 0xffff_ffff);
        png_write_chunk_data(l, n, b"x".as_ptr(), 1);
        png_write_chunk_end(l, n);
        png_image_free(l, std::ptr::null_mut());
    });
    exp_clean(&r, "NULL png_ptr");
    assert_eq!(r.data.len(), 0, "a NULL png_ptr must emit nothing");

    // NULL info_ptr on the info-taking entry points.
    let r = diff("NULL info_ptr", |l| unsafe {
        let pp = new_writer(l);
        png_write_info_before_PLTE(l, pp, std::ptr::null_mut());
        png_write_info(l, pp, std::ptr::null_mut());
        png_write_png(l, pp, std::ptr::null_mut(), 0, std::ptr::null_mut());
        // png_write_end tolerates a NULL info_ptr but still needs an IDAT.
        png_write_end(l, pp, std::ptr::null_mut());
    });
    exp_err(&r, "NULL info_ptr", b"No IDATs written into file");
    assert_eq!(r.data.len(), 0);

    // png_write_chunk_data with a NULL data pointer / zero length is a no-op.
    let r = diff("chunk_data NULL/0", |l| unsafe {
        let pp = new_writer(l);
        png_write_chunk_start(l, pp, b"teSt".as_ptr(), 0);
        png_write_chunk_data(l, pp, std::ptr::null(), 99);
        png_write_chunk_data(l, pp, b"x".as_ptr(), 0);
        png_write_chunk_end(l, pp);
    });
    exp_clean(&r, "chunk_data NULL/0");
    assert_eq!(r.data.len(), 12);
}

#[test]
fn t_out_of_order_write_sequences() {
    // png_write_info twice: the pre-PLTE part is skipped, the rest is repeated.
    let r = diff("write_info twice", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        let t = png_time { year: 2001, month: 2, day: 3, hour: 4, minute: 5, second: 6 };
        png_set_tIME(l, pp, ip, &t);
        png_write_info(l, pp, ip);
        png_write_info(l, pp, ip);
    });
    exp_clean(&r, "write_info twice");

    // png_write_info_before_PLTE twice: the second call does nothing at all.
    let r = diff("write_info_before_PLTE twice", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info_before_PLTE(l, pp, ip);
        let n = sink_len();
        png_write_info_before_PLTE(l, pp, ip);
        sink_len() - n
    });
    exp_clean(&r, "write_info_before_PLTE twice");
    assert_eq!(r.out, Some(0));

    // A complete image, then png_write_end twice.
    let r = diff("write_end twice", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
        let row = [1u8, 2];
        png_write_row(l, pp, row.as_ptr());
        png_write_row(l, pp, row.as_ptr());
        png_write_end(l, pp, ip);
        png_write_end(l, pp, ip);
    });
    exp_clean(&r, "write_end twice");

    // More rows than the image height, and a row after png_write_end.
    let r = diff("extra rows and a row after end", |l| unsafe {
        let (pp, ip) = wr(l);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
        let row = [3u8, 4];
        for _ in 0..5 {
            png_write_row(l, pp, row.as_ptr());
        }
        png_write_end(l, pp, ip);
        png_write_row(l, pp, row.as_ptr());
    });
    exp_clean(&r, "extra rows and a row after end");

    // png_write_info on a READ struct: there is no write_data_fn.
    let r = diff("write_info on a read struct", |l| unsafe {
        src_set(&[]);
        let pp = new_reader(l);
        let ip = png_create_info_struct(l, pp);
        png_set_IHDR(
            l, pp, ip, 2, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        png_write_info(l, pp, ip);
    });
    exp_err(&r, "write_info on a read struct", b"Call to NULL write function");

    // png_write_end on a READ struct.
    let r = diff("write_end on a read struct", |l| unsafe {
        src_set(&[]);
        let pp = new_reader(l);
        png_write_end(l, pp, std::ptr::null_mut());
    });
    exp_err(&r, "write_end on a read struct", b"No IDATs written into file");
}

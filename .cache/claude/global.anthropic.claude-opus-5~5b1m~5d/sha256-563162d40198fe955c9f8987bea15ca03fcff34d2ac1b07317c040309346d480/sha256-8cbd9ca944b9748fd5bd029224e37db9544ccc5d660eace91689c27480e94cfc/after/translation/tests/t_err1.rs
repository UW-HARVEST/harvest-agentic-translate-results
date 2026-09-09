//! Differential error-path verification, group E1:
//! the rejection / diagnostic sites in `png.c`, `pngerror.c`, `pngmem.c`,
//! `pngget.c` and `pngtrans.c` (rows 1-67 and 280-285 of `ERRORS.md`).
//!
//! Every assertion drives the *same* invalid input through both the reference
//! C `libpng.so` and the translated Rust `liblibpng.so` (always through
//! `dlsym`, never by calling the crate directly) and requires that the two
//! agree on
//!   * the exact sequence of warning / error message bytes, and
//!   * the return value / sentinel (or the fact that the call did not return
//!     at all because it called `png_error`).
//!
//! Each individual assertion carries a `// row N` comment naming the
//! `ERRORS.md` row it covers.  Rows that cannot be reached through the
//! exported API surface are called out explicitly with the reason.
#![allow(non_snake_case)]

mod common;

use common::api;
use common::*;
use libloading::Library;
use std::ffi::{c_char, c_int, c_void};

// --------------------------------------------------------------------------
// Extra entry points not present in tests/common/api.rs (declared locally,
// exactly as the rules require - tests/common/api.rs is not modified).
// --------------------------------------------------------------------------
crate::decl_api! {
    fn png_malloc_array(pp: png_structp, nelements: c_int, element_size: usize) -> png_voidp;
    fn png_realloc_array(pp: png_structp, old_array: *const c_void, old_elements: c_int,
                         add_elements: c_int, element_size: usize) -> png_voidp;
    fn png_set_rgb_coefficients(pp: png_structp);
    fn png_longjmp(pp: png_structp, val: c_int);
    fn png_get_cHRM(pp: png_structp, ip: png_infop, wx: *mut f64, wy: *mut f64,
                    rx: *mut f64, ry: *mut f64, gx: *mut f64, gy: *mut f64,
                    bx: *mut f64, by: *mut f64) -> png_uint_32;
    fn png_get_cHRM_XYZ(pp: png_structp, ip: png_infop, rX: *mut f64, rY: *mut f64,
                        rZ: *mut f64, gX: *mut f64, gY: *mut f64, gZ: *mut f64,
                        bX: *mut f64, bY: *mut f64, bZ: *mut f64) -> png_uint_32;
    fn png_get_mDCV(pp: png_structp, ip: png_infop, wx: *mut f64, wy: *mut f64,
                    rx: *mut f64, ry: *mut f64, gx: *mut f64, gy: *mut f64,
                    bx: *mut f64, by: *mut f64, maxl: *mut f64, minl: *mut f64)
                   -> png_uint_32;
}

// --------------------------------------------------------------------------
// harness helpers
// --------------------------------------------------------------------------

/// The reference `libpng.so` was linked without `-lm`, so `floor`, `pow`,
/// `modf` and friends are undefined in it and are expected to come from the
/// global symbol scope of the process.  The test executable does not pull in
/// libm, so load it explicitly with RTLD_GLOBAL before any libpng call.  (The
/// translated Rust library does have libm as a DT_NEEDED entry, but it is
/// loaded RTLD_LOCAL so that does not help the C library.)
fn ensure_libm() {
    use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
    static LIBM: std::sync::OnceLock<UnixLibrary> = std::sync::OnceLock::new();
    LIBM.get_or_init(|| unsafe {
        UnixLibrary::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL)
            .expect("dlopen libm.so.6 with RTLD_GLOBAL")
    });
}

/// Run `f` against the C library and against the Rust library and require the
/// captured message log *and* the result to be identical.
fn diff<T, F>(label: &str, f: F)
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&'static Library) -> T,
{
    ensure_libm();
    let l = libs();
    let c = capture(|| f(&l.c));
    let r = capture(|| f(&l.rs));
    c.assert_eq(&r, label);
}

/// Like [`diff`], but *also* pins the exact diagnostics the C reference is
/// expected to produce.  Without this the differential comparison could pass
/// vacuously (both libraries silently doing nothing) if a trigger condition
/// were mis-constructed.
fn diff_expect<T, F>(label: &str, expect: &[&str], f: F)
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&'static Library) -> T,
{
    ensure_libm();
    let l = libs();
    let c = capture(|| f(&l.c));
    let r = capture(|| f(&l.rs));
    let got: Vec<String> = c.log.iter().map(|m| m.to_string()).collect();
    let want: Vec<String> = expect.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        got, want,
        "{label}: the C reference did not produce the expected diagnostics \
         (so the differential assertion would be vacuous)"
    );
    c.assert_eq(&r, label);
}

const PNG_CHUNK_WARNING: c_int = 0;
const PNG_CHUNK_WRITE_ERROR: c_int = 1;
const PNG_CHUNK_ERROR: c_int = 2;
const PNG_OFFSET_PIXEL: c_int = 0;
const PNG_OFFSET_MICROMETER: c_int = 1;
const IDAT_ID: png_uint_32 = 0x4944_4154;

fn crc32(data: &[u8]) -> u32 {
    let mut c: u32 = 0xFFFF_FFFF;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let mut crcbuf = name.to_vec();
    crcbuf.extend_from_slice(data);
    v.extend_from_slice(&crc32(&crcbuf).to_be_bytes());
    v
}

/// A minimal, fully valid 1x1 8-bit greyscale PNG datastream.  The bytes are
/// built here (with a locally computed CRC) so that neither library is used to
/// produce the input for the other.
fn tiny_png() -> Vec<u8> {
    let mut v = vec![137u8, 80, 78, 71, 13, 10, 26, 10];
    v.extend(chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0]));
    // zlib stream: stored deflate block holding {filter=0, pixel=0}
    v.extend(chunk(
        b"IDAT",
        &[0x78, 0x01, 0x01, 0x02, 0x00, 0xFD, 0xFF, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01],
    ));
    v.extend(chunk(b"IEND", &[]));
    v
}

/// Read struct positioned right after `png_read_info`, i.e. with
/// `png_ptr->chunk_name == IDAT` (non-zero), which is what selects the
/// `png_chunk_*` routing inside `png_benign_error` / `png_chunk_report`.
unsafe fn reader_at_idat(l: &'static Library, data: &[u8]) -> (png_structp, png_infop) {
    src_set(data);
    let pp = api::new_reader(l);
    let ip = api::png_create_info_struct(l, pp);
    assert!(!ip.is_null());
    api::png_read_info(l, pp, ip);
    (pp, ip)
}

/// Stand-in for the application's `longjmp`, installed with
/// `png_set_longjmp_fn` so that `png_longjmp` can be exercised without
/// reaching `PNG_ABORT()` (which would kill the test process).
unsafe extern "C-unwind" fn fake_longjmp(_jb: *mut c_void, val: c_int) {
    log_push(Msg::Err(format!("harness: longjmp({val})").into_bytes()));
    panic!("harness longjmp");
}

fn fake_longjmp_ptr() -> png_voidp {
    let f: unsafe extern "C-unwind" fn(*mut c_void, c_int) = fake_longjmp;
    f as usize as png_voidp
}

fn cstr(p: png_const_charp) -> Vec<u8> {
    if p.is_null() {
        b"<null>".to_vec()
    } else {
        unsafe { std::ffi::CStr::from_ptr(p).to_bytes().to_vec() }
    }
}

fn buf_str(b: &[u8]) -> Vec<u8> {
    b.iter().copied().take_while(|&c| c != 0).collect()
}

// ==========================================================================
// png.c
// ==========================================================================

/// row 1 - `png_set_sig_bytes`: "Too many bytes for PNG signature"
#[test]
fn err_png_c_set_sig_bytes_row_1() {
    for nb in [i32::MIN, -100, -1, 0, 1, 3, 7, 8, 9, 10, 255, 256, i32::MAX] {
        // row 1
        diff(&format!("row 1 png_set_sig_bytes({nb})"), move |l| unsafe {
            let pp = api::new_reader(l);
            api::png_set_sig_bytes(l, pp, nb);
            let mut p = pp;
            api::png_destroy_read_struct(
                l,
                &mut p,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            0u32
        });
    }
    // row 1 - NULL png_ptr is a silent no-op
    diff("row 1 png_set_sig_bytes(NULL, 9)", |l| unsafe {
        api::png_set_sig_bytes(l, std::ptr::null_mut(), 9);
        0u32
    });
}

/// row 2 - `png_zalloc`: "Potential overflow in png_zalloc()"
///
/// UNREACHABLE (row 2): the guard is `items >= (~(png_alloc_size_t)0) / size`.
/// `items` and `size` are `uInt` (32-bit) while `png_alloc_size_t` is 64-bit
/// here, so `SIZE_MAX / size >= SIZE_MAX / 0xffffffff > 0xffffffff >= items`
/// always holds - the branch is dead on any 64-bit target.  Everything else
/// about `png_zalloc` is still checked below (including the enormous request
/// that falls through to `png_malloc_warn`).
#[test]
fn err_png_c_zalloc_row_2() {
    // row 2 (fall-through path: huge product -> png_malloc_warn -> warning)
    diff("row 2 png_zalloc(pp, 0xffffffff, 0xffffffff)", |l| unsafe {
        let pp = api::new_writer(l);
        let p = api::png_zalloc(l, pp, 0xffff_ffff, 0xffff_ffff);
        p.is_null()
    });
    // row 2 - NULL png_ptr short-circuits to NULL before the guard
    diff("row 2 png_zalloc(NULL, 1, 1)", |l| unsafe {
        api::png_zalloc(l, std::ptr::null_mut(), 1, 1).is_null()
    });
    for (items, size) in [(0u32, 0u32), (1, 0), (0, 1), (4, 16), (1, 0xffff_ffff)] {
        // row 2
        diff(
            &format!("row 2 png_zalloc(pp, {items}, {size})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let p = api::png_zalloc(l, pp, items, size);
                let null = p.is_null();
                api::png_zfree(l, pp, p);
                null
            },
        );
    }
}

/// row 3 - `png_user_version_check`: "Application built with libpng-... but
/// running with ..."
#[test]
fn err_png_c_user_version_check_row_3() {
    for ver in ["1.6.59", "1.6.0", "1.5.59", "1.2.3", "", "9", "1.6", "x"] {
        // row 3
        diff(
            &format!("row 3 png_user_version_check({ver:?})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let s = cs(ver);
                api::png_user_version_check(l, pp, s.as_ptr())
            },
        );
    }
    // row 3 - NULL version string also flags a library mismatch
    diff("row 3 png_user_version_check(NULL)", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_user_version_check(l, pp, std::ptr::null())
    });
}

/// row 4 - `png_data_freer`: "Unknown freer parameter in png_data_freer",
/// plus the out-of-range-enum boundary required for this file.
#[test]
fn err_png_c_data_freer_row_4() {
    for freer in [
        PNG_DESTROY_WILL_FREE_DATA,
        PNG_USER_WILL_FREE_DATA,
        0,
        3,
        4,
        99,
        -1,
        i32::MIN,
        i32::MAX,
    ] {
        for mask in [0u32, PNG_FREE_ALL as u32, 0xffff_ffff] {
            // row 4 (out-of-range enum crossing the FFI boundary)
            diff(
                &format!("row 4 png_data_freer(freer={freer}, mask={mask:#x})"),
                move |l| unsafe {
                    let pp = api::new_writer(l);
                    let ip = api::png_create_info_struct(l, pp);
                    api::png_data_freer(l, pp, ip, freer, mask);
                    // observable side effect: free_me governs png_free_data
                    api::png_free_data(l, pp, ip, mask, -1);
                    0u32
                },
            );
        }
    }
    // row 4 - NULL png_ptr / NULL info_ptr are silent no-ops even for a
    // completely bogus freer value
    diff("row 4 png_data_freer NULL sentinels", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        api::png_data_freer(l, std::ptr::null_mut(), ip, 99, 0xffff_ffff);
        api::png_data_freer(l, pp, std::ptr::null_mut(), 99, 0xffff_ffff);
        api::png_data_freer(l, std::ptr::null_mut(), std::ptr::null_mut(), 99, 0);
        0u32
    });
}

/// row 5 - `png_convert_to_rfc1123`: "Ignoring invalid time value"
#[test]
fn err_png_c_convert_to_rfc1123_row_5() {
    let bad = [
        png_time { year: 10000, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 255, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
        png_time { year: 65535, month: 255, day: 255, hour: 255, minute: 255, second: 255 },
    ];
    for (i, t) in bad.iter().enumerate() {
        let t = *t;
        // row 5
        diff(&format!("row 5 png_convert_to_rfc1123 bad[{i}]"), move |l| unsafe {
            let pp = api::new_writer(l);
            let p = api::png_convert_to_rfc1123(l, pp, &t);
            (p.is_null(), cstr(p as png_const_charp))
        });
    }
    // row 5 - the accepted boundary values must NOT warn
    for t in [
        png_time { year: 9999, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
        png_time { year: 0, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
    ] {
        // row 5
        diff("row 5 png_convert_to_rfc1123 good", move |l| unsafe {
            let pp = api::new_writer(l);
            let p = api::png_convert_to_rfc1123(l, pp, &t);
            (p.is_null(), cstr(p as png_const_charp))
        });
    }
    // row 5 - NULL png_ptr returns NULL without a message
    diff("row 5 png_convert_to_rfc1123(NULL)", |l| unsafe {
        let t = png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
        let p = api::png_convert_to_rfc1123(l, std::ptr::null_mut(), &t);
        p.is_null()
    });
}

/// row 6 - `png_set_rgb_coefficients`: "internal error handling cHRM
/// coefficients"
///
/// UNREACHABLE (row 6): the site is explicitly labelled "Check for an internal
/// error" in the C.  It is guarded by `r+g+b <= 32769` after
/// `png_muldiv(&x, x, 32768, r+g+b)` has normalised the three Y colorants, and
/// the following `add` fix-up drives the sum to exactly 32768 for every input
/// that passes the guard.  In addition it can only run at all when the
/// png_struct already carries chromaticities read from a cHRM/mDCV chunk, which
/// no exported setter can make inconsistent.  The reachable (no-op) behaviour
/// is still compared here.
#[test]
fn err_png_c_set_rgb_coefficients_row_6() {
    // row 6 (reachable part only: no chromaticities -> silent no-op)
    diff("row 6 png_set_rgb_coefficients(writer)", |l| unsafe {
        let pp = api::new_writer(l);
        png_set_rgb_coefficients(l, pp);
        api::png_get_rgb_to_gray_status(l, pp)
    });
    diff("row 6 png_set_rgb_coefficients(reader)", |l| unsafe {
        let pp = api::new_reader(l);
        png_set_rgb_coefficients(l, pp);
        api::png_get_rgb_to_gray_status(l, pp)
    });
}

/// rows 7-23 - every diagnostic in `png_check_IHDR`.
///
/// UNREACHABLE (row 9): "Image width is too large for this architecture" needs
/// `((width+7) & ~7) > (SIZE_MAX-49)/8 - 1`, i.e. width above ~2.3e18, but
/// `width` is a 32-bit `png_uint_32`; dead on a 64-bit target.
///
/// UNREACHABLE (row 22): png.c:2115 is the `#else` arm of
/// `#ifdef PNG_MNG_FEATURES_SUPPORTED`, and this build defines
/// PNG_MNG_FEATURES_SUPPORTED (see include/pnglibconf.h).  Its message
/// ("Unknown filter method in IHDR") is identical to row 20, which *is*
/// covered below.
#[test]
fn err_png_c_check_ihdr_rows_7_23() {
    // (width, height, bit_depth, color_type, interlace, compression, filter)
    let cases: &[(png_uint_32, png_uint_32, c_int, c_int, c_int, c_int, c_int, &str)] = &[
        // valid reference case: no message at all
        (1, 1, 8, 0, 0, 0, 0, "valid"),
        (16, 16, 16, 6, 1, 0, 0, "valid rgba16 adam7"),
        // row 7: width == 0
        (0, 1, 8, 0, 0, 0, 0, "row 7 width=0"),
        // row 8: width > PNG_UINT_31_MAX (also trips row 10's user limit)
        (0x8000_0000, 1, 8, 0, 0, 0, 0, "row 8 width=2^31"),
        (0xffff_ffff, 1, 8, 0, 0, 0, 0, "row 8 width=2^32-1"),
        // row 10: width above the default user limit (1000000)
        (1_000_001, 1, 8, 0, 0, 0, 0, "row 10 width=1000001"),
        (1_000_000, 1, 8, 0, 0, 0, 0, "row 10 width=1000000 (ok)"),
        // row 11: height == 0
        (1, 0, 8, 0, 0, 0, 0, "row 11 height=0"),
        // row 12: height > PNG_UINT_31_MAX (also trips row 13)
        (1, 0x8000_0000, 8, 0, 0, 0, 0, "row 12 height=2^31"),
        (1, 0xffff_ffff, 8, 0, 0, 0, 0, "row 12 height=2^32-1"),
        // row 13: height above the default user limit
        (1, 1_000_001, 8, 0, 0, 0, 0, "row 13 height=1000001"),
        (1, 1_000_000, 8, 0, 0, 0, 0, "row 13 height=1000000 (ok)"),
        // row 14: invalid bit depth
        (1, 1, 0, 0, 0, 0, 0, "row 14 bd=0"),
        (1, 1, 3, 0, 0, 0, 0, "row 14 bd=3"),
        (1, 1, 5, 0, 0, 0, 0, "row 14 bd=5"),
        (1, 1, 7, 0, 0, 0, 0, "row 14 bd=7"),
        (1, 1, 9, 0, 0, 0, 0, "row 14 bd=9"),
        (1, 1, 15, 0, 0, 0, 0, "row 14 bd=15"),
        (1, 1, 17, 0, 0, 0, 0, "row 14 bd=17"),
        (1, 1, 32, 0, 0, 0, 0, "row 14 bd=32"),
        (1, 1, -1, 0, 0, 0, 0, "row 14 bd=-1"),
        (1, 1, 99, 0, 0, 0, 0, "row 14 bd=99"),
        (1, 1, i32::MAX, 0, 0, 0, 0, "row 14 bd=INT_MAX"),
        // row 15: invalid colour type (out-of-range enum across the FFI)
        (1, 1, 8, -1, 0, 0, 0, "row 15 ct=-1"),
        (1, 1, 8, 1, 0, 0, 0, "row 15 ct=1"),
        (1, 1, 8, 5, 0, 0, 0, "row 15 ct=5"),
        (1, 1, 8, 7, 0, 0, 0, "row 15 ct=7"),
        (1, 1, 8, 99, 0, 0, 0, "row 15 ct=99"),
        (1, 1, 8, i32::MIN, 0, 0, 0, "row 15 ct=INT_MIN"),
        (1, 1, 8, i32::MAX, 0, 0, 0, "row 15 ct=INT_MAX"),
        // row 16: colour type / bit depth combinations
        (1, 1, 16, 3, 0, 0, 0, "row 16 palette16"),
        (1, 1, 4, 2, 0, 0, 0, "row 16 rgb4"),
        (1, 1, 1, 4, 0, 0, 0, "row 16 ga1"),
        (1, 1, 2, 6, 0, 0, 0, "row 16 rgba2"),
        (1, 1, 8, 3, 0, 0, 0, "row 16 palette8 (ok)"),
        // row 17: interlace method (note: negative values pass the >= test)
        (1, 1, 8, 0, 2, 0, 0, "row 17 il=2"),
        (1, 1, 8, 0, 3, 0, 0, "row 17 il=3"),
        (1, 1, 8, 0, 99, 0, 0, "row 17 il=99"),
        (1, 1, 8, 0, -1, 0, 0, "row 17 il=-1"),
        (1, 1, 8, 0, i32::MAX, 0, 0, "row 17 il=INT_MAX"),
        (1, 1, 8, 0, i32::MIN, 0, 0, "row 17 il=INT_MIN"),
        // row 18: compression method
        (1, 1, 8, 0, 0, 1, 0, "row 18 cm=1"),
        (1, 1, 8, 0, 0, 99, 0, "row 18 cm=99"),
        (1, 1, 8, 0, 0, -1, 0, "row 18 cm=-1"),
        (1, 1, 8, 0, 0, i32::MAX, 0, "row 18 cm=INT_MAX"),
        // row 20: filter method (no PNG signature seen yet)
        (1, 1, 8, 0, 0, 0, 1, "row 20 ft=1"),
        (1, 1, 8, 0, 0, 0, 64, "row 20 ft=64 without mng"),
        (1, 1, 8, 0, 0, 0, 99, "row 20 ft=99"),
        (1, 1, 8, 0, 0, 0, -1, "row 20 ft=-1"),
        (1, 1, 8, 0, 0, 0, i32::MAX, "row 20 ft=INT_MAX"),
        // row 23: every combination above with error==1 ends in png_error
        (0, 0, 0, 99, 99, 99, 99, "row 23 everything invalid"),
    ];

    for &(w, h, bd, ct, il, cm, ft, label) in cases {
        // rows 7,8,10,11,12,13,14,15,16,17,18,20,23
        diff(&format!("png_check_IHDR {label}"), move |l| unsafe {
            let pp = api::new_writer(l);
            api::png_check_IHDR(l, pp, w, h, bd, ct, il, cm, ft);
            0u32
        });
    }

    // row 10 / row 13 - the limits move with png_set_user_limits
    for (uw, uh, w, h) in [(10u32, 10u32, 11u32, 1u32), (10, 10, 1, 11), (0, 0, 1, 1)] {
        // rows 10, 13
        diff(
            &format!("rows 10,13 user_limits({uw},{uh}) img({w},{h})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                api::png_set_user_limits(l, pp, uw, uh);
                api::png_check_IHDR(l, pp, w, h, 8, 0, 0, 0, 0);
                0u32
            },
        );
    }

    // row 19 - MNG features permitted *and* a PNG signature already written.
    // This warning does NOT set `error`, so png_check_IHDR still returns.
    diff("row 19 MNG features in a PNG datastream", |l| unsafe {
        let pp = api::new_writer(l);
        sink_reset();
        api::png_write_sig(l, pp); // sets PNG_HAVE_PNG_SIGNATURE
        api::png_permit_mng_features(l, pp, PNG_ALL_MNG_FEATURES as png_uint_32);
        api::png_check_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        7u32
    });

    // rows 20+21 - filter 64 with the signature already written: the MNG
    // exception cannot apply, so both "Unknown" and "Invalid filter method"
    // fire and png_check_IHDR errors out.
    diff("rows 20,21 filter 64 after signature", |l| unsafe {
        let pp = api::new_writer(l);
        sink_reset();
        api::png_write_sig(l, pp);
        api::png_permit_mng_features(l, pp, PNG_FLAG_MNG_FILTER_64 as png_uint_32);
        api::png_check_IHDR(
            l,
            pp,
            1,
            1,
            8,
            PNG_COLOR_TYPE_RGB,
            0,
            0,
            PNG_INTRAPIXEL_DIFFERENCING,
        );
        7u32
    });

    // row 21 - signature written, filter 1, no MNG: "Unknown" + "Invalid"
    diff("row 21 filter 1 after signature", |l| unsafe {
        let pp = api::new_writer(l);
        sink_reset();
        api::png_write_sig(l, pp);
        api::png_check_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB, 0, 0, 1);
        7u32
    });

    // row 20 - the legitimate MNG intrapixel-differencing exception: no
    // signature written, FILTER_64 permitted, RGB/RGBA => no diagnostic.
    for ct in [PNG_COLOR_TYPE_RGB, PNG_COLOR_TYPE_RGB_ALPHA, PNG_COLOR_TYPE_GRAY] {
        // row 20
        diff(
            &format!("row 20 MNG filter64 exception ct={ct}"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                api::png_permit_mng_features(l, pp, PNG_FLAG_MNG_FILTER_64 as png_uint_32);
                api::png_check_IHDR(
                    l,
                    pp,
                    1,
                    1,
                    8,
                    ct,
                    0,
                    0,
                    PNG_INTRAPIXEL_DIFFERENCING,
                );
                7u32
            },
        );
    }
}

/// row 23 - reached through the public `png_get_IHDR`, which re-validates the
/// contents of a (here: freshly zeroed) info struct.
#[test]
fn err_pngget_get_ihdr_row_23() {
    // row 23
    diff("row 23 png_get_IHDR on a zeroed info struct", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        let mut w = 0u32;
        let mut h = 0u32;
        let (mut bd, mut ct, mut il, mut cm, mut ft) = (0, 0, 0, 0, 0);
        let rc = api::png_get_IHDR(
            l, pp, ip, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut ft,
        );
        (rc, w, h, bd, ct, il, cm, ft)
    });
    // row 23 - all-NULL output pointers still run the validation
    diff("row 23 png_get_IHDR NULL out params", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        api::png_get_IHDR(
            l,
            pp,
            ip,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    });
}

/// rows 24, 25 - "ASCII conversion buffer too small"
#[test]
fn err_png_c_ascii_conversion_rows_24_25() {
    for size in [0usize, 1, 2, 5, 9, 10, 11, 20, 64] {
        for (fp, precision) in [(1.0f64, 5u32), (0.0, 5), (-1.5, 5), (1e300, 5), (1.0, 0)] {
            // row 24
            diff(
                &format!("row 24 png_ascii_from_fp(size={size}, fp={fp}, p={precision})"),
                move |l| unsafe {
                    let pp = api::new_writer(l);
                    let mut buf = [0u8; 128];
                    api::png_ascii_from_fp(
                        l,
                        pp,
                        buf.as_mut_ptr() as png_charp,
                        size,
                        fp,
                        precision,
                    );
                    buf_str(&buf)
                },
            );
        }
    }
    for size in [0usize, 1, 5, 12, 13, 14, 64] {
        for fp in [0i32, 1, -1, 100000, i32::MAX, i32::MIN, -99999] {
            // row 25
            diff(
                &format!("row 25 png_ascii_from_fixed(size={size}, fp={fp})"),
                move |l| unsafe {
                    let pp = api::new_writer(l);
                    let mut buf = [0u8; 128];
                    api::png_ascii_from_fixed(
                        l,
                        pp,
                        buf.as_mut_ptr() as png_charp,
                        size,
                        fp,
                    );
                    buf_str(&buf)
                },
            );
        }
    }
}

/// rows 26, 27, 56 - `png_fixed` / `png_fixed_ITU` overflow -> `png_fixed_error`
/// ("fixed point overflow in <name>", built by the copy loop at pngerror.c:534)
#[test]
fn err_png_c_fixed_rows_26_27_56() {
    for fp in [
        0.0f64,
        1.0,
        -1.0,
        21474.83647,
        21474.83648,
        -21474.83648,
        -21474.83649,
        1e30,
        -1e30,
        f64::MAX,
    ] {
        // row 26
        diff(&format!("row 26 png_fixed({fp})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let name = cs("sCAL width");
            api::png_fixed(l, pp, fp, name.as_ptr())
        });
    }
    // row 26 - NULL name string reaches png_fixed_error(png_ptr, NULL)
    diff("row 26 png_fixed overflow with NULL name", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_fixed(l, pp, 1e30, std::ptr::null())
    });

    for fp in [0.0f64, 1.0, -0.00001, -1.0, 214748.3647, 214748.3648, 1e30, -1e30] {
        // row 27
        diff(&format!("row 27 png_fixed_ITU({fp})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let name = cs("cLLI maxCLL");
            api::png_fixed_ITU(l, pp, fp, name.as_ptr())
        });
    }

    // row 56 - the name is copied with a hard limit of PNG_MAX_ERROR_TEXT-1
    // (195) characters; check either side of that boundary.
    for n in [0usize, 1, 23, 194, 195, 196, 300] {
        // row 56
        diff(&format!("row 56 png_fixed_error(name len {n})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let name = cs(&"x".repeat(n));
            api::png_fixed_error(l, pp, name.as_ptr());
            0u32
        });
    }
    // row 56 - NULL name
    diff("row 56 png_fixed_error(NULL)", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_fixed_error(l, pp, std::ptr::null());
        0u32
    });
}

/// rows 28, 29 - gamma helpers.
///
/// NOT A REJECTION SITE (row 28): png.c:3377 is inside the block comment above
/// `png_build_16bit_table` ("... gets cleaned up on png_error ..."); the
/// extractor matched the word `png_error` in prose.  `png_gamma_correct` itself
/// contains no diagnostic.  Its behaviour is compared anyway.
#[test]
fn err_png_c_gamma_rows_28_29() {
    for (value, gamma) in [
        (0u32, PNG_FP_1),
        (128, PNG_FP_1),
        (255, 45455),
        (65535, 220000),
        (65535, 0),
        (1, i32::MAX),
    ] {
        // row 28
        diff(
            &format!("row 28 png_gamma_correct({value}, {gamma})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                api::png_gamma_correct(l, pp, value, gamma)
            },
        );
    }

    // row 29 - "gamma table being rebuilt" on the second build
    for bd in [1, 2, 4, 8, 16] {
        // row 29
        diff(&format!("row 29 png_build_gamma_table twice bd={bd}"), move |l| unsafe {
            let pp = api::new_writer(l);
            api::png_build_gamma_table(l, pp, bd);
            api::png_build_gamma_table(l, pp, bd);
            api::png_destroy_gamma_table(l, pp);
            0u32
        });
    }
    // row 29 - a single build must NOT warn
    diff("row 29 png_build_gamma_table once", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_build_gamma_table(l, pp, 8);
        api::png_destroy_gamma_table(l, pp);
        0u32
    });
}

/// rows 30, 31 - `png_image_free_function`
///
/// UNREACHABLE (rows 30 and 31): both `png_error` calls sit in the `#else`
/// arms of `#ifdef PNG_SIMPLIFIED_WRITE_SUPPORTED` / `#ifdef
/// PNG_SIMPLIFIED_READ_SUPPORTED`, and this build defines *both*
/// (include/pnglibconf.h lines 119 and 123), so the code is not compiled at
/// all.  The reachable side of `png_image_free` is still compared.
#[test]
fn err_png_c_image_free_rows_30_31() {
    // rows 30, 31 (reachable part: the simplified-API teardown itself)
    diff("rows 30,31 png_image_free of an unopened png_image", |l| unsafe {
        let mut img = png_image::default();
        api::png_image_free(l, &mut img);
        img.cmp_key()
    });
    diff("rows 30,31 png_image_free(NULL)", |l| unsafe {
        api::png_image_free(l, std::ptr::null_mut());
        0u32
    });
    diff("rows 30,31 png_image_free after a failed read", |l| unsafe {
        let mut img = png_image::default();
        let junk = [0u8; 8];
        let rc = api::png_image_begin_read_from_memory(
            l,
            &mut img,
            junk.as_ptr() as *const c_void,
            junk.len(),
        );
        api::png_image_free(l, &mut img);
        (rc, img.cmp_key())
    });
}

// ==========================================================================
// pngerror.c
// ==========================================================================

/// rows 32, 33 - `png_warning` and `png_formatted_warning`
#[test]
fn err_pngerror_c_warning_rows_32_33() {
    for msg in ["", "hello", "a message with @ in it", &"y".repeat(500)] {
        // row 32
        diff(&format!("row 32 png_warning({} bytes)", msg.len()), move |l| unsafe {
            let pp = api::new_writer(l);
            let m = cs(msg);
            api::png_warning(l, pp, m.as_ptr());
            0u32
        });
    }

    // row 33 - parameter substitution, including out-of-range parameter
    // indices and formats.
    let messages = [
        "no parameters",
        "one @1 param",
        "@1@2@3",
        "trailing at sign@",
        "@0 is not a parameter",
        "@9 is not a parameter",
        "@8 is an uninitialised parameter",
        "@a is not a parameter",
    ];
    for msg in messages {
        // row 33
        diff(&format!("row 33 png_formatted_warning({msg:?})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let mut p = [0 as c_char; PNG_WARNING_PARAMETER_SIZE * PNG_WARNING_PARAMETER_COUNT];
            let pp_ = p.as_mut_ptr();
            let a = cs("AAA");
            let b = cs("BBBB");
            api::png_warning_parameter(l, pp_, 1, a.as_ptr());
            api::png_warning_parameter(l, pp_, 2, b.as_ptr());
            api::png_warning_parameter_unsigned(l, pp_, 3, PNG_NUMBER_FORMAT_u, 12345);
            let m = cs(msg);
            api::png_formatted_warning(l, pp, pp_, m.as_ptr());
            0u32
        });
    }
    // row 33 - NULL parameter block: '@' must be printed literally
    diff("row 33 png_formatted_warning(p=NULL)", |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("@1 and @2");
        api::png_formatted_warning(l, pp, std::ptr::null_mut(), m.as_ptr());
        0u32
    });
    // row 33 - the 192-byte internal buffer truncates long messages
    diff("row 33 png_formatted_warning long message", |l| unsafe {
        let pp = api::new_writer(l);
        let mut p = [0 as c_char; PNG_WARNING_PARAMETER_SIZE * PNG_WARNING_PARAMETER_COUNT];
        let a = cs(&"P".repeat(40));
        api::png_warning_parameter(l, p.as_mut_ptr(), 1, a.as_ptr());
        let m = cs(&format!("{}@1{}", "z".repeat(180), "w".repeat(80)));
        api::png_formatted_warning(l, pp, p.as_mut_ptr(), m.as_ptr());
        0u32
    });
    // row 33 - out-of-range parameter numbers and formats are silent no-ops
    for number in [i32::MIN, -1, 0, 9, 99, i32::MAX] {
        // row 33
        diff(
            &format!("row 33 png_warning_parameter(number={number})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let mut p =
                    [0 as c_char; PNG_WARNING_PARAMETER_SIZE * PNG_WARNING_PARAMETER_COUNT];
                let a = cs("XYZ");
                api::png_warning_parameter(l, p.as_mut_ptr(), number, a.as_ptr());
                api::png_warning_parameter_unsigned(l, p.as_mut_ptr(), number, 99, 7);
                api::png_warning_parameter_signed(l, p.as_mut_ptr(), number, -1, -7);
                let m = cs("[@1][@8]");
                api::png_formatted_warning(l, pp, p.as_mut_ptr(), m.as_ptr());
                0u32
            },
        );
    }
    for format in [i32::MIN, -1, 0, 6, 99, i32::MAX] {
        // row 33
        diff(
            &format!("row 33 png_warning_parameter_unsigned(format={format})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let mut p =
                    [0 as c_char; PNG_WARNING_PARAMETER_SIZE * PNG_WARNING_PARAMETER_COUNT];
                api::png_warning_parameter_unsigned(l, p.as_mut_ptr(), 1, format, 4294967295);
                api::png_warning_parameter_signed(l, p.as_mut_ptr(), 2, format, i32::MIN);
                let m = cs("[@1][@2]");
                api::png_formatted_warning(l, pp, p.as_mut_ptr(), m.as_ptr());
                0u32
            },
        );
    }
}

/// rows 34-38 - `png_benign_error` routing (warning vs error, chunk vs plain).
#[test]
fn err_pngerror_c_benign_error_rows_34_38() {
    let data = tiny_png();

    // rows 34, 38 - a write struct has neither BENIGN_ERRORS_WARN (this build
    // does not define PNG_BENIGN_WRITE_ERRORS_SUPPORTED) nor a chunk name, so
    // the plain png_error arm runs.
    diff("rows 34,38 png_benign_error(writer)", |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("benign boom");
        api::png_benign_error(l, pp, m.as_ptr());
        0u32
    });

    // rows 34, 36 - warning arm without a chunk name
    diff("rows 34,36 png_benign_error(writer, benign=1)", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_benign_errors(l, pp, 1);
        let m = cs("benign boom");
        api::png_benign_error(l, pp, m.as_ptr());
        0u32
    });
    // row 36 - a read struct defaults to BENIGN_ERRORS_WARN; chunk_name is
    // still 0 before any chunk has been read.
    diff("row 36 png_benign_error(fresh reader)", |l| unsafe {
        let pp = api::new_reader(l);
        let m = cs("benign boom");
        api::png_benign_error(l, pp, m.as_ptr());
        api::png_get_io_chunk_type(l, pp)
    });

    // row 35 - read struct with chunk_name != 0 -> png_chunk_warning
    {
        let data = data.clone();
        diff("row 35 png_benign_error(reader at IDAT)", move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &data);
            let name = api::png_get_io_chunk_type(l, pp);
            let m = cs("benign boom");
            api::png_benign_error(l, pp, m.as_ptr());
            name
        });
    }
    // row 37 - same, but benign errors are fatal -> png_chunk_error
    {
        let data = data.clone();
        diff("row 37 png_benign_error(reader at IDAT, benign=0)", move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &data);
            assert_eq!(api::png_get_io_chunk_type(l, pp), IDAT_ID);
            api::png_set_benign_errors(l, pp, 0);
            let m = cs("benign boom");
            api::png_benign_error(l, pp, m.as_ptr());
            0u32
        });
    }
    // row 38 - read struct, chunk_name == 0, benign errors fatal -> png_error
    diff("row 38 png_benign_error(fresh reader, benign=0)", |l| unsafe {
        let pp = api::new_reader(l);
        api::png_set_benign_errors(l, pp, 0);
        let m = cs("benign boom");
        api::png_benign_error(l, pp, m.as_ptr());
        0u32
    });
    // rows 34-38 - out-of-range "allowed" values for png_set_benign_errors
    for allowed in [i32::MIN, -1, 0, 1, 2, 99, i32::MAX] {
        // rows 34, 36, 38
        diff(
            &format!("rows 34-38 png_set_benign_errors({allowed})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                api::png_set_benign_errors(l, pp, allowed);
                let m = cs("benign boom");
                api::png_benign_error(l, pp, m.as_ptr());
                0u32
            },
        );
    }
}

/// rows 39-41 - `png_app_warning` routing
#[test]
fn err_pngerror_c_app_warning_rows_39_41() {
    // rows 39, 41 - APP_WARNINGS_WARN is not set in a non-release build, so
    // an app warning is fatal.
    diff("rows 39,41 png_app_warning(writer)", |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("app warning");
        api::png_app_warning(l, pp, m.as_ptr());
        0u32
    });
    diff("rows 39,41 png_app_warning(reader)", |l| unsafe {
        let pp = api::new_reader(l);
        let m = cs("app warning");
        api::png_app_warning(l, pp, m.as_ptr());
        0u32
    });
    // rows 39, 40 - png_set_benign_errors(1) turns on APP_WARNINGS_WARN
    diff("rows 39,40 png_app_warning(writer, benign=1)", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_benign_errors(l, pp, 1);
        let m = cs("app warning");
        api::png_app_warning(l, pp, m.as_ptr());
        0u32
    });
    diff("rows 39,40 png_app_warning(reader, benign=1)", |l| unsafe {
        let pp = api::new_reader(l);
        api::png_set_benign_errors(l, pp, 1);
        let m = cs("");
        api::png_app_warning(l, pp, m.as_ptr());
        0u32
    });
}

/// rows 42-44 - `png_app_error` routing
#[test]
fn err_pngerror_c_app_error_rows_42_44() {
    // rows 42, 44
    diff("rows 42,44 png_app_error(writer)", |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("app error");
        api::png_app_error(l, pp, m.as_ptr());
        0u32
    });
    diff("rows 42,44 png_app_error(reader)", |l| unsafe {
        let pp = api::new_reader(l);
        let m = cs("app error");
        api::png_app_error(l, pp, m.as_ptr());
        0u32
    });
    // rows 42, 43
    diff("rows 42,43 png_app_error(writer, benign=1)", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_benign_errors(l, pp, 1);
        let m = cs("app error");
        api::png_app_error(l, pp, m.as_ptr());
        0u32
    });
    diff("rows 42,43 png_app_error(reader, benign=1)", |l| unsafe {
        let pp = api::new_reader(l);
        api::png_set_benign_errors(l, pp, 1);
        let m = cs(&"e".repeat(300));
        api::png_app_error(l, pp, m.as_ptr());
        0u32
    });
}

/// rows 45-49 - `png_chunk_error` and `png_chunk_warning`
///
/// UNREACHABLE (row 45): `png_chunk_error(NULL, msg)` forwards to
/// `png_error(NULL, msg)`, which - with no png_struct and therefore no
/// error_fn and no jmp_buf - reaches `png_default_error` -> `png_longjmp(NULL,
/// 1)` -> `PNG_ABORT()`, i.e. `abort()`.  It cannot be observed from a test
/// process that must survive; it is deliberately not called.
#[test]
fn err_pngerror_c_chunk_diagnostics_rows_45_49() {
    let data = tiny_png();

    for msg in ["", "chunk boom", &"c".repeat(300)] {
        // row 46 - png_chunk_error with a real png_struct prefixes the
        // (here all-zero, hence hex-escaped) chunk name.
        diff(
            &format!("row 46 png_chunk_error(reader, {} bytes)", msg.len()),
            move |l| unsafe {
                let pp = api::new_reader(l);
                let m = cs(msg);
                api::png_chunk_error(l, pp, m.as_ptr());
                0u32
            },
        );
        // rows 47, 49 - png_chunk_warning, same formatting, non-fatal
        diff(
            &format!("rows 47,49 png_chunk_warning(reader, {} bytes)", msg.len()),
            move |l| unsafe {
                let pp = api::new_reader(l);
                let m = cs(msg);
                api::png_chunk_warning(l, pp, m.as_ptr());
                0u32
            },
        );
    }
    // row 46 - with a genuine chunk name in place
    {
        let data = data.clone();
        diff("row 46 png_chunk_error(reader at IDAT)", move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &data);
            let m = cs("chunk boom");
            api::png_chunk_error(l, pp, m.as_ptr());
            0u32
        });
    }
    {
        let data = data.clone();
        // rows 47, 49
        diff("rows 47,49 png_chunk_warning(reader at IDAT)", move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &data);
            let m = cs("chunk boom");
            api::png_chunk_warning(l, pp, m.as_ptr());
            api::png_get_io_chunk_type(l, pp)
        });
    }
    // row 48 - NULL png_ptr degrades to a plain png_warning (which, with no
    // struct, goes to the default handler and returns).
    diff("row 48 png_chunk_warning(NULL)", |l| unsafe {
        let m = cs("chunk boom");
        api::png_chunk_warning(l, std::ptr::null_mut(), m.as_ptr());
        0u32
    });
    // rows 47, 49 - NULL message: png_format_buffer only emits the chunk name
    diff("rows 47,49 png_chunk_warning(reader, NULL msg)", |l| unsafe {
        let pp = api::new_reader(l);
        api::png_chunk_warning(l, pp, std::ptr::null());
        0u32
    });
}

/// rows 50, 51 - `png_chunk_benign_error`
#[test]
fn err_pngerror_c_chunk_benign_error_rows_50_51() {
    let data = tiny_png();
    // row 50 - read struct defaults to BENIGN_ERRORS_WARN -> chunk warning
    {
        let data = data.clone();
        diff("row 50 png_chunk_benign_error(reader at IDAT)", move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &data);
            let m = cs("chunk benign");
            api::png_chunk_benign_error(l, pp, m.as_ptr());
            api::png_get_io_chunk_type(l, pp)
        });
    }
    diff("row 50 png_chunk_benign_error(fresh reader)", |l| unsafe {
        let pp = api::new_reader(l);
        let m = cs("chunk benign");
        api::png_chunk_benign_error(l, pp, m.as_ptr());
        0u32
    });
    // row 51 - fatal variant
    {
        let data = data.clone();
        diff("row 51 png_chunk_benign_error(benign=0)", move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &data);
            api::png_set_benign_errors(l, pp, 0);
            let m = cs("chunk benign");
            api::png_chunk_benign_error(l, pp, m.as_ptr());
            0u32
        });
    }
    // row 51 - a write struct never has BENIGN_ERRORS_WARN in this build
    diff("row 51 png_chunk_benign_error(writer)", |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("chunk benign");
        api::png_chunk_benign_error(l, pp, m.as_ptr());
        0u32
    });
}

/// rows 52-55 - `png_chunk_report` routing, including out-of-range `error`
/// values crossing the FFI.
#[test]
fn err_pngerror_c_chunk_report_rows_52_55() {
    let data = tiny_png();
    let levels = [
        PNG_CHUNK_WARNING,
        PNG_CHUNK_WRITE_ERROR,
        PNG_CHUNK_ERROR,
        3,
        99,
        -1,
        i32::MIN,
        i32::MAX,
    ];
    for e in levels {
        // rows 52, 53 - read struct: below PNG_CHUNK_ERROR -> chunk warning,
        // at or above -> chunk benign error.
        let d = data.clone();
        diff(
            &format!("rows 52,53 png_chunk_report(reader at IDAT, {e})"),
            move |l| unsafe {
                let (pp, _ip) = reader_at_idat(l, &d);
                let m = cs("report");
                api::png_chunk_report(l, pp, m.as_ptr(), e);
                api::png_get_io_chunk_type(l, pp)
            },
        );
        // rows 52, 53 - and with benign errors fatal
        let d = data.clone();
        diff(
            &format!("rows 52,53 png_chunk_report(reader, benign=0, {e})"),
            move |l| unsafe {
                let (pp, _ip) = reader_at_idat(l, &d);
                api::png_set_benign_errors(l, pp, 0);
                let m = cs("report");
                api::png_chunk_report(l, pp, m.as_ptr(), e);
                0u32
            },
        );
        // rows 52, 54, 55 - write struct: below PNG_CHUNK_WRITE_ERROR ->
        // png_app_warning, at or above -> png_app_error.
        diff(
            &format!("rows 52,54,55 png_chunk_report(writer, {e})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let m = cs("report");
                api::png_chunk_report(l, pp, m.as_ptr(), e);
                0u32
            },
        );
        diff(
            &format!("rows 52,54,55 png_chunk_report(writer, benign=1, {e})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                api::png_set_benign_errors(l, pp, 1);
                let m = cs("report");
                api::png_chunk_report(l, pp, m.as_ptr(), e);
                0u32
            },
        );
    }
}

/// rows 57-60 - `png_set_longjmp_fn` / `png_longjmp`
///
/// UNREACHABLE (row 57): "Libpng jmp_buf still allocated" needs
/// `jmp_buf_size == 0` together with `jmp_buf_ptr != &png_ptr->jmp_buf_local`.
/// A freshly created png_struct has `jmp_buf_ptr == NULL`; the only way to make
/// it non-NULL from the API is `png_set_longjmp_fn`, which either points it at
/// `&jmp_buf_local` (leaving size 0) or heap-allocates it (leaving size != 0).
/// `png_free_jmpbuf` resets both.  The C comment says as much: "This is an
/// internal error in libpng".
#[test]
fn err_pngerror_c_longjmp_rows_57_60() {
    // row 58 - "Application jmp_buf size changed".  Whichever of the two
    // sizes exceeds sizeof(jmp_buf), the second call must disagree with the
    // recorded size and warn.
    for (s1, s2) in [(1024usize, 1023usize), (8, 7), (1, 4096), (200, 8)] {
        // row 58
        diff(
            &format!("row 58 png_set_longjmp_fn({s1}) then ({s2})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let f = fake_longjmp_ptr();
                let a = api::png_set_longjmp_fn(l, pp, f, s1);
                let b = api::png_set_longjmp_fn(l, pp, f, s2);
                (a.is_null(), b.is_null())
            },
        );
    }
    // row 58 - the same size twice must NOT warn
    for s in [8usize, 200, 1024] {
        // row 58
        diff(&format!("row 58 png_set_longjmp_fn({s}) twice"), move |l| unsafe {
            let pp = api::new_writer(l);
            let f = fake_longjmp_ptr();
            let a = api::png_set_longjmp_fn(l, pp, f, s);
            let b = api::png_set_longjmp_fn(l, pp, f, s);
            (a.is_null(), b.is_null())
        });
    }
    // rows 57, 58 - NULL png_ptr returns NULL without a diagnostic
    diff("rows 57,58 png_set_longjmp_fn(NULL)", |l| unsafe {
        api::png_set_longjmp_fn(l, std::ptr::null_mut(), fake_longjmp_ptr(), 8).is_null()
    });

    // row 60 - png_longjmp dispatches through png_ptr->longjmp_fn.
    for val in [0, 1, -1, 7, i32::MAX] {
        // row 60
        diff(&format!("row 60 png_longjmp(pp, {val})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let r = api::png_set_longjmp_fn(l, pp, fake_longjmp_ptr(), 8);
            assert!(!r.is_null());
            png_longjmp(l, pp, val);
            0u32
        });
    }
    // row 60 - png_free_jmpbuf clears the dispatch, after which png_longjmp
    // would abort(); only the clearing itself is checked here.
    diff("row 60 png_free_jmpbuf", |l| unsafe {
        let pp = api::new_writer(l);
        let r = api::png_set_longjmp_fn(l, pp, fake_longjmp_ptr(), 1024);
        api::png_free_jmpbuf(l, pp);
        api::png_free_jmpbuf(l, std::ptr::null_mut());
        r.is_null()
    });

    // row 59 - png_default_error (reached when the app installs no error_fn)
    // ends in png_longjmp.
    diff("row 59 png_error -> png_default_error -> png_longjmp", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_longjmp_fn(l, pp, fake_longjmp_ptr(), 8);
        // remove the recording error handler so png_default_error runs
        api::png_set_error_fn(l, pp, std::ptr::null_mut(), None, Some(rec_warning));
        let m = cs("default error path");
        api::png_error(l, pp, m.as_ptr());
        0u32
    });
    // row 59 - same via a warning-then-error sequence, NULL message
    diff("row 59 png_error(NULL msg) -> png_longjmp", |l| unsafe {
        let pp = api::new_reader(l);
        api::png_set_longjmp_fn(l, pp, fake_longjmp_ptr(), 8);
        api::png_set_error_fn(l, pp, std::ptr::null_mut(), None, Some(rec_warning));
        let m = cs("boom");
        api::png_warning(l, pp, m.as_ptr()); // recorded, warning_fn still set
        api::png_error(l, pp, m.as_ptr());
        0u32
    });
}

// ==========================================================================
// pngget.c
// ==========================================================================

/// row 61 - `png_fixed_inches_from_microns`: "fixed point overflow ignored"
#[test]
fn err_pngget_fixed_inches_row_61() {
    // The conversion is microns * 500 / 127, so |microns| above
    // 2^31 * 127 / 500 ~= 545460846 overflows a png_fixed_point.
    for microns in [
        0i32,
        1,
        -1,
        545_460_846,
        545_460_847,
        600_000_000,
        -600_000_000,
        i32::MAX,
        i32::MIN,
    ] {
        // row 61
        diff(
            &format!("row 61 png_get_x_offset_inches_fixed(microns={microns})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let ip = api::png_create_info_struct(l, pp);
                api::png_set_oFFs(l, pp, ip, microns, microns, PNG_OFFSET_MICROMETER);
                let x = api::png_get_x_offset_inches_fixed(l, pp, ip);
                let y = api::png_get_y_offset_inches_fixed(l, pp, ip);
                (x, y)
            },
        );
    }
    // row 61 - PIXEL units make png_get_*_offset_microns return 0, so no
    // overflow and no warning.
    diff("row 61 offsets in pixel units do not warn", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        api::png_set_oFFs(l, pp, ip, i32::MAX, i32::MIN, PNG_OFFSET_PIXEL);
        (
            api::png_get_x_offset_inches_fixed(l, pp, ip),
            api::png_get_y_offset_inches_fixed(l, pp, ip),
        )
    });
    // row 61 - out-of-range oFFs unit values crossing the FFI
    for unit in [2, 99, -1, i32::MAX, i32::MIN] {
        // row 61
        diff(&format!("row 61 oFFs unit={unit}"), move |l| unsafe {
            let pp = api::new_writer(l);
            let ip = api::png_create_info_struct(l, pp);
            api::png_set_oFFs(l, pp, ip, 600_000_000, 600_000_000, unit);
            let mut x = 0i32;
            let mut y = 0i32;
            let mut u = 0i32;
            let rc = api::png_get_oFFs(l, pp, ip, &mut x, &mut y, &mut u);
            (
                rc,
                x,
                y,
                u,
                api::png_get_x_offset_inches_fixed(l, pp, ip),
                api::png_get_y_offset_inches_fixed(l, pp, ip),
                api::png_get_x_offset_microns(l, pp, ip),
                api::png_get_x_offset_pixels(l, pp, ip),
            )
        });
    }
}

/// row 62 - `png_get_eXIf`: "png_get_eXIf does not work; use png_get_eXIf_1"
#[test]
fn err_pngget_get_eXIf_row_62() {
    // row 62
    diff("row 62 png_get_eXIf(writer, info)", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        let mut exif: png_bytep = std::ptr::null_mut();
        let rc = api::png_get_eXIf(l, pp, ip, &mut exif);
        (rc, exif.is_null())
    });
    // row 62 - the warning is unconditional, even with NULL info_ptr / exif
    diff("row 62 png_get_eXIf(writer, NULL info, NULL out)", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_get_eXIf(l, pp, std::ptr::null_mut(), std::ptr::null_mut())
    });
    // row 62 - with a NULL png_ptr the warning goes to the default handler
    diff("row 62 png_get_eXIf(NULL)", |l| unsafe {
        api::png_get_eXIf(l, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
    });
    // row 62 - png_get_eXIf_1 is the working variant: no diagnostic
    diff("row 62 png_get_eXIf_1", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        let mut n = 0u32;
        let mut exif: png_bytep = std::ptr::null_mut();
        let rc = api::png_get_eXIf_1(l, pp, ip, &mut n, &mut exif);
        (rc, n, exif.is_null())
    });
}

/// NULL `png_ptr` / NULL `info_ptr` sentinels for the whole of pngget.c plus
/// the struct-level accessors named in the task.
///
/// `png_get_io_state` and `png_get_io_chunk_type` are deliberately *not*
/// called with a NULL png_ptr: the C dereferences `png_ptr->io_state` /
/// `png_ptr->chunk_name` with no NULL check, so a NULL argument is undefined
/// behaviour in the reference implementation (it segfaults).  They are
/// exercised with a live struct instead.
#[test]
fn err_pngget_null_sentinels() {
    unsafe fn get_all(l: &'static Library, pp: png_structp, ip: png_infop) -> Vec<i64> {
        let mut v: Vec<i64> = Vec::new();
        macro_rules! push {
            ($e:expr) => {
                v.push($e as i64)
            };
        }
        // scratch out-parameters
        let mut u32a = 0u32;
        let mut u32b = 0u32;
        let mut i32a = 0i32;
        let mut i32b = 0i32;
        let mut i32c = 0i32;
        let mut i32d = 0i32;
        let mut i32e = 0i32;
        let mut fx = [0i32; 10];
        let mut f64s = [0f64; 10];
        let mut u32s = [0u32; 10];
        let mut b8 = [0u8; 4];

        for flag in [
            0u32,
            PNG_INFO_gAMA,
            PNG_INFO_tRNS,
            PNG_INFO_PLTE,
            0x10000,
            0xffff_ffff,
        ] {
            push!(api::png_get_valid(l, pp, ip, flag));
        }
        push!(api::png_get_rowbytes(l, pp, ip));
        push!(api::png_get_rows(l, pp, ip).is_null());
        push!(api::png_get_image_width(l, pp, ip));
        push!(api::png_get_image_height(l, pp, ip));
        push!(api::png_get_bit_depth(l, pp, ip));
        push!(api::png_get_color_type(l, pp, ip));
        push!(api::png_get_filter_type(l, pp, ip));
        push!(api::png_get_interlace_type(l, pp, ip));
        push!(api::png_get_compression_type(l, pp, ip));
        push!(api::png_get_channels(l, pp, ip));
        push!(api::png_get_x_pixels_per_meter(l, pp, ip));
        push!(api::png_get_y_pixels_per_meter(l, pp, ip));
        push!(api::png_get_pixels_per_meter(l, pp, ip));
        push!(api::png_get_x_pixels_per_inch(l, pp, ip));
        push!(api::png_get_y_pixels_per_inch(l, pp, ip));
        push!(api::png_get_pixels_per_inch(l, pp, ip));
        push!(api::png_get_pixel_aspect_ratio(l, pp, ip).to_bits());
        push!(api::png_get_pixel_aspect_ratio_fixed(l, pp, ip));
        push!(api::png_get_x_offset_microns(l, pp, ip));
        push!(api::png_get_y_offset_microns(l, pp, ip));
        push!(api::png_get_x_offset_pixels(l, pp, ip));
        push!(api::png_get_y_offset_pixels(l, pp, ip));
        push!(api::png_get_x_offset_inches(l, pp, ip).to_bits());
        push!(api::png_get_y_offset_inches(l, pp, ip).to_bits());
        push!(api::png_get_x_offset_inches_fixed(l, pp, ip));
        push!(api::png_get_y_offset_inches_fixed(l, pp, ip));
        push!(api::png_get_pHYs_dpi(l, pp, ip, &mut u32a, &mut u32b, &mut i32a));
        push!(u32a);
        push!(u32b);
        push!(i32a);
        push!(api::png_get_signature(l, pp, ip).is_null());
        push!(api::png_get_palette_max(l, pp, ip));

        let mut bg: *mut png_color_16 = std::ptr::null_mut();
        push!(api::png_get_bKGD(l, pp, ip, &mut bg));
        push!(bg.is_null());

        push!(api::png_get_cHRM_fixed(
            l, pp, ip, &mut fx[0], &mut fx[1], &mut fx[2], &mut fx[3], &mut fx[4],
            &mut fx[5], &mut fx[6], &mut fx[7]
        ));
        push!(api::png_get_cHRM_XYZ_fixed(
            l, pp, ip, &mut fx[0], &mut fx[1], &mut fx[2], &mut fx[3], &mut fx[4],
            &mut fx[5], &mut fx[6], &mut fx[7], &mut fx[8]
        ));
        push!(png_get_cHRM(
            l, pp, ip, &mut f64s[0], &mut f64s[1], &mut f64s[2], &mut f64s[3],
            &mut f64s[4], &mut f64s[5], &mut f64s[6], &mut f64s[7]
        ));
        push!(png_get_cHRM_XYZ(
            l, pp, ip, &mut f64s[0], &mut f64s[1], &mut f64s[2], &mut f64s[3],
            &mut f64s[4], &mut f64s[5], &mut f64s[6], &mut f64s[7], &mut f64s[8]
        ));
        push!(api::png_get_gAMA_fixed(l, pp, ip, &mut fx[0]));
        push!(api::png_get_gAMA(l, pp, ip, &mut f64s[0]));
        push!(api::png_get_sRGB(l, pp, ip, &mut i32a));

        let mut name: png_charp = std::ptr::null_mut();
        let mut profile: png_bytep = std::ptr::null_mut();
        push!(api::png_get_iCCP(l, pp, ip, &mut name, &mut i32a, &mut profile, &mut u32a));
        push!(name.is_null());
        push!(profile.is_null());

        let mut splt: *mut png_sPLT_t = std::ptr::null_mut();
        push!(api::png_get_sPLT(l, pp, ip, &mut splt));
        push!(splt.is_null());

        push!(api::png_get_cICP(
            l, pp, ip, &mut b8[0], &mut b8[1], &mut b8[2], &mut b8[3]
        ));
        push!(api::png_get_cLLI_fixed(l, pp, ip, &mut u32s[0], &mut u32s[1]));
        push!(api::png_get_cLLI(l, pp, ip, &mut f64s[0], &mut f64s[1]));
        push!(api::png_get_mDCV_fixed(
            l, pp, ip, &mut u32s[0], &mut u32s[1], &mut u32s[2], &mut u32s[3],
            &mut u32s[4], &mut u32s[5], &mut u32s[6], &mut u32s[7], &mut u32s[8],
            &mut u32s[9]
        ));
        push!(png_get_mDCV(
            l, pp, ip, &mut f64s[0], &mut f64s[1], &mut f64s[2], &mut f64s[3],
            &mut f64s[4], &mut f64s[5], &mut f64s[6], &mut f64s[7], &mut f64s[8],
            &mut f64s[9]
        ));

        let mut exif: png_bytep = std::ptr::null_mut();
        push!(api::png_get_eXIf_1(l, pp, ip, &mut u32a, &mut exif));
        push!(exif.is_null());

        let mut hist: *mut png_uint_16 = std::ptr::null_mut();
        push!(api::png_get_hIST(l, pp, ip, &mut hist));
        push!(hist.is_null());

        push!(api::png_get_oFFs(l, pp, ip, &mut i32a, &mut i32b, &mut i32c));

        let mut purpose: png_charp = std::ptr::null_mut();
        let mut units: png_charp = std::ptr::null_mut();
        let mut params: *mut png_charp = std::ptr::null_mut();
        push!(api::png_get_pCAL(
            l, pp, ip, &mut purpose, &mut i32a, &mut i32b, &mut i32c, &mut i32d,
            &mut units, &mut params
        ));
        push!(purpose.is_null());
        push!(units.is_null());
        push!(params.is_null());

        push!(api::png_get_sCAL_fixed(l, pp, ip, &mut i32e, &mut fx[0], &mut fx[1]));
        push!(api::png_get_sCAL(l, pp, ip, &mut i32e, &mut f64s[0], &mut f64s[1]));
        let mut sw: png_charp = std::ptr::null_mut();
        let mut sh: png_charp = std::ptr::null_mut();
        push!(api::png_get_sCAL_s(l, pp, ip, &mut i32e, &mut sw, &mut sh));
        push!(sw.is_null());
        push!(sh.is_null());

        push!(api::png_get_pHYs(l, pp, ip, &mut u32a, &mut u32b, &mut i32a));

        let mut pal: *mut png_color = std::ptr::null_mut();
        push!(api::png_get_PLTE(l, pp, ip, &mut pal, &mut i32a));
        push!(pal.is_null());

        let mut sbit: *mut png_color_8 = std::ptr::null_mut();
        push!(api::png_get_sBIT(l, pp, ip, &mut sbit));
        push!(sbit.is_null());

        let mut text: *mut png_text = std::ptr::null_mut();
        push!(api::png_get_text(l, pp, ip, &mut text, &mut i32a));
        push!(text.is_null());
        push!(i32a);

        let mut tm: *mut png_time = std::ptr::null_mut();
        push!(api::png_get_tIME(l, pp, ip, &mut tm));
        push!(tm.is_null());

        let mut trans: png_bytep = std::ptr::null_mut();
        let mut tc: *mut png_color_16 = std::ptr::null_mut();
        push!(api::png_get_tRNS(l, pp, ip, &mut trans, &mut i32a, &mut tc));
        push!(trans.is_null());
        push!(tc.is_null());

        let mut unk: *mut png_unknown_chunk = std::ptr::null_mut();
        push!(api::png_get_unknown_chunks(l, pp, ip, &mut unk));
        push!(unk.is_null());

        // struct-only accessors named in the task
        push!(api::png_get_rgb_to_gray_status(l, pp));
        push!(api::png_get_user_chunk_ptr(l, pp).is_null());
        push!(api::png_get_compression_buffer_size(l, pp));
        push!(api::png_get_user_width_max(l, pp));
        push!(api::png_get_user_height_max(l, pp));
        push!(api::png_get_chunk_cache_max(l, pp));
        push!(api::png_get_chunk_malloc_max(l, pp));
        push!(api::png_get_error_ptr(l, pp).is_null());
        push!(api::png_get_mem_ptr(l, pp).is_null());
        push!(api::png_get_io_ptr(l, pp).is_null());
        push!(api::png_get_progressive_ptr(l, pp).is_null());
        push!(api::png_get_user_transform_ptr(l, pp).is_null());
        push!(api::png_get_current_row_number(l, pp));
        push!(api::png_get_current_pass_number(l, pp));
        v
    }

    // NULL png_ptr and NULL info_ptr in every combination.  png_get_IHDR is
    // covered separately because with two live pointers it runs
    // png_check_IHDR (see err_pngget_get_ihdr_row_23).
    diff("pngget NULL/NULL", |l| unsafe {
        get_all(l, std::ptr::null_mut(), std::ptr::null_mut())
    });
    diff("pngget pp/NULL", |l| unsafe {
        let pp = api::new_writer(l);
        get_all(l, pp, std::ptr::null_mut())
    });
    diff("pngget NULL/ip", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        get_all(l, std::ptr::null_mut(), ip)
    });
    diff("pngget writer pp/ip", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        get_all(l, pp, ip)
    });
    diff("pngget reader pp/ip", |l| unsafe {
        let pp = api::new_reader(l);
        let ip = api::png_create_info_struct(l, pp);
        get_all(l, pp, ip)
    });

    // png_get_IHDR NULL sentinels only (no validation runs)
    diff("png_get_IHDR NULL sentinels", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        let n32 = std::ptr::null_mut::<png_uint_32>();
        let ni = std::ptr::null_mut::<c_int>();
        let a = api::png_get_IHDR(
            l,
            std::ptr::null_mut(),
            ip,
            n32,
            n32,
            ni,
            ni,
            ni,
            ni,
            ni,
        );
        let b = api::png_get_IHDR(l, pp, std::ptr::null_mut(), n32, n32, ni, ni, ni, ni, ni);
        let c = api::png_get_IHDR(
            l,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            n32,
            n32,
            ni,
            ni,
            ni,
            ni,
            ni,
        );
        (a, b, c)
    });

    // io_state / io_chunk_type with a live struct (NULL would be UB in C)
    diff("png_get_io_state / io_chunk_type", |l| unsafe {
        let pp = api::new_writer(l);
        let rp = api::new_reader(l);
        (
            api::png_get_io_state(l, pp),
            api::png_get_io_chunk_type(l, pp),
            api::png_get_io_state(l, rp),
            api::png_get_io_chunk_type(l, rp),
        )
    });

    // version / copyright strings are NULL-tolerant
    diff("png.c version strings with NULL png_ptr", |l| unsafe {
        let n = std::ptr::null_mut();
        (
            cstr(api::png_get_libpng_ver(l, n)),
            cstr(api::png_get_header_ver(l, n)),
            cstr(api::png_get_header_version(l, n)),
            cstr(api::png_get_copyright(l, n)),
        )
    });
}

/// NULL sentinels for the struct/info lifecycle helpers named in the task.
#[test]
fn err_png_c_free_data_null_sentinels() {
    // row 4 / png_free_data / png_set_invalid / png_get_valid with NULL args
    diff("png_free_data + png_set_invalid NULL sentinels", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        for mask in [0u32, PNG_FREE_ALL as u32, 0xffff_ffff] {
            for num in [-1, 0, 1, 99, i32::MAX, i32::MIN] {
                api::png_free_data(l, std::ptr::null_mut(), ip, mask, num);
                api::png_free_data(l, pp, std::ptr::null_mut(), mask, num);
                api::png_free_data(l, std::ptr::null_mut(), std::ptr::null_mut(), mask, num);
            }
        }
        for m in [0, PNG_FREE_ALL, 99, -1, i32::MAX, i32::MIN] {
            api::png_set_invalid(l, std::ptr::null_mut(), ip, m);
            api::png_set_invalid(l, pp, std::ptr::null_mut(), m);
            api::png_set_invalid(l, pp, ip, m);
        }
        let mut valids = Vec::new();
        for flag in [0u32, PNG_INFO_gAMA, 0x10000, 0xffff_ffff] {
            valids.push(api::png_get_valid(l, pp, ip, flag));
        }
        valids
    });

    // png_free_data with an out-of-range `num` on an info struct that really
    // owns text entries is *not* exercised: the C indexes
    // info_ptr->text[num] without a bounds check, so num=99 there is
    // undefined behaviour in the reference implementation.  With a freshly
    // created info struct (text == NULL, free_me == 0) every branch is
    // guarded, which is what the loop above relies on.
    diff("png_free_data on a fresh info struct", |l| unsafe {
        let pp = api::new_writer(l);
        let ip = api::png_create_info_struct(l, pp);
        api::png_free_data(l, pp, ip, 0xffff_ffff, -1);
        api::png_free_data(l, pp, ip, 0xffff_ffff, -1);
        api::png_get_rowbytes(l, pp, ip)
    });
}

// ==========================================================================
// pngmem.c
// ==========================================================================

/// rows 63, 64 - `png_malloc_array` / `png_realloc_array` internal-error
/// guards ("internal error: array alloc" / "internal error: array realloc").
#[test]
fn err_pngmem_array_rows_63_64() {
    for (n, es) in [
        (0i32, 4usize),
        (-1, 4),
        (i32::MIN, 4),
        (1, 0),
        (0, 0),
        (-1, 0),
        (i32::MAX, 0),
    ] {
        // row 63
        diff(
            &format!("row 63 png_malloc_array({n}, {es})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let p = png_malloc_array(l, pp, n, es);
                let null = p.is_null();
                api::png_free(l, pp, p);
                null
            },
        );
    }
    // row 63 - valid arguments, and the silent NULL on a too-large request
    for (n, es) in [(1i32, 4usize), (8, 16), (2, usize::MAX), (i32::MAX, usize::MAX)] {
        // row 63
        diff(
            &format!("row 63 png_malloc_array ok/oversize({n}, {es})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let p = png_malloc_array(l, pp, n, es);
                let null = p.is_null();
                api::png_free(l, pp, p);
                null
            },
        );
    }

    for (old_n, add_n, es, with_array) in [
        (0i32, 0i32, 4usize, false),
        (0, -1, 4, false),
        (0, i32::MIN, 4, false),
        (-1, 1, 4, false),
        (i32::MIN, 1, 4, false),
        (0, 1, 0, false),
        (1, 1, 4, false), // old_array == NULL but old_elements > 0
        (99, 1, 4, false),
    ] {
        // row 64
        diff(
            &format!("row 64 png_realloc_array({old_n}, {add_n}, {es}, arr={with_array})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let old = if with_array {
                    png_malloc_array(l, pp, 4, 4)
                } else {
                    std::ptr::null_mut()
                };
                let p = png_realloc_array(l, pp, old as *const c_void, old_n, add_n, es);
                let null = p.is_null();
                api::png_free(l, pp, p);
                api::png_free(l, pp, old);
                null
            },
        );
    }
    // row 64 - valid growth, and the INT_MAX element-count overflow guard
    for (old_n, add_n, es, with_array) in [
        (0i32, 4i32, 8usize, false),
        (4, 4, 8, true),
        (i32::MAX, i32::MAX, 8, true),
        (1, i32::MAX, 8, true),
    ] {
        // row 64
        diff(
            &format!("row 64 png_realloc_array ok({old_n}, {add_n}, {es})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                let old = if with_array {
                    png_malloc_array(l, pp, 4, es as i32 as usize)
                } else {
                    std::ptr::null_mut()
                };
                let use_old = if with_array { 4 } else { 0 };
                let p = png_realloc_array(
                    l,
                    pp,
                    old as *const c_void,
                    if old_n == i32::MAX || old_n > 4 { old_n } else { use_old },
                    add_n,
                    es,
                );
                let null = p.is_null();
                api::png_free(l, pp, p);
                api::png_free(l, pp, old);
                null
            },
        );
    }
}

/// rows 65, 66, 67 - the allocator diagnostics, plus the zero/oversized-length
/// boundaries required for this file.
#[test]
fn err_pngmem_alloc_rows_65_67() {
    // row 65 - png_malloc: "Out of memory"
    for size in [usize::MAX, usize::MAX - 1, usize::MAX / 2 + 1] {
        // row 65
        diff(&format!("row 65 png_malloc({size:#x})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let p = api::png_malloc(l, pp, size);
            let null = p.is_null();
            api::png_free(l, pp, p);
            null
        });
    }
    // row 65 - zero length must not error
    diff("row 65 png_malloc(pp, 0)", |l| unsafe {
        let pp = api::new_writer(l);
        let p = api::png_malloc(l, pp, 0);
        let null = p.is_null();
        api::png_free(l, pp, p);
        null
    });
    // row 65 - NULL png_ptr returns NULL before any allocation
    diff("row 65 png_malloc(NULL, 16)", |l| unsafe {
        api::png_malloc(l, std::ptr::null_mut(), 16).is_null()
    });
    // row 65 - png_calloc funnels through png_malloc
    for size in [0usize, 1, 32, usize::MAX] {
        // row 65
        diff(&format!("row 65 png_calloc({size:#x})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let p = api::png_calloc(l, pp, size);
            let null = p.is_null();
            if !null && size > 0 {
                let s = std::slice::from_raw_parts(p as *const u8, size.min(32));
                assert!(s.iter().all(|&b| b == 0), "png_calloc did not zero");
            }
            api::png_free(l, pp, p);
            null
        });
    }
    diff("row 65 png_calloc(NULL, 16)", |l| unsafe {
        api::png_calloc(l, std::ptr::null_mut(), 16).is_null()
    });

    // row 66 - png_malloc_default: "Out of Memory" (capital M)
    for size in [0usize, 1, usize::MAX] {
        // row 66
        diff(&format!("row 66 png_malloc_default({size:#x})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let p = api::png_malloc_default(l, pp, size);
            let null = p.is_null();
            api::png_free_default(l, pp, p);
            null
        });
    }
    // row 66 - NULL png_ptr
    diff("row 66 png_malloc_default(NULL, 16)", |l| unsafe {
        api::png_malloc_default(l, std::ptr::null_mut(), 16).is_null()
    });

    // row 67 - png_malloc_warn: warning + NULL, never an error
    for size in [0usize, 1, 64, usize::MAX] {
        // row 67
        diff(&format!("row 67 png_malloc_warn({size:#x})"), move |l| unsafe {
            let pp = api::new_writer(l);
            let p = api::png_malloc_warn(l, pp, size);
            let null = p.is_null();
            api::png_free(l, pp, p);
            null
        });
    }
    // row 67 - NULL png_ptr: no warning at all
    diff("row 67 png_malloc_warn(NULL, 16)", |l| unsafe {
        api::png_malloc_warn(l, std::ptr::null_mut(), usize::MAX).is_null()
    });

    // rows 65-67 - png_free / png_free_default NULL handling
    diff("rows 65-67 png_free NULL handling", |l| unsafe {
        let pp = api::new_writer(l);
        api::png_free(l, pp, std::ptr::null_mut());
        api::png_free(l, std::ptr::null_mut(), std::ptr::null_mut());
        api::png_free_default(l, pp, std::ptr::null_mut());
        api::png_free_default(l, std::ptr::null_mut(), std::ptr::null_mut());
        let p = api::png_malloc(l, pp, 8);
        api::png_free(l, std::ptr::null_mut(), p); // leaks, must not crash
        let q = api::png_malloc(l, pp, 8);
        api::png_free(l, pp, q);
        0u32
    });
}

// ==========================================================================
// pngtrans.c
// ==========================================================================

/// row 280 - `png_set_shift`: "png_set_shift: invalid shift values"
#[test]
fn err_pngtrans_set_shift_row_280() {
    let sh = |r, g, b, gray, alpha| png_color_8 {
        red: r,
        green: g,
        blue: b,
        gray,
        alpha,
    };
    // A fresh write struct has bit_depth == 0 and color_type == 0 (GRAY), so
    // every shift value is out of range.
    for bits in [
        sh(0, 0, 0, 0, 0),
        sh(1, 1, 1, 1, 1),
        sh(8, 8, 8, 8, 8),
        sh(99, 99, 99, 99, 99),
        sh(255, 255, 255, 255, 255),
    ] {
        // row 280
        diff(&format!("row 280 png_set_shift(fresh writer, {bits:?})"), move |l| unsafe {
            let pp = api::new_writer(l);
            api::png_set_shift(l, pp, &bits);
            0u32
        });
        // row 280 - as a warning when app errors are benign
        diff(
            &format!("row 280 png_set_shift(benign=1, {bits:?})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                api::png_set_benign_errors(l, pp, 1);
                api::png_set_shift(l, pp, &bits);
                0u32
            },
        );
    }

    // With a real IHDR in place the valid/invalid boundary is bit_depth.
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ] {
        for bits in [
            sh(0, 0, 0, 0, 0),
            sh(1, 1, 1, 1, 1),
            sh(8, 8, 8, 8, 8),
            sh(9, 8, 8, 8, 8),
            sh(8, 9, 8, 8, 8),
            sh(8, 8, 9, 8, 8),
            sh(8, 8, 8, 9, 8),
            sh(8, 8, 8, 8, 9),
            sh(0, 8, 8, 8, 8),
            sh(8, 8, 8, 8, 0),
        ] {
            // row 280
            diff(
                &format!("row 280 png_set_shift(ct={ct}, bd={bd}, {bits:?})"),
                move |l| unsafe {
                    let pp = api::new_writer(l);
                    sink_reset();
                    api::png_write_IHDR(l, pp, 1, 1, bd, ct, 0, 0, 0);
                    api::png_set_shift(l, pp, &bits);
                    0u32
                },
            );
        }
    }

    // row 280 - NULL arguments are silent no-ops
    diff("row 280 png_set_shift NULL sentinels", |l| unsafe {
        let bits = sh(0, 0, 0, 0, 0);
        let pp = api::new_writer(l);
        api::png_set_shift(l, pp, std::ptr::null());
        api::png_set_shift(l, std::ptr::null_mut(), &bits);
        api::png_set_shift(l, std::ptr::null_mut(), std::ptr::null());
        0u32
    });
}

/// rows 281-284 - `png_set_filler`
///
/// UNREACHABLE (row 281): "png_set_filler not supported on read" is the `#else`
/// arm of `#ifdef PNG_READ_FILLER_SUPPORTED`, which this build defines
/// (include/pnglibconf.h line 64).
///
/// UNREACHABLE (row 284): "png_set_filler not supported on write" is the
/// `#else` arm of `#ifdef PNG_WRITE_FILLER_SUPPORTED`, which this build also
/// defines (include/pnglibconf.h line 142).
#[test]
fn err_pngtrans_set_filler_rows_281_284() {
    // row 282 - write struct, greyscale, bit depth < 8
    for flags in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER, 99, -1, i32::MAX] {
        // row 282 (also: out-of-range filler_loc across the FFI)
        diff(
            &format!("row 282 png_set_filler(fresh writer, flags={flags})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                api::png_set_filler(l, pp, 0xffff_ffff, flags);
                0u32
            },
        );
        diff(
            &format!("row 282 png_set_add_alpha(fresh writer, flags={flags})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                api::png_set_add_alpha(l, pp, 0xffff_ffff, flags);
                0u32
            },
        );
    }
    for bd in [1, 2, 4] {
        // row 282
        diff(&format!("row 282 png_set_filler(gray bd={bd})"), move |l| unsafe {
            let pp = api::new_writer(l);
            sink_reset();
            api::png_write_IHDR(l, pp, 8, 1, bd, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            api::png_set_filler(l, pp, 0, PNG_FILLER_AFTER);
            0u32
        });
    }

    // row 283 - write struct with a colour type that has no filler slot
    for ct in [
        PNG_COLOR_TYPE_PALETTE,
        PNG_COLOR_TYPE_GRAY_ALPHA,
        PNG_COLOR_TYPE_RGB_ALPHA,
    ] {
        // row 283
        diff(&format!("row 283 png_set_filler(ct={ct})"), move |l| unsafe {
            let pp = api::new_writer(l);
            sink_reset();
            api::png_write_IHDR(l, pp, 1, 1, 8, ct, 0, 0, 0);
            api::png_set_filler(l, pp, 1, PNG_FILLER_AFTER);
            0u32
        });
        // row 283 - as a warning when app errors are benign
        diff(
            &format!("row 283 png_set_filler(ct={ct}, benign=1)"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                sink_reset();
                api::png_write_IHDR(l, pp, 1, 1, 8, ct, 0, 0, 0);
                api::png_set_benign_errors(l, pp, 1);
                api::png_set_filler(l, pp, 1, PNG_FILLER_AFTER);
                0u32
            },
        );
    }
    // rows 282, 283 - the accepted write cases must be silent
    for (ct, bd) in [
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
    ] {
        // rows 282, 283
        diff(
            &format!("rows 282,283 png_set_filler ok(ct={ct}, bd={bd})"),
            move |l| unsafe {
                let pp = api::new_writer(l);
                sink_reset();
                api::png_write_IHDR(l, pp, 1, 1, bd, ct, 0, 0, 0);
                api::png_set_filler(l, pp, 0x1234, PNG_FILLER_AFTER);
                0u32
            },
        );
    }
    // row 281 - on a read struct png_set_filler is always accepted, for every
    // filler value and every (even out-of-range) filler_loc.
    for flags in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER, 2, 99, -1, i32::MAX, i32::MIN] {
        // row 281
        diff(
            &format!("row 281 png_set_filler(reader, flags={flags})"),
            move |l| unsafe {
                let pp = api::new_reader(l);
                api::png_set_filler(l, pp, 0xffff_ffff, flags);
                api::png_set_add_alpha(l, pp, 0, flags);
                0u32
            },
        );
    }
    // rows 281-284 - NULL png_ptr
    diff("rows 281-284 png_set_filler(NULL)", |l| unsafe {
        api::png_set_filler(l, std::ptr::null_mut(), 0, 99);
        api::png_set_add_alpha(l, std::ptr::null_mut(), 0, 99);
        0u32
    });
}

/// row 285 - `png_set_user_transform_info`: "info change after
/// png_start_read_image or png_read_update_info"
#[test]
fn err_pngtrans_user_transform_info_row_285() {
    let data = tiny_png();
    // row 285 - after png_start_read_image PNG_FLAG_ROW_INIT is set
    {
        let d = data.clone();
        diff("row 285 after png_start_read_image", move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &d);
            api::png_start_read_image(l, pp);
            api::png_set_user_transform_info(l, pp, 1usize as png_voidp, 8, 1);
            0u32
        });
    }
    // row 285 - after png_read_update_info, likewise
    {
        let d = data.clone();
        diff("row 285 after png_read_update_info", move |l| unsafe {
            let (pp, ip) = reader_at_idat(l, &d);
            api::png_read_update_info(l, pp, ip);
            api::png_set_user_transform_info(l, pp, 1usize as png_voidp, 8, 1);
            0u32
        });
    }
    // row 285 - and as a warning when app errors are benign
    {
        let d = data.clone();
        diff("row 285 after start_read_image, benign=1", move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &d);
            api::png_start_read_image(l, pp);
            api::png_set_benign_errors(l, pp, 1);
            api::png_set_user_transform_info(l, pp, 1usize as png_voidp, 8, 1);
            api::png_get_user_transform_ptr(l, pp).is_null()
        });
    }
    // row 285 - before row init, and on a write struct, out-of-range depths
    // and channel counts are simply truncated to png_byte without complaint.
    for (bd, ch) in [
        (0, 0),
        (8, 1),
        (99, -1),
        (-1, 99),
        (256, 256),
        (i32::MAX, i32::MIN),
    ] {
        // row 285
        diff(
            &format!("row 285 png_set_user_transform_info(bd={bd}, ch={ch})"),
            move |l| unsafe {
                let pw = api::new_writer(l);
                api::png_set_user_transform_info(l, pw, 2usize as png_voidp, bd, ch);
                let pr = api::new_reader(l);
                api::png_set_user_transform_info(l, pr, 3usize as png_voidp, bd, ch);
                (
                    api::png_get_user_transform_ptr(l, pw).is_null(),
                    api::png_get_user_transform_ptr(l, pr).is_null(),
                )
            },
        );
    }
    // row 285 - NULL png_ptr
    diff("row 285 png_set_user_transform_info(NULL)", |l| unsafe {
        api::png_set_user_transform_info(l, std::ptr::null_mut(), 1usize as png_voidp, 8, 1);
        api::png_get_user_transform_ptr(l, std::ptr::null_mut()).is_null()
    });
}

// ==========================================================================
// Exact message texts, one canonical trigger per row.  This is the audit
// trail: if any of these stopped firing in the C reference the assertion
// would fail rather than passing vacuously.
// ==========================================================================

#[test]
fn err_e1_exact_message_texts_all_rows() {
    let data = tiny_png();
    let ihdr = |label: &str,
                expect: &[&str],
                w: png_uint_32,
                h: png_uint_32,
                bd: c_int,
                ct: c_int,
                il: c_int,
                cm: c_int,
                ft: c_int| {
        diff_expect(label, expect, move |l| unsafe {
            let pp = api::new_writer(l);
            api::png_check_IHDR(l, pp, w, h, bd, ct, il, cm, ft);
            0u32
        });
    };

    // row 1
    diff_expect(
        "row 1 text",
        &["ERR(Too many bytes for PNG signature)"],
        |l| unsafe {
            let pp = api::new_reader(l);
            api::png_set_sig_bytes(l, pp, 9);
            0u32
        },
    );
    // row 2 (only the reachable fall-through; the overflow guard itself is
    // dead code on 64-bit - see err_png_c_zalloc_row_2)
    diff_expect("row 2 text", &["WARN(Out of memory)"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_zalloc(l, pp, 0xffff_ffff, 0xffff_ffff).is_null()
    });
    // row 3
    diff_expect(
        "row 3 text",
        &["WARN(Application built with libpng-1.2.3 but running with 1.6.59.git)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let s = cs("1.2.3");
            api::png_user_version_check(l, pp, s.as_ptr())
        },
    );
    // row 4
    diff_expect(
        "row 4 text",
        &["ERR(Unknown freer parameter in png_data_freer)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let ip = api::png_create_info_struct(l, pp);
            api::png_data_freer(l, pp, ip, 99, PNG_FREE_ALL as png_uint_32);
            0u32
        },
    );
    // row 5
    diff_expect("row 5 text", &["WARN(Ignoring invalid time value)"], |l| unsafe {
        let pp = api::new_writer(l);
        let t = png_time { year: 2000, month: 13, day: 1, hour: 0, minute: 0, second: 0 };
        api::png_convert_to_rfc1123(l, pp, &t).is_null()
    });
    // row 6: UNREACHABLE (documented in err_png_c_set_rgb_coefficients_row_6)
    // row 7
    ihdr(
        "row 7 text",
        &["WARN(Image width is zero in IHDR)", "ERR(Invalid IHDR data)"],
        0, 1, 8, 0, 0, 0, 0,
    );
    // row 8
    ihdr(
        "row 8 text",
        &[
            "WARN(Invalid image width in IHDR)",
            "WARN(Image width exceeds user limit in IHDR)",
            "ERR(Invalid IHDR data)",
        ],
        0x8000_0000, 1, 8, 0, 0, 0, 0,
    );
    // row 9: UNREACHABLE on a 64-bit target (see err_png_c_check_ihdr_rows_7_23)
    // row 10
    ihdr(
        "row 10 text",
        &["WARN(Image width exceeds user limit in IHDR)", "ERR(Invalid IHDR data)"],
        1_000_001, 1, 8, 0, 0, 0, 0,
    );
    // row 11
    ihdr(
        "row 11 text",
        &["WARN(Image height is zero in IHDR)", "ERR(Invalid IHDR data)"],
        1, 0, 8, 0, 0, 0, 0,
    );
    // row 12
    ihdr(
        "row 12 text",
        &[
            "WARN(Invalid image height in IHDR)",
            "WARN(Image height exceeds user limit in IHDR)",
            "ERR(Invalid IHDR data)",
        ],
        1, 0x8000_0000, 8, 0, 0, 0, 0,
    );
    // row 13
    ihdr(
        "row 13 text",
        &["WARN(Image height exceeds user limit in IHDR)", "ERR(Invalid IHDR data)"],
        1, 1_000_001, 8, 0, 0, 0, 0,
    );
    // row 14
    ihdr(
        "row 14 text",
        &["WARN(Invalid bit depth in IHDR)", "ERR(Invalid IHDR data)"],
        1, 1, 3, 0, 0, 0, 0,
    );
    // row 15
    ihdr(
        "row 15 text",
        &["WARN(Invalid color type in IHDR)", "ERR(Invalid IHDR data)"],
        1, 1, 8, 5, 0, 0, 0,
    );
    // row 16
    ihdr(
        "row 16 text",
        &[
            "WARN(Invalid color type/bit depth combination in IHDR)",
            "ERR(Invalid IHDR data)",
        ],
        1, 1, 16, PNG_COLOR_TYPE_PALETTE, 0, 0, 0,
    );
    // row 17
    ihdr(
        "row 17 text",
        &["WARN(Unknown interlace method in IHDR)", "ERR(Invalid IHDR data)"],
        1, 1, 8, 0, 2, 0, 0,
    );
    // row 18
    ihdr(
        "row 18 text",
        &["WARN(Unknown compression method in IHDR)", "ERR(Invalid IHDR data)"],
        1, 1, 8, 0, 0, 1, 0,
    );
    // row 19
    diff_expect(
        "row 19 text",
        &["WARN(MNG features are not allowed in a PNG datastream)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            sink_reset();
            api::png_write_sig(l, pp);
            api::png_permit_mng_features(l, pp, PNG_ALL_MNG_FEATURES as png_uint_32);
            api::png_check_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            0u32
        },
    );
    // row 20
    ihdr(
        "row 20 text",
        &["WARN(Unknown filter method in IHDR)", "ERR(Invalid IHDR data)"],
        1, 1, 8, 0, 0, 0, 1,
    );
    // row 21 (always accompanied by row 20)
    diff_expect(
        "row 21 text",
        &[
            "WARN(Unknown filter method in IHDR)",
            "WARN(Invalid filter method in IHDR)",
            "ERR(Invalid IHDR data)",
        ],
        |l| unsafe {
            let pp = api::new_writer(l);
            sink_reset();
            api::png_write_sig(l, pp);
            api::png_check_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB, 0, 0, 1);
            0u32
        },
    );
    // row 22: UNREACHABLE (#else of PNG_MNG_FEATURES_SUPPORTED)
    // row 23
    ihdr(
        "row 23 text",
        &["WARN(Invalid bit depth in IHDR)", "ERR(Invalid IHDR data)"],
        1, 1, 0, 0, 0, 0, 0,
    );
    // row 24
    diff_expect(
        "row 24 text",
        &["ERR(ASCII conversion buffer too small)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let mut buf = [0u8; 128];
            api::png_ascii_from_fp(l, pp, buf.as_mut_ptr() as png_charp, 1, 1.0, 5);
            0u32
        },
    );
    // row 25
    diff_expect(
        "row 25 text",
        &["ERR(ASCII conversion buffer too small)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let mut buf = [0u8; 128];
            api::png_ascii_from_fixed(l, pp, buf.as_mut_ptr() as png_charp, 12, 100000);
            0u32
        },
    );
    // row 26
    diff_expect(
        "row 26 text",
        &["ERR(fixed point overflow in sCAL width)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let n = cs("sCAL width");
            api::png_fixed(l, pp, 1e30, n.as_ptr())
        },
    );
    // row 27
    diff_expect(
        "row 27 text",
        &["ERR(fixed point overflow in cLLI maxCLL)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let n = cs("cLLI maxCLL");
            api::png_fixed_ITU(l, pp, -1.0, n.as_ptr())
        },
    );
    // row 28: NOT A REJECTION SITE (png.c:3377 is inside a comment)
    // row 29
    diff_expect("row 29 text", &["WARN(gamma table being rebuilt)"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_build_gamma_table(l, pp, 8);
        api::png_build_gamma_table(l, pp, 8);
        api::png_destroy_gamma_table(l, pp);
        0u32
    });
    // rows 30, 31: UNREACHABLE (#else of PNG_SIMPLIFIED_{WRITE,READ}_SUPPORTED)
    // row 32
    diff_expect("row 32 text", &["WARN(hello)"], |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("hello");
        api::png_warning(l, pp, m.as_ptr());
        0u32
    });
    // row 33
    diff_expect("row 33 text", &["WARN(one AAA param)"], |l| unsafe {
        let pp = api::new_writer(l);
        let mut p = [0 as c_char; PNG_WARNING_PARAMETER_SIZE * PNG_WARNING_PARAMETER_COUNT];
        let a = cs("AAA");
        api::png_warning_parameter(l, p.as_mut_ptr(), 1, a.as_ptr());
        let m = cs("one @1 param");
        api::png_formatted_warning(l, pp, p.as_mut_ptr(), m.as_ptr());
        0u32
    });
    // rows 34, 38
    diff_expect("rows 34,38 text", &["ERR(benign boom)"], |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("benign boom");
        api::png_benign_error(l, pp, m.as_ptr());
        0u32
    });
    // rows 34, 35
    {
        let d = data.clone();
        diff_expect("rows 34,35 text", &["WARN(IDAT: benign boom)"], move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &d);
            let m = cs("benign boom");
            api::png_benign_error(l, pp, m.as_ptr());
            0u32
        });
    }
    // rows 34, 36
    diff_expect("rows 34,36 text", &["WARN(benign boom)"], |l| unsafe {
        let pp = api::new_reader(l);
        let m = cs("benign boom");
        api::png_benign_error(l, pp, m.as_ptr());
        0u32
    });
    // rows 34, 37
    {
        let d = data.clone();
        diff_expect("rows 34,37 text", &["ERR(IDAT: benign boom)"], move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &d);
            api::png_set_benign_errors(l, pp, 0);
            let m = cs("benign boom");
            api::png_benign_error(l, pp, m.as_ptr());
            0u32
        });
    }
    // rows 39, 41
    diff_expect("rows 39,41 text", &["ERR(app warning)"], |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("app warning");
        api::png_app_warning(l, pp, m.as_ptr());
        0u32
    });
    // rows 39, 40
    diff_expect("rows 39,40 text", &["WARN(app warning)"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_benign_errors(l, pp, 1);
        let m = cs("app warning");
        api::png_app_warning(l, pp, m.as_ptr());
        0u32
    });
    // rows 42, 44
    diff_expect("rows 42,44 text", &["ERR(app error)"], |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("app error");
        api::png_app_error(l, pp, m.as_ptr());
        0u32
    });
    // rows 42, 43
    diff_expect("rows 42,43 text", &["WARN(app error)"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_benign_errors(l, pp, 1);
        let m = cs("app error");
        api::png_app_error(l, pp, m.as_ptr());
        0u32
    });
    // row 45: UNREACHABLE without abort()ing the process
    // row 46
    diff_expect("row 46 text", &["ERR([00][00][00][00]: chunk boom)"], |l| unsafe {
        let pp = api::new_reader(l);
        let m = cs("chunk boom");
        api::png_chunk_error(l, pp, m.as_ptr());
        0u32
    });
    {
        let d = data.clone();
        // row 46
        diff_expect("row 46 text (IDAT)", &["ERR(IDAT: chunk boom)"], move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &d);
            let m = cs("chunk boom");
            api::png_chunk_error(l, pp, m.as_ptr());
            0u32
        });
    }
    // rows 47, 49
    diff_expect(
        "rows 47,49 text",
        &["WARN([00][00][00][00]: chunk boom)"],
        |l| unsafe {
            let pp = api::new_reader(l);
            let m = cs("chunk boom");
            api::png_chunk_warning(l, pp, m.as_ptr());
            0u32
        },
    );
    // row 48 (the message goes to the default handler, i.e. stderr, so the
    // recorded log is empty - what matters is that it does not error)
    diff_expect("row 48 text", &[], |l| unsafe {
        let m = cs("chunk boom");
        api::png_chunk_warning(l, std::ptr::null_mut(), m.as_ptr());
        0u32
    });
    // rows 52, 50
    {
        let d = data.clone();
        diff_expect("row 50 text", &["WARN(IDAT: chunk benign)"], move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &d);
            let m = cs("chunk benign");
            api::png_chunk_benign_error(l, pp, m.as_ptr());
            0u32
        });
    }
    // row 51
    {
        let d = data.clone();
        diff_expect("row 51 text", &["ERR(IDAT: chunk benign)"], move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &d);
            api::png_set_benign_errors(l, pp, 0);
            let m = cs("chunk benign");
            api::png_chunk_benign_error(l, pp, m.as_ptr());
            0u32
        });
    }
    // rows 52, 53
    {
        let d = data.clone();
        diff_expect("rows 52,53 text", &["WARN(IDAT: report)"], move |l| unsafe {
            let (pp, _ip) = reader_at_idat(l, &d);
            let m = cs("report");
            api::png_chunk_report(l, pp, m.as_ptr(), PNG_CHUNK_WARNING);
            0u32
        });
    }
    // rows 52, 54
    diff_expect("rows 52,54 text", &["ERR(report)"], |l| unsafe {
        let pp = api::new_writer(l);
        let m = cs("report");
        api::png_chunk_report(l, pp, m.as_ptr(), PNG_CHUNK_WARNING);
        0u32
    });
    // rows 52, 55
    diff_expect("rows 52,55 text", &["WARN(report)"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_benign_errors(l, pp, 1);
        let m = cs("report");
        api::png_chunk_report(l, pp, m.as_ptr(), PNG_CHUNK_WRITE_ERROR);
        0u32
    });
    // row 56 - the name is truncated to PNG_MAX_ERROR_TEXT-1 == 195 bytes
    diff_expect(
        "row 56 text",
        &[&format!("ERR(fixed point overflow in {})", "x".repeat(195))],
        |l| unsafe {
            let pp = api::new_writer(l);
            let n = cs(&"x".repeat(300));
            api::png_fixed_error(l, pp, n.as_ptr());
            0u32
        },
    );
    // row 57: UNREACHABLE (internal-error-only state)
    // row 58
    diff_expect(
        "row 58 text",
        &["WARN(Application jmp_buf size changed)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let f = fake_longjmp_ptr();
            let a = api::png_set_longjmp_fn(l, pp, f, 1024);
            let b = api::png_set_longjmp_fn(l, pp, f, 1023);
            (a.is_null(), b.is_null())
        },
    );
    // row 59
    diff_expect("row 59 text", &["ERR(harness: longjmp(1))"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_longjmp_fn(l, pp, fake_longjmp_ptr(), 8);
        api::png_set_error_fn(l, pp, std::ptr::null_mut(), None, Some(rec_warning));
        let m = cs("default error path");
        api::png_error(l, pp, m.as_ptr());
        0u32
    });
    // row 60
    diff_expect("row 60 text", &["ERR(harness: longjmp(7))"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_set_longjmp_fn(l, pp, fake_longjmp_ptr(), 8);
        png_longjmp(l, pp, 7);
        0u32
    });
    // row 61
    diff_expect(
        "row 61 text",
        &["WARN(fixed point overflow ignored)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let ip = api::png_create_info_struct(l, pp);
            api::png_set_oFFs(l, pp, ip, 600_000_000, 0, PNG_OFFSET_MICROMETER);
            api::png_get_x_offset_inches_fixed(l, pp, ip)
        },
    );
    // row 62
    diff_expect(
        "row 62 text",
        &["WARN(png_get_eXIf does not work; use png_get_eXIf_1)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let ip = api::png_create_info_struct(l, pp);
            let mut exif: png_bytep = std::ptr::null_mut();
            api::png_get_eXIf(l, pp, ip, &mut exif)
        },
    );
    // row 63
    diff_expect("row 63 text", &["ERR(internal error: array alloc)"], |l| unsafe {
        let pp = api::new_writer(l);
        png_malloc_array(l, pp, 0, 4).is_null()
    });
    // row 64
    diff_expect("row 64 text", &["ERR(internal error: array realloc)"], |l| unsafe {
        let pp = api::new_writer(l);
        png_realloc_array(l, pp, std::ptr::null(), 0, 0, 4).is_null()
    });
    // row 65
    diff_expect("row 65 text", &["ERR(Out of memory)"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_malloc(l, pp, usize::MAX).is_null()
    });
    // row 66
    diff_expect("row 66 text", &["ERR(Out of Memory)"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_malloc_default(l, pp, usize::MAX).is_null()
    });
    // row 67
    diff_expect("row 67 text", &["WARN(Out of memory)"], |l| unsafe {
        let pp = api::new_writer(l);
        api::png_malloc_warn(l, pp, usize::MAX).is_null()
    });
    // row 280
    diff_expect(
        "row 280 text",
        &["ERR(png_set_shift: invalid shift values)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            let bits = png_color_8::default();
            api::png_set_shift(l, pp, &bits);
            0u32
        },
    );
    // row 281: UNREACHABLE (#else of PNG_READ_FILLER_SUPPORTED)
    // row 282
    diff_expect(
        "row 282 text",
        &["ERR(png_set_filler is invalid for low bit depth gray output)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            api::png_set_filler(l, pp, 0, PNG_FILLER_AFTER);
            0u32
        },
    );
    // row 283
    diff_expect(
        "row 283 text",
        &["ERR(png_set_filler: inappropriate color type)"],
        |l| unsafe {
            let pp = api::new_writer(l);
            sink_reset();
            api::png_write_IHDR(l, pp, 1, 1, 8, PNG_COLOR_TYPE_RGB_ALPHA, 0, 0, 0);
            api::png_set_filler(l, pp, 0, PNG_FILLER_AFTER);
            0u32
        },
    );
    // row 284: UNREACHABLE (#else of PNG_WRITE_FILLER_SUPPORTED)
    // row 285
    {
        let d = data.clone();
        diff_expect(
            "row 285 text",
            &["ERR(info change after png_start_read_image or png_read_update_info)"],
            move |l| unsafe {
                let (pp, _ip) = reader_at_idat(l, &d);
                api::png_start_read_image(l, pp);
                api::png_set_user_transform_info(l, pp, 1usize as png_voidp, 8, 1);
                0u32
            },
        );
    }
}

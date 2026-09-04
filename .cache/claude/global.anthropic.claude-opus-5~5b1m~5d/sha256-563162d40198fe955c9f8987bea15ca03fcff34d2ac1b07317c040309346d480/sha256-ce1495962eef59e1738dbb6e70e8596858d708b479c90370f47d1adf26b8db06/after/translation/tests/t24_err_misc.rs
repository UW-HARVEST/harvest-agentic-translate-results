//! Phase C — the "misc" rejection surface: `png.c`, `pngerror.c`, `pngmem.c`,
//! `pngrio.c` and `pngwio.c` rows that `tests/t23_err_write.rs` does not cover.
//!
//! Two mechanisms are used, chosen per row from the C source:
//!
//!  * **In-process** for every helper that reports a failure through a
//!    *sentinel* return value (`png_muldiv`, `png_reciprocal*`,
//!    `png_XYZ_from_xy`, `png_xy_from_XYZ`, `png_convert_to_rfc1123_buffer`,
//!    `png_malloc_array`/`png_realloc_array` NULL returns, `png_zalloc`,
//!    `png_icc_check_*`, `png_check_keyword`, `png_handle_as_unknown`,
//!    `png_chunk_unknown_handling`, `png_reset_zstream`,
//!    `png_user_version_check`) or through a *warning*.  Warnings are captured
//!    by `warn_cb` into a thread-local transcript and compared string by
//!    string, so the message text and its order are part of the comparison.
//!    Note that on a **read** struct `PNG_FLAG_BENIGN_ERRORS_WARN` is set by
//!    `png_create_read_struct` (`pngread.c:62`), so every
//!    `png_chunk_benign_error` — which is what all the iCCP profile rejections
//!    use — is a *warning* and can therefore be observed in-process.
//!
//!  * **Sub-process** (`diff_case`) for every row that reaches `png_error`:
//!    `"ASCII conversion buffer too small"`, `"fixed point overflow in ..."`,
//!    `"internal error: array alloc"/"... realloc"`, `"Invalid IHDR data"`,
//!    `"Call to NULL read/write function"`, `"Read Error"`, `"Write Error"`.
//!
//! ### `png_zstream_error` (item 5) — how it is observed
//!
//! `png_ptr->zstream.msg` is not reachable from outside the library (the
//! `png_struct` layout is private and is not required to agree between the two
//! builds), so the ten message strings cannot be read back directly.  Injecting
//! a message with `png_zstream_error` and then provoking a diagnostic that
//! prints `zstream.msg` does not work either: **every** libpng path that ends in
//! `png_error(png_ptr, png_ptr->zstream.msg)` first calls
//! `deflateInit2`/`deflateReset`/`inflateInit2`/`inflateReset2`, and all four
//! zlib entry points assign `strm->msg = Z_NULL` before doing anything else, so
//! the injected string is always destroyed.  (The one branch that reports
//! `zstream.msg` without touching zlib, `pngrutil.c:824`, is unreachable because
//! `png_read_buffer` at `pngrutil.c:380` rejects the chunk first — its limit is
//! strictly larger.)
//!
//! Therefore this file uses the fallback the task allows:
//!   1. `png_zstream_error` is *called directly* with every zlib return code and
//!      several out-of-range codes on both a read and a write struct; the test
//!      asserts both libraries agree that the call is silent (no warning, no
//!      error) and that the struct is still usable afterwards
//!      (`png_reset_zstream` returns the same code).
//!   2. the *reachable* messages are driven through real corrupt-zlib streams
//!      (bad zlib header, `FDICT` set, damaged deflate data, truncated data,
//!      trailing garbage) and through the write-side `deflateInit2` failure that
//!      `png_set_compression_strategy` can force, and the resulting chunk
//!      warning / fatal error text is compared.
//! `see zstream_error_reachable_messages` below for the list of strings that are
//! actually observable; `Z_ERRNO` ("zlib IO error") and `Z_VERSION_ERROR`
//! ("unsupported zlib version") are not producible by any input because zlib
//! never returns them here (gz-API only / version mismatch only).
#![allow(clippy::too_many_arguments)]
#![allow(non_snake_case)]

mod common;

use common::api::{apis, Api};
use common::harness::*;
use common::pngbuild as pb;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// repr(C) mirrors of the two private colour-space structs (see t01_low_level.rs)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_xy {
    pub redx: i32,
    pub redy: i32,
    pub greenx: i32,
    pub greeny: i32,
    pub bluex: i32,
    pub bluey: i32,
    pub whitex: i32,
    pub whitey: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_XYZ {
    pub red_X: i32,
    pub red_Y: i32,
    pub red_Z: i32,
    pub green_X: i32,
    pub green_Y: i32,
    pub green_Z: i32,
    pub blue_X: i32,
    pub blue_Y: i32,
    pub blue_Z: i32,
}

type FnXYZfromxy = unsafe extern "C" fn(*mut png_XYZ, *const png_xy) -> c_int;
type FnxyfromXYZ = unsafe extern "C" fn(*mut png_xy, *const png_XYZ) -> c_int;

// `png_icc_check_*` are internal and are not in the generated `common::api`
// table, so they are resolved by name here (the task forbids editing api.rs).
type FnIccLength =
    unsafe extern "C" fn(png_structp, *const c_char, png_uint_32) -> c_int;
type FnIccHeader = unsafe extern "C" fn(
    png_structp,
    *const c_char,
    png_uint_32,
    *const png_byte,
    c_int,
) -> c_int;
type FnIccTags = unsafe extern "C" fn(
    png_structp,
    *const c_char,
    png_uint_32,
    *const png_byte,
) -> c_int;

/// `png_warning_parameters` from pngpriv.h: `char [8][32]`.
type WarnParams = [[c_char; 32]; 8];

// ---------------------------------------------------------------------------
// struct helpers
// ---------------------------------------------------------------------------

unsafe fn new_write(a: &Api) -> (png_structp, png_infop) {
    let p = (a.png_create_write_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null());
    let info = (a.png_create_info_struct)(p);
    assert!(!info.is_null());
    (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
    (p, info)
}

unsafe fn new_read(a: &Api) -> (png_structp, png_infop) {
    let p = (a.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null());
    let info = (a.png_create_info_struct)(p);
    assert!(!info.is_null());
    (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
    (p, info)
}

/// `fopen`/`fclose` for the two `png_default_*_data` rows.  They are resolved
/// from libc rather than reimplemented because the FILE* has to be a genuine
/// stdio stream.
fn libc() -> &'static libloading::Library {
    use std::sync::OnceLock;
    static L: OnceLock<libloading::Library> = OnceLock::new();
    L.get_or_init(|| unsafe {
        libloading::Library::new("libc.so.6")
            .or_else(|_| libloading::Library::new("libc.so"))
            .expect("libc")
    })
}

unsafe fn c_fopen(path: &str, mode: &str) -> *mut c_void {
    type F = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void;
    let s: libloading::Symbol<F> = libc().get(b"fopen\0").unwrap();
    let p = std::ffi::CString::new(path).unwrap();
    let m = std::ffi::CString::new(mode).unwrap();
    s(p.as_ptr(), m.as_ptr())
}

// ---------------------------------------------------------------------------
// value tables shared by parent and child (indices travel in the case name)
// ---------------------------------------------------------------------------

fn fp_values() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        1e300,
        -1e300,
        1e-300,
        1.0 / 3.0,
        99999.99999,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN_POSITIVE,
        12345.6789,
        -12345.6789,
    ]
}

fn fixed_values() -> Vec<i32> {
    vec![
        0,
        1,
        -1,
        9,
        10,
        99999,
        100000,
        -100000,
        12345,
        1_000_000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        2_000_000_000,
        -2_000_000_000,
    ]
}

fn hex(bytes: &[c_char]) -> String {
    let mut s = String::new();
    for b in bytes {
        s.push_str(&format!("{:02x}", *b as u8));
    }
    s
}

// ---------------------------------------------------------------------------
// ICC profile construction
// ---------------------------------------------------------------------------

const D50: [u8; 12] = [0, 0, 0xf6, 0xd6, 0, 1, 0, 0, 0, 0, 0xd3, 0x2d];

/// A minimal ICC profile header that passes every `png_icc_check_header` test
/// for an RGB (colour) PNG, followed by `tag_count` zeroed 12-byte tag entries.
fn base_icc(tag_count: u32) -> Vec<u8> {
    let total = 132 + 12 * tag_count as usize;
    let mut p = vec![0u8; total];
    p[0..4].copy_from_slice(&(total as u32).to_be_bytes());
    p[4..8].copy_from_slice(b"ADBE");
    p[8] = 2; // major version 2 (<= 3, so the 4-byte alignment rule is skipped)
    p[9] = 0x10;
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(b"RGB ");
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p[64..68].copy_from_slice(&0u32.to_be_bytes());
    p[68..80].copy_from_slice(&D50);
    p[128..132].copy_from_slice(&tag_count.to_be_bytes());
    p
}

fn put32(p: &mut [u8], off: usize, v: u32) {
    p[off..off + 4].copy_from_slice(&v.to_be_bytes());
}

/// One ICC test case: a profile buffer, the `profile_length` argument, the PNG
/// colour type, and whether the tag table may be walked (only when the buffer
/// really contains `132 + 12*tag_count` bytes).
struct IccCase {
    label: String,
    profile: Vec<u8>,
    length: u32,
    color_type: c_int,
    tags_ok: bool,
}

fn icc_cases() -> Vec<IccCase> {
    let mut v: Vec<IccCase> = Vec::new();
    let mut push = |label: &str, profile: Vec<u8>, length: u32, ct: c_int, tags_ok: bool| {
        v.push(IccCase {
            label: label.to_string(),
            profile,
            length,
            color_type: ct,
            tags_ok,
        });
    };

    // ---- baseline (must be accepted) ----
    push("valid-rgb-0tags", base_icc(0), 132, PNG_COLOR_TYPE_RGB, true);
    push("valid-rgb-3tags", base_icc(3), 168, PNG_COLOR_TYPE_RGB, true);
    {
        let mut p = base_icc(0);
        put32(&mut p, 16, 0x47524159); // 'GRAY'
        push("valid-gray", p, 132, PNG_COLOR_TYPE_GRAY, true);
    }
    {
        let mut p = base_icc(0);
        put32(&mut p, 20, 0x4c616220); // 'Lab '
        push("valid-pcs-Lab", p, 132, PNG_COLOR_TYPE_RGB, true);
    }
    for cls in [b"scnr", b"mntr", b"prtr", b"spac"] {
        let mut p = base_icc(0);
        p[12..16].copy_from_slice(cls);
        push(
            &format!("valid-class-{}", std::str::from_utf8(cls).unwrap()),
            p,
            132,
            PNG_COLOR_TYPE_RGB,
            true,
        );
    }

    // ---- icc_check_length: "too short" (png.c:1588) ----
    for l in [0u32, 1, 4, 63, 127, 128, 131] {
        // the header check is not run for these (the buffer is short), only the
        // length check, so the profile buffer is a full valid one and only the
        // `profile_length` argument is small.
        push(
            &format!("len-too-short-{l}"),
            base_icc(0),
            l,
            PNG_COLOR_TYPE_RGB,
            false,
        );
    }

    // ---- png_icc_check_header: "length does not match profile" (png.c:1626) ----
    for declared in [0u32, 1, 131, 132, 133, 168, 0xffff_ffff] {
        let mut p = base_icc(0);
        put32(&mut p, 0, declared);
        push(
            &format!("hdr-length-field-{declared}"),
            p,
            132,
            PNG_COLOR_TYPE_RGB,
            true,
        );
    }

    // ---- "invalid length": profile[8] > 3 and length not a multiple of 4 ----
    for major in [0u8, 2, 3, 4, 5, 255] {
        for extra in [0usize, 1, 2, 3] {
            let mut p = base_icc(0);
            p.extend(std::iter::repeat(0).take(extra));
            let n = p.len() as u32;
            put32(&mut p, 0, n);
            p[8] = major;
            push(
                &format!("hdr-major{major}-len{n}"),
                p,
                n,
                PNG_COLOR_TYPE_RGB,
                false,
            );
        }
    }

    // ---- "tag count too large" (png.c:1636 / 1637) ----
    for tc in [
        0u32,
        1,
        2,
        3,
        4,
        357913930,
        357913931,
        0x7fff_ffff,
        0xffff_ffff,
    ] {
        let mut p = base_icc(0);
        put32(&mut p, 128, tc);
        push(
            &format!("hdr-tagcount-{tc}"),
            p,
            132,
            PNG_COLOR_TYPE_RGB,
            false,
        );
    }
    // a declared tag count that exactly fits / just overflows the length
    for (tc, len) in [(1u32, 144u32), (1, 143), (2, 156), (2, 155), (3, 168), (3, 167)] {
        let mut p = base_icc(0);
        p.resize(len as usize, 0);
        put32(&mut p, 0, len);
        put32(&mut p, 128, tc);
        push(
            &format!("hdr-tagcount{tc}-len{len}"),
            p,
            len,
            PNG_COLOR_TYPE_RGB,
            len >= 132 + 12 * tc,
        );
    }

    // ---- rendering intent (png.c:1645 error, png.c:1652 warning) ----
    for intent in [0u32, 1, 2, 3, 4, 5, 0xfffe, 0xffff, 0x1_0000, 0xffff_ffff] {
        let mut p = base_icc(0);
        put32(&mut p, 64, intent);
        push(
            &format!("hdr-intent-{intent}"),
            p,
            132,
            PNG_COLOR_TYPE_RGB,
            true,
        );
    }

    // ---- 'acsp' signature at offset 36 (png.c:1669) ----
    for sig in [
        0x61637370u32,
        0x61637371,
        0,
        0xffff_ffff,
        0x41435350,
        0x20202020,
    ] {
        let mut p = base_icc(0);
        put32(&mut p, 36, sig);
        push(
            &format!("hdr-signature-{sig:08x}"),
            p,
            132,
            PNG_COLOR_TYPE_RGB,
            true,
        );
    }

    // ---- PCS illuminant at 68..80 (png.c:1680, warning only) ----
    for i in 0..12usize {
        let mut p = base_icc(0);
        p[68 + i] ^= 0xff;
        push(
            &format!("hdr-illuminant-byte{i}"),
            p,
            132,
            PNG_COLOR_TYPE_RGB,
            true,
        );
    }
    {
        let mut p = base_icc(0);
        for i in 68..80 {
            p[i] = 0;
        }
        push("hdr-illuminant-zero", p, 132, PNG_COLOR_TYPE_RGB, true);
    }

    // ---- data colour space at 16 x colour type (png.c:1708/1714/1719) ----
    for space in [
        0x52474220u32, // 'RGB '
        0x47524159,    // 'GRAY'
        0x434d594bu32, // 'CMYK'
        0x4c616220,    // 'Lab '
        0,
        0xffff_ffff,
    ] {
        for ct in [
            PNG_COLOR_TYPE_GRAY,
            PNG_COLOR_TYPE_RGB,
            PNG_COLOR_TYPE_PALETTE,
            PNG_COLOR_TYPE_GRAY_ALPHA,
            PNG_COLOR_TYPE_RGB_ALPHA,
        ] {
            let mut p = base_icc(0);
            put32(&mut p, 16, space);
            push(
                &format!("hdr-space-{space:08x}-ct{ct}"),
                p,
                132,
                ct,
                true,
            );
        }
    }

    // ---- device class at 12 (png.c:1743/1748/1758/1767) ----
    for cls in [
        0x73636e72u32, // 'scnr'
        0x6d6e7472,    // 'mntr'
        0x70727472,    // 'prtr'
        0x73706163,    // 'spac'
        0x61627374,    // 'abst'
        0x6c696e6b,    // 'link'
        0x6e6d636c,    // 'nmcl'
        0x00000000,
        0xffff_ffff,
        0x41424344, // 'ABCD'
        0x7f7f7f7f,
    ] {
        let mut p = base_icc(0);
        put32(&mut p, 12, cls);
        push(
            &format!("hdr-class-{cls:08x}"),
            p,
            132,
            PNG_COLOR_TYPE_RGB,
            true,
        );
    }

    // ---- PCS at 20 (png.c:1788) ----
    for pcs in [
        0x58595a20u32, // 'XYZ '
        0x4c616220,    // 'Lab '
        0x52474220,    // 'RGB '
        0,
        0xffff_ffff,
    ] {
        let mut p = base_icc(0);
        put32(&mut p, 20, pcs);
        push(
            &format!("hdr-pcs-{pcs:08x}"),
            p,
            132,
            PNG_COLOR_TYPE_RGB,
            true,
        );
    }

    // ---- tag table (png.c:1824 error, png.c:1828 warning) ----
    for &(start, tlen, tag) in &[
        (132u32, 12u32, 0x41424344u32),
        (168, 0, 0x41424344),
        // start <= profile_length but not a multiple of 4 -> warning only
        (1, 0, 0x41424344),
        (2, 0, 0x00010203), // non-printable tag id -> '?' substitution
        (3, 0, 0x7e7f8081),
        (133, 0, 0x41424344),
        (134, 4, 0x41424344),
        (135, 1, 0x41424344),
        (165, 3, 0x41424344),
        (166, 2, 0x20202020),
        (167, 1, 0x41424344),
        // tag_start / tag_length outside the profile -> hard error
        (168, 1, 0x41424344),
        (169, 12, 0x41424344),
        (0xffff_ffff, 0, 0x41424344),
        (0, 0xffff_ffff, 0x41424344),
        (132, 36, 0x41424344),
        (132, 37, 0x41424344),
        (16, 8, 0x20202020),
    ] {
        let mut p = base_icc(3);
        let n = p.len() as u32; // 168
        put32(&mut p, 0, n);
        put32(&mut p, 132, tag);
        put32(&mut p, 136, start);
        put32(&mut p, 140, tlen);
        push(
            &format!("tags-start{start}-len{tlen}"),
            p,
            n,
            PNG_COLOR_TYPE_RGB,
            true,
        );
    }
    // several tags, the second of which is bad
    {
        let mut p = base_icc(4);
        let n = p.len() as u32;
        put32(&mut p, 0, n);
        for i in 0..4u32 {
            put32(&mut p, 132 + 12 * i as usize, 0x74616730 + i);
            put32(&mut p, 136 + 12 * i as usize, 132 + 4 * i);
            put32(&mut p, 140 + 12 * i as usize, 0);
        }
        put32(&mut p, 136 + 12, 133); // misaligned -> warning
        put32(&mut p, 136 + 24, n + 1); // outside -> hard error
        push("tags-mixed", p, n, PNG_COLOR_TYPE_RGB, true);
    }

    v
}

/// A PNG carrying a crafted `iCCP` chunk.
fn iccp_png(profile: &[u8], color_type: u8) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(b"icc");
    d.push(0); // keyword terminator
    d.push(0); // compression method 0
    d.extend_from_slice(&pb::zlib_store(profile));
    let mut spec = pb::PngSpec::new(2, 2, 8, color_type, 0);
    if color_type == 3 {
        spec.palette = vec![0u8; 3 * 4];
    }
    spec.pre_idat.push((*b"iCCP", d));
    spec.raw = pb::raw_rows_none(2, 2, 8, color_type, &mut |_y, rb| vec![0u8; rb]);
    spec.build()
}

// ---------------------------------------------------------------------------
// PNG builders for the zlib-failure reads
// ---------------------------------------------------------------------------

/// A PNG whose `zTXt` chunk carries `payload` as its (broken) zlib stream.
fn ztxt_png(payload: &[u8]) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(b"Key");
    d.push(0);
    d.push(0); // compression method
    d.extend_from_slice(payload);
    let mut spec = pb::PngSpec::new(2, 2, 8, 2, 0);
    spec.pre_idat.push((*b"zTXt", d));
    spec.raw = pb::raw_rows_none(2, 2, 8, 2, &mut |_y, rb| vec![0u8; rb]);
    spec.build()
}

/// A PNG whose IDAT is `payload` verbatim.
fn idat_png(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&pb::PNG_SIG);
    pb::push_chunk(&mut out, b"IHDR", &pb::ihdr_data(2, 2, 8, 2, 0, 0, 0));
    pb::push_chunk(&mut out, b"IDAT", payload);
    pb::push_chunk(&mut out, b"IEND", &[]);
    out
}

// ---------------------------------------------------------------------------
// the child: performs one named case against one library
// ---------------------------------------------------------------------------

fn run_case(a: &Api, case: &str) {
    unsafe {
        let f: Vec<&str> = case.split(':').collect();
        match f[0] {
            // ---------------- png_ascii_from_fp ----------------
            // afp:<size>:<value index>:<precision>
            "afp" => {
                let size: usize = f[1].parse().unwrap();
                let vi: usize = f[2].parse().unwrap();
                let prec: u32 = f[3].parse().unwrap();
                let v = fp_values()[vi];
                let (p, _i) = new_write(a);
                // The real allocation is 128 bytes; only `size` is small, so a
                // buffer overrun in the test is impossible.
                let mut buf = [0x41u8 as c_char; 128];
                (a.png_ascii_from_fp)(p, buf.as_mut_ptr(), size, v, prec);
                emit(format!("afp ok buf={}", hex(&buf[..48])));
            }
            // afx:<size>:<value index>
            "afx" => {
                let size: usize = f[1].parse().unwrap();
                let vi: usize = f[2].parse().unwrap();
                let v = fixed_values()[vi];
                let (p, _i) = new_write(a);
                let mut buf = [0x41u8 as c_char; 128];
                (a.png_ascii_from_fixed)(p, buf.as_mut_ptr(), size, v);
                emit(format!("afx ok buf={}", hex(&buf[..32])));
            }

            // ---------------- png_fixed / png_fixed_ITU ----------------
            // fx:<f64 bits hex>:<text index>
            "fx" => {
                let bits = u64::from_str_radix(f[1], 16).unwrap();
                let v = f64::from_bits(bits);
                let (p, _i) = new_write(a);
                let text: &[u8] = match f[2] {
                    "0" => b"cHRM White X\0",
                    "1" => b"\0",
                    _ => b"a-very-long-name-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\0",
                };
                let r = (a.png_fixed)(p, v, text.as_ptr() as *const c_char);
                emit(format!("png_fixed={r}"));
            }
            "fxnull" => {
                let bits = u64::from_str_radix(f[1], 16).unwrap();
                let v = f64::from_bits(bits);
                let (p, _i) = new_write(a);
                // png_fixed_error explicitly guards `name == NULL` (pngerror.c:527)
                let r = (a.png_fixed)(p, v, std::ptr::null());
                emit(format!("png_fixed={r}"));
            }
            // fxitu:<f64 bits hex>
            "fxitu" => {
                let bits = u64::from_str_radix(f[1], 16).unwrap();
                let v = f64::from_bits(bits);
                let (p, _i) = new_write(a);
                let r = (a.png_fixed_ITU)(p, v, c"png_set_cLLI(maxCLL)".as_ptr());
                emit(format!("png_fixed_ITU={r}"));
            }

            // ---------------- png_malloc_array / png_realloc_array ----------
            // marr:<nelements>:<element_size>
            "marr" => {
                let n: c_int = f[1].parse().unwrap();
                let esz: usize = f[2].parse().unwrap();
                let (p, _i) = new_write(a);
                let q = (a.png_malloc_array)(p, n, esz);
                emit(format!(
                    "malloc_array={}",
                    if q.is_null() { "NULL" } else { "ptr" }
                ));
                if !q.is_null() {
                    (a.png_free)(p, q);
                }
            }
            // rarr:<old_elements>:<add_elements>:<element_size>:<null old?>
            "rarr" => {
                let old_n: c_int = f[1].parse().unwrap();
                let add: c_int = f[2].parse().unwrap();
                let esz: usize = f[3].parse().unwrap();
                let null_old: bool = f[4] == "1";
                let (p, _i) = new_write(a);
                let mut backing = vec![0u8; 4096];
                let old: *const c_void = if null_old {
                    std::ptr::null()
                } else {
                    backing.as_mut_ptr() as *const c_void
                };
                let q = (a.png_realloc_array)(p, old, old_n, add, esz);
                emit(format!(
                    "realloc_array={}",
                    if q.is_null() { "NULL" } else { "ptr" }
                ));
                if !q.is_null() {
                    (a.png_free)(p, q);
                }
            }

            // ---------------- png_check_IHDR (direct) ----------------
            // cihdr:<w>:<h>:<bd>:<ct>:<il>:<cm>:<fm>:<flags>
            "cihdr" => {
                let w: png_uint_32 = f[1].parse().unwrap();
                let h: png_uint_32 = f[2].parse().unwrap();
                let bd: c_int = f[3].parse().unwrap();
                let ct: c_int = f[4].parse().unwrap();
                let il: c_int = f[5].parse().unwrap();
                let cm: c_int = f[6].parse().unwrap();
                let fm: c_int = f[7].parse().unwrap();
                let flags: u32 = f[8].parse().unwrap();
                let (p, _i) = new_write(a);
                if flags & 4 != 0 {
                    (a.png_set_user_limits)(p, 100, 200);
                }
                if flags & 2 != 0 {
                    (a.png_permit_mng_features)(p, PNG_ALL_MNG_FEATURES as u32);
                }
                if flags & 1 != 0 {
                    (a.png_write_sig)(p);
                }
                (a.png_check_IHDR)(p, w, h, bd, ct, il, cm, fm);
                emit("check_IHDR returned");
            }

            // ---------------- png_zstream_error observable messages ---------
            // zwrite:<strategy>:<level>:<memlevel>:<windowbits>
            "zwrite" => {
                let (p, info) = new_write(a);
                let st: c_int = f[1].parse().unwrap();
                let lv: c_int = f[2].parse().unwrap();
                let ml: c_int = f[3].parse().unwrap();
                let wb: c_int = f[4].parse().unwrap();
                if st != 1000 {
                    (a.png_set_compression_strategy)(p, st);
                }
                if lv != 1000 {
                    (a.png_set_compression_level)(p, lv);
                }
                if ml != 1000 {
                    (a.png_set_compression_mem_level)(p, ml);
                }
                if wb != 1000 {
                    (a.png_set_compression_window_bits)(p, wb);
                }
                (a.png_set_IHDR)(p, info, 4, 2, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit(format!("bytes:{}", out_len()));
            }

            // ---------------- pngrio.c / pngwio.c ----------------
            "read-fn-nulled-then-read" => {
                // png_set_write_fn on a read struct clears read_data_fn
                // (pngrio.c has no way to set it to NULL directly), which is the
                // only route to pngrio.c:39.
                let (p, _i) = new_read(a);
                (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
                emit("set_write_fn on read struct returned");
                let mut buf = [0u8; 8];
                (a.png_read_data)(p, buf.as_mut_ptr(), 8);
                emit("read_data returned");
            }
            "write-fn-nulled-then-write" => {
                // Mirror image: png_set_read_fn on a write struct clears
                // write_data_fn, reaching pngwio.c:40.  Passing NULL to
                // png_set_write_fn does NOT: it installs png_default_write_data.
                let (p, _i) = new_write(a);
                (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
                emit("set_read_fn on write struct returned");
                let data = [0u8; 8];
                (a.png_write_data)(p, data.as_ptr(), 8);
                emit("write_data returned");
            }
            "read-error-eof" => {
                // png_default_read_data -> fread short read -> "Read Error".
                // A genuine FILE* is required: with io_ptr == NULL the C calls
                // fread(NULL) and segfaults, so that input is NOT tested.
                let (p, _i) = new_read(a);
                let fp = c_fopen("/dev/null", "r");
                assert!(!fp.is_null());
                (a.png_init_io)(p, fp);
                let mut buf = [0u8; 8];
                (a.png_read_data)(p, buf.as_mut_ptr(), 8);
                emit("read_data returned");
            }
            "read-error-zero-length" => {
                let (p, _i) = new_read(a);
                let fp = c_fopen("/dev/null", "r");
                assert!(!fp.is_null());
                (a.png_init_io)(p, fp);
                let mut buf = [0u8; 8];
                // length 0: fread returns 0 == length, so this must NOT error
                (a.png_read_data)(p, buf.as_mut_ptr(), 0);
                emit("read_data(0) returned");
            }
            "read-error-default-direct" => {
                let (p, _i) = new_read(a);
                let fp = c_fopen("/dev/null", "r");
                assert!(!fp.is_null());
                (a.png_init_io)(p, fp);
                let mut buf = [0u8; 8];
                (a.png_default_read_data)(p, buf.as_mut_ptr(), 4);
                emit("default_read_data returned");
            }
            "read-default-null-struct" => {
                let mut buf = [0u8; 8];
                (a.png_default_read_data)(std::ptr::null_mut(), buf.as_mut_ptr(), 4);
                emit("default_read_data(NULL) returned");
            }
            "write-error-readonly-file" => {
                // png_default_write_data -> fwrite to a read-only stream fails.
                let (p, _i) = new_write(a);
                let fp = c_fopen("/dev/null", "r");
                assert!(!fp.is_null());
                (a.png_init_io)(p, fp);
                // reinstate the stdio default that new_write replaced
                (a.png_set_write_fn)(p, fp, None, None);
                let data = [0u8; 8];
                (a.png_write_data)(p, data.as_ptr(), 8);
                emit("write_data returned");
            }
            "write-error-default-direct" => {
                let (p, _i) = new_write(a);
                let fp = c_fopen("/dev/null", "r");
                assert!(!fp.is_null());
                (a.png_init_io)(p, fp);
                let mut data = [0u8; 8];
                (a.png_default_write_data)(p, data.as_mut_ptr(), 4);
                emit("default_write_data returned");
            }
            "write-default-null-struct" => {
                let mut data = [0u8; 8];
                (a.png_default_write_data)(std::ptr::null_mut(), data.as_mut_ptr(), 4);
                emit("default_write_data(NULL) returned");
            }
            "write-error-zero-length" => {
                let (p, _i) = new_write(a);
                let fp = c_fopen("/dev/null", "r");
                assert!(!fp.is_null());
                (a.png_set_write_fn)(p, fp, None, None);
                let data = [0u8; 8];
                (a.png_write_data)(p, data.as_ptr(), 0);
                emit("write_data(0) returned");
            }
            "flush-null-io" => {
                // png_default_flush with io_ptr == NULL: fflush(NULL) is defined
                // (flush every stream), so this is a legal, silent call.
                let (p, _i) = new_write(a);
                (a.png_set_write_fn)(p, std::ptr::null_mut(), None, None);
                (a.png_flush)(p);
                (a.png_default_flush)(p);
                (a.png_default_flush)(std::ptr::null_mut());
                emit("flush returned");
            }

            // uvc-create:<version string>
            "uvc-create" => {
                let v = std::ffi::CString::new(f[1]).unwrap();
                let pr = (a.png_create_read_struct)(
                    v.as_ptr(),
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                emit(format!(
                    "read struct={}",
                    if pr.is_null() { "NULL" } else { "ptr" }
                ));
                let pw = (a.png_create_write_struct)(
                    v.as_ptr(),
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                emit(format!(
                    "write struct={}",
                    if pw.is_null() { "NULL" } else { "ptr" }
                ));
            }

            other => {
                emit(format!("UNKNOWN CASE {other}"));
                std::process::exit(3);
            }
        }
    }
    child_finish();
}

/// The sub-process entry point.  Does nothing in the parent.
#[test]
fn harness_child() {
    let Some((case, which)) = child_case() else {
        return;
    };
    set_child_mode(true);
    let b = apis();
    let a = if which == "c" { &b.c } else { &b.rs };
    set_cur_is_c(which == "c");
    reset_all();
    run_case(a, &case);
}

/// `diff_case` for every case, plus a non-vacuity check: a case that produces
/// no transcript at all (for example because the case name failed to parse and
/// the child panicked) would compare equal for the wrong reason, so require a
/// clean `exit(0)` or the `exit(70)` of the error handler AND a non-empty
/// transcript.
#[track_caller]
fn run_all(cases: &[String]) {
    for case in cases {
        let c = run_child(case, "c");
        let r = run_child(case, "rs");
        if c != r {
            panic!(
                "ERROR-PATH MISMATCH for case {case:?}\n  C   : exit={:?} signal={:?} lines={:#?}\n  RUST: exit={:?} signal={:?} lines={:#?}",
                c.exit, c.signal, c.lines, r.exit, r.signal, r.lines
            );
        }
        assert!(
            matches!(c.exit, Some(0) | Some(70)) && !c.lines.is_empty(),
            "case {case:?} produced no usable transcript: {c:?}"
        );
    }
    eprintln!("{} sub-process cases compared", cases.len());
}

// ===========================================================================
// 1. png_ascii_from_fp / png_ascii_from_fixed with a too-small buffer
// ===========================================================================

#[test]
fn ascii_conversion_buffer_too_small() {
    let mut cases = Vec::new();
    // (a) the `size >= precision+5` gate (png.c:2333), swept exhaustively over
    //     the interesting size range for every clamping of `precision`.
    for prec in [0u32, 1, 2, 3, 6, 15, 16, 17, 40] {
        for size in 0..=24usize {
            cases.push(format!("afp:{size}:2:{prec}"));
        }
    }
    // (b) the second site (png.c:2608): an exponent is needed but its digits do
    //     not fit.  Driven by large/small magnitudes at many sizes.
    for vi in 0..fp_values().len() {
        for size in [0usize, 5, 6, 7, 8, 10, 15, 20, 21, 22, 25, 30] {
            for prec in [1u32, 6] {
                cases.push(format!("afp:{size}:{vi}:{prec}"));
            }
        }
    }
    // (c) png_ascii_from_fixed: the `size > 12` gate (png.c:2649) and the
    //     `num <= 0x80000000` gate (png.c:2661).
    for vi in 0..fixed_values().len() {
        for size in 0..=16usize {
            cases.push(format!("afx:{size}:{vi}"));
        }
    }
    run_all(&cases);
}

// ===========================================================================
// 2. png_fixed / png_fixed_ITU out of range
// ===========================================================================

#[test]
fn fixed_point_overflow() {
    let mut cases = Vec::new();
    // png_fixed: floor(100000*fp+.5) must be in [-2147483648, 2147483647]
    let vals: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        21474.0,
        -21474.0,
        21474.83647,
        21474.836475,
        21474.8364749,
        21474.83648,
        21474.83649,
        -21474.83648,
        -21474.836485,
        -21474.83649,
        21475.0,
        -21475.0,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        1e-7,
        -1e-7,
    ];
    for v in &vals {
        for t in 0..3 {
            cases.push(format!("fx:{:016x}:{t}", v.to_bits()));
        }
        cases.push(format!("fxnull:{:016x}", v.to_bits()));
    }
    // png_fixed_ITU: floor(10000*fp+.5) must be in [0, 2147483647]
    let vals2: Vec<f64> = vec![
        0.0,
        -0.0,
        -1e-9,
        -0.00004,
        -0.00005,
        -0.00006,
        -1.0,
        1.0,
        2.0,
        214748.36474,
        214748.364745,
        214748.36475,
        214748.3648,
        214748.4,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
    ];
    for v in &vals2 {
        cases.push(format!("fxitu:{:016x}", v.to_bits()));
    }
    // NOTE: NaN is deliberately NOT tested.  `png_fixed` computes
    // `r = floor(100000*fp+.5)` (NaN), neither range test fires, and the
    // function then evaluates `(png_fixed_point)r` -- a cast of NaN to a signed
    // integer, which is undefined behaviour in C (png.c:2737, and png.c:2756 for
    // png_fixed_ITU).  There is no guard to compare against.
    run_all(&cases);
}

// ===========================================================================
// 3. png_muldiv / png_reciprocal / png_reciprocal2 overflow sentinels
// ===========================================================================

#[test]
fn muldiv_and_reciprocal_sentinels() {
    let b = apis();
    let mut rng = Rng::new(0x2403);
    let specials: [i32; 20] = [
        0,
        1,
        -1,
        2,
        -2,
        4,
        5,
        -5,
        8,
        -8,
        100000,
        -100000,
        110000,
        65535,
        0x10000,
        -65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];

    let mut n = 0usize;
    let do_muldiv = |x: i32, m: i32, d: i32| {
        let mut ra: i32 = 0x5555_5555;
        let mut rb: i32 = 0x5555_5555;
        unsafe {
            let ok_c = (b.c.png_muldiv)(&mut ra, x, m, d);
            let ok_r = (b.rs.png_muldiv)(&mut rb, x, m, d);
            eq_dbg(&format!("png_muldiv({x},{m},{d}).ret"), ok_c, ok_r);
            // The result is only defined when the return is 1; on the failure
            // path the C leaves `*res` untouched (png.c:2881) and so must Rust.
            eq_dbg(&format!("png_muldiv({x},{m},{d}).res"), ra, rb);
        }
    };
    for &x in &specials {
        for &m in &specials {
            for &d in &specials {
                do_muldiv(x, m, d);
                n += 1;
            }
        }
    }
    for _ in 0..30000 {
        do_muldiv(
            rng.next_u32() as i32,
            rng.next_u32() as i32,
            rng.next_u32() as i32,
        );
        n += 1;
    }
    // small divisors are where the overflow sentinel actually fires
    for _ in 0..30000 {
        do_muldiv(
            rng.next_u32() as i32,
            rng.next_u32() as i32,
            (rng.below(21) as i32) - 10,
        );
        n += 1;
    }

    // png_reciprocal returns 0 for a == 0 and for |a| < 5 (overflow)
    let mut recips: Vec<i32> = specials.to_vec();
    for v in -12i32..=12 {
        recips.push(v);
    }
    for _ in 0..20000 {
        recips.push(rng.next_u32() as i32);
    }
    for &x in &recips {
        unsafe {
            eq_dbg(
                &format!("png_reciprocal({x})"),
                (b.c.png_reciprocal)(x),
                (b.rs.png_reciprocal)(x),
            );
        }
        n += 1;
    }
    for i in 0..recips.len() {
        let x = recips[i];
        let y = recips[recips.len() - 1 - i];
        unsafe {
            eq_dbg(
                &format!("png_reciprocal2({x},{y})"),
                (b.c.png_reciprocal2)(x, y),
                (b.rs.png_reciprocal2)(x, y),
            );
        }
        n += 1;
    }
    // and the whole grid of small reciprocal2 arguments, which is where the
    // 1E15/a/b range test bites
    for x in [0i32, 1, -1, 4, 5, -5, 100000, -100000, i32::MAX, i32::MIN] {
        for y in [0i32, 1, -1, 4, 5, -5, 100000, -100000, i32::MAX, i32::MIN] {
            unsafe {
                eq_dbg(
                    &format!("png_reciprocal2({x},{y})"),
                    (b.c.png_reciprocal2)(x, y),
                    (b.rs.png_reciprocal2)(x, y),
                );
            }
            n += 1;
        }
    }
    // png_gamma_significant boundary (95000..105000 -> 0)
    for g in [
        94999i32, 95000, 95001, 99999, 100000, 100001, 104999, 105000, 105001, 0, -1, i32::MAX,
        i32::MIN,
    ] {
        unsafe {
            eq_dbg(
                &format!("png_gamma_significant({g})"),
                (b.c.png_gamma_significant)(g),
                (b.rs.png_gamma_significant)(g),
            );
        }
        n += 1;
    }
    eprintln!("muldiv/reciprocal comparisons: {n}");
    assert!(n > 100000, "expected a large sweep, got {n}");
}

// ===========================================================================
// 4. png_XYZ_from_xy / png_xy_from_XYZ failure branches
// ===========================================================================

#[test]
fn colourspace_conversion_failures() {
    let (cf, rf) = both::<FnXYZfromxy>("png_XYZ_from_xy");
    let (cb, rb) = both::<FnxyfromXYZ>("png_xy_from_XYZ");
    let mut rng = Rng::new(0x2404);

    let srgb = png_xy {
        redx: 64000,
        redy: 33000,
        greenx: 30000,
        greeny: 60000,
        bluex: 15000,
        bluey: 6000,
        whitex: 31270,
        whitey: 32900,
    };

    // Targeted values around every constant in the range checks.
    let edge: [i32; 22] = [
        i32::MIN,
        i32::MIN + 1,
        -110001,
        -110000,
        -1,
        0,
        1,
        4,
        5,
        6,
        7,
        99999,
        100000,
        100001,
        109999,
        110000,
        110001,
        110002,
        200000,
        0x7fff_fffe,
        i32::MAX,
        46000,
    ];

    let mut cases: Vec<png_xy> = vec![srgb, png_xy::default()];
    // one field at a time
    for field in 0..8usize {
        for &e in &edge {
            let mut xy = srgb;
            match field {
                0 => xy.redx = e,
                1 => xy.redy = e,
                2 => xy.greenx = e,
                3 => xy.greeny = e,
                4 => xy.bluex = e,
                5 => xy.bluey = e,
                6 => xy.whitex = e,
                _ => xy.whitey = e,
            }
            cases.push(xy);
        }
    }
    // pairs that make the fpLimit-minus-other-coordinate test fire
    for &x in &edge {
        for &y in &edge {
            cases.push(png_xy {
                redx: x,
                redy: y,
                greenx: 30000,
                greeny: 60000,
                bluex: 15000,
                bluey: 6000,
                whitex: 31270,
                whitey: 32900,
            });
            cases.push(png_xy {
                redx: 64000,
                redy: 33000,
                greenx: 30000,
                greeny: 60000,
                bluex: 15000,
                bluey: 6000,
                whitex: x,
                whitey: y,
            });
        }
    }
    // degenerate primaries: all equal (denominator -> 0), collinear, etc.
    for v in [0i32, 5, 1, 100, 33333, 50000, 110000] {
        cases.push(png_xy {
            redx: v,
            redy: v,
            greenx: v,
            greeny: v,
            bluex: v,
            bluey: v,
            whitex: v,
            whitey: v,
        });
    }
    for w in [5i32, 6, 10, 100, 1000, 32900, 110000] {
        cases.push(png_xy {
            redx: 33333,
            redy: 33333,
            greenx: 33333,
            greeny: 33333,
            bluex: 33333,
            bluey: 33333,
            whitex: 33333,
            whitey: w,
        });
        // red_inverse <= whitey
        cases.push(png_xy {
            redx: 1,
            redy: 1,
            greenx: 2,
            greeny: 2,
            bluex: 3,
            bluey: 3,
            whitex: 4,
            whitey: w,
        });
    }
    // randomised sweeps: in-range, near-range and full i32
    for _ in 0..8000 {
        cases.push(png_xy {
            redx: rng.below(110_002) as i32,
            redy: rng.below(110_002) as i32,
            greenx: rng.below(110_002) as i32,
            greeny: rng.below(110_002) as i32,
            bluex: rng.below(110_002) as i32,
            bluey: rng.below(110_002) as i32,
            whitex: rng.below(110_002) as i32,
            whitey: rng.below(110_002) as i32,
        });
    }
    for _ in 0..8000 {
        cases.push(png_xy {
            redx: rng.below(60_001) as i32,
            redy: rng.below(60_001) as i32,
            greenx: rng.below(60_001) as i32,
            greeny: rng.below(60_001) as i32,
            bluex: rng.below(60_001) as i32,
            bluey: rng.below(60_001) as i32,
            whitex: rng.below(60_001) as i32,
            whitey: rng.below(60_001) as i32,
        });
    }
    for _ in 0..4000 {
        cases.push(png_xy {
            redx: rng.next_u32() as i32,
            redy: rng.next_u32() as i32,
            greenx: rng.next_u32() as i32,
            greeny: rng.next_u32() as i32,
            bluex: rng.next_u32() as i32,
            bluey: rng.next_u32() as i32,
            whitex: rng.next_u32() as i32,
            whitey: rng.next_u32() as i32,
        });
    }

    let mut fails = 0usize;
    let mut oks = 0usize;
    for xy in &cases {
        let mut ca = png_XYZ::default();
        let mut ra = png_XYZ::default();
        unsafe {
            let rc = cf(&mut ca, xy);
            let rr = rf(&mut ra, xy);
            eq_dbg(&format!("png_XYZ_from_xy({xy:?}).ret"), rc, rr);
            if rc == 0 {
                eq_dbg(&format!("png_XYZ_from_xy({xy:?}).out"), ca, ra);
                oks += 1;
            } else {
                fails += 1;
            }
        }
    }
    assert!(fails > 500, "expected many rejections, got {fails}");
    assert!(oks > 100, "expected some successes, got {oks}");

    // ---- png_xy_from_XYZ ----
    let mut xyzs: Vec<png_XYZ> = vec![png_XYZ::default()];
    {
        let mut a = png_XYZ::default();
        unsafe { cf(&mut a, &srgb) };
        xyzs.push(a);
    }
    // one field at a time from a known-good XYZ, using values that make the
    // png_safe_add overflow and the muldiv divide-by-zero fire
    let base = xyzs[1];
    let xedge: [i32; 14] = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        100000,
        1_000_000,
        0x4000_0000,
        0x7fff_fffe,
        i32::MAX,
        -0x4000_0000,
        -100000,
        2,
        0x3fff_ffff,
    ];
    for field in 0..9usize {
        for &e in &xedge {
            let mut z = base;
            match field {
                0 => z.red_X = e,
                1 => z.red_Y = e,
                2 => z.red_Z = e,
                3 => z.green_X = e,
                4 => z.green_Y = e,
                5 => z.green_Z = e,
                6 => z.blue_X = e,
                7 => z.blue_Y = e,
                _ => z.blue_Z = e,
            }
            xyzs.push(z);
        }
    }
    // whole-vector extremes: each component 0 (dred/dgreen/dblue == 0)
    for &e in &xedge {
        xyzs.push(png_XYZ {
            red_X: e,
            red_Y: e,
            red_Z: e,
            green_X: e,
            green_Y: e,
            green_Z: e,
            blue_X: e,
            blue_Y: e,
            blue_Z: e,
        });
    }
    for _ in 0..8000 {
        xyzs.push(png_XYZ {
            red_X: rng.below(200_001) as i32,
            red_Y: rng.below(200_001) as i32,
            red_Z: rng.below(200_001) as i32,
            green_X: rng.below(200_001) as i32,
            green_Y: rng.below(200_001) as i32,
            green_Z: rng.below(200_001) as i32,
            blue_X: rng.below(200_001) as i32,
            blue_Y: rng.below(200_001) as i32,
            blue_Z: rng.below(200_001) as i32,
        });
    }
    for _ in 0..6000 {
        xyzs.push(png_XYZ {
            red_X: rng.next_u32() as i32,
            red_Y: rng.next_u32() as i32,
            red_Z: rng.next_u32() as i32,
            green_X: rng.next_u32() as i32,
            green_Y: rng.next_u32() as i32,
            green_Z: rng.next_u32() as i32,
            blue_X: rng.next_u32() as i32,
            blue_Y: rng.next_u32() as i32,
            blue_Z: rng.next_u32() as i32,
        });
    }
    let mut bfails = 0usize;
    for xyz in &xyzs {
        let mut ca = png_xy::default();
        let mut ra = png_xy::default();
        unsafe {
            let rc = cb(&mut ca, xyz);
            let rr = rb(&mut ra, xyz);
            eq_dbg(&format!("png_xy_from_XYZ({xyz:?}).ret"), rc, rr);
            if rc == 0 {
                eq_dbg(&format!("png_xy_from_XYZ({xyz:?}).out"), ca, ra);
            } else {
                bfails += 1;
            }
        }
    }
    eprintln!(
        "colourspace comparisons: {} xy cases + {} XYZ cases",
        cases.len(),
        xyzs.len()
    );
    assert!(bfails > 100, "expected many rejections, got {bfails}");
}

// ===========================================================================
// 5. png_zstream_error
// ===========================================================================

/// Every zlib return code, plus out-of-range codes, passed straight to
/// `png_zstream_error`.  All the function can do is store a string into
/// `png_ptr->zstream.msg`, which is not visible from here, so what is compared
/// is that the call is silent in both libraries and leaves the struct usable.
#[test]
fn zstream_error_all_codes_are_silent() {
    let b = apis();
    let codes: [c_int; 15] = [
        0, 1, 2, -1, -2, -3, -4, -5, -6, -7, -100, 3, 100, i32::MIN, i32::MAX,
    ];
    for &code in &codes {
        for read in [false, true] {
            let mut got = Vec::new();
            for (label, a) in [("C", &b.c), ("RUST", &b.rs)] {
                log_reset();
                unsafe {
                    let (p, _i) = if read { new_read(a) } else { new_write(a) };
                    (a.png_zstream_error)(p, code);
                    let r1 = if read { (a.png_reset_zstream)(p) } else { -999 };
                    // a second call must not change anything either
                    (a.png_zstream_error)(p, code);
                    let r2 = if read { (a.png_reset_zstream)(p) } else { -999 };
                    got.push((label, log_take(), r1, r2));
                }
            }
            eq_dbg(
                &format!("png_zstream_error({code}, read={read}) transcript"),
                got[0].1.clone(),
                got[1].1.clone(),
            );
            eq_dbg(
                &format!("png_zstream_error({code}, read={read}) reset1"),
                got[0].2,
                got[1].2,
            );
            eq_dbg(
                &format!("png_zstream_error({code}, read={read}) reset2"),
                got[0].3,
                got[1].3,
            );
            assert!(
                got[0].1.is_empty(),
                "png_zstream_error must be silent, got {:?}",
                got[0].1
            );
        }
    }
}

/// The messages that ARE observable: corrupt zlib streams inside a `zTXt`
/// chunk / the IDAT stream make libpng report `png_ptr->zstream.msg` through
/// `png_chunk_benign_error` (a warning on a read struct).
#[test]
fn zstream_error_reachable_messages() {
    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();

    // valid zlib stream (control)
    streams.push(("valid".into(), pb::zlib_store(b"hello world")));
    // Z_DATA_ERROR: bad zlib header check bits
    streams.push(("bad-header-check".into(), vec![0x78, 0x02, 0x03, 0x00]));
    // "invalid window size (libpng)": CINFO > 7
    streams.push(("cinfo-8".into(), vec![0x88, 0x1d, 0x03, 0x00]));
    streams.push(("cinfo-15".into(), vec![0xf8, 0x1d, 0x03, 0x00]));
    // Z_NEED_DICT: FDICT set in FLG
    streams.push(("fdict".into(), vec![0x78, 0xbb, 0, 0, 0, 1, 0x03, 0x00]));
    // Z_DATA_ERROR in the deflate body
    {
        let mut s = pb::zlib_store(b"hello world");
        s[2] = 0x07; // invalid block type
        streams.push(("bad-block-type".into(), s));
    }
    // truncated stream (Z_BUF_ERROR -> "truncated", or Z_DATA_ERROR)
    {
        let long = b"the quick brown fox jumps over the lazy dog, repeatedly and at length";
        let s = pb::zlib_store(long);
        for keep in [
            0usize, 1, 2, 3, 5, 6, 7, 8, 10, 12, 20, 40, 60, 70, 71, 72, 73,
        ] {
            if keep > s.len() {
                continue;
            }
            streams.push((format!("truncated-{keep}"), s[..keep].to_vec()));
        }
        // a complete stream missing only its adler32 trailer
        streams.push(("no-adler".into(), s[..s.len() - 4].to_vec()));
        streams.push(("short-adler".into(), s[..s.len() - 1].to_vec()));
    }
    // corrupted adler32 trailer
    {
        let mut s = pb::zlib_store(b"hello world");
        let n = s.len();
        s[n - 1] ^= 0xff;
        streams.push(("bad-adler".into(), s));
    }
    // extra data after the end of the stream
    {
        let mut s = pb::zlib_store(b"hello");
        s.extend_from_slice(b"XXXXXXXX");
        streams.push(("extra-data".into(), s));
    }
    // empty
    streams.push(("empty".into(), Vec::new()));

    let b = apis();
    let mut seen: Vec<String> = Vec::new();
    for (label, s) in &streams {
        for (what, png) in [
            (format!("zTXt/{label}"), ztxt_png(s)),
            (format!("IDAT/{label}"), idat_png(s)),
        ] {
            let mut logs = Vec::new();
            for a in [&b.c, &b.rs] {
                logs.push(read_transcript(a, &png));
            }
            eq_dbg(&format!("zlib failure {what}"), logs[0].clone(), logs[1].clone());
            seen.extend(logs[0].iter().cloned());
        }
    }
    // the write-side deflateInit2 failure, which yields "bad parameters to zlib"
    let mut cases = Vec::new();
    for st in [-1i32, 0, 1, 2, 3, 4, 5, 100, i32::MIN, i32::MAX] {
        cases.push(format!("zwrite:{st}:1000:1000:1000"));
    }
    for lv in [-2i32, -1, 0, 9, 10, 100] {
        cases.push(format!("zwrite:1000:{lv}:1000:1000"));
    }
    for ml in [0i32, 1, 9, 10] {
        cases.push(format!("zwrite:1000:1000:{ml}:1000"));
    }
    for wb in [7i32, 8, 15, 16] {
        cases.push(format!("zwrite:1000:1000:1000:{wb}"));
    }
    run_all(&cases);

    // Only three of the ten `png_zstream_error` strings are reachable at all
    // (see the file comment); assert that those three really did appear so this
    // test cannot pass vacuously.  The remaining diagnostics come from zlib's
    // own `strm->msg`, which `png_zstream_error` deliberately does not overwrite.
    for want in ["missing LZ dictionary", "truncated"] {
        assert!(
            seen.iter().any(|l| l.contains(want)),
            "expected the zstream message {want:?} in {seen:#?}"
        );
    }
    let t = run_child("zwrite:100:1000:1000:1000", "c");
    assert!(
        t.lines.iter().any(|l| l == "ERROR:bad parameters to zlib"),
        "expected the Z_STREAM_ERROR message from the write side, got {:?}",
        t.lines
    );

    eprintln!("observable zstream messages: {seen:#?}");
}

/// Read `png` with both a sequential reader, returning the ordered transcript of
/// warnings plus the outcome.  Any `png_error` would abort the process, so the
/// simplified API (which converts `png_error` into a return code) is used.
fn read_transcript(a: &Api, png: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    log_reset();
    unsafe {
        let mut img = png_image::default();
        let begin = (a.png_image_begin_read_from_memory)(
            &mut img,
            png.as_ptr() as *const c_void,
            png.len(),
        );
        out.push(format!("begin={begin} msg={:?}", img.msg()));
        if begin != 0 {
            img.format = PNG_FORMAT_RGBA;
            let total = (img.width as u64).saturating_mul(img.height as u64) * 8 + 8192;
            if total < 8 * 1024 * 1024 {
                let mut buf = vec![0u8; total as usize];
                let finish = (a.png_image_finish_read)(
                    &mut img,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    0,
                    std::ptr::null_mut(),
                );
                out.push(format!("finish={finish} msg={:?}", img.msg()));
            }
        }
        (a.png_image_free)(&mut img);
    }
    for l in log_take() {
        out.push(l);
    }
    out
}

// ===========================================================================
// 6. png_icc_check_header / _length / _tag_table
// ===========================================================================

#[test]
fn icc_profile_rejections_direct() {
    let (cl, rl) = both::<FnIccLength>("png_icc_check_length");
    let (ch, rh) = both::<FnIccHeader>("png_icc_check_header");
    let (ct, rt) = both::<FnIccTags>("png_icc_check_tag_table");
    let b = apis();

    let name = std::ffi::CString::new("my profile").unwrap();
    let long_name = std::ffi::CString::new("N".repeat(200)).unwrap();

    let cases = icc_cases();
    assert!(cases.len() > 100, "expected a large ICC case table");
    let mut warn_count = 0usize;
    let mut reject_count = 0usize;

    for case in &cases {
        for nm in [&name, &long_name] {
            let mut res = Vec::new();
            for (a, fl, fh, ft) in [(&b.c, cl, ch, ct), (&b.rs, rl, rh, rt)] {
                log_reset();
                unsafe {
                    let (p, _i) = new_read(a);
                    let r1 = fl(p, nm.as_ptr(), case.length);
                    let r2 = fh(
                        p,
                        nm.as_ptr(),
                        case.length,
                        case.profile.as_ptr(),
                        case.color_type,
                    );
                    let r3 = if case.tags_ok {
                        ft(p, nm.as_ptr(), case.length, case.profile.as_ptr())
                    } else {
                        -1
                    };
                    res.push((r1, r2, r3, log_take()));
                }
            }
            let what = format!("icc {} name-len={}", case.label, nm.as_bytes().len());
            eq_dbg(&format!("{what} check_length"), res[0].0, res[1].0);
            eq_dbg(&format!("{what} check_header"), res[0].1, res[1].1);
            eq_dbg(&format!("{what} check_tag_table"), res[0].2, res[1].2);
            eq_dbg(&format!("{what} warnings"), res[0].3.clone(), res[1].3.clone());
            warn_count += res[0].3.len();
            if res[0].1 == 0 || res[0].0 == 0 || res[0].2 == 0 {
                reject_count += 1;
            }
        }
    }
    eprintln!(
        "ICC direct comparisons: {} (2 names x {} profiles) + 36 length-limit cases",
        cases.len() * 2,
        cases.len()
    );
    assert!(warn_count > 200, "expected many ICC warnings, got {warn_count}");
    assert!(
        reject_count > 50,
        "expected many ICC rejections, got {reject_count}"
    );

    // ---- png_icc_check_length "profile too long" (png.c:1606) ----
    // png_chunk_max(png_ptr) == png_ptr->user_chunk_malloc_max
    for limit in [0usize, 1, 132, 200, 8_000_000, usize::MAX] {
        for len in [132u32, 133, 200, 8_000_000, 8_000_001, 0xffff_ffff] {
            let mut res = Vec::new();
            for (a, fl) in [(&b.c, cl), (&b.rs, rl)] {
                log_reset();
                unsafe {
                    let (p, _i) = new_read(a);
                    (a.png_set_chunk_malloc_max)(p, limit);
                    let r = fl(p, name.as_ptr(), len);
                    res.push((r, log_take()));
                }
            }
            let what = format!("icc_check_length limit={limit} len={len}");
            eq_dbg(&format!("{what} ret"), res[0].0, res[1].0);
            eq_dbg(&format!("{what} warnings"), res[0].1.clone(), res[1].1.clone());
        }
    }
}

#[test]
fn icc_profile_rejections_through_iccp_chunk() {
    let b = apis();
    let cases = icc_cases();
    // A representative slice through the crafted-chunk path (the direct test
    // above covers every branch; this proves the same decisions are taken when
    // the profile arrives inside a real iCCP chunk).
    let mut n = 0usize;
    for case in cases.iter() {
        if case.profile.len() > 4096 {
            continue;
        }
        for ct in [0u8, 2, 3] {
            let png = iccp_png(&case.profile, ct);
            let mut logs = Vec::new();
            for a in [&b.c, &b.rs] {
                logs.push(read_transcript(a, &png));
            }
            eq_dbg(
                &format!("iCCP chunk {} ct={ct}", case.label),
                logs[0].clone(),
                logs[1].clone(),
            );
            n += 1;
        }
    }
    eprintln!("iCCP chunk-read comparisons: {n}");
    assert!(n > 100, "expected many iCCP chunk cases, got {n}");
}

// ===========================================================================
// 7. png_convert_to_rfc1123_buffer field checks
// ===========================================================================

#[test]
fn rfc1123_buffer_field_checks() {
    let b = apis();
    let mut rng = Rng::new(0x2407);

    let mut times: Vec<png_time> = Vec::new();
    let good = png_time {
        year: 2024,
        month: 6,
        day: 15,
        hour: 12,
        minute: 30,
        second: 45,
    };
    times.push(good);
    // exhaustive boundaries, one field at a time
    for y in [0u16, 1, 9998, 9999, 10000, 10001, 65535] {
        let mut t = good;
        t.year = y;
        times.push(t);
    }
    for m in 0u8..=13 {
        let mut t = good;
        t.month = m;
        times.push(t);
    }
    for m in [14u8, 100, 254, 255] {
        let mut t = good;
        t.month = m;
        times.push(t);
    }
    for d in [0u8, 1, 30, 31, 32, 33, 100, 255] {
        let mut t = good;
        t.day = d;
        times.push(t);
    }
    for h in [0u8, 1, 22, 23, 24, 25, 100, 255] {
        let mut t = good;
        t.hour = h;
        times.push(t);
    }
    for mi in [0u8, 1, 58, 59, 60, 61, 100, 255] {
        let mut t = good;
        t.minute = mi;
        times.push(t);
    }
    for s in [0u8, 1, 59, 60, 61, 62, 100, 255] {
        let mut t = good;
        t.second = s;
        times.push(t);
    }
    // every combination of "one field invalid" x "another field invalid"
    for m in [0u8, 13] {
        for d in [0u8, 32] {
            for h in [24u8, 0] {
                let mut t = good;
                t.month = m;
                t.day = d;
                t.hour = h;
                times.push(t);
            }
        }
    }
    // full sweep over the widest legal formatting (four-digit year, 1-digit day)
    for y in [0u16, 9, 99, 999, 1000, 9999] {
        for d in [1u8, 9, 10, 31] {
            let mut t = good;
            t.year = y;
            t.day = d;
            times.push(t);
        }
    }
    for _ in 0..20000 {
        times.push(png_time {
            year: if rng.bool() {
                rng.range(9990, 10010) as u16
            } else {
                rng.next_u16()
            },
            month: rng.next_u8(),
            day: rng.next_u8(),
            hour: rng.next_u8(),
            minute: rng.next_u8(),
            second: rng.next_u8(),
        });
    }

    let mut zeros = 0usize;
    let mut ones = 0usize;
    for t in &times {
        let mut ba = [0x41u8 as c_char; 40];
        let mut bb = [0x41u8 as c_char; 40];
        unsafe {
            let rc = (b.c.png_convert_to_rfc1123_buffer)(ba.as_mut_ptr(), t);
            let rr = (b.rs.png_convert_to_rfc1123_buffer)(bb.as_mut_ptr(), t);
            eq_dbg(&format!("rfc1123({t:?}).ret"), rc, rr);
            if rc == 0 {
                zeros += 1;
            } else {
                ones += 1;
            }
        }
        let av: Vec<u8> = ba.iter().map(|x| *x as u8).collect();
        let bv: Vec<u8> = bb.iter().map(|x| *x as u8).collect();
        eq_bytes(&format!("rfc1123({t:?}).buf"), &av, &bv);
    }
    eprintln!("rfc1123 comparisons: {}", times.len());
    assert!(zeros > 100 && ones > 50, "zeros={zeros} ones={ones}");

    // out == NULL is an explicit guard (png.c:748).  `ptime == NULL` is NOT
    // guarded -- the C dereferences ptime->year at png.c:751 -- so it is UB and
    // is not tested.
    unsafe {
        eq_dbg(
            "rfc1123(NULL, &good)",
            (b.c.png_convert_to_rfc1123_buffer)(std::ptr::null_mut(), &good),
            (b.rs.png_convert_to_rfc1123_buffer)(std::ptr::null_mut(), &good),
        );
        for t in [good, png_time::default()] {
            eq_dbg(
                &format!("rfc1123(NULL,{t:?})"),
                (b.c.png_convert_to_rfc1123_buffer)(std::ptr::null_mut(), &t),
                (b.rs.png_convert_to_rfc1123_buffer)(std::ptr::null_mut(), &t),
            );
        }
    }

    // png_convert_to_rfc1123 relays the failure as a warning
    for t in [
        good,
        png_time::default(),
        png_time {
            year: 10000,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        },
    ] {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            log_reset();
            unsafe {
                let (p, _i) = new_read(a);
                let s = (a.png_convert_to_rfc1123)(p, &t);
                res.push((cstr_to_string(s), s.is_null(), log_take()));
            }
        }
        eq_dbg(&format!("rfc1123_str({t:?})"), res[0].clone(), res[1].clone());
    }
}

// ===========================================================================
// 8. png_user_version_check
// ===========================================================================

#[test]
fn user_version_check_mismatches() {
    let b = apis();
    let versions: Vec<Option<String>> = vec![
        None,
        Some("".into()),
        Some("1".into()),
        Some("1.".into()),
        Some("1.6".into()),
        Some("1.6.".into()),
        Some("1.6.5".into()),
        Some("1.6.59".into()),
        Some("1.6.59.git".into()),
        Some("1.6.59.gitx".into()),
        Some("1.6.590".into()),
        Some("1.6.0".into()),
        Some("1.6.99".into()),
        Some("1.5.59".into()),
        Some("1.7.0".into()),
        Some("2.6.59".into()),
        Some("0.6.59".into()),
        Some("11.6.59".into()),
        Some("1.16.59".into()),
        Some("x".into()),
        Some("garbage".into()),
        Some("1.6.59.git\u{1}".into()),
        Some(".".into()),
        Some("..".into()),
        Some("...".into()),
        Some("1..".into()),
        Some("1.6..".into()),
        Some("V".repeat(200)),
        Some(format!("1.6.{}", "9".repeat(150))),
        Some("\u{7f}".into()),
    ];
    for v in &versions {
        let cs = v.as_ref().map(|s| std::ffi::CString::new(s.as_str()).unwrap());
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            log_reset();
            unsafe {
                // A FRESH struct per call: PNG_FLAG_LIBRARY_MISMATCH is sticky.
                let (p, _i) = new_read(a);
                let ptr = cs.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
                let r = (a.png_user_version_check)(p, ptr);
                res.push((r, log_take()));
            }
        }
        let what = format!("png_user_version_check({v:?})");
        eq_dbg(&format!("{what} ret"), res[0].0, res[1].0);
        eq_dbg(&format!("{what} warnings"), res[0].1.clone(), res[1].1.clone());
    }
    eprintln!("png_user_version_check comparisons: {}", versions.len());

    // ... and through the public creators, which emit the same warning and then
    // return NULL (t23_err_write.rs covers five version strings this way; the
    // set below is wider).  Sub-process, because the caller of a NULL struct
    // cannot continue.
    let mut cases = Vec::new();
    for v in [
        "1.6.59.git", "1.6.59", "1.6.5", "1.6.", "1.6", "1.", "1", "1.5.59",
        "1.7.59", "2.6.59", "0.0.0", "garbage", "", ".", "..", "1.6.590",
    ] {
        cases.push(format!("uvc-create:{v}"));
    }
    run_all(&cases);
}

// ===========================================================================
// 9. png_malloc_array / png_realloc_array
// ===========================================================================

#[test]
fn malloc_array_null_sentinels() {
    let b = apis();
    // Only combinations whose C path returns NULL without erroring, so they can
    // run in-process.  `nelements <= 0` and `element_size == 0` are png_errors
    // and live in `malloc_array_internal_errors` below.
    //
    // NOTE: `png_malloc_base`'s own `size > PNG_SIZE_MAX` guard (pngmem.c:88) is
    // DEAD CODE in this configuration: `png_alloc_size_t` and `size_t` are both
    // 64 bits, so `PNG_SIZE_MAX == SIZE_MAX` and the comparison can never be
    // true.  The reachable "too large" rejection is
    // `png_malloc_array_checked`'s `req <= PNG_SIZE_MAX/element_size`
    // (pngmem.c:113), which the cases below straddle, plus the plain
    // malloc-failure return (pngmem.c:98).
    let cases: [(c_int, usize); 12] = [
        (1, 1),
        (1, 8),
        (4, 8),
        (1000, 16),
        (1, usize::MAX),
        (2, usize::MAX / 2),
        (2, usize::MAX / 2 + 1),
        (i32::MAX, usize::MAX),
        (i32::MAX, 1 << 40),
        (i32::MAX, 4),
        (65536, 1 << 48),
        (3, usize::MAX / 2),
    ];
    let mut nulls = 0usize;
    for &(n, esz) in &cases {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            log_reset();
            unsafe {
                let (p, _i) = new_write(a);
                let q = (a.png_malloc_array)(p, n, esz);
                let isnull = q.is_null();
                if !q.is_null() {
                    (a.png_free)(p, q);
                }
                res.push((isnull, log_take()));
            }
        }
        let what = format!("png_malloc_array({n},{esz})");
        eq_dbg(&format!("{what} null"), res[0].0, res[1].0);
        eq_dbg(&format!("{what} warnings"), res[0].1.clone(), res[1].1.clone());
        if res[0].0 {
            nulls += 1;
        }
    }
    assert!(nulls >= 6, "expected NULL sentinels, got {nulls}");

    // png_realloc_array: the INT_MAX - old_elements guard and the size overflow.
    // `old_elements` must be 0 whenever the allocation can succeed, otherwise
    // the C memcpy's `old_elements*element_size` bytes out of `old_array`.
    let rcases: [(c_int, c_int, usize, bool); 14] = [
        // (old_elements, add_elements, element_size, expect_no_alloc)
        (0, 1, 8, false),
        (0, 4, 8, false),
        (0, 1000, 16, false),
        (i32::MAX, 1, 8, true),
        (i32::MAX, i32::MAX, 8, true),
        (i32::MAX - 1, 2, 8, true),
        (i32::MAX - 1, 1, usize::MAX, true),
        (1, i32::MAX, 8, true),
        (1000, 1000, usize::MAX, true),
        (1000, 1000, usize::MAX / 4, true),
        (0, i32::MAX, usize::MAX, true),
        (0, 2, usize::MAX / 2 + 1, true),
        (0, 1, usize::MAX, true),
        (0, 65536, 1 << 48, true),
    ];
    let mut rnulls = 0usize;
    for &(old_n, add, esz, _) in &rcases {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            log_reset();
            unsafe {
                let (p, _i) = new_write(a);
                let mut backing = vec![0u8; 65536];
                let q = (a.png_realloc_array)(
                    p,
                    backing.as_mut_ptr() as *const c_void,
                    old_n,
                    add,
                    esz,
                );
                let isnull = q.is_null();
                if !q.is_null() {
                    (a.png_free)(p, q);
                }
                res.push((isnull, log_take()));
            }
        }
        let what = format!("png_realloc_array(old={old_n},add={add},esz={esz})");
        eq_dbg(&format!("{what} null"), res[0].0, res[1].0);
        eq_dbg(&format!("{what} warnings"), res[0].1.clone(), res[1].1.clone());
        if res[0].0 {
            rnulls += 1;
        }
    }
    assert!(rnulls >= 8, "expected NULL sentinels, got {rnulls}");

    // a real grow, with the copied prefix checked
    for (old_n, add, esz) in [(1i32, 1i32, 8usize), (4, 4, 8), (3, 5, 1), (10, 1, 16)] {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            unsafe {
                let (p, _i) = new_write(a);
                let old = (a.png_malloc_array)(p, old_n, esz);
                assert!(!old.is_null());
                let bytes = old as *mut u8;
                for i in 0..(old_n as usize * esz) {
                    *bytes.add(i) = (i as u8) ^ 0x5a;
                }
                let q = (a.png_realloc_array)(p, old as *const c_void, old_n, add, esz);
                assert!(!q.is_null());
                let total = (old_n as usize + add as usize) * esz;
                let v = std::slice::from_raw_parts(q as *const u8, total).to_vec();
                (a.png_free)(p, q);
                (a.png_free)(p, old);
                res.push(v);
            }
        }
        eq_bytes(
            &format!("png_realloc_array grow({old_n},{add},{esz})"),
            &res[0],
            &res[1],
        );
    }
}

#[test]
fn malloc_array_internal_errors() {
    let mut cases = Vec::new();
    // "internal error: array alloc" (pngmem.c:126)
    for n in [-1i32, 0, i32::MIN, -1000] {
        for esz in [0usize, 1, 8] {
            cases.push(format!("marr:{n}:{esz}"));
        }
    }
    for n in [1i32, 2, 100, i32::MAX] {
        cases.push(format!("marr:{n}:0"));
    }
    // successful / NULL-returning calls too, so the transcript is not vacuous
    for n in [1i32, 8] {
        for esz in [1usize, 8, usize::MAX] {
            cases.push(format!("marr:{n}:{esz}"));
        }
    }
    // "internal error: array realloc" (pngmem.c:139)
    for add in [-1i32, 0, i32::MIN] {
        cases.push(format!("rarr:0:{add}:8:0"));
    }
    cases.push("rarr:0:4:0:0".into());
    cases.push("rarr:1:4:0:0".into());
    for old in [-1i32, i32::MIN, -100] {
        cases.push(format!("rarr:{old}:4:8:0"));
    }
    // old_array == NULL && old_elements > 0
    for old in [1i32, 2, 100, i32::MAX] {
        cases.push(format!("rarr:{old}:4:8:1"));
    }
    // legal: NULL old array with zero old elements
    cases.push("rarr:0:4:8:1".into());
    // the INT_MAX - old_elements guard (returns NULL, no error)
    cases.push(format!("rarr:{}:1:8:0", i32::MAX));
    cases.push(format!("rarr:{}:2:8:0", i32::MAX - 1));
    run_all(&cases);
}

// ===========================================================================
// 10. png_zalloc / png_zfree
// ===========================================================================

#[test]
fn zalloc_and_zfree() {
    let b = apis();
    let sizes: [c_uint; 12] = [0, 1, 2, 8, 4096, 65536, 0x1000_0000, 0x7fff_ffff, 0x8000_0000, 0xffff_fffe, 0xffff_ffff, 3];
    let items: [c_uint; 12] = [0, 1, 2, 8, 256, 65536, 0x1000_0000, 0x7fff_ffff, 0x8000_0000, 0xffff_fffe, 0xffff_ffff, 7];
    for &it in &items {
        for &sz in &sizes {
            let mut res = Vec::new();
            for a in [&b.c, &b.rs] {
                log_reset();
                unsafe {
                    let (p, _i) = new_write(a);
                    let q = (a.png_zalloc)(p, it, sz);
                    let isnull = q.is_null();
                    // png_zfree(p, NULL) and png_zfree(p, ptr) are both guarded
                    (a.png_zfree)(p, std::ptr::null_mut());
                    (a.png_zfree)(p, q);
                    res.push((isnull, log_take()));
                }
            }
            let what = format!("png_zalloc(items={it},size={sz})");
            eq_dbg(&format!("{what} null"), res[0].0, res[1].0);
            eq_dbg(&format!("{what} warnings"), res[0].1.clone(), res[1].1.clone());
        }
    }
    // png_ptr == NULL is guarded (png.c:109); png_zfree(NULL, x) reaches
    // png_free(NULL, x) which is also guarded (pngmem.c:236) and leaks, so only
    // NULL/NULL is exercised.
    for &it in &[0u32, 1, 0xffff_ffff] {
        for &sz in &[0u32, 1, 0xffff_ffff] {
            unsafe {
                let qc = (b.c.png_zalloc)(std::ptr::null_mut(), it, sz);
                let qr = (b.rs.png_zalloc)(std::ptr::null_mut(), it, sz);
                eq_dbg(
                    &format!("png_zalloc(NULL,{it},{sz})"),
                    qc.is_null(),
                    qr.is_null(),
                );
            }
        }
    }
    unsafe {
        (b.c.png_zfree)(std::ptr::null_mut(), std::ptr::null_mut());
        (b.rs.png_zfree)(std::ptr::null_mut(), std::ptr::null_mut());
    }
    // NOTE: the "Potential overflow in png_zalloc()" warning (png.c:118) is
    // UNREACHABLE in this build.  The test is
    // `items >= (~(png_alloc_size_t)0)/size` with `items`/`size` both `uInt`
    // (32 bits) and `png_alloc_size_t` 64 bits: the smallest right-hand side is
    // (2^64-1)/(2^32-1) == 2^32+1, which is larger than any 32-bit `items`.
}

// ===========================================================================
// 11. png_safecat / png_format_number / png_check_fp_* extra cases
// ===========================================================================

#[test]
fn safecat_format_number_and_fp_extras() {
    let b = apis();
    let mut n = 0usize;

    // ---- png_safecat: the two guarded NULL rows (pngerror.c:76, :78) ----
    for bufsize in [0usize, 1, 4, 16] {
        for pos in [0usize, 1, 3, 15, 16, 17, 100] {
            unsafe {
                eq_dbg(
                    &format!("png_safecat(NULL,{bufsize},{pos},\"abc\")"),
                    (b.c.png_safecat)(std::ptr::null_mut(), bufsize, pos, c"abc".as_ptr()),
                    (b.rs.png_safecat)(std::ptr::null_mut(), bufsize, pos, c"abc".as_ptr()),
                );
                let mut ba = [0x41u8 as c_char; 64];
                let mut bb = [0x41u8 as c_char; 64];
                let rc = (b.c.png_safecat)(ba.as_mut_ptr(), bufsize, pos, std::ptr::null());
                let rr = (b.rs.png_safecat)(bb.as_mut_ptr(), bufsize, pos, std::ptr::null());
                eq_dbg(
                    &format!("png_safecat(buf,{bufsize},{pos},NULL).ret"),
                    rc,
                    rr,
                );
                let av: Vec<u8> = ba.iter().map(|x| *x as u8).collect();
                let bv: Vec<u8> = bb.iter().map(|x| *x as u8).collect();
                eq_bytes(
                    &format!("png_safecat(buf,{bufsize},{pos},NULL).buf"),
                    &av,
                    &bv,
                );
            }
            unsafe {
                eq_dbg(
                    &format!("png_safecat(NULL,{bufsize},{pos},NULL)"),
                    (b.c.png_safecat)(std::ptr::null_mut(), bufsize, pos, std::ptr::null()),
                    (b.rs.png_safecat)(std::ptr::null_mut(), bufsize, pos, std::ptr::null()),
                );
            }
        }
    }
    n += 4 * 7 * 3;
    // huge `pos` values (pos >= bufsize is the guard)
    for pos in [usize::MAX, usize::MAX - 1, usize::MAX / 2, 1 << 40] {
        unsafe {
            let mut ba = [0x41u8 as c_char; 64];
            let mut bb = [0x41u8 as c_char; 64];
            eq_dbg(
                &format!("png_safecat(buf,32,{pos},\"x\")"),
                (b.c.png_safecat)(ba.as_mut_ptr(), 32, pos, c"x".as_ptr()),
                (b.rs.png_safecat)(bb.as_mut_ptr(), 32, pos, c"x".as_ptr()),
            );
            let av: Vec<u8> = ba.iter().map(|x| *x as u8).collect();
            let bv: Vec<u8> = bb.iter().map(|x| *x as u8).collect();
            eq_bytes(&format!("png_safecat(buf,32,{pos}).buf"), &av, &bv);
        }
    }

    n += 4;
    // ---- png_format_number: out-of-range `format` (pngerror.c:144) and the
    //      zero-window case (pngerror.c:106) ----
    let formats: [c_int; 16] = [
        1, 2, 3, 4, 5, 0, -1, -2, 6, 7, 8, 100, 255, 0x1_0000, i32::MIN, i32::MAX,
    ];
    let nums: [usize; 12] = [
        0,
        1,
        9,
        10,
        99999,
        100000,
        100001,
        u32::MAX as usize,
        usize::MAX,
        usize::MAX / 2,
        1 << 63,
        123456789,
    ];
    for &fmt in &formats {
        for &num in &nums {
            // window sizes including 0 (end == start on entry): the buffer is
            // offset by 8 so the unconditional `*--end` stays inside it.
            for win in [0usize, 1, 2, 3, 6, 8, 16, 24] {
                let mut ba = vec![0x41u8 as c_char; 64];
                let mut bb = vec![0x41u8 as c_char; 64];
                unsafe {
                    let sa = ba.as_ptr().add(8);
                    let sb = bb.as_ptr().add(8);
                    let pa = (b.c.png_format_number)(sa, ba.as_mut_ptr().add(8 + win), fmt, num);
                    let pb2 = (b.rs.png_format_number)(sb, bb.as_mut_ptr().add(8 + win), fmt, num);
                    let oa = pa as usize - ba.as_ptr() as usize;
                    let ob = pb2 as usize - bb.as_ptr() as usize;
                    eq_dbg(
                        &format!("png_format_number(fmt={fmt},num={num},win={win}).off"),
                        oa,
                        ob,
                    );
                }
                let av: Vec<u8> = ba.iter().map(|x| *x as u8).collect();
                let bv: Vec<u8> = bb.iter().map(|x| *x as u8).collect();
                eq_bytes(
                    &format!("png_format_number(fmt={fmt},num={num},win={win}).buf"),
                    &av,
                    &bv,
                );
            }
        }
    }

    n += formats.len() * nums.len() * 8;
    // ---- png_check_fp_number: out-of-range initial `state` and `whereami` ----
    let strings: [&[u8]; 18] = [
        b"1", b"0", b"-1", b"+1", b"1.5", b"-1.5e10", b"1E-10", b".5", b"5.", b"",
        b".", b"e", b"1e", b"1e+", b"1.2.3", b"1e1e1", b"1E5+", b"1E5.2",
    ];
    let states: [c_int; 18] = [
        0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 0x7f, 0xff, -1, i32::MIN, i32::MAX,
    ];
    for s in strings {
        for &size in &[0usize, 1, s.len()] {
            if size > s.len() {
                continue;
            }
            for &st0 in &states {
                for &w0 in &[0usize, 1, size] {
                    if w0 > size {
                        continue;
                    }
                    let mut sa = st0;
                    let mut sb = st0;
                    let mut wa = w0;
                    let mut wb = w0;
                    unsafe {
                        let rc = (b.c.png_check_fp_number)(
                            s.as_ptr() as *const c_char,
                            size,
                            &mut sa,
                            &mut wa,
                        );
                        let rr = (b.rs.png_check_fp_number)(
                            s.as_ptr() as *const c_char,
                            size,
                            &mut sb,
                            &mut wb,
                        );
                        let d = format!(
                            "png_check_fp_number({:?},{size},state={st0},w={w0})",
                            String::from_utf8_lossy(s)
                        );
                        eq_dbg(&format!("{d}.ret"), rc, rr);
                        eq_dbg(&format!("{d}.state"), sa, sb);
                        eq_dbg(&format!("{d}.whereami"), wa, wb);
                    }
                    n += 1;
                }
            }
        }
        // png_check_fp_string with sizes past / at the terminator
        for size in 0..=s.len() {
            unsafe {
                eq_dbg(
                    &format!(
                        "png_check_fp_string({:?},{size})",
                        String::from_utf8_lossy(s)
                    ),
                    (b.c.png_check_fp_string)(s.as_ptr() as *const c_char, size),
                    (b.rs.png_check_fp_string)(s.as_ptr() as *const c_char, size),
                );
            }
            n += 1;
        }
    }

    // ---- png_warning_parameter* / png_formatted_warning (rows 10-14) ----
    let msgs: [&[u8]; 14] = [
        b"plain\0",
        b"@1\0",
        b"@1 and @2\0",
        b"@8 @9 @0\0",
        b"@\0",
        b"a@\0",
        b"@@1\0",
        b"@x\0",
        b"@1@2@3@4@5@6@7@8\0",
        b"\0",
        b"@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1@1\0",
        b"@-1\0",
        b"@ \0",
        b"tail@1tail\0",
    ];
    for m in msgs {
        for numbers in [
            vec![0i32, 1, 2, 8, 9, -1, 100],
            vec![1i32, 2, 3, 4, 5, 6, 7, 8],
        ] {
            let mut res = Vec::new();
            for a in [&b.c, &b.rs] {
                log_reset();
                unsafe {
                    let (p, _i) = new_write(a);
                    let mut params: WarnParams = [[0; 32]; 8];
                    for (i, n) in numbers.iter().enumerate() {
                        let s = std::ffi::CString::new(format!("P{n}#{i}")).unwrap();
                        (a.png_warning_parameter)(
                            params.as_mut_ptr() as *mut c_void,
                            *n,
                            s.as_ptr(),
                        );
                        (a.png_warning_parameter_unsigned)(
                            params.as_mut_ptr() as *mut c_void,
                            *n,
                            1,
                            (i as usize) * 1000,
                        );
                        (a.png_warning_parameter_signed)(
                            params.as_mut_ptr() as *mut c_void,
                            *n,
                            4,
                            -(i as i32) - 1,
                        );
                    }
                    // an over-long parameter, to hit the png_safecat truncation
                    let long = std::ffi::CString::new("L".repeat(100)).unwrap();
                    (a.png_warning_parameter)(
                        params.as_mut_ptr() as *mut c_void,
                        1,
                        long.as_ptr(),
                    );
                    (a.png_formatted_warning)(
                        p,
                        params.as_mut_ptr() as *mut c_void,
                        m.as_ptr() as *const c_char,
                    );
                    // and with a NULL parameter block
                    (a.png_formatted_warning)(
                        p,
                        std::ptr::null_mut(),
                        m.as_ptr() as *const c_char,
                    );
                    res.push(log_take());
                }
            }
            eq_dbg(
                &format!(
                    "png_formatted_warning({:?},{numbers:?})",
                    String::from_utf8_lossy(m)
                ),
                res[0].clone(),
                res[1].clone(),
            );
            n += 1;
        }
    }
    eprintln!("safecat/format_number/check_fp/formatted_warning comparisons: {n}");
}

// ===========================================================================
// 12. png_check_keyword
// ===========================================================================

#[test]
fn check_keyword_cases() {
    let b = apis();
    let mut rng = Rng::new(0x2412);

    let mut keys: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"Title".to_vec(),
        b" Title".to_vec(),
        b"Title ".to_vec(),
        b" ".to_vec(),
        b"   ".to_vec(),
        b"A  B".to_vec(),
        b"A   B".to_vec(),
        b"A B".to_vec(),
        b"A\tB".to_vec(),
        b"\t".to_vec(),
        b"\x01\x02\x03".to_vec(),
        b"A\x01B".to_vec(),
        b"A\x7fB".to_vec(),
        b"\x7f".to_vec(),
        b"\x80".to_vec(),          // non-Latin-1 control range
        b"A\x80B".to_vec(),
        b"\xa0".to_vec(),          // 0xa0 = NBSP, forbidden
        b"A\xa0B".to_vec(),
        b"\xa1".to_vec(),          // 0xa1 is allowed
        b"A\xa1B".to_vec(),
        b"\xff".to_vec(),
        b"\x20\x20\x20\x41".to_vec(),
        b"\x41\x20\x20\x20".to_vec(),
        b"a".repeat(78),
        b"a".repeat(79),
        b"a".repeat(80),
        b"a".repeat(81),
        b"a".repeat(200),
        b"\x7f".repeat(100),
    ];
    // 79/80-byte keywords with a space or bad char at the boundary
    for n in [77usize, 78, 79, 80] {
        let mut k = b"a".repeat(n);
        k.push(b' ');
        k.extend_from_slice(b"bcd");
        keys.push(k);
        let mut k = b"a".repeat(n);
        k.push(0x01);
        k.extend_from_slice(b"bcd");
        keys.push(k);
    }
    // random keywords over an alphabet that mixes every class
    let alphabet: Vec<u8> = vec![
        1, 9, 0x20, 0x21, 0x41, 0x61, 0x7e, 0x7f, 0x80, 0x9f, 0xa0, 0xa1, 0xfe, 0xff, 0x30,
    ];
    for _ in 0..4000 {
        let n = rng.below(90) as usize;
        keys.push((0..n).map(|_| *rng.pick(&alphabet)).collect());
    }

    let mut nonzero = 0usize;
    let mut zero = 0usize;
    for k in &keys {
        let cs = std::ffi::CString::new(k.clone()).unwrap_or_else(|_| {
            // no interior NULs are generated, but be defensive
            std::ffi::CString::new(k.iter().copied().filter(|c| *c != 0).collect::<Vec<u8>>())
                .unwrap()
        });
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            log_reset();
            unsafe {
                let (p, _i) = new_write(a);
                let mut nk = [0xAAu8; 128];
                let len = (a.png_check_keyword)(p, cs.as_ptr(), nk.as_mut_ptr());
                res.push((len, nk.to_vec(), log_take()));
            }
        }
        let what = format!("png_check_keyword({:?})", String::from_utf8_lossy(k));
        eq_dbg(&format!("{what} len"), res[0].0, res[1].0);
        eq_bytes(&format!("{what} new_key"), &res[0].1, &res[1].1);
        eq_dbg(&format!("{what} warnings"), res[0].2.clone(), res[1].2.clone());
        if res[0].0 == 0 {
            zero += 1;
        } else {
            nonzero += 1;
        }
    }
    eprintln!("png_check_keyword comparisons: {}", keys.len() + 1);
    assert!(zero > 5 && nonzero > 100, "zero={zero} nonzero={nonzero}");

    // `key == NULL` is explicitly guarded (pngset.c:1992)
    let mut res = Vec::new();
    for a in [&b.c, &b.rs] {
        log_reset();
        unsafe {
            let (p, _i) = new_write(a);
            let mut nk = [0xAAu8; 128];
            let len = (a.png_check_keyword)(p, std::ptr::null(), nk.as_mut_ptr());
            res.push((len, nk.to_vec(), log_take()));
        }
    }
    eq_dbg("png_check_keyword(NULL) len", res[0].0, res[1].0);
    eq_bytes("png_check_keyword(NULL) new_key", &res[0].1, &res[1].1);
    eq_dbg(
        "png_check_keyword(NULL) warnings",
        res[0].2.clone(),
        res[1].2.clone(),
    );
}

// ===========================================================================
// 13. png_check_IHDR called directly
// ===========================================================================

#[test]
fn check_IHDR_direct_all_warning_sites() {
    let mut cases = Vec::new();
    // every bit depth x colour type
    for bd in [0i32, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 32, -1, i32::MAX, i32::MIN] {
        for ct in [0i32, 1, 2, 3, 4, 5, 6, 7, 8, -1, i32::MAX, i32::MIN] {
            cases.push(format!("cihdr:4:2:{bd}:{ct}:0:0:0:0"));
        }
    }
    // width / height, including the PNG_UINT_31_MAX and user-limit boundaries
    for (w, h) in [
        (0u32, 2u32),
        (4, 0),
        (0, 0),
        (1, 1),
        (999_999, 1),
        (1_000_000, 1),
        (1_000_001, 1),
        (1, 999_999),
        (1, 1_000_000),
        (1, 1_000_001),
        (0x7fff_ffff, 1),
        (0x8000_0000, 1),
        (0xffff_ffff, 1),
        (1, 0x7fff_ffff),
        (1, 0x8000_0000),
        (1, 0xffff_ffff),
        (0x8000_0000, 0x8000_0000),
        (0, 0xffff_ffff),
    ] {
        cases.push(format!("cihdr:{w}:{h}:8:2:0:0:0:0"));
        // with png_set_user_limits(100, 200)
        cases.push(format!("cihdr:{w}:{h}:8:2:0:0:0:4"));
    }
    for (w, h) in [(99u32, 199u32), (100, 200), (101, 200), (100, 201)] {
        cases.push(format!("cihdr:{w}:{h}:8:2:0:0:0:4"));
    }
    // interlace / compression / filter
    for il in [0i32, 1, 2, 3, 255, -1, i32::MAX] {
        cases.push(format!("cihdr:4:2:8:2:{il}:0:0:0"));
    }
    for cm in [0i32, 1, 2, 8, 255, -1, i32::MAX] {
        cases.push(format!("cihdr:4:2:8:2:0:{cm}:0:0"));
    }
    // filter method x MNG permission x signature-seen, which selects between
    // "Unknown filter method in IHDR" and "Invalid filter method in IHDR"
    for fm in [0i32, 1, 2, 63, 64, 65, 255, -1] {
        for ct in [0i32, 2, 3, 6] {
            for flags in [0u32, 1, 2, 3] {
                cases.push(format!("cihdr:4:2:8:{ct}:0:0:{fm}:{flags}"));
            }
        }
    }
    // the MNG-features-in-a-PNG-datastream warning on its own (valid IHDR)
    for flags in [0u32, 1, 2, 3] {
        cases.push(format!("cihdr:4:2:8:2:0:0:0:{flags}"));
    }
    // NOTE: "Image width is too large for this architecture" (png.c:1989) is
    // UNREACHABLE on a 64-bit target: the threshold is
    // ((PNG_SIZE_MAX-48-1)/8)-1 == 0x1fff_ffff_ffff_fffd, far above any
    // png_uint_32 width, so the row is recorded here rather than tested.
    run_all(&cases);
}

// ===========================================================================
// 14. png_chunk_unknown_handling / png_handle_as_unknown
// ===========================================================================

#[test]
fn unknown_chunk_handling() {
    let b = apis();

    // chunk names, including ones with bytes that are not valid PNG chunk chars
    let names: Vec<[u8; 4]> = vec![
        *b"bKGD", *b"cHRM", *b"gAMA", *b"iCCP", *b"zTXt", *b"tEXt", *b"IHDR", *b"IDAT",
        *b"IEND", *b"PLTE", *b"tRNS", *b"sTER", *b"prVt", *b"pRvT", *b"XXXX", *b"aaaa",
        *b"ZZZZ", [0, 0, 0, 0], [1, 2, 3, 4], [0xff, 0xff, 0xff, 0xff], [0x20, 0x20, 0x20, 0x20],
        [0x7f, 0x80, 0x81, 0x82], *b"a\0bc", [b'a', b'B', b'0', b'9'],
    ];

    // every configuration of png_set_keep_unknown_chunks
    // (keep, num_chunks_in, list) -- keep must be 0..=3 and, when
    // num_chunks_in > 0, chunk_list must be non-NULL, otherwise
    // png_set_keep_unknown_chunks raises a FATAL png_app_error (pngset.c:1611,
    // :1665) which cannot be observed in-process.
    let lists: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"bKGD\0".to_vec(),
        b"bKGD\0cHRM\0".to_vec(),
        b"prVt\0XXXX\0aaaa\0".to_vec(),
        vec![0, 0, 0, 0, 0],
        vec![0xff, 0xff, 0xff, 0xff, 0],
        b"bKGD\0bKGD\0".to_vec(),
    ];

    let mut n = 0usize;
    for read in [false, true] {
        for keep in [0i32, 1, 2, 3] {
            for (li, list) in lists.iter().enumerate() {
                for num in [-1i32, 0, (list.len() / 5) as i32] {
                    if num > 0 && list.is_empty() {
                        continue;
                    }
                    let mut res = Vec::new();
                    for a in [&b.c, &b.rs] {
                        log_reset();
                        unsafe {
                            let (p, _i) = if read { new_read(a) } else { new_write(a) };
                            let lp = if list.is_empty() {
                                std::ptr::null()
                            } else {
                                list.as_ptr()
                            };
                            (a.png_set_keep_unknown_chunks)(p, keep, lp, num);
                            let mut got: Vec<i32> = Vec::new();
                            for nm in &names {
                                got.push((a.png_handle_as_unknown)(p, nm.as_ptr()));
                                let u32name = u32::from_be_bytes(*nm);
                                got.push((a.png_chunk_unknown_handling)(p, u32name));
                            }
                            // the two guarded NULL rows
                            got.push((a.png_handle_as_unknown)(p, std::ptr::null()));
                            got.push((a.png_handle_as_unknown)(
                                std::ptr::null_mut(),
                                names[0].as_ptr(),
                            ));
                            got.push((a.png_handle_as_unknown)(
                                std::ptr::null_mut(),
                                std::ptr::null(),
                            ));
                            res.push((got, log_take()));
                        }
                    }
                    let what =
                        format!("keep={keep} list={li} num={num} read={read}");
                    eq_dbg(&format!("{what} handling"), res[0].0.clone(), res[1].0.clone());
                    eq_dbg(&format!("{what} warnings"), res[0].1.clone(), res[1].1.clone());
                    n += 1;
                }
            }
        }
    }
    eprintln!("unknown-chunk configurations: {n} (x {} names each)", names.len());
    assert!(n > 50, "expected many configurations, got {n}");

    // arbitrary 32-bit chunk names through png_chunk_unknown_handling, with a
    // list installed so the memcmp loop actually runs
    let list = b"bKGD\0prVt\0\0\0\0\0\0".to_vec();
    let mut res = Vec::new();
    for a in [&b.c, &b.rs] {
        log_reset();
        unsafe {
            let (p, _i) = new_read(a);
            (a.png_set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, list.as_ptr(), 3);
            let mut r = Rng::new(0x2414);
            let mut got = Vec::new();
            for _ in 0..5000 {
                got.push((a.png_chunk_unknown_handling)(p, r.next_u32()));
            }
            res.push((got, log_take()));
        }
    }
    eq_dbg(
        "png_chunk_unknown_handling random names",
        res[0].0.clone(),
        res[1].0.clone(),
    );
    eq_dbg(
        "png_chunk_unknown_handling random names warnings",
        res[0].1.clone(),
        res[1].1.clone(),
    );
}

// ===========================================================================
// 15. png_reset_zstream
// ===========================================================================

#[test]
fn reset_zstream_states() {
    let b = apis();
    // (a) NULL struct -> Z_STREAM_ERROR (png.c:981)
    unsafe {
        eq_dbg(
            "png_reset_zstream(NULL)",
            (b.c.png_reset_zstream)(std::ptr::null_mut()),
            (b.rs.png_reset_zstream)(std::ptr::null_mut()),
        );
    }
    // (b) a never-initialised zstream -> inflateReset fails (png.c:985)
    let mut res = Vec::new();
    for a in [&b.c, &b.rs] {
        log_reset();
        unsafe {
            let (p, _i) = new_read(a);
            let r1 = (a.png_reset_zstream)(p);
            let r2 = (a.png_reset_zstream)(p);
            res.push((r1, r2, log_take()));
        }
    }
    eq_dbg("reset_zstream fresh", res[0].clone(), res[1].clone());
    assert_eq!(res[0].0, -2, "expected Z_STREAM_ERROR, got {:?}", res[0]);

    // (c) after a successful read of a stream that inflates something, and
    //     after a failed one
    let valid = pb::make_png(0x2415, 4, 3, 8, 2, 0);
    let with_ztxt = ztxt_png(&pb::zlib_store(b"some text"));
    let broken = ztxt_png(&[0x78, 0x02, 0x03, 0x00]);
    let truncated = {
        let mut v = valid.clone();
        v.truncate(v.len() - 12);
        v
    };
    for (label, png) in [
        ("valid", &valid),
        ("ztxt", &with_ztxt),
        ("broken-ztxt", &broken),
        ("truncated", &truncated),
    ] {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            log_reset();
            set_cur_is_c(std::ptr::eq(a, &b.c));
            unsafe {
                let mut img = png_image::default();
                let begin = (a.png_image_begin_read_from_memory)(
                    &mut img,
                    png.as_ptr() as *const c_void,
                    png.len(),
                );
                let mut finish = -1;
                if begin != 0 {
                    img.format = PNG_FORMAT_RGBA;
                    let mut buf = vec![0u8; 4096];
                    finish = (a.png_image_finish_read)(
                        &mut img,
                        std::ptr::null(),
                        buf.as_mut_ptr() as *mut c_void,
                        0,
                        std::ptr::null_mut(),
                    );
                }
                (a.png_image_free)(&mut img);
                res.push((begin, finish, log_take()));
            }
        }
        eq_dbg(
            &format!("simplified read of {label}"),
            res[0].clone(),
            res[1].clone(),
        );
    }
    // the low-level route, which keeps the struct alive so png_reset_zstream can
    // be called afterwards
    for (label, png) in [("valid", &valid), ("ztxt", &with_ztxt)] {
        let mut res = Vec::new();
        for a in [&b.c, &b.rs] {
            log_reset();
            set_cur_is_c(std::ptr::eq(a, &b.c));
            unsafe {
                let (p, info) = new_read(a);
                in_set(png);
                (a.png_read_info)(p, info);
                let before = (a.png_reset_zstream)(p);
                let rb = (a.png_get_rowbytes)(p, info);
                let h = (a.png_get_image_height)(p, info);
                let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb + 8]).collect();
                let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
                (a.png_read_image)(p, ptrs.as_mut_ptr());
                (a.png_read_end)(p, info);
                let after = (a.png_reset_zstream)(p);
                let mut pp = p;
                (a.png_destroy_read_struct)(
                    &mut pp,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                res.push((before, after, log_take()));
            }
        }
        eq_dbg(
            &format!("reset_zstream around a {label} read"),
            res[0].clone(),
            res[1].clone(),
        );
        assert_eq!(
            res[0].1, 0,
            "after a real read inflateReset must succeed: {:?}",
            res[0]
        );
    }
}

// ===========================================================================
// 16. pngrio.c / pngwio.c
// ===========================================================================

#[test]
fn rio_wio_rejections() {
    // NOTE on reachability, from the C:
    //  * `png_create_read_struct` ends with `png_set_read_fn(png_ptr, NULL,
    //    NULL)` (pngread.c:76), and `png_set_read_fn` substitutes
    //    `png_default_read_data` for a NULL function (pngrio.c:95).  The same is
    //    true on the write side (pngwio.c:130).  So "Call to NULL read/write
    //    function" can NOT be produced by passing NULL to the setters; the only
    //    route is the cross-wiring branch (pngrio.c:106 / pngwio.c:157) which
    //    NULLs the *other* direction's function pointer and warns.
    //  * `png_default_read_data`/`png_default_write_data` with `io_ptr == NULL`
    //    call `fread`/`fwrite` on a NULL `FILE*`, which segfaults: there is no
    //    guard, so those inputs are NOT tested.  A real stdio stream is used
    //    instead (`/dev/null` opened read-only), which makes the short-read and
    //    the failed-write branches fire deterministically.
    run_all(&[
        "read-fn-nulled-then-read".into(),
        "write-fn-nulled-then-write".into(),
        "read-error-eof".into(),
        "read-error-zero-length".into(),
        "read-error-default-direct".into(),
        "read-default-null-struct".into(),
        "write-error-readonly-file".into(),
        "write-error-default-direct".into(),
        "write-default-null-struct".into(),
        "write-error-zero-length".into(),
        "flush-null-io".into(),
    ]);
}

// ===========================================================================
// self-check: prove the comparisons are not vacuous
// ===========================================================================

#[test]
fn self_check() {
    // --- the two fatal messages this file is mainly about ---
    let t = run_child("afp:0:2:1", "c");
    assert_eq!(t.exit, Some(70), "expected a fatal error, got {t:?}");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:ASCII conversion buffer too small"),
        "expected the ASCII-buffer message, got {:?}",
        t.lines
    );
    let t = run_child("afx:0:0", "c");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:ASCII conversion buffer too small"),
        "expected the ASCII-buffer message from png_ascii_from_fixed, got {:?}",
        t.lines
    );

    let t = run_child(&format!("fx:{:016x}:0", 1e300f64.to_bits()), "c");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:fixed point overflow in cHRM White X"),
        "expected the fixed-point message, got {:?}",
        t.lines
    );
    let t = run_child(&format!("fxnull:{:016x}", 1e300f64.to_bits()), "c");
    assert!(
        t.lines.iter().any(|l| l == "ERROR:fixed point overflow in "),
        "expected the NULL-name fixed-point message, got {:?}",
        t.lines
    );
    let t = run_child(&format!("fxitu:{:016x}", (-1.0f64).to_bits()), "c");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:fixed point overflow in png_set_cLLI(maxCLL)"),
        "expected the ITU fixed-point message, got {:?}",
        t.lines
    );

    // --- pngmem.c internal errors ---
    let t = run_child("marr:0:8", "c");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:internal error: array alloc"),
        "got {:?}",
        t.lines
    );
    let t = run_child("rarr:0:0:8:0", "c");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:internal error: array realloc"),
        "got {:?}",
        t.lines
    );

    // --- png_check_IHDR: each warning site really produces its own text ---
    let expect: [(&str, &str); 8] = [
        ("cihdr:0:2:8:2:0:0:0:0", "WARN:Image width is zero in IHDR"),
        (
            "cihdr:2147483648:2:8:2:0:0:0:0",
            "WARN:Invalid image width in IHDR",
        ),
        (
            "cihdr:1000001:2:8:2:0:0:0:0",
            "WARN:Image width exceeds user limit in IHDR",
        ),
        ("cihdr:4:0:8:2:0:0:0:0", "WARN:Image height is zero in IHDR"),
        (
            "cihdr:4:2147483648:8:2:0:0:0:0",
            "WARN:Invalid image height in IHDR",
        ),
        ("cihdr:4:2:3:2:0:0:0:0", "WARN:Invalid bit depth in IHDR"),
        ("cihdr:4:2:8:1:0:0:0:0", "WARN:Invalid color type in IHDR"),
        (
            "cihdr:4:2:8:2:2:0:0:0",
            "WARN:Unknown interlace method in IHDR",
        ),
    ];
    for (case, want) in expect {
        let t = run_child(case, "c");
        assert!(
            t.lines.iter().any(|l| l == want),
            "case {case}: expected {want:?}, got {:?}",
            t.lines
        );
        assert!(
            t.lines.iter().any(|l| l == "ERROR:Invalid IHDR data"),
            "case {case}: expected the terminal error, got {:?}",
            t.lines
        );
    }
    // the remaining warning sites
    for (case, want) in [
        (
            "cihdr:4:2:16:3:0:0:0:0",
            "WARN:Invalid color type/bit depth combination in IHDR",
        ),
        (
            "cihdr:4:2:8:2:0:1:0:0",
            "WARN:Unknown compression method in IHDR",
        ),
        ("cihdr:4:2:8:2:0:0:1:0", "WARN:Unknown filter method in IHDR"),
        ("cihdr:4:2:8:2:0:0:64:1", "WARN:Invalid filter method in IHDR"),
        (
            "cihdr:4:2:8:2:0:0:0:3",
            "WARN:MNG features are not allowed in a PNG datastream",
        ),
    ] {
        let t = run_child(case, "c");
        assert!(
            t.lines.iter().any(|l| l == want),
            "case {case}: expected {want:?}, got {:?}",
            t.lines
        );
    }

    // --- pngrio.c / pngwio.c ---
    let t = run_child("read-fn-nulled-then-read", "c");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:Call to NULL read function"),
        "got {:?}",
        t.lines
    );
    assert!(
        t.lines.iter().any(|l| l.starts_with(
            "WARN:Can't set both read_data_fn and write_data_fn in the same structure"
        )),
        "expected the cross-wiring warning, got {:?}",
        t.lines
    );
    let t = run_child("write-fn-nulled-then-write", "c");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:Call to NULL write function"),
        "got {:?}",
        t.lines
    );
    let t = run_child("read-error-eof", "c");
    assert!(
        t.lines.iter().any(|l| l == "ERROR:Read Error"),
        "got {:?}",
        t.lines
    );
    let t = run_child("write-error-readonly-file", "c");
    assert!(
        t.lines.iter().any(|l| l == "ERROR:Write Error"),
        "got {:?}",
        t.lines
    );

    // --- the ICC rejection strings really appear, from the C library ---
    let b = apis();
    let (cl, _) = both::<FnIccLength>("png_icc_check_length");
    let (ch, _) = both::<FnIccHeader>("png_icc_check_header");
    let (ct, _) = both::<FnIccTags>("png_icc_check_tag_table");
    let name = std::ffi::CString::new("p").unwrap();
    let mut all: Vec<String> = Vec::new();
    for case in icc_cases() {
        log_reset();
        unsafe {
            let (p, _i) = new_read(&b.c);
            (cl)(p, name.as_ptr(), case.length);
            (ch)(
                p,
                name.as_ptr(),
                case.length,
                case.profile.as_ptr(),
                case.color_type,
            );
            if case.tags_ok {
                (ct)(p, name.as_ptr(), case.length, case.profile.as_ptr());
            }
        }
        all.extend(log_take());
    }
    for want in [
        "too short",
        "invalid length",
        "tag count too large",
        "invalid rendering intent",
        "intent outside defined range",
        "invalid signature",
        "PCS illuminant is not D50",
        "RGB color space not permitted on grayscale PNG",
        "Gray color space not permitted on RGB PNG",
        "invalid ICC profile color space",
        "invalid embedded Abstract ICC profile",
        "unexpected DeviceLink ICC profile class",
        "unexpected NamedColor ICC profile class",
        "unrecognized ICC profile class",
        "unexpected ICC PCS encoding",
        "ICC profile tag outside profile",
        "ICC profile tag start not a multiple of 4",
        "length does not match profile",
    ] {
        assert!(
            all.iter().any(|l| l.ends_with(want)),
            "expected an ICC warning ending in {want:?}; collected {} warnings",
            all.len()
        );
    }
    // and "profile too long", which needs a lowered chunk limit
    log_reset();
    unsafe {
        let (p, _i) = new_read(&b.c);
        (b.c.png_set_chunk_malloc_max)(p, 200);
        let r = (cl)(p, name.as_ptr(), 1000);
        assert_eq!(r, 0);
    }
    let l = log_take();
    assert!(
        l.iter().any(|x| x.ends_with("profile too long")),
        "got {l:?}"
    );

    // --- the sentinels really are sentinels in the C ---
    unsafe {
        let mut r: i32 = 0x1234;
        assert_eq!(
            (b.c.png_muldiv)(&mut r, 1, 1, 0),
            0,
            "png_muldiv must fail on a zero divisor"
        );
        assert_eq!(r, 0x1234, "png_muldiv must not write *res on failure");
        assert_eq!((b.c.png_reciprocal)(0), 0);
        assert_eq!((b.c.png_reciprocal2)(0, 1), 0);
        assert_eq!((b.c.png_reciprocal2)(1, 0), 0);
        assert_eq!((b.c.png_reset_zstream)(std::ptr::null_mut()), -2);
        let (p, _i) = new_read(&b.c);
        assert_eq!(
            (b.c.png_handle_as_unknown)(p, std::ptr::null()),
            PNG_HANDLE_CHUNK_AS_DEFAULT
        );
        assert!((b.c.png_malloc_array)(p, i32::MAX, usize::MAX).is_null());
        let bad = png_time {
            year: 10000,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        };
        let mut buf = [0x41u8 as c_char; 40];
        assert_eq!((b.c.png_convert_to_rfc1123_buffer)(buf.as_mut_ptr(), &bad), 0);
        assert_eq!(
            (b.c.png_convert_to_rfc1123_buffer)(std::ptr::null_mut(), &bad),
            0
        );
        // png_XYZ_from_xy rejects whitey < 5 and anything above 110000
        let mut xyz = png_XYZ::default();
        let (cf, _) = both::<FnXYZfromxy>("png_XYZ_from_xy");
        let mut xy = png_xy {
            redx: 64000,
            redy: 33000,
            greenx: 30000,
            greeny: 60000,
            bluex: 15000,
            bluey: 6000,
            whitex: 31270,
            whitey: 32900,
        };
        assert_eq!(cf(&mut xyz, &xy), 0, "sRGB primaries must be accepted");
        xy.whitey = 4;
        assert_eq!(cf(&mut xyz, &xy), 1, "whitey < 5 must be rejected");
        xy.whitey = 32900;
        xy.redx = 110001;
        assert_eq!(cf(&mut xyz, &xy), 1, "redx > 110000 must be rejected");
    }

    // --- png_user_version_check really warns and returns 0 ---
    log_reset();
    let r = unsafe {
        let (p, _i) = new_read(&b.c);
        (b.c.png_user_version_check)(p, c"1.5.59".as_ptr())
    };
    let l = log_take();
    assert_eq!(r, 0);
    assert!(
        l.iter().any(|x| x
            == "WARN:Application built with libpng-1.5.59 but running with 1.6.59.git"),
        "got {l:?}"
    );
    log_reset();
    let r = unsafe {
        let (p, _i) = new_read(&b.c);
        (b.c.png_user_version_check)(p, PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char)
    };
    assert_eq!(r, 1, "the exact version must be accepted");
    assert!(log_take().is_empty());

    // --- png_check_keyword really rewrites and warns ---
    log_reset();
    let (len, key, warns) = unsafe {
        let (p, _i) = new_write(&b.c);
        let mut nk = [0xAAu8; 128];
        let l = (b.c.png_check_keyword)(p, c" A  B ".as_ptr(), nk.as_mut_ptr());
        (l, nk, log_take())
    };
    assert_eq!(len, 3, "\" A  B \" must collapse to \"A B\"");
    assert_eq!(&key[..4], b"A B\0");
    assert!(
        warns.iter().any(|w| w.contains("bad character")),
        "got {warns:?}"
    );
    log_reset();
    let l0 = unsafe {
        let (p, _i) = new_write(&b.c);
        let mut nk = [0xAAu8; 128];
        (b.c.png_check_keyword)(p, c"   ".as_ptr(), nk.as_mut_ptr())
    };
    let _ = log_take();
    assert_eq!(l0, 0, "an all-space keyword must be rejected");
    log_reset();
    let (lt, warns) = unsafe {
        let (p, _i) = new_write(&b.c);
        let mut nk = [0xAAu8; 128];
        let long = std::ffi::CString::new("k".repeat(200)).unwrap();
        let l = (b.c.png_check_keyword)(p, long.as_ptr(), nk.as_mut_ptr());
        (l, log_take())
    };
    assert_eq!(lt, 79, "keywords are truncated to 79 bytes");
    assert!(
        warns.iter().any(|w| w == "WARN:keyword truncated"),
        "got {warns:?}"
    );

    // --- distinct cases really produce distinct transcripts ---
    let a = run_child("afp:0:2:1", "c");
    let c2 = run_child("marr:0:8", "c");
    assert_ne!(a.lines, c2.lines);
}

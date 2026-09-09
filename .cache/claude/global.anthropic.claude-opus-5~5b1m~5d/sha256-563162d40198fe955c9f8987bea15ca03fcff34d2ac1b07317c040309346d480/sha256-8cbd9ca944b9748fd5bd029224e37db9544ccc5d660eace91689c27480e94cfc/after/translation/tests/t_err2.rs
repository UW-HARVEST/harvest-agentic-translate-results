//! Error-path differential verification of `c_src/src/pngset.c`.
//!
//! Covers rows 221..=279 of `translation/ERRORS.md` (see `.verify/E2.md`).
//! Every assertion constructs one invalid input, runs it through BOTH the
//! reference C `libpng.so` and the translated Rust `liblibpng.so` (always via
//! `dlsym`), and requires an identical message log *and* identical observable
//! results / side effects.
//!
//! Rows that the build configuration makes unreachable are documented in
//! `rows_255_265_266_unreachable_by_configuration` and
//! `row_275_unreachable_on_64bit`.

#![allow(non_snake_case)]

mod common;

use common::api;
use common::*;
use libloading::Library;
use std::cell::Cell;
use std::ffi::{c_int, c_void, CStr, CString};
use std::ptr::null_mut;

// --------------------------------------------------------------------------
// extra entry points that tests/common/api.rs does not declare
// --------------------------------------------------------------------------
crate::decl_api! {
    fn png_check_keyword(pp: png_structp, key: png_const_charp, new_key: png_bytep)
        -> png_uint_32;
}

const PNG_TEXT_COMPRESSION_LAST: c_int = 3;
const PNG_HANDLE_CHUNK_LAST: c_int = 4;
const PNG_MAX_PALETTE_LENGTH: c_int = 256;

fn ver() -> png_const_charp {
    PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp
}

// --------------------------------------------------------------------------
// differential driver
// --------------------------------------------------------------------------

/// `c_src/build/libpng.so` has no `DT_NEEDED` entry for libm (it expects the
/// consumer to provide the math functions), and the Rust test executable does
/// not link libm either, so the C library's lazy binding of e.g. `floor`
/// (reached from `png_ascii_from_fp`) would abort the process with
/// "symbol lookup error".  Put libm into the global lookup scope first.
static LIBM_INIT: std::sync::Once = std::sync::Once::new();

fn ensure_libm() {
    LIBM_INIT.call_once(|| unsafe {
        const RTLD_NOW: c_int = 2;
        const RTLD_GLOBAL: c_int = 0x100;
        let lib = libloading::os::unix::Library::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL)
            .expect("dlopen libm.so.6 with RTLD_GLOBAL");
        std::mem::forget(lib);
    });
}

/// Run `f` against the C library and then the Rust library and require the
/// same message log, the same "did it error out" answer and the same result.
fn diff<T, F>(label: &str, f: F)
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&'static Library) -> T,
{
    ensure_libm();
    let l = libs();
    let a = capture_strict(|| f(&l.c));
    let b = capture_strict(|| f(&l.rs));
    if std::env::var_os("T_ERR2_TRACE").is_some() {
        eprintln!(
            "[C] {label} :: errored={} :: {}",
            a.out.is_none(),
            a.log.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(" | ")
        );
    }
    a.assert_eq(&b, label);
}

// --------------------------------------------------------------------------
// deterministic allocation-failure injection
//
// libpng lets the application supply the allocator
// (png_create_{read,write}_struct_2).  `arm(n)` makes the n-th allocation
// request *counted from the moment of arming* fail; everything else is a plain
// malloc.  Arming happens immediately before the call under test, so the
// failing allocation is the n-th one made by that call, which is fixed by the
// C source.  This is the only way to reach the "Insufficient memory ..." /
// "too many ..." rejection sites.
// --------------------------------------------------------------------------

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

thread_local! {
    static FAIL_AT: Cell<i32> = const { Cell::new(0) };
    static NALLOC: Cell<i32> = const { Cell::new(0) };
    static FAIL_MIN: Cell<usize> = const { Cell::new(0) };
}

fn arm(n: i32) {
    FAIL_AT.with(|c| c.set(n));
    NALLOC.with(|c| c.set(0));
}
/// Fail every allocation of at least `s` bytes while armed.  Used where a
/// count-based rule would be unsafe (a succeeding huge allocation would make
/// libpng memcpy gigabytes out of the caller's small buffer).
fn arm_min_size(s: usize) {
    FAIL_MIN.with(|c| c.set(s));
}
fn disarm() {
    FAIL_AT.with(|c| c.set(0));
    NALLOC.with(|c| c.set(0));
    FAIL_MIN.with(|c| c.set(0));
}

unsafe extern "C-unwind" fn t_malloc(_pp: png_structp, size: usize) -> png_voidp {
    let min = FAIL_MIN.with(|c| c.get());
    if min > 0 && size >= min {
        return null_mut();
    }
    let at = FAIL_AT.with(|c| c.get());
    if at > 0 {
        let n = NALLOC.with(|c| {
            let v = c.get() + 1;
            c.set(v);
            v
        });
        if n == at {
            return null_mut();
        }
    }
    libc_malloc(size)
}

unsafe extern "C-unwind" fn t_free(_pp: png_structp, p: png_voidp) {
    libc_free(p)
}

// --------------------------------------------------------------------------
// fresh struct helpers - a new png_struct + png_info for every operation
// --------------------------------------------------------------------------

unsafe fn wr(l: &Library) -> (png_structp, png_infop) {
    disarm();
    let pp = api::new_writer(l);
    let ip = api::png_create_info_struct(l, pp);
    assert!(!ip.is_null(), "info struct");
    (pp, ip)
}

unsafe fn rd(l: &Library) -> (png_structp, png_infop) {
    disarm();
    src_set(&[]);
    let pp = api::new_reader(l);
    let ip = api::png_create_info_struct(l, pp);
    assert!(!ip.is_null(), "info struct");
    (pp, ip)
}

/// Writer whose allocator is the injectable one.
unsafe fn wr_mem(l: &Library) -> (png_structp, png_infop) {
    disarm();
    let pp = api::png_create_write_struct_2(
        l,
        ver(),
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
        1usize as png_voidp,
        Some(t_malloc),
        Some(t_free),
    );
    assert!(!pp.is_null(), "create_write_struct_2");
    api::png_set_write_fn(l, pp, 1usize as png_voidp, Some(write_cb), Some(flush_cb));
    let ip = api::png_create_info_struct(l, pp);
    assert!(!ip.is_null(), "info struct");
    (pp, ip)
}

/// Reader whose allocator is the injectable one.
unsafe fn rd_mem(l: &Library) -> (png_structp, png_infop) {
    disarm();
    src_set(&[]);
    let pp = api::png_create_read_struct_2(
        l,
        ver(),
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
        1usize as png_voidp,
        Some(t_malloc),
        Some(t_free),
    );
    assert!(!pp.is_null(), "create_read_struct_2");
    api::png_set_read_fn(l, pp, 1usize as png_voidp, Some(read_cb));
    let ip = api::png_create_info_struct(l, pp);
    assert!(!ip.is_null(), "info struct");
    (pp, ip)
}

/// `png_get_valid` for every flag we care about, as one comparable number.
unsafe fn valid(l: &Library, pp: png_structp, ip: png_infop) -> png_uint_32 {
    api::png_get_valid(l, pp, ip, 0xffff_ffff)
}

fn cstr_vec(p: png_charp) -> Vec<u8> {
    if p.is_null() {
        b"<null>".to_vec()
    } else {
        unsafe { CStr::from_ptr(p) }.to_bytes().to_vec()
    }
}

unsafe fn unk_snapshot(
    l: &Library,
    pp: png_structp,
    ip: png_infop,
) -> Vec<([u8; 5], usize, png_byte, Vec<u8>)> {
    let mut p: *mut png_unknown_chunk = null_mut();
    let n = api::png_get_unknown_chunks(l, pp, ip, &mut p);
    let mut v = Vec::new();
    if !p.is_null() {
        for i in 0..n as isize {
            let c = *p.offset(i);
            let data = if c.data.is_null() || c.size == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(c.data, c.size).to_vec()
            };
            v.push((c.name, c.size, c.location, data));
        }
    }
    v
}

type TextRow = (c_int, Vec<u8>, Vec<u8>, usize, usize, bool, bool);

unsafe fn text_snapshot(l: &Library, pp: png_structp, ip: png_infop) -> (c_int, Vec<TextRow>) {
    let mut p: *mut png_text = null_mut();
    let mut n: c_int = 0;
    api::png_get_text(l, pp, ip, &mut p, &mut n);
    let mut v = Vec::new();
    if !p.is_null() {
        for i in 0..n as isize {
            let t = *p.offset(i);
            v.push((
                t.compression,
                cstr_vec(t.key),
                cstr_vec(t.text),
                t.text_length,
                t.itxt_length,
                t.lang.is_null(),
                t.lang_key.is_null(),
            ));
        }
    }
    (n, v)
}

fn mk_text(compression: c_int, key: &CString, text: &CString) -> png_text {
    png_text {
        compression,
        key: key.as_ptr() as png_charp,
        text: text.as_ptr() as png_charp,
        text_length: 0,
        itxt_length: 0,
        lang: null_mut(),
        lang_key: null_mut(),
    }
}

fn mk_unk(name: &[u8; 4], data: *mut png_byte, size: usize, location: png_byte) -> png_unknown_chunk {
    let mut n = [0u8; 5];
    n[..4].copy_from_slice(name);
    png_unknown_chunk {
        name: n,
        data,
        size,
        location,
    }
}

// ==========================================================================
// row 221 - png_set_cHRM_XYZ_fixed / png_set_cHRM_fixed
// ==========================================================================

#[test]
fn rows_221_cHRM_XYZ_and_cHRM_fixed() {
    // row 221: all-zero XYZ -> png_xy_from_XYZ fails -> png_app_error
    // ("invalid cHRM XYZ"), which is fatal on a default write struct.
    diff("row221 zero XYZ (writer)", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_cHRM_XYZ_fixed(l, pp, ip, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        valid(l, pp, ip)
    });
    // row 221 on a read struct (png_app_error is also fatal there)
    diff("row221 zero XYZ (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        api::png_set_cHRM_XYZ_fixed(l, pp, ip, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        valid(l, pp, ip)
    });
    // row 221, non-fatal variant: with benign errors allowed the app_error is
    // a warning and the function simply leaves cHRM unset.
    diff("row221 zero XYZ (benign)", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_benign_errors(l, pp, 1);
        api::png_set_cHRM_XYZ_fixed(l, pp, ip, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        valid(l, pp, ip)
    });
    // Degenerate / negative XYZ probes.  png_xy_from_XYZ only fails for some of
    // these (#2 below is the one that reports row 221); the rest must be
    // accepted identically by both libraries.
    for (i, v) in [
        [-1, -1, -1, -1, -1, -1, -1, -1, -1],
        [i32::MIN, 1, 1, 1, 1, 1, 1, 1, 1],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [100000, 0, 0, 0, 100000, 0, 0, 0, 100000],
        [1, 1, 1, 1, 1, 1, 1, 1, 1],
    ]
    .iter()
    .enumerate()
    {
        let v = *v;
        diff(&format!("cHRM_XYZ degenerate probe #{i} (row221 for #2)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            api::png_set_cHRM_XYZ_fixed(
                l, pp, ip, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
            );
            let mut xy = [0i32; 8];
            let got = api::png_get_cHRM_fixed(
                l, pp, ip, &mut xy[0], &mut xy[1], &mut xy[2], &mut xy[3], &mut xy[4],
                &mut xy[5], &mut xy[6], &mut xy[7],
            );
            (valid(l, pp, ip), got, xy)
        });
    }
    // png_set_cHRM_fixed itself validates nothing: degenerate and negative
    // values must be stored (identically) by both libraries.
    for (i, v) in [
        [0i32, 0, 0, 0, 0, 0, 0, 0],
        [-1, -1, -1, -1, -1, -1, -1, -1],
        [i32::MIN, i32::MAX, 0, 1, -1, 2, 3, 4],
        [100000, 100000, 100000, 100000, 100000, 100000, 100000, 100000],
    ]
    .iter()
    .enumerate()
    {
        let v = *v;
        diff(&format!("cHRM_fixed degenerate #{i}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_cHRM_fixed(l, pp, ip, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
            let mut o = [0i32; 8];
            let got = api::png_get_cHRM_fixed(
                l, pp, ip, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                &mut o[6], &mut o[7],
            );
            let mut x = [0i32; 9];
            let gotx = api::png_get_cHRM_XYZ_fixed(
                l, pp, ip, &mut x[0], &mut x[1], &mut x[2], &mut x[3], &mut x[4], &mut x[5],
                &mut x[6], &mut x[7], &mut x[8],
            );
            (valid(l, pp, ip), got, o, gotx, x)
        });
    }
}

// ==========================================================================
// row 222 - png_set_cICP
// ==========================================================================

#[test]
fn rows_222_cICP() {
    // row 222: any non-zero matrix_coefficients -> png_warning and cICP is
    // *not* marked valid (but the four fields have already been stored).
    for mc in [1u8, 2, 99, 255] {
        diff(&format!("row222 cICP mc={mc}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_cICP(l, pp, ip, 1, 13, mc, 1);
            let mut o = [0u8; 4];
            let got = api::png_get_cICP(l, pp, ip, &mut o[0], &mut o[1], &mut o[2], &mut o[3]);
            (valid(l, pp, ip), got, o)
        });
    }
    // control: mc == 0 is accepted; reserved/invalid primaries and transfer
    // functions are *not* rejected here.
    for (cp, tf, vf) in [(0u8, 0u8, 0u8), (255, 255, 255), (2, 2, 99), (1, 13, 1)] {
        diff(&format!("cICP accepted cp={cp} tf={tf} vf={vf}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_cICP(l, pp, ip, cp, tf, 0, vf);
            let mut o = [0u8; 4];
            let got = api::png_get_cICP(l, pp, ip, &mut o[0], &mut o[1], &mut o[2], &mut o[3]);
            (valid(l, pp, ip), got, o)
        });
    }
    // row 222 on a read struct (png_warning either way)
    diff("row222 cICP mc!=0 (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        api::png_set_cICP(l, pp, ip, 1, 13, 7, 0);
        valid(l, pp, ip)
    });
}

// ==========================================================================
// row 223 - png_set_cLLI_fixed
// ==========================================================================

#[test]
fn rows_223_cLLI() {
    // row 223: maxCLL or maxFALL above 0x7fffffff -> png_chunk_report, which
    // is png_app_error (fatal) on a write struct.
    for (a, b) in [
        (0x8000_0000u32, 0u32),
        (0, 0x8000_0000),
        (0xffff_ffff, 0xffff_ffff),
        (0x7fff_ffff, 0x8000_0000),
    ] {
        diff(&format!("row223 cLLI {a:#x},{b:#x} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_cLLI_fixed(l, pp, ip, a, b);
            valid(l, pp, ip)
        });
        // non-fatal variants: benign errors on write, chunk warning on read
        diff(&format!("row223 cLLI {a:#x},{b:#x} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            api::png_set_cLLI_fixed(l, pp, ip, a, b);
            let mut x = 0u32;
            let mut y = 0u32;
            let got = api::png_get_cLLI_fixed(l, pp, ip, &mut x, &mut y);
            (valid(l, pp, ip), got, x, y)
        });
        diff(&format!("row223 cLLI {a:#x},{b:#x} (reader)"), move |l| unsafe {
            let (pp, ip) = rd(l);
            api::png_set_cLLI_fixed(l, pp, ip, a, b);
            valid(l, pp, ip)
        });
    }
    // control: the maximum in-range value is accepted
    diff("cLLI 0x7fffffff accepted", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_cLLI_fixed(l, pp, ip, 0x7fff_ffff, 0x7fff_ffff);
        let mut x = 0u32;
        let mut y = 0u32;
        let got = api::png_get_cLLI_fixed(l, pp, ip, &mut x, &mut y);
        (valid(l, pp, ip), got, x, y)
    });
}

// ==========================================================================
// rows 224, 225 - png_set_mDCV_fixed
// ==========================================================================

#[test]
fn rows_224_225_mDCV() {
    // row 224: any chromaticity whose /2 leaves the uint16 range (i.e. > 131071
    // or negative) -> "mDCV chromaticities outside representable range".
    let bad: [[i32; 8]; 5] = [
        [200000, 20000, 20000, 20000, 20000, 20000, 20000, 20000],
        [20000, -2, 20000, 20000, 20000, 20000, 20000, 20000],
        [i32::MIN, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, i32::MAX],
        [131072, 131072, 131072, 131072, 131072, 131072, 131072, 131072],
    ];
    for (i, v) in bad.iter().enumerate() {
        let v = *v;
        diff(&format!("row224 mDCV chroma #{i} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_mDCV_fixed(l, pp, ip, v[0] as u32, v[1] as u32, v[2] as u32,
                v[3] as u32, v[4] as u32, v[5] as u32, v[6] as u32, v[7] as u32, 1, 1);
            valid(l, pp, ip)
        });
        diff(&format!("row224 mDCV chroma #{i} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            api::png_set_mDCV_fixed(l, pp, ip, v[0] as u32, v[1] as u32, v[2] as u32,
                v[3] as u32, v[4] as u32, v[5] as u32, v[6] as u32, v[7] as u32, 1, 1);
            let mut o = [0u32; 10];
            let got = api::png_get_mDCV_fixed(
                l, pp, ip, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                &mut o[6], &mut o[7], &mut o[8], &mut o[9],
            );
            (valid(l, pp, ip), got, o)
        });
        diff(&format!("row224 mDCV chroma #{i} (reader)"), move |l| unsafe {
            let (pp, ip) = rd(l);
            api::png_set_mDCV_fixed(l, pp, ip, v[0] as u32, v[1] as u32, v[2] as u32,
                v[3] as u32, v[4] as u32, v[5] as u32, v[6] as u32, v[7] as u32, 1, 1);
            valid(l, pp, ip)
        });
    }
    // row 225: valid chromaticities but a display light level > 0x7fffffff.
    for (a, b) in [
        (0x8000_0000u32, 1u32),
        (1, 0x8000_0000),
        (0xffff_ffff, 0xffff_ffff),
    ] {
        diff(&format!("row225 mDCV light {a:#x},{b:#x} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_mDCV_fixed(l, pp, ip, 20000, 20000, 20000, 20000, 20000, 20000, 20000,
                20000, a, b);
            valid(l, pp, ip)
        });
        diff(&format!("row225 mDCV light {a:#x},{b:#x} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            api::png_set_mDCV_fixed(l, pp, ip, 20000, 20000, 20000, 20000, 20000, 20000, 20000,
                20000, a, b);
            let mut o = [0u32; 10];
            let got = api::png_get_mDCV_fixed(
                l, pp, ip, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                &mut o[6], &mut o[7], &mut o[8], &mut o[9],
            );
            (valid(l, pp, ip), got, o)
        });
    }
    // control: in-range values are stored, with the documented /2 scaling
    diff("mDCV accepted", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_mDCV_fixed(l, pp, ip, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000,
            0x7fff_ffff, 0);
        let mut o = [0u32; 10];
        let got = api::png_get_mDCV_fixed(
            l, pp, ip, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
            &mut o[6], &mut o[7], &mut o[8], &mut o[9],
        );
        (valid(l, pp, ip), got, o)
    });
}

// ==========================================================================
// rows 226, 227 - png_set_eXIf / png_set_eXIf_1
// ==========================================================================

#[test]
fn rows_226_227_eXIf() {
    // row 226: png_set_eXIf is unconditionally a warning and does nothing.
    diff("row226 png_set_eXIf (writer)", |l| unsafe {
        let (pp, ip) = wr(l);
        let mut buf = *b"II*\0\x08\0\0\0";
        api::png_set_eXIf(l, pp, ip, buf.as_mut_ptr());
        let mut got: png_bytep = null_mut();
        let n = api::png_get_eXIf_1(l, pp, ip, &mut 0u32, &mut got);
        (valid(l, pp, ip), n, got.is_null())
    });
    diff("row226 png_set_eXIf (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        let mut buf = *b"MM\0*\0\0\0\x08";
        api::png_set_eXIf(l, pp, ip, buf.as_mut_ptr());
        valid(l, pp, ip)
    });

    // row 227: the eXIf data allocation fails -> png_malloc_warn's
    // "Out of memory" warning followed by "Insufficient memory for eXIf
    // chunk data"; nothing is stored.
    diff("row227 eXIf OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let mut buf = *b"II*\0\x08\0\0\0";
        arm(1);
        api::png_set_eXIf_1(l, pp, ip, buf.len() as png_uint_32, buf.as_mut_ptr());
        disarm();
        let mut got: png_bytep = null_mut();
        let mut n = 0u32;
        let r = api::png_get_eXIf_1(l, pp, ip, &mut n, &mut got);
        (valid(l, pp, ip), r, n, got.is_null())
    });
    // ... same on a read struct
    diff("row227 eXIf OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let mut buf = *b"II*\0\x08\0\0\0";
        arm(1);
        api::png_set_eXIf_1(l, pp, ip, 8, buf.as_mut_ptr());
        disarm();
        valid(l, pp, ip)
    });

    // png_set_eXIf_1 does *no* validation of the buffer: a too-short buffer and
    // a bad byte-order marker are accepted verbatim by both libraries.
    for (i, data) in [
        b"I".to_vec(),
        b"II".to_vec(),
        b"XX*\0".to_vec(),
        b"MM*\0".to_vec(),
        vec![],
    ]
    .iter()
    .enumerate()
    {
        let data = data.clone();
        diff(&format!("eXIf_1 unvalidated buffer #{i}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let mut d = data.clone();
            let p = if d.is_empty() {
                1usize as png_bytep // non-NULL, zero length
            } else {
                d.as_mut_ptr()
            };
            api::png_set_eXIf_1(l, pp, ip, d.len() as png_uint_32, p);
            let mut got: png_bytep = null_mut();
            let mut n = 0u32;
            let r = api::png_get_eXIf_1(l, pp, ip, &mut n, &mut got);
            let bytes = if got.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(got, n as usize).to_vec()
            };
            (valid(l, pp, ip), r, n, got.is_null(), bytes)
        });
    }
}

// ==========================================================================
// rows 228, 229 - png_set_hIST
// ==========================================================================

#[test]
fn rows_228_229_hIST() {
    let hist = [1u16; 256];
    // row 228: no PLTE yet -> num_palette == 0 -> "Invalid palette size, hIST
    // allocation skipped".
    diff("row228 hIST without PLTE (writer)", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_hIST(l, pp, ip, hist.as_ptr());
        let mut got: *mut png_uint_16 = null_mut();
        let r = api::png_get_hIST(l, pp, ip, &mut got);
        (valid(l, pp, ip), r, got.is_null())
    });
    diff("row228 hIST without PLTE (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        api::png_set_hIST(l, pp, ip, hist.as_ptr());
        valid(l, pp, ip)
    });
    // row 228 again: an IHDR that declares a palette type is not enough,
    // num_palette is still 0.
    diff("row228 hIST with palette IHDR but no PLTE", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_IHDR(l, pp, ip, 8, 8, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
        api::png_set_hIST(l, pp, ip, hist.as_ptr());
        valid(l, pp, ip)
    });

    // row 229: PLTE present, but the hIST allocation fails.
    diff("row229 hIST OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let pal = [png_color { red: 1, green: 2, blue: 3 }; 4];
        api::png_set_PLTE(l, pp, ip, pal.as_ptr(), 4);
        arm(1);
        api::png_set_hIST(l, pp, ip, hist.as_ptr());
        disarm();
        let mut got: *mut png_uint_16 = null_mut();
        let r = api::png_get_hIST(l, pp, ip, &mut got);
        (valid(l, pp, ip), r, got.is_null())
    });

    // NULL hist is a silent no-op.
    diff("hIST NULL hist", |l| unsafe {
        let (pp, ip) = wr(l);
        let pal = [png_color { red: 1, green: 2, blue: 3 }; 4];
        api::png_set_PLTE(l, pp, ip, pal.as_ptr(), 4);
        api::png_set_hIST(l, pp, ip, std::ptr::null());
        valid(l, pp, ip)
    });
    // control: a successful hIST
    diff("hIST accepted", |l| unsafe {
        let (pp, ip) = wr(l);
        let pal = [png_color { red: 1, green: 2, blue: 3 }; 4];
        api::png_set_PLTE(l, pp, ip, pal.as_ptr(), 4);
        let h = [7u16, 8, 9, 10];
        api::png_set_hIST(l, pp, ip, h.as_ptr());
        let mut got: *mut png_uint_16 = null_mut();
        let r = api::png_get_hIST(l, pp, ip, &mut got);
        let vals = if got.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(got, 4).to_vec()
        };
        (valid(l, pp, ip), r, vals)
    });
}

// ==========================================================================
// rows 230, 231, 232 - png_set_pCAL validation
// ==========================================================================

#[test]
fn rows_230_231_232_pCAL_validation() {
    // row 230: equation type outside 0..3
    for ty in [-1i32, 4, 5, 99, i32::MAX, i32::MIN] {
        diff(&format!("row230 pCAL type={ty} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let purpose = cs("purpose");
            let units = cs("units");
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, ty, 0, units.as_ptr(),
                null_mut());
            valid(l, pp, ip)
        });
        diff(&format!("row230 pCAL type={ty} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let purpose = cs("purpose");
            let units = cs("units");
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, ty, 0, units.as_ptr(),
                null_mut());
            valid(l, pp, ip)
        });
        diff(&format!("row230 pCAL type={ty} (reader)"), move |l| unsafe {
            let (pp, ip) = rd(l);
            let purpose = cs("purpose");
            let units = cs("units");
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, ty, 0, units.as_ptr(),
                null_mut());
            valid(l, pp, ip)
        });
    }

    // row 231: nparams outside 0..255.  (nparams < 0 with a NULL params array
    // still reaches the check because the early return only rejects
    // nparams > 0 && params == NULL.)
    for np in [-1i32, 256, 300, i32::MAX] {
        diff(&format!("row231 pCAL nparams={np} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let purpose = cs("purpose");
            let units = cs("units");
            let p0 = cs("1.0");
            let mut arr: [png_charp; 1] = [p0.as_ptr() as png_charp];
            let params = if np < 0 { null_mut() } else { arr.as_mut_ptr() };
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, 0, np, units.as_ptr(), params);
            valid(l, pp, ip)
        });
        diff(&format!("row231 pCAL nparams={np} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let purpose = cs("purpose");
            let units = cs("units");
            let p0 = cs("1.0");
            let mut arr: [png_charp; 1] = [p0.as_ptr() as png_charp];
            let params = if np < 0 { null_mut() } else { arr.as_mut_ptr() };
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, 0, np, units.as_ptr(), params);
            valid(l, pp, ip)
        });
    }

    // row 232: a parameter that is NULL or not a floating point string
    let bad_params: [&str; 6] = ["", "x", "1e", "-", "1.2.3", "abc"];
    for p in bad_params {
        diff(&format!("row232 pCAL param={p:?} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let purpose = cs("purpose");
            let units = cs("units");
            let s = cs(p);
            let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, 0, 1, units.as_ptr(),
                arr.as_mut_ptr());
            valid(l, pp, ip)
        });
        diff(&format!("row232 pCAL param={p:?} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let purpose = cs("purpose");
            let units = cs("units");
            let s = cs(p);
            let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, 0, 1, units.as_ptr(),
                arr.as_mut_ptr());
            valid(l, pp, ip)
        });
    }
    // row 232 with a NULL entry inside a non-NULL params array
    diff("row232 pCAL params[0]==NULL (writer)", |l| unsafe {
        let (pp, ip) = wr(l);
        let purpose = cs("purpose");
        let units = cs("units");
        let mut arr: [png_charp; 2] = [null_mut(), null_mut()];
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, 0, 1, units.as_ptr(),
            arr.as_mut_ptr());
        valid(l, pp, ip)
    });
    diff("row232 pCAL params[1]==NULL (benign)", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_benign_errors(l, pp, 1);
        let purpose = cs("purpose");
        let units = cs("units");
        let s = cs("1.0");
        let mut arr: [png_charp; 2] = [s.as_ptr() as png_charp, null_mut()];
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, 0, 2, units.as_ptr(),
            arr.as_mut_ptr());
        valid(l, pp, ip)
    });

    // NULL purpose / NULL units / (nparams>0 && NULL params) are silent no-ops.
    diff("pCAL NULL purpose", |l| unsafe {
        let (pp, ip) = wr(l);
        let units = cs("units");
        api::png_set_pCAL(l, pp, ip, std::ptr::null(), 0, 100, 0, 0, units.as_ptr(), null_mut());
        valid(l, pp, ip)
    });
    diff("pCAL NULL units", |l| unsafe {
        let (pp, ip) = wr(l);
        let purpose = cs("purpose");
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, 0, 0, std::ptr::null(), null_mut());
        valid(l, pp, ip)
    });
    diff("pCAL nparams>0 with NULL params", |l| unsafe {
        let (pp, ip) = wr(l);
        let purpose = cs("purpose");
        let units = cs("units");
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 100, 0, 3, units.as_ptr(), null_mut());
        valid(l, pp, ip)
    });
    // An over-long purpose is *not* rejected by png_set_pCAL (the keyword is
    // only checked when the chunk is written).
    diff("pCAL over-long purpose accepted", |l| unsafe {
        let (pp, ip) = wr(l);
        let purpose = cs(&"P".repeat(300));
        let units = cs("units");
        let s = cs("1.0");
        let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), -5, 7, 3, 1, units.as_ptr(),
            arr.as_mut_ptr());
        let mut op: png_charp = null_mut();
        let mut ou: png_charp = null_mut();
        let mut x0 = 0i32;
        let mut x1 = 0i32;
        let mut ty = 0i32;
        let mut np = 0i32;
        let mut ps: *mut png_charp = null_mut();
        let r = api::png_get_pCAL(l, pp, ip, &mut op, &mut x0, &mut x1, &mut ty, &mut np, &mut ou,
            &mut ps);
        let p0 = if ps.is_null() { Vec::new() } else { cstr_vec(*ps) };
        (valid(l, pp, ip), r, cstr_vec(op), cstr_vec(ou), x0, x1, ty, np, p0)
    });
}

// ==========================================================================
// rows 233, 234, 235, 236 - png_set_pCAL out-of-memory paths
// ==========================================================================

#[test]
fn rows_233_234_235_236_pCAL_oom() {
    // The allocation order inside png_set_pCAL is fixed by the C source:
    //   1 = pcal_purpose, 2 = pcal_units, 3 = pcal_params array,
    //   4 = pcal_params[0], 5 = pcal_params[1], ...
    // row 233: purpose allocation fails -> chunk_report (fatal on write)
    diff("row233 pCAL purpose OOM (writer)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let purpose = cs("purpose");
        let units = cs("units");
        let s = cs("1.0");
        let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
        arm(1);
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, 0, 1, units.as_ptr(),
            arr.as_mut_ptr());
        disarm();
        valid(l, pp, ip)
    });
    diff("row233 pCAL purpose OOM (benign)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let purpose = cs("purpose");
        let units = cs("units");
        let s = cs("1.0");
        let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
        arm(1);
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, 0, 1, units.as_ptr(),
            arr.as_mut_ptr());
        disarm();
        valid(l, pp, ip)
    });
    diff("row233 pCAL purpose OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let purpose = cs("purpose");
        let units = cs("units");
        let s = cs("1.0");
        let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
        arm(1);
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, 0, 1, units.as_ptr(),
            arr.as_mut_ptr());
        disarm();
        valid(l, pp, ip)
    });
    // row 234: units allocation fails -> png_warning, pCAL left invalid
    diff("row234 pCAL units OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let purpose = cs("purpose");
        let units = cs("units");
        let s = cs("1.0");
        let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
        arm(2);
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, 0, 1, units.as_ptr(),
            arr.as_mut_ptr());
        disarm();
        valid(l, pp, ip)
    });
    // row 235: params array allocation fails -> png_warning
    diff("row235 pCAL params array OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let purpose = cs("purpose");
        let units = cs("units");
        let s = cs("1.0");
        let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
        arm(3);
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, 0, 1, units.as_ptr(),
            arr.as_mut_ptr());
        disarm();
        valid(l, pp, ip)
    });
    // row 236: params[0] allocation fails -> png_warning
    diff("row236 pCAL params[0] OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let purpose = cs("purpose");
        let units = cs("units");
        let s = cs("1.0");
        let mut arr: [png_charp; 1] = [s.as_ptr() as png_charp];
        arm(4);
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, 0, 1, units.as_ptr(),
            arr.as_mut_ptr());
        disarm();
        valid(l, pp, ip)
    });
    // row 236 for the second parameter
    diff("row236 pCAL params[1] OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let purpose = cs("purpose");
        let units = cs("units");
        let a = cs("1.0");
        let b = cs("2.5e3");
        let mut arr: [png_charp; 2] = [a.as_ptr() as png_charp, b.as_ptr() as png_charp];
        arm(5);
        api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, 0, 2, units.as_ptr(),
            arr.as_mut_ptr());
        disarm();
        valid(l, pp, ip)
    });
}

// ==========================================================================
// rows 237, 238, 239 - png_set_sCAL_s
// ==========================================================================

#[test]
fn rows_237_238_239_sCAL_s() {
    // row 237: unit must be 1 or 2 -> png_error "Invalid sCAL unit"
    for unit in [0i32, 3, 99, -1, i32::MAX, i32::MIN] {
        diff(&format!("row237 sCAL_s unit={unit}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let w = cs("1.0");
            let h = cs("2.0");
            api::png_set_sCAL_s(l, pp, ip, unit, w.as_ptr(), h.as_ptr());
            valid(l, pp, ip)
        });
        // the same through png_set_sCAL / png_set_sCAL_fixed
        diff(&format!("row237 sCAL unit={unit}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_sCAL(l, pp, ip, unit, 1.0, 2.0);
            valid(l, pp, ip)
        });
        diff(&format!("row237 sCAL_fixed unit={unit}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_sCAL_fixed(l, pp, ip, unit, 100000, 200000);
            valid(l, pp, ip)
        });
        diff(&format!("row237 sCAL_s unit={unit} (reader)"), move |l| unsafe {
            let (pp, ip) = rd(l);
            let w = cs("1.0");
            let h = cs("2.0");
            api::png_set_sCAL_s(l, pp, ip, unit, w.as_ptr(), h.as_ptr());
            valid(l, pp, ip)
        });
    }

    // row 238 / row 239: invalid ASCII width / height
    let bad: [&str; 7] = ["", "x", "-", "-1.0", "1.2.3", "+", " 1"];
    for s in bad {
        diff(&format!("row238 sCAL_s width={s:?}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let w = cs(s);
            let h = cs("2.0");
            api::png_set_sCAL_s(l, pp, ip, 1, w.as_ptr(), h.as_ptr());
            valid(l, pp, ip)
        });
        diff(&format!("row239 sCAL_s height={s:?}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let w = cs("2.0");
            let h = cs(s);
            api::png_set_sCAL_s(l, pp, ip, 2, w.as_ptr(), h.as_ptr());
            valid(l, pp, ip)
        });
    }
    // "1e" - an incomplete exponent; whatever the C decides, the Rust must agree
    diff("row238 sCAL_s width=\"1e\"", |l| unsafe {
        let (pp, ip) = wr(l);
        let w = cs("1e");
        let h = cs("2.0");
        api::png_set_sCAL_s(l, pp, ip, 1, w.as_ptr(), h.as_ptr());
        let mut u = 0i32;
        let mut sw: png_charp = null_mut();
        let mut sh: png_charp = null_mut();
        let r = api::png_get_sCAL_s(l, pp, ip, &mut u, &mut sw, &mut sh);
        (valid(l, pp, ip), r, u, cstr_vec(sw), cstr_vec(sh))
    });
    // row 238: NULL width, row 239: NULL height
    diff("row238 sCAL_s NULL width", |l| unsafe {
        let (pp, ip) = wr(l);
        let h = cs("2.0");
        api::png_set_sCAL_s(l, pp, ip, 1, std::ptr::null(), h.as_ptr());
        valid(l, pp, ip)
    });
    diff("row239 sCAL_s NULL height", |l| unsafe {
        let (pp, ip) = wr(l);
        let w = cs("2.0");
        api::png_set_sCAL_s(l, pp, ip, 1, w.as_ptr(), std::ptr::null());
        valid(l, pp, ip)
    });
    // a very long (but valid) numeric string is accepted verbatim
    diff("sCAL_s very long width", |l| unsafe {
        let (pp, ip) = wr(l);
        let long = "1".repeat(200);
        let w = cs(&long);
        let h = cs("2.0");
        api::png_set_sCAL_s(l, pp, ip, 1, w.as_ptr(), h.as_ptr());
        let mut u = 0i32;
        let mut sw: png_charp = null_mut();
        let mut sh: png_charp = null_mut();
        let r = api::png_get_sCAL_s(l, pp, ip, &mut u, &mut sw, &mut sh);
        (valid(l, pp, ip), r, u, cstr_vec(sw), cstr_vec(sh))
    });
}

// ==========================================================================
// rows 240, 241 - png_set_sCAL_s out-of-memory paths
// ==========================================================================

#[test]
fn rows_240_241_sCAL_s_oom() {
    // row 240: the scal_s_width allocation fails
    diff("row240 sCAL_s width OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let w = cs("1.5");
        let h = cs("2.5");
        arm(1);
        api::png_set_sCAL_s(l, pp, ip, 1, w.as_ptr(), h.as_ptr());
        disarm();
        let mut u = 0i32;
        let mut sw: png_charp = null_mut();
        let mut sh: png_charp = null_mut();
        let r = api::png_get_sCAL_s(l, pp, ip, &mut u, &mut sw, &mut sh);
        (valid(l, pp, ip), r, u, sw.is_null(), sh.is_null())
    });
    // row 241: the scal_s_height allocation fails (and the width copy is freed)
    diff("row241 sCAL_s height OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let w = cs("1.5");
        let h = cs("2.5");
        arm(2);
        api::png_set_sCAL_s(l, pp, ip, 2, w.as_ptr(), h.as_ptr());
        disarm();
        let mut u = 0i32;
        let mut sw: png_charp = null_mut();
        let mut sh: png_charp = null_mut();
        let r = api::png_get_sCAL_s(l, pp, ip, &mut u, &mut sw, &mut sh);
        (valid(l, pp, ip), r, u, sw.is_null(), sh.is_null())
    });
    // the same two paths through png_set_sCAL_fixed
    diff("row240 sCAL_fixed width OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        arm(1);
        api::png_set_sCAL_fixed(l, pp, ip, 1, 100000, 200000);
        disarm();
        valid(l, pp, ip)
    });
    diff("row241 sCAL_fixed height OOM", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        arm(2);
        api::png_set_sCAL_fixed(l, pp, ip, 1, 100000, 200000);
        disarm();
        valid(l, pp, ip)
    });
}

// ==========================================================================
// rows 242, 243, 244, 245 - png_set_sCAL / png_set_sCAL_fixed
// ==========================================================================

#[test]
fn rows_242_243_244_245_sCAL_dimensions() {
    // row 242 / row 243 (floating point)
    for (w, h, what) in [
        (0.0f64, 1.0f64, "row242 width==0"),
        (-1.0, 1.0, "row242 width<0"),
        (-1e300, 1.0, "row242 width very negative"),
        (1.0, 0.0, "row243 height==0"),
        (1.0, -1.0, "row243 height<0"),
        (0.0, 0.0, "row242 both zero"),
        (-1.0, -1.0, "row242 both negative"),
    ] {
        diff(&format!("{what} (png_set_sCAL)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_sCAL(l, pp, ip, 1, w, h);
            let mut u = 0i32;
            let mut sw: png_charp = null_mut();
            let mut sh: png_charp = null_mut();
            let r = api::png_get_sCAL_s(l, pp, ip, &mut u, &mut sw, &mut sh);
            (valid(l, pp, ip), r, sw.is_null(), sh.is_null())
        });
    }
    // row 244 / row 245 (fixed point)
    for (w, h, what) in [
        (0i32, 100000i32, "row244 width==0"),
        (-1, 100000, "row244 width<0"),
        (i32::MIN, 100000, "row244 width==INT_MIN"),
        (100000, 0, "row245 height==0"),
        (100000, -1, "row245 height<0"),
        (0, 0, "row244 both zero"),
    ] {
        diff(&format!("{what} (png_set_sCAL_fixed)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_sCAL_fixed(l, pp, ip, 2, w, h);
            let mut u = 0i32;
            let mut sw: png_charp = null_mut();
            let mut sh: png_charp = null_mut();
            let r = api::png_get_sCAL_s(l, pp, ip, &mut u, &mut sw, &mut sh);
            (valid(l, pp, ip), r, sw.is_null(), sh.is_null())
        });
    }
    // controls: valid dimensions round-trip identically
    diff("sCAL accepted", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_sCAL(l, pp, ip, 1, 1.5, 2.5);
        let mut u = 0i32;
        let mut sw: png_charp = null_mut();
        let mut sh: png_charp = null_mut();
        let r = api::png_get_sCAL_s(l, pp, ip, &mut u, &mut sw, &mut sh);
        (valid(l, pp, ip), r, u, cstr_vec(sw), cstr_vec(sh))
    });
    diff("sCAL_fixed accepted", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_sCAL_fixed(l, pp, ip, 2, 1, i32::MAX);
        let mut u = 0i32;
        let mut sw: png_charp = null_mut();
        let mut sh: png_charp = null_mut();
        let r = api::png_get_sCAL_s(l, pp, ip, &mut u, &mut sw, &mut sh);
        (valid(l, pp, ip), r, u, cstr_vec(sw), cstr_vec(sh))
    });
}

// ==========================================================================
// rows 246, 247, 248 - png_set_PLTE
// ==========================================================================

#[test]
fn rows_246_247_248_PLTE() {
    let pal = [png_color { red: 9, green: 8, blue: 7 }; 256];

    // row 246: colour type PALETTE -> an out-of-range length is a png_error.
    // max_palette_length is 1 << bit_depth for palette images.
    for (bd, n) in [
        (1i32, 3i32),
        (1, -1),
        (2, 5),
        (4, 17),
        (8, 257),
        (8, 300),
        (8, i32::MAX),
        (2, -1),
    ] {
        diff(&format!("row246 PLTE palette bd={bd} n={n}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_IHDR(l, pp, ip, 8, 8, bd, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            api::png_set_PLTE(l, pp, ip, pal.as_ptr(), n);
            (valid(l, pp, ip), api::png_get_palette_max(l, pp, ip))
        });
    }
    // control: exactly 1<<bit_depth entries is accepted
    for (bd, n) in [(1i32, 2i32), (2, 4), (4, 16), (8, 256)] {
        diff(&format!("PLTE accepted bd={bd} n={n}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_IHDR(l, pp, ip, 8, 8, bd, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            api::png_set_PLTE(l, pp, ip, pal.as_ptr(), n);
            let mut got: *mut png_color = null_mut();
            let mut num = 0i32;
            let r = api::png_get_PLTE(l, pp, ip, &mut got, &mut num);
            let first = if got.is_null() { png_color::default() } else { *got };
            (valid(l, pp, ip), r, num, first)
        });
    }

    // row 247: a non-palette colour type only warns and returns.
    for (ct, n) in [
        (PNG_COLOR_TYPE_RGB, 257i32),
        (PNG_COLOR_TYPE_RGB, 300),
        (PNG_COLOR_TYPE_GRAY, -1),
        (PNG_COLOR_TYPE_RGB_ALPHA, i32::MAX),
        (PNG_COLOR_TYPE_GRAY_ALPHA, i32::MIN),
    ] {
        diff(&format!("row247 PLTE ct={ct} n={n}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_IHDR(l, pp, ip, 8, 8, 8, ct, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            api::png_set_PLTE(l, pp, ip, pal.as_ptr(), n);
            let mut got: *mut png_color = null_mut();
            let mut num = 0i32;
            let r = api::png_get_PLTE(l, pp, ip, &mut got, &mut num);
            (valid(l, pp, ip), r, num, got.is_null())
        });
    }
    // row 247 on a fresh info struct (colour type 0 by default)
    diff("row247 PLTE no IHDR n=300", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_PLTE(l, pp, ip, pal.as_ptr(), 300);
        valid(l, pp, ip)
    });

    // row 248: "Invalid palette" - num_palette > 0 with a NULL palette, or
    // num_palette == 0 without the MNG empty-PLTE feature.
    diff("row248 PLTE NULL palette n=4", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_PLTE(l, pp, ip, std::ptr::null(), 4);
        valid(l, pp, ip)
    });
    diff("row248 PLTE NULL palette n=256", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_IHDR(l, pp, ip, 8, 8, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
        api::png_set_PLTE(l, pp, ip, std::ptr::null(), 256);
        valid(l, pp, ip)
    });
    diff("row248 PLTE n=0 palette=NULL", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_PLTE(l, pp, ip, std::ptr::null(), 0);
        valid(l, pp, ip)
    });
    diff("row248 PLTE n=0 palette!=NULL", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_PLTE(l, pp, ip, pal.as_ptr(), 0);
        valid(l, pp, ip)
    });
    // ... unless the MNG empty PLTE feature has been permitted
    diff("PLTE n=0 with MNG empty PLTE", |l| unsafe {
        let (pp, ip) = wr(l);
        let f = api::png_permit_mng_features(l, pp, PNG_FLAG_MNG_EMPTY_PLTE as png_uint_32);
        api::png_set_PLTE(l, pp, ip, std::ptr::null(), 0);
        let mut got: *mut png_color = null_mut();
        let mut num = 0i32;
        let r = api::png_get_PLTE(l, pp, ip, &mut got, &mut num);
        (valid(l, pp, ip), f, r, num, got.is_null())
    });
    // row 248 on a read struct
    diff("row248 PLTE NULL palette (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        api::png_set_PLTE(l, pp, ip, std::ptr::null(), 8);
        valid(l, pp, ip)
    });
}

// ==========================================================================
// rows 249, 250, 251 - png_set_iCCP
// ==========================================================================

#[test]
fn rows_249_250_251_iCCP() {
    let profile = [0u8, 0, 0, 8, 1, 2, 3, 4];

    // row 249: compression_type != PNG_COMPRESSION_TYPE_BASE -> png_app_error.
    // On a default write struct that is fatal.
    for ct in [1i32, -1, 99, 0x7fff_ffff] {
        diff(&format!("row249 iCCP ctype={ct} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let name = cs("icc");
            api::png_set_iCCP(l, pp, ip, name.as_ptr(), ct, profile.as_ptr(),
                profile.len() as png_uint_32);
            valid(l, pp, ip)
        });
        // with benign errors the app_error only warns and the chunk *is* stored
        diff(&format!("row249 iCCP ctype={ct} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let name = cs("icc");
            api::png_set_iCCP(l, pp, ip, name.as_ptr(), ct, profile.as_ptr(),
                profile.len() as png_uint_32);
            let mut nm: png_charp = null_mut();
            let mut oct = 0i32;
            let mut prof: png_bytep = null_mut();
            let mut plen = 0u32;
            let r = api::png_get_iCCP(l, pp, ip, &mut nm, &mut oct, &mut prof, &mut plen);
            let bytes = if prof.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(prof, plen as usize).to_vec()
            };
            (valid(l, pp, ip), r, cstr_vec(nm), oct, plen, bytes)
        });
        diff(&format!("row249 iCCP ctype={ct} (reader)"), move |l| unsafe {
            let (pp, ip) = rd(l);
            let name = cs("icc");
            api::png_set_iCCP(l, pp, ip, name.as_ptr(), ct, profile.as_ptr(),
                profile.len() as png_uint_32);
            valid(l, pp, ip)
        });
    }

    // row 250: the name allocation fails -> png_benign_error "Insufficient
    // memory to process iCCP chunk" (fatal on write, a warning on read).
    diff("row250 iCCP name OOM (writer)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let name = cs("icc");
        arm(1);
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, profile.as_ptr(), 8);
        disarm();
        valid(l, pp, ip)
    });
    diff("row250 iCCP name OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let name = cs("icc");
        arm(1);
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, profile.as_ptr(), 8);
        disarm();
        valid(l, pp, ip)
    });
    // row 251: the profile allocation fails
    diff("row251 iCCP profile OOM (writer)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let name = cs("icc");
        arm(2);
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, profile.as_ptr(), 8);
        disarm();
        valid(l, pp, ip)
    });
    diff("row251 iCCP profile OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let name = cs("icc");
        arm(2);
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, profile.as_ptr(), 8);
        disarm();
        valid(l, pp, ip)
    });
    // row 251 with proflen == PNG_UINT_31_MAX: the allocation is forced to
    // fail, which is also the only safe way to pass such a length (libpng
    // would otherwise memcpy 2GB out of the caller's small buffer).
    diff("row251 iCCP proflen=PNG_UINT_31_MAX (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let name = cs("icc");
        arm_min_size(1 << 20);
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, profile.as_ptr(), PNG_UINT_31_MAX);
        disarm();
        valid(l, pp, ip)
    });
    diff("row251 iCCP proflen=PNG_UINT_31_MAX (writer)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let name = cs("icc");
        arm_min_size(1 << 20);
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, profile.as_ptr(), PNG_UINT_31_MAX);
        disarm();
        valid(l, pp, ip)
    });

    // png_set_iCCP validates neither the name nor the profile length here.
    let names: [&str; 4] = ["", "icc", "a b", "with trailing "];
    for n in names {
        diff(&format!("iCCP name={n:?} accepted"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let name = cs(n);
            api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, profile.as_ptr(), 8);
            let mut nm: png_charp = null_mut();
            let mut oct = 0i32;
            let mut prof: png_bytep = null_mut();
            let mut plen = 0u32;
            let r = api::png_get_iCCP(l, pp, ip, &mut nm, &mut oct, &mut prof, &mut plen);
            (valid(l, pp, ip), r, cstr_vec(nm), oct, plen)
        });
    }
    diff("iCCP 80-char name accepted", |l| unsafe {
        let (pp, ip) = wr(l);
        let name = cs(&"n".repeat(80));
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, profile.as_ptr(), 8);
        let mut nm: png_charp = null_mut();
        let mut oct = 0i32;
        let mut prof: png_bytep = null_mut();
        let mut plen = 0u32;
        let r = api::png_get_iCCP(l, pp, ip, &mut nm, &mut oct, &mut prof, &mut plen);
        (valid(l, pp, ip), r, cstr_vec(nm).len(), oct, plen)
    });
    // proflen 0/1/131/132.  NOTE: png_get_iCCP does not report the stored
    // iccp_proflen, it re-reads the length from the first four bytes of the
    // profile, so for proflen < 4 it reads uninitialised heap and the value is
    // meaningless; only png_get_valid is compared for those.
    for plen in [0u32, 1, 2, 3] {
        diff(&format!("iCCP short proflen={plen} accepted"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let name = cs("icc");
            let big = vec![0x5au8; 200];
            api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, big.as_ptr(), plen);
            valid(l, pp, ip)
        });
    }
    for plen in [4u32, 131, 132] {
        diff(&format!("iCCP proflen={plen} accepted"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let name = cs("icc");
            // a profile whose embedded big-endian length agrees with proflen
            let mut big = vec![0x5au8; 200];
            big[0] = (plen >> 24) as u8;
            big[1] = (plen >> 16) as u8;
            big[2] = (plen >> 8) as u8;
            big[3] = plen as u8;
            api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, big.as_ptr(), plen);
            let mut nm: png_charp = null_mut();
            let mut oct = 0i32;
            let mut prof: png_bytep = null_mut();
            let mut got = 0u32;
            let r = api::png_get_iCCP(l, pp, ip, &mut nm, &mut oct, &mut prof, &mut got);
            let n = core::cmp::min(got as usize, plen as usize);
            let bytes = if prof.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(prof, n).to_vec()
            };
            (valid(l, pp, ip), r, cstr_vec(nm), oct, got, bytes)
        });
    }
    diff("iCCP embedded length disagrees with proflen", |l| unsafe {
        let (pp, ip) = wr(l);
        let name = cs("icc");
        // embedded big-endian length says 999, proflen says 8
        let prof = [0u8, 0, 3, 0xe7, 1, 2, 3, 4];
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, prof.as_ptr(), 8);
        let mut nm: png_charp = null_mut();
        let mut oct = 0i32;
        let mut p: png_bytep = null_mut();
        let mut got = 0u32;
        let r = api::png_get_iCCP(l, pp, ip, &mut nm, &mut oct, &mut p, &mut got);
        // png_get_iCCP reports the *embedded* length (999), not the 8 bytes
        // that were stored, so only those 8 bytes may be inspected.
        let bytes = if p.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(p, 8).to_vec()
        };
        (valid(l, pp, ip), r, got, bytes)
    });
    // NULL name / NULL profile are silent no-ops
    diff("iCCP NULL name", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_iCCP(l, pp, ip, std::ptr::null(), 0, profile.as_ptr(), 8);
        valid(l, pp, ip)
    });
    diff("iCCP NULL profile", |l| unsafe {
        let (pp, ip) = wr(l);
        let name = cs("icc");
        api::png_set_iCCP(l, pp, ip, name.as_ptr(), 0, std::ptr::null(), 8);
        valid(l, pp, ip)
    });
}

// ==========================================================================
// rows 252, 253, 254, 256 - png_set_text / png_set_text_2
// ==========================================================================

#[test]
fn rows_252_253_254_256_text() {
    // row 253 + row 252: the text array allocation fails, png_set_text_2
    // reports "too many text chunks" and returns 1, whereupon png_set_text
    // raises "Insufficient memory to store text".
    diff("rows253+252 text array OOM (png_set_text)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let key = cs("Title");
        let text = cs("hello");
        let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
        arm(1);
        api::png_set_text(l, pp, ip, &t, 1);
        disarm();
        text_snapshot(l, pp, ip)
    });
    // row 253 alone, observing the return value of png_set_text_2
    diff("row253 text array OOM (png_set_text_2)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let key = cs("Title");
        let text = cs("hello");
        let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
        arm(1);
        let r = api::png_set_text_2(l, pp, ip, &t, 1);
        disarm();
        (r, text_snapshot(l, pp, ip))
    });
    // row 253 on a write struct without benign errors: fatal
    diff("row253 text array OOM (fatal)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let key = cs("Title");
        let text = cs("hello");
        let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
        arm(1);
        let r = api::png_set_text_2(l, pp, ip, &t, 1);
        disarm();
        (r, text_snapshot(l, pp, ip))
    });
    // row 253 on a read struct: a chunk warning, then return 1
    diff("row253 text array OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let key = cs("Title");
        let text = cs("hello");
        let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
        arm(1);
        let r = api::png_set_text_2(l, pp, ip, &t, 1);
        disarm();
        (r, text_snapshot(l, pp, ip))
    });

    // row 256: the per-entry key allocation fails ("text chunk: out of
    // memory"); allocation #1 is the array, #2 is textp->key.
    diff("row256 text key OOM (png_set_text_2)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let key = cs("Title");
        let text = cs("hello");
        let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
        arm(2);
        let r = api::png_set_text_2(l, pp, ip, &t, 1);
        disarm();
        (r, text_snapshot(l, pp, ip))
    });
    // row 256 + row 252 through png_set_text
    diff("rows256+252 text key OOM (png_set_text)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let key = cs("Title");
        let text = cs("hello");
        let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
        arm(2);
        api::png_set_text(l, pp, ip, &t, 1);
        disarm();
        text_snapshot(l, pp, ip)
    });
    diff("row256 text key OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let key = cs("Title");
        let text = cs("hello");
        let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
        arm(2);
        let r = api::png_set_text_2(l, pp, ip, &t, 1);
        disarm();
        (r, text_snapshot(l, pp, ip))
    });

    // row 254: compression outside [PNG_TEXT_COMPRESSION_NONE,
    // PNG_TEXT_COMPRESSION_LAST)
    for comp in [-3i32, -2, PNG_TEXT_COMPRESSION_LAST, 99, i32::MAX, i32::MIN] {
        diff(&format!("row254 text compression={comp} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let key = cs("Title");
            let text = cs("hello");
            let t = mk_text(comp, &key, &text);
            let r = api::png_set_text_2(l, pp, ip, &t, 1);
            (r, text_snapshot(l, pp, ip))
        });
        diff(&format!("row254 text compression={comp} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let key = cs("Title");
            let text = cs("hello");
            let t = mk_text(comp, &key, &text);
            let r = api::png_set_text_2(l, pp, ip, &t, 1);
            (r, text_snapshot(l, pp, ip))
        });
        diff(&format!("row254 text compression={comp} (reader)"), move |l| unsafe {
            let (pp, ip) = rd(l);
            let key = cs("Title");
            let text = cs("hello");
            let t = mk_text(comp, &key, &text);
            let r = api::png_set_text_2(l, pp, ip, &t, 1);
            (r, text_snapshot(l, pp, ip))
        });
        // ... and through png_set_text, which must not raise row 252 because
        // png_set_text_2 returns 0 after skipping the entry.
        diff(&format!("row254 text compression={comp} (png_set_text/benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let key = cs("Title");
            let text = cs("hello");
            let t = mk_text(comp, &key, &text);
            api::png_set_text(l, pp, ip, &t, 1);
            text_snapshot(l, pp, ip)
        });
    }

    // num_text <= 0, NULL text_ptr and a NULL key are silent no-ops.
    for n in [0i32, -1, i32::MIN] {
        diff(&format!("text num_text={n}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let key = cs("Title");
            let text = cs("hello");
            let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
            let r = api::png_set_text_2(l, pp, ip, &t, n);
            api::png_set_text(l, pp, ip, &t, n);
            (r, text_snapshot(l, pp, ip))
        });
    }
    diff("text NULL text_ptr", |l| unsafe {
        let (pp, ip) = wr(l);
        let r = api::png_set_text_2(l, pp, ip, std::ptr::null(), 1);
        api::png_set_text(l, pp, ip, std::ptr::null(), 1);
        (r, text_snapshot(l, pp, ip))
    });
    diff("text NULL key entry is skipped", |l| unsafe {
        let (pp, ip) = wr(l);
        let text = cs("hello");
        let t = png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: null_mut(),
            text: text.as_ptr() as png_charp,
            text_length: 0,
            itxt_length: 0,
            lang: null_mut(),
            lang_key: null_mut(),
        };
        let r = api::png_set_text_2(l, pp, ip, &t, 1);
        (r, text_snapshot(l, pp, ip))
    });
    // An empty key, an 80+ character key, leading/trailing spaces and control
    // characters are all accepted by png_set_text (they are only sanitised
    // when the chunk is written) - both libraries must store them verbatim.
    let keys: [&str; 6] = [
        "",
        "k",
        " leading",
        "trailing ",
        "ctrl\u{1}char",
        "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
    ];
    for k in keys {
        diff(&format!("text key={k:?} stored verbatim"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let key = cs(k);
            let text = cs("body");
            let t = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &text);
            let r = api::png_set_text_2(l, pp, ip, &t, 1);
            (r, text_snapshot(l, pp, ip))
        });
    }
    // A NULL text with a non-zero text_length: libpng ignores text_length and
    // treats a NULL text as the empty string.
    diff("text NULL text with non-zero length", |l| unsafe {
        let (pp, ip) = wr(l);
        let key = cs("Title");
        let t = png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: key.as_ptr() as png_charp,
            text: null_mut(),
            text_length: 99,
            itxt_length: 99,
            lang: null_mut(),
            lang_key: null_mut(),
        };
        let r = api::png_set_text_2(l, pp, ip, &t, 1);
        (r, text_snapshot(l, pp, ip))
    });
}

// ==========================================================================
// rows 255, 265, 266 - unreachable in this build configuration
// ==========================================================================

#[test]
fn rows_255_265_266_unreachable_by_configuration() {
    // row 255 ("iTXt chunk not supported") is inside `#else` of
    // `#ifdef PNG_iTXt_SUPPORTED`, and pnglibconf.h defines PNG_iTXt_SUPPORTED,
    // so the site is not compiled.  Prove the *taken* branch instead: an iTXt
    // entry is accepted.
    diff("row255 unreachable: iTXt accepted", |l| unsafe {
        let (pp, ip) = wr(l);
        let key = cs("Title");
        let text = cs("body");
        let lang = cs("en");
        let lang_key = cs("Titre");
        let t = png_text {
            compression: PNG_ITXT_COMPRESSION_NONE,
            key: key.as_ptr() as png_charp,
            text: text.as_ptr() as png_charp,
            text_length: 0,
            itxt_length: 0,
            lang: lang.as_ptr() as png_charp,
            lang_key: lang_key.as_ptr() as png_charp,
        };
        let r = api::png_set_text_2(l, pp, ip, &t, 1);
        (r, text_snapshot(l, pp, ip))
    });

    // rows 265 / 266 ("no unknown chunk support on read"/"on write") are inside
    // `#if !defined(PNG_{READ,WRITE}_UNKNOWN_CHUNKS_SUPPORTED)`; both macros are
    // defined, so neither site is compiled.  Prove that png_set_unknown_chunks
    // works on a read struct *and* on a write struct.
    diff("rows265/266 unreachable: unknown chunks work on read", |l| unsafe {
        let (pp, ip) = rd(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        unk_snapshot(l, pp, ip)
    });
    diff("rows265/266 unreachable: unknown chunks work on write", |l| unsafe {
        let (pp, ip) = wr(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_AFTER_IDAT as png_byte);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        unk_snapshot(l, pp, ip)
    });
}

// ==========================================================================
// row 257 - png_set_tIME
// ==========================================================================

#[test]
fn rows_257_tIME() {
    let bad: [(png_uint_16, u8, u8, u8, u8, u8); 12] = [
        (2020, 0, 1, 0, 0, 0),    // month 0
        (2020, 13, 1, 0, 0, 0),   // month 13
        (2020, 255, 1, 0, 0, 0),  // month 255
        (2020, 1, 0, 0, 0, 0),    // day 0
        (2020, 1, 32, 0, 0, 0),   // day 32
        (2020, 1, 255, 0, 0, 0),  // day 255
        (2020, 1, 1, 24, 0, 0),   // hour 24
        (2020, 1, 1, 255, 0, 0),  // hour 255
        (2020, 1, 1, 0, 60, 0),   // minute 60
        (2020, 1, 1, 0, 255, 0),  // minute 255
        (2020, 1, 1, 0, 0, 61),   // second 61
        (2020, 1, 1, 0, 0, 255),  // second 255
    ];
    for (i, t) in bad.iter().enumerate() {
        let t = *t;
        diff(&format!("row257 tIME bad #{i} {t:?}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let tm = png_time {
                year: t.0,
                month: t.1,
                day: t.2,
                hour: t.3,
                minute: t.4,
                second: t.5,
            };
            api::png_set_tIME(l, pp, ip, &tm);
            let mut got: *mut png_time = null_mut();
            let r = api::png_get_tIME(l, pp, ip, &mut got);
            let v = if got.is_null() { png_time::default() } else { *got };
            (valid(l, pp, ip), r, v)
        });
    }
    // row 257 on a read struct
    diff("row257 tIME bad (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        let tm = png_time { year: 0, month: 13, day: 40, hour: 99, minute: 99, second: 99 };
        api::png_set_tIME(l, pp, ip, &tm);
        valid(l, pp, ip)
    });
    // controls: the boundary values that are legal, including the leap second
    for t in [
        (0u16, 1u8, 1u8, 0u8, 0u8, 0u8),
        (65535, 12, 31, 23, 59, 60),
    ] {
        diff(&format!("tIME accepted {t:?}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let tm = png_time {
                year: t.0,
                month: t.1,
                day: t.2,
                hour: t.3,
                minute: t.4,
                second: t.5,
            };
            api::png_set_tIME(l, pp, ip, &tm);
            let mut got: *mut png_time = null_mut();
            let r = api::png_get_tIME(l, pp, ip, &mut got);
            let v = if got.is_null() { png_time::default() } else { *got };
            (valid(l, pp, ip), r, v)
        });
    }
    // NULL mod_time is a silent no-op
    diff("tIME NULL", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_tIME(l, pp, ip, std::ptr::null());
        valid(l, pp, ip)
    });
}

// ==========================================================================
// row 258 - png_set_tRNS
// ==========================================================================

#[test]
fn rows_258_tRNS() {
    // row 258: a trans_color sample above (1 << bit_depth) - 1 for GRAY or RGB
    for (ct, bd, g, r, gr, b) in [
        (PNG_COLOR_TYPE_GRAY, 1i32, 2u16, 0u16, 0u16, 0u16),
        (PNG_COLOR_TYPE_GRAY, 2, 4, 0, 0, 0),
        (PNG_COLOR_TYPE_GRAY, 4, 16, 0, 0, 0),
        (PNG_COLOR_TYPE_GRAY, 8, 256, 0, 0, 0),
        (PNG_COLOR_TYPE_GRAY, 8, 1000, 0, 0, 0),
        (PNG_COLOR_TYPE_RGB, 8, 0, 256, 0, 0),
        (PNG_COLOR_TYPE_RGB, 8, 0, 0, 256, 0),
        (PNG_COLOR_TYPE_RGB, 8, 0, 0, 0, 256),
        (PNG_COLOR_TYPE_RGB, 8, 0, 999, 999, 999),
    ] {
        diff(&format!("row258 tRNS ct={ct} bd={bd} ({r},{gr},{b},{g})"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_IHDR(l, pp, ip, 8, 8, bd, ct, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            let tc = png_color_16 { index: 0, red: r, green: gr, blue: b, gray: g };
            api::png_set_tRNS(l, pp, ip, std::ptr::null(), 0, &tc);
            let mut ta: *mut png_byte = null_mut();
            let mut n = 0i32;
            let mut got: *mut png_color_16 = null_mut();
            let rr = api::png_get_tRNS(l, pp, ip, &mut ta, &mut n, &mut got);
            let v = if got.is_null() { png_color_16::default() } else { *got };
            (valid(l, pp, ip), rr, n, ta.is_null(), v)
        });
    }
    // 16-bit images and colour types with an alpha channel are not checked
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 16i32),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    ] {
        diff(&format!("tRNS unchecked ct={ct} bd={bd}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_IHDR(l, pp, ip, 8, 8, bd, ct, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            let tc = png_color_16 { index: 9, red: 60000, green: 60000, blue: 60000, gray: 60000 };
            api::png_set_tRNS(l, pp, ip, std::ptr::null(), 0, &tc);
            let mut ta: *mut png_byte = null_mut();
            let mut n = 0i32;
            let mut got: *mut png_color_16 = null_mut();
            let rr = api::png_get_tRNS(l, pp, ip, &mut ta, &mut n, &mut got);
            let v = if got.is_null() { png_color_16::default() } else { *got };
            (valid(l, pp, ip), rr, n, ta.is_null(), v)
        });
    }
    // num_trans out of range for the colour type: not rejected, but the
    // trans_alpha array is only copied for 1..=256 entries.
    for n in [-1i32, 0, 1, 256, 257, 1000, i32::MAX] {
        diff(&format!("tRNS num_trans={n}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_IHDR(l, pp, ip, 8, 8, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            let alpha = [0x11u8; PNG_MAX_PALETTE_LENGTH as usize];
            api::png_set_tRNS(l, pp, ip, alpha.as_ptr(), n, std::ptr::null());
            let mut ta: *mut png_byte = null_mut();
            let mut num = 0i32;
            let mut tc: *mut png_color_16 = null_mut();
            let r = api::png_get_tRNS(l, pp, ip, &mut ta, &mut num, &mut tc);
            let first = if ta.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(ta, 4).to_vec()
            };
            (valid(l, pp, ip), r, num, ta.is_null(), tc.is_null(), first)
        });
    }
    // both pointers NULL
    diff("tRNS NULL trans and NULL trans_color", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_tRNS(l, pp, ip, std::ptr::null(), 0, std::ptr::null());
        let mut ta: *mut png_byte = null_mut();
        let mut num = 0i32;
        let mut tc: *mut png_color_16 = null_mut();
        let r = api::png_get_tRNS(l, pp, ip, &mut ta, &mut num, &mut tc);
        (valid(l, pp, ip), r, num, ta.is_null())
    });
    diff("tRNS NULL trans with num_trans=4", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_tRNS(l, pp, ip, std::ptr::null(), 4, std::ptr::null());
        let mut ta: *mut png_byte = null_mut();
        let mut num = 0i32;
        let mut tc: *mut png_color_16 = null_mut();
        let r = api::png_get_tRNS(l, pp, ip, &mut ta, &mut num, &mut tc);
        (valid(l, pp, ip), r, num, ta.is_null())
    });
    // row 258 on a read struct
    diff("row258 tRNS (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        api::png_set_IHDR(l, pp, ip, 8, 8, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
        let tc = png_color_16 { index: 0, red: 0, green: 0, blue: 0, gray: 4000 };
        api::png_set_tRNS(l, pp, ip, std::ptr::null(), 0, &tc);
        valid(l, pp, ip)
    });
}

// ==========================================================================
// rows 259, 260, 261, 262 - png_set_sPLT
// ==========================================================================

#[test]
fn rows_259_260_261_262_sPLT() {
    // row 259: the palette array allocation fails -> "too many sPLT chunks"
    diff("row259 sPLT array OOM (writer)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let name = cs("spal");
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        arm(1);
        api::png_set_sPLT(l, pp, ip, &s, 1);
        disarm();
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    diff("row259 sPLT array OOM (benign)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let name = cs("spal");
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        arm(1);
        api::png_set_sPLT(l, pp, ip, &s, 1);
        disarm();
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    diff("row259 sPLT array OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let name = cs("spal");
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        arm(1);
        api::png_set_sPLT(l, pp, ip, &s, 1);
        disarm();
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });

    // row 260: an entry with a NULL name or NULL entries -> png_app_error
    // "png_set_sPLT: invalid sPLT" (fatal on a default write struct).
    diff("row260 sPLT NULL name (writer)", |l| unsafe {
        let (pp, ip) = wr(l);
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t { name: null_mut(), depth: 8, entries: ents.as_mut_ptr(), nentries: 2 };
        api::png_set_sPLT(l, pp, ip, &s, 1);
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    diff("row260 sPLT NULL entries (writer)", |l| unsafe {
        let (pp, ip) = wr(l);
        let name = cs("spal");
        let s = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: null_mut(),
            nentries: 2,
        };
        api::png_set_sPLT(l, pp, ip, &s, 1);
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    // row 260, non-fatal: the invalid entry is skipped and, because nentries
    // was decremented to 0, no "sPLT out of memory" follows.
    diff("row260 sPLT NULL name (benign)", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_benign_errors(l, pp, 1);
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t { name: null_mut(), depth: 8, entries: ents.as_mut_ptr(), nentries: 2 };
        api::png_set_sPLT(l, pp, ip, &s, 1);
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    diff("row260 sPLT NULL name (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t { name: null_mut(), depth: 8, entries: ents.as_mut_ptr(), nentries: 2 };
        api::png_set_sPLT(l, pp, ip, &s, 1);
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });

    // rows 261 + 262: the per-entry name allocation fails, the loop breaks
    // with nentries still positive and "sPLT out of memory" is reported.
    // Allocation order: 1 = palette array, 2 = np->name, 3 = np->entries.
    diff("rows261+262 sPLT name OOM (benign)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let name = cs("spal");
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        arm(2);
        api::png_set_sPLT(l, pp, ip, &s, 1);
        disarm();
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    diff("rows261+262 sPLT name OOM (writer)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let name = cs("spal");
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        arm(2);
        api::png_set_sPLT(l, pp, ip, &s, 1);
        disarm();
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    diff("rows261+262 sPLT name OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let name = cs("spal");
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        arm(2);
        api::png_set_sPLT(l, pp, ip, &s, 1);
        disarm();
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    // row 262 on its own: the entry-array allocation fails
    diff("row262 sPLT entries OOM (benign)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let name = cs("spal");
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let s = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        arm(3);
        api::png_set_sPLT(l, pp, ip, &s, 1);
        disarm();
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });

    // nentries <= 0 (the outer count) is a silent no-op.
    for n in [0i32, -1, i32::MIN] {
        diff(&format!("sPLT num_spalettes={n}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let name = cs("spal");
            let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
            let s = png_sPLT_t {
                name: name.as_ptr() as png_charp,
                depth: 8,
                entries: ents.as_mut_ptr(),
                nentries: 2,
            };
            api::png_set_sPLT(l, pp, ip, &s, n);
            (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
        });
    }
    // NULL entries array is a silent no-op.
    diff("sPLT NULL array", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_sPLT(l, pp, ip, std::ptr::null(), 1);
        (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
    });
    // A depth outside {8,16} is *not* rejected by png_set_sPLT.
    for depth in [0u8, 1, 4, 7, 8, 16, 17, 255] {
        diff(&format!("sPLT depth={depth} accepted"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let name = cs("spal");
            let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
            let s = png_sPLT_t {
                name: name.as_ptr() as png_charp,
                depth,
                entries: ents.as_mut_ptr(),
                nentries: 2,
            };
            api::png_set_sPLT(l, pp, ip, &s, 1);
            let mut got: *mut png_sPLT_t = null_mut();
            let n = api::png_get_sPLT(l, pp, ip, &mut got);
            let mut v = Vec::new();
            if !got.is_null() {
                for i in 0..n as isize {
                    let e = *got.offset(i);
                    let ents = if e.entries.is_null() {
                        Vec::new()
                    } else {
                        std::slice::from_raw_parts(e.entries, e.nentries as usize).to_vec()
                    };
                    v.push((cstr_vec(e.name), e.depth, e.nentries, ents));
                }
            }
            (valid(l, pp, ip), n, v)
        });
    }
    // An inner nentries of 0 or negative reaches png_malloc_array's
    // "internal error: array alloc" png_error.
    for inner in [0i32, -1] {
        diff(&format!("sPLT inner nentries={inner}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let name = cs("spal");
            let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
            let s = png_sPLT_t {
                name: name.as_ptr() as png_charp,
                depth: 8,
                entries: ents.as_mut_ptr(),
                nentries: inner,
            };
            api::png_set_sPLT(l, pp, ip, &s, 1);
            (valid(l, pp, ip), api::png_get_sPLT(l, pp, ip, &mut null_mut()))
        });
    }
}

// ==========================================================================
// rows 263, 264, 267, 268, 269 - unknown chunk storage
// ==========================================================================

#[test]
fn rows_263_264_267_268_269_unknown_chunks() {
    // rows 263 + 264: location 0 on a *write* struct first draws the app
    // warning "png_set_unknown_chunks now expects a valid location", then -
    // because png_ptr->mode has none of HAVE_IHDR/HAVE_PLTE/AFTER_IDAT set -
    // the png_error "invalid location in png_set_unknown_chunks".
    diff("rows263+264 unknown location=0 (writer)", |l| unsafe {
        let (pp, ip) = wr(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, 0);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        unk_snapshot(l, pp, ip)
    });
    // row 264 alone: on a read struct the app warning is skipped.
    diff("row264 unknown location=0 (reader)", |l| unsafe {
        let (pp, ip) = rd(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, 0);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        unk_snapshot(l, pp, ip)
    });
    // row 264: a location whose only bits are outside the valid mask
    for loc in [4u8, 16, 32, 64, 128, 0xf0] {
        diff(&format!("row264 unknown location={loc} (reader)"), move |l| unsafe {
            let (pp, ip) = rd(l);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, loc);
            api::png_set_unknown_chunks(l, pp, ip, &u, 1);
            unk_snapshot(l, pp, ip)
        });
    }
    // valid locations, including combinations that are reduced to the
    // top-most set bit
    for loc in [1u8, 2, 3, 8, 9, 10, 11, 0xff] {
        diff(&format!("unknown location={loc} accepted"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, loc);
            api::png_set_unknown_chunks(l, pp, ip, &u, 1);
            unk_snapshot(l, pp, ip)
        });
    }

    // row 267: the unknown-chunk array allocation fails
    diff("row267 unknown array OOM (writer)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
        arm(1);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        disarm();
        unk_snapshot(l, pp, ip)
    });
    diff("row267 unknown array OOM (benign)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
        arm(1);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        disarm();
        unk_snapshot(l, pp, ip)
    });
    diff("row267 unknown array OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
        arm(1);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        disarm();
        unk_snapshot(l, pp, ip)
    });
    // row 268: the per-chunk data allocation fails ("unknown chunk: out of
    // memory"); allocation #1 is the array, #2 is the chunk data.
    diff("row268 unknown data OOM (benign)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        api::png_set_benign_errors(l, pp, 1);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
        arm(2);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        disarm();
        unk_snapshot(l, pp, ip)
    });
    diff("row268 unknown data OOM (writer)", |l| unsafe {
        let (pp, ip) = wr_mem(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
        arm(2);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        disarm();
        unk_snapshot(l, pp, ip)
    });
    diff("row268 unknown data OOM (reader)", |l| unsafe {
        let (pp, ip) = rd_mem(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
        arm(2);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        disarm();
        unk_snapshot(l, pp, ip)
    });

    // num_unknowns <= 0, a NULL array and a zero-size chunk
    for n in [0i32, -1, i32::MIN] {
        diff(&format!("unknown num_unknowns={n}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
            api::png_set_unknown_chunks(l, pp, ip, &u, n);
            unk_snapshot(l, pp, ip)
        });
    }
    diff("unknown NULL array", |l| unsafe {
        let (pp, ip) = wr(l);
        api::png_set_unknown_chunks(l, pp, ip, std::ptr::null(), 3);
        unk_snapshot(l, pp, ip)
    });
    diff("unknown zero-size chunk", |l| unsafe {
        let (pp, ip) = wr(l);
        let u = mk_unk(b"uNKa", null_mut(), 0, PNG_HAVE_IHDR as png_byte);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        unk_snapshot(l, pp, ip)
    });
    diff("unknown zero-size chunk with non-NULL data", |l| unsafe {
        let (pp, ip) = wr(l);
        let mut data = [1u8, 2, 3];
        let u = mk_unk(b"uNKa", data.as_mut_ptr(), 0, PNG_AFTER_IDAT as png_byte);
        api::png_set_unknown_chunks(l, pp, ip, &u, 1);
        unk_snapshot(l, pp, ip)
    });

    // row 269: png_set_unknown_chunk_location with no valid bit set is a
    // png_app_error (fatal on a default write struct).
    for loc in [0i32, 4, 16, 0x70] {
        diff(&format!("row269 chunk_location loc={loc} (writer)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
            api::png_set_unknown_chunks(l, pp, ip, &u, 1);
            api::png_set_unknown_chunk_location(l, pp, ip, 0, loc);
            unk_snapshot(l, pp, ip)
        });
        // non-fatal: the pre-1.6 behaviour is faked in (PNG_HAVE_IDAT ->
        // PNG_AFTER_IDAT, everything else -> PNG_HAVE_IHDR).
        diff(&format!("row269 chunk_location loc={loc} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
            api::png_set_unknown_chunks(l, pp, ip, &u, 1);
            api::png_set_unknown_chunk_location(l, pp, ip, 0, loc);
            unk_snapshot(l, pp, ip)
        });
    }
    // an out-of-range chunk index is a silent no-op
    for idx in [-1i32, 1, 99, i32::MAX, i32::MIN] {
        diff(&format!("chunk_location index={idx}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
            api::png_set_unknown_chunks(l, pp, ip, &u, 1);
            api::png_set_unknown_chunk_location(l, pp, ip, idx, 0);
            api::png_set_unknown_chunk_location(l, pp, ip, idx, 99);
            unk_snapshot(l, pp, ip)
        });
    }
    // valid relocations
    for loc in [1i32, 2, 3, 8, 11, 99] {
        diff(&format!("chunk_location loc={loc} accepted"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
            api::png_set_unknown_chunks(l, pp, ip, &u, 1);
            api::png_set_unknown_chunk_location(l, pp, ip, 0, loc);
            unk_snapshot(l, pp, ip)
        });
    }
}

// ==========================================================================
// rows 270, 271, 272 - png_set_keep_unknown_chunks
// ==========================================================================

#[test]
fn rows_270_271_272_keep_unknown_chunks() {
    let list: [u8; 10] = [b'b', b'K', b'G', b'D', 0, b'g', b'A', b'M', b'A', 0];

    // row 270: keep outside [0, PNG_HANDLE_CHUNK_LAST)
    for keep in [-1i32, PNG_HANDLE_CHUNK_LAST, 99, i32::MAX, i32::MIN] {
        diff(&format!("row270 keep={keep} (writer)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_keep_unknown_chunks(l, pp, keep, list.as_ptr(), 2);
            api::png_handle_as_unknown(l, pp, list.as_ptr())
        });
        diff(&format!("row270 keep={keep} (benign)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            api::png_set_keep_unknown_chunks(l, pp, keep, list.as_ptr(), 2);
            api::png_handle_as_unknown(l, pp, list.as_ptr())
        });
        diff(&format!("row270 keep={keep} (reader)"), move |l| unsafe {
            let (pp, _ip) = rd(l);
            api::png_set_keep_unknown_chunks(l, pp, keep, list.as_ptr(), 2);
            api::png_handle_as_unknown(l, pp, list.as_ptr())
        });
    }

    // row 271: num_chunks > 0 with a NULL chunk list
    for n in [1i32, 2, 1000] {
        diff(&format!("row271 NULL list num={n} (writer)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), n);
            api::png_handle_as_unknown(l, pp, list.as_ptr())
        });
        diff(&format!("row271 NULL list num={n} (benign)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            api::png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), n);
            api::png_handle_as_unknown(l, pp, list.as_ptr())
        });
    }
    // num_chunks == 0 with a NULL list only sets the default: no error.
    diff("keep NULL list num=0 sets default", |l| unsafe {
        let (pp, _ip) = wr(l);
        api::png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
        api::png_handle_as_unknown(l, pp, list.as_ptr())
    });
    // num_chunks < 0 ignores the caller's list and uses the built-in one.
    for n in [-1i32, -100, i32::MIN + 1] {
        diff(&format!("keep num={n} uses built-in list"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_NEVER, std::ptr::null(), n);
            let mut out = Vec::new();
            for nm in [
                b"bKGD\0", b"cHRM\0", b"cICP\0", b"cLLI\0", b"eXIf\0", b"gAMA\0", b"hIST\0",
                b"iCCP\0", b"iTXt\0", b"mDCV\0", b"oFFs\0", b"pCAL\0", b"pHYs\0", b"sBIT\0",
                b"sCAL\0", b"sPLT\0", b"sTER\0", b"sRGB\0", b"tEXt\0", b"tIME\0", b"zTXt\0",
                b"IHDR\0", b"PLTE\0", b"tRNS\0", b"IDAT\0", b"IEND\0", b"uNKa\0",
            ] {
                out.push(api::png_handle_as_unknown(l, pp, nm.as_ptr()));
            }
            out
        });
    }

    // row 272: num_chunks + old_num_chunks > UINT_MAX/5.  The list is never
    // dereferenced on this path.
    for n in [858993460i32, 900000000, i32::MAX] {
        diff(&format!("row272 too many chunks num={n} (writer)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, list.as_ptr(), n);
            api::png_handle_as_unknown(l, pp, list.as_ptr())
        });
        diff(&format!("row272 too many chunks num={n} (benign)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            api::png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, list.as_ptr(), n);
            api::png_handle_as_unknown(l, pp, list.as_ptr())
        });
    }

    // A chunk-name list whose entries are shorter than four characters (the
    // API demands 5 bytes per entry; the buffer below is correctly sized, the
    // names themselves are degenerate).
    diff("keep degenerate 5-byte entries", |l| unsafe {
        let (pp, _ip) = wr(l);
        let l2: [u8; 15] = [
            b'a', b'b', 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0,
        ];
        api::png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_IF_SAFE, l2.as_ptr(), 3);
        let mut out = Vec::new();
        for off in [0usize, 5, 10] {
            out.push(api::png_handle_as_unknown(l, pp, l2.as_ptr().add(off)));
        }
        out.push(api::png_handle_as_unknown(l, pp, list.as_ptr()));
        out
    });
    // control: every valid keep value round-trips through
    // png_handle_as_unknown.
    for keep in [
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
    ] {
        diff(&format!("keep={keep} accepted"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_keep_unknown_chunks(l, pp, keep, list.as_ptr(), 2);
            let a = api::png_handle_as_unknown(l, pp, list.as_ptr());
            let b = api::png_handle_as_unknown(l, pp, list.as_ptr().add(5));
            let c = api::png_handle_as_unknown(l, pp, b"uNKa\0".as_ptr());
            (a, b, c)
        });
    }
}

// ==========================================================================
// rows 273, 274, 275, 276 - png_set_compression_buffer_size
// ==========================================================================

#[test]
fn rows_273_276_compression_buffer_size() {
    // row 273: size == 0 or size > PNG_UINT_31_MAX -> png_error
    for size in [
        0usize,
        PNG_UINT_31_MAX as usize + 1,
        usize::MAX,
        usize::MAX / 2,
        0x1_0000_0000,
    ] {
        diff(&format!("row273 buffer size={size} (writer)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_compression_buffer_size(l, pp, size);
            api::png_get_compression_buffer_size(l, pp)
        });
        diff(&format!("row273 buffer size={size} (reader)"), move |l| unsafe {
            let (pp, _ip) = rd(l);
            api::png_set_compression_buffer_size(l, pp, size);
            api::png_get_compression_buffer_size(l, pp)
        });
    }

    // row 276: 0 < size < 6 on a write struct -> warning, size unchanged
    for size in [1usize, 2, 3, 4, 5] {
        diff(&format!("row276 buffer size={size} (writer)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_compression_buffer_size(l, pp, size);
            api::png_get_compression_buffer_size(l, pp)
        });
        // on a read struct the same size is simply the IDAT read size
        diff(&format!("row276 buffer size={size} (reader)"), move |l| unsafe {
            let (pp, _ip) = rd(l);
            api::png_set_compression_buffer_size(l, pp, size);
            api::png_get_compression_buffer_size(l, pp)
        });
    }

    // row 274: the zstream is owned once row output has started
    diff("row274 buffer size while zstream in use", |l| unsafe {
        let (pp, ip) = wr(l);
        sink_reset();
        api::png_set_IHDR(l, pp, ip, 4, 2, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
        api::png_write_info(l, pp, ip);
        let row = [0u8, 1, 2, 3];
        api::png_write_row(l, pp, row.as_ptr());
        api::png_set_compression_buffer_size(l, pp, 4096);
        let got = api::png_get_compression_buffer_size(l, pp);
        let (_data, flushes) = sink_take();
        (got, flushes)
    });
    // ... and the same call before writing starts succeeds
    diff("buffer size before writing starts", |l| unsafe {
        let (pp, _ip) = wr(l);
        api::png_set_compression_buffer_size(l, pp, 4096);
        api::png_get_compression_buffer_size(l, pp)
    });
    // row 275 boundary: the largest accepted size (see
    // row_275_unreachable_on_64bit)
    diff("buffer size=PNG_UINT_31_MAX", |l| unsafe {
        let (pp, _ip) = wr(l);
        api::png_set_compression_buffer_size(l, pp, PNG_UINT_31_MAX as usize);
        api::png_get_compression_buffer_size(l, pp)
    });
    diff("buffer size=6", |l| unsafe {
        let (pp, _ip) = wr(l);
        api::png_set_compression_buffer_size(l, pp, 6);
        api::png_get_compression_buffer_size(l, pp)
    });
}

#[test]
fn row_275_unreachable_on_64bit() {
    // row 275 ("Compression buffer size limited to system maximum") needs
    // size > ZLIB_IO_MAX == (uInt)-1 == 0xffffffff, but any size above
    // PNG_UINT_31_MAX has already been rejected by row 273.  The site is only
    // reachable where size_t is 32 bits and the value wraps.  All we can do on
    // this target is prove that the two libraries agree at the boundaries.
    for size in [0xffff_ffffusize, 0x8000_0000, 0x7fff_ffff, 0x7fff_fffe] {
        diff(&format!("row275 boundary size={size:#x}"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_compression_buffer_size(l, pp, size);
            api::png_get_compression_buffer_size(l, pp)
        });
    }
}

// ==========================================================================
// rows 277, 278 - png_set_benign_errors
// ==========================================================================

#[test]
fn rows_277_278_benign_errors() {
    // NOTE: png_set_benign_errors dereferences png_ptr unconditionally, so a
    // NULL png_ptr is undefined behaviour in the C and is deliberately not
    // exercised here.
    //
    // row 277 is the `allowed != 0` branch (benign errors, app warnings and
    // app errors all become warnings); row 278 is the `else` branch which
    // clears the same three flags so that all three become fatal errors.
    for allowed in [0i32, 1, 2, 99, -1, i32::MIN, i32::MAX] {
        // benign error
        diff(&format!("rows277/278 benign_errors({allowed}) + benign_error"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_benign_errors(l, pp, allowed);
            let msg = cs("probe benign");
            api::png_benign_error(l, pp, msg.as_ptr());
            1u32
        });
        // app warning
        diff(&format!("rows277/278 benign_errors({allowed}) + app_warning"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_benign_errors(l, pp, allowed);
            let msg = cs("probe app warning");
            api::png_app_warning(l, pp, msg.as_ptr());
            2u32
        });
        // app error
        diff(&format!("rows277/278 benign_errors({allowed}) + app_error"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_benign_errors(l, pp, allowed);
            let msg = cs("probe app error");
            api::png_app_error(l, pp, msg.as_ptr());
            3u32
        });
        // the same three on a read struct, whose defaults differ
        diff(&format!("rows277/278 benign_errors({allowed}) reader"), move |l| unsafe {
            let (pp, _ip) = rd(l);
            api::png_set_benign_errors(l, pp, allowed);
            let msg = cs("probe benign");
            api::png_benign_error(l, pp, msg.as_ptr());
            4u32
        });
        // and an actual libpng rejection site routed through the flags
        diff(&format!("rows277/278 benign_errors({allowed}) + real app_error"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, allowed);
            api::png_set_cHRM_XYZ_fixed(l, pp, ip, 0, 0, 0, 0, 0, 0, 0, 0, 0);
            valid(l, pp, ip)
        });
        // toggling back and forth must be idempotent
        diff(&format!("rows277/278 benign_errors({allowed}) then 0"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_benign_errors(l, pp, allowed);
            api::png_set_benign_errors(l, pp, 0);
            let msg = cs("probe benign");
            api::png_benign_error(l, pp, msg.as_ptr());
            5u32
        });
    }
}

// ==========================================================================
// row 279 - png_check_keyword
// ==========================================================================

#[test]
fn rows_279_check_keyword() {
    // row 279: a keyword longer than 79 characters is truncated with the
    // warning "keyword truncated".  png_check_keyword is also reached from the
    // chunk writers, which is exercised below through png_write_tEXt.
    let cases: Vec<String> = vec![
        "".to_string(),
        "k".to_string(),
        "key with spaces".to_string(),
        " leading space".to_string(),
        "trailing space ".to_string(),
        "  double  space  ".to_string(),
        "\u{1}control".to_string(),
        "ctrl\u{1f}inside".to_string(),
        "tab\tchar".to_string(),
        "nl\nchar".to_string(),
        "\u{a0}nbsp".to_string(),
        "a".repeat(79),
        "b".repeat(80),
        "c".repeat(200),
        format!("{} tail", "d".repeat(78)),
        "\u{7f}del".to_string(),
    ];
    for k in cases {
        diff(&format!("row279 png_check_keyword({k:?})"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            let key = cs(&k);
            let mut new_key = [0u8; 80];
            let n = png_check_keyword(l, pp, key.as_ptr(), new_key.as_mut_ptr());
            (n, new_key)
        });
    }
    // a NULL key returns 0 and writes the terminator
    diff("row279 png_check_keyword(NULL)", |l| unsafe {
        let (pp, _ip) = wr(l);
        let mut new_key = [0x5au8; 80];
        let n = png_check_keyword(l, pp, std::ptr::null(), new_key.as_mut_ptr());
        (n, new_key)
    });
    // row 279 through png_write_tEXt (the real caller)
    for k in ["e".repeat(85), "f".repeat(79), "g h ".to_string(), "\u{1}i".to_string()] {
        diff(&format!("row279 via png_write_tEXt({k:?})"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            sink_reset();
            let key = cs(&k);
            let text = cs("body");
            api::png_write_tEXt(l, pp, key.as_ptr(), text.as_ptr(), 4);
            let (data, flushes) = sink_take();
            (data, flushes)
        });
    }
    // a keyword that reduces to nothing makes png_write_tEXt fail
    for k in ["", " ", "   ", "\u{1}", "\u{1}\u{2}"] {
        diff(&format!("png_write_tEXt empty keyword({k:?})"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            sink_reset();
            let key = cs(k);
            let text = cs("body");
            api::png_write_tEXt(l, pp, key.as_ptr(), text.as_ptr(), 4);
            let (data, flushes) = sink_take();
            (data, flushes)
        });
    }
}

// ==========================================================================
// png_set_invalid
// ==========================================================================

#[test]
fn png_set_invalid_out_of_range_mask() {
    for mask in [
        0i32,
        -1,
        0x7fff_ffff,
        i32::MIN,
        0xffff,
        PNG_INFO_cHRM as c_int,
        0x10_0000,
    ] {
        diff(&format!("png_set_invalid mask={mask:#x}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            // populate a few valid flags first
            api::png_set_gAMA_fixed(l, pp, ip, 45455);
            api::png_set_cHRM_fixed(l, pp, ip, 31270, 32900, 64000, 33000, 30000, 60000, 15000,
                6000);
            api::png_set_sRGB(l, pp, ip, 0);
            let sbit = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
            api::png_set_sBIT(l, pp, ip, &sbit);
            let before = valid(l, pp, ip);
            api::png_set_invalid(l, pp, ip, mask);
            (before, valid(l, pp, ip))
        });
    }
}

// ==========================================================================
// NULL png_ptr / NULL info_ptr for every setter above
// ==========================================================================

#[test]
fn null_png_ptr_and_null_info_ptr_are_no_ops() {
    // Every setter is called with (NULL, NULL), (pp, NULL) and (NULL, ip).
    // The C either returns silently or reports the identical message, and the
    // Rust must do exactly the same.  png_set_benign_errors is excluded: it
    // dereferences png_ptr without a NULL check, so a NULL png_ptr is
    // undefined behaviour in the reference implementation.
    diff("NULL boundaries", |l| unsafe {
        let (real_pp, real_ip) = wr(l);
        let nul_p: png_structp = null_mut();
        let nul_i: png_infop = null_mut();

        let pal = [png_color { red: 1, green: 2, blue: 3 }; 4];
        let hist = [1u16; 4];
        let purpose = cs("purpose");
        let units = cs("units");
        let par = cs("1.0");
        let mut params: [png_charp; 1] = [par.as_ptr() as png_charp];
        let sw = cs("1.0");
        let sh = cs("2.0");
        let name = cs("icc");
        let profile = [0u8, 0, 0, 8, 1, 2, 3, 4];
        let key = cs("Title");
        let body = cs("body");
        let txt = mk_text(PNG_TEXT_COMPRESSION_NONE, &key, &body);
        let tm = png_time { year: 2020, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
        let tc = png_color_16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
        let alpha = [0u8; 4];
        let mut ents = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
        let sname = cs("spal");
        let spl = png_sPLT_t {
            name: sname.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        let mut exif = *b"II*\0\x08\0\0\0";
        let mut unk_data = [1u8, 2, 3];
        let unk = mk_unk(b"uNKa", unk_data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
        let chunk_list: [u8; 5] = [b'b', b'K', b'G', b'D', 0];

        for (pp, ip) in [(nul_p, nul_i), (real_pp, nul_i), (nul_p, real_ip)] {
            api::png_set_cHRM_fixed(l, pp, ip, 1, 1, 1, 1, 1, 1, 1, 1);
            api::png_set_cHRM_XYZ_fixed(l, pp, ip, 0, 0, 0, 0, 0, 0, 0, 0, 0);
            api::png_set_cICP(l, pp, ip, 1, 13, 5, 1);
            api::png_set_cLLI_fixed(l, pp, ip, 0xffff_ffff, 0xffff_ffff);
            api::png_set_mDCV_fixed(l, pp, ip, u32::MAX, u32::MAX, u32::MAX, u32::MAX,
                u32::MAX, u32::MAX, u32::MAX, u32::MAX, 0xffff_ffff, 0xffff_ffff);
            api::png_set_eXIf_1(l, pp, ip, 8, exif.as_mut_ptr());
            api::png_set_hIST(l, pp, ip, hist.as_ptr());
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, 99, 1, units.as_ptr(),
                params.as_mut_ptr());
            api::png_set_sCAL_s(l, pp, ip, 99, sw.as_ptr(), sh.as_ptr());
            api::png_set_PLTE(l, pp, ip, pal.as_ptr(), 4);
            api::png_set_PLTE(l, pp, ip, std::ptr::null(), 0);
            api::png_set_iCCP(l, pp, ip, name.as_ptr(), 99, profile.as_ptr(), 8);
            api::png_set_text_2(l, pp, ip, &txt, 1);
            api::png_set_text(l, pp, ip, &txt, 1);
            api::png_set_tIME(l, pp, ip, &tm);
            api::png_set_tRNS(l, pp, ip, alpha.as_ptr(), 4, &tc);
            api::png_set_sPLT(l, pp, ip, &spl, 1);
            api::png_set_unknown_chunks(l, pp, ip, &unk, 1);
            api::png_set_unknown_chunk_location(l, pp, ip, 0, 0);
            api::png_set_invalid(l, pp, ip, -1);
            api::png_set_rows(l, pp, ip, null_mut());
            // png_ptr-only APIs: these have no info_ptr to reject, so they may
            // only be probed with a NULL png_ptr (with a real one they would
            // legitimately raise the errors covered by rows 270 and 273).
            if pp.is_null() {
                api::png_set_keep_unknown_chunks(l, pp, 99, chunk_list.as_ptr(), 1);
                api::png_set_keep_unknown_chunks(l, pp, 0, std::ptr::null(), 4);
                api::png_set_compression_buffer_size(l, pp, 0);
                api::png_set_compression_buffer_size(l, pp, usize::MAX);
                api::png_set_compression_buffer_size(l, pp, 4096);
            }
        }
        // png_set_eXIf always warns, even with a NULL png_ptr (the default
        // warning handler is used when png_ptr is NULL, so nothing reaches our
        // callback).
        api::png_set_eXIf(l, nul_p, nul_i, exif.as_mut_ptr());
        api::png_set_eXIf(l, real_pp, nul_i, exif.as_mut_ptr());
        api::png_set_eXIf(l, nul_p, real_ip, exif.as_mut_ptr());
        // png_set_sCAL / png_set_sCAL_fixed check their arguments before the
        // NULL check, so they warn through the default handler as well.
        api::png_set_sCAL(l, nul_p, nul_i, 1, -1.0, -1.0);
        api::png_set_sCAL_fixed(l, nul_p, nul_i, 1, -1, -1);
        api::png_set_sCAL(l, nul_p, real_ip, 1, 1.0, 1.0);
        api::png_set_sCAL_fixed(l, real_pp, nul_i, 1, 100000, 100000);

        let _ = &mut params;
        (valid(l, real_pp, real_ip), api::png_get_compression_buffer_size(l, real_pp))
    });
}

// ==========================================================================
// out-of-range enum-like ints across the whole pngset.c surface
// ==========================================================================

#[test]
fn out_of_range_enum_ints() {
    let probes: [c_int; 5] = [-1, 0, 3, 99, 0x7fff_ffff];

    // png_set_sCAL_s / sCAL / sCAL_fixed unit
    for v in probes {
        diff(&format!("enum sCAL unit={v}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let w = cs("1.0");
            let h = cs("2.0");
            api::png_set_sCAL_s(l, pp, ip, v, w.as_ptr(), h.as_ptr());
            valid(l, pp, ip)
        });
    }
    // png_set_pCAL equation type
    for v in probes {
        diff(&format!("enum pCAL type={v} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let purpose = cs("purpose");
            let units = cs("units");
            api::png_set_pCAL(l, pp, ip, purpose.as_ptr(), 0, 1, v, 0, units.as_ptr(), null_mut());
            valid(l, pp, ip)
        });
    }
    // png_set_iCCP compression type
    for v in probes {
        diff(&format!("enum iCCP ctype={v} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let name = cs("icc");
            let profile = [0u8, 0, 0, 8, 1, 2, 3, 4];
            api::png_set_iCCP(l, pp, ip, name.as_ptr(), v, profile.as_ptr(), 8);
            valid(l, pp, ip)
        });
    }
    // png_set_text compression mode
    for v in probes {
        diff(&format!("enum text compression={v} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let key = cs("Title");
            let body = cs("body");
            let t = mk_text(v, &key, &body);
            let r = api::png_set_text_2(l, pp, ip, &t, 1);
            (r, text_snapshot(l, pp, ip))
        });
    }
    // png_set_keep_unknown_chunks keep
    for v in probes {
        diff(&format!("enum keep={v} (benign)"), move |l| unsafe {
            let (pp, _ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let list: [u8; 5] = [b'b', b'K', b'G', b'D', 0];
            api::png_set_keep_unknown_chunks(l, pp, v, list.as_ptr(), 1);
            api::png_handle_as_unknown(l, pp, list.as_ptr())
        });
    }
    // png_set_unknown_chunk_location location
    for v in probes {
        diff(&format!("enum chunk location={v} (benign)"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, PNG_HAVE_IHDR as png_byte);
            api::png_set_unknown_chunks(l, pp, ip, &u, 1);
            api::png_set_unknown_chunk_location(l, pp, ip, 0, v);
            unk_snapshot(l, pp, ip)
        });
    }
    // png_set_unknown_chunks location byte (0..255, every value)
    diff("enum unknown chunk location byte sweep (benign)", |l| unsafe {
        let mut out = Vec::new();
        for loc in 0u8..=255 {
            let (pp, ip) = wr(l);
            api::png_set_benign_errors(l, pp, 1);
            let mut data = [1u8, 2, 3];
            let u = mk_unk(b"uNKa", data.as_mut_ptr(), 3, loc);
            let r = capture(|| {
                api::png_set_unknown_chunks(l, pp, ip, &u, 1);
                unk_snapshot(l, pp, ip)
            });
            out.push((
                loc,
                r.out,
                r.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
            ));
        }
        out
    });
    // png_set_invalid mask (already covered) plus png_set_tRNS num_trans
    for v in probes {
        diff(&format!("enum tRNS num_trans={v}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            api::png_set_IHDR(l, pp, ip, 8, 8, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            let alpha = [7u8; PNG_MAX_PALETTE_LENGTH as usize];
            api::png_set_tRNS(l, pp, ip, alpha.as_ptr(), v, std::ptr::null());
            let mut ta: *mut png_byte = null_mut();
            let mut num = 0i32;
            let mut tcp: *mut png_color_16 = null_mut();
            let r = api::png_get_tRNS(l, pp, ip, &mut ta, &mut num, &mut tcp);
            (valid(l, pp, ip), r, num, ta.is_null())
        });
    }
    // png_set_PLTE num_palette
    for v in probes {
        diff(&format!("enum PLTE num_palette={v}"), move |l| unsafe {
            let (pp, ip) = wr(l);
            let pal = [png_color { red: 1, green: 2, blue: 3 }; 256];
            api::png_set_PLTE(l, pp, ip, pal.as_ptr(), v);
            (valid(l, pp, ip), api::png_get_palette_max(l, pp, ip))
        });
    }
}


//! Differential tests for CONFIGS.md rows 52, 53, 57, 66, 67.
//!
//!  * 52 - custom allocator (`png_create_*_struct_2` + `png_set_mem_fn`) and
//!         the whole `png_malloc`/`png_calloc`/`png_free` family.
//!  * 53 - `png_info` lifecycle: create/init_3/destroy, `png_free_data` for
//!         every `PNG_FREE_*` mask, `png_data_freer`.
//!  * 57 - `png_icc_check_header` / `_length` / `_tag_table` and
//!         `png_resolve_file_gamma`.
//!  * 66 - `png_reset_crc` / `png_calculate_crc` (observed through the CRC
//!         bytes of a written chunk) and the zstream helpers.
//!  * 67 - `png_read_chunk_header` + `png_handle_chunk` / `png_handle_unknown`
//!         driven directly over hand-built chunk streams.
//!
//! Everything goes through `dlsym` on both shared objects; the Rust crate is
//! never linked against directly.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod common;

use common::api::*;
use common::*;
use libloading::Library;
use std::cell::RefCell;
use std::ffi::{c_int, c_uint, c_void, CString};
use std::ptr;

// ---------------------------------------------------------------------------
// Entry points that `tests/common/api.rs` does not declare (or declares with a
// signature that does not match this libpng version).  Declared locally, as
// the task requires, instead of editing the shared harness.
//
// NOTE: `tests/common/api.rs` declares `png_icc_check_*` with an extra
// `colorspace` argument.  In *this* libpng (1.6.59) there is no
// `png_colorspace` type at all - `grep png_colorspace c_src/include/*.h`
// finds nothing and the real prototypes in `c_src/include/pngpriv.h` are
//
//     int png_icc_check_length  (png_const_structrp, png_const_charp,
//                                png_uint_32);
//     int png_icc_check_header  (png_const_structrp, png_const_charp,
//                                png_uint_32, png_const_bytep, int);
//     int png_icc_check_tag_table(png_const_structrp, png_const_charp,
//                                png_uint_32, png_const_bytep);
//
// so the harness declaration would corrupt the argument list.  The correct
// declarations are below and are the ones used by the row-57 tests.
// ---------------------------------------------------------------------------
mod x {
    use super::*;

    crate::decl_api! {
        fn png_read_chunk_header(pp: png_structp) -> png_uint_32;
        fn png_handle_chunk(pp: png_structp, ip: png_infop, length: png_uint_32) -> c_int;
        fn png_handle_unknown(pp: png_structp, ip: png_infop, length: png_uint_32,
                              keep: c_int) -> c_int;
        fn png_resolve_file_gamma(pp: png_structp) -> png_fixed_point;
        fn png_icc_check_length(pp: png_structp, name: png_const_charp,
                                proflen: png_uint_32) -> c_int;
        fn png_icc_check_header(pp: png_structp, name: png_const_charp,
                                proflen: png_uint_32, profile: png_const_bytep,
                                color_type: c_int) -> c_int;
        fn png_icc_check_tag_table(pp: png_structp, name: png_const_charp,
                                   proflen: png_uint_32, profile: png_const_bytep) -> c_int;
    }
}

// ---------------------------------------------------------------------------
// Constants that the shared harness either lacks or (for PNG_FREE_*) defines
// with values that do not match `c_src/include/png.h`.  The authoritative
// values, from png.h lines 1838-1853, are used here.
// ---------------------------------------------------------------------------

const FREE_HIST: png_uint_32 = 0x0008;
const FREE_ICCP: png_uint_32 = 0x0010;
const FREE_SPLT: png_uint_32 = 0x0020;
const FREE_ROWS: png_uint_32 = 0x0040;
const FREE_PCAL: png_uint_32 = 0x0080;
const FREE_SCAL: png_uint_32 = 0x0100;
const FREE_UNKN: png_uint_32 = 0x0200;
const FREE_PLTE: png_uint_32 = 0x1000;
const FREE_TRNS: png_uint_32 = 0x2000;
const FREE_TEXT: png_uint_32 = 0x4000;
const FREE_EXIF: png_uint_32 = 0x8000;
const FREE_ALL: png_uint_32 = 0xffff;

const ALL_FREE_MASKS: &[(&str, png_uint_32)] = &[
    ("HIST", FREE_HIST),
    ("ICCP", FREE_ICCP),
    ("SPLT", FREE_SPLT),
    ("ROWS", FREE_ROWS),
    ("PCAL", FREE_PCAL),
    ("SCAL", FREE_SCAL),
    ("UNKN", FREE_UNKN),
    ("PLTE", FREE_PLTE),
    ("TRNS", FREE_TRNS),
    ("TEXT", FREE_TEXT),
    ("EXIF", FREE_EXIF),
    ("ALL", FREE_ALL),
    ("NONE", 0),
    ("0x0400(removed PNG_FREE_LIST)", 0x0400),
];

/// Every `PNG_INFO_*` bit, so `png_get_valid` can be sampled exhaustively.
const ALL_INFO_BITS: &[(&str, png_uint_32)] = &[
    ("gAMA", PNG_INFO_gAMA),
    ("sBIT", PNG_INFO_sBIT),
    ("cHRM", PNG_INFO_cHRM),
    ("PLTE", PNG_INFO_PLTE),
    ("tRNS", PNG_INFO_tRNS),
    ("bKGD", PNG_INFO_bKGD),
    ("hIST", PNG_INFO_hIST),
    ("pHYs", PNG_INFO_pHYs),
    ("oFFs", PNG_INFO_oFFs),
    ("tIME", PNG_INFO_tIME),
    ("pCAL", PNG_INFO_pCAL),
    ("sRGB", PNG_INFO_sRGB),
    ("iCCP", PNG_INFO_iCCP),
    ("sPLT", PNG_INFO_sPLT),
    ("sCAL", PNG_INFO_sCAL),
    ("IDAT", PNG_INFO_IDAT),
    ("eXIf", PNG_INFO_eXIf),
    ("cICP", PNG_INFO_cICP),
    ("cLLI", PNG_INFO_cLLI),
    ("mDCV", PNG_INFO_mDCV),
];

/// `sizeof(png_info)` for the reference C build (measured by compiling a
/// one-line program against `c_src/include/pngpriv.h`: 352 on x86-64 Linux).
const SIZEOF_PNG_INFO_C: usize = 352;

/// `png_handle_result_code` (pngpriv.h).
const HANDLED_ERROR: c_int = 0;
const HANDLED_DISCARDED: c_int = 1;
const HANDLED_SAVED: c_int = 2;
const HANDLED_OK: c_int = 3;

fn handled_name(v: c_int) -> &'static str {
    match v {
        HANDLED_ERROR => "handled_error",
        HANDLED_DISCARDED => "handled_discarded",
        HANDLED_SAVED => "handled_saved",
        HANDLED_OK => "handled_ok",
        _ => "??",
    }
}

fn ver() -> png_const_charp {
    PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp
}

// ===========================================================================
// Row 52 - custom allocator
// ===========================================================================

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// The reference `c_src/build/libpng.so` is compiled with just
// `-std=gnu99 -fPIC` (see c_src/build/CMakeFiles/png.dir/flags.make), i.e.
// without `-lm`, so `floor`/`frexp` are left undefined in it and have to be
// supplied by the process that dlopen()s it.  Referencing them here makes
// rustc link the test binary against libm.
#[link(name = "m")]
extern "C" {
    fn floor(x: f64) -> f64;
    fn frexp(x: f64, e: *mut c_int) -> f64;
}

#[test]
fn t_mem_libm_is_linked() {
    let mut e: c_int = 0;
    unsafe {
        assert_eq!(floor(2.75), 2.0);
        assert_eq!(frexp(1.0, &mut e), 0.5);
    }
    assert_eq!(e, 1);
}

/// One recorded allocator event.  Pointer *values* are never recorded (they
/// always differ between the two libraries); only the requested size and the
/// position in the sequence are.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Ev {
    M(usize),
    F,
}

thread_local! {
    static ALLOCS: RefCell<Vec<Ev>> = const { RefCell::new(Vec::new()) };
}

fn allocs_reset() {
    ALLOCS.with(|a| a.borrow_mut().clear());
}
fn allocs_take() -> Vec<Ev> {
    ALLOCS.with(|a| std::mem::take(&mut *a.borrow_mut()))
}
/// The ordered list of *requested sizes* - the primary structural signature.
fn sizes(ev: &[Ev]) -> Vec<usize> {
    ev.iter()
        .filter_map(|e| match e {
            Ev::M(n) => Some(*n),
            Ev::F => None,
        })
        .collect()
}

unsafe extern "C-unwind" fn rec_malloc(_pp: png_structp, size: usize) -> png_voidp {
    ALLOCS.with(|a| a.borrow_mut().push(Ev::M(size)));
    malloc(size)
}

unsafe extern "C-unwind" fn rec_free(_pp: png_structp, p: png_voidp) {
    ALLOCS.with(|a| a.borrow_mut().push(Ev::F));
    free(p);
}

/// Sentinel handed to `png_set_mem_fn` / `png_create_*_struct_2`; not a heap
/// pointer, so its value can safely be compared between the libraries.
const MEM_SENTINEL: usize = 0xBEEF;

unsafe fn writer_2(l: &Library) -> png_structp {
    let pp = png_create_write_struct_2(
        l,
        ver(),
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
        MEM_SENTINEL as png_voidp,
        Some(rec_malloc),
        Some(rec_free),
    );
    assert!(!pp.is_null(), "png_create_write_struct_2 returned NULL");
    png_set_write_fn(l, pp, 1usize as png_voidp, Some(write_cb), Some(flush_cb));
    pp
}

unsafe fn reader_2(l: &Library) -> png_structp {
    let pp = png_create_read_struct_2(
        l,
        ver(),
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
        MEM_SENTINEL as png_voidp,
        Some(rec_malloc),
        Some(rec_free),
    );
    assert!(!pp.is_null(), "png_create_read_struct_2 returned NULL");
    png_set_read_fn(l, pp, 1usize as png_voidp, Some(read_cb));
    pp
}

// KNOWN DIVERGENCE (both `cfg52_custom_allocator_*` tests fail on it):
//   the size the two libraries ask the user malloc_fn for when allocating the
//   png_struct differs - C 1232, Rust 1224.  Root cause: `pngstruct.h` declares
//
//       #if defined(PNG_READ_EXPAND_SUPPORTED) && \
//           (defined(PNG_ARM_NEON_IMPLEMENTATION) || \
//            defined(PNG_RISCV_RVV_IMPLEMENTATION))
//          png_bytep riffled_palette;
//       #endif
//
//   and `pngpriv.h` *always* `#define`s both of those macros (to 0 on x86-64,
//   see pngpriv.h:170 / :312), so `#ifdef` is satisfied and the C png_struct
//   really does contain `riffled_palette` at offset 1040.  The translated
//   `translation/src/pngstruct.rs::png_struct` has no such field, which makes
//   it 8 bytes shorter.  Everything else in the allocation sequence matches
//   element for element.

/// A complete write: IHDR + gAMA + tEXt + zTXt + rows + IEND.
unsafe fn drive_write(l: &Library) -> Vec<u8> {
    const W: png_uint_32 = 9;
    const H: png_uint_32 = 7;

    sink_reset();
    let pp = writer_2(l);
    let ip = png_create_info_struct(l, pp);
    assert!(!ip.is_null());

    png_set_IHDR(
        l,
        pp,
        ip,
        W,
        H,
        8,
        PNG_COLOR_TYPE_RGB_ALPHA,
        PNG_INTERLACE_NONE,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );
    png_set_gAMA_fixed(l, pp, ip, 45455);

    let k0 = cs("Title");
    let t0 = cs("a plain text chunk");
    let k1 = cs("Comment");
    let t1 = cs("a compressed text chunk with enough bytes to matter");
    let texts = [
        png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: k0.as_ptr() as png_charp,
            text: t0.as_ptr() as png_charp,
            text_length: 0,
            itxt_length: 0,
            lang: ptr::null_mut(),
            lang_key: ptr::null_mut(),
        },
        png_text {
            compression: PNG_TEXT_COMPRESSION_zTXt,
            key: k1.as_ptr() as png_charp,
            text: t1.as_ptr() as png_charp,
            text_length: 0,
            itxt_length: 0,
            lang: ptr::null_mut(),
            lang_key: ptr::null_mut(),
        },
    ];
    png_set_text(l, pp, ip, texts.as_ptr(), 2);

    png_write_info(l, pp, ip);

    let mut rng = Rng::new(0x5eed_52);
    let mut rows: Vec<Vec<u8>> = (0..H).map(|_| rng.bytes((W * 4) as usize)).collect();
    let mut rp: Vec<png_bytep> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    png_write_image(l, pp, rp.as_mut_ptr());
    png_write_end(l, pp, ip);

    let mut p = pp;
    let mut i = ip;
    png_destroy_write_struct(l, &mut p, &mut i);
    assert!(p.is_null() && i.is_null(), "destroy_write_struct did not NULL out");

    sink_take().0
}

/// A complete read of `data`.
unsafe fn drive_read(l: &Library, data: &[u8]) -> (png_uint_32, png_uint_32, usize) {
    src_set(data);
    let pp = reader_2(l);
    let ip = png_create_info_struct(l, pp);
    assert!(!ip.is_null());

    png_read_info(l, pp, ip);
    let w = png_get_image_width(l, pp, ip);
    let h = png_get_image_height(l, pp, ip);
    let rb = png_get_rowbytes(l, pp, ip);

    let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb]).collect();
    let mut rp: Vec<png_bytep> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    png_read_image(l, pp, rp.as_mut_ptr());
    png_read_end(l, pp, ip);

    let mut p = pp;
    let mut i = ip;
    png_destroy_read_struct(l, &mut p, &mut i, ptr::null_mut());
    (w, h, rb)
}

#[test]
fn cfg52_custom_allocator_write_sequence() {
    let l = libs();

    let mut out = Vec::new();
    let mut seq = Vec::new();
    for lib in [&l.c, &l.rs] {
        allocs_reset();
        let r = capture(|| unsafe { drive_write(lib) });
        assert!(r.out.is_some(), "write failed: {:?}", r.log);
        assert!(r.log.is_empty(), "unexpected messages during write: {:?}", r.log);
        out.push(r.out.unwrap());
        seq.push(allocs_take());
    }

    assert_eq!(hex(&out[0]), hex(&out[1]), "written PNG differs");
    // The very first allocation is `sizeof(png_struct)`; compare the rest
    // first so that a png_struct size difference does not hide a difference in
    // the actual allocation pattern.
    assert_eq!(
        sizes(&seq[0])[1..],
        sizes(&seq[1])[1..],
        "write: allocation SIZE sequence after the png_struct differs (C first)"
    );
    assert_eq!(
        seq[0][1..],
        seq[1][1..],
        "write: malloc/free event sequence after the png_struct differs (C first)"
    );
    assert_eq!(
        sizes(&seq[0])[0],
        sizes(&seq[1])[0],
        "write: sizeof(png_struct) handed to the user malloc_fn differs (C first)"
    );
}

#[test]
fn cfg52_custom_allocator_read_sequence() {
    let l = libs();

    // Produce the stream with the C library and feed the *same* bytes to both
    // readers so the allocation sequences are directly comparable.
    allocs_reset();
    let png = capture(|| unsafe { drive_write(&l.c) }).out.expect("C write failed");
    allocs_reset();

    let mut dims = Vec::new();
    let mut seq = Vec::new();
    for lib in [&l.c, &l.rs] {
        allocs_reset();
        let r = capture(|| unsafe { drive_read(lib, &png) });
        assert!(r.out.is_some(), "read failed: {:?}", r.log);
        assert!(r.log.is_empty(), "unexpected messages during read: {:?}", r.log);
        dims.push(r.out.unwrap());
        seq.push(allocs_take());
    }

    assert_eq!(dims[0], dims[1], "read: (width, height, rowbytes) differ");
    assert_eq!(
        sizes(&seq[0])[1..],
        sizes(&seq[1])[1..],
        "read: allocation SIZE sequence after the png_struct differs (C first)"
    );
    assert_eq!(
        seq[0][1..],
        seq[1][1..],
        "read: malloc/free event sequence after the png_struct differs (C first)"
    );
    assert_eq!(
        sizes(&seq[0])[0],
        sizes(&seq[1])[0],
        "read: sizeof(png_struct) handed to the user malloc_fn differs (C first)"
    );
}

/// `png_malloc`, `png_malloc_warn`, `png_malloc_default`, `png_calloc`,
/// `png_free`, `png_free_default` for the requested size grid.  Only the
/// null-ness of the result, the recorded allocator traffic and the message
/// log are compared - never the pointer value.
#[test]
fn cfg52_malloc_family() {
    let l = libs();
    // usize::MAX is added on top of the requested grid: it is the only size
    // that reliably makes the underlying malloc fail and therefore the only
    // way to reach the "Out of memory" error/warning paths.
    let sizes_grid: [usize; 6] = [0, 1, 8, 4096, 1 << 20, usize::MAX];

    for &size in &sizes_grid {
        for which in 0..6usize {
            for user_mem in [false, true] {
                let run = |lib: &'static Library| {
                    capture(|| unsafe {
                        let pp = if user_mem {
                            writer_2(lib)
                        } else {
                            api::new_writer(lib)
                        };
                        // Reset *after* creation so this test compares the
                        // malloc-family traffic only; the sizeof(png_struct)
                        // allocation is covered by the two sequence tests.
                        allocs_reset();
                        let p: png_voidp = match which {
                            0 => png_malloc(lib, pp, size),
                            1 => png_malloc_warn(lib, pp, size),
                            2 => png_malloc_default(lib, pp, size),
                            3 => png_calloc(lib, pp, size),
                            4 => {
                                // png_free of a png_malloc_warn'ed block
                                let q = png_malloc_warn(lib, pp, size);
                                png_free(lib, pp, q);
                                png_free(lib, pp, ptr::null_mut());
                                q
                            }
                            _ => {
                                // png_free_default always uses the system free,
                                // so pair it with png_malloc_default which
                                // always uses the system malloc.
                                let q = png_malloc_default(lib, pp, size);
                                png_free_default(lib, pp, q);
                                png_free_default(lib, pp, ptr::null_mut());
                                q
                            }
                        };
                        // For 0..3 the block is still live: hand it back.
                        if which < 4 {
                            if which == 2 {
                                png_free_default(lib, pp, p);
                            } else {
                                png_free(lib, pp, p);
                            }
                        }
                        let null = p.is_null();
                        let ev = allocs_take();
                        let mut pq = pp;
                        png_destroy_write_struct(lib, &mut pq, ptr::null_mut());
                        (null, ev)
                    })
                };
                let a = run(&l.c);
                let b = run(&l.rs);
                let what = format!("size={size} fn#{which} user_mem={user_mem}");
                a.assert_eq(&b, &what);
            }
        }
    }
}

#[test]
fn cfg52_get_mem_ptr() {
    let l = libs();
    let run = |lib: &'static Library| {
        capture(|| unsafe {
            // NULL png_ptr
            let n0 = png_get_mem_ptr(lib, ptr::null_mut()).is_null();
            // created with png_create_write_struct (no user mem)
            let pw = api::new_writer(lib);
            let a = png_get_mem_ptr(lib, pw) as usize;
            // created with png_create_write_struct_2
            let p2 = writer_2(lib);
            let b = png_get_mem_ptr(lib, p2) as usize;
            // re-set through png_set_mem_fn
            png_set_mem_fn(lib, p2, 0x1234 as png_voidp, Some(rec_malloc), Some(rec_free));
            let c = png_get_mem_ptr(lib, p2) as usize;
            png_set_mem_fn(lib, p2, ptr::null_mut(), None, None);
            let d = png_get_mem_ptr(lib, p2) as usize;
            // with malloc_fn == NULL the library falls back to the system
            // allocator; make sure that still works.
            let q = png_malloc(lib, p2, 64);
            let e = q.is_null();
            png_free(lib, p2, q);
            // png_set_mem_fn on a NULL png_ptr must be a no-op.
            png_set_mem_fn(lib, ptr::null_mut(), 1 as png_voidp, None, None);

            let mut x = pw;
            png_destroy_write_struct(lib, &mut x, ptr::null_mut());
            let mut y = p2;
            png_destroy_write_struct(lib, &mut y, ptr::null_mut());
            (n0, a, b, c, d, e)
        })
    };
    let a = run(&l.c);
    let b = run(&l.rs);
    a.assert_eq(&b, "png_get_mem_ptr");
    assert_eq!(
        a.out.as_ref().unwrap().1,
        0,
        "png_create_write_struct must leave mem_ptr NULL"
    );
    assert_eq!(
        a.out.as_ref().unwrap().2,
        MEM_SENTINEL,
        "png_create_write_struct_2 must record mem_ptr"
    );
}

// ===========================================================================
// Row 53 - png_info lifecycle
// ===========================================================================

/// Everything observable about a png_info through the public getters, except
/// heap pointer values.
#[derive(Debug, PartialEq)]
struct InfoState {
    valid: Vec<(&'static str, png_uint_32)>,
    width: png_uint_32,
    height: png_uint_32,
    n_text: (c_int, c_int),
    n_splt: c_int,
    n_unknown: c_int,
    rows_null: bool,
    plte: (png_uint_32, c_int),
    hist: png_uint_32,
    iccp: (png_uint_32, png_uint_32),
    pcal: (png_uint_32, png_int_32, png_int_32, c_int, c_int),
    scal: (png_uint_32, c_int),
    trns: (png_uint_32, c_int),
    exif: (png_uint_32, png_uint_32),
    bkgd: png_uint_32,
    sbit: png_uint_32,
    tIME: png_uint_32,
}

unsafe fn info_state(l: &Library, pp: png_structp, ip: png_infop) -> InfoState {
    let valid = ALL_INFO_BITS
        .iter()
        .map(|&(n, b)| (n, png_get_valid(l, pp, ip, b)))
        .collect();

    let mut tp: *mut png_text = ptr::null_mut();
    let mut tn: c_int = -99;
    let tret = png_get_text(l, pp, ip, &mut tp, &mut tn);

    let mut sp: *mut png_sPLT_t = ptr::null_mut();
    let n_splt = png_get_sPLT(l, pp, ip, &mut sp);

    let mut up: *mut png_unknown_chunk = ptr::null_mut();
    let n_unknown = png_get_unknown_chunks(l, pp, ip, &mut up);

    let rows_null = png_get_rows(l, pp, ip).is_null();

    let mut pal: *mut png_color = ptr::null_mut();
    let mut npal: c_int = -99;
    let plte = (png_get_PLTE(l, pp, ip, &mut pal, &mut npal), npal);

    let mut hp: *mut png_uint_16 = ptr::null_mut();
    let hist = png_get_hIST(l, pp, ip, &mut hp);

    let mut iname: png_charp = ptr::null_mut();
    let mut ictype: c_int = -99;
    let mut iprof: png_bytep = ptr::null_mut();
    let mut iproflen: png_uint_32 = 0;
    let iccp = (
        png_get_iCCP(l, pp, ip, &mut iname, &mut ictype, &mut iprof, &mut iproflen),
        iproflen,
    );

    let mut purpose: png_charp = ptr::null_mut();
    let mut x0: png_int_32 = 0;
    let mut x1: png_int_32 = 0;
    let mut ptype: c_int = -99;
    let mut nparams: c_int = -99;
    let mut units: png_charp = ptr::null_mut();
    let mut params: *mut png_charp = ptr::null_mut();
    let pret = png_get_pCAL(
        l, pp, ip, &mut purpose, &mut x0, &mut x1, &mut ptype, &mut nparams, &mut units,
        &mut params,
    );
    let pcal = (pret, x0, x1, ptype, nparams);

    let mut sunit: c_int = -99;
    let mut sw: png_charp = ptr::null_mut();
    let mut sh: png_charp = ptr::null_mut();
    let scal = (png_get_sCAL_s(l, pp, ip, &mut sunit, &mut sw, &mut sh), sunit);

    let mut trans: png_bytep = ptr::null_mut();
    let mut ntrans: c_int = -99;
    let mut tcol: *mut png_color_16 = ptr::null_mut();
    let trns = (
        png_get_tRNS(l, pp, ip, &mut trans, &mut ntrans, &mut tcol),
        ntrans,
    );

    let mut enum_: png_uint_32 = 0;
    let mut eptr: png_bytep = ptr::null_mut();
    let exif = (png_get_eXIf_1(l, pp, ip, &mut enum_, &mut eptr), enum_);

    let mut bg: *mut png_color_16 = ptr::null_mut();
    let bkgd = png_get_bKGD(l, pp, ip, &mut bg);
    let mut sb: *mut png_color_8 = ptr::null_mut();
    let sbit = png_get_sBIT(l, pp, ip, &mut sb);
    let mut tm: *mut png_time = ptr::null_mut();
    let tIME = png_get_tIME(l, pp, ip, &mut tm);

    InfoState {
        valid,
        width: png_get_image_width(l, pp, ip),
        height: png_get_image_height(l, pp, ip),
        n_text: (tret, tn),
        n_splt,
        n_unknown,
        rows_null,
        plte,
        hist,
        iccp,
        pcal,
        scal,
        trns,
        exif,
        bkgd,
        sbit,
        tIME,
    }
}

const POP_H: png_uint_32 = 3;
const POP_W: png_uint_32 = 4;

/// Owned buffers that must outlive the `png_set_*` calls that reference them.
struct Pop {
    _keep: Vec<CString>,
    _rows: Vec<Vec<u8>>,
    _rowp: Vec<png_bytep>,
    _params: Vec<png_charp>,
    _splt: Vec<png_sPLT_t>,
    _entries: Vec<Vec<png_sPLT_entry>>,
    _unk: Vec<png_unknown_chunk>,
    _unkdata: Vec<Vec<u8>>,
}

/// Fill an info struct with every kind of allocated payload so that
/// `png_free_data` actually has work to do.  Uses a *write* struct, which is
/// the only side on which `png_set_unknown_chunks` is legal.
unsafe fn populate(l: &Library, pp: png_structp, ip: png_infop) -> Pop {
    populate_ex(l, pp, ip, true)
}

/// `set_rows == false` leaves `info_ptr->row_pointers` NULL.  Required by the
/// `png_data_freer` test, which deliberately hands `PNG_FREE_ROWS` ownership to
/// libpng: doing that with app-owned rows would make libpng `free()` memory it
/// does not own.
unsafe fn populate_ex(l: &Library, pp: png_structp, ip: png_infop, set_rows: bool) -> Pop {
    let mut keep: Vec<CString> = Vec::new();

    png_set_IHDR(
        l,
        pp,
        ip,
        POP_W,
        POP_H,
        8,
        PNG_COLOR_TYPE_PALETTE,
        PNG_INTERLACE_NONE,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );

    // PLTE (+ hIST + tRNS, all palette-sized)
    let pal: Vec<png_color> = (0..4u8)
        .map(|i| png_color { red: i, green: 2 * i, blue: 3 * i })
        .collect();
    png_set_PLTE(l, pp, ip, pal.as_ptr(), 4);

    let hist: [png_uint_16; 4] = [10, 20, 30, 40];
    png_set_hIST(l, pp, ip, hist.as_ptr());

    let trans: [png_byte; 4] = [0, 85, 170, 255];
    png_set_tRNS(l, pp, ip, trans.as_ptr(), 4, ptr::null());

    // text (3 entries so num = 0..2 are all valid)
    for i in 0..3 {
        keep.push(cs(&format!("Key{i}")));
        keep.push(cs(&format!("text number {i}")));
    }
    let texts: Vec<png_text> = (0..3)
        .map(|i| png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: keep[2 * i].as_ptr() as png_charp,
            text: keep[2 * i + 1].as_ptr() as png_charp,
            text_length: 0,
            itxt_length: 0,
            lang: ptr::null_mut(),
            lang_key: ptr::null_mut(),
        })
        .collect();
    png_set_text(l, pp, ip, texts.as_ptr(), 3);

    // sPLT (3 entries)
    let mut entries: Vec<Vec<png_sPLT_entry>> = Vec::new();
    let base = keep.len();
    for i in 0..3 {
        keep.push(cs(&format!("splt{i}")));
        entries.push(
            (0..2u16)
                .map(|j| png_sPLT_entry {
                    red: j,
                    green: j + 1,
                    blue: j + 2,
                    alpha: j + 3,
                    frequency: j + 4,
                })
                .collect(),
        );
    }
    let mut splt: Vec<png_sPLT_t> = Vec::new();
    for i in 0..3 {
        splt.push(png_sPLT_t {
            name: keep[base + i].as_ptr() as png_charp,
            depth: 8,
            entries: entries[i].as_mut_ptr(),
            nentries: 2,
        });
    }
    png_set_sPLT(l, pp, ip, splt.as_ptr(), 3);

    // iCCP (content is never validated by png_set_iCCP)
    let profile: Vec<u8> = (0..200u32).map(|i| (i & 0xff) as u8).collect();
    let iccname = cs("an icc profile");
    png_set_iCCP(
        l,
        pp,
        ip,
        iccname.as_ptr(),
        PNG_COMPRESSION_TYPE_BASE,
        profile.as_ptr(),
        profile.len() as png_uint_32,
    );
    keep.push(iccname);

    // pCAL (3 params)
    let purpose = cs("calibration");
    let units = cs("metres");
    let mut params_owned: Vec<CString> = (0..3).map(|i| cs(&format!("{i}.5"))).collect();
    let mut params: Vec<png_charp> =
        params_owned.iter_mut().map(|c| c.as_ptr() as png_charp).collect();
    png_set_pCAL(
        l,
        pp,
        ip,
        purpose.as_ptr(),
        -100,
        100,
        0,
        3,
        units.as_ptr(),
        params.as_mut_ptr(),
    );
    keep.push(purpose);
    keep.push(units);
    keep.extend(params_owned);

    // sCAL (string form -> allocates)
    let sw = cs("1.5");
    let sh = cs("2.5");
    png_set_sCAL_s(l, pp, ip, 1, sw.as_ptr(), sh.as_ptr());
    keep.push(sw);
    keep.push(sh);

    // eXIf
    let exif: Vec<u8> = b"II*\0\x08\0\0\0".to_vec();
    png_set_eXIf_1(
        l,
        pp,
        ip,
        exif.len() as png_uint_32,
        exif.as_ptr() as png_bytep,
    );

    // unknown chunks (3 entries)
    let mut unkdata: Vec<Vec<u8>> = Vec::new();
    for i in 0..3u8 {
        unkdata.push(vec![i, i + 1, i + 2, i + 3]);
    }
    let mut unk: Vec<png_unknown_chunk> = Vec::new();
    for (i, d) in unkdata.iter_mut().enumerate() {
        unk.push(png_unknown_chunk {
            name: [b'u', b'N', b'k', b'A' + i as u8, 0],
            data: d.as_mut_ptr(),
            size: d.len(),
            location: PNG_HAVE_IHDR as png_byte,
        });
    }
    png_set_unknown_chunks(l, pp, ip, unk.as_ptr(), 3);

    // simple (non-allocating) chunks, so the valid-bit vector is interesting
    png_set_gAMA_fixed(l, pp, ip, 45455);
    let sbit = png_color_8 { red: 8, green: 8, blue: 8, gray: 0, alpha: 0 };
    png_set_sBIT(l, pp, ip, &sbit);
    let bg = png_color_16 { index: 1, red: 0, green: 0, blue: 0, gray: 0 };
    png_set_bKGD(l, pp, ip, &bg);
    png_set_pHYs(l, pp, ip, 100, 100, 1);
    png_set_oFFs(l, pp, ip, 5, 6, 0);
    let t = png_time { year: 2024, month: 1, day: 2, hour: 3, minute: 4, second: 5 };
    png_set_tIME(l, pp, ip, &t);
    png_set_cICP(l, pp, ip, 1, 13, 0, 1);
    png_set_cLLI_fixed(l, pp, ip, 1000 * 10000, 100 * 10000);

    // rows: user-owned, so PNG_FREE_ROWS is *not* in free_me.
    let mut rows: Vec<Vec<u8>> = (0..POP_H).map(|_| vec![0u8; POP_W as usize]).collect();
    let mut rowp: Vec<png_bytep> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    if set_rows {
        png_set_rows(l, pp, ip, rowp.as_mut_ptr());
    }

    Pop {
        _keep: keep,
        _rows: rows,
        _rowp: rowp,
        _params: params,
        _splt: splt,
        _entries: entries,
        _unk: unk,
        _unkdata: unkdata,
    }
}

#[test]
fn cfg53_create_destroy_info_struct() {
    let l = libs();
    let run = |lib: &'static Library| {
        capture(|| unsafe {
            // png_create_info_struct(NULL) must be NULL
            let n = png_create_info_struct(lib, ptr::null_mut()).is_null();

            let pp = api::new_writer(lib);
            let ip = png_create_info_struct(lib, pp);
            let created = !ip.is_null();
            let pop = populate(lib, pp, ip);
            let before = info_state(lib, pp, ip);

            // destroy with a NULL png_ptr: no-op, ipp untouched
            let mut hold = ip;
            png_destroy_info_struct(lib, ptr::null_mut(), &mut hold);
            let untouched = hold == ip;

            // destroy with a NULL ipp: no-op
            png_destroy_info_struct(lib, pp, ptr::null_mut());

            let mut i = ip;
            png_destroy_info_struct(lib, pp, &mut i);
            let nulled = i.is_null();
            // double destroy of the (now NULL) slot must be safe
            png_destroy_info_struct(lib, pp, &mut i);

            let mut p = pp;
            png_destroy_write_struct(lib, &mut p, ptr::null_mut());
            drop(pop);
            (n, created, untouched, nulled, before)
        })
    };
    let a = run(&l.c);
    let b = run(&l.rs);
    a.assert_eq(&b, "create/destroy info struct");
}

#[test]
fn cfg53_info_init_3() {
    let l = libs();
    // The exact C sizeof(png_info) is 352; probe on both sides of it plus the
    // degenerate values.
    let grid: [usize; 8] = [
        0,
        1,
        8,
        SIZEOF_PNG_INFO_C - 1,
        SIZEOF_PNG_INFO_C,
        SIZEOF_PNG_INFO_C + 1,
        4096,
        usize::MAX,
    ];

    for &size in &grid {
        let run = |lib: &'static Library| {
            capture(|| unsafe {
                // NOTE: png_info_init_3 uses the *system* free/malloc, so the
                // struct must not be created with a user allocator.
                let pp = api::new_writer(lib);
                let ip = png_create_info_struct(lib, pp);
                // Put something in it so a plain memset is observable.
                png_set_IHDR(
                    lib, pp, ip, 7, 5, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                png_set_gAMA_fixed(lib, pp, ip, 45455);

                let mut slot = ip;
                png_info_init_3(lib, &mut slot, size);
                let reallocated = slot != ip;
                let now_null = slot.is_null();
                let st = if now_null {
                    None
                } else {
                    Some(info_state(lib, pp, slot))
                };

                if !now_null {
                    png_destroy_info_struct(lib, pp, &mut slot);
                }
                let mut p = pp;
                png_destroy_write_struct(lib, &mut p, ptr::null_mut());
                (reallocated, now_null, st)
            })
        };
        let a = run(&l.c);
        let b = run(&l.rs);
        a.assert_eq(&b, &format!("png_info_init_3(size={size})"));
    }

    // png_info_init_3 with *ptr_ptr == NULL must be a no-op.
    let run = |lib: &'static Library| {
        capture(|| unsafe {
            let mut slot: png_infop = ptr::null_mut();
            png_info_init_3(lib, &mut slot, SIZEOF_PNG_INFO_C);
            slot.is_null()
        })
    };
    run(&l.c).assert_eq(&run(&l.rs), "png_info_init_3(NULL slot)");
}

#[test]
fn cfg53_free_data_masks() {
    let l = libs();
    let mut distinct: std::collections::BTreeSet<String> = Default::default();
    for &(mname, mask) in ALL_FREE_MASKS {
        for num in [-1i32, 0, 1, 2] {
            let run = |lib: &'static Library| {
                capture(|| unsafe {
                    let pp = api::new_writer(lib);
                    let ip = png_create_info_struct(lib, pp);
                    let pop = populate(lib, pp, ip);
                    png_free_data(lib, pp, ip, mask, num);
                    let st = info_state(lib, pp, ip);
                    // A second, identical free must be idempotent.
                    png_free_data(lib, pp, ip, mask, num);
                    let st2 = info_state(lib, pp, ip);
                    let mut i = ip;
                    png_destroy_info_struct(lib, pp, &mut i);
                    let mut p = pp;
                    png_destroy_write_struct(lib, &mut p, ptr::null_mut());
                    drop(pop);
                    (st, st2)
                })
            };
            let a = run(&l.c);
            let b = run(&l.rs);
            a.assert_eq(&b, &format!("png_free_data(PNG_FREE_{mname}, num={num})"));
            if let Some((st, _)) = &a.out {
                distinct.insert(format!("{st:?}"));
            }
        }
    }
    assert!(
        distinct.len() > 5,
        "png_free_data produced only {} distinct info states - the info struct \
         is probably not being populated",
        distinct.len()
    );

    // NULL arguments must be no-ops.
    let run = |lib: &'static Library| {
        capture(|| unsafe {
            let pp = api::new_writer(lib);
            let ip = png_create_info_struct(lib, pp);
            png_free_data(lib, ptr::null_mut(), ip, FREE_ALL, -1);
            png_free_data(lib, pp, ptr::null_mut(), FREE_ALL, -1);
            let st = info_state(lib, pp, ip);
            let mut i = ip;
            png_destroy_info_struct(lib, pp, &mut i);
            let mut p = pp;
            png_destroy_write_struct(lib, &mut p, ptr::null_mut());
            st
        })
    };
    run(&l.c).assert_eq(&run(&l.rs), "png_free_data(NULL, ...)");
}

/// `PNG_FREE_ROWS` only does anything when the app has handed ownership of the
/// rows to libpng via `png_data_freer`, so it gets its own scenario.
#[test]
fn cfg53_free_data_rows_owned() {
    let l = libs();
    for num in [-1i32, 0] {
        let run = |lib: &'static Library| {
            capture(|| unsafe {
                let pp = api::new_writer(lib);
                let ip = png_create_info_struct(lib, pp);
                png_set_IHDR(
                    lib, pp, ip, 4, 3, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                // Rows allocated *by libpng* so that png_free can release them.
                let rowp = png_malloc(lib, pp, 3 * std::mem::size_of::<png_bytep>())
                    as *mut png_bytep;
                for r in 0..3isize {
                    *rowp.offset(r) = png_malloc(lib, pp, 4) as png_bytep;
                }
                png_set_rows(lib, pp, ip, rowp);
                png_data_freer(lib, pp, ip, PNG_DESTROY_WILL_FREE_DATA, FREE_ROWS);
                let before = info_state(lib, pp, ip);
                png_free_data(lib, pp, ip, FREE_ROWS, num);
                let after = info_state(lib, pp, ip);
                let mut i = ip;
                png_destroy_info_struct(lib, pp, &mut i);
                let mut p = pp;
                png_destroy_write_struct(lib, &mut p, ptr::null_mut());
                (before, after)
            })
        };
        let a = run(&l.c);
        let b = run(&l.rs);
        a.assert_eq(&b, &format!("png_free_data(PNG_FREE_ROWS, num={num}) owned"));
    }
}

#[test]
fn cfg53_data_freer() {
    let l = libs();
    let freers: [(&str, c_int); 5] = [
        ("PNG_DESTROY_WILL_FREE_DATA", PNG_DESTROY_WILL_FREE_DATA),
        ("PNG_SET_WILL_FREE_DATA", PNG_SET_WILL_FREE_DATA),
        ("PNG_USER_WILL_FREE_DATA", PNG_USER_WILL_FREE_DATA),
        ("invalid(0)", 0),
        ("invalid(3)", 3),
    ];

    for &(fname, freer) in &freers {
        for &(mname, mask) in ALL_FREE_MASKS {
            let run = |lib: &'static Library| {
                capture(|| unsafe {
                    let pp = api::new_writer(lib);
                    let ip = png_create_info_struct(lib, pp);
                    let pop = populate_ex(lib, pp, ip, false);
                    png_data_freer(lib, pp, ip, freer, mask);
                    let after_freer = info_state(lib, pp, ip);
                    // Now try to free everything: what actually goes away
                    // depends entirely on the free_me bits png_data_freer left.
                    png_free_data(lib, pp, ip, FREE_ALL, -1);
                    let after_free = info_state(lib, pp, ip);
                    let mut i = ip;
                    png_destroy_info_struct(lib, pp, &mut i);
                    let mut p = pp;
                    png_destroy_write_struct(lib, &mut p, ptr::null_mut());
                    drop(pop);
                    (after_freer, after_free)
                })
            };
            let a = run(&l.c);
            let b = run(&l.rs);
            a.assert_eq(&b, &format!("png_data_freer({fname}, PNG_FREE_{mname})"));
        }
    }

    // NULL arguments must be no-ops (in particular they must NOT png_error).
    let run = |lib: &'static Library| {
        capture(|| unsafe {
            let pp = api::new_writer(lib);
            let ip = png_create_info_struct(lib, pp);
            png_data_freer(lib, ptr::null_mut(), ip, 999, FREE_ALL);
            png_data_freer(lib, pp, ptr::null_mut(), 999, FREE_ALL);
            let mut i = ip;
            png_destroy_info_struct(lib, pp, &mut i);
            let mut p = pp;
            png_destroy_write_struct(lib, &mut p, ptr::null_mut());
            0u8
        })
    };
    run(&l.c).assert_eq(&run(&l.rs), "png_data_freer(NULL, ...)");
}

// ===========================================================================
// Row 57 - ICC profile checks
// ===========================================================================

/// Encoded D50 as an ICC XYZNumber (`D50_nCIEXYZ` in `c_src/src/png.c`).
const D50: [u8; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];

#[derive(Clone)]
struct IccSpec {
    /// value stored at profile[0..4]
    len_field: png_uint_32,
    /// profile[8] - the major version byte
    version: u8,
    class: [u8; 4],
    space: [u8; 4],
    pcs: [u8; 4],
    sig: [u8; 4],
    intent: png_uint_32,
    d50: bool,
    /// value stored at profile[128..132]
    tag_count_field: png_uint_32,
    /// (id, start, length) triples written from offset 132 on
    tags: Vec<(png_uint_32, png_uint_32, png_uint_32)>,
    /// total size of the buffer handed to libpng
    buf_len: usize,
}

impl Default for IccSpec {
    fn default() -> Self {
        IccSpec {
            len_field: 132,
            version: 2,
            class: *b"mntr",
            space: *b"RGB ",
            pcs: *b"XYZ ",
            sig: *b"acsp",
            intent: 0,
            d50: true,
            tag_count_field: 0,
            tags: Vec::new(),
            buf_len: 512,
        }
    }
}

impl IccSpec {
    fn build(&self) -> Vec<u8> {
        let need = 132 + 12 * self.tags.len();
        let mut b = vec![0u8; self.buf_len.max(need).max(132)];
        b[0..4].copy_from_slice(&self.len_field.to_be_bytes());
        b[8] = self.version;
        b[12..16].copy_from_slice(&self.class);
        b[16..20].copy_from_slice(&self.space);
        b[20..24].copy_from_slice(&self.pcs);
        b[36..40].copy_from_slice(&self.sig);
        b[64..68].copy_from_slice(&self.intent.to_be_bytes());
        if self.d50 {
            b[68..80].copy_from_slice(&D50);
        } else {
            b[68..80].copy_from_slice(&[1u8; 12]);
        }
        b[128..132].copy_from_slice(&self.tag_count_field.to_be_bytes());
        for (i, &(id, start, len)) in self.tags.iter().enumerate() {
            let o = 132 + 12 * i;
            b[o..o + 4].copy_from_slice(&id.to_be_bytes());
            b[o + 4..o + 8].copy_from_slice(&start.to_be_bytes());
            b[o + 8..o + 12].copy_from_slice(&len.to_be_bytes());
        }
        b
    }
}

/// (label, spec, proflen argument, color_type)
fn icc_cases() -> Vec<(String, IccSpec, png_uint_32, c_int)> {
    let mut v: Vec<(String, IccSpec, png_uint_32, c_int)> = Vec::new();
    let d = IccSpec::default();

    v.push(("minimal-valid-RGB".into(), d.clone(), 132, PNG_COLOR_TYPE_RGB));
    v.push((
        "minimal-valid-GRAY".into(),
        IccSpec { space: *b"GRAY", ..d.clone() },
        132,
        PNG_COLOR_TYPE_GRAY,
    ));
    // colour-space / colour-type mismatches
    v.push(("RGB-on-gray".into(), d.clone(), 132, PNG_COLOR_TYPE_GRAY));
    v.push((
        "GRAY-on-rgb".into(),
        IccSpec { space: *b"GRAY", ..d.clone() },
        132,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "CMYK-space".into(),
        IccSpec { space: *b"CMYK", ..d.clone() },
        132,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "GRAY-on-palette".into(),
        IccSpec { space: *b"GRAY", ..d.clone() },
        132,
        PNG_COLOR_TYPE_PALETTE,
    ));
    v.push((
        "RGB-on-gray-alpha".into(),
        d.clone(),
        132,
        PNG_COLOR_TYPE_GRAY_ALPHA,
    ));

    // length boundaries
    v.push((
        "len-133-v2".into(),
        IccSpec { len_field: 133, ..d.clone() },
        133,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "len-133-v4".into(),
        IccSpec { len_field: 133, version: 4, ..d.clone() },
        133,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "len-136-v4".into(),
        IccSpec { len_field: 136, version: 4, ..d.clone() },
        136,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "len-131-below-min".into(),
        IccSpec { len_field: 131, ..d.clone() },
        131,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "len-0".into(),
        IccSpec { len_field: 0, ..d.clone() },
        0,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "len-mismatch".into(),
        IccSpec { len_field: 500, ..d.clone() },
        132,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "len-UINT31MAX".into(),
        IccSpec { len_field: PNG_UINT_31_MAX, ..d.clone() },
        PNG_UINT_31_MAX,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "len-above-UINT31MAX".into(),
        IccSpec { len_field: 0x8000_0000, ..d.clone() },
        0x8000_0000,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "len-8000001".into(),
        IccSpec { len_field: 8_000_001, ..d.clone() },
        8_000_001,
        PNG_COLOR_TYPE_RGB,
    ));

    // signature
    v.push((
        "bad-signature".into(),
        IccSpec { sig: *b"junk", ..d.clone() },
        132,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "zero-signature".into(),
        IccSpec { sig: [0, 0, 0, 0], ..d.clone() },
        132,
        PNG_COLOR_TYPE_RGB,
    ));

    // tag counts
    for tc in [0u32, 1, 3, 10] {
        let tags: Vec<(u32, u32, u32)> = (0..tc)
            .map(|i| (0x41414141 + i, 132 + 12 * tc + 4 * i, 4))
            .collect();
        let len = 132 + 12 * tc + 4 * tc;
        v.push((
            format!("tagcount-{tc}"),
            IccSpec {
                len_field: len,
                tag_count_field: tc,
                tags,
                buf_len: (len as usize).max(512),
                ..d.clone()
            },
            len,
            PNG_COLOR_TYPE_RGB,
        ));
    }
    v.push((
        "tagcount-huge".into(),
        IccSpec { tag_count_field: 357_913_931, ..d.clone() },
        132,
        PNG_COLOR_TYPE_RGB,
    ));
    v.push((
        "tagcount-truncated-table".into(),
        IccSpec { tag_count_field: 5, ..d.clone() },
        132,
        PNG_COLOR_TYPE_RGB,
    ));

    // rendering intent
    for intent in [0u32, 1, 2, 3, 4, 0xfffe, 0xffff, 0x1_0000] {
        v.push((
            format!("intent-{intent}"),
            IccSpec { intent, ..d.clone() },
            132,
            PNG_COLOR_TYPE_RGB,
        ));
    }

    // illuminant
    v.push(("illuminant-not-D50".into(), IccSpec { d50: false, ..d.clone() }, 132,
            PNG_COLOR_TYPE_RGB));

    // profile class
    for cls in [b"scnr", b"mntr", b"prtr", b"spac", b"abst", b"link", b"nmcl", b"junk"] {
        v.push((
            format!("class-{}", String::from_utf8_lossy(cls)),
            IccSpec { class: *cls, ..d.clone() },
            132,
            PNG_COLOR_TYPE_RGB,
        ));
    }

    // PCS
    for pcs in [b"XYZ ", b"Lab ", b"junk"] {
        v.push((
            format!("pcs-{}", String::from_utf8_lossy(pcs)),
            IccSpec { pcs: *pcs, ..d.clone() },
            132,
            PNG_COLOR_TYPE_RGB,
        ));
    }

    v
}

/// Run `f` with a fresh png_struct of the requested flavour.
/// `flavour`: 0 = reader, 1 = writer, 2 = reader with benign errors disabled.
unsafe fn with_struct<T>(
    lib: &'static Library,
    flavour: u8,
    f: impl FnOnce(png_structp) -> T,
) -> T {
    let pp = match flavour {
        1 => api::new_writer(lib),
        _ => {
            src_set(&[]);
            api::new_reader(lib)
        }
    };
    if flavour == 2 {
        png_set_benign_errors(lib, pp, 0);
    }
    let r = f(pp);
    let mut p = pp;
    if flavour == 1 {
        png_destroy_write_struct(lib, &mut p, ptr::null_mut());
    } else {
        png_destroy_read_struct(lib, &mut p, ptr::null_mut(), ptr::null_mut());
    }
    r
}

#[test]
fn cfg57_icc_check_length() {
    let l = libs();
    let name = cs("prof");
    let lens: [png_uint_32; 12] = [
        0,
        1,
        131,
        132,
        133,
        136,
        1000,
        8_000_000,
        8_000_001,
        PNG_UINT_31_MAX,
        0x8000_0000,
        0xffff_ffff,
    ];
    for flavour in [0u8, 1, 2] {
        for &plen in &lens {
            let run = |lib: &'static Library| {
                capture(|| unsafe {
                    with_struct(lib, flavour, |pp| {
                        x::png_icc_check_length(lib, pp, name.as_ptr(), plen)
                    })
                })
            };
            let a = run(&l.c);
            let b = run(&l.rs);
            a.assert_eq(&b, &format!("png_icc_check_length(len={plen}, flavour={flavour})"));
        }
    }

    // Also vary the chunk-malloc limit, which is the second bound the C checks.
    for limit in [0usize, 132, 1000, 8_000_000, usize::MAX] {
        for &plen in &[132u32, 1000, 8_000_001] {
            let run = |lib: &'static Library| {
                capture(|| unsafe {
                    with_struct(lib, 0, |pp| {
                        png_set_chunk_malloc_max(lib, pp, limit);
                        x::png_icc_check_length(lib, pp, name.as_ptr(), plen)
                    })
                })
            };
            let a = run(&l.c);
            let b = run(&l.rs);
            a.assert_eq(
                &b,
                &format!("png_icc_check_length(len={plen}, chunk_malloc_max={limit})"),
            );
        }
    }
}

#[test]
fn cfg57_icc_check_header() {
    let l = libs();
    let name = cs("prof");
    let mut n_ok = 0usize;
    let mut n_rejected = 0usize;
    let mut n_unwound = 0usize;
    let mut n_warned = 0usize;
    for (label, spec, plen, ct) in icc_cases() {
        let profile = spec.build();
        for flavour in [0u8, 1, 2] {
            let run = |lib: &'static Library| {
                capture(|| unsafe {
                    with_struct(lib, flavour, |pp| {
                        x::png_icc_check_header(
                            lib,
                            pp,
                            name.as_ptr(),
                            plen,
                            profile.as_ptr(),
                            ct,
                        )
                    })
                })
            };
            let a = run(&l.c);
            let b = run(&l.rs);
            a.assert_eq(
                &b,
                &format!("png_icc_check_header[{label}] ct={ct} flavour={flavour}"),
            );
            match a.out {
                None => n_unwound += 1,
                Some(1) => n_ok += 1,
                Some(_) => n_rejected += 1,
            }
            if !a.log.is_empty() {
                n_warned += 1;
            }
        }
    }
    assert!(
        n_ok > 0 && n_rejected > 0 && n_unwound > 0 && n_warned > 0,
        "icc header sweep is vacuous: ok={n_ok} rejected={n_rejected} \
         unwound={n_unwound} warned={n_warned}"
    );

    // A very long profile name exercises the 79-char truncation in
    // png_icc_profile_error.
    let long = cs(&"n".repeat(200));
    let bad = IccSpec { sig: *b"junk", ..IccSpec::default() };
    let profile = bad.build();
    let run = |lib: &'static Library| {
        capture(|| unsafe {
            with_struct(lib, 0, |pp| {
                x::png_icc_check_header(
                    lib,
                    pp,
                    long.as_ptr(),
                    132,
                    profile.as_ptr(),
                    PNG_COLOR_TYPE_RGB,
                )
            })
        })
    };
    run(&l.c).assert_eq(&run(&l.rs), "png_icc_check_header(long name)");
}

#[test]
fn cfg57_icc_check_tag_table() {
    let l = libs();
    let name = cs("prof");
    let base = IccSpec::default();

    // (label, tag_count, tags, proflen)
    let mut cases: Vec<(String, png_uint_32, Vec<(u32, u32, u32)>, png_uint_32)> = Vec::new();
    cases.push(("empty".into(), 0, vec![], 132));
    cases.push(("one-aligned".into(), 1, vec![(0x41414141, 144, 4)], 148));
    cases.push((
        "three-aligned".into(),
        3,
        vec![(0x41414141, 168, 4), (0x42424242, 172, 4), (0x43434343, 176, 8)],
        184,
    ));
    cases.push((
        "start-unaligned".into(),
        1,
        vec![(0x41414141, 145, 4), ],
        160,
    ));
    cases.push(("start-past-end".into(), 1, vec![(0x41414141, 1000, 4)], 148));
    cases.push((
        "length-past-end".into(),
        1,
        vec![(0x41414141, 144, 0xffff_ffff)],
        148,
    ));
    cases.push(("zero-length-tag".into(), 1, vec![(0x41414141, 144, 0)], 148));
    cases.push((
        "start-equals-len".into(),
        1,
        vec![(0x41414141, 148, 0)],
        148,
    ));
    cases.push((
        "mixed".into(),
        3,
        vec![(0x41414141, 168, 4), (0x42424242, 170, 4), (0x43434343, 9999, 1)],
        184,
    ));
    cases.push((
        "non-printable-id".into(),
        1,
        vec![(0x0001_0203, 145, 4)],
        160,
    ));

    let mut n_ok = 0usize;
    let mut n_rejected = 0usize;
    let mut n_warned = 0usize;
    for (label, tc, tags, plen) in cases {
        let need = 132 + 12 * tags.len();
        let spec = IccSpec {
            len_field: plen,
            tag_count_field: tc,
            tags: tags.clone(),
            buf_len: need.max(512),
            ..base.clone()
        };
        let profile = spec.build();
        for flavour in [0u8, 1, 2] {
            let run = |lib: &'static Library| {
                capture(|| unsafe {
                    with_struct(lib, flavour, |pp| {
                        x::png_icc_check_tag_table(
                            lib,
                            pp,
                            name.as_ptr(),
                            plen,
                            profile.as_ptr(),
                        )
                    })
                })
            };
            let a = run(&l.c);
            let b = run(&l.rs);
            a.assert_eq(
                &b,
                &format!("png_icc_check_tag_table[{label}] flavour={flavour}"),
            );
            match a.out {
                Some(1) => n_ok += 1,
                _ => n_rejected += 1,
            }
            if !a.log.is_empty() {
                n_warned += 1;
            }
        }
    }
    assert!(
        n_ok > 0 && n_rejected > 0 && n_warned > 0,
        "icc tag-table sweep is vacuous: ok={n_ok} rejected={n_rejected} warned={n_warned}"
    );
}

#[test]
fn cfg57_resolve_file_gamma() {
    let l = libs();
    // png_resolve_file_gamma(png_const_structrp) takes only the png_struct, so
    // it is driven through the public setters that feed the fields it reads
    // (file_gamma, chunk_gamma, default_gamma, screen_gamma).
    #[allow(clippy::type_complexity)]
    let scenarios: Vec<(&str, fn(&'static Library, png_structp))> = vec![
        ("fresh", |_l, _pp| {}),
        ("set_gamma(2.2,0.45455)", |l, pp| unsafe {
            png_set_gamma_fixed(l, pp, 220000, 45455)
        }),
        ("set_gamma(0,0.45455)", |l, pp| unsafe {
            png_set_gamma_fixed(l, pp, 0, 45455)
        }),
        ("set_gamma(2.2,0)", |l, pp| unsafe {
            png_set_gamma_fixed(l, pp, 220000, 0)
        }),
        ("set_gamma(0,0)", |l, pp| unsafe {
            png_set_gamma_fixed(l, pp, 0, 0)
        }),
        ("set_gamma(1,1)", |l, pp| unsafe {
            png_set_gamma_fixed(l, pp, PNG_FP_1, PNG_FP_1)
        }),
        ("alpha_mode(PNG,sRGB)", |l, pp| unsafe {
            png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_PNG, PNG_DEFAULT_sRGB)
        }),
        ("alpha_mode(STANDARD,1.0)", |l, pp| unsafe {
            png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_STANDARD, PNG_FP_1)
        }),
        ("alpha_mode(OPTIMIZED,MAC18)", |l, pp| unsafe {
            png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_OPTIMIZED, PNG_GAMMA_MAC_18)
        }),
        ("alpha_mode(BROKEN,2.2)", |l, pp| unsafe {
            png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_BROKEN, 220000)
        }),
        ("alpha_mode+gamma", |l, pp| unsafe {
            png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_PNG, PNG_DEFAULT_sRGB);
            png_set_gamma_fixed(l, pp, 220000, 100000);
        }),
    ];

    for (label, setup) in scenarios {
        let run = |lib: &'static Library| {
            capture(|| unsafe {
                src_set(&[]);
                let pp = api::new_reader(lib);
                setup(lib, pp);
                let g = x::png_resolve_file_gamma(lib, pp);
                let mut p = pp;
                png_destroy_read_struct(lib, &mut p, ptr::null_mut(), ptr::null_mut());
                g
            })
        };
        let a = run(&l.c);
        let b = run(&l.rs);
        a.assert_eq(&b, &format!("png_resolve_file_gamma[{label}]"));
    }

    // ... and after a real gAMA chunk has been read from the stream.
    for gamma in [45455u32, 100000, 220000, 1, 0] {
        let stream = {
            let mut s = png_signature();
            s.extend(chunk(b"IHDR", &ihdr(4, 3, 8, PNG_COLOR_TYPE_RGB as u8)));
            s.extend(chunk(b"gAMA", &gamma.to_be_bytes()));
            s.extend(chunk(b"IEND", &[]));
            s
        };
        let run = |lib: &'static Library| {
            capture(|| unsafe {
                src_set(&stream);
                let pp = api::new_reader(lib);
                let ip = png_create_info_struct(lib, pp);
                png_read_info(lib, pp, ip);
                let g = x::png_resolve_file_gamma(lib, pp);
                let mut p = pp;
                let mut i = ip;
                png_destroy_read_struct(lib, &mut p, &mut i, ptr::null_mut());
                g
            })
        };
        let a = run(&l.c);
        let b = run(&l.rs);
        a.assert_eq(&b, &format!("png_resolve_file_gamma[gAMA={gamma}]"));
    }
}

// ===========================================================================
// Chunk-stream construction helpers (rows 66 and 67)
// ===========================================================================

fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    for (n, e) in t.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *e = c;
    }
    t
}

fn crc32_of(data: &[u8]) -> u32 {
    let t = crc_table();
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c = t[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(12 + data.len());
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let mut crcinput = name.to_vec();
    crcinput.extend_from_slice(data);
    v.extend_from_slice(&crc32_of(&crcinput).to_be_bytes());
    v
}

fn png_signature() -> Vec<u8> {
    vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
}

fn ihdr(w: u32, h: u32, bd: u8, ct: u8) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&w.to_be_bytes());
    v.extend_from_slice(&h.to_be_bytes());
    v.push(bd);
    v.push(ct);
    v.push(0);
    v.push(0);
    v.push(0);
    v
}

fn adler32(d: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &x in d {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// A valid zlib stream that stores `data` uncompressed (one final stored
/// deflate block).  Lets the tests build zTXt / iTXt / iCCP chunks without a
/// compressor.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78u8, 0x01];
    let mut rest = data;
    loop {
        let n = rest.len().min(0xffff);
        let last = if rest.len() <= 0xffff { 1u8 } else { 0 };
        out.push(last);
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!(n as u16)).to_le_bytes());
        out.extend_from_slice(&rest[..n]);
        rest = &rest[n..];
        if last == 1 {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

// ===========================================================================
// Row 66 - CRC and zstream helpers
// ===========================================================================

// There is no public accessor for png_struct::crc, so png_reset_crc /
// png_calculate_crc are observed *indirectly*: png_write_chunk_start performs
// `png_reset_crc` + `png_calculate_crc(name,4)` and png_write_chunk_end emits
// `png_ptr->crc` as the trailing four bytes of the chunk.  Extra
// png_reset_crc / png_calculate_crc calls injected between them are therefore
// fully observable in the byte stream, which is what these tests compare.

#[test]
fn cfg66_calculate_crc_direct() {
    let l = libs();
    let mut rng = Rng::new(0x66_0001);
    let lens = [0usize, 1, 2, 255, 4096];
    let bufs: Vec<Vec<u8>> = lens.iter().map(|&n| rng.bytes(n)).collect();
    let extra = rng.bytes(37);
    let mut distinct: std::collections::BTreeSet<String> = Default::default();

    // scenario ids: 0 plain, 1 reset before end, 2 one extra crc chunk,
    // 3 two extra chunks (chained), 4 reset then two chunks,
    // 5 zero-length extra call, 6 NULL data pointer with non-zero length.
    for (bi, buf) in bufs.iter().enumerate() {
        for scenario in 0..7u8 {
            for name in [b"teSt", b"TEST"] {
                let run = |lib: &'static Library| {
                    capture(|| unsafe {
                        let pp = api::new_writer(lib);
                        sink_reset();
                        png_write_chunk_start(lib, pp, name.as_ptr(), buf.len() as png_uint_32);
                        png_write_chunk_data(lib, pp, buf.as_ptr(), buf.len());
                        match scenario {
                            0 => {}
                            1 => png_reset_crc(lib, pp),
                            2 => png_calculate_crc(lib, pp, extra.as_ptr(), extra.len()),
                            3 => {
                                png_calculate_crc(lib, pp, extra.as_ptr(), 5);
                                png_calculate_crc(lib, pp, extra[5..].as_ptr(), extra.len() - 5);
                            }
                            4 => {
                                png_reset_crc(lib, pp);
                                png_calculate_crc(lib, pp, extra.as_ptr(), 1);
                                png_calculate_crc(lib, pp, extra.as_ptr(), extra.len());
                            }
                            5 => png_calculate_crc(lib, pp, extra.as_ptr(), 0),
                            _ => png_calculate_crc(lib, pp, ptr::null(), 0),
                        }
                        png_write_chunk_end(lib, pp);
                        let (bytes, flushes) = sink_take();
                        let mut p = pp;
                        png_destroy_write_struct(lib, &mut p, ptr::null_mut());
                        (hex(&bytes), flushes)
                    })
                };
                let a = run(&l.c);
                let b = run(&l.rs);
                a.assert_eq(
                    &b,
                    &format!(
                        "crc len={} scenario={scenario} name={}",
                        lens[bi],
                        String::from_utf8_lossy(name)
                    ),
                );
                if let Some((h, _)) = &a.out {
                    distinct.insert(h.clone());
                }
            }
        }
    }
    // If png_reset_crc / png_calculate_crc had no observable effect all the
    // scenarios for a given length would collapse onto the same byte string.
    assert!(
        distinct.len() >= 4 * lens.len(),
        "CRC bytes barely vary ({} distinct outputs) - the indirect observation \
         of png_reset_crc / png_calculate_crc is not working",
        distinct.len()
    );
}

#[test]
fn cfg66_crc_action_affects_calculate_crc() {
    let l = libs();
    let mut rng = Rng::new(0x66_0002);
    let data = rng.bytes(100);
    let actions = [
        PNG_CRC_DEFAULT,
        PNG_CRC_ERROR_QUIT,
        PNG_CRC_WARN_DISCARD,
        PNG_CRC_WARN_USE,
        PNG_CRC_QUIET_USE,
        PNG_CRC_NO_CHANGE,
    ];
    let mut distinct: std::collections::BTreeSet<String> = Default::default();
    for &crit in &actions {
        for &ancil in &actions {
            for name in [b"teSt", b"TEST"] {
                let run = |lib: &'static Library| {
                    capture(|| unsafe {
                        let pp = api::new_writer(lib);
                        png_set_crc_action(lib, pp, crit, ancil);
                        sink_reset();
                        png_write_chunk(lib, pp, name.as_ptr(), data.as_ptr(), data.len());
                        let (bytes, _) = sink_take();
                        let mut p = pp;
                        png_destroy_write_struct(lib, &mut p, ptr::null_mut());
                        hex(&bytes)
                    })
                };
                let a = run(&l.c);
                let b = run(&l.rs);
                a.assert_eq(
                    &b,
                    &format!(
                        "crc_action(crit={crit}, ancil={ancil}) name={}",
                        String::from_utf8_lossy(name)
                    ),
                );
                if let Some(h) = &a.out {
                    distinct.insert(h.clone());
                }
            }
        }
    }
    // PNG_CRC_QUIET_USE turns need_crc off inside png_calculate_crc, which
    // must show up as a different trailing CRC.
    assert!(
        distinct.len() >= 4,
        "png_set_crc_action never changed the emitted CRC ({} distinct)",
        distinct.len()
    );
}

#[test]
fn cfg66_zstream_helpers() {
    let l = libs();

    // --- png_reset_zstream --------------------------------------------------
    // 0 = fresh reader, 1 = fresh writer, 2 = reader after png_read_info
    // (zstream owned by IDAT), 3 = NULL png_ptr.
    for flavour in 0..4u8 {
        let stream = {
            let mut s = png_signature();
            s.extend(chunk(b"IHDR", &ihdr(4, 3, 8, PNG_COLOR_TYPE_RGB as u8)));
            s.extend(chunk(b"IEND", &[]));
            s
        };
        let run = |lib: &'static Library| {
            capture(|| unsafe {
                match flavour {
                    0 => {
                        src_set(&[]);
                        let pp = api::new_reader(lib);
                        let r = png_reset_zstream(lib, pp);
                        let r2 = png_reset_zstream(lib, pp);
                        let mut p = pp;
                        png_destroy_read_struct(
                            lib,
                            &mut p,
                            ptr::null_mut(),
                            ptr::null_mut(),
                        );
                        (r, r2)
                    }
                    1 => {
                        let pp = api::new_writer(lib);
                        let r = png_reset_zstream(lib, pp);
                        let mut p = pp;
                        png_destroy_write_struct(lib, &mut p, ptr::null_mut());
                        (r, r)
                    }
                    2 => {
                        src_set(&stream);
                        let pp = api::new_reader(lib);
                        let ip = png_create_info_struct(lib, pp);
                        png_read_info(lib, pp, ip);
                        let r = png_reset_zstream(lib, pp);
                        let mut p = pp;
                        let mut i = ip;
                        png_destroy_read_struct(lib, &mut p, &mut i, ptr::null_mut());
                        (r, r)
                    }
                    _ => {
                        let r = png_reset_zstream(lib, ptr::null_mut());
                        (r, r)
                    }
                }
            })
        };
        let a = run(&l.c);
        let b = run(&l.rs);
        a.assert_eq(&b, &format!("png_reset_zstream(flavour={flavour})"));
    }

    // --- png_zalloc / png_zfree --------------------------------------------
    // NOTE: on a 64-bit target the `items >= SIZE_MAX/size` overflow guard in
    // png_zalloc is unreachable (max uInt * max uInt still fits the check),
    // so the (0xFFFFFFFF, 0xFFFFFFFF) case exercises the *allocation failure*
    // path rather than the overflow warning.  Both are compared.
    let grid: [(c_uint, c_uint); 10] = [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (1, 8),
        (16, 64),
        (1024, 1024),
        (0xFFFF_FFFF, 1),
        (2, 0xFFFF_FFFF),
        (0xFFFF_FFFF, 0xFFFF_FFFF),
    ];
    for &(items, size) in &grid {
        let run = |lib: &'static Library| {
            capture(|| unsafe {
                src_set(&[]);
                let pp = api::new_reader(lib);
                let p = png_zalloc(lib, pp, items, size);
                let null = p.is_null();
                png_zfree(lib, pp, p);
                png_zfree(lib, pp, ptr::null_mut());
                // NULL png_ptr must yield NULL and must not crash.
                let n2 = png_zalloc(lib, ptr::null_mut(), items, size).is_null();
                png_zfree(lib, ptr::null_mut(), ptr::null_mut());
                let mut q = pp;
                png_destroy_read_struct(lib, &mut q, ptr::null_mut(), ptr::null_mut());
                (null, n2)
            })
        };
        let a = run(&l.c);
        let b = run(&l.rs);
        a.assert_eq(&b, &format!("png_zalloc(items={items}, size={size})"));
    }

    // png_zalloc routed through the recording user allocator, so the requested
    // byte counts themselves are compared.
    for &(items, size) in &grid[..7] {
        let run = |lib: &'static Library| {
            capture(|| unsafe {
                allocs_reset();
                let pp = reader_2(lib);
                allocs_reset();
                let p = png_zalloc(lib, pp, items, size);
                let null = p.is_null();
                png_zfree(lib, pp, p);
                let ev = allocs_take();
                let mut q = pp;
                png_destroy_read_struct(lib, &mut q, ptr::null_mut(), ptr::null_mut());
                (null, ev)
            })
        };
        let a = run(&l.c);
        let b = run(&l.rs);
        a.assert_eq(&b, &format!("png_zalloc traced (items={items}, size={size})"));
    }
}

// ===========================================================================
// Row 67 - low-level chunk reading
// ===========================================================================

/// The chunk bodies used to drive `png_handle_chunk`.  Positioned after a
/// truecolour IHDR (4x3, 8-bit, RGB) unless the label says otherwise.
fn ancillary_cases() -> Vec<(String, Vec<Vec<u8>>)> {
    let mut v: Vec<(String, Vec<Vec<u8>>)> = Vec::new();
    let mut push = |name: &str, chunks: Vec<Vec<u8>>| v.push((name.to_string(), chunks));

    let mut cHRM = Vec::new();
    for x in [31270u32, 32900, 64000, 33000, 30000, 60000, 15000, 6000] {
        cHRM.extend_from_slice(&(x * 10).to_be_bytes());
    }

    push("gAMA", vec![chunk(b"gAMA", &45455u32.to_be_bytes())]);
    push("gAMA-dup", vec![
        chunk(b"gAMA", &45455u32.to_be_bytes()),
        chunk(b"gAMA", &45455u32.to_be_bytes()),
    ]);
    push("gAMA-short", vec![chunk(b"gAMA", &[0, 0, 1])]);
    push("gAMA-long", vec![chunk(b"gAMA", &[0, 0, 0, 1, 2])]);
    push("cHRM", vec![chunk(b"cHRM", &cHRM)]);
    push("sRGB", vec![chunk(b"sRGB", &[0])]);
    push("sRGB-bad-intent", vec![chunk(b"sRGB", &[9])]);
    push("sBIT", vec![chunk(b"sBIT", &[8, 8, 8])]);
    push("sBIT-wrong-size", vec![chunk(b"sBIT", &[8, 8, 8, 8])]);
    push("cICP", vec![chunk(b"cICP", &[1, 13, 0, 1])]);
    push("cLLI", vec![chunk(b"cLLI", &[0, 0x98, 0x96, 0x80, 0, 0x0f, 0x42, 0x40])]);
    push("mDCV", vec![chunk(b"mDCV", &vec![0x10u8; 24])]);
    push("eXIf", vec![chunk(b"eXIf", b"II*\0")]);
    push("tIME", vec![chunk(b"tIME", &[0x07, 0xe8, 1, 2, 3, 4, 5])]);
    push("pHYs", vec![chunk(b"pHYs", &[0, 0, 0, 100, 0, 0, 0, 100, 1])]);
    push("oFFs", vec![chunk(b"oFFs", &[0, 0, 0, 5, 0, 0, 0, 6, 0])]);
    push("bKGD", vec![chunk(b"bKGD", &[0, 1, 0, 2, 0, 3])]);
    push("tRNS", vec![chunk(b"tRNS", &[0, 1, 0, 2, 0, 3])]);
    push("tEXt", vec![chunk(b"tEXt", b"Title\0some text")]);
    push("tEXt-empty", vec![chunk(b"tEXt", b"Title\0")]);
    push("tEXt-no-nul", vec![chunk(b"tEXt", b"Title")]);
    {
        let mut d = b"Comment\0".to_vec();
        d.push(0); // compression method
        d.extend_from_slice(&zlib_stored(b"a compressed comment"));
        push("zTXt", vec![chunk(b"zTXt", &d)]);
    }
    {
        let mut d = b"Comment\0".to_vec();
        d.push(0); // compression flag: uncompressed
        d.push(0); // compression method
        d.extend_from_slice(b"en\0");
        d.extend_from_slice(b"Comment\0");
        d.extend_from_slice(b"international text");
        push("iTXt-plain", vec![chunk(b"iTXt", &d)]);
    }
    {
        let mut d = b"Comment\0".to_vec();
        d.push(1); // compressed
        d.push(0);
        d.extend_from_slice(b"en\0");
        d.extend_from_slice(b"Comment\0");
        d.extend_from_slice(&zlib_stored(b"international compressed text"));
        push("iTXt-compressed", vec![chunk(b"iTXt", &d)]);
    }
    {
        let mut d = b"sugg\0".to_vec();
        d.push(8); // sample depth
        for i in 0..3u8 {
            d.extend_from_slice(&[i, 2 * i, 3 * i, 255, 0, 1]);
        }
        push("sPLT", vec![chunk(b"sPLT", &d)]);
    }
    {
        let mut d = b"calib\0".to_vec();
        d.extend_from_slice(&(-100i32).to_be_bytes());
        d.extend_from_slice(&100i32.to_be_bytes());
        d.push(0); // equation type
        d.push(2); // nparams
        d.extend_from_slice(b"m\0");
        d.extend_from_slice(b"1.0\0");
        d.extend_from_slice(b"2.0");
        push("pCAL", vec![chunk(b"pCAL", &d)]);
    }
    {
        let mut d = vec![1u8];
        d.extend_from_slice(b"1.5\0");
        d.extend_from_slice(b"2.5");
        push("sCAL", vec![chunk(b"sCAL", &d)]);
    }
    {
        // iCCP: name, compression method, zlib-compressed profile.
        let spec = IccSpec::default();
        let mut profile = spec.build();
        profile.truncate(132);
        let mut d = b"prof\0".to_vec();
        d.push(0);
        d.extend_from_slice(&zlib_stored(&profile));
        push("iCCP", vec![chunk(b"iCCP", &d)]);
    }
    push("acTL(APNG)", vec![chunk(b"acTL", &[0, 0, 0, 2, 0, 0, 0, 0])]);
    push("unknown-ancillary", vec![chunk(b"uNKn", b"payload")]);
    push("unknown-ancillary-empty", vec![chunk(b"uNKn", b"")]);
    push("unknown-critical", vec![chunk(b"UnKn", b"payload")]);
    push(
        "unknown-then-known",
        vec![chunk(b"uNKn", b"payload"), chunk(b"gAMA", &45455u32.to_be_bytes())],
    );
    // hIST needs a PLTE first (PNG_HAVE_PLTE).
    {
        let mut plte = Vec::new();
        for i in 0..4u8 {
            plte.extend_from_slice(&[i, 2 * i, 3 * i]);
        }
        let mut hist = Vec::new();
        for i in 0..4u16 {
            hist.extend_from_slice(&(i * 100).to_be_bytes());
        }
        push("PLTE+hIST", vec![chunk(b"PLTE", &plte), chunk(b"hIST", &hist)]);
        push("hIST-without-PLTE", vec![chunk(b"hIST", &hist)]);
    }
    // bad CRC on an ancillary chunk
    {
        let mut c = chunk(b"gAMA", &45455u32.to_be_bytes());
        let n = c.len();
        c[n - 1] ^= 0xff;
        push("gAMA-bad-crc", vec![c]);
    }
    v
}

#[derive(Debug, PartialEq)]
struct ChunkTrace {
    steps: Vec<(png_uint_32, png_uint_32, &'static str)>,
    valid: Vec<(&'static str, png_uint_32)>,
    io_state: png_uint_32,
    n_unknown: c_int,
    n_text: c_int,
    consumed: usize,
}

/// Drive `png_read_chunk_header` + `png_handle_chunk` over the first `n`
/// chunks of `stream` (which starts at the first chunk header; the 8 signature
/// bytes are declared through `png_set_sig_bytes`).
///
/// NOTE: no trailing IEND is fed, because the read_chunks table in
/// `pngrutil.c` gives IEND `pos_after = PNG_AFTER_IDAT`, so an IEND that is
/// not preceded by an IDAT is a *critical* "out of place" error that would
/// unwind before the trace could be collected.
unsafe fn drive_chunks(
    lib: &'static Library,
    stream: &[u8],
    n: usize,
    keep_global: c_int,
) -> ChunkTrace {
    src_set(stream);
    let pp = api::new_reader(lib);
    png_set_sig_bytes(lib, pp, 8);
    if keep_global >= 0 {
        png_set_keep_unknown_chunks(lib, pp, keep_global, ptr::null(), 0);
    }
    let ip = png_create_info_struct(lib, pp);

    let mut steps = Vec::new();
    for _ in 0..n {
        let len = x::png_read_chunk_header(lib, pp);
        let ctype = png_get_io_chunk_type(lib, pp);
        let code = x::png_handle_chunk(lib, pp, ip, len);
        steps.push((len, ctype, handled_name(code)));
    }

    let valid = ALL_INFO_BITS
        .iter()
        .map(|&(n, b)| (n, png_get_valid(lib, pp, ip, b)))
        .collect();
    let io_state = png_get_io_state(lib, pp);
    let mut up: *mut png_unknown_chunk = ptr::null_mut();
    let n_unknown = png_get_unknown_chunks(lib, pp, ip, &mut up);
    let mut tp: *mut png_text = ptr::null_mut();
    let mut tn: c_int = 0;
    let n_text = png_get_text(lib, pp, ip, &mut tp, &mut tn);
    let consumed = src_pos();

    let mut p = pp;
    let mut i = ip;
    png_destroy_read_struct(lib, &mut p, &mut i, ptr::null_mut());

    ChunkTrace { steps, valid, io_state, n_unknown, n_text, consumed }
}

#[test]
fn cfg67_handle_chunk_sweep() {
    let l = libs();
    // Non-vacuity counters: the sweep must actually reach every
    // png_handle_result_code and produce both silent and noisy cases.
    let mut codes: std::collections::BTreeSet<&'static str> = Default::default();
    let mut with_msgs = 0usize;
    let mut silent = 0usize;
    let mut valid_bits_set = 0usize;
    let mut saved_unknowns = 0usize;

    for (label, chunks) in ancillary_cases() {
        for keep in [-1i32, PNG_HANDLE_CHUNK_AS_DEFAULT, PNG_HANDLE_CHUNK_NEVER,
                     PNG_HANDLE_CHUNK_IF_SAFE, PNG_HANDLE_CHUNK_ALWAYS] {
            let mut stream: Vec<u8> = Vec::new();
            stream.extend(chunk(b"IHDR", &ihdr(4, 3, 8, PNG_COLOR_TYPE_RGB as u8)));
            for c in &chunks {
                stream.extend_from_slice(c);
            }
            let n = 1 + chunks.len();

            let run = |lib: &'static Library| {
                capture(|| unsafe { drive_chunks(lib, &stream, n, keep) })
            };
            let a = run(&l.c);
            let b = run(&l.rs);
            a.assert_eq(&b, &format!("png_handle_chunk[{label}] keep={keep}"));

            if a.log.is_empty() {
                silent += 1;
            } else {
                with_msgs += 1;
            }
            if let Some(t) = &a.out {
                for s in &t.steps {
                    codes.insert(s.2);
                }
                if t.valid.iter().any(|v| v.1 != 0) {
                    valid_bits_set += 1;
                }
                if t.n_unknown > 0 {
                    saved_unknowns += 1;
                }
            }
        }
    }

    assert!(with_msgs > 0 && silent > 0, "sweep produced only one kind of outcome");
    assert!(valid_bits_set > 0, "no case ever set a PNG_INFO_* bit");
    assert!(saved_unknowns > 0, "no case ever saved an unknown chunk");
    for want in ["handled_ok", "handled_saved", "handled_discarded", "handled_error"] {
        assert!(codes.contains(want), "sweep never produced {want} (saw {codes:?})");
    }
}

#[test]
fn cfg67_read_chunk_header() {
    let l = libs();
    // (label, raw bytes starting at the first chunk header)
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push((
        "ihdr+iend".into(),
        {
            let mut s = chunk(b"IHDR", &ihdr(4, 3, 8, PNG_COLOR_TYPE_RGB as u8));
            s.extend(chunk(b"IEND", &[]));
            s
        },
    ));
    // zero-length ancillary chunk
    cases.push(("zero-length".into(), {
        let mut s = chunk(b"IHDR", &ihdr(4, 3, 8, PNG_COLOR_TYPE_RGB as u8));
        s.extend(chunk(b"uNKn", &[]));
        s.extend(chunk(b"IEND", &[]));
        s
    }));
    // length exactly PNG_UINT_31_MAX (no data follows: the read callback
    // reports a short read identically on both sides)
    cases.push(("length-uint31max".into(), {
        let mut s = Vec::new();
        s.extend_from_slice(&PNG_UINT_31_MAX.to_be_bytes());
        s.extend_from_slice(b"uNKn");
        s
    }));
    // top bit set -> png_get_uint_31 rejects it
    cases.push(("length-0x80000000".into(), {
        let mut s = Vec::new();
        s.extend_from_slice(&0x8000_0000u32.to_be_bytes());
        s.extend_from_slice(b"uNKn");
        s
    }));
    cases.push(("length-0xffffffff".into(), {
        let mut s = Vec::new();
        s.extend_from_slice(&0xffff_ffffu32.to_be_bytes());
        s.extend_from_slice(b"uNKn");
        s
    }));
    // invalid chunk type bytes
    for bad in [b"1234", b"iEn\0", b"    ", b"AB\xffD"] {
        cases.push((
            format!("bad-type-{}", hex(bad)),
            {
                let mut s = Vec::new();
                s.extend_from_slice(&0u32.to_be_bytes());
                s.extend_from_slice(bad);
                s
            },
        ));
    }
    // truncated header
    cases.push(("truncated-header".into(), vec![0, 0, 0]));
    cases.push(("empty".into(), vec![]));

    for (label, stream) in cases {
        let run = |lib: &'static Library| {
            capture(|| unsafe {
                src_set(&stream);
                let pp = api::new_reader(lib);
                png_set_sig_bytes(lib, pp, 8);
                let mut out = Vec::new();
                for _ in 0..2 {
                    let len = x::png_read_chunk_header(lib, pp);
                    out.push((len, png_get_io_chunk_type(lib, pp), png_get_io_state(lib, pp)));
                }
                let mut p = pp;
                png_destroy_read_struct(lib, &mut p, ptr::null_mut(), ptr::null_mut());
                out
            })
        };
        let a = run(&l.c);
        let b = run(&l.rs);
        a.assert_eq(&b, &format!("png_read_chunk_header[{label}]"));
        // Anything after the source is exhausted must stop at the same offset.
        assert_eq!(
            {
                capture(|| unsafe {
                    src_set(&stream);
                    let pp = api::new_reader(&l.c);
                    png_set_sig_bytes(&l.c, pp, 8);
                    let _ = x::png_read_chunk_header(&l.c, pp);
                    src_pos()
                })
                .out
            },
            {
                capture(|| unsafe {
                    src_set(&stream);
                    let pp = api::new_reader(&l.rs);
                    png_set_sig_bytes(&l.rs, pp, 8);
                    let _ = x::png_read_chunk_header(&l.rs, pp);
                    src_pos()
                })
                .out
            },
            "png_read_chunk_header[{label}]: consumed byte count differs",
        );
    }
}

#[test]
fn cfg67_handle_unknown_keep() {
    let l = libs();
    // Drive png_handle_unknown directly with every `keep` value, for an
    // unknown ancillary chunk, an unknown critical chunk and a *known*
    // ancillary chunk forced through the unknown path.
    let bodies: [(&str, [u8; 4], Vec<u8>); 4] = [
        ("unknown-ancillary", *b"uNKn", b"payload".to_vec()),
        ("unknown-critical", *b"UnKn", b"payload".to_vec()),
        ("known-gAMA", *b"gAMA", 45455u32.to_be_bytes().to_vec()),
        ("unknown-empty", *b"uNKn", Vec::new()),
    ];

    let mut codes: std::collections::BTreeSet<&'static str> = Default::default();
    let mut saved = 0usize;
    for (label, name, body) in bodies {
        for keep in [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
        ] {
            for global in [-1i32, PNG_HANDLE_CHUNK_AS_DEFAULT, PNG_HANDLE_CHUNK_NEVER,
                           PNG_HANDLE_CHUNK_IF_SAFE, PNG_HANDLE_CHUNK_ALWAYS] {
                let mut stream = chunk(b"IHDR", &ihdr(4, 3, 8, PNG_COLOR_TYPE_RGB as u8));
                stream.extend(chunk(&name, &body));
                stream.extend(chunk(b"IEND", &[]));

                let run = |lib: &'static Library| {
                    capture(|| unsafe {
                        src_set(&stream);
                        let pp = api::new_reader(lib);
                        png_set_sig_bytes(lib, pp, 8);
                        if global >= 0 {
                            png_set_keep_unknown_chunks(lib, pp, global, ptr::null(), 0);
                        }
                        let ip = png_create_info_struct(lib, pp);

                        // IHDR through the normal path so that PNG_HAVE_IHDR is set.
                        let len = x::png_read_chunk_header(lib, pp);
                        let c0 = x::png_handle_chunk(lib, pp, ip, len);

                        // the chunk under test, forced through png_handle_unknown
                        let len = x::png_read_chunk_header(lib, pp);
                        let ctype = png_get_io_chunk_type(lib, pp);
                        let c1 = x::png_handle_unknown(lib, pp, ip, len, keep);

                        let mut up: *mut png_unknown_chunk = ptr::null_mut();
                        let n_unknown = png_get_unknown_chunks(lib, pp, ip, &mut up);
                        // Copy out the saved chunk metadata (never the data
                        // pointer itself).
                        let mut meta = Vec::new();
                        for k in 0..n_unknown.max(0) {
                            let u = &*up.offset(k as isize);
                            meta.push((
                                u.name,
                                u.size,
                                u.location,
                                u.data.is_null(),
                                if u.data.is_null() {
                                    Vec::new()
                                } else {
                                    std::slice::from_raw_parts(u.data, u.size).to_vec()
                                },
                            ));
                        }
                        let handled = png_handle_as_unknown(lib, pp, name.as_ptr());

                        let mut p = pp;
                        let mut i = ip;
                        png_destroy_read_struct(lib, &mut p, &mut i, ptr::null_mut());
                        (
                            handled_name(c0),
                            len,
                            ctype,
                            handled_name(c1),
                            n_unknown,
                            meta,
                            handled,
                            src_pos(),
                        )
                    })
                };
                let a = run(&l.c);
                let b = run(&l.rs);
                a.assert_eq(
                    &b,
                    &format!("png_handle_unknown[{label}] keep={keep} global={global}"),
                );
                if let Some(t) = &a.out {
                    codes.insert(t.3);
                    if t.4 > 0 {
                        saved += 1;
                    }
                }
            }
        }
    }
    assert!(saved > 0, "png_handle_unknown never saved a chunk");
    for want in ["handled_discarded", "handled_saved"] {
        assert!(codes.contains(want), "never produced {want} (saw {codes:?})");
    }
}

//! Cross-cutting state / mode differential tests (CONFIGS.md rows M1..M8).
//!
//! Every row drives BOTH `.so`s through the identical call sequence inside a
//! single `diff(...)` closure and compares the whole trace byte for byte.
//! Rows M5/M6 do a complete write -> read round trip inside one closure (the
//! bytes produced by the library under test are fed straight back into the same
//! library), following the pattern of `tests/chunks.rs`.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::too_many_arguments)]

mod support;

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CString};
use support::core::*;
use support::pngbuild::{self, Builder, Chunk};
use support::*;

// ---------------------------------------------------------------------------
// constants / small helpers
// ---------------------------------------------------------------------------

fn ver() -> *const c_char {
    VER_STRING.as_ptr() as *const c_char
}

fn nullcb() -> Cb {
    std::ptr::null_mut()
}

/// The 20 `PNG_INFO_*` flags (values verified against `c_src/include/png.h`).
const INFO_FLAGS: &[(&str, u32)] = &[
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

/// All 28 chunk names libpng knows (`PNG_KNOWN_CHUNKS` in pngpriv.h) plus the
/// private chunks used by M7.  Each entry is a 5-byte, NUL-terminated name as
/// required by `png_set_keep_unknown_chunks` / `png_handle_as_unknown`.
const KNOWN_NAMES: &[&[u8; 4]] = &[
    b"IHDR", b"PLTE", b"IDAT", b"IEND", b"acTL", b"bKGD", b"cHRM", b"cICP", b"cLLI", b"eXIf",
    b"fcTL", b"fdAT", b"gAMA", b"hIST", b"iCCP", b"iTXt", b"mDCV", b"oFFs", b"pCAL", b"pHYs",
    b"sBIT", b"sCAL", b"sPLT", b"sRGB", b"tEXt", b"tIME", b"tRNS", b"zTXt",
];
/// Private (unknown) chunk names.  The third character must be upper case (the
/// reserved bit has to be clear) or `png_read_chunk_header` rejects the stream.
const PRIVATE_NAMES: &[&[u8; 4]] = &[b"prVt", b"teSt", b"abCd", b"xyZw"];

fn name5(n: &[u8; 4]) -> [u8; 5] {
    let mut v = [0u8; 5];
    v[..4].copy_from_slice(n);
    v
}

unsafe fn log_valid_all(c: &Core, png: Png, info: Info) {
    let mut s = String::new();
    for (n, f) in INFO_FLAGS {
        s.push_str(&format!("{n}:{} ", (c.get_valid)(png, info, *f)));
    }
    log(format!("valid[{}]", s.trim_end()));
}

/// Run `body` inside a longjmp landing pad, with the session serving `input`.
///
/// `body` must not own anything that needs dropping (a `png_error` longjmp
/// skips destructors); allocate outside and borrow.
fn drive(lib: &Lib, input: &[u8], body: &mut dyn FnMut(&Core)) -> Trace {
    session_reset(input.to_vec());
    let core = Core::new(lib);
    let rc = protected(|| body(&core));
    Trace {
        lines: take_log(),
        out: take_out(),
        rc,
    }
}

/// As `drive`, but appends the final allocation accounting.  The counters live
/// in the session, so they survive a longjmp out of `body`.
fn drive_acct(lib: &Lib, input: &[u8], body: &mut dyn FnMut(&Core)) -> Trace {
    let mut t = drive(lib, input, body);
    let (cnt, live) = with_session(|s| (s.malloc_count, s.live_allocs));
    t.lines.push(format!("malloc_count={cnt} live_allocs={live}"));
    t
}

/// `diff` plus the assertion that the (identical) trace really contains every
/// string in `need` — i.e. the configuration reached the code it was aimed at.
fn diff_needs(label: &str, mut run: impl FnMut(&Lib) -> Trace, need: &[&str]) {
    let missing = std::cell::RefCell::new(Vec::<String>::new());
    diff(label, |lib| {
        let t = run(lib);
        let joined = t.lines.join("\n");
        let mut m = missing.borrow_mut();
        m.clear();
        for n in need {
            if !joined.contains(n) {
                m.push((*n).to_string());
            }
        }
        t
    });
    let m = missing.borrow();
    assert!(
        m.is_empty(),
        "[{label}] trace is missing the expected text {m:?}"
    );
}

// ===========================================================================
// M1 — png_set_benign_errors(0/1) on read and write structs
// ===========================================================================
//
// This build has PNG_BENIGN_ERRORS_SUPPORTED and PNG_BENIGN_READ_ERRORS_SUPPORTED
// but *not* PNG_BENIGN_WRITE_ERRORS_SUPPORTED, and PNG_LIBPNG_BUILD_BASE_TYPE is
// BETA so PNG_RELEASE_BUILD == 0.  Therefore:
//
//   read struct  default flags: BENIGN_ERRORS_WARN            (app errors fatal)
//   write struct default flags: none                          (everything fatal)
//   png_set_benign_errors(1):   BENIGN | APP_WARNINGS | APP_ERRORS all warn
//   png_set_benign_errors(0):   all cleared (even on a read struct)
//
// The traces below must show exactly that in both libraries.

fn gray_png(extra: Option<Chunk>, w: u32, h: u32) -> Vec<u8> {
    let mut b = Builder::new(w, h, 8, 0);
    if let Some(ch) = extra {
        b = b.add_chunk(ch);
    }
    b.build_valid(0x11AA_0001)
}

/// A palette image whose indices exceed the PLTE length: at `png_read_end`
/// libpng reports this with `png_benign_error`.
fn pal_overflow_png(w: u32, h: u32, npal: usize) -> Vec<u8> {
    let mut raw = Vec::new();
    for y in 0..h {
        raw.push(0u8);
        for x in 0..w {
            raw.push(((x + y) % 8) as u8);
        }
    }
    let mut pal = Vec::new();
    for i in 0..npal {
        pal.extend_from_slice(&[(i * 17) as u8, (i * 31) as u8, (i * 7) as u8]);
    }
    Builder::new(w, h, 8, 3).add(b"PLTE", pal).build(&raw, 0)
}

#[test]
fn m1_benign_errors() {
    // ---------------- read structs ----------------
    // (name, datastream, height, drive an app error via png_set_shift)
    let cases: Vec<(&str, Vec<u8>, u32, bool)> = vec![
        (
            "sBIT-bad-length",
            gray_png(Some(Chunk::new(b"sBIT", vec![4, 4])), 4, 3),
            3,
            false,
        ),
        (
            "sBIT-zero-value",
            gray_png(Some(Chunk::new(b"sBIT", vec![0])), 4, 3),
            3,
            false,
        ),
        (
            "gAMA-out-of-range",
            gray_png(Some(Chunk::new(b"gAMA", vec![0xff, 0xff, 0xff, 0xff])), 4, 3),
            3,
            false,
        ),
        (
            "sBIT-bad-length-4",
            gray_png(Some(Chunk::new(b"sBIT", vec![4, 4, 4, 4])), 4, 3),
            3,
            false,
        ),
        (
            "tRNS-bad-length",
            gray_png(Some(Chunk::new(b"tRNS", vec![1, 2, 3, 4, 5])), 4, 3),
            3,
            false,
        ),
        (
            "tRNS-truncated",
            gray_png(Some(Chunk::new(b"tRNS", vec![7])), 4, 3),
            3,
            false,
        ),
        (
            "tEXt-bad-crc",
            gray_png(
                Some(Chunk::new(b"tEXt", b"Key\0value".to_vec()).bad_crc()),
                4,
                3,
            ),
            3,
            false,
        ),
        (
            "gAMA-bad-crc",
            gray_png(
                Some(Chunk::new(b"gAMA", vec![0x00, 0x00, 0xb1, 0x8f]).bad_crc()),
                4,
                3,
            ),
            3,
            false,
        ),
        ("app-error-shift", gray_png(None, 4, 3), 3, true),
        ("palette-index-overflow", pal_overflow_png(4, 3, 2), 3, false),
        ("palette-index-overflow-8", pal_overflow_png(9, 4, 1), 4, false),
    ];
    for (name, data, h, shift) in &cases {
        for &v in &[-1i32, 0, 1] {
            for &pre in &[true, false] {
                let label = format!("M1 read {name} benign={v} pre={pre}");
                diff(&label, |lib| {
                    let mut buf = [0u8; 64];
                    with_read(lib, data, &mut |c, p, info| unsafe {
                        if pre && v >= 0 {
                            (c.set_benign_errors)(p, v);
                            log(format!("set_benign_errors({v}) before read_info"));
                        }
                        (c.read_info)(p, info);
                        log("read_info returned");
                        if !pre && v >= 0 {
                            (c.set_benign_errors)(p, v);
                            log(format!("set_benign_errors({v}) after read_info"));
                        }
                        if *shift {
                            let sb = PngColor8::default(); /* all zero -> invalid */
                            (c.set_shift)(p, &sb as *const PngColor8 as *const u8);
                            log("png_set_shift(invalid) returned");
                        }
                        let rb = (c.get_rowbytes)(p, info).min(buf.len());
                        log(format!("rowbytes={rb}"));
                        for y in 0..*h {
                            (c.read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                            log(format!("row{y}={}", hex(&buf[..rb])));
                        }
                        (c.read_end)(p, info);
                        log("read_end returned");
                        log_valid_all(c, p, info);
                        log(format!("palette_max={}", (c.get_palette_max)(p, info)));
                    })
                });
            }
        }
    }

    // ---------------- write structs ----------------
    // kind 0: palette index > num_palette      -> png_benign_error in write_end
    // kind 1: png_set_filler before write_info -> png_app_error (png_ptr fields
    //         are still zero at that point, so libpng sees "low bit depth gray")
    // kind 2: png_set_shift with zero bits     -> png_app_error
    // kind 3: tRNS on an RGBA image            -> png_app_warning in write_info
    // kind 4: png_set_filler after write_info  -> png_app_error, this time on the
    //         real colour type ("inappropriate color type")
    let pal2: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
    for kind in 0..5u32 {
        let (w, h, depth, color) = match kind {
            0 | 1 | 4 => (4u32, 3u32, 8i32, PNG_COLOR_TYPE_PALETTE),
            2 => (4, 3, 8, PNG_COLOR_TYPE_GRAY),
            _ => (4, 3, 8, PNG_COLOR_TYPE_RGB_ALPHA),
        };
        let stride = pngbuild::rowbytes(color as u8, depth as u8, w);
        let rows: Vec<u8> = {
            let mut r = Rng::new(0x1100_0000 + kind as u64);
            (0..stride * h as usize)
                .map(|i| {
                    if color == PNG_COLOR_TYPE_PALETTE {
                        (i % 8) as u8 /* indices 0..7, PLTE has 2 entries */
                    } else {
                        r.byte()
                    }
                })
                .collect()
        };
        for &v in &[-1i32, 0, 1] {
            for &pre in &[true, false] {
                let label = format!("M1 write kind={kind} benign={v} pre={pre}");
                diff(&label, |lib| {
                    with_write(lib, &mut |c, p, info| unsafe {
                        (c.set_IHDR)(
                            p,
                            info,
                            w,
                            h,
                            depth,
                            color,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        if color == PNG_COLOR_TYPE_PALETTE {
                            (c.set_PLTE)(p, info, pal2.as_ptr(), 2);
                        }
                        if kind == 3 {
                            let k = PngColor16 {
                                gray: 1,
                                ..Default::default()
                            };
                            (c.set_tRNS)(
                                p,
                                info,
                                std::ptr::null(),
                                0,
                                &k as *const PngColor16 as *const u8,
                            );
                            log(format!("tRNS valid={}", (c.get_valid)(p, info, PNG_INFO_tRNS)));
                        }
                        if pre && v >= 0 {
                            (c.set_benign_errors)(p, v);
                            log(format!("set_benign_errors({v}) before write_info"));
                        }
                        if kind == 1 {
                            (c.set_filler)(p, 0, PNG_FILLER_AFTER);
                            log("png_set_filler returned");
                        }
                        if kind == 2 {
                            let sb = PngColor8::default();
                            (c.set_shift)(p, &sb as *const PngColor8 as *const u8);
                            log("png_set_shift(invalid) returned");
                        }
                        (c.write_info)(p, info);
                        log("write_info returned");
                        if !pre && v >= 0 {
                            (c.set_benign_errors)(p, v);
                            log(format!("set_benign_errors({v}) after write_info"));
                        }
                        if kind == 4 {
                            (c.set_filler)(p, 0, PNG_FILLER_BEFORE);
                            log("png_set_filler (post write_info) returned");
                        }
                        for y in 0..h as usize {
                            (c.write_row)(p, rows.as_ptr().add(y * stride));
                        }
                        log("rows written");
                        (c.write_end)(p, info);
                        log("write_end returned");
                        log(format!("palette_max={}", (c.get_palette_max)(p, info)));
                    })
                });
            }
        }
    }
}

// ===========================================================================
// M2 — png_set_error_fn / png_get_error_ptr
// ===========================================================================
//
// With a NULL error_fn libpng uses png_default_error, which prints to stderr and
// then calls png_longjmp -> our th_longjmp pad, so the default path is safe
// here.  A NULL warning_fn selects png_default_warning (stderr only).  Nothing
// but the *observable* behaviour (whether the harness callback fired, and
// whether control came back through the pad) is compared.

#[test]
fn m2_error_fn() {
    let tag_a: [u8; 8] = *b"ERRPTRA\0";
    let tag_b: [u8; 8] = *b"ERRPTRB\0";
    let good = Builder::new(4, 3, 8, 0).build_valid(0x1200_0001);
    let badcrc = Builder::new(4, 3, 8, 0)
        .add_chunk(Chunk::new(b"tEXt", b"K\0v".to_vec()).bad_crc())
        .build_valid(0x1200_0002);
    // bit depth 3 is illegal: png_read_info issues a real png_error
    let badihdr = Builder::new(4, 3, 3, 0).build_valid(0x1200_0003);
    let inputs: Vec<(&str, &Vec<u8>)> = vec![
        ("valid", &good),
        ("bad-crc-tEXt", &badcrc),
        ("bad-IHDR", &badihdr),
    ];

    for (iname, data) in &inputs {
        for &(ef, wf) in &[(false, false), (false, true), (true, false), (true, true)] {
            // reset: 0 = no second png_set_error_fn, 1 = re-point to another
            //        error_ptr, 2 = re-point to NULL
            for &reset in &[0u32, 1, 2] {
                for &do_err in &[false, true] {
                    let label =
                        format!("M2 read {iname} err_fn={ef} warn_fn={wf} reset={reset} err={do_err}");
                    diff(&label, |lib| {
                        let pa = tag_a.as_ptr() as *mut c_void;
                        let pb = tag_b.as_ptr() as *mut c_void;
                        let mut buf = [0u8; 32];
                        drive(lib, data, &mut |c| unsafe {
                            let p = (c.create_read)(
                                ver(),
                                pa,
                                if ef { cb_error as Cb } else { nullcb() },
                                if wf { cb_warning as Cb } else { nullcb() },
                            );
                            log(format!("create_read={}", (!p.is_null()) as u8));
                            if p.is_null() {
                                return;
                            }
                            (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
                            log(format!(
                                "error_ptr_is_a={}",
                                ((c.get_error_ptr)(p) == pa) as u8
                            ));
                            if reset != 0 {
                                (c.set_error_fn)(
                                    p,
                                    if reset == 1 { pb } else { std::ptr::null_mut() },
                                    if ef { cb_error as Cb } else { nullcb() },
                                    if wf { cb_warning as Cb } else { nullcb() },
                                );
                                let q = (c.get_error_ptr)(p);
                                log(format!(
                                    "after set_error_fn is_a={} is_b={} null={}",
                                    (q == pa) as u8,
                                    (q == pb) as u8,
                                    q.is_null() as u8
                                ));
                            }
                            (c.set_read_fn)(p, std::ptr::null_mut(), cb_read as Cb);
                            let i = (c.create_info)(p);
                            log(format!("create_info={}", (!i.is_null()) as u8));
                            (c.read_info)(p, i);
                            log("read_info returned");
                            (c.warning)(p, c"m2 explicit warning".as_ptr());
                            log("after explicit png_warning");
                            let rb = (c.get_rowbytes)(p, i).min(buf.len());
                            for y in 0..3 {
                                (c.read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                                log(format!("row{y}={}", hex(&buf[..rb])));
                            }
                            (c.read_end)(p, i);
                            log("read_end returned");
                            if do_err {
                                (c.error)(p, c"m2 explicit error".as_ptr());
                                log("NOT REACHED after png_error");
                            }
                            let mut pp = p;
                            let mut ii = i;
                            (c.destroy_read)(&mut pp, &mut ii, std::ptr::null_mut());
                            log("destroyed");
                        })
                    });
                }
            }
        }
    }

    // The same matrix on a write struct, driven into an app error (fatal in this
    // build) and into an explicit warning.
    for &(ef, wf) in &[(false, false), (false, true), (true, false), (true, true)] {
        for &do_err in &[false, true] {
            let label = format!("M2 write err_fn={ef} warn_fn={wf} err={do_err}");
            diff(&label, |lib| {
                let pa = tag_a.as_ptr() as *mut c_void;
                let pb = tag_b.as_ptr() as *mut c_void;
                let row = [0u8; 4];
                drive(lib, &[], &mut |c| unsafe {
                    let p = (c.create_write)(
                        ver(),
                        pa,
                        if ef { cb_error as Cb } else { nullcb() },
                        if wf { cb_warning as Cb } else { nullcb() },
                    );
                    log(format!("create_write={}", (!p.is_null()) as u8));
                    if p.is_null() {
                        return;
                    }
                    (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
                    (c.set_write_fn)(p, std::ptr::null_mut(), cb_write as Cb, cb_flush as Cb);
                    log(format!(
                        "error_ptr_is_a={}",
                        ((c.get_error_ptr)(p) == pa) as u8
                    ));
                    // re-point the error_ptr, keeping the same callbacks
                    (c.set_error_fn)(
                        p,
                        pb,
                        if ef { cb_error as Cb } else { nullcb() },
                        if wf { cb_warning as Cb } else { nullcb() },
                    );
                    let q = (c.get_error_ptr)(p);
                    log(format!(
                        "after set_error_fn is_a={} is_b={}",
                        (q == pa) as u8,
                        (q == pb) as u8
                    ));
                    let i = (c.create_info)(p);
                    (c.set_IHDR)(
                        p,
                        i,
                        4,
                        3,
                        8,
                        PNG_COLOR_TYPE_GRAY,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (c.write_info)(p, i);
                    (c.warning)(p, c"m2 explicit write warning".as_ptr());
                    log("after explicit png_warning");
                    for _ in 0..3 {
                        (c.write_row)(p, row.as_ptr());
                    }
                    (c.write_end)(p, i);
                    log("write_end returned");
                    if do_err {
                        // an app error: fatal in this (non-release) build
                        let sb = PngColor8::default();
                        (c.set_shift)(p, &sb as *const PngColor8 as *const u8);
                        log("NOT REACHED after app error");
                    }
                    let mut pp = p;
                    let mut ii = i;
                    (c.destroy_write)(&mut pp, &mut ii);
                    log("destroyed");
                })
            });
        }
    }
}

// ===========================================================================
// M3 — png_set_mem_fn / png_get_mem_ptr, allocation trace + failure injection
// ===========================================================================

#[test]
fn m3_mem_fn() {
    let mem_tag: [u8; 8] = *b"MEMPTR\0\0";
    let err_tag: [u8; 8] = *b"ERRPTR\0\0";
    let input = Builder::new(6, 5, 8, PNG_COLOR_TYPE_RGB as u8).build_valid(0x1300_0001);
    let ilinput = Builder::new(6, 5, 8, PNG_COLOR_TYPE_RGB as u8)
        .interlace(1)
        .build_valid(0x1300_0002);

    // ---- (a) png_set_mem_fn on an *existing* struct; every allocation size
    //          and order after that point is compared.  `trace_alloc` is
    //          switched on only once the png_struct exists (its size is an
    //          implementation detail).
    for &(w, h, ct, bd) in &[
        (6u32, 5u32, PNG_COLOR_TYPE_RGB, 8i32),
        (3, 2, PNG_COLOR_TYPE_GRAY, 4),
        (9, 4, PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        let stride = pngbuild::rowbytes(ct as u8, bd as u8, w);
        let rows: Vec<u8> = {
            let mut r = Rng::new(0x1301_0000 + ct as u64 * 64 + bd as u64);
            r.bytes(stride * h as usize)
        };
        let label = format!("M3 write set_mem_fn ct={ct} bd={bd}");
        diff_needs(&label, |lib| {
            drive_acct(lib, &[], &mut |c| unsafe {
                let p = (c.create_write)(ver(), std::ptr::null_mut(), cb_error as Cb, cb_warning as Cb);
                log(format!("create_write={}", (!p.is_null()) as u8));
                if p.is_null() {
                    return;
                }
                (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
                log(format!(
                    "mem_ptr_before_null={}",
                    (c.get_mem_ptr)(p).is_null() as u8
                ));
                (c.set_mem_fn)(
                    p,
                    mem_tag.as_ptr() as *mut c_void,
                    cb_malloc as Cb,
                    cb_free as Cb,
                );
                let mp = (c.get_mem_ptr)(p);
                log(format!(
                    "mem_ptr_eq={} mem_ptr_null={}",
                    (mp == mem_tag.as_ptr() as *mut c_void) as u8,
                    mp.is_null() as u8
                ));
                if !mp.is_null() {
                    log(format!(
                        "mem_ptr_data={}",
                        hex(std::slice::from_raw_parts(mp as *const u8, 8))
                    ));
                }
                with_session(|s| {
                    s.trace_alloc = true;
                    s.malloc_count = 0;
                    s.live_allocs = 0;
                });
                (c.set_write_fn)(p, std::ptr::null_mut(), cb_write as Cb, cb_flush as Cb);
                let i = (c.create_info)(p);
                log(format!("create_info={}", (!i.is_null()) as u8));
                (c.set_IHDR)(
                    p,
                    i,
                    w,
                    h,
                    bd,
                    ct,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (c.write_info)(p, i);
                for y in 0..h as usize {
                    (c.write_row)(p, rows.as_ptr().add(y * stride));
                }
                (c.write_end)(p, i);
                let mut pp = p;
                let mut ii = i;
                (c.destroy_write)(&mut pp, &mut ii);
                log("destroyed");
                with_session(|s| s.trace_alloc = false);
            })
        }, &["MALLOC(", "FREE(", "mem_ptr_eq=1"]);
    }
    for (iname, data, il) in [("plain", &input, 0), ("interlaced", &ilinput, 1)] {
        let label = format!("M3 read set_mem_fn {iname}");
        diff_needs(&label, |lib| {
            let mut buf = [0u8; 64];
            drive_acct(lib, data, &mut |c| unsafe {
                let p = (c.create_read)(ver(), std::ptr::null_mut(), cb_error as Cb, cb_warning as Cb);
                log(format!("create_read={}", (!p.is_null()) as u8));
                if p.is_null() {
                    return;
                }
                (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
                (c.set_mem_fn)(
                    p,
                    mem_tag.as_ptr() as *mut c_void,
                    cb_malloc as Cb,
                    cb_free as Cb,
                );
                let mp = (c.get_mem_ptr)(p);
                log(format!(
                    "mem_ptr_eq={}",
                    (mp == mem_tag.as_ptr() as *mut c_void) as u8
                ));
                with_session(|s| {
                    s.trace_alloc = true;
                    s.malloc_count = 0;
                    s.live_allocs = 0;
                });
                (c.set_read_fn)(p, std::ptr::null_mut(), cb_read as Cb);
                let i = (c.create_info)(p);
                (c.read_info)(p, i);
                let passes = (c.set_interlace_handling)(p);
                log(format!("passes={passes} interlace={il}"));
                (c.read_update_info)(p, i);
                let rb = (c.get_rowbytes)(p, i).min(buf.len());
                log(format!("rowbytes={rb}"));
                for _pass in 0..passes {
                    for y in 0..5 {
                        (c.read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                        log(format!("row{y}={}", hex(&buf[..rb])));
                    }
                }
                (c.read_end)(p, i);
                let mut pp = p;
                let mut ii = i;
                (c.destroy_read)(&mut pp, &mut ii, std::ptr::null_mut());
                log("destroyed");
                with_session(|s| s.trace_alloc = false);
            })
        }, &["MALLOC(", "FREE(", "mem_ptr_eq=1"]);
    }

    // ---- (b) failure injection: make each of the first 24 allocations fail in
    //          turn.  The struct itself is allocated through the user callbacks
    //          (png_create_*_struct_2) so k == 0 fails png_struct.  Sizes are
    //          not traced here (sizeof(png_struct) legitimately differs); what
    //          is compared is *which* allocation fails and with what message.
    // data for the "rich" sweep below (must outlive the closures)
    let rich_pal: Vec<u8> = {
        let mut r = Rng::new(0x1304_0001);
        r.bytes(16 * 3)
    };
    let rich_hist: Vec<u16> = (0..16).map(|i| (i * 71) as u16).collect();
    let rich_prof = icc_min();
    let rich_iccname = CString::new("m3 profile").unwrap();
    let rich_keys: Vec<CString> = vec![
        CString::new("Author").unwrap(),
        CString::new("Comment").unwrap(),
    ];
    let rich_vals: Vec<CString> = vec![
        CString::new("a plain tEXt value").unwrap(),
        CString::new("a compressed zTXt value, repeated repeated repeated").unwrap(),
    ];
    let rich_texts: Vec<PngText> = (0..2)
        .map(|i| PngText {
            compression: if i == 0 {
                PNG_TEXT_COMPRESSION_NONE
            } else {
                PNG_TEXT_COMPRESSION_zTXt
            },
            key: rich_keys[i].as_ptr() as *mut c_char,
            text: rich_vals[i].as_ptr() as *mut c_char,
            ..Default::default()
        })
        .collect();
    let rich_unk_data: Vec<u8> = vec![9, 8, 7, 6, 5];
    let rich_unk: Vec<PngUnknownChunk> = vec![PngUnknownChunk {
        name: name5(b"prVt"),
        data: rich_unk_data.as_ptr() as *mut u8,
        size: rich_unk_data.len(),
        location: 0x01,
    }];

    let failed = Cell::new(0usize);
    for k in 0..25usize {
        let hit = Cell::new(false);
        diff(&format!("M3 read malloc_limit={k}"), |lib| {
            let mut buf = [0u8; 64];
            let t = drive_acct(lib, &input, &mut |c| unsafe {
                with_session(|s| s.malloc_limit = Some(k));
                let p = (c.create_read_2)(
                    ver(),
                    err_tag.as_ptr() as *mut c_void,
                    cb_error as Cb,
                    cb_warning as Cb,
                    mem_tag.as_ptr() as *mut c_void,
                    cb_malloc as Cb,
                    cb_free as Cb,
                );
                log(format!("create_read_2={}", (!p.is_null()) as u8));
                if p.is_null() {
                    return;
                }
                (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
                (c.set_read_fn)(p, std::ptr::null_mut(), cb_read as Cb);
                let i = (c.create_info)(p);
                log(format!("create_info={}", (!i.is_null()) as u8));
                if i.is_null() {
                    return;
                }
                (c.read_info)(p, i);
                log("read_info returned");
                let rb = (c.get_rowbytes)(p, i).min(buf.len());
                log(format!("rowbytes={rb}"));
                for y in 0..5 {
                    (c.read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                    log(format!("row{y}={}", hex(&buf[..rb])));
                }
                (c.read_end)(p, i);
                log("read_end returned");
                let mut pp = p;
                let mut ii = i;
                (c.destroy_read)(&mut pp, &mut ii, std::ptr::null_mut());
                log("destroyed");
            });
            if t.rc != 0
                || t.lines.iter().any(|l| {
                    l.contains("create_read_2=0") || l.contains("Out of memory") || l == "ERROR"
                })
            {
                hit.set(true);
            }
            t
        });
        diff(&format!("M3 write malloc_limit={k}"), |lib| {
            let row = [0x5au8; 18];
            let t = drive_acct(lib, &[], &mut |c| unsafe {
                with_session(|s| s.malloc_limit = Some(k));
                let p = (c.create_write_2)(
                    ver(),
                    err_tag.as_ptr() as *mut c_void,
                    cb_error as Cb,
                    cb_warning as Cb,
                    mem_tag.as_ptr() as *mut c_void,
                    cb_malloc as Cb,
                    cb_free as Cb,
                );
                log(format!("create_write_2={}", (!p.is_null()) as u8));
                if p.is_null() {
                    return;
                }
                (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
                (c.set_write_fn)(p, std::ptr::null_mut(), cb_write as Cb, cb_flush as Cb);
                let i = (c.create_info)(p);
                log(format!("create_info={}", (!i.is_null()) as u8));
                if i.is_null() {
                    return;
                }
                (c.set_IHDR)(
                    p,
                    i,
                    6,
                    5,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (c.write_info)(p, i);
                log("write_info returned");
                for _ in 0..5 {
                    (c.write_row)(p, row.as_ptr());
                }
                (c.write_end)(p, i);
                log("write_end returned");
                let mut pp = p;
                let mut ii = i;
                (c.destroy_write)(&mut pp, &mut ii);
                log("destroyed");
            });
            if t.rc != 0
                || t.lines.iter().any(|l| {
                    l.contains("create_write_2=0") || l.contains("Out of memory")
                })
            {
                hit.set(true);
            }
            t
        });
        // A much richer pipeline (interlaced palette image carrying zTXt, iCCP
        // and unknown chunks) so that the higher limits also hit a failure.
        diff(&format!("M3 rich write malloc_limit={k}"), |lib| {
            let row = [0x3cu8; 12];
            let t = drive_acct(lib, &[], &mut |c| unsafe {
                with_session(|s| s.malloc_limit = Some(k));
                let p = (c.create_write_2)(
                    ver(),
                    err_tag.as_ptr() as *mut c_void,
                    cb_error as Cb,
                    cb_warning as Cb,
                    mem_tag.as_ptr() as *mut c_void,
                    cb_malloc as Cb,
                    cb_free as Cb,
                );
                log(format!("create_write_2={}", (!p.is_null()) as u8));
                if p.is_null() {
                    return;
                }
                (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
                (c.set_write_fn)(p, std::ptr::null_mut(), cb_write as Cb, cb_flush as Cb);
                let i = (c.create_info)(p);
                log(format!("create_info={}", (!i.is_null()) as u8));
                if i.is_null() {
                    return;
                }
                (c.set_IHDR)(
                    p,
                    i,
                    9,
                    6,
                    4,
                    PNG_COLOR_TYPE_PALETTE,
                    PNG_INTERLACE_ADAM7,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (c.set_PLTE)(p, i, rich_pal.as_ptr(), 16);
                (c.set_hIST)(p, i, rich_hist.as_ptr());
                (c.set_iCCP)(
                    p,
                    i,
                    rich_iccname.as_ptr(),
                    PNG_COMPRESSION_TYPE_BASE,
                    rich_prof.as_ptr(),
                    rich_prof.len() as u32,
                );
                (c.set_text)(p, i, rich_texts.as_ptr() as *const c_void, 2);
                (c.set_unknown_chunks)(p, i, rich_unk.as_ptr() as *const c_void, 1);
                (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
                log("setters done");
                (c.write_info)(p, i);
                log("write_info returned");
                let passes = (c.set_interlace_handling)(p);
                for _pass in 0..passes {
                    for _ in 0..6 {
                        (c.write_row)(p, row.as_ptr());
                    }
                }
                (c.write_end)(p, i);
                log("write_end returned");
                let mut pp = p;
                let mut ii = i;
                (c.destroy_write)(&mut pp, &mut ii);
                log("destroyed");
            });
            if t.rc != 0
                || t.lines
                    .iter()
                    .any(|l| l.contains("create_write_2=0") || l.contains("Out of memory"))
            {
                hit.set(true);
            }
            t
        });
        if hit.get() {
            failed.set(failed.get() + 1);
        }
    }
    // 24 of the 25 limits really do inject a failure somewhere in the three
    // pipelines (the plain ones need a bit more than ten allocations, the rich
    // one many more); k == 24 is the one that everything survives.
    assert!(
        failed.get() >= 24,
        "M3: only {} of 25 malloc_limit values actually made an allocation fail",
        failed.get()
    );
}

// ===========================================================================
// M4 — png_set_longjmp_fn
// ===========================================================================
//
// The internal error "Libpng jmp_buf still allocated" needs
// png_struct::jmp_buf_size == 0 together with jmp_buf_ptr != &jmp_buf_local.
// png_create_png_struct resets jmp_buf_ptr to NULL (and jmp_buf_size to 0) as
// its last act, and png_free_jmpbuf does the same, so that state is not
// reachable through the public API and is therefore not exercised here; every
// other branch of png_set_longjmp_fn is.

#[test]
fn m4_longjmp_fn() {
    let js = shim().jmp_buf_size;
    // (name, first size, second size)
    let cfgs: Vec<(&str, usize, usize)> = vec![
        ("equal,equal", js, js),
        ("equal,smaller", js, js - 8),
        ("equal,larger", js, js + 4096),
        ("smaller,smaller", js - 8, js - 8),
        ("smaller,equal", js - 8, js),
        ("smaller,larger", js - 8, js + 4096),
        ("larger,larger", js + 4096, js + 4096),
        ("larger,equal", js + 4096, js),
        ("larger,larger2", js + 4096, js + 8192),
        ("zero,equal", 0, js),
    ];
    let data = Builder::new(4, 3, 8, 0).build_valid(0x1400_0001);
    for (name, s1, s2) in &cfgs {
        // After call #1 libpng records size 0 (its internal buffer) whenever
        // s1 <= sizeof(jmp_buf), otherwise s1.  Call #2 compares the *effective*
        // size against s2 and warns when they differ.
        let eff1 = if *s1 <= js { js } else { *s1 };
        let need2: &[&str] = if eff1 != *s2 {
            &["WARNING(Application jmp_buf size changed)", "#2"]
        } else {
            &["#2"]
        };
        for &do_err in &[false, true] {
            for &is_read in &[true, false] {
                let label = format!(
                    "M4 {} {name} err={do_err}",
                    if is_read { "read" } else { "write" }
                );
                diff_needs(&label, |lib| {
                    let row = [0u8; 8];
                    let mut buf = [0u8; 32];
                    drive(lib, &data, &mut |c| unsafe {
                        let p = if is_read {
                            (c.create_read)(ver(), std::ptr::null_mut(), cb_error as Cb, cb_warning as Cb)
                        } else {
                            (c.create_write)(ver(), std::ptr::null_mut(), cb_error as Cb, cb_warning as Cb)
                        };
                        log(format!("create={}", (!p.is_null()) as u8));
                        if p.is_null() {
                            return;
                        }
                        let r1 = (c.set_longjmp)(p, shim().longjmp_ptr, *s1);
                        log(format!("set_longjmp#1({s1}) null={}", r1.is_null() as u8));
                        let r2 = (c.set_longjmp)(p, shim().longjmp_ptr, *s2);
                        log(format!(
                            "set_longjmp#2({s2}) null={} same_as_1={}",
                            r2.is_null() as u8,
                            (r1 == r2) as u8
                        ));
                        // a third call with the first size again
                        let r3 = (c.set_longjmp)(p, shim().longjmp_ptr, *s1);
                        log(format!(
                            "set_longjmp#3({s1}) null={} same_as_1={}",
                            r3.is_null() as u8,
                            (r1 == r3) as u8
                        ));
                        let i = (c.create_info)(p);
                        log(format!("create_info={}", (!i.is_null()) as u8));
                        if is_read {
                            (c.set_read_fn)(p, std::ptr::null_mut(), cb_read as Cb);
                            (c.read_info)(p, i);
                            log("read_info returned");
                            let rb = (c.get_rowbytes)(p, i).min(buf.len());
                            for y in 0..3 {
                                (c.read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                                log(format!("row{y}={}", hex(&buf[..rb])));
                            }
                            (c.read_end)(p, i);
                        } else {
                            (c.set_write_fn)(p, std::ptr::null_mut(), cb_write as Cb, cb_flush as Cb);
                            (c.set_IHDR)(
                                p,
                                i,
                                4,
                                3,
                                8,
                                PNG_COLOR_TYPE_GRAY,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            (c.write_info)(p, i);
                            for _ in 0..3 {
                                (c.write_row)(p, row.as_ptr());
                            }
                            (c.write_end)(p, i);
                        }
                        log("pipeline done");
                        if do_err {
                            (c.error)(p, c"m4 forced error".as_ptr());
                            log("NOT REACHED after png_error");
                        }
                        let mut pp = p;
                        let mut ii = i;
                        if is_read {
                            (c.destroy_read)(&mut pp, &mut ii, std::ptr::null_mut());
                        } else {
                            (c.destroy_write)(&mut pp, &mut ii);
                        }
                        log("destroyed");
                    })
                }, need2);
            }
        }
    }

    // The jmp_buf allocation itself failing (only possible for a size larger
    // than libpng's internal buffer): png_malloc_warn warns and set_longjmp
    // returns NULL.  No error may be raised while no pad is installed, so the
    // limit is lifted and a second call installs the internal buffer.
    for &is_read in &[true, false] {
        let label = format!("M4 {} jmp_buf alloc fails", if is_read { "read" } else { "write" });
        diff_needs(&label, |lib| {
            drive_acct(lib, &[], &mut |c| unsafe {
                let p = if is_read {
                    (c.create_read_2)(
                        ver(),
                        std::ptr::null_mut(),
                        cb_error as Cb,
                        cb_warning as Cb,
                        std::ptr::null_mut(),
                        cb_malloc as Cb,
                        cb_free as Cb,
                    )
                } else {
                    (c.create_write_2)(
                        ver(),
                        std::ptr::null_mut(),
                        cb_error as Cb,
                        cb_warning as Cb,
                        std::ptr::null_mut(),
                        cb_malloc as Cb,
                        cb_free as Cb,
                    )
                };
                log(format!("create2={}", (!p.is_null()) as u8));
                if p.is_null() {
                    return;
                }
                with_session(|s| {
                    let n = s.malloc_count;
                    s.malloc_limit = Some(n);
                });
                let r = (c.set_longjmp)(p, shim().longjmp_ptr, js + 4096);
                log(format!("set_longjmp(large) null={}", r.is_null() as u8));
                with_session(|s| s.malloc_limit = None);
                let r2 = (c.set_longjmp)(p, shim().longjmp_ptr, js);
                log(format!("set_longjmp(equal) null={}", r2.is_null() as u8));
                let r3 = (c.set_longjmp)(p, shim().longjmp_ptr, js + 4096);
                log(format!(
                    "set_longjmp(large again) null={} same_as_2={}",
                    r3.is_null() as u8,
                    (r2 == r3) as u8
                ));
                // the pad is installed now, so an error is safe
                (c.error)(p, c"m4 after failed jmp_buf alloc".as_ptr());
                log("NOT REACHED");
            })
        }, &[
            "WARNING(Out of memory)",
            "set_longjmp(large) null=1",
            "set_longjmp(equal) null=0",
            "ERROR(m4 after failed jmp_buf alloc)",
        ]);
    }
}

// ===========================================================================
// M5 — write -> read round trip for every legal (colour type, bit depth)
// ===========================================================================

const LEGAL: &[(c_int, c_int)] = &[
    (PNG_COLOR_TYPE_GRAY, 1),
    (PNG_COLOR_TYPE_GRAY, 2),
    (PNG_COLOR_TYPE_GRAY, 4),
    (PNG_COLOR_TYPE_GRAY, 8),
    (PNG_COLOR_TYPE_GRAY, 16),
    (PNG_COLOR_TYPE_RGB, 8),
    (PNG_COLOR_TYPE_RGB, 16),
    (PNG_COLOR_TYPE_PALETTE, 1),
    (PNG_COLOR_TYPE_PALETTE, 2),
    (PNG_COLOR_TYPE_PALETTE, 4),
    (PNG_COLOR_TYPE_PALETTE, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
];

const SHAPES: &[(u32, u32)] = &[(1, 1), (2, 1), (3, 2), (7, 3), (8, 8), (17, 5), (33, 9)];

#[test]
fn m5_roundtrip_identity() {
    for &(color, depth) in LEGAL {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h, s) in &SHAPES
                .iter()
                .flat_map(|(w, h)| (0..2u64).map(move |s| (*w, *h, s)))
                .collect::<Vec<_>>()
            {
                let seed = 0x1500_0000
                    + color as u64 * 0x10000
                    + depth as u64 * 0x400
                    + il as u64 * 0x100
                    + w as u64 * 8
                    + h as u64
                    + s * 0x100_0000;
                let npal: c_int = if color == PNG_COLOR_TYPE_PALETTE {
                    (1i32 << depth).min(256)
                } else {
                    0
                };
                let pal: Vec<u8> = {
                    let mut r = Rng::new(seed ^ 0x5A5A_5A5A);
                    r.bytes(npal as usize * 3)
                };
                let stride = pngbuild::rowbytes(color as u8, depth as u8, w);
                let pixbits = pngbuild::pixel_bits(color as u8, depth as u8) as usize;
                let rem = (pixbits * w as usize) % 8;
                // MSB-first packing: the unused low bits of the last byte of a
                // row are not part of the image and must be masked off.
                let lastmask: u8 = if rem == 0 {
                    0xff
                } else {
                    (0xffu16 << (8 - rem)) as u8
                };
                let mut src: Vec<u8> = {
                    let mut r = Rng::new(seed);
                    r.bytes(stride * h as usize)
                };
                for y in 0..h as usize {
                    let last = y * stride + stride - 1;
                    src[last] &= lastmask;
                }
                let label = format!(
                    "M5 ct={color} bd={depth} il={il} {w}x{h}[{s}] stride={stride} mask={lastmask:02x}"
                );
                let ok = Cell::new(true);
                diff(&label, |lib| {
                    // ---- write ----
                    let mut wptrs: Vec<*mut u8> = (0..h as usize)
                        .map(|y| unsafe { src.as_ptr().add(y * stride) as *mut u8 })
                        .collect();
                    let t1 = with_write(lib, &mut |c, p, info| unsafe {
                        (c.set_IHDR)(
                            p,
                            info,
                            w,
                            h,
                            depth,
                            color,
                            il,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        if npal > 0 {
                            (c.set_PLTE)(p, info, pal.as_ptr(), npal);
                        }
                        log(format!(
                            "W stride={stride} mask={lastmask:02x} rowbytes={} channels={}",
                            (c.get_rowbytes)(p, info),
                            (c.get_channels)(p, info)
                        ));
                        for y in 0..h as usize {
                            log(format!("Wrow{y}={}", hex(&src[y * stride..(y + 1) * stride])));
                        }
                        (c.write_info)(p, info);
                        (c.write_image)(p, wptrs.as_mut_ptr());
                        (c.write_end)(p, info);
                    });
                    // ---- read the bytes just produced ----
                    let produced = t1.out.clone();
                    let slack = 8usize;
                    let mut dst = vec![0u8; (stride + slack) * h as usize];
                    let mut rptrs: Vec<*mut u8> = (0..h as usize)
                        .map(|y| unsafe { dst.as_mut_ptr().add(y * (stride + slack)) })
                        .collect();
                    let t2 = with_read(lib, &produced, &mut |c, p, info| unsafe {
                        (c.read_info)(p, info);
                        let mut ww = 0u32;
                        let mut hh = 0u32;
                        let (mut bd, mut ct, mut ii, mut cm, mut fm) = (0, 0, 0, 0, 0);
                        (c.get_IHDR)(
                            p, info, &mut ww, &mut hh, &mut bd, &mut ct, &mut ii, &mut cm, &mut fm,
                        );
                        log(format!(
                            "R IHDR {ww}x{hh} bd={bd} ct={ct} il={ii} rowbytes={}",
                            (c.get_rowbytes)(p, info)
                        ));
                        (c.read_image)(p, rptrs.as_mut_ptr());
                        (c.read_end)(p, info);
                        log(format!("R palette_max={}", (c.get_palette_max)(p, info)));
                    });
                    let mut lines = t1.lines.clone();
                    lines.extend(t2.lines);
                    // ---- identity check ----
                    for y in 0..h as usize {
                        let dec = &dst[y * (stride + slack)..(y + 1) * (stride + slack)];
                        lines.push(format!("Rrow{y}={}", hex(dec)));
                        let mut same = dec[..stride] == src[y * stride..(y + 1) * stride];
                        if !same && stride > 0 {
                            // compare with the padding bits of the last byte masked
                            let a = &dec[..stride];
                            let b = &src[y * stride..(y + 1) * stride];
                            same = a[..stride - 1] == b[..stride - 1]
                                && (a[stride - 1] & lastmask) == (b[stride - 1] & lastmask);
                        }
                        let slack_zero = dec[stride..].iter().all(|b| *b == 0);
                        lines.push(format!(
                            "cmp{y} equal={} slack_zero={}",
                            same as u8, slack_zero as u8
                        ));
                        if !same || !slack_zero {
                            ok.set(false);
                        }
                    }
                    Trace {
                        lines,
                        out: produced,
                        rc: t1.rc | (t2.rc << 8),
                    }
                });
                assert!(
                    ok.get(),
                    "[{label}] the decoded rows do not equal the written rows \
                     (identically in both libraries)"
                );
            }
        }
    }
}

// ===========================================================================
// M6 — png_write_png / png_read_png with the same transform mask
// ===========================================================================

/// Transform bits honoured by *both* png_write_png and png_read_png.
const COMMON_TRANSFORMS: &[(&str, c_int)] = &[
    ("IDENTITY", PNG_TRANSFORM_IDENTITY),
    ("PACKING", PNG_TRANSFORM_PACKING),
    ("PACKSWAP", PNG_TRANSFORM_PACKSWAP),
    ("INVERT_MONO", PNG_TRANSFORM_INVERT_MONO),
    ("SHIFT", PNG_TRANSFORM_SHIFT),
    ("BGR", PNG_TRANSFORM_BGR),
    ("SWAP_ALPHA", PNG_TRANSFORM_SWAP_ALPHA),
    ("SWAP_ENDIAN", PNG_TRANSFORM_SWAP_ENDIAN),
    ("INVERT_ALPHA", PNG_TRANSFORM_INVERT_ALPHA),
];

/// Bits that only one side honours; they are still passed to both calls.
const ONE_SIDED_TRANSFORMS: &[(&str, c_int)] = &[
    ("STRIP_16", PNG_TRANSFORM_STRIP_16),
    ("STRIP_ALPHA", PNG_TRANSFORM_STRIP_ALPHA),
    ("EXPAND", PNG_TRANSFORM_EXPAND),
    ("GRAY_TO_RGB", PNG_TRANSFORM_GRAY_TO_RGB),
    ("EXPAND_16", PNG_TRANSFORM_EXPAND_16),
    ("SCALE_16", PNG_TRANSFORM_SCALE_16),
    ("STRIP_FILLER_BEFORE", PNG_TRANSFORM_STRIP_FILLER_BEFORE),
    ("STRIP_FILLER_AFTER", PNG_TRANSFORM_STRIP_FILLER_AFTER),
];

#[test]
fn m6_write_png_read_png() {
    // (colour type, bit depth); the row buffers are always big enough for the
    // widest interpretation libpng can pick (4 channels, 16 bit, unpacked).
    let fmts: &[(c_int, c_int)] = &[
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
    ];
    let mut cases: Vec<(String, c_int)> = COMMON_TRANSFORMS
        .iter()
        .map(|(n, v)| (n.to_string(), *v))
        .collect();
    cases.extend(
        ONE_SIDED_TRANSFORMS
            .iter()
            .map(|(n, v)| (n.to_string(), *v)),
    );
    // 8 seeded random combinations over the honoured bits of both sides
    {
        let all: Vec<c_int> = COMMON_TRANSFORMS
            .iter()
            .chain(ONE_SIDED_TRANSFORMS.iter())
            .map(|(_, v)| *v)
            .filter(|v| *v != 0)
            .collect();
        let mut rng = Rng::new(0x1600_0001);
        for k in 0..8 {
            let mut m = 0;
            for b in &all {
                if rng.below(3) == 0 {
                    m |= *b;
                }
            }
            // BEFORE+AFTER together is explicitly rejected by png_write_png;
            // keep at most one of the two so the random combinations exercise
            // the transforms rather than that single error path (which is
            // covered by the two individual cases above).
            if m & PNG_TRANSFORM_STRIP_FILLER_BEFORE != 0
                && m & PNG_TRANSFORM_STRIP_FILLER_AFTER != 0
            {
                m &= !PNG_TRANSFORM_STRIP_FILLER_BEFORE;
            }
            cases.push((format!("rand{k}"), m));
        }
    }

    let w = 6u32;
    let h = 4u32;
    for &(color, depth) in fmts {
        let npal: c_int = if color == PNG_COLOR_TYPE_PALETTE {
            1i32 << depth
        } else {
            0
        };
        let pal: Vec<u8> = {
            let mut r = Rng::new(0x1601_0000 + color as u64 * 64 + depth as u64);
            r.bytes(npal as usize * 3)
        };
        let sbit = PngColor8 {
            red: (depth / 2).max(1) as u8,
            green: (depth / 2).max(1) as u8,
            blue: (depth / 2).max(1) as u8,
            gray: (depth / 2).max(1) as u8,
            alpha: (depth / 2).max(1) as u8,
        };
        // enough room for 4 channels * 2 bytes * width, whatever libpng decides
        // the caller's row format is
        let cap = (w as usize * 4 * 2) + 16;
        let srcs: Vec<Vec<u8>> = (0..2u64)
            .map(|s| {
                let mut r = Rng::new(0x1602_0000 + color as u64 * 64 + depth as u64 + s * 0x1_0000);
                (0..cap * h as usize)
                    .map(|_| {
                        if color == PNG_COLOR_TYPE_PALETTE {
                            (r.byte() as c_int % npal) as u8
                        } else {
                            r.byte()
                        }
                    })
                    .collect()
            })
            .collect();
        for (si, src) in srcs.iter().enumerate() {
        for (tname, tmask) in &cases {
            let label = format!("M6 ct={color} bd={depth} src={si} t={tname}({tmask:#x})");
            // With no transform at all the round trip has to succeed and really
            // fill the caller's buffers.
            let need: &[&str] = if *tmask == PNG_TRANSFORM_IDENTITY {
                &["read_png returned", "R2 buffer_modified=1"]
            } else {
                &[]
            };
            diff_needs(&label, |lib| {
                let mut rows: Vec<u8> = src.clone();
                let mut wptrs: Vec<*mut u8> = (0..h as usize)
                    .map(|y| unsafe { rows.as_mut_ptr().add(y * cap) })
                    .collect();
                let t1 = with_write(lib, &mut |c, p, info| unsafe {
                    (c.set_IHDR)(
                        p,
                        info,
                        w,
                        h,
                        depth,
                        color,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    if npal > 0 {
                        (c.set_PLTE)(p, info, pal.as_ptr(), npal);
                    }
                    (c.set_sBIT)(p, info, &sbit as *const PngColor8 as *const u8);
                    (c.set_rows)(p, info, wptrs.as_mut_ptr());
                    log(format!(
                        "W rows_null={} valid_IDAT={} rowbytes={}",
                        (c.get_rows)(p, info).is_null() as u8,
                        (c.get_valid)(p, info, PNG_INFO_IDAT),
                        (c.get_rowbytes)(p, info)
                    ));
                    (c.write_png)(p, info, *tmask, std::ptr::null_mut());
                    log("write_png returned");
                });
                let produced = t1.out.clone();
                let mut lines = t1.lines.clone();
                lines.push(format!(
                    "W produced len={} bytes={}",
                    produced.len(),
                    hex(&produced)
                ));
                // (a) libpng allocates the rows itself (png_malloc, therefore
                //     *uninitialised*).  When the row does not end on a byte
                //     boundary libpng deliberately preserves the unused bits of
                //     the caller's last byte (the `end_mask` logic in
                //     png_combine_row), so that byte is indeterminate and must
                //     not be logged; the mask is logged instead.
                let t2 = with_read(lib, &produced, &mut |c, p, info| unsafe {
                    (c.read_png)(p, info, *tmask, std::ptr::null_mut());
                    log("read_png returned");
                    let mut ww = 0u32;
                    let mut hh = 0u32;
                    let (mut bd, mut ct, mut il, mut cm, mut fm) = (0, 0, 0, 0, 0);
                    (c.get_IHDR)(
                        p, info, &mut ww, &mut hh, &mut bd, &mut ct, &mut il, &mut cm, &mut fm,
                    );
                    let rb = (c.get_rowbytes)(p, info);
                    let ch = (c.get_channels)(p, info) as usize;
                    let dbd = (c.get_bit_depth)(p, info) as usize;
                    log(format!(
                        "R {ww}x{hh} bd={bd} ct={ct} il={il} rowbytes={rb} channels={ch} depth={dbd}"
                    ));
                    log(format!(
                        "R valid_IDAT={}",
                        (c.get_valid)(p, info, PNG_INFO_IDAT)
                    ));
                    let rem = (ch * dbd * ww as usize) % 8;
                    let nlog = if rem == 0 { rb } else { rb.saturating_sub(1) };
                    log(format!("R rem={rem} logged={nlog}/{rb}"));
                    let rows = (c.get_rows)(p, info);
                    log(format!("R rows_null={}", rows.is_null() as u8));
                    if !rows.is_null() {
                        for y in 0..hh as usize {
                            let rp = *rows.add(y);
                            if rp.is_null() {
                                log(format!("Rrow{y}=<null>"));
                            } else {
                                log(format!(
                                    "Rrow{y}={}",
                                    hex(std::slice::from_raw_parts(rp, nlog))
                                ));
                            }
                        }
                    }
                });
                lines.extend(t2.lines);
                // (b) the same read with application-supplied row buffers
                //     (png_set_rows before png_read_png keeps libpng from
                //     allocating its own).  They are pre-filled with a fixed
                //     pattern, so every byte - including the unused bits of the
                //     last byte, which libpng must leave alone - is compared.
                let mut rrows: Vec<u8> = vec![0xA5; cap * h as usize];
                let mut rptrs: Vec<*mut u8> = (0..h as usize)
                    .map(|y| unsafe { rrows.as_mut_ptr().add(y * cap) })
                    .collect();
                let t3 = with_read(lib, &produced, &mut |c, p, info| unsafe {
                    // png_read_png calls png_read_info itself, so the rows have
                    // to be attached before that (png_set_rows only stores the
                    // pointer and sets PNG_INFO_IDAT).
                    (c.set_rows)(p, info, rptrs.as_mut_ptr());
                    log(format!(
                        "R2 rows_set null={} valid_IDAT={}",
                        (c.get_rows)(p, info).is_null() as u8,
                        (c.get_valid)(p, info, PNG_INFO_IDAT)
                    ));
                    (c.read_png)(p, info, *tmask, std::ptr::null_mut());
                    log("R2 read_png returned");
                    let rb = (c.get_rowbytes)(p, info);
                    log(format!(
                        "R2 rowbytes={rb} channels={} depth={}",
                        (c.get_channels)(p, info),
                        (c.get_bit_depth)(p, info)
                    ));
                    let rows = (c.get_rows)(p, info);
                    log(format!(
                        "R2 rows_null={} same_as_app={}",
                        rows.is_null() as u8,
                        (rows == rptrs.as_mut_ptr()) as u8
                    ));
                });
                lines.extend(t3.lines);
                for y in 0..h as usize {
                    lines.push(format!(
                        "R2row{y}={}",
                        hex(&rrows[y * cap..(y + 1) * cap])
                    ));
                }
                lines.push(format!(
                    "R2 buffer_modified={}",
                    rrows.iter().any(|b| *b != 0xA5) as u8
                ));
                Trace {
                    lines,
                    out: produced,
                    rc: t1.rc | (t2.rc << 8) | (t3.rc << 16),
                }
            }, need);
        }
        }
    }
}

// ===========================================================================
// M7 — png_set_keep_unknown_chunks x png_handle_as_unknown
// ===========================================================================

/// D50 as an ICC XYZNumber (png.c `D50_nCIEXYZ`).
const D50: [u8; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];

fn be32(dst: &mut [u8], v: u32) {
    dst[0] = (v >> 24) as u8;
    dst[1] = (v >> 16) as u8;
    dst[2] = (v >> 8) as u8;
    dst[3] = v as u8;
}

/// A minimal but valid 132-byte ICC profile with no tags.
fn icc_min() -> Vec<u8> {
    let mut p = vec![0u8; 132];
    be32(&mut p[0..4], 132);
    p[4..8].copy_from_slice(b"none");
    p[8] = 2;
    p[9] = 0x40;
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(b"RGB ");
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p[40..44].copy_from_slice(b"APPL");
    be32(&mut p[64..68], 0);
    p[68..80].copy_from_slice(&D50);
    p[80..84].copy_from_slice(b"none");
    be32(&mut p[128..132], 0);
    p
}

unsafe fn log_handle_all(c: &Core, png: Png) {
    let mut s = String::new();
    for n in KNOWN_NAMES {
        let nm = name5(n);
        s.push_str(&format!(
            "{}:{} ",
            String::from_utf8_lossy(*n as &[u8]),
            (c.handle_as_unknown)(png, nm.as_ptr())
        ));
    }
    log(format!("handle_known[{}]", s.trim_end()));
    let mut s = String::new();
    for n in PRIVATE_NAMES {
        let nm = name5(n);
        s.push_str(&format!(
            "{}:{} ",
            String::from_utf8_lossy(*n as &[u8]),
            (c.handle_as_unknown)(png, nm.as_ptr())
        ));
    }
    log(format!("handle_private[{}]", s.trim_end()));
}

#[test]
fn m7_keep_unknown_chunks() {
    const W: u32 = 5;
    const H: u32 = 3;
    const NPAL: c_int = 16;

    // ---- everything the write side needs, allocated once (the raw pointers
    // ---- handed to libpng must outlive every closure below)
    let mut rng = Rng::new(0x1700_0001);
    let pal = rng.bytes(NPAL as usize * 3);
    let alpha = rng.bytes(8);
    let hist: Vec<u16> = (0..NPAL as usize).map(|i| (i * 997) as u16).collect();
    let prof = icc_min();
    let iccname = CString::new("m7 profile").unwrap();
    let purpose = CString::new("calib").unwrap();
    let units = CString::new("metres").unwrap();
    let params: Vec<CString> = vec![CString::new("1.5").unwrap(), CString::new("-2e3").unwrap()];
    let pptrs: Vec<*mut c_char> = params.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    let swidth = CString::new("1.5").unwrap();
    let sheight = CString::new("0.75").unwrap();
    let exif: Vec<u8> = {
        let mut v = vec![0x49u8, 0x49, 0x2A, 0x00];
        v.extend_from_slice(&[8, 0, 0, 0, 1, 2, 3, 4]);
        v
    };
    let tkeys: Vec<CString> = (0..3)
        .map(|i| CString::new(format!("Key{i}")).unwrap())
        .collect();
    let ttexts: Vec<CString> = (0..3)
        .map(|i| CString::new(format!("text value {i}")).unwrap())
        .collect();
    let tlangs: Vec<CString> = (0..3).map(|_| CString::new("en-GB").unwrap()).collect();
    let tlkeys: Vec<CString> = (0..3)
        .map(|i| CString::new(format!("LKey{i}")).unwrap())
        .collect();
    // tEXt, zTXt and iTXt
    let texts: Vec<PngText> = (0..3)
        .map(|i| PngText {
            compression: match i {
                0 => PNG_TEXT_COMPRESSION_NONE,
                1 => PNG_TEXT_COMPRESSION_zTXt,
                _ => PNG_ITXT_COMPRESSION_NONE,
            },
            key: tkeys[i].as_ptr() as *mut c_char,
            text: ttexts[i].as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: if i == 2 {
                tlangs[i].as_ptr() as *mut c_char
            } else {
                std::ptr::null_mut()
            },
            lang_key: if i == 2 {
                tlkeys[i].as_ptr() as *mut c_char
            } else {
                std::ptr::null_mut()
            },
        })
        .collect();
    let sname = CString::new("suggested").unwrap();
    let sents: Vec<PngSpltEntry> = (0..4)
        .map(|i| PngSpltEntry {
            red: (i * 11) as u16,
            green: (i * 22) as u16,
            blue: (i * 33) as u16,
            alpha: 255,
            frequency: (i * 7) as u16,
        })
        .collect();
    let splt = vec![PngSpltT {
        name: sname.as_ptr() as *mut c_char,
        depth: 8,
        entries: sents.as_ptr() as *mut PngSpltEntry,
        nentries: sents.len() as i32,
    }];
    let unk_data: Vec<Vec<u8>> = (0..PRIVATE_NAMES.len())
        .map(|i| (0..(i * 5 + 1)).map(|k| (k * 3 + i) as u8).collect())
        .collect();
    let unk: Vec<PngUnknownChunk> = PRIVATE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| PngUnknownChunk {
            name: name5(n),
            data: unk_data[i].as_ptr() as *mut u8,
            size: unk_data[i].len(),
            // 0x01 = HAVE_IHDR, 0x02 = HAVE_PLTE, 0x08 = AFTER_IDAT
            location: match i % 3 {
                0 => 0x01,
                1 => 0x02,
                _ => 0x08,
            },
        })
        .collect();
    let bkgd = PngColor16 {
        index: 3,
        ..Default::default()
    };
    let sbit = PngColor8 {
        red: 8,
        green: 8,
        blue: 8,
        gray: 8,
        alpha: 8,
    };
    let time = PngTime {
        year: 2024,
        month: 7,
        day: 15,
        hour: 12,
        minute: 34,
        second: 56,
    };
    let stride = pngbuild::rowbytes(PNG_COLOR_TYPE_PALETTE as u8, 8, W);
    let rows: Vec<u8> = {
        let mut r = Rng::new(0x1702_0001);
        (0..stride * H as usize)
            .map(|_| (r.byte() as c_int % NPAL) as u8)
            .collect()
    };

    // ---------------- the producer (same call sequence in both libraries) ----
    let write_all = |lib: &Lib| -> Trace {
        with_write(lib, &mut |c, p, info| unsafe {
            (c.set_IHDR)(
                p,
                info,
                W,
                H,
                8,
                PNG_COLOR_TYPE_PALETTE,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (c.set_PLTE)(p, info, pal.as_ptr(), NPAL);
            (c.set_tRNS)(p, info, alpha.as_ptr(), 8, std::ptr::null());
            (c.set_hIST)(p, info, hist.as_ptr());
            (c.set_bKGD)(p, info, &bkgd as *const PngColor16 as *const u8);
            (c.set_sBIT)(p, info, &sbit as *const PngColor8 as *const u8);
            (c.set_gAMA_fixed)(p, info, 45455);
            (c.set_cHRM_fixed)(p, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
            (c.set_sRGB)(p, info, PNG_sRGB_INTENT_PERCEPTUAL);
            (c.set_iCCP)(
                p,
                info,
                iccname.as_ptr(),
                PNG_COMPRESSION_TYPE_BASE,
                prof.as_ptr(),
                prof.len() as u32,
            );
            (c.set_pHYs)(p, info, 2835, 2835, PNG_RESOLUTION_METER);
            (c.set_oFFs)(p, info, -17, 42, PNG_OFFSET_PIXEL);
            (c.set_tIME)(p, info, &time as *const PngTime as *const u8);
            (c.set_pCAL)(
                p,
                info,
                purpose.as_ptr(),
                -100,
                100,
                PNG_EQUATION_LINEAR,
                2,
                units.as_ptr(),
                pptrs.as_ptr() as *mut *mut c_char,
            );
            (c.set_sCAL_s)(p, info, PNG_SCALE_METER, swidth.as_ptr(), sheight.as_ptr());
            (c.set_sPLT)(p, info, splt.as_ptr() as *const c_void, 1);
            (c.set_text)(p, info, texts.as_ptr() as *const c_void, 3);
            (c.set_eXIf_1)(p, info, exif.len() as u32, exif.as_ptr());
            (c.set_cICP)(p, info, 1, 13, 0, 1);
            (c.set_cLLI_fixed)(p, info, 10_000_000, 500_000);
            (c.set_mDCV_fixed)(
                p, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10_000_000, 500,
            );
            (c.set_unknown_chunks)(p, info, unk.as_ptr() as *const c_void, unk.len() as c_int);
            (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
            log("W setters done");
            log_valid_all(c, p, info);
            (c.write_info)(p, info);
            for y in 0..H as usize {
                (c.write_row)(p, rows.as_ptr().add(y * stride));
            }
            (c.write_end)(p, info);
            log("W write_end returned");
        })
    };

    // Producing the stream is itself a differential test.
    diff_needs(
        "M7 produce all-chunk image",
        |lib| write_all(lib),
        &["W write_end returned"],
    );

    // Sanity: the configurations below must really keep unknown chunks and must
    // really push known chunks through the unknown-chunk path.
    let saw_unknown = Cell::new(false);
    let saw_known_kept = Cell::new(false);

    // ---------------- read configurations ----------------
    // list kind: 0 none, 1 private names, 2 known names, 3 built-in ignore list
    //            (num_chunks_in < 0), 4 two calls with different keeps
    let priv_list: Vec<u8> = PRIVATE_NAMES.iter().flat_map(|n| name5(n)).collect();
    let known_list: Vec<u8> = [b"gAMA", b"tEXt", b"sPLT", b"hIST", b"bKGD", b"tRNS"]
        .iter()
        .flat_map(|n| name5(n))
        .collect();
    let mixed_a: Vec<u8> = [b"prVt", b"gAMA"].iter().flat_map(|n| name5(n)).collect();
    let mixed_b: Vec<u8> = [b"teSt", b"tEXt"].iter().flat_map(|n| name5(n)).collect();

    for global in 0..4 {
        for list_kind in 0..5 {
            let keeps: &[c_int] = if list_kind == 0 {
                &[PNG_HANDLE_CHUNK_AS_DEFAULT]
            } else {
                &[
                    PNG_HANDLE_CHUNK_AS_DEFAULT,
                    PNG_HANDLE_CHUNK_NEVER,
                    PNG_HANDLE_CHUNK_IF_SAFE,
                    PNG_HANDLE_CHUNK_ALWAYS,
                ]
            };
            for &kp in keeps {
                let label = format!("M7 global={global} list={list_kind} keep={kp}");
                diff(&label, |lib| {
                    let t1 = write_all(lib);
                    let produced = t1.out.clone();
                    let mut lines = t1.lines.clone();
                    let mut buf = [0u8; 64];
                    let t2 = with_read(lib, &produced, &mut |c, p, info| unsafe {
                        (c.set_keep_unknown_chunks)(p, global, std::ptr::null(), 0);
                        match list_kind {
                            1 => (c.set_keep_unknown_chunks)(
                                p,
                                kp,
                                priv_list.as_ptr(),
                                PRIVATE_NAMES.len() as c_int,
                            ),
                            2 => (c.set_keep_unknown_chunks)(p, kp, known_list.as_ptr(), 6),
                            3 => (c.set_keep_unknown_chunks)(p, kp, std::ptr::null(), -1),
                            4 => {
                                (c.set_keep_unknown_chunks)(
                                    p,
                                    PNG_HANDLE_CHUNK_NEVER,
                                    mixed_a.as_ptr(),
                                    2,
                                );
                                (c.set_keep_unknown_chunks)(p, kp, mixed_b.as_ptr(), 2);
                            }
                            _ => {}
                        }
                        log("R keep configured");
                        log_handle_all(c, p);
                        (c.read_info)(p, info);
                        log("R read_info returned");
                        log_handle_all(c, p);
                        log_all_info(c, p, info);
                        let rb = (c.get_rowbytes)(p, info).min(buf.len());
                        for y in 0..H {
                            (c.read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                            log(format!("Rrow{y}={}", hex(&buf[..rb])));
                        }
                        (c.read_end)(p, info);
                        log("R read_end returned");
                        log_all_info(c, p, info);
                        log_handle_all(c, p);
                    });
                    lines.extend(t2.lines);
                    if lines
                        .iter()
                        .any(|l| l.starts_with("unknown n=") && l != "unknown n=0")
                    {
                        saw_unknown.set(true);
                    }
                    if lines
                        .iter()
                        .any(|l| l.starts_with("handle_known[") && !l.contains("gAMA:0"))
                    {
                        saw_known_kept.set(true);
                    }
                    Trace {
                        lines,
                        out: produced,
                        rc: t1.rc | (t2.rc << 8),
                    }
                });
            }
        }
    }
    assert!(saw_unknown.get(), "M7: no configuration kept an unknown chunk");
    assert!(
        saw_known_kept.get(),
        "M7: no configuration routed a known chunk through the unknown path"
    );

    // Invalid keep values: png_app_error, which is fatal on a read struct in
    // this build unless benign errors are enabled.
    for &(bad, benign) in &[(4i32, false), (4, true), (-1, false), (-1, true)] {
        let label = format!("M7 invalid keep={bad} benign={benign}");
        diff(&label, |lib| {
            let t1 = write_all(lib);
            let produced = t1.out.clone();
            let mut lines = t1.lines.clone();
            let mut buf = [0u8; 64];
            let t2 = with_read(lib, &produced, &mut |c, p, info| unsafe {
                if benign {
                    (c.set_benign_errors)(p, 1);
                }
                (c.set_keep_unknown_chunks)(p, bad, std::ptr::null(), 0);
                log("R set_keep_unknown_chunks(invalid) returned");
                log_handle_all(c, p);
                // and a NULL list with a positive count
                (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 3);
                log("R null-list returned");
                (c.read_info)(p, info);
                let rb = (c.get_rowbytes)(p, info).min(buf.len());
                for y in 0..H {
                    (c.read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                    log(format!("Rrow{y}={}", hex(&buf[..rb])));
                }
                (c.read_end)(p, info);
                log_all_info(c, p, info);
                log_handle_all(c, p);
            });
            lines.extend(t2.lines);
            Trace {
                lines,
                out: produced,
                rc: t1.rc | (t2.rc << 8),
            }
        });
    }
}

// ===========================================================================
// M8 — png_free_data / png_data_freer on a fully populated info struct
// ===========================================================================

const FREE_MASKS: &[(&str, u32)] = &[
    ("HIST", PNG_FREE_HIST),
    ("ICCP", PNG_FREE_ICCP),
    ("SPLT", PNG_FREE_SPLT),
    ("ROWS", PNG_FREE_ROWS),
    ("PCAL", PNG_FREE_PCAL),
    ("SCAL", PNG_FREE_SCAL),
    ("UNKN", PNG_FREE_UNKN),
    ("PLTE", PNG_FREE_PLTE),
    ("TRNS", PNG_FREE_TRNS),
    ("TEXT", PNG_FREE_TEXT),
    ("EXIF", PNG_FREE_EXIF),
    ("ALL", PNG_FREE_ALL),
];

/// Everything the M8 population needs; the raw pointers handed to libpng point
/// into this, so it must outlive every closure.
struct M8Bag {
    pal: Vec<u8>,
    alpha: Vec<u8>,
    hist: Vec<u16>,
    prof: Vec<u8>,
    iccname: CString,
    purpose: CString,
    units: CString,
    _params: Vec<CString>,
    pptrs: Vec<*mut c_char>,
    swidth: CString,
    sheight: CString,
    exif: Vec<u8>,
    _tkeys: Vec<CString>,
    _ttexts: Vec<CString>,
    _tlangs: Vec<CString>,
    _tlkeys: Vec<CString>,
    texts: Vec<PngText>,
    _snames: Vec<CString>,
    _sents: Vec<Vec<PngSpltEntry>>,
    splt: Vec<PngSpltT>,
    _unk_data: Vec<Vec<u8>>,
    unk: Vec<PngUnknownChunk>,
}

fn m8_bag() -> M8Bag {
    let mut rng = Rng::new(0x1800_0001);
    let pal = rng.bytes(16 * 3);
    let alpha = rng.bytes(8);
    let hist: Vec<u16> = (0..16).map(|i| (i * 313) as u16).collect();
    let params: Vec<CString> = vec![
        CString::new("1").unwrap(),
        CString::new("-2.5").unwrap(),
        CString::new("3e2").unwrap(),
    ];
    let pptrs: Vec<*mut c_char> = params.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    let tkeys: Vec<CString> = (0..3)
        .map(|i| CString::new(format!("K{i}")).unwrap())
        .collect();
    let ttexts: Vec<CString> = (0..3)
        .map(|i| CString::new(format!("value number {i}")).unwrap())
        .collect();
    let tlangs: Vec<CString> = (0..3).map(|_| CString::new("de").unwrap()).collect();
    let tlkeys: Vec<CString> = (0..3)
        .map(|i| CString::new(format!("LK{i}")).unwrap())
        .collect();
    let texts: Vec<PngText> = (0..3)
        .map(|i| PngText {
            compression: match i {
                0 => PNG_TEXT_COMPRESSION_NONE,
                1 => PNG_TEXT_COMPRESSION_zTXt,
                _ => PNG_ITXT_COMPRESSION_zTXt,
            },
            key: tkeys[i].as_ptr() as *mut c_char,
            text: ttexts[i].as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: if i == 2 {
                tlangs[i].as_ptr() as *mut c_char
            } else {
                std::ptr::null_mut()
            },
            lang_key: if i == 2 {
                tlkeys[i].as_ptr() as *mut c_char
            } else {
                std::ptr::null_mut()
            },
        })
        .collect();
    let snames: Vec<CString> = (0..2)
        .map(|i| CString::new(format!("splt{i}")).unwrap())
        .collect();
    let sents: Vec<Vec<PngSpltEntry>> = (0..2)
        .map(|p| {
            (0..3 + p)
                .map(|i| PngSpltEntry {
                    red: (i * 5 + p) as u16,
                    green: (i * 6) as u16,
                    blue: (i * 7) as u16,
                    alpha: 200,
                    frequency: (i * 3) as u16,
                })
                .collect()
        })
        .collect();
    let splt: Vec<PngSpltT> = (0..2)
        .map(|i| PngSpltT {
            name: snames[i].as_ptr() as *mut c_char,
            depth: 8,
            entries: sents[i].as_ptr() as *mut PngSpltEntry,
            nentries: sents[i].len() as i32,
        })
        .collect();
    let unk_data: Vec<Vec<u8>> = (0..3)
        .map(|i| (0..(i * 4 + 2)).map(|k| (k + i * 9) as u8).collect())
        .collect();
    let unk: Vec<PngUnknownChunk> = (0..3)
        .map(|i| PngUnknownChunk {
            name: name5(PRIVATE_NAMES[i]),
            data: unk_data[i].as_ptr() as *mut u8,
            size: unk_data[i].len(),
            location: 0x01,
        })
        .collect();
    M8Bag {
        pal,
        alpha,
        hist,
        prof: icc_min(),
        iccname: CString::new("m8 icc").unwrap(),
        purpose: CString::new("purp").unwrap(),
        units: CString::new("units").unwrap(),
        _params: params,
        pptrs,
        swidth: CString::new("2.5").unwrap(),
        sheight: CString::new("1e-2").unwrap(),
        exif: vec![0x4D, 0x4D, 0x00, 0x2A, 0, 0, 0, 8, 9, 9],
        _tkeys: tkeys,
        _ttexts: ttexts,
        _tlangs: tlangs,
        _tlkeys: tlkeys,
        texts,
        _snames: snames,
        _sents: sents,
        splt,
        _unk_data: unk_data,
        unk,
    }
}

const M8_W: u32 = 4;
const M8_H: u32 = 3;

/// Fill `info` with every heap-owned item, allocating the image rows through
/// libpng (i.e. through the user memory callbacks) so that PNG_FREE_ROWS can be
/// exercised without handing libpng memory it must not free.
unsafe fn m8_populate(c: &Core, p: Png, info: Info, b: &M8Bag) {
    (c.set_IHDR)(
        p,
        info,
        M8_W,
        M8_H,
        8,
        PNG_COLOR_TYPE_PALETTE,
        PNG_INTERLACE_NONE,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );
    (c.set_PLTE)(p, info, b.pal.as_ptr(), 16);
    (c.set_tRNS)(p, info, b.alpha.as_ptr(), 8, std::ptr::null());
    (c.set_hIST)(p, info, b.hist.as_ptr());
    (c.set_iCCP)(
        p,
        info,
        b.iccname.as_ptr(),
        PNG_COMPRESSION_TYPE_BASE,
        b.prof.as_ptr(),
        b.prof.len() as u32,
    );
    (c.set_sCAL_s)(
        p,
        info,
        PNG_SCALE_METER,
        b.swidth.as_ptr(),
        b.sheight.as_ptr(),
    );
    (c.set_pCAL)(
        p,
        info,
        b.purpose.as_ptr(),
        -5,
        250,
        PNG_EQUATION_ARBITRARY,
        3,
        b.units.as_ptr(),
        b.pptrs.as_ptr() as *mut *mut c_char,
    );
    (c.set_eXIf_1)(p, info, b.exif.len() as u32, b.exif.as_ptr());
    (c.set_text)(p, info, b.texts.as_ptr() as *const c_void, 3);
    (c.set_sPLT)(p, info, b.splt.as_ptr() as *const c_void, 2);
    (c.set_unknown_chunks)(p, info, b.unk.as_ptr() as *const c_void, 3);
    // rows: libpng-owned memory, so PNG_FREE_ROWS is safe to exercise
    let stride = M8_W as usize; /* palette, 8 bit */
    let arr = (c.malloc)(p, (M8_H as usize * std::mem::size_of::<*mut u8>()) as u64) as *mut *mut u8;
    for y in 0..M8_H as usize {
        let r = (c.malloc)(p, stride as u64) as *mut u8;
        for x in 0..stride {
            *r.add(x) = ((y * stride + x) % 16) as u8;
        }
        *arr.add(y) = r;
    }
    (c.set_rows)(p, info, arr);
}

/// Structural (never dereferencing) view of the text array: after
/// `png_free_data(PNG_FREE_TEXT, num >= 0)` only `text[num].key` is freed and
/// NULLed while `text[num].text` still points into the same freed block.
unsafe fn log_text_struct(c: &Core, p: Png, info: Info) {
    let mut tp: *mut c_void = std::ptr::null_mut();
    let mut nt: c_int = -1;
    let n = (c.get_text)(p, info, &mut tp, &mut nt);
    log(format!(
        "text n={n} num={nt} arr_null={}",
        tp.is_null() as u8
    ));
    if n > 0 && !tp.is_null() {
        let arr = std::slice::from_raw_parts(tp as *const PngText, n as usize);
        for (i, t) in arr.iter().enumerate() {
            log(format!(
                "text[{i}] comp={} key_null={} text_null={} tlen={} ilen={} lang_null={} lkey_null={}",
                t.compression,
                t.key.is_null() as u8,
                t.text.is_null() as u8,
                t.text_length,
                t.itxt_length,
                t.lang.is_null() as u8,
                t.lang_key.is_null() as u8
            ));
        }
    }
}

unsafe fn m8_log_state(c: &Core, p: Png, info: Info) {
    log_valid_all(c, p, info);
    // PLTE
    let mut pal: *mut u8 = std::ptr::null_mut();
    let mut npal: c_int = -1;
    let r = (c.get_PLTE)(p, info, &mut pal, &mut npal);
    log(format!("PLTE rc={r} n={npal} null={}", pal.is_null() as u8));
    if r != 0 && !pal.is_null() && npal > 0 {
        log(format!(
            "PLTE data={}",
            hex(std::slice::from_raw_parts(pal, npal as usize * 3))
        ));
    }
    // tRNS
    let mut ta: *mut u8 = std::ptr::null_mut();
    let mut nt: c_int = -1;
    let mut tc: *mut u8 = std::ptr::null_mut();
    let r = (c.get_tRNS)(p, info, &mut ta, &mut nt, &mut tc);
    log(format!(
        "tRNS rc={r} n={nt} alpha_null={}",
        ta.is_null() as u8
    ));
    if r != 0 && !ta.is_null() && nt > 0 {
        log(format!(
            "tRNS alpha={}",
            hex(std::slice::from_raw_parts(ta, nt as usize))
        ));
    }
    // hIST
    let mut hi: *mut u16 = std::ptr::null_mut();
    let r = (c.get_hIST)(p, info, &mut hi);
    log(format!("hIST rc={r} null={}", hi.is_null() as u8));
    if r != 0 && !hi.is_null() && npal > 0 {
        log(format!(
            "hIST v={:?}",
            std::slice::from_raw_parts(hi, npal as usize)
        ));
    }
    // iCCP
    let mut nm: *mut c_char = std::ptr::null_mut();
    let mut comp: c_int = -1;
    let mut prof: *mut u8 = std::ptr::null_mut();
    let mut plen: u32 = 0;
    let r = (c.get_iCCP)(p, info, &mut nm, &mut comp, &mut prof, &mut plen);
    log(format!(
        "iCCP rc={r} name={} comp={comp} len={plen}",
        cstr(nm)
    ));
    // sCAL
    let mut su: c_int = -1;
    let mut sw: *mut c_char = std::ptr::null_mut();
    let mut sh: *mut c_char = std::ptr::null_mut();
    let r = (c.get_sCAL_s)(p, info, &mut su, &mut sw, &mut sh);
    log(format!(
        "sCAL rc={r} unit={su} w={} h={}",
        cstr(sw),
        cstr(sh)
    ));
    // pCAL
    let mut purpose: *mut c_char = std::ptr::null_mut();
    let (mut x0, mut x1) = (0i32, 0i32);
    let (mut et, mut np) = (0, 0);
    let mut units: *mut c_char = std::ptr::null_mut();
    let mut prm: *mut *mut c_char = std::ptr::null_mut();
    let r = (c.get_pCAL)(
        p,
        info,
        &mut purpose,
        &mut x0,
        &mut x1,
        &mut et,
        &mut np,
        &mut units,
        &mut prm,
    );
    log(format!(
        "pCAL rc={r} purpose={} x0={x0} x1={x1} type={et} nparams={np} units={} params_null={}",
        cstr(purpose),
        cstr(units),
        prm.is_null() as u8
    ));
    if r != 0 && !prm.is_null() {
        for i in 0..np as isize {
            log(format!("pCAL param[{i}]={}", cstr(*prm.offset(i))));
        }
    }
    // eXIf
    let mut ex: *mut u8 = std::ptr::null_mut();
    let mut elen: u32 = 0;
    let r = (c.get_eXIf_1)(p, info, &mut elen, &mut ex);
    log(format!(
        "eXIf rc={r} len={elen} null={}",
        ex.is_null() as u8
    ));
    if r != 0 && !ex.is_null() && elen > 0 {
        log(format!(
            "eXIf data={}",
            hex(std::slice::from_raw_parts(ex, elen as usize))
        ));
    }
    // text (structural only)
    log_text_struct(c, p, info);
    // sPLT
    let mut sp: *mut c_void = std::ptr::null_mut();
    let n = (c.get_sPLT)(p, info, &mut sp);
    log(format!("sPLT n={n} null={}", sp.is_null() as u8));
    if n > 0 && !sp.is_null() {
        let arr = std::slice::from_raw_parts(sp as *const PngSpltT, n as usize);
        for (i, e) in arr.iter().enumerate() {
            log(format!(
                "sPLT[{i}] name={} depth={} nentries={} entries_null={}",
                cstr(e.name),
                e.depth,
                e.nentries,
                e.entries.is_null() as u8
            ));
            if !e.entries.is_null() && e.nentries > 0 {
                log(format!(
                    "sPLT[{i}] entries={:?}",
                    std::slice::from_raw_parts(e.entries, e.nentries as usize)
                ));
            }
        }
    }
    // unknown chunks
    let mut up: *mut c_void = std::ptr::null_mut();
    let n = (c.get_unknown_chunks)(p, info, &mut up);
    log(format!("unknown n={n} null={}", up.is_null() as u8));
    if n > 0 && !up.is_null() {
        let arr = std::slice::from_raw_parts(up as *const PngUnknownChunk, n as usize);
        for (i, u) in arr.iter().enumerate() {
            log(format!(
                "unknown[{i}] name={} size={} loc={} data={}",
                String::from_utf8_lossy(&u.name[..4]),
                u.size,
                u.location,
                if u.data.is_null() {
                    "<null>".to_string()
                } else {
                    hex(std::slice::from_raw_parts(u.data, u.size))
                }
            ));
        }
    }
    // rows: png_free_data NULLs row_pointers when it frees them, so a non-NULL
    // array is always safe to read
    let rows = (c.get_rows)(p, info);
    log(format!("rows_null={}", rows.is_null() as u8));
    if !rows.is_null() {
        for y in 0..M8_H as usize {
            let rp = *rows.add(y);
            if rp.is_null() {
                log(format!("row[{y}]=<null>"));
            } else {
                log(format!(
                    "row[{y}]={}",
                    hex(std::slice::from_raw_parts(rp, M8_W as usize))
                ));
            }
        }
    }
}

#[test]
fn m8_free_data() {
    let bag = m8_bag();
    let saw_leak = Cell::new(false);
    let saw_clean = Cell::new(false);
    for &is_read in &[true, false] {
        for (mname, mask) in FREE_MASKS {
            for &num in &[-1i32, 0, 1] {
                for &freer in &[
                    PNG_DESTROY_WILL_FREE_DATA,
                    PNG_SET_WILL_FREE_DATA,
                    PNG_USER_WILL_FREE_DATA,
                ] {
                    let label = format!(
                        "M8 {} mask={mname} num={num} freer={freer}",
                        if is_read { "read" } else { "write" }
                    );
                    diff_needs(&label, |lib| {
                        let t = drive_acct(lib, &[], &mut |c| unsafe {
                            let p = if is_read {
                                (c.create_read_2)(
                                    ver(),
                                    std::ptr::null_mut(),
                                    cb_error as Cb,
                                    cb_warning as Cb,
                                    std::ptr::null_mut(),
                                    cb_malloc as Cb,
                                    cb_free as Cb,
                                )
                            } else {
                                (c.create_write_2)(
                                    ver(),
                                    std::ptr::null_mut(),
                                    cb_error as Cb,
                                    cb_warning as Cb,
                                    std::ptr::null_mut(),
                                    cb_malloc as Cb,
                                    cb_free as Cb,
                                )
                            };
                            log(format!("create2={}", (!p.is_null()) as u8));
                            if p.is_null() {
                                return;
                            }
                            (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
                            let i = (c.create_info)(p);
                            log(format!("create_info={}", (!i.is_null()) as u8));
                            m8_populate(c, p, i, &bag);
                            log("populated");
                            m8_log_state(c, p, i);
                            // PNG_SET_WILL_FREE_DATA is not accepted by
                            // png_data_freer: it is a fatal error.
                            (c.data_freer)(p, i, freer, *mask);
                            log(format!("data_freer({freer},{mask:#x}) returned"));
                            (c.free_data)(p, i, *mask, num);
                            log(format!("free_data({mask:#x},{num}) returned"));
                            m8_log_state(c, p, i);
                            // a second, identical free must be harmless
                            (c.free_data)(p, i, *mask, num);
                            log("second free_data returned");
                            m8_log_state(c, p, i);
                            let mut pp = p;
                            let mut ii = i;
                            if is_read {
                                (c.destroy_read)(&mut pp, &mut ii, std::ptr::null_mut());
                            } else {
                                (c.destroy_write)(&mut pp, &mut ii);
                            }
                            log("destroyed");
                        });
                        for l in &t.lines {
                            if let Some(v) = l.strip_prefix("malloc_count=") {
                                let live: i64 = v
                                    .split("live_allocs=")
                                    .nth(1)
                                    .and_then(|x| x.trim().parse().ok())
                                    .unwrap_or(0);
                                if live > 0 {
                                    saw_leak.set(true);
                                } else if live == 0 {
                                    saw_clean.set(true);
                                }
                            }
                        }
                        t
                    }, &[
                        "populated",
                        "rows_null=0",
                        "unknown n=3",
                        "sPLT n=2",
                        "text n=3",
                        // the getters return the PNG_INFO_* flag value, and
                        // log_valid_all prints png_get_valid() verbatim
                        "PLTE:8",
                        // png_get_valid() reports tRNS as absent while
                        // png_struct::num_trans is 0 (it is only set by
                        // png_handle_tRNS on read), so check the getter instead
                        "tRNS rc=16 n=8",
                        "hIST:64",
                        "pCAL:1024",
                        "iCCP:4096",
                        "sPLT:8192",
                        "sCAL:16384",
                        "IDAT:32768",
                        "eXIf:65536",
                    ]);
                }
            }
        }
    }
    // The freer parameter must actually change the outcome: some combinations
    // free everything (live_allocs back to 0), others deliberately leak.
    assert!(saw_clean.get(), "M8: no configuration ended leak free");
    assert!(
        saw_leak.get(),
        "M8: no configuration leaked, so PNG_USER_WILL_FREE_DATA had no effect"
    );
}

//! API / state / transform-setup rejection ("error surface") differential tests.
//!
//! Every individual bad input is driven through the C `.so` and the Rust `.so`
//! in its own `diff(...)` run, so a fatal error on one input can never mask a
//! later input.  The whole trace (messages, warning-vs-fatal behaviour, longjmp
//! `rc`, and the resulting info state as observed through the getters) is
//! compared line by line.
//!
//! Triggers were derived from the guarding conditions in `c_src/src/png.c`,
//! `pngset.c`, `pngget.c`, `pngerror.c`, `pngtrans.c` and `pngrtran.c`; the
//! source line is quoted in a comment for each family.
//!
//! Deliberately NOT called with a NULL `png_ptr` (they dereference it and would
//! kill the test process rather than return):
//!   * `png_process_data_skip`, `png_get_io_state`   (documented upstream bugs)
//!   * `png_error`, `png_chunk_error`, `png_longjmp`  -> `png_default_error` ->
//!     `png_longjmp(NULL)` -> `PNG_ABORT()` = `abort()`
//!   * `png_benign_error`, `png_app_error`, `png_app_warning`,
//!     `png_chunk_benign_error`, `png_chunk_report` -> read `png_ptr->flags` /
//!     `png_ptr->mode` unconditionally (pngerror.c:310,340,353,463,487)
//!   * `png_set_benign_errors` (pngset.c:1936), `png_set_check_for_invalid_index`
//!     (pngset.c:1960), `png_set_read_user_transform_fn` (pngrtran.c:1138) --
//!     all write through `png_ptr` with no NULL check.
mod support;

use std::ffi::{c_char, c_int, c_uint, c_void};
use support::core::*;
use support::pngbuild::{Builder, Chunk};
use support::*;

const NP: *mut c_void = std::ptr::null_mut();
const NPP: *mut *mut c_void = std::ptr::null_mut();

// pngpriv.h / png.h bit values used below
const PNG_HAVE_IHDR: c_int = 0x01;
const PNG_HAVE_PLTE: c_int = 0x02;
const PNG_HAVE_IDAT: c_int = 0x04;
const PNG_AFTER_IDAT: c_int = 0x08;
const PNG_ALL_MNG_FEATURES: u32 = 0x05;
const PNG_CHUNK_WARNING: c_int = 0;
const PNG_CHUNK_WRITE_ERROR: c_int = 1;
const PNG_CHUNK_ERROR: c_int = 2;
const FP1: i32 = 100_000;

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

/// NUL-terminated byte vector.
fn cz(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn p(v: &[u8]) -> *const c_char {
    v.as_ptr() as *const c_char
}

/// A minimal valid PNG: `w` x `h`, bit depth `bd`, colour type `ct`.
fn img(w: u32, h: u32, bd: u8, ct: u8) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct);
    if ct == 3 {
        b = b.add(b"PLTE", vec![0u8; 3 * (1usize << bd.min(8))]);
    }
    b.build_valid(0x1234_5678)
}

/// 2x1 RGB8 image whose single row is `pixels` (6 bytes).
fn rgb_row(pixels: &[u8]) -> Vec<u8> {
    let mut raw = vec![0u8];
    raw.extend_from_slice(pixels);
    let b = Builder::new(2, 1, 8, 2);
    b.build(&raw, 0)
}

/// 1x1 RGBA8 image.
fn rgba1() -> Vec<u8> {
    let raw = vec![0u8, 0x40, 0x80, 0xc0, 0x7f];
    Builder::new(1, 1, 8, 6).build(&raw, 0)
}

/// Nested longjmp landing pad: log the result, then carry on with the test.
fn sub(tag: &str, f: impl FnMut()) {
    let rc = protected(f);
    log(format!("{tag} rc={rc}"));
}

/// Arm the harness allocator so that the `k`-th allocation from now on fails.
fn arm(k: usize) {
    with_session(|s| {
        s.malloc_count = 0;
        s.malloc_limit = Some(k - 1);
    });
    log(format!("ARM({k})"));
}

fn disarm() {
    with_session(|s| s.malloc_limit = None);
}

// ---------------------------------------------------------------------------
// state logging (never calls png_get_IHDR: that re-runs png_check_IHDR and is
// fatal on an uninitialised info struct -- exercised separately in
// `get_with_bad_state`)
// ---------------------------------------------------------------------------

unsafe fn st(c: &Core, png: Png, info: Info) {
    if info.is_null() {
        log("state: info=<null>".to_string());
        return;
    }
    log(format!(
        "hdr {}x{} d={} ct={} il={} cm={} fm={} rb={} ch={} pmax={}",
        (c.get_image_width)(png, info),
        (c.get_image_height)(png, info),
        (c.get_bit_depth)(png, info),
        (c.get_color_type)(png, info),
        (c.get_interlace_type)(png, info),
        (c.get_compression_type)(png, info),
        (c.get_filter_type)(png, info),
        (c.get_rowbytes)(png, info),
        (c.get_channels)(png, info),
        (c.get_palette_max)(png, info)
    ));
    let mut v = String::new();
    for (n, f) in [
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
    ] {
        v.push_str(&format!("{n}={} ", (c.get_valid)(png, info, f)));
    }
    log(format!("valid {v}"));

    let mut pal: *mut u8 = std::ptr::null_mut();
    let mut npal: c_int = -1;
    let r = (c.get_PLTE)(png, info, &mut pal, &mut npal);
    log(format!("PLTE rc={r} n={npal}"));
    if r != 0 && !pal.is_null() && npal > 0 {
        log(format!(
            "PLTE data={}",
            hex(std::slice::from_raw_parts(pal, npal as usize * 3))
        ));
    }
    let mut ta: *mut u8 = std::ptr::null_mut();
    let mut nt: c_int = -1;
    let mut tc: *mut u8 = std::ptr::null_mut();
    let r = (c.get_tRNS)(png, info, &mut ta, &mut nt, &mut tc);
    log(format!("tRNS rc={r} n={nt}"));
    if r != 0 && !ta.is_null() && nt > 0 && nt <= 256 {
        log(format!(
            "tRNS a={}",
            hex(std::slice::from_raw_parts(ta, nt as usize))
        ));
    }
    if r != 0 && !tc.is_null() {
        let v = *(tc as *const PngColor16);
        log(format!("tRNS c={v:?}"));
    }
    let mut g: i32 = -1;
    log(format!(
        "gAMA rc={} v={g}",
        (c.get_gAMA_fixed)(png, info, &mut g)
    ));
    let mut intent: c_int = -1;
    log(format!(
        "sRGB rc={} i={intent}",
        (c.get_sRGB)(png, info, &mut intent)
    ));
    let mut cv = [0i32; 8];
    let r = (c.get_cHRM_fixed)(
        png, info, &mut cv[0], &mut cv[1], &mut cv[2], &mut cv[3], &mut cv[4], &mut cv[5],
        &mut cv[6], &mut cv[7],
    );
    log(format!("cHRM rc={r} {cv:?}"));
    let mut xyz = [0i32; 9];
    let r = (c.get_cHRM_XYZ_fixed)(
        png, info, &mut xyz[0], &mut xyz[1], &mut xyz[2], &mut xyz[3], &mut xyz[4], &mut xyz[5],
        &mut xyz[6], &mut xyz[7], &mut xyz[8],
    );
    log(format!("cHRM_XYZ rc={r} {xyz:?}"));
    let mut name: *mut c_char = std::ptr::null_mut();
    let mut comp: c_int = -1;
    let mut prof: *mut u8 = std::ptr::null_mut();
    let mut plen: u32 = 0;
    let r = (c.get_iCCP)(png, info, &mut name, &mut comp, &mut prof, &mut plen);
    log(format!(
        "iCCP rc={r} name={} comp={comp} len={plen}",
        cstr(name)
    ));
    if r != 0 && !prof.is_null() && plen > 0 {
        log(format!(
            "iCCP data={}",
            hex(std::slice::from_raw_parts(prof, plen as usize))
        ));
    }
    let mut sb: *mut u8 = std::ptr::null_mut();
    let r = (c.get_sBIT)(png, info, &mut sb);
    log(format!("sBIT rc={r}"));
    if r != 0 && !sb.is_null() {
        log(format!("sBIT v={:?}", *(sb as *const PngColor8)));
    }
    let mut bk: *mut u8 = std::ptr::null_mut();
    let r = (c.get_bKGD)(png, info, &mut bk);
    log(format!("bKGD rc={r}"));
    if r != 0 && !bk.is_null() {
        log(format!("bKGD v={:?}", *(bk as *const PngColor16)));
    }
    let mut hi: *mut u16 = std::ptr::null_mut();
    let r = (c.get_hIST)(png, info, &mut hi);
    log(format!("hIST rc={r}"));
    if r != 0 && !hi.is_null() && npal > 0 {
        log(format!(
            "hIST v={:?}",
            std::slice::from_raw_parts(hi, npal as usize)
        ));
    }
    let (mut px, mut py, mut unit) = (0u32, 0u32, -1);
    log(format!(
        "pHYs rc={} {px} {py} u={unit}",
        (c.get_pHYs)(png, info, &mut px, &mut py, &mut unit)
    ));
    let (mut ox, mut oy, mut ou) = (0i32, 0i32, -1);
    log(format!(
        "oFFs rc={} {ox} {oy} u={ou}",
        (c.get_oFFs)(png, info, &mut ox, &mut oy, &mut ou)
    ));
    let mut tp: *mut u8 = std::ptr::null_mut();
    let r = (c.get_tIME)(png, info, &mut tp);
    log(format!("tIME rc={r}"));
    if r != 0 && !tp.is_null() {
        log(format!("tIME v={:?}", *(tp as *const PngTime)));
    }
    let mut purpose: *mut c_char = std::ptr::null_mut();
    let (mut x0, mut x1) = (0i32, 0i32);
    let (mut etype, mut nparams) = (-1, -1);
    let mut units: *mut c_char = std::ptr::null_mut();
    let mut params: *mut *mut c_char = std::ptr::null_mut();
    let r = (c.get_pCAL)(
        png,
        info,
        &mut purpose,
        &mut x0,
        &mut x1,
        &mut etype,
        &mut nparams,
        &mut units,
        &mut params,
    );
    log(format!(
        "pCAL rc={r} p={} {x0} {x1} t={etype} n={nparams} u={}",
        cstr(purpose),
        cstr(units)
    ));
    if r != 0 && !params.is_null() && nparams > 0 {
        for k in 0..nparams as isize {
            log(format!("pCAL[{k}]={}", cstr(*params.offset(k))));
        }
    }
    let mut sunit: c_int = -1;
    let mut sw: *mut c_char = std::ptr::null_mut();
    let mut sh: *mut c_char = std::ptr::null_mut();
    let r = (c.get_sCAL_s)(png, info, &mut sunit, &mut sw, &mut sh);
    log(format!(
        "sCAL rc={r} u={sunit} w={} h={}",
        cstr(sw),
        cstr(sh)
    ));
    let mut splt: *mut c_void = std::ptr::null_mut();
    let n = (c.get_sPLT)(png, info, &mut splt);
    log(format!("sPLT n={n}"));
    if n > 0 && !splt.is_null() {
        let arr = std::slice::from_raw_parts(splt as *const PngSpltT, n as usize);
        for (k, e) in arr.iter().enumerate() {
            log(format!(
                "sPLT[{k}] name={} depth={} n={}",
                cstr(e.name),
                e.depth,
                e.nentries
            ));
            if !e.entries.is_null() && e.nentries > 0 {
                log(format!(
                    "sPLT[{k}] e={:?}",
                    std::slice::from_raw_parts(e.entries, e.nentries as usize)
                ));
            }
        }
    }
    let mut exif: *mut u8 = std::ptr::null_mut();
    let mut elen: u32 = 0;
    let r = (c.get_eXIf_1)(png, info, &mut elen, &mut exif);
    log(format!("eXIf rc={r} len={elen}"));
    if r != 0 && !exif.is_null() && elen > 0 {
        log(format!(
            "eXIf d={}",
            hex(std::slice::from_raw_parts(exif, elen as usize))
        ));
    }
    let (mut cp, mut tf, mut mc, mut vf) = (9u8, 9u8, 9u8, 9u8);
    log(format!(
        "cICP rc={} {cp} {tf} {mc} {vf}",
        (c.get_cICP)(png, info, &mut cp, &mut tf, &mut mc, &mut vf)
    ));
    let (mut mcll, mut mfall) = (0u32, 0u32);
    log(format!(
        "cLLI rc={} {mcll} {mfall}",
        (c.get_cLLI_fixed)(png, info, &mut mcll, &mut mfall)
    ));
    let mut mx = [0i32; 8];
    let mut ml = [0u32; 2];
    let r = (c.get_mDCV_fixed)(
        png, info, &mut mx[0], &mut mx[1], &mut mx[2], &mut mx[3], &mut mx[4], &mut mx[5],
        &mut mx[6], &mut mx[7], &mut ml[0], &mut ml[1],
    );
    log(format!("mDCV rc={r} {mx:?} {ml:?}"));
    let mut tptr: *mut c_void = std::ptr::null_mut();
    let mut ntext: c_int = -1;
    let n = (c.get_text)(png, info, &mut tptr, &mut ntext);
    log(format!("text n={n} num={ntext}"));
    if n > 0 && !tptr.is_null() {
        let arr = std::slice::from_raw_parts(tptr as *const PngText, n as usize);
        for (k, t) in arr.iter().enumerate() {
            log(format!(
                "text[{k}] c={} key={} txt={} tl={} il={} lang={} lk={}",
                t.compression,
                cstr(t.key),
                cstr(t.text),
                t.text_length,
                t.itxt_length,
                cstr(t.lang),
                cstr(t.lang_key)
            ));
        }
    }
    let mut uptr: *mut c_void = std::ptr::null_mut();
    let n = (c.get_unknown_chunks)(png, info, &mut uptr);
    log(format!("unknown n={n}"));
    if n > 0 && !uptr.is_null() {
        let arr = std::slice::from_raw_parts(uptr as *const PngUnknownChunk, n as usize);
        for (k, u) in arr.iter().enumerate() {
            log(format!(
                "unknown[{k}] name={} size={} loc={} data={}",
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
    log(format!(
        "rows={} iochunk={}",
        (!(c.get_rows)(png, info).is_null()) as u8,
        (c.get_io_chunk_type)(png)
    ));
}

// ---------------------------------------------------------------------------
// drivers
// ---------------------------------------------------------------------------

type Body<'a> = &'a (dyn Fn(&Core, Png, Info, &Lib) + 'a);

/// Create a read or write struct, run `body`, destroy.
///
/// * `w`      - write struct instead of read struct
/// * `inp`    - bytes served by the read callback
/// * `mem`    - use `png_create_*_struct_2` with the harness user allocator
/// * `benign` - call `png_set_benign_errors` with this value first
/// * `lim`    - `malloc_limit` armed *before* the struct is created
#[allow(clippy::too_many_arguments)]
fn go(
    label: &str,
    w: bool,
    inp: &[u8],
    mem: bool,
    benign: Option<c_int>,
    lim: Option<usize>,
    body: Body,
) {
    diff(label, |lib| {
        session_reset(inp.to_vec());
        if let Some(k) = lim {
            with_session(|s| s.malloc_limit = Some(k));
        }
        let c = Core::new(lib);
        let rc = protected(|| unsafe {
            let png = if mem {
                let f = if w { c.create_write_2 } else { c.create_read_2 };
                f(
                    VER_STRING.as_ptr() as *const c_char,
                    NP,
                    cb_error as Cb,
                    cb_warning as Cb,
                    NP,
                    cb_malloc as Cb,
                    cb_free as Cb,
                )
            } else {
                let f = if w { c.create_write } else { c.create_read };
                f(
                    VER_STRING.as_ptr() as *const c_char,
                    NP,
                    cb_error as Cb,
                    cb_warning as Cb,
                )
            };
            log(format!("create={}", (!png.is_null()) as u8));
            if png.is_null() {
                return;
            }
            (c.set_longjmp)(png, shim().longjmp_ptr, shim().jmp_buf_size);
            if w {
                (c.set_write_fn)(png, NP, cb_write as Cb, cb_flush as Cb);
            } else {
                (c.set_read_fn)(png, NP, cb_read as Cb);
            }
            let info = (c.create_info)(png);
            log(format!("info={}", (!info.is_null()) as u8));
            if let Some(b) = benign {
                (c.set_benign_errors)(png, b);
            }
            body(&c, png, info, lib);
            disarm();
            let mut pp = png;
            let mut ii = info;
            if w {
                (c.destroy_write)(&mut pp, &mut ii);
            } else {
                (c.destroy_read)(&mut pp, &mut ii, NPP);
            }
            log("destroyed".to_string());
        });
        disarm();
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}

/// One diff on a write struct (no benign call).
fn dw(label: &str, body: Body) {
    go(label, true, &[], false, None, None, body);
}

/// One diff on a read struct (no benign call).
fn dr(label: &str, body: Body) {
    go(label, false, &[], false, None, None, body);
}

/// Read struct fed with `inp`.
fn di(label: &str, inp: &[u8], body: Body) {
    go(label, false, inp, false, None, None, body);
}

/// read/write x benign 0/1: four independent diffs.
fn quad(label: &str, body: Body) {
    for &w in &[false, true] {
        for &b in &[0, 1] {
            go(
                &format!("{label} [{}b{b}]", if w { "W" } else { "R" }),
                w,
                &[],
                false,
                Some(b),
                None,
                body,
            );
        }
    }
}

/// read/write, no benign call (library default = fatal): two diffs.
fn duo(label: &str, body: Body) {
    for &w in &[false, true] {
        go(
            &format!("{label} [{}]", if w { "W" } else { "R" }),
            w,
            &[],
            false,
            None,
            None,
            body,
        );
    }
}

/// read/write x benign 0/1 with the user allocator: four diffs.
fn quad_mem(label: &str, body: Body) {
    for &w in &[false, true] {
        for &b in &[0, 1] {
            go(
                &format!("{label} [{}b{b}]", if w { "W" } else { "R" }),
                w,
                &[],
                true,
                Some(b),
                None,
                body,
            );
        }
    }
}

/// A single diff that only needs the two libraries (no png_struct at all).
fn solo(label: &str, body: &dyn Fn(&Core, &Lib)) {
    diff(label, |lib| {
        session_reset(Vec::new());
        let c = Core::new(lib);
        let rc = protected(|| body(&c, lib));
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}

// ===========================================================================
// 1. version strings, struct creation, png_info_init_3
// ===========================================================================

#[test]
fn version_and_struct() {
    // png.c:206 png_user_version_check: the version string must match through
    // the second '.'; anything else warns and the create call returns NULL.
    let vers: [(&str, Option<&str>); 9] = [
        ("ok", Some("1.6.59.git")),
        ("same-minor", Some("1.6.58")),
        ("short", Some("1.6")),
        ("empty", Some("")),
        ("minor", Some("1.5.59")),
        ("major", Some("2.6.59.git")),
        ("garbage", Some("xyz")),
        ("dots", Some("...")),
        ("null", None),
    ];
    for (tag, v) in vers {
        let vb = v.map(cz);
        for &w in &[false, true] {
            for &two in &[false, true] {
                let label = format!(
                    "V1 create_{}{} ver={tag}",
                    if w { "write" } else { "read" },
                    if two { "_2" } else { "" }
                );
                let vp = vb
                    .as_ref()
                    .map(|b| p(b))
                    .unwrap_or(std::ptr::null::<c_char>());
                diff(&label, |lib| {
                    session_reset(Vec::new());
                    let c = Core::new(lib);
                    let rc = protected(|| unsafe {
                        let png = if two {
                            let f = if w { c.create_write_2 } else { c.create_read_2 };
                            f(
                                vp,
                                NP,
                                cb_error as Cb,
                                cb_warning as Cb,
                                NP,
                                cb_malloc as Cb,
                                cb_free as Cb,
                            )
                        } else {
                            let f = if w { c.create_write } else { c.create_read };
                            f(vp, NP, cb_error as Cb, cb_warning as Cb)
                        };
                        log(format!("create={}", (!png.is_null()) as u8));
                        if png.is_null() {
                            return;
                        }
                        (c.set_longjmp)(png, shim().longjmp_ptr, shim().jmp_buf_size);
                        log(format!(
                            "libpng_ver={} header_ver={}",
                            cstr((c.get_libpng_ver)(png)),
                            cstr((c.get_header_ver)(png))
                        ));
                        let mut pp = png;
                        let mut ii: Info = NP;
                        if w {
                            (c.destroy_write)(&mut pp, &mut ii);
                        } else {
                            (c.destroy_read)(&mut pp, &mut ii, NPP);
                        }
                        log("destroyed".to_string());
                    });
                    Trace {
                        lines: take_log(),
                        out: take_out(),
                        rc,
                    }
                });
            }
        }
    }

    // png.c:365 png_create_info_struct(NULL) -> NULL, no message.
    solo("V2 create_info(NULL)", &|c, _| unsafe {
        let i = (c.create_info)(NP);
        log(format!("info={}", (!i.is_null()) as u8));
    });

    // png.c:435 png_destroy_info_struct with a NULL png_ptr / NULL handle.
    solo("V3 destroy_info(NULL,NULL)", &|c, _| unsafe {
        (c.destroy_info)(NP, NPP);
        log("ok".to_string());
    });

    // png.c:770 png_destroy_read/write_struct with NULL handles.
    solo("V4 destroy_read(NULL)", &|c, _| unsafe {
        (c.destroy_read)(NPP, NPP, NPP);
        (c.destroy_write)(NPP, NPP);
        log("ok".to_string());
    });

    // png_access_version_number / copyright / header strings on a NULL struct.
    solo("V5 version strings", &|c, _| unsafe {
        log(format!("num={}", (c.access_version_number)()));
        log(format!("copyright={}", cstr((c.get_copyright)(NP))));
        log(format!("libpng_ver={}", cstr((c.get_libpng_ver)(NP))));
        log(format!("header_ver={}", cstr((c.get_header_ver)(NP))));
        log(format!("header_version={}", cstr((c.get_header_version)(NP))));
    });
}

#[test]
fn info_init_3_sizes() {
    // png.c:437 png_info_init_3: when sizeof(png_info) is larger than the size
    // the application passes, the info struct is reallocated (and the old one
    // released with the *system* free).  Only the null-ness / identity of the
    // pointer is logged, never its value.
    for &size in &[0usize, 1, 8, 64, 4096, 1 << 20] {
        dr(&format!("V6 info_init_3 size={size}"), &|c, png, _i, _l| unsafe {
            let mut info = (c.create_info)(png);
            let before = info;
            sub("init", || (c.info_init_3)(&mut info, size));
            log(format!(
                "info_null={} changed={}",
                info.is_null() as u8,
                (info != before) as u8
            ));
            if !info.is_null() {
                st(c, png, info);
                let mut ii = info;
                (c.destroy_info)(png, &mut ii);
            }
        });
    }
    // A handle that points at NULL: nothing must happen.
    dr("V7 info_init_3 *ptr=NULL", &|c, _png, _i, _l| unsafe {
        let mut info: Info = NP;
        (c.info_init_3)(&mut info, 4096);
        log(format!("info_null={}", info.is_null() as u8));
    });
}

// ===========================================================================
// 2. the exported error/warning API
// ===========================================================================

#[test]
fn error_and_warning_api() {
    let msg = cz("hello");
    let empty = cz("");

    // pngerror.c:177 png_warning with a valid and with a NULL png_ptr (the
    // latter goes to png_default_warning -> stderr, and must not crash).
    duo("E1 png_warning", &|c, png, _i, _l| unsafe {
        (c.warning)(png, p(&cz("plain warning")));
        log("after".to_string());
    });
    solo("E2 png_warning(NULL)", &|c, _| unsafe {
        (c.warning)(NP, p(&cz("null warning")));
        log("after".to_string());
    });
    solo("E3 png_warning(png,NULL)", &|_c, lib| unsafe {
        // NULL message with a NULL struct: png_default_warning prints it.
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_warning");
        let _ = f;
        log("skipped-null-message".to_string());
    });

    // pngerror.c:39 png_error: fatal, the harness error callback returns so the
    // library longjmps.
    duo("E4 png_error", &|c, png, _i, _l| unsafe {
        sub("err", || (c.error)(png, p(&cz("explicit error"))));
        log("survived".to_string());
    });
    duo("E5 png_error empty msg", &|c, png, _i, _l| unsafe {
        sub("err", || (c.error)(png, p(&empty)));
    });

    // pngerror.c:426 png_chunk_error / :443 png_chunk_warning.  chunk_name is 0
    // on a fresh struct, so the message is prefixed with "[00][00][00][00]".
    duo("E6 png_chunk_warning", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_chunk_warning");
        f(png, p(&msg));
        log("after".to_string());
    });
    duo("E7 png_chunk_error", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_chunk_error");
        sub("cerr", || f(png, p(&msg)));
    });
    solo("E8 png_chunk_warning(NULL)", &|_c, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_chunk_warning");
        f(NP, p(&cz("null chunk warning")));
        log("after".to_string());
    });
    // After png_read_info the chunk name is IEND, so the prefix changes.
    let good = img(2, 2, 8, 0);
    di("E9 png_chunk_warning after read", &good, &|c, png, i, lib| unsafe {
        (c.read_info)(png, i);
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_chunk_warning");
        f(png, p(&msg));
        (c.read_end)(png, i);
        f(png, p(&msg));
    });

    // pngerror.c:308 png_benign_error, :338 png_app_warning, :351 png_app_error
    // -- warning or fatal according to the benign-errors flag.
    quad("E10 png_benign_error", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_benign_error");
        sub("benign", || f(png, p(&msg)));
        log("after".to_string());
    });
    quad("E11 png_app_warning", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_app_warning");
        sub("appwarn", || f(png, p(&msg)));
        log("after".to_string());
    });
    quad("E12 png_app_error", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_app_error");
        sub("apperr", || f(png, p(&msg)));
        log("after".to_string());
    });
    quad("E13 png_chunk_benign_error", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_chunk_benign_error");
        sub("cbenign", || f(png, p(&msg)));
        log("after".to_string());
    });

    // pngerror.c:477 png_chunk_report: the `error` argument selects the
    // severity; C enums accept any int, so out-of-range values are covered too.
    for sel in [PNG_CHUNK_WARNING, PNG_CHUNK_WRITE_ERROR, PNG_CHUNK_ERROR, 3, 99, -1, -99] {
        quad(&format!("E14 png_chunk_report sel={sel}"), &|_c, png, _i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, *const c_char, c_int) = lib.f("png_chunk_report");
            sub("report", || f(png, p(&cz("reported")), sel));
            log("after".to_string());
        });
    }

    // pngerror.c:672 png_longjmp with a jmp_buf installed: rc is the value.
    for v in [1, 5, -3, 0] {
        duo(&format!("E15 png_longjmp val={v}"), &|c, png, _i, _l| unsafe {
            sub("lj", || (c.longjmp)(png, v));
            log("after".to_string());
        });
    }

    // pngerror.c:718 png_set_error_fn(NULL) / png_get_error_ptr(NULL).
    solo("E16 error_fn NULL struct", &|c, _| unsafe {
        (c.set_error_fn)(NP, NP, cb_error as Cb, cb_warning as Cb);
        log(format!(
            "error_ptr={}",
            (!(c.get_error_ptr)(NP).is_null()) as u8
        ));
    });
    // Removing the error handler entirely is *not* tested with png_error: the
    // default handler would abort() the test process (pngerror.c:668).
    dr("E17 error_fn set to NULL then warning", &|c, png, _i, _l| unsafe {
        (c.set_error_fn)(png, NP, cb_error as Cb, NP as Cb);
        (c.warning)(png, p(&msg)); // default warning -> stderr
        log("after".to_string());
        (c.set_error_fn)(png, NP, cb_error as Cb, cb_warning as Cb);
    });
}

#[test]
fn longjmp_fn_api() {
    let n = shim().jmp_buf_size;
    // pngerror.c:544 png_set_longjmp_fn.
    solo("E18 set_longjmp(NULL)", &|c, _| unsafe {
        let r = (c.set_longjmp)(NP, shim().longjmp_ptr, shim().jmp_buf_size);
        log(format!("ret={}", (!r.is_null()) as u8));
    });
    // Sizes at, below and above sizeof(jmp_buf); `go` already installed the
    // exact size once, so these are all "already allocated" paths.
    for &(tag, sz) in &[
        ("same", 0usize),
        ("zero", 1),
        ("small", 2),
        ("exact", 3),
        ("big", 4),
    ] {
        let size = match sz {
            0 => n,
            1 => 0,
            2 => n / 2,
            3 => n,
            _ => n + 64,
        };
        duo(
            &format!("E19 set_longjmp again {tag} ({size} vs {n})"),
            &|c, png, _i, _l| unsafe {
                sub("set", || {
                    let r = (c.set_longjmp)(png, shim().longjmp_ptr, size);
                    log(format!("ret={}", (!r.is_null()) as u8));
                });
                // The jmp_buf must still work afterwards.
                sub("still-works", || (c.error)(png, p(&cz("after set_longjmp"))));
            },
        );
    }
    // A fresh struct whose first png_set_longjmp_fn call asks for a huge buffer
    // (heap allocated), then a second call with a different size -> warning.
    for &first in &[0usize, 8, 200, 1 << 12] {
        diff(&format!("E20 fresh set_longjmp first={first}"), |lib| {
            session_reset(Vec::new());
            let c = Core::new(lib);
            let rc = protected(|| unsafe {
                let png = (c.create_read)(
                    VER_STRING.as_ptr() as *const c_char,
                    NP,
                    cb_error as Cb,
                    cb_warning as Cb,
                );
                if png.is_null() {
                    log("create=0".to_string());
                    return;
                }
                let r1 = (c.set_longjmp)(png, shim().longjmp_ptr, first);
                log(format!("r1={}", (!r1.is_null()) as u8));
                let r2 = (c.set_longjmp)(png, shim().longjmp_ptr, first + 1);
                log(format!("r2={}", (!r2.is_null()) as u8));
                let r3 = (c.set_longjmp)(png, shim().longjmp_ptr, first);
                log(format!("r3={}", (!r3.is_null()) as u8));
                // Reinstall the real pad before doing anything fatal.
                let r4 = (c.set_longjmp)(png, shim().longjmp_ptr, first);
                log(format!("r4={}", (!r4.is_null()) as u8));
                let mut pp = png;
                let mut ii: Info = NP;
                (c.destroy_read)(&mut pp, &mut ii, NPP);
                log("destroyed".to_string());
            });
            Trace {
                lines: take_log(),
                out: take_out(),
                rc,
            }
        });
    }
}

// ===========================================================================
// 3. colour-space setters
// ===========================================================================

#[test]
fn colorspace_rejections() {
    // ---- gAMA (pngset.c:361) - no validation, values are stored verbatim.
    for v in [0i32, -1, 1, 2_147_483_647, -2_147_483_648] {
        duo(&format!("C1 gAMA_fixed {v}"), &|c, png, i, _l| unsafe {
            (c.set_gAMA_fixed)(png, i, v);
            st(c, png, i);
        });
    }
    // png.c:2726 png_fixed: out of range double -> "fixed point overflow in ..."
    for v in [0.0f64, -1.0, 1e10, -1e10, f64::INFINITY] {
        duo(&format!("C2 gAMA {v}"), &|c, png, i, _l| unsafe {
            sub("set", || (c.set_gAMA)(png, i, v));
            st(c, png, i);
        });
    }

    // ---- cHRM (pngset.c:39) - also unvalidated.
    duo("C3 cHRM_fixed zeros", &|c, png, i, _l| unsafe {
        (c.set_cHRM_fixed)(png, i, 0, 0, 0, 0, 0, 0, 0, 0);
        st(c, png, i);
    });
    duo("C4 cHRM_fixed negative", &|c, png, i, _l| unsafe {
        (c.set_cHRM_fixed)(png, i, -1, -1, -1, -1, -1, -1, -1, -1);
        st(c, png, i);
    });
    duo("C5 cHRM_fixed extremes", &|c, png, i, _l| unsafe {
        (c.set_cHRM_fixed)(
            png,
            i,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
        );
        st(c, png, i);
    });
    duo("C6 cHRM double overflow", &|c, png, i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, Info, f64, f64, f64, f64, f64, f64, f64, f64) =
            lib.f("png_set_cHRM");
        sub("set", || f(png, i, 1e9, 0.3, 0.6, 0.3, 0.1, 0.6, 0.15, 0.06));
        st(c, png, i);
    });

    // ---- pngset.c:94 "invalid cHRM XYZ" (png_xy_from_XYZ fails)
    let xyz_cases: [(&str, [i32; 9]); 5] = [
        ("zeros", [0; 9]),
        ("negative", [-100000; 9]),
        ("zero-sum", [1, -1, 0, 1, -1, 0, 1, -1, 0]),
        ("huge", [i32::MAX; 9]),
        ("valid", [
            64000, 33000, 3000, 30000, 60000, 10000, 15000, 6000, 79000,
        ]),
    ];
    for (tag, v) in xyz_cases {
        quad(&format!("C7 cHRM_XYZ_fixed {tag}"), &|c, png, i, _l| unsafe {
            sub("set", || {
                (c.set_cHRM_XYZ_fixed)(
                    png, i, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
                )
            });
            st(c, png, i);
        });
    }
    duo("C8 cHRM_XYZ double zeros", &|c, png, i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, Info, f64, f64, f64, f64, f64, f64, f64, f64, f64) =
            lib.f("png_set_cHRM_XYZ");
        sub("set", || f(png, i, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        st(c, png, i);
    });

    // ---- sRGB (pngset.c:850): the intent is stored unchecked.
    for v in [-1, 0, 3, 4, 99, i32::MAX] {
        duo(&format!("C9 sRGB intent={v}"), &|c, png, i, _l| unsafe {
            (c.set_sRGB)(png, i, v);
            st(c, png, i);
        });
    }
    for v in [-1, 4, 99] {
        duo(
            &format!("C10 sRGB_gAMA_and_cHRM intent={v}"),
            &|c, png, i, _l| unsafe {
                (c.set_sRGB_gAMA_and_cHRM)(png, i, v);
                st(c, png, i);
            },
        );
    }
    // sRGB after iCCP and iCCP after sRGB (both accepted by pngset).
    let prof = icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 2);
    let nm = cz("ICC");
    duo("C11 sRGB then iCCP", &|c, png, i, _l| unsafe {
        (c.set_sRGB)(png, i, 0);
        sub("iCCP", || {
            (c.set_iCCP)(png, i, p(&nm), 0, prof.as_ptr(), prof.len() as u32)
        });
        st(c, png, i);
    });
    duo("C12 iCCP then sRGB", &|c, png, i, _l| unsafe {
        sub("iCCP", || {
            (c.set_iCCP)(png, i, p(&nm), 0, prof.as_ptr(), prof.len() as u32)
        });
        (c.set_sRGB_gAMA_and_cHRM)(png, i, 1);
        st(c, png, i);
    });

    // ---- pngset.c:904 "Invalid iCCP compression method"
    for ct in [-1, 1, 2, 99] {
        quad(&format!("C13 iCCP comp={ct}"), &|c, png, i, _l| unsafe {
            sub("set", || {
                (c.set_iCCP)(png, i, p(&nm), ct, prof.as_ptr(), prof.len() as u32)
            });
            st(c, png, i);
        });
    }
    // NULL name / NULL profile: silently ignored.
    duo("C14 iCCP NULL name", &|c, png, i, _l| unsafe {
        (c.set_iCCP)(png, i, std::ptr::null(), 0, prof.as_ptr(), 132);
        st(c, png, i);
    });
    duo("C15 iCCP NULL profile", &|c, png, i, _l| unsafe {
        (c.set_iCCP)(png, i, p(&nm), 0, std::ptr::null(), 132);
        st(c, png, i);
    });
    duo("C16 iCCP proflen=0", &|c, png, i, _l| unsafe {
        (c.set_iCCP)(png, i, p(&nm), 0, prof.as_ptr(), 0);
        st(c, png, i);
    });

    // pngset.c:911 / :923 - the two allocation failures inside png_set_iCCP.
    for k in 1..=2usize {
        quad_mem(&format!("C17 iCCP oom k={k}"), &|c, png, i, _l| unsafe {
            arm(k);
            sub("set", || {
                (c.set_iCCP)(png, i, p(&nm), 0, prof.as_ptr(), prof.len() as u32)
            });
            disarm();
            st(c, png, i);
        });
    }

    // ---- pngset.c:136 png_set_cICP: matrix coefficients must be 0.
    for mc in [0u8, 1, 255] {
        duo(&format!("C18 cICP matrix={mc}"), &|c, png, i, _l| unsafe {
            (c.set_cICP)(png, i, 1, 13, mc, 1);
            st(c, png, i);
        });
    }

    // ---- pngset.c:182 "cLLI light level exceeds PNG limit"
    for &(a, b) in &[
        (0u32, 0u32),
        (0x7FFF_FFFF, 0x7FFF_FFFF),
        (0x8000_0000, 0),
        (0, 0x8000_0000),
        (0xFFFF_FFFF, 0xFFFF_FFFF),
    ] {
        quad(
            &format!("C19 cLLI_fixed {a:#x} {b:#x}"),
            &|c, png, i, _l| unsafe {
                sub("set", || (c.set_cLLI_fixed)(png, i, a, b));
                st(c, png, i);
            },
        );
    }
    for &(a, b) in &[(0.0f64, 0.0f64), (1e9, 1.0), (-1.0, 1.0)] {
        quad(&format!("C20 cLLI {a} {b}"), &|c, png, i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, Info, f64, f64) = lib.f("png_set_cLLI");
            sub("set", || f(png, i, a, b));
            st(c, png, i);
        });
    }

    // ---- pngset.c:254 "mDCV chromaticities outside representable range"
    let mdcv: [(&str, [i32; 8], u32, u32); 5] = [
        ("ok", [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000], 1000, 1),
        ("neg-red", [31270, 32900, -1, 33000, 30000, 60000, 15000, 6000], 1000, 1),
        ("big-red", [31270, 32900, 200000, 33000, 30000, 60000, 15000, 6000], 1000, 1),
        ("maxDL", [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000], 0x8000_0000, 1),
        ("minDL", [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000], 1, 0x8000_0000),
    ];
    for (tag, v, maxdl, mindl) in mdcv {
        quad(&format!("C21 mDCV_fixed {tag}"), &|c, png, i, _l| unsafe {
            sub("set", || {
                (c.set_mDCV_fixed)(
                    png, i, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], maxdl, mindl,
                )
            });
            st(c, png, i);
        });
    }
    quad("C22 mDCV double out of range", &|c, png, i, lib| unsafe {
        type F = unsafe extern "C" fn(Png, Info, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64);
        let f: F = lib.f("png_set_mDCV");
        sub("set", || {
            f(png, i, 0.3127, 0.329, 5.0, 0.33, 0.3, 0.6, 0.15, 0.06, 1000.0, 0.1)
        });
        st(c, png, i);
    });
}

/// Build a 132-byte ICC profile header with the given fields.
#[allow(clippy::too_many_arguments)]
fn icc_profile(
    len: u32,
    class: &[u8; 4],
    space: &[u8; 4],
    pcs: &[u8; 4],
    sig: &[u8; 4],
    intent: u32,
    tags: u32,
    d50: bool,
    ver: u8,
) -> Vec<u8> {
    const D50: [u8; 12] = [
        0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
    ];
    let mut v = vec![0u8; 132];
    v[0..4].copy_from_slice(&len.to_be_bytes());
    v[8] = ver;
    v[12..16].copy_from_slice(class);
    v[16..20].copy_from_slice(space);
    v[20..24].copy_from_slice(pcs);
    v[36..40].copy_from_slice(sig);
    v[64..68].copy_from_slice(&intent.to_be_bytes());
    if d50 {
        v[68..80].copy_from_slice(&D50);
    } else {
        v[68..80].copy_from_slice(&[0xffu8; 12]);
    }
    v[128..132].copy_from_slice(&tags.to_be_bytes());
    v
}

#[test]
fn icc_check_api() {
    let nm = cz("prof");
    // png.c:1594 png_icc_check_length: "too short" / "profile too long".
    for len in [0u32, 1, 131, 132, 133] {
        quad(&format!("I1 icc_check_length {len}"), &|_c, png, _i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, *const c_char, u32) -> c_int =
                lib.f("png_icc_check_length");
            sub("chk", || log(format!("ret={}", f(png, p(&nm), len))));
        });
    }
    // png_chunk_max() is png_ptr->user_chunk_malloc_max, so a small limit makes
    // the "profile too long" branch reachable (png.c:1607).
    for &(lim, len) in &[(200usize, 300u32), (8_000_000, 9_000_000), (200, 150)] {
        quad(
            &format!("I2 icc_check_length lim={lim} len={len}"),
            &|c, png, _i, lib| unsafe {
                (c.set_chunk_malloc_max)(png, lim);
                let f: unsafe extern "C" fn(Png, *const c_char, u32) -> c_int =
                    lib.f("png_icc_check_length");
                sub("chk", || log(format!("ret={}", f(png, p(&nm), len))));
            },
        );
    }

    // png.c:1614 png_icc_check_header - one diff per malformed field.
    struct H(&'static str, Vec<u8>, u32, c_int);
    let cases: Vec<H> = vec![
        H("valid-gray", icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 0),
        H("valid-rgb", icc_profile(132, b"mntr", b"RGB ", b"Lab ", b"ascp", 0, 0, true, 2), 132, 2),
        H("len-mismatch", icc_profile(140, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 0),
        H("invalid-length", icc_profile(133, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 4), 133, 0),
        H("tagcount-trunc", icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0, 1, true, 2), 132, 0),
        H("tagcount-huge", icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0, 400_000_000, true, 2), 132, 0),
        H("intent-invalid", icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0x1_0000, 0, true, 2), 132, 0),
        H("intent-range", icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"ascp", 4, 0, true, 2), 132, 0),
        H("bad-signature", icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"scpa", 0, 0, true, 2), 132, 0),
        H("not-d50", icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0, 0, false, 2), 132, 0),
        H("gray-on-rgb", icc_profile(132, b"mntr", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 2),
        H("rgb-on-gray", icc_profile(132, b"mntr", b"RGB ", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 0),
        H("bad-space", icc_profile(132, b"mntr", b"CMYK", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 0),
        H("abstract", icc_profile(132, b"abst", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 0),
        H("devlink", icc_profile(132, b"link", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 0),
        H("namedcolor", icc_profile(132, b"nmcl", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 0),
        H("unknown-class", icc_profile(132, b"zzzz", b"GRAY", b"XYZ ", b"ascp", 0, 0, true, 2), 132, 0),
        H("bad-pcs", icc_profile(132, b"mntr", b"GRAY", b"CMYK", b"ascp", 0, 0, true, 2), 132, 0),
    ];
    for H(tag, prof, len, ct) in cases {
        quad(&format!("I3 icc_check_header {tag}"), &|_c, png, _i, lib| unsafe {
            type F = unsafe extern "C" fn(Png, *const c_char, u32, *const u8, c_int) -> c_int;
            let f: F = lib.f("png_icc_check_header");
            sub("chk", || {
                log(format!("ret={}", f(png, p(&nm), len, prof.as_ptr(), ct)))
            });
        });
    }
}

// ===========================================================================
// 4. png_check_IHDR (png.c:1961) reached through png_set_IHDR
// ===========================================================================

#[test]
fn ihdr_rejections() {
    // (w, h, depth, colour, interlace, compression, filter)
    let cases: [(&str, u32, u32, c_int, c_int, c_int, c_int, c_int); 16] = [
        ("valid", 1, 1, 8, 0, 0, 0, 0),
        ("w0", 0, 1, 8, 0, 0, 0, 0),
        ("h0", 1, 0, 8, 0, 0, 0, 0),
        ("w-huge", 0x8000_0000, 1, 8, 0, 0, 0, 0),
        ("h-huge", 1, 0x8000_0000, 8, 0, 0, 0, 0),
        ("depth3", 1, 1, 3, 0, 0, 0, 0),
        ("depth0", 1, 1, 0, 0, 0, 0, 0),
        ("depth32", 1, 1, 32, 0, 0, 0, 0),
        ("ct1", 1, 1, 8, 1, 0, 0, 0),
        ("ct5", 1, 1, 8, 5, 0, 0, 0),
        ("ct7", 1, 1, 8, 7, 0, 0, 0),
        ("ct-neg", 1, 1, 8, -1, 0, 0, 0),
        ("pal16", 1, 1, 16, 3, 0, 0, 0),
        ("rgb4", 1, 1, 4, 2, 0, 0, 0),
        ("ga1", 1, 1, 1, 4, 0, 0, 0),
        ("rgba2", 1, 1, 2, 6, 0, 0, 0),
    ];
    for (tag, w, h, d, ct, il, cm, fm) in cases {
        duo(&format!("H1 set_IHDR {tag}"), &|c, png, i, _l| unsafe {
            sub("set", || (c.set_IHDR)(png, i, w, h, d, ct, il, cm, fm));
            st(c, png, i);
        });
    }
    for il in [2, 3, 99, -1] {
        duo(&format!("H2 set_IHDR interlace={il}"), &|c, png, i, _l| unsafe {
            sub("set", || (c.set_IHDR)(png, i, 1, 1, 8, 0, il, 0, 0));
            st(c, png, i);
        });
    }
    for cm in [1, 99, -1] {
        duo(&format!("H3 set_IHDR compression={cm}"), &|c, png, i, _l| unsafe {
            sub("set", || (c.set_IHDR)(png, i, 1, 1, 8, 0, 0, cm, 0));
            st(c, png, i);
        });
    }
    // png.c:2115 "Unknown filter method in IHDR" (and :2109 for filter 64 after
    // a signature has been seen).
    for fm in [1, 64, 99, -1] {
        duo(&format!("H4 set_IHDR filter={fm}"), &|c, png, i, _l| unsafe {
            sub("set", || (c.set_IHDR)(png, i, 1, 1, 8, 2, 0, 0, fm));
            st(c, png, i);
        });
    }
    // MNG intrapixel differencing is accepted on a write struct before the
    // signature is written, refused afterwards; and png.c:2091 "MNG features
    // are not allowed in a PNG datastream" needs mng_features_permitted != 0
    // together with PNG_HAVE_PNG_SIGNATURE.
    for &(tag, sig, mng, fm) in &[
        ("mng64-nosig", false, PNG_ALL_MNG_FEATURES, 64),
        ("mng64-sig", true, PNG_ALL_MNG_FEATURES, 64),
        ("mng0-sig", true, PNG_ALL_MNG_FEATURES, 0),
        ("nomng-sig", true, 0u32, 64),
        ("mng-none-nosig", false, 0, 64),
    ] {
        dw(&format!("H5 set_IHDR {tag}"), &|c, png, i, _l| unsafe {
            log(format!("mng={}", (c.permit_mng_features)(png, mng)));
            if sig {
                (c.write_sig)(png);
            }
            sub("set", || (c.set_IHDR)(png, i, 1, 1, 8, 2, 0, 0, fm));
            st(c, png, i);
        });
    }
    // The user limits are consulted by png_check_IHDR.
    for &(uw, uh) in &[(0u32, 0u32), (1, 1), (4, 4)] {
        duo(
            &format!("H6 set_IHDR user limits {uw}x{uh}"),
            &|c, png, i, _l| unsafe {
                (c.set_user_limits)(png, uw, uh);
                log(format!(
                    "limits {} {}",
                    (c.get_user_width_max)(png),
                    (c.get_user_height_max)(png)
                ));
                sub("set", || (c.set_IHDR)(png, i, 2, 2, 8, 0, 0, 0, 0));
                st(c, png, i);
            },
        );
    }
    // png_set_IHDR with a NULL info / NULL png_ptr.
    dw("H7 set_IHDR NULL info", &|c, png, _i, _l| unsafe {
        (c.set_IHDR)(png, NP, 1, 1, 8, 0, 0, 0, 0);
        log("ok".to_string());
    });
    solo("H8 set_IHDR NULL png", &|c, _| unsafe {
        (c.set_IHDR)(NP, NP, 1, 1, 8, 0, 0, 0, 0);
        log("ok".to_string());
    });
}

// ===========================================================================
// 5. pngset.c chunk-setter validation
// ===========================================================================

#[test]
fn set_chunk_rejections() {
    let pal = vec![0x11u8; 3 * 300];
    let trans = vec![0x22u8; 300];

    // ---- pngset.c:750 png_set_PLTE: fatal for a palette image, a warning
    // otherwise ("Invalid palette length"), plus "Invalid palette" for a NULL
    // or empty palette.
    for &(ct, bd, n) in &[
        (3i32, 1u8, 3i32),
        (3, 1, 2),
        (3, 2, 5),
        (3, 8, 257),
        (3, 8, 256),
        (3, 8, -1),
        (2, 8, 257),
        (2, 8, 300),
        (2, 8, -1),
        (0, 8, 257),
    ] {
        duo(
            &format!("S1 set_PLTE ct={ct} bd={bd} n={n}"),
            &|c, png, i, _l| unsafe {
                (c.set_IHDR)(png, i, 1, 1, bd as c_int, ct, 0, 0, 0);
                sub("plte", || (c.set_PLTE)(png, i, pal.as_ptr(), n));
                st(c, png, i);
            },
        );
    }
    duo("S2 set_PLTE NULL palette", &|c, png, i, _l| unsafe {
        (c.set_IHDR)(png, i, 1, 1, 8, 3, 0, 0, 0);
        sub("plte", || (c.set_PLTE)(png, i, std::ptr::null(), 4));
        st(c, png, i);
    });
    duo("S3 set_PLTE n=0", &|c, png, i, _l| unsafe {
        (c.set_IHDR)(png, i, 1, 1, 8, 3, 0, 0, 0);
        sub("plte", || (c.set_PLTE)(png, i, pal.as_ptr(), 0));
        st(c, png, i);
    });
    // MNG empty PLTE is permitted when the feature flag is on.
    duo("S4 set_PLTE n=0 mng", &|c, png, i, _l| unsafe {
        (c.permit_mng_features)(png, PNG_ALL_MNG_FEATURES);
        (c.set_IHDR)(png, i, 1, 1, 8, 3, 0, 0, 0);
        sub("plte", || (c.set_PLTE)(png, i, pal.as_ptr(), 0));
        st(c, png, i);
    });

    // ---- pngset.c:1182 png_set_tRNS
    let tc = PngColor16 {
        index: 0,
        red: 300,
        green: 4,
        blue: 5,
        gray: 300,
    };
    for &(ct, bd) in &[(0i32, 1u8), (0, 8), (0, 16), (2, 8), (2, 16), (3, 8)] {
        duo(
            &format!("S5 set_tRNS out-of-range ct={ct} bd={bd}"),
            &|c, png, i, _l| unsafe {
                (c.set_IHDR)(png, i, 1, 1, bd as c_int, ct, 0, 0, 0);
                sub("trns", || {
                    (c.set_tRNS)(png, i, std::ptr::null(), 1, &tc as *const _ as *const u8)
                });
                st(c, png, i);
            },
        );
    }
    for n in [-1i32, 0, 1, 256, 257, 1000] {
        duo(&format!("S6 set_tRNS num={n}"), &|c, png, i, _l| unsafe {
            (c.set_IHDR)(png, i, 1, 1, 8, 3, 0, 0, 0);
            sub("trns", || (c.set_tRNS)(png, i, trans.as_ptr(), n, NP as *const u8));
            st(c, png, i);
        });
    }
    duo("S7 set_tRNS both NULL", &|c, png, i, _l| unsafe {
        (c.set_tRNS)(png, i, std::ptr::null(), 5, std::ptr::null());
        st(c, png, i);
    });

    // ---- pngset.c:835 png_set_sBIT: no validation at all in 1.6.
    for &(bd, ct, sb) in &[
        (8u8, 0i32, [0u8, 0, 0, 0, 0]),
        (8, 0, [0, 0, 0, 9, 0]),
        (8, 2, [9, 9, 9, 0, 0]),
        (8, 6, [8, 8, 8, 0, 9]),
        (1, 0, [0, 0, 0, 2, 0]),
    ] {
        let v = PngColor8 {
            red: sb[0],
            green: sb[1],
            blue: sb[2],
            gray: sb[3],
            alpha: sb[4],
        };
        duo(
            &format!("S8 set_sBIT bd={bd} ct={ct} {sb:?}"),
            &|c, png, i, _l| unsafe {
                (c.set_IHDR)(png, i, 1, 1, bd as c_int, ct, 0, 0, 0);
                (c.set_sBIT)(png, i, &v as *const _ as *const u8);
                st(c, png, i);
            },
        );
    }
    duo("S9 set_sBIT NULL", &|c, png, i, _l| unsafe {
        (c.set_sBIT)(png, i, std::ptr::null());
        st(c, png, i);
    });

    // ---- pngset.c:385 png_set_hIST: needs a palette first.
    let hist = vec![7u16; 300];
    duo("S10 set_hIST no palette", &|c, png, i, _l| unsafe {
        sub("hist", || (c.set_hIST)(png, i, hist.as_ptr()));
        st(c, png, i);
    });
    duo("S11 set_hIST NULL", &|c, png, i, _l| unsafe {
        (c.set_hIST)(png, i, std::ptr::null());
        st(c, png, i);
    });
    duo("S12 set_hIST with palette", &|c, png, i, _l| unsafe {
        (c.set_IHDR)(png, i, 1, 1, 8, 3, 0, 0, 0);
        (c.set_PLTE)(png, i, pal.as_ptr(), 4);
        sub("hist", || (c.set_hIST)(png, i, hist.as_ptr()));
        st(c, png, i);
    });
    // pngset.c:422 "Insufficient memory for hIST chunk data"
    quad_mem("S13 set_hIST oom", &|c, png, i, _l| unsafe {
        (c.set_IHDR)(png, i, 1, 1, 8, 3, 0, 0, 0);
        (c.set_PLTE)(png, i, pal.as_ptr(), 4);
        arm(1);
        sub("hist", || (c.set_hIST)(png, i, hist.as_ptr()));
        disarm();
        st(c, png, i);
    });

    // ---- pngset.c:734 png_set_pHYs / :476 png_set_oFFs: any unit is stored.
    for u in [-1, 0, 1, 2, 99] {
        duo(&format!("S14 set_pHYs unit={u}"), &|c, png, i, _l| unsafe {
            (c.set_pHYs)(png, i, 10, 20, u);
            st(c, png, i);
        });
        duo(&format!("S15 set_oFFs unit={u}"), &|c, png, i, _l| unsafe {
            (c.set_oFFs)(png, i, -5, 6, u);
            st(c, png, i);
        });
    }

    // ---- pngset.c:1156 png_set_tIME / png.c:802 png_convert_to_rfc1123
    let times: [(&str, PngTime); 8] = [
        ("valid", PngTime { year: 2026, month: 8, day: 14, hour: 12, minute: 30, second: 45 }),
        ("month0", PngTime { year: 2026, month: 0, day: 14, hour: 0, minute: 0, second: 0 }),
        ("month13", PngTime { year: 2026, month: 13, day: 1, hour: 0, minute: 0, second: 0 }),
        ("day0", PngTime { year: 2026, month: 1, day: 0, hour: 0, minute: 0, second: 0 }),
        ("day32", PngTime { year: 2026, month: 1, day: 32, hour: 0, minute: 0, second: 0 }),
        ("hour24", PngTime { year: 2026, month: 1, day: 1, hour: 24, minute: 0, second: 0 }),
        ("min60", PngTime { year: 2026, month: 1, day: 1, hour: 0, minute: 60, second: 0 }),
        ("sec61", PngTime { year: 2026, month: 1, day: 1, hour: 0, minute: 0, second: 61 }),
    ];
    for (tag, t) in times {
        duo(&format!("S16 set_tIME {tag}"), &|c, png, i, _l| unsafe {
            (c.set_tIME)(png, i, &t as *const _ as *const u8);
            st(c, png, i);
        });
        duo(&format!("S17 rfc1123 {tag}"), &|_c, png, _i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, *const u8) -> *const c_char =
                lib.f("png_convert_to_rfc1123");
            let r = f(png, &t as *const _ as *const u8);
            log(format!("rfc1123={}", cstr(r)));
        });
        solo(&format!("S18 rfc1123_buffer {tag}"), &|c, _| unsafe {
            let mut buf = [0u8; 40];
            let r = (c.convert_to_rfc1123_buffer)(buf.as_mut_ptr() as *mut c_char, &t as *const _ as *const u8);
            log(format!("rc={r} buf={}", cstr(buf.as_ptr() as *const c_char)));
        });
    }
    duo("S19 set_tIME NULL", &|c, png, i, _l| unsafe {
        (c.set_tIME)(png, i, std::ptr::null());
        st(c, png, i);
    });
    duo("S20 rfc1123 NULL time", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const u8) -> *const c_char =
            lib.f("png_convert_to_rfc1123");
        log(format!("r={}", cstr(f(png, std::ptr::null()))));
    });
    solo("S21 rfc1123_buffer NULL args", &|c, _| unsafe {
        let mut buf = [0u8; 40];
        log(format!(
            "rc={}",
            (c.convert_to_rfc1123_buffer)(buf.as_mut_ptr() as *mut c_char, std::ptr::null())
        ));
        let t = PngTime { year: 2026, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
        log(format!(
            "rc={}",
            (c.convert_to_rfc1123_buffer)(std::ptr::null_mut(), &t as *const _ as *const u8)
        ));
    });

    // ---- pngset.c:319 png_set_eXIf / :328 png_set_eXIf_1
    let exif = vec![b'M', b'M', 0, 42, 0, 0, 0, 8];
    duo("S22 set_eXIf (deprecated)", &|c, png, i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, Info, *mut u8) = lib.f("png_set_eXIf");
        f(png, i, exif.as_ptr() as *mut u8);
        st(c, png, i);
    });
    for n in [0u32, 1, 2, 8] {
        duo(&format!("S23 set_eXIf_1 len={n}"), &|c, png, i, _l| unsafe {
            (c.set_eXIf_1)(png, i, n, exif.as_ptr());
            st(c, png, i);
        });
    }
    duo("S24 set_eXIf_1 NULL", &|c, png, i, _l| unsafe {
        (c.set_eXIf_1)(png, i, 8, std::ptr::null());
        st(c, png, i);
    });
    // pngset.c:344 "Insufficient memory for eXIf chunk data"
    quad_mem("S25 set_eXIf_1 oom", &|c, png, i, _l| unsafe {
        arm(1);
        sub("exif", || (c.set_eXIf_1)(png, i, 8, exif.as_ptr()));
        disarm();
        st(c, png, i);
    });
}

#[test]
fn pcal_and_scal_rejections() {
    let purpose = cz("calibration purpose");
    let units = cz("metres");
    let ok1 = cz("1.5");
    let ok2 = cz("-2.5e3");
    let bad = cz("not a number");
    let mut params_ok: Vec<*mut c_char> =
        vec![ok1.as_ptr() as *mut c_char, ok2.as_ptr() as *mut c_char];
    let mut params_bad: Vec<*mut c_char> = vec![bad.as_ptr() as *mut c_char];
    let mut params_null: Vec<*mut c_char> = vec![std::ptr::null_mut()];
    let pok = params_ok.as_mut_ptr();
    let pbad = params_bad.as_mut_ptr();
    let pnull = params_null.as_mut_ptr();

    // ---- pngset.c:493 png_set_pCAL: equation type and parameter count.
    for t in [-1, 0, 3, 4, 99] {
        quad(&format!("P1 set_pCAL type={t}"), &|c, png, i, _l| unsafe {
            sub("pcal", || {
                (c.set_pCAL)(
                    png,
                    i,
                    p(&purpose),
                    0,
                    100,
                    t,
                    2,
                    p(&units),
                    pok,
                )
            });
            st(c, png, i);
        });
    }
    // pngset.c:522 "Invalid pCAL parameter count"
    for n in [-1, 256, 1000] {
        quad(&format!("P2 set_pCAL nparams={n}"), &|c, png, i, _l| unsafe {
            sub("pcal", || {
                (c.set_pCAL)(png, i, p(&purpose), 0, 100, 0, n, p(&units), std::ptr::null_mut())
            });
            st(c, png, i);
        });
    }
    quad("P3 set_pCAL bad param text", &|c, png, i, _l| unsafe {
        sub("pcal", || {
            (c.set_pCAL)(png, i, p(&purpose), 0, 1, 2, 1, p(&units), pbad)
        });
        st(c, png, i);
    });
    quad("P4 set_pCAL NULL param", &|c, png, i, _l| unsafe {
        sub("pcal", || {
            (c.set_pCAL)(png, i, p(&purpose), 0, 1, 2, 1, p(&units), pnull)
        });
        st(c, png, i);
    });
    duo("P5 set_pCAL NULL purpose", &|c, png, i, _l| unsafe {
        (c.set_pCAL)(png, i, std::ptr::null(), 0, 1, 0, 0, p(&units), std::ptr::null_mut());
        st(c, png, i);
    });
    duo("P6 set_pCAL NULL units", &|c, png, i, _l| unsafe {
        (c.set_pCAL)(png, i, p(&purpose), 0, 1, 0, 0, std::ptr::null(), std::ptr::null_mut());
        st(c, png, i);
    });
    duo("P7 set_pCAL NULL params n>0", &|c, png, i, _l| unsafe {
        (c.set_pCAL)(png, i, p(&purpose), 0, 1, 0, 2, p(&units), std::ptr::null_mut());
        st(c, png, i);
    });
    // pngset.c:544/:568/:579/:596 - the four allocation failures, in order:
    // purpose, units, the params array, one parameter.
    for k in 1..=4usize {
        quad_mem(&format!("P8 set_pCAL oom k={k}"), &|c, png, i, _l| unsafe {
            arm(k);
            sub("pcal", || {
                (c.set_pCAL)(png, i, p(&purpose), 0, 100, 2, 2, p(&units), pok)
            });
            disarm();
            st(c, png, i);
        });
    }

    // ---- pngset.c:609 png_set_sCAL_s
    let sw = cz("2.5");
    let sh = cz("3.5e2");
    let neg = cz("-1");
    let emp = cz("");
    let nan = cz("abc");
    for u in [-1, 0, 3, 99] {
        quad(&format!("P9 set_sCAL_s unit={u}"), &|c, png, i, _l| unsafe {
            sub("scal", || (c.set_sCAL_s)(png, i, u, p(&sw), p(&sh)));
            st(c, png, i);
        });
    }
    for (tag, w, h) in [
        ("wnull", std::ptr::null(), p(&sh)),
        ("wempty", p(&emp), p(&sh)),
        ("wneg", p(&neg), p(&sh)),
        ("wnan", p(&nan), p(&sh)),
        ("hnull", p(&sw), std::ptr::null()),
        ("hempty", p(&sw), p(&emp)),
        ("hneg", p(&sw), p(&neg)),
        ("hnan", p(&sw), p(&nan)),
        ("ok", p(&sw), p(&sh)),
    ] {
        quad(&format!("P10 set_sCAL_s {tag}"), &|c, png, i, _l| unsafe {
            sub("scal", || (c.set_sCAL_s)(png, i, 1, w, h));
            st(c, png, i);
        });
    }
    // pngset.c:644/:663 "Memory allocation failed while processing sCAL"
    for k in 1..=2usize {
        quad_mem(&format!("P11 set_sCAL_s oom k={k}"), &|c, png, i, _l| unsafe {
            arm(k);
            sub("scal", || (c.set_sCAL_s)(png, i, 2, p(&sw), p(&sh)));
            disarm();
            st(c, png, i);
        });
    }
    // pngset.c:682/:685 and :712/:715 "Invalid sCAL width/height ignored"
    for &(w, h) in &[
        (0.0f64, 1.0f64),
        (-1.0, 1.0),
        (1.0, 0.0),
        (1.0, -1.0),
        (0.0, 0.0),
        (1.0, 1.0),
    ] {
        quad(&format!("P12 set_sCAL {w} {h}"), &|c, png, i, _l| unsafe {
            sub("scal", || (c.set_sCAL)(png, i, 1, w, h));
            st(c, png, i);
        });
    }
    for &(w, h) in &[(0i32, FP1), (-1, FP1), (FP1, 0), (FP1, -1), (FP1, FP1)] {
        quad(
            &format!("P13 set_sCAL_fixed {w} {h}"),
            &|c, png, i, _l| unsafe {
                sub("scal", || (c.set_sCAL_fixed)(png, i, 1, w, h));
                st(c, png, i);
            },
        );
    }
    // An invalid unit reaches png_set_sCAL_s through the float/fixed wrappers.
    quad("P14 set_sCAL bad unit", &|c, png, i, _l| unsafe {
        sub("scal", || (c.set_sCAL)(png, i, 0, 1.0, 1.0));
        st(c, png, i);
    });
    quad("P15 set_sCAL_fixed bad unit", &|c, png, i, _l| unsafe {
        sub("scal", || (c.set_sCAL_fixed)(png, i, 7, FP1, FP1));
        st(c, png, i);
    });
}

#[test]
fn text_rejections() {
    let key = cz("Title");
    let txt = cz("some text");
    let lang = cz("en");
    let lkey = cz("Titel");
    let badkey = cz("bad\tkey");
    let spacekey = cz(" leading and  double  spaces ");
    let longkey = cz(&"k".repeat(100));
    let emptykey = cz("");

    let mk = |k: *const u8, t: *const u8, comp: c_int| PngText {
        compression: comp,
        key: k as *mut c_char,
        text: t as *mut c_char,
        text_length: 0,
        itxt_length: 0,
        lang: lang.as_ptr() as *mut c_char,
        lang_key: lkey.as_ptr() as *mut c_char,
    };

    // ---- pngset.c:1031 "text compression mode is out of range"
    for comp in [-2, -1, 0, 1, 2, 3, 4, 99] {
        let t = mk(key.as_ptr(), txt.as_ptr(), comp);
        quad(&format!("T1 set_text comp={comp}"), &|c, png, i, _l| unsafe {
            sub("text", || (c.set_text)(png, i, &t as *const _ as *const c_void, 1));
            st(c, png, i);
        });
    }
    // num_text <= 0 and a NULL array are silently ignored.
    for n in [-1, 0] {
        let t = mk(key.as_ptr(), txt.as_ptr(), -1);
        duo(&format!("T2 set_text num={n}"), &|c, png, i, _l| unsafe {
            (c.set_text)(png, i, &t as *const _ as *const c_void, n);
            st(c, png, i);
        });
    }
    duo("T3 set_text NULL array", &|c, png, i, _l| unsafe {
        (c.set_text)(png, i, std::ptr::null(), 1);
        st(c, png, i);
    });
    // A NULL key entry is skipped.
    let tk = mk(std::ptr::null(), txt.as_ptr(), -1);
    duo("T4 set_text NULL key", &|c, png, i, _l| unsafe {
        (c.set_text)(png, i, &tk as *const _ as *const c_void, 1);
        st(c, png, i);
    });
    let tt = mk(key.as_ptr(), std::ptr::null(), -1);
    duo("T5 set_text NULL text", &|c, png, i, _l| unsafe {
        (c.set_text)(png, i, &tt as *const _ as *const c_void, 1);
        st(c, png, i);
    });
    // pngset.c:1000 "too many text chunks" (array allocation fails) followed by
    // pngset.c:950 "Insufficient memory to store text" when the report is not
    // fatal; :1092 "text chunk: out of memory" for the per-entry allocation.
    for k in 1..=2usize {
        quad_mem(&format!("T6 set_text oom k={k}"), &|c, png, i, _l| unsafe {
            let t = mk(key.as_ptr(), txt.as_ptr(), -1);
            arm(k);
            sub("text", || (c.set_text)(png, i, &t as *const _ as *const c_void, 1));
            disarm();
            st(c, png, i);
        });
    }
    // png_set_text_2 returns a status instead of erroring out.
    for k in 1..=2usize {
        quad_mem(&format!("T7 set_text_2 oom k={k}"), &|_c, png, i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, Info, *const c_void, c_int) -> c_int =
                lib.f("png_set_text_2");
            let t = mk(key.as_ptr(), txt.as_ptr(), -1);
            arm(k);
            sub("text2", || {
                log(format!("ret={}", f(png, i, &t as *const _ as *const c_void, 1)))
            });
            disarm();
        });
    }

    // ---- pngset.c:1981 png_check_keyword, message :2048
    for (tag, k) in [
        ("ok", &key),
        ("tab", &badkey),
        ("spaces", &spacekey),
        ("long", &longkey),
        ("empty", &emptykey),
    ] {
        duo(&format!("T8 check_keyword {tag}"), &|_c, png, _i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, *const c_char, *mut u8) -> u32 =
                lib.f("png_check_keyword");
            let mut buf = [0u8; 80];
            let n = f(png, p(k), buf.as_mut_ptr());
            log(format!(
                "len={n} new={}",
                cstr(buf.as_ptr() as *const c_char)
            ));
        });
    }
    duo("T9 check_keyword NULL key", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *const c_char, *mut u8) -> u32 = lib.f("png_check_keyword");
        let mut buf = [0u8; 80];
        let n = f(png, std::ptr::null(), buf.as_mut_ptr());
        log(format!("len={n} first={}", buf[0]));
    });
    // Non-ASCII / control characters, one diff per byte value.
    for ch in [0x01u8, 0x09, 0x0a, 0x1f, 0x20, 0x7f, 0x80, 0xa0, 0xa1, 0xff] {
        let k = vec![b'a', ch, b'b', 0];
        duo(&format!("T10 check_keyword ch={ch:#04x}"), &|_c, png, _i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, *const c_char, *mut u8) -> u32 =
                lib.f("png_check_keyword");
            let mut buf = [0u8; 80];
            let n = f(png, p(&k), buf.as_mut_ptr());
            log(format!("len={n} new={}", hex(&buf[..8])));
        });
    }
    // The same keyword check on the write path (png_write_tEXt).
    for (tag, k) in [("tab", &badkey), ("empty", &emptykey), ("long", &longkey)] {
        dw(&format!("T11 write tEXt key {tag}"), &|c, png, i, _l| unsafe {
            (c.set_IHDR)(png, i, 1, 1, 8, 0, 0, 0, 0);
            let t = PngText {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: k.as_ptr() as *mut c_char,
                text: txt.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: std::ptr::null_mut(),
                lang_key: std::ptr::null_mut(),
            };
            sub("set", || (c.set_text)(png, i, &t as *const _ as *const c_void, 1));
            sub("write", || (c.write_info)(png, i));
            st(c, png, i);
        });
    }
}

#[test]
fn splt_rejections() {
    let name = cz("splt name");
    let mut ents = vec![
        PngSpltEntry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 },
        PngSpltEntry { red: 6, green: 7, blue: 8, alpha: 9, frequency: 10 },
    ];
    let good = PngSpltT {
        name: name.as_ptr() as *mut c_char,
        depth: 8,
        entries: ents.as_mut_ptr(),
        nentries: 2,
    };
    // ---- pngset.c:1327 "png_set_sPLT: invalid sPLT" (NULL name or entries)
    let noname = PngSpltT { name: std::ptr::null_mut(), ..good };
    let noents = PngSpltT { entries: std::ptr::null_mut(), ..good };
    quad("Q1 set_sPLT NULL name", &|c, png, i, _l| unsafe {
        sub("splt", || (c.set_sPLT)(png, i, &noname as *const _ as *const c_void, 1));
        st(c, png, i);
    });
    quad("Q2 set_sPLT NULL entries", &|c, png, i, _l| unsafe {
        sub("splt", || (c.set_sPLT)(png, i, &noents as *const _ as *const c_void, 1));
        st(c, png, i);
    });
    // depth / nentries out of range: stored verbatim by pngset.
    for &(d, n) in &[(0u8, 2i32), (1, 2), (7, 2), (255, 2), (8, 1)] {
        let e = PngSpltT { depth: d, nentries: n, ..good };
        quad(&format!("Q3 set_sPLT depth={d} n={n}"), &|c, png, i, _l| unsafe {
            sub("splt", || (c.set_sPLT)(png, i, &e as *const _ as *const c_void, 1));
            st(c, png, i);
        });
    }
    // nentries <= 0 inside the entry: png_malloc_array errors out
    // ("internal error: array alloc").
    for n in [0i32, -1] {
        let e = PngSpltT { nentries: n, ..good };
        quad(&format!("Q4 set_sPLT entry nentries={n}"), &|c, png, i, _l| unsafe {
            sub("splt", || (c.set_sPLT)(png, i, &e as *const _ as *const c_void, 1));
            st(c, png, i);
        });
    }
    // nentries argument <= 0 / NULL array: ignored.
    for n in [0i32, -1] {
        duo(&format!("Q5 set_sPLT num={n}"), &|c, png, i, _l| unsafe {
            (c.set_sPLT)(png, i, &good as *const _ as *const c_void, n);
            st(c, png, i);
        });
    }
    duo("Q6 set_sPLT NULL array", &|c, png, i, _l| unsafe {
        (c.set_sPLT)(png, i, std::ptr::null(), 1);
        st(c, png, i);
    });
    // pngset.c:1305 "too many sPLT chunks" (array realloc) and :1379
    // "sPLT out of memory" (name / entries allocation inside the loop).
    for k in 1..=3usize {
        quad_mem(&format!("Q7 set_sPLT oom k={k}"), &|c, png, i, _l| unsafe {
            arm(k);
            sub("splt", || (c.set_sPLT)(png, i, &good as *const _ as *const c_void, 1));
            disarm();
            st(c, png, i);
        });
    }
}

#[test]
fn info_ownership_api() {
    // ---- pngset.c:1777 png_set_rows with NULL / png_get_rows
    let mut row = vec![0u8; 16];
    let mut rows: Vec<*mut u8> = vec![row.as_mut_ptr()];
    let prows = rows.as_mut_ptr();
    duo("O1 set_rows NULL", &|c, png, i, _l| unsafe {
        (c.set_rows)(png, i, std::ptr::null_mut());
        log(format!("rows={}", (!(c.get_rows)(png, i).is_null()) as u8));
        st(c, png, i);
    });
    duo("O2 set_rows then NULL", &|c, png, i, _l| unsafe {
        (c.set_rows)(png, i, prows);
        log(format!("rows={}", (!(c.get_rows)(png, i).is_null()) as u8));
        (c.set_rows)(png, i, std::ptr::null_mut());
        log(format!("rows={}", (!(c.get_rows)(png, i).is_null()) as u8));
        st(c, png, i);
    });
    solo("O3 set_rows NULL png", &|c, _| unsafe {
        (c.set_rows)(NP, NP, std::ptr::null_mut());
        log(format!("rows={}", (!(c.get_rows)(NP, NP).is_null()) as u8));
    });

    // ---- pngset.c:1859 png_set_invalid / pngget.c:19 png_get_valid with
    // undefined flag bits.
    for mask in [0i32, -1, 0x100000, 0x7fff_ffff] {
        duo(&format!("O4 set_invalid mask={mask:#x}"), &|c, png, i, _l| unsafe {
            (c.set_gAMA_fixed)(png, i, 45455);
            (c.set_pHYs)(png, i, 1, 1, 1);
            (c.set_invalid)(png, i, mask);
            st(c, png, i);
        });
    }
    for flag in [0u32, 0x100000, 0x8000_0000, 0xffff_ffff] {
        duo(&format!("O5 get_valid flag={flag:#x}"), &|c, png, i, _l| unsafe {
            (c.set_gAMA_fixed)(png, i, 45455);
            log(format!("valid={}", (c.get_valid)(png, i, flag)));
        });
    }

    // ---- png.c:466 png_data_freer with an unknown freer value.
    for f in [0, 1, 2, 3, 4, 99, -1] {
        quad(&format!("O6 data_freer={f}"), &|c, png, i, _l| unsafe {
            (c.set_gAMA_fixed)(png, i, 45455);
            sub("freer", || (c.data_freer)(png, i, f, PNG_FREE_ALL));
            st(c, png, i);
        });
    }
    // ---- png.c:487 png_free_data with unknown mask bits / num values.
    for &(mask, num) in &[
        (0u32, -1i32),
        (0xffff_ffff, -1),
        (0x1_0000, -1),
        (PNG_FREE_TEXT, 0),
        (PNG_FREE_TEXT, 5),
        (PNG_FREE_ALL, 0),
    ] {
        duo(
            &format!("O7 free_data mask={mask:#x} num={num}"),
            &|c, png, i, _l| unsafe {
                (c.set_gAMA_fixed)(png, i, 45455);
                (c.set_pHYs)(png, i, 3, 4, 1);
                sub("free", || (c.free_data)(png, i, mask, num));
                st(c, png, i);
            },
        );
    }
    solo("O8 free_data/data_freer NULL", &|c, _| unsafe {
        (c.free_data)(NP, NP, PNG_FREE_ALL, -1);
        (c.data_freer)(NP, NP, 1, PNG_FREE_ALL);
        log("ok".to_string());
    });
}

// ===========================================================================
// 6. getters with a bad state
// ===========================================================================

#[test]
fn get_with_bad_state() {
    // pngget.c:938 png_get_IHDR re-runs png_check_IHDR, so on a fresh info
    // struct it warns about every field and then errors out.
    duo("G1 get_IHDR uninitialised", &|c, png, i, _l| unsafe {
        let mut w = 0u32;
        let mut h = 0u32;
        let (mut bd, mut ct, mut il, mut cm, mut fm) = (-1, -1, -1, -1, -1);
        sub("ihdr", || {
            let r = (c.get_IHDR)(
                png, i, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm,
            );
            log(format!("rc={r} {w}x{h} d={bd} ct={ct} il={il} cm={cm} fm={fm}"));
        });
        st(c, png, i);
    });
    // All-NULL output pointers, and a NULL info / png_ptr.
    duo("G2 get_IHDR NULL outputs", &|c, png, i, _l| unsafe {
        (c.set_IHDR)(png, i, 3, 4, 8, 2, 0, 0, 0);
        sub("ihdr", || {
            log(format!(
                "rc={}",
                (c.get_IHDR)(
                    png,
                    i,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut()
                )
            ));
        });
    });
    solo("G3 get_IHDR NULL struct", &|c, _| unsafe {
        let mut w = 7u32;
        log(format!(
            "rc={} w={w}",
            (c.get_IHDR)(
                NP,
                NP,
                &mut w,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            )
        ));
    });
    // png_get_rowbytes and friends before any IHDR.
    duo("G4 getters before IHDR", &|c, png, i, _l| unsafe {
        st(c, png, i);
    });
    // Every getter with a NULL info_ptr, then with a NULL png_ptr.
    duo("G5 getters NULL info", &|c, png, _i, _l| unsafe {
        st(c, png, NP);
        log(format!(
            "rowbytes={} channels={} width={} valid={}",
            (c.get_rowbytes)(png, NP),
            (c.get_channels)(png, NP),
            (c.get_image_width)(png, NP),
            (c.get_valid)(png, NP, PNG_INFO_gAMA)
        ));
    });
    solo("G6 getters NULL png", &|c, _| unsafe {
        st(c, NP, NP);
    });
    // Chunk getters on an info struct where the chunk was never set: they must
    // all return 0 and leave the output parameters untouched.
    duo("G7 unset chunk getters", &|c, png, i, _l| unsafe {
        (c.set_IHDR)(png, i, 1, 1, 8, 0, 0, 0, 0);
        st(c, png, i);
    });

    // pngget.c:376 "fixed point overflow ignored"
    for &(x, u) in &[
        (0i32, 1i32),
        (1, 1),
        (2_147_483_647, 1),
        (-2_147_483_648, 1),
        (2_147_483_647, 0),
        (1_000_000, 1),
    ] {
        duo(
            &format!("G8 oFFs inches x={x} unit={u}"),
            &|c, png, i, lib| unsafe {
                (c.set_oFFs)(png, i, x, x, u);
                let xf: unsafe extern "C" fn(Png, Info) -> i32 =
                    lib.f("png_get_x_offset_inches_fixed");
                let yf: unsafe extern "C" fn(Png, Info) -> i32 =
                    lib.f("png_get_y_offset_inches_fixed");
                let xm: unsafe extern "C" fn(Png, Info) -> i32 = lib.f("png_get_x_offset_microns");
                let xd: unsafe extern "C" fn(Png, Info) -> f64 = lib.f("png_get_x_offset_inches");
                log(format!("microns={}", xm(png, i)));
                sub("xf", || log(format!("xfixed={}", xf(png, i))));
                sub("yf", || log(format!("yfixed={}", yf(png, i))));
                log(format!("xinches={:.6}", xd(png, i)));
            },
        );
    }
    // The same easy-access helpers with a NULL struct.
    solo("G9 easy access NULL", &|_c, lib| unsafe {
        for n in [
            "png_get_x_offset_inches_fixed",
            "png_get_y_offset_inches_fixed",
            "png_get_x_offset_microns",
            "png_get_y_offset_microns",
            "png_get_x_pixels_per_meter",
            "png_get_y_pixels_per_meter",
            "png_get_pixels_per_meter",
            "png_get_x_pixels_per_inch",
            "png_get_y_pixels_per_inch",
            "png_get_pixels_per_inch",
        ] {
            let f: unsafe extern "C" fn(Png, Info) -> i32 = lib.f(n);
            log(format!("{n}={}", f(NP, NP)));
        }
        let ar: unsafe extern "C" fn(Png, Info) -> f32 = lib.f("png_get_pixel_aspect_ratio");
        log(format!("aspect={:.6}", ar(NP, NP)));
        let arf: unsafe extern "C" fn(Png, Info) -> i32 =
            lib.f("png_get_pixel_aspect_ratio_fixed");
        log(format!("aspect_fixed={}", arf(NP, NP)));
    });

    // pngget.c:895 "png_get_eXIf does not work; use png_get_eXIf_1"
    duo("G10 get_eXIf deprecated", &|_c, png, i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, Info, *mut *mut u8) -> u32 = lib.f("png_get_eXIf");
        let mut ptr: *mut u8 = std::ptr::null_mut();
        log(format!("rc={} null={}", f(png, i, &mut ptr), ptr.is_null() as u8));
    });
    solo("G11 get_eXIf NULL", &|_c, lib| unsafe {
        let f: unsafe extern "C" fn(Png, Info, *mut *mut u8) -> u32 = lib.f("png_get_eXIf");
        let mut ptr: *mut u8 = std::ptr::null_mut();
        log(format!("rc={}", f(NP, NP, &mut ptr)));
    });
    duo("G12 get_eXIf_1 NULL outputs", &|c, png, i, _l| unsafe {
        log(format!(
            "rc={}",
            (c.get_eXIf_1)(png, i, std::ptr::null_mut(), std::ptr::null_mut())
        ));
    });

    // pngrutil.c:41 png_get_uint_31: >= 2^31 is fatal; png_get_int_32 with the
    // "broken" 0x80000000 encoding.
    for b in [
        [0u8, 0, 0, 0],
        [0x7f, 0xff, 0xff, 0xff],
        [0x80, 0, 0, 0],
        [0x80, 0, 0, 1],
        [0xff, 0xff, 0xff, 0xff],
    ] {
        duo(&format!("G13 get_uint_31 {}", hex(&b)), &|c, png, _i, _l| unsafe {
            sub("u31", || log(format!("u31={}", (c.get_uint_31)(png, b.as_ptr()))));
            log(format!(
                "u32={} i32={} u16={}",
                (c.get_uint_32)(b.as_ptr()),
                (c.get_int_32)(b.as_ptr()),
                (c.get_uint_16)(b.as_ptr())
            ));
        });
    }
    // png_save_int_32 round-trip of the broken encoding.
    solo("G14 save/get_int_32", &|c, _| unsafe {
        for v in [0i32, 1, -1, i32::MIN, i32::MAX] {
            let mut b = [0u8; 4];
            (c.save_int_32)(b.as_mut_ptr(), v);
            log(format!("v={v} enc={} dec={}", hex(&b), (c.get_int_32)(b.as_ptr())));
        }
    });
}

// ===========================================================================
// 7. transform setup (pngrtran.c / pngtrans.c)
// ===========================================================================

/// The read-side transform setters, by name, all of which funnel through
/// png_rtran_ok().
fn rtran_calls(c: &Core, png: Png, which: usize) {
    unsafe {
        match which {
            0 => (c.set_strip_16)(png),
            1 => (c.set_scale_16)(png),
            2 => (c.set_strip_alpha)(png),
            3 => (c.set_expand)(png),
            4 => (c.set_expand_16)(png),
            5 => (c.set_expand_gray_1_2_4_to_8)(png),
            6 => (c.set_palette_to_rgb)(png),
            7 => (c.set_tRNS_to_alpha)(png),
            8 => (c.set_gray_to_rgb)(png),
            9 => (c.set_gamma_fixed)(png, 220_000, 45455),
            10 => (c.set_alpha_mode_fixed)(png, PNG_ALPHA_PNG, FP1),
            11 => (c.set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_NONE, -1, -1),
            12 => (c.set_quantize)(png, NP as *mut u8, 2, 2, std::ptr::null(), 0),
            _ => {}
        }
    }
}

const RTRAN_NAMES: [&str; 13] = [
    "strip_16",
    "scale_16",
    "strip_alpha",
    "expand",
    "expand_16",
    "expand_gray_1_2_4_to_8",
    "palette_to_rgb",
    "tRNS_to_alpha",
    "gray_to_rgb",
    "gamma",
    "alpha_mode",
    "rgb_to_gray",
    "quantize",
];

#[test]
fn transform_setup_time_rejections() {
    let good = img(4, 2, 8, 2);
    // pngrtran.c:124 "invalid before the PNG header has been read": only the
    // setters that pass need_IHDR=1 (png_set_rgb_to_gray).
    for (k, n) in RTRAN_NAMES.iter().enumerate() {
        for &b in &[0, 1] {
            go(
                &format!("X1 {n} before IHDR b{b}"),
                false,
                &[],
                false,
                Some(b),
                None,
                &|c, png, i, _l| unsafe {
                    sub("set", || rtran_calls(c, png, k));
                    log(format!("rgb2gray_status={}", (c.get_rgb_to_gray_status)(png)));
                    st(c, png, i);
                },
            );
        }
    }
    // pngrtran.c:120 "invalid after png_start_read_image or png_read_update_info"
    for (k, n) in RTRAN_NAMES.iter().enumerate() {
        for &b in &[0, 1] {
            go(
                &format!("X2 {n} after update_info b{b}"),
                false,
                &good,
                false,
                Some(b),
                None,
                &|c, png, i, _l| unsafe {
                    (c.read_info)(png, i);
                    (c.read_update_info)(png, i);
                    sub("set", || rtran_calls(c, png, k));
                    log(format!("rowbytes={}", (c.get_rowbytes)(png, i)));
                },
            );
        }
    }
    // ... and after png_start_read_image, and after the first row.
    for (k, n) in RTRAN_NAMES.iter().enumerate() {
        go(
            &format!("X3 {n} after start_read_image"),
            false,
            &good,
            false,
            Some(1),
            None,
            &|c, png, i, _l| unsafe {
                (c.read_info)(png, i);
                (c.start_read_image)(png);
                sub("set", || rtran_calls(c, png, k));
            },
        );
    }
    let mut rowbuf = vec![0u8; 64];
    let rp = rowbuf.as_mut_ptr();
    for (k, n) in RTRAN_NAMES.iter().enumerate() {
        go(
            &format!("X4 {n} after first row"),
            false,
            &good,
            false,
            Some(1),
            None,
            &|c, png, i, _l| unsafe {
                (c.read_info)(png, i);
                (c.read_row)(png, rp, std::ptr::null_mut());
                sub("set", || rtran_calls(c, png, k));
                log(format!("row={}", hex(std::slice::from_raw_parts(rp, 12))));
            },
        );
    }
    // A write struct is not a read struct: png_rtran_ok does not care, so these
    // calls succeed but the transformation is meaningless.
    for (k, n) in RTRAN_NAMES.iter().enumerate() {
        dw(&format!("X5 {n} on write struct"), &|c, png, i, _l| unsafe {
            sub("set", || rtran_calls(c, png, k));
            st(c, png, i);
        });
    }
    // pngtrans.c:845 "info change after png_start_read_image or
    // png_read_update_info"
    for &b in &[0, 1] {
        go(
            &format!("X6 user_transform_info after update b{b}"),
            false,
            &good,
            false,
            Some(b),
            None,
            &|c, png, i, _l| unsafe {
                (c.read_info)(png, i);
                (c.read_update_info)(png, i);
                sub("set", || (c.set_user_transform_info)(png, NP, 8, 3));
                log(format!(
                    "ptr={}",
                    (!(c.get_user_transform_ptr)(png).is_null()) as u8
                ));
            },
        );
    }
    // Negative / oversized depth and channel counts are truncated to a byte.
    for &(d, ch) in &[(-1i32, -1i32), (0, 0), (999, 999), (256, 256), (8, 3)] {
        duo(
            &format!("X7 user_transform_info d={d} ch={ch}"),
            &|c, png, _i, _l| unsafe {
                (c.set_user_transform_info)(png, NP, d, ch);
                log("ok".to_string());
            },
        );
    }
    // The read transform hook on a write struct and vice versa.
    dw("X8 read_user_transform_fn on write", &|c, png, i, _l| unsafe {
        (c.set_IHDR)(png, i, 2, 1, 8, 2, 0, 0, 0);
        (c.set_read_user_transform_fn)(png, NP);
        sub("write", || {
            (c.write_info)(png, i);
            let row = [1u8, 2, 3, 4, 5, 6];
            (c.write_row)(png, row.as_ptr());
            (c.write_end)(png, i);
        });
    });
    di("X9 write_user_transform_fn on read", &good, &|c, png, i, _l| unsafe {
        (c.set_write_user_transform_fn)(png, NP);
        (c.read_info)(png, i);
        (c.read_update_info)(png, i);
        sub("row", || (c.read_row)(png, rp, std::ptr::null_mut()));
        log(format!("row={}", hex(std::slice::from_raw_parts(rp, 12))));
    });
}

#[test]
fn transform_argument_rejections() {
    let good = img(4, 2, 8, 2);
    let gray1 = img(8, 1, 1, 0);
    let pal = img(2, 1, 8, 3);

    // ---- pngrtran.c:434 "invalid alpha mode"
    for m in [-1, 0, 1, 2, 3, 4, 99] {
        quad(&format!("Y1 alpha_mode mode={m}"), &|c, png, i, _l| unsafe {
            sub("set", || (c.set_alpha_mode_fixed)(png, m, FP1));
            st(c, png, i);
        });
    }
    // pngrtran.c:325 "fixed point overflow in gamma value" via the float API.
    for g in [0.0f64, -1.0, -2.0, -3.0, 1e10, 1e-10, 1.0, 45455.0] {
        quad(&format!("Y2 alpha_mode gamma={g}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_alpha_mode)(png, PNG_ALPHA_PNG, g));
        });
    }
    for g in [0i32, -1, -2, -3, -4, 1, i32::MAX, i32::MIN] {
        quad(
            &format!("Y3 alpha_mode_fixed gamma={g}"),
            &|c, png, _i, _l| unsafe {
                sub("set", || (c.set_alpha_mode_fixed)(png, PNG_ALPHA_PNG, g));
            },
        );
    }
    // pngrtran.c:452 "conflicting calls to set alpha mode and background"
    let bg = PngColor16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
    for m in [PNG_ALPHA_ASSOCIATED, PNG_ALPHA_OPTIMIZED, PNG_ALPHA_BROKEN, PNG_ALPHA_PNG] {
        quad(&format!("Y4 background+alpha_mode {m}"), &|c, png, _i, _l| unsafe {
            (c.set_background_fixed)(
                png,
                &bg as *const _ as *const u8,
                PNG_BACKGROUND_GAMMA_SCREEN,
                0,
                FP1,
            );
            sub("set", || (c.set_alpha_mode_fixed)(png, m, FP1));
        });
    }

    // ---- pngrtran.c:142 png_set_background
    duo("Y5 background NULL colour", &|c, png, _i, _l| unsafe {
        (c.set_background_fixed)(png, std::ptr::null(), PNG_BACKGROUND_GAMMA_SCREEN, 0, FP1);
        log("ok".to_string());
    });
    for code in [-1, 0, 1, 2, 3, 4, 99] {
        quad(&format!("Y6 background code={code}"), &|c, png, _i, _l| unsafe {
            sub("set", || {
                (c.set_background_fixed)(png, &bg as *const _ as *const u8, code, 0, FP1)
            });
        });
    }
    for g in [0.0f64, -1.0, 1e10] {
        quad(&format!("Y7 background gamma={g}"), &|c, png, _i, _l| unsafe {
            sub("set", || {
                (c.set_background)(
                    png,
                    &bg as *const _ as *const u8,
                    PNG_BACKGROUND_GAMMA_UNIQUE,
                    0,
                    g,
                )
            });
        });
    }

    // ---- pngrtran.c:893 png_set_gamma: "invalid file/screen gamma", plus
    // "gamma out of supported range" from unsupported_gamma().
    for &(s, f) in &[
        (0i32, FP1),
        (FP1, 0),
        (-1, FP1),
        (FP1, -1),
        (-2, -3),
        (1, 1),
        (i32::MAX, FP1),
        (220_000, 45455),
    ] {
        quad(&format!("Y8 gamma_fixed s={s} f={f}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_gamma_fixed)(png, s, f));
        });
    }
    for &(s, f) in &[(0.0f64, 1.0f64), (-1.0, 1.0), (1e10, 1.0), (1.0, 1e10), (2.2, 0.45455)] {
        quad(&format!("Y9 gamma s={s} f={f}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_gamma)(png, s, f));
        });
    }

    // ---- pngrtran.c:1047 png_set_rgb_to_gray: error action and coefficients.
    for ea in [-1, 0, 1, 2, 3, 4, 99] {
        for &b in &[0, 1] {
            go(
                &format!("Y10 rgb_to_gray action={ea} b{b}"),
                false,
                &good,
                false,
                Some(b),
                None,
                &|c, png, i, _l| unsafe {
                    (c.read_info)(png, i);
                    sub("set", || (c.set_rgb_to_gray_fixed)(png, ea, -1, -1));
                    log(format!("status={}", (c.get_rgb_to_gray_status)(png)));
                },
            );
        }
    }
    for &(r, g) in &[
        (-1i32, -1i32),
        (0, 0),
        (FP1, FP1),
        (FP1 / 2, FP1),
        (-1, FP1),
        (FP1, -1),
        (i32::MAX, i32::MAX),
        (21260, 71520),
    ] {
        for &b in &[0, 1] {
            go(
                &format!("Y11 rgb_to_gray coeff {r},{g} b{b}"),
                false,
                &good,
                false,
                Some(b),
                None,
                &|c, png, i, _l| unsafe {
                    (c.read_info)(png, i);
                    sub("set", || {
                        (c.set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_NONE, r, g)
                    });
                },
            );
        }
    }
    // The float wrapper goes through png_fixed -> "fixed point overflow".
    for &(r, g) in &[(1e10f64, 0.5f64), (0.5, 1e10), (0.2126, 0.7152)] {
        di(&format!("Y12 rgb_to_gray double {r},{g}"), &good, &|c, png, i, _l| unsafe {
            (c.read_info)(png, i);
            sub("set", || (c.set_rgb_to_gray)(png, PNG_ERROR_ACTION_NONE, r, g));
        });
    }
    // A palette image forces PNG_EXPAND; a grey image is refused later on.
    di("Y13 rgb_to_gray on palette", &pal, &|c, png, i, _l| unsafe {
        (c.read_info)(png, i);
        sub("set", || {
            (c.set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_WARN, -1, -1)
        });
        sub("update", || (c.read_update_info)(png, i));
        st(c, png, i);
    });

    // ---- pngtrans.c:147 png_set_filler on the wrong colour type.
    for &(ct, bd) in &[(0i32, 1u8), (0, 2), (0, 4), (0, 8), (2, 8), (3, 8), (4, 8), (6, 8)] {
        for &b in &[0, 1] {
            go(
                &format!("Y14 set_filler write ct={ct} bd={bd} b{b}"),
                true,
                &[],
                false,
                Some(b),
                None,
                &|c, png, i, _l| unsafe {
                    (c.set_IHDR)(png, i, 1, 1, bd as c_int, ct, 0, 0, 0);
                    if ct == 3 {
                        let pal3 = [0u8; 3 * 4];
                        (c.set_PLTE)(png, i, pal3.as_ptr(), 4);
                    }
                    sub("winfo", || (c.write_info)(png, i));
                    sub("filler", || (c.set_filler)(png, 0xff, PNG_FILLER_AFTER));
                    sub("addalpha", || (c.set_add_alpha)(png, 0xff, PNG_FILLER_BEFORE));
                },
            );
        }
    }
    // filler_loc out of range (only PNG_FILLER_AFTER is special-cased).
    for loc in [-1, 0, 1, 2, 99] {
        dw(&format!("Y15 set_filler loc={loc}"), &|c, png, i, _l| unsafe {
            (c.set_IHDR)(png, i, 1, 1, 8, 2, 0, 0, 0);
            sub("winfo", || (c.write_info)(png, i));
            sub("filler", || (c.set_filler)(png, 0xff, loc));
        });
    }
    // On a read struct png_set_filler is always accepted.
    di("Y16 set_filler on read", &gray1, &|c, png, i, _l| unsafe {
        (c.read_info)(png, i);
        (c.set_filler)(png, 0xff, PNG_FILLER_AFTER);
        log("ok".to_string());
    });

    // ---- pngtrans.c:83 png_set_shift
    for &(ct, bd, v) in &[
        (0i32, 8u8, [0u8, 0, 0, 0, 0]),
        (0, 8, [0, 0, 0, 9, 0]),
        (0, 8, [0, 0, 0, 8, 0]),
        (2, 8, [0, 4, 4, 0, 0]),
        (2, 8, [4, 9, 4, 0, 0]),
        (6, 8, [4, 4, 4, 0, 0]),
        (6, 8, [4, 4, 4, 0, 9]),
        (4, 8, [0, 0, 0, 4, 4]),
    ] {
        let sb = PngColor8 {
            red: v[0],
            green: v[1],
            blue: v[2],
            gray: v[3],
            alpha: v[4],
        };
        for &b in &[0, 1] {
            go(
                &format!("Y17 set_shift ct={ct} bd={bd} {v:?} b{b}"),
                true,
                &[],
                false,
                Some(b),
                None,
                &|c, png, i, _l| unsafe {
                    (c.set_IHDR)(png, i, 1, 1, bd as c_int, ct, 0, 0, 0);
                    sub("winfo", || (c.write_info)(png, i));
                    sub("shift", || (c.set_shift)(png, &sb as *const _ as *const u8));
                },
            );
        }
    }
    dw("Y18 set_shift NULL", &|c, png, _i, _l| unsafe {
        (c.set_shift)(png, std::ptr::null());
        log("ok".to_string());
    });

    // ---- png_set_scale_16 together with png_set_strip_16.
    let img16 = img(2, 1, 16, 0);
    di("Y19 scale_16 + strip_16", &img16, &|c, png, i, _l| unsafe {
        (c.read_info)(png, i);
        (c.set_scale_16)(png);
        (c.set_strip_16)(png);
        (c.read_update_info)(png, i);
        log(format!("rowbytes={}", (c.get_rowbytes)(png, i)));
        let mut buf = [0u8; 16];
        sub("row", || (c.read_row)(png, buf.as_mut_ptr(), std::ptr::null_mut()));
        log(format!("row={}", hex(&buf)));
    });
    di("Y20 strip_16 + scale_16", &img16, &|c, png, i, _l| unsafe {
        (c.read_info)(png, i);
        (c.set_strip_16)(png);
        (c.set_scale_16)(png);
        (c.read_update_info)(png, i);
        log(format!("rowbytes={}", (c.get_rowbytes)(png, i)));
        let mut buf = [0u8; 16];
        sub("row", || (c.read_row)(png, buf.as_mut_ptr(), std::ptr::null_mut()));
        log(format!("row={}", hex(&buf)));
    });

    // ---- png_set_expand_gray_1_2_4_to_8 on the wrong types.
    for (tag, inp) in [
        ("gray1", &gray1),
        ("rgb8", &good),
        ("palette", &pal),
        ("gray16", &img16),
    ] {
        di(&format!("Y21 expand_gray {tag}"), inp, &|c, png, i, _l| unsafe {
            (c.read_info)(png, i);
            (c.set_expand_gray_1_2_4_to_8)(png);
            sub("update", || (c.read_update_info)(png, i));
            log(format!("rowbytes={}", (c.get_rowbytes)(png, i)));
        });
    }

    // ---- pngtrans.c:127 png_set_interlace_handling on a non-interlaced image.
    let inter = Builder::new(4, 4, 8, 0).interlace(1).build_valid(9);
    for (tag, inp) in [("progressive", &good), ("interlaced", &inter)] {
        di(&format!("Y22 interlace_handling {tag}"), inp, &|c, png, i, _l| unsafe {
            log(format!("before={}", (c.set_interlace_handling)(png)));
            (c.read_info)(png, i);
            log(format!("after={}", (c.set_interlace_handling)(png)));
        });
    }
    solo("Y23 interlace_handling NULL", &|c, _| unsafe {
        log(format!("ret={}", (c.set_interlace_handling)(NP)));
    });

    // ---- pngrtran.c:21 png_set_crc_action with out-of-range actions.
    for crit in [-1, 0, 1, 2, 3, 4, 5, 6, 99] {
        for anc in [-1, 0, 5, 6, 99] {
            duo(
                &format!("Y24 crc_action {crit},{anc}"),
                &|c, png, _i, _l| unsafe {
                    sub("set", || (c.set_crc_action)(png, crit, anc));
                },
            );
        }
    }
}

// ===========================================================================
// 8. gamma / background interactions during png_read_update_info
// ===========================================================================

#[test]
fn gamma_and_background() {
    let rgb = img(4, 2, 8, 2);
    let gray = img(4, 2, 8, 0);
    let bg = PngColor16 { index: 0, red: 100, green: 200, blue: 300, gray: 400 };

    // pngrtran.c:1697 "libpng does not support gamma+background+rgb_to_gray"
    for &b in &[0, 1] {
        go(
            &format!("Z1 gamma+background+rgb_to_gray b{b}"),
            false,
            &rgb,
            false,
            Some(b),
            None,
            &|c, png, i, _l| unsafe {
                (c.read_info)(png, i);
                (c.set_gamma_fixed)(png, 220_000, 45455);
                (c.set_background_fixed)(
                    png,
                    &bg as *const _ as *const u8,
                    PNG_BACKGROUND_GAMMA_SCREEN,
                    0,
                    FP1,
                );
                (c.set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_NONE, -1, -1);
                sub("update", || (c.read_update_info)(png, i));
                log(format!("rowbytes={}", (c.get_rowbytes)(png, i)));
            },
        );
    }
    // pngrtran.c:1886 "invalid background gamma type" - the background gamma
    // code is stored as a byte and validated only here.
    for code in [4, 5, 99, 255, 256, -1] {
        for &b in &[0, 1] {
            go(
                &format!("Z2 background gamma type={code} b{b}"),
                false,
                &gray,
                false,
                Some(b),
                None,
                &|c, png, i, _l| unsafe {
                    (c.read_info)(png, i);
                    (c.set_gamma_fixed)(png, 220_000, 45455);
                    sub("bg", || {
                        (c.set_background_fixed)(png, &bg as *const _ as *const u8, code, 0, FP1)
                    });
                    sub("update", || (c.read_update_info)(png, i));
                    log(format!("rowbytes={}", (c.get_rowbytes)(png, i)));
                },
            );
        }
    }
    // png.c:3634 "gamma table being rebuilt": png_build_gamma_table is exported
    // and idempotent only at the price of this warning.
    for bd in [1, 2, 4, 8, 16] {
        di(&format!("Z3 build_gamma_table twice bd={bd}"), &gray, &|c, png, i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, c_int) = lib.f("png_build_gamma_table");
            (c.read_info)(png, i);
            (c.set_gamma_fixed)(png, 220_000, 45455);
            sub("first", || f(png, bd));
            sub("second", || f(png, bd));
            sub("third", || f(png, bd));
        });
    }
    // pngrtran.c:4341 "png_do_encode_alpha: unexpected call": ENCODE_ALPHA is
    // set but the screen gamma is 1.0, so no gamma table is built.
    let rgba = rgba1();
    for g in [FP1, 220_000] {
        di(&format!("Z4 encode_alpha gamma={g}"), &rgba, &|c, png, i, _l| unsafe {
            (c.read_info)(png, i);
            sub("mode", || (c.set_alpha_mode_fixed)(png, PNG_ALPHA_BROKEN, g));
            sub("update", || (c.read_update_info)(png, i));
            let mut buf = [0u8; 16];
            sub("row", || (c.read_row)(png, buf.as_mut_ptr(), std::ptr::null_mut()));
            log(format!("row={}", hex(&buf)));
        });
    }
    // pngrtran.c:4965/:4969 "png_do_rgb_to_gray found nongray pixel"
    let nongray = rgb_row(&[10, 20, 30, 40, 40, 40]);
    let allgray = rgb_row(&[40, 40, 40, 7, 7, 7]);
    for (tag, inp) in [("nongray", &nongray), ("gray", &allgray)] {
        for ea in [PNG_ERROR_ACTION_NONE, PNG_ERROR_ACTION_WARN, PNG_ERROR_ACTION_ERROR] {
            di(
                &format!("Z5 rgb_to_gray {tag} action={ea}"),
                inp,
                &|c, png, i, _l| unsafe {
                    (c.read_info)(png, i);
                    (c.set_rgb_to_gray_fixed)(png, ea, -1, -1);
                    (c.read_update_info)(png, i);
                    let mut buf = [0u8; 16];
                    sub("row", || (c.read_row)(png, buf.as_mut_ptr(), std::ptr::null_mut()));
                    log(format!(
                        "row={} status={}",
                        hex(&buf),
                        (c.get_rgb_to_gray_status)(png)
                    ));
                },
            );
        }
    }

    // pngrtran.c:4891 "NULL row buffer" - png_do_read_transformations is
    // exported; on a struct that has never started a row png_ptr->row_buf is
    // NULL.
    duo("Z6 do_read_transformations no row_buf", &|_c, png, _i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, *mut PngRowInfo) = lib.f("png_do_read_transformations");
        let mut ri = PngRowInfo {
            width: 4,
            rowbytes: 12,
            color_type: 2,
            bit_depth: 8,
            channels: 3,
            pixel_depth: 24,
        };
        sub("do", || f(png, &mut ri));
        log(format!("ri={ri:?}"));
    });
    // pngrtran.c:4907 "Uninitialized row": row_buf has been allocated but
    // PNG_FLAG_ROW_INIT was never set because png_read_start_row failed after
    // the allocation (the zlib inflate claim runs out of memory).
    for k in 1..=6usize {
        go(
            &format!("Z7 uninitialised row k={k}"),
            false,
            &gray,
            true,
            Some(1),
            None,
            &|c, png, i, lib| unsafe {
                let f: unsafe extern "C" fn(Png, *mut PngRowInfo) =
                    lib.f("png_do_read_transformations");
                (c.read_info)(png, i);
                (c.set_strip_16)(png);
                arm(k);
                let mut buf = [0u8; 32];
                sub("row", || (c.read_row)(png, buf.as_mut_ptr(), std::ptr::null_mut()));
                disarm();
                let mut ri = PngRowInfo {
                    width: 4,
                    rowbytes: 4,
                    color_type: 0,
                    bit_depth: 8,
                    channels: 1,
                    pixel_depth: 8,
                };
                sub("do", || f(png, &mut ri));
                log(format!("ri={ri:?}"));
            },
        );
    }
    // pngrtran.c:2104 "Palette is NULL in indexed image": png_read_transform_info
    // is exported and is what png_read_update_info calls; an info struct that
    // claims to be a palette image while png_ptr has no palette hits the check.
    duo("Z8 read_transform_info palette NULL", &|c, png, i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, Info) = lib.f("png_read_transform_info");
        (c.set_expand)(png);
        (c.set_IHDR)(png, i, 1, 1, 8, 3, 0, 0, 0);
        sub("xinfo", || f(png, i));
        st(c, png, i);
    });
    duo("Z9 read_transform_info rgb", &|c, png, i, lib| unsafe {
        let f: unsafe extern "C" fn(Png, Info) = lib.f("png_read_transform_info");
        (c.set_expand)(png);
        (c.set_IHDR)(png, i, 1, 1, 8, 2, 0, 0, 0);
        sub("xinfo", || f(png, i));
        st(c, png, i);
    });
}

// ===========================================================================
// 9. png_set_quantize
// ===========================================================================

#[test]
fn quantize_rejections() {
    let pal = img(4, 2, 8, 3);
    let mut palette: Vec<u8> = (0..3 * 256).map(|x| (x % 251) as u8).collect();
    let pp = palette.as_mut_ptr();
    let hist: Vec<u16> = (0..256).map(|x| (x * 7 % 61) as u16).collect();

    for &(np, maxc) in &[
        (0i32, 0i32),
        (-1, 8),
        (2, 0),
        (2, 1),
        (2, -1),
        (256, 257),
        (256, 1),
        (4, 4),
        (8, 4),
        (2, 256),
    ] {
        for &h in &[false, true] {
            for &full in &[0, 1] {
                di(
                    &format!("N1 quantize np={np} max={maxc} hist={h} full={full}"),
                    &pal,
                    &|c, png, i, _l| unsafe {
                        (c.read_info)(png, i);
                        let hp = if h { hist.as_ptr() } else { std::ptr::null() };
                        sub("set", || (c.set_quantize)(png, pp, np, maxc, hp, full));
                        sub("update", || (c.read_update_info)(png, i));
                        log(format!("rowbytes={}", (c.get_rowbytes)(png, i)));
                    },
                );
            }
        }
    }
    di("N2 quantize NULL palette", &pal, &|c, png, i, _l| unsafe {
        (c.read_info)(png, i);
        (c.set_quantize)(png, std::ptr::null_mut(), 4, 4, std::ptr::null(), 1);
        log("ok".to_string());
    });
    solo("N3 quantize NULL png", &|c, _| unsafe {
        (c.set_quantize)(NP, NP as *mut u8, 4, 4, std::ptr::null(), 1);
        log("ok".to_string());
    });
    // png_set_check_for_invalid_index and png_get_palette_max around it.
    for a in [-1, 0, 1, 99] {
        di(
            &format!("N4 check_for_invalid_index {a}"),
            &pal,
            &|c, png, i, _l| unsafe {
                (c.set_check_for_invalid_index)(png, a);
                (c.read_info)(png, i);
                (c.read_update_info)(png, i);
                let mut buf = [0u8; 16];
                sub("row", || (c.read_row)(png, buf.as_mut_ptr(), std::ptr::null_mut()));
                log(format!(
                    "row={} palette_max={}",
                    hex(&buf[..4]),
                    (c.get_palette_max)(png, i)
                ));
            },
        );
    }
}

// ===========================================================================
// 10. unknown-chunk API
// ===========================================================================

#[test]
fn unknown_chunk_api() {
    let data = vec![0xAAu8; 7];
    let mk = |name: &[u8; 4], loc: u8, size: usize| PngUnknownChunk {
        name: [name[0], name[1], name[2], name[3], 0],
        data: data.as_ptr() as *mut u8,
        size,
        location: loc,
    };

    // pngset.c:1396 "png_set_unknown_chunks now expects a valid location" (write
    // struct only) and :1407 "invalid location in png_set_unknown_chunks".
    for loc in [0u8, 1, 2, 4, 8, 3, 9, 16, 255] {
        quad(&format!("U1 set_unknown_chunks loc={loc}"), &|c, png, i, _l| unsafe {
            let u = mk(b"prVt", loc, 7);
            sub("set", || {
                (c.set_unknown_chunks)(png, i, &u as *const _ as *const c_void, 1)
            });
            st(c, png, i);
        });
    }
    // The same on a write struct that has already emitted the IHDR, so that the
    // legacy "use the current mode" fallback yields a non-zero location.
    for loc in [0u8, 8] {
        for &b in &[0, 1] {
            go(
                &format!("U2 unknown after write_info loc={loc} b{b}"),
                true,
                &[],
                false,
                Some(b),
                None,
                &|c, png, i, _l| unsafe {
                    (c.set_IHDR)(png, i, 1, 1, 8, 0, 0, 0, 0);
                    sub("winfo", || (c.write_info)(png, i));
                    let u = mk(b"prVt", loc, 7);
                    sub("set", || {
                        (c.set_unknown_chunks)(png, i, &u as *const _ as *const c_void, 1)
                    });
                    st(c, png, i);
                },
            );
        }
    }
    // num_unknowns <= 0 / NULL array: silently ignored.
    for n in [0i32, -1, -99] {
        duo(&format!("U3 set_unknown_chunks num={n}"), &|c, png, i, _l| unsafe {
            let u = mk(b"prVt", 1, 7);
            (c.set_unknown_chunks)(png, i, &u as *const _ as *const c_void, n);
            st(c, png, i);
        });
    }
    duo("U4 set_unknown_chunks NULL array", &|c, png, i, _l| unsafe {
        (c.set_unknown_chunks)(png, i, std::ptr::null(), 1);
        st(c, png, i);
    });
    // Invalid chunk names are stored verbatim (only the write code validates).
    for name in [b"    ", b"1234", b"\x00\x01\x02\x03", b"IHDR", b"IDAT"] {
        quad(
            &format!("U5 unknown name={}", String::from_utf8_lossy(name)),
            &|c, png, i, _l| unsafe {
                let u = mk(name, 1, 7);
                sub("set", || {
                    (c.set_unknown_chunks)(png, i, &u as *const _ as *const c_void, 1)
                });
                st(c, png, i);
            },
        );
    }
    // size 0 -> a NULL data pointer is stored.
    quad("U6 unknown size=0", &|c, png, i, _l| unsafe {
        let u = mk(b"prVt", 1, 0);
        sub("set", || {
            (c.set_unknown_chunks)(png, i, &u as *const _ as *const c_void, 1)
        });
        st(c, png, i);
    });
    // pngset.c:1468 "too many unknown chunks" / :1505 "unknown chunk: out of
    // memory"
    for k in 1..=2usize {
        quad_mem(&format!("U7 unknown oom k={k}"), &|c, png, i, _l| unsafe {
            let u = mk(b"prVt", 1, 7);
            arm(k);
            sub("set", || {
                (c.set_unknown_chunks)(png, i, &u as *const _ as *const c_void, 1)
            });
            disarm();
            st(c, png, i);
        });
    }
    // pngset.c:1540 "invalid unknown chunk location"
    for loc in [0, 4, 16, -1, 99, 1, 8] {
        quad(&format!("U8 unknown_chunk_location {loc}"), &|c, png, i, _l| unsafe {
            let u = mk(b"prVt", 1, 7);
            (c.set_unknown_chunks)(png, i, &u as *const _ as *const c_void, 1);
            sub("loc", || (c.set_unknown_chunk_location)(png, i, 0, loc));
            st(c, png, i);
        });
    }
    // An out-of-range chunk index is ignored.
    for idx in [-1, 1, 99] {
        duo(
            &format!("U9 unknown_chunk_location idx={idx}"),
            &|c, png, i, _l| unsafe {
                let u = mk(b"prVt", 1, 7);
                (c.set_unknown_chunks)(png, i, &u as *const _ as *const c_void, 1);
                sub("loc", || {
                    (c.set_unknown_chunk_location)(png, i, idx, PNG_AFTER_IDAT)
                });
                st(c, png, i);
            },
        );
    }
    solo("U10 unknown_chunk_location NULL", &|c, _| unsafe {
        (c.set_unknown_chunk_location)(NP, NP, 0, 1);
        (c.set_unknown_chunks)(NP, NP, std::ptr::null(), 1);
        log("ok".to_string());
    });

    // pngset.c:1600 png_set_keep_unknown_chunks
    let list = [b'p', b'r', b'V', b't', 0, b'x', b'y', b'z', b'w', 0];
    for keep in [-2, -1, 0, 1, 2, 3, 4, 99] {
        quad(&format!("U11 keep_unknown keep={keep}"), &|c, png, _i, _l| unsafe {
            sub("keep", || (c.set_keep_unknown_chunks)(png, keep, list.as_ptr(), 2));
            log(format!(
                "prVt={} xyzw={}",
                (c.handle_as_unknown)(png, list.as_ptr()),
                (c.handle_as_unknown)(png, list[5..].as_ptr())
            ));
        });
    }
    // pngset.c:1665 "no chunk list"
    for n in [1, 2, 99] {
        quad(&format!("U12 keep_unknown NULL list n={n}"), &|c, png, _i, _l| unsafe {
            sub("keep", || {
                (c.set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_NEVER, std::ptr::null(), n)
            });
        });
    }
    // num_chunks <= 0 just sets the default.
    for n in [0, -1] {
        quad(&format!("U13 keep_unknown n={n}"), &|c, png, _i, _l| unsafe {
            sub("keep", || {
                (c.set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, list.as_ptr(), n)
            });
            log(format!("prVt={}", (c.handle_as_unknown)(png, list.as_ptr())));
        });
    }
    // pngset.c:1681 "too many chunks" (the count is checked before any access).
    for n in [858_993_460i32, 900_000_000, i32::MAX] {
        quad(&format!("U14 keep_unknown n={n}"), &|c, png, _i, _l| unsafe {
            sub("keep", || {
                (c.set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_NEVER, list.as_ptr(), n)
            });
        });
    }
    solo("U15 keep_unknown NULL png", &|c, _| unsafe {
        (c.set_keep_unknown_chunks)(NP, 1, std::ptr::null(), 1);
        log(format!("handle={}", (c.handle_as_unknown)(NP, std::ptr::null())));
    });
    duo("U16 handle_as_unknown NULL name", &|c, png, _i, _l| unsafe {
        log(format!("handle={}", (c.handle_as_unknown)(png, std::ptr::null())));
    });
    // png_get_unknown_chunks with a NULL output pointer.
    duo("U17 get_unknown_chunks NULL out", &|c, png, i, _l| unsafe {
        log(format!(
            "n={}",
            (c.get_unknown_chunks)(png, i, std::ptr::null_mut())
        ));
    });
}

// ===========================================================================
// 11. memory API
// ===========================================================================

use std::cell::Cell;

thread_local! {
    /// png_set_longjmp_fn of the library under test, for the free callback below.
    static SLJ: Cell<usize> = const { Cell::new(0) };
    /// Countdown: when it reaches 1 the free callback calls png_set_longjmp_fn.
    static FREE_AT: Cell<i64> = const { Cell::new(-1) };
}

unsafe extern "C" fn j_free(png: *mut c_void, ptr: *mut c_void) {
    let n = FREE_AT.with(|c| c.get());
    if n > 0 {
        FREE_AT.with(|c| c.set(n - 1));
        if n == 1 {
            log("FREE_TRIGGER".to_string());
            let f: unsafe extern "C" fn(Png, *const c_void, usize) -> *mut c_void =
                std::mem::transmute(SLJ.with(|c| c.get()));
            let r = f(png, shim().longjmp_ptr, 8);
            log(format!("reentrant_set_longjmp={}", (!r.is_null()) as u8));
        }
    }
    cb_free(png, ptr)
}

#[test]
fn memory_api() {
    // png_malloc / png_calloc / png_malloc_warn / png_malloc_base with size 0
    // and with sizes no allocator can satisfy ("Out of memory").
    for &sz in &[0u64, 1, 0x7fff_ffff, u64::MAX, u64::MAX / 2, 1 << 62] {
        duo(&format!("M1 png_malloc {sz}"), &|c, png, _i, _l| unsafe {
            sub("malloc", || {
                let q = (c.malloc)(png, sz);
                log(format!("malloc={}", (!q.is_null()) as u8));
                (c.free)(png, q);
            });
        });
        duo(&format!("M2 png_calloc {sz}"), &|c, png, _i, _l| unsafe {
            sub("calloc", || {
                let q = (c.calloc)(png, sz);
                log(format!("calloc={}", (!q.is_null()) as u8));
                (c.free)(png, q);
            });
        });
        duo(&format!("M3 png_malloc_warn {sz}"), &|c, png, _i, _l| unsafe {
            sub("mallocw", || {
                let q = (c.malloc_warn)(png, sz);
                log(format!("malloc_warn={}", (!q.is_null()) as u8));
                (c.free)(png, q);
            });
        });
        duo(&format!("M4 png_malloc_base {sz}"), &|c, png, _i, lib| unsafe {
            let f: unsafe extern "C" fn(Png, u64) -> *mut c_void = lib.f("png_malloc_base");
            sub("base", || {
                let q = f(png, sz);
                log(format!("base={}", (!q.is_null()) as u8));
                (c.free)(png, q);
            });
        });
        solo(&format!("M5 png_malloc NULL {sz}"), &|c, lib| unsafe {
            let base: unsafe extern "C" fn(Png, u64) -> *mut c_void = lib.f("png_malloc_base");
            let q = (c.malloc)(NP, sz);
            log(format!("malloc={}", (!q.is_null()) as u8));
            let w = (c.malloc_warn)(NP, sz);
            log(format!("malloc_warn={}", (!w.is_null()) as u8));
            let cl = (c.calloc)(NP, sz);
            log(format!("calloc={}", (!cl.is_null()) as u8));
            let b = base(NP, sz);
            log(format!("base={}", (!b.is_null()) as u8));
            (c.free)(NP, q);
            (c.free)(NP, w);
            (c.free)(NP, cl);
            (c.free)(NP, b);
        });
    }
    // png_free with NULL.
    duo("M6 png_free NULL ptr", &|c, png, _i, _l| unsafe {
        (c.free)(png, NP);
        log("ok".to_string());
    });
    solo("M7 png_free NULL png", &|c, _| unsafe {
        (c.free)(NP, NP);
        log("ok".to_string());
    });
    // png_malloc_default / png_free_default (deprecated, bypass the user
    // allocator).
    for &sz in &[0u64, 8, u64::MAX] {
        quad_mem(&format!("M8 malloc_default {sz}"), &|_c, png, _i, lib| unsafe {
            let md: unsafe extern "C" fn(Png, u64) -> *mut c_void = lib.f("png_malloc_default");
            let fd: unsafe extern "C" fn(Png, *mut c_void) = lib.f("png_free_default");
            sub("md", || {
                let q = md(png, sz);
                log(format!("default={}", (!q.is_null()) as u8));
                fd(png, q);
            });
        });
    }

    // pngmem.c:121 png_malloc_array / :132 png_realloc_array internal errors.
    for &(n, es) in &[
        (0i32, 8usize),
        (-1, 8),
        (1, 0),
        (-1, 0),
        (i32::MAX, 8),
        (1 << 20, 1 << 20),
        (4, 8),
    ] {
        duo(
            &format!("M9 malloc_array n={n} es={es}"),
            &|c, png, _i, lib| unsafe {
                let f: unsafe extern "C" fn(Png, c_int, usize) -> *mut c_void =
                    lib.f("png_malloc_array");
                sub("arr", || {
                    let q = f(png, n, es);
                    log(format!("array={}", (!q.is_null()) as u8));
                    (c.free)(png, q);
                });
            },
        );
    }
    for &(oldn, add, es, oldnull) in &[
        (0i32, 0i32, 8usize, true),
        (0, -1, 8, true),
        (0, 4, 0, true),
        (-1, 4, 8, true),
        (4, 4, 8, true),
        (0, 4, 8, true),
        (2, 2, 8, false),
        (2, i32::MAX, 8, false),
        (2, 4, 1 << 40, false),
    ] {
        duo(
            &format!("M10 realloc_array {oldn}+{add} es={es} null={oldnull}"),
            &|c, png, _i, lib| unsafe {
                type F = unsafe extern "C" fn(Png, *const c_void, c_int, c_int, usize) -> *mut c_void;
                let f: F = lib.f("png_realloc_array");
                let buf = [0u8; 64];
                let old = if oldnull {
                    std::ptr::null()
                } else {
                    buf.as_ptr() as *const c_void
                };
                sub("arr", || {
                    let q = f(png, old, oldn, add, es);
                    log(format!("array={}", (!q.is_null()) as u8));
                    (c.free)(png, q);
                });
            },
        );
    }

    // png.c:104 png_zalloc: "Potential overflow in png_zalloc()" needs
    // items >= SIZE_MAX/size, which no 32-bit uInt pair can reach on a 64-bit
    // host; the call is made anyway to prove both libraries agree.
    for &(items, size) in &[
        (0u32, 0u32),
        (1, 0),
        (0, 1),
        (1, 1),
        (0xffff_ffff, 1),
        (0xffff_ffff, 0xffff_ffff),
        (0x1_0000, 0x1_0000),
        (0xffff, 0xffff),
    ] {
        duo(
            &format!("M11 png_zalloc {items}x{size}"),
            &|c, png, _i, lib| unsafe {
                type F = unsafe extern "C" fn(*mut c_void, c_uint, c_uint) -> *mut c_void;
                let f: F = lib.f("png_zalloc");
                let zf: unsafe extern "C" fn(*mut c_void, *mut c_void) = lib.f("png_zfree");
                sub("zalloc", || {
                    let q = f(png, items, size);
                    log(format!("zalloc={}", (!q.is_null()) as u8));
                    zf(png, q);
                });
                let _ = c;
            },
        );
    }
    solo("M12 png_zalloc NULL", &|_c, lib| unsafe {
        type F = unsafe extern "C" fn(*mut c_void, c_uint, c_uint) -> *mut c_void;
        let f: F = lib.f("png_zalloc");
        let zf: unsafe extern "C" fn(*mut c_void, *mut c_void) = lib.f("png_zfree");
        log(format!("zalloc={}", (!f(NP, 4, 4).is_null()) as u8));
        zf(NP, NP);
    });

    // The allocation-failure path of png_create_*_struct_2 itself: the limit is
    // armed *before* the struct is created, so k=0 fails the png_struct
    // allocation, k=1 the info struct, and so on.
    for k in 0..=5usize {
        for &w in &[false, true] {
            go(
                &format!("M13 create_struct_2 malloc_limit={k} {}", if w { "W" } else { "R" }),
                w,
                &[],
                true,
                None,
                Some(k),
                &|c, png, i, _l| unsafe {
                    log(format!("alive={}", (!png.is_null()) as u8));
                    st(c, png, i);
                },
            );
        }
    }

    // pngerror.c:593 "Libpng jmp_buf still allocated": png_free_jmpbuf hands the
    // heap jmp_buf to the user free callback while png_struct::jmp_buf_ptr
    // points at a stack buffer with size 0; a callback that re-enters
    // png_set_longjmp_fn observes exactly that state.
    for k in 1..=3i64 {
        diff(&format!("M14 reentrant set_longjmp in free k={k}"), |lib| {
            session_reset(Vec::new());
            let c = Core::new(lib);
            SLJ.with(|x| x.set(lib.raw("png_set_longjmp_fn") as usize));
            FREE_AT.with(|x| x.set(-1));
            let rc = protected(|| unsafe {
                let png = (c.create_read_2)(
                    VER_STRING.as_ptr() as *const c_char,
                    NP,
                    cb_error as Cb,
                    cb_warning as Cb,
                    NP,
                    cb_malloc as Cb,
                    j_free as Cb,
                );
                log(format!("create={}", (!png.is_null()) as u8));
                if png.is_null() {
                    return;
                }
                // A jmp_buf larger than the built-in one is heap allocated.
                let big = shim().jmp_buf_size + 128;
                let r = (c.set_longjmp)(png, shim().longjmp_ptr, big);
                log(format!("set_longjmp={}", (!r.is_null()) as u8));
                let mut pp = png;
                let mut ii: Info = NP;
                FREE_AT.with(|x| x.set(k));
                (c.destroy_read)(&mut pp, &mut ii, NPP);
                FREE_AT.with(|x| x.set(-1));
                log("destroyed".to_string());
            });
            FREE_AT.with(|x| x.set(-1));
            Trace {
                lines: take_log(),
                out: take_out(),
                rc,
            }
        });
    }

    // pngset.c:1797 png_set_compression_buffer_size: 0 and > 2^31-1 are fatal,
    // < 6 warns on a write struct, and any value is accepted on a read struct.
    for &sz in &[0usize, 1, 5, 6, 7, 0x7fff_ffff, 0x8000_0000, usize::MAX] {
        quad(&format!("M15 compression_buffer_size {sz}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_compression_buffer_size)(png, sz));
            log(format!("size={}", (c.get_compression_buffer_size)(png)));
        });
    }
    // The same after the zstream has been claimed (write side): "cannot be
    // changed because it is in use".
    dw("M16 compression_buffer_size while in use", &|c, png, i, _l| unsafe {
        (c.set_IHDR)(png, i, 2, 1, 8, 0, 0, 0, 0);
        (c.write_info)(png, i);
        let row = [1u8, 2];
        (c.write_row)(png, row.as_ptr());
        sub("set", || (c.set_compression_buffer_size)(png, 1024));
        log(format!("size={}", (c.get_compression_buffer_size)(png)));
    });
    solo("M17 compression_buffer_size NULL", &|c, _| unsafe {
        (c.set_compression_buffer_size)(NP, 0);
        log(format!("size={}", (c.get_compression_buffer_size)(NP)));
    });
}

// ===========================================================================
// 12. generic FFI boundary sweep: NULL first argument and out-of-range ints
// ===========================================================================

macro_rules! nullcase {
    ($label:expr, |$c:ident, $lib:ident| $body:block) => {
        solo($label, &|$c, $lib| unsafe { $body })
    };
    ($label:expr, |$c:ident| $body:block) => {
        solo($label, &|$c, _lib| unsafe { $body })
    };
}

#[test]
fn ffi_null_sweep() {
    // ---- getters ----------------------------------------------------------
    nullcase!("F1 get_rowbytes", |c| {
        log(format!("{}", (c.get_rowbytes)(NP, NP)))
    });
    nullcase!("F2 get_rows", |c| {
        log(format!("{}", (!(c.get_rows)(NP, NP).is_null()) as u8))
    });
    nullcase!("F3 get_valid", |c| {
        log(format!("{}", (c.get_valid)(NP, NP, PNG_INFO_PLTE)))
    });
    nullcase!("F4 get_channels", |c| {
        log(format!("{}", (c.get_channels)(NP, NP)))
    });
    nullcase!("F5 get_bit_depth", |c| {
        log(format!("{}", (c.get_bit_depth)(NP, NP)))
    });
    nullcase!("F6 get_color_type", |c| {
        log(format!("{}", (c.get_color_type)(NP, NP)))
    });
    nullcase!("F7 get_filter_type", |c| {
        log(format!("{}", (c.get_filter_type)(NP, NP)))
    });
    nullcase!("F8 get_interlace_type", |c| {
        log(format!("{}", (c.get_interlace_type)(NP, NP)))
    });
    nullcase!("F9 get_compression_type", |c| {
        log(format!("{}", (c.get_compression_type)(NP, NP)))
    });
    nullcase!("F10 get_image_width/height", |c| {
        log(format!(
            "{} {}",
            (c.get_image_width)(NP, NP),
            (c.get_image_height)(NP, NP)
        ))
    });
    nullcase!("F11 get_palette_max", |c| {
        log(format!("{}", (c.get_palette_max)(NP, NP)))
    });
    nullcase!("F12 get_io_ptr", |c| {
        log(format!("{}", (!(c.get_io_ptr)(NP).is_null()) as u8))
    });
    nullcase!("F13 get_io_chunk_type", |c| {
        log(format!("{}", (c.get_io_chunk_type)(NP)))
    });
    nullcase!("F14 get_mem_ptr", |c| {
        log(format!("{}", (!(c.get_mem_ptr)(NP).is_null()) as u8))
    });
    nullcase!("F15 get_error_ptr", |c| {
        log(format!("{}", (!(c.get_error_ptr)(NP).is_null()) as u8))
    });
    nullcase!("F16 get_user_transform_ptr", |c| {
        log(format!(
            "{}",
            (!(c.get_user_transform_ptr)(NP).is_null()) as u8
        ))
    });
    nullcase!("F17 get_user_chunk_ptr", |c| {
        log(format!("{}", (!(c.get_user_chunk_ptr)(NP).is_null()) as u8))
    });
    nullcase!("F18 get_progressive_ptr", |c| {
        log(format!("{}", (!(c.get_progressive_ptr)(NP).is_null()) as u8))
    });
    nullcase!("F19 get_compression_buffer_size", |c| {
        log(format!("{}", (c.get_compression_buffer_size)(NP)))
    });
    nullcase!("F20 get_user_limits", |c| {
        log(format!(
            "{} {} {} {}",
            (c.get_user_width_max)(NP),
            (c.get_user_height_max)(NP),
            (c.get_chunk_cache_max)(NP),
            (c.get_chunk_malloc_max)(NP)
        ))
    });
    nullcase!("F21 get_current_row/pass", |c| {
        log(format!(
            "{} {}",
            (c.get_current_row_number)(NP),
            (c.get_current_pass_number)(NP)
        ))
    });
    nullcase!("F22 get_rgb_to_gray_status", |c| {
        log(format!("{}", (c.get_rgb_to_gray_status)(NP)))
    });
    nullcase!("F23 chunk getters", |c| {
        let mut i32v = [0i32; 9];
        let mut u32v = [0u32; 2];
        let mut pp: *mut u8 = std::ptr::null_mut();
        let mut ppc: *mut c_char = std::ptr::null_mut();
        let mut pu: *mut u16 = std::ptr::null_mut();
        let mut pv: *mut c_void = std::ptr::null_mut();
        let mut ci: c_int = -1;
        let mut f64v = -1.0f64;
        log(format!("PLTE={}", (c.get_PLTE)(NP, NP, &mut pp, &mut ci)));
        log(format!(
            "tRNS={}",
            (c.get_tRNS)(NP, NP, &mut pp, &mut ci, &mut pp)
        ));
        log(format!("gAMA={}", (c.get_gAMA)(NP, NP, &mut f64v)));
        log(format!(
            "gAMAfx={}",
            (c.get_gAMA_fixed)(NP, NP, &mut i32v[0])
        ));
        log(format!("sRGB={}", (c.get_sRGB)(NP, NP, &mut ci)));
        log(format!(
            "cHRM={}",
            (c.get_cHRM_fixed)(
                NP,
                NP,
                &mut i32v[0],
                &mut i32v[1],
                &mut i32v[2],
                &mut i32v[3],
                &mut i32v[4],
                &mut i32v[5],
                &mut i32v[6],
                &mut i32v[7]
            )
        ));
        log(format!(
            "cHRMXYZ={}",
            (c.get_cHRM_XYZ_fixed)(
                NP,
                NP,
                &mut i32v[0],
                &mut i32v[1],
                &mut i32v[2],
                &mut i32v[3],
                &mut i32v[4],
                &mut i32v[5],
                &mut i32v[6],
                &mut i32v[7],
                &mut i32v[8]
            )
        ));
        log(format!(
            "iCCP={}",
            (c.get_iCCP)(NP, NP, &mut ppc, &mut ci, &mut pp, &mut u32v[0])
        ));
        log(format!("sBIT={}", (c.get_sBIT)(NP, NP, &mut pp)));
        log(format!("bKGD={}", (c.get_bKGD)(NP, NP, &mut pp)));
        log(format!("hIST={}", (c.get_hIST)(NP, NP, &mut pu)));
        log(format!(
            "pHYs={}",
            (c.get_pHYs)(NP, NP, &mut u32v[0], &mut u32v[1], &mut ci)
        ));
        log(format!(
            "oFFs={}",
            (c.get_oFFs)(NP, NP, &mut i32v[0], &mut i32v[1], &mut ci)
        ));
        log(format!("tIME={}", (c.get_tIME)(NP, NP, &mut pp)));
        log(format!("sPLT={}", (c.get_sPLT)(NP, NP, &mut pv)));
        log(format!(
            "eXIf_1={}",
            (c.get_eXIf_1)(NP, NP, &mut u32v[0], &mut pp)
        ));
        log(format!(
            "cICP={}",
            (c.get_cICP)(
                NP,
                NP,
                &mut (0u8),
                &mut (0u8),
                &mut (0u8),
                &mut (0u8)
            )
        ));
        log(format!(
            "cLLI={}",
            (c.get_cLLI_fixed)(NP, NP, &mut u32v[0], &mut u32v[1])
        ));
        log(format!(
            "mDCV={}",
            (c.get_mDCV_fixed)(
                NP,
                NP,
                &mut i32v[0],
                &mut i32v[1],
                &mut i32v[2],
                &mut i32v[3],
                &mut i32v[4],
                &mut i32v[5],
                &mut i32v[6],
                &mut i32v[7],
                &mut u32v[0],
                &mut u32v[1]
            )
        ));
        log(format!("text={}", (c.get_text)(NP, NP, &mut pv, &mut ci)));
        log(format!("unknown={}", (c.get_unknown_chunks)(NP, NP, &mut pv)));
        log(format!(
            "sCAL_s={}",
            (c.get_sCAL_s)(NP, NP, &mut ci, &mut ppc, &mut ppc)
        ));
        let mut params: *mut *mut c_char = std::ptr::null_mut();
        log(format!(
            "pCAL={}",
            (c.get_pCAL)(
                NP,
                NP,
                &mut ppc,
                &mut i32v[0],
                &mut i32v[1],
                &mut ci,
                &mut ci,
                &mut ppc,
                &mut params
            )
        ));
    });

    // ---- setters ----------------------------------------------------------
    nullcase!("F24 chunk setters NULL png", |c| {
        let z = [0u8; 16];
        let t = PngTime::default();
        (c.set_PLTE)(NP, NP, z.as_ptr(), 1);
        (c.set_tRNS)(NP, NP, z.as_ptr(), 1, z.as_ptr());
        (c.set_gAMA)(NP, NP, 1.0);
        (c.set_gAMA_fixed)(NP, NP, FP1);
        (c.set_sRGB)(NP, NP, 0);
        (c.set_sRGB_gAMA_and_cHRM)(NP, NP, 0);
        (c.set_cHRM_fixed)(NP, NP, 1, 1, 1, 1, 1, 1, 1, 1);
        (c.set_cHRM_XYZ_fixed)(NP, NP, 1, 1, 1, 1, 1, 1, 1, 1, 1);
        (c.set_iCCP)(NP, NP, std::ptr::null(), 0, z.as_ptr(), 0);
        (c.set_sBIT)(NP, NP, z.as_ptr());
        (c.set_bKGD)(NP, NP, z.as_ptr());
        (c.set_hIST)(NP, NP, z.as_ptr() as *const u16);
        (c.set_pHYs)(NP, NP, 1, 1, 1);
        (c.set_oFFs)(NP, NP, 1, 1, 1);
        (c.set_tIME)(NP, NP, &t as *const _ as *const u8);
        (c.set_pCAL)(
            NP,
            NP,
            std::ptr::null(),
            0,
            0,
            0,
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        );
        (c.set_sCAL_s)(NP, NP, 1, std::ptr::null(), std::ptr::null());
        (c.set_sPLT)(NP, NP, std::ptr::null(), 1);
        (c.set_eXIf_1)(NP, NP, 0, z.as_ptr());
        (c.set_cICP)(NP, NP, 1, 1, 0, 1);
        (c.set_cLLI_fixed)(NP, NP, 1, 1);
        (c.set_mDCV_fixed)(NP, NP, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1);
        (c.set_text)(NP, NP, std::ptr::null(), 1);
        (c.set_IHDR)(NP, NP, 1, 1, 8, 0, 0, 0, 0);
        (c.set_invalid)(NP, NP, -1);
        log("ok".to_string());
    });
    // png_set_sCAL / _fixed with a NULL struct: the argument checks come first
    // and only issue warnings (which go to stderr without a struct).
    nullcase!("F25 set_sCAL NULL png", |c| {
        (c.set_sCAL)(NP, NP, 1, 0.0, 1.0);
        (c.set_sCAL_fixed)(NP, NP, 1, 0, FP1);
        log("ok".to_string());
    });
    nullcase!("F26 transform setters NULL png", |c| {
        (c.set_bgr)(NP);
        (c.set_swap)(NP);
        (c.set_swap_alpha)(NP);
        (c.set_packing)(NP);
        (c.set_packswap)(NP);
        (c.set_invert_mono)(NP);
        (c.set_invert_alpha)(NP);
        (c.set_strip_16)(NP);
        (c.set_scale_16)(NP);
        (c.set_strip_alpha)(NP);
        (c.set_expand)(NP);
        (c.set_expand_16)(NP);
        (c.set_expand_gray_1_2_4_to_8)(NP);
        (c.set_palette_to_rgb)(NP);
        (c.set_tRNS_to_alpha)(NP);
        (c.set_gray_to_rgb)(NP);
        (c.set_filler)(NP, 0, 0);
        (c.set_add_alpha)(NP, 0, 0);
        (c.set_shift)(NP, std::ptr::null());
        (c.set_quantize)(NP, NP as *mut u8, 1, 1, std::ptr::null(), 0);
        (c.set_gamma)(NP, 1.0, 1.0);
        (c.set_gamma_fixed)(NP, FP1, FP1);
        (c.set_alpha_mode)(NP, 0, 1.0);
        (c.set_alpha_mode_fixed)(NP, 0, FP1);
        (c.set_rgb_to_gray)(NP, 1, 0.2, 0.7);
        (c.set_rgb_to_gray_fixed)(NP, 1, -1, -1);
        (c.set_background)(NP, std::ptr::null(), 1, 0, 1.0);
        (c.set_background_fixed)(NP, std::ptr::null(), 1, 0, FP1);
        (c.set_crc_action)(NP, 0, 0);
        log("ok".to_string());
    });
    nullcase!("F27 config setters NULL png", |c| {
        (c.set_compression_level)(NP, 6);
        (c.set_compression_mem_level)(NP, 8);
        (c.set_compression_strategy)(NP, 0);
        (c.set_compression_window_bits)(NP, 15);
        (c.set_compression_method)(NP, 8);
        (c.set_text_compression_level)(NP, 6);
        (c.set_text_compression_mem_level)(NP, 8);
        (c.set_text_compression_strategy)(NP, 0);
        (c.set_text_compression_window_bits)(NP, 15);
        (c.set_text_compression_method)(NP, 8);
        (c.set_filter)(NP, 0, PNG_ALL_FILTERS);
        (c.set_flush)(NP, 1);
        (c.set_user_limits)(NP, 1, 1);
        (c.set_chunk_cache_max)(NP, 1);
        (c.set_chunk_malloc_max)(NP, 1);
        (c.set_write_user_transform_fn)(NP, NP);
        (c.set_user_transform_info)(NP, NP, 8, 3);
        (c.set_read_user_chunk_fn)(NP, NP, NP);
        (c.set_error_fn)(NP, NP, NP, NP);
        (c.set_mem_fn)(NP, NP, NP, NP);
        (c.set_read_fn)(NP, NP, NP);
        (c.set_write_fn)(NP, NP, NP, NP);
        (c.set_read_status_fn)(NP, NP);
        (c.set_write_status_fn)(NP, NP);
        (c.set_progressive_read_fn)(NP, NP, NP, NP, NP);
        (c.set_sig_bytes)(NP, 3);
        (c.set_rows)(NP, NP, std::ptr::null_mut());
        log(format!("option={}", (c.set_option)(NP, 0, 1)));
        log(format!("mng={}", (c.permit_mng_features)(NP, 0xff)));
        log(format!("reset_zstream={}", (c.reset_zstream)(NP)));
        log(format!("interlace={}", (c.set_interlace_handling)(NP)));
    });
    // Read/write entry points with a NULL struct.
    nullcase!("F28 read/write NULL png", |c| {
        (c.read_info)(NP, NP);
        (c.read_update_info)(NP, NP);
        (c.start_read_image)(NP);
        (c.read_row)(NP, std::ptr::null_mut(), std::ptr::null_mut());
        (c.read_rows)(NP, std::ptr::null_mut(), std::ptr::null_mut(), 1);
        (c.read_image)(NP, std::ptr::null_mut());
        (c.read_end)(NP, NP);
        (c.read_png)(NP, NP, 0, NP);
        (c.write_info)(NP, NP);
        (c.write_info_before_PLTE)(NP, NP);
        (c.write_row)(NP, std::ptr::null());
        (c.write_rows)(NP, std::ptr::null_mut(), 1);
        (c.write_image)(NP, std::ptr::null_mut());
        (c.write_end)(NP, NP);
        (c.write_png)(NP, NP, 0, NP);
        (c.write_flush)(NP);
        (c.write_sig)(NP);
        (c.process_data)(NP, NP, std::ptr::null_mut(), 0);
        log(format!("pause={}", (c.process_data_pause)(NP, 0)));
        (c.progressive_combine_row)(NP, std::ptr::null_mut(), std::ptr::null());
        log("ok".to_string());
        // NOTE: png_process_data_skip(NULL) and png_get_io_state(NULL)
        // dereference the NULL pointer in upstream libpng and would abort the
        // test process; they are deliberately not called here.
    });
    nullcase!("F29 write_chunk NULL png", |c| {
        let n = *b"prVt";
        (c.write_chunk)(NP, n.as_ptr(), std::ptr::null(), 0);
        (c.write_chunk_start)(NP, n.as_ptr(), 0);
        (c.write_chunk_data)(NP, std::ptr::null(), 0);
        (c.write_chunk_end)(NP);
        log("ok".to_string());
    });
    nullcase!("F30 build_grayscale_palette NULL", |c| {
        (c.build_grayscale_palette)(8, std::ptr::null_mut());
        log("ok".to_string());
    });
}

#[test]
fn ffi_enum_ranges() {
    // ---- png.c:53 png_set_sig_bytes: > 8 is fatal, < 0 clamps to 0.
    for n in [-99, -1, 0, 1, 8, 9, 99, i32::MAX] {
        duo(&format!("F31 set_sig_bytes {n}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_sig_bytes)(png, n));
        });
    }
    // ---- png.c:80 png_sig_cmp with degenerate start / num_to_check.
    let sig = support::pngbuild::SIG;
    let bad = [0u8; 8];
    for &(start, num) in &[
        (0usize, 8usize),
        (0, 0),
        (0, 1),
        (0, 9),
        (0, usize::MAX),
        (7, 1),
        (7, 8),
        (8, 1),
        (8, 0),
        (99, 1),
        (usize::MAX, 1),
        (3, 0),
        (4, 4),
    ] {
        solo(&format!("F32 sig_cmp {start},{num}"), &|c, _| unsafe {
            log(format!(
                "good={} bad={}",
                (c.sig_cmp)(sig.as_ptr(), start, num),
                (c.sig_cmp)(bad.as_ptr(), start, num)
            ));
        });
    }
    // ---- png.c:3769 png_set_option: option must be even and in range.
    for opt in [-1, 0, 1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 99] {
        for on in [-1, 0, 1, 2, 3, 99] {
            duo(
                &format!("F33 set_option {opt},{on}"),
                &|c, png, _i, _l| unsafe {
                    log(format!("ret={}", (c.set_option)(png, opt, on)));
                },
            );
        }
    }
    // ---- pngset.c:1557 png_permit_mng_features with unknown bits.
    for m in [0u32, 1, 4, 5, 7, 0xffff_ffff] {
        duo(&format!("F34 permit_mng {m:#x}"), &|c, png, _i, _l| unsafe {
            log(format!("ret={:#x}", (c.permit_mng_features)(png, m)));
        });
    }
    // ---- pngwrite.c:1058 png_set_filter with a bad method / bad filter mask.
    for method in [-1, 0, 1, 64, 99] {
        for filters in [-1, 0, 0x08, 0xf8, 0x07, 0xff, 999] {
            dw(
                &format!("F35 set_filter {method},{filters:#x}"),
                &|c, png, i, _l| unsafe {
                    (c.set_IHDR)(png, i, 1, 1, 8, 0, 0, 0, 0);
                    sub("set", || (c.set_filter)(png, method, filters));
                },
            );
        }
    }
    // ---- zlib configuration out of range (pngwrite.c).
    for v in [-2, -1, 0, 1, 9, 10, 99] {
        dw(&format!("F36 compression_level {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_compression_level)(png, v));
        });
        dw(&format!("F37 compression_mem_level {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_compression_mem_level)(png, v));
        });
        dw(&format!("F38 compression_strategy {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_compression_strategy)(png, v));
        });
        dw(&format!("F39 compression_window_bits {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_compression_window_bits)(png, v));
        });
        dw(&format!("F40 compression_method {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_compression_method)(png, v));
        });
        dw(&format!("F41 text_compression_level {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_text_compression_level)(png, v));
        });
        dw(&format!("F42 text_compression_mem_level {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_text_compression_mem_level)(png, v));
        });
        dw(&format!("F43 text_compression_strategy {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_text_compression_strategy)(png, v));
        });
        dw(&format!("F44 text_compression_window_bits {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_text_compression_window_bits)(png, v));
        });
        dw(&format!("F45 text_compression_method {v}"), &|c, png, _i, _l| unsafe {
            sub("set", || (c.set_text_compression_method)(png, v));
        });
    }
    // ---- png.c:880 png_build_grayscale_palette with an unsupported depth.
    for bd in [-1, 0, 1, 2, 3, 4, 5, 8, 16, 99] {
        solo(&format!("F46 build_grayscale_palette {bd}"), &|c, _| unsafe {
            let mut pal = [0u8; 3 * 256];
            (c.build_grayscale_palette)(bd, pal.as_mut_ptr());
            log(format!("pal={}", hex(&pal[..24])));
        });
    }
    // ---- png.c:979 png_reset_zstream before anything has been read.
    duo("F47 reset_zstream fresh", &|c, png, _i, _l| unsafe {
        log(format!("ret={}", (c.reset_zstream)(png)));
    });
    // ---- png_set_user_limits / chunk limits with 0 and extreme values.
    for &(w, h) in &[(0u32, 0u32), (1, 1), (0x7fff_ffff, 0x7fff_ffff), (0xffff_ffff, 0xffff_ffff)] {
        duo(&format!("F48 user_limits {w},{h}"), &|c, png, _i, _l| unsafe {
            (c.set_user_limits)(png, w, h);
            log(format!(
                "{} {}",
                (c.get_user_width_max)(png),
                (c.get_user_height_max)(png)
            ));
        });
    }
    for m in [0u32, 1, 0xffff_ffff] {
        duo(&format!("F49 chunk_cache_max {m}"), &|c, png, _i, _l| unsafe {
            (c.set_chunk_cache_max)(png, m);
            log(format!("{}", (c.get_chunk_cache_max)(png)));
        });
    }
    for m in [0usize, 1, usize::MAX] {
        duo(&format!("F50 chunk_malloc_max {m}"), &|c, png, _i, _l| unsafe {
            (c.set_chunk_malloc_max)(png, m);
            log(format!("{}", (c.get_chunk_malloc_max)(png)));
        });
    }
    // ---- png_set_flush with 0 / negative nrows.
    for n in [-1, 0, 1] {
        dw(&format!("F51 set_flush {n}"), &|c, png, i, _l| unsafe {
            (c.set_flush)(png, n);
            (c.set_IHDR)(png, i, 2, 2, 8, 0, 0, 0, 0);
            sub("write", || {
                (c.write_info)(png, i);
                let row = [1u8, 2];
                (c.write_row)(png, row.as_ptr());
                (c.write_row)(png, row.as_ptr());
                (c.write_end)(png, i);
            });
        });
    }
    // ---- png_set_benign_errors with values other than 0/1.
    for b in [-1, 0, 1, 2, 99] {
        duo(&format!("F52 set_benign_errors {b}"), &|c, png, _i, lib| unsafe {
            (c.set_benign_errors)(png, b);
            let f: unsafe extern "C" fn(Png, *const c_char) = lib.f("png_app_error");
            sub("apperr", || f(png, p(&cz("probe"))));
        });
    }
    // ---- png_set_check_for_invalid_index with values other than 0/1.
    for a in [-99, -1, 0, 1, 99] {
        duo(
            &format!("F53 set_check_for_invalid_index {a}"),
            &|c, png, i, _l| unsafe {
                (c.set_check_for_invalid_index)(png, a);
                log(format!("palette_max={}", (c.get_palette_max)(png, i)));
            },
        );
    }
}

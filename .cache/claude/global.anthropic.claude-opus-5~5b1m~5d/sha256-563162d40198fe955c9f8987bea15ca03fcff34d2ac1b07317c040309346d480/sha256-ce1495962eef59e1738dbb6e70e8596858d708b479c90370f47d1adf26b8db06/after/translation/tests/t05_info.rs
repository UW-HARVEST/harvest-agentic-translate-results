//! Phase B — the info/state layer.
//!
//! Every `png_set_*` / `png_get_*` pair that stores or retrieves data in a
//! `png_info` (or in the `png_struct` state) is driven with identical,
//! *valid* inputs against BOTH shared libraries and every getter result plus
//! the ordered warning transcript must match exactly.
//!
//! Reference-behaviour notes that constrain what "valid" means here are cited
//! against the C sources (`c_src/src/...`) at each site.
//!
//! Two properties of this build (`c_src/include/pnglibconf.h`) shape the tests:
//!   * `PNG_BENIGN_READ_ERRORS_SUPPORTED` is on, `PNG_BENIGN_WRITE_ERRORS` is
//!     off, and `PNG_RELEASE_BUILD` is 0 (`PNG_LIBPNG_BUILD_BASE_TYPE` is
//!     BETA, png.h:310).  So on a *fresh* struct `png_app_error` /
//!     `png_app_warning` both call `png_error` (pngerror.c:338/351) — those
//!     paths therefore only appear in `t_benign_warning_paths`, which first
//!     calls `png_set_benign_errors(p, 1)` (pngset.c:1936) to turn them into
//!     warnings so the message text can be compared.
//!   * `png_chunk_report(..., PNG_CHUNK_WRITE_ERROR)` is a *warning* on a read
//!     struct (pngerror.c:490) and an app error on a write struct.
#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]
#![allow(unused_mut)]

mod common;

use common::api::{apis, Api};
use common::harness::*;
use common::*;
use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

// ---------------------------------------------------------------------------
// constants that are not in tests/common/mod.rs
// ---------------------------------------------------------------------------

const PNG_FREE_HIST: png_uint_32 = 0x0008;
const PNG_FREE_ICCP: png_uint_32 = 0x0010;
const PNG_FREE_SPLT: png_uint_32 = 0x0020;
const PNG_FREE_ROWS: png_uint_32 = 0x0040;
const PNG_FREE_PCAL: png_uint_32 = 0x0080;
const PNG_FREE_SCAL: png_uint_32 = 0x0100;
const PNG_FREE_UNKN: png_uint_32 = 0x0200;
const PNG_FREE_PLTE: png_uint_32 = 0x1000;
const PNG_FREE_TRNS: png_uint_32 = 0x2000;
const PNG_FREE_TEXT: png_uint_32 = 0x4000;
const PNG_FREE_EXIF: png_uint_32 = 0x8000;
const PNG_FREE_ALL: png_uint_32 = 0xffff;

const PNG_DESTROY_WILL_FREE_DATA: c_int = 1;
const PNG_USER_WILL_FREE_DATA: c_int = 2;

/// `png_set_unknown_chunks` locations (png.h / pngpriv.h:640).
const LOC_IHDR: c_int = 0x01;
const LOC_PLTE: c_int = 0x02;
const LOC_AFTER_IDAT: c_int = 0x08;

/// png.h:3493 — the real value (the copy in tests/common/mod.rs is stale).
const OPTION_NEXT: c_int = 16;

const ALL_INFO_FLAGS: [(&str, png_uint_32); 20] = [
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

/// The legal (bit_depth, color_type) pairs; anything else makes
/// `png_check_IHDR` raise `png_error "Invalid IHDR data"` (png.c:2120).
const DEPTH_TYPE: [(c_int, c_int); 15] = [
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

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

fn f64s(v: f64) -> String {
    format!("{v:?}")
}
fn f32s(v: f32) -> String {
    format!("{v:?}")
}

/// Raw addresses legitimately differ between the two libraries, so pointers are
/// only ever described relative to a value the test itself handed in.
fn pdesc(got: *const c_void, expect: *const c_void) -> &'static str {
    if got.is_null() {
        "NULL"
    } else if got == expect {
        "EQ"
    } else {
        "OTHER"
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Kind {
    Read,
    Write,
}

impl Kind {
    fn tag(self) -> &'static str {
        match self {
            Kind::Read => "R",
            Kind::Write => "W",
        }
    }
}

struct Ctx {
    p: png_structp,
    info: png_infop,
    end: png_infop,
    kind: Kind,
}

unsafe fn make(a: &Api, kind: Kind) -> Ctx {
    let p = match kind {
        Kind::Read => (a.png_create_read_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
        ),
        Kind::Write => (a.png_create_write_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
        ),
    };
    assert!(!p.is_null(), "png_create_*_struct returned NULL");
    let info = (a.png_create_info_struct)(p);
    assert!(!info.is_null());
    let end = (a.png_create_info_struct)(p);
    assert!(!end.is_null());
    match kind {
        Kind::Read => (a.png_set_read_fn)(p, ptr::null_mut(), Some(read_cb)),
        Kind::Write => {
            (a.png_set_write_fn)(p, ptr::null_mut(), Some(write_cb), Some(flush_cb))
        }
    }
    Ctx {
        p,
        info,
        end,
        kind,
    }
}

unsafe fn destroy(a: &Api, c: &mut Ctx) {
    match c.kind {
        Kind::Read => (a.png_destroy_read_struct)(&mut c.p, &mut c.info, &mut c.end),
        Kind::Write => {
            (a.png_destroy_info_struct)(c.p, &mut c.end);
            (a.png_destroy_write_struct)(&mut c.p, &mut c.info);
        }
    }
}

/// Run the same closure against both libraries and compare the transcripts.
#[track_caller]
fn diff(what: &str, f: &dyn Fn(&'static Api) -> Vec<String>) {
    if std::env::var_os("PNG_TRACE").is_some() {
        eprintln!("CASE {what}");
    }
    let b = apis();
    reset_all();
    set_cur_is_c(true);
    let mut c = f(&b.c);
    c.extend(log_take().into_iter().map(|s| format!("log:{s}")));
    reset_all();
    set_cur_is_c(false);
    let mut r = f(&b.rs);
    r.extend(log_take().into_iter().map(|s| format!("log:{s}")));
    if std::env::var_os("PNG_TRACE").is_some() {
        eprintln!("  {what}: {} lines compared", c.len());
    }
    eq_dbg(what, c, r);
}

// ---------------------------------------------------------------------------
// the full info dump
// ---------------------------------------------------------------------------

/// `true` when `png_get_IHDR` (which re-runs `png_check_IHDR`, pngget.c:974)
/// will not raise `png_error "Invalid IHDR data"`.
unsafe fn ihdr_ok(a: &Api, p: png_structp, info: png_infop) -> bool {
    let w = (a.png_get_image_width)(p, info);
    let h = (a.png_get_image_height)(p, info);
    if w == 0 || h == 0 || w > 1_000_000 || h > 1_000_000 {
        return false;
    }
    if (a.png_get_compression_type)(p, info) != 0 || (a.png_get_filter_type)(p, info) != 0 {
        return false;
    }
    if (a.png_get_interlace_type)(p, info) as c_int >= 2 {
        return false;
    }
    let d = (a.png_get_bit_depth)(p, info) as c_int;
    match (a.png_get_color_type)(p, info) as c_int {
        0 => matches!(d, 1 | 2 | 4 | 8 | 16),
        2 | 4 | 6 => matches!(d, 8 | 16),
        3 => matches!(d, 1 | 2 | 4 | 8),
        _ => false,
    }
}

/// Everything reachable through the info getters.  `scal_fixed` gates
/// `png_get_sCAL_fixed`, which calls `png_fixed(atof(...))` and therefore
/// `png_error`s on overflow (pngget.c:1047) — the callers below only enable it
/// when the stored sCAL values are known to be in range.
unsafe fn dump_info(
    a: &Api,
    p: png_structp,
    info: png_infop,
    tag: &str,
    scal_fixed: bool,
) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();

    // ---- IHDR / shape ----
    if ihdr_ok(a, p, info) {
        let (mut w, mut h) = (0u32, 0u32);
        let (mut bd, mut ct, mut il, mut cm, mut fm) = (0i32, 0i32, 0i32, 0i32, 0i32);
        let r = (a.png_get_IHDR)(
            p, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm,
        );
        v.push(format!("{tag}.IHDR:{r}:{w}:{h}:{bd}:{ct}:{il}:{cm}:{fm}"));
        // the NULL-argument form must be accepted too (pngget.c:948)
        let r2 = (a.png_get_IHDR)(
            p,
            info,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        v.push(format!("{tag}.IHDR.null:{r2}"));
    } else {
        v.push(format!("{tag}.IHDR:absent"));
    }
    v.push(format!("{tag}.w:{}", (a.png_get_image_width)(p, info)));
    v.push(format!("{tag}.h:{}", (a.png_get_image_height)(p, info)));
    v.push(format!("{tag}.bd:{}", (a.png_get_bit_depth)(p, info)));
    v.push(format!("{tag}.ct:{}", (a.png_get_color_type)(p, info)));
    v.push(format!("{tag}.ch:{}", (a.png_get_channels)(p, info)));
    v.push(format!("{tag}.rb:{}", (a.png_get_rowbytes)(p, info)));
    v.push(format!("{tag}.il:{}", (a.png_get_interlace_type)(p, info)));
    v.push(format!("{tag}.cm:{}", (a.png_get_compression_type)(p, info)));
    v.push(format!("{tag}.fm:{}", (a.png_get_filter_type)(p, info)));
    v.push(format!("{tag}.pmax:{}", (a.png_get_palette_max)(p, info)));

    // ---- valid flags ----
    for (name, flag) in ALL_INFO_FLAGS {
        v.push(format!(
            "{tag}.valid.{name}:{}",
            (a.png_get_valid)(p, info, flag)
        ));
    }
    // a mask with several bits, and the all-ones mask
    v.push(format!(
        "{tag}.valid.mask:{}",
        (a.png_get_valid)(p, info, PNG_INFO_gAMA | PNG_INFO_sBIT | PNG_INFO_tRNS)
    ));
    v.push(format!(
        "{tag}.valid.all:{}",
        (a.png_get_valid)(p, info, 0xffff_ffff)
    ));

    // ---- PLTE ----
    let mut pal: *mut png_color = ptr::null_mut();
    let mut npal = 0i32;
    let got = (a.png_get_PLTE)(p, info, &mut pal, &mut npal);
    v.push(format!("{tag}.PLTE:{got}:{npal}"));
    if got != 0 && !pal.is_null() {
        for i in 0..npal.max(0) as usize {
            let c = *pal.add(i);
            v.push(format!("{tag}.PLTE[{i}]:{},{},{}", c.red, c.green, c.blue));
        }
    }

    // ---- tRNS ----
    let mut ta: *mut png_byte = ptr::null_mut();
    let mut nt = 0i32;
    let mut tc: *mut png_color_16 = ptr::null_mut();
    let got = (a.png_get_tRNS)(p, info, &mut ta, &mut nt, &mut tc);
    v.push(format!("{tag}.tRNS:{got}:{nt}"));
    if got != 0 {
        if !ta.is_null() {
            let s: Vec<u8> = (0..nt.clamp(0, 256) as usize).map(|i| *ta.add(i)).collect();
            v.push(format!("{tag}.tRNS.a:{s:?}"));
        } else {
            v.push(format!("{tag}.tRNS.a:none"));
        }
        if !tc.is_null() {
            v.push(format!("{tag}.tRNS.c:{:?}", *tc));
        } else {
            v.push(format!("{tag}.tRNS.c:none"));
        }
    }

    // ---- gAMA, both representations ----
    {
        let mut g = 0i32;
        v.push(format!(
            "{tag}.gAMA.fx:{}:{g}",
            (a.png_get_gAMA_fixed)(p, info, &mut g)
        ));
        let mut gd = 0f64;
        let r = (a.png_get_gAMA)(p, info, &mut gd);
        v.push(format!("{tag}.gAMA.fp:{r}:{}", f64s(gd)));
        // NULL out-parameter form
        v.push(format!(
            "{tag}.gAMA.null:{}",
            (a.png_get_gAMA_fixed)(p, info, ptr::null_mut())
        ));
    }

    // ---- sRGB ----
    {
        let mut si = -99i32;
        v.push(format!(
            "{tag}.sRGB:{}:{si}",
            (a.png_get_sRGB)(p, info, &mut si)
        ));
    }

    // ---- cHRM / cHRM_XYZ, both representations ----
    {
        let mut x = [0i32; 8];
        let got = (a.png_get_cHRM_fixed)(
            p, info, &mut x[0], &mut x[1], &mut x[2], &mut x[3], &mut x[4], &mut x[5], &mut x[6],
            &mut x[7],
        );
        v.push(format!("{tag}.cHRM.fx:{got}:{x:?}"));
        let mut d = [0f64; 8];
        let got = (a.png_get_cHRM)(
            p, info, &mut d[0], &mut d[1], &mut d[2], &mut d[3], &mut d[4], &mut d[5], &mut d[6],
            &mut d[7],
        );
        v.push(format!(
            "{tag}.cHRM.fp:{got}:{:?}",
            d.iter().map(|z| f64s(*z)).collect::<Vec<_>>()
        ));
        let mut y = [0i32; 9];
        let got = (a.png_get_cHRM_XYZ_fixed)(
            p, info, &mut y[0], &mut y[1], &mut y[2], &mut y[3], &mut y[4], &mut y[5], &mut y[6],
            &mut y[7], &mut y[8],
        );
        v.push(format!("{tag}.cHRMXYZ.fx:{got}:{y:?}"));
        let mut e = [0f64; 9];
        let got = (a.png_get_cHRM_XYZ)(
            p, info, &mut e[0], &mut e[1], &mut e[2], &mut e[3], &mut e[4], &mut e[5], &mut e[6],
            &mut e[7], &mut e[8],
        );
        v.push(format!(
            "{tag}.cHRMXYZ.fp:{got}:{:?}",
            e.iter().map(|z| f64s(*z)).collect::<Vec<_>>()
        ));
    }

    // ---- sBIT / bKGD / hIST ----
    {
        let mut sb: *mut png_color_8 = ptr::null_mut();
        let got = (a.png_get_sBIT)(p, info, &mut sb);
        v.push(format!(
            "{tag}.sBIT:{got}:{:?}",
            if got != 0 && !sb.is_null() {
                Some(*sb)
            } else {
                None
            }
        ));
        let mut bg: *mut png_color_16 = ptr::null_mut();
        let got = (a.png_get_bKGD)(p, info, &mut bg);
        v.push(format!(
            "{tag}.bKGD:{got}:{:?}",
            if got != 0 && !bg.is_null() {
                Some(*bg)
            } else {
                None
            }
        ));
        let mut hi: *mut png_uint_16 = ptr::null_mut();
        let got = (a.png_get_hIST)(p, info, &mut hi);
        v.push(format!("{tag}.hIST:{got}"));
        if got != 0 && !hi.is_null() {
            let s: Vec<u16> = (0..npal.clamp(0, 256) as usize).map(|i| *hi.add(i)).collect();
            v.push(format!("{tag}.hIST.v:{s:?}"));
        }
    }

    // ---- pHYs / oFFs + every EASY_ACCESS derivative ----
    {
        let (mut rx, mut ry, mut ut) = (0u32, 0u32, -99i32);
        v.push(format!(
            "{tag}.pHYs:{}:{rx}:{ry}:{ut}",
            (a.png_get_pHYs)(p, info, &mut rx, &mut ry, &mut ut)
        ));
        let (mut dx, mut dy, mut du) = (0u32, 0u32, -99i32);
        v.push(format!(
            "{tag}.pHYs_dpi:{}:{dx}:{dy}:{du}",
            (a.png_get_pHYs_dpi)(p, info, &mut dx, &mut dy, &mut du)
        ));
        // partial-output forms
        let mut only_x = 0u32;
        v.push(format!(
            "{tag}.pHYs.x_only:{}:{only_x}",
            (a.png_get_pHYs)(p, info, &mut only_x, ptr::null_mut(), ptr::null_mut())
        ));
        let (mut ox, mut oy, mut ou) = (0i32, 0i32, -99i32);
        v.push(format!(
            "{tag}.oFFs:{}:{ox}:{oy}:{ou}",
            (a.png_get_oFFs)(p, info, &mut ox, &mut oy, &mut ou)
        ));
    }
    v.push(format!("{tag}.ppm:{}", (a.png_get_pixels_per_meter)(p, info)));
    v.push(format!(
        "{tag}.xppm:{}",
        (a.png_get_x_pixels_per_meter)(p, info)
    ));
    v.push(format!(
        "{tag}.yppm:{}",
        (a.png_get_y_pixels_per_meter)(p, info)
    ));
    v.push(format!("{tag}.ppi:{}", (a.png_get_pixels_per_inch)(p, info)));
    v.push(format!(
        "{tag}.xppi:{}",
        (a.png_get_x_pixels_per_inch)(p, info)
    ));
    v.push(format!(
        "{tag}.yppi:{}",
        (a.png_get_y_pixels_per_inch)(p, info)
    ));
    v.push(format!(
        "{tag}.par:{}",
        f32s((a.png_get_pixel_aspect_ratio)(p, info))
    ));
    v.push(format!(
        "{tag}.par.fx:{}",
        (a.png_get_pixel_aspect_ratio_fixed)(p, info)
    ));
    v.push(format!(
        "{tag}.xoff.px:{}",
        (a.png_get_x_offset_pixels)(p, info)
    ));
    v.push(format!(
        "{tag}.yoff.px:{}",
        (a.png_get_y_offset_pixels)(p, info)
    ));
    v.push(format!(
        "{tag}.xoff.um:{}",
        (a.png_get_x_offset_microns)(p, info)
    ));
    v.push(format!(
        "{tag}.yoff.um:{}",
        (a.png_get_y_offset_microns)(p, info)
    ));
    v.push(format!(
        "{tag}.xoff.in:{}",
        f32s((a.png_get_x_offset_inches)(p, info))
    ));
    v.push(format!(
        "{tag}.yoff.in:{}",
        f32s((a.png_get_y_offset_inches)(p, info))
    ));
    v.push(format!(
        "{tag}.xoff.in.fx:{}",
        (a.png_get_x_offset_inches_fixed)(p, info)
    ));
    v.push(format!(
        "{tag}.yoff.in.fx:{}",
        (a.png_get_y_offset_inches_fixed)(p, info)
    ));

    // ---- tIME ----
    {
        let mut tm: *mut png_time = ptr::null_mut();
        let got = (a.png_get_tIME)(p, info, &mut tm);
        v.push(format!(
            "{tag}.tIME:{got}:{:?}",
            if got != 0 && !tm.is_null() {
                Some(*tm)
            } else {
                None
            }
        ));
    }

    // ---- sCAL, all three representations ----
    {
        if scal_fixed {
            let (mut u, mut w, mut h) = (-99i32, 0i32, 0i32);
            v.push(format!(
                "{tag}.sCAL.fx:{}:{u}:{w}:{h}",
                (a.png_get_sCAL_fixed)(p, info, &mut u, &mut w, &mut h)
            ));
        }
        let (mut u2, mut wd, mut hd) = (-99i32, 0f64, 0f64);
        let got = (a.png_get_sCAL)(p, info, &mut u2, &mut wd, &mut hd);
        v.push(format!(
            "{tag}.sCAL.fp:{got}:{u2}:{}:{}",
            f64s(wd),
            f64s(hd)
        ));
        let (mut u3, mut ws, mut hs): (c_int, *mut c_char, *mut c_char) =
            (-99, ptr::null_mut(), ptr::null_mut());
        let got = (a.png_get_sCAL_s)(p, info, &mut u3, &mut ws, &mut hs);
        v.push(format!(
            "{tag}.sCAL.s:{got}:{u3}:{}:{}",
            cstr_to_string(ws),
            cstr_to_string(hs)
        ));
    }

    // ---- eXIf ----
    {
        let mut n = 0u32;
        let mut e: *mut png_byte = ptr::null_mut();
        let got = (a.png_get_eXIf_1)(p, info, &mut n, &mut e);
        v.push(format!("{tag}.eXIf:{got}:{n}"));
        if got != 0 && !e.is_null() {
            let s: Vec<u8> = (0..(n as usize).min(4096)).map(|i| *e.add(i)).collect();
            v.push(format!("{tag}.eXIf.d:{s:02x?}"));
        }
    }

    // ---- cICP / cLLI / mDCV, both representations ----
    {
        let mut b = [0u8; 4];
        let got = (a.png_get_cICP)(p, info, &mut b[0], &mut b[1], &mut b[2], &mut b[3]);
        v.push(format!("{tag}.cICP:{got}:{b:?}"));

        let mut cl = [0u32; 2];
        let got = (a.png_get_cLLI_fixed)(p, info, &mut cl[0], &mut cl[1]);
        v.push(format!("{tag}.cLLI.fx:{got}:{cl:?}"));
        let mut cld = [0f64; 2];
        let got = (a.png_get_cLLI)(p, info, &mut cld[0], &mut cld[1]);
        v.push(format!(
            "{tag}.cLLI.fp:{got}:{},{}",
            f64s(cld[0]),
            f64s(cld[1])
        ));

        let mut m = [0i32; 8];
        let mut lum = [0u32; 2];
        let got = (a.png_get_mDCV_fixed)(
            p, info, &mut m[0], &mut m[1], &mut m[2], &mut m[3], &mut m[4], &mut m[5], &mut m[6],
            &mut m[7], &mut lum[0], &mut lum[1],
        );
        v.push(format!("{tag}.mDCV.fx:{got}:{m:?}:{lum:?}"));
        let mut md = [0f64; 10];
        let got = (a.png_get_mDCV)(
            p, info, &mut md[0], &mut md[1], &mut md[2], &mut md[3], &mut md[4], &mut md[5],
            &mut md[6], &mut md[7], &mut md[8], &mut md[9],
        );
        v.push(format!(
            "{tag}.mDCV.fp:{got}:{:?}",
            md.iter().map(|z| f64s(*z)).collect::<Vec<_>>()
        ));
    }

    // ---- iCCP ----
    {
        let mut name: *mut c_char = ptr::null_mut();
        let mut comp = -99i32;
        let mut prof: *mut png_byte = ptr::null_mut();
        let mut plen = 0u32;
        let got = (a.png_get_iCCP)(p, info, &mut name, &mut comp, &mut prof, &mut plen);
        v.push(format!(
            "{tag}.iCCP:{got}:{}:{comp}:{plen}",
            cstr_to_string(name)
        ));
        if got != 0 && !prof.is_null() {
            // pngget.c:730 derives the length from the profile header itself,
            // so every profile built below carries its own big-endian length.
            let n = (plen as usize).min(4096);
            let s: Vec<u8> = (0..n).map(|i| *prof.add(i)).collect();
            v.push(format!("{tag}.iCCP.d:{:02x?}", &s[..s.len().min(48)]));
        }
    }

    // ---- pCAL ----
    {
        let mut purpose: *mut c_char = ptr::null_mut();
        let (mut x0, mut x1) = (0i32, 0i32);
        let (mut ty, mut np) = (-99i32, -99i32);
        let mut units: *mut c_char = ptr::null_mut();
        let mut params: *mut *mut c_char = ptr::null_mut();
        let got = (a.png_get_pCAL)(
            p, info, &mut purpose, &mut x0, &mut x1, &mut ty, &mut np, &mut units, &mut params,
        );
        v.push(format!(
            "{tag}.pCAL:{got}:{}:{x0}:{x1}:{ty}:{np}:{}",
            cstr_to_string(purpose),
            cstr_to_string(units)
        ));
        if got != 0 && !params.is_null() {
            for i in 0..np.clamp(0, 255) as usize {
                v.push(format!(
                    "{tag}.pCAL.p[{i}]:{}",
                    cstr_to_string(*params.add(i))
                ));
            }
        }
    }

    // ---- sPLT ----
    {
        let mut e: *mut png_sPLT_t = ptr::null_mut();
        let n = (a.png_get_sPLT)(p, info, &mut e);
        v.push(format!("{tag}.sPLT:{n}"));
        if n > 0 && !e.is_null() {
            for i in 0..n as usize {
                let s = *e.add(i);
                v.push(format!(
                    "{tag}.sPLT[{i}]:{}:{}:{}",
                    cstr_to_string(s.name),
                    s.depth,
                    s.nentries
                ));
                if !s.entries.is_null() {
                    for j in 0..s.nentries.clamp(0, 4096) as usize {
                        v.push(format!("{tag}.sPLT[{i}][{j}]:{:?}", *s.entries.add(j)));
                    }
                }
            }
        }
    }

    // ---- text ----
    {
        let mut tp: *mut png_text = ptr::null_mut();
        let mut n = -99i32;
        let got = (a.png_get_text)(p, info, &mut tp, &mut n);
        v.push(format!("{tag}.text:{got}:{n}"));
        if got > 0 && !tp.is_null() {
            for i in 0..n.clamp(0, 4096) as usize {
                let t = *tp.add(i);
                if t.key.is_null() {
                    // `png_free_data(PNG_FREE_TEXT, num)` frees the single block
                    // that holds key/lang/lang_key/text and NULLs only `key`
                    // (png.c:494), so the other members dangle: do not read them.
                    v.push(format!("{tag}.text[{i}]:freed:{}", t.compression));
                    continue;
                }
                v.push(format!(
                    "{tag}.text[{i}]:{}:{}:{}:{}:{}:{}:{}",
                    t.compression,
                    cstr_to_string(t.key),
                    cstr_to_string(t.text),
                    t.text_length,
                    t.itxt_length,
                    cstr_to_string(t.lang),
                    cstr_to_string(t.lang_key),
                ));
            }
        }
    }

    // ---- unknown chunks ----
    {
        let mut u: *mut png_unknown_chunk = ptr::null_mut();
        let n = (a.png_get_unknown_chunks)(p, info, &mut u);
        v.push(format!("{tag}.unk:{n}"));
        if n > 0 && !u.is_null() {
            for i in 0..n as usize {
                let c = *u.add(i);
                let name = String::from_utf8_lossy(&c.name[..4]).into_owned();
                let data: Vec<u8> = if c.data.is_null() {
                    Vec::new()
                } else {
                    (0..c.size.min(4096)).map(|j| *c.data.add(j)).collect()
                };
                v.push(format!(
                    "{tag}.unk[{i}]:{name}:{}:{}:{}:{:02x?}",
                    c.size,
                    c.location,
                    c.data.is_null(),
                    &data[..data.len().min(48)]
                ));
            }
        }
    }

    // ---- rows ----
    v.push(format!(
        "{tag}.rows.null:{}",
        (a.png_get_rows)(p, info).is_null()
    ));

    // ---- signature ----
    let sig = (a.png_get_signature)(p, info);
    if sig.is_null() {
        v.push(format!("{tag}.sig:NULL"));
    } else {
        let s: Vec<u8> = (0..8).map(|i| *sig.add(i)).collect();
        v.push(format!("{tag}.sig:{s:02x?}"));
    }

    v
}

/// Just the `png_get_valid` answers for every documented flag.
unsafe fn valid_flags(a: &Api, p: png_structp, info: png_infop, tag: &str) -> Vec<String> {
    ALL_INFO_FLAGS
        .iter()
        .map(|(n, f)| format!("{tag}.{n}:{}", (a.png_get_valid)(p, info, *f)))
        .collect()
}

/// Everything reachable through the `png_struct` getters.
unsafe fn dump_struct(a: &Api, p: png_structp, tag: &str) -> Vec<String> {
    vec![
        format!("{tag}.io_state:{}", (a.png_get_io_state)(p)),
        format!("{tag}.io_chunk:{:#x}", (a.png_get_io_chunk_type)(p)),
        format!("{tag}.uwmax:{}", (a.png_get_user_width_max)(p)),
        format!("{tag}.uhmax:{}", (a.png_get_user_height_max)(p)),
        format!("{tag}.ccmax:{}", (a.png_get_chunk_cache_max)(p)),
        format!("{tag}.cmmax:{}", (a.png_get_chunk_malloc_max)(p)),
        format!("{tag}.cbufsz:{}", (a.png_get_compression_buffer_size)(p)),
        format!("{tag}.pass:{}", (a.png_get_current_pass_number)(p)),
        format!("{tag}.row:{}", (a.png_get_current_row_number)(p)),
        format!("{tag}.r2g:{}", (a.png_get_rgb_to_gray_status)(p)),
    ]
}

// ---------------------------------------------------------------------------
// 1. png_set_IHDR / png_get_IHDR and the shape getters
// ---------------------------------------------------------------------------

#[test]
fn t_ihdr_roundtrip() {
    for kind in [Kind::Write, Kind::Read] {
        for (i, &(bd, ct)) in DEPTH_TYPE.iter().enumerate() {
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                let mut rng = Rng::new(0x5100_0000 + (i as u64) * 37 + il as u64);
                let mut dims: Vec<(u32, u32)> = vec![(1, 1), (1, 999), (1_000_000, 1)];
                for _ in 0..5 {
                    dims.push((rng.range(1, 4000), rng.range(1, 4000)));
                }
                diff(
                    &format!("IHDR:{}:{bd}:{ct}:{il}", kind.tag()),
                    &move |a| unsafe {
                        let mut c = make(a, kind);
                        let mut out = dump_info(a, c.p, c.info, "empty", true);
                        for (w, h) in dims.iter() {
                            (a.png_set_IHDR)(
                                c.p,
                                c.info,
                                *w,
                                *h,
                                bd,
                                ct,
                                il,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            out.extend(dump_info(a, c.p, c.info, &format!("i{w}x{h}"), true));
                        }
                        out.extend(dump_struct(a, c.p, "st"));
                        destroy(a, &mut c);
                        out
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. PLTE / tRNS (both forms) / hIST / bKGD / sBIT
// ---------------------------------------------------------------------------

#[test]
fn t_plte_trns_hist_bkgd_sbit() {
    let mut rng = Rng::new(0xA11E_5EED);
    for (i, &(bd, ct)) in DEPTH_TYPE.iter().enumerate() {
        let maxn: i32 = if ct == PNG_COLOR_TYPE_PALETTE {
            1i32 << bd
        } else {
            256
        };
        for k in 0..3u32 {
            let n: i32 = match k {
                0 => 1,
                1 => maxn,
                _ => rng.range(1, maxn as u32) as i32,
            };
            let pal: Vec<png_color> = (0..n)
                .map(|_| png_color {
                    red: rng.next_u8(),
                    green: rng.next_u8(),
                    blue: rng.next_u8(),
                })
                .collect();
            let hist: Vec<png_uint_16> = (0..n).map(|_| rng.next_u16()).collect();
            let ntr = rng.range(1, n as u32) as i32;
            let trns: Vec<png_byte> = (0..ntr).map(|_| rng.next_u8()).collect();
            // Keep every colour-key sample inside the bit-depth range so that
            // pngset.c:1253 ("tRNS chunk has out-of-range samples for
            // bit_depth") does not fire here; that warning is compared in
            // `t_benign_warning_paths`.
            let smax: u32 = if bd < 16 { (1u32 << bd) - 1 } else { 0xffff };
            let mk16 = |r: &mut Rng| png_color_16 {
                index: r.next_u8(),
                red: (r.next_u32() % (smax + 1)) as png_uint_16,
                green: (r.next_u32() % (smax + 1)) as png_uint_16,
                blue: (r.next_u32() % (smax + 1)) as png_uint_16,
                gray: (r.next_u32() % (smax + 1)) as png_uint_16,
            };
            let key = mk16(&mut rng);
            let bkgd = mk16(&mut rng);
            let sbit = png_color_8 {
                red: rng.range(1, bd as u32) as png_byte,
                green: rng.range(1, bd as u32) as png_byte,
                blue: rng.range(1, bd as u32) as png_byte,
                gray: rng.range(1, bd as u32) as png_byte,
                alpha: rng.range(1, bd as u32) as png_byte,
            };
            for kind in [Kind::Write, Kind::Read] {
                let pal = pal.clone();
                let hist = hist.clone();
                let trns = trns.clone();
                diff(
                    &format!("PLTE:{}:{i}:{k}:{bd}:{ct}", kind.tag()),
                    &move |a| unsafe {
                        let mut c = make(a, kind);
                        (a.png_set_IHDR)(
                            c.p,
                            c.info,
                            64,
                            32,
                            bd,
                            ct,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        let mut out = dump_info(a, c.p, c.info, "0", true);
                        (a.png_set_PLTE)(c.p, c.info, pal.as_ptr(), n);
                        out.extend(dump_info(a, c.p, c.info, "plte", true));
                        // tRNS: palette (alpha array) form
                        (a.png_set_tRNS)(c.p, c.info, trns.as_ptr(), ntr, ptr::null());
                        out.extend(dump_info(a, c.p, c.info, "trnsA", true));
                        // tRNS: colour-key form
                        (a.png_set_tRNS)(c.p, c.info, ptr::null(), 0, &key);
                        out.extend(dump_info(a, c.p, c.info, "trnsK", true));
                        // tRNS: both at once
                        (a.png_set_tRNS)(c.p, c.info, trns.as_ptr(), ntr, &key);
                        out.extend(dump_info(a, c.p, c.info, "trnsB", true));
                        // getter-to-setter aliasing (pngset.c:1193 snapshot)
                        {
                            let mut ta: *mut png_byte = ptr::null_mut();
                            let mut nt = 0i32;
                            let mut tc: *mut png_color_16 = ptr::null_mut();
                            (a.png_get_tRNS)(c.p, c.info, &mut ta, &mut nt, &mut tc);
                            if !ta.is_null() && nt > 0 {
                                (a.png_set_tRNS)(c.p, c.info, ta, nt, ptr::null());
                            }
                            out.extend(dump_info(a, c.p, c.info, "trnsAlias", true));
                        }
                        (a.png_set_hIST)(c.p, c.info, hist.as_ptr());
                        out.extend(dump_info(a, c.p, c.info, "hist", true));
                        (a.png_set_bKGD)(c.p, c.info, &bkgd);
                        (a.png_set_sBIT)(c.p, c.info, &sbit);
                        out.extend(dump_info(a, c.p, c.info, "bkgd_sbit", true));
                        // PLTE getter-to-setter aliasing
                        {
                            let mut pp: *mut png_color = ptr::null_mut();
                            let mut np = 0i32;
                            if (a.png_get_PLTE)(c.p, c.info, &mut pp, &mut np) != 0 && np > 0 {
                                (a.png_set_PLTE)(c.p, c.info, pp, np);
                            }
                            out.extend(dump_info(a, c.p, c.info, "plteAlias", true));
                        }
                        // hIST getter-to-setter aliasing
                        {
                            let mut hp: *mut png_uint_16 = ptr::null_mut();
                            if (a.png_get_hIST)(c.p, c.info, &mut hp) != 0 && !hp.is_null() {
                                (a.png_set_hIST)(c.p, c.info, hp);
                            }
                            out.extend(dump_info(a, c.p, c.info, "histAlias", true));
                        }
                        // NULL arguments are no-ops (pngset.c:29 / 393 / 840)
                        (a.png_set_bKGD)(c.p, c.info, ptr::null());
                        (a.png_set_sBIT)(c.p, c.info, ptr::null());
                        (a.png_set_hIST)(c.p, c.info, ptr::null());
                        out.extend(dump_info(a, c.p, c.info, "nulls", true));
                        destroy(a, &mut c);
                        out
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. gAMA / sRGB / cHRM / cHRM_XYZ, fixed <-> floating point cross-checks
// ---------------------------------------------------------------------------

/// A guaranteed-`png_xy_from_XYZ`-clean XYZ 9-tuple, derived once from the C
/// library so that both libraries receive byte-identical inputs.
fn probe_xyz(xy: [i32; 8]) -> Option<[i32; 9]> {
    let a = &apis().c;
    reset_all();
    set_cur_is_c(true);
    unsafe {
        let mut c = make(a, Kind::Write);
        (a.png_set_cHRM_fixed)(
            c.p, c.info, xy[0], xy[1], xy[2], xy[3], xy[4], xy[5], xy[6], xy[7],
        );
        let mut y = [0i32; 9];
        let got = (a.png_get_cHRM_XYZ_fixed)(
            c.p, c.info, &mut y[0], &mut y[1], &mut y[2], &mut y[3], &mut y[4], &mut y[5],
            &mut y[6], &mut y[7], &mut y[8],
        );
        destroy(a, &mut c);
        let _ = log_take();
        if got != 0 {
            Some(y)
        } else {
            None
        }
    }
}

#[test]
fn t_gama_srgb_chrm() {
    let mut rng = Rng::new(0x6A0A_0001);
    // fixed-point gammas: png_set_gAMA_fixed stores the value verbatim
    // (pngset.c:369), so anything goes.
    let mut gammas: Vec<i32> = vec![0, 1, 45455, 100000, 220000, -1, -2, i32::MAX, i32::MIN];
    for _ in 0..8 {
        gammas.push(rng.next_u32() as i32);
    }
    // floating-point gammas must survive png_fixed (|v| <= 21474.83647, png.c:2727)
    let mut fgammas: Vec<f64> = vec![0.0, 1.0, 0.45455, 2.2, -1.0, 21474.0, -21474.0];
    for _ in 0..8 {
        fgammas.push((rng.next_u32() % 4_000_000) as f64 / 1000.0);
    }
    // realistic-ish chromaticity sets plus wild fixed-point ones
    let mut chrms: Vec<[i32; 8]> = vec![
        [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [i32::MAX, i32::MIN, 0, 1, -1, 2, -2, 3],
    ];
    for _ in 0..8 {
        let mut v = [0i32; 8];
        for x in v.iter_mut() {
            *x = rng.range(1, 100000) as i32;
        }
        chrms.push(v);
    }
    let xyzs: Vec<[i32; 9]> = chrms.iter().filter_map(|c| probe_xyz(*c)).collect();
    assert!(!xyzs.is_empty(), "no valid XYZ triple could be derived");

    for kind in [Kind::Write, Kind::Read] {
        let gammas = gammas.clone();
        let fgammas = fgammas.clone();
        diff(&format!("gAMA:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for g in gammas.iter() {
                let mut c = make(a, kind);
                (a.png_set_gAMA_fixed)(c.p, c.info, *g);
                out.extend(dump_info(a, c.p, c.info, &format!("fx{g}"), true));
                destroy(a, &mut c);
            }
            for g in fgammas.iter() {
                let mut c = make(a, kind);
                (a.png_set_gAMA)(c.p, c.info, *g);
                out.extend(dump_info(a, c.p, c.info, &format!("fp{}", f64s(*g)), true));
                destroy(a, &mut c);
            }
            out
        });

        let chrms2 = chrms.clone();
        diff(&format!("cHRM:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for x in chrms2.iter() {
                let mut c = make(a, kind);
                (a.png_set_cHRM_fixed)(
                    c.p, c.info, x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7],
                );
                out.extend(dump_info(a, c.p, c.info, "fx", true));
                destroy(a, &mut c);
                // now the floating-point setter with the same numbers / 1e5
                let mut c = make(a, kind);
                (a.png_set_cHRM)(
                    c.p,
                    c.info,
                    x[0] as f64 / 100000.0,
                    x[1] as f64 / 100000.0,
                    x[2] as f64 / 100000.0,
                    x[3] as f64 / 100000.0,
                    x[4] as f64 / 100000.0,
                    x[5] as f64 / 100000.0,
                    x[6] as f64 / 100000.0,
                    x[7] as f64 / 100000.0,
                );
                out.extend(dump_info(a, c.p, c.info, "fp", true));
                destroy(a, &mut c);
            }
            out
        });

        let xyzs2 = xyzs.clone();
        diff(&format!("cHRM_XYZ:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for y in xyzs2.iter() {
                let mut c = make(a, kind);
                (a.png_set_cHRM_XYZ_fixed)(
                    c.p, c.info, y[0], y[1], y[2], y[3], y[4], y[5], y[6], y[7], y[8],
                );
                out.extend(dump_info(a, c.p, c.info, "xyzfx", true));
                destroy(a, &mut c);
                let mut c = make(a, kind);
                (a.png_set_cHRM_XYZ)(
                    c.p,
                    c.info,
                    y[0] as f64 / 100000.0,
                    y[1] as f64 / 100000.0,
                    y[2] as f64 / 100000.0,
                    y[3] as f64 / 100000.0,
                    y[4] as f64 / 100000.0,
                    y[5] as f64 / 100000.0,
                    y[6] as f64 / 100000.0,
                    y[7] as f64 / 100000.0,
                    y[8] as f64 / 100000.0,
                );
                out.extend(dump_info(a, c.p, c.info, "xyzfp", true));
                destroy(a, &mut c);
            }
            out
        });

        diff(&format!("sRGB:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for intent in [0i32, 1, 2, 3, 4, 7, -1] {
                let mut c = make(a, kind);
                (a.png_set_sRGB)(c.p, c.info, intent);
                out.extend(dump_info(a, c.p, c.info, &format!("s{intent}"), true));
                destroy(a, &mut c);
                let mut c = make(a, kind);
                (a.png_set_sRGB_gAMA_and_cHRM)(c.p, c.info, intent);
                out.extend(dump_info(a, c.p, c.info, &format!("g{intent}"), true));
                destroy(a, &mut c);
            }
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 4. iCCP and sPLT
// ---------------------------------------------------------------------------

/// Build an ICC-profile-shaped blob: `png_get_iCCP` reports the length taken
/// from the first four bytes of the profile itself (pngget.c:730), so the
/// header has to be consistent or the getter would over-read.
fn make_profile(rng: &mut Rng, len: usize) -> Vec<u8> {
    let len = len.max(4);
    let mut v = vec![0u8; len];
    v[0] = (len >> 24) as u8;
    v[1] = (len >> 16) as u8;
    v[2] = (len >> 8) as u8;
    v[3] = len as u8;
    for b in v[4..].iter_mut() {
        *b = rng.next_u8();
    }
    v
}

#[test]
fn t_iccp_splt() {
    let mut rng = Rng::new(0x1CC0_5EED);
    let names: Vec<CString> = ["ICC", "sRGB IEC61966-2.1", "x", "a b c", &"L".repeat(70)]
        .iter()
        .map(|s| CString::new(*s).unwrap())
        .collect();
    let profiles: Vec<Vec<u8>> = [4usize, 5, 16, 128, 300]
        .iter()
        .map(|n| make_profile(&mut rng, *n))
        .collect();

    for kind in [Kind::Write, Kind::Read] {
        let names = names.clone();
        let profiles = profiles.clone();
        diff(&format!("iCCP:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for (i, nm) in names.iter().enumerate() {
                for (j, pf) in profiles.iter().enumerate() {
                    let mut c = make(a, kind);
                    out.extend(dump_info(a, c.p, c.info, "pre", true));
                    (a.png_set_iCCP)(
                        c.p,
                        c.info,
                        nm.as_ptr(),
                        PNG_COMPRESSION_TYPE_BASE,
                        pf.as_ptr(),
                        pf.len() as png_uint_32,
                    );
                    out.extend(dump_info(a, c.p, c.info, &format!("i{i}p{j}"), true));
                    // a second call replaces the first (pngset.c:931)
                    (a.png_set_iCCP)(
                        c.p,
                        c.info,
                        names[0].as_ptr(),
                        PNG_COMPRESSION_TYPE_BASE,
                        profiles[0].as_ptr(),
                        profiles[0].len() as png_uint_32,
                    );
                    out.extend(dump_info(a, c.p, c.info, "again", true));
                    // NULL name / NULL profile are no-ops (pngset.c:900)
                    (a.png_set_iCCP)(
                        c.p,
                        c.info,
                        ptr::null(),
                        PNG_COMPRESSION_TYPE_BASE,
                        pf.as_ptr(),
                        4,
                    );
                    (a.png_set_iCCP)(
                        c.p,
                        c.info,
                        nm.as_ptr(),
                        PNG_COMPRESSION_TYPE_BASE,
                        ptr::null(),
                        4,
                    );
                    out.extend(dump_info(a, c.p, c.info, "nulls", true));
                    destroy(a, &mut c);
                }
            }
            out
        });

        // sPLT: 8- and 16-bit depths, 1 / 2 / many entries, several chunks.
        for &depth in &[8u8, 16u8] {
            for &counts in &[&[1i32][..], &[2][..], &[17][..], &[1, 3, 9][..]] {
                let counts: Vec<i32> = counts.to_vec();
                let mut seed = Rng::new(0x5817_0000 + depth as u64 * 91 + counts.len() as u64);
                let entries: Vec<Vec<png_sPLT_entry>> = counts
                    .iter()
                    .map(|n| {
                        (0..*n)
                            .map(|_| png_sPLT_entry {
                                red: seed.next_u16(),
                                green: seed.next_u16(),
                                blue: seed.next_u16(),
                                alpha: seed.next_u16(),
                                frequency: seed.next_u16(),
                            })
                            .collect()
                    })
                    .collect();
                diff(
                    &format!("sPLT:{}:{depth}:{}", kind.tag(), counts.len()),
                    &move |a| unsafe {
                        let mut c = make(a, kind);
                        let mut out = dump_info(a, c.p, c.info, "pre", true);
                        // one call per palette, then one call carrying them all
                        let cnames: Vec<CString> = (0..counts.len())
                            .map(|i| CString::new(format!("pal{i}")).unwrap())
                            .collect();
                        for (i, n) in counts.iter().enumerate() {
                            let mut e = entries[i].clone();
                            let s = png_sPLT_t {
                                name: cnames[i].as_ptr() as *mut c_char,
                                depth,
                                entries: e.as_mut_ptr(),
                                nentries: *n,
                            };
                            (a.png_set_sPLT)(c.p, c.info, &s, 1);
                            out.extend(dump_info(a, c.p, c.info, &format!("one{i}"), true));
                        }
                        // now an array of all of them in a single call
                        let mut ents: Vec<Vec<png_sPLT_entry>> = entries.clone();
                        let arr: Vec<png_sPLT_t> = ents
                            .iter_mut()
                            .enumerate()
                            .map(|(i, e)| png_sPLT_t {
                                name: cnames[i].as_ptr() as *mut c_char,
                                depth,
                                entries: e.as_mut_ptr(),
                                nentries: counts[i],
                            })
                            .collect();
                        (a.png_set_sPLT)(c.p, c.info, arr.as_ptr(), arr.len() as c_int);
                        out.extend(dump_info(a, c.p, c.info, "all", true));
                        // nentries <= 0 / NULL entries are no-ops (pngset.c:1292)
                        (a.png_set_sPLT)(c.p, c.info, arr.as_ptr(), 0);
                        (a.png_set_sPLT)(c.p, c.info, ptr::null(), 3);
                        out.extend(dump_info(a, c.p, c.info, "noop", true));
                        destroy(a, &mut c);
                        out
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 5. pHYs / oFFs / sCAL / pCAL and the EASY_ACCESS derivatives
// ---------------------------------------------------------------------------

#[test]
fn t_phys_offs_easy_access() {
    let mut rng = Rng::new(0x9845_0001);
    let mut phys: Vec<(u32, u32, c_int)> = Vec::new();
    for &u in &[
        PNG_RESOLUTION_UNKNOWN,
        PNG_RESOLUTION_METER,
        2, /* not a defined unit; stored verbatim, pngset.c:744 */
    ] {
        phys.push((0, 0, u));
        phys.push((1, 1, u));
        phys.push((2835, 2835, u));
        phys.push((2835, 1417, u));
        phys.push((PNG_UINT_31_MAX, 1, u));
        phys.push((1, PNG_UINT_31_MAX, u));
        phys.push((u32::MAX, u32::MAX, u));
        for _ in 0..3 {
            phys.push((rng.interesting_u32(), rng.interesting_u32(), u));
        }
    }
    let mut offs: Vec<(i32, i32, c_int)> = Vec::new();
    for &u in &[PNG_OFFSET_PIXEL, PNG_OFFSET_MICROMETER, 3] {
        offs.push((0, 0, u));
        offs.push((1, -1, u));
        offs.push((i32::MAX, i32::MIN, u));
        offs.push((1_000_000, -1_000_000, u));
        offs.push((4_000_000, 4_000_000, u)); // overflows png_muldiv -> warning
        for _ in 0..3 {
            offs.push((rng.interesting_u32() as i32, rng.interesting_u32() as i32, u));
        }
    }

    for kind in [Kind::Write, Kind::Read] {
        let phys = phys.clone();
        diff(&format!("pHYs:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for (x, y, u) in phys.iter() {
                let mut c = make(a, kind);
                out.extend(dump_info(a, c.p, c.info, "pre", true));
                (a.png_set_pHYs)(c.p, c.info, *x, *y, *u);
                out.extend(dump_info(a, c.p, c.info, &format!("p{x}_{y}_{u}"), true));
                (a.png_set_invalid)(c.p, c.info, PNG_INFO_pHYs as c_int);
                out.extend(dump_info(a, c.p, c.info, "inv", true));
                destroy(a, &mut c);
            }
            out
        });
        let offs = offs.clone();
        diff(&format!("oFFs:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for (x, y, u) in offs.iter() {
                let mut c = make(a, kind);
                out.extend(dump_info(a, c.p, c.info, "pre", true));
                (a.png_set_oFFs)(c.p, c.info, *x, *y, *u);
                out.extend(dump_info(a, c.p, c.info, &format!("o{x}_{y}_{u}"), true));
                (a.png_set_invalid)(c.p, c.info, PNG_INFO_oFFs as c_int);
                out.extend(dump_info(a, c.p, c.info, "inv", true));
                destroy(a, &mut c);
            }
            out
        });
    }
}

#[test]
fn t_scal() {
    let mut rng = Rng::new(0x5CA1_0001);
    // Every stored value stays well inside png_fixed's range so that
    // `png_get_sCAL_fixed` (pngget.c:1047) cannot overflow.
    let mut fixed: Vec<(i32, i32)> = vec![(1, 1), (100000, 100000), (1, 2147483647), (7, 13)];
    let mut floats: Vec<(f64, f64)> = vec![(1.0, 1.0), (0.5, 2.0), (1e-4, 20000.0)];
    for _ in 0..6 {
        fixed.push((rng.range(1, 2_000_000_000) as i32, rng.range(1, 2_000_000_000) as i32));
        floats.push((
            rng.range(1, 2_000_000) as f64 / 1000.0,
            rng.range(1, 2_000_000) as f64 / 1000.0,
        ));
    }
    let strs: Vec<(CString, CString)> = [
        ("1", "1"),
        ("0.5", "12345.678"),
        ("1e2", "3E-2"),
        ("0", "0"),
        ("00012.500", ".5"),
        ("9999.99999", "1"),
    ]
    .iter()
    .map(|(w, h)| (CString::new(*w).unwrap(), CString::new(*h).unwrap()))
    .collect();

    for kind in [Kind::Write, Kind::Read] {
        for &unit in &[PNG_SCALE_METER, PNG_SCALE_RADIAN] {
            let fixed = fixed.clone();
            let floats = floats.clone();
            let strs = strs.clone();
            diff(
                &format!("sCAL:{}:{unit}", kind.tag()),
                &move |a| unsafe {
                    let mut out = Vec::new();
                    for (w, h) in fixed.iter() {
                        let mut c = make(a, kind);
                        out.extend(dump_info(a, c.p, c.info, "pre", true));
                        (a.png_set_sCAL_fixed)(c.p, c.info, unit, *w, *h);
                        out.extend(dump_info(a, c.p, c.info, &format!("fx{w}_{h}"), true));
                        destroy(a, &mut c);
                    }
                    for (w, h) in floats.iter() {
                        let mut c = make(a, kind);
                        (a.png_set_sCAL)(c.p, c.info, unit, *w, *h);
                        out.extend(dump_info(a, c.p, c.info, "fp", true));
                        destroy(a, &mut c);
                    }
                    for (w, h) in strs.iter() {
                        let mut c = make(a, kind);
                        (a.png_set_sCAL_s)(c.p, c.info, unit, w.as_ptr(), h.as_ptr());
                        out.extend(dump_info(
                            a,
                            c.p,
                            c.info,
                            &format!("s{}", w.to_str().unwrap()),
                            true,
                        ));
                        (a.png_set_invalid)(c.p, c.info, PNG_INFO_sCAL as c_int);
                        out.extend(dump_info(a, c.p, c.info, "inv", true));
                        destroy(a, &mut c);
                    }
                    out
                },
            );
        }
    }
}

#[test]
fn t_pcal() {
    // Valid parameter strings only: pngset.c:531 rejects anything that is not a
    // `png_check_fp_string`.
    let param_pool = ["0", "1", "-1", "2.5", "-3.75", "100000", "1e3", "-2.5E-2"];
    for kind in [Kind::Write, Kind::Read] {
        for ty in [
            PNG_EQUATION_LINEAR,
            PNG_EQUATION_BASE_E,
            PNG_EQUATION_ARBITRARY,
            PNG_EQUATION_HYPERBOLIC,
        ] {
            for nparams in [0usize, 1, 3, 8] {
                for (xi, &(x0, x1)) in [(0i32, 1i32), (-100, 100), (i32::MIN, i32::MAX)]
                    .iter()
                    .enumerate()
                {
                    diff(
                        &format!("pCAL:{}:{ty}:{nparams}:{xi}", kind.tag()),
                        &move |a| unsafe {
                            let purpose = CString::new(format!("purpose {ty}")).unwrap();
                            let units = CString::new("metres").unwrap();
                            let owned: Vec<CString> = (0..nparams)
                                .map(|i| CString::new(param_pool[i % param_pool.len()]).unwrap())
                                .collect();
                            let mut raw: Vec<*mut c_char> =
                                owned.iter().map(|s| s.as_ptr() as *mut c_char).collect();
                            let pp = if nparams == 0 {
                                ptr::null_mut()
                            } else {
                                raw.as_mut_ptr()
                            };
                            let mut c = make(a, kind);
                            let mut out = dump_info(a, c.p, c.info, "pre", true);
                            (a.png_set_pCAL)(
                                c.p,
                                c.info,
                                purpose.as_ptr(),
                                x0,
                                x1,
                                ty,
                                nparams as c_int,
                                units.as_ptr(),
                                pp,
                            );
                            out.extend(dump_info(a, c.p, c.info, "set", true));
                            // NULL purpose / units are no-ops (pngset.c:502)
                            (a.png_set_pCAL)(
                                c.p,
                                c.info,
                                ptr::null(),
                                x0,
                                x1,
                                ty,
                                0,
                                units.as_ptr(),
                                ptr::null_mut(),
                            );
                            (a.png_set_pCAL)(
                                c.p,
                                c.info,
                                purpose.as_ptr(),
                                x0,
                                x1,
                                ty,
                                0,
                                ptr::null(),
                                ptr::null_mut(),
                            );
                            out.extend(dump_info(a, c.p, c.info, "noop", true));
                            (a.png_set_invalid)(c.p, c.info, PNG_INFO_pCAL as c_int);
                            out.extend(dump_info(a, c.p, c.info, "inv", true));
                            destroy(a, &mut c);
                            out
                        },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 6. text / tIME / eXIf
// ---------------------------------------------------------------------------

#[test]
fn t_text() {
    let long_text = "y".repeat(400);
    let long_key = "K".repeat(120);
    // compression: -1 tEXt, 0 zTXt, 1 iTXt uncompressed, 2 iTXt compressed
    // (png.h:590 -- anything outside [-1, 3) is rejected).
    let comps = [
        PNG_TEXT_COMPRESSION_NONE,
        PNG_TEXT_COMPRESSION_zTXt,
        PNG_ITXT_COMPRESSION_NONE,
        PNG_ITXT_COMPRESSION_zTXt,
    ];
    for kind in [Kind::Write, Kind::Read] {
        for (ci, &comp) in comps.iter().enumerate() {
            let long_text = long_text.clone();
            let long_key = long_key.clone();
            diff(
                &format!("text:{}:{comp}:{ci}", kind.tag()),
                &move |a| unsafe {
                    let keys: Vec<CString> = ["Title", "Author", "a b", &long_key]
                        .iter()
                        .map(|s| CString::new(*s).unwrap())
                        .collect();
                    let texts: Vec<CString> = ["", "hello", &long_text, "line1\nline2"]
                        .iter()
                        .map(|s| CString::new(*s).unwrap())
                        .collect();
                    let lang = CString::new("en-GB").unwrap();
                    let lang_empty = CString::new("").unwrap();
                    let lkey = CString::new("Titel").unwrap();

                    let mut c = make(a, kind);
                    let mut out = dump_info(a, c.p, c.info, "pre", true);
                    for (i, k) in keys.iter().enumerate() {
                        for (j, t) in texts.iter().enumerate() {
                            let e = png_text {
                                compression: comp,
                                key: k.as_ptr() as *mut c_char,
                                text: t.as_ptr() as *mut c_char,
                                text_length: 0,
                                itxt_length: 0,
                                lang: if j % 2 == 0 {
                                    lang.as_ptr() as *mut c_char
                                } else {
                                    lang_empty.as_ptr() as *mut c_char
                                },
                                lang_key: lkey.as_ptr() as *mut c_char,
                            };
                            (a.png_set_text)(c.p, c.info, &e, 1);
                            out.extend(dump_info(a, c.p, c.info, &format!("t{i}_{j}"), true));
                        }
                    }
                    // a NULL text pointer: text_length becomes 0 and the
                    // compression is forced to an uncompressed mode
                    // (pngset.c:1069)
                    let e = png_text {
                        compression: comp,
                        key: keys[0].as_ptr() as *mut c_char,
                        text: ptr::null_mut(),
                        text_length: 0,
                        itxt_length: 0,
                        lang: lang.as_ptr() as *mut c_char,
                        lang_key: lkey.as_ptr() as *mut c_char,
                    };
                    (a.png_set_text)(c.p, c.info, &e, 1);
                    out.extend(dump_info(a, c.p, c.info, "nulltext", true));
                    // a NULL key entry is silently skipped (pngset.c:1025)
                    let e2 = png_text {
                        compression: comp,
                        key: ptr::null_mut(),
                        text: texts[1].as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: lang.as_ptr() as *mut c_char,
                        lang_key: lkey.as_ptr() as *mut c_char,
                    };
                    (a.png_set_text)(c.p, c.info, &e2, 1);
                    // num_text <= 0 and a NULL array are no-ops (pngset.c:963)
                    (a.png_set_text)(c.p, c.info, &e, 0);
                    (a.png_set_text)(c.p, c.info, &e, -3);
                    (a.png_set_text)(c.p, c.info, ptr::null(), 2);
                    out.extend(dump_info(a, c.p, c.info, "nullkey", true));
                    // a multi-entry array in one call, forcing the realloc path
                    let many: Vec<png_text> = (0..11)
                        .map(|i| png_text {
                            compression: comp,
                            key: keys[i % keys.len()].as_ptr() as *mut c_char,
                            text: texts[i % texts.len()].as_ptr() as *mut c_char,
                            text_length: 0,
                            itxt_length: 0,
                            lang: lang.as_ptr() as *mut c_char,
                            lang_key: lkey.as_ptr() as *mut c_char,
                        })
                        .collect();
                    (a.png_set_text)(c.p, c.info, many.as_ptr(), many.len() as c_int);
                    out.extend(dump_info(a, c.p, c.info, "many", true));
                    // getter-to-setter aliasing (pngset.c:1006)
                    {
                        let mut tp: *mut png_text = ptr::null_mut();
                        let mut n = 0i32;
                        if (a.png_get_text)(c.p, c.info, &mut tp, &mut n) > 0 && !tp.is_null() {
                            (a.png_set_text)(c.p, c.info, tp, n.min(4));
                        }
                        out.extend(dump_info(a, c.p, c.info, "alias", true));
                    }
                    destroy(a, &mut c);
                    out
                },
            );
        }
    }
}

#[test]
fn t_time() {
    let mut rng = Rng::new(0x7113_0001);
    let mut times: Vec<png_time> = vec![
        png_time { year: 0, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 1970, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
        png_time { year: 65535, month: 6, day: 15, hour: 12, minute: 30, second: 30 },
    ];
    for _ in 0..10 {
        times.push(png_time {
            year: rng.next_u16(),
            month: rng.range(1, 12) as png_byte,
            day: rng.range(1, 31) as png_byte,
            hour: rng.range(0, 23) as png_byte,
            minute: rng.range(0, 59) as png_byte,
            second: rng.range(0, 60) as png_byte,
        });
    }
    for kind in [Kind::Write, Kind::Read] {
        let times = times.clone();
        diff(&format!("tIME:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            let mut out = dump_info(a, c.p, c.info, "pre", true);
            for (i, t) in times.iter().enumerate() {
                (a.png_set_tIME)(c.p, c.info, t);
                out.extend(dump_info(a, c.p, c.info, &format!("t{i}"), true));
            }
            // NULL is a no-op (pngset.c:1161)
            (a.png_set_tIME)(c.p, c.info, ptr::null());
            out.extend(dump_info(a, c.p, c.info, "null", true));
            (a.png_set_invalid)(c.p, c.info, PNG_INFO_tIME as c_int);
            out.extend(dump_info(a, c.p, c.info, "inv", true));
            destroy(a, &mut c);
            out
        });
    }
}

#[test]
fn t_exif() {
    let mut rng = Rng::new(0xE81F_0001);
    // num_exif == 0 would go through png_malloc_warn(0); keep it >= 1.
    let blobs: Vec<Vec<u8>> = [1usize, 2, 8, 64, 513]
        .iter()
        .map(|n| rng.bytes(*n))
        .collect();
    for kind in [Kind::Write, Kind::Read] {
        let blobs = blobs.clone();
        diff(&format!("eXIf:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            let mut out = dump_info(a, c.p, c.info, "pre", true);
            for (i, b) in blobs.iter().enumerate() {
                let mut b = b.clone();
                (a.png_set_eXIf_1)(c.p, c.info, b.len() as png_uint_32, b.as_mut_ptr());
                out.extend(dump_info(a, c.p, c.info, &format!("e{i}"), true));
            }
            // NULL data is a no-op (pngset.c:335)
            (a.png_set_eXIf_1)(c.p, c.info, 4, ptr::null_mut());
            out.extend(dump_info(a, c.p, c.info, "null", true));
            // the deprecated pair: both unconditionally warn
            // (pngset.c:322 / pngget.c:895)
            let mut b = blobs[0].clone();
            (a.png_set_eXIf)(c.p, c.info, b.as_mut_ptr());
            let mut got: *mut png_byte = ptr::null_mut();
            out.push(format!(
                "get_eXIf:{}:{}",
                (a.png_get_eXIf)(c.p, c.info, &mut got),
                got.is_null()
            ));
            out.extend(dump_info(a, c.p, c.info, "deprecated", true));
            (a.png_set_invalid)(c.p, c.info, PNG_INFO_eXIf as c_int);
            out.extend(dump_info(a, c.p, c.info, "inv", true));
            destroy(a, &mut c);
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 7. cICP / cLLI / mDCV
// ---------------------------------------------------------------------------

#[test]
fn t_cicp_clli_mdcv() {
    let mut rng = Rng::new(0xC1C0_0001);
    let mut cicp: Vec<(u8, u8, u8, u8)> = vec![(1, 13, 0, 1), (9, 16, 0, 0), (255, 255, 0, 255)];
    for _ in 0..6 {
        // matrix_coefficients != 0 is rejected with a plain png_warning
        // (pngset.c:152), which is safe on both struct kinds, so both are used.
        cicp.push((
            rng.next_u8(),
            rng.next_u8(),
            if rng.bool() { 0 } else { rng.next_u8() },
            rng.next_u8(),
        ));
    }
    // cLLI: <= 0x7FFFFFFF (pngset.c:174)
    let mut clli: Vec<(u32, u32)> = vec![(0, 0), (1, 1), (10_000_000, 4_000_000), (0x7fffffff, 0)];
    for _ in 0..6 {
        clli.push((rng.range(0, 0x7fff_ffff), rng.range(0, 0x7fff_ffff)));
    }
    // mDCV chromaticities: v/2 must land in 0..=65535 (pngset.c:215)
    let mut mdcv: Vec<([i32; 8], u32, u32)> = vec![
        ([31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000], 10_000_000, 500),
        ([0, 0, 0, 0, 0, 0, 0, 0], 0, 0),
        ([131070, 131070, 131070, 131070, 131070, 131070, 131070, 131070], 0x7fffffff, 0x7fffffff),
    ];
    for _ in 0..6 {
        let mut v = [0i32; 8];
        for x in v.iter_mut() {
            *x = rng.range(0, 131_070) as i32;
        }
        mdcv.push((v, rng.range(0, 0x7fff_ffff), rng.range(0, 0x7fff_ffff)));
    }

    for kind in [Kind::Write, Kind::Read] {
        let cicp = cicp.clone();
        let clli = clli.clone();
        let mdcv = mdcv.clone();
        diff(&format!("cICP:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for (i, (cp, tf, mc, vf)) in cicp.iter().enumerate() {
                let mut c = make(a, kind);
                out.extend(dump_info(a, c.p, c.info, "pre", true));
                (a.png_set_cICP)(c.p, c.info, *cp, *tf, *mc, *vf);
                out.extend(dump_info(a, c.p, c.info, &format!("c{i}"), true));
                (a.png_set_invalid)(c.p, c.info, PNG_INFO_cICP as c_int);
                out.extend(dump_info(a, c.p, c.info, "inv", true));
                destroy(a, &mut c);
            }
            out
        });
        diff(&format!("cLLI:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for (i, (cll, fall)) in clli.iter().enumerate() {
                let mut c = make(a, kind);
                (a.png_set_cLLI_fixed)(c.p, c.info, *cll, *fall);
                out.extend(dump_info(a, c.p, c.info, &format!("fx{i}"), true));
                destroy(a, &mut c);
                let mut c = make(a, kind);
                (a.png_set_cLLI)(
                    c.p,
                    c.info,
                    *cll as f64 / 10000.0,
                    *fall as f64 / 10000.0,
                );
                out.extend(dump_info(a, c.p, c.info, &format!("fp{i}"), true));
                destroy(a, &mut c);
            }
            out
        });
        diff(&format!("mDCV:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for (i, (v, maxdl, mindl)) in mdcv.iter().enumerate() {
                let mut c = make(a, kind);
                (a.png_set_mDCV_fixed)(
                    c.p, c.info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], *maxdl, *mindl,
                );
                out.extend(dump_info(a, c.p, c.info, &format!("fx{i}"), true));
                destroy(a, &mut c);
                let mut c = make(a, kind);
                (a.png_set_mDCV)(
                    c.p,
                    c.info,
                    v[0] as f64 / 100000.0,
                    v[1] as f64 / 100000.0,
                    v[2] as f64 / 100000.0,
                    v[3] as f64 / 100000.0,
                    v[4] as f64 / 100000.0,
                    v[5] as f64 / 100000.0,
                    v[6] as f64 / 100000.0,
                    v[7] as f64 / 100000.0,
                    *maxdl as f64 / 10000.0,
                    *mindl as f64 / 10000.0,
                );
                out.extend(dump_info(a, c.p, c.info, &format!("fp{i}"), true));
                destroy(a, &mut c);
            }
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 8. unknown chunks
// ---------------------------------------------------------------------------

#[test]
fn t_unknown_chunks() {
    let mut rng = Rng::new(0xBEEF_0001);
    let names: [[u8; 5]; 6] = [
        *b"pRVt\0",
        *b"ABCD\0",
        *b"zzzz\0",
        *b"aA0_\0",
        *b"\x01\x02\x03\x04\0",
        *b"tEsT\0",
    ];
    // `check_location` (pngset.c:1387) masks with 0x0b then keeps the top bit,
    // and rejects 0 outright, so only these values are on the valid path.
    let locs = [LOC_IHDR, LOC_PLTE, LOC_AFTER_IDAT, 0x03, 0x09, 0x0b, 0x0f];
    let datas: Vec<Vec<u8>> = [0usize, 1, 7, 200]
        .iter()
        .map(|n| rng.bytes(*n))
        .collect();

    for kind in [Kind::Write, Kind::Read] {
        for &loc in locs.iter() {
            let datas = datas.clone();
            diff(
                &format!("unk:{}:{loc}", kind.tag()),
                &move |a| unsafe {
                    let mut c = make(a, kind);
                    let mut out = dump_info(a, c.p, c.info, "pre", true);
                    // one chunk at a time
                    let mut owned: Vec<Vec<u8>> = datas.clone();
                    for (i, d) in owned.iter_mut().enumerate() {
                        let u = png_unknown_chunk {
                            name: names[i % names.len()],
                            data: if d.is_empty() {
                                ptr::null_mut()
                            } else {
                                d.as_mut_ptr()
                            },
                            size: d.len(),
                            location: loc as png_byte,
                        };
                        (a.png_set_unknown_chunks)(c.p, c.info, &u, 1);
                        out.extend(dump_info(a, c.p, c.info, &format!("u{i}"), true));
                    }
                    // several at once
                    let mut owned2: Vec<Vec<u8>> = datas.clone();
                    let arr: Vec<png_unknown_chunk> = owned2
                        .iter_mut()
                        .enumerate()
                        .map(|(i, d)| png_unknown_chunk {
                            name: names[(i + 2) % names.len()],
                            data: if d.is_empty() {
                                ptr::null_mut()
                            } else {
                                d.as_mut_ptr()
                            },
                            size: d.len(),
                            location: loc as png_byte,
                        })
                        .collect();
                    (a.png_set_unknown_chunks)(c.p, c.info, arr.as_ptr(), arr.len() as c_int);
                    out.extend(dump_info(a, c.p, c.info, "arr", true));
                    // num_unknowns <= 0 / NULL are no-ops (pngset.c:1428)
                    (a.png_set_unknown_chunks)(c.p, c.info, arr.as_ptr(), 0);
                    (a.png_set_unknown_chunks)(c.p, c.info, ptr::null(), 4);
                    out.extend(dump_info(a, c.p, c.info, "noop", true));
                    // relocate the stored chunks
                    let mut probe_u: *mut png_unknown_chunk = ptr::null_mut();
                    let n = (a.png_get_unknown_chunks)(c.p, c.info, &mut probe_u);
                    for i in 0..n {
                        for &nl in &[LOC_IHDR, LOC_PLTE, LOC_AFTER_IDAT, 0x0b] {
                            (a.png_set_unknown_chunk_location)(c.p, c.info, i, nl);
                        }
                    }
                    out.extend(dump_info(a, c.p, c.info, "reloc", true));
                    // out-of-range chunk indices are silently ignored
                    // (pngset.c:1535)
                    (a.png_set_unknown_chunk_location)(c.p, c.info, -1, LOC_IHDR);
                    (a.png_set_unknown_chunk_location)(c.p, c.info, n + 5, LOC_IHDR);
                    out.extend(dump_info(a, c.p, c.info, "oob", true));
                    destroy(a, &mut c);
                    out
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 9. png_get_valid for every flag, before and after every setter, plus
//    png_set_invalid with single flags / 0 / all-ones
// ---------------------------------------------------------------------------

/// The full set of "apply one chunk" actions, used by several tests below.
/// Every one of them is on the strictly-valid path for both struct kinds.
unsafe fn apply_setter(a: &Api, c: &Ctx, which: usize) -> png_uint_32 {
    let p = c.p;
    let info = c.info;
    match which {
        0 => {
            (a.png_set_gAMA_fixed)(p, info, 45455);
            PNG_INFO_gAMA
        }
        1 => {
            let sb = png_color_8 { red: 5, green: 6, blue: 7, gray: 4, alpha: 8 };
            (a.png_set_sBIT)(p, info, &sb);
            PNG_INFO_sBIT
        }
        2 => {
            (a.png_set_cHRM_fixed)(p, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
            PNG_INFO_cHRM
        }
        3 => {
            let pal = [png_color { red: 1, green: 2, blue: 3 }; 4];
            (a.png_set_PLTE)(p, info, pal.as_ptr(), 4);
            PNG_INFO_PLTE
        }
        4 => {
            let ta = [0u8, 1, 2, 3];
            (a.png_set_tRNS)(p, info, ta.as_ptr(), 4, ptr::null());
            PNG_INFO_tRNS
        }
        5 => {
            let bg = png_color_16 { index: 1, red: 2, green: 3, blue: 4, gray: 5 };
            (a.png_set_bKGD)(p, info, &bg);
            PNG_INFO_bKGD
        }
        6 => {
            // hIST needs a palette first (pngset.c:396)
            let pal = [png_color { red: 9, green: 8, blue: 7 }; 4];
            (a.png_set_PLTE)(p, info, pal.as_ptr(), 4);
            let h = [11u16, 22, 33, 44];
            (a.png_set_hIST)(p, info, h.as_ptr());
            PNG_INFO_hIST
        }
        7 => {
            (a.png_set_pHYs)(p, info, 2835, 2835, PNG_RESOLUTION_METER);
            PNG_INFO_pHYs
        }
        8 => {
            (a.png_set_oFFs)(p, info, -7, 9, PNG_OFFSET_MICROMETER);
            PNG_INFO_oFFs
        }
        9 => {
            let t = png_time { year: 2024, month: 2, day: 29, hour: 1, minute: 2, second: 3 };
            (a.png_set_tIME)(p, info, &t);
            PNG_INFO_tIME
        }
        10 => {
            let purpose = CString::new("purpose").unwrap();
            let units = CString::new("units").unwrap();
            let p0 = CString::new("1.5").unwrap();
            let mut raw = [p0.as_ptr() as *mut c_char];
            (a.png_set_pCAL)(
                p,
                info,
                purpose.as_ptr(),
                0,
                100,
                PNG_EQUATION_LINEAR,
                1,
                units.as_ptr(),
                raw.as_mut_ptr(),
            );
            PNG_INFO_pCAL
        }
        11 => {
            (a.png_set_sRGB)(p, info, PNG_sRGB_INTENT_PERCEPTUAL);
            PNG_INFO_sRGB
        }
        12 => {
            let nm = CString::new("icc").unwrap();
            let prof = [0u8, 0, 0, 8, 1, 2, 3, 4];
            (a.png_set_iCCP)(p, info, nm.as_ptr(), PNG_COMPRESSION_TYPE_BASE, prof.as_ptr(), 8);
            PNG_INFO_iCCP
        }
        13 => {
            let nm = CString::new("spal").unwrap();
            let mut ent = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 2];
            let s = png_sPLT_t {
                name: nm.as_ptr() as *mut c_char,
                depth: 8,
                entries: ent.as_mut_ptr(),
                nentries: 2,
            };
            (a.png_set_sPLT)(p, info, &s, 1);
            PNG_INFO_sPLT
        }
        14 => {
            (a.png_set_sCAL_fixed)(p, info, PNG_SCALE_METER, 100000, 200000);
            PNG_INFO_sCAL
        }
        15 => {
            let mut e = [1u8, 2, 3, 4, 5];
            (a.png_set_eXIf_1)(p, info, 5, e.as_mut_ptr());
            PNG_INFO_eXIf
        }
        16 => {
            (a.png_set_cICP)(p, info, 9, 16, 0, 1);
            PNG_INFO_cICP
        }
        17 => {
            (a.png_set_cLLI_fixed)(p, info, 10_000_000, 1_000_000);
            PNG_INFO_cLLI
        }
        18 => {
            (a.png_set_mDCV_fixed)(
                p, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10_000_000, 500,
            );
            PNG_INFO_mDCV
        }
        19 => {
            let t = CString::new("Title").unwrap();
            let v = CString::new("value").unwrap();
            let e = png_text {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: t.as_ptr() as *mut c_char,
                text: v.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: ptr::null_mut(),
                lang_key: ptr::null_mut(),
            };
            (a.png_set_text)(p, info, &e, 1);
            0
        }
        20 => {
            let mut d = [7u8, 7, 7];
            let u = png_unknown_chunk {
                name: *b"pRVt\0",
                data: d.as_mut_ptr(),
                size: 3,
                location: LOC_IHDR as png_byte,
            };
            (a.png_set_unknown_chunks)(p, info, &u, 1);
            0
        }
        _ => 0,
    }
}

const N_SETTERS: usize = 21;

#[test]
fn t_valid_and_invalid() {
    for kind in [Kind::Write, Kind::Read] {
        for which in 0..N_SETTERS {
            diff(
                &format!("valid:{}:{which}", kind.tag()),
                &move |a| unsafe {
                    let mut c = make(a, kind);
                    (a.png_set_IHDR)(
                        c.p,
                        c.info,
                        16,
                        16,
                        8,
                        PNG_COLOR_TYPE_PALETTE,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    let mut out = valid_flags(a, c.p, c.info, "before");
                    let flag = apply_setter(a, &c, which);
                    out.push(format!("flag:{flag:#x}"));
                    out.extend(valid_flags(a, c.p, c.info, "after"));
                    // clear just this flag
                    (a.png_set_invalid)(c.p, c.info, flag as c_int);
                    out.extend(valid_flags(a, c.p, c.info, "inv1"));
                    // re-apply, then clear with a 0 mask (a no-op, pngset.c:1859)
                    let flag2 = apply_setter(a, &c, which);
                    out.push(format!("flag2:{flag2:#x}"));
                    (a.png_set_invalid)(c.p, c.info, 0);
                    out.extend(valid_flags(a, c.p, c.info, "inv0"));
                    // clear everything
                    (a.png_set_invalid)(c.p, c.info, -1);
                    out.extend(valid_flags(a, c.p, c.info, "invAll"));
                    out.extend(dump_info(a, c.p, c.info, "post", true));
                    destroy(a, &mut c);
                    out
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 10. the EASY_ACCESS getters with the underlying chunk absent
// ---------------------------------------------------------------------------

#[test]
fn t_easy_access_absent() {
    for kind in [Kind::Write, Kind::Read] {
        diff(&format!("easy_absent:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            let mut out = Vec::new();
            // completely empty info
            out.extend(dump_info(a, c.p, c.info, "empty", true));
            out.extend(dump_struct(a, c.p, "empty"));
            // IHDR only
            (a.png_set_IHDR)(
                c.p,
                c.info,
                7,
                5,
                8,
                PNG_COLOR_TYPE_RGB_ALPHA,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            out.extend(dump_info(a, c.p, c.info, "ihdr", true));
            // pHYs with the "unknown" unit: the *_per_meter/_inch getters must
            // all report 0 while png_get_pHYs still reports the raw values
            // (pngget.c:134)
            (a.png_set_pHYs)(c.p, c.info, 100, 200, PNG_RESOLUTION_UNKNOWN);
            out.extend(dump_info(a, c.p, c.info, "phys_unk", true));
            (a.png_set_pHYs)(c.p, c.info, 100, 200, PNG_RESOLUTION_METER);
            out.extend(dump_info(a, c.p, c.info, "phys_m", true));
            // oFFs in pixels then in microns
            (a.png_set_oFFs)(c.p, c.info, 11, -22, PNG_OFFSET_PIXEL);
            out.extend(dump_info(a, c.p, c.info, "offs_px", true));
            (a.png_set_oFFs)(c.p, c.info, 11, -22, PNG_OFFSET_MICROMETER);
            out.extend(dump_info(a, c.p, c.info, "offs_um", true));
            // and once more with the flags forcibly cleared
            (a.png_set_invalid)(c.p, c.info, (PNG_INFO_pHYs | PNG_INFO_oFFs) as c_int);
            out.extend(dump_info(a, c.p, c.info, "cleared", true));
            destroy(a, &mut c);
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 11. struct-level state and pointer getters
// ---------------------------------------------------------------------------

unsafe extern "C" fn user_chunk_cb(_p: png_structp, _c: *mut c_void) -> c_int {
    0
}
unsafe extern "C" fn utrans_cb(_p: png_structp, _i: *mut c_void, _r: png_bytep) {}
unsafe extern "C" fn prog_info_cb(_p: png_structp, _i: png_infop) {}
unsafe extern "C" fn prog_row_cb(_p: png_structp, _r: png_bytep, _n: png_uint_32, _pa: c_int) {}
unsafe extern "C" fn prog_end_cb(_p: png_structp, _i: png_infop) {}

/// Distinct, stable addresses to hand to the libraries as opaque user pointers.
static TOKENS: [u8; 8] = [0; 8];

fn tok(i: usize) -> *mut c_void {
    unsafe { (TOKENS.as_ptr() as *mut u8).add(i) as *mut c_void }
}

#[test]
fn t_state_and_pointer_getters() {
    for kind in [Kind::Write, Kind::Read] {
        diff(&format!("state:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            let mut out = Vec::new();
            // defaults, before anything is installed
            out.extend(dump_struct(a, c.p, "d"));
            out.push(format!(
                "d.io_ptr:{}",
                pdesc((a.png_get_io_ptr)(c.p), ptr::null())
            ));
            out.push(format!(
                "d.err_ptr:{}",
                pdesc((a.png_get_error_ptr)(c.p), ptr::null())
            ));
            out.push(format!(
                "d.mem_ptr:{}",
                pdesc((a.png_get_mem_ptr)(c.p), ptr::null())
            ));
            out.push(format!(
                "d.prog_ptr:{}",
                pdesc((a.png_get_progressive_ptr)(c.p), ptr::null())
            ));
            out.push(format!(
                "d.uchunk_ptr:{}",
                pdesc((a.png_get_user_chunk_ptr)(c.p), ptr::null())
            ));
            out.push(format!(
                "d.utrans_ptr:{}",
                pdesc((a.png_get_user_transform_ptr)(c.p), ptr::null())
            ));

            // error handling
            (a.png_set_error_fn)(c.p, tok(0), Some(error_cb), Some(warn_cb));
            out.push(format!(
                "err_ptr:{}",
                pdesc((a.png_get_error_ptr)(c.p), tok(0))
            ));
            // memory handling: NULL callbacks keep the default allocator
            // (pngmem.c:91/240) so this is safe on a default-created struct
            (a.png_set_mem_fn)(c.p, tok(1), None, None);
            out.push(format!(
                "mem_ptr:{}",
                pdesc((a.png_get_mem_ptr)(c.p), tok(1))
            ));
            // i/o
            match kind {
                Kind::Write => {
                    (a.png_set_write_fn)(c.p, tok(2), Some(write_cb), Some(flush_cb));
                    out.push(format!("io_ptr:{}", pdesc((a.png_get_io_ptr)(c.p), tok(2))));
                    (a.png_set_write_status_fn)(c.p, Some(write_status_cb));
                    (a.png_set_flush)(c.p, 3);
                    (a.png_set_flush)(c.p, 0);
                    (a.png_set_flush)(c.p, -5);
                }
                Kind::Read => {
                    (a.png_set_read_fn)(c.p, tok(3), Some(read_cb));
                    out.push(format!("io_ptr:{}", pdesc((a.png_get_io_ptr)(c.p), tok(3))));
                    (a.png_set_read_status_fn)(c.p, Some(read_status_cb));
                    (a.png_set_read_user_chunk_fn)(c.p, tok(4), Some(user_chunk_cb));
                    out.push(format!(
                        "uchunk_ptr:{}",
                        pdesc((a.png_get_user_chunk_ptr)(c.p), tok(4))
                    ));
                    // png_set_progressive_read_fn routes through png_set_read_fn
                    // (pngpread.c:934), so io_ptr and progressive_ptr coincide
                    (a.png_set_progressive_read_fn)(
                        c.p,
                        tok(5),
                        Some(prog_info_cb),
                        Some(prog_row_cb),
                        Some(prog_end_cb),
                    );
                    out.push(format!(
                        "prog_ptr:{}",
                        pdesc((a.png_get_progressive_ptr)(c.p), tok(5))
                    ));
                    out.push(format!(
                        "io_ptr2:{}",
                        pdesc((a.png_get_io_ptr)(c.p), tok(5))
                    ));
                }
            }
            // user transform info
            (a.png_set_user_transform_info)(c.p, tok(6), 8, 3);
            out.push(format!(
                "utrans_ptr:{}",
                pdesc((a.png_get_user_transform_ptr)(c.p), tok(6))
            ));
            match kind {
                Kind::Read => (a.png_set_read_user_transform_fn)(c.p, Some(utrans_cb)),
                Kind::Write => (a.png_set_write_user_transform_fn)(c.p, Some(utrans_cb)),
            }
            out.extend(dump_struct(a, c.p, "afterptrs"));

            // limits
            for (w, h) in [(0u32, 0u32), (1, 1), (1000, 2000), (PNG_UINT_31_MAX, PNG_UINT_31_MAX), (u32::MAX, u32::MAX)] {
                (a.png_set_user_limits)(c.p, w, h);
                out.push(format!(
                    "lim:{}:{}",
                    (a.png_get_user_width_max)(c.p),
                    (a.png_get_user_height_max)(c.p)
                ));
            }
            for v in [0u32, 1, 1000, PNG_UINT_31_MAX, u32::MAX] {
                (a.png_set_chunk_cache_max)(c.p, v);
                out.push(format!("ccmax:{}", (a.png_get_chunk_cache_max)(c.p)));
            }
            // 0 means "unlimited" and is stored as PNG_SIZE_MAX (pngset.c:1909)
            for v in [1usize, 8_000_000, 0, usize::MAX] {
                (a.png_set_chunk_malloc_max)(c.p, v);
                out.push(format!("cmmax:{}", (a.png_get_chunk_malloc_max)(c.p)));
            }
            // compression buffer: >= 6 on a write struct (pngset.c:1838)
            for v in [6usize, 7, 1024, 8192, 1 << 20, PNG_UINT_31_MAX as usize] {
                (a.png_set_compression_buffer_size)(c.p, v);
                out.push(format!(
                    "cbuf:{}:{}",
                    v,
                    (a.png_get_compression_buffer_size)(c.p)
                ));
            }
            out.extend(dump_struct(a, c.p, "final"));
            // restore the default limits so the struct destroys cleanly
            (a.png_set_user_limits)(c.p, 1_000_000, 1_000_000);
            destroy(a, &mut c);
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 12. png_free_data / png_data_freer
// ---------------------------------------------------------------------------

/// Populate one info struct with every chunk that owns heap memory.
/// `png_set_rows` is deliberately NOT used here: `png_data_freer` below sets
/// `free_me |= PNG_FREE_ROWS`, and `png_free_data` would then try to `png_free`
/// caller-owned row buffers (png.c:664).
unsafe fn populate(a: &Api, c: &Ctx) {
    let p = c.p;
    let info = c.info;
    (a.png_set_IHDR)(
        p,
        info,
        16,
        8,
        8,
        PNG_COLOR_TYPE_PALETTE,
        PNG_INTERLACE_NONE,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );
    let pal: Vec<png_color> = (0..8u8)
        .map(|i| png_color { red: i, green: i * 2, blue: i * 3 })
        .collect();
    (a.png_set_PLTE)(p, info, pal.as_ptr(), 8);
    let ta = [0u8, 1, 2, 3, 4, 5, 6, 7];
    (a.png_set_tRNS)(p, info, ta.as_ptr(), 8, ptr::null());
    let hist = [1u16, 2, 3, 4, 5, 6, 7, 8];
    (a.png_set_hIST)(p, info, hist.as_ptr());
    let nm = CString::new("icc-name").unwrap();
    let prof = [0u8, 0, 0, 12, 1, 2, 3, 4, 5, 6, 7, 8];
    (a.png_set_iCCP)(p, info, nm.as_ptr(), PNG_COMPRESSION_TYPE_BASE, prof.as_ptr(), 12);
    // three sPLT chunks so that a per-index free can be observed
    for i in 0..3 {
        let sn = CString::new(format!("spal{i}")).unwrap();
        let mut ent = [png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 3];
        let s = png_sPLT_t {
            name: sn.as_ptr() as *mut c_char,
            depth: 8,
            entries: ent.as_mut_ptr(),
            nentries: 3,
        };
        (a.png_set_sPLT)(p, info, &s, 1);
    }
    let purpose = CString::new("purpose").unwrap();
    let units = CString::new("units").unwrap();
    let pa = CString::new("1").unwrap();
    let pb = CString::new("-2.5").unwrap();
    let mut raw = [pa.as_ptr() as *mut c_char, pb.as_ptr() as *mut c_char];
    (a.png_set_pCAL)(
        p,
        info,
        purpose.as_ptr(),
        -10,
        10,
        PNG_EQUATION_ARBITRARY,
        2,
        units.as_ptr(),
        raw.as_mut_ptr(),
    );
    (a.png_set_sCAL_fixed)(p, info, PNG_SCALE_METER, 150000, 250000);
    let mut ex = [9u8, 8, 7, 6, 5];
    (a.png_set_eXIf_1)(p, info, 5, ex.as_mut_ptr());
    // three text chunks
    let keys: Vec<CString> = (0..3)
        .map(|i| CString::new(format!("Key{i}")).unwrap())
        .collect();
    let val = CString::new("some text").unwrap();
    for k in keys.iter() {
        let e = png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: k.as_ptr() as *mut c_char,
            text: val.as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: ptr::null_mut(),
            lang_key: ptr::null_mut(),
        };
        (a.png_set_text)(p, info, &e, 1);
    }
    // three unknown chunks
    for i in 0..3u8 {
        let mut d = [i, i + 1, i + 2, i + 3];
        let u = png_unknown_chunk {
            name: [b'u', b'n', b'k', b'0' + i, 0],
            data: d.as_mut_ptr(),
            size: 4,
            location: LOC_AFTER_IDAT as png_byte,
        };
        (a.png_set_unknown_chunks)(p, info, &u, 1);
    }
    (a.png_set_gAMA_fixed)(p, info, 45455);
    (a.png_set_cHRM_fixed)(p, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
    let bg = png_color_16 { index: 3, red: 4, green: 5, blue: 6, gray: 7 };
    (a.png_set_bKGD)(p, info, &bg);
    let sb = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
    (a.png_set_sBIT)(p, info, &sb);
    (a.png_set_pHYs)(p, info, 2835, 2835, PNG_RESOLUTION_METER);
    (a.png_set_oFFs)(p, info, 1, 2, PNG_OFFSET_PIXEL);
    let t = png_time { year: 2000, month: 1, day: 2, hour: 3, minute: 4, second: 5 };
    (a.png_set_tIME)(p, info, &t);
    (a.png_set_sRGB)(p, info, PNG_sRGB_INTENT_RELATIVE);
    (a.png_set_cICP)(p, info, 1, 13, 0, 1);
    (a.png_set_cLLI_fixed)(p, info, 1_000_000, 100_000);
    (a.png_set_mDCV_fixed)(
        p, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 1_000_000, 50,
    );
}

const FREE_MASKS: [(&str, png_uint_32); 12] = [
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

#[test]
fn t_free_data() {
    for kind in [Kind::Write, Kind::Read] {
        for (name, mask) in FREE_MASKS {
            diff(
                &format!("free_data:{}:{name}", kind.tag()),
                &move |a| unsafe {
                    let mut out = Vec::new();
                    // num == -1: free everything covered by the mask
                    let mut c = make(a, kind);
                    populate(a, &c);
                    out.extend(dump_info(a, c.p, c.info, "full", true));
                    (a.png_free_data)(c.p, c.info, mask, -1);
                    out.extend(dump_info(a, c.p, c.info, "freed_all", true));
                    // a second call must be idempotent
                    (a.png_free_data)(c.p, c.info, mask, -1);
                    out.extend(dump_info(a, c.p, c.info, "freed_twice", true));
                    destroy(a, &mut c);

                    // num == n: only the multi-item masks honour an index
                    // (PNG_FREE_MUL == TEXT|SPLT|UNKN, png.h:1853)
                    for idx in [0i32, 1, 2] {
                        let mut c = make(a, kind);
                        populate(a, &c);
                        (a.png_free_data)(c.p, c.info, mask, idx);
                        out.extend(dump_info(a, c.p, c.info, &format!("idx{idx}"), true));
                        destroy(a, &mut c);
                    }
                    out
                },
            );
        }

        // png_data_freer with each freer value and a selection of masks
        diff(&format!("data_freer:{}", kind.tag()), &move |a| unsafe {
            let mut out = Vec::new();
            for (name, mask) in FREE_MASKS {
                for freer in [PNG_USER_WILL_FREE_DATA, PNG_DESTROY_WILL_FREE_DATA] {
                    let mut c = make(a, kind);
                    populate(a, &c);
                    (a.png_data_freer)(c.p, c.info, freer, mask);
                    out.extend(dump_info(
                        a,
                        c.p,
                        c.info,
                        &format!("{name}:{freer}:set"),
                        true,
                    ));
                    // after handing ownership away, png_free_data must be a
                    // no-op for that mask (png.c:494 tests free_me)
                    (a.png_free_data)(c.p, c.info, PNG_FREE_ALL, -1);
                    out.extend(dump_info(
                        a,
                        c.p,
                        c.info,
                        &format!("{name}:{freer}:freed"),
                        true,
                    ));
                    // hand everything back to libpng so the struct destroys
                    // cleanly (all of the memory is libpng-owned)
                    (a.png_data_freer)(c.p, c.info, PNG_DESTROY_WILL_FREE_DATA, PNG_FREE_ALL);
                    destroy(a, &mut c);
                }
            }
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 13. info-struct lifecycle and the memory API
// ---------------------------------------------------------------------------

thread_local! {
    /// Outstanding allocations made through the test's own allocator.
    static ALLOC_LIVE: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
}

/// 16-byte header holding the payload size so that `dealloc` can rebuild the
/// exact `Layout`.
const AHDR: usize = 16;

unsafe extern "C" fn my_malloc(_p: png_structp, size: usize) -> *mut c_void {
    let n = if size == 0 { 1 } else { size };
    let layout = match std::alloc::Layout::from_size_align(n + AHDR, AHDR) {
        Ok(l) => l,
        Err(_) => return ptr::null_mut(),
    };
    let base = std::alloc::alloc(layout);
    if base.is_null() {
        return ptr::null_mut();
    }
    (base as *mut usize).write(n);
    ALLOC_LIVE.with(|c| c.set(c.get() + 1));
    base.add(AHDR) as *mut c_void
}

unsafe extern "C" fn my_free(_p: png_structp, p: *mut c_void) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(AHDR);
    let n = (base as *mut usize).read();
    let layout = std::alloc::Layout::from_size_align(n + AHDR, AHDR).unwrap();
    std::alloc::dealloc(base, layout);
    ALLOC_LIVE.with(|c| c.set(c.get() - 1));
}

#[test]
fn t_info_struct_lifecycle() {
    for kind in [Kind::Write, Kind::Read] {
        diff(&format!("info_life:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            let mut out = Vec::new();
            populate(a, &c);
            out.extend(dump_info(a, c.p, c.info, "full", true));
            // destroy and re-create the info struct through the public API
            (a.png_destroy_info_struct)(c.p, &mut c.info);
            out.push(format!("destroyed:{}", c.info.is_null()));
            c.info = (a.png_create_info_struct)(c.p);
            out.push(format!("recreated:{}", c.info.is_null()));
            out.extend(dump_info(a, c.p, c.info, "fresh", true));
            // png_destroy_info_struct with a NULL slot is a no-op (png.c:409)
            let mut nul: png_infop = ptr::null_mut();
            (a.png_destroy_info_struct)(c.p, &mut nul);
            (a.png_destroy_info_struct)(c.p, ptr::null_mut());
            out.push("destroy_null:ok".to_string());
            // png_info_init_3: a too-small size makes it reallocate the struct
            // (png.c:447), any size >= sizeof(png_info) is just a memset
            populate(a, &c);
            let before = c.info;
            (a.png_info_init_3)(&mut c.info, 0);
            out.push(format!("init3_0.moved:{}", c.info != before));
            out.push(format!("init3_0.null:{}", c.info.is_null()));
            out.extend(dump_info(a, c.p, c.info, "init3_0", true));
            populate(a, &c);
            let before = c.info;
            (a.png_info_init_3)(&mut c.info, 1 << 20);
            out.push(format!("init3_big.moved:{}", c.info != before));
            out.extend(dump_info(a, c.p, c.info, "init3_big", true));
            destroy(a, &mut c);
            out
        });
    }
}

#[test]
fn t_custom_allocators() {
    for kind in [Kind::Write, Kind::Read] {
        diff(&format!("mem_fn:{}", kind.tag()), &move |a| unsafe {
            ALLOC_LIVE.with(|c| c.set(0));
            let mut out = Vec::new();
            let p = match kind {
                Kind::Read => (a.png_create_read_struct_2)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                    tok(7),
                    Some(my_malloc),
                    Some(my_free),
                ),
                Kind::Write => (a.png_create_write_struct_2)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                    tok(7),
                    Some(my_malloc),
                    Some(my_free),
                ),
            };
            out.push(format!("created:{}", !p.is_null()));
            out.push(format!("mem_ptr:{}", pdesc((a.png_get_mem_ptr)(p), tok(7))));
            let info = (a.png_create_info_struct)(p);
            let end = (a.png_create_info_struct)(p);
            let mut c = Ctx { p, info, end, kind };
            populate(a, &c);
            out.extend(dump_info(a, c.p, c.info, "full", true));
            out.extend(dump_struct(a, c.p, "st"));
            // png_malloc / png_calloc / png_malloc_warn all route through the
            // user allocator (pngmem.c:91)
            let m = (a.png_malloc)(c.p, 100);
            out.push(format!("malloc:{}", !m.is_null()));
            if !m.is_null() {
                std::ptr::write_bytes(m as *mut u8, 0xab, 100);
                out.push(format!("malloc.rw:{}", *(m as *mut u8).add(99)));
            }
            (a.png_free)(c.p, m);
            let z = (a.png_calloc)(c.p, 64);
            out.push(format!("calloc:{}", !z.is_null()));
            if !z.is_null() {
                let all0 = (0..64).all(|i| *(z as *mut u8).add(i) == 0);
                out.push(format!("calloc.zero:{all0}"));
            }
            (a.png_free)(c.p, z);
            let w = (a.png_malloc_warn)(c.p, 33);
            out.push(format!("malloc_warn:{}", !w.is_null()));
            (a.png_free)(c.p, w);
            // png_free with NULL is a no-op (pngmem.c:236)
            (a.png_free)(c.p, ptr::null_mut());
            destroy(a, &mut c);
            out.push(format!("live_after_destroy:{}", ALLOC_LIVE.with(|c| c.get())));
            out
        });

        // the default allocator: png_malloc_default / png_free_default use the
        // system malloc directly (pngmem.c:200/255), so they are only exercised
        // on a struct that has no user allocator installed.
        diff(&format!("mem_default:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            let mut out = Vec::new();
            for sz in [1usize, 4, 100, 8192, 100_000] {
                let m = (a.png_malloc)(c.p, sz);
                out.push(format!("m{sz}:{}", !m.is_null()));
                (a.png_free)(c.p, m);
                let d = (a.png_malloc_default)(c.p, sz);
                out.push(format!("d{sz}:{}", !d.is_null()));
                (a.png_free_default)(c.p, d);
                let z = (a.png_calloc)(c.p, sz);
                out.push(format!(
                    "z{sz}:{}",
                    !z.is_null() && (0..sz).all(|i| *(z as *mut u8).add(i) == 0)
                ));
                (a.png_free)(c.p, z);
                let w = (a.png_malloc_warn)(c.p, sz);
                out.push(format!("w{sz}:{}", !w.is_null()));
                (a.png_free)(c.p, w);
            }
            (a.png_free_default)(c.p, ptr::null_mut());
            destroy(a, &mut c);
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 14. options, MNG features, keep-unknown, signature bytes, compression
//     parameters, filters, interlace handling
// ---------------------------------------------------------------------------

#[test]
fn t_options_and_mng() {
    for kind in [Kind::Write, Kind::Read] {
        diff(&format!("options:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            let mut out = Vec::new();
            // every option number (valid, odd, and out of range) x every onoff
            for option in -2..=OPTION_NEXT + 2 {
                for onoff in [0i32, 1, 2, -1] {
                    let r1 = (a.png_set_option)(c.p, option, onoff);
                    let r2 = (a.png_set_option)(c.p, option, onoff);
                    out.push(format!("opt{option}:{onoff}:{r1}:{r2}"));
                }
            }
            // and read every option back by re-setting it to its own state
            for option in (0..OPTION_NEXT).step_by(2) {
                out.push(format!(
                    "optread{option}:{}",
                    (a.png_set_option)(c.p, option, 0)
                ));
            }
            // MNG features: the value is masked with PNG_ALL_MNG_FEATURES
            // (pngset.c:1564)
            for v in [0u32, 1, 2, 4, 5, 7, 0xffff_ffff] {
                out.push(format!(
                    "mng{v}:{}",
                    (a.png_permit_mng_features)(c.p, v)
                ));
            }
            (a.png_permit_mng_features)(c.p, 0);
            // benign errors on/off (no getter; must not warn)
            for v in [1i32, 0, 1] {
                (a.png_set_benign_errors)(c.p, v);
                out.push(format!("benign{v}:set"));
            }
            (a.png_set_benign_errors)(c.p, 0);
            // invalid-index checking maps onto png_get_palette_max
            // (pngset.c:1960, pngget.c:1359)
            for v in [1i32, 0, -1, 7] {
                (a.png_set_check_for_invalid_index)(c.p, v);
                out.push(format!(
                    "cii{v}:{}",
                    (a.png_get_palette_max)(c.p, c.info)
                ));
            }
            // signature bytes 0..8 (9 or more is a png_error, png.c:65)
            for n in 0..=8 {
                (a.png_set_sig_bytes)(c.p, n);
                let sig = (a.png_get_signature)(c.p, c.info);
                let s: Vec<u8> = if sig.is_null() {
                    Vec::new()
                } else {
                    (0..8).map(|i| *sig.add(i)).collect()
                };
                out.push(format!("sig{n}:{s:02x?}"));
            }
            (a.png_set_sig_bytes)(c.p, -3);
            out.push("sig_negative:ok".to_string());
            destroy(a, &mut c);
            out
        });
    }
}

#[test]
fn t_keep_unknown_and_handle_as_unknown() {
    let probe: Vec<[u8; 5]> = vec![
        *b"bKGD\0",
        *b"cHRM\0",
        *b"gAMA\0",
        *b"tEXt\0",
        *b"zTXt\0",
        *b"iTXt\0",
        *b"sBIT\0",
        *b"pRVt\0",
        *b"ABCD\0",
        *b"IHDR\0",
        *b"IDAT\0",
        *b"IEND\0",
        *b"vpAg\0",
        *b"sTER\0",
    ];
    // Two custom chunk lists (5 bytes per entry: 4 name + 1 keep, filled in by
    // png_set_keep_unknown_chunks itself, so only the names matter here).
    let list_a: Vec<u8> = b"pRVt\0ABCD\0vpAg\0".to_vec();
    let list_b: Vec<u8> = b"bKGD\0gAMA\0".to_vec();

    for kind in [Kind::Write, Kind::Read] {
        for keep in [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
        ] {
            let probe = probe.clone();
            let list_a = list_a.clone();
            let list_b = list_b.clone();
            diff(
                &format!("keep:{}:{keep}", kind.tag()),
                &move |a| unsafe {
                    let mut c = make(a, kind);
                    let mut out = Vec::new();
                    let dump = |a: &Api, p: png_structp, tag: &str, o: &mut Vec<String>| {
                        for n in probe.iter() {
                            o.push(format!(
                                "{tag}.{}:{}",
                                String::from_utf8_lossy(&n[..4]),
                                (a.png_handle_as_unknown)(p, n.as_ptr())
                            ));
                        }
                    };
                    dump(a, c.p, "pre", &mut out);
                    // num_chunks == 0: only the default is changed
                    // (pngset.c:1616)
                    (a.png_set_keep_unknown_chunks)(c.p, keep, ptr::null(), 0);
                    dump(a, c.p, "default", &mut out);
                    // an explicit list
                    (a.png_set_keep_unknown_chunks)(
                        c.p,
                        keep,
                        list_a.as_ptr(),
                        (list_a.len() / 5) as c_int,
                    );
                    dump(a, c.p, "listA", &mut out);
                    (a.png_set_keep_unknown_chunks)(
                        c.p,
                        keep,
                        list_b.as_ptr(),
                        (list_b.len() / 5) as c_int,
                    );
                    dump(a, c.p, "listB", &mut out);
                    // num_chunks < 0: the built-in "all known ancillary chunks"
                    // list (pngset.c:1630)
                    (a.png_set_keep_unknown_chunks)(c.p, keep, ptr::null(), -1);
                    dump(a, c.p, "allknown", &mut out);
                    // reset everything back to the default, which empties the
                    // list again (pngset.c:1736)
                    (a.png_set_keep_unknown_chunks)(
                        c.p,
                        PNG_HANDLE_CHUNK_AS_DEFAULT,
                        ptr::null(),
                        -1,
                    );
                    dump(a, c.p, "reset", &mut out);
                    // NULL chunk_name is always the default (png.c:936)
                    out.push(format!(
                        "nullname:{}",
                        (a.png_handle_as_unknown)(c.p, ptr::null())
                    ));
                    destroy(a, &mut c);
                    out
                },
            );
        }
    }
}

#[test]
fn t_compression_and_filter_params() {
    diff("compression_params", &|a| unsafe {
        let mut c = make(a, Kind::Write);
        let mut out = Vec::new();
        for lvl in [-1i32, 0, 1, 6, 9] {
            (a.png_set_compression_level)(c.p, lvl);
            (a.png_set_text_compression_level)(c.p, lvl);
        }
        for ml in [1i32, 8, 9] {
            (a.png_set_compression_mem_level)(c.p, ml);
            (a.png_set_text_compression_mem_level)(c.p, ml);
        }
        for st in [0i32, 1, 2, 3, 4] {
            (a.png_set_compression_strategy)(c.p, st);
            (a.png_set_text_compression_strategy)(c.p, st);
        }
        // out-of-range window bits are clamped with a warning
        // (pngwrite.c:1268/1274)
        for wb in [7i32, 8, 9, 15, 16, 20, 0, -5] {
            (a.png_set_compression_window_bits)(c.p, wb);
            (a.png_set_text_compression_window_bits)(c.p, wb);
            out.push(format!("wb{wb}:set"));
        }
        // only method 8 is legal; anything else warns (pngwrite.c:1294)
        for m in [8i32, 9, 0, -1] {
            (a.png_set_compression_method)(c.p, m);
            (a.png_set_text_compression_method)(c.p, m);
            out.push(format!("method{m}:set"));
        }
        (a.png_set_compression_method)(c.p, 8);
        (a.png_set_text_compression_method)(c.p, 8);
        (a.png_set_compression_window_bits)(c.p, 15);
        out.push(format!(
            "cbuf:{}",
            (a.png_get_compression_buffer_size)(c.p)
        ));
        // png_set_filter: filter values 5/6/7 would be app errors
        // (pngwrite.c:1078) and are therefore excluded here.
        for f in [
            PNG_NO_FILTERS,
            1,
            2,
            3,
            4,
            PNG_FILTER_NONE,
            PNG_FILTER_SUB,
            PNG_FILTER_UP,
            PNG_FILTER_AVG,
            PNG_FILTER_PAETH,
            PNG_FILTER_NONE | PNG_FILTER_SUB,
            PNG_ALL_FILTERS,
        ] {
            (a.png_set_filter)(c.p, PNG_FILTER_TYPE_BASE, f);
            out.push(format!("filter{f:#x}:set"));
        }
        // with MNG filter 64 permitted, method 64 is silently mapped onto the
        // base method (pngwrite.c:1066)
        (a.png_permit_mng_features)(c.p, PNG_FLAG_MNG_FILTER_64 as png_uint_32);
        (a.png_set_filter)(c.p, PNG_INTRAPIXEL_DIFFERENCING, PNG_ALL_FILTERS);
        out.push("filter64:set".to_string());
        (a.png_permit_mng_features)(c.p, 0);
        // the deprecated weighted-filter API is a no-op (pngwrite.c:1186)
        let w = [1.0f64, 2.0, 3.0];
        let cst = [1.0f64, 1.5];
        (a.png_set_filter_heuristics)(c.p, 1, 3, w.as_ptr(), cst.as_ptr());
        (a.png_set_filter_heuristics)(c.p, 0, 0, ptr::null(), ptr::null());
        let wf = [100000i32, 200000, 300000];
        let cf = [100000i32, 150000];
        (a.png_set_filter_heuristics_fixed)(c.p, 2, 3, wf.as_ptr(), cf.as_ptr());
        (a.png_set_filter_heuristics_fixed)(c.p, 0, 0, ptr::null(), ptr::null());
        out.push("heuristics:set".to_string());
        out.extend(dump_struct(a, c.p, "st"));
        destroy(a, &mut c);
        out
    });
}

#[test]
fn t_interlace_handling() {
    // On a fresh struct `png_ptr->interlaced` is 0 so the pass count is 1;
    // after `png_write_info` on an ADAM7 image it becomes 7 (pngtrans.c:131).
    for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
        diff(&format!("interlace:{il}"), &move |a| unsafe {
            let mut out = Vec::new();
            let mut c = make(a, Kind::Read);
            out.push(format!("read_fresh:{}", (a.png_set_interlace_handling)(c.p)));
            destroy(a, &mut c);
            let mut c = make(a, Kind::Write);
            out.push(format!("write_fresh:{}", (a.png_set_interlace_handling)(c.p)));
            (a.png_set_IHDR)(
                c.p,
                c.info,
                8,
                8,
                8,
                PNG_COLOR_TYPE_RGB,
                il,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (a.png_write_info)(c.p, c.info);
            out.push(format!(
                "write_after_info:{}",
                (a.png_set_interlace_handling)(c.p)
            ));
            out.extend(dump_struct(a, c.p, "st"));
            out.push(format!("bytes:{}", out_take().len()));
            destroy(a, &mut c);
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 15. png_set_rows / png_get_rows
// ---------------------------------------------------------------------------

#[test]
fn t_rows() {
    for kind in [Kind::Write, Kind::Read] {
        diff(&format!("rows:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            (a.png_set_IHDR)(
                c.p,
                c.info,
                4,
                3,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            let mut rows: Vec<Vec<u8>> = (0..3).map(|i| vec![i as u8; 12]).collect();
            let mut ptrs: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
            let mut out = dump_info(a, c.p, c.info, "pre", true);
            (a.png_set_rows)(c.p, c.info, ptrs.as_mut_ptr());
            out.push(format!(
                "rows:{}",
                pdesc(
                    (a.png_get_rows)(c.p, c.info) as *const c_void,
                    ptrs.as_mut_ptr() as *const c_void
                )
            ));
            out.extend(dump_info(a, c.p, c.info, "set", true));
            // setting the same pointer again must not free anything
            // (pngset.c:1785)
            (a.png_set_rows)(c.p, c.info, ptrs.as_mut_ptr());
            out.extend(dump_info(a, c.p, c.info, "again", true));
            // NULL clears the pointer but leaves PNG_INFO_IDAT set
            (a.png_set_rows)(c.p, c.info, ptr::null_mut());
            out.push(format!(
                "rows_null:{}",
                (a.png_get_rows)(c.p, c.info).is_null()
            ));
            out.extend(dump_info(a, c.p, c.info, "cleared", true));
            // and once more, then clear again before destroying so that
            // PNG_FREE_ROWS can never reach the caller's buffers
            (a.png_set_rows)(c.p, c.info, ptrs.as_mut_ptr());
            (a.png_free_data)(c.p, c.info, PNG_FREE_ROWS, -1);
            out.extend(dump_info(a, c.p, c.info, "freed", true));
            (a.png_set_rows)(c.p, c.info, ptr::null_mut());
            destroy(a, &mut c);
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 16. paths on which the C library merely warns and carries on
// ---------------------------------------------------------------------------

#[test]
fn t_benign_warning_paths() {
    for kind in [Kind::Write, Kind::Read] {
        diff(&format!("warnpaths:{}", kind.tag()), &move |a| unsafe {
            let mut c = make(a, kind);
            // Turn png_app_error / png_app_warning / png_benign_error into
            // warnings so that the messages can be compared instead of
            // aborting (pngset.c:1936).
            (a.png_set_benign_errors)(c.p, 1);
            let mut out = Vec::new();
            (a.png_set_IHDR)(
                c.p,
                c.info,
                8,
                8,
                8,
                PNG_COLOR_TYPE_GRAY,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );

            // cICP with non-zero matrix coefficients: plain png_warning
            // (pngset.c:152) and the chunk is not marked valid
            (a.png_set_cICP)(c.p, c.info, 9, 16, 3, 1);
            out.push(format!(
                "cICP.valid:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_cICP)
            ));

            // tIME out of range: "Ignoring invalid time value" (pngset.c:1170)
            for t in [
                png_time { year: 2000, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
                png_time { year: 2000, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
                png_time { year: 2000, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
                png_time { year: 2000, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
                png_time { year: 2000, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
                png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
                png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
            ] {
                (a.png_set_tIME)(c.p, c.info, &t);
            }
            out.push(format!(
                "tIME.valid:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_tIME)
            ));

            // sCAL with a non-positive dimension: "Invalid sCAL width/height
            // ignored" (pngset.c:682/685 and 711/714)
            (a.png_set_sCAL)(c.p, c.info, PNG_SCALE_METER, 0.0, 1.0);
            (a.png_set_sCAL)(c.p, c.info, PNG_SCALE_METER, -1.0, 1.0);
            (a.png_set_sCAL)(c.p, c.info, PNG_SCALE_METER, 1.0, 0.0);
            (a.png_set_sCAL)(c.p, c.info, PNG_SCALE_METER, 1.0, -2.0);
            (a.png_set_sCAL_fixed)(c.p, c.info, PNG_SCALE_RADIAN, 0, 1);
            (a.png_set_sCAL_fixed)(c.p, c.info, PNG_SCALE_RADIAN, -1, 1);
            (a.png_set_sCAL_fixed)(c.p, c.info, PNG_SCALE_RADIAN, 1, 0);
            (a.png_set_sCAL_fixed)(c.p, c.info, PNG_SCALE_RADIAN, 1, -1);
            out.push(format!(
                "sCAL.valid:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_sCAL)
            ));

            // the deprecated eXIf pair (pngset.c:322, pngget.c:895)
            let mut ex = [1u8, 2, 3, 4];
            (a.png_set_eXIf)(c.p, c.info, ex.as_mut_ptr());
            let mut gp: *mut png_byte = ptr::null_mut();
            out.push(format!(
                "eXIf.get:{}",
                (a.png_get_eXIf)(c.p, c.info, &mut gp)
            ));

            // hIST without a palette: "Invalid palette size, hIST allocation
            // skipped" (pngset.c:399)
            let h = [1u16, 2, 3, 4];
            (a.png_set_hIST)(c.p, c.info, h.as_ptr());
            out.push(format!(
                "hIST.valid:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_hIST)
            ));

            // tRNS colour key out of range for an 8-bit grey image
            // (pngset.c:1253)
            let key = png_color_16 { index: 0, red: 300, green: 400, blue: 500, gray: 600 };
            (a.png_set_tRNS)(c.p, c.info, ptr::null(), 0, &key);
            out.push(format!(
                "tRNS.num:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_tRNS)
            ));

            // cLLI above the PNG light-level limit (pngset.c:182)
            (a.png_set_cLLI_fixed)(c.p, c.info, 0x8000_0000, 1);
            (a.png_set_cLLI_fixed)(c.p, c.info, 1, 0xffff_ffff);
            out.push(format!(
                "cLLI.valid:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_cLLI)
            ));

            // mDCV chromaticities outside the representable range
            // (pngset.c:254) and light levels above the limit (pngset.c:269)
            (a.png_set_mDCV_fixed)(
                c.p, c.info, 200_000, 1, 1, 1, 1, 1, 1, 1, 100, 100,
            );
            (a.png_set_mDCV_fixed)(c.p, c.info, 1, 1, 1, 1, 1, 1, 1, 1, 0x8000_0000, 1);
            (a.png_set_mDCV_fixed)(c.p, c.info, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0x8000_0000);
            out.push(format!(
                "mDCV.valid:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_mDCV)
            ));

            // cHRM XYZ that png_xy_from_XYZ rejects (pngset.c:94)
            (a.png_set_cHRM_XYZ_fixed)(c.p, c.info, 0, 0, 0, 0, 0, 0, 0, 0, 0);
            out.push(format!(
                "cHRM.valid:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_cHRM)
            ));

            // iCCP with a bogus compression method: png_app_error but the
            // profile is still stored (pngset.c:903 has no early return)
            let nm = CString::new("bad-cm").unwrap();
            let prof = [0u8, 0, 0, 8, 9, 9, 9, 9];
            (a.png_set_iCCP)(c.p, c.info, nm.as_ptr(), 1, prof.as_ptr(), 8);
            out.push(format!(
                "iCCP.valid:{}",
                (a.png_get_valid)(c.p, c.info, PNG_INFO_iCCP)
            ));

            // pCAL rejections (pngset.c:515 / 522 / 533)
            {
                let purpose = CString::new("p").unwrap();
                let units = CString::new("u").unwrap();
                let good = CString::new("1").unwrap();
                let bad = CString::new("not a number").unwrap();
                let mut ok = [good.as_ptr() as *mut c_char];
                let mut nok = [bad.as_ptr() as *mut c_char];
                (a.png_set_pCAL)(
                    c.p, c.info, purpose.as_ptr(), 0, 1, 4, 1, units.as_ptr(),
                    ok.as_mut_ptr(),
                );
                (a.png_set_pCAL)(
                    c.p, c.info, purpose.as_ptr(), 0, 1, -1, 1, units.as_ptr(),
                    ok.as_mut_ptr(),
                );
                (a.png_set_pCAL)(
                    c.p, c.info, purpose.as_ptr(), 0, 1, 0, 256, units.as_ptr(),
                    ok.as_mut_ptr(),
                );
                (a.png_set_pCAL)(
                    c.p, c.info, purpose.as_ptr(), 0, 1, 0, 1, units.as_ptr(),
                    nok.as_mut_ptr(),
                );
                out.push(format!(
                    "pCAL.valid:{}",
                    (a.png_get_valid)(c.p, c.info, PNG_INFO_pCAL)
                ));
            }

            // text with a compression mode out of range (pngset.c:1031)
            {
                let k = CString::new("K").unwrap();
                let t = CString::new("T").unwrap();
                for comp in [-3i32, -2, 3, 99] {
                    let e = png_text {
                        compression: comp,
                        key: k.as_ptr() as *mut c_char,
                        text: t.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: ptr::null_mut(),
                        lang_key: ptr::null_mut(),
                    };
                    (a.png_set_text)(c.p, c.info, &e, 1);
                }
                let mut n = 0i32;
                out.push(format!(
                    "text.n:{}",
                    (a.png_get_text)(c.p, c.info, ptr::null_mut(), &mut n)
                ));
            }

            // sPLT with a NULL name / NULL entry array (pngset.c:1327)
            {
                let mut ent = [png_sPLT_entry { red: 1, green: 1, blue: 1, alpha: 1, frequency: 1 }];
                let bad = png_sPLT_t {
                    name: ptr::null_mut(),
                    depth: 8,
                    entries: ent.as_mut_ptr(),
                    nentries: 1,
                };
                (a.png_set_sPLT)(c.p, c.info, &bad, 1);
                let nm2 = CString::new("x").unwrap();
                let bad2 = png_sPLT_t {
                    name: nm2.as_ptr() as *mut c_char,
                    depth: 8,
                    entries: ptr::null_mut(),
                    nentries: 1,
                };
                (a.png_set_sPLT)(c.p, c.info, &bad2, 1);
                let mut sp: *mut png_sPLT_t = ptr::null_mut();
                out.push(format!(
                    "sPLT.n:{}",
                    (a.png_get_sPLT)(c.p, c.info, &mut sp)
                ));
            }

            // keep_unknown with an out-of-range keep, and with a missing list
            // (pngset.c:1611 / 1665)
            for k in [-1i32, PNG_HANDLE_CHUNK_LAST, 99] {
                (a.png_set_keep_unknown_chunks)(c.p, k, ptr::null(), 0);
            }
            (a.png_set_keep_unknown_chunks)(c.p, PNG_HANDLE_CHUNK_NEVER, ptr::null(), 3);
            out.push("keep:done".to_string());

            // an unknown-chunk relocation to a location with no valid bit
            // (pngset.c:1540 falls back to PNG_HAVE_IHDR)
            {
                let mut d = [1u8, 2, 3];
                let u = png_unknown_chunk {
                    name: *b"pRVt\0",
                    data: d.as_mut_ptr(),
                    size: 3,
                    location: LOC_AFTER_IDAT as png_byte,
                };
                (a.png_set_unknown_chunks)(c.p, c.info, &u, 1);
                (a.png_set_unknown_chunk_location)(c.p, c.info, 0, 0);
                (a.png_set_unknown_chunk_location)(c.p, c.info, 0, 0x04);
                let mut up: *mut png_unknown_chunk = ptr::null_mut();
                let n = (a.png_get_unknown_chunks)(c.p, c.info, &mut up);
                if n > 0 && !up.is_null() {
                    out.push(format!("unk.loc:{}", (*up).location));
                }
            }

            // PLTE longer than 256 entries on a non-palette image: a plain
            // png_warning followed by a return (pngset.c:771)
            {
                let big: Vec<png_color> = vec![png_color { red: 1, green: 2, blue: 3 }; 300];
                (a.png_set_PLTE)(c.p, c.info, big.as_ptr(), 300);
                out.push(format!(
                    "PLTE.valid:{}",
                    (a.png_get_valid)(c.p, c.info, PNG_INFO_PLTE)
                ));
            }

            // compression buffer below the deflate minimum on a write struct
            // (pngset.c:1838); on a read struct any non-zero size is accepted
            (a.png_set_compression_buffer_size)(c.p, 1);
            (a.png_set_compression_buffer_size)(c.p, 5);
            out.push(format!(
                "cbuf:{}",
                (a.png_get_compression_buffer_size)(c.p)
            ));
            (a.png_set_compression_buffer_size)(c.p, 8192);

            out.extend(dump_info(a, c.p, c.info, "post", false));
            out.extend(dump_struct(a, c.p, "post"));
            destroy(a, &mut c);
            out
        });
    }
}

// ---------------------------------------------------------------------------
// 17. the compression / filter setters, compared through the bytes they
//     actually produce in a complete write
// ---------------------------------------------------------------------------

#[test]
fn t_compression_output() {
    // (level, mem_level, strategy, window_bits, buffer_size, filters,
    //  text level, text strategy, text window bits)
    let mut settings: Vec<(c_int, c_int, c_int, c_int, usize, c_int, c_int, c_int, c_int)> =
        Vec::new();
    for level in [0i32, 1, 3, 6, 9] {
        settings.push((level, 8, 0, 15, 8192, PNG_ALL_FILTERS, 6, 0, 15));
    }
    for ml in [1i32, 5, 8, 9] {
        settings.push((6, ml, 0, 15, 8192, PNG_ALL_FILTERS, 6, 0, 15));
    }
    for st in [0i32, 1, 2, 3, 4] {
        settings.push((6, 8, st, 15, 8192, PNG_ALL_FILTERS, 6, st, 15));
    }
    for wb in [8i32, 9, 11, 13, 15] {
        settings.push((6, 8, 0, wb, 8192, PNG_ALL_FILTERS, 6, 0, wb));
    }
    for bs in [6usize, 32, 64, 1024, 8192, 65536] {
        settings.push((6, 8, 0, 15, bs, PNG_ALL_FILTERS, 6, 0, 15));
    }
    for f in [
        PNG_NO_FILTERS,
        PNG_FILTER_NONE,
        PNG_FILTER_SUB,
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_FILTER_NONE | PNG_FILTER_PAETH,
        PNG_ALL_FILTERS,
    ] {
        settings.push((6, 8, 0, 15, 8192, f, 6, 0, 15));
    }
    for tl in [0i32, 1, 9] {
        settings.push((6, 8, 0, 15, 8192, PNG_ALL_FILTERS, tl, 0, 15));
    }

    for (i, s) in settings.into_iter().enumerate() {
        diff(&format!("cout:{i}"), &move |a| unsafe {
            let (level, ml, st, wb, bs, filters, tl, tst, twb) = s;
            let mut c = make(a, Kind::Write);
            let mut out = Vec::new();
            let (w, h) = (24u32, 9u32);
            (a.png_set_IHDR)(
                c.p,
                c.info,
                w,
                h,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            // a compressible text chunk so that the text-compression settings
            // affect the output too
            let key = CString::new("Comment").unwrap();
            let val = CString::new("abababababababababababababababababababab").unwrap();
            let e = png_text {
                compression: PNG_TEXT_COMPRESSION_zTXt,
                key: key.as_ptr() as *mut c_char,
                text: val.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: ptr::null_mut(),
                lang_key: ptr::null_mut(),
            };
            (a.png_set_text)(c.p, c.info, &e, 1);
            (a.png_set_compression_level)(c.p, level);
            (a.png_set_compression_mem_level)(c.p, ml);
            (a.png_set_compression_strategy)(c.p, st);
            (a.png_set_compression_window_bits)(c.p, wb);
            (a.png_set_compression_method)(c.p, 8);
            (a.png_set_compression_buffer_size)(c.p, bs);
            (a.png_set_text_compression_level)(c.p, tl);
            (a.png_set_text_compression_strategy)(c.p, tst);
            (a.png_set_text_compression_window_bits)(c.p, twb);
            (a.png_set_text_compression_mem_level)(c.p, 8);
            (a.png_set_text_compression_method)(c.p, 8);
            (a.png_set_filter)(c.p, PNG_FILTER_TYPE_BASE, filters);
            (a.png_set_write_status_fn)(c.p, Some(write_status_cb));

            let rb = (w * 3) as usize;
            let mut rows: Vec<Vec<u8>> = (0..h)
                .map(|y| {
                    (0..rb)
                        .map(|x| ((x as u32 * 7 + y * 31) % 251) as u8)
                        .collect()
                })
                .collect();
            let mut ptrs: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
            (a.png_write_info)(c.p, c.info);
            out.extend(dump_struct(a, c.p, "mid"));
            (a.png_write_image)(c.p, ptrs.as_mut_ptr());
            (a.png_write_end)(c.p, c.info);
            out.extend(dump_struct(a, c.p, "end"));
            let bytes = out_take();
            out.push(format!("len:{}", bytes.len()));
            out.push(format!("bytes:{bytes:02x?}"));
            destroy(a, &mut c);
            out
        });
    }
}

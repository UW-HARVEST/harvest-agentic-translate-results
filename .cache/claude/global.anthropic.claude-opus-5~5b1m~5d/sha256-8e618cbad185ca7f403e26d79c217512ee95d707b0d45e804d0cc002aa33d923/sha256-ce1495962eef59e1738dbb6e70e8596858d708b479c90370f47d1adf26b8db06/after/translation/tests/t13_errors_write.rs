//! Phase C — ERRORS.md rows 956..1128 of the
//! `pngrutil.c / pngwrite.c / pngwutil.c / pngwtran.c` section, restricted to
//! the WRITE side plus the ICC-profile validators (`png_icc_check_length` /
//! `_header` / `_tag_table`, which are also reachable from the write path via
//! `png_set_iCCP` + `png_write_info` -> `png_write_iCCP`).
//!
//! The malformed-input-stream / `png_handle_*` READ half of `pngrutil.c`
//! (rows 787..955) is deliberately NOT covered here; another test file owns it.
//!
//! Every case constructs the exact invalid input, drives BOTH libraries and
//! asserts the same rejection: same return value, same success/failure, the
//! same captured `Diag` (warning + error message text) and — for the write
//! session probes — the same bytes emitted before the failure.
//!
//! Build configuration facts that make some ERRORS.md rows UNREACHABLE in this
//! build (see `c_src/include/pnglibconf.h`):
//!   * `PNG_BENIGN_WRITE_ERRORS_SUPPORTED` is **undefined** and
//!     `PNG_LIBPNG_BUILD_BASE_TYPE == PNG_LIBPNG_BUILD_BETA` (so
//!     `PNG_RELEASE_BUILD == 0`).  Therefore a *write* png_struct has neither
//!     `PNG_FLAG_BENIGN_ERRORS_WARN` nor `PNG_FLAG_APP_WARNINGS_WARN` nor
//!     `PNG_FLAG_APP_ERRORS_WARN` set (pngwrite.c:590-607), so on the write
//!     side `png_benign_error`, `png_app_warning` and `png_app_error` are all
//!     hard `png_error`s (pngerror.c:308-360).
//!   * every write transform and every write chunk writer IS compiled in, so
//!     rows 980, 981, 982, 986, 993, 1002 and 1025 ("... is not defined" /
//!     "... not supported" warnings) can never fire.
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use std::ffi::CString;

// ---------------------------------------------------------------------------
// probes
// ---------------------------------------------------------------------------

/// Result of one plain probe: return value (as i64), whether it errored, diags.
#[derive(Debug, PartialEq)]
struct P(i64, bool, Diag);

fn probe<F: FnOnce(&'static Api) -> i64>(api: &'static Api, f: F) -> P {
    set_current_api(api);
    diag_reset();
    let r = guard(|| f(api));
    P(r.unwrap_or(i64::MIN), r.is_some(), diag_take())
}

macro_rules! same {
    ($label:expr, $f:expr) => {{
        if std::env::var_os("PNGTRACE").is_some() {
            eprintln!("TRACE {}", $label);
        }
        let c = probe(c_api(), $f);
        let r = probe(rs_api(), $f);
        if std::env::var_os("PNGDUMP").is_some() { eprintln!("DUMP {} => {:?}", $label, c); }
        assert_eq!(c, r, "{}", $label);
        c
    }};
}

/// Result of one write-session probe.  The png_struct is created *outside* the
/// `guard` so the bytes emitted before an aborting `png_error` are still
/// observable and can be compared.
#[derive(PartialEq)]
struct WP {
    ok: bool,
    ret: i64,
    diag: Diag,
    bytes: String,
    flushes: u32,
}

impl std::fmt::Debug for WP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WP {{ ok: {}, ret: {}, flushes: {}, warnings: {:?}, errors: {:?}, bytes({}): {} }}",
            self.ok,
            self.ret,
            self.flushes,
            self.diag.warnings,
            self.diag.errors,
            self.bytes.len() / 2,
            if self.bytes.len() > 400 {
                format!("{}...", &self.bytes[..400])
            } else {
                self.bytes.clone()
            }
        )
    }
}

fn wprobe<F: FnOnce(&'static Api, png_structp, png_infop) -> i64>(
    api: &'static Api,
    f: F,
) -> WP {
    set_current_api(api);
    diag_reset();
    let mut s = unsafe { WriteSess::new(api) };
    let (png, info) = (s.png, s.info);
    let r = guard(|| f(api, png, info));
    let diag = diag_take();
    let bytes = hex(&std::mem::take(&mut s.sink.buf));
    let flushes = s.sink.flushes;
    drop(s);
    WP {
        ok: r.is_some(),
        ret: r.unwrap_or(i64::MIN),
        diag,
        bytes,
        flushes,
    }
}

macro_rules! wsame {
    ($label:expr, $f:expr) => {{
        if std::env::var_os("PNGTRACE").is_some() {
            eprintln!("TRACE {}", $label);
        }
        let c = wprobe(c_api(), $f);
        let r = wprobe(rs_api(), $f);
        if std::env::var_os("PNGDUMP").is_some() { eprintln!("DUMP {} => {:?}", $label, c); }
        assert_eq!(c, r, "{}", $label);
        c
    }};
}

/// Result of one simplified-API probe.  The simplified API installs its own
/// `png_safe_error`/`png_safe_warning` (pngwrite.c:1531) so nothing reaches the
/// harness `Diag`; the observable state is the return value plus
/// `png_image::warning_or_error` and `png_image::message`.
#[derive(Debug, PartialEq)]
struct SP {
    ret: i64,
    woe: u32,
    msg: String,
    extra: i64,
}

/// `img` is a raw pointer so the closure may take `&mut png_image` itself.
fn sprobe<F: FnOnce(&'static Api) -> (i64, i64)>(
    api: &'static Api,
    img: *const png_image,
    f: F,
) -> SP {
    set_current_api(api);
    diag_reset();
    let (ret, extra) = guard(|| f(api)).unwrap_or((i64::MIN, i64::MIN));
    let (woe, msg) = unsafe {
        let b = &(*img).message;
        let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        (
            (*img).warning_or_error,
            b[..n].iter().map(|&c| c as u8 as char).collect::<String>(),
        )
    };
    SP {
        ret,
        woe,
        msg,
        extra,
    }
}

fn fnv(b: &[u8]) -> i64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    (h & 0x7fff_ffff_ffff_ffff) as i64
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

const IL_NONE: c_int = PNG_INTERLACE_NONE;

/// A deterministic RGB8 row, generously over-allocated (write transforms may
/// consume more application bytes per row than PNG_ROWBYTES).
fn row_bytes(w: u32) -> Vec<u8> {
    (0..(w as usize * 8 + 32)).map(|i| (i * 37 + 11) as u8).collect()
}

/// Drive a complete, valid, minimal write (RGB8) through the low-level API.
unsafe fn write_small(
    api: &'static Api,
    png: png_structp,
    info: png_infop,
    w: u32,
    h: u32,
) {
    (api.png_set_IHDR)(png, info, w, h, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
    (api.png_write_info)(png, info);
    let row = row_bytes(w);
    for _ in 0..h {
        (api.png_write_row)(png, row.as_ptr());
    }
    (api.png_write_end)(png, info);
}

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// Encoded D50 as an ICC XYZNumber (png.c:1578).
const D50: [u8; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];

/// Build a syntactically well-formed minimal ICC profile.
///
/// `tag_count` tags are appended after the 132-byte header; each is
/// `(id, start, length)`.  `declared_len` goes into the profile's own length
/// word (offset 0).
fn icc(
    declared_len: u32,
    version_major: u8,
    class: &[u8; 4],
    space: &[u8; 4],
    pcs: &[u8; 4],
    sig: &[u8; 4],
    intent: u32,
    d50_ok: bool,
    tags: &[(u32, u32, u32)],
) -> Vec<u8> {
    let n = 132 + 12 * tags.len();
    let mut p = vec![0u8; n];
    p[0..4].copy_from_slice(&be32(declared_len));
    p[8] = version_major;
    p[12..16].copy_from_slice(class);
    p[16..20].copy_from_slice(space);
    p[20..24].copy_from_slice(pcs);
    p[36..40].copy_from_slice(sig);
    p[64..68].copy_from_slice(&be32(intent));
    if d50_ok {
        p[68..80].copy_from_slice(&D50);
    } else {
        p[68..80].copy_from_slice(&[0xffu8; 12]);
    }
    p[128..132].copy_from_slice(&be32(tags.len() as u32));
    for (i, &(id, start, len)) in tags.iter().enumerate() {
        let o = 132 + 12 * i;
        p[o..o + 4].copy_from_slice(&be32(id));
        p[o + 4..o + 8].copy_from_slice(&be32(start));
        p[o + 8..o + 12].copy_from_slice(&be32(len));
    }
    p
}

/// The canonical, accepted grey profile: 132 bytes, no tags.
fn icc_good_gray() -> Vec<u8> {
    icc(132, 2, b"mntr", b"GRAY", b"XYZ ", b"acsp", 0, true, &[])
}

fn icc_good_rgb() -> Vec<u8> {
    icc(132, 2, b"mntr", b"RGB ", b"XYZ ", b"acsp", 0, true, &[])
}

// ===========================================================================
// 1. Ordering errors in the write pipeline
//    rows 976, 978, 979, 983, 984, 988, 989, 991, 992, 999, 1023, 1024
// ===========================================================================

#[test]
fn write_pipeline_ordering() {
    // -- NULL png_ptr / info_ptr guards -----------------------------------
    // rows 976/978 png_write_info(_before_PLTE) NULL guards
    same!("png_write_info(NULL,NULL)", |api| {
        unsafe { (api.png_write_info)(std::ptr::null_mut(), std::ptr::null()) };
        0
    });
    same!("png_write_info_before_PLTE(NULL,NULL)", |api| {
        unsafe {
            (api.png_write_info_before_PLTE)(std::ptr::null_mut(), std::ptr::null())
        };
        0
    });
    wsame!("png_write_info(png,NULL info)", |api, png, _info| {
        unsafe { (api.png_write_info)(png, std::ptr::null()) };
        0
    });
    wsame!("png_write_info_before_PLTE(png,NULL info)", |api, png, _i| {
        unsafe { (api.png_write_info_before_PLTE)(png, std::ptr::null()) };
        0
    });
    // row 983 png_write_end(NULL)
    same!("png_write_end(NULL,NULL)", |api| {
        unsafe { (api.png_write_end)(std::ptr::null_mut(), std::ptr::null_mut()) };
        0
    });
    // rows 988/989/991 png_write_rows/_image/_row NULL png_ptr
    same!("png_write_rows(NULL)", |api| {
        unsafe { (api.png_write_rows)(std::ptr::null_mut(), std::ptr::null_mut(), 7) };
        0
    });
    same!("png_write_image(NULL)", |api| {
        unsafe { (api.png_write_image)(std::ptr::null_mut(), std::ptr::null_mut()) };
        0
    });
    same!("png_write_row(NULL)", |api| {
        unsafe { (api.png_write_row)(std::ptr::null_mut(), std::ptr::null()) };
        0
    });
    // row 999 png_destroy_write_struct NULL guards
    same!("png_destroy_write_struct(NULL)", |api| {
        unsafe { (api.png_destroy_write_struct)(std::ptr::null_mut(), std::ptr::null_mut()) };
        0
    });
    same!("png_destroy_write_struct(ptr to NULL)", |api| {
        let mut p: png_structp = std::ptr::null_mut();
        unsafe { (api.png_destroy_write_struct)(&mut p, std::ptr::null_mut()) };
        0
    });
    // rows 1021/1022 setter NULL guards
    same!("png_set_write_status_fn(NULL)", |api| {
        unsafe { (api.png_set_write_status_fn)(std::ptr::null_mut(), None) };
        0
    });
    same!("png_set_write_user_transform_fn(NULL)", |api| {
        unsafe { (api.png_set_write_user_transform_fn)(std::ptr::null_mut(), None) };
        0
    });
    // row 1128 png_do_write_transformations(NULL)
    same!("png_do_write_transformations(NULL)", |api| {
        unsafe {
            (api.png_do_write_transformations)(std::ptr::null_mut(), std::ptr::null_mut())
        };
        0
    });
    // rows 997/998 png_set_flush / png_write_flush NULL + no-op paths
    same!("png_set_flush(NULL)", |api| {
        unsafe { (api.png_set_flush)(std::ptr::null_mut(), 3) };
        0
    });
    same!("png_write_flush(NULL)", |api| {
        unsafe { (api.png_write_flush)(std::ptr::null_mut()) };
        0
    });
    for n in [-1000i32, -1, 0, 1, i32::MIN, i32::MAX] {
        wsame!(format!("png_set_flush({}) then flush", n), |api, png, info| {
            unsafe {
                (api.png_set_flush)(png, n);
                write_small(api, png, info, 4, 4);
                // row 998: all rows already written -> png_write_flush returns
                (api.png_write_flush)(png);
            }
            0
        });
    }

    // -- png_write_info twice --------------------------------------------
    // The `PNG_WROTE_INFO_BEFORE_PLTE` guard (pngwrite.c:236) means the second
    // call skips the signature+IHDR but repeats every ancillary chunk.
    wsame!("png_write_info twice", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_set_gAMA_fixed)(png, info, 45455);
            (api.png_write_info)(png, info);
            (api.png_write_info)(png, info);
        }
        0
    });
    wsame!("png_write_info_before_PLTE twice", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_write_info_before_PLTE)(png, info);
            (api.png_write_info_before_PLTE)(png, info);
        }
        0
    });

    // -- row 992: png_write_row before png_write_info ---------------------
    wsame!("png_write_row before png_write_info", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            let row = row_bytes(4);
            (api.png_write_row)(png, row.as_ptr());
        }
        0
    });
    wsame!("png_write_rows before png_write_info", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            let row = row_bytes(4);
            let mut ps: Vec<png_bytep> = (0..4).map(|_| row.as_ptr() as png_bytep).collect();
            (api.png_write_rows)(png, ps.as_mut_ptr(), 4);
        }
        0
    });
    wsame!("png_write_image before png_write_info", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            let row = row_bytes(4);
            let mut ps: Vec<png_bytep> = (0..4).map(|_| row.as_ptr() as png_bytep).collect();
            (api.png_write_image)(png, ps.as_mut_ptr());
        }
        0
    });

    // -- row 984: png_write_end with no IDAT -----------------------------
    wsame!("png_write_end before png_write_info", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_write_end)(png, info);
        }
        0
    });
    wsame!("png_write_end right after png_write_info", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_write_info)(png, info);
            (api.png_write_end)(png, info);
        }
        0
    });
    wsame!("png_write_end(png, NULL info) with no IDAT", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_write_info)(png, info);
            (api.png_write_end)(png, std::ptr::null_mut());
        }
        0
    });

    // -- too few / too many rows -----------------------------------------
    for (h, written) in [
        (4u32, 0u32),
        (4, 1),
        (4, 3),
        (4, 4),
        (4, 5),
        (4, 8),
        (1, 0),
        (1, 2),
    ] {
        wsame!(format!("height {} but {} rows written", h, written), |api, png, info| {
            unsafe {
                (api.png_set_IHDR)(png, info, 4, h, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
                (api.png_write_info)(png, info);
                let row = row_bytes(4);
                for _ in 0..written {
                    (api.png_write_row)(png, row.as_ptr());
                }
                (api.png_write_end)(png, info);
            }
            0
        });
        wsame!(format!("png_write_rows h={} n={}", h, written), |api, png, info| {
            unsafe {
                (api.png_set_IHDR)(png, info, 4, h, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
                (api.png_write_info)(png, info);
                let row = row_bytes(4);
                let mut ps: Vec<png_bytep> =
                    (0..16).map(|_| row.as_ptr() as png_bytep).collect();
                (api.png_write_rows)(png, ps.as_mut_ptr(), written);
                (api.png_write_end)(png, info);
            }
            0
        });
    }

    // NOTE (C UB, dropped): `png_write_image(png, NULL)` and
    // `png_write_rows(png, NULL, n)` with n > 0 both evaluate `*rp` in the
    // caller (pngwrite.c:641 and pngwrite.c:672) *before* png_write_row is
    // entered, i.e. they dereference the NULL row-pointer array.  Likewise
    // `png_write_row(png, NULL)` reaches
    // `memcpy(png_ptr->row_buf + 1, row, row_info.rowbytes)` (pngwrite.c:890)
    // with a NULL source.  All three segfault in BOTH libraries, so they are
    // C undefined behaviour and not error paths.  Only the zero-count case is
    // well defined and is tested here:
    wsame!("png_write_rows(png, NULL, 0)", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_write_info)(png, info);
            (api.png_write_rows)(png, std::ptr::null_mut(), 0);
        }
        0
    });

    // -- rows 1023/1024: png_write_png ----------------------------------
    same!("png_write_png(NULL,NULL)", |api| {
        unsafe {
            (api.png_write_png)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            )
        };
        0
    });
    wsame!("png_write_png(png, NULL info)", |api, png, _i| {
        unsafe {
            (api.png_write_png)(png, std::ptr::null_mut(), 0, std::ptr::null_mut())
        };
        0
    });
    wsame!("png_write_png with no rows set", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_write_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut());
        }
        0
    });
    // png_set_rows + a height that consumes fewer rows than were supplied.
    // (The opposite -- a height larger than the row array -- reads past the
    // end of the application's row-pointer array in both libraries, so it is
    // not a testable error path.)  8 pointers are always supplied.
    for h in [1u32, 2, 4, 7, 8] {
        wsame!(format!("png_set_rows 8 rows, IHDR height {}", h), |api, png, info| {
            unsafe {
                (api.png_set_IHDR)(png, info, 5, h, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
                let row = row_bytes(5);
                let mut ps: Vec<png_bytep> =
                    (0..8).map(|_| row.as_ptr() as png_bytep).collect();
                (api.png_set_rows)(png, info, ps.as_mut_ptr());
                (api.png_write_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut());
            }
            0
        });
    }
    // row 1026: STRIP_FILLER BEFORE+AFTER together
    for m in [
        PNG_TRANSFORM_STRIP_FILLER_BEFORE,
        PNG_TRANSFORM_STRIP_FILLER_AFTER,
        PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER,
        -1,
        0x7fff_ffff,
        i32::MIN,
    ] {
        wsame!(format!("png_write_png transforms {:#x}", m), |api, png, info| {
            unsafe {
                (api.png_set_IHDR)(png, info, 5, 3, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
                let row = row_bytes(5);
                let mut ps: Vec<png_bytep> =
                    (0..3).map(|_| row.as_ptr() as png_bytep).collect();
                (api.png_set_rows)(png, info, ps.as_mut_ptr());
                (api.png_write_png)(png, info, m, std::ptr::null_mut());
            }
            0
        });
    }

    // -- row 979: paletted image without a PLTE --------------------------
    for bd in [1i32, 2, 4, 8] {
        wsame!(format!("palette bd={} without PLTE", bd), |api, png, info| {
            unsafe {
                (api.png_set_IHDR)(
                    png, info, 4, 4, bd, PNG_COLOR_TYPE_PALETTE, IL_NONE, 0, 0,
                );
                (api.png_write_info)(png, info);
            }
            0
        });
    }

    // -- row 977: MNG features in a real PNG datastream ------------------
    for feat in [0u32, PNG_FLAG_MNG_EMPTY_PLTE, PNG_FLAG_MNG_FILTER_64, PNG_ALL_MNG_FEATURES, 0xffff_ffff] {
        wsame!(format!("mng features {:#x} after sig", feat), |api, png, info| {
            unsafe {
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
                (api.png_write_sig)(png);
                (api.png_permit_mng_features)(png, feat);
                (api.png_write_info_before_PLTE)(png, info);
            }
            0
        });
    }
}

#[test]
fn cross_struct_calls() {
    // Write functions on a READ struct: png_write_data finds write_data_fn ==
    // NULL and raises "Call to NULL write function" (pngwio.c:39).
    same!("png_write_info on read struct", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_set_IHDR)(s.png, s.info, 4, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_write_info)(s.png, s.info);
        }
        0
    });
    same!("png_write_sig on read struct", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_write_sig)(s.png);
        }
        0
    });
    same!("png_write_IHDR on read struct", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_write_IHDR)(s.png, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, IL_NONE);
        }
        0
    });
    same!("png_write_IEND on read struct", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_write_IEND)(s.png);
        }
        0
    });
    same!("png_write_end on read struct", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_write_end)(s.png, s.info);
        }
        0
    });
    same!("png_write_flush on read struct", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_write_flush)(s.png);
        }
        0
    });
    same!("png_set_filter on read struct", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_set_filter)(s.png, PNG_FILTER_TYPE_BASE, PNG_ALL_FILTERS);
        }
        0
    });
    same!("png_set_compression_buffer_size on read struct", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_set_compression_buffer_size)(s.png, 1);
            (api.png_set_compression_buffer_size)(s.png, 0);
        }
        0
    });

    // Read functions on a WRITE struct: png_read_data finds read_data_fn ==
    // NULL and raises "Call to NULL read function" (pngrio.c:38).
    wsame!("png_read_info on write struct", |api, png, info| {
        unsafe { (api.png_read_info)(png, info) };
        0
    });
    // NOTE (C UB, dropped): png_read_row / png_read_image on a *write* struct.
    // png_read_row has no "is this a read struct?" guard (pngread.c:288-302);
    // it calls png_read_start_row, which sets
    // `png_ptr->row_buf = png_ptr->big_row_buf + 32` (pngrutil.c).  The
    // subsequent png_destroy_write_struct then png_free()s that interior
    // pointer, which glibc aborts with "free(): invalid pointer".  Confirmed to
    // abort in the C reference, so this is undefined behaviour, not an error
    // path.  Only png_read_info -- which fails cleanly in png_read_data with
    // "Call to NULL read function" (pngrio.c:38) before allocating anything --
    // is a defined rejection and is tested above.
    wsame!("png_read_end-ish: png_read_info twice", |api, png, info| {
        unsafe {
            (api.png_read_info)(png, info);
            (api.png_read_info)(png, info);
        }
        0
    });
    wsame!("png_set_sig_bytes on write struct", |api, png, _i| {
        unsafe {
            (api.png_set_sig_bytes)(png, 4);
            (api.png_write_sig)(png);
        }
        0
    });
}

// ===========================================================================
// 2. png_write_sig / png_write_IHDR / chunk framing
//    rows 1053..1059, 1071..1079
// ===========================================================================

#[test]
fn write_sig_and_ihdr() {
    // png_write_sig twice, and with sig_bytes preset.
    for sb in [0i32, 1, 2, 3, 4, 7, 8] {
        wsame!(format!("png_write_sig twice, sig_bytes={}", sb), |api, png, _i| {
            unsafe {
                (api.png_set_sig_bytes)(png, sb);
                (api.png_write_sig)(png);
                (api.png_write_sig)(png);
            }
            0
        });
    }
    // NOTE (C UB, dropped): png_write_sig(NULL) dereferences
    // `png_ptr->sig_bytes` (pngwutil.c:78) with no NULL guard at all.

    // rows 1071..1076: every (bit_depth, color_type) combination.
    let depths = [-1i32, 0, 1, 2, 3, 4, 5, 7, 8, 9, 16, 17, 32, 255, 256, i32::MAX, i32::MIN];
    let ctypes = [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 100, i32::MAX, i32::MIN];
    for &bd in &depths {
        for &ct in &ctypes {
            wsame!(format!("png_write_IHDR bd={} ct={}", bd, ct), |api, png, _i| {
                unsafe { (api.png_write_IHDR)(png, 4, 4, bd, ct, 0, 0, IL_NONE) };
                0
            });
        }
    }
    // extreme widths / heights (accepted verbatim by png_write_IHDR, which
    // performs no dimension checks at all -- png_check_IHDR does that)
    for &w in &[0u32, 1, 7, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        for &h in &[0u32, 1, 0x7fff_ffff, 0xffff_ffff] {
            wsame!(format!("png_write_IHDR w={} h={}", w, h), |api, png, _i| {
                unsafe { (api.png_write_IHDR)(png, w, h, 8, PNG_COLOR_TYPE_RGB, 0, 0, IL_NONE) };
                0
            });
        }
    }
    // rows 1077/1078/1079: compression / filter / interlace type
    for comp in [-1i32, 0, 1, 2, 64, 255, 256, i32::MAX, i32::MIN] {
        for filt in [-1i32, 0, 1, 64, 65, 255, i32::MAX, i32::MIN] {
            for il in [-1i32, 0, 1, 2, 7, 255, i32::MAX, i32::MIN] {
                wsame!(
                    format!("png_write_IHDR c={} f={} il={}", comp, filt, il),
                    |api, png, _i| {
                        unsafe {
                            (api.png_write_IHDR)(
                                png, 4, 4, 8, PNG_COLOR_TYPE_RGB, comp, filt, il,
                            )
                        };
                        0
                    }
                );
                // ... and with the MNG intrapixel-differencing filter allowed
                wsame!(
                    format!("png_write_IHDR mng c={} f={} il={}", comp, filt, il),
                    |api, png, _i| {
                        unsafe {
                            (api.png_permit_mng_features)(png, PNG_ALL_MNG_FEATURES);
                            (api.png_write_IHDR)(
                                png, 4, 4, 8, PNG_COLOR_TYPE_RGB, comp, filt, il,
                            )
                        };
                        0
                    }
                );
                // MNG filter 64 is only legal for RGB/RGBA and only before the
                // PNG signature was written.
                wsame!(
                    format!("png_write_IHDR mng+sig c={} f={} il={}", comp, filt, il),
                    |api, png, _i| {
                        unsafe {
                            (api.png_permit_mng_features)(png, PNG_ALL_MNG_FEATURES);
                            (api.png_write_sig)(png);
                            (api.png_write_IHDR)(
                                png, 4, 4, 8, PNG_COLOR_TYPE_GRAY, comp, filt, il,
                            )
                        };
                        0
                    }
                );
            }
        }
    }
    // row 1079 for a paletted image (do_filter defaulting differs)
    for ct in [PNG_COLOR_TYPE_PALETTE, PNG_COLOR_TYPE_GRAY] {
        for bd in [1i32, 2, 4, 8] {
            wsame!(format!("png_write_IHDR ct={} bd={} il=9", ct, bd), |api, png, _i| {
                unsafe { (api.png_write_IHDR)(png, 4, 4, bd, ct, 0, 0, 9) };
                0
            });
        }
    }
}

#[test]
fn write_plte_rejections() {
    let pal: Vec<png_color> = (0..300u32)
        .map(|i| png_color {
            red: i as u8,
            green: (i * 3) as u8,
            blue: (i * 7) as u8,
        })
        .collect();
    // rows 1080/1081/1082
    let counts: [u32; 13] = [0, 1, 2, 3, 4, 5, 16, 17, 255, 256, 257, 300, 0xffff_ffff];
    for &n in &counts {
        for &(ct, bd) in &[
            (PNG_COLOR_TYPE_PALETTE, 1i32),
            (PNG_COLOR_TYPE_PALETTE, 2),
            (PNG_COLOR_TYPE_PALETTE, 4),
            (PNG_COLOR_TYPE_PALETTE, 8),
            (PNG_COLOR_TYPE_RGB, 8),
            (PNG_COLOR_TYPE_RGB_ALPHA, 8),
            (PNG_COLOR_TYPE_GRAY, 8),
            (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        ] {
            wsame!(
                format!("png_write_PLTE n={} ct={} bd={}", n, ct, bd),
                |api, png, _i| {
                    unsafe {
                        (api.png_write_IHDR)(png, 4, 4, bd, ct, 0, 0, IL_NONE);
                        (api.png_write_PLTE)(png, pal.as_ptr(), n);
                    }
                    0
                }
            );
            // with the MNG empty-PLTE permission, num_pal == 0 is legal
            wsame!(
                format!("png_write_PLTE mng n={} ct={} bd={}", n, ct, bd),
                |api, png, _i| {
                    unsafe {
                        (api.png_permit_mng_features)(png, PNG_FLAG_MNG_EMPTY_PLTE);
                        (api.png_write_IHDR)(png, 4, 4, bd, ct, 0, 0, IL_NONE);
                        (api.png_write_PLTE)(png, pal.as_ptr(), n);
                    }
                    0
                }
            );
        }
    }
    // PLTE twice
    wsame!("png_write_PLTE twice", |api, png, _i| {
        unsafe {
            (api.png_write_IHDR)(png, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, IL_NONE);
            (api.png_write_PLTE)(png, pal.as_ptr(), 4);
            (api.png_write_PLTE)(png, pal.as_ptr(), 8);
        }
        0
    });
    // PLTE before IHDR: png_ptr->color_type is still 0 (grayscale) so this
    // takes the "Ignoring request ... in grayscale PNG" branch.
    wsame!("png_write_PLTE before IHDR", |api, png, _i| {
        unsafe { (api.png_write_PLTE)(png, pal.as_ptr(), 4) };
        0
    });
    // NOTE (C UB, dropped): png_write_PLTE(png, NULL, n) with n inside the
    // legal range dereferences `pal_ptr->red` (pngwutil.c:898) without a NULL
    // check.  Only out-of-range counts return before the loop, which the
    // `n = 0xffffffff` cases above already cover.
}

#[test]
fn chunk_framing_rejections() {
    let name = *b"teSt";
    let data: Vec<u8> = (0..64u8).collect();
    // rows 1053/1054/1056/1057: NULL png_ptr guards
    same!("png_write_chunk_start(NULL)", |api| {
        unsafe { (api.png_write_chunk_start)(std::ptr::null_mut(), name.as_ptr(), 0) };
        0
    });
    same!("png_write_chunk_data(NULL)", |api| {
        unsafe { (api.png_write_chunk_data)(std::ptr::null_mut(), data.as_ptr(), 4) };
        0
    });
    same!("png_write_chunk_end(NULL)", |api| {
        unsafe { (api.png_write_chunk_end)(std::ptr::null_mut()) };
        0
    });
    same!("png_write_chunk(NULL)", |api| {
        unsafe {
            (api.png_write_chunk)(std::ptr::null_mut(), name.as_ptr(), data.as_ptr(), 4)
        };
        0
    });
    // NOTE (C UB, dropped): png_write_data(NULL, data, len).  png_write_data
    // has no NULL guard at all -- it loads `png_ptr->write_data_fn`
    // (pngwio.c:34) unconditionally.  Confirmed to SIGSEGV in both libraries.
    // NOTE (C UB, dropped): png_write_chunk_start/png_write_chunk with a NULL
    // chunk_string.  `PNG_CHUNK_FROM_STRING(chunk_string)` (pngwutil.c:131 /
    // :215) loads all four name bytes in the *caller*, before the callee's
    // png_ptr NULL check, so a NULL name segfaults in both libraries.

    // bogus / non-alphabetic chunk names -- the write side performs no name
    // validation whatsoever, so these are emitted verbatim.
    for n in [
        *b"teSt", *b"    ", *b"0000", *b"\x00\x00\x00\x00", *b"\xff\xff\xff\xff",
        *b"IHDR", *b"IEND", *b"IDAT", *b"..\x01\x7f",
    ] {
        wsame!(format!("chunk name {:?}", n), |api, png, _i| {
            unsafe {
                (api.png_write_chunk_start)(png, n.as_ptr(), 4);
                (api.png_write_chunk_data)(png, data.as_ptr(), 4);
                (api.png_write_chunk_end)(png);
            }
            0
        });
    }
    // huge / mismatched declared lengths (png_write_chunk_header does not
    // validate the length; only png_write_complete_chunk does -- row 1058)
    for len in [0u32, 1, 4, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        wsame!(format!("chunk_start declared len {:#x}", len), |api, png, _i| {
            unsafe {
                (api.png_write_chunk_start)(png, name.as_ptr(), len);
                (api.png_write_chunk_data)(png, data.as_ptr(), 4);
                (api.png_write_chunk_end)(png);
            }
            0
        });
    }
    // row 1055: NULL data / zero length -> nothing written, CRC untouched
    wsame!("chunk_data NULL / zero length", |api, png, _i| {
        unsafe {
            (api.png_write_chunk_start)(png, name.as_ptr(), 8);
            (api.png_write_chunk_data)(png, std::ptr::null(), 8);
            (api.png_write_chunk_data)(png, data.as_ptr(), 0);
            (api.png_write_chunk_data)(png, std::ptr::null(), 0);
            (api.png_write_chunk_end)(png);
        }
        0
    });
    // row 1056: png_write_chunk_end with no preceding start
    wsame!("chunk_end without start", |api, png, _i| {
        unsafe {
            (api.png_write_chunk_end)(png);
            (api.png_write_chunk_end)(png);
        }
        0
    });
    // png_write_chunk_data without a start
    wsame!("chunk_data without start", |api, png, _i| {
        unsafe { (api.png_write_chunk_data)(png, data.as_ptr(), 4) };
        0
    });
    // row 1058: png_write_complete_chunk length > PNG_UINT_31_MAX
    for len in [0usize, 1, 64, 0x7fff_ffffusize, 0x8000_0000, usize::MAX] {
        wsame!(format!("png_write_chunk total len {:#x}", len), |api, png, _i| {
            unsafe {
                // Only lengths <= data.len() actually copy; the larger ones are
                // rejected before any read (pngwutil.c:199).
                if len <= data.len() || len > PNG_UINT_31_MAX as usize {
                    (api.png_write_chunk)(png, name.as_ptr(), data.as_ptr(), len);
                }
            }
            0
        });
    }
    // row: png_write_IEND twice
    wsame!("png_write_IEND twice", |api, png, _i| {
        unsafe {
            (api.png_write_IEND)(png);
            (api.png_write_IEND)(png);
        }
        0
    });
    // IEND before IHDR
    wsame!("png_write_IEND before IHDR", |api, png, _i| {
        unsafe { (api.png_write_IEND)(png) };
        0
    });
}

// ===========================================================================
// 3. individual chunk writers with out-of-range arguments
//    rows 1086..1121
// ===========================================================================

#[test]
fn chunk_writer_rejections_simple() {
    // gAMA -- no validation at all in png_write_gAMA_fixed (pngwutil.c:1084)
    for g in [i32::MIN, -1, 0, 1, 100_000, 0x7fff_ffff] {
        wsame!(format!("png_write_gAMA_fixed({})", g), |api, png, _i| {
            unsafe { (api.png_write_gAMA_fixed)(png, g) };
            0
        });
    }
    // cHRM -- likewise unvalidated
    for v in [i32::MIN, -1, 0, 1, 100_000, i32::MAX] {
        wsame!(format!("png_write_cHRM_fixed({})", v), |api, png, _i| {
            let xy = png_xy {
                redx: v,
                redy: v,
                greenx: v,
                greeny: v,
                bluex: v,
                bluey: v,
                whitex: v,
                whitey: v,
            };
            unsafe { (api.png_write_cHRM_fixed)(png, &xy) };
            0
        });
    }
    // NOTE (C UB, dropped): png_write_cHRM_fixed(png, NULL) loads `xy->whitex`
    // (pngwutil.c:1300) with no NULL check.

    // row 1086: sRGB rendering intent
    for i in [i32::MIN, -100, -1, 0, 1, 2, 3, 4, 5, 99, 255, 256, i32::MAX] {
        wsame!(format!("png_write_sRGB({})", i), |api, png, _i| {
            unsafe { (api.png_write_sRGB)(png, i) };
            0
        });
    }
    // row 1116: oFFs unit type
    for u in [i32::MIN, -1, 0, 1, 2, 3, 99, i32::MAX] {
        wsame!(format!("png_write_oFFs(unit={})", u), |api, png, _i| {
            unsafe { (api.png_write_oFFs)(png, i32::MIN, i32::MAX, u) };
            0
        });
    }
    // row 1120: pHYs unit type
    for u in [i32::MIN, -1, 0, 1, 2, 3, 99, i32::MAX] {
        wsame!(format!("png_write_pHYs(unit={})", u), |api, png, _i| {
            unsafe { (api.png_write_pHYs)(png, 0, 0xffff_ffff, u) };
            0
        });
    }
    // cICP / cLLI / mDCV -- no validation in the writers
    for v in [0u8, 1, 2, 9, 255] {
        wsame!(format!("png_write_cICP({})", v), |api, png, _i| {
            unsafe { (api.png_write_cICP)(png, v, v, v, v) };
            0
        });
    }
    for (a, b) in [(0u32, 0u32), (1, 1), (0xffff_ffff, 0), (0, 0xffff_ffff), (0xffff_ffff, 0xffff_ffff)] {
        wsame!(format!("png_write_cLLI_fixed({},{})", a, b), |api, png, _i| {
            unsafe { (api.png_write_cLLI_fixed)(png, a, b) };
            0
        });
        wsame!(format!("png_write_mDCV_fixed(...,{},{})", a, b), |api, png, _i| {
            unsafe {
                (api.png_write_mDCV_fixed)(
                    png, 0, 0xffff, 1, 0xfffe, 2, 3, 0x8000, 0x7fff, a, b,
                )
            };
            0
        });
    }
    // row 1105: hIST entry count
    let hist: Vec<u16> = (0..300u32).map(|i| (i * 257) as u16).collect();
    for n in [i32::MIN, -1, 0, 1, 2, 3, 4, 5, 255, 256, 257, i32::MAX] {
        for pal_n in [0u32, 2, 4, 256] {
            wsame!(format!("png_write_hIST(n={}, pal={})", n, pal_n), |api, png, _i| {
                unsafe {
                    (api.png_write_IHDR)(png, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, IL_NONE);
                    if pal_n > 0 {
                        let pal: Vec<png_color> = vec![png_color::default(); 300];
                        (api.png_write_PLTE)(png, pal.as_ptr(), pal_n);
                    }
                    // only ever hand over as many entries as we own
                    if n <= hist.len() as c_int {
                        (api.png_write_hIST)(png, hist.as_ptr(), n);
                    }
                }
                0
            });
        }
    }
    // row 1119: sCAL_s buffer too small
    for (wl, hl) in [
        (0usize, 0usize),
        (1, 1),
        (30, 30),
        (31, 31),
        (32, 30),
        (31, 32),
        (62, 0),
        (63, 0),
        (0, 62),
        (0, 63),
        (100, 100),
    ] {
        let ws = cs(&"1".repeat(wl));
        let hs = cs(&"2".repeat(hl));
        for unit in [i32::MIN, -1, 0, 1, 2, 3, 99, i32::MAX] {
            wsame!(
                format!("png_write_sCAL_s(unit={}, {}+{})", unit, wl, hl),
                |api, png, _i| {
                    unsafe { (api.png_write_sCAL_s)(png, unit, ws.as_ptr(), hs.as_ptr()) };
                    0
                }
            );
        }
    }
    // NOTE (C UB, dropped): png_write_sCAL_s with a NULL width or height calls
    // strlen(NULL) (pngwutil.c:1852-1853).

    // row 1121: tIME field ranges
    let times: [(u16, u8, u8, u8, u8, u8); 18] = [
        (0, 0, 0, 0, 0, 0),
        (1970, 1, 1, 0, 0, 0),
        (1970, 12, 31, 23, 59, 60),
        (1970, 13, 1, 0, 0, 0),
        (1970, 0, 1, 0, 0, 0),
        (1970, 255, 1, 0, 0, 0),
        (1970, 1, 0, 0, 0, 0),
        (1970, 1, 32, 0, 0, 0),
        (1970, 1, 255, 0, 0, 0),
        (1970, 1, 1, 24, 0, 0),
        (1970, 1, 1, 255, 0, 0),
        (1970, 1, 1, 0, 60, 0),
        (1970, 1, 1, 0, 255, 0),
        (1970, 1, 1, 0, 0, 61),
        (1970, 1, 1, 0, 0, 255),
        (0xffff, 12, 31, 23, 59, 60),
        (0xffff, 6, 15, 12, 30, 30),
        (2024, 2, 30, 25, 61, 61),
    ];
    for &(y, mo, d, h, mi, s) in &times {
        wsame!(
            format!("png_write_tIME({},{},{},{},{},{})", y, mo, d, h, mi, s),
            |api, png, _i| {
                let t = png_time {
                    year: y,
                    month: mo,
                    day: d,
                    hour: h,
                    minute: mi,
                    second: s,
                };
                unsafe { (api.png_write_tIME)(png, &t) };
                0
            }
        );
    }
    // NOTE (C UB, dropped): png_write_tIME(png, NULL) loads `mod_time->month`
    // (pngwutil.c:1908) with no NULL check.

    // eXIf: negative / zero counts are well defined (the copy loop simply does
    // not run); a positive count with a NULL buffer is not.
    let exif: Vec<u8> = b"II\x2a\x00abcdefgh".to_vec();
    for n in [i32::MIN, -1, 0, 1, 4, 12, i32::MAX] {
        wsame!(format!("png_write_eXIf(n={})", n), |api, png, _i| {
            unsafe {
                if n <= exif.len() as c_int {
                    (api.png_write_eXIf)(png, exif.as_ptr() as png_bytep, n);
                }
            }
            0
        });
    }
    wsame!("png_write_eXIf(NULL, 0)", |api, png, _i| {
        unsafe { (api.png_write_eXIf)(png, std::ptr::null_mut(), 0) };
        0
    });
}

#[test]
fn chunk_writer_rejections_sbit_trns_bkgd() {
    // rows 1095/1096/1097: sBIT
    let sbits: [png_color_8; 10] = [
        png_color_8 { red: 0, green: 0, blue: 0, gray: 0, alpha: 0 },
        png_color_8 { red: 1, green: 1, blue: 1, gray: 1, alpha: 1 },
        png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 },
        png_color_8 { red: 9, green: 8, blue: 8, gray: 8, alpha: 8 },
        png_color_8 { red: 8, green: 0, blue: 8, gray: 8, alpha: 8 },
        png_color_8 { red: 8, green: 8, blue: 255, gray: 8, alpha: 8 },
        png_color_8 { red: 8, green: 8, blue: 8, gray: 0, alpha: 8 },
        png_color_8 { red: 8, green: 8, blue: 8, gray: 17, alpha: 8 },
        png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 0 },
        png_color_8 { red: 16, green: 16, blue: 16, gray: 16, alpha: 255 },
    ];
    for (i, sb) in sbits.iter().enumerate() {
        for &(ct, bd) in &[
            (PNG_COLOR_TYPE_GRAY, 1i32),
            (PNG_COLOR_TYPE_GRAY, 8),
            (PNG_COLOR_TYPE_GRAY, 16),
            (PNG_COLOR_TYPE_RGB, 8),
            (PNG_COLOR_TYPE_RGB, 16),
            (PNG_COLOR_TYPE_PALETTE, 4),
            (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
            (PNG_COLOR_TYPE_RGB_ALPHA, 16),
        ] {
            // ...and pass a *different* color_type argument as well, since the
            // writer takes it as a parameter separate from png_ptr.
            for &arg_ct in &[ct, -1, 0, 3, 6, 7, 100] {
                wsame!(
                    format!("png_write_sBIT #{} ct={} bd={} arg={}", i, ct, bd, arg_ct),
                    |api, png, _i| {
                        unsafe {
                            (api.png_write_IHDR)(png, 4, 4, bd, ct, 0, 0, IL_NONE);
                            (api.png_write_sBIT)(png, sb, arg_ct);
                        }
                        0
                    }
                );
            }
        }
    }
    // NOTE (C UB, dropped): png_write_sBIT(png, NULL, ct) loads `sbit->red`
    // (pngwutil.c:1250) with no NULL check.

    // rows 1098..1101: tRNS
    let alpha: Vec<u8> = (0..300u32).map(|i| i as u8).collect();
    let tr16 = png_color_16 { index: 3, red: 0x1234, green: 0x5678, blue: 0x9abc, gray: 0xdef0 };
    let tr8 = png_color_16 { index: 3, red: 0x00ff, green: 0x0080, blue: 0x0001, gray: 0x000f };
    let tr0 = png_color_16::default();
    for (tn, tr) in [("16bit", &tr16), ("8bit", &tr8), ("zero", &tr0)] {
        for nt in [i32::MIN, -1, 0, 1, 2, 4, 5, 255, 256, 257, i32::MAX] {
            for &(ct, bd, pal_n) in &[
                (PNG_COLOR_TYPE_PALETTE, 4i32, 4u32),
                (PNG_COLOR_TYPE_PALETTE, 8, 256),
                (PNG_COLOR_TYPE_PALETTE, 1, 0),
                (PNG_COLOR_TYPE_GRAY, 1, 0),
                (PNG_COLOR_TYPE_GRAY, 8, 0),
                (PNG_COLOR_TYPE_GRAY, 16, 0),
                (PNG_COLOR_TYPE_RGB, 8, 0),
                (PNG_COLOR_TYPE_RGB, 16, 0),
                (PNG_COLOR_TYPE_GRAY_ALPHA, 8, 0),
                (PNG_COLOR_TYPE_RGB_ALPHA, 8, 0),
            ] {
                wsame!(
                    format!("png_write_tRNS {} nt={} ct={} bd={} pal={}", tn, nt, ct, bd, pal_n),
                    |api, png, _i| {
                        unsafe {
                            (api.png_write_IHDR)(png, 4, 4, bd, ct, 0, 0, IL_NONE);
                            if pal_n > 0 {
                                let pal: Vec<png_color> = vec![png_color::default(); 300];
                                (api.png_write_PLTE)(png, pal.as_ptr(), pal_n);
                            }
                            if nt <= alpha.len() as c_int {
                                (api.png_write_tRNS)(png, alpha.as_ptr(), tr, nt, ct);
                            }
                        }
                        0
                    }
                );
            }
        }
    }

    // rows 1102/1103/1104: bKGD
    let bks: [png_color_16; 6] = [
        png_color_16 { index: 0, red: 0, green: 0, blue: 0, gray: 0 },
        png_color_16 { index: 3, red: 0, green: 0, blue: 0, gray: 1 },
        png_color_16 { index: 4, red: 0, green: 0, blue: 0, gray: 2 },
        png_color_16 { index: 255, red: 0x0100, green: 0, blue: 0, gray: 0x0100 },
        png_color_16 { index: 1, red: 0x00ff, green: 0x00ff, blue: 0x00ff, gray: 0x00ff },
        png_color_16 { index: 2, red: 0xffff, green: 0xffff, blue: 0xffff, gray: 0xffff },
    ];
    for (i, bk) in bks.iter().enumerate() {
        for &(ct, bd, pal_n) in &[
            (PNG_COLOR_TYPE_PALETTE, 4i32, 4u32),
            (PNG_COLOR_TYPE_PALETTE, 8, 256),
            (PNG_COLOR_TYPE_PALETTE, 2, 0),
            (PNG_COLOR_TYPE_GRAY, 1, 0),
            (PNG_COLOR_TYPE_GRAY, 8, 0),
            (PNG_COLOR_TYPE_GRAY, 16, 0),
            (PNG_COLOR_TYPE_RGB, 8, 0),
            (PNG_COLOR_TYPE_RGB, 16, 0),
            (PNG_COLOR_TYPE_RGB_ALPHA, 8, 0),
        ] {
            for &arg_ct in &[ct, -1, 3, 100] {
                wsame!(
                    format!("png_write_bKGD #{} ct={} bd={} pal={} arg={}", i, ct, bd, pal_n, arg_ct),
                    |api, png, _i| {
                        unsafe {
                            (api.png_write_IHDR)(png, 4, 4, bd, ct, 0, 0, IL_NONE);
                            if pal_n > 0 {
                                let pal: Vec<png_color> = vec![png_color::default(); 300];
                                (api.png_write_PLTE)(png, pal.as_ptr(), pal_n);
                            }
                            (api.png_write_bKGD)(png, bk, arg_ct);
                        }
                        0
                    }
                );
            }
            // ...and with the MNG empty-PLTE permission, which relaxes the
            // palette-index check (pngwutil.c:1394)
            wsame!(
                format!("png_write_bKGD mng #{} ct={} bd={} pal={}", i, ct, bd, pal_n),
                |api, png, _i| {
                    unsafe {
                        (api.png_permit_mng_features)(png, PNG_FLAG_MNG_EMPTY_PLTE);
                        (api.png_write_IHDR)(png, 4, 4, bd, ct, 0, 0, IL_NONE);
                        (api.png_write_bKGD)(png, bk, ct);
                    }
                    0
                }
            );
        }
    }
    // NOTE (C UB, dropped): png_write_bKGD(png, NULL, ct) and
    // png_write_tRNS(png, alpha, NULL, n, GRAY/RGB) dereference `back`/`tran`
    // (pngwutil.c:1400, 1343) with no NULL check.
}

#[test]
fn chunk_writer_rejections_pcal_splt() {
    // rows 1117/1118: pCAL
    let purposes = ["", " ", "ok", &"p".repeat(80), "bad\tchar", " lead", "trail "];
    for p in purposes {
        let pu = cs(p);
        let un = cs("units");
        for typ in [i32::MIN, -1, 0, 1, 2, 3, 4, 5, 99, i32::MAX] {
            for np in [0i32, 1, 2, 3, 4] {
                let ps: Vec<CString> = (0..np).map(|i| cs(&format!("param{}", i))).collect();
                wsame!(
                    format!("png_write_pCAL purpose={:?} type={} np={}", p, typ, np),
                    |api, png, _i| {
                        let mut pp: Vec<png_charp> =
                            ps.iter().map(|c| c.as_ptr() as png_charp).collect();
                        unsafe {
                            (api.png_write_pCAL)(
                                png,
                                pu.as_ptr() as png_charp,
                                -1,
                                1,
                                typ,
                                np,
                                un.as_ptr(),
                                if pp.is_empty() {
                                    std::ptr::null_mut()
                                } else {
                                    pp.as_mut_ptr()
                                },
                            )
                        };
                        0
                    }
                );
            }
        }
    }
    // a negative nparams makes the params_len allocation overflow
    // (pngwutil.c:1811) -> png_malloc fails -> png_error("Out of memory")
    for np in [-1i32, -2, i32::MIN] {
        let pu = cs("ok");
        let un = cs("u");
        wsame!(format!("png_write_pCAL nparams={}", np), |api, png, _i| {
            unsafe {
                (api.png_write_pCAL)(
                    png,
                    pu.as_ptr() as png_charp,
                    0,
                    1,
                    0,
                    np,
                    un.as_ptr(),
                    std::ptr::null_mut(),
                )
            };
            0
        });
    }
    // NOTE (C UB, dropped): png_write_pCAL with a NULL `units` calls
    // strlen(units) (pngwutil.c:1806); with a NULL `params` and nparams > 0 it
    // calls strlen(params[i]) (pngwutil.c:1818).  Both segfault in C.

    // row 1094: sPLT keyword
    let entries: Vec<png_sPLT_entry> = (0..8u32)
        .map(|i| png_sPLT_entry {
            red: i as u16,
            green: (i * 3) as u16,
            blue: (i * 5) as u16,
            alpha: (i * 7) as u16,
            frequency: (i * 11) as u16,
        })
        .collect();
    for nm in ["", " ", "  ", "spl", &"n".repeat(80), "bad\x01", " x ", "a  b"] {
        let n = cs(nm);
        for depth in [0u8, 1, 4, 8, 16, 255] {
            for ne in [0i32, 1, 8, -1] {
                wsame!(
                    format!("png_write_sPLT name={:?} depth={} n={}", nm, depth, ne),
                    |api, png, _i| {
                        let sp = png_sPLT_t {
                            name: n.as_ptr() as png_charp,
                            depth,
                            entries: entries.as_ptr() as png_sPLT_entryp,
                            nentries: ne,
                        };
                        unsafe { (api.png_write_sPLT)(png, &sp) };
                        0
                    }
                );
            }
        }
    }
    // a NULL sPLT name is *not* UB: png_check_keyword handles key == NULL
    // (pngset.c:1992) and the writer then png_errors on the empty keyword.
    wsame!("png_write_sPLT NULL name", |api, png, _i| {
        let sp = png_sPLT_t {
            name: std::ptr::null_mut(),
            depth: 8,
            entries: entries.as_ptr() as png_sPLT_entryp,
            nentries: 4,
        };
        unsafe { (api.png_write_sPLT)(png, &sp) };
        0
    });
    // NOTE (C UB, dropped): png_write_sPLT(png, NULL) loads `spalette->depth`
    // (pngwutil.c:1180) with no NULL check.
}

// ===========================================================================
// 4. compression parameter rejections
//    rows 1005..1020 + png_set_compression_buffer_size
// ===========================================================================

#[test]
fn compression_parameter_rejections() {
    // rows 1005..1020: NULL png_ptr guards
    same!("compression setters on NULL", |api| {
        unsafe {
            let n: png_structp = std::ptr::null_mut();
            (api.png_set_compression_level)(n, 5);
            (api.png_set_compression_mem_level)(n, 8);
            (api.png_set_compression_strategy)(n, 0);
            (api.png_set_compression_window_bits)(n, 15);
            (api.png_set_compression_method)(n, 8);
            (api.png_set_compression_buffer_size)(n, 8192);
            (api.png_set_text_compression_level)(n, 5);
            (api.png_set_text_compression_mem_level)(n, 8);
            (api.png_set_text_compression_strategy)(n, 0);
            (api.png_set_text_compression_window_bits)(n, 15);
            (api.png_set_text_compression_method)(n, 8);
        }
        0
    });

    let bad = [i32::MIN, -100, -2, -1, 0, 1, 2, 7, 8, 9, 10, 15, 16, 999, i32::MAX];
    for &v in &bad {
        // level: -1..9 legal; anything else makes deflateInit2 fail inside
        // png_deflate_claim (row 1064) -> png_error(zstream.msg)
        wsame!(format!("compression_level {}", v), |api, png, info| {
            unsafe {
                (api.png_set_compression_level)(png, v);
                write_small(api, png, info, 8, 4);
            }
            0
        });
        wsame!(format!("compression_mem_level {}", v), |api, png, info| {
            unsafe {
                (api.png_set_compression_mem_level)(png, v);
                write_small(api, png, info, 8, 4);
            }
            0
        });
        wsame!(format!("compression_strategy {}", v), |api, png, info| {
            unsafe {
                (api.png_set_compression_strategy)(png, v);
                write_small(api, png, info, 8, 4);
            }
            0
        });
        // window_bits: clamped with a warning (rows 1009/1010)
        wsame!(format!("compression_window_bits {}", v), |api, png, info| {
            unsafe {
                (api.png_set_compression_window_bits)(png, v);
                write_small(api, png, info, 8, 4);
            }
            0
        });
        // method: warning for != 8 (row 1012), then deflateInit2 fails
        wsame!(format!("compression_method {}", v), |api, png, info| {
            unsafe {
                (api.png_set_compression_method)(png, v);
                write_small(api, png, info, 8, 4);
            }
            0
        });
        // the text twins (rows 1013..1020); exercised through a zTXt chunk
        let key = cs("Comment");
        let txt = cs("some text that is long enough to compress a little bit");
        wsame!(format!("text_compression_level {}", v), |api, png, info| {
            unsafe {
                (api.png_set_text_compression_level)(png, v);
                write_ztxt(api, png, info, &key, &txt);
            }
            0
        });
        wsame!(format!("text_compression_mem_level {}", v), |api, png, info| {
            unsafe {
                (api.png_set_text_compression_mem_level)(png, v);
                write_ztxt(api, png, info, &key, &txt);
            }
            0
        });
        wsame!(format!("text_compression_strategy {}", v), |api, png, info| {
            unsafe {
                (api.png_set_text_compression_strategy)(png, v);
                write_ztxt(api, png, info, &key, &txt);
            }
            0
        });
        wsame!(format!("text_compression_window_bits {}", v), |api, png, info| {
            unsafe {
                (api.png_set_text_compression_window_bits)(png, v);
                write_ztxt(api, png, info, &key, &txt);
            }
            0
        });
        wsame!(format!("text_compression_method {}", v), |api, png, info| {
            unsafe {
                (api.png_set_text_compression_method)(png, v);
                write_ztxt(api, png, info, &key, &txt);
            }
            0
        });
    }

    // png_set_compression_buffer_size: 0 and > PNG_UINT_31_MAX are hard
    // errors; 1..5 warn and are ignored (pngset.c).
    //
    // NOTE: the "Compression buffer size limited to system maximum" branch
    // (size > ZLIB_IO_MAX == 0xffffffff) is UNREACHABLE, because the
    // `size > PNG_UINT_31_MAX` png_error above it fires first.
    for sz in [
        0usize,
        1,
        2,
        5,
        6,
        7,
        64,
        8192,
        PNG_UINT_31_MAX as usize,
        PNG_UINT_31_MAX as usize + 1,
        1usize << 32,
        usize::MAX,
    ] {
        wsame!(format!("compression_buffer_size {}", sz), |api, png, info| {
            unsafe {
                (api.png_set_compression_buffer_size)(png, sz);
                write_small(api, png, info, 8, 4);
            }
            0
        });
        // ...and after writing has started (zowner != 0) -> "Compression
        // buffer size cannot be changed because it is in use"
        wsame!(format!("compression_buffer_size {} mid-write", sz), |api, png, info| {
            unsafe {
                (api.png_set_IHDR)(png, info, 8, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
                (api.png_write_info)(png, info);
                let row = row_bytes(8);
                (api.png_write_row)(png, row.as_ptr());
                (api.png_set_compression_buffer_size)(png, sz);
                for _ in 1..4 {
                    (api.png_write_row)(png, row.as_ptr());
                }
                (api.png_write_end)(png, info);
            }
            0
        });
    }
}

unsafe fn write_ztxt(
    api: &'static Api,
    png: png_structp,
    info: png_infop,
    key: &CString,
    txt: &CString,
) {
    (api.png_set_IHDR)(png, info, 4, 2, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
    let t = png_text {
        compression: PNG_TEXT_COMPRESSION_zTXt,
        key: key.as_ptr() as png_charp,
        text: txt.as_ptr() as png_charp,
        text_length: 0,
        itxt_length: 0,
        lang: std::ptr::null_mut(),
        lang_key: std::ptr::null_mut(),
    };
    (api.png_set_text)(png, info, &t, 1);
    (api.png_write_info)(png, info);
    let row = row_bytes(4);
    for _ in 0..2 {
        (api.png_write_row)(png, row.as_ptr());
    }
    (api.png_write_end)(png, info);
}

// ===========================================================================
// 5. filter rejections
//    rows 1000..1004, 1122, 1123
// ===========================================================================

#[test]
fn filter_rejections() {
    // row 1000: NULL png_ptr
    same!("png_set_filter(NULL)", |api| {
        unsafe { (api.png_set_filter)(std::ptr::null_mut(), 0, PNG_ALL_FILTERS) };
        0
    });
    // row 1004: method != PNG_FILTER_TYPE_BASE
    for m in [i32::MIN, -100, -1, 1, 2, 63, 64, 65, 999, i32::MAX] {
        wsame!(format!("png_set_filter method {}", m), |api, png, _i| {
            unsafe { (api.png_set_filter)(png, m, PNG_ALL_FILTERS) };
            0
        });
        // ...method 64 is legalised by PNG_FLAG_MNG_FILTER_64
        wsame!(format!("png_set_filter mng method {}", m), |api, png, _i| {
            unsafe {
                (api.png_permit_mng_features)(png, PNG_ALL_MNG_FEATURES);
                (api.png_set_filter)(png, m, PNG_ALL_FILTERS);
            }
            0
        });
    }
    // row 1001: invalid mask bits.  `filters & (PNG_ALL_FILTERS|0x07)` == 5, 6
    // or 7 is png_app_error("Unknown row filter for method 0") which, on a
    // write struct in this build, is a hard png_error.
    let masks = [
        i32::MIN, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0x0f, 0x10, 0x18, 0x1f, 0x20, 0x40,
        0x80, 0xf8, 0xff, 0x100, 0x107, 999, i32::MAX,
    ];
    for &f in &masks {
        wsame!(format!("png_set_filter mask {:#x}", f), |api, png, _i| {
            unsafe { (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, f) };
            0
        });
        // ...and then actually write, so the row 1122 restrictions in
        // png_write_start_row are exercised too.
        for &(w, h) in &[(1u32, 1u32), (1, 4), (4, 1), (6, 5)] {
            wsame!(
                format!("png_set_filter mask {:#x} then write {}x{}", f, w, h),
                |api, png, info| {
                    unsafe {
                        (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, f);
                        write_small(api, png, info, w, h);
                    }
                    0
                }
            );
        }
    }
    // palette / low bit depth images (png_write_IHDR forces PNG_FILTER_NONE
    // when nothing was selected, pngwutil.c:838)
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_PALETTE, 1i32),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 2),
        (PNG_COLOR_TYPE_GRAY, 4),
    ] {
        for &f in &[
            PNG_NO_FILTERS,
            PNG_FILTER_NONE,
            PNG_FILTER_SUB,
            PNG_FILTER_UP,
            PNG_FILTER_AVG,
            PNG_FILTER_PAETH,
            PNG_ALL_FILTERS,
            0x05,
            0xff,
            -1,
        ] {
            for before in [true, false] {
                wsame!(
                    format!("filter {:#x} ct={} bd={} before_info={}", f, ct, bd, before),
                    |api, png, info| {
                        unsafe {
                            (api.png_set_IHDR)(png, info, 5, 3, bd, ct, IL_NONE, 0, 0);
                            let pal: Vec<png_color> = vec![png_color::default(); 300];
                            if ct == PNG_COLOR_TYPE_PALETTE {
                                (api.png_set_PLTE)(png, info, pal.as_ptr(), 1 << bd);
                            }
                            if before {
                                (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, f);
                            }
                            (api.png_write_info)(png, info);
                            if !before {
                                (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, f);
                            }
                            let row = row_bytes(5);
                            for _ in 0..3 {
                                (api.png_write_row)(png, row.as_ptr());
                            }
                            (api.png_write_end)(png, info);
                        }
                        0
                    }
                );
            }
        }
    }
    // row 1003: UP/AVG/PAETH added after the first row when prev_row was never
    // allocated -> png_app_warning (a hard error in this build).
    for &f in &[
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_FILTER_SUB,
        PNG_FILTER_NONE,
        PNG_ALL_FILTERS,
    ] {
        wsame!(format!("add filter {:#x} after start", f), |api, png, info| {
            unsafe {
                (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, PNG_FILTER_NONE);
                (api.png_set_IHDR)(png, info, 6, 4, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
                (api.png_write_info)(png, info);
                let row = row_bytes(6);
                (api.png_write_row)(png, row.as_ptr());
                (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, f);
                for _ in 1..4 {
                    (api.png_write_row)(png, row.as_ptr());
                }
                (api.png_write_end)(png, info);
            }
            0
        });
    }

    // png_set_filter_heuristics / _fixed are deprecated no-ops in 1.6
    // (pngwrite.c:1185-1211), so every invalid argument is silently accepted.
    for m in [i32::MIN, -1, 0, 1, 2, 3, 4, 999, i32::MAX] {
        for nw in [i32::MIN, -1, 0, 1, 5, 999, i32::MAX] {
            wsame!(format!("filter_heuristics m={} nw={}", m, nw), |api, png, _i| {
                let w = [1.0f64, 2.0, 3.0, 4.0, 5.0];
                let c = [1.0f64, 2.0, 3.0, 4.0, 5.0];
                unsafe {
                    (api.png_set_filter_heuristics)(png, m, nw, w.as_ptr(), c.as_ptr());
                    (api.png_set_filter_heuristics)(
                        png,
                        m,
                        nw,
                        std::ptr::null(),
                        std::ptr::null(),
                    );
                    (api.png_set_filter_heuristics)(png, m, nw, w.as_ptr(), std::ptr::null());
                    (api.png_set_filter_heuristics)(png, m, nw, std::ptr::null(), c.as_ptr());
                }
                0
            });
            wsame!(format!("filter_heuristics_fixed m={} nw={}", m, nw), |api, png, _i| {
                let w = [100000i32, 200000, 300000, 400000, 500000];
                let c = [100000i32, 200000, 300000, 400000, 500000];
                unsafe {
                    (api.png_set_filter_heuristics_fixed)(png, m, nw, w.as_ptr(), c.as_ptr());
                    (api.png_set_filter_heuristics_fixed)(
                        png,
                        m,
                        nw,
                        std::ptr::null(),
                        std::ptr::null(),
                    );
                }
                0
            });
        }
    }
    same!("filter_heuristics on NULL png_ptr", |api| {
        unsafe {
            (api.png_set_filter_heuristics)(
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null(),
                std::ptr::null(),
            );
            (api.png_set_filter_heuristics_fixed)(
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null(),
                std::ptr::null(),
            );
        }
        0
    });
}

// ===========================================================================
// 6. text rejections
//    rows 1106..1115 + png_set_text / png_set_text_2 + png_check_keyword
// ===========================================================================

/// Every invalid keyword form accepted by `png_check_keyword` (pngset.c:1981).
fn bad_keywords() -> Vec<(&'static str, String)> {
    vec![
        ("empty", String::new()),
        ("one space", " ".to_string()),
        ("two spaces", "  ".to_string()),
        ("many spaces", " ".repeat(90)),
        ("ok", "Title".to_string()),
        ("79 chars", "k".repeat(79)),
        ("80 chars", "k".repeat(80)),
        ("81 chars", "k".repeat(81)),
        ("200 chars", "k".repeat(200)),
        ("leading space", " Title".to_string()),
        ("trailing space", "Title ".to_string()),
        ("both", " Title ".to_string()),
        ("consecutive", "Ti  tle".to_string()),
        ("triple", "Ti   tle".to_string()),
        ("tab", "Ti\ttle".to_string()),
        ("ctrl 0x01", "Ti\u{1}tle".to_string()),
        ("ctrl 0x1f", "Ti\u{1f}tle".to_string()),
        ("del 0x7f", "Ti\u{7f}tle".to_string()),
        ("0x80", "Ti\u{80}tle".to_string()),
        ("0xa0", "Ti\u{a0}tle".to_string()),
        ("0xa1 valid", "Ti\u{a1}tle".to_string()),
        ("0xff valid", "Ti\u{ff}tle".to_string()),
        ("only ctrl", "\u{1}\u{2}\u{3}".to_string()),
        ("space then ctrl", " \u{1} ".to_string()),
        ("79 then space", format!("{} ", "k".repeat(79))),
        ("space then 79", format!(" {}", "k".repeat(79))),
    ]
}

/// The keyword bytes as libpng sees them (latin-1, not UTF-8).
fn latin1(s: &str) -> CString {
    CString::new(s.chars().map(|c| c as u8).collect::<Vec<u8>>()).unwrap()
}

#[test]
fn check_keyword_rejections() {
    for (nm, k) in bad_keywords() {
        let ck = latin1(&k);
        // png_check_keyword called directly.  The 80-byte `new_key` output
        // buffer is over-allocated to 256 bytes so a mis-translation that
        // writes too much is detected rather than corrupting the test process.
        wsame!(format!("png_check_keyword {:?}", nm), |api, png, _i| {
            let mut nk = [0u8; 256];
            let n = unsafe { (api.png_check_keyword)(png, ck.as_ptr(), nk.as_mut_ptr()) };
            let end = nk.iter().position(|&c| c == 0).unwrap_or(nk.len());
            (n as i64) * 1_000_000 + fnv(&nk[..end]) % 1_000_000
        });
    }
    // key == NULL is explicitly handled (pngset.c:1992): *new_key = 0, return 0
    wsame!("png_check_keyword(NULL key)", |api, png, _i| {
        let mut nk = [0xffu8; 256];
        let n = unsafe { (api.png_check_keyword)(png, std::ptr::null(), nk.as_mut_ptr()) };
        (n as i64) * 1000 + nk[0] as i64
    });
    // NOTE (C UB, dropped): png_check_keyword(png, NULL, NULL) writes through
    // `*new_key` (pngset.c:1994) before any check, so a NULL output buffer is
    // undefined behaviour, not an error path.
}

#[test]
fn text_rejections() {
    // -- png_set_text / png_set_text_2 argument validation ----------------
    same!("png_set_text_2 NULL args", |api| {
        unsafe {
            let s = WriteSess::new(api);
            let a = (api.png_set_text_2)(std::ptr::null(), s.info, std::ptr::null(), 1);
            let b = (api.png_set_text_2)(s.png, std::ptr::null_mut(), std::ptr::null(), 1);
            let c = (api.png_set_text_2)(s.png, s.info, std::ptr::null(), 1);
            (a as i64) * 100 + (b as i64) * 10 + c as i64
        }
    });
    for n in [i32::MIN, -1, 0, 1, i32::MAX] {
        let key = cs("Title");
        let txt = cs("text");
        wsame!(format!("png_set_text num_text={}", n), |api, png, info| {
            let t = png_text {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: key.as_ptr() as png_charp,
                text: txt.as_ptr() as png_charp,
                text_length: 0,
                itxt_length: 0,
                lang: std::ptr::null_mut(),
                lang_key: std::ptr::null_mut(),
            };
            // Only n <= 1 is safe: a larger count would read png_text entries
            // we do not own.
            let r = unsafe {
                if n <= 1 {
                    (api.png_set_text_2)(png, info, &t, n)
                } else {
                    0
                }
            };
            r as i64
        });
    }

    // -- every compression value ----------------------------------------
    let comps = [
        i32::MIN,
        -100,
        -5,
        -4,
        PNG_TEXT_COMPRESSION_NONE_WR, // -3
        PNG_TEXT_COMPRESSION_zTXt_WR, // -2
        PNG_TEXT_COMPRESSION_NONE,    // -1
        PNG_TEXT_COMPRESSION_zTXt,    // 0
        PNG_ITXT_COMPRESSION_NONE,    // 1
        PNG_ITXT_COMPRESSION_zTXt,    // 2
        PNG_TEXT_COMPRESSION_LAST,    // 3
        4,
        99,
        i32::MAX,
    ];
    let key = cs("Title");
    let txt = cs("The quick brown fox jumps over the lazy dog, repeatedly.");
    let lang = cs("en-GB");
    let lkey = cs("Titel");
    for &c in &comps {
        for with_lang in [false, true] {
            for null_text in [false, true] {
                wsame!(
                    format!("png_set_text comp={} lang={} nulltext={}", c, with_lang, null_text),
                    |api, png, info| {
                        unsafe {
                            (api.png_set_IHDR)(
                                png, info, 4, 2, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0,
                            );
                            let t = png_text {
                                compression: c,
                                key: key.as_ptr() as png_charp,
                                text: if null_text {
                                    std::ptr::null_mut()
                                } else {
                                    txt.as_ptr() as png_charp
                                },
                                text_length: 0,
                                itxt_length: 0,
                                lang: if with_lang {
                                    lang.as_ptr() as png_charp
                                } else {
                                    std::ptr::null_mut()
                                },
                                lang_key: if with_lang {
                                    lkey.as_ptr() as png_charp
                                } else {
                                    std::ptr::null_mut()
                                },
                            };
                            let r = (api.png_set_text_2)(png, info, &t, 1);
                            (api.png_write_info)(png, info);
                            let row = row_bytes(4);
                            for _ in 0..2 {
                                (api.png_write_row)(png, row.as_ptr());
                            }
                            (api.png_write_end)(png, info);
                            r as i64
                        }
                    }
                );
            }
        }
    }
    // key == NULL entries are skipped (pngset.c: `continue`)
    wsame!("png_set_text NULL key", |api, png, info| {
        let t = png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: std::ptr::null_mut(),
            text: txt.as_ptr() as png_charp,
            text_length: 0,
            itxt_length: 0,
            lang: std::ptr::null_mut(),
            lang_key: std::ptr::null_mut(),
        };
        unsafe { (api.png_set_text_2)(png, info, &t, 1) as i64 }
    });

    // -- bad keywords through the whole write pipeline --------------------
    for (nm, k) in bad_keywords() {
        let ck = latin1(&k);
        for &c in &[
            PNG_TEXT_COMPRESSION_NONE,
            PNG_TEXT_COMPRESSION_zTXt,
            PNG_ITXT_COMPRESSION_NONE,
            PNG_ITXT_COMPRESSION_zTXt,
        ] {
            wsame!(format!("write text key {:?} comp={}", nm, c), |api, png, info| {
                unsafe {
                    (api.png_set_IHDR)(png, info, 4, 2, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
                    let t = png_text {
                        compression: c,
                        key: ck.as_ptr() as png_charp,
                        text: txt.as_ptr() as png_charp,
                        text_length: 0,
                        itxt_length: 0,
                        lang: lang.as_ptr() as png_charp,
                        lang_key: lkey.as_ptr() as png_charp,
                    };
                    (api.png_set_text_2)(png, info, &t, 1);
                    (api.png_write_info)(png, info);
                    let row = row_bytes(4);
                    for _ in 0..2 {
                        (api.png_write_row)(png, row.as_ptr());
                    }
                    (api.png_write_end)(png, info);
                }
                0
            });
        }
        // ... and directly through each chunk writer (rows 1106, 1109, 1111)
        wsame!(format!("png_write_tEXt key {:?}", nm), |api, png, _i| {
            unsafe { (api.png_write_tEXt)(png, ck.as_ptr(), txt.as_ptr(), 0) };
            0
        });
        wsame!(format!("png_write_zTXt key {:?}", nm), |api, png, _i| {
            unsafe {
                (api.png_write_zTXt)(png, ck.as_ptr(), txt.as_ptr(), PNG_TEXT_COMPRESSION_zTXt)
            };
            0
        });
        wsame!(format!("png_write_iTXt key {:?}", nm), |api, png, _i| {
            unsafe {
                (api.png_write_iTXt)(
                    png,
                    PNG_ITXT_COMPRESSION_NONE,
                    ck.as_ptr(),
                    lang.as_ptr(),
                    lkey.as_ptr(),
                    txt.as_ptr(),
                )
            };
            0
        });
    }
    // NULL keys reach png_check_keyword's own NULL handling
    wsame!("png_write_tEXt NULL key", |api, png, _i| {
        unsafe { (api.png_write_tEXt)(png, std::ptr::null(), txt.as_ptr(), 0) };
        0
    });
    wsame!("png_write_zTXt NULL key", |api, png, _i| {
        unsafe { (api.png_write_zTXt)(png, std::ptr::null(), txt.as_ptr(), 0) };
        0
    });
    wsame!("png_write_iTXt NULL key", |api, png, _i| {
        unsafe {
            (api.png_write_iTXt)(
                png,
                1,
                std::ptr::null(),
                lang.as_ptr(),
                lkey.as_ptr(),
                txt.as_ptr(),
            )
        };
        0
    });
    // NULL / empty text is explicitly handled (pngwutil.c:1582, 1641, 1706)
    wsame!("png_write_tEXt NULL text", |api, png, _i| {
        unsafe { (api.png_write_tEXt)(png, key.as_ptr(), std::ptr::null(), 12345) };
        0
    });
    wsame!("png_write_zTXt NULL text", |api, png, _i| {
        unsafe { (api.png_write_zTXt)(png, key.as_ptr(), std::ptr::null(), 0) };
        0
    });
    wsame!("png_write_iTXt NULL text/lang/lang_key", |api, png, _i| {
        unsafe {
            (api.png_write_iTXt)(
                png,
                1,
                key.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        0
    });
    // the `text_len` argument of png_write_tEXt is ignored -- strlen wins
    for tl in [0usize, 1, 5, usize::MAX] {
        wsame!(format!("png_write_tEXt text_len={}", tl), |api, png, _i| {
            unsafe { (api.png_write_tEXt)(png, key.as_ptr(), txt.as_ptr(), tl) };
            0
        });
    }
    // NOTE (unreachable, dropped): row 1107 ("tEXt: text too long") requires
    // strlen(text) > PNG_UINT_31_MAX - (key_len+1), i.e. a >2GB NUL-terminated
    // string; likewise row 1115 ("iTXt: uncompressed text too long") and row
    // 1113 (prefix_len saturation).  Not constructible in a test.

    // rows 1108/1112: invalid compression values in the chunk writers
    for &c in &comps {
        wsame!(format!("png_write_zTXt comp={}", c), |api, png, _i| {
            unsafe { (api.png_write_zTXt)(png, key.as_ptr(), txt.as_ptr(), c) };
            0
        });
        wsame!(format!("png_write_iTXt comp={}", c), |api, png, _i| {
            unsafe {
                (api.png_write_iTXt)(
                    png,
                    c,
                    key.as_ptr(),
                    lang.as_ptr(),
                    lkey.as_ptr(),
                    txt.as_ptr(),
                )
            };
            0
        });
        // an invalid keyword is rejected *before* the compression value
        // (pngwutil.c:1673 vs :1679)
        let empty = cs("");
        wsame!(format!("png_write_iTXt comp={} bad key", c), |api, png, _i| {
            unsafe {
                (api.png_write_iTXt)(
                    png,
                    c,
                    empty.as_ptr(),
                    lang.as_ptr(),
                    lkey.as_ptr(),
                    txt.as_ptr(),
                )
            };
            0
        });
    }
}

// ===========================================================================
// 7. ICC profile rejections
//    rows 956..974 + rows 1087..1093
// ===========================================================================

/// Run the three exported ICC validators on `prof`/`len` and fold their
/// results into a single comparable integer.  A *read* struct is used because
/// it has `PNG_FLAG_BENIGN_ERRORS_WARN` set (pngread.c:62), so
/// `png_icc_profile_error` becomes a warning and the `0`/`1` return value of
/// each checker is observable.
fn icc_case(label: String, prof: &[u8], len: u32, color_type: c_int, skip_srgb: bool) {
    let name = cs("test profile");
    same!(label, |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            if skip_srgb {
                (api.png_set_option)(s.png, PNG_SKIP_sRGB_CHECK_PROFILE, PNG_OPTION_ON);
            }
            let a = (api.png_icc_check_length)(s.png, name.as_ptr(), len);
            let b = (api.png_icc_check_header)(
                s.png,
                name.as_ptr(),
                len,
                prof.as_ptr(),
                color_type,
            );
            let c = (api.png_icc_check_tag_table)(s.png, name.as_ptr(), len, prof.as_ptr());
            (a as i64) * 100 + (b as i64) * 10 + c as i64
        }
    });
}

#[test]
fn icc_check_length_rejections() {
    // rows 956/957: profile_length < 132 and > png_chunk_max
    // (PNG_USER_CHUNK_MALLOC_MAX == 8000000 in this build).
    let name = cs("p");
    for len in [
        0u32,
        1,
        4,
        11,
        128,
        131,
        132,
        133,
        134,
        135,
        136,
        1000,
        7_999_999,
        8_000_000,
        8_000_001,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ] {
        same!(format!("png_icc_check_length({}) read", len), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                (api.png_icc_check_length)(s.png, name.as_ptr(), len) as i64
            }
        });
        // ...on a write struct benign errors are hard errors
        same!(format!("png_icc_check_length({}) write", len), |api| {
            unsafe {
                let s = WriteSess::new(api);
                (api.png_icc_check_length)(s.png, name.as_ptr(), len) as i64
            }
        });
        // ...and with an application-lowered chunk limit
        for max in [0usize, 1, 132, 1000, 8_000_000] {
            same!(
                format!("png_icc_check_length({}) max={}", len, max),
                |api| unsafe {
                    let s = ReadSess::new(api, &[]);
                    (api.png_set_chunk_malloc_max)(s.png, max);
                    (api.png_icc_check_length)(s.png, name.as_ptr(), len) as i64
                }
            );
        }
    }
    // NOTE (C UB, dropped): a NULL `name` reaches
    // `png_safecat(message, pos+79, pos, name)` inside png_icc_profile_error
    // (png.c:1546) which walks the string, so NULL is undefined behaviour on
    // every failing path.
}

#[test]
fn icc_check_header_rejections() {
    // the reference: a well-formed minimal profile must be ACCEPTED by both
    icc_case(
        "icc good gray on gray".into(),
        &icc_good_gray(),
        132,
        PNG_COLOR_TYPE_GRAY,
        false,
    );
    icc_case(
        "icc good gray on gray+alpha".into(),
        &icc_good_gray(),
        132,
        PNG_COLOR_TYPE_GRAY_ALPHA,
        false,
    );
    icc_case(
        "icc good rgb on rgb".into(),
        &icc_good_rgb(),
        132,
        PNG_COLOR_TYPE_RGB,
        false,
    );
    icc_case(
        "icc good rgb on palette".into(),
        &icc_good_rgb(),
        132,
        PNG_COLOR_TYPE_PALETTE,
        false,
    );
    icc_case(
        "icc good rgb on rgba".into(),
        &icc_good_rgb(),
        132,
        PNG_COLOR_TYPE_RGB_ALPHA,
        false,
    );
    // row 965/966: colour space vs PNG colour type mismatch
    icc_case(
        "icc RGB on gray".into(),
        &icc_good_rgb(),
        132,
        PNG_COLOR_TYPE_GRAY,
        false,
    );
    icc_case(
        "icc GRAY on rgb".into(),
        &icc_good_gray(),
        132,
        PNG_COLOR_TYPE_RGB,
        false,
    );
    // row 958: declared length != passed length
    for dl in [0u32, 1, 131, 133, 136, 0xffff_ffff] {
        let p = icc(dl, 2, b"mntr", b"GRAY", b"XYZ ", b"acsp", 0, true, &[]);
        icc_case(
            format!("icc declared {} vs 132", dl),
            &p,
            132,
            PNG_COLOR_TYPE_GRAY,
            false,
        );
    }
    // row 959: major version > 3 with a length that is not a multiple of 4
    for (vmaj, len) in [
        (0u8, 132u32),
        (2, 132),
        (3, 133),
        (4, 132),
        (4, 133),
        (4, 134),
        (4, 135),
        (4, 136),
        (5, 133),
        (255, 133),
    ] {
        let mut p = icc(len, vmaj, b"mntr", b"GRAY", b"XYZ ", b"acsp", 0, true, &[]);
        p.resize(136.max(p.len()), 0);
        icc_case(
            format!("icc version {} len {}", vmaj, len),
            &p,
            len,
            PNG_COLOR_TYPE_GRAY,
            false,
        );
    }
    // row 960: tag count too large / truncated tag table
    for (tc, len) in [
        (0u32, 132u32),
        (1, 132),
        (1, 143),
        (1, 144),
        (2, 155),
        (2, 156),
        (357_913_930, 132),
        (357_913_931, 132),
        (0xffff_ffff, 132),
    ] {
        // build a *header only* buffer (132 bytes) but declare `tc` tags
        let mut p = icc(len, 2, b"mntr", b"GRAY", b"XYZ ", b"acsp", 0, true, &[]);
        p[128..132].copy_from_slice(&be32(tc));
        // png_icc_check_tag_table would read 12*tc bytes, so grow the buffer
        // whenever the declared count is small enough to be traversed.
        if tc <= 4 {
            p.resize(132 + 12 * tc as usize, 0);
            for i in 0..tc as usize {
                let o = 132 + 12 * i;
                p[o..o + 4].copy_from_slice(b"desc");
                p[o + 4..o + 8].copy_from_slice(&be32(132 + 12 * tc));
                p[o + 8..o + 12].copy_from_slice(&be32(0));
            }
        }
        if tc <= 4 {
            icc_case(
                format!("icc tag_count {} len {}", tc, len),
                &p,
                len,
                PNG_COLOR_TYPE_GRAY,
                false,
            );
        } else {
            // only the header check is safe to run: the tag-table walker would
            // read 12*tag_count bytes past our buffer.
            let name = cs("p");
            same!(format!("icc header only tag_count {}", tc), |api| {
                unsafe {
                    let s = ReadSess::new(api, &[]);
                    (api.png_icc_check_header)(
                        s.png,
                        name.as_ptr(),
                        len,
                        p.as_ptr(),
                        PNG_COLOR_TYPE_GRAY,
                    ) as i64
                }
            });
        }
    }
    // rows 961/962: rendering intent
    for intent in [0u32, 1, 2, 3, 4, 5, 100, 0xfffe, 0xffff, 0x1_0000, 0xffff_ffff] {
        let p = icc(132, 2, b"mntr", b"GRAY", b"XYZ ", b"acsp", intent, true, &[]);
        icc_case(
            format!("icc intent {}", intent),
            &p,
            132,
            PNG_COLOR_TYPE_GRAY,
            false,
        );
    }
    // row 963: bad file signature
    for sig in [b"acsp", b"ACSP", b"XXXX", b"\x00\x00\x00\x00", b"acs\x00"] {
        let p = icc(132, 2, b"mntr", b"GRAY", b"XYZ ", sig, 0, true, &[]);
        icc_case(
            format!("icc signature {:?}", sig),
            &p,
            132,
            PNG_COLOR_TYPE_GRAY,
            false,
        );
    }
    // row 964: PCS illuminant not D50 (warning only)
    let p = icc(132, 2, b"mntr", b"GRAY", b"XYZ ", b"acsp", 0, false, &[]);
    icc_case("icc PCS illuminant not D50".into(), &p, 132, PNG_COLOR_TYPE_GRAY, false);
    // rows 965..967: data colour space
    for space in [b"RGB ", b"GRAY", b"CMYK", b"XYZ ", b"\x00\x00\x00\x00", b"rgb "] {
        for ct in [PNG_COLOR_TYPE_GRAY, PNG_COLOR_TYPE_RGB, PNG_COLOR_TYPE_PALETTE] {
            let p = icc(132, 2, b"mntr", space, b"XYZ ", b"acsp", 0, true, &[]);
            icc_case(
                format!("icc space {:?} ct={}", space, ct),
                &p,
                132,
                ct,
                false,
            );
        }
    }
    // rows 968..971: profile / device class
    for class in [
        b"scnr", b"mntr", b"prtr", b"spac", b"abst", b"link", b"nmcl", b"zzzz",
        b"\x00\x00\x00\x00",
    ] {
        let p = icc(132, 2, class, b"GRAY", b"XYZ ", b"acsp", 0, true, &[]);
        icc_case(
            format!("icc class {:?}", class),
            &p,
            132,
            PNG_COLOR_TYPE_GRAY,
            false,
        );
    }
    // row 972: PCS encoding
    for pcs in [b"XYZ ", b"Lab ", b"CMYK", b"xyz ", b"\x00\x00\x00\x00"] {
        let p = icc(132, 2, b"mntr", b"GRAY", pcs, b"acsp", 0, true, &[]);
        icc_case(
            format!("icc pcs {:?}", pcs),
            &p,
            132,
            PNG_COLOR_TYPE_GRAY,
            false,
        );
    }
    // ...and every check again with PNG_SKIP_sRGB_CHECK_PROFILE turned on
    // (row: the option only affects png_colorspace_set_ICC, so the checkers
    // themselves must behave identically).
    icc_case(
        "icc good gray, skip sRGB check".into(),
        &icc_good_gray(),
        132,
        PNG_COLOR_TYPE_GRAY,
        true,
    );
    let bad = icc(132, 2, b"abst", b"CMYK", b"CMYK", b"XXXX", 0xffff, false, &[]);
    icc_case("icc all bad, skip sRGB check".into(), &bad, 132, PNG_COLOR_TYPE_GRAY, true);
    icc_case("icc all bad".into(), &bad, 132, PNG_COLOR_TYPE_GRAY, false);
    for v in [PNG_OPTION_OFF, PNG_OPTION_ON, 0, 1, -1, 99] {
        let name = cs("p");
        let g = icc_good_gray();
        same!(format!("png_set_option(SKIP_sRGB,{}) + icc", v), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                let o = (api.png_set_option)(s.png, PNG_SKIP_sRGB_CHECK_PROFILE, v);
                let r = (api.png_icc_check_header)(
                    s.png,
                    name.as_ptr(),
                    132,
                    g.as_ptr(),
                    PNG_COLOR_TYPE_GRAY,
                );
                (o as i64) * 10 + r as i64
            }
        });
    }
}

#[test]
fn icc_check_tag_table_rejections() {
    let name = cs("p");
    // rows 973/974: tag outside the profile, tag start not 4-byte aligned
    let cases: [(u32, u32, u32); 14] = [
        // (profile_length, tag_start, tag_length)
        (144, 144, 0),
        (144, 132, 12),
        (144, 132, 13),   // length runs past the end
        (144, 145, 0),    // start past the end
        (144, 0xffff_ffff, 0),
        (144, 0, 0xffff_ffff),
        (144, 144, 1),
        (144, 133, 0),    // unaligned (warning only)
        (144, 134, 0),
        (144, 135, 0),
        (144, 136, 4),
        (144, 0, 144),
        (144, 0, 145),
        (144, 1, 143),
    ];
    for (i, &(plen, ts, tl)) in cases.iter().enumerate() {
        let p = icc(
            plen,
            2,
            b"mntr",
            b"GRAY",
            b"XYZ ",
            b"acsp",
            0,
            true,
            &[(u32::from_be_bytes(*b"desc"), ts, tl)],
        );
        same!(format!("icc tag #{} start={} len={}", i, ts, tl), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                let a = (api.png_icc_check_header)(
                    s.png,
                    name.as_ptr(),
                    plen,
                    p.as_ptr(),
                    PNG_COLOR_TYPE_GRAY,
                );
                let b =
                    (api.png_icc_check_tag_table)(s.png, name.as_ptr(), plen, p.as_ptr());
                (a as i64) * 10 + b as i64
            }
        });
        // on a write struct these are hard errors
        same!(format!("icc tag #{} on write struct", i), |api| {
            unsafe {
                let s = WriteSess::new(api);
                (api.png_icc_check_tag_table)(s.png, name.as_ptr(), plen, p.as_ptr()) as i64
            }
        });
    }
    // several tags, some good some bad -- one warning per offending tag
    let tags = [
        (u32::from_be_bytes(*b"desc"), 168u32, 0u32),
        (u32::from_be_bytes(*b"wtpt"), 169, 0),
        (u32::from_be_bytes(*b"rXYZ"), 170, 0),
        (u32::from_be_bytes(*b"gXYZ"), 400, 0),
    ];
    let p = icc(180, 2, b"mntr", b"GRAY", b"XYZ ", b"acsp", 0, true, &tags);
    same!("icc four tags mixed", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_icc_check_tag_table)(s.png, name.as_ptr(), 180, p.as_ptr()) as i64
        }
    });
    // zero tags -> accepted with no diagnostics
    let p0 = icc_good_gray();
    same!("icc zero tags", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_icc_check_tag_table)(s.png, name.as_ptr(), 132, p0.as_ptr()) as i64
        }
    });
    // NOTE (C UB, dropped): a NULL `profile` reaches
    // `png_get_uint_32(profile+128)` (png.c:1802) with no NULL check.
}

#[test]
fn write_iccp_rejections() {
    // rows 1087..1093, reached through png_set_iCCP + png_write_info as well as
    // by calling png_write_iCCP directly.
    let good = icc_good_rgb();
    let name = cs("ICC profile");
    // row 1087: NULL profile
    wsame!("png_write_iCCP NULL profile", |api, png, _i| {
        unsafe { (api.png_write_iCCP)(png, name.as_ptr(), std::ptr::null(), 132) };
        0
    });
    // rows 1088/1089/1090/1091: length checks
    for (dl, pl) in [
        (0u32, 0u32),
        (0, 132),
        (131, 131),
        (132, 131),
        (132, 132),
        (132, 133),
        (133, 133),
        (134, 134),
        (135, 135),
        (136, 136),
        (0xffff_ffff, 132),
    ] {
        for vmaj in [2u8, 4] {
            let mut p = icc(dl, vmaj, b"mntr", b"RGB ", b"XYZ ", b"acsp", 0, true, &[]);
            p.resize(160, 0);
            wsame!(
                format!("png_write_iCCP declared={} len={} v={}", dl, pl, vmaj),
                |api, png, _i| {
                    unsafe {
                        (api.png_write_IHDR)(png, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, IL_NONE);
                        (api.png_write_iCCP)(png, name.as_ptr(), p.as_ptr(), pl);
                    }
                    0
                }
            );
        }
    }
    // row 1092: keyword rejected
    for (nm, k) in bad_keywords() {
        let ck = latin1(&k);
        wsame!(format!("png_write_iCCP keyword {:?}", nm), |api, png, _i| {
            unsafe {
                (api.png_write_IHDR)(png, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, IL_NONE);
                (api.png_write_iCCP)(png, ck.as_ptr(), good.as_ptr(), 132);
            }
            0
        });
    }
    wsame!("png_write_iCCP NULL keyword", |api, png, _i| {
        unsafe {
            (api.png_write_IHDR)(png, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, IL_NONE);
            (api.png_write_iCCP)(png, std::ptr::null(), good.as_ptr(), 132);
        }
        0
    });
    // ...and the same profiles through png_set_iCCP + png_write_info
    for (lbl, prof, len) in [
        ("good", icc_good_rgb(), 132u32),
        ("truncated", icc_good_rgb()[..100].to_vec(), 100),
        ("declared mismatch", icc(999, 2, b"mntr", b"RGB ", b"XYZ ", b"acsp", 0, true, &[]), 132),
        ("v4 unaligned", {
            let mut p = icc(133, 4, b"mntr", b"RGB ", b"XYZ ", b"acsp", 0, true, &[]);
            p.resize(133, 0);
            p
        }, 133),
        ("empty", Vec::new(), 0),
    ] {
        for comp in [0i32, -1, 1, 99] {
            for key in ["ICC", "", " "] {
                let ck = cs(key);
                wsame!(
                    format!("png_set_iCCP {} comp={} key={:?}", lbl, comp, key),
                    |api, png, info| {
                        unsafe {
                            (api.png_set_IHDR)(
                                png, info, 4, 2, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0,
                            );
                            (api.png_set_iCCP)(
                                png,
                                info,
                                ck.as_ptr(),
                                comp,
                                if prof.is_empty() {
                                    good.as_ptr()
                                } else {
                                    prof.as_ptr()
                                },
                                len,
                            );
                            (api.png_write_info)(png, info);
                            let row = row_bytes(4);
                            for _ in 0..2 {
                                (api.png_write_row)(png, row.as_ptr());
                            }
                            (api.png_write_end)(png, info);
                        }
                        0
                    }
                );
            }
        }
    }
    // png_set_iCCP NULL guards (pngset.c: silent early return)
    wsame!("png_set_iCCP NULL name/profile", |api, png, info| {
        unsafe {
            (api.png_set_iCCP)(std::ptr::null(), info, name.as_ptr(), 0, good.as_ptr(), 132);
            (api.png_set_iCCP)(
                png,
                std::ptr::null_mut(),
                name.as_ptr(),
                0,
                good.as_ptr(),
                132,
            );
            (api.png_set_iCCP)(png, info, std::ptr::null(), 0, good.as_ptr(), 132);
            (api.png_set_iCCP)(png, info, name.as_ptr(), 0, std::ptr::null(), 132);
        }
        0
    });
}

// ===========================================================================
// 8. simplified write API failures
//    rows 1027..1052
// ===========================================================================

fn tmp_path(tag: &str) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push(format!("t13_{}.png", tag));
    p
}

#[test]
fn simplified_write_to_memory_rejections() {
    // row 1040: image == NULL
    same!("png_image_write_to_memory(NULL image)", |api| {
        let mut mb: png_alloc_size_t = 0;
        unsafe {
            (api.png_image_write_to_memory)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut mb,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
            ) as i64
        }
    });
    // row 1039: version mismatch
    for v in [0u32, 1, 2, 99, 0xffff_ffff] {
        let buf = vec![0x40u8; 4 * 4 * 3 + 32];
        let run = |api: &'static Api| -> SP {
            let mut img = png_image {
                version: v,
                width: 4,
                height: 4,
                format: PNG_FORMAT_RGB,
                ..Default::default()
            };
            let p: *const png_image = &img;
            sprobe(api, p, |api| {
                let mut mb: png_alloc_size_t = 0;
                let r = unsafe {
                    (api.png_image_write_to_memory)(
                        &mut img,
                        std::ptr::null_mut(),
                        &mut mb,
                        0,
                        buf.as_ptr() as *const c_void,
                        0,
                        std::ptr::null(),
                    )
                };
                (r as i64, mb as i64)
            })
        };
        let c = run(c_api());
        let r = run(rs_api());
        assert_eq!(c, r, "png_image_write_to_memory version {}", v);
    }

    // rows 1038/1042/1037/1036 and the png_image_write_main checks.
    // (width == 0 is NOT tested: pngwrite.c:2044 evaluates
    // `image->height > 0xffffffffU/png_row_stride` with png_row_stride == 0,
    // i.e. an integer division by zero -> SIGFPE in BOTH libraries.  That is C
    // undefined behaviour, not an error path.)
    struct Case {
        lbl: &'static str,
        w: u32,
        h: u32,
        fmt: png_uint_32,
        cmap_entries: u32,
        with_cmap: bool,
        stride: i32,
        conv8: c_int,
        with_memory: bool,
        mem_cap: usize,
        null_bytes: bool,
        null_buffer: bool,
    }
    let base = Case {
        lbl: "",
        w: 4,
        h: 3,
        fmt: PNG_FORMAT_RGB,
        cmap_entries: 0,
        with_cmap: false,
        stride: 0,
        conv8: 0,
        with_memory: true,
        mem_cap: 4096,
        null_bytes: false,
        null_buffer: false,
    };
    let cases = vec![
        Case { lbl: "ok rgb", ..base },
        Case { lbl: "null memory_bytes", null_bytes: true, ..base },
        Case { lbl: "null buffer", null_buffer: true, ..base },
        Case { lbl: "count only (memory NULL)", with_memory: false, ..base },
        Case { lbl: "buffer 1 byte", mem_cap: 1, ..base },
        Case { lbl: "buffer 32 bytes", mem_cap: 32, ..base },
        Case { lbl: "buffer 0 bytes", mem_cap: 0, ..base },
        Case { lbl: "height 0", h: 0, ..base },
        Case { lbl: "format 0x40", fmt: 0x40, ..base },
        Case { lbl: "format 0x50", fmt: 0x50, ..base },
        Case { lbl: "format 0xff", fmt: 0xff, ..base },
        Case { lbl: "format 0xffffffff", fmt: 0xffff_ffff, ..base },
        Case { lbl: "format GA", fmt: PNG_FORMAT_GA, ..base },
        Case { lbl: "format LINEAR_RGB", fmt: PNG_FORMAT_LINEAR_RGB, mem_cap: 8192, ..base },
        Case { lbl: "format LINEAR_RGB_ALPHA", fmt: PNG_FORMAT_LINEAR_RGB_ALPHA, mem_cap: 8192, ..base },
        Case { lbl: "colormap without map", fmt: PNG_FORMAT_RGB_COLORMAP, ..base },
        Case { lbl: "colormap 0 entries", fmt: PNG_FORMAT_RGB_COLORMAP, with_cmap: true, cmap_entries: 0, ..base },
        Case { lbl: "colormap 4 entries", fmt: PNG_FORMAT_RGB_COLORMAP, with_cmap: true, cmap_entries: 4, ..base },
        Case { lbl: "colormap 257 entries", fmt: PNG_FORMAT_RGB_COLORMAP, with_cmap: true, cmap_entries: 257, ..base },
        Case { lbl: "colormap 1000 entries", fmt: PNG_FORMAT_RGB_COLORMAP, with_cmap: true, cmap_entries: 1000, ..base },
        Case { lbl: "stride too small", stride: 1, ..base },
        Case { lbl: "stride exact", stride: 12, ..base },
        Case { lbl: "stride negative exact", stride: -12, ..base },
        Case { lbl: "stride negative too small", stride: -1, ..base },
        // NOTE (C UB, dropped): row_stride == i32::MAX / i32::MIN.  Both pass
        // the `check >= png_row_stride` and `height > 0xffffffffU/stride`
        // guards (pngwrite.c:2038-2049) and then walk the application buffer in
        // 2GB steps (`row += row_step`, pngwrite.c:2166/2216), reading far
        // outside it.  That is an application-contract violation (the buffer
        // must be `height * |row_stride|` bytes), so both libraries segfault.
        Case { lbl: "row stride too large", w: 0x4000_0000, ..base },
        Case { lbl: "memory image too large", w: 0x1000_0000, h: 0x1000, ..base },
        Case { lbl: "convert_to_8bit -1", conv8: -1, ..base },
        Case { lbl: "convert_to_8bit 2", conv8: 2, ..base },
        Case { lbl: "convert_to_8bit 999", conv8: 999, ..base },
        Case { lbl: "convert_to_8bit i32::MIN", conv8: i32::MIN, ..base },
        Case { lbl: "linear + convert 2", fmt: PNG_FORMAT_LINEAR_Y, conv8: 2, mem_cap: 8192, ..base },
        Case { lbl: "linear + convert 0", fmt: PNG_FORMAT_LINEAR_Y, conv8: 0, mem_cap: 8192, ..base },
    ];
    for cse in &cases {
        if std::env::var_os("PNGTRACE").is_some() {
            eprintln!("TRACE to_memory {}", cse.lbl);
        }
        // A buffer large enough for any of the *valid* small cases (16-bit
        // linear RGBA at 4x3 = 96 bytes); over-allocated so nothing reads out
        // of bounds.
        let buf = vec![0x37u8; 4096];
        let cmap = vec![0x21u8; 4 * 1024];
        let run = |api: &'static Api| -> SP {
            let mut img = png_image {
                version: PNG_IMAGE_VERSION,
                width: cse.w,
                height: cse.h,
                format: cse.fmt,
                colormap_entries: cse.cmap_entries,
                ..Default::default()
            };
            let mut mem = vec![0u8; cse.mem_cap.max(1)];
            let mut mb: png_alloc_size_t = cse.mem_cap;
            let ip: *const png_image = &img;
            let out = sprobe(api, ip, |api| {
                let r = unsafe {
                    (api.png_image_write_to_memory)(
                        &mut img,
                        if cse.with_memory {
                            mem.as_mut_ptr() as *mut c_void
                        } else {
                            std::ptr::null_mut()
                        },
                        if cse.null_bytes {
                            std::ptr::null_mut()
                        } else {
                            &mut mb
                        },
                        cse.conv8,
                        if cse.null_buffer {
                            std::ptr::null()
                        } else {
                            buf.as_ptr() as *const c_void
                        },
                        cse.stride,
                        if cse.with_cmap {
                            cmap.as_ptr() as *const c_void
                        } else {
                            std::ptr::null()
                        },
                    )
                };
                (r as i64, (mb as i64) * 1_000_000_007 + fnv(&mem))
            });
            // colormap_entries may be rewritten by png_image_set_PLTE (row 1035)
            SP {
                extra: out.extra * 1000 + img.colormap_entries as i64 % 1000,
                ..out
            }
        };
        let c = run(c_api());
        let r = run(rs_api());
        if std::env::var_os("PNGDUMP").is_some() {
            eprintln!("DUMP to_memory {} => {:?}", cse.lbl, c);
        }
        assert_eq!(c, r, "png_image_write_to_memory: {}", cse.lbl);
    }
}

#[test]
fn simplified_write_to_file_and_stdio_rejections() {
    let buf = vec![0x5cu8; 4096];
    // rows 1045/1052: image == NULL
    same!("png_image_write_to_file(NULL image)", |api| {
        let n = cs("/dev/null");
        unsafe {
            (api.png_image_write_to_file)(
                std::ptr::null_mut(),
                n.as_ptr(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
            ) as i64
        }
    });
    same!("png_image_write_to_stdio(NULL image)", |api| {
        unsafe {
            (api.png_image_write_to_stdio)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
            ) as i64
        }
    });

    // rows 1043/1044/1047/1048/1051
    struct FC {
        lbl: &'static str,
        version: u32,
        w: u32,
        h: u32,
        fmt: png_uint_32,
        name: Option<String>,
        null_buffer: bool,
        conv8: c_int,
        stride: i32,
    }
    let cases = vec![
        FC { lbl: "ok", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some(tmp_path("ok").display().to_string()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "bad version 0", version: 0, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some(tmp_path("v0").display().to_string()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "bad version 2", version: 2, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some(tmp_path("v2").display().to_string()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "bad version max", version: 0xffff_ffff, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some(tmp_path("vm").display().to_string()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "NULL file name", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: None, null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "NULL buffer", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some(tmp_path("nb").display().to_string()), null_buffer: true, conv8: 0, stride: 0 },
        FC { lbl: "unopenable path", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some("/t13-no-such-dir/x.png".to_string()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "empty path", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some(String::new()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "directory path", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some("/tmp".to_string()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "height 0", version: 1, w: 4, h: 0, fmt: PNG_FORMAT_RGB, name: Some(tmp_path("h0").display().to_string()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "bad format", version: 1, w: 4, h: 3, fmt: 0x40, name: Some(tmp_path("bf").display().to_string()), null_buffer: false, conv8: 0, stride: 0 },
        FC { lbl: "stride too small", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_RGB, name: Some(tmp_path("st").display().to_string()), null_buffer: false, conv8: 0, stride: 2 },
        FC { lbl: "convert 2", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_LINEAR_Y, name: Some(tmp_path("c2").display().to_string()), null_buffer: false, conv8: 2, stride: 0 },
        FC { lbl: "colormap no map", version: 1, w: 4, h: 3, fmt: PNG_FORMAT_RGB_COLORMAP, name: Some(tmp_path("cm").display().to_string()), null_buffer: false, conv8: 0, stride: 0 },
    ];
    for fc in &cases {
        let cname = fc.name.as_ref().map(|s| cs(s));
        let run = |api: &'static Api| -> SP {
            let mut img = png_image {
                version: fc.version,
                width: fc.w,
                height: fc.h,
                format: fc.fmt,
                ..Default::default()
            };
            let ip: *const png_image = &img;
            let out = sprobe(api, ip, |api| {
                let r = unsafe {
                    (api.png_image_write_to_file)(
                        &mut img,
                        match &cname {
                            Some(c) => c.as_ptr(),
                            None => std::ptr::null(),
                        },
                        fc.conv8,
                        if fc.null_buffer {
                            std::ptr::null()
                        } else {
                            buf.as_ptr() as *const c_void
                        },
                        fc.stride,
                        std::ptr::null(),
                    )
                };
                (r as i64, 0)
            });
            out
        };
        let c = run(c_api());
        let r = run(rs_api());
        if std::env::var_os("PNGDUMP").is_some() {
            eprintln!("DUMP to_file {} => {:?}", fc.lbl, c);
        }
        assert_eq!(c, r, "png_image_write_to_file: {}", fc.lbl);
        if let Some(n) = &fc.name {
            let _ = std::fs::remove_file(n);
        }

        // The same argument checks for png_image_write_to_stdio, with a NULL
        // FILE* (rows 1043/1044).  A real FILE* cannot be produced without
        // libc bindings, and the successful path is already covered by
        // png_image_write_to_file above, which calls _to_stdio internally.
        let run2 = |api: &'static Api| -> SP {
            let mut img = png_image {
                version: fc.version,
                width: fc.w,
                height: fc.h,
                format: fc.fmt,
                ..Default::default()
            };
            let ip: *const png_image = &img;
            sprobe(api, ip, |api| {
                let r = unsafe {
                    (api.png_image_write_to_stdio)(
                        &mut img,
                        std::ptr::null_mut(),
                        fc.conv8,
                        if fc.null_buffer {
                            std::ptr::null()
                        } else {
                            buf.as_ptr() as *const c_void
                        },
                        fc.stride,
                        std::ptr::null(),
                    )
                };
                (r as i64, 0)
            })
        };
        let c2 = run2(c_api());
        let r2 = run2(rs_api());
        if std::env::var_os("PNGDUMP").is_some() {
            eprintln!("DUMP to_stdio {} => {:?}", fc.lbl, c2);
        }
        assert_eq!(c2, r2, "png_image_write_to_stdio(NULL file): {}", fc.lbl);
    }
}

// ===========================================================================
// 9. row-level write helpers (rows 1124, 1128)
// ===========================================================================

#[test]
fn row_transform_rejections() {
    // row 1124: png_do_write_interlace with pass >= 6 is a no-op.
    //
    // NOTE (C UB, dropped): a *negative* pass indexes the file-scope
    // `png_pass_start[pass]` / `png_pass_inc[pass]` arrays out of bounds
    // (pngwutil.c:2117-2118).  That reads whatever static happens to precede
    // the table, which differs between the two builds, so it is undefined
    // behaviour rather than an error path.
    for pd in [1u8, 2, 4, 8, 16, 24, 32, 48, 64] {
        for pass in [0i32, 1, 2, 3, 4, 5, 6, 7, 99, i32::MAX] {
            for w in [1u32, 3, 8, 17] {
                same!(format!("png_do_write_interlace pd={} pass={} w={}", pd, pass, w), |api| {
                    let mut ri = png_row_info {
                        width: w,
                        rowbytes: rowbytes(pd as u32, w),
                        color_type: 0,
                        bit_depth: if pd >= 8 { 8 } else { pd },
                        channels: if pd >= 8 { pd / 8 } else { 1 },
                        pixel_depth: pd,
                    };
                    let mut row: Vec<u8> =
                        (0..(rowbytes(pd as u32, w) + 64)).map(|i| (i * 91 + 7) as u8).collect();
                    unsafe { (api.png_do_write_interlace)(&mut ri, row.as_mut_ptr(), pass) };
                    let end = rowbytes(pd as u32, w) + 64;
                    (ri.width as i64) * 1_000_000
                        + (ri.rowbytes as i64) * 1000
                        + fnv(&row[..end]) % 1000
                });
            }
        }
    }
    // png_do_packswap / png_do_write_transformations argument guards
    same!("png_do_packswap on a >=8bit row", |api| {
        let mut ri = png_row_info {
            width: 8,
            rowbytes: 8,
            color_type: 0,
            bit_depth: 8,
            channels: 1,
            pixel_depth: 8,
        };
        let mut row: Vec<u8> = (0..16u8).collect();
        unsafe { (api.png_do_packswap)(&mut ri, row.as_mut_ptr()) };
        fnv(&row)
    });
    // row 1128 (NULL png_ptr) is covered in write_pipeline_ordering; here the
    // NULL row_info half is NOT tested: png_do_write_transformations
    // dereferences `row_info->color_type` once past the png_ptr check
    // (pngwtran.c:507), so a NULL row_info is C undefined behaviour.
    wsame!("png_do_write_transformations with no transforms", |api, png, info| {
        unsafe {
            (api.png_set_IHDR)(png, info, 8, 2, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0);
            (api.png_write_info)(png, info);
            let mut ri = png_row_info {
                width: 8,
                rowbytes: 24,
                color_type: PNG_COLOR_TYPE_RGB as png_byte,
                bit_depth: 8,
                channels: 3,
                pixel_depth: 24,
            };
            (api.png_do_write_transformations)(png, &mut ri);
            (ri.width as i64) * 1000 + ri.rowbytes as i64
        }
    });
}

// ===========================================================================
// 10. unknown chunks on the write side (row 975)
// ===========================================================================

#[test]
fn write_unknown_chunk_rejections() {
    let data: Vec<u8> = (0..16u8).collect();
    for (nm, size) in [
        (*b"prVt\0", 0usize),
        (*b"prVt\0", 4),
        (*b"PRVT\0", 0),
        (*b"PRVT\0", 4),
        (*b"prvt\0", 0),
        (*b"PrVt\0", 0),
    ] {
        for keep in [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
            -1,
            PNG_HANDLE_CHUNK_LAST,
            99,
        ] {
            for loc in [0i32, PNG_HAVE_IHDR as c_int, PNG_HAVE_PLTE as c_int, PNG_AFTER_IDAT as c_int, 0xff] {
                wsame!(
                    format!("unknown chunk {:?} size={} keep={} loc={:#x}", nm, size, keep, loc),
                    |api, png, info| {
                        unsafe {
                            (api.png_set_IHDR)(
                                png, info, 4, 2, 8, PNG_COLOR_TYPE_RGB, IL_NONE, 0, 0,
                            );
                            (api.png_set_keep_unknown_chunks)(
                                png,
                                keep,
                                std::ptr::null(),
                                0,
                            );
                            let ch = png_unknown_chunk {
                                name: nm,
                                data: if size == 0 {
                                    std::ptr::null_mut()
                                } else {
                                    data.as_ptr() as *mut png_byte
                                },
                                size,
                                location: loc as png_byte,
                            };
                            (api.png_set_unknown_chunks)(png, info, &ch, 1);
                            (api.png_set_unknown_chunk_location)(png, info, 0, loc);
                            (api.png_write_info)(png, info);
                            let row = row_bytes(4);
                            for _ in 0..2 {
                                (api.png_write_row)(png, row.as_ptr());
                            }
                            (api.png_write_end)(png, info);
                        }
                        0
                    }
                );
            }
        }
    }
}

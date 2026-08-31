//! Phase C — ERRORS.md rows 287..568, i.e. the whole
//! `pngget.c` / `pngset.c` / `pngtrans.c` section.
//!
//! Every row constructs its exact invalid input / condition and asserts both
//! libraries reject it identically: same return value, same out-parameters,
//! same captured `Diag` (warning + error message text).
//!
//! Out-parameters are always pre-filled with a recognisable sentinel and then
//! reported, so a "the C did not write to it" row is distinguishable from a
//! "the C wrote 0" row.
//!
//! Rows that are *unreachable* in this build configuration, and rows whose C
//! implementation dereferences a pointer before checking it (C undefined
//! behaviour, not an error path), are called out in comments at the point where
//! they would otherwise be tested.
#![allow(clippy::too_many_arguments)]
mod common;
use common::*;
use std::cell::Cell;
use std::ffi::CString;

// ---------------------------------------------------------------------------
// probe / same!  (same shape as t10_errors_core.rs, but the observable value is
// a String so that out-parameters can be folded in)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
struct P(String, bool, Diag);

fn probe<F: FnOnce(&'static Api) -> String>(api: &'static Api, f: F) -> P {
    if std::env::var_os("PNGTRACE").is_some() {
        eprintln!("  TRACE   lib={}", api.name);
    }
    set_current_api(api);
    diag_reset();
    budget_set(-1);
    let r = guard(|| f(api));
    let ok = r.is_some();
    P(r.unwrap_or_else(|| "<png_error>".to_string()), ok, diag_take())
}

macro_rules! same {
    ($label:expr, $f:expr) => {{
        if std::env::var_os("PNGTRACE").is_some() {
            eprintln!("TRACE {}", $label);
        }
        let c = probe(c_api(), $f);
        let r = probe(rs_api(), $f);
        assert_eq!(c, r, "{}", $label);
        c
    }};
}

// ---------------------------------------------------------------------------
// A "budget" allocator: the Nth and every later allocation fails.  This is how
// the out-of-memory rows (431, 436, 444..447, 452, 453, 469, 470, 474, 478,
// 491, 493, 494, 495, 501, 502) are reached: libpng routes *every* allocation
// through png_malloc_base -> png_ptr->malloc_fn.
// ---------------------------------------------------------------------------

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

thread_local! {
    static BUDGET: Cell<i64> = const { Cell::new(-1) };
}

fn budget_set(n: i64) {
    BUDGET.with(|c| c.set(n));
}

unsafe extern "C-unwind" fn budget_malloc(_png: png_structp, size: usize) -> png_voidp {
    let b = BUDGET.with(|c| c.get());
    if b == 0 {
        return std::ptr::null_mut();
    }
    if b > 0 {
        BUDGET.with(|c| c.set(b - 1));
    }
    malloc(size)
}

/// Install the budget allocator with `n` successful allocations remaining.
unsafe fn starve(api: &'static Api, png: png_structp, n: i64) {
    (api.png_set_mem_fn)(png, std::ptr::null_mut(), Some(budget_malloc), None);
    budget_set(n);
}

/// Put the default allocator back (so teardown cannot be perturbed).
unsafe fn unstarve(api: &'static Api, png: png_structp) {
    budget_set(-1);
    (api.png_set_mem_fn)(png, std::ptr::null_mut(), None, None);
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

const PNG_RESOLUTION_UNKNOWN: c_int = 0;
const PNG_RESOLUTION_METER: c_int = 1;
const PNG_OFFSET_PIXEL: c_int = 0;
const PNG_OFFSET_MICROMETER: c_int = 1;

fn nn<T>(p: *const T) -> &'static str {
    if p.is_null() {
        "NULL"
    } else {
        "ptr"
    }
}

fn f32b(v: f32) -> String {
    format!("{:#010x}", v.to_bits())
}

fn f64b(v: f64) -> String {
    format!("{:#018x}", v.to_bits())
}

/// A write session whose info_struct already carries a legal IHDR.
unsafe fn wsess(api: &'static Api, ct: c_int, bd: c_int) -> WriteSess {
    let s = WriteSess::new(api);
    (api.png_set_IHDR)(s.png, s.info, 8, 8, bd, ct, 0, 0, 0);
    s
}

/// The 4 standard pointer variants plus "chunk unset" plus "chunk set", run
/// through `body`.  `setup` installs the chunk for the last variant.
fn matrix<F>(label: &str, setup: fn(&'static Api, png_structp, png_infop), body: F)
where
    F: Fn(&'static Api, png_structp, png_infop) -> String + Copy,
{
    same!(format!("{} [png=NULL info=NULL]", label), |api| body(
        api,
        std::ptr::null_mut(),
        std::ptr::null_mut()
    ));
    same!(format!("{} [png=NULL]", label), |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        body(api, std::ptr::null_mut(), s.info)
    });
    same!(format!("{} [info=NULL]", label), |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        body(api, s.png, std::ptr::null_mut())
    });
    same!(format!("{} [not set]", label), |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        body(api, s.png, s.info)
    });
    same!(format!("{} [set]", label), |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        setup(api, s.png, s.info);
        body(api, s.png, s.info)
    });
}

fn setup_none(_a: &'static Api, _p: png_structp, _i: png_infop) {}

// ===========================================================================
// pngget.c
// ===========================================================================

/// rows 287, 288, 289
#[test]
fn get_valid_rejections() {
    let flags = [
        0u32,
        PNG_INFO_gAMA,
        PNG_INFO_sBIT,
        PNG_INFO_cHRM,
        PNG_INFO_PLTE,
        PNG_INFO_tRNS,
        PNG_INFO_bKGD,
        PNG_INFO_hIST,
        PNG_INFO_pHYs,
        PNG_INFO_oFFs,
        PNG_INFO_tIME,
        PNG_INFO_pCAL,
        PNG_INFO_sRGB,
        PNG_INFO_iCCP,
        PNG_INFO_sPLT,
        PNG_INFO_sCAL,
        PNG_INFO_IDAT,
        PNG_INFO_eXIf,
        PNG_INFO_cICP,
        PNG_INFO_cLLI,
        PNG_INFO_mDCV,
        0x0010_0000,
        0x8000_0000,
        0xffff_ffff,
    ];
    for f in flags {
        // row 287: either pointer NULL -> 0
        same!(format!("get_valid(NULL,NULL,{:#x})", f), |api| format!("{}", unsafe {
            (api.png_get_valid)(std::ptr::null(), std::ptr::null(), f)
        }));
        same!(format!("get_valid(png,NULL,{:#x})", f), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            format!("{}", (api.png_get_valid)(s.png, std::ptr::null(), f))
        });
        same!(format!("get_valid(NULL,info,{:#x})", f), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            format!("{}", (api.png_get_valid)(std::ptr::null(), s.info, f))
        });
        // row 289: bit clear in info_ptr->valid
        same!(format!("get_valid(fresh,{:#x})", f), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            format!("{}", (api.png_get_valid)(s.png, s.info, f))
        });
        // row 288: tRNS 'valid' bit set but png_ptr->num_trans == 0.  On a write
        // struct png_ptr->num_trans is never assigned by png_set_tRNS, so this
        // is exactly the "tRNS canceled" state pngget.c:29-30 guards against.
        same!(format!("get_valid(after set_tRNS,{:#x})", f), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let tc = png_color_16 {
                index: 0,
                red: 1,
                green: 2,
                blue: 3,
                gray: 0,
            };
            (api.png_set_tRNS)(s.png, s.info, std::ptr::null(), 0, &tc);
            format!(
                "valid={} get_tRNS={}",
                (api.png_get_valid)(s.png, s.info, f),
                {
                    let mut n: c_int = -12345;
                    (api.png_get_tRNS)(
                        s.png,
                        s.info,
                        std::ptr::null_mut(),
                        &mut n,
                        std::ptr::null_mut(),
                    )
                }
            )
        });
        // ... and with several other chunks set, so 'valid' has real bits in it
        same!(format!("get_valid(after gAMA+pHYs,{:#x})", f), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_gAMA_fixed)(s.png, s.info, 45455);
            (api.png_set_pHYs)(s.png, s.info, 1, 1, PNG_RESOLUTION_METER);
            format!("{}", (api.png_get_valid)(s.png, s.info, f))
        });
    }
}

/// rows 290..298, 325, 402..409
#[test]
fn get_scalar_null_guards() {
    fn all(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            format!(
                "rowbytes={} rows={} w={} h={} bd={} ct={} ft={} it={} comp={} ch={} pmax={}",
                (api.png_get_rowbytes)(png, info),
                nn((api.png_get_rows)(png, info)),
                (api.png_get_image_width)(png, info),
                (api.png_get_image_height)(png, info),
                (api.png_get_bit_depth)(png, info),
                (api.png_get_color_type)(png, info),
                (api.png_get_filter_type)(png, info),
                (api.png_get_interlace_type)(png, info),
                (api.png_get_compression_type)(png, info),
                (api.png_get_channels)(png, info),
                (api.png_get_palette_max)(png, info),
            )
        }
    }
    matrix("scalar getters", setup_none, all);

    // png_ptr-only getters (rows 402..408)
    same!("png_ptr-only getters(NULL)", |api| unsafe {
        let p: png_structp = std::ptr::null_mut();
        format!(
            "rgb2gray={} userchunk={} cbs={} uwm={} uhm={} ccm={} cmm={}",
            (api.png_get_rgb_to_gray_status)(p),
            nn((api.png_get_user_chunk_ptr)(p)),
            (api.png_get_compression_buffer_size)(p),
            (api.png_get_user_width_max)(p),
            (api.png_get_user_height_max)(p),
            (api.png_get_chunk_cache_max)(p),
            (api.png_get_chunk_malloc_max)(p),
        )
    });
    for read in [true, false] {
        same!(format!("png_ptr-only getters(read={})", read), |api| unsafe {
            let (png, _kr, _kw);
            if read {
                let s = ReadSess::new(api, &[]);
                png = s.png;
                _kr = Some(s);
                _kw = None;
            } else {
                let s = WriteSess::new(api);
                png = s.png;
                _kr = None;
                _kw = Some(s);
            }
            format!(
                "rgb2gray={} userchunk={} cbs={} uwm={} uhm={} ccm={} cmm={}",
                (api.png_get_rgb_to_gray_status)(png),
                nn((api.png_get_user_chunk_ptr)(png)),
                (api.png_get_compression_buffer_size)(png),
                (api.png_get_user_width_max)(png),
                (api.png_get_user_height_max)(png),
                (api.png_get_chunk_cache_max)(png),
                (api.png_get_chunk_malloc_max)(png),
            )
        });
    }
    // row 409: png_get_palette_max returns -1 for a NULL pointer, but the real
    // num_palette_max (-1 by default, 0 after png_set_check_for_invalid_index)
    for allowed in [-1i32, 0, 1, 2] {
        same!(format!("palette_max(check_index={})", allowed), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
            (api.png_set_check_for_invalid_index)(s.png, allowed);
            format!("{}", (api.png_get_palette_max)(s.png, s.info))
        });
    }
}

/// rows 299..311, 320, 321, 323, 324, 384, 385
#[test]
fn get_phys_rejections() {
    // Every (res_x, res_y, unit_type) that matters, including a unit type that
    // is not PNG_RESOLUTION_METER (rows 300/302/304) and non-square pixels
    // (row 305), plus the divide-by-zero / cast-overflow guards of
    // png_get_pixel_aspect_ratio{,_fixed} (rows 307, 309, 310, 311).
    let cases: [(u32, u32, c_int); 14] = [
        (0, 0, PNG_RESOLUTION_METER),
        (1, 1, PNG_RESOLUTION_METER),
        (100, 100, PNG_RESOLUTION_METER),
        (100, 200, PNG_RESOLUTION_METER),
        (0, 100, PNG_RESOLUTION_METER),
        (100, 0, PNG_RESOLUTION_METER),
        (100, 100, PNG_RESOLUTION_UNKNOWN),
        (100, 100, 2),
        (100, 100, 255),
        (100, 100, -1),
        (PNG_UINT_31_MAX, 1, PNG_RESOLUTION_METER),
        (1, PNG_UINT_31_MAX, PNG_RESOLUTION_METER),
        (0x8000_0000, 0x8000_0000, PNG_RESOLUTION_METER),
        (0xffff_ffff, 0xffff_ffff, PNG_RESOLUTION_METER),
    ];
    for (rx, ry, ut) in cases {
        same!(format!("pHYs getters({},{},{})", rx, ry, ut), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_pHYs)(s.png, s.info, rx, ry, ut);
            let mut ox: png_uint_32 = 0xdead_beef;
            let mut oy: png_uint_32 = 0xdead_beef;
            let mut ou: c_int = -12345;
            let r1 = (api.png_get_pHYs)(s.png, s.info, &mut ox, &mut oy, &mut ou);
            let mut dx: png_uint_32 = 0xdead_beef;
            let mut dy: png_uint_32 = 0xdead_beef;
            let mut du: c_int = -12345;
            let r2 = (api.png_get_pHYs_dpi)(s.png, s.info, &mut dx, &mut dy, &mut du);
            format!(
                "xppm={} yppm={} ppm={} ar={} arf={} ppi={} xppi={} yppi={} \
                 pHYs={}/{}/{}/{} dpi={}/{}/{}/{}",
                (api.png_get_x_pixels_per_meter)(s.png, s.info),
                (api.png_get_y_pixels_per_meter)(s.png, s.info),
                (api.png_get_pixels_per_meter)(s.png, s.info),
                f32b((api.png_get_pixel_aspect_ratio)(s.png, s.info)),
                (api.png_get_pixel_aspect_ratio_fixed)(s.png, s.info),
                (api.png_get_pixels_per_inch)(s.png, s.info),
                (api.png_get_x_pixels_per_inch)(s.png, s.info),
                (api.png_get_y_pixels_per_inch)(s.png, s.info),
                r1,
                ox,
                oy,
                ou,
                r2,
                dx,
                dy,
                du,
            )
        });
    }
    // rows 299, 301, 303, 306, 308, 323, 384: pHYs never set / NULL pointers
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut ox: png_uint_32 = 0xdead_beef;
            let mut oy: png_uint_32 = 0xdead_beef;
            let mut ou: c_int = -12345;
            let r1 = (api.png_get_pHYs)(png, info, &mut ox, &mut oy, &mut ou);
            let mut dx: png_uint_32 = 0xdead_beef;
            let mut dy: png_uint_32 = 0xdead_beef;
            let mut du: c_int = -12345;
            let r2 = (api.png_get_pHYs_dpi)(png, info, &mut dx, &mut dy, &mut du);
            format!(
                "xppm={} yppm={} ppm={} ar={} arf={} pHYs={}/{}/{}/{} dpi={}/{}/{}/{}",
                (api.png_get_x_pixels_per_meter)(png, info),
                (api.png_get_y_pixels_per_meter)(png, info),
                (api.png_get_pixels_per_meter)(png, info),
                f32b((api.png_get_pixel_aspect_ratio)(png, info)),
                (api.png_get_pixel_aspect_ratio_fixed)(png, info),
                r1,
                ox,
                oy,
                ou,
                r2,
                dx,
                dy,
                du,
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        unsafe { (api.png_set_pHYs)(png, info, 300, 300, PNG_RESOLUTION_METER) }
    }
    matrix("pHYs", setup, body);

    // rows 324, 385: every out-parameter NULL -> retval stays 0 even though the
    // chunk *is* valid.
    for set in [false, true] {
        for mask in 0u32..8 {
            same!(format!("pHYs out-params mask={} set={}", mask, set), |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                if set {
                    (api.png_set_pHYs)(s.png, s.info, 7, 9, PNG_RESOLUTION_METER);
                }
                let mut ox: png_uint_32 = 0xdead_beef;
                let mut oy: png_uint_32 = 0xdead_beef;
                let mut ou: c_int = -12345;
                let px = if mask & 1 != 0 { &mut ox as *mut _ } else { std::ptr::null_mut() };
                let py = if mask & 2 != 0 { &mut oy as *mut _ } else { std::ptr::null_mut() };
                let pu = if mask & 4 != 0 { &mut ou as *mut _ } else { std::ptr::null_mut() };
                let r1 = (api.png_get_pHYs)(s.png, s.info, px, py, pu);
                let a = (ox, oy, ou);
                ox = 0xdead_beef;
                oy = 0xdead_beef;
                ou = -12345;
                let px = if mask & 1 != 0 { &mut ox as *mut _ } else { std::ptr::null_mut() };
                let py = if mask & 2 != 0 { &mut oy as *mut _ } else { std::ptr::null_mut() };
                let pu = if mask & 4 != 0 { &mut ou as *mut _ } else { std::ptr::null_mut() };
                let r2 = (api.png_get_pHYs_dpi)(s.png, s.info, px, py, pu);
                format!("{} {:?} | {} {:?}", r1, a, r2, (ox, oy, ou))
            });
        }
    }
    // rows 320, 321: ppi_from_ppm overflow -- ppm > PNG_UINT_31_MAX, and
    // png_muldiv(ppm,127,5000) overflow.
    for ppm in [
        0u32,
        1,
        5000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0x0400_0000,
        0x1000_0000,
    ] {
        same!(format!("ppi_from_ppm({:#x})", ppm), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_pHYs)(s.png, s.info, ppm, ppm, PNG_RESOLUTION_METER);
            format!(
                "{}/{}/{}",
                (api.png_get_pixels_per_inch)(s.png, s.info),
                (api.png_get_x_pixels_per_inch)(s.png, s.info),
                (api.png_get_y_pixels_per_inch)(s.png, s.info),
            )
        });
    }
}

/// rows 312..319, 322, 371, 372, 373
#[test]
fn get_offs_rejections() {
    let cases: [(png_int_32, png_int_32, c_int); 12] = [
        (0, 0, PNG_OFFSET_PIXEL),
        (0, 0, PNG_OFFSET_MICROMETER),
        (1, -1, PNG_OFFSET_PIXEL),
        (1, -1, PNG_OFFSET_MICROMETER),
        (i32::MAX, i32::MAX, PNG_OFFSET_MICROMETER),
        (i32::MIN, i32::MIN, PNG_OFFSET_MICROMETER),
        (i32::MAX, i32::MIN, PNG_OFFSET_PIXEL),
        (100, 200, 2),
        (100, 200, 255),
        (100, 200, -1),
        (5_000_000, 5_000_000, PNG_OFFSET_MICROMETER),
        (545_260_000, -545_260_000, PNG_OFFSET_MICROMETER),
    ];
    for (x, y, ut) in cases {
        same!(format!("oFFs getters({},{},{})", x, y, ut), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_oFFs)(s.png, s.info, x, y, ut);
            let mut ox: png_int_32 = -12345;
            let mut oy: png_int_32 = -12345;
            let mut ou: c_int = -12345;
            let r = (api.png_get_oFFs)(s.png, s.info, &mut ox, &mut oy, &mut ou);
            format!(
                "xm={} ym={} xp={} yp={} xif={} yif={} xi={} yi={} oFFs={}/{}/{}/{}",
                (api.png_get_x_offset_microns)(s.png, s.info),
                (api.png_get_y_offset_microns)(s.png, s.info),
                (api.png_get_x_offset_pixels)(s.png, s.info),
                (api.png_get_y_offset_pixels)(s.png, s.info),
                (api.png_get_x_offset_inches_fixed)(s.png, s.info),
                (api.png_get_y_offset_inches_fixed)(s.png, s.info),
                f32b((api.png_get_x_offset_inches)(s.png, s.info)),
                f32b((api.png_get_y_offset_inches)(s.png, s.info)),
                r,
                ox,
                oy,
                ou,
            )
        });
    }
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut ox: png_int_32 = -12345;
            let mut oy: png_int_32 = -12345;
            let mut ou: c_int = -12345;
            let r = (api.png_get_oFFs)(png, info, &mut ox, &mut oy, &mut ou);
            format!(
                "xm={} ym={} xp={} yp={} xif={} yif={} oFFs={}/{}/{}/{}",
                (api.png_get_x_offset_microns)(png, info),
                (api.png_get_y_offset_microns)(png, info),
                (api.png_get_x_offset_pixels)(png, info),
                (api.png_get_y_offset_pixels)(png, info),
                (api.png_get_x_offset_inches_fixed)(png, info),
                (api.png_get_y_offset_inches_fixed)(png, info),
                r,
                ox,
                oy,
                ou,
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        unsafe { (api.png_set_oFFs)(png, info, 11, 22, PNG_OFFSET_PIXEL) }
    }
    matrix("oFFs", setup, body);

    // row 373: any of the three out-parameters NULL -> return 0, nothing written
    for set in [false, true] {
        for mask in 0u32..8 {
            same!(format!("oFFs out-params mask={} set={}", mask, set), |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                if set {
                    (api.png_set_oFFs)(s.png, s.info, 3, 4, PNG_OFFSET_PIXEL);
                }
                let mut ox: png_int_32 = -12345;
                let mut oy: png_int_32 = -12345;
                let mut ou: c_int = -12345;
                let px = if mask & 1 != 0 { &mut ox as *mut _ } else { std::ptr::null_mut() };
                let py = if mask & 2 != 0 { &mut oy as *mut _ } else { std::ptr::null_mut() };
                let pu = if mask & 4 != 0 { &mut ou as *mut _ } else { std::ptr::null_mut() };
                let r = (api.png_get_oFFs)(s.png, s.info, px, py, pu);
                format!("{} {:?}", r, (ox, oy, ou))
            });
        }
    }
}

/// row 326
#[test]
fn get_signature_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe { nn((api.png_get_signature)(png, info)).to_string() }
    }
    matrix("png_get_signature", setup_none, body);
}

/// rows 327, 328, 329
#[test]
fn get_bkgd_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut bg: png_color_16p = 0x1 as png_color_16p;
            let r = (api.png_get_bKGD)(png, info, &mut bg);
            let mut r2s = String::new();
            // row 329: background == NULL
            let r2 = (api.png_get_bKGD)(png, info, std::ptr::null_mut());
            r2s.push_str(&format!("{}", r2));
            format!(
                "r={} bg={} contents={} nullout={}",
                r,
                if bg as usize == 1 { "untouched" } else { nn(bg) },
                if bg.is_null() || bg as usize == 1 {
                    "-".to_string()
                } else {
                    format!("{:?}", *bg)
                },
                r2s
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let bg = png_color_16 {
            index: 3,
            red: 4,
            green: 5,
            blue: 6,
            gray: 7,
        };
        unsafe { (api.png_set_bKGD)(png, info, &bg) }
    }
    matrix("png_get_bKGD", setup, body);
}

/// rows 330..339
#[test]
fn get_chrm_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut d = [-1.0f64; 9];
            let r1 = (api.png_get_cHRM)(
                png, info, &mut d[0], &mut d[1], &mut d[2], &mut d[3], &mut d[4], &mut d[5],
                &mut d[6], &mut d[7],
            );
            let a: Vec<String> = d.iter().map(|&v| f64b(v)).collect();
            let mut e = [-1.0f64; 9];
            let r2 = (api.png_get_cHRM_XYZ)(
                png, info, &mut e[0], &mut e[1], &mut e[2], &mut e[3], &mut e[4], &mut e[5],
                &mut e[6], &mut e[7], &mut e[8],
            );
            let b: Vec<String> = e.iter().map(|&v| f64b(v)).collect();
            let mut f = [-1i32; 9];
            let r3 = (api.png_get_cHRM_fixed)(
                png, info, &mut f[0], &mut f[1], &mut f[2], &mut f[3], &mut f[4], &mut f[5],
                &mut f[6], &mut f[7],
            );
            let mut g = [-1i32; 9];
            let r4 = (api.png_get_cHRM_XYZ_fixed)(
                png, info, &mut g[0], &mut g[1], &mut g[2], &mut g[3], &mut g[4], &mut g[5],
                &mut g[6], &mut g[7], &mut g[8],
            );
            // all-NULL out-parameters
            let r5 = (api.png_get_cHRM_fixed)(
                png,
                info,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            format!(
                "cHRM={} {:?} XYZ={} {:?} fixed={} {:?} XYZfixed={} {:?} allnull={}",
                r1, a, r2, b, r3, f, r4, g, r5
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        unsafe {
            (api.png_set_cHRM_fixed)(png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000)
        }
    }
    matrix("cHRM getters", setup, body);

    // rows 334, 337: png_XYZ_from_xy fails -> the XYZ getters return 0 even
    // though PNG_INFO_cHRM is set.  These chromaticity sets are degenerate.
    let bad: [[png_fixed_point; 8]; 8] = [
        [0, 0, 0, 0, 0, 0, 0, 0],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [31270, 0, 64000, 33000, 30000, 60000, 15000, 6000],
        [31270, 32900, 0, 0, 0, 0, 0, 0],
        [31270, 32900, 33333, 33333, 33333, 33333, 33333, 33333],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN, i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [-31270, -32900, -64000, -33000, -30000, -60000, -15000, -6000],
    ];
    for (i, v) in bad.iter().enumerate() {
        let v = *v;
        same!(format!("cHRM XYZ degenerate #{}", i), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_cHRM_fixed)(s.png, s.info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
            let mut g = [-1i32; 9];
            let r4 = (api.png_get_cHRM_XYZ_fixed)(
                s.png, s.info, &mut g[0], &mut g[1], &mut g[2], &mut g[3], &mut g[4], &mut g[5],
                &mut g[6], &mut g[7], &mut g[8],
            );
            let mut e = [-1.0f64; 9];
            let r2 = (api.png_get_cHRM_XYZ)(
                s.png, s.info, &mut e[0], &mut e[1], &mut e[2], &mut e[3], &mut e[4], &mut e[5],
                &mut e[6], &mut e[7], &mut e[8],
            );
            let b: Vec<String> = e.iter().map(|&x| f64b(x)).collect();
            format!("fixed={} {:?} float={} {:?}", r4, g, r2, b)
        });
    }
}

/// rows 340..343
#[test]
fn get_gama_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut fx: png_fixed_point = -12345;
            let r1 = (api.png_get_gAMA_fixed)(png, info, &mut fx);
            let mut fl: f64 = -1.0;
            let r2 = (api.png_get_gAMA)(png, info, &mut fl);
            let r3 = (api.png_get_gAMA_fixed)(png, info, std::ptr::null_mut());
            let r4 = (api.png_get_gAMA)(png, info, std::ptr::null_mut());
            format!("{}/{} {}/{} nulls={}/{}", r1, fx, r2, f64b(fl), r3, r4)
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        unsafe { (api.png_set_gAMA_fixed)(png, info, 45455) }
    }
    matrix("gAMA getters", setup, body);
}

/// rows 344, 345
#[test]
fn get_srgb_rejections() {
    for intent in [-1i32, 0, 1, 2, 3, 4, 100, i32::MIN, i32::MAX] {
        same!(format!("get_sRGB(intent={})", intent), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_sRGB)(s.png, s.info, intent);
            let mut i: c_int = -12345;
            let r = (api.png_get_sRGB)(s.png, s.info, &mut i);
            let r2 = (api.png_get_sRGB)(s.png, s.info, std::ptr::null_mut());
            format!("{}/{} null={}", r, i, r2)
        });
    }
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut i: c_int = -12345;
            let r = (api.png_get_sRGB)(png, info, &mut i);
            let r2 = (api.png_get_sRGB)(png, info, std::ptr::null_mut());
            format!("{}/{} null={}", r, i, r2)
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        unsafe { (api.png_set_sRGB)(png, info, 1) }
    }
    matrix("sRGB getters", setup, body);
}

/// rows 346, 347, 348
#[test]
fn get_iccp_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut out = String::new();
            for mask in 0u32..16 {
                let mut name: png_charp = 0x1 as png_charp;
                let mut ct: c_int = -12345;
                let mut prof: png_bytep = 0x1 as png_bytep;
                let mut len: png_uint_32 = 0xdead_beef;
                let pn = if mask & 1 != 0 { &mut name as *mut _ } else { std::ptr::null_mut() };
                let pc = if mask & 2 != 0 { &mut ct as *mut _ } else { std::ptr::null_mut() };
                let pp = if mask & 4 != 0 { &mut prof as *mut _ } else { std::ptr::null_mut() };
                let pl = if mask & 8 != 0 { &mut len as *mut _ } else { std::ptr::null_mut() };
                let r = (api.png_get_iCCP)(png, info, pn, pc, pp, pl);
                out.push_str(&format!(
                    "[{} r={} name={} ct={} prof={} len={}]",
                    mask,
                    r,
                    if name as usize == 1 { "untouched" } else { nn(name) },
                    ct,
                    if prof as usize == 1 { "untouched" } else { nn(prof) },
                    len
                ));
            }
            out
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let n = cs("ICC profile");
        let prof: [u8; 16] = [0, 0, 0, 16, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        unsafe {
            (api.png_set_iCCP)(png, info, n.as_ptr(), 0, prof.as_ptr(), 16);
        }
    }
    matrix("iCCP getters", setup, body);
}

/// rows 349, 350
#[test]
fn get_splt_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut sp: png_sPLT_tp = 0x1 as png_sPLT_tp;
            let r = (api.png_get_sPLT)(png, info, &mut sp);
            let r2 = (api.png_get_sPLT)(png, info, std::ptr::null_mut());
            format!(
                "{} {} null={}",
                r,
                if sp as usize == 1 { "untouched" } else { nn(sp) },
                r2
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let name = cs("pal");
        let mut ents = [png_sPLT_entry::default(); 2];
        ents[0].red = 1;
        let sp = png_sPLT_t {
            name: name.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 2,
        };
        unsafe { (api.png_set_sPLT)(png, info, &sp, 1) }
    }
    matrix("sPLT getters", setup, body);
}

/// rows 351, 352, 353
#[test]
fn get_cicp_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut out = String::new();
            for mask in 0u32..16 {
                let mut a: png_byte = 0xaa;
                let mut b: png_byte = 0xaa;
                let mut c: png_byte = 0xaa;
                let mut d: png_byte = 0xaa;
                let pa = if mask & 1 != 0 { &mut a as *mut _ } else { std::ptr::null_mut() };
                let pb = if mask & 2 != 0 { &mut b as *mut _ } else { std::ptr::null_mut() };
                let pc = if mask & 4 != 0 { &mut c as *mut _ } else { std::ptr::null_mut() };
                let pd = if mask & 8 != 0 { &mut d as *mut _ } else { std::ptr::null_mut() };
                let r = (api.png_get_cICP)(png, info, pa, pb, pc, pd);
                out.push_str(&format!("[{} r={} {} {} {} {}]", mask, r, a, b, c, d));
            }
            out
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        unsafe { (api.png_set_cICP)(png, info, 9, 16, 0, 1) }
    }
    matrix("cICP getters", setup, body);
}

/// rows 354, 355, 356, 357
#[test]
fn get_clli_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut a: png_uint_32 = 0xdead_beef;
            let mut b: png_uint_32 = 0xdead_beef;
            let r1 = (api.png_get_cLLI_fixed)(png, info, &mut a, &mut b);
            let mut x: f64 = -1.0;
            let mut y: f64 = -1.0;
            let r2 = (api.png_get_cLLI)(png, info, &mut x, &mut y);
            let r3 = (api.png_get_cLLI_fixed)(png, info, std::ptr::null_mut(), std::ptr::null_mut());
            let r4 = (api.png_get_cLLI)(png, info, std::ptr::null_mut(), std::ptr::null_mut());
            format!(
                "{}/{}/{} {}/{}/{} nulls={}/{}",
                r1,
                a,
                b,
                r2,
                f64b(x),
                f64b(y),
                r3,
                r4
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        unsafe { (api.png_set_cLLI_fixed)(png, info, 10_000_000, 1_000_000) }
    }
    matrix("cLLI getters", setup, body);
}

/// rows 358, 359, 360, 361
#[test]
fn get_mdcv_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut f = [-1i32; 8];
            let mut dl: png_uint_32 = 0xdead_beef;
            let mut ml: png_uint_32 = 0xdead_beef;
            let r1 = (api.png_get_mDCV_fixed)(
                png, info, &mut f[0], &mut f[1], &mut f[2], &mut f[3], &mut f[4], &mut f[5],
                &mut f[6], &mut f[7], &mut dl, &mut ml,
            );
            let mut d = [-1.0f64; 10];
            let r2 = (api.png_get_mDCV)(
                png, info, &mut d[0], &mut d[1], &mut d[2], &mut d[3], &mut d[4], &mut d[5],
                &mut d[6], &mut d[7], &mut d[8], &mut d[9],
            );
            let b: Vec<String> = d.iter().map(|&v| f64b(v)).collect();
            let r3 = (api.png_get_mDCV_fixed)(
                png,
                info,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            format!("{} {:?} {} {} {:?} allnull={}", r1, f, (dl, ml).0, r2, b, r3)
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        unsafe {
            (api.png_set_mDCV_fixed)(
                png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10_000_000,
                50,
            )
        }
    }
    matrix("mDCV getters", setup, body);
}

/// rows 362, 363, 364, 365
#[test]
fn get_exif_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            // row 362: png_get_eXIf is permanently disabled -> warning + 0
            let mut e: png_bytep = 0x1 as png_bytep;
            let r0 = (api.png_get_eXIf)(png, info, &mut e);
            let mut n: png_uint_32 = 0xdead_beef;
            let mut p: png_bytep = 0x1 as png_bytep;
            let r1 = (api.png_get_eXIf_1)(png, info, &mut n, &mut p);
            // row 365: exif == NULL
            let mut n2: png_uint_32 = 0xdead_beef;
            let r2 = (api.png_get_eXIf_1)(png, info, &mut n2, std::ptr::null_mut());
            format!(
                "eXIf={}/{} eXIf_1={}/{}/{} nullexif={}/{}",
                r0,
                if e as usize == 1 { "untouched" } else { nn(e) },
                r1,
                n,
                if p as usize == 1 { "untouched" } else { nn(p) },
                r2,
                n2
            )
        }
    }
    // NOTE: png_get_eXIf_1 writes *num_exif WITHOUT checking it for NULL
    // (pngget.c:910 -- only `exif` is checked at :908), so calling it with
    // num_exif == NULL while PNG_INFO_eXIf is set is C undefined behaviour, not
    // an error path.  That case is therefore not tested.
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let data: [u8; 8] = [b'I', b'I', 42, 0, 8, 0, 0, 0];
        unsafe { (api.png_set_eXIf_1)(png, info, 8, data.as_ptr() as png_bytep) }
    }
    matrix("eXIf getters", setup, body);
}

/// rows 366, 367, 368
#[test]
fn get_hist_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut h: png_uint_16p = 0x1 as png_uint_16p;
            let r = (api.png_get_hIST)(png, info, &mut h);
            let r2 = (api.png_get_hIST)(png, info, std::ptr::null_mut());
            format!(
                "{} {} null={}",
                r,
                if h as usize == 1 { "untouched" } else { nn(h) },
                r2
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let pal = [png_color::default(); 4];
        let hist = [1u16, 2, 3, 4];
        unsafe {
            (api.png_set_PLTE)(png as png_structp, info, pal.as_ptr(), 4);
            (api.png_set_hIST)(png, info, hist.as_ptr());
        }
    }
    matrix("hIST getters", setup, body);
}

/// rows 369, 370
#[test]
fn get_ihdr_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut w: png_uint_32 = 0xdead_beef;
            let mut h: png_uint_32 = 0xdead_beef;
            let mut bd: c_int = -12345;
            let mut ct: c_int = -12345;
            let mut it: c_int = -12345;
            let mut cm: c_int = -12345;
            let mut ft: c_int = -12345;
            let r = (api.png_get_IHDR)(
                png, info, &mut w, &mut h, &mut bd, &mut ct, &mut it, &mut cm, &mut ft,
            );
            // every out-parameter NULL is explicitly allowed here
            let r2 = (api.png_get_IHDR)(
                png,
                info,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            format!(
                "r={} {} {} {} {} {} {} {} allnull={}",
                r, w, h, bd, ct, it, cm, ft, r2
            )
        }
    }
    matrix("png_get_IHDR", setup_none, body);

    // row 370: info_ptr carries an *invalid* IHDR, so png_get_IHDR's defensive
    // png_check_IHDR fires.  png_set_IHDR assigns the fields before validating
    // (pngset.c:445-455), so the bad values survive the first png_error.
    let bad: [(u32, u32, c_int, c_int, c_int, c_int, c_int); 8] = [
        (0, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0),
        (4, 0, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0),
        (4, 4, 3, PNG_COLOR_TYPE_GRAY, 0, 0, 0),
        (4, 4, 0, PNG_COLOR_TYPE_GRAY, 0, 0, 0),
        (4, 4, 8, 1, 0, 0, 0),
        (4, 4, 16, PNG_COLOR_TYPE_PALETTE, 0, 0, 0),
        (4, 4, 8, PNG_COLOR_TYPE_RGB, 2, 0, 0),
        (0x8000_0000, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0),
    ];
    for (w, h, bd, ct, il, cm, ft) in bad {
        same!(
            format!("get_IHDR after bad set_IHDR({},{},{},{})", w, h, bd, ct),
            |api| unsafe {
                let s = WriteSess::new(api);
                // The set may png_error; swallow it but keep the session alive.
                let set_ok =
                    guard(|| (api.png_set_IHDR)(s.png, s.info, w, h, bd, ct, il, cm, ft)).is_some();
                let mut ow: png_uint_32 = 0xdead_beef;
                let mut oh: png_uint_32 = 0xdead_beef;
                let mut obd: c_int = -12345;
                let mut oct: c_int = -12345;
                let mut oil: c_int = -12345;
                let mut ocm: c_int = -12345;
                let mut oft: c_int = -12345;
                let r = (api.png_get_IHDR)(
                    s.png, s.info, &mut ow, &mut oh, &mut obd, &mut oct, &mut oil, &mut ocm,
                    &mut oft,
                );
                format!(
                    "set_ok={} r={} {} {} {} {} {} {} {}",
                    set_ok, r, ow, oh, obd, oct, oil, ocm, oft
                )
            }
        );
    }
}

/// rows 374, 375, 376
#[test]
fn get_pcal_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut out = String::new();
            // one out-parameter NULL at a time (there are 7)
            for hole in 0..8usize {
                let mut purpose: png_charp = 0x1 as png_charp;
                let mut x0: png_int_32 = -12345;
                let mut x1: png_int_32 = -12345;
                let mut ty: c_int = -12345;
                let mut np: c_int = -12345;
                let mut units: png_charp = 0x1 as png_charp;
                let mut params: png_charpp = 0x1 as png_charpp;
                let r = (api.png_get_pCAL)(
                    png,
                    info,
                    if hole == 0 { std::ptr::null_mut() } else { &mut purpose },
                    if hole == 1 { std::ptr::null_mut() } else { &mut x0 },
                    if hole == 2 { std::ptr::null_mut() } else { &mut x1 },
                    if hole == 3 { std::ptr::null_mut() } else { &mut ty },
                    if hole == 4 { std::ptr::null_mut() } else { &mut np },
                    if hole == 5 { std::ptr::null_mut() } else { &mut units },
                    if hole == 6 { std::ptr::null_mut() } else { &mut params },
                );
                out.push_str(&format!(
                    "[hole={} r={} p={} {} {} {} {} u={} pp={}]",
                    hole,
                    r,
                    if purpose as usize == 1 { "untouched" } else { nn(purpose) },
                    x0,
                    x1,
                    ty,
                    np,
                    if units as usize == 1 { "untouched" } else { nn(units) },
                    if params as usize == 1 { "untouched" } else { nn(params) },
                ));
            }
            out
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let purpose = cs("purpose");
        let units = cs("units");
        let p0 = cs("1.5");
        let p1 = cs("-2");
        let mut params = [p0.as_ptr() as png_charp, p1.as_ptr() as png_charp];
        unsafe {
            (api.png_set_pCAL)(
                png,
                info,
                purpose.as_ptr(),
                0,
                100,
                1,
                2,
                units.as_ptr(),
                params.as_mut_ptr(),
            )
        }
    }
    matrix("pCAL getters", setup, body);
}

/// rows 377..383
#[test]
fn get_scal_rejections() {
    // rows 377, 378, 380, 381, 382, 383: NULL pointers / sCAL never set.
    // NOTE: when PNG_INFO_sCAL *is* set, png_get_sCAL{,_fixed,_s} dereference
    // *unit / *width / *height unconditionally (pngget.c:1042-1049, :1067-1069,
    // :1085-1087), so NULL out-parameters there are C undefined behaviour and
    // are not tested.  With the flag clear they return before any store, which
    // is what the rows below exercise.
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let valid = if png.is_null() || info.is_null() {
                0
            } else {
                (api.png_get_valid)(png, info, PNG_INFO_sCAL)
            };
            if valid == 0 {
                let rf = (api.png_get_sCAL_fixed)(
                    png,
                    info,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                let rd = (api.png_get_sCAL)(
                    png,
                    info,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                let rs = (api.png_get_sCAL_s)(
                    png,
                    info,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                return format!("unset nulls={}/{}/{}", rf, rd, rs);
            }
            let mut u: c_int = -12345;
            let mut w: png_fixed_point = -12345;
            let mut h: png_fixed_point = -12345;
            let rf = (api.png_get_sCAL_fixed)(png, info, &mut u, &mut w, &mut h);
            let mut u2: c_int = -12345;
            let mut dw: f64 = -1.0;
            let mut dh: f64 = -1.0;
            let rd = (api.png_get_sCAL)(png, info, &mut u2, &mut dw, &mut dh);
            let mut u3: c_int = -12345;
            let mut sw: png_charp = 0x1 as png_charp;
            let mut sh: png_charp = 0x1 as png_charp;
            let rs = (api.png_get_sCAL_s)(png, info, &mut u3, &mut sw, &mut sh);
            format!(
                "fixed={}/{}/{}/{} float={}/{}/{}/{} s={}/{}/{}/{}",
                rf,
                u,
                w,
                h,
                rd,
                u2,
                f64b(dw),
                f64b(dh),
                rs,
                u3,
                rs_str(sw as png_const_charp).unwrap_or_else(|| "-".into()),
                rs_str(sh as png_const_charp).unwrap_or_else(|| "-".into()),
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let w = cs("1.5");
        let h = cs("2.5");
        unsafe { (api.png_set_sCAL_s)(png, info, 1, w.as_ptr(), h.as_ptr()) }
    }
    matrix("sCAL getters", setup, body);

    // row 379: the stored width/height strings are not representable as fixed
    // point -> png_fixed -> png_error("fixed point overflow in sCAL width").
    for (w, h) in [
        ("1e10", "1"),
        ("1", "1e10"),
        ("1e300", "1e300"),
        ("21475", "1"),
        ("21474.83648", "1"),
        ("21474.83647", "1"),
        ("0.00000001", "0.00000001"),
    ] {
        let cw = cs(w);
        let ch = cs(h);
        same!(format!("get_sCAL_fixed overflow({},{})", w, h), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let set = guard(|| (api.png_set_sCAL_s)(s.png, s.info, 1, cw.as_ptr(), ch.as_ptr()))
                .is_some();
            let mut u: c_int = -12345;
            let mut fw: png_fixed_point = -12345;
            let mut fh: png_fixed_point = -12345;
            let r = (api.png_get_sCAL_fixed)(s.png, s.info, &mut u, &mut fw, &mut fh);
            format!("set={} r={} {} {} {}", set, r, u, fw, fh)
        });
    }
}

/// rows 386, 387, 388
#[test]
fn get_plte_rejections() {
    // NOTE: png_get_PLTE stores through `num_palette` without checking it for
    // NULL (pngget.c:1140 -- only `palette` is checked at :1137), so calling it
    // with num_palette == NULL while PNG_INFO_PLTE is set is C undefined
    // behaviour, not an error path, and is not tested.
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut pal: png_colorp = 0x1 as png_colorp;
            let mut n: c_int = -12345;
            let r = (api.png_get_PLTE)(png, info, &mut pal, &mut n);
            // row 388: palette == NULL -> return 0, num_palette untouched
            let mut n2: c_int = -12345;
            let r2 = (api.png_get_PLTE)(png, info, std::ptr::null_mut(), &mut n2);
            format!(
                "r={} pal={} n={} nullpal={}/{}",
                r,
                if pal as usize == 1 { "untouched" } else { nn(pal) },
                n,
                r2,
                n2
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let pal = [png_color { red: 1, green: 2, blue: 3 }; 4];
        unsafe { (api.png_set_PLTE)(png as png_structp, info, pal.as_ptr(), 4) }
    }
    matrix("PLTE getters", setup, body);
}

/// rows 389, 390, 391
#[test]
fn get_sbit_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut sb: png_color_8p = 0x1 as png_color_8p;
            let r = (api.png_get_sBIT)(png, info, &mut sb);
            let r2 = (api.png_get_sBIT)(png, info, std::ptr::null_mut());
            format!(
                "r={} sb={} contents={} null={}",
                r,
                if sb as usize == 1 { "untouched" } else { nn(sb) },
                if sb.is_null() || sb as usize == 1 {
                    "-".to_string()
                } else {
                    format!("{:?}", *sb)
                },
                r2
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let sb = png_color_8 {
            red: 8,
            green: 8,
            blue: 8,
            gray: 0,
            alpha: 0,
        };
        unsafe { (api.png_set_sBIT)(png, info, &sb) }
    }
    matrix("sBIT getters", setup, body);
}

/// row 392
#[test]
fn get_text_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut tp: png_textp = 0x1 as png_textp;
            let mut n: c_int = -12345;
            let r = (api.png_get_text)(png, info, &mut tp, &mut n);
            let mut n2: c_int = -12345;
            let r2 = (api.png_get_text)(png, info, std::ptr::null_mut(), &mut n2);
            let r3 = (api.png_get_text)(png, info, std::ptr::null_mut(), std::ptr::null_mut());
            format!(
                "r={} tp={} n={} nulltext={}/{} allnull={}",
                r,
                if tp as usize == 1 { "untouched" } else { nn(tp) },
                n,
                r2,
                n2,
                r3
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let key = cs("Title");
        let val = cs("hello");
        let t = png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: key.as_ptr() as png_charp,
            text: val.as_ptr() as png_charp,
            text_length: 0,
            itxt_length: 0,
            lang: std::ptr::null_mut(),
            lang_key: std::ptr::null_mut(),
        };
        unsafe { (api.png_set_text)(png, info, &t, 1) }
    }
    matrix("text getters", setup, body);
}

/// rows 393, 394, 395
#[test]
fn get_time_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut t: png_timep = 0x1 as png_timep;
            let r = (api.png_get_tIME)(png, info, &mut t);
            let r2 = (api.png_get_tIME)(png, info, std::ptr::null_mut());
            format!(
                "r={} t={} contents={} null={}",
                r,
                if t as usize == 1 { "untouched" } else { nn(t) },
                if t.is_null() || t as usize == 1 {
                    "-".to_string()
                } else {
                    format!("{:?}", *t)
                },
                r2
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let t = png_time {
            year: 2020,
            month: 6,
            day: 15,
            hour: 12,
            minute: 30,
            second: 45,
        };
        unsafe { (api.png_set_tIME)(png, info, &t) }
    }
    matrix("tIME getters", setup, body);
}

/// rows 396, 397, 398, 399
#[test]
fn get_trns_rejections() {
    // Every combination of colour type x which out-parameters are NULL.
    for &ct in &[
        PNG_COLOR_TYPE_PALETTE,
        PNG_COLOR_TYPE_GRAY,
        PNG_COLOR_TYPE_RGB,
        PNG_COLOR_TYPE_GRAY_ALPHA,
        PNG_COLOR_TYPE_RGB_ALPHA,
    ] {
        for set in [false, true] {
            for mask in 0u32..8 {
                same!(
                    format!("get_tRNS(ct={} set={} mask={})", ct, set, mask),
                    |api| unsafe {
                        let bd = if ct == PNG_COLOR_TYPE_PALETTE { 8 } else { 8 };
                        let s = wsess(api, ct, bd);
                        if set {
                            let alpha = [0u8, 1, 2, 3];
                            let tc = png_color_16 {
                                index: 0,
                                red: 1,
                                green: 2,
                                blue: 3,
                                gray: 4,
                            };
                            if ct == PNG_COLOR_TYPE_PALETTE {
                                let pal = [png_color::default(); 4];
                                (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), 4);
                                (api.png_set_tRNS)(s.png, s.info, alpha.as_ptr(), 4, &tc);
                            } else {
                                (api.png_set_tRNS)(s.png, s.info, std::ptr::null(), 0, &tc);
                            }
                        }
                        let mut ta: png_bytep = 0x1 as png_bytep;
                        let mut n: c_int = -12345;
                        let mut tc: png_color_16p = 0x1 as png_color_16p;
                        let pa =
                            if mask & 1 != 0 { &mut ta as *mut _ } else { std::ptr::null_mut() };
                        let pn =
                            if mask & 2 != 0 { &mut n as *mut _ } else { std::ptr::null_mut() };
                        let pc =
                            if mask & 4 != 0 { &mut tc as *mut _ } else { std::ptr::null_mut() };
                        let r = (api.png_get_tRNS)(s.png, s.info, pa, pn, pc);
                        format!(
                            "r={} ta={} n={} tc={}",
                            r,
                            if ta as usize == 1 { "untouched" } else { nn(ta) },
                            n,
                            if tc as usize == 1 { "untouched" } else { nn(tc) },
                        )
                    }
                );
            }
        }
    }
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut ta: png_bytep = 0x1 as png_bytep;
            let mut n: c_int = -12345;
            let mut tc: png_color_16p = 0x1 as png_color_16p;
            let r = (api.png_get_tRNS)(png, info, &mut ta, &mut n, &mut tc);
            format!(
                "r={} ta={} n={} tc={}",
                r,
                if ta as usize == 1 { "untouched" } else { nn(ta) },
                n,
                if tc as usize == 1 { "untouched" } else { nn(tc) },
            )
        }
    }
    matrix("tRNS getters", setup_none, body);
}

/// rows 400, 401
#[test]
fn get_unknown_chunks_rejections() {
    fn body(api: &'static Api, png: png_structp, info: png_infop) -> String {
        unsafe {
            let mut u: png_unknown_chunkp = 0x1 as png_unknown_chunkp;
            let r = (api.png_get_unknown_chunks)(png, info, &mut u);
            let r2 = (api.png_get_unknown_chunks)(png, info, std::ptr::null_mut());
            format!(
                "r={} u={} null={}",
                r,
                if u as usize == 1 { "untouched" } else { nn(u) },
                r2
            )
        }
    }
    fn setup(api: &'static Api, png: png_structp, info: png_infop) {
        let data = [1u8, 2, 3];
        let ch = png_unknown_chunk {
            name: [b'v', b'p', b'A', b'g', 0],
            data: data.as_ptr() as *mut png_byte,
            size: 3,
            location: PNG_HAVE_IHDR as png_byte,
        };
        unsafe { (api.png_set_unknown_chunks)(png, info, &ch, 1) }
    }
    matrix("unknown chunk getters", setup, body);
}

// ===========================================================================
// pngset.c
// ===========================================================================

const ALL_INFO_FLAGS: [png_uint_32; 20] = [
    PNG_INFO_gAMA,
    PNG_INFO_sBIT,
    PNG_INFO_cHRM,
    PNG_INFO_PLTE,
    PNG_INFO_tRNS,
    PNG_INFO_bKGD,
    PNG_INFO_hIST,
    PNG_INFO_pHYs,
    PNG_INFO_oFFs,
    PNG_INFO_tIME,
    PNG_INFO_pCAL,
    PNG_INFO_sRGB,
    PNG_INFO_iCCP,
    PNG_INFO_sPLT,
    PNG_INFO_sCAL,
    PNG_INFO_IDAT,
    PNG_INFO_eXIf,
    PNG_INFO_cICP,
    PNG_INFO_cLLI,
    PNG_INFO_mDCV,
];

/// The whole `info_ptr->valid` mask, reconstructed through png_get_valid.
unsafe fn validmask(api: &'static Api, png: png_structp, info: png_infop) -> png_uint_32 {
    if png.is_null() || info.is_null() {
        return 0;
    }
    let mut m = 0;
    for f in ALL_INFO_FLAGS {
        m |= (api.png_get_valid)(png, info, f);
    }
    m
}

/// row 410
#[test]
fn set_bkgd_rejections() {
    let bg = png_color_16 {
        index: 1,
        red: 2,
        green: 3,
        blue: 4,
        gray: 5,
    };
    // (png NULL, info NULL, background NULL) -- all 7 rejecting combinations
    for mask in 0u32..8 {
        same!(format!("png_set_bKGD nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            let b = if mask & 4 != 0 { &bg as *const _ } else { std::ptr::null() };
            (api.png_set_bKGD)(p, i, b);
            let mut out: png_color_16p = 0x1 as png_color_16p;
            let r = (api.png_get_bKGD)(s.png, s.info, &mut out);
            format!(
                "valid={:#x} r={} contents={}",
                validmask(api, s.png, s.info),
                r,
                if out.is_null() || out as usize == 1 {
                    "-".to_string()
                } else {
                    format!("{:?}", *out)
                }
            )
        });
    }
}

/// rows 411, 412, 413, 414, 415
#[test]
fn set_chrm_rejections() {
    // rows 411, 412: NULL png_ptr / info_ptr -> silent return
    for mask in 0u32..4 {
        same!(format!("png_set_cHRM_fixed nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_cHRM_fixed)(p, i, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
            (api.png_set_cHRM_XYZ_fixed)(
                p, i, 41239, 21264, 1933, 35758, 71517, 11919, 18048, 7219, 95053,
            );
            // in-range doubles only: with a NULL png_ptr, png_fixed()'s error path
            // would reach png_error(NULL) -> PNG_ABORT(), not an error return.
            (api.png_set_cHRM)(p, i, 0.3127, 0.329, 0.64, 0.33, 0.3, 0.6, 0.15, 0.06);
            (api.png_set_cHRM_XYZ)(p, i, 0.41, 0.21, 0.02, 0.36, 0.72, 0.12, 0.18, 0.07, 0.95);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 413: png_xy_from_XYZ fails -> png_app_error("invalid cHRM XYZ")
    let bad_xyz: [[png_fixed_point; 9]; 8] = [
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 1, 1, 1, 1, 1, 1, 1, 1],
        [-1, -1, -1, -1, -1, -1, -1, -1, -1],
        [i32::MAX; 9],
        [i32::MIN; 9],
        [41239, 21264, 1933, 35758, 71517, 11919, 18048, 7219, 95053], // good
        [0, 0, 0, 35758, 71517, 11919, 18048, 7219, 95053],
        [41239, 0, 0, 0, 0, 0, 0, 0, 0],
    ];
    for (n, v) in bad_xyz.iter().enumerate() {
        let v = *v;
        for benign in [0i32, 1] {
            same!(
                format!("png_set_cHRM_XYZ_fixed bad #{} benign={}", n, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    (api.png_set_cHRM_XYZ_fixed)(
                        s.png, s.info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
                    );
                    format!("valid={:#x}", validmask(api, s.png, s.info))
                }
            );
        }
    }
    // rows 414, 415: any double not representable as png_fixed_point ->
    // png_error("fixed point overflow in cHRM <name>").  One argument at a time,
    // so the *name* in the message is checked too.
    let overflow = [1e10f64, -1e10, 21475.0, -21475.0, f64::MAX, f64::MIN, 1e300];
    for &bad in &overflow {
        for hole in 0..9usize {
            same!(
                format!("png_set_cHRM overflow arg={} v={}", hole, bad),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let mut a = [0.3f64; 8];
                    if hole < 8 {
                        a[hole] = bad;
                    }
                    (api.png_set_cHRM)(
                        s.png, s.info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                    );
                    format!("valid={:#x}", validmask(api, s.png, s.info))
                }
            );
            same!(
                format!("png_set_cHRM_XYZ overflow arg={} v={}", hole, bad),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let mut a = [0.3f64; 9];
                    a[hole] = bad;
                    (api.png_set_cHRM_XYZ)(
                        s.png, s.info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
                    );
                    format!("valid={:#x}", validmask(api, s.png, s.info))
                }
            );
        }
    }
}

/// rows 416, 417
#[test]
fn set_cicp_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_cICP nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_cICP)(p, i, 9, 16, 0, 1);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 417: matrix_coefficients != 0 -> warning, PNG_INFO_cICP not set.
    // Every byte value of every parameter that has a documented small range.
    for mc in [0u8, 1, 2, 3, 14, 255] {
        for cp in [0u8, 1, 2, 9, 255] {
            for tf in [0u8, 1, 2, 16, 255] {
                for vfr in [0u8, 1, 2, 255] {
                    same!(
                        format!("png_set_cICP({},{},{},{})", cp, tf, mc, vfr),
                        |api| unsafe {
                            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                            (api.png_set_cICP)(s.png, s.info, cp, tf, mc, vfr);
                            let mut a: png_byte = 0xaa;
                            let mut b: png_byte = 0xaa;
                            let mut c: png_byte = 0xaa;
                            let mut d: png_byte = 0xaa;
                            let r = (api.png_get_cICP)(
                                s.png, s.info, &mut a, &mut b, &mut c, &mut d,
                            );
                            format!(
                                "valid={:#x} r={} {} {} {} {}",
                                validmask(api, s.png, s.info),
                                r,
                                a,
                                b,
                                c,
                                d
                            )
                        }
                    );
                }
            }
        }
    }
}

/// rows 418, 419, 420
#[test]
fn set_clli_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_cLLI_fixed nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_cLLI_fixed)(p, i, 1000, 100);
            // in-range doubles only (see the cHRM note about png_error(NULL))
            (api.png_set_cLLI)(p, i, 1.0, 0.5);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 419: maxCLL / maxFALL > 0x7FFFFFFF -> png_chunk_report(WRITE_ERROR)
    let vals = [
        0u32,
        1,
        10_000,
        0x7fff_ffff,
        0x8000_0000,
        0x8000_0001,
        0xffff_ffff,
    ];
    for &a in &vals {
        for &b in &vals {
            for benign in [0i32, 1] {
                for read in [false, true] {
                    same!(
                        format!("png_set_cLLI_fixed({:#x},{:#x}) benign={} read={}", a, b, benign, read),
                        |api| unsafe {
                            let (png, info, _kr, _kw);
                            if read {
                                let s = ReadSess::new(api, &[]);
                                png = s.png;
                                info = s.info;
                                _kr = Some(s);
                                _kw = None;
                            } else {
                                let s = WriteSess::new(api);
                                png = s.png;
                                info = s.info;
                                _kr = None;
                                _kw = Some(s);
                            }
                            (api.png_set_benign_errors)(png, benign);
                            (api.png_set_cLLI_fixed)(png, info, a, b);
                            let mut x: png_uint_32 = 0xdead_beef;
                            let mut y: png_uint_32 = 0xdead_beef;
                            let r = (api.png_get_cLLI_fixed)(png, info, &mut x, &mut y);
                            format!(
                                "valid={:#x} r={} {} {}",
                                validmask(api, png, info),
                                r,
                                x,
                                y
                            )
                        }
                    );
                }
            }
        }
    }
    // row 420: negative or too-large double -> png_error("fixed point overflow
    // in png_set_cLLI(maxCLL)") / "(maxFALL)"
    for &v in &[
        -1.0f64,
        -0.0001,
        0.0,
        1.0,
        214748.3647,
        214748.3648,
        1e10,
        f64::MAX,
        f64::MIN,
    ] {
        for hole in 0..2usize {
            same!(format!("png_set_cLLI({} at {})", v, hole), |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                let mut a = [1.0f64; 2];
                a[hole] = v;
                (api.png_set_cLLI)(s.png, s.info, a[0], a[1]);
                format!("valid={:#x}", validmask(api, s.png, s.info))
            });
        }
    }
}

/// rows 421, 422, 423, 424, 425, 426
#[test]
fn set_mdcv_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_mDCV_fixed nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_mDCV_fixed)(
                p, i, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 1000, 1,
            );
            (api.png_set_mDCV)(
                p, i, 0.3127, 0.329, 0.64, 0.33, 0.3, 0.6, 0.15, 0.06, 1.0, 0.1,
            );
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // rows 421, 423: png_ITU_fixed_16 rejects v/2 > 65535 or v/2 < 0.  Note that
    // C integer division truncates towards zero, so -1/2 == 0 is *accepted*.
    let chroma = [
        i32::MIN,
        -131072,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        65535,
        131070,
        131071,
        131072,
        131073,
        i32::MAX,
    ];
    for &v in &chroma {
        for hole in 0..8usize {
            for benign in [0i32, 1] {
                same!(
                    format!("png_set_mDCV_fixed chroma[{}]={} benign={}", hole, v, benign),
                    |api| unsafe {
                        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                        (api.png_set_benign_errors)(s.png, benign);
                        let mut a = [31270i32; 8];
                        a[hole] = v;
                        (api.png_set_mDCV_fixed)(
                            s.png, s.info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], 1000,
                            1,
                        );
                        let mut f = [-1i32; 8];
                        let mut dl: png_uint_32 = 0xdead_beef;
                        let mut ml: png_uint_32 = 0xdead_beef;
                        let r = (api.png_get_mDCV_fixed)(
                            s.png, s.info, &mut f[0], &mut f[1], &mut f[2], &mut f[3], &mut f[4],
                            &mut f[5], &mut f[6], &mut f[7], &mut dl, &mut ml,
                        );
                        format!(
                            "valid={:#x} r={} {:?} {} {}",
                            validmask(api, s.png, s.info),
                            r,
                            f,
                            dl,
                            ml
                        )
                    }
                );
            }
        }
    }
    // row 424: maxDL / minDL > 0x7FFFFFFF
    for &dl in &[0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        for hole in 0..2usize {
            for benign in [0i32, 1] {
                same!(
                    format!("png_set_mDCV_fixed DL[{}]={:#x} benign={}", hole, dl, benign),
                    |api| unsafe {
                        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                        (api.png_set_benign_errors)(s.png, benign);
                        let mut d = [1000u32; 2];
                        d[hole] = dl;
                        (api.png_set_mDCV_fixed)(
                            s.png, s.info, 31270, 32900, 64000, 33000, 30000, 60000, 15000,
                            6000, d[0], d[1],
                        );
                        format!("valid={:#x}", validmask(api, s.png, s.info))
                    }
                );
            }
        }
    }
    // rows 425, 426: the floating-point entry point, one bad argument at a time
    // (checks the per-argument name in the png_error message).
    for &bad in &[1e10f64, -1e10, 21475.0, -1.0, f64::MAX, f64::MIN] {
        for hole in 0..10usize {
            same!(format!("png_set_mDCV arg[{}]={}", hole, bad), |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                let mut a = [0.3f64; 10];
                a[8] = 1.0;
                a[9] = 0.1;
                a[hole] = bad;
                (api.png_set_mDCV)(
                    s.png, s.info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9],
                );
                format!("valid={:#x}", validmask(api, s.png, s.info))
            });
        }
    }
}

/// rows 427, 428, 429, 430, 431
#[test]
fn set_exif_rejections() {
    // row 427: png_set_eXIf is permanently disabled -> warning, nothing stored
    same!("png_set_eXIf", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        let mut data = [b'I', b'I', 42u8, 0, 8, 0, 0, 0];
        (api.png_set_eXIf)(s.png, s.info, data.as_mut_ptr());
        format!("valid={:#x}", validmask(api, s.png, s.info))
    });
    // rows 428, 430: NULL png_ptr / info_ptr / exif
    for mask in 0u32..8 {
        same!(format!("png_set_eXIf_1 nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let mut data = [b'I', b'I', 42u8, 0, 8, 0, 0, 0];
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            let e = if mask & 4 != 0 { data.as_mut_ptr() } else { std::ptr::null_mut() };
            (api.png_set_eXIf_1)(p, i, 8, e);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 429: (png_ptr->mode & PNG_WROTE_eXIf) != 0.  The flag is only set by
    // png_write_eXIf, so a full write is needed to reach it.
    same!("png_set_eXIf_1 after the chunk was written", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        let data = [b'I', b'I', 42u8, 0, 8, 0, 0, 0];
        (api.png_set_eXIf_1)(s.png, s.info, 8, data.as_ptr() as png_bytep);
        (api.png_write_info)(s.png, s.info);
        let row = vec![0u8; 8 * 3];
        for _ in 0..8 {
            (api.png_write_row)(s.png, row.as_ptr());
        }
        (api.png_write_end)(s.png, s.info);
        let data2 = [b'M', b'M', 0u8, 42, 0, 0, 0, 8];
        (api.png_set_eXIf_1)(s.png, s.info, 8, data2.as_ptr() as png_bytep);
        let mut n: png_uint_32 = 0xdead_beef;
        let mut p: png_bytep = std::ptr::null_mut();
        let r = (api.png_get_eXIf_1)(s.png, s.info, &mut n, &mut p);
        format!(
            "valid={:#x} r={} n={} first={}",
            validmask(api, s.png, s.info),
            r,
            n,
            if p.is_null() { 0u8 } else { *p }
        )
    });
    // row 431: png_malloc_warn returns NULL -> warning, PNG_INFO_eXIf not set
    for n in [0u32, 1, 8, 1000] {
        same!(format!("png_set_eXIf_1 OOM (num_exif={})", n), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let data = vec![7u8; 1024];
            starve(api, s.png, 0);
            (api.png_set_eXIf_1)(s.png, s.info, n, data.as_ptr() as png_bytep);
            unstarve(api, s.png);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
}

/// rows 432, 433
#[test]
fn set_gama_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_gAMA_fixed nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_gAMA_fixed)(p, i, 45455);
            (api.png_set_gAMA)(p, i, 0.45455); // in-range only, see cHRM note
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // png_set_gAMA_fixed stores anything, including nonsense
    for v in [
        i32::MIN,
        -100_000,
        -1,
        0,
        1,
        45455,
        100_000,
        i32::MAX,
        PNG_DEFAULT_sRGB,
        PNG_GAMMA_MAC_18,
    ] {
        same!(format!("png_set_gAMA_fixed({})", v), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_gAMA_fixed)(s.png, s.info, v);
            let mut g: png_fixed_point = -12345;
            let r = (api.png_get_gAMA_fixed)(s.png, s.info, &mut g);
            format!("valid={:#x} r={} g={}", validmask(api, s.png, s.info), r, g)
        });
    }
    // row 433: floor(100000*fp+.5) out of png_fixed_point range
    for v in [
        0.0f64,
        1.0,
        -1.0,
        21474.83647,
        21474.83648,
        -21474.83648,
        1e10,
        -1e10,
        f64::MAX,
        f64::MIN,
        1e-10,
    ] {
        same!(format!("png_set_gAMA({})", v), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_gAMA)(s.png, s.info, v);
            let mut g: png_fixed_point = -12345;
            let r = (api.png_get_gAMA_fixed)(s.png, s.info, &mut g);
            format!("valid={:#x} r={} g={}", validmask(api, s.png, s.info), r, g)
        });
    }
}

/// rows 434, 435, 436
#[test]
fn set_hist_rejections() {
    let hist = [1u16; 256];
    // row 434: NULL png_ptr / info_ptr / hist
    for mask in 0u32..8 {
        same!(format!("png_set_hIST nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
            let pal = [png_color::default(); 4];
            (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), 4);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            let h = if mask & 4 != 0 { hist.as_ptr() } else { std::ptr::null() };
            (api.png_set_hIST)(p, i, h);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 435: hIST set before (or without) a valid PLTE -> warning, not stored
    for np in [0i32, 1, 2, 4, 256] {
        same!(format!("png_set_hIST with num_palette={}", np), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
            if np > 0 {
                let pal = vec![png_color::default(); 256];
                (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), np);
            }
            (api.png_set_hIST)(s.png, s.info, hist.as_ptr());
            let mut h: png_uint_16p = std::ptr::null_mut();
            let r = (api.png_get_hIST)(s.png, s.info, &mut h);
            format!(
                "valid={:#x} r={} h={}",
                validmask(api, s.png, s.info),
                r,
                nn(h)
            )
        });
    }
    // hIST on a non-palette image (num_palette stays 0) -> same warning
    same!("png_set_hIST on an RGB image", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        (api.png_set_hIST)(s.png, s.info, hist.as_ptr());
        format!("valid={:#x}", validmask(api, s.png, s.info))
    });
    // row 436: png_malloc_warn for PNG_MAX_PALETTE_LENGTH entries fails
    same!("png_set_hIST OOM", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
        let pal = [png_color::default(); 4];
        (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), 4);
        starve(api, s.png, 0);
        (api.png_set_hIST)(s.png, s.info, hist.as_ptr());
        unstarve(api, s.png);
        let mut h: png_uint_16p = std::ptr::null_mut();
        let r = (api.png_get_hIST)(s.png, s.info, &mut h);
        format!(
            "valid={:#x} r={} h={}",
            validmask(api, s.png, s.info),
            r,
            nn(h)
        )
    });
}

/// rows 437, 438  (png_check_IHDR itself is exercised exhaustively in t10)
#[test]
fn set_ihdr_rejections_setget() {
    for mask in 0u32..4 {
        same!(format!("png_set_IHDR nulls mask={}", mask), |api| unsafe {
            let s = WriteSess::new(api);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_IHDR)(p, i, 8, 8, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            format!(
                "w={} h={} bd={} ct={} ch={} rb={}",
                (api.png_get_image_width)(s.png, s.info),
                (api.png_get_image_height)(s.png, s.info),
                (api.png_get_bit_depth)(s.png, s.info),
                (api.png_get_color_type)(s.png, s.info),
                (api.png_get_channels)(s.png, s.info),
                (api.png_get_rowbytes)(s.png, s.info),
            )
        });
    }
    // row 438: the derived fields (channels / pixel_depth / rowbytes) after a
    // rejected IHDR -- the assignments happen *before* png_check_IHDR.
    for (w, h, bd, ct) in [
        (0u32, 4u32, 8, PNG_COLOR_TYPE_RGB),
        (4, 4, 3, PNG_COLOR_TYPE_GRAY),
        (4, 4, 8, 7),
        (4, 4, 1, PNG_COLOR_TYPE_RGB),
        (4, 4, 16, PNG_COLOR_TYPE_PALETTE),
        (0x8000_0000, 4, 8, PNG_COLOR_TYPE_RGB),
    ] {
        same!(
            format!("png_set_IHDR rejected({},{},{},{})", w, h, bd, ct),
            |api| unsafe {
                let s = WriteSess::new(api);
                let ok = guard(|| (api.png_set_IHDR)(s.png, s.info, w, h, bd, ct, 0, 0, 0))
                    .is_some();
                format!(
                    "ok={} w={} h={} bd={} ct={} ch={} rb={}",
                    ok,
                    (api.png_get_image_width)(s.png, s.info),
                    (api.png_get_image_height)(s.png, s.info),
                    (api.png_get_bit_depth)(s.png, s.info),
                    (api.png_get_color_type)(s.png, s.info),
                    (api.png_get_channels)(s.png, s.info),
                    (api.png_get_rowbytes)(s.png, s.info),
                )
            }
        );
    }
}

/// row 439
#[test]
fn set_offs_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_oFFs nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_oFFs)(p, i, 1, 2, PNG_OFFSET_PIXEL);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // png_set_oFFs performs no validation at all: the unit type is truncated to
    // a byte and stored verbatim.
    for ut in [-1i32, 0, 1, 2, 3, 255, 256, 257, 1000, i32::MIN, i32::MAX] {
        same!(format!("png_set_oFFs(unit={})", ut), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_oFFs)(s.png, s.info, i32::MIN, i32::MAX, ut);
            let mut ox: png_int_32 = -12345;
            let mut oy: png_int_32 = -12345;
            let mut ou: c_int = -12345;
            let r = (api.png_get_oFFs)(s.png, s.info, &mut ox, &mut oy, &mut ou);
            format!(
                "valid={:#x} r={} {} {} {} xp={} xm={}",
                validmask(api, s.png, s.info),
                r,
                ox,
                oy,
                ou,
                (api.png_get_x_offset_pixels)(s.png, s.info),
                (api.png_get_x_offset_microns)(s.png, s.info),
            )
        });
    }
}

/// rows 440..447
#[test]
fn set_pcal_rejections() {
    // row 440: png_ptr / info_ptr / purpose / units NULL, or nparams > 0 with
    // params == NULL
    for mask in 0u32..16 {
        same!(format!("png_set_pCAL nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let purpose = cs("purpose");
            let units = cs("units");
            let p0 = cs("1.5");
            let mut params = [p0.as_ptr() as png_charp];
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            let pu = if mask & 4 != 0 { purpose.as_ptr() } else { std::ptr::null() };
            let un = if mask & 8 != 0 { units.as_ptr() } else { std::ptr::null() };
            (api.png_set_pCAL)(p, i, pu, 0, 10, 0, 1, un, params.as_mut_ptr());
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    same!("png_set_pCAL nparams>0 with params=NULL", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        let purpose = cs("purpose");
        let units = cs("units");
        (api.png_set_pCAL)(
            s.png,
            s.info,
            purpose.as_ptr(),
            0,
            10,
            0,
            1,
            units.as_ptr(),
            std::ptr::null_mut(),
        );
        format!("valid={:#x}", validmask(api, s.png, s.info))
    });
    same!("png_set_pCAL nparams=0 with params=NULL", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        let purpose = cs("purpose");
        let units = cs("units");
        (api.png_set_pCAL)(
            s.png,
            s.info,
            purpose.as_ptr(),
            0,
            10,
            0,
            0,
            units.as_ptr(),
            std::ptr::null_mut(),
        );
        format!("valid={:#x}", validmask(api, s.png, s.info))
    });
    // rows 441, 442: equation type outside 0..3, parameter count outside 0..255
    for ty in [-1i32, 0, 1, 2, 3, 4, 5, 100, i32::MIN, i32::MAX] {
        for np in [-1i32, 0, 1, 2, 255, 256, 1000, i32::MIN, i32::MAX] {
            for benign in [0i32, 1] {
                same!(
                    format!("png_set_pCAL(type={},nparams={},benign={})", ty, np, benign),
                    |api| unsafe {
                        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                        (api.png_set_benign_errors)(s.png, benign);
                        let purpose = cs("purpose");
                        let units = cs("units");
                        let p0 = cs("1.5");
                        // Only the first min(np,4) entries are ever read; the
                        // rejecting paths return before the loop.
                        let mut params = [p0.as_ptr() as png_charp; 4];
                        (api.png_set_pCAL)(
                            s.png,
                            s.info,
                            purpose.as_ptr(),
                            -5,
                            5,
                            ty,
                            if np > 4 { 4 } else { np },
                            units.as_ptr(),
                            params.as_mut_ptr(),
                        );
                        let r = report_pcal(api, s.png, s.info);
                        format!("valid={:#x} {}", validmask(api, s.png, s.info), r)
                    }
                );
            }
        }
    }
    // The real out-of-range nparams values (they must not be clamped by the
    // test, so params points at a 256-entry array that is never read).
    for np in [256i32, 257, 1000, i32::MAX] {
        same!(format!("png_set_pCAL(nparams={}) unclamped", np), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let purpose = cs("purpose");
            let units = cs("units");
            let p0 = cs("1.5");
            let mut params = vec![p0.as_ptr() as png_charp; 256];
            (api.png_set_pCAL)(
                s.png,
                s.info,
                purpose.as_ptr(),
                0,
                1,
                0,
                np,
                units.as_ptr(),
                params.as_mut_ptr(),
            );
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 443: params[i] == NULL, or not a valid PNG floating-point string
    let strings = [
        "1.5", "-2", "+3", "1e5", "1E5", "1e+5", "1e-5", ".5", "5.", "", " ", "abc", "1.2.3",
        "--1", "1e", "1e+", "0x10", "1 ", "NaN", "inf", "1,5",
    ];
    for sv in strings {
        let cv = cs(sv);
        for benign in [0i32, 1] {
            same!(
                format!("png_set_pCAL param={:?} benign={}", sv, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    let purpose = cs("purpose");
                    let units = cs("units");
                    let mut params = [cv.as_ptr() as png_charp];
                    (api.png_set_pCAL)(
                        s.png,
                        s.info,
                        purpose.as_ptr(),
                        0,
                        1,
                        0,
                        1,
                        units.as_ptr(),
                        params.as_mut_ptr(),
                    );
                    format!(
                        "valid={:#x} {}",
                        validmask(api, s.png, s.info),
                        report_pcal(api, s.png, s.info)
                    )
                }
            );
        }
    }
    for benign in [0i32, 1] {
        same!(format!("png_set_pCAL param=NULL benign={}", benign), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_benign_errors)(s.png, benign);
            let purpose = cs("purpose");
            let units = cs("units");
            let good = cs("1");
            let mut params = [good.as_ptr() as png_charp, std::ptr::null_mut()];
            (api.png_set_pCAL)(
                s.png,
                s.info,
                purpose.as_ptr(),
                0,
                1,
                0,
                2,
                units.as_ptr(),
                params.as_mut_ptr(),
            );
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // rows 444..447: allocation failure at each of the four allocation sites
    // (purpose, units, the params array, one params[i]).
    for budget in [0i64, 1, 2, 3, 4, 5] {
        for benign in [0i32, 1] {
            same!(
                format!("png_set_pCAL OOM budget={} benign={}", budget, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    let purpose = cs("purpose");
                    let units = cs("units");
                    let p0 = cs("1");
                    let p1 = cs("2");
                    let mut params = [p0.as_ptr() as png_charp, p1.as_ptr() as png_charp];
                    starve(api, s.png, budget);
                    (api.png_set_pCAL)(
                        s.png,
                        s.info,
                        purpose.as_ptr(),
                        0,
                        1,
                        0,
                        2,
                        units.as_ptr(),
                        params.as_mut_ptr(),
                    );
                    unstarve(api, s.png);
                    format!(
                        "valid={:#x} {}",
                        validmask(api, s.png, s.info),
                        report_pcal(api, s.png, s.info)
                    )
                }
            );
        }
    }
}

unsafe fn report_pcal(api: &'static Api, png: png_structp, info: png_infop) -> String {
    let mut purpose: png_charp = std::ptr::null_mut();
    let mut x0: png_int_32 = -12345;
    let mut x1: png_int_32 = -12345;
    let mut ty: c_int = -12345;
    let mut np: c_int = -12345;
    let mut units: png_charp = std::ptr::null_mut();
    let mut params: png_charpp = std::ptr::null_mut();
    let r = (api.png_get_pCAL)(
        png,
        info,
        &mut purpose,
        &mut x0,
        &mut x1,
        &mut ty,
        &mut np,
        &mut units,
        &mut params,
    );
    format!(
        "pCAL r={} purpose={:?} {} {} {} {} units={:?}",
        r,
        rs_str(purpose as png_const_charp),
        x0,
        x1,
        ty,
        np,
        rs_str(units as png_const_charp),
    )
}

/// rows 448..457
#[test]
fn set_scal_rejections() {
    // row 448: NULL png_ptr / info_ptr -> silent return (before the unit check!)
    for mask in 0u32..4 {
        same!(format!("png_set_sCAL_s nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let w = cs("1");
            let h = cs("2");
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            // unit 99 would png_error, but only when png_ptr and info_ptr are
            // both non-NULL; with a real png_ptr the error is captured.
            (api.png_set_sCAL_s)(p, i, 1, w.as_ptr(), h.as_ptr());
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 449: unit != 1 && unit != 2
    for unit in [-1i32, 0, 1, 2, 3, 4, 99, 255, 256, i32::MIN, i32::MAX] {
        same!(format!("png_set_sCAL_s(unit={})", unit), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let w = cs("1.5");
            let h = cs("2.5");
            (api.png_set_sCAL_s)(s.png, s.info, unit, w.as_ptr(), h.as_ptr());
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
        same!(format!("png_set_sCAL(unit={})", unit), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_sCAL)(s.png, s.info, unit, 1.5, 2.5);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
        same!(format!("png_set_sCAL_fixed(unit={})", unit), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_sCAL_fixed)(s.png, s.info, unit, 150_000, 250_000);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // rows 450, 451: empty / negative / malformed width and height strings
    let strings = [
        "1", "1.5", "0", "0.0", "-1", "-0.5", "", " ", "+1", ".5", "5.", "1e5", "1e-5", "abc",
        "1.2.3", "1 ", " 1", "1e", "--1", "0x1", "NaN", "inf",
    ];
    for sw in strings {
        for sh in ["1", "", "-1", "abc"] {
            let cw = cs(sw);
            let ch = cs(sh);
            same!(format!("png_set_sCAL_s({:?},{:?})", sw, sh), |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                (api.png_set_sCAL_s)(s.png, s.info, 1, cw.as_ptr(), ch.as_ptr());
                format!(
                    "valid={:#x} {}",
                    validmask(api, s.png, s.info),
                    report_scal(api, s.png, s.info)
                )
            });
        }
    }
    for which in 0..3usize {
        same!(format!("png_set_sCAL_s NULL string {}", which), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let g = cs("1");
            let w = if which == 0 { std::ptr::null() } else { g.as_ptr() };
            let h = if which == 1 { std::ptr::null() } else { g.as_ptr() };
            (api.png_set_sCAL_s)(s.png, s.info, 1, w, h);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // rows 454..457: width <= 0 / height <= 0 in the float and fixed wrappers
    for w in [-1.0f64, -0.0, 0.0, 1e-30, 1.0, 1e10, f64::MAX, f64::MIN] {
        for h in [-1.0f64, 0.0, 1.0] {
            same!(format!("png_set_sCAL({},{})", w, h), |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                (api.png_set_sCAL)(s.png, s.info, 1, w, h);
                format!(
                    "valid={:#x} {}",
                    validmask(api, s.png, s.info),
                    report_scal(api, s.png, s.info)
                )
            });
        }
    }
    for w in [i32::MIN, -1i32, 0, 1, 100_000, i32::MAX] {
        for h in [i32::MIN, -1i32, 0, 1, 100_000, i32::MAX] {
            same!(format!("png_set_sCAL_fixed({},{})", w, h), |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                (api.png_set_sCAL_fixed)(s.png, s.info, 1, w, h);
                format!(
                    "valid={:#x} {}",
                    validmask(api, s.png, s.info),
                    report_scal(api, s.png, s.info)
                )
            });
        }
    }
    // rows 452, 453: allocation failure for scal_s_width / scal_s_height
    for budget in [0i64, 1, 2] {
        same!(format!("png_set_sCAL_s OOM budget={}", budget), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let w = cs("1.5");
            let h = cs("2.5");
            starve(api, s.png, budget);
            (api.png_set_sCAL_s)(s.png, s.info, 1, w.as_ptr(), h.as_ptr());
            unstarve(api, s.png);
            format!(
                "valid={:#x} {}",
                validmask(api, s.png, s.info),
                report_scal(api, s.png, s.info)
            )
        });
    }
}

unsafe fn report_scal(api: &'static Api, png: png_structp, info: png_infop) -> String {
    if (api.png_get_valid)(png, info, PNG_INFO_sCAL) == 0 {
        return "sCAL invalid".to_string();
    }
    let mut u: c_int = -12345;
    let mut w: png_charp = std::ptr::null_mut();
    let mut h: png_charp = std::ptr::null_mut();
    let r = (api.png_get_sCAL_s)(png, info, &mut u, &mut w, &mut h);
    format!(
        "sCAL r={} unit={} w={:?} h={:?}",
        r,
        u,
        rs_str(w as png_const_charp),
        rs_str(h as png_const_charp)
    )
}

/// row 458
#[test]
fn set_phys_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_pHYs nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_pHYs)(p, i, 100, 100, PNG_RESOLUTION_METER);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    for ut in [-1i32, 0, 1, 2, 3, 255, 256, 257, i32::MIN, i32::MAX] {
        same!(format!("png_set_pHYs(unit={})", ut), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_pHYs)(s.png, s.info, 0xffff_ffff, 0, ut);
            let mut rx: png_uint_32 = 0xdead_beef;
            let mut ry: png_uint_32 = 0xdead_beef;
            let mut ou: c_int = -12345;
            let r = (api.png_get_pHYs)(s.png, s.info, &mut rx, &mut ry, &mut ou);
            format!(
                "valid={:#x} r={} {} {} {}",
                validmask(api, s.png, s.info),
                r,
                rx,
                ry,
                ou
            )
        });
    }
}

/// rows 459, 460, 461, 462, 463
#[test]
fn set_plte_rejections() {
    let pal = vec![
        png_color {
            red: 9,
            green: 8,
            blue: 7
        };
        512
    ];
    // row 459
    for mask in 0u32..4 {
        same!(format!("png_set_PLTE nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_PLTE)(p, i, pal.as_ptr(), 4);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // rows 460, 461, 462, 463: the whole num_palette / colour-type / palette-NULL
    // / MNG-empty-PLTE decision table.
    let nums = [
        i32::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        8,
        9,
        16,
        17,
        255,
        256,
        257,
        1000,
        i32::MAX,
    ];
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_PALETTE, 1),
        (PNG_COLOR_TYPE_PALETTE, 2),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for &n in &nums {
            for null_pal in [false, true] {
                for mng in [false, true] {
                    same!(
                        format!(
                            "png_set_PLTE(ct={},bd={},n={},nullpal={},mng={})",
                            ct, bd, n, null_pal, mng
                        ),
                        |api| unsafe {
                            let s = wsess(api, ct, bd);
                            if mng {
                                (api.png_permit_mng_features)(s.png, PNG_ALL_MNG_FEATURES);
                            }
                            let p = if null_pal { std::ptr::null() } else { pal.as_ptr() };
                            // Guard the memcpy: only pass an n the C would copy
                            // when we actually have that many entries.
                            let n = if n > 512 { 512 } else { n };
                            (api.png_set_PLTE)(s.png, s.info, p, n);
                            let mut op: png_colorp = std::ptr::null_mut();
                            let mut on: c_int = -12345;
                            let r = (api.png_get_PLTE)(s.png, s.info, &mut op, &mut on);
                            format!(
                                "valid={:#x} r={} n={} first={:?}",
                                validmask(api, s.png, s.info),
                                r,
                                on,
                                if op.is_null() || on <= 0 {
                                    None
                                } else {
                                    Some(*op)
                                }
                            )
                        }
                    );
                }
            }
        }
    }
    // Truly out-of-range counts (no clamping), with a big enough source array so
    // that the *rejection* is what is observed, not a buffer overrun.
    for &n in &[513i32, 1000, 65536, i32::MAX] {
        same!(format!("png_set_PLTE unclamped n={}", n), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
            (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), n);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
        same!(format!("png_set_PLTE unclamped rgb n={}", n), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), n);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
}

/// row 464
#[test]
fn set_sbit_rejections() {
    for mask in 0u32..8 {
        same!(format!("png_set_sBIT nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let sb = png_color_8 {
                red: 8,
                green: 8,
                blue: 8,
                gray: 8,
                alpha: 8,
            };
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            let b = if mask & 4 != 0 { &sb as *const _ } else { std::ptr::null() };
            (api.png_set_sBIT)(p, i, b);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // png_set_sBIT performs *no* validation: every nonsensical significant-bit
    // depth is stored verbatim (the write code rejects them later).
    for depth in [0u8, 1, 8, 9, 16, 17, 255] {
        for &(ct, bd) in &[
            (PNG_COLOR_TYPE_GRAY, 1),
            (PNG_COLOR_TYPE_GRAY, 8),
            (PNG_COLOR_TYPE_RGB, 8),
            (PNG_COLOR_TYPE_RGB, 16),
            (PNG_COLOR_TYPE_PALETTE, 8),
            (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        ] {
            same!(
                format!("png_set_sBIT(depth={},ct={},bd={})", depth, ct, bd),
                |api| unsafe {
                    let s = wsess(api, ct, bd);
                    let sb = png_color_8 {
                        red: depth,
                        green: depth,
                        blue: depth,
                        gray: depth,
                        alpha: depth,
                    };
                    (api.png_set_sBIT)(s.png, s.info, &sb);
                    let mut out: png_color_8p = std::ptr::null_mut();
                    let r = (api.png_get_sBIT)(s.png, s.info, &mut out);
                    format!(
                        "valid={:#x} r={} {:?}",
                        validmask(api, s.png, s.info),
                        r,
                        if out.is_null() { None } else { Some(*out) }
                    )
                }
            );
        }
    }
}

/// rows 465, 466
#[test]
fn set_srgb_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_sRGB nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_sRGB)(p, i, 0);
            (api.png_set_sRGB_gAMA_and_cHRM)(p, i, 0);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // png_set_sRGB does NOT range-check the intent; the write code does.
    for intent in [-1i32, 0, 1, 2, 3, 4, 5, 100, 255, 256, i32::MIN, i32::MAX] {
        same!(format!("png_set_sRGB(intent={})", intent), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_sRGB)(s.png, s.info, intent);
            let mut i: c_int = -12345;
            let r = (api.png_get_sRGB)(s.png, s.info, &mut i);
            format!("valid={:#x} r={} i={}", validmask(api, s.png, s.info), r, i)
        });
        same!(
            format!("png_set_sRGB_gAMA_and_cHRM(intent={})", intent),
            |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                (api.png_set_sRGB_gAMA_and_cHRM)(s.png, s.info, intent);
                let mut i: c_int = -12345;
                let r = (api.png_get_sRGB)(s.png, s.info, &mut i);
                let mut g: png_fixed_point = -12345;
                let rg = (api.png_get_gAMA_fixed)(s.png, s.info, &mut g);
                let mut f = [-1i32; 8];
                let rc = (api.png_get_cHRM_fixed)(
                    s.png, s.info, &mut f[0], &mut f[1], &mut f[2], &mut f[3], &mut f[4],
                    &mut f[5], &mut f[6], &mut f[7],
                );
                format!(
                    "valid={:#x} r={} i={} gAMA={}/{} cHRM={}/{:?}",
                    validmask(api, s.png, s.info),
                    r,
                    i,
                    rg,
                    g,
                    rc,
                    f
                )
            }
        );
        // the write-time rejection of an out-of-range intent
        same!(format!("write sRGB(intent={})", intent), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_sRGB)(s.png, s.info, intent);
            let ok = guard(|| (api.png_write_info)(s.png, s.info)).is_some();
            format!("write_info_ok={} bytes={}", ok, s.sink.buf.len())
        });
    }
}

/// rows 467, 468, 469, 470
#[test]
fn set_iccp_rejections() {
    let prof: [u8; 32] = [
        0, 0, 0, 32, b'a', b'c', b's', b'p', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0,
    ];
    // row 467: NULL png_ptr / info_ptr / name / profile
    for mask in 0u32..16 {
        same!(format!("png_set_iCCP nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let name = cs("ICC");
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            let n = if mask & 4 != 0 { name.as_ptr() } else { std::ptr::null() };
            let pr = if mask & 8 != 0 { prof.as_ptr() } else { std::ptr::null() };
            (api.png_set_iCCP)(p, i, n, 0, pr, 32);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 468: compression_type != PNG_COMPRESSION_TYPE_BASE -> png_app_error
    for ct in [-1i32, 0, 1, 2, 8, 100, 255, i32::MIN, i32::MAX] {
        for benign in [0i32, 1] {
            same!(
                format!("png_set_iCCP(compression={},benign={})", ct, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    let name = cs("ICC profile");
                    (api.png_set_iCCP)(s.png, s.info, name.as_ptr(), ct, prof.as_ptr(), 32);
                    let mut on: png_charp = std::ptr::null_mut();
                    let mut oc: c_int = -12345;
                    let mut op: png_bytep = std::ptr::null_mut();
                    let mut ol: png_uint_32 = 0xdead_beef;
                    let r =
                        (api.png_get_iCCP)(s.png, s.info, &mut on, &mut oc, &mut op, &mut ol);
                    format!(
                        "valid={:#x} r={} name={:?} ct={} len={}",
                        validmask(api, s.png, s.info),
                        r,
                        rs_str(on as png_const_charp),
                        oc,
                        ol
                    )
                }
            );
        }
    }
    // keyword edge cases for the name (empty, spaces, too long, high bytes)
    for nm in [
        "",
        " ",
        "  ",
        " leading",
        "trailing ",
        "double  space",
        "\x01ctl",
        "\u{a0}nbsp",
        &"x".repeat(79),
        &"x".repeat(80),
        &"x".repeat(200),
    ] {
        let cn = cs(nm);
        same!(format!("png_set_iCCP(name={:?})", nm), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_iCCP)(s.png, s.info, cn.as_ptr(), 0, prof.as_ptr(), 32);
            let mut on: png_charp = std::ptr::null_mut();
            let mut oc: c_int = -12345;
            let mut op: png_bytep = std::ptr::null_mut();
            let mut ol: png_uint_32 = 0xdead_beef;
            let r = (api.png_get_iCCP)(s.png, s.info, &mut on, &mut oc, &mut op, &mut ol);
            format!(
                "valid={:#x} r={} name={:?} len={}",
                validmask(api, s.png, s.info),
                r,
                rs_str(on as png_const_charp),
                ol
            )
        });
    }
    // rows 469, 470: allocation failure for the name and for the profile
    for budget in [0i64, 1, 2] {
        for benign in [0i32, 1] {
            same!(
                format!("png_set_iCCP OOM budget={} benign={}", budget, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    let name = cs("ICC profile");
                    starve(api, s.png, budget);
                    (api.png_set_iCCP)(s.png, s.info, name.as_ptr(), 0, prof.as_ptr(), 32);
                    unstarve(api, s.png);
                    format!("valid={:#x}", validmask(api, s.png, s.info))
                }
            );
        }
    }
}

unsafe fn report_text(api: &'static Api, png: png_structp, info: png_infop) -> String {
    let mut tp: png_textp = std::ptr::null_mut();
    let mut n: c_int = -12345;
    let r = (api.png_get_text)(png, info, &mut tp, &mut n);
    let mut out = format!("text r={} n={}", r, n);
    if !tp.is_null() && n > 0 {
        for k in 0..n as isize {
            let t = &*tp.offset(k);
            out.push_str(&format!(
                " [c={} key={:?} text={:?} tl={} il={} lang={:?} lk={:?}]",
                t.compression,
                rs_str(t.key as png_const_charp),
                rs_str(t.text as png_const_charp),
                t.text_length,
                t.itxt_length,
                rs_str(t.lang as png_const_charp),
                rs_str(t.lang_key as png_const_charp),
            ));
        }
    }
    out
}

fn mk_text(compression: c_int, key: png_charp, text: png_charp, lang: png_charp) -> png_text {
    png_text {
        compression,
        key,
        text,
        text_length: 0,
        itxt_length: 0,
        lang,
        lang_key: lang,
    }
}

/// rows 471..478
#[test]
fn set_text_rejections() {
    let key = cs("Title");
    let val = cs("some text");
    let lang = cs("en");
    // row 472: NULL png_ptr / info_ptr / text_ptr, or num_text <= 0
    for mask in 0u32..8 {
        for n in [i32::MIN, -1i32, 0, 1] {
            same!(
                format!("png_set_text_2 nulls mask={} n={}", mask, n),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let t = mk_text(
                        PNG_TEXT_COMPRESSION_NONE,
                        key.as_ptr() as png_charp,
                        val.as_ptr() as png_charp,
                        lang.as_ptr() as png_charp,
                    );
                    let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
                    let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
                    let tp = if mask & 4 != 0 { &t as *const _ } else { std::ptr::null() };
                    let r = (api.png_set_text_2)(p, i, tp, n);
                    format!(
                        "r={} valid={:#x} {}",
                        r,
                        validmask(api, s.png, s.info),
                        report_text(api, s.png, s.info)
                    )
                }
            );
            same!(
                format!("png_set_text nulls mask={} n={}", mask, n),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let t = mk_text(
                        PNG_TEXT_COMPRESSION_NONE,
                        key.as_ptr() as png_charp,
                        val.as_ptr() as png_charp,
                        lang.as_ptr() as png_charp,
                    );
                    let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
                    let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
                    let tp = if mask & 4 != 0 { &t as *const _ } else { std::ptr::null() };
                    (api.png_set_text)(p, i, tp, n);
                    format!(
                        "valid={:#x} {}",
                        validmask(api, s.png, s.info),
                        report_text(api, s.png, s.info)
                    )
                }
            );
        }
    }
    // row 476: compression outside [PNG_TEXT_COMPRESSION_NONE,
    // PNG_TEXT_COMPRESSION_LAST).  Includes every named constant plus LAST,
    // LAST+1, -1, 999, i32::MIN, i32::MAX.
    for c in [
        i32::MIN,
        -1000,
        -100,
        PNG_TEXT_COMPRESSION_NONE_WR,
        PNG_TEXT_COMPRESSION_zTXt_WR,
        PNG_TEXT_COMPRESSION_NONE,
        PNG_TEXT_COMPRESSION_zTXt,
        PNG_ITXT_COMPRESSION_NONE,
        PNG_ITXT_COMPRESSION_zTXt,
        PNG_TEXT_COMPRESSION_LAST,
        PNG_TEXT_COMPRESSION_LAST + 1,
        999,
        i32::MAX,
    ] {
        for benign in [0i32, 1] {
            for read in [false, true] {
                same!(
                    format!("png_set_text_2(compression={},benign={},read={})", c, benign, read),
                    |api| unsafe {
                        let (png, info, _kr, _kw);
                        if read {
                            let s = ReadSess::new(api, &[]);
                            png = s.png;
                            info = s.info;
                            _kr = Some(s);
                            _kw = None;
                        } else {
                            let s = WriteSess::new(api);
                            png = s.png;
                            info = s.info;
                            _kr = None;
                            _kw = Some(s);
                        }
                        (api.png_set_benign_errors)(png, benign);
                        let t = mk_text(
                            c,
                            key.as_ptr() as png_charp,
                            val.as_ptr() as png_charp,
                            lang.as_ptr() as png_charp,
                        );
                        let r = (api.png_set_text_2)(png, info, &t, 1);
                        format!("r={} {}", r, report_text(api, png, info))
                    }
                );
            }
        }
    }
    // row 475: key == NULL -> the entry is silently skipped
    for nulls in 0..4usize {
        same!(format!("png_set_text_2 key=NULL variant {}", nulls), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let t = [
                mk_text(
                    PNG_TEXT_COMPRESSION_NONE,
                    if nulls & 1 != 0 { std::ptr::null_mut() } else { key.as_ptr() as png_charp },
                    val.as_ptr() as png_charp,
                    lang.as_ptr() as png_charp,
                ),
                mk_text(
                    PNG_TEXT_COMPRESSION_NONE,
                    if nulls & 2 != 0 { std::ptr::null_mut() } else { key.as_ptr() as png_charp },
                    std::ptr::null_mut(),
                    lang.as_ptr() as png_charp,
                ),
            ];
            let r = (api.png_set_text_2)(s.png, s.info, t.as_ptr(), 2);
            format!(
                "r={} valid={:#x} {}",
                r,
                validmask(api, s.png, s.info),
                report_text(api, s.png, s.info)
            )
        });
    }
    // empty / whitespace / over-long keys go through png_check_keyword at write
    // time; png_set_text_2 itself stores them verbatim.
    for k in ["", " ", "  ", " lead", "trail ", &"k".repeat(80), "a\u{a0}b"] {
        let ck = cs(k);
        same!(format!("png_set_text_2(key={:?})", k), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let t = mk_text(
                PNG_TEXT_COMPRESSION_NONE,
                ck.as_ptr() as png_charp,
                val.as_ptr() as png_charp,
                lang.as_ptr() as png_charp,
            );
            let r = (api.png_set_text_2)(s.png, s.info, &t, 1);
            format!("r={} {}", r, report_text(api, s.png, s.info))
        });
    }
    // row 473: num_text > INT_MAX - info_ptr->num_text (count overflow).  One
    // entry must already be stored so that old_num_text > 0.
    for benign in [0i32, 1] {
        same!(format!("png_set_text_2 count overflow benign={}", benign), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let t = mk_text(
                PNG_TEXT_COMPRESSION_NONE,
                key.as_ptr() as png_charp,
                val.as_ptr() as png_charp,
                lang.as_ptr() as png_charp,
            );
            let r0 = (api.png_set_text_2)(s.png, s.info, &t, 1);
            (api.png_set_benign_errors)(s.png, benign);
            let r1 = (api.png_set_text_2)(s.png, s.info, &t, i32::MAX);
            format!("r0={} r1={} {}", r0, r1, report_text(api, s.png, s.info))
        });
    }
    // row 474: png_realloc_array for the text array fails
    for benign in [0i32, 1] {
        same!(format!("png_set_text_2 array OOM benign={}", benign), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_benign_errors)(s.png, benign);
            let t = mk_text(
                PNG_TEXT_COMPRESSION_NONE,
                key.as_ptr() as png_charp,
                val.as_ptr() as png_charp,
                lang.as_ptr() as png_charp,
            );
            starve(api, s.png, 0);
            let r = (api.png_set_text_2)(s.png, s.info, &t, 1);
            unstarve(api, s.png);
            format!("r={} {}", r, report_text(api, s.png, s.info))
        });
        // row 471: png_set_text turns that non-zero return into a png_error
        same!(format!("png_set_text array OOM benign={}", benign), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_benign_errors)(s.png, benign);
            let t = mk_text(
                PNG_TEXT_COMPRESSION_NONE,
                key.as_ptr() as png_charp,
                val.as_ptr() as png_charp,
                lang.as_ptr() as png_charp,
            );
            starve(api, s.png, 0);
            (api.png_set_text)(s.png, s.info, &t, 1);
            unstarve(api, s.png);
            format!("{}", report_text(api, s.png, s.info))
        });
    }
    // row 478: the per-entry key/text/lang buffer allocation fails while the
    // array itself is already big enough.
    for benign in [0i32, 1] {
        for c in [PNG_TEXT_COMPRESSION_NONE, PNG_ITXT_COMPRESSION_NONE] {
            same!(
                format!("png_set_text_2 entry OOM benign={} c={}", benign, c),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let t = mk_text(
                        c,
                        key.as_ptr() as png_charp,
                        val.as_ptr() as png_charp,
                        lang.as_ptr() as png_charp,
                    );
                    let r0 = (api.png_set_text_2)(s.png, s.info, &t, 1);
                    (api.png_set_benign_errors)(s.png, benign);
                    starve(api, s.png, 0);
                    let r1 = (api.png_set_text_2)(s.png, s.info, &t, 1);
                    unstarve(api, s.png);
                    format!("r0={} r1={} {}", r0, r1, report_text(api, s.png, s.info))
                }
            );
        }
    }
    // NOTE: row 477 ("iTXt chunk not supported") is UNREACHABLE in this build:
    // PNG_iTXt_SUPPORTED is defined in pnglibconf.h, so pngset.c:1061-1066 is
    // not compiled in.
}

/// rows 479..485
#[test]
fn set_time_rejections() {
    // row 479: NULL png_ptr / info_ptr / mod_time
    for mask in 0u32..8 {
        same!(format!("png_set_tIME nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let t = png_time {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            };
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            let m = if mask & 4 != 0 { &t as *const _ } else { std::ptr::null() };
            (api.png_set_tIME)(p, i, m);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // rows 481..485: each field pushed just outside its legal range
    let times: [png_time; 22] = [
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 0, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 65535, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
        png_time { year: 2000, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 12, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 255, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 31, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 255, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 23, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 255, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 59, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 255, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 60 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 255 },
        png_time { year: 10000, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 9999, month: 2, day: 30, hour: 0, minute: 0, second: 0 },
    ];
    for (n, t) in times.iter().enumerate() {
        let t = *t;
        same!(format!("png_set_tIME #{} {:?}", n, t), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_tIME)(s.png, s.info, &t);
            let mut out: png_timep = std::ptr::null_mut();
            let r = (api.png_get_tIME)(s.png, s.info, &mut out);
            format!(
                "valid={:#x} r={} {:?}",
                validmask(api, s.png, s.info),
                r,
                if out.is_null() { None } else { Some(*out) }
            )
        });
    }
    // row 480: (png_ptr->mode & PNG_WROTE_tIME) != 0 -- the flag is only set by
    // png_write_tIME, so a real write is needed.
    same!("png_set_tIME after the chunk was written", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        let t1 = png_time { year: 2001, month: 2, day: 3, hour: 4, minute: 5, second: 6 };
        (api.png_set_tIME)(s.png, s.info, &t1);
        (api.png_write_info)(s.png, s.info);
        let t2 = png_time { year: 1999, month: 9, day: 9, hour: 9, minute: 9, second: 9 };
        (api.png_set_tIME)(s.png, s.info, &t2);
        let mut out: png_timep = std::ptr::null_mut();
        let r = (api.png_get_tIME)(s.png, s.info, &mut out);
        format!(
            "r={} {:?}",
            r,
            if out.is_null() { None } else { Some(*out) }
        )
    });
}

/// rows 486, 487, 488, 489
#[test]
fn set_trns_rejections() {
    // row 486: NULL png_ptr / info_ptr
    for mask in 0u32..4 {
        same!(format!("png_set_tRNS nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
            let alpha = [1u8, 2, 3, 4];
            let tc = png_color_16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_tRNS)(p, i, alpha.as_ptr(), 4, &tc);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    // row 487: trans_alpha != NULL but num_trans out of 1..=256
    let alpha = vec![0x5au8; 512];
    let nums = [
        i32::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        255,
        256,
        257,
        1000,
        65535,
        65536,
        65537,
        i32::MAX,
    ];
    for &n in &nums {
        for with_alpha in [false, true] {
            for with_color in [false, true] {
                same!(
                    format!("png_set_tRNS(n={},alpha={},color={})", n, with_alpha, with_color),
                    |api| unsafe {
                        let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
                        let pal = vec![png_color::default(); 256];
                        (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), 256);
                        let tc = png_color_16 { index: 7, red: 1, green: 2, blue: 3, gray: 4 };
                        let pa = if with_alpha { alpha.as_ptr() } else { std::ptr::null() };
                        let pc = if with_color { &tc as *const _ } else { std::ptr::null() };
                        (api.png_set_tRNS)(s.png, s.info, pa, n, pc);
                        let mut ta: png_bytep = std::ptr::null_mut();
                        let mut on: c_int = -12345;
                        let mut oc: png_color_16p = std::ptr::null_mut();
                        let r = (api.png_get_tRNS)(s.png, s.info, &mut ta, &mut on, &mut oc);
                        format!(
                            "valid={:#x} r={} n={} ta={} first={:?} tc={:?}",
                            validmask(api, s.png, s.info),
                            r,
                            on,
                            nn(ta),
                            if ta.is_null() { None } else { Some(*ta) },
                            if oc.is_null() { None } else { Some(*oc) },
                        )
                    }
                );
            }
        }
    }
    // rows 488, 489: out-of-range samples for the declared bit depth
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 2),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    ] {
        for v in [0u16, 1, 2, 3, 4, 15, 16, 255, 256, 65535] {
            same!(
                format!("png_set_tRNS samples(ct={},bd={},v={})", ct, bd, v),
                |api| unsafe {
                    let s = wsess(api, ct, bd);
                    let tc = png_color_16 {
                        index: 0,
                        red: v,
                        green: v,
                        blue: v,
                        gray: v,
                    };
                    (api.png_set_tRNS)(s.png, s.info, std::ptr::null(), 0, &tc);
                    let mut ta: png_bytep = std::ptr::null_mut();
                    let mut on: c_int = -12345;
                    let mut oc: png_color_16p = std::ptr::null_mut();
                    let r = (api.png_get_tRNS)(s.png, s.info, &mut ta, &mut on, &mut oc);
                    format!(
                        "valid={:#x} r={} n={} tc={:?}",
                        validmask(api, s.png, s.info),
                        r,
                        on,
                        if oc.is_null() { None } else { Some(*oc) }
                    )
                }
            );
        }
    }
    // both pointers NULL: nothing at all is stored, but num_trans is still
    // assigned and the valid bit is still set when num_trans != 0.
    for &n in &[-1i32, 0, 1, 5] {
        same!(format!("png_set_tRNS(NULL,NULL,n={})", n), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_tRNS)(s.png, s.info, std::ptr::null(), n, std::ptr::null());
            let mut ta: png_bytep = std::ptr::null_mut();
            let mut on: c_int = -12345;
            let mut oc: png_color_16p = std::ptr::null_mut();
            let r = (api.png_get_tRNS)(s.png, s.info, &mut ta, &mut on, &mut oc);
            format!(
                "valid={:#x} r={} n={}",
                validmask(api, s.png, s.info),
                r,
                on
            )
        });
    }
}

/// rows 490..495
#[test]
fn set_splt_rejections() {
    // row 490: NULL png_ptr / info_ptr / entries, or nentries <= 0
    for mask in 0u32..8 {
        for n in [i32::MIN, -1i32, 0, 1] {
            same!(
                format!("png_set_sPLT nulls mask={} n={}", mask, n),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let name = cs("p");
                    let mut ents = [png_sPLT_entry::default(); 2];
                    ents[0].red = 5;
                    let sp = png_sPLT_t {
                        name: name.as_ptr() as png_charp,
                        depth: 8,
                        entries: ents.as_mut_ptr(),
                        nentries: 2,
                    };
                    let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
                    let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
                    let e = if mask & 4 != 0 { &sp as *const _ } else { std::ptr::null() };
                    (api.png_set_sPLT)(p, i, e, n);
                    format!(
                        "valid={:#x} {}",
                        validmask(api, s.png, s.info),
                        report_splt(api, s.png, s.info)
                    )
                }
            );
        }
    }
    // row 492: entries->name == NULL or entries->entries == NULL
    for hole in 0..4usize {
        for benign in [0i32, 1] {
            for count in [1i32, 2] {
                same!(
                    format!("png_set_sPLT invalid entry hole={} benign={} n={}", hole, benign, count),
                    |api| unsafe {
                        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                        (api.png_set_benign_errors)(s.png, benign);
                        let name = cs("p");
                        let mut ents = [png_sPLT_entry::default(); 2];
                        ents[1].blue = 3;
                        let sp = [
                            png_sPLT_t {
                                name: if hole & 1 != 0 {
                                    std::ptr::null_mut()
                                } else {
                                    name.as_ptr() as png_charp
                                },
                                depth: 8,
                                entries: if hole & 2 != 0 {
                                    std::ptr::null_mut()
                                } else {
                                    ents.as_mut_ptr()
                                },
                                nentries: 2,
                            },
                            png_sPLT_t {
                                name: name.as_ptr() as png_charp,
                                depth: 16,
                                entries: ents.as_mut_ptr(),
                                nentries: 2,
                            },
                        ];
                        (api.png_set_sPLT)(s.png, s.info, sp.as_ptr(), count);
                        format!(
                            "valid={:#x} {}",
                            validmask(api, s.png, s.info),
                            report_splt(api, s.png, s.info)
                        )
                    }
                );
            }
        }
    }
    // entries->nentries out of range (0 / negative) -> png_malloc_array errors
    for ne in [i32::MIN, -1i32, 0, 1, 2] {
        for benign in [0i32, 1] {
            same!(
                format!("png_set_sPLT entry nentries={} benign={}", ne, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    let name = cs("p");
                    let mut ents = [png_sPLT_entry::default(); 4];
                    ents[0].green = 2;
                    let sp = png_sPLT_t {
                        name: name.as_ptr() as png_charp,
                        depth: 8,
                        entries: ents.as_mut_ptr(),
                        nentries: ne,
                    };
                    (api.png_set_sPLT)(s.png, s.info, &sp, 1);
                    format!(
                        "valid={:#x} {}",
                        validmask(api, s.png, s.info),
                        report_splt(api, s.png, s.info)
                    )
                }
            );
        }
    }
    // rows 491, 493, 494, 495: allocation failures at the array, the name and
    // the entry-array allocation sites.
    for budget in [0i64, 1, 2, 3, 4] {
        for benign in [0i32, 1] {
            same!(
                format!("png_set_sPLT OOM budget={} benign={}", budget, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    let name = cs("palette name");
                    let mut ents = [png_sPLT_entry::default(); 2];
                    ents[0].alpha = 9;
                    let sp = [
                        png_sPLT_t {
                            name: name.as_ptr() as png_charp,
                            depth: 8,
                            entries: ents.as_mut_ptr(),
                            nentries: 2,
                        },
                        png_sPLT_t {
                            name: name.as_ptr() as png_charp,
                            depth: 16,
                            entries: ents.as_mut_ptr(),
                            nentries: 2,
                        },
                    ];
                    starve(api, s.png, budget);
                    (api.png_set_sPLT)(s.png, s.info, sp.as_ptr(), 2);
                    unstarve(api, s.png);
                    format!(
                        "valid={:#x} {}",
                        validmask(api, s.png, s.info),
                        report_splt(api, s.png, s.info)
                    )
                }
            );
        }
    }
    // A very large nentries: the realloc_array size check is what must reject
    // it, so the allocator is starved to keep the test from asking the OS for
    // tens of gigabytes.
    for n in [1000i32, 65536, i32::MAX] {
        same!(format!("png_set_sPLT huge nentries={}", n), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let name = cs("p");
            let mut ents = [png_sPLT_entry::default(); 2];
            let sp = png_sPLT_t {
                name: name.as_ptr() as png_charp,
                depth: 8,
                entries: ents.as_mut_ptr(),
                nentries: 2,
            };
            starve(api, s.png, 0);
            (api.png_set_sPLT)(s.png, s.info, &sp, n);
            unstarve(api, s.png);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
}

unsafe fn report_splt(api: &'static Api, png: png_structp, info: png_infop) -> String {
    let mut sp: png_sPLT_tp = std::ptr::null_mut();
    let n = (api.png_get_sPLT)(png, info, &mut sp);
    let mut out = format!("sPLT n={} p={}", n, nn(sp));
    if !sp.is_null() && n > 0 {
        for k in 0..n as isize {
            let e = &*sp.offset(k);
            out.push_str(&format!(
                " [name={:?} depth={} ne={} ents={}]",
                rs_str(e.name as png_const_charp),
                e.depth,
                e.nentries,
                nn(e.entries)
            ));
        }
    }
    out
}

/// rows 496, 497, 498, 501, 502
#[test]
fn set_unknown_chunks_rejections() {
    let data = [1u8, 2, 3, 4];
    // row 498: NULL png_ptr / info_ptr / unknowns, or num_unknowns <= 0
    for mask in 0u32..8 {
        for n in [i32::MIN, -1i32, 0, 1] {
            same!(
                format!("png_set_unknown_chunks nulls mask={} n={}", mask, n),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let ch = png_unknown_chunk {
                        name: [b'v', b'p', b'A', b'g', 0],
                        data: data.as_ptr() as *mut png_byte,
                        size: 4,
                        location: PNG_HAVE_IHDR as png_byte,
                    };
                    let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
                    let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
                    let u = if mask & 4 != 0 { &ch as *const _ } else { std::ptr::null() };
                    (api.png_set_unknown_chunks)(p, i, u, n);
                    format!("{}", report_unknown(api, s.png, s.info))
                }
            );
        }
    }
    // rows 496, 497: check_location.  location == 0 on a *write* struct produces
    // png_app_warning + the fallback, which is also 0 on a fresh struct, and
    // then png_error.  On a *read* struct it goes straight to png_error.
    for loc in [
        0u8,
        PNG_HAVE_IHDR as u8,
        PNG_HAVE_PLTE as u8,
        0x04,
        PNG_AFTER_IDAT as u8,
        0x03,
        0x0b,
        0x10,
        0x20,
        0xf0,
        0xff,
    ] {
        for read in [false, true] {
            for benign in [0i32, 1] {
                same!(
                    format!("set_unknown_chunks(loc={:#x},read={},benign={})", loc, read, benign),
                    |api| unsafe {
                        let (png, info, _kr, _kw);
                        if read {
                            let s = ReadSess::new(api, &[]);
                            png = s.png;
                            info = s.info;
                            _kr = Some(s);
                            _kw = None;
                        } else {
                            let s = WriteSess::new(api);
                            png = s.png;
                            info = s.info;
                            _kr = None;
                            _kw = Some(s);
                        }
                        (api.png_set_benign_errors)(png, benign);
                        let ch = png_unknown_chunk {
                            name: [b'v', b'p', b'A', b'g', 0],
                            data: data.as_ptr() as *mut png_byte,
                            size: 4,
                            location: loc,
                        };
                        (api.png_set_unknown_chunks)(png, info, &ch, 1);
                        format!("{}", report_unknown(api, png, info))
                    }
                );
            }
        }
    }
    // the same, but after png_write_info so that png_ptr->mode has HAVE_IHDR set
    // and the write-struct fallback in check_location can actually succeed
    for loc in [0u8, 0x10, 0xf0] {
        same!(format!("set_unknown_chunks after write_info loc={:#x}", loc), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_write_info)(s.png, s.info);
            let ch = png_unknown_chunk {
                name: [b'v', b'p', b'A', b'g', 0],
                data: data.as_ptr() as *mut png_byte,
                size: 4,
                location: loc,
            };
            (api.png_set_unknown_chunks)(s.png, s.info, &ch, 1);
            format!("{}", report_unknown(api, s.png, s.info))
        });
    }
    // a zero-size chunk stores no data at all
    same!("set_unknown_chunks size=0", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        let ch = png_unknown_chunk {
            name: [b'v', b'p', b'A', b'g', 0],
            data: std::ptr::null_mut(),
            size: 0,
            location: PNG_HAVE_IHDR as png_byte,
        };
        (api.png_set_unknown_chunks)(s.png, s.info, &ch, 1);
        format!("{}", report_unknown(api, s.png, s.info))
    });
    // rows 501, 502: the array allocation and the per-chunk data allocation fail
    for budget in [0i64, 1, 2, 3] {
        for benign in [0i32, 1] {
            same!(
                format!("set_unknown_chunks OOM budget={} benign={}", budget, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    let ch = [
                        png_unknown_chunk {
                            name: [b'v', b'p', b'A', b'g', 0],
                            data: data.as_ptr() as *mut png_byte,
                            size: 4,
                            location: PNG_HAVE_IHDR as png_byte,
                        },
                        png_unknown_chunk {
                            name: [b's', b'T', b'E', b'R', 0],
                            data: data.as_ptr() as *mut png_byte,
                            size: 4,
                            location: PNG_AFTER_IDAT as png_byte,
                        },
                    ];
                    starve(api, s.png, budget);
                    (api.png_set_unknown_chunks)(s.png, s.info, ch.as_ptr(), 2);
                    unstarve(api, s.png);
                    format!("{}", report_unknown(api, s.png, s.info))
                }
            );
        }
    }
    // NOTE: rows 499 and 500 ("no unknown chunk support on read"/"on write") are
    // UNREACHABLE in this build: both PNG_READ_UNKNOWN_CHUNKS_SUPPORTED and
    // PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED are defined in pnglibconf.h, so
    // pngset.c:1440-1445 and :1449-1454 are not compiled in.
}

unsafe fn report_unknown(api: &'static Api, png: png_structp, info: png_infop) -> String {
    let mut u: png_unknown_chunkp = std::ptr::null_mut();
    let n = (api.png_get_unknown_chunks)(png, info, &mut u);
    let mut out = format!("unknown n={} p={}", n, nn(u));
    if !u.is_null() && n > 0 {
        for k in 0..n as isize {
            let c = &*u.offset(k);
            out.push_str(&format!(
                " [name={:?} size={} loc={:#x} data={}]",
                c.name, c.size, c.location, nn(c.data)
            ));
        }
    }
    out
}

/// rows 503, 504
#[test]
fn set_unknown_chunk_location_rejections() {
    let data = [1u8, 2, 3, 4];
    // row 503: NULL pointers, or chunk index out of 0..unknown_chunks_num
    for stored in [0i32, 1, 2] {
        for chunk in [i32::MIN, -1000, -1, 0, 1, 2, 3, 1000, i32::MAX] {
            for mask in 0u32..4 {
                same!(
                    format!("set_unknown_chunk_location(stored={},chunk={},mask={})", stored, chunk, mask),
                    |api| unsafe {
                        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                        if stored > 0 {
                            let ch = [
                                png_unknown_chunk {
                                    name: [b'v', b'p', b'A', b'g', 0],
                                    data: data.as_ptr() as *mut png_byte,
                                    size: 4,
                                    location: PNG_HAVE_IHDR as png_byte,
                                },
                                png_unknown_chunk {
                                    name: [b's', b'T', b'E', b'R', 0],
                                    data: data.as_ptr() as *mut png_byte,
                                    size: 4,
                                    location: PNG_AFTER_IDAT as png_byte,
                                },
                            ];
                            (api.png_set_unknown_chunks)(s.png, s.info, ch.as_ptr(), stored);
                        }
                        let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
                        let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
                        (api.png_set_unknown_chunk_location)(p, i, chunk, PNG_HAVE_PLTE as c_int);
                        format!("{}", report_unknown(api, s.png, s.info))
                    }
                );
            }
        }
    }
    // row 504: (location & (HAVE_IHDR|HAVE_PLTE|AFTER_IDAT)) == 0 ->
    // png_app_error, then the location is forced to AFTER_IDAT (when the
    // undocumented PNG_HAVE_IDAT bit is set) or to HAVE_IHDR.
    for loc in [
        i32::MIN,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        8,
        11,
        0x10,
        0x14,
        0x20,
        0xf0,
        0xff,
        0x100,
        1000,
        i32::MAX,
    ] {
        for benign in [0i32, 1] {
            same!(
                format!("set_unknown_chunk_location(loc={},benign={})", loc, benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let ch = png_unknown_chunk {
                        name: [b'v', b'p', b'A', b'g', 0],
                        data: data.as_ptr() as *mut png_byte,
                        size: 4,
                        location: PNG_HAVE_IHDR as png_byte,
                    };
                    (api.png_set_unknown_chunks)(s.png, s.info, &ch, 1);
                    (api.png_set_benign_errors)(s.png, benign);
                    (api.png_set_unknown_chunk_location)(s.png, s.info, 0, loc);
                    format!("{}", report_unknown(api, s.png, s.info))
                }
            );
        }
    }
}

/// rows 505, 506
#[test]
fn permit_mng_features_rejections() {
    same!("png_permit_mng_features(NULL)", |api| unsafe {
        format!(
            "{}",
            (api.png_permit_mng_features)(std::ptr::null_mut(), PNG_ALL_MNG_FEATURES)
        )
    });
    for m in [
        0u32,
        PNG_FLAG_MNG_EMPTY_PLTE,
        0x02,
        PNG_FLAG_MNG_FILTER_64,
        PNG_ALL_MNG_FEATURES,
        0x06,
        0x08,
        0x10,
        0xff,
        0xffff_ffff,
        0x8000_0000,
    ] {
        for read in [false, true] {
            same!(format!("png_permit_mng_features({:#x},read={})", m, read), |api| unsafe {
                let (png, _kr, _kw);
                if read {
                    let s = ReadSess::new(api, &[]);
                    png = s.png;
                    _kr = Some(s);
                    _kw = None;
                } else {
                    let s = WriteSess::new(api);
                    png = s.png;
                    _kr = None;
                    _kw = Some(s);
                }
                let a = (api.png_permit_mng_features)(png, m);
                // calling it a second time must replace, not accumulate
                let b = (api.png_permit_mng_features)(png, 0);
                format!("{:#x} then {:#x}", a, b)
            });
        }
    }
}

/// rows 507..511 (plus png_handle_as_unknown, rows 43/44)
#[test]
fn set_keep_unknown_chunks_rejections() {
    // row 507
    same!("png_set_keep_unknown_chunks(NULL)", |api| unsafe {
        (api.png_set_keep_unknown_chunks)(
            std::ptr::null_mut(),
            PNG_HANDLE_CHUNK_NEVER,
            std::ptr::null(),
            0,
        );
        "ok".to_string()
    });
    let list: [u8; 15] = [
        b'v', b'p', b'A', b'g', 0, b's', b'T', b'E', b'R', 0, b'g', b'I', b'F', b'g', 0,
    ];
    // rows 508, 509, 510: invalid keep, num_chunks_in == 0, NULL chunk list
    for keep in [
        i32::MIN,
        -1000,
        -1,
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
        PNG_HANDLE_CHUNK_LAST,
        PNG_HANDLE_CHUNK_LAST + 1,
        999,
        i32::MAX,
    ] {
        for n in [i32::MIN, -1000, -1, 0, 1, 3] {
            for null_list in [false, true] {
                for benign in [0i32, 1] {
                    same!(
                        format!(
                            "keep_unknown(keep={},n={},nulllist={},benign={})",
                            keep, n, null_list, benign
                        ),
                        |api| unsafe {
                            let s = ReadSess::new(api, &[]);
                            (api.png_set_benign_errors)(s.png, benign);
                            let l = if null_list { std::ptr::null() } else { list.as_ptr() };
                            (api.png_set_keep_unknown_chunks)(s.png, keep, l, n);
                            let q = [b'v', b'p', b'A', b'g', 0u8];
                            let q2 = [b'g', b'A', b'M', b'A', 0u8];
                            format!(
                                "vpAg={} gAMA={}",
                                (api.png_handle_as_unknown)(s.png, q.as_ptr()),
                                (api.png_handle_as_unknown)(s.png, q2.as_ptr()),
                            )
                        }
                    );
                }
            }
        }
    }
    // row 511: num_chunks + old_num_chunks > UINT_MAX/5.  The check happens
    // before any allocation, so no huge malloc is attempted.
    for n in [858_993_459i32, 858_993_460, 900_000_000, i32::MAX] {
        for benign in [0i32, 1] {
            same!(
                format!("keep_unknown too many chunks n={} benign={}", n, benign),
                |api| unsafe {
                    let s = ReadSess::new(api, &[]);
                    (api.png_set_benign_errors)(s.png, benign);
                    starve(api, s.png, 0); // in case the check is passed
                    (api.png_set_keep_unknown_chunks)(
                        s.png,
                        PNG_HANDLE_CHUNK_NEVER,
                        list.as_ptr(),
                        n,
                    );
                    unstarve(api, s.png);
                    let q = [b'v', b'p', b'A', b'g', 0u8];
                    format!("{}", (api.png_handle_as_unknown)(s.png, q.as_ptr()))
                }
            );
        }
    }
    // rows 43, 44: png_handle_as_unknown with a NULL png_ptr / NULL name / empty
    // list / a name that is not in the list.
    same!("png_handle_as_unknown(NULL,NULL)", |api| unsafe {
        format!(
            "{}",
            (api.png_handle_as_unknown)(std::ptr::null(), std::ptr::null())
        )
    });
    same!("png_handle_as_unknown(png,NULL)", |api| unsafe {
        let s = ReadSess::new(api, &[]);
        (api.png_set_keep_unknown_chunks)(s.png, PNG_HANDLE_CHUNK_NEVER, list.as_ptr(), 3);
        format!(
            "{}",
            (api.png_handle_as_unknown)(s.png, std::ptr::null())
        )
    });
    for nm in [
        [b'v', b'p', b'A', b'g', 0u8],
        [b's', b'T', b'E', b'R', 0u8],
        [b'g', b'I', b'F', b'g', 0u8],
        [b'n', b'o', b'p', b'e', 0u8],
        [0u8, 0, 0, 0, 0],
        [0xffu8, 0xff, 0xff, 0xff, 0],
    ] {
        for populate in [false, true] {
            same!(
                format!("png_handle_as_unknown({:?},populate={})", nm, populate),
                |api| unsafe {
                    let s = ReadSess::new(api, &[]);
                    if populate {
                        (api.png_set_keep_unknown_chunks)(
                            s.png,
                            PNG_HANDLE_CHUNK_ALWAYS,
                            list.as_ptr(),
                            3,
                        );
                    }
                    format!("{}", (api.png_handle_as_unknown)(s.png, nm.as_ptr()))
                }
            );
        }
    }
}

/// row 512
#[test]
fn set_read_user_chunk_fn_rejections() {
    unsafe extern "C-unwind" fn cb(_p: png_structp, _c: png_unknown_chunkp) -> c_int {
        0
    }
    same!("png_set_read_user_chunk_fn(NULL)", |api| unsafe {
        (api.png_set_read_user_chunk_fn)(std::ptr::null_mut(), 0x1234 as png_voidp, Some(cb));
        format!("{}", nn((api.png_get_user_chunk_ptr)(std::ptr::null())))
    });
    for with_fn in [false, true] {
        same!(format!("png_set_read_user_chunk_fn(fn={})", with_fn), |api| unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_set_read_user_chunk_fn)(
                s.png,
                0x1234 as png_voidp,
                if with_fn { Some(cb) } else { None },
            );
            format!("{}", nn((api.png_get_user_chunk_ptr)(s.png)))
        });
    }
}

/// row 513
#[test]
fn set_rows_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_rows nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let mut r0 = vec![0u8; 24];
            let mut rows = vec![r0.as_mut_ptr(); 8];
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_rows)(p, i, rows.as_mut_ptr());
            format!(
                "valid={:#x} rows={}",
                validmask(api, s.png, s.info),
                nn((api.png_get_rows)(s.png, s.info))
            )
        });
    }
    // row_pointers == NULL clears the pointer and does *not* set PNG_INFO_IDAT
    same!("png_set_rows(NULL rows)", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        (api.png_set_rows)(s.png, s.info, std::ptr::null_mut());
        format!(
            "valid={:#x} rows={}",
            validmask(api, s.png, s.info),
            nn((api.png_get_rows)(s.png, s.info))
        )
    });
    // setting the same array twice must not free it
    same!("png_set_rows twice with the same array", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        let mut r0 = vec![0u8; 24];
        let mut rows = vec![r0.as_mut_ptr(); 8];
        (api.png_set_rows)(s.png, s.info, rows.as_mut_ptr());
        (api.png_set_rows)(s.png, s.info, rows.as_mut_ptr());
        (api.png_set_rows)(s.png, s.info, std::ptr::null_mut());
        format!(
            "valid={:#x} rows={}",
            validmask(api, s.png, s.info),
            nn((api.png_get_rows)(s.png, s.info))
        )
    });
}

/// rows 514, 515, 516, 517, 518
#[test]
fn set_compression_buffer_size_rejections() {
    // row 514
    same!("png_set_compression_buffer_size(NULL)", |api| unsafe {
        (api.png_set_compression_buffer_size)(std::ptr::null_mut(), 8192);
        "ok".to_string()
    });
    // rows 515, 518: size == 0 or > PNG_UINT_31_MAX -> png_error; size < 6 on a
    // write struct -> warning and no change.
    let sizes = [
        0usize,
        1,
        2,
        5,
        6,
        7,
        8192,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        usize::MAX,
        usize::MAX / 2,
    ];
    for &sz in &sizes {
        for read in [false, true] {
            same!(
                format!("png_set_compression_buffer_size({:#x},read={})", sz, read),
                |api| unsafe {
                    let (png, _kr, _kw);
                    if read {
                        let s = ReadSess::new(api, &[]);
                        png = s.png;
                        _kr = Some(s);
                        _kw = None;
                    } else {
                        let s = WriteSess::new(api);
                        png = s.png;
                        _kr = None;
                        _kw = Some(s);
                    }
                    let before = (api.png_get_compression_buffer_size)(png);
                    (api.png_set_compression_buffer_size)(png, sz);
                    format!(
                        "before={} after={}",
                        before,
                        (api.png_get_compression_buffer_size)(png)
                    )
                }
            );
        }
    }
    // row 516: write struct with png_ptr->zowner != 0.  The IDAT zstream is
    // claimed by the first png_write_row, so it is still owned afterwards.
    for &sz in &[6usize, 4096, 8192, 1] {
        same!(
            format!("png_set_compression_buffer_size({}) while in use", sz),
            |api| unsafe {
                let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                (api.png_write_info)(s.png, s.info);
                let row = vec![0x37u8; 8 * 3];
                (api.png_write_row)(s.png, row.as_ptr());
                let before = (api.png_get_compression_buffer_size)(s.png);
                (api.png_set_compression_buffer_size)(s.png, sz);
                format!(
                    "before={} after={}",
                    before,
                    (api.png_get_compression_buffer_size)(s.png)
                )
            }
        );
    }
    // NOTE: row 517 ("Compression buffer size limited to system maximum") is
    // UNREACHABLE on this platform: ZLIB_IO_MAX is UINT_MAX (0xffffffff) while
    // pngset.c:1804 has already rejected everything above PNG_UINT_31_MAX
    // (0x7fffffff), so `size > ZLIB_IO_MAX` can never be true here.
}

/// row 519
#[test]
fn set_invalid_rejections() {
    for mask in 0u32..4 {
        same!(format!("png_set_invalid nulls mask={}", mask), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_gAMA_fixed)(s.png, s.info, 45455);
            (api.png_set_pHYs)(s.png, s.info, 1, 1, PNG_RESOLUTION_METER);
            let p = if mask & 1 != 0 { s.png } else { std::ptr::null_mut() };
            let i = if mask & 2 != 0 { s.info } else { std::ptr::null_mut() };
            (api.png_set_invalid)(p, i, PNG_INFO_gAMA as c_int);
            format!("valid={:#x}", validmask(api, s.png, s.info))
        });
    }
    for m in [
        0i32,
        1,
        2,
        -1,
        i32::MIN,
        i32::MAX,
        PNG_INFO_gAMA as c_int,
        PNG_INFO_pHYs as c_int,
        (PNG_INFO_gAMA | PNG_INFO_pHYs) as c_int,
        0x0001_0000,
        0x000f_ffff,
        0x7fff_ffff,
    ] {
        same!(format!("png_set_invalid({:#x})", m), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            (api.png_set_gAMA_fixed)(s.png, s.info, 45455);
            (api.png_set_pHYs)(s.png, s.info, 1, 1, PNG_RESOLUTION_METER);
            (api.png_set_sRGB)(s.png, s.info, 0);
            let before = validmask(api, s.png, s.info);
            (api.png_set_invalid)(s.png, s.info, m);
            format!("{:#x} -> {:#x}", before, validmask(api, s.png, s.info))
        });
    }
}

/// rows 520, 521, 522, 523
#[test]
fn set_user_limits_rejections() {
    same!("png_set_user_limits(NULL)", |api| unsafe {
        (api.png_set_user_limits)(std::ptr::null_mut(), 1, 1);
        (api.png_set_chunk_cache_max)(std::ptr::null_mut(), 1);
        (api.png_set_chunk_malloc_max)(std::ptr::null_mut(), 1);
        "ok".to_string()
    });
    let vals = [0u32, 1, 2, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 1_000_000];
    for &w in &vals {
        for &h in &vals {
            same!(format!("png_set_user_limits({:#x},{:#x})", w, h), |api| unsafe {
                let s = ReadSess::new(api, &[]);
                (api.png_set_user_limits)(s.png, w, h);
                format!(
                    "{}/{}",
                    (api.png_get_user_width_max)(s.png),
                    (api.png_get_user_height_max)(s.png)
                )
            });
        }
    }
    for &v in &vals {
        same!(format!("png_set_chunk_cache_max({:#x})", v), |api| unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_set_chunk_cache_max)(s.png, v);
            format!("{}", (api.png_get_chunk_cache_max)(s.png))
        });
    }
    // row 523: user_chunk_malloc_max == 0 means "unlimited" -> PNG_SIZE_MAX
    for v in [
        0usize,
        1,
        2,
        65535,
        65536,
        65537,
        0x7fff_ffff,
        0xffff_ffff,
        usize::MAX,
        usize::MAX - 1,
    ] {
        same!(format!("png_set_chunk_malloc_max({:#x})", v), |api| unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_set_chunk_malloc_max)(s.png, v);
            format!("{}", (api.png_get_chunk_malloc_max)(s.png))
        });
    }
    // ... and the IHDR rejection that the limits cause
    for &(w, h) in &[(1u32, 1u32), (8, 8), (0x7fff_ffff, 0x7fff_ffff)] {
        same!(format!("png_set_IHDR under user limits {}x{}", w, h), |api| unsafe {
            let s = WriteSess::new(api);
            (api.png_set_user_limits)(s.png, w, h);
            let ok = guard(|| {
                (api.png_set_IHDR)(s.png, s.info, 8, 8, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0)
            })
            .is_some();
            format!("ok={}", ok)
        });
    }
}

/// rows 524..529
#[test]
fn check_keyword_rejections() {
    let keys: [&str; 26] = [
        "",
        " ",
        "  ",
        "a",
        "ab",
        " a",
        "a ",
        " a ",
        "a  b",
        "a   b",
        "a\tb",
        "a\nb",
        "\u{1}",
        "a\u{1}b",
        "a\u{7f}b",
        "a\u{80}b",
        "a\u{a0}b",
        "a\u{a1}b",
        "a\u{ff}b",
        "\u{a0}",
        "\u{a0}\u{a0}",
        "abcdefghij",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // 79
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // 80
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa b", // 79 with trailing sep
        "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~",
    ];
    for k in keys {
        let ck = cs(k);
        for null_png in [false, true] {
            same!(
                format!("png_check_keyword({:?},nullpng={})", k, null_png),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let mut buf = [0u8; 80];
                    let p = if null_png { std::ptr::null_mut() } else { s.png };
                    let n = (api.png_check_keyword)(p, ck.as_ptr(), buf.as_mut_ptr());
                    let end = buf.iter().position(|&b| b == 0).unwrap_or(80);
                    format!("len={} key={:?}", n, &buf[..end])
                }
            );
        }
    }
    // row 524: key == NULL -> *new_key = 0, return 0.  (new_key is written
    // unconditionally, so it must not be NULL as well.)
    same!("png_check_keyword(NULL key)", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        let mut buf = [0xaau8; 80];
        let n = (api.png_check_keyword)(s.png, std::ptr::null(), buf.as_mut_ptr());
        format!("len={} first={}", n, buf[0])
    });
    // keys containing every single byte value, one at a time
    for b in 1u8..=255 {
        let ck = CString::new(vec![b'a', b, b'b']).unwrap();
        same!(format!("png_check_keyword(a<{:#04x}>b)", b), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let mut buf = [0u8; 80];
            let n = (api.png_check_keyword)(s.png, ck.as_ptr(), buf.as_mut_ptr());
            let end = buf.iter().position(|&x| x == 0).unwrap_or(80);
            format!("len={} key={:?}", n, &buf[..end])
        });
        let ck2 = CString::new(vec![b]).unwrap();
        same!(format!("png_check_keyword(<{:#04x}>)", b), |api| unsafe {
            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
            let mut buf = [0u8; 80];
            let n = (api.png_check_keyword)(s.png, ck2.as_ptr(), buf.as_mut_ptr());
            let end = buf.iter().position(|&x| x == 0).unwrap_or(80);
            format!("len={} key={:?}", n, &buf[..end])
        });
    }
}

// ===========================================================================
// pngtrans.c
// ===========================================================================

/// FNV-ish digest so a whole produced byte stream can be folded into the
/// comparison string.
fn dig(b: &[u8]) -> String {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    format!("{}:{:016x}", b.len(), h)
}

/// A 4x4 RGB8 PNG, produced once by the *C* library so that both libraries are
/// handed byte-identical input.
fn tiny_png() -> &'static Vec<u8> {
    static P: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    P.get_or_init(|| unsafe {
        let api = c_api();
        set_current_api(api);
        let mut s = WriteSess::new(api);
        (api.png_set_IHDR)(s.png, s.info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        (api.png_write_info)(s.png, s.info);
        let row = vec![0x40u8; 12];
        for _ in 0..4 {
            (api.png_write_row)(s.png, row.as_ptr());
        }
        (api.png_write_end)(s.png, s.info);
        std::mem::take(&mut s.sink.buf)
    })
}

const TRANS_NAMES: [&str; 15] = [
    "none",
    "bgr",
    "swap",
    "packing",
    "packswap",
    "shift(valid)",
    "shift(zero)",
    "shift(too big)",
    "filler(AFTER)",
    "filler(BEFORE)",
    "add_alpha(AFTER)",
    "add_alpha(bad loc)",
    "swap_alpha",
    "invert_alpha",
    "invert_mono",
];

/// Apply transform `which` to `png`; returns whether it survived.
unsafe fn apply_trans(api: &'static Api, png: png_structp, which: usize, bd: c_int) -> bool {
    let big = png_color_8 {
        red: 99,
        green: 99,
        blue: 99,
        gray: 99,
        alpha: 99,
    };
    let zero = png_color_8 {
        red: 0,
        green: 0,
        blue: 0,
        gray: 0,
        alpha: 0,
    };
    let good = png_color_8 {
        red: bd as png_byte,
        green: bd as png_byte,
        blue: bd as png_byte,
        gray: bd as png_byte,
        alpha: bd as png_byte,
    };
    guard(|| match which {
        0 => {}
        1 => (api.png_set_bgr)(png),
        2 => (api.png_set_swap)(png),
        3 => (api.png_set_packing)(png),
        4 => (api.png_set_packswap)(png),
        5 => (api.png_set_shift)(png, &good),
        6 => (api.png_set_shift)(png, &zero),
        7 => (api.png_set_shift)(png, &big),
        8 => (api.png_set_filler)(png, 0xff, PNG_FILLER_AFTER),
        9 => (api.png_set_filler)(png, 0xff, PNG_FILLER_BEFORE),
        10 => (api.png_set_add_alpha)(png, 0xff, PNG_FILLER_AFTER),
        11 => (api.png_set_add_alpha)(png, 0xff, 999),
        12 => (api.png_set_swap_alpha)(png),
        13 => (api.png_set_invert_alpha)(png),
        _ => (api.png_set_invert_mono)(png),
    })
    .is_some()
}

/// rows 530..540, 542, 544, 545, 547..551
#[test]
fn trans_setter_null_guards() {
    // Every transform setter with png_ptr == NULL must be a silent no-op.  The
    // *_shift variants also take a second pointer, which is checked too.
    for which in 0..TRANS_NAMES.len() {
        same!(format!("{} with NULL png_ptr", TRANS_NAMES[which]), |api| unsafe {
            let ok = apply_trans(api, std::ptr::null_mut(), which, 8);
            format!("ok={}", ok)
        });
    }
    // row 537: png_set_shift with true_bits == NULL
    same!("png_set_shift(NULL true_bits)", |api| unsafe {
        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
        (api.png_set_shift)(s.png, std::ptr::null());
        (api.png_set_shift)(std::ptr::null_mut(), std::ptr::null());
        "ok".to_string()
    });
    // row 541: png_set_interlace_handling
    same!("png_set_interlace_handling(NULL)", |api| unsafe {
        format!("{}", (api.png_set_interlace_handling)(std::ptr::null_mut()))
    });
    for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
        for after in [false, true] {
            same!(
                format!("png_set_interlace_handling(il={},after_info={})", il, after),
                |api| unsafe {
                    let s = WriteSess::new(api);
                    (api.png_set_IHDR)(s.png, s.info, 8, 8, 8, PNG_COLOR_TYPE_RGB, il, 0, 0);
                    let a = (api.png_set_interlace_handling)(s.png);
                    if after {
                        (api.png_write_info)(s.png, s.info);
                    }
                    let b = (api.png_set_interlace_handling)(s.png);
                    format!("{} {}", a, b)
                }
            );
        }
    }
    // rows 566, 567, 568
    same!("png_get_user_transform_ptr(NULL)", |api| unsafe {
        format!("{}", nn((api.png_get_user_transform_ptr)(std::ptr::null())))
    });
    same!("png_get_current_row_number(NULL)", |api| unsafe {
        format!("{}", (api.png_get_current_row_number)(std::ptr::null()))
    });
    same!("png_get_current_pass_number(NULL)", |api| unsafe {
        format!("{}", (api.png_get_current_pass_number)(std::ptr::null()))
    });
    for read in [false, true] {
        same!(format!("row/pass number on a fresh struct(read={})", read), |api| unsafe {
            let (png, _kr, _kw);
            if read {
                let s = ReadSess::new(api, &[]);
                png = s.png;
                _kr = Some(s);
                _kw = None;
            } else {
                let s = WriteSess::new(api);
                png = s.png;
                _kr = None;
                _kw = Some(s);
            }
            format!(
                "{} {} {}",
                (api.png_get_current_row_number)(png),
                (api.png_get_current_pass_number)(png),
                nn((api.png_get_user_transform_ptr)(png)),
            )
        });
    }
}

/// rows 532, 534, 536, 538, 539, 540, 544, 545, 548 -- observed through the
/// bytes an actual write produces, which is the only externally visible effect
/// of `png_ptr->transformations`.
#[test]
fn trans_setter_gating() {
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 2),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_PALETTE, 1),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for which in 0..TRANS_NAMES.len() {
            for after in [false, true] {
                for benign in [0i32, 1] {
                    same!(
                        format!(
                            "write ct={} bd={} trans={} after_info={} benign={}",
                            ct, bd, TRANS_NAMES[which], after, benign
                        ),
                        |api| unsafe {
                            let s = wsess(api, ct, bd);
                            (api.png_set_benign_errors)(s.png, benign);
                            if ct == PNG_COLOR_TYPE_PALETTE {
                                let pal = vec![
                                    png_color {
                                        red: 1,
                                        green: 2,
                                        blue: 3
                                    };
                                    256
                                ];
                                (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), 1 << bd);
                            }
                            let mut set_ok = true;
                            if !after {
                                set_ok = apply_trans(api, s.png, which, bd);
                                if !set_ok {
                                    return "set=false".to_string();
                                }
                            }
                            let info_ok =
                                guard(|| (api.png_write_info)(s.png, s.info)).is_some();
                            if after {
                                set_ok = apply_trans(api, s.png, which, bd);
                            }
                            // A png_struct must be abandoned once png_error has
                            // fired, so the write is not continued in that case.
                            if !set_ok {
                                return format!("info={} set=false", info_ok);
                            }
                            // Deliberately over-allocated: some transforms make
                            // libpng consume more application bytes per row than
                            // PNG_ROWBYTES (see HARNESS.md).
                            let mut row = vec![0u8; 512];
                            for (i, b) in row.iter_mut().enumerate() {
                                *b = (i as u8).wrapping_mul(37).wrapping_add(11);
                            }
                            let mut rows_ok = true;
                            if info_ok {
                                for _ in 0..8 {
                                    if guard(|| (api.png_write_row)(s.png, row.as_ptr()))
                                        .is_none()
                                    {
                                        rows_ok = false;
                                        break;
                                    }
                                }
                            }
                            let end_ok = if info_ok && rows_ok {
                                guard(|| (api.png_write_end)(s.png, s.info)).is_some()
                            } else {
                                false
                            };
                            format!(
                                "set={} info={} rows={} end={} out={}",
                                set_ok,
                                info_ok,
                                rows_ok,
                                end_ok,
                                dig(&s.sink.buf)
                            )
                        }
                    );
                }
            }
        }
    }
}

/// row 552
#[test]
fn do_invert_rejections() {
    for ct in [0u8, 1, 2, 3, 4, 5, 6, 7, 255] {
        for bd in [1u8, 2, 4, 8, 16, 3, 0, 255] {
            same!(format!("png_do_invert(ct={},bd={})", ct, bd), |api| unsafe {
                let mut ri = png_row_info {
                    width: 4,
                    rowbytes: 16,
                    color_type: ct,
                    bit_depth: bd,
                    channels: 1,
                    pixel_depth: bd,
                };
                let mut row = [0x5au8; 64];
                (api.png_do_invert)(&mut ri, row.as_mut_ptr());
                format!("{:?} {}", ri, hex(&row[..24]))
            });
        }
    }
}

/// row 553
#[test]
fn do_swap_rejections() {
    for bd in [0u8, 1, 2, 4, 8, 15, 16, 17, 32, 255] {
        for ch in [1u8, 2, 3, 4] {
            same!(format!("png_do_swap(bd={},ch={})", bd, ch), |api| unsafe {
                let mut ri = png_row_info {
                    width: 4,
                    rowbytes: 32,
                    color_type: PNG_COLOR_TYPE_RGB as png_byte,
                    bit_depth: bd,
                    channels: ch,
                    pixel_depth: bd * ch,
                };
                let mut row = [0u8; 128];
                for (i, b) in row.iter_mut().enumerate() {
                    *b = i as u8;
                }
                (api.png_do_swap)(&mut ri, row.as_mut_ptr());
                format!("{:?} {}", ri, hex(&row[..40]))
            });
        }
    }
}

/// rows 554, 555
#[test]
fn do_packswap_rejections() {
    for bd in [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 255] {
        same!(format!("png_do_packswap(bd={})", bd), |api| unsafe {
            let mut ri = png_row_info {
                width: 8,
                rowbytes: 8,
                color_type: PNG_COLOR_TYPE_GRAY as png_byte,
                bit_depth: bd,
                channels: 1,
                pixel_depth: bd,
            };
            let mut row = [0u8; 32];
            for (i, b) in row.iter_mut().enumerate() {
                *b = (i as u8).wrapping_mul(17);
            }
            (api.png_do_packswap)(&mut ri, row.as_mut_ptr());
            format!("{:?} {}", ri, hex(&row[..16]))
        });
    }
}

/// rows 556, 557, 558
#[test]
fn do_strip_channel_rejections() {
    for ch in [0u8, 1, 2, 3, 4, 5, 8, 255] {
        for bd in [0u8, 1, 2, 4, 8, 12, 16, 17, 32, 255] {
            for at_start in [0i32, 1, -1, 999] {
                same!(
                    format!("png_do_strip_channel(ch={},bd={},at_start={})", ch, bd, at_start),
                    |api| unsafe {
                        let mut ri = png_row_info {
                            width: 4,
                            rowbytes: 32,
                            color_type: PNG_COLOR_TYPE_RGB_ALPHA as png_byte,
                            bit_depth: bd,
                            channels: ch,
                            pixel_depth: bd.wrapping_mul(ch),
                        };
                        let mut row = [0u8; 256];
                        for (i, b) in row.iter_mut().enumerate() {
                            *b = i as u8;
                        }
                        (api.png_do_strip_channel)(&mut ri, row.as_mut_ptr(), at_start);
                        format!("{:?} {}", ri, hex(&row[..40]))
                    }
                );
            }
        }
    }
}

/// rows 559, 560, 561
#[test]
fn do_bgr_rejections() {
    for ct in [0u8, 1, 2, 3, 4, 5, 6, 7, 255] {
        for bd in [0u8, 1, 2, 4, 8, 12, 16, 17, 255] {
            same!(format!("png_do_bgr(ct={},bd={})", ct, bd), |api| unsafe {
                let mut ri = png_row_info {
                    width: 4,
                    rowbytes: 32,
                    color_type: ct,
                    bit_depth: bd,
                    channels: 4,
                    pixel_depth: bd.wrapping_mul(4),
                };
                let mut row = [0u8; 256];
                for (i, b) in row.iter_mut().enumerate() {
                    *b = i as u8;
                }
                (api.png_do_bgr)(&mut ri, row.as_mut_ptr());
                format!("{:?} {}", ri, hex(&row[..40]))
            });
        }
    }
}

/// rows 562, 563
#[test]
fn do_check_palette_indexes_rejections() {
    // NOTE: png_do_check_palette_indexes reads png_ptr->row_buf without any NULL
    // check (pngtrans.c:742), so it must only be called on a png_struct that has
    // already allocated a row buffer.  png_write_row does that, so the session
    // below performs a real write first.  Calling it on a fresh png_struct would
    // be C undefined behaviour, not an error path.
    for np in [0i32, 1, 2, 3, 4, 16, 255, 256] {
        for bd in [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 255] {
            same!(
                format!("png_do_check_palette_indexes(np={},bd={})", np, bd),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_PALETTE, 8);
                    let pal = vec![png_color::default(); 256];
                    if np > 0 {
                        (api.png_set_PLTE)(s.png, s.info, pal.as_ptr(), np);
                    } else {
                        // MNG empty PLTE: num_palette == 0 with the valid bit set
                        (api.png_permit_mng_features)(s.png, PNG_ALL_MNG_FEATURES);
                        (api.png_set_PLTE)(s.png, s.info, std::ptr::null(), 0);
                    }
                    (api.png_set_check_for_invalid_index)(s.png, 1);
                    let info_ok = guard(|| (api.png_write_info)(s.png, s.info)).is_some();
                    let row = vec![0x03u8; 64];
                    let row_ok = info_ok
                        && guard(|| (api.png_write_row)(s.png, row.as_ptr())).is_some();
                    if !row_ok {
                        return format!("info={} row={}", info_ok, row_ok);
                    }
                    let mut ri = png_row_info {
                        width: 8,
                        rowbytes: 4,
                        color_type: PNG_COLOR_TYPE_PALETTE as png_byte,
                        bit_depth: bd,
                        channels: 1,
                        pixel_depth: bd,
                    };
                    (api.png_do_check_palette_indexes)(s.png, &mut ri);
                    format!(
                        "max={} {:?}",
                        (api.png_get_palette_max)(s.png, s.info),
                        ri
                    )
                }
            );
        }
    }
}

/// rows 564, 565
#[test]
fn set_user_transform_info_rejections() {
    // row 564
    same!("png_set_user_transform_info(NULL)", |api| unsafe {
        (api.png_set_user_transform_info)(std::ptr::null_mut(), 0x1234 as png_voidp, 8, 3);
        format!("{}", nn((api.png_get_user_transform_ptr)(std::ptr::null())))
    });
    for read in [false, true] {
        for depth in [-1i32, 0, 1, 8, 16, 17, 255, 256, i32::MIN, i32::MAX] {
            for chans in [-1i32, 0, 1, 4, 5, 255, 256, i32::MAX] {
                same!(
                    format!("png_set_user_transform_info(read={},d={},c={})", read, depth, chans),
                    |api| unsafe {
                        let (png, _kr, _kw);
                        if read {
                            let s = ReadSess::new(api, &[]);
                            png = s.png;
                            _kr = Some(s);
                            _kw = None;
                        } else {
                            let s = WriteSess::new(api);
                            png = s.png;
                            _kr = None;
                            _kw = Some(s);
                        }
                        (api.png_set_user_transform_info)(
                            png,
                            0x1234 as png_voidp,
                            depth,
                            chans,
                        );
                        format!("{}", nn((api.png_get_user_transform_ptr)(png)))
                    }
                );
            }
        }
    }
    // row 565: read struct with PNG_FLAG_ROW_INIT already set (i.e. after
    // png_read_update_info / png_start_read_image).
    for benign in [0i32, 1] {
        for use_start in [false, true] {
            same!(
                format!("user_transform_info after row init (benign={},start={})", benign, use_start),
                |api| unsafe {
                    let data = tiny_png();
                    let s = ReadSess::new(api, data);
                    (api.png_read_info)(s.png, s.info);
                    if use_start {
                        (api.png_start_read_image)(s.png);
                    } else {
                        (api.png_read_update_info)(s.png, s.info);
                    }
                    (api.png_set_benign_errors)(s.png, benign);
                    (api.png_set_user_transform_info)(s.png, 0x1234 as png_voidp, 8, 3);
                    format!("{}", nn((api.png_get_user_transform_ptr)(s.png)))
                }
            );
        }
    }
}

/// `png_rtran_ok` gating: a read transform requested too late, a read transform
/// on a write struct, and a write transform on a read struct.
#[test]
fn rtran_ordering_rejections() {
    const RT: [&str; 14] = [
        "background",
        "gamma",
        "expand",
        "expand_gray_1_2_4_to_8",
        "palette_to_rgb",
        "tRNS_to_alpha",
        "expand_16",
        "gray_to_rgb",
        "rgb_to_gray",
        "strip_alpha",
        "strip_16",
        "scale_16",
        "quantize",
        "alpha_mode",
    ];
    unsafe fn apply_rtran(api: &'static Api, png: png_structp, which: usize) -> bool {
        let bg = png_color_16 {
            index: 0,
            red: 1,
            green: 2,
            blue: 3,
            gray: 4,
        };
        let mut pal = vec![png_color::default(); 16];
        guard(|| match which {
            0 => (api.png_set_background)(png, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 1.0),
            1 => (api.png_set_gamma)(png, 2.2, 0.45455),
            2 => (api.png_set_expand)(png),
            3 => (api.png_set_expand_gray_1_2_4_to_8)(png),
            4 => (api.png_set_palette_to_rgb)(png),
            5 => (api.png_set_tRNS_to_alpha)(png),
            6 => (api.png_set_expand_16)(png),
            7 => (api.png_set_gray_to_rgb)(png),
            8 => (api.png_set_rgb_to_gray)(png, 1, 0.3, 0.6),
            9 => (api.png_set_strip_alpha)(png),
            10 => (api.png_set_strip_16)(png),
            11 => (api.png_set_scale_16)(png),
            12 => (api.png_set_quantize)(png, pal.as_mut_ptr(), 16, 16, std::ptr::null(), 1),
            _ => (api.png_set_alpha_mode)(png, PNG_ALPHA_STANDARD, 2.2),
        })
        .is_some()
    }
    for which in 0..RT.len() {
        for benign in [0i32, 1] {
            // (a) on a WRITE struct
            same!(
                format!("read transform {} on a write struct (benign={})", RT[which], benign),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    (api.png_set_benign_errors)(s.png, benign);
                    let ok = apply_rtran(api, s.png, which);
                    format!("ok={}", ok)
                }
            );
            // (b) on a READ struct before png_read_info (no IHDR yet)
            same!(
                format!("read transform {} before read_info (benign={})", RT[which], benign),
                |api| unsafe {
                    let s = ReadSess::new(api, tiny_png());
                    (api.png_set_benign_errors)(s.png, benign);
                    let ok = apply_rtran(api, s.png, which);
                    format!("ok={}", ok)
                }
            );
            // (c) on a READ struct after png_read_info (IHDR present, row not
            // initialised yet) -- the legal case, for contrast
            same!(
                format!("read transform {} after read_info (benign={})", RT[which], benign),
                |api| unsafe {
                    let s = ReadSess::new(api, tiny_png());
                    (api.png_read_info)(s.png, s.info);
                    (api.png_set_benign_errors)(s.png, benign);
                    let ok = apply_rtran(api, s.png, which);
                    format!("ok={}", ok)
                }
            );
            // (d) after png_read_update_info -> PNG_FLAG_ROW_INIT is set, so
            // png_rtran_ok reports "invalid after png_start_read_image or
            // png_read_update_info"
            same!(
                format!("read transform {} after read_update_info (benign={})", RT[which], benign),
                |api| unsafe {
                    let s = ReadSess::new(api, tiny_png());
                    (api.png_read_info)(s.png, s.info);
                    (api.png_read_update_info)(s.png, s.info);
                    (api.png_set_benign_errors)(s.png, benign);
                    let ok = apply_rtran(api, s.png, which);
                    format!("ok={}", ok)
                }
            );
            // (e) after png_start_read_image
            same!(
                format!("read transform {} after start_read_image (benign={})", RT[which], benign),
                |api| unsafe {
                    let s = ReadSess::new(api, tiny_png());
                    (api.png_read_info)(s.png, s.info);
                    (api.png_start_read_image)(s.png);
                    (api.png_set_benign_errors)(s.png, benign);
                    let ok = apply_rtran(api, s.png, which);
                    format!("ok={}", ok)
                }
            );
        }
    }
    // write-only transforms on a READ struct
    for benign in [0i32, 1] {
        same!(format!("write transforms on a read struct (benign={})", benign), |api| unsafe {
            let s = ReadSess::new(api, tiny_png());
            (api.png_read_info)(s.png, s.info);
            (api.png_set_benign_errors)(s.png, benign);
            let a = guard(|| (api.png_set_filler)(s.png, 0xff, PNG_FILLER_AFTER)).is_some();
            let b = guard(|| (api.png_set_filter)(s.png, PNG_FILTER_TYPE_BASE, PNG_ALL_FILTERS))
                .is_some();
            let c = guard(|| (api.png_set_compression_level)(s.png, 6)).is_some();
            let d = guard(|| (api.png_set_flush)(s.png, 3)).is_some();
            let e = guard(|| (api.png_set_compression_buffer_size)(s.png, 8192)).is_some();
            format!("{} {} {} {} {}", a, b, c, d, e)
        });
    }
}

// ===========================================================================
// The write-side setters named in the task brief.  (These live in pngwrite.c,
// not in the pngget/pngset/pngtrans section of ERRORS.md, but they are the
// `png_set_*` out-of-range validations the brief asks for.)
// ===========================================================================

#[test]
fn set_filter_rejections() {
    let methods = [
        i32::MIN,
        -1,
        PNG_FILTER_TYPE_BASE,
        1,
        2,
        63,
        PNG_INTRAPIXEL_DIFFERENCING,
        65,
        100,
        i32::MAX,
    ];
    let filters = [
        i32::MIN,
        -1,
        PNG_NO_FILTERS,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        PNG_FILTER_NONE,
        PNG_FILTER_SUB,
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_FAST_FILTERS,
        PNG_ALL_FILTERS,
        0x0f,
        0xff,
        0x100,
        999,
        i32::MAX,
    ];
    same!("png_set_filter(NULL)", |api| unsafe {
        (api.png_set_filter)(std::ptr::null_mut(), 0, PNG_ALL_FILTERS);
        "ok".to_string()
    });
    for &m in &methods {
        for &f in &filters {
            for mng in [false, true] {
                for benign in [0i32, 1] {
                    same!(
                        format!("png_set_filter({},{:#x},mng={},benign={})", m, f, mng, benign),
                        |api| unsafe {
                            let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                            (api.png_set_benign_errors)(s.png, benign);
                            if mng {
                                (api.png_permit_mng_features)(s.png, PNG_ALL_MNG_FEATURES);
                            }
                            let set_ok = guard(|| (api.png_set_filter)(s.png, m, f)).is_some();
                            // After a png_error the png_struct must be abandoned
                            // (see the note in the mid-write block below), so the
                            // write is only continued when the setter survived.
                            if !set_ok {
                                return "set=false".to_string();
                            }
                            let info_ok =
                                guard(|| (api.png_write_info)(s.png, s.info)).is_some();
                            let row = vec![0x21u8; 8 * 3];
                            let mut rows_ok = info_ok;
                            if info_ok {
                                for _ in 0..8 {
                                    if guard(|| (api.png_write_row)(s.png, row.as_ptr()))
                                        .is_none()
                                    {
                                        rows_ok = false;
                                        break;
                                    }
                                }
                            }
                            let end_ok = rows_ok
                                && guard(|| (api.png_write_end)(s.png, s.info)).is_some();
                            format!(
                                "set={} info={} rows={} end={} out={}",
                                set_ok,
                                info_ok,
                                rows_ok,
                                end_ok,
                                dig(&s.sink.buf)
                            )
                        }
                    );
                }
            }
        }
    }
    // ... and once the row buffers exist, so that the "too late for this filter"
    // benign error inside png_set_filter can fire.
    for &f in &[
        PNG_NO_FILTERS,
        PNG_FILTER_NONE,
        PNG_FILTER_SUB,
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_ALL_FILTERS,
    ] {
        for &first in &[PNG_NO_FILTERS, PNG_FILTER_NONE, PNG_ALL_FILTERS] {
            for benign in [0i32, 1] {
                same!(
                    format!("png_set_filter({:#x}) mid-write (first={:#x},benign={})", f, first, benign),
                    |api| unsafe {
                        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                        (api.png_set_filter)(s.png, PNG_FILTER_TYPE_BASE, first);
                        (api.png_write_info)(s.png, s.info);
                        let row = vec![0x21u8; 8 * 3];
                        (api.png_write_row)(s.png, row.as_ptr());
                        (api.png_set_benign_errors)(s.png, benign);
                        let ok =
                            guard(|| (api.png_set_filter)(s.png, PNG_FILTER_TYPE_BASE, f))
                                .is_some();
                        // NOTE: when this png_set_filter raises the
                        // "UP/AVG/PAETH cannot be added after start" app_error
                        // (i.e. benign errors are off), the C longjmps out of
                        // pngwrite.c:1141 *after* the switch at pngwrite.c:1096
                        // has already stored the new filter mask in
                        // png_ptr->do_filter, but *before* pngwrite.c:1143
                        // masks UP/AVG/PAETH back out.  png_ptr->prev_row and
                        // png_ptr->try_row are still NULL, so a subsequent
                        // png_write_row dereferences NULL in
                        // png_write_find_filter.  Continuing to use a png_struct
                        // after png_error is undefined by the libpng API
                        // contract (the app is supposed to longjmp out and
                        // destroy the struct), and the *C* library segfaults
                        // here just as the Rust one would, so the write is
                        // abandoned instead.
                        if !ok {
                            return "set=false".to_string();
                        }
                        let mut rows_ok = true;
                        for _ in 0..7 {
                            if guard(|| (api.png_write_row)(s.png, row.as_ptr())).is_none() {
                                rows_ok = false;
                                break;
                            }
                        }
                        let end_ok =
                            rows_ok && guard(|| (api.png_write_end)(s.png, s.info)).is_some();
                        format!(
                            "set={} rows={} end={} out={}",
                            ok,
                            rows_ok,
                            end_ok,
                            dig(&s.sink.buf)
                        )
                    }
                );
            }
        }
    }
}

#[test]
fn set_filter_heuristics_rejections() {
    // png_set_filter_heuristics{,_fixed} are deprecated no-op stubs in 1.6.x:
    // every argument, valid or not, must be ignored silently.
    let methods = [
        i32::MIN,
        -1,
        PNG_FILTER_HEURISTIC_DEFAULT,
        PNG_FILTER_HEURISTIC_UNWEIGHTED,
        PNG_FILTER_HEURISTIC_WEIGHTED,
        PNG_FILTER_HEURISTIC_LAST,
        PNG_FILTER_HEURISTIC_LAST + 1,
        999,
        i32::MAX,
    ];
    for &m in &methods {
        for nw in [i32::MIN, -1i32, 0, 1, 5, 1000, i32::MAX] {
            for nulls in [false, true] {
                same!(
                    format!("png_set_filter_heuristics({},{},nulls={})", m, nw, nulls),
                    |api| unsafe {
                        let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                        let w = [0.0f64, -1.0, 1e300, 1.0, 2.0];
                        let c = [1.0f64, 0.0, -1.0, 1e-300, 3.0];
                        let (pw, pc) = if nulls {
                            (std::ptr::null(), std::ptr::null())
                        } else {
                            (w.as_ptr(), c.as_ptr())
                        };
                        (api.png_set_filter_heuristics)(s.png, m, nw, pw, pc);
                        let wf = [0i32, -1, i32::MAX, 100_000, 1];
                        let cf = [1i32, 0, -1, i32::MIN, 2];
                        let (pwf, pcf) = if nulls {
                            (std::ptr::null(), std::ptr::null())
                        } else {
                            (wf.as_ptr(), cf.as_ptr())
                        };
                        (api.png_set_filter_heuristics_fixed)(s.png, m, nw, pwf, pcf);
                        (api.png_set_filter_heuristics)(
                            std::ptr::null_mut(),
                            m,
                            nw,
                            std::ptr::null(),
                            std::ptr::null(),
                        );
                        (api.png_set_filter_heuristics_fixed)(
                            std::ptr::null_mut(),
                            m,
                            nw,
                            std::ptr::null(),
                            std::ptr::null(),
                        );
                        let info_ok = guard(|| (api.png_write_info)(s.png, s.info)).is_some();
                        let row = vec![0x11u8; 8 * 3];
                        let mut rows_ok = info_ok;
                        if info_ok {
                            for _ in 0..8 {
                                if guard(|| (api.png_write_row)(s.png, row.as_ptr())).is_none()
                                {
                                    rows_ok = false;
                                    break;
                                }
                            }
                        }
                        let end_ok =
                            rows_ok && guard(|| (api.png_write_end)(s.png, s.info)).is_some();
                        format!(
                            "info={} rows={} end={} out={}",
                            info_ok,
                            rows_ok,
                            end_ok,
                            dig(&s.sink.buf)
                        )
                    }
                );
            }
        }
    }
}

#[test]
fn set_compression_rejections() {
    // Every out-of-range zlib parameter, on both the IDAT and the text streams.
    // 0 = level, 1 = mem_level, 2 = strategy, 3 = window_bits, 4 = method.
    let cases: [(usize, i32); 44] = [
        (0, i32::MIN),
        (0, -2),
        (0, -1),
        (0, 0),
        (0, 1),
        (0, 6),
        (0, 9),
        (0, 10),
        (0, 100),
        (0, i32::MAX),
        (1, i32::MIN),
        (1, -1),
        (1, 0),
        (1, 1),
        (1, 8),
        (1, 9),
        (1, 10),
        (1, 100),
        (1, i32::MAX),
        (2, i32::MIN),
        (2, -1),
        (2, 0),
        (2, 1),
        (2, 2),
        (2, 3),
        (2, 4),
        (2, 5),
        (2, 100),
        (2, i32::MAX),
        (3, i32::MIN),
        (3, -15),
        (3, -1),
        (3, 0),
        (3, 7),
        (3, 8),
        (3, 15),
        (3, 16),
        (3, 100),
        (3, i32::MAX),
        (4, i32::MIN),
        (4, 0),
        (4, 8),
        (4, 9),
        (4, i32::MAX),
    ];
    for (which, v) in cases {
        same!(format!("compression setters NULL which={} v={}", which, v), |api| unsafe {
            let p: png_structp = std::ptr::null_mut();
            match which {
                0 => {
                    (api.png_set_compression_level)(p, v);
                    (api.png_set_text_compression_level)(p, v);
                }
                1 => {
                    (api.png_set_compression_mem_level)(p, v);
                    (api.png_set_text_compression_mem_level)(p, v);
                }
                2 => {
                    (api.png_set_compression_strategy)(p, v);
                    (api.png_set_text_compression_strategy)(p, v);
                }
                3 => {
                    (api.png_set_compression_window_bits)(p, v);
                    (api.png_set_text_compression_window_bits)(p, v);
                }
                _ => {
                    (api.png_set_compression_method)(p, v);
                    (api.png_set_text_compression_method)(p, v);
                }
            }
            "ok".to_string()
        });
        for text_stream in [false, true] {
            same!(
                format!("compression setter which={} v={} text={}", which, v, text_stream),
                |api| unsafe {
                    let s = wsess(api, PNG_COLOR_TYPE_RGB, 8);
                    let set_ok = guard(|| {
                        if text_stream {
                            match which {
                                0 => (api.png_set_text_compression_level)(s.png, v),
                                1 => (api.png_set_text_compression_mem_level)(s.png, v),
                                2 => (api.png_set_text_compression_strategy)(s.png, v),
                                3 => (api.png_set_text_compression_window_bits)(s.png, v),
                                _ => (api.png_set_text_compression_method)(s.png, v),
                            }
                        } else {
                            match which {
                                0 => (api.png_set_compression_level)(s.png, v),
                                1 => (api.png_set_compression_mem_level)(s.png, v),
                                2 => (api.png_set_compression_strategy)(s.png, v),
                                3 => (api.png_set_compression_window_bits)(s.png, v),
                                _ => (api.png_set_compression_method)(s.png, v),
                            }
                        }
                    })
                    .is_some();
                    // a zTXt chunk so that the text stream is actually used
                    let key = cs("Comment");
                    let val = cs("compressible text compressible text compressible text");
                    let t = mk_text(
                        PNG_TEXT_COMPRESSION_zTXt,
                        key.as_ptr() as png_charp,
                        val.as_ptr() as png_charp,
                        std::ptr::null_mut(),
                    );
                    (api.png_set_text)(s.png, s.info, &t, 1);
                    let info_ok = guard(|| (api.png_write_info)(s.png, s.info)).is_some();
                    let row = vec![0x5cu8; 8 * 3];
                    let mut rows_ok = info_ok;
                    if info_ok {
                        for _ in 0..8 {
                            if guard(|| (api.png_write_row)(s.png, row.as_ptr())).is_none() {
                                rows_ok = false;
                                break;
                            }
                        }
                    }
                    let end_ok =
                        rows_ok && guard(|| (api.png_write_end)(s.png, s.info)).is_some();
                    format!(
                        "set={} info={} rows={} end={} out={}",
                        set_ok,
                        info_ok,
                        rows_ok,
                        end_ok,
                        dig(&s.sink.buf)
                    )
                }
            );
        }
    }
}


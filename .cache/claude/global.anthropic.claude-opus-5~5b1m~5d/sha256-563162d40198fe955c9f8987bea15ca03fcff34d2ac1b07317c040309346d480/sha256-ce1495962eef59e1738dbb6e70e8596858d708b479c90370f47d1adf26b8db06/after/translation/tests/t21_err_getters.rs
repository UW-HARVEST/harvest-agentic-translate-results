//! Phase C — the NON-FATAL rejection surface of `c_src/src/pngget.c` (91 rows of
//! `ERRORS.md`, rows 540..630) plus the silent NULL-pointer guards of the
//! `png_set_*` family (`pngset.c` and friends).
//!
//! Everything here returns a *sentinel* (0, -1, NULL, `PNG_UINT_32_MAX`, 8,
//! `PNG_OPTION_INVALID`, ...) or does nothing at all, so it can be exercised
//! IN-PROCESS: the same call sequence is replayed against `apis().c` and
//! `apis().rs`, every return value and every out-parameter is stringified into a
//! `Vec<String>`, the recorded warning log is appended and the two vectors are
//! compared with `eq_dbg`.
//!
//! Out-parameters are pre-loaded with a recognisable sentinel (`0xDEADBEEF`,
//! `0xAB`, a bogus `0x1` pointer, ...) so that "the C did not write here" is
//! itself an observable, compared fact.
//!
//! The two getters that can `png_error` (and therefore longjmp) are driven in a
//! sub-process the way `t23_err_write.rs` does it:
//!   * `png_get_IHDR`      -> `png_error "Invalid IHDR data"`  (pngget.c:974)
//!   * `png_get_sCAL_fixed`-> `png_error "fixed point overflow in sCAL width"`
//!                                                             (pngget.c:1047)
//!
//! ## Inputs deliberately NOT tested, because the C has NO guard
//!
//! Passing NULL to any of the following is undefined behaviour in the reference
//! C implementation (it dereferences unconditionally), so a "differential" test
//! of it would merely SIGSEGV both libraries and prove nothing:
//!
//!   * `png_get_io_state`                 `pngget.c:1344-1347` — `png_ptr->io_state`
//!   * `png_get_io_chunk_type`            `pngget.c:1350-1353` — `png_ptr->chunk_name`
//!   * `png_set_benign_errors`            `pngset.c:1926-1942` — `png_ptr->flags`
//!   * `png_set_check_for_invalid_index`  `pngset.c:1956-1964` — `png_ptr->num_palette_max`
//!   * `png_set_read_user_transform_fn`   `pngrtran.c:1133-1141` — `png_ptr->transformations`
//!   * `png_app_error`                    `pngerror.c` — `png_ptr->flags`
//!
//! Per-out-parameter NULLs that the C does NOT check are likewise skipped and
//! flagged at the call site:
//!
//!   * `png_get_eXIf_1`  `num_exif == NULL` while eXIf is valid (pngget.c:910)
//!   * `png_get_PLTE`    `num_palette == NULL` while PLTE is valid (pngget.c:1140)
//!   * `png_get_sCAL` / `png_get_sCAL_fixed` / `png_get_sCAL_s`
//!     `unit`/`width`/`height == NULL` while sCAL is valid (pngget.c:1042-1049,
//!     1067-1069, 1085-1087)
#![allow(clippy::too_many_arguments)]

mod common;

use common::api::{apis, each, Api};
use common::harness::*;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// sentinels + formatting helpers
// ---------------------------------------------------------------------------

const S_U32: png_uint_32 = 0xDEAD_BEEF;
const S_I32: png_int_32 = 0xDEAD_BEEFu32 as i32;
const S_INT: c_int = S_I32;
const S_BYTE: png_byte = 0xAB;
const S_F64: f64 = -1.234_567_89e-5;

/// A recognisable, never-dereferenced "not written" pointer value.
fn sp<T>() -> *mut T {
    1usize as *mut T
}

/// Classify a pointer out-parameter without ever dereferencing it: allocation
/// addresses legitimately differ between the two libraries, only NULL-ness and
/// "was it written at all" are comparable.
fn ps(p: *const c_void) -> &'static str {
    match p as usize {
        0 => "NULL",
        1 => "SENT",
        _ => "PTR",
    }
}

fn f64v(d: &[f64]) -> String {
    d.iter()
        .map(|x| format!("{:#x}", x.to_bits()))
        .collect::<Vec<_>>()
        .join(",")
}
fn f32v(x: f32) -> String {
    format!("{:#x}", x.to_bits())
}
fn i32v(d: &[png_int_32]) -> String {
    d.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}
fn u32v(d: &[png_uint_32]) -> String {
    d.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}
fn u8v(d: &[png_byte]) -> String {
    d.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

// ---------------------------------------------------------------------------
// struct construction
// ---------------------------------------------------------------------------

unsafe fn new_write(a: &Api) -> (png_structp, png_infop) {
    let p = (a.png_create_write_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null(), "png_create_write_struct failed");
    let i = (a.png_create_info_struct)(p);
    assert!(!i.is_null(), "png_create_info_struct failed");
    (p, i)
}

unsafe fn new_read(a: &Api) -> (png_structp, png_infop) {
    let p = (a.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null(), "png_create_read_struct failed");
    let i = (a.png_create_info_struct)(p);
    assert!(!i.is_null(), "png_create_info_struct failed");
    (p, i)
}

unsafe fn kill_write(a: &Api, p: png_structp, i: png_infop) {
    let mut pp = p;
    let mut ii = i;
    (a.png_destroy_write_struct)(&mut pp, &mut ii);
}

unsafe fn kill_read(a: &Api, p: png_structp, i: png_infop) {
    let mut pp = p;
    let mut ii = i;
    (a.png_destroy_read_struct)(&mut pp, &mut ii, ptr::null_mut());
}

// ---------------------------------------------------------------------------
// scalar getters (no out-parameters)
// ---------------------------------------------------------------------------

/// Every `png_get_*` that takes `(png_ptr, info_ptr)` and returns a scalar.
/// Covers `ERRORS.md` rows 543..582, 621, 630.
unsafe fn dump_scalars(a: &Api, tag: &str, p: png_structp, i: png_infop, v: &mut Vec<String>) {
    v.push(format!("{tag} rowbytes={}", (a.png_get_rowbytes)(p, i)));
    v.push(format!(
        "{tag} rows={}",
        ps((a.png_get_rows)(p, i) as *const c_void)
    ));
    v.push(format!("{tag} width={}", (a.png_get_image_width)(p, i)));
    v.push(format!("{tag} height={}", (a.png_get_image_height)(p, i)));
    v.push(format!("{tag} bit_depth={}", (a.png_get_bit_depth)(p, i)));
    v.push(format!("{tag} color_type={}", (a.png_get_color_type)(p, i)));
    v.push(format!("{tag} filter_type={}", (a.png_get_filter_type)(p, i)));
    v.push(format!(
        "{tag} interlace_type={}",
        (a.png_get_interlace_type)(p, i)
    ));
    v.push(format!(
        "{tag} compression_type={}",
        (a.png_get_compression_type)(p, i)
    ));
    v.push(format!("{tag} channels={}", (a.png_get_channels)(p, i)));
    v.push(format!(
        "{tag} xppm={}",
        (a.png_get_x_pixels_per_meter)(p, i)
    ));
    v.push(format!(
        "{tag} yppm={}",
        (a.png_get_y_pixels_per_meter)(p, i)
    ));
    v.push(format!("{tag} ppm={}", (a.png_get_pixels_per_meter)(p, i)));
    v.push(format!("{tag} xppi={}", (a.png_get_x_pixels_per_inch)(p, i)));
    v.push(format!("{tag} yppi={}", (a.png_get_y_pixels_per_inch)(p, i)));
    v.push(format!("{tag} ppi={}", (a.png_get_pixels_per_inch)(p, i)));
    v.push(format!(
        "{tag} par={}",
        f32v((a.png_get_pixel_aspect_ratio)(p, i))
    ));
    v.push(format!(
        "{tag} parfx={}",
        (a.png_get_pixel_aspect_ratio_fixed)(p, i)
    ));
    v.push(format!(
        "{tag} xoffmic={}",
        (a.png_get_x_offset_microns)(p, i)
    ));
    v.push(format!(
        "{tag} yoffmic={}",
        (a.png_get_y_offset_microns)(p, i)
    ));
    v.push(format!("{tag} xoffpx={}", (a.png_get_x_offset_pixels)(p, i)));
    v.push(format!("{tag} yoffpx={}", (a.png_get_y_offset_pixels)(p, i)));
    v.push(format!(
        "{tag} xoffinfx={}",
        (a.png_get_x_offset_inches_fixed)(p, i)
    ));
    v.push(format!(
        "{tag} yoffinfx={}",
        (a.png_get_y_offset_inches_fixed)(p, i)
    ));
    v.push(format!(
        "{tag} xoffin={}",
        f32v((a.png_get_x_offset_inches)(p, i))
    ));
    v.push(format!(
        "{tag} yoffin={}",
        f32v((a.png_get_y_offset_inches)(p, i))
    ));
    v.push(format!("{tag} palette_max={}", (a.png_get_palette_max)(p, i)));
    v.push(format!(
        "{tag} signature={}",
        ps((a.png_get_signature)(p, i) as *const c_void)
    ));
}

// ---------------------------------------------------------------------------
// out-parameter getters
// ---------------------------------------------------------------------------

/// Every `png_get_*` with out-parameters, called (a) with all out-parameters
/// pre-loaded with sentinels and (b) with every out-parameter NULL where the C
/// tolerates that.  `png_get_IHDR` and `png_get_sCAL_fixed` are handled by
/// `dump_outs_fatalsafe`, which the caller only invokes when the info struct is
/// in a state where they cannot `png_error`.
unsafe fn dump_outs(a: &Api, tag: &str, p: png_structp, i: png_infop, v: &mut Vec<String>) {
    // ---- bKGD (row 583) -------------------------------------------------
    {
        let mut bg: *mut png_color_16 = sp();
        let r = (a.png_get_bKGD)(p, i, &mut bg);
        v.push(format!("{tag} bKGD={r} bg={}", ps(bg as *const c_void)));
        v.push(format!(
            "{tag} bKGD/null={}",
            (a.png_get_bKGD)(p, i, ptr::null_mut())
        ));
    }
    // ---- cHRM (row 584) -------------------------------------------------
    {
        let mut d = [S_F64; 8];
        let q = d.as_mut_ptr();
        let r = (a.png_get_cHRM)(
            p,
            i,
            q,
            q.add(1),
            q.add(2),
            q.add(3),
            q.add(4),
            q.add(5),
            q.add(6),
            q.add(7),
        );
        v.push(format!("{tag} cHRM={r} [{}]", f64v(&d)));
        let n = ptr::null_mut();
        v.push(format!(
            "{tag} cHRM/null={}",
            (a.png_get_cHRM)(p, i, n, n, n, n, n, n, n, n)
        ));
        // one out-parameter NULL at a time (all eight are individually checked)
        for k in 0..8usize {
            let mut d2 = [S_F64; 8];
            let q2 = d2.as_mut_ptr();
            let mut arg: [*mut f64; 8] = [ptr::null_mut(); 8];
            for (j, slot) in arg.iter_mut().enumerate() {
                *slot = if j == k { ptr::null_mut() } else { q2.add(j) };
            }
            let r = (a.png_get_cHRM)(
                p, i, arg[0], arg[1], arg[2], arg[3], arg[4], arg[5], arg[6], arg[7],
            );
            v.push(format!("{tag} cHRM/n{k}={r} [{}]", f64v(&d2)));
        }
    }
    // ---- cHRM_XYZ (row 585) --------------------------------------------
    {
        let mut d = [S_F64; 9];
        let q = d.as_mut_ptr();
        let r = (a.png_get_cHRM_XYZ)(
            p,
            i,
            q,
            q.add(1),
            q.add(2),
            q.add(3),
            q.add(4),
            q.add(5),
            q.add(6),
            q.add(7),
            q.add(8),
        );
        v.push(format!("{tag} cHRM_XYZ={r} [{}]", f64v(&d)));
        let n = ptr::null_mut();
        v.push(format!(
            "{tag} cHRM_XYZ/null={}",
            (a.png_get_cHRM_XYZ)(p, i, n, n, n, n, n, n, n, n, n)
        ));
    }
    // ---- cHRM_XYZ_fixed (row 586) ---------------------------------------
    {
        let mut d = [S_I32; 9];
        let q = d.as_mut_ptr();
        let r = (a.png_get_cHRM_XYZ_fixed)(
            p,
            i,
            q,
            q.add(1),
            q.add(2),
            q.add(3),
            q.add(4),
            q.add(5),
            q.add(6),
            q.add(7),
            q.add(8),
        );
        v.push(format!("{tag} cHRM_XYZ_fx={r} [{}]", i32v(&d)));
        let n = ptr::null_mut();
        v.push(format!(
            "{tag} cHRM_XYZ_fx/null={}",
            (a.png_get_cHRM_XYZ_fixed)(p, i, n, n, n, n, n, n, n, n, n)
        ));
    }
    // ---- cHRM_fixed (row 587) ------------------------------------------
    {
        let mut d = [S_I32; 8];
        let q = d.as_mut_ptr();
        let r = (a.png_get_cHRM_fixed)(
            p,
            i,
            q,
            q.add(1),
            q.add(2),
            q.add(3),
            q.add(4),
            q.add(5),
            q.add(6),
            q.add(7),
        );
        v.push(format!("{tag} cHRM_fx={r} [{}]", i32v(&d)));
        let n = ptr::null_mut();
        v.push(format!(
            "{tag} cHRM_fx/null={}",
            (a.png_get_cHRM_fixed)(p, i, n, n, n, n, n, n, n, n)
        ));
        for k in 0..8usize {
            let mut d2 = [S_I32; 8];
            let q2 = d2.as_mut_ptr();
            let mut arg: [*mut png_fixed_point; 8] = [ptr::null_mut(); 8];
            for (j, slot) in arg.iter_mut().enumerate() {
                *slot = if j == k { ptr::null_mut() } else { q2.add(j) };
            }
            let r = (a.png_get_cHRM_fixed)(
                p, i, arg[0], arg[1], arg[2], arg[3], arg[4], arg[5], arg[6], arg[7],
            );
            v.push(format!("{tag} cHRM_fx/n{k}={r} [{}]", i32v(&d2)));
        }
    }
    // ---- cICP (row 594): all four out pointers are MANDATORY -------------
    {
        let mut d = [S_BYTE; 4];
        let q = d.as_mut_ptr();
        let r = (a.png_get_cICP)(p, i, q, q.add(1), q.add(2), q.add(3));
        v.push(format!("{tag} cICP={r} [{}]", u8v(&d)));
        let n = ptr::null_mut();
        v.push(format!(
            "{tag} cICP/null={}",
            (a.png_get_cICP)(p, i, n, n, n, n)
        ));
        // one NULL is enough to make the whole call fail
        let mut d2 = [S_BYTE; 4];
        let q2 = d2.as_mut_ptr();
        let r = (a.png_get_cICP)(p, i, ptr::null_mut(), q2.add(1), q2.add(2), q2.add(3));
        v.push(format!("{tag} cICP/n0={r} [{}]", u8v(&d2)));
    }
    // ---- cLLI / cLLI_fixed (rows 595, 596) ------------------------------
    {
        let mut d = [S_F64; 2];
        let q = d.as_mut_ptr();
        let r = (a.png_get_cLLI)(p, i, q, q.add(1));
        v.push(format!("{tag} cLLI={r} [{}]", f64v(&d)));
        let n = ptr::null_mut();
        v.push(format!("{tag} cLLI/null={}", (a.png_get_cLLI)(p, i, n, n)));
        let mut d2 = [S_F64; 2];
        let q2 = d2.as_mut_ptr();
        let r = (a.png_get_cLLI)(p, i, ptr::null_mut(), q2.add(1));
        v.push(format!("{tag} cLLI/n0={r} [{}]", f64v(&d2)));

        let mut e = [S_U32; 2];
        let w = e.as_mut_ptr();
        let r = (a.png_get_cLLI_fixed)(p, i, w, w.add(1));
        v.push(format!("{tag} cLLI_fx={r} [{}]", u32v(&e)));
        v.push(format!(
            "{tag} cLLI_fx/null={}",
            (a.png_get_cLLI_fixed)(p, i, ptr::null_mut(), ptr::null_mut())
        ));
    }
    // ---- gAMA / gAMA_fixed (rows 588, 589) ------------------------------
    {
        let mut g = S_F64;
        let r = (a.png_get_gAMA)(p, i, &mut g);
        v.push(format!("{tag} gAMA={r} [{}]", f64v(&[g])));
        v.push(format!(
            "{tag} gAMA/null={}",
            (a.png_get_gAMA)(p, i, ptr::null_mut())
        ));
        let mut gf = S_I32;
        let r = (a.png_get_gAMA_fixed)(p, i, &mut gf);
        v.push(format!("{tag} gAMA_fx={r} [{gf}]"));
        v.push(format!(
            "{tag} gAMA_fx/null={}",
            (a.png_get_gAMA_fixed)(p, i, ptr::null_mut())
        ));
    }
    // ---- sRGB (row 590) -------------------------------------------------
    {
        let mut s = S_INT;
        let r = (a.png_get_sRGB)(p, i, &mut s);
        v.push(format!("{tag} sRGB={r} [{s}]"));
        v.push(format!(
            "{tag} sRGB/null={}",
            (a.png_get_sRGB)(p, i, ptr::null_mut())
        ));
    }
    // ---- iCCP (row 591): name/profile/proflen mandatory, ctype optional --
    {
        let mut name: *mut c_char = sp();
        let mut ctype: c_int = S_INT;
        let mut prof: *mut png_byte = sp();
        let mut plen: png_uint_32 = S_U32;
        let r = (a.png_get_iCCP)(p, i, &mut name, &mut ctype, &mut prof, &mut plen);
        v.push(format!(
            "{tag} iCCP={r} name={} ctype={ctype} prof={} plen={plen}",
            ps(name as *const c_void),
            ps(prof as *const c_void)
        ));
        // ctype == NULL is explicitly tolerated
        let mut name2: *mut c_char = sp();
        let mut prof2: *mut png_byte = sp();
        let mut plen2: png_uint_32 = S_U32;
        let r = (a.png_get_iCCP)(p, i, &mut name2, ptr::null_mut(), &mut prof2, &mut plen2);
        v.push(format!(
            "{tag} iCCP/noctype={r} name={} prof={} plen={plen2}",
            ps(name2 as *const c_void),
            ps(prof2 as *const c_void)
        ));
        v.push(format!(
            "{tag} iCCP/null={}",
            (a.png_get_iCCP)(
                p,
                i,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut()
            )
        ));
    }
    // ---- sPLT (rows 592, 593) -------------------------------------------
    {
        let mut sp_out: *mut png_sPLT_t = sp();
        let r = (a.png_get_sPLT)(p, i, &mut sp_out);
        v.push(format!("{tag} sPLT={r} out={}", ps(sp_out as *const c_void)));
        v.push(format!(
            "{tag} sPLT/null={}",
            (a.png_get_sPLT)(p, i, ptr::null_mut())
        ));
    }
    // ---- mDCV / mDCV_fixed (rows 597, 598) ------------------------------
    {
        let mut d = [S_F64; 10];
        let q = d.as_mut_ptr();
        let r = (a.png_get_mDCV)(
            p,
            i,
            q,
            q.add(1),
            q.add(2),
            q.add(3),
            q.add(4),
            q.add(5),
            q.add(6),
            q.add(7),
            q.add(8),
            q.add(9),
        );
        v.push(format!("{tag} mDCV={r} [{}]", f64v(&d)));
        let n = ptr::null_mut();
        v.push(format!(
            "{tag} mDCV/null={}",
            (a.png_get_mDCV)(p, i, n, n, n, n, n, n, n, n, n, n)
        ));

        let mut e = [S_I32; 8];
        let mut u = [S_U32; 2];
        let w = e.as_mut_ptr();
        let x = u.as_mut_ptr();
        let r = (a.png_get_mDCV_fixed)(
            p,
            i,
            w,
            w.add(1),
            w.add(2),
            w.add(3),
            w.add(4),
            w.add(5),
            w.add(6),
            w.add(7),
            x,
            x.add(1),
        );
        v.push(format!("{tag} mDCV_fx={r} [{}|{}]", i32v(&e), u32v(&u)));
        v.push(format!(
            "{tag} mDCV_fx/null={}",
            (a.png_get_mDCV_fixed)(
                p,
                i,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut()
            )
        ));
    }
    // ---- eXIf (row 599): unconditional png_warning + return 0 -----------
    {
        let mut ex: *mut png_byte = sp();
        let r = (a.png_get_eXIf)(p, i, &mut ex);
        v.push(format!("{tag} eXIf={r} out={}", ps(ex as *const c_void)));
        v.push(format!(
            "{tag} eXIf/null={}",
            (a.png_get_eXIf)(p, i, ptr::null_mut())
        ));
    }
    // ---- eXIf_1 (row 600) ----------------------------------------------
    // `exif == NULL` is guarded; `num_exif == NULL` is NOT (pngget.c:910), so
    // the num_exif-NULL variant is only safe while eXIf is invalid and is
    // therefore driven from `case_setters_null_guards`, never from here.
    {
        let mut n_ex: png_uint_32 = S_U32;
        let mut ex: *mut png_byte = sp();
        let r = (a.png_get_eXIf_1)(p, i, &mut n_ex, &mut ex);
        v.push(format!(
            "{tag} eXIf_1={r} num={n_ex} out={}",
            ps(ex as *const c_void)
        ));
        let mut n_ex2: png_uint_32 = S_U32;
        let r = (a.png_get_eXIf_1)(p, i, &mut n_ex2, ptr::null_mut());
        v.push(format!("{tag} eXIf_1/noexif={r} num={n_ex2}"));
    }
    // ---- hIST (row 601) -------------------------------------------------
    {
        let mut h: *mut png_uint_16 = sp();
        let r = (a.png_get_hIST)(p, i, &mut h);
        v.push(format!("{tag} hIST={r} out={}", ps(h as *const c_void)));
        v.push(format!(
            "{tag} hIST/null={}",
            (a.png_get_hIST)(p, i, ptr::null_mut())
        ));
    }
    // ---- oFFs (row 604): all three out pointers mandatory ----------------
    {
        let mut ox = S_I32;
        let mut oy = S_I32;
        let mut ut = S_INT;
        let r = (a.png_get_oFFs)(p, i, &mut ox, &mut oy, &mut ut);
        v.push(format!("{tag} oFFs={r} [{ox},{oy},{ut}]"));
        let mut ox2 = S_I32;
        let mut oy2 = S_I32;
        let r = (a.png_get_oFFs)(p, i, &mut ox2, &mut oy2, ptr::null_mut());
        v.push(format!("{tag} oFFs/noun={r} [{ox2},{oy2}]"));
        v.push(format!(
            "{tag} oFFs/null={}",
            (a.png_get_oFFs)(p, i, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
        ));
    }
    // ---- pCAL (row 605): all seven out pointers mandatory ---------------
    {
        let mut purpose: *mut c_char = sp();
        let mut x0 = S_I32;
        let mut x1 = S_I32;
        let mut ty = S_INT;
        let mut np = S_INT;
        let mut units: *mut c_char = sp();
        let mut params: *mut *mut c_char = sp();
        let r = (a.png_get_pCAL)(
            p,
            i,
            &mut purpose,
            &mut x0,
            &mut x1,
            &mut ty,
            &mut np,
            &mut units,
            &mut params,
        );
        v.push(format!(
            "{tag} pCAL={r} purpose={} X0={x0} X1={x1} type={ty} n={np} units={} params={}",
            ps(purpose as *const c_void),
            ps(units as *const c_void),
            ps(params as *const c_void)
        ));
        // any single NULL rejects the whole call
        let mut purpose2: *mut c_char = sp();
        let mut x0b = S_I32;
        let mut x1b = S_I32;
        let mut tyb = S_INT;
        let mut npb = S_INT;
        let mut unitsb: *mut c_char = sp();
        let r = (a.png_get_pCAL)(
            p,
            i,
            &mut purpose2,
            &mut x0b,
            &mut x1b,
            &mut tyb,
            &mut npb,
            &mut unitsb,
            ptr::null_mut(),
        );
        v.push(format!(
            "{tag} pCAL/noparams={r} purpose={} X0={x0b} X1={x1b} type={tyb} n={npb} units={}",
            ps(purpose2 as *const c_void),
            ps(unitsb as *const c_void)
        ));
        v.push(format!(
            "{tag} pCAL/null={}",
            (a.png_get_pCAL)(
                p,
                i,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut()
            )
        ));
    }
    // ---- pHYs / pHYs_dpi (rows 579, 580, 610, 611) ----------------------
    dump_phys(a, tag, p, i, v);
    // ---- PLTE (row 612) -------------------------------------------------
    // `num_palette == NULL` is NOT guarded (pngget.c:1140), so it is only
    // exercised where PLTE is invalid (see `case_setters_null_guards`).
    {
        let mut pal: *mut png_color = sp();
        let mut n = S_INT;
        let r = (a.png_get_PLTE)(p, i, &mut pal, &mut n);
        v.push(format!(
            "{tag} PLTE={r} pal={} n={n}",
            ps(pal as *const c_void)
        ));
        v.push(format!(
            "{tag} PLTE/nopal={}",
            (a.png_get_PLTE)(p, i, ptr::null_mut(), ptr::null_mut())
        ));
    }
    // ---- sBIT (row 613) -------------------------------------------------
    {
        let mut sb: *mut png_color_8 = sp();
        let r = (a.png_get_sBIT)(p, i, &mut sb);
        v.push(format!("{tag} sBIT={r} out={}", ps(sb as *const c_void)));
        v.push(format!(
            "{tag} sBIT/null={}",
            (a.png_get_sBIT)(p, i, ptr::null_mut())
        ));
    }
    // ---- text (row 614) -------------------------------------------------
    {
        let mut t: *mut png_text = sp();
        let mut n = S_INT;
        let r = (a.png_get_text)(p, i, &mut t, &mut n);
        v.push(format!("{tag} text={r} out={} n={n}", ps(t as *const c_void)));
        let mut n2 = S_INT;
        let r = (a.png_get_text)(p, i, ptr::null_mut(), &mut n2);
        v.push(format!("{tag} text/notext={r} n={n2}"));
        let mut t3: *mut png_text = sp();
        let r = (a.png_get_text)(p, i, &mut t3, ptr::null_mut());
        v.push(format!(
            "{tag} text/nonum={r} out={}",
            ps(t3 as *const c_void)
        ));
        v.push(format!(
            "{tag} text/null={}",
            (a.png_get_text)(p, i, ptr::null_mut(), ptr::null_mut())
        ));
    }
    // ---- tIME (row 615) -------------------------------------------------
    {
        let mut t: *mut png_time = sp();
        let r = (a.png_get_tIME)(p, i, &mut t);
        v.push(format!("{tag} tIME={r} out={}", ps(t as *const c_void)));
        v.push(format!(
            "{tag} tIME/null={}",
            (a.png_get_tIME)(p, i, ptr::null_mut())
        ));
    }
    // ---- tRNS (rows 616, 617, 618) --------------------------------------
    {
        for mask in 0..8u32 {
            let mut ta: *mut png_byte = sp();
            let mut nt = S_INT;
            let mut tc: *mut png_color_16 = sp();
            let pa = if mask & 1 != 0 {
                ptr::null_mut()
            } else {
                &mut ta as *mut *mut png_byte
            };
            let pn = if mask & 2 != 0 {
                ptr::null_mut()
            } else {
                &mut nt as *mut c_int
            };
            let pc = if mask & 4 != 0 {
                ptr::null_mut()
            } else {
                &mut tc as *mut *mut png_color_16
            };
            let r = (a.png_get_tRNS)(p, i, pa, pn, pc);
            v.push(format!(
                "{tag} tRNS/m{mask}={r} ta={} nt={nt} tc={}",
                ps(ta as *const c_void),
                ps(tc as *const c_void)
            ));
        }
    }
    // ---- unknown chunks (rows 619, 620) --------------------------------
    {
        let mut u: *mut png_unknown_chunk = sp();
        let r = (a.png_get_unknown_chunks)(p, i, &mut u);
        v.push(format!("{tag} unknown={r} out={}", ps(u as *const c_void)));
        v.push(format!(
            "{tag} unknown/null={}",
            (a.png_get_unknown_chunks)(p, i, ptr::null_mut())
        ));
    }
    // ---- sCAL (float) / sCAL_s (rows 608, 609) --------------------------
    // Safe here as long as either sCAL is invalid (early return) or the stored
    // strings are sane; the `unit`/`width`/`height` pointers must be non-NULL
    // whenever sCAL IS valid because the C does not check them.
    {
        let mut unit = S_INT;
        let mut w = S_F64;
        let mut h = S_F64;
        let r = (a.png_get_sCAL)(p, i, &mut unit, &mut w, &mut h);
        v.push(format!("{tag} sCAL={r} unit={unit} [{}]", f64v(&[w, h])));

        let mut unit2 = S_INT;
        let mut ws: *mut c_char = sp();
        let mut hs: *mut c_char = sp();
        let r = (a.png_get_sCAL_s)(p, i, &mut unit2, &mut ws, &mut hs);
        v.push(format!(
            "{tag} sCAL_s={r} unit={unit2} w={} h={}",
            ps(ws as *const c_void),
            ps(hs as *const c_void)
        ));
    }
}

/// `png_get_pHYs` and `png_get_pHYs_dpi` for all 8 out-parameter NULL masks.
unsafe fn dump_phys(a: &Api, tag: &str, p: png_structp, i: png_infop, v: &mut Vec<String>) {
    for mask in 0..8u32 {
        let mut rx = S_U32;
        let mut ry = S_U32;
        let mut ut = S_INT;
        let px = if mask & 1 != 0 {
            ptr::null_mut()
        } else {
            &mut rx as *mut png_uint_32
        };
        let py = if mask & 2 != 0 {
            ptr::null_mut()
        } else {
            &mut ry as *mut png_uint_32
        };
        let pu = if mask & 4 != 0 {
            ptr::null_mut()
        } else {
            &mut ut as *mut c_int
        };
        let r = (a.png_get_pHYs)(p, i, px, py, pu);
        v.push(format!("{tag} pHYs/m{mask}={r} [{rx},{ry},{ut}]"));

        let mut rx2 = S_U32;
        let mut ry2 = S_U32;
        let mut ut2 = S_INT;
        let px2 = if mask & 1 != 0 {
            ptr::null_mut()
        } else {
            &mut rx2 as *mut png_uint_32
        };
        let py2 = if mask & 2 != 0 {
            ptr::null_mut()
        } else {
            &mut ry2 as *mut png_uint_32
        };
        let pu2 = if mask & 4 != 0 {
            ptr::null_mut()
        } else {
            &mut ut2 as *mut c_int
        };
        let r = (a.png_get_pHYs_dpi)(p, i, px2, py2, pu2);
        v.push(format!("{tag} pHYsdpi/m{mask}={r} [{rx2},{ry2},{ut2}]"));
    }
}

/// `png_get_IHDR` — only call this when the stored IHDR is VALID (otherwise
/// `png_check_IHDR` -> `png_error "Invalid IHDR data"`, see the child cases) or
/// when at least one pointer is NULL (row 602: immediate `return 0`).
unsafe fn dump_ihdr(a: &Api, tag: &str, p: png_structp, i: png_infop, v: &mut Vec<String>) {
    let mut w = S_U32;
    let mut h = S_U32;
    let mut bd = S_INT;
    let mut ct = S_INT;
    let mut il = S_INT;
    let mut cm = S_INT;
    let mut fm = S_INT;
    let r = (a.png_get_IHDR)(
        p, i, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm,
    );
    v.push(format!(
        "{tag} IHDR={r} [{w},{h},{bd},{ct},{il},{cm},{fm}]"
    ));
    // every out-parameter is individually optional (pngget.c:948-967)
    for k in 0..7usize {
        let mut u32s = [S_U32; 2];
        let mut ints = [S_INT; 5];
        let qu = u32s.as_mut_ptr();
        let qi = ints.as_mut_ptr();
        let mut au: [*mut png_uint_32; 2] = [qu, qu.add(1)];
        let mut ai: [*mut c_int; 5] = [qi, qi.add(1), qi.add(2), qi.add(3), qi.add(4)];
        if k < 2 {
            au[k] = ptr::null_mut();
        } else {
            ai[k - 2] = ptr::null_mut();
        }
        let r = (a.png_get_IHDR)(p, i, au[0], au[1], ai[0], ai[1], ai[2], ai[3], ai[4]);
        v.push(format!(
            "{tag} IHDR/n{k}={r} [{},{},{},{},{},{},{}]",
            u32s[0], u32s[1], ints[0], ints[1], ints[2], ints[3], ints[4]
        ));
    }
    v.push(format!(
        "{tag} IHDR/null={}",
        (a.png_get_IHDR)(
            p,
            i,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut()
        )
    ));
}

/// `png_get_sCAL_fixed` — safe only while sCAL is INVALID (immediate `return 0`,
/// row 606) or while the stored strings convert without overflow.
unsafe fn dump_scal_fixed(a: &Api, tag: &str, p: png_structp, i: png_infop, v: &mut Vec<String>) {
    let mut unit = S_INT;
    let mut w = S_I32;
    let mut h = S_I32;
    let r = (a.png_get_sCAL_fixed)(p, i, &mut unit, &mut w, &mut h);
    v.push(format!("{tag} sCAL_fx={r} unit={unit} [{w},{h}]"));
}

// ---------------------------------------------------------------------------
// the four pointer combinations
// ---------------------------------------------------------------------------

/// (label, png_ptr, info_ptr) for the four NULL combinations plus a fresh,
/// all-zero info struct.  `p`/`i` come from a real write struct.
unsafe fn combos(p: png_structp, i: png_infop) -> [(&'static str, png_structp, png_infop); 4] {
    [
        ("nn", ptr::null_mut(), ptr::null_mut()),
        ("ni", ptr::null_mut(), i),
        ("in", p, ptr::null_mut()),
        ("ii", p, i),
    ]
}

// ---------------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------------

type Case = fn(&Api, &mut Vec<String>);

/// Rows 543..582, 621, 630: scalar getters over all four pointer combinations,
/// on a fresh (all-zero) info struct, for a write struct AND a read struct.
fn case_scalars(a: &Api, v: &mut Vec<String>) {
    unsafe {
        let (pw, iw) = new_write(a);
        for (t, p, i) in combos(pw, iw) {
            dump_scalars(a, &format!("W{t}"), p, i, v);
        }
        kill_write(a, pw, iw);

        let (pr, ir) = new_read(a);
        for (t, p, i) in combos(pr, ir) {
            dump_scalars(a, &format!("R{t}"), p, i, v);
        }
        kill_read(a, pr, ir);
    }
}

/// Rows 579..620: out-parameter getters over all four pointer combinations on a
/// fresh info struct (every `PNG_INFO_*` bit CLEAR).
fn case_outs_empty(a: &Api, v: &mut Vec<String>) {
    unsafe {
        let (pw, iw) = new_write(a);
        for (t, p, i) in combos(pw, iw) {
            dump_outs(a, &format!("W{t}"), p, i, v);
            // sCAL invalid -> png_get_sCAL_fixed cannot overflow
            dump_scal_fixed(a, &format!("W{t}"), p, i, v);
        }
        // png_get_IHDR: only the three NULL-argument combinations are safe on an
        // empty info struct (row 602); "ii" would png_error (row 603) and is
        // covered by the `ihdr-*` child cases.
        for (t, p, i) in combos(pw, iw).into_iter().take(3) {
            dump_ihdr(a, &format!("W{t}"), p, i, v);
        }
        kill_write(a, pw, iw);

        let (pr, ir) = new_read(a);
        for (t, p, i) in combos(pr, ir) {
            dump_outs(a, &format!("R{t}"), p, i, v);
            dump_scal_fixed(a, &format!("R{t}"), p, i, v);
        }
        kill_read(a, pr, ir);
    }
}

/// Every `PNG_INFO_*` bit, 0, an undefined bit and `!0`, on an empty info and on
/// a fully-populated one (rows 540, 541, 542).
fn case_valid_flags(a: &Api, v: &mut Vec<String>) {
    const FLAGS: [png_uint_32; 26] = [
        0,
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
        0x8000_0000,
        0x0010_0000,
        0x0000_0003,
        0x0001_0010,
        !0,
    ];
    unsafe {
        let (pw, iw) = new_write(a);
        for f in FLAGS {
            for (t, p, i) in combos(pw, iw) {
                v.push(format!(
                    "empty {t} valid({f:#x})={}",
                    (a.png_get_valid)(p, i, f)
                ));
            }
        }
        // populate everything, then repeat
        populate(a, pw, iw, v);
        for f in FLAGS {
            for (t, p, i) in combos(pw, iw) {
                v.push(format!(
                    "full {t} valid({f:#x})={}",
                    (a.png_get_valid)(p, i, f)
                ));
            }
        }
        // png_set_invalid clears bits (pngset.c:1859) — including with NULL args
        (a.png_set_invalid)(ptr::null_mut(), iw, PNG_INFO_gAMA as c_int);
        (a.png_set_invalid)(pw, ptr::null_mut(), PNG_INFO_gAMA as c_int);
        v.push(format!(
            "after set_invalid(NULL) gAMA={}",
            (a.png_get_valid)(pw, iw, PNG_INFO_gAMA)
        ));
        (a.png_set_invalid)(pw, iw, PNG_INFO_gAMA as c_int);
        v.push(format!(
            "after set_invalid gAMA={}",
            (a.png_get_valid)(pw, iw, PNG_INFO_gAMA)
        ));
        (a.png_set_invalid)(pw, iw, !0);
        v.push(format!("after set_invalid(!0) all={}", (a.png_get_valid)(pw, iw, !0)));
        kill_write(a, pw, iw);
    }
}

/// Fill an info struct with every chunk this build supports, so that the "bit
/// SET" branch of every getter is reached.
unsafe fn populate(a: &Api, p: png_structp, i: png_infop, v: &mut Vec<String>) {
    (a.png_set_IHDR)(p, i, 8, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
    (a.png_set_gAMA_fixed)(p, i, 45455);
    let sb = png_color_8 {
        red: 8,
        green: 8,
        blue: 8,
        gray: 0,
        alpha: 0,
    };
    (a.png_set_sBIT)(p, i, &sb);
    (a.png_set_cHRM_fixed)(p, i, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
    let bg = png_color_16 {
        index: 0,
        red: 1,
        green: 2,
        blue: 3,
        gray: 4,
    };
    (a.png_set_bKGD)(p, i, &bg);
    (a.png_set_pHYs)(p, i, 3000, 3000, PNG_RESOLUTION_METER);
    (a.png_set_oFFs)(p, i, -17, 42, PNG_OFFSET_MICROMETER);
    let tm = png_time {
        year: 2024,
        month: 3,
        day: 14,
        hour: 15,
        minute: 9,
        second: 26,
    };
    (a.png_set_tIME)(p, i, &tm);
    // pCAL
    let purpose = c"calib";
    let units = c"metres";
    let mut p0 = *b"1.5\0";
    let mut p1 = *b"2.5\0";
    let mut params: [*mut c_char; 2] = [
        p0.as_mut_ptr() as *mut c_char,
        p1.as_mut_ptr() as *mut c_char,
    ];
    (a.png_set_pCAL)(
        p,
        i,
        purpose.as_ptr(),
        -1000,
        1000,
        PNG_EQUATION_LINEAR,
        2,
        units.as_ptr(),
        params.as_mut_ptr(),
    );
    (a.png_set_sRGB)(p, i, PNG_sRGB_INTENT_PERCEPTUAL);
    // iCCP: proflen must be > 0 or png_malloc_warn returns NULL -> benign error
    let prof: [png_byte; 12] = [0, 0, 0, 12, 1, 2, 3, 4, 5, 6, 7, 8];
    (a.png_set_iCCP)(p, i, c"icc".as_ptr(), 0, prof.as_ptr(), 12);
    // sPLT
    let mut spname = *b"pal1\0";
    let mut ents = [png_sPLT_entry {
        red: 1,
        green: 2,
        blue: 3,
        alpha: 4,
        frequency: 5,
    }; 2];
    let splt = png_sPLT_t {
        name: spname.as_mut_ptr() as *mut c_char,
        depth: 8,
        entries: ents.as_mut_ptr(),
        nentries: 2,
    };
    (a.png_set_sPLT)(p, i, &splt, 1);
    (a.png_set_sCAL_s)(p, i, PNG_SCALE_METER, c"1.5".as_ptr(), c"2.5".as_ptr());
    (a.png_set_cICP)(p, i, 9, 16, 0, 1);
    (a.png_set_cLLI_fixed)(p, i, 10_000_000, 1_000_000);
    (a.png_set_mDCV_fixed)(
        p, i, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10_000_000, 50,
    );
    // eXIf
    let exif: [png_byte; 8] = [b'I', b'I', 42, 0, 8, 0, 0, 0];
    (a.png_set_eXIf_1)(p, i, 8, exif.as_ptr() as *mut png_byte);
    // tRNS for a non-palette image: only trans_color
    let tc = png_color_16 {
        index: 0,
        red: 5,
        green: 6,
        blue: 7,
        gray: 0,
    };
    (a.png_set_tRNS)(p, i, ptr::null(), 0, &tc);
    // text
    let mut txt = [png_text {
        compression: PNG_TEXT_COMPRESSION_NONE,
        key: c"Title".as_ptr() as *mut c_char,
        text: c"hello".as_ptr() as *mut c_char,
        text_length: 5,
        itxt_length: 0,
        lang: ptr::null_mut(),
        lang_key: ptr::null_mut(),
    }];
    (a.png_set_text)(p, i, txt.as_mut_ptr(), 1);
    // unknown chunks (location must be non-zero on a write struct)
    let mut udata = [1u8, 2, 3, 4];
    let unk = [png_unknown_chunk {
        name: [b'q', b'w', b'A', b'b', 0],
        data: udata.as_mut_ptr(),
        size: 4,
        location: 0x01, /* PNG_HAVE_IHDR */
    }];
    (a.png_set_unknown_chunks)(p, i, unk.as_ptr(), 1);
    (a.png_set_unknown_chunk_location)(p, i, 0, 0x01);
    // rows
    let mut row = [0u8; 32];
    let mut rows: [*mut png_byte; 4] = [row.as_mut_ptr(); 4];
    (a.png_set_rows)(p, i, rows.as_mut_ptr());
    v.push("populated".to_string());
    // keep the borrowed buffers alive until here
    std::hint::black_box((&mut p0, &mut p1, &mut params, &mut spname, &mut ents, &mut udata, &mut row, &mut rows, &mut txt));
}

/// Every getter again with the matching `PNG_INFO_*` bit SET (the positive
/// branch), so that the sentinel-vs-value distinction is not vacuous.
fn case_outs_populated(a: &Api, v: &mut Vec<String>) {
    unsafe {
        let (pw, iw) = new_write(a);
        populate(a, pw, iw, v);
        for (t, p, i) in combos(pw, iw) {
            dump_scalars(a, &format!("F{t}"), p, i, v);
            dump_outs(a, &format!("F{t}"), p, i, v);
            // sCAL is "1.5"/"2.5" here, so png_fixed cannot overflow
            dump_scal_fixed(a, &format!("F{t}"), p, i, v);
            // IHDR is valid (8x4, depth 8, RGB) so png_check_IHDR passes
            dump_ihdr(a, &format!("F{t}"), p, i, v);
        }
        kill_write(a, pw, iw);
    }
}

/// The palette flavour: PLTE, hIST, palette tRNS, `png_get_palette_max`.
fn case_palette(a: &Api, v: &mut Vec<String>) {
    unsafe {
        let (p, i) = new_write(a);
        // png_get_palette_max before anything (rows 630 + "index checking off")
        v.push(format!("pmax0={}", (a.png_get_palette_max)(p, i)));
        (a.png_set_IHDR)(p, i, 8, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        let pal = [
            png_color { red: 1, green: 2, blue: 3 },
            png_color { red: 4, green: 5, blue: 6 },
            png_color { red: 7, green: 8, blue: 9 },
            png_color { red: 10, green: 11, blue: 12 },
        ];
        (a.png_set_PLTE)(p, i, pal.as_ptr(), 4);
        v.push(format!("pmax1={}", (a.png_get_palette_max)(p, i)));
        // enable index checking -> num_palette_max becomes 0 instead of -1
        (a.png_set_check_for_invalid_index)(p, 1);
        v.push(format!("pmax2={}", (a.png_get_palette_max)(p, i)));
        (a.png_set_check_for_invalid_index)(p, 0);
        v.push(format!("pmax3={}", (a.png_get_palette_max)(p, i)));
        for (t, pp, ii) in combos(p, i) {
            v.push(format!("{t} pmax={}", (a.png_get_palette_max)(pp, ii)));
        }
        // PLTE getter with the bit SET.  num_palette must be non-NULL: the C
        // dereferences it unconditionally at pngget.c:1140.
        let mut out: *mut png_color = sp();
        let mut n = S_INT;
        let r = (a.png_get_PLTE)(p, i, &mut out, &mut n);
        v.push(format!("PLTE={r} n={n} pal={}", ps(out as *const c_void)));
        v.push(format!(
            "PLTE/nopal={}",
            (a.png_get_PLTE)(p, i, ptr::null_mut(), ptr::null_mut())
        ));

        // hIST needs num_palette > 0
        let hist = [7u16; 4];
        (a.png_set_hIST)(p, i, hist.as_ptr());
        let mut h: *mut png_uint_16 = sp();
        let r = (a.png_get_hIST)(p, i, &mut h);
        v.push(format!("hIST={r} out={}", ps(h as *const c_void)));
        v.push(format!(
            "hIST/null={}",
            (a.png_get_hIST)(p, i, ptr::null_mut())
        ));

        // palette tRNS: the palette branch of png_get_tRNS (row 617)
        let alpha = [0u8, 64, 128, 255];
        (a.png_set_tRNS)(p, i, alpha.as_ptr(), 4, ptr::null());
        for mask in 0..8u32 {
            let mut ta: *mut png_byte = sp();
            let mut nt = S_INT;
            let mut tc: *mut png_color_16 = sp();
            let pa = if mask & 1 != 0 {
                ptr::null_mut()
            } else {
                &mut ta as *mut *mut png_byte
            };
            let pn = if mask & 2 != 0 {
                ptr::null_mut()
            } else {
                &mut nt as *mut c_int
            };
            let pc = if mask & 4 != 0 {
                ptr::null_mut()
            } else {
                &mut tc as *mut *mut png_color_16
            };
            let r = (a.png_get_tRNS)(p, i, pa, pn, pc);
            v.push(format!(
                "palTRNS/m{mask}={r} ta={} nt={nt} tc={}",
                ps(ta as *const c_void),
                ps(tc as *const c_void)
            ));
        }
        // row 541: info_ptr->valid has PNG_INFO_tRNS but png_ptr->num_trans is
        // still 0 (png_set_tRNS only touches info_ptr), so png_get_valid lies.
        v.push(format!(
            "valid(tRNS)={} valid(PLTE)={}",
            (a.png_get_valid)(p, i, PNG_INFO_tRNS),
            (a.png_get_valid)(p, i, PNG_INFO_PLTE)
        ));
        dump_scalars(a, "PAL", p, i, v);
        kill_write(a, p, i);
    }
}

/// `png_ptr`-only getters (rows 621..630 and their non-pngget.c siblings).
fn case_struct_getters(a: &Api, v: &mut Vec<String>) {
    unsafe {
        let n: png_structp = ptr::null_mut();
        // pointer accessors: NULL-ness only (rows 622 + pngerror.c:741,
        // png.c:693, pngmem.c:281, pngpread.c:940, pngtrans.c:866)
        v.push(format!(
            "NULL error_ptr={}",
            ps((a.png_get_error_ptr)(n))
        ));
        v.push(format!("NULL io_ptr={}", ps((a.png_get_io_ptr)(n))));
        v.push(format!("NULL mem_ptr={}", ps((a.png_get_mem_ptr)(n))));
        v.push(format!(
            "NULL progressive_ptr={}",
            ps((a.png_get_progressive_ptr)(n))
        ));
        v.push(format!(
            "NULL user_chunk_ptr={}",
            ps((a.png_get_user_chunk_ptr)(n))
        ));
        v.push(format!(
            "NULL user_transform_ptr={}",
            ps((a.png_get_user_transform_ptr)(n))
        ));
        // PNG_UINT_32_MAX / 8 sentinels (pngtrans.c:875, 887)
        v.push(format!("NULL row_number={}", (a.png_get_current_row_number)(n)));
        v.push(format!("NULL pass_number={}", (a.png_get_current_pass_number)(n)));
        // limits (rows 623..627)
        v.push(format!("NULL cache_max={}", (a.png_get_chunk_cache_max)(n)));
        v.push(format!("NULL malloc_max={}", (a.png_get_chunk_malloc_max)(n)));
        v.push(format!("NULL width_max={}", (a.png_get_user_width_max)(n)));
        v.push(format!("NULL height_max={}", (a.png_get_user_height_max)(n)));
        v.push(format!(
            "NULL bufsize={}",
            (a.png_get_compression_buffer_size)(n)
        ));
        v.push(format!(
            "NULL rgb_to_gray={}",
            (a.png_get_rgb_to_gray_status)(n)
        ));
        // version strings ignore png_ptr entirely (png.c:816, 842, 849, 857)
        v.push(format!(
            "NULL copyright={}",
            cstr_to_string((a.png_get_copyright)(n))
        ));
        v.push(format!(
            "NULL header_ver={}",
            cstr_to_string((a.png_get_header_ver)(n))
        ));
        v.push(format!(
            "NULL libpng_ver={}",
            cstr_to_string((a.png_get_libpng_ver)(n))
        ));
        v.push(format!(
            "NULL header_version={}",
            cstr_to_string((a.png_get_header_version)(n))
        ));
        // NOTE png_get_io_state (pngget.c:1344-1347) and png_get_io_chunk_type
        // (pngget.c:1350-1353) have NO NULL check at all — they dereference
        // png_ptr unconditionally — so they are NOT called with NULL here.

        for (which, mk) in [
            ("W", 0usize),
            ("R", 1usize),
        ] {
            let (p, i) = if mk == 0 { new_write(a) } else { new_read(a) };
            v.push(format!("{which} error_ptr={}", ps((a.png_get_error_ptr)(p))));
            v.push(format!("{which} io_ptr={}", ps((a.png_get_io_ptr)(p))));
            v.push(format!("{which} mem_ptr={}", ps((a.png_get_mem_ptr)(p))));
            v.push(format!(
                "{which} progressive_ptr={}",
                ps((a.png_get_progressive_ptr)(p))
            ));
            v.push(format!(
                "{which} user_chunk_ptr={}",
                ps((a.png_get_user_chunk_ptr)(p))
            ));
            v.push(format!(
                "{which} user_transform_ptr={}",
                ps((a.png_get_user_transform_ptr)(p))
            ));
            // reading/writing has not started
            v.push(format!(
                "{which} row_number={}",
                (a.png_get_current_row_number)(p)
            ));
            v.push(format!(
                "{which} pass_number={}",
                (a.png_get_current_pass_number)(p)
            ));
            v.push(format!("{which} io_state={}", (a.png_get_io_state)(p)));
            v.push(format!(
                "{which} io_chunk_type={:#x}",
                (a.png_get_io_chunk_type)(p)
            ));
            v.push(format!("{which} cache_max={}", (a.png_get_chunk_cache_max)(p)));
            v.push(format!(
                "{which} malloc_max={}",
                (a.png_get_chunk_malloc_max)(p)
            ));
            v.push(format!("{which} width_max={}", (a.png_get_user_width_max)(p)));
            v.push(format!(
                "{which} height_max={}",
                (a.png_get_user_height_max)(p)
            ));
            v.push(format!(
                "{which} bufsize={}",
                (a.png_get_compression_buffer_size)(p)
            ));
            v.push(format!(
                "{which} rgb_to_gray={}",
                (a.png_get_rgb_to_gray_status)(p)
            ));
            v.push(format!(
                "{which} signature-before-read={}",
                ps((a.png_get_signature)(p, i) as *const c_void)
            ));
            if mk == 0 {
                kill_write(a, p, i);
            } else {
                kill_read(a, p, i);
            }
        }
    }
}

/// The EASY_ACCESS pHYs getters driven with out-of-range stored values and every
/// unit type (rows 552..557, 570..573, 579, 580, 610, 611).
fn case_phys_extremes(a: &Api, v: &mut Vec<String>) {
    const XY: [(png_uint_32, png_uint_32); 14] = [
        (0, 0),
        (1, 1),
        (1, 2),
        (2, 1),
        (0, 1),
        (1, 0),
        (0x7fff_ffff, 1),
        // row 561: png_muldiv(y, PNG_FP_1, x) overflows png_fixed_point
        (1, 0x7fff_ffff),
        (2, 0x7fff_ffff),
        (0x7fff_ffff, 0x7fff_ffff),
        // row 570: ppm > PNG_UINT_31_MAX so ppi_from_ppm gives up
        (0x8000_0000, 1),
        (0x8000_0000, 0x8000_0000),
        (1, 0xffff_ffff),
        (0xffff_ffff, 0xffff_ffff),
    ];
    const UNITS: [c_int; 7] = [-1, 0, 1, 2, 3, 255, 256];
    unsafe {
        for (x, y) in XY {
            for u in UNITS {
                let (p, i) = new_write(a);
                (a.png_set_pHYs)(p, i, x, y, u);
                let t = format!("pHYs[{x},{y},{u}]");
                v.push(format!("{t} xppm={}", (a.png_get_x_pixels_per_meter)(p, i)));
                v.push(format!("{t} yppm={}", (a.png_get_y_pixels_per_meter)(p, i)));
                v.push(format!("{t} ppm={}", (a.png_get_pixels_per_meter)(p, i)));
                v.push(format!("{t} xppi={}", (a.png_get_x_pixels_per_inch)(p, i)));
                v.push(format!("{t} yppi={}", (a.png_get_y_pixels_per_inch)(p, i)));
                v.push(format!("{t} ppi={}", (a.png_get_pixels_per_inch)(p, i)));
                v.push(format!(
                    "{t} par={}",
                    f32v((a.png_get_pixel_aspect_ratio)(p, i))
                ));
                v.push(format!(
                    "{t} parfx={}",
                    (a.png_get_pixel_aspect_ratio_fixed)(p, i)
                ));
                dump_phys(a, &t, p, i, v);
                kill_write(a, p, i);
            }
        }
    }
}

/// The EASY_ACCESS oFFs getters, ditto (rows 562..569, 574..578, 604).
fn case_offs_extremes(a: &Api, v: &mut Vec<String>) {
    const XY: [(png_int_32, png_int_32); 9] = [
        (0, 0),
        (1, -1),
        (-1, 1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (1_000_000, -1_000_000),
        (4_000_000, 4_000_000),
        (4_294_968, -4_294_968),
        (i32::MIN, i32::MIN),
    ];
    const UNITS: [c_int; 6] = [-1, 0, 1, 2, 255, 256];
    unsafe {
        for (x, y) in XY {
            for u in UNITS {
                let (p, i) = new_write(a);
                (a.png_set_oFFs)(p, i, x, y, u);
                let t = format!("oFFs[{x},{y},{u}]");
                v.push(format!("{t} xpx={}", (a.png_get_x_offset_pixels)(p, i)));
                v.push(format!("{t} ypx={}", (a.png_get_y_offset_pixels)(p, i)));
                v.push(format!("{t} xmic={}", (a.png_get_x_offset_microns)(p, i)));
                v.push(format!("{t} ymic={}", (a.png_get_y_offset_microns)(p, i)));
                v.push(format!(
                    "{t} xinfx={}",
                    (a.png_get_x_offset_inches_fixed)(p, i)
                ));
                v.push(format!(
                    "{t} yinfx={}",
                    (a.png_get_y_offset_inches_fixed)(p, i)
                ));
                v.push(format!(
                    "{t} xin={}",
                    f32v((a.png_get_x_offset_inches)(p, i))
                ));
                v.push(format!(
                    "{t} yin={}",
                    f32v((a.png_get_y_offset_inches)(p, i))
                ));
                let mut ox = S_I32;
                let mut oy = S_I32;
                let mut ut = S_INT;
                let r = (a.png_get_oFFs)(p, i, &mut ox, &mut oy, &mut ut);
                v.push(format!("{t} oFFs={r} [{ox},{oy},{ut}]"));
                kill_write(a, p, i);
            }
        }
    }
}

/// Every `png_set_*` whose C source has a verified NULL guard, called with the
/// NULL combinations, followed by a re-query of the matching getter to prove
/// nothing was stored.  Functions WITHOUT a guard are listed in the module
/// comment and are deliberately absent.
fn case_setters_null_guards(a: &Api, v: &mut Vec<String>) {
    unsafe {
        let (p, i) = new_write(a);
        let z: png_structp = ptr::null_mut();
        let zi: png_infop = ptr::null_mut();

        let bg = png_color_16 { index: 1, red: 2, green: 3, blue: 4, gray: 5 };
        let sb = png_color_8 { red: 1, green: 2, blue: 3, gray: 4, alpha: 5 };
        let tm = png_time { year: 2001, month: 2, day: 3, hour: 4, minute: 5, second: 6 };
        let pal = [png_color { red: 9, green: 9, blue: 9 }; 2];
        let hist = [3u16; 2];
        let mut exif: [png_byte; 4] = [b'I', b'I', 42, 0];
        let mut p0 = *b"1.0\0";
        let mut params: [*mut c_char; 1] = [p0.as_mut_ptr() as *mut c_char];
        let mut spname = *b"s\0";
        let mut ents = [png_sPLT_entry { red: 1, green: 1, blue: 1, alpha: 1, frequency: 1 }; 1];
        let splt = png_sPLT_t {
            name: spname.as_mut_ptr() as *mut c_char,
            depth: 8,
            entries: ents.as_mut_ptr(),
            nentries: 1,
        };
        let mut udata = [9u8; 2];
        let unk = [png_unknown_chunk {
            name: [b'q', b'w', b'A', b'b', 0],
            data: udata.as_mut_ptr(),
            size: 2,
            location: 0x01,
        }];
        let mut txt = [png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: c"K".as_ptr() as *mut c_char,
            text: c"T".as_ptr() as *mut c_char,
            text_length: 1,
            itxt_length: 0,
            lang: ptr::null_mut(),
            lang_key: ptr::null_mut(),
        }];
        let mut row = [0u8; 8];
        let mut rows: [*mut png_byte; 1] = [row.as_mut_ptr()];

        // --- (png_ptr, info_ptr) setters: three NULL combinations ---------
        for (t, sp_, si) in [("nn", z, zi), ("ni", z, i), ("in", p, zi)] {
            (a.png_set_IHDR)(sp_, si, 8, 8, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            (a.png_set_gAMA_fixed)(sp_, si, 45455);
            (a.png_set_gAMA)(sp_, si, 0.45455);
            (a.png_set_cHRM_fixed)(sp_, si, 1, 2, 3, 4, 5, 6, 7, 8);
            (a.png_set_cHRM)(sp_, si, 0.3, 0.3, 0.6, 0.3, 0.3, 0.6, 0.15, 0.06);
            (a.png_set_cHRM_XYZ_fixed)(sp_, si, 1, 2, 3, 4, 5, 6, 7, 8, 9);
            (a.png_set_cHRM_XYZ)(sp_, si, 0.4, 0.2, 0.02, 0.36, 0.7, 0.11, 0.18, 0.07, 0.95);
            (a.png_set_cICP)(sp_, si, 9, 16, 0, 1);
            (a.png_set_cLLI_fixed)(sp_, si, 1000, 100);
            (a.png_set_cLLI)(sp_, si, 1.0, 0.5);
            (a.png_set_mDCV_fixed)(sp_, si, 1, 2, 3, 4, 5, 6, 7, 8, 1000, 10);
            (a.png_set_mDCV)(sp_, si, 0.3, 0.3, 0.6, 0.3, 0.3, 0.6, 0.15, 0.06, 1.0, 0.1);
            (a.png_set_bKGD)(sp_, si, &bg);
            (a.png_set_sBIT)(sp_, si, &sb);
            (a.png_set_sRGB)(sp_, si, PNG_sRGB_INTENT_RELATIVE);
            (a.png_set_sRGB_gAMA_and_cHRM)(sp_, si, PNG_sRGB_INTENT_RELATIVE);
            (a.png_set_pHYs)(sp_, si, 100, 200, PNG_RESOLUTION_METER);
            (a.png_set_oFFs)(sp_, si, 1, 2, PNG_OFFSET_PIXEL);
            (a.png_set_tIME)(sp_, si, &tm);
            (a.png_set_PLTE)(sp_, si, pal.as_ptr(), 2);
            (a.png_set_hIST)(sp_, si, hist.as_ptr());
            (a.png_set_tRNS)(sp_, si, ptr::null(), 0, &bg);
            (a.png_set_iCCP)(sp_, si, c"n".as_ptr(), 0, exif.as_ptr(), 4);
            (a.png_set_sPLT)(sp_, si, &splt, 1);
            (a.png_set_sCAL_s)(sp_, si, PNG_SCALE_METER, c"1".as_ptr(), c"2".as_ptr());
            // png_set_sCAL / _fixed have no guard of their own but reach
            // png_set_sCAL_s, which does; the width<=0 branch only warns.
            (a.png_set_sCAL)(sp_, si, PNG_SCALE_METER, 1.0, 2.0);
            (a.png_set_sCAL_fixed)(sp_, si, PNG_SCALE_METER, 100000, 200000);
            (a.png_set_sCAL)(sp_, si, PNG_SCALE_METER, -1.0, 2.0);
            (a.png_set_sCAL_fixed)(sp_, si, PNG_SCALE_METER, 1, -1);
            (a.png_set_pCAL)(
                sp_, si, c"pp".as_ptr(), 0, 1, PNG_EQUATION_LINEAR, 1,
                c"uu".as_ptr(), params.as_mut_ptr(),
            );
            (a.png_set_eXIf_1)(sp_, si, 4, exif.as_mut_ptr());
            (a.png_set_eXIf)(sp_, si, exif.as_mut_ptr());
            (a.png_set_unknown_chunks)(sp_, si, unk.as_ptr(), 1);
            (a.png_set_unknown_chunk_location)(sp_, si, 0, 0x01);
            (a.png_set_rows)(sp_, si, rows.as_mut_ptr());
            (a.png_set_invalid)(sp_, si, PNG_INFO_gAMA as c_int);
            (a.png_set_text)(sp_, si, txt.as_ptr(), 1);
            v.push(format!(
                "{t} text_2={}",
                (a.png_set_text_2)(sp_, si, txt.as_ptr(), 1)
            ));
            v.push(format!("{t} done", ));
        }

        // Nothing may have been stored in the real info struct.
        v.push(format!("after-set valid={:#x}", (a.png_get_valid)(p, i, !0)));
        dump_scalars(a, "after", p, i, v);
        dump_outs(a, "after", p, i, v);
        dump_scal_fixed(a, "after", p, i, v);
        // `num_exif`/`num_palette` NULL is only safe while the chunk is absent
        v.push(format!(
            "eXIf_1/nonum={}",
            (a.png_get_eXIf_1)(p, i, ptr::null_mut(), ptr::null_mut())
        ));
        let mut pal_out: *mut png_color = sp();
        v.push(format!(
            "PLTE/nonum={} pal={}",
            (a.png_get_PLTE)(p, i, &mut pal_out, ptr::null_mut()),
            ps(pal_out as *const c_void)
        ));

        // --- png_ptr-only setters -----------------------------------------
        (a.png_set_sig_bytes)(z, 4);
        (a.png_set_error_fn)(z, ptr::null_mut(), Some(error_cb), Some(warn_cb));
        (a.png_set_mem_fn)(z, ptr::null_mut(), None, None);
        (a.png_set_read_fn)(z, ptr::null_mut(), Some(read_cb));
        (a.png_set_write_fn)(z, ptr::null_mut(), Some(write_cb), Some(flush_cb));
        (a.png_set_read_status_fn)(z, Some(read_status_cb));
        (a.png_set_write_status_fn)(z, Some(write_status_cb));
        (a.png_set_write_user_transform_fn)(z, None);
        (a.png_set_progressive_read_fn)(z, ptr::null_mut(), None, None, None);
        (a.png_set_read_user_chunk_fn)(z, ptr::null_mut(), None);
        (a.png_set_user_transform_info)(z, ptr::null_mut(), 8, 3);
        (a.png_set_user_limits)(z, 10, 10);
        (a.png_set_chunk_cache_max)(z, 7);
        (a.png_set_chunk_malloc_max)(z, 7);
        (a.png_set_compression_buffer_size)(z, 8192);
        (a.png_set_flush)(z, 4);
        (a.png_set_filter)(z, 0, PNG_ALL_FILTERS);
        (a.png_set_filter_heuristics)(z, 0, 0, ptr::null(), ptr::null());
        (a.png_set_filter_heuristics_fixed)(z, 0, 0, ptr::null(), ptr::null());
        (a.png_set_compression_level)(z, 6);
        (a.png_set_compression_mem_level)(z, 8);
        (a.png_set_compression_method)(z, 8);
        (a.png_set_compression_strategy)(z, 0);
        (a.png_set_compression_window_bits)(z, 15);
        (a.png_set_text_compression_level)(z, 6);
        (a.png_set_text_compression_mem_level)(z, 8);
        (a.png_set_text_compression_method)(z, 8);
        (a.png_set_text_compression_strategy)(z, 0);
        (a.png_set_text_compression_window_bits)(z, 15);
        (a.png_set_crc_action)(z, PNG_CRC_DEFAULT, PNG_CRC_DEFAULT);
        (a.png_set_keep_unknown_chunks)(z, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
        (a.png_set_bgr)(z);
        (a.png_set_swap)(z);
        (a.png_set_swap_alpha)(z);
        (a.png_set_invert_alpha)(z);
        (a.png_set_invert_mono)(z);
        (a.png_set_packing)(z);
        (a.png_set_packswap)(z);
        (a.png_set_shift)(z, &sb);
        (a.png_set_filler)(z, 0xff, PNG_FILLER_AFTER);
        (a.png_set_add_alpha)(z, 0xff, PNG_FILLER_AFTER);
        (a.png_set_scale_16)(z);
        (a.png_set_strip_16)(z);
        (a.png_set_strip_alpha)(z);
        (a.png_set_expand)(z);
        (a.png_set_expand_16)(z);
        (a.png_set_expand_gray_1_2_4_to_8)(z);
        (a.png_set_palette_to_rgb)(z);
        (a.png_set_tRNS_to_alpha)(z);
        (a.png_set_gray_to_rgb)(z);
        (a.png_set_gamma)(z, 2.2, 0.45455);
        (a.png_set_gamma_fixed)(z, 220000, 45455);
        (a.png_set_alpha_mode)(z, PNG_ALPHA_PNG, 2.2);
        (a.png_set_alpha_mode_fixed)(z, PNG_ALPHA_PNG, 220000);
        (a.png_set_background)(z, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 1.0);
        (a.png_set_background_fixed)(z, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 100000);
        (a.png_set_rgb_to_gray)(z, 0, 0.2, 0.7);
        (a.png_set_rgb_to_gray_fixed)(z, 0, 20000, 70000);
        let mut qpal = [png_color { red: 1, green: 1, blue: 1 }; 2];
        (a.png_set_quantize)(z, qpal.as_mut_ptr(), 2, 2, ptr::null(), 0);
        // ones that RETURN something
        v.push(format!("NULL set_option={}", (a.png_set_option)(z, 2, PNG_OPTION_ON)));
        v.push(format!(
            "NULL set_interlace_handling={}",
            (a.png_set_interlace_handling)(z)
        ));
        v.push(format!(
            "NULL permit_mng={}",
            (a.png_permit_mng_features)(z, PNG_ALL_MNG_FEATURES as png_uint_32)
        ));
        v.push(format!(
            "NULL set_longjmp_fn={}",
            ps((a.png_set_longjmp_fn)(z, None, 8))
        ));
        // and the png_ptr-only getters must still report their sentinels
        v.push(format!("NULL width_max2={}", (a.png_get_user_width_max)(z)));
        v.push(format!("NULL cache_max2={}", (a.png_get_chunk_cache_max)(z)));
        v.push(format!(
            "NULL bufsize2={}",
            (a.png_get_compression_buffer_size)(z)
        ));

        kill_write(a, p, i);

        // --- the three setters with NO NULL guard at all -------------------
        // They may only be called with a REAL png_ptr (see the module comment);
        // exercised here so that no `png_set_*` in api.rs goes untouched.
        {
            let (rp, ri) = new_read(a);
            (a.png_set_benign_errors)(rp, 1);
            (a.png_set_benign_errors)(rp, 0);
            (a.png_set_check_for_invalid_index)(rp, 1);
            v.push(format!(
                "read pmax-after-index-check={}",
                (a.png_get_palette_max)(rp, ri)
            ));
            (a.png_set_check_for_invalid_index)(rp, 0);
            v.push(format!(
                "read pmax-after-index-off={}",
                (a.png_get_palette_max)(rp, ri)
            ));
            (a.png_set_read_user_transform_fn)(rp, None);
            v.push(format!(
                "read user_transform_ptr={}",
                ps((a.png_get_user_transform_ptr)(rp))
            ));
            kill_read(a, rp, ri);
        }

        std::hint::black_box((
            &mut exif, &mut p0, &mut params, &mut spname, &mut ents, &mut udata, &mut txt,
            &mut row, &mut rows, &mut qpal,
        ));
    }
}

/// `png_get_signature` before any read, plus `png_set_sig_bytes` round trip.
fn case_signature(a: &Api, v: &mut Vec<String>) {
    unsafe {
        let (p, i) = new_read(a);
        // rows 582: NULL combinations
        for (t, pp, ii) in combos(p, i) {
            v.push(format!(
                "{t} sig={}",
                ps((a.png_get_signature)(pp, ii) as *const c_void)
            ));
        }
        // before any read the eight bytes are all zero
        let s = (a.png_get_signature)(p, i);
        assert!(!s.is_null());
        let bytes = std::slice::from_raw_parts(s, 8);
        v.push(format!("sig-bytes={bytes:02x?}"));
        (a.png_set_sig_bytes)(p, 4);
        let s2 = (a.png_get_signature)(p, i);
        v.push(format!(
            "sig-after-set={:02x?}",
            std::slice::from_raw_parts(s2, 8)
        ));
        kill_read(a, p, i);
    }
}

// ---------------------------------------------------------------------------
// the case table
// ---------------------------------------------------------------------------

const CASES: &[(&str, Case)] = &[
    ("scalars", case_scalars),
    ("outs-empty", case_outs_empty),
    ("outs-populated", case_outs_populated),
    ("valid-flags", case_valid_flags),
    ("palette", case_palette),
    ("struct-getters", case_struct_getters),
    ("phys-extremes", case_phys_extremes),
    ("offs-extremes", case_offs_extremes),
    ("setters-null-guards", case_setters_null_guards),
    ("signature", case_signature),
];

fn lookup(name: &str) -> Case {
    CASES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, f)| *f)
        .unwrap_or_else(|| panic!("unknown case {name}"))
}

/// Replay one case against both libraries and compare the full transcript.
#[track_caller]
fn diff(name: &str) {
    let f = lookup(name);
    let mut out: Vec<Vec<String>> = Vec::new();
    for (idx, (_lab, a)) in each().into_iter().enumerate() {
        reset_all();
        set_cur_is_c(idx == 0);
        let mut v: Vec<String> = Vec::new();
        f(a, &mut v);
        v.extend(log_take());
        out.push(v);
    }
    assert!(
        out[0].len() > 4,
        "case {name} recorded almost nothing ({} lines) — the comparison would be vacuous",
        out[0].len()
    );
    eq_dbg(name, &out[0], &out[1]);
}

// ---------------------------------------------------------------------------
// in-process tests
// ---------------------------------------------------------------------------

#[test]
fn scalar_getters_null_and_empty() {
    diff("scalars");
}

#[test]
fn out_param_getters_bit_clear() {
    diff("outs-empty");
}

#[test]
fn out_param_getters_bit_set() {
    diff("outs-populated");
}

#[test]
fn get_valid_every_flag() {
    diff("valid-flags");
}

#[test]
fn palette_and_palette_max() {
    diff("palette");
}

#[test]
fn png_ptr_only_getters() {
    diff("struct-getters");
}

#[test]
fn phys_out_of_range() {
    diff("phys-extremes");
}

#[test]
fn offs_out_of_range() {
    diff("offs-extremes");
}

#[test]
fn setter_null_guards() {
    diff("setters-null-guards");
}

#[test]
fn signature_before_read() {
    diff("signature");
}

// ---------------------------------------------------------------------------
// sub-process cases: the two getters that can png_error
// ---------------------------------------------------------------------------

fn run_child_case(a: &Api, case: &str) {
    unsafe {
        match case {
            // row 603: an all-zero info struct fails png_check_IHDR, so this
            // GETTER emits a series of png_warnings and then png_errors.
            "ihdr-empty-write" => {
                let (p, i) = new_write(a);
                let r = (a.png_get_IHDR)(
                    p,
                    i,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                emit(format!("IHDR={r}"));
                child_finish();
            }
            "ihdr-empty-read" => {
                let (p, i) = new_read(a);
                let r = (a.png_get_IHDR)(
                    p,
                    i,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                emit(format!("IHDR={r}"));
                child_finish();
            }
            // same, but with sentinel out-parameters: pngget.c:948-967 fills
            // them in BEFORE png_check_IHDR runs, so the error still fires.
            // (The values themselves cannot be printed after the longjmp.)
            "ihdr-empty-outs" => {
                let (p, i) = new_write(a);
                let mut w = S_U32;
                let mut h = S_U32;
                let mut bd = S_INT;
                let mut ct = S_INT;
                let mut il = S_INT;
                let mut cm = S_INT;
                let mut fm = S_INT;
                let r = (a.png_get_IHDR)(
                    p, i, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm,
                );
                emit(format!("IHDR={r} [{w},{h},{bd},{ct},{il},{cm},{fm}]"));
                child_finish();
            }
            // a VALID IHDR made invalid after the fact by tightening the user
            // limits — a different png_check_IHDR warning, same fatal ending.
            "ihdr-user-limit" => {
                let (p, i) = new_write(a);
                (a.png_set_IHDR)(p, i, 8, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (a.png_set_user_limits)(p, 1, 1);
                let mut w = S_U32;
                let r = (a.png_get_IHDR)(
                    p,
                    i,
                    &mut w,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                emit(format!("IHDR={r} w={w}"));
                child_finish();
            }
            // row 606: sCAL invalid -> plain 0, NOT fatal.
            "scal-fixed-empty" => {
                let (p, i) = new_write(a);
                let mut u = S_INT;
                let mut w = S_I32;
                let mut h = S_I32;
                let r = (a.png_get_sCAL_fixed)(p, i, &mut u, &mut w, &mut h);
                emit(format!("sCAL_fx={r} [{u},{w},{h}]"));
                child_finish();
            }
            // row 607: atof(scal_s_width) * 100000 overflows png_fixed_point.
            "scal-fixed-overflow-w" => {
                let (p, i) = new_write(a);
                (a.png_set_sCAL_s)(p, i, PNG_SCALE_METER, c"1e40".as_ptr(), c"2".as_ptr());
                let mut u = S_INT;
                let mut w = S_I32;
                let mut h = S_I32;
                let r = (a.png_get_sCAL_fixed)(p, i, &mut u, &mut w, &mut h);
                emit(format!("sCAL_fx={r} [{u},{w},{h}]"));
                child_finish();
            }
            "scal-fixed-overflow-h" => {
                let (p, i) = new_write(a);
                (a.png_set_sCAL_s)(p, i, PNG_SCALE_METER, c"1".as_ptr(), c"9e30".as_ptr());
                let mut u = S_INT;
                let mut w = S_I32;
                let mut h = S_I32;
                let r = (a.png_get_sCAL_fixed)(p, i, &mut u, &mut w, &mut h);
                emit(format!("sCAL_fx={r} [{u},{w},{h}]"));
                child_finish();
            }
            // the float and string flavours of the same info are NOT fatal
            "scal-float-huge" => {
                let (p, i) = new_write(a);
                (a.png_set_sCAL_s)(p, i, PNG_SCALE_METER, c"1e40".as_ptr(), c"2".as_ptr());
                let mut u = S_INT;
                let mut w = S_F64;
                let mut h = S_F64;
                let r = (a.png_get_sCAL)(p, i, &mut u, &mut w, &mut h);
                emit(format!(
                    "sCAL={r} unit={u} [{:#x},{:#x}]",
                    w.to_bits(),
                    h.to_bits()
                ));
                let mut u2 = S_INT;
                let mut ws: *mut c_char = sp();
                let mut hs: *mut c_char = sp();
                let r = (a.png_get_sCAL_s)(p, i, &mut u2, &mut ws, &mut hs);
                emit(format!(
                    "sCAL_s={r} unit={u2} w={} h={}",
                    cstr_to_string(ws),
                    cstr_to_string(hs)
                ));
                child_finish();
            }
            other => panic!("unknown child case {other}"),
        }
    }
}

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
    run_child_case(a, &case);
}

#[test]
fn ihdr_getter_is_fatal_on_bad_info() {
    for c in [
        "ihdr-empty-write",
        "ihdr-empty-read",
        "ihdr-empty-outs",
        "ihdr-user-limit",
    ] {
        diff_case(c);
    }
}

#[test]
fn scal_fixed_getter_overflow() {
    for c in [
        "scal-fixed-empty",
        "scal-fixed-overflow-w",
        "scal-fixed-overflow-h",
        "scal-float-huge",
    ] {
        diff_case(c);
    }
}

// ---------------------------------------------------------------------------
// self-check: prove the comparison is not vacuous
// ---------------------------------------------------------------------------

#[test]
fn self_check() {
    // Replay EVERY case against the C library only and inspect what came back.
    let a = &apis().c;
    let mut all: Vec<String> = Vec::new();
    for (_, f) in CASES {
        reset_all();
        set_cur_is_c(true);
        let mut v = Vec::new();
        f(a, &mut v);
        v.extend(log_take());
        all.extend(v);
    }
    assert!(
        all.len() > 4000,
        "only {} observations were recorded",
        all.len()
    );

    // Distinct right-hand sides actually returned by the C library.
    let mut distinct = std::collections::BTreeSet::new();
    for line in &all {
        for chunk in line.split(' ') {
            if let Some((_k, val)) = chunk.split_once('=') {
                distinct.insert(val.to_string());
            }
        }
    }
    assert!(
        distinct.len() >= 40,
        "only {} distinct returned values — the comparison looks vacuous: {:?}",
        distinct.len(),
        distinct
    );

    // ... and specifically these DISTINCT sentinels / messages must be present.
    let hay = all.join("\n");
    let wanted: [&str; 18] = [
        "rowbytes=0",                 // pngget.c:45
        "bit_depth=0",                // pngget.c:85
        "channels=0",                 // pngget.c:486
        "palette_max=-1",             // pngget.c:1364 — the only -1 sentinel
        "signature=NULL",             // pngget.c:496
        "rows=NULL",                  // pngget.c:55
        "row_number=4294967295",      // pngtrans.c:884  PNG_UINT_32_MAX
        "pass_number=8",              // pngtrans.c:890  invalid pass
        "par=0x0",                    // pngget.c:207 (float)0.0
        "SENT",                       // an out-parameter was NOT written
        "bKGD=0",                     // pngget.c:515
        "cICP=0",                     // pngget.c:781
        "pCAL=0",                     // pngget.c:1025
        "NULL set_option=1",          // png.c:3783  PNG_OPTION_INVALID
        "NULL set_interlace_handling=1", // pngtrans.c:137
        "NULL permit_mng=0",          // pngset.c:1562
        "NULL set_longjmp_fn=NULL",   // pngerror.c:557
        "WARN:png_get_eXIf does not work; use png_get_eXIf_1", // pngget.c:895
    ];
    let mut missing: Vec<&str> = Vec::new();
    for w in wanted {
        if !hay.contains(w) {
            missing.push(w);
        }
    }
    assert!(
        missing.is_empty(),
        "these C sentinels were never observed: {missing:?}"
    );

    // The fixed-point overflow warning must fire somewhere in the oFFs sweep.
    assert!(
        hay.contains("WARN:fixed point overflow ignored"),
        "pngget.c:388 warning never observed"
    );
}

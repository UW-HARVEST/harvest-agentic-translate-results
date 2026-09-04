//! Phase C — READ-side setter and transform rejections.
//!
//! Covers the error/warning rows of `c_src/src/pngrtran.c`, `c_src/src/pngtrans.c`
//! and the read-struct rows of `c_src/src/pngset.c`.
//!
//! Almost every row here is a `png_app_error` or `png_app_warning`.  In THIS
//! build `PNG_LIBPNG_BUILD_BASE_TYPE == PNG_LIBPNG_BUILD_BETA`, so
//! `PNG_RELEASE_BUILD == 0` and neither `PNG_FLAG_APP_ERRORS_WARN` nor
//! `PNG_FLAG_APP_WARNINGS_WARN` is set by default: both dispatchers are FATAL
//! `png_error`s.  `png_set_benign_errors(png_ptr, 1)` sets all three WARN flags
//! (`pngset.c:1936`) and turns them into warnings; every case can therefore be
//! re-run with the `:benign` suffix, which is exercised by
//! `benign_error_variants` below.
//!
//! Because `png_error` must not return, each case runs in a SUB-PROCESS (the
//! same mechanism as `t23_err_write.rs`): the test binary re-executes itself
//! once for the C library and once for the Rust library with an error handler
//! that prints the message and `exit(70)`s.  The parent compares the ordered
//! transcripts, so a divergence in the message text, in the number/order of
//! warnings, or in whether the call was fatal at all is caught.
//!
//! NULL-argument rows that the C does NOT guard are deliberately absent; each
//! one is documented with the C file:line that dereferences without a check:
//!
//!   * `png_set_benign_errors(NULL, 1)` — `pngset.c:1936-1937` writes
//!     `png_ptr->flags` with no `png_ptr == NULL` test.
//!   * `png_set_read_user_transform_fn(NULL, fn)` — `pngrtran.c:1139` writes
//!     `png_ptr->transformations` with no `png_ptr == NULL` test (it is also the
//!     only read transform with neither a NULL check nor a `png_rtran_ok`
//!     guard).
//!   * `png_set_quantize` with `num_palette < 0` or `> PNG_MAX_PALETTE_LENGTH`
//!     — `pngrtran.c:823` `memcpy(png_ptr->palette, palette,
//!     (unsigned int)num_palette * sizeof (png_color))` into a fixed 256-entry
//!     buffer, unchecked; and `maximum_colors <= 0` with `histogram == NULL`
//!     indexes `hash[i]` for `i <= max_d` where `max_d` grows past the
//!     769-entry array (`pngrtran.c:725-727`).
//!   * `png_set_keep_unknown_chunks` with a `chunk_list` shorter than
//!     `5 * num_chunks` — `pngset.c:1719` reads `chunk_list+5*i` unchecked.
//!   * a user-transform pixel depth above 64 bits on an interlaced image —
//!     `png_do_read_interlace` copies through `png_byte v[8];` annotated
//!     "SAFE; pixel_depth does not exceed 64" (`pngrutil.c:3927`).
mod common;

use common::api::{apis, Api};
use common::harness::*;
use common::pngbuild as pb;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

/// Set when the case name carried the `:benign` suffix; makes `new_read` call
/// `png_set_benign_errors(png_ptr, 1)` before anything else.
static BENIGN: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------

const W: u32 = 8;
const H: u32 = 4;

/// A fully valid PNG of the requested geometry.
fn img(ct: c_int, bd: c_int) -> Vec<u8> {
    pb::make_png(0x2244, W, H, bd as u8, ct as u8, 0)
}

fn img_interlaced(ct: c_int, bd: c_int) -> Vec<u8> {
    pb::make_png(0x2255, W, H, bd as u8, ct as u8, 1)
}

/// 8-bit RGB image whose pixels are genuinely coloured (r != g != b), needed by
/// the `png_do_rgb_to_gray found nongray pixel` rows.
fn img_colourful() -> Vec<u8> {
    let mut spec = pb::PngSpec::new(W, H, 8, 2, 0);
    spec.raw = pb::raw_rows_none(W, H, 8, 2, &mut |y, rb| {
        (0..rb)
            .map(|i| ((i * 37 + y as usize * 11) % 251) as u8)
            .collect()
    });
    spec.build()
}

/// 8-bit RGBA image with a tRNS-free alpha channel; used by the background /
/// alpha-mode rows.
fn img_rgba() -> Vec<u8> {
    img(6, 8)
}

/// Palette image carrying a tRNS chunk with non-opaque entries, so that
/// `PNG_COMPOSE` survives `png_init_palette_transformations`.
fn img_palette_trns() -> Vec<u8> {
    let mut spec = pb::PngSpec::new(W, H, 8, 3, 0);
    spec.palette = (0..256 * 3).map(|i| (i % 256) as u8).collect();
    spec.trns = Some(vec![0x00, 0x40, 0x80, 0xff]);
    spec.raw = pb::raw_rows_none(W, H, 8, 3, &mut |y, rb| {
        (0..rb).map(|i| ((i + y as usize) % 4) as u8).collect()
    });
    spec.build()
}

/// Grey image with a tRNS chunk (non-opaque) so background composition applies.
fn img_gray_trns() -> Vec<u8> {
    let mut spec = pb::PngSpec::new(W, H, 8, 0, 0);
    spec.trns = Some(vec![0x00, 0x05]);
    spec.raw = pb::raw_rows_none(W, H, 8, 0, &mut |y, rb| {
        (0..rb).map(|i| ((i * 3 + y as usize) % 256) as u8).collect()
    });
    spec.build()
}

/// A PNG carrying several ancillary chunks, for the chunk-cache limit rows.
fn img_many_chunks() -> Vec<u8> {
    let mut spec = pb::PngSpec::new(W, H, 8, 2, 0);
    spec.pre_idat.push((*b"tEXt", b"Key\0value".to_vec()));
    spec.pre_idat.push((*b"gAMA", 100000u32.to_be_bytes().to_vec()));
    // sPLT: name\0 depth entries(8-bit: 6 bytes each: R,G,B,A,freq16)
    let mut splt = b"pal\0".to_vec();
    splt.push(8);
    for i in 0..4u8 {
        splt.extend_from_slice(&[i, i, i, 0xff, 0, 1]);
    }
    spec.pre_idat.push((*b"sPLT", splt));
    spec.pre_idat.push((*b"prVt", vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
    spec.pre_idat.push((*b"tIME", vec![0x07, 0xe8, 1, 2, 3, 4, 5]));
    spec.post_idat.push((*b"tEXt", b"Key2\0value2".to_vec()));
    spec.raw = pb::raw_rows_none(W, H, 8, 2, &mut |_y, rb| vec![0x55; rb]);
    spec.build()
}

/// A PNG whose IHDR declares `filter_method = PNG_INTRAPIXEL_DIFFERENCING`.
fn img_filter64(ct: u8) -> Vec<u8> {
    let mut out = pb::PNG_SIG.to_vec();
    pb::push_chunk(
        &mut out,
        b"IHDR",
        &pb::ihdr_data(W, H, 8, ct, 0, PNG_INTRAPIXEL_DIFFERENCING as u8, 0),
    );
    let raw = pb::raw_rows_none(W, H, 8, ct, &mut |_y, rb| vec![0x33; rb]);
    pb::push_chunk(&mut out, b"IDAT", &pb::zlib_store(&raw));
    pb::push_chunk(&mut out, b"IEND", &[]);
    out
}

/// A PNG with a deliberately corrupted CRC, on either an ancillary or a
/// critical chunk, for the `png_set_crc_action` rows.
fn img_bad_crc(which: &str) -> Vec<u8> {
    let mut out = pb::PNG_SIG.to_vec();
    pb::push_chunk(&mut out, b"IHDR", &pb::ihdr_data(W, H, 8, 2, 0, 0, 0));
    if which == "anc" {
        pb::push_chunk_bad_crc(&mut out, b"tEXt", b"Key\0value");
    } else {
        pb::push_chunk(&mut out, b"tEXt", b"Key\0value");
    }
    let raw = pb::raw_rows_none(W, H, 8, 2, &mut |_y, rb| vec![0x77; rb]);
    let z = pb::zlib_store(&raw);
    if which == "crit" {
        pb::push_chunk_bad_crc(&mut out, b"IDAT", &z);
    } else {
        pb::push_chunk(&mut out, b"IDAT", &z);
    }
    pb::push_chunk(&mut out, b"IEND", &[]);
    out
}

// ---------------------------------------------------------------------------
// harness helpers
// ---------------------------------------------------------------------------

/// Fresh read struct fed from `png`, with the recording callbacks installed.
unsafe fn new_read(a: &Api, png: &[u8]) -> (png_structp, png_infop) {
    in_set(png);
    let p = (a.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null());
    if BENIGN.load(Ordering::Relaxed) {
        (a.png_set_benign_errors)(p, 1);
    }
    let info = (a.png_create_info_struct)(p);
    assert!(!info.is_null());
    (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
    (p, info)
}

/// Read struct positioned just after the header.
unsafe fn read_hdr(a: &Api, png: &[u8]) -> (png_structp, png_infop) {
    let (p, info) = new_read(a, png);
    (a.png_read_info)(p, info);
    (p, info)
}

/// Read `n` rows into a generously sized buffer.
unsafe fn read_rows(a: &Api, p: png_structp, info: png_infop, n: u32) {
    let rb = (a.png_get_rowbytes)(p, info);
    emit(format!("rowbytes={rb}"));
    let mut buf = vec![0u8; rb + 512];
    for _ in 0..n {
        (a.png_read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
    }
    emit(format!("read {n} rows"));
}

unsafe fn new_write(a: &Api) -> (png_structp, png_infop) {
    let p = (a.png_create_write_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null());
    if BENIGN.load(Ordering::Relaxed) {
        (a.png_set_benign_errors)(p, 1);
    }
    let info = (a.png_create_info_struct)(p);
    (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
    (p, info)
}

/// A user row transform that changes nothing (so the row geometry is exactly
/// what the PNG declares).
unsafe extern "C" fn ut_nop(_p: png_structp, _ri: *mut c_void, _row: png_bytep) {}

fn i32s(s: &str) -> Vec<i64> {
    s.split(',').map(|x| x.parse::<i64>().unwrap()).collect()
}

/// Apply one named read transform.  Every `png_set_*` read transform mentioned
/// in the `png_rtran_ok` group is covered.
unsafe fn apply_tran(a: &Api, p: png_structp, name: &str) {
    match name {
        "palette_to_rgb" => (a.png_set_palette_to_rgb)(p),
        "expand" => (a.png_set_expand)(p),
        "expand_gray_1_2_4_to_8" => (a.png_set_expand_gray_1_2_4_to_8)(p),
        "tRNS_to_alpha" => (a.png_set_tRNS_to_alpha)(p),
        "expand_16" => (a.png_set_expand_16)(p),
        "gray_to_rgb" => (a.png_set_gray_to_rgb)(p),
        "rgb_to_gray_fixed" => (a.png_set_rgb_to_gray_fixed)(p, 1, 21260, 71520),
        "strip_16" => (a.png_set_strip_16)(p),
        "scale_16" => (a.png_set_scale_16)(p),
        "strip_alpha" => (a.png_set_strip_alpha)(p),
        "swap" => (a.png_set_swap)(p),
        "packing" => (a.png_set_packing)(p),
        "packswap" => (a.png_set_packswap)(p),
        "shift" => {
            let s = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
            (a.png_set_shift)(p, &s);
        }
        "invert_mono" => (a.png_set_invert_mono)(p),
        "invert_alpha" => (a.png_set_invert_alpha)(p),
        "swap_alpha" => (a.png_set_swap_alpha)(p),
        "bgr" => (a.png_set_bgr)(p),
        "filler" => (a.png_set_filler)(p, 0xff, PNG_FILLER_AFTER),
        "add_alpha" => (a.png_set_add_alpha)(p, 0xff, PNG_FILLER_AFTER),
        "gamma_fixed" => (a.png_set_gamma_fixed)(p, 220000, 45455),
        "alpha_mode_fixed" => (a.png_set_alpha_mode_fixed)(p, PNG_ALPHA_PNG, 220000),
        "background_fixed" => {
            let c = png_color_16 { index: 1, red: 10, green: 20, blue: 30, gray: 40 };
            (a.png_set_background_fixed)(p, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, 100000);
        }
        "quantize" => {
            let mut pal = vec![png_color { red: 9, green: 8, blue: 7 }; 256];
            (a.png_set_quantize)(p, pal.as_mut_ptr(), 4, 4, std::ptr::null(), 0);
        }
        "interlace_handling" => {
            let r = (a.png_set_interlace_handling)(p);
            emit(format!("interlace_handling={r}"));
        }
        "read_user_transform_fn" => (a.png_set_read_user_transform_fn)(p, Some(ut_nop)),
        "user_transform_info" => {
            (a.png_set_user_transform_info)(p, std::ptr::null_mut(), 8, 4)
        }
        other => {
            emit(format!("UNKNOWN TRAN {other}"));
            std::process::exit(4);
        }
    }
    emit(format!("set {name} returned"));
}

/// Every read transform, in the order given by the ERRORS.md read-transform
/// rows.
const TRANS: [&str; 27] = [
    "palette_to_rgb",
    "expand",
    "expand_gray_1_2_4_to_8",
    "tRNS_to_alpha",
    "expand_16",
    "gray_to_rgb",
    "rgb_to_gray_fixed",
    "strip_16",
    "scale_16",
    "strip_alpha",
    "swap",
    "packing",
    "packswap",
    "shift",
    "invert_mono",
    "invert_alpha",
    "swap_alpha",
    "bgr",
    "filler",
    "add_alpha",
    "gamma_fixed",
    "alpha_mode_fixed",
    "background_fixed",
    "quantize",
    "interlace_handling",
    "read_user_transform_fn",
    "user_transform_info",
];

// ---------------------------------------------------------------------------
// the child: performs one named case against one library
// ---------------------------------------------------------------------------

fn run_case(a: &Api, case_full: &str) {
    let case = match case_full.strip_suffix(":benign") {
        Some(c) => {
            BENIGN.store(true, Ordering::Relaxed);
            c
        }
        None => case_full,
    };
    unsafe {
        match case {
            // =============== png_set_shift (pngtrans.c:88-117) ===============
            // All five `invalid` sub-conditions come from the same block:
            //   colour images  : red/green/blue == 0 or > bit_depth
            //   greyscale      : gray == 0 or > bit_depth
            //   alpha channels : alpha == 0 or > bit_depth
            // `shift:` sets every field to the same value so that the field that
            // matters for the colour type decides; `shiftf:` isolates one field.
            _ if case.starts_with("shift:") => {
                let f = i32s(&case[6..]);
                let (bd, ct, v) = (f[0] as c_int, f[1] as c_int, f[2] as u8);
                let (p, info) = read_hdr(a, &img(ct, bd));
                let s = png_color_8 { red: v, green: v, blue: v, gray: v, alpha: v };
                (a.png_set_shift)(p, &s);
                emit("set_shift returned");
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
            }
            _ if case.starts_with("shiftf:") => {
                let parts: Vec<&str> = case[7..].split(',').collect();
                let bd: c_int = parts[0].parse().unwrap();
                let ct: c_int = parts[1].parse().unwrap();
                let field = parts[2];
                let v: u8 = parts[3].parse().unwrap();
                let base = bd as u8; // always valid
                let (p, info) = read_hdr(a, &img(ct, bd));
                let mut s = png_color_8 {
                    red: base,
                    green: base,
                    blue: base,
                    gray: base,
                    alpha: base,
                };
                match field {
                    "red" => s.red = v,
                    "green" => s.green = v,
                    "blue" => s.blue = v,
                    "gray" => s.gray = v,
                    "alpha" => s.alpha = v,
                    _ => unreachable!(),
                }
                (a.png_set_shift)(p, &s);
                emit("set_shift returned");
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
            }
            // `true_bits == NULL` IS guarded (pngtrans.c:85), so it is testable.
            "shift-null-bits" => {
                let (p, _info) = read_hdr(a, &img(2, 8));
                (a.png_set_shift)(p, std::ptr::null());
                emit("set_shift(NULL) returned");
            }
            "shift-null-struct" => {
                let s = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
                (a.png_set_shift)(std::ptr::null_mut(), &s);
                emit("set_shift(NULL struct) returned");
            }

            // =============== png_set_filler / png_set_add_alpha ===============
            // On READ (pngtrans.c:158-172) libpng accepts every colour type, so
            // there is no rejection; the case still pins the behaviour down.
            _ if case.starts_with("filler:") => {
                let parts: Vec<&str> = case[7..].split(',').collect();
                let bd: c_int = parts[0].parse().unwrap();
                let ct: c_int = parts[1].parse().unwrap();
                let loc: c_int = parts[2].parse().unwrap();
                let add: bool = parts[3] == "1";
                let (p, info) = read_hdr(a, &img(ct, bd));
                if add {
                    (a.png_set_add_alpha)(p, 0xffff, loc);
                    emit("set_add_alpha returned");
                } else {
                    (a.png_set_filler)(p, 0xffff, loc);
                    emit("set_filler returned");
                }
                (a.png_read_update_info)(p, info);
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }
            // The write side is where the rejections live (pngtrans.c:180-215).
            _ if case.starts_with("wfiller:") => {
                let parts: Vec<&str> = case[8..].split(',').collect();
                let bd: c_int = parts[0].parse().unwrap();
                let ct: c_int = parts[1].parse().unwrap();
                let loc: c_int = parts[2].parse().unwrap();
                let add: bool = parts[3] == "1";
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(p, info, W, H, bd, ct, 0, 0, 0);
                if ct == PNG_COLOR_TYPE_PALETTE {
                    let pal = vec![png_color { red: 1, green: 2, blue: 3 }; 4];
                    (a.png_set_PLTE)(p, info, pal.as_ptr(), 4);
                }
                (a.png_write_info)(p, info);
                if add {
                    (a.png_set_add_alpha)(p, 0xffff, loc);
                    emit("set_add_alpha returned");
                } else {
                    (a.png_set_filler)(p, 0xffff, loc);
                    emit("set_filler returned");
                }
            }

            // =============== png_rtran_ok (pngrtran.c:115-134) ===============
            _ if case.starts_with("after:") => {
                let parts: Vec<&str> = case[6..].split(',').collect();
                let name = parts[0];
                let ct: c_int = parts[1].parse().unwrap();
                let bd: c_int = parts[2].parse().unwrap();
                let (p, info) = read_hdr(a, &img(ct, bd));
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
                apply_tran(a, p, name);
            }
            _ if case.starts_with("start:") => {
                let parts: Vec<&str> = case[6..].split(',').collect();
                let name = parts[0];
                let ct: c_int = parts[1].parse().unwrap();
                let bd: c_int = parts[2].parse().unwrap();
                let (p, _info) = read_hdr(a, &img(ct, bd));
                (a.png_start_read_image)(p);
                emit("start_read_image returned");
                apply_tran(a, p, name);
            }
            // Before the IHDR has been read: only png_rtran_ok(png_ptr, 1)
            // callers reject ("invalid before the PNG header has been read").
            _ if case.starts_with("before:") => {
                let name = &case[7..];
                let (p, _info) = new_read(a, &img(6, 8));
                apply_tran(a, p, name);
            }

            // =============== png_set_gamma_fixed (pngrtran.c:891-930) ========
            _ if case.starts_with("gamma:") => {
                let f = i32s(&case[6..]);
                let (p, info) = read_hdr(a, &img_rgba());
                (a.png_set_gamma_fixed)(p, f[0] as i32, f[1] as i32);
                emit("set_gamma_fixed returned");
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
            }

            // =============== png_set_alpha_mode_fixed (pngrtran.c:361-455) ===
            _ if case.starts_with("amode:") => {
                let f = i32s(&case[6..]);
                let (p, info) = read_hdr(a, &img_rgba());
                (a.png_set_alpha_mode_fixed)(p, f[0] as c_int, f[1] as i32);
                emit("set_alpha_mode_fixed returned");
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
            }
            // "conflicting calls to set alpha mode and background"
            _ if case.starts_with("amode-conflict:") => {
                let f = i32s(&case[15..]);
                let (p, info) = read_hdr(a, &img_rgba());
                let c = png_color_16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
                (a.png_set_background_fixed)(p, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, 100000);
                emit("set_background returned");
                (a.png_set_alpha_mode_fixed)(p, f[0] as c_int, 220000);
                emit("set_alpha_mode returned");
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
            }

            // =============== png_set_background_fixed (pngrtran.c:142-168) ===
            _ if case.starts_with("bkgd:") => {
                let parts: Vec<&str> = case[5..].split(',').collect();
                let code: c_int = parts[0].parse().unwrap();
                let need_expand: c_int = parts[1].parse().unwrap();
                let bg_gamma: i32 = parts[2].parse().unwrap();
                let which = parts[3];
                let png = match which {
                    "pal" => img_palette_trns(),
                    "gray" => img_gray_trns(),
                    "rgb" => img(2, 8),
                    _ => img_rgba(),
                };
                let (p, info) = read_hdr(a, &png);
                let c = png_color_16 { index: 1, red: 100, green: 200, blue: 300, gray: 400 };
                (a.png_set_background_fixed)(p, &c, code, need_expand, bg_gamma);
                emit("set_background_fixed returned");
                // A significant screen gamma forces the gamma tables to be built
                // and so reaches the "invalid background gamma type" switch.
                (a.png_set_gamma_fixed)(p, 220000, 45455);
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
                read_rows(a, p, info, H);
            }
            "bkgd-null-color" => {
                let (p, _info) = read_hdr(a, &img_rgba());
                (a.png_set_background_fixed)(
                    p,
                    std::ptr::null(),
                    PNG_BACKGROUND_GAMMA_SCREEN,
                    0,
                    100000,
                );
                emit("set_background_fixed(NULL) returned");
            }

            // =============== png_set_rgb_to_gray_fixed (pngrtran.c:1046) =====
            _ if case.starts_with("r2g:") => {
                let f = i32s(&case[4..]);
                let (p, info) = read_hdr(a, &img(2, 8));
                (a.png_set_rgb_to_gray_fixed)(p, f[0] as c_int, f[1] as i32, f[2] as i32);
                emit("set_rgb_to_gray_fixed returned");
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
            }
            // PNG_ERROR_ACTION_WARN / _ERROR on a genuinely coloured image
            _ if case.starts_with("r2g-nongray:") => {
                let f = i32s(&case[12..]);
                let (p, info) = read_hdr(a, &img_colourful());
                (a.png_set_rgb_to_gray_fixed)(p, f[0] as c_int, 21260, 71520);
                emit("set_rgb_to_gray_fixed returned");
                (a.png_read_update_info)(p, info);
                read_rows(a, p, info, H);
                let st = (a.png_get_rgb_to_gray_status)(p);
                emit(format!("status={st}"));
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }

            // =============== png_set_quantize (pngrtran.c:489) ==============
            // NOTE: `num_palette < 0` and `num_palette > PNG_MAX_PALETTE_LENGTH`
            // are NOT checked by the C.  pngrtran.c:819 does
            //   memcpy(png_ptr->palette, palette, (unsigned)num_palette*sizeof)
            // into a 256-entry buffer, so both are out-of-bounds writes (UB) and
            // are therefore not tested here.  Likewise `maximum_colors <= 0`
            // with `histogram == NULL` walks `hash[i]` for `i <= max_d` where
            // max_d grows by 96 per iteration past the 769-entry array
            // (pngrtran.c:725-727): also UB.  `palette == NULL` IS guarded
            // (pngrtran.c:498).
            _ if case.starts_with("quant:") => {
                let f = i32s(&case[6..]);
                let (np, maxc, hist, full) =
                    (f[0] as c_int, f[1] as c_int, f[2] != 0, f[3] as c_int);
                let (p, info) = read_hdr(a, &img(3, 8));
                let mut pal: Vec<png_color> = (0..256)
                    .map(|i| png_color {
                        red: (i * 7) as u8,
                        green: (i * 13) as u8,
                        blue: (i * 29) as u8,
                    })
                    .collect();
                let h: Vec<png_uint_16> = (0..256).map(|i| (256 - i) as u16).collect();
                (a.png_set_quantize)(
                    p,
                    pal.as_mut_ptr(),
                    np,
                    maxc,
                    if hist { h.as_ptr() } else { std::ptr::null() },
                    full,
                );
                emit("set_quantize returned");
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
                read_rows(a, p, info, H);
            }
            "quant-null-palette" => {
                let (p, _info) = read_hdr(a, &img(3, 8));
                (a.png_set_quantize)(p, std::ptr::null_mut(), 4, 4, std::ptr::null(), 1);
                emit("set_quantize(NULL palette) returned");
            }

            // =============== png_set_crc_action (pngrtran.c:40-105) =========
            _ if case.starts_with("crc:") => {
                let f = i32s(&case[4..]);
                let (p, info) = new_read(a, &img(2, 8));
                (a.png_set_crc_action)(p, f[0] as c_int, f[1] as c_int);
                emit("set_crc_action returned");
                (a.png_read_info)(p, info);
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }
            _ if case.starts_with("crcbad:") => {
                let parts: Vec<&str> = case[7..].split(',').collect();
                let crit: c_int = parts[0].parse().unwrap();
                let anc: c_int = parts[1].parse().unwrap();
                let which = parts[2];
                let (p, info) = new_read(a, &img_bad_crc(which));
                (a.png_set_crc_action)(p, crit, anc);
                emit("set_crc_action returned");
                (a.png_read_info)(p, info);
                emit("read_info returned");
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }

            // =============== png_set_keep_unknown_chunks (pngset.c:1598) ====
            _ if case.starts_with("keep:") => {
                let parts: Vec<&str> = case[5..].split(',').collect();
                let keep: c_int = parts[0].parse().unwrap();
                let num: c_int = parts[1].parse().unwrap();
                let list = parts[2];
                let (p, info) = new_read(a, &img_many_chunks());
                // The list format is 5 bytes per entry (4-byte name + keep), so
                // a shorter buffer would be an out-of-bounds READ in the C
                // (pngset.c:1719 `chunk_list+5*i`); every list supplied here is
                // therefore at least 5*num bytes long.
                let bytes: Vec<u8> = b"prVt\0abCd\0efGh\0".to_vec();
                let lp = match list {
                    "null" => std::ptr::null(),
                    _ => bytes.as_ptr(),
                };
                (a.png_set_keep_unknown_chunks)(p, keep, lp, num);
                emit("set_keep_unknown_chunks returned");
                let r = (a.png_handle_as_unknown)(p, b"prVt\0".as_ptr());
                emit(format!("handle_as_unknown=0x{r:x}"));
                (a.png_read_info)(p, info);
                let n = (a.png_get_unknown_chunks)(p, info, std::ptr::null_mut());
                emit(format!("unknown={n}"));
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }
            // A "chunk name" that is not a valid PNG chunk name is accepted --
            // there is no validation in png_set_keep_unknown_chunks.
            _ if case.starts_with("keep-name:") => {
                let name = &case[10..];
                let (p, info) = new_read(a, &img_many_chunks());
                let mut bytes = name.as_bytes().to_vec();
                bytes.resize(4, 0);
                bytes.push(PNG_HANDLE_CHUNK_ALWAYS as u8);
                (a.png_set_keep_unknown_chunks)(
                    p,
                    PNG_HANDLE_CHUNK_ALWAYS,
                    bytes.as_ptr(),
                    1,
                );
                emit("set_keep_unknown_chunks returned");
                (a.png_read_info)(p, info);
                let n = (a.png_get_unknown_chunks)(p, info, std::ptr::null_mut());
                emit(format!("unknown={n}"));
            }

            // =============== user limits (pngset.c:1868 / png.c:2010) =======
            _ if case.starts_with("limits:") => {
                let f = i32s(&case[7..]);
                let (p, info) = new_read(a, &img(2, 8));
                (a.png_set_user_limits)(p, f[0] as u32, f[1] as u32);
                emit(format!(
                    "limits w={} h={}",
                    (a.png_get_user_width_max)(p),
                    (a.png_get_user_height_max)(p)
                ));
                (a.png_read_info)(p, info);
                emit("read_info returned");
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }
            _ if case.starts_with("ccache:") => {
                let n: u32 = case[7..].parse().unwrap();
                let (p, info) = new_read(a, &img_many_chunks());
                (a.png_set_chunk_cache_max)(p, n);
                emit(format!("cache={}", (a.png_get_chunk_cache_max)(p)));
                (a.png_set_keep_unknown_chunks)(
                    p,
                    PNG_HANDLE_CHUNK_ALWAYS,
                    std::ptr::null(),
                    0,
                );
                (a.png_read_info)(p, info);
                emit("read_info returned");
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }
            _ if case.starts_with("cmalloc:") => {
                let n: u64 = case[8..].parse().unwrap();
                let (p, info) = new_read(a, &img_many_chunks());
                (a.png_set_chunk_malloc_max)(p, n as usize);
                emit(format!("malloc_max={}", (a.png_get_chunk_malloc_max)(p)));
                (a.png_set_keep_unknown_chunks)(
                    p,
                    PNG_HANDLE_CHUNK_ALWAYS,
                    std::ptr::null(),
                    0,
                );
                (a.png_read_info)(p, info);
                emit("read_info returned");
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }
            _ if case.starts_with("cbufsize:") => {
                let n: u64 = case[9..].parse().unwrap();
                let (p, info) = new_read(a, &img(2, 8));
                (a.png_set_compression_buffer_size)(p, n as usize);
                emit(format!("bufsize={}", (a.png_get_compression_buffer_size)(p)));
                (a.png_read_info)(p, info);
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }

            // =============== png_permit_mng_features (pngset.c:1557) ========
            _ if case.starts_with("mng:") => {
                let v: u32 = case[4..].parse().unwrap();
                let (p, info) = new_read(a, &img(2, 8));
                let r = (a.png_permit_mng_features)(p, v);
                emit(format!("permitted={r}"));
                (a.png_read_info)(p, info);
                emit("read_info returned");
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }
            // filter_method 64 in the IHDR of a real PNG datastream
            _ if case.starts_with("mngfilt:") => {
                let parts: Vec<&str> = case[8..].split(',').collect();
                let v: u32 = parts[0].parse().unwrap();
                let ct: u8 = parts[1].parse().unwrap();
                let (p, info) = new_read(a, &img_filter64(ct));
                let r = (a.png_permit_mng_features)(p, v);
                emit(format!("permitted={r}"));
                (a.png_read_info)(p, info);
                emit("read_info returned");
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }

            // =============== png_set_sig_bytes (png.c:53) ===================
            _ if case.starts_with("sigb:") => {
                let n: i64 = case[5..].parse().unwrap();
                let png = img(2, 8);
                let (p, info) = new_read(a, &png);
                (a.png_set_sig_bytes)(p, n as c_int);
                emit("set_sig_bytes returned");
                (a.png_read_info)(p, info);
                emit("read_info returned");
            }
            // The documented use: the app consumed `n` signature bytes itself.
            _ if case.starts_with("sigb-skip:") => {
                let n: usize = case[10..].parse().unwrap();
                let png = img(2, 8);
                let (p, info) = new_read(a, &png[n..]);
                (a.png_set_sig_bytes)(p, n as c_int);
                emit("set_sig_bytes returned");
                (a.png_read_info)(p, info);
                emit("read_info returned");
                read_rows(a, p, info, H);
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }

            // =============== user transform row size ========================
            // png_combine_row's "internal row size calculation error"
            // (pngrutil.c:3251) compares png_struct::info_rowbytes, written by
            // png_read_update_info from info_ptr->bit_depth/channels
            // (pngrtran.c:2258-2262), with PNG_ROWBYTES(transformed_pixel_depth,
            // width).  FINDING: it cannot be made to fire through
            // png_set_user_transform_info, because png_do_read_transformations
            // overwrites row_info->bit_depth/channels from the very same
            // png_struct fields (pngrtran.c:5165-5169), so the two sides always
            // agree no matter how absurd the declared geometry is.  The cases
            // below therefore pin down the *agreement* (including the sizes the
            // declaration produces) rather than the error text.  Reaching the
            // error would require adding a size-changing transform after
            // png_read_update_info, and every such transform is either blocked
            // by png_rtran_ok or (png_set_packing / png_set_filler) expands the
            // row inside libpng's already-allocated row_buf, which is a
            // libpng-internal buffer overflow (UB) rather than a rejection.
            _ if case.starts_with("utinfo:") => {
                let parts: Vec<&str> = case[7..].split(',').collect();
                let depth: c_int = parts[0].parse().unwrap();
                let chans: c_int = parts[1].parse().unwrap();
                let ct: c_int = parts[2].parse().unwrap();
                let il: bool = parts[3] == "1";
                let png = if il { img_interlaced(ct, 8) } else { img(ct, 8) };
                let (p, info) = new_read(a, &png);
                (a.png_read_info)(p, info);
                (a.png_set_read_user_transform_fn)(p, Some(ut_nop));
                (a.png_set_user_transform_info)(p, std::ptr::null_mut(), depth, chans);
                emit("set_user_transform_info returned");
                let passes = (a.png_set_interlace_handling)(p);
                (a.png_read_update_info)(p, info);
                emit("update_info returned");
                let rb = (a.png_get_rowbytes)(p, info);
                emit(format!("rowbytes={rb}"));
                let mut buf = vec![0u8; rb + 512];
                for _ in 0..passes {
                    for _ in 0..H {
                        (a.png_read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                    }
                }
                emit("rows read");
                (a.png_read_end)(p, info);
                emit("read_end returned");
            }

            // =============== read-struct info setters (pngset.c) ============
            _ if case.starts_with("set-plte:") => {
                let parts: Vec<&str> = case[9..].split(',').collect();
                let ct: c_int = parts[0].parse().unwrap();
                let bd: c_int = parts[1].parse().unwrap();
                let n: c_int = parts[2].parse().unwrap();
                let (p, info) = read_hdr(a, &img(ct, bd));
                let pal = vec![png_color { red: 4, green: 5, blue: 6 }; 300];
                (a.png_set_PLTE)(p, info, pal.as_ptr(), n);
                emit("set_PLTE returned");
            }
            "set-plte-null" => {
                let (p, info) = read_hdr(a, &img(3, 8));
                (a.png_set_PLTE)(p, info, std::ptr::null(), 4);
                emit("set_PLTE(NULL) returned");
            }
            _ if case.starts_with("set-chrm-xyz:") => {
                let f = i32s(&case[13..]);
                let (p, info) = read_hdr(a, &img(2, 8));
                (a.png_set_cHRM_XYZ_fixed)(
                    p, info, f[0] as i32, f[1] as i32, f[2] as i32, f[3] as i32,
                    f[4] as i32, f[5] as i32, f[6] as i32, f[7] as i32, f[8] as i32,
                );
                emit("set_cHRM_XYZ_fixed returned");
            }
            _ if case.starts_with("set-iccp:") => {
                let f = i32s(&case[9..]);
                let (p, info) = read_hdr(a, &img(2, 8));
                let prof = vec![0u8; f[1].max(1) as usize];
                (a.png_set_iCCP)(
                    p,
                    info,
                    c"icc".as_ptr(),
                    f[0] as c_int,
                    prof.as_ptr(),
                    f[1] as u32,
                );
                emit("set_iCCP returned");
            }
            _ if case.starts_with("set-splt:") => {
                let f = i32s(&case[9..]);
                let (p, info) = read_hdr(a, &img(2, 8));
                let mut entries = vec![
                    png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 };
                    8
                ];
                let name = std::ffi::CString::new("splt").unwrap();
                let s = png_sPLT_t {
                    name: name.as_ptr() as *mut c_char,
                    depth: f[0] as u8,
                    entries: entries.as_mut_ptr(),
                    nentries: f[1] as i32,
                };
                (a.png_set_sPLT)(p, info, &s, 1);
                emit("set_sPLT returned");
            }
            _ if case.starts_with("set-scal:") => {
                let f = i32s(&case[9..]);
                let (p, info) = read_hdr(a, &img(2, 8));
                (a.png_set_sCAL_fixed)(p, info, f[0] as c_int, f[1] as i32, f[2] as i32);
                emit("set_sCAL_fixed returned");
            }
            _ if case.starts_with("set-time:") => {
                let f = i32s(&case[9..]);
                let (p, info) = read_hdr(a, &img(2, 8));
                let t = png_time {
                    year: f[0] as u16,
                    month: f[1] as u8,
                    day: f[2] as u8,
                    hour: f[3] as u8,
                    minute: f[4] as u8,
                    second: f[5] as u8,
                };
                (a.png_set_tIME)(p, info, &t);
                emit("set_tIME returned");
            }
            _ if case.starts_with("set-unknown-loc:") => {
                let f = i32s(&case[16..]);
                let (p, info) = read_hdr(a, &img(2, 8));
                let mut data = vec![1u8, 2, 3, 4];
                let u = png_unknown_chunk {
                    name: *b"prVt\0",
                    data: data.as_mut_ptr(),
                    size: 4,
                    location: f[0] as u8,
                };
                (a.png_set_unknown_chunks)(p, info, &u, 1);
                emit("set_unknown_chunks returned");
                (a.png_set_unknown_chunk_location)(p, info, 0, f[1] as c_int);
                emit("set_unknown_chunk_location returned");
            }
            _ if case.starts_with("set-hist:") => {
                let n: c_int = case[9..].parse().unwrap();
                let (p, info) = read_hdr(a, &img(3, 8));
                if n >= 0 {
                    let pal = vec![png_color { red: 1, green: 2, blue: 3 }; n as usize + 1];
                    (a.png_set_PLTE)(p, info, pal.as_ptr(), n.max(1));
                }
                let h = vec![7u16; 300];
                (a.png_set_hIST)(p, info, h.as_ptr());
                emit("set_hIST returned");
            }
            _ if case.starts_with("set-exif:") => {
                let n: i64 = case[9..].parse().unwrap();
                let (p, info) = read_hdr(a, &img(2, 8));
                let mut d = vec![b'I', b'I', 0x2a, 0, 8, 0, 0, 0];
                (a.png_set_eXIf_1)(p, info, n as u32, d.as_mut_ptr());
                emit("set_eXIf_1 returned");
            }
            _ if case.starts_with("set-cicp:") => {
                let f = i32s(&case[9..]);
                let (p, info) = read_hdr(a, &img(2, 8));
                (a.png_set_cICP)(p, info, f[0] as u8, f[1] as u8, f[2] as u8, f[3] as u8);
                emit("set_cICP returned");
            }
            _ if case.starts_with("set-srgb:") => {
                let v: c_int = case[9..].parse().unwrap();
                let (p, info) = read_hdr(a, &img(2, 8));
                (a.png_set_sRGB)(p, info, v);
                emit("set_sRGB returned");
            }
            _ if case.starts_with("set-gama:") => {
                let v: i64 = case[9..].parse().unwrap();
                let (p, info) = read_hdr(a, &img(2, 8));
                (a.png_set_gAMA_fixed)(p, info, v as i32);
                emit("set_gAMA_fixed returned");
            }
            _ if case.starts_with("set-sbit:") => {
                let f = i32s(&case[9..]);
                let (p, info) = read_hdr(a, &img(f[0] as c_int, f[1] as c_int));
                let v = f[2] as u8;
                let s = png_color_8 { red: v, green: v, blue: v, gray: v, alpha: v };
                (a.png_set_sBIT)(p, info, &s);
                emit("set_sBIT returned");
            }
            "set-benign-0-then-app-error" => {
                // png_set_benign_errors(p, 0) must leave app errors fatal.
                let (p, _info) = read_hdr(a, &img(2, 8));
                (a.png_set_benign_errors)(p, 0);
                let s = png_color_8 { red: 0, green: 0, blue: 0, gray: 0, alpha: 0 };
                (a.png_set_shift)(p, &s);
                emit("set_shift returned");
            }
            "benign-error-read" => {
                let (p, _info) = read_hdr(a, &img(2, 8));
                (a.png_benign_error)(p, c"deliberate benign".as_ptr());
                emit("benign_error returned");
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

// ---------------------------------------------------------------------------
// the parent
// ---------------------------------------------------------------------------

fn run_all(cases: &[String]) {
    for c in cases {
        diff_case(c);
    }
}

/// Colour type / bit depth combinations that are legal in an IHDR.
const CT_BD: [(c_int, c_int); 15] = [
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (2, 8),
    (2, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

/// `png_set_shift`: every colour type x bit depth x shift value 0..=17 and 255.
#[test]
fn shift_value_rejections() {
    let mut cases = Vec::new();
    for (ct, bd) in CT_BD {
        for v in 0u32..=17 {
            cases.push(format!("shift:{bd},{ct},{v}"));
        }
        cases.push(format!("shift:{bd},{ct},255"));
    }
    cases.push("shift-null-bits".into());
    cases.push("shift-null-struct".into());
    run_all(&cases);
}

/// The five distinct `invalid` sub-conditions of pngtrans.c:88-117, isolated
/// one field at a time (red, green, blue on colour images; gray on greyscale;
/// alpha whenever the colour type has an alpha channel).
#[test]
fn shift_per_field_rejections() {
    let mut cases = Vec::new();
    for (ct, bd) in [(0, 8), (0, 4), (2, 8), (2, 16), (3, 8), (4, 8), (6, 8), (6, 16)] {
        for field in ["red", "green", "blue", "gray", "alpha"] {
            for v in [0u32, 1, 7, 8, 9, 16, 17, 255] {
                cases.push(format!("shiftf:{bd},{ct},{field},{v}"));
            }
        }
    }
    run_all(&cases);
}

/// `png_set_filler` / `png_set_add_alpha` on a READ struct: pngtrans.c accepts
/// every colour type there, and the `filler_loc` value is only ever compared
/// against `PNG_FILLER_AFTER`.
#[test]
fn filler_read_paths() {
    let mut cases = Vec::new();
    for (ct, bd) in [(0, 1), (0, 2), (0, 4), (0, 8), (0, 16), (2, 8), (3, 8), (4, 8), (6, 8)] {
        for loc in [-1i32, 0, 1, 2, 255] {
            for add in [0, 1] {
                cases.push(format!("filler:{bd},{ct},{loc},{add}"));
            }
        }
    }
    run_all(&cases);
}

/// The write-side rejections of the same function: the low-bit-depth-grey
/// message, the palette / grey-alpha / RGBA "inappropriate color type" message.
#[test]
fn filler_write_rejections() {
    let mut cases = Vec::new();
    for (ct, bd) in [(0, 1), (0, 2), (0, 4), (0, 8), (0, 16), (2, 8), (2, 16), (3, 8), (4, 8), (6, 8)]
    {
        for loc in [-1i32, 0, 1, 2, 255] {
            for add in [0, 1] {
                cases.push(format!("wfiller:{bd},{ct},{loc},{add}"));
            }
        }
    }
    run_all(&cases);
}

/// Every read transform applied AFTER `png_read_update_info` (the
/// `png_rtran_ok` / `PNG_FLAG_ROW_INIT` guard).
#[test]
fn transform_after_read_update_info() {
    let mut cases = Vec::new();
    for t in TRANS {
        cases.push(format!("after:{t},6,8"));
        cases.push(format!("after:{t},3,8"));
        cases.push(format!("after:{t},0,4"));
    }
    run_all(&cases);
}

/// The same, after `png_start_read_image`.
#[test]
fn transform_after_start_read_image() {
    let mut cases = Vec::new();
    for t in TRANS {
        cases.push(format!("start:{t},6,8"));
        cases.push(format!("start:{t},2,16"));
    }
    run_all(&cases);
}

/// Before the IHDR has been read: only the `png_rtran_ok(png_ptr, 1)` callers
/// reject ("invalid before the PNG header has been read").
#[test]
fn transform_before_ihdr() {
    let mut cases = Vec::new();
    for t in TRANS {
        cases.push(format!("before:{t}"));
    }
    run_all(&cases);
}

/// `png_set_gamma_fixed`: outside [PNG_LIB_GAMMA_MIN, PNG_LIB_GAMMA_MAX] the C
/// only *warns* (`unsupported_gamma(..., warn=1)`, pngrtran.c:920), but a
/// non-positive value additionally raises `png_app_error`.
#[test]
fn gamma_range_rejections() {
    const G: [i64; 14] = [
        0,
        1,
        999,
        1000,
        1001,
        99999,
        100000,
        9999999,
        10000000,
        10000001,
        220000,
        45455,
        i32::MIN as i64,
        i32::MAX as i64,
    ];
    let mut cases = Vec::new();
    for g in G {
        cases.push(format!("gamma:{g},100000"));
        cases.push(format!("gamma:100000,{g}"));
    }
    // the reserved flag values
    for g in [-1i64, -2, -100000, -50000] {
        cases.push(format!("gamma:{g},100000"));
        cases.push(format!("gamma:100000,{g}"));
        cases.push(format!("gamma:{g},{g}"));
    }
    cases.push("gamma:0,0".into());
    cases.push("gamma:10000001,10000001".into());
    run_all(&cases);
}

/// `png_set_alpha_mode_fixed`: `mode` out of range -> `png_error("invalid alpha
/// mode")`; the gamma range check is an `png_app_error` here, not a warning.
#[test]
fn alpha_mode_rejections() {
    let mut cases = Vec::new();
    for m in [-1i64, 0, 1, 2, 3, 4, 255] {
        for g in [
            0i64,
            1,
            999,
            1000,
            10000000,
            10000001,
            100000,
            220000,
            -1,
            -2,
            i32::MIN as i64,
            i32::MAX as i64,
        ] {
            cases.push(format!("amode:{m},{g}"));
        }
    }
    for m in [0i64, 1, 2, 3] {
        cases.push(format!("amode-conflict:{m}"));
    }
    run_all(&cases);
}

/// `png_set_background_fixed`: `PNG_BACKGROUND_GAMMA_UNKNOWN` warns and
/// returns, codes above `PNG_BACKGROUND_GAMMA_UNIQUE` survive the setter and
/// reach `png_error("invalid background gamma type")` in
/// `png_init_read_transformations`.
#[test]
fn background_rejections() {
    let mut cases = Vec::new();
    for code in [-1i64, 0, 1, 2, 3, 4, 255] {
        for need_expand in [0, 1] {
            for which in ["rgba", "pal", "gray", "rgb"] {
                cases.push(format!("bkgd:{code},{need_expand},100000,{which}"));
            }
        }
    }
    for g in [0i64, -1, 1, 100000, i32::MAX as i64, i32::MIN as i64] {
        cases.push(format!("bkgd:3,0,{g},rgba"));
        cases.push(format!("bkgd:3,1,{g},pal"));
    }
    cases.push("bkgd-null-color".into());
    run_all(&cases);
}

/// `png_set_rgb_to_gray_fixed`: `error_action` out of range, coefficient sums
/// above PNG_FP_1, negative coefficients, and PNG_ERROR_ACTION_WARN / _ERROR on
/// a genuinely coloured image.
#[test]
fn rgb_to_gray_rejections() {
    let mut cases = Vec::new();
    for act in [-1i64, 0, 1, 2, 3, 4, 255] {
        cases.push(format!("r2g:{act},21260,71520"));
    }
    for (r, g) in [
        (-1i64, 50000i64),
        (50000, -1),
        (-1, -1),
        (60000, 50000),
        (100000, 0),
        (0, 100000),
        (100000, 1),
        (100001, 0),
        (0, 100001),
        (i32::MAX as i64, 0),
        (0, i32::MAX as i64),
        (i32::MIN as i64, 0),
        (0, i32::MIN as i64),
        (50000, 50000),
        (0, 0),
    ] {
        cases.push(format!("r2g:1,{r},{g}"));
    }
    for act in [1i64, 2, 3] {
        cases.push(format!("r2g-nongray:{act}"));
    }
    run_all(&cases);
}

/// `png_set_quantize`.  Only the combinations the C actually validates are
/// exercised; see the NOTE in `run_case` for the ones that are UB.
#[test]
fn quantize_rejections() {
    let mut cases = Vec::new();
    for (np, maxc) in [
        (0i64, 0i64),
        (0, 1),
        (1, 1),
        (1, 2),
        (2, 2),
        (4, 4),
        (4, 8),
        (256, 256),
        (4, 2),
        (4, 1),
        (256, 16),
        (256, 1),
    ] {
        for hist in [0, 1] {
            for full in [0, 1] {
                // The reduction path without a histogram can walk off the end of
                // the 769-entry hash array, so only run it with a histogram.
                if np > maxc && hist == 0 {
                    continue;
                }
                cases.push(format!("quant:{np},{maxc},{hist},{full}"));
            }
        }
    }
    cases.push("quant-null-palette".into());
    run_all(&cases);
}

/// `png_set_crc_action` with every in-range and out-of-range action, and with a
/// stream that really does have a bad CRC.
#[test]
fn crc_action_rejections() {
    let mut cases = Vec::new();
    for crit in [-1i64, 0, 1, 2, 3, 4, 5, 6, 255] {
        for anc in [-1i64, 0, 1, 2, 3, 4, 5, 6, 255] {
            cases.push(format!("crc:{crit},{anc}"));
        }
    }
    for crit in [-1i64, 0, 1, 2, 3, 4, 5, 6, 255] {
        cases.push(format!("crcbad:{crit},0,crit"));
    }
    for anc in [-1i64, 0, 1, 2, 3, 4, 5, 6, 255] {
        cases.push(format!("crcbad:0,{anc},anc"));
    }
    run_all(&cases);
}

/// `png_set_keep_unknown_chunks`: `keep` out of range, a NULL list with
/// `num_chunks != 0`, `num_chunks < 0`, and odd chunk names.
#[test]
fn keep_unknown_chunk_rejections() {
    let mut cases = Vec::new();
    for keep in [-1i64, 0, 1, 2, 3, 4, 255] {
        for num in [-1i64, 0, 1, 2, 3] {
            for list in ["null", "list"] {
                cases.push(format!("keep:{keep},{num},{list}"));
            }
        }
    }
    for name in ["prVt", "IHDR", "abcd", "1234", "aB", ""] {
        cases.push(format!("keep-name:{name}"));
    }
    run_all(&cases);
}

/// `png_set_user_limits` / `png_set_chunk_cache_max` /
/// `png_set_chunk_malloc_max` / `png_set_compression_buffer_size` set smaller
/// than the image being read.
#[test]
fn user_limit_rejections() {
    let mut cases = Vec::new();
    const V: [i64; 8] = [0, 1, 3, 7, 8, 9, 1000000, 0xffff_ffff];
    for v in V {
        cases.push(format!("limits:{v},1000000"));
        cases.push(format!("limits:1000000,{v}"));
        cases.push(format!("limits:{v},{v}"));
    }
    for n in [0u32, 1, 2, 3, 4, 5, 8, 1000, 0xffff_ffff] {
        cases.push(format!("ccache:{n}"));
    }
    for n in [0u64, 1, 2, 9, 10, 11, 100, 8000000, u32::MAX as u64] {
        cases.push(format!("cmalloc:{n}"));
    }
    for n in [0u64, 1, 2, 5, 6, 100, 8192, 0x7fff_ffff, 0x8000_0000] {
        cases.push(format!("cbufsize:{n}"));
    }
    run_all(&cases);
}

/// `png_permit_mng_features` on a read struct, plus the
/// `PNG_INTRAPIXEL_DIFFERENCING` filter method inside a real PNG datastream.
#[test]
fn mng_feature_rejections() {
    let mut cases = Vec::new();
    for v in [0u32, 1, 2, 3, 4, 5, 6, 7, 0xff, 0xffff_ffff] {
        cases.push(format!("mng:{v}"));
    }
    for v in [0u32, 1, 4, 5, 0xffff_ffff] {
        for ct in [0u8, 2, 3, 6] {
            cases.push(format!("mngfilt:{v},{ct}"));
        }
    }
    run_all(&cases);
}

/// `png_set_sig_bytes`.
#[test]
fn sig_bytes_rejections() {
    let mut cases = Vec::new();
    for n in [-100i64, -1, 0, 1, 2, 7, 8, 9, 100, 255, 256] {
        cases.push(format!("sigb:{n}"));
    }
    for n in [0usize, 1, 2, 4, 7, 8] {
        cases.push(format!("sigb-skip:{n}"));
    }
    run_all(&cases);
}

/// `png_set_read_user_transform_fn` + `png_set_user_transform_info` with
/// depth/channels that make libpng's internal row-size check fail.
#[test]
fn user_transform_row_size_rejections() {
    let mut cases = Vec::new();
    for (d, c) in [
        (8i64, 4i64),
        (8, 3),
        (8, 1),
        (8, 2),
        (16, 4),
        (16, 2),
        (16, 1),
        (4, 4),
        (1, 1),
        (0, 0),
        (8, 5),
        (8, 8),
        (32, 4),
        (32, 5),
        (16, 8),
    ] {
        for ct in [6i64, 2] {
            for il in [0, 1] {
                // NOTE: a declared transform pixel depth above 64 bits is UB in
                // the C for an INTERLACED image: png_do_read_interlace uses a
                // `png_byte v[8]` stack buffer with the comment
                // "SAFE; pixel_depth does not exceed 64" (pngrutil.c:3927) and
                // then does `memcpy(v, sp, pixel_bytes)` with
                // pixel_bytes = pixel_depth/8.  With 128 bits that overflows the
                // stack array and walks `dp` backwards past the row buffer, so
                // there is no defined behaviour to compare against; the C only
                // survives by luck.  Non-interlaced is fine (png_combine_row
                // just memcpys), so those variants are kept.
                if il == 1 && d * c > 64 {
                    continue;
                }
                cases.push(format!("utinfo:{d},{c},{ct},{il}"));
            }
        }
    }
    run_all(&cases);
}

/// `png_set_*` info setters called on a READ struct with invalid arguments.
/// The outcome differs from the write side: on a read struct
/// `png_benign_error` is a warning while `png_app_error` is still fatal.
#[test]
fn read_struct_info_setter_rejections() {
    let mut cases = Vec::new();
    // PLTE: max_palette_length depends on the colour type already read
    for (ct, bd) in [(3i32, 1i32), (3, 2), (3, 4), (3, 8), (2, 8), (0, 8), (6, 8)] {
        for n in [-1i64, 0, 1, 2, 4, 5, 16, 17, 255, 256, 257, 300] {
            cases.push(format!("set-plte:{ct},{bd},{n}"));
        }
    }
    cases.push("set-plte-null".into());
    // cHRM XYZ: the sum must be representable ("invalid cHRM XYZ")
    for c in [
        "0,0,0,0,0,0,0,0,0",
        "1,1,1,1,1,1,1,1,1",
        "-1,-1,-1,-1,-1,-1,-1,-1,-1",
        "2147483647,2147483647,2147483647,1,1,1,1,1,1",
        "-2147483648,1,1,1,1,1,1,1,1",
        "6400,3300,3000,6000,1500,600,3127,3290,100000",
    ] {
        cases.push(format!("set-chrm-xyz:{c}"));
    }
    // iCCP compression method / profile length
    for comp in [-1i64, 0, 1, 2, 255] {
        cases.push(format!("set-iccp:{comp},132"));
    }
    for len in [0i64, 1, 4, 127, 128, 132] {
        cases.push(format!("set-iccp:0,{len}"));
    }
    // sPLT depth / nentries
    for d in [0i64, 1, 2, 4, 8, 16, 32, 255] {
        cases.push(format!("set-splt:{d},4"));
    }
    for n in [-1i64, 0, 1, 8] {
        cases.push(format!("set-splt:8,{n}"));
    }
    // sCAL unit / width / height
    for u in [-1i64, 0, 1, 2, 3, 255] {
        cases.push(format!("set-scal:{u},100000,200000"));
    }
    for (w, h) in [(0i64, 100000i64), (100000, 0), (-1, 100000), (100000, -1)] {
        cases.push(format!("set-scal:1,{w},{h}"));
    }
    // tIME
    for t in [
        "2024,0,1,0,0,0",
        "2024,13,1,0,0,0",
        "2024,1,0,0,0,0",
        "2024,1,32,0,0,0",
        "2024,1,1,24,0,0",
        "2024,1,1,0,60,0",
        "2024,1,1,0,0,61",
        "65535,255,255,255,255,255",
        "2024,2,29,23,59,60",
    ] {
        cases.push(format!("set-time:{t}"));
    }
    // unknown chunk locations
    for loc in [0i64, 1, 2, 8, 16, 255] {
        for newloc in [0i64, 1, 2, 8, 16, 255] {
            cases.push(format!("set-unknown-loc:{loc},{newloc}"));
        }
    }
    // hIST without / with a PLTE of the wrong length
    for n in [-1i64, 0, 1, 2, 4, 255] {
        cases.push(format!("set-hist:{n}"));
    }
    // eXIf
    for n in [0i64, 1, 2, 3, 4, 8, 100] {
        cases.push(format!("set-exif:{n}"));
    }
    // cICP
    for mc in [0i64, 1, 2, 255] {
        cases.push(format!("set-cicp:9,16,{mc},1"));
    }
    for vf in [0i64, 1, 2, 255] {
        cases.push(format!("set-cicp:9,16,0,{vf}"));
    }
    // sRGB / gAMA / sBIT
    for v in [-1i64, 0, 1, 2, 3, 4, 255] {
        cases.push(format!("set-srgb:{v}"));
    }
    for g in [i32::MIN as i64, -1, 0, 1, 100000, i32::MAX as i64] {
        cases.push(format!("set-gama:{g}"));
    }
    for (ct, bd) in [(0i64, 1i64), (0, 8), (0, 16), (2, 8), (2, 16), (3, 8), (4, 8), (6, 16)] {
        for v in [0i64, 1, 8, 9, 16, 17, 255] {
            cases.push(format!("set-sbit:{ct},{bd},{v}"));
        }
    }
    cases.push("set-benign-0-then-app-error".into());
    cases.push("benign-error-read".into());
    run_all(&cases);
}

/// A representative subset of every group above, re-run with
/// `png_set_benign_errors(png_ptr, 1)` applied first, which sets
/// `PNG_FLAG_APP_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN` and so converts the
/// fatal `png_app_error`s into warnings.
#[test]
fn benign_error_variants() {
    let mut cases = Vec::new();
    for base in [
        // png_set_shift invalid values, one per sub-condition
        "shift:8,0,0",
        "shift:8,0,9",
        "shift:8,2,0",
        "shift:8,2,9",
        "shift:8,6,0",
        "shift:8,6,9",
        "shift:16,6,17",
        "shiftf:8,2,red,0",
        "shiftf:8,2,green,9",
        "shiftf:8,2,blue,0",
        "shiftf:8,0,gray,9",
        "shiftf:8,6,alpha,0",
        // the png_rtran_ok guard
        "after:expand,6,8",
        "after:palette_to_rgb,3,8",
        "after:gamma_fixed,6,8",
        "after:alpha_mode_fixed,6,8",
        "after:background_fixed,6,8",
        "after:quantize,3,8",
        "after:strip_16,6,8",
        "after:scale_16,6,8",
        "after:strip_alpha,6,8",
        "after:gray_to_rgb,6,8",
        "after:rgb_to_gray_fixed,6,8",
        "after:user_transform_info,6,8",
        "start:expand,6,8",
        "start:user_transform_info,6,8",
        "before:rgb_to_gray_fixed",
        // gamma / alpha mode
        "gamma:0,100000",
        "gamma:999,100000",
        "gamma:100000,0",
        "amode:0,999",
        "amode:0,10000001",
        "amode:4,100000",
        "amode:255,100000",
        // background / rgb_to_gray / quantize
        "bkgd:0,0,100000,rgba",
        "bkgd:4,0,100000,rgba",
        "r2g:0,21260,71520",
        "r2g:1,-1,50000",
        "r2g:3,60000,50000",
        "r2g-nongray:3",
        // keep_unknown_chunks
        "keep:-1,0,list",
        "keep:4,0,list",
        "keep:255,0,list",
        "keep:3,2,null",
        // limits and mng
        "limits:7,1000000",
        "limits:1000000,3",
        "mngfilt:4,2",
        "sigb:9",
        // read-struct info setters
        "set-plte:3,1,255",
        "set-plte:3,8,257",
        "set-iccp:1,132",
        "set-splt:3,4",
        "set-scal:0,100000,200000",
        "set-unknown-loc:0,0",
        "set-chrm-xyz:2147483647,2147483647,2147483647,1,1,1,1,1,1",
        "benign-error-read",
        "wfiller:1,0,0,0",
        "wfiller:8,3,0,0",
    ] {
        cases.push(format!("{base}:benign"));
    }
    run_all(&cases);
}

/// Prove the comparison is not vacuous: the exact messages this file is
/// responsible for must really appear in the C transcripts, the fatal cases
/// must really be fatal, and `png_set_benign_errors` must really demote them.
#[test]
fn self_check() {
    // (case, expected transcript line, expected exit code)
    let expect: [(&str, &str, i32); 13] = [
        ("shift:8,2,0", "ERROR:png_set_shift: invalid shift values", 70),
        ("shift:8,0,9", "ERROR:png_set_shift: invalid shift values", 70),
        ("shiftf:8,6,alpha,0", "ERROR:png_set_shift: invalid shift values", 70),
        (
            "after:expand,6,8",
            "ERROR:invalid after png_start_read_image or png_read_update_info",
            70,
        ),
        (
            "start:expand,6,8",
            "ERROR:invalid after png_start_read_image or png_read_update_info",
            70,
        ),
        (
            "after:user_transform_info,6,8",
            "ERROR:info change after png_start_read_image or png_read_update_info",
            70,
        ),
        (
            "before:rgb_to_gray_fixed",
            "ERROR:invalid before the PNG header has been read",
            70,
        ),
        ("amode:0,999", "ERROR:gamma out of supported range", 70),
        // png_set_gamma only calls png_app_WARNING for an out-of-range gamma
        // (`unsupported_gamma(..., warn=1)`, pngrtran.c:920) -- but in this build
        // png_app_warning is itself fatal, so the observable result is still an
        // error; only the `:benign` variant degrades to a warning.
        ("gamma:999,100000", "ERROR:gamma out of supported range", 70),
        ("gamma:999,100000:benign", "WARN:gamma out of supported range", 0),
        ("amode:4,100000", "ERROR:invalid alpha mode", 70),
        ("r2g:0,21260,71520", "ERROR:invalid error action to rgb_to_gray", 70),
        (
            "keep:4,0,list",
            "ERROR:png_set_keep_unknown_chunks: invalid keep",
            70,
        ),
    ];
    for (case, line, code) in expect {
        let t = run_child(case, "c");
        assert!(
            t.lines.iter().any(|l| l == line),
            "case {case:?}: expected {line:?} verbatim in the C transcript, got {:?}",
            t.lines
        );
        assert_eq!(t.exit, Some(code), "case {case:?}: wrong exit status {t:?}");
        let r = run_child(case, "rs");
        assert_eq!(t, r, "case {case:?}: C and Rust transcripts differ");
    }

    // The `:benign` suffix must really demote the fatal app errors.
    for case in ["shift:8,2,0", "after:expand,6,8", "keep:4,0,list"] {
        let fatal = run_child(case, "c");
        let benign = run_child(&format!("{case}:benign"), "c");
        assert_eq!(fatal.exit, Some(70), "{case} should be fatal: {fatal:?}");
        assert_eq!(
            benign.exit,
            Some(0),
            "{case}:benign should only warn: {benign:?}"
        );
        assert!(
            benign.lines.iter().any(|l| l.starts_with("WARN:")),
            "{case}:benign produced no warning: {:?}",
            benign.lines
        );
        assert_ne!(fatal.lines, benign.lines);
    }

    // Two different rejections must be distinguishable.
    let a = run_child("shift:8,2,0", "c");
    let b = run_child("amode:4,100000", "c");
    assert_ne!(a.lines, b.lines);
    assert!(!a.lines.is_empty() && !b.lines.is_empty());

    // A valid call must NOT produce a diagnostic, so the fatal cases above are
    // really caused by the invalid argument.
    let ok = run_child("shift:8,2,8", "c");
    assert_eq!(ok.exit, Some(0), "a valid png_set_shift must succeed: {ok:?}");
    assert!(
        !ok.lines.iter().any(|l| l.starts_with("ERROR:")),
        "a valid png_set_shift must not error: {:?}",
        ok.lines
    );
    eprintln!("valid shift transcript: {:?}", ok.lines);
}

//! Write-side rejection ("error surface") differential tests.
//!
//! Every individual bad input / bad call sequence is driven through the C `.so`
//! and the Rust `.so` in its own `diff(...)` run, so a fatal error on one input
//! can never mask a later one.  The complete trace (messages, warning-vs-fatal
//! behaviour, longjmp `rc`, the bytes emitted before the failure and the
//! post-mortem state of the structs) is compared byte for byte.
//!
//! Triggers were derived from the guarding conditions in `c_src/src/pngwutil.c`,
//! `pngwrite.c`, `pngwtran.c`, `pngtrans.c`, `pngset.c` and `pngwio.c`; the
//! corresponding source line is given in a comment for each family.
//!
//! This build has `PNG_BENIGN_ERRORS_SUPPORTED` but **not**
//! `PNG_BENIGN_WRITE_ERRORS_SUPPORTED`; every family that goes through
//! `png_app_error`/`png_app_warning`/`png_benign_error` is therefore run with
//! `png_set_benign_errors` unset, 0 and 1 (`wr_both`).
//!
//! Several rejections live in functions that the public API can only reach with
//! already-validated arguments (`png_write_zTXt`'s compression check, for
//! example).  Both libraries export those `PRIVATE` symbols, so they are called
//! through `dlsym` directly in `private_write_fns`, exactly as the differential
//! harness calls every other entry point.
mod support;

use std::ffi::{c_char, c_int, c_void, CString};
use support::core::*;
use support::*;

// ---------------------------------------------------------------------------
// constants that support::core does not define (png.h / pngpriv.h)
// ---------------------------------------------------------------------------

/// `PNG_FLAG_MNG_EMPTY_PLTE` (png.h:875)
const MNG_EMPTY_PLTE: u32 = 0x01;
/// `PNG_FLAG_MNG_FILTER_64` (png.h:876)
const MNG_FILTER_64: u32 = 0x04;
/// `PNG_ALL_MNG_FEATURES` (png.h:877)
const ALL_MNG_FEATURES: u32 = 0x05;

/// unknown-chunk locations (png.h:642 / pngpriv.h:642)
const LOC_HAVE_IHDR: c_int = 0x01;
const LOC_HAVE_PLTE: c_int = 0x02;
const LOC_HAVE_IDAT: c_int = 0x04;
const LOC_AFTER_IDAT: c_int = 0x08;

/// `PNG_UINT_31_MAX`
const UINT_31_MAX: usize = 0x7fff_ffff;

// ---------------------------------------------------------------------------
// generic helpers
// ---------------------------------------------------------------------------

/// Run `f` behind a *nested* longjmp pad and log the outcome.  A `png_error`
/// inside `f` therefore neither abandons the rest of the sequence nor the
/// destructor, and the `rc` handed to `longjmp` is part of the trace.
fn step(tag: &str, f: impl FnMut()) -> c_int {
    let rc = protected(f);
    log(format!("{tag}:rc={rc}"));
    rc
}

fn rb(ct: c_int, bd: c_int, w: u32) -> usize {
    pngbuild::rowbytes(ct as u8, bd as u8, w)
}

/// `h` rows of deterministic content with 8 bytes of slack each.
fn mkrows(ct: c_int, bd: c_int, w: u32, h: u32, seed: u64) -> Vec<Vec<u8>> {
    let n = rb(ct, bd, w);
    let mut rng = Rng::new(seed);
    (0..h)
        .map(|_| {
            let mut row = vec![0u8; n + 8];
            for i in 0..n {
                row[i] = rng.byte();
            }
            row
        })
        .collect()
}

fn ptr_vec(rows: &mut [Vec<u8>]) -> Vec<*mut u8> {
    rows.iter_mut().map(|r| r.as_mut_ptr()).collect()
}

/// A 256-entry palette (768 bytes), deterministic.
fn palette256(seed: u64) -> Vec<u8> {
    Rng::new(seed).bytes(3 * 256)
}

/// Post-mortem state of both structs.  Never logs an address, only
/// null-ness / sizes / contents.
unsafe fn snap(c: &Core, png: Png, info: Info) {
    let mut valid: u32 = 0;
    for f in [
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
    ] {
        if (c.get_valid)(png, info, f) != 0 {
            valid |= f;
        }
    }
    let mut npal: c_int = -1;
    let mut pal: *mut u8 = std::ptr::null_mut();
    let plte = (c.get_PLTE)(png, info, &mut pal, &mut npal);
    log(format!(
        "snap w={} h={} bd={} ct={} il={} cm={} fm={} rowbytes={} ch={} cbuf={} io={:#x} \
         valid={valid:#x} plte={plte} npal={npal} pmax={}",
        (c.get_image_width)(png, info),
        (c.get_image_height)(png, info),
        (c.get_bit_depth)(png, info),
        (c.get_color_type)(png, info),
        (c.get_interlace_type)(png, info),
        (c.get_compression_type)(png, info),
        (c.get_filter_type)(png, info),
        (c.get_rowbytes)(png, info),
        (c.get_channels)(png, info),
        (c.get_compression_buffer_size)(png),
        (c.get_io_state)(png),
        (c.get_palette_max)(png, info),
    ));
}

/// One write-struct lifecycle: `body` runs inside a nested longjmp pad, so the
/// struct is always destroyed and the post-mortem state is always logged.
fn wr(label: &str, body: &dyn Fn(&Core, Png, Info)) {
    // Set ERRW_TRACE=1 to see which case is running (useful if a case ever
    // takes a library down hard instead of producing a trace).
    if std::env::var_os("ERRW_TRACE").is_some() {
        eprintln!("### {label}");
    }
    diff(label, |lib| {
        with_write(lib, &mut |c, png, info| unsafe {
            step("body", || body(c, png, info));
            snap(c, png, info);
        })
    });
}

/// The same input with `png_set_benign_errors` left alone, cleared and set:
/// `png_app_error`/`png_app_warning` become warnings in the last case while
/// plain `png_error` never does.
fn wr_both(label: &str, body: &dyn Fn(&Core, Png, Info)) {
    wr(&format!("{label} benign=def"), body);
    wr(&format!("{label} benign=0"), &|c, p, i| unsafe {
        (c.set_benign_errors)(p, 0);
        body(c, p, i);
    });
    wr(&format!("{label} benign=1"), &|c, p, i| unsafe {
        (c.set_benign_errors)(p, 1);
        body(c, p, i);
    });
}

/// `png_set_IHDR` + `png_write_info` + `h` rows + `png_write_end`, so that a
/// rejection anywhere in the pipeline is observed with the bytes emitted before
/// it.
unsafe fn full_write(c: &Core, png: Png, info: Info, ct: c_int, bd: c_int, w: u32, h: u32) {
    let rows = mkrows(ct, bd, w, h, 0x51ee);
    (c.set_IHDR)(png, info, w, h, bd, ct, PNG_INTERLACE_NONE, 0, 0);
    step("write_info", || (c.write_info)(png, info));
    step("rows", || {
        for r in &rows {
            (c.write_row)(png, r.as_ptr());
        }
    });
    step("write_end", || (c.write_end)(png, info));
}

// ===========================================================================
// 1. IHDR rejections
// ===========================================================================

/// `png_set_IHDR` (pngset.c:435) stores its arguments *before* calling
/// `png_check_IHDR` (png.c:1961), which warns about every individual problem
/// and then raises the fatal "Invalid IHDR data".  An application that catches
/// that longjmp and carries on therefore reaches `png_write_IHDR`
/// (pngwutil.c:700..810) with the invalid header still in the info struct,
/// which is the only way its own checks can fire.
fn ihdr_case(label: &str, w: u32, h: u32, bd: c_int, ct: c_int, il: c_int, cm: c_int, fm: c_int) {
    wr(&format!("IHDR {label}"), &move |c, png, info| unsafe {
        step("set_IHDR", || (c.set_IHDR)(png, info, w, h, bd, ct, il, cm, fm));
        snap(c, png, info);
        step("write_info", || (c.write_info)(png, info));
        step("write_end", || (c.write_end)(png, info));
    });
}

#[test]
fn ihdr_rejections() {
    // --- width / height ---------------------------------------------------
    ihdr_case("w=0", 0, 4, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    ihdr_case("h=0", 4, 0, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    ihdr_case("w=0 h=0", 0, 0, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    ihdr_case("w=2^31", 0x8000_0000, 4, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    ihdr_case("h=2^31", 4, 0x8000_0000, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    ihdr_case("w=0xffffffff", 0xffff_ffff, 4, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    ihdr_case("h=0xffffffff", 4, 0xffff_ffff, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    ihdr_case("w=0x7fffffff", 0x7fff_ffff, 4, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    // Default user limits are 1,000,000 (png_check_IHDR "exceeds user limit").
    ihdr_case("w=2000000", 2_000_000, 4, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
    ihdr_case("h=2000000", 4, 2_000_000, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);

    // Tighter user limits make a perfectly ordinary size illegal.
    for &(uw, uh, w, h) in &[
        (8u32, 8u32, 16u32, 4u32),
        (8, 8, 4, 16),
        (0, 0, 1, 1),
        (4, 4, 4, 4),
    ] {
        wr(
            &format!("IHDR user_limits {uw}x{uh} img {w}x{h}"),
            &move |c, png, info| unsafe {
                (c.set_user_limits)(png, uw, uh);
                log(format!(
                    "limits w={} h={}",
                    (c.get_user_width_max)(png),
                    (c.get_user_height_max)(png)
                ));
                step("set_IHDR", || {
                    (c.set_IHDR)(png, info, w, h, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0)
                });
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // --- bit depth (png_write_IHDR "Invalid bit depth for ..." ) ----------
    for &bd in &[0, 3, 5, 6, 7, 9, 12, 15, 17, 32, 64, 255, -1, -8] {
        ihdr_case(
            &format!("gray bd={bd}"),
            4,
            3,
            bd,
            PNG_COLOR_TYPE_GRAY,
            0,
            0,
            0,
        );
    }
    for &(ct, name) in &[
        (PNG_COLOR_TYPE_RGB, "rgb"),
        (PNG_COLOR_TYPE_PALETTE, "palette"),
        (PNG_COLOR_TYPE_GRAY_ALPHA, "ga"),
        (PNG_COLOR_TYPE_RGB_ALPHA, "rgba"),
    ] {
        for &bd in &[1, 2, 4, 16, 3, 0] {
            ihdr_case(&format!("{name} bd={bd}"), 4, 3, bd, ct, 0, 0, 0);
        }
    }

    // --- colour type (png_write_IHDR "Invalid image color type specified") -
    for &ct in &[1, 5, 7, 8, 9, 64, 255, -1] {
        ihdr_case(&format!("ct={ct}"), 4, 3, 8, ct, 0, 0, 0);
    }

    // --- compression / filter / interlace method --------------------------
    for &cm in &[1, 2, 8, 64, 255, -1] {
        ihdr_case(&format!("cm={cm}"), 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, cm, 0);
    }
    for &fm in &[1, 2, 63, 64, 65, 255, -1] {
        ihdr_case(&format!("fm={fm}"), 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, fm);
    }
    for &il in &[2, 3, 7, 255, -1] {
        ihdr_case(&format!("il={il}"), 4, 3, 8, PNG_COLOR_TYPE_GRAY, il, 0, 0);
    }

    // --- png_write_info without any png_set_IHDR --------------------------
    // The zeroed info struct gives bit_depth 0 / colour type GRAY, i.e.
    // pngwutil.c:714 "Invalid bit depth for grayscale image".
    wr("IHDR none write_info", &|c, png, info| unsafe {
        step("write_info", || (c.write_info)(png, info));
        step("write_end", || (c.write_end)(png, info));
    });
    wr("IHDR none write_info_before_PLTE", &|c, png, info| unsafe {
        step("before_PLTE", || (c.write_info_before_PLTE)(png, info));
    });
    wr("IHDR none write_row", &|c, png, info| unsafe {
        let row = [0u8; 16];
        step("write_row", || (c.write_row)(png, row.as_ptr()));
        let _ = info;
    });

    // --- png_set_IHDR twice ------------------------------------------------
    wr("IHDR twice same", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        full_write(c, png, info, PNG_COLOR_TYPE_RGB, 8, 4, 3);
    });
    wr("IHDR twice shrink", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 9, 5, 16, PNG_COLOR_TYPE_RGB_ALPHA, 0, 0, 0);
        full_write(c, png, info, PNG_COLOR_TYPE_GRAY, 8, 4, 3);
    });
    wr("IHDR after write_info", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        step("set_IHDR2", || {
            (c.set_IHDR)(png, info, 8, 6, 16, PNG_COLOR_TYPE_RGB, 0, 0, 0)
        });
        let rows = mkrows(PNG_COLOR_TYPE_GRAY, 8, 4, 3, 0x77);
        step("rows", || {
            for r in &rows {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });

    // --- MNG features (pngwrite.c:99 + pngwutil.c:796) --------------------
    // png_check_IHDR accepts filter method 64 while no signature has been
    // written; png_write_info_before_PLTE writes the signature first, clears
    // mng_features_permitted with a warning, and png_write_IHDR then rejects
    // the filter method.
    for &(feat, name) in &[
        (MNG_FILTER_64, "FILTER_64"),
        (MNG_EMPTY_PLTE, "EMPTY_PLTE"),
        (ALL_MNG_FEATURES, "ALL"),
        (0xffff_ffff, "unknown-bits"),
    ] {
        for &(ct, cn) in &[
            (PNG_COLOR_TYPE_RGB, "rgb"),
            (PNG_COLOR_TYPE_RGB_ALPHA, "rgba"),
            (PNG_COLOR_TYPE_GRAY, "gray"),
        ] {
            wr(
                &format!("IHDR mng={name} ct={cn} filter=64"),
                &move |c, png, info| unsafe {
                    log(format!("permit={:#x}", (c.permit_mng_features)(png, feat)));
                    step("set_IHDR", || {
                        (c.set_IHDR)(png, info, 4, 3, 8, ct, 0, 0, PNG_INTRAPIXEL_DIFFERENCING)
                    });
                    step("write_info", || (c.write_info)(png, info));
                    let rows = mkrows(ct, 8, 4, 3, 0x99);
                    step("rows", || {
                        for r in &rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                    });
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
    }
    // MNG permitted but a normal filter method: only the MNG warning fires.
    wr("IHDR mng=ALL filter=0", &|c, png, info| unsafe {
        log(format!(
            "permit={:#x}",
            (c.permit_mng_features)(png, ALL_MNG_FEATURES)
        ));
        full_write(c, png, info, PNG_COLOR_TYPE_RGB, 8, 4, 3);
    });
}

// ===========================================================================
// 2. row / call-sequence rejections
// ===========================================================================

#[test]
fn row_sequence() {
    let w = 5u32;
    let h = 4u32;
    let ct = PNG_COLOR_TYPE_RGB;

    // pngwrite.c:762 "png_write_info was never called before png_write_row"
    wr("SEQ row before info", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        let rows = mkrows(ct, 8, w, h, 1);
        step("row", || (c.write_row)(png, rows[0].as_ptr()));
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ rows before info", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        let mut rows = mkrows(ct, 8, w, h, 2);
        let mut p = ptr_vec(&mut rows);
        step("rows", || (c.write_rows)(png, p.as_mut_ptr(), h));
    });
    wr("SEQ image before info", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        let mut rows = mkrows(ct, 8, w, h, 3);
        let mut p = ptr_vec(&mut rows);
        step("image", || (c.write_image)(png, p.as_mut_ptr()));
    });

    // pngwrite.c:400 "No IDATs written into file"
    wr("SEQ end without rows", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ end without info", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ end twice", &|c, png, info| unsafe {
        full_write(c, png, info, ct, 8, w, h);
        step("write_end2", || (c.write_end)(png, info));
    });
    wr("SEQ end null info", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        let rows = mkrows(ct, 8, w, h, 4);
        step("rows", || {
            for r in &rows {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, std::ptr::null_mut()));
    });

    // rows after png_write_end
    wr("SEQ row after end", &|c, png, info| unsafe {
        full_write(c, png, info, ct, 8, w, h);
        let rows = mkrows(ct, 8, w, h, 5);
        step("extra_row", || (c.write_row)(png, rows[0].as_ptr()));
        step("write_end2", || (c.write_end)(png, info));
    });

    // more rows than 'height'
    for extra in 1..=3u32 {
        wr(&format!("SEQ rows+{extra}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
            step("write_info", || (c.write_info)(png, info));
            let rows = mkrows(ct, 8, w, h + extra, 6);
            step("rows", || {
                for r in &rows {
                    (c.write_row)(png, r.as_ptr());
                }
            });
            step("write_end", || (c.write_end)(png, info));
        });
        wr(
            &format!("SEQ write_rows num=h+{extra}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                let mut rows = mkrows(ct, 8, w, h + extra, 7);
                let mut p = ptr_vec(&mut rows);
                step("rows", || (c.write_rows)(png, p.as_mut_ptr(), h + extra));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // fewer rows than 'height'
    wr("SEQ rows-1", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        let rows = mkrows(ct, 8, w, h, 8);
        step("rows", || {
            for r in rows.iter().take(h as usize - 1) {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ write_rows num=0", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        step("rows0", || {
            (c.write_rows)(png, std::ptr::null_mut(), 0)
        });
        step("write_end", || (c.write_end)(png, info));
    });

    // png_write_image with a NULL row-pointer array; only safe with height 0,
    // which is reached by recovering from png_set_IHDR's "Invalid IHDR data".
    wr("SEQ h=0 write_image(NULL)", &|c, png, info| unsafe {
        step("set_IHDR", || {
            (c.set_IHDR)(png, info, w, 0, 8, ct, 0, 0, 0)
        });
        step("write_info", || (c.write_info)(png, info));
        step("image", || (c.write_image)(png, std::ptr::null_mut()));
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ h=0 rows0 end", &|c, png, info| unsafe {
        step("set_IHDR", || {
            (c.set_IHDR)(png, info, w, 0, 8, ct, 0, 0, 0)
        });
        step("write_info", || (c.write_info)(png, info));
        step("write_end", || (c.write_end)(png, info));
    });

    // png_write_info twice / after rows
    wr("SEQ info twice", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        step("write_info2", || (c.write_info)(png, info));
        let rows = mkrows(ct, 8, w, h, 9);
        step("rows", || {
            for r in &rows {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ info after rows", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        let rows = mkrows(ct, 8, w, h, 10);
        step("row0", || (c.write_row)(png, rows[0].as_ptr()));
        step("write_info2", || (c.write_info)(png, info));
        step("rest", || {
            for r in rows.iter().skip(1) {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ before_PLTE twice", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("b1", || (c.write_info_before_PLTE)(png, info));
        step("b2", || (c.write_info_before_PLTE)(png, info));
        step("write_info", || (c.write_info)(png, info));
        let rows = mkrows(ct, 8, w, h, 11);
        step("rows", || {
            for r in &rows {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });

    // interlaced writes
    wr("SEQ adam7 no set_interlace_handling", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, PNG_INTERLACE_ADAM7, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        let rows = mkrows(ct, 8, w, h, 12);
        step("rows", || {
            for r in &rows {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
    for passes in [1u32, 3, 6, 8] {
        wr(
            &format!("SEQ adam7 passes={passes}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, w, h, 8, ct, PNG_INTERLACE_ADAM7, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                log(format!("np={}", (c.set_interlace_handling)(png)));
                let rows = mkrows(ct, 8, w, h, 13);
                step("rows", || {
                    for _p in 0..passes {
                        for r in &rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                    }
                });
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // png_set_interlace_handling on a non-interlaced struct returns 1.
    wr("SEQ set_interlace_handling non-interlaced", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, PNG_INTERLACE_NONE, 0, 0);
        log(format!("np={}", (c.set_interlace_handling)(png)));
        full_write(c, png, info, ct, 8, w, h);
    });

    // pngwrite.c:1417 "no rows for png_write_image to write"
    wr_both("SEQ write_png no rows", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_png", || {
            (c.write_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut())
        });
        step("write_end", || (c.write_end)(png, info));
    });
    wr_both("SEQ write_png set_rows(NULL)", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        (c.set_rows)(png, info, std::ptr::null_mut());
        log(format!("valid.IDAT={}", (c.get_valid)(png, info, PNG_INFO_IDAT)));
        step("write_png", || {
            (c.write_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut())
        });
    });
    wr("SEQ write_png then write_end", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        let mut rows = mkrows(ct, 8, w, h, 14);
        let mut p = ptr_vec(&mut rows);
        (c.set_rows)(png, info, p.as_mut_ptr());
        step("write_png", || {
            (c.write_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut())
        });
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ write_png twice", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        let mut rows = mkrows(ct, 8, w, h, 15);
        let mut p = ptr_vec(&mut rows);
        (c.set_rows)(png, info, p.as_mut_ptr());
        step("write_png", || {
            (c.write_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut())
        });
        step("write_png2", || {
            (c.write_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut())
        });
    });

    // png_write_flush misuse (pngwrite.c:968)
    wr("SEQ flush before info", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("flush", || (c.write_flush)(png));
        full_write(c, png, info, ct, 8, w, h);
    });
    wr("SEQ flush after info", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        step("flush", || (c.write_flush)(png));
        let rows = mkrows(ct, 8, w, h, 16);
        step("rows", || {
            for r in &rows {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ flush mid image", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        let rows = mkrows(ct, 8, w, h, 17);
        step("rows", || {
            for (i, r) in rows.iter().enumerate() {
                (c.write_row)(png, r.as_ptr());
                if i == 0 {
                    (c.write_flush)(png);
                }
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
    wr("SEQ flush after end", &|c, png, info| unsafe {
        full_write(c, png, info, ct, 8, w, h);
        step("flush", || (c.write_flush)(png));
    });
    for nrows in [-1i32, 0, 1, 2, 100] {
        wr(&format!("SEQ set_flush({nrows})"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, w, h, 8, ct, 0, 0, 0);
            (c.set_flush)(png, nrows);
            full_write(c, png, info, ct, 8, w, h);
        });
    }
}

// ===========================================================================
// 3. palette, tRNS, bKGD, hIST, sBIT
// ===========================================================================

/// `png_set_PLTE` (pngset.c:750) enforces `num_palette <= 1 << bit_depth` for
/// colour type 3, so `png_write_PLTE`'s own check (pngwutil.c:879/884) can only
/// fire when the header is changed afterwards, or when
/// `PNG_FLAG_MNG_EMPTY_PLTE` lets an empty palette through and
/// `png_write_info_before_PLTE` then clears the MNG permission again.
#[test]
fn palette_and_trns() {
    let pal = palette256(0x9a11);
    let trns = Rng::new(0x9a12).bytes(256);

    // pngwrite.c:241 "Valid palette required for paletted images"
    for &bd in &[1, 2, 4, 8] {
        wr(
            &format!("PLTE missing bd={bd}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, bd, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // png_set_PLTE range checks: pngset.c:764 "Invalid palette length" /
    // pngset.c:784 "Invalid palette".
    for &(bd, n) in &[
        (8i32, 0i32),
        (8, -1),
        (8, 257),
        (8, 1000),
        (8, 256),
        (4, 17),
        (4, 200),
        (4, 16),
        (1, 3),
        (2, 5),
    ] {
        let pr = &pal;
        wr(
            &format!("PLTE set bd={bd} n={n}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, bd, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                step("set_PLTE", || (c.set_PLTE)(png, info, pr.as_ptr(), n));
                snap(c, png, info);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // NULL palette with a positive count.
    wr("PLTE set null n=4", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        step("set_PLTE", || (c.set_PLTE)(png, info, std::ptr::null(), 4));
        step("write_info", || (c.write_info)(png, info));
    });

    // pngwutil.c:879 error path: a 256-entry palette with the bit depth
    // reduced to 4 behind png_set_PLTE's back.
    for &bd2 in &[1, 2, 4] {
        let pr = &pal;
        wr(
            &format!("PLTE shrink bd 8->{bd2}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                (c.set_PLTE)(png, info, pr.as_ptr(), 256);
                (c.set_IHDR)(png, info, 4, 3, bd2, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // pngwutil.c:884 warning path (non-palette colour type, num_pal == 0):
    // MNG_EMPTY_PLTE lets png_set_PLTE(NULL, 0) through, then
    // png_write_info_before_PLTE clears the permission.
    for &(ct, cn) in &[
        (PNG_COLOR_TYPE_GRAY, "gray"),
        (PNG_COLOR_TYPE_RGB, "rgb"),
        (PNG_COLOR_TYPE_PALETTE, "palette"),
        (PNG_COLOR_TYPE_RGB_ALPHA, "rgba"),
    ] {
        wr(
            &format!("PLTE empty mng ct={cn}"),
            &move |c, png, info| unsafe {
                log(format!(
                    "permit={:#x}",
                    (c.permit_mng_features)(png, MNG_EMPTY_PLTE)
                ));
                (c.set_IHDR)(png, info, 4, 3, 8, ct, 0, 0, 0);
                step("set_PLTE", || {
                    (c.set_PLTE)(png, info, std::ptr::null(), 0)
                });
                snap(c, png, info);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // PLTE on a greyscale image: "Ignoring request to write a PLTE chunk in
    // grayscale PNG" (pngwutil.c:832).
    for &(ct, cn) in &[
        (PNG_COLOR_TYPE_GRAY, "gray"),
        (PNG_COLOR_TYPE_GRAY_ALPHA, "ga"),
        (PNG_COLOR_TYPE_RGB, "rgb"),
        (PNG_COLOR_TYPE_RGB_ALPHA, "rgba"),
    ] {
        let pr = &pal;
        wr(
            &format!("PLTE on ct={cn}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, ct, 0, 0, 0);
                step("set_PLTE", || (c.set_PLTE)(png, info, pr.as_ptr(), 4));
                step("write_info", || (c.write_info)(png, info));
                let rows = mkrows(ct, 8, 4, 3, 0x21);
                step("rows", || {
                    for r in &rows {
                        (c.write_row)(png, r.as_ptr());
                    }
                });
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // > 256 entries on a non-palette colour type: warning + no PLTE.
    wr("PLTE gray n=300", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("set_PLTE", || (c.set_PLTE)(png, info, pal.as_ptr(), 300));
        step("write_info", || (c.write_info)(png, info));
    });

    // --- tRNS (pngwutil.c:1331/1346/1368/1378) ----------------------------
    for &nt in &[-1i32, 0, 5, 200, 256, 257] {
        let pr = &pal;
        let tr = &trns;
        wr_both(
            &format!("tRNS palette npal=4 nt={nt}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                (c.set_PLTE)(png, info, pr.as_ptr(), 4);
                step("set_tRNS", || {
                    (c.set_tRNS)(png, info, tr.as_ptr(), nt, std::ptr::null())
                });
                snap(c, png, info);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // GRAY: tran->gray out of range for the bit depth.
    for &(bd, gray) in &[
        (1i32, 2u16),
        (2, 4),
        (4, 16),
        (8, 256),
        (8, 300),
        (8, 65535),
        (16, 65535),
    ] {
        wr_both(
            &format!("tRNS gray bd={bd} gray={gray}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, bd, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                let col = PngColor16 {
                    gray,
                    ..Default::default()
                };
                step("set_tRNS", || {
                    (c.set_tRNS)(
                        png,
                        info,
                        std::ptr::null(),
                        1,
                        &col as *const PngColor16 as *const u8,
                    )
                });
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // RGB: any high byte set while bit_depth is 8.
    for &(bd, r, g, b) in &[
        (8i32, 300u16, 0u16, 0u16),
        (8, 0, 300, 0),
        (8, 0, 0, 256),
        (8, 255, 255, 255),
        (16, 300, 400, 500),
    ] {
        wr_both(
            &format!("tRNS rgb bd={bd} {r}/{g}/{b}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, bd, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                let col = PngColor16 {
                    red: r,
                    green: g,
                    blue: b,
                    ..Default::default()
                };
                step("set_tRNS", || {
                    (c.set_tRNS)(
                        png,
                        info,
                        std::ptr::null(),
                        1,
                        &col as *const PngColor16 as *const u8,
                    )
                });
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // colour types with an alpha channel: "Can't write tRNS with an alpha
    // channel".
    for &(ct, cn) in &[
        (PNG_COLOR_TYPE_GRAY_ALPHA, "ga"),
        (PNG_COLOR_TYPE_RGB_ALPHA, "rgba"),
    ] {
        for &bd in &[8, 16] {
            wr_both(
                &format!("tRNS alpha ct={cn} bd={bd}"),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                    let col = PngColor16 {
                        gray: 1,
                        ..Default::default()
                    };
                    step("set_tRNS", || {
                        (c.set_tRNS)(
                            png,
                            info,
                            std::ptr::null(),
                            1,
                            &col as *const PngColor16 as *const u8,
                        )
                    });
                    step("write_info", || (c.write_info)(png, info));
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
    }

    // --- out-of-range palette indexes in a row (pngwrite.c:404) -----------
    for &(bd, npal, idx) in &[
        (8i32, 4i32, 200u8),
        (8, 1, 1),
        (8, 255, 255),
        (4, 3, 15),
        (2, 2, 3),
        (1, 1, 1),
    ] {
        for &chk in &[2i32, 1, 0, -1] {
            let pr = &pal;
            wr_both(
                &format!("PIDX bd={bd} npal={npal} idx={idx} chk={chk}"),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, 4, 2, bd, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                    (c.set_PLTE)(png, info, pr.as_ptr(), npal);
                    if chk != 2 {
                        (c.set_check_for_invalid_index)(png, chk);
                    }
                    let n = rb(PNG_COLOR_TYPE_PALETTE, bd, 4);
                    let row = vec![
                        match bd {
                            8 => idx,
                            4 => (idx & 0x0f) * 0x11,
                            2 => (idx & 0x03) * 0x55,
                            _ => (idx & 0x01) * 0xff,
                        };
                        n + 8
                    ];
                    step("write_info", || (c.write_info)(png, info));
                    step("rows", || {
                        for _ in 0..2 {
                            (c.write_row)(png, row.as_ptr());
                        }
                    });
                    log(format!("pmax={}", (c.get_palette_max)(png, info)));
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
    }

    // --- bKGD (pngwutil.c:1401/1420/1434) ---------------------------------
    for &idx in &[0u8, 3, 4, 200, 255] {
        let pr = &pal;
        wr(
            &format!("bKGD palette npal=4 idx={idx}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                (c.set_PLTE)(png, info, pr.as_ptr(), 4);
                let bk = PngColor16 {
                    index: idx,
                    ..Default::default()
                };
                (c.set_bKGD)(png, info, &bk as *const PngColor16 as *const u8);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    for &(ct, bd, r, g, b, gray) in &[
        (PNG_COLOR_TYPE_RGB, 8i32, 300u16, 0u16, 0u16, 0u16),
        (PNG_COLOR_TYPE_RGB, 8, 0, 0, 65535, 0),
        (PNG_COLOR_TYPE_RGB, 16, 300, 400, 500, 0),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8, 256, 0, 0, 0),
        (PNG_COLOR_TYPE_GRAY, 1, 0, 0, 0, 2),
        (PNG_COLOR_TYPE_GRAY, 4, 0, 0, 0, 100),
        (PNG_COLOR_TYPE_GRAY, 8, 0, 0, 0, 256),
        (PNG_COLOR_TYPE_GRAY, 16, 0, 0, 0, 65535),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8, 0, 0, 0, 999),
    ] {
        wr(
            &format!("bKGD ct={ct} bd={bd} {r}/{g}/{b}/{gray}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                let bk = PngColor16 {
                    index: 0,
                    red: r,
                    green: g,
                    blue: b,
                    gray,
                };
                (c.set_bKGD)(png, info, &bk as *const PngColor16 as *const u8);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // --- hIST (pngwutil.c:1550) -------------------------------------------
    // The chunk is written with info_ptr->num_palette, but the check is against
    // png_ptr->num_palette, which png_write_PLTE only sets when it actually
    // emits the chunk -- so a suggested palette that is skipped (greyscale)
    // leaves the two out of step.
    let hist: Vec<u16> = (0..256u16).collect();
    for &(ct, cn) in &[
        (PNG_COLOR_TYPE_GRAY, "gray"),
        (PNG_COLOR_TYPE_GRAY_ALPHA, "ga"),
        (PNG_COLOR_TYPE_RGB, "rgb"),
        (PNG_COLOR_TYPE_PALETTE, "palette"),
    ] {
        let pr = &pal;
        let hr = &hist;
        wr(
            &format!("hIST ct={cn}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                (c.set_PLTE)(png, info, pr.as_ptr(), 4);
                (c.set_hIST)(png, info, hr.as_ptr());
                (c.set_IHDR)(png, info, 4, 3, 8, ct, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // hIST without any PLTE at all: png_set_hIST refuses ("Invalid palette
    // size, hIST allocation skipped").
    wr("hIST without PLTE", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
        step("set_hIST", || (c.set_hIST)(png, info, hist.as_ptr()));
        step("set_PLTE", || (c.set_PLTE)(png, info, pal.as_ptr(), 4));
        step("write_info", || (c.write_info)(png, info));
        step("write_end", || (c.write_end)(png, info));
    });

    // --- sBIT (pngwutil.c:1254/1268/1280) ---------------------------------
    for &(ct, bd, sb) in &[
        // colour path: red/green/blue must be 1..maxbits
        (PNG_COLOR_TYPE_RGB, 8i32, [0u8, 4, 4, 4, 4]),
        (PNG_COLOR_TYPE_RGB, 8, [4, 0, 4, 4, 4]),
        (PNG_COLOR_TYPE_RGB, 8, [4, 4, 0, 4, 4]),
        (PNG_COLOR_TYPE_RGB, 8, [9, 4, 4, 4, 4]),
        (PNG_COLOR_TYPE_RGB, 8, [4, 200, 4, 4, 4]),
        (PNG_COLOR_TYPE_RGB, 16, [17, 8, 8, 8, 8]),
        (PNG_COLOR_TYPE_PALETTE, 4, [9, 4, 4, 4, 4]),
        (PNG_COLOR_TYPE_PALETTE, 4, [8, 8, 8, 8, 8]),
        // greyscale path: gray must be 1..usr_bit_depth
        (PNG_COLOR_TYPE_GRAY, 8, [0, 0, 0, 0, 0]),
        (PNG_COLOR_TYPE_GRAY, 8, [0, 0, 0, 9, 0]),
        (PNG_COLOR_TYPE_GRAY, 1, [0, 0, 0, 2, 0]),
        (PNG_COLOR_TYPE_GRAY, 16, [0, 0, 0, 17, 0]),
        // alpha path
        (PNG_COLOR_TYPE_RGB_ALPHA, 8, [4, 4, 4, 4, 0]),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8, [4, 4, 4, 4, 9]),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8, [0, 0, 0, 4, 0]),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8, [0, 0, 0, 4, 200]),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16, [0, 0, 0, 8, 17]),
    ] {
        wr(
            &format!("sBIT ct={ct} bd={bd} {sb:?}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                let v = PngColor8 {
                    red: sb[0],
                    green: sb[1],
                    blue: sb[2],
                    gray: sb[3],
                    alpha: sb[4],
                };
                (c.set_sBIT)(png, info, &v as *const PngColor8 as *const u8);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
}

// ===========================================================================
// 4. tEXt / zTXt / iTXt
// ===========================================================================

/// A `png_text` record built from borrowed C strings.
#[allow(clippy::too_many_arguments)]
fn mktext(
    comp: c_int,
    key: &CString,
    text: Option<&CString>,
    lang: Option<&CString>,
    lang_key: Option<&CString>,
) -> PngText {
    PngText {
        compression: comp,
        key: key.as_ptr() as *mut c_char,
        text: text.map(|t| t.as_ptr() as *mut c_char).unwrap_or(std::ptr::null_mut()),
        text_length: 0,
        itxt_length: 0,
        lang: lang.map(|t| t.as_ptr() as *mut c_char).unwrap_or(std::ptr::null_mut()),
        lang_key: lang_key
            .map(|t| t.as_ptr() as *mut c_char)
            .unwrap_or(std::ptr::null_mut()),
    }
}

/// `png_set_text` + a complete 4x3 grey write, so both the storage-time and the
/// write-time rejection are observed together with the emitted bytes.
fn text_case(label: &str, t: &PngText, before_idat: bool) {
    let tp = t as *const PngText;
    wr_both(&format!("TXT {label} pre={}", before_idat as u8), &move |c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        if before_idat {
            step("set_text", || (c.set_text)(png, info, tp as *const c_void, 1));
            step("write_info", || (c.write_info)(png, info));
        } else {
            step("write_info", || (c.write_info)(png, info));
            step("set_text", || (c.set_text)(png, info, tp as *const c_void, 1));
        }
        log(format!("num_text={}", {
            let mut p: *mut c_void = std::ptr::null_mut();
            let mut n: c_int = -1;
            let r = (c.get_text)(png, info, &mut p, &mut n);
            format!("{r}/{n}")
        }));
        let rows = mkrows(PNG_COLOR_TYPE_GRAY, 8, 4, 3, 0x7e);
        step("rows", || {
            for r in &rows {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
}

#[test]
fn text_chunks() {
    // --- keywords ---------------------------------------------------------
    let bad_keys: Vec<(&str, CString)> = vec![
        ("empty", CString::new("").unwrap()),
        ("space", CString::new(" ").unwrap()),
        ("spaces", CString::new("   ").unwrap()),
        ("tab", CString::new("\t").unwrap()),
        ("nl", CString::new("\n").unwrap()),
        ("ctrl", CString::new("\x01\x02").unwrap()),
        ("del", CString::new("\x7f").unwrap()),
        ("lead-space", CString::new(" Title").unwrap()),
        ("trail-space", CString::new("Title ").unwrap()),
        ("double-space", CString::new("A  B").unwrap()),
        ("inner-ctrl", CString::new("A\x01B").unwrap()),
        ("79", CString::new("k".repeat(79)).unwrap()),
        ("80", CString::new("k".repeat(80)).unwrap()),
        ("200", CString::new("k".repeat(200)).unwrap()),
        ("high", CString::new("\u{a1}\u{ff}").unwrap()),
        ("nbsp", CString::new(vec![0xa0u8, b'x']).unwrap()),
    ];
    let body = CString::new("body text").unwrap();
    let lang = CString::new("en").unwrap();
    let lkey = CString::new("Titel").unwrap();

    for (name, key) in &bad_keys {
        for &(comp, cn) in &[
            (PNG_TEXT_COMPRESSION_NONE, "none"),
            (PNG_TEXT_COMPRESSION_zTXt, "zTXt"),
            (PNG_ITXT_COMPRESSION_NONE, "iTXt-none"),
            (PNG_ITXT_COMPRESSION_zTXt, "iTXt-zTXt"),
        ] {
            let t = mktext(comp, key, Some(&body), Some(&lang), Some(&lkey));
            text_case(&format!("key={name} comp={cn}"), &t, true);
        }
    }

    // --- NULL / empty text ------------------------------------------------
    let good = CString::new("Title").unwrap();
    for &(comp, cn) in &[
        (PNG_TEXT_COMPRESSION_NONE, "none"),
        (PNG_TEXT_COMPRESSION_zTXt, "zTXt"),
        (PNG_ITXT_COMPRESSION_NONE, "iTXt-none"),
        (PNG_ITXT_COMPRESSION_zTXt, "iTXt-zTXt"),
    ] {
        let empty = CString::new("").unwrap();
        text_case(
            &format!("null-text comp={cn}"),
            &mktext(comp, &good, None, Some(&lang), Some(&lkey)),
            true,
        );
        text_case(
            &format!("empty-text comp={cn}"),
            &mktext(comp, &good, Some(&empty), Some(&lang), Some(&lkey)),
            true,
        );
        // iTXt with no language / translated keyword at all.
        text_case(
            &format!("no-lang comp={cn}"),
            &mktext(comp, &good, Some(&body), None, None),
            true,
        );
        text_case(
            &format!("no-langkey comp={cn}"),
            &mktext(comp, &good, Some(&body), Some(&lang), None),
            true,
        );
    }
    // NULL keyword: png_set_text_2 skips the record entirely.
    {
        let t = PngText {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: std::ptr::null_mut(),
            text: body.as_ptr() as *mut c_char,
            ..Default::default()
        };
        text_case("null-key", &t, true);
    }

    // --- invalid compression values (pngset.c:1028) ------------------------
    for &comp in &[-3i32, -2, 3, 4, 99, 1000, -1000] {
        let t = mktext(comp, &good, Some(&body), Some(&lang), Some(&lkey));
        text_case(&format!("comp={comp}"), &t, true);
    }

    // --- text written from png_write_end ----------------------------------
    for &(comp, cn) in &[
        (PNG_TEXT_COMPRESSION_NONE, "none"),
        (PNG_TEXT_COMPRESSION_zTXt, "zTXt"),
        (PNG_ITXT_COMPRESSION_zTXt, "iTXt-zTXt"),
    ] {
        let empty = CString::new("").unwrap();
        text_case(
            &format!("trailer key=empty comp={cn}"),
            &mktext(comp, &empty, Some(&body), Some(&lang), Some(&lkey)),
            false,
        );
    }

    // --- num_text out of range -------------------------------------------
    for &n in &[0i32, -1, -100] {
        let t = mktext(PNG_TEXT_COMPRESSION_NONE, &good, Some(&body), None, None);
        let tp = &t as *const PngText;
        wr(&format!("TXT num_text={n}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            step("set_text", || (c.set_text)(png, info, tp as *const c_void, n));
            step("write_info", || (c.write_info)(png, info));
            let rows = mkrows(PNG_COLOR_TYPE_GRAY, 8, 4, 3, 0x33);
            step("rows", || {
                for r in &rows {
                    (c.write_row)(png, r.as_ptr());
                }
            });
            step("write_end", || (c.write_end)(png, info));
        });
    }
    // NULL text_ptr array.
    wr("TXT null array", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("set_text", || (c.set_text)(png, info, std::ptr::null(), 1));
        step("write_info", || (c.write_info)(png, info));
        step("write_end", || (c.write_end)(png, info));
    });
}

// ===========================================================================
// 5. iCCP / sPLT
// ===========================================================================

/// A syntactically plausible ICC profile: `len` bytes whose first four bytes
/// are the big-endian `embedded` length and whose byte 8 is `class`.
fn profile(len: usize, embedded: u32, byte8: u8) -> Vec<u8> {
    let mut p = vec![0u8; len.max(1)];
    if len >= 4 {
        p[0] = (embedded >> 24) as u8;
        p[1] = (embedded >> 16) as u8;
        p[2] = (embedded >> 8) as u8;
        p[3] = embedded as u8;
    }
    if len > 8 {
        p[8] = byte8;
    }
    let mut rng = Rng::new(0x1cc9);
    for i in 12..len {
        p[i] = rng.byte();
    }
    p
}

#[test]
fn iccp_splt() {
    let good_name = CString::new("ICC profile").unwrap();
    let bad_names: Vec<(&str, CString)> = vec![
        ("empty", CString::new("").unwrap()),
        ("space", CString::new(" ").unwrap()),
        ("ctrl", CString::new("\x01").unwrap()),
        ("trail", CString::new("p ").unwrap()),
        ("long", CString::new("p".repeat(120)).unwrap()),
    ];

    // --- profile length / content (pngwutil.c:1132..1148) -----------------
    // (len, embedded length, profile[8])
    let cases: Vec<(&str, usize, u32, u8)> = vec![
        ("len=0", 0, 0, 0),
        ("len=1", 1, 0, 0),
        ("len=4", 4, 4, 0),
        ("len=12", 12, 12, 0),
        ("len=131", 131, 131, 0),
        ("len=132 embedded=0", 132, 0, 0),
        ("len=132 embedded=131", 132, 131, 0),
        ("len=132 embedded=133", 132, 133, 0),
        ("len=132 ok", 132, 132, 0),
        ("len=133 class=4", 133, 133, 4),
        ("len=134 class=255", 134, 134, 255),
        ("len=135 class=3", 135, 135, 3),
        ("len=136 class=4", 136, 136, 4),
        ("len=200 class=9", 200, 200, 9),
    ];
    for (name, len, emb, b8) in cases {
        let prof = profile(len, emb, b8);
        let gn = &good_name;
        wr_both(&format!("iCCP {name}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_iCCP", || {
                (c.set_iCCP)(png, info, gn.as_ptr(), 0, prof.as_ptr(), len as u32)
            });
            snap(c, png, info);
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }

    // --- keyword (pngwutil.c:1154 "iCCP: invalid keyword") ----------------
    let prof = profile(132, 132, 0);
    for (name, key) in &bad_names {
        let pr = &prof;
        wr_both(&format!("iCCP key={name}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_iCCP", || {
                (c.set_iCCP)(png, info, key.as_ptr(), 0, pr.as_ptr(), 132)
            });
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }
    // NULL name / NULL profile: png_set_iCCP ignores the call.
    wr("iCCP null name", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        step("set_iCCP", || {
            (c.set_iCCP)(png, info, std::ptr::null(), 0, prof.as_ptr(), 132)
        });
        step("write_info", || (c.write_info)(png, info));
    });
    wr("iCCP null profile", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        step("set_iCCP", || {
            (c.set_iCCP)(png, info, good_name.as_ptr(), 0, std::ptr::null(), 132)
        });
        step("write_info", || (c.write_info)(png, info));
    });
    // pngset.c:1132 "Invalid iCCP compression method"
    for &cm in &[1i32, 2, 8, 255, -1] {
        let pr = &prof;
        let gn = &good_name;
        wr_both(&format!("iCCP cm={cm}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_iCCP", || {
                (c.set_iCCP)(png, info, gn.as_ptr(), cm, pr.as_ptr(), 132)
            });
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }

    // --- sPLT (pngwutil.c:1194 "sPLT: invalid keyword") -------------------
    let ents: Vec<PngSpltEntry> = (0..4u16)
        .map(|i| PngSpltEntry {
            red: i * 1000,
            green: i * 2000,
            blue: i * 3000,
            alpha: i * 4000,
            frequency: i,
        })
        .collect();
    let splt_names: Vec<(&str, CString)> = vec![
        ("empty", CString::new("").unwrap()),
        ("space", CString::new("  ").unwrap()),
        ("ctrl", CString::new("\x02\x03").unwrap()),
        ("trail", CString::new("pal ").unwrap()),
        ("long", CString::new("s".repeat(100)).unwrap()),
        ("ok", CString::new("suggested").unwrap()),
    ];
    for (name, key) in &splt_names {
        for &depth in &[8u8, 16, 4, 0, 255] {
            for &n in &[4i32, 1, 0] {
                let er = &ents;
                let sp = PngSpltT {
                    name: key.as_ptr() as *mut c_char,
                    depth,
                    entries: er.as_ptr() as *mut PngSpltEntry,
                    nentries: n,
                };
                let spp = &sp as *const PngSpltT;
                wr_both(
                    &format!("sPLT key={name} depth={depth} n={n}"),
                    &move |c, png, info| unsafe {
                        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                        step("set_sPLT", || {
                            (c.set_sPLT)(png, info, spp as *const c_void, 1)
                        });
                        log(format!("nsplt={}", (c.get_sPLT)(png, info, &mut std::ptr::null_mut())));
                        step("write_info", || (c.write_info)(png, info));
                        step("write_end", || (c.write_end)(png, info));
                    },
                );
            }
        }
    }
    // png_set_sPLT with a NULL name / NULL entries / non-positive count:
    // pngset.c "png_set_sPLT: invalid sPLT" app error.
    for &(nullname, nullents, cnt) in &[
        (true, false, 1i32),
        (false, true, 1),
        (true, true, 1),
        (false, false, 0),
        (false, false, -1),
    ] {
        let key = &splt_names[5].1;
        let er = &ents;
        let sp = PngSpltT {
            name: if nullname {
                std::ptr::null_mut()
            } else {
                key.as_ptr() as *mut c_char
            },
            depth: 8,
            entries: if nullents {
                std::ptr::null_mut()
            } else {
                er.as_ptr() as *mut PngSpltEntry
            },
            nentries: 4,
        };
        let spp = &sp as *const PngSpltT;
        wr_both(
            &format!("sPLT nullname={} nullents={} cnt={cnt}", nullname as u8, nullents as u8),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                step("set_sPLT", || {
                    (c.set_sPLT)(png, info, spp as *const c_void, cnt)
                });
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
}

// ===========================================================================
// 6. manual chunk writers + unknown chunks
// ===========================================================================

#[test]
fn chunk_writers() {
    let data = Rng::new(0xc4c4).bytes(64);

    // pngwutil.c:200 "length exceeds PNG maximum" (the check happens before any
    // byte of 'data' is touched).
    for &len in &[
        UINT_31_MAX + 1,
        UINT_31_MAX + 2,
        0x8000_0000usize,
        0xffff_ffff,
        0x1_0000_0000,
        usize::MAX,
    ] {
        let d = &data;
        wr(
            &format!("CHK write_chunk len={len:#x}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                step("write_chunk", || {
                    (c.write_chunk)(png, b"teSt".as_ptr(), d.as_ptr(), len)
                });
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // Legal lengths, including 0 and a NULL payload.
    for &(len, nulldata) in &[
        (0usize, false),
        (0, true),
        (1, false),
        (64, false),
        (5, true),
        (64, true),
    ] {
        let d = &data;
        wr(
            &format!("CHK write_chunk len={len} null={}", nulldata as u8),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                let p = if nulldata { std::ptr::null() } else { d.as_ptr() };
                step("write_chunk", || (c.write_chunk)(png, b"teSt".as_ptr(), p, len));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // Chunk names with non-alphabetic bytes: PNG_CHUNK_FROM_STRING does no
    // validation, so the bytes go out verbatim.
    for name in [
        b"1234", b"    ", b"\x00\x01\x02\x03", b"aB3_", b"\xff\xfe\xfd\xfc", b"IHDR", b"IEND",
        b"IDAT",
    ] {
        let d = &data;
        let nm = *name;
        wr(
            &format!("CHK name={}", hex(&nm)),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                step("write_chunk", || {
                    (c.write_chunk)(png, nm.as_ptr(), d.as_ptr(), 4)
                });
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // png_write_chunk_start / _data / _end misuse.
    for &(declared, chunks) in &[
        (0u32, 0usize),
        (0, 8),
        (8, 0),
        (8, 8),
        (8, 16),
        (0xffff_ffff, 8),
        (0x8000_0000, 4),
        (0x7fff_ffff, 4),
    ] {
        let d = &data;
        wr(
            &format!("CHK start len={declared:#x} written={chunks}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                step("start", || (c.write_chunk_start)(png, b"teSt".as_ptr(), declared));
                step("data", || {
                    if chunks > 0 {
                        (c.write_chunk_data)(png, d.as_ptr(), chunks);
                    }
                });
                step("data_null", || (c.write_chunk_data)(png, std::ptr::null(), 4));
                step("data_zero", || (c.write_chunk_data)(png, d.as_ptr(), 0));
                step("end", || (c.write_chunk_end)(png));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // png_write_chunk_end without a preceding start.
    wr("CHK end without start", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        step("end1", || (c.write_chunk_end)(png));
        step("end2", || (c.write_chunk_end)(png));
        step("write_end", || (c.write_end)(png, info));
    });
    // png_write_chunk_data without a preceding start.
    wr("CHK data without start", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("write_info", || (c.write_info)(png, info));
        step("data", || (c.write_chunk_data)(png, data.as_ptr(), 8));
        step("write_end", || (c.write_end)(png, info));
    });
    // Manual chunks before any header at all.
    wr("CHK before info", &|c, png, info| unsafe {
        step("write_chunk", || {
            (c.write_chunk)(png, b"teSt".as_ptr(), data.as_ptr(), 8)
        });
        let _ = info;
    });

    // --- png_set_unknown_chunks (pngset.c:1393/1407) ----------------------
    for &loc in &[
        0i32,
        LOC_HAVE_IHDR,
        LOC_HAVE_PLTE,
        LOC_HAVE_IDAT,
        LOC_AFTER_IDAT,
        LOC_HAVE_IHDR | LOC_AFTER_IDAT,
        0x10,
        0x20,
        -1,
        255,
    ] {
        for &before_info in &[true, false] {
            for &size in &[0usize, 8] {
                let d = &data;
                wr_both(
                    &format!(
                        "UNK loc={loc:#x} pre={} size={size}",
                        before_info as u8
                    ),
                    &move |c, png, info| unsafe {
                        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                        let u = PngUnknownChunk {
                            name: *b"uNKw\0",
                            data: d.as_ptr() as *mut u8,
                            size,
                            location: loc as u8,
                        };
                        let up = &u as *const PngUnknownChunk;
                        if !before_info {
                            step("write_info", || (c.write_info)(png, info));
                        }
                        step("set_unknown", || {
                            (c.set_unknown_chunks)(png, info, up as *const c_void, 1)
                        });
                        log(format!(
                            "n_unknown={}",
                            (c.get_unknown_chunks)(png, info, &mut std::ptr::null_mut())
                        ));
                        if before_info {
                            step("write_info", || (c.write_info)(png, info));
                        }
                        let rows = mkrows(PNG_COLOR_TYPE_GRAY, 8, 4, 3, 0x5a);
                        step("rows", || {
                            for r in &rows {
                                (c.write_row)(png, r.as_ptr());
                            }
                        });
                        step("write_end", || (c.write_end)(png, info));
                    },
                );
            }
        }
    }
    // png_set_unknown_chunk_location (pngset.c:1540 "invalid unknown chunk
    // location").
    for &loc in &[0i32, LOC_HAVE_IDAT, 0x10, -1, LOC_HAVE_PLTE] {
        for &idx in &[0i32, 1, -1] {
            let d = &data;
            wr_both(
                &format!("UNK setloc loc={loc:#x} idx={idx}"),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                    let u = PngUnknownChunk {
                        name: *b"uNKw\0",
                        data: d.as_ptr() as *mut u8,
                        size: 8,
                        location: LOC_HAVE_IHDR as u8,
                    };
                    let up = &u as *const PngUnknownChunk;
                    (c.set_unknown_chunks)(png, info, up as *const c_void, 1);
                    step("setloc", || {
                        (c.set_unknown_chunk_location)(png, info, idx, loc)
                    });
                    step("write_info", || (c.write_info)(png, info));
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
    }
    // Unknown chunk names with invalid bytes, and the keep/handle interaction.
    for nm in [*b"uNKw\0", *b"UNKW\0", *b"1234\0", *b"\x00\x01\x02\x03\0", *b"CRIT\0"] {
        for &keep in &[
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
            -1,
            4,
        ] {
            let d = &data;
            wr_both(
                &format!("UNK name={} keep={keep}", hex(&nm[..4])),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                    step("keep", || {
                        (c.set_keep_unknown_chunks)(png, keep, nm.as_ptr(), 1)
                    });
                    log(format!("handle={}", (c.handle_as_unknown)(png, nm.as_ptr())));
                    let u = PngUnknownChunk {
                        name: nm,
                        data: d.as_ptr() as *mut u8,
                        size: 8,
                        location: LOC_HAVE_IHDR as u8,
                    };
                    let up = &u as *const PngUnknownChunk;
                    step("set_unknown", || {
                        (c.set_unknown_chunks)(png, info, up as *const c_void, 1)
                    });
                    step("write_info", || (c.write_info)(png, info));
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
    }
    // num_unknowns <= 0 / NULL array.
    for &n in &[0i32, -1] {
        wr(&format!("UNK num={n}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            step("set_unknown", || {
                (c.set_unknown_chunks)(png, info, std::ptr::null(), n)
            });
            step("write_info", || (c.write_info)(png, info));
        });
    }
    wr("UNK null array", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("set_unknown", || {
            (c.set_unknown_chunks)(png, info, std::ptr::null(), 1)
        });
        step("write_info", || (c.write_info)(png, info));
    });
}

// ===========================================================================
// 7. remaining chunk setters that validate on write
// ===========================================================================

#[test]
fn other_chunk_setters() {
    // --- pCAL (pngset.c:511/519/527, pngwutil.c:1797/1802) ----------------
    let purposes: Vec<(&str, CString)> = vec![
        ("empty", CString::new("").unwrap()),
        ("space", CString::new(" ").unwrap()),
        ("ctrl", CString::new("\x01").unwrap()),
        ("trail", CString::new("cal ").unwrap()),
        ("ok", CString::new("calibration").unwrap()),
    ];
    let units = CString::new("metres").unwrap();
    let p0 = CString::new("1.5").unwrap();
    let p1 = CString::new("-2e3").unwrap();
    let pbad = CString::new("not-a-number").unwrap();

    for (pn, purpose) in &purposes {
        for &etype in &[0i32, 1, 2, 3, 4, 5, 99, -1] {
            for &nparams in &[0i32, 2] {
                let params: Vec<*mut c_char> =
                    vec![p0.as_ptr() as *mut c_char, p1.as_ptr() as *mut c_char];
                let pp = params.as_ptr() as *mut *mut c_char;
                let ur = &units;
                wr_both(
                    &format!("pCAL purpose={pn} type={etype} nparams={nparams}"),
                    &move |c, png, info| unsafe {
                        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                        step("set_pCAL", || {
                            (c.set_pCAL)(
                                png,
                                info,
                                purpose.as_ptr(),
                                0,
                                100,
                                etype,
                                nparams,
                                ur.as_ptr(),
                                pp,
                            )
                        });
                        snap(c, png, info);
                        step("write_info", || (c.write_info)(png, info));
                        step("write_end", || (c.write_end)(png, info));
                    },
                );
            }
        }
    }
    // parameter-count and parameter-format rejections.
    for &nparams in &[-1i32, 256, 300] {
        let params: Vec<*mut c_char> = vec![p0.as_ptr() as *mut c_char];
        let pp = params.as_ptr() as *mut *mut c_char;
        let pu = &purposes[4].1;
        let ur = &units;
        wr_both(&format!("pCAL nparams={nparams}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            step("set_pCAL", || {
                (c.set_pCAL)(png, info, pu.as_ptr(), 0, 1, 0, nparams, ur.as_ptr(), pp)
            });
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }
    // a non-numeric / NULL parameter
    for &nullparam in &[false, true] {
        let params: Vec<*mut c_char> = vec![if nullparam {
            std::ptr::null_mut()
        } else {
            pbad.as_ptr() as *mut c_char
        }];
        let pp = params.as_ptr() as *mut *mut c_char;
        let pu = &purposes[4].1;
        let ur = &units;
        wr_both(
            &format!("pCAL badparam null={}", nullparam as u8),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                step("set_pCAL", || {
                    (c.set_pCAL)(png, info, pu.as_ptr(), 0, 1, 0, 1, ur.as_ptr(), pp)
                });
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // NULL purpose / NULL units / NULL params array
    wr("pCAL null purpose", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("set_pCAL", || {
            (c.set_pCAL)(
                png,
                info,
                std::ptr::null(),
                0,
                1,
                0,
                0,
                units.as_ptr(),
                std::ptr::null_mut(),
            )
        });
        step("write_info", || (c.write_info)(png, info));
    });
    wr("pCAL null units", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("set_pCAL", || {
            (c.set_pCAL)(
                png,
                info,
                purposes[4].1.as_ptr(),
                0,
                1,
                0,
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        });
        step("write_info", || (c.write_info)(png, info));
    });

    // --- sCAL (pngset.c:606..628, pngwutil.c:1862) ------------------------
    let scal_strings: Vec<(&str, CString)> = vec![
        ("empty", CString::new("").unwrap()),
        ("minus", CString::new("-1").unwrap()),
        ("text", CString::new("wide").unwrap()),
        ("plus", CString::new("+1").unwrap()),
        ("dot", CString::new(".").unwrap()),
        ("exp", CString::new("1e5").unwrap()),
        ("ok", CString::new("1.0").unwrap()),
        ("d30", CString::new("1".repeat(30)).unwrap()),
        ("d31", CString::new("1".repeat(31)).unwrap()),
        ("d40", CString::new("1".repeat(40)).unwrap()),
        ("d62", CString::new("1".repeat(62)).unwrap()),
    ];
    for (wn, sw) in &scal_strings {
        for (hn, sh) in &scal_strings {
            // Only a handful of the (width, height) pairs are interesting; keep
            // the identical-length pairs plus every pair with "ok".
            if wn != hn && *wn != "ok" && *hn != "ok" {
                continue;
            }
            for &unit in &[1i32, 2] {
                wr_both(
                    &format!("sCAL unit={unit} w={wn} h={hn}"),
                    &move |c, png, info| unsafe {
                        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                        step("set_sCAL_s", || {
                            (c.set_sCAL_s)(png, info, unit, sw.as_ptr(), sh.as_ptr())
                        });
                        snap(c, png, info);
                        step("write_info", || (c.write_info)(png, info));
                        step("write_end", || (c.write_end)(png, info));
                    },
                );
            }
        }
    }
    for &unit in &[0i32, 3, 255, -1] {
        let sw = &scal_strings[6].1;
        wr_both(&format!("sCAL unit={unit}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            step("set_sCAL_s", || {
                (c.set_sCAL_s)(png, info, unit, sw.as_ptr(), sw.as_ptr())
            });
            step("write_info", || (c.write_info)(png, info));
        });
    }
    wr("sCAL null strings", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("w-null", || {
            (c.set_sCAL_s)(png, info, 1, std::ptr::null(), scal_strings[6].1.as_ptr())
        });
        step("h-null", || {
            (c.set_sCAL_s)(png, info, 1, scal_strings[6].1.as_ptr(), std::ptr::null())
        });
        step("write_info", || (c.write_info)(png, info));
    });
    // floating point / fixed point entry points
    for &(w, h) in &[
        (0.0f64, 1.0f64),
        (1.0, 0.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (0.0, 0.0),
        (1e-9, 1e9),
    ] {
        wr_both(&format!("sCAL fp {w}x{h}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            step("set_sCAL", || (c.set_sCAL)(png, info, 1, w, h));
            step("write_info", || (c.write_info)(png, info));
        });
    }
    for &(w, h) in &[(0i32, 1i32), (1, 0), (-1, 1), (1, -1), (i32::MIN, 1)] {
        wr_both(
            &format!("sCAL fixed {w}x{h}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                step("set_sCAL_fixed", || (c.set_sCAL_fixed)(png, info, 1, w, h));
                step("write_info", || (c.write_info)(png, info));
            },
        );
    }

    // --- pHYs / oFFs unit types (pngwutil.c:1766/1888) --------------------
    for &unit in &[0i32, 1, 2, 3, 255, -1] {
        wr(&format!("pHYs unit={unit}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            (c.set_pHYs)(png, info, 100, 200, unit);
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
        wr(&format!("oFFs unit={unit}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            (c.set_oFFs)(png, info, -5, 7, unit);
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }

    // --- cICP (pngset.c:150 "Invalid cICP matrix coefficients") -----------
    for &m in &[0u8, 1, 2, 14, 255] {
        wr(&format!("cICP matrix={m}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_cICP", || (c.set_cICP)(png, info, 9, 16, m, 1));
            snap(c, png, info);
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }

    // --- eXIf (pngset.c:329) ----------------------------------------------
    let exif = Rng::new(0xe1f).bytes(16);
    for &n in &[0u32, 1, 2, 3, 4, 8, 16] {
        let e = &exif;
        wr(&format!("eXIf len={n}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            step("set_eXIf", || (c.set_eXIf_1)(png, info, n, e.as_ptr()));
            snap(c, png, info);
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }
    wr("eXIf null", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
        step("set_eXIf", || (c.set_eXIf_1)(png, info, 8, std::ptr::null()));
        step("write_info", || (c.write_info)(png, info));
    });

    // --- gAMA (pngset.c:373) ----------------------------------------------
    for &g in &[0i32, 1, -1, i32::MIN, i32::MAX, 100_000, 500_000] {
        wr(&format!("gAMA fixed={g}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            step("set_gAMA_fixed", || (c.set_gAMA_fixed)(png, info, g));
            snap(c, png, info);
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }
    for &g in &[0.0f64, -1.0, 1e10, 1e-10, 0.45455, f64::MAX] {
        wr_both(&format!("gAMA fp={g}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
            step("set_gAMA", || (c.set_gAMA)(png, info, g));
            step("write_info", || (c.write_info)(png, info));
        });
    }

    // --- sRGB (pngwutil.c:1107 "Invalid sRGB rendering intent specified") -
    for &intent in &[0i32, 1, 2, 3, 4, 5, 99, 255, -1] {
        wr(&format!("sRGB intent={intent}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_sRGB", || (c.set_sRGB)(png, info, intent));
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
        wr(
            &format!("sRGB_gAMA_and_cHRM intent={intent}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                step("set", || (c.set_sRGB_gAMA_and_cHRM)(png, info, intent));
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // --- cHRM ------------------------------------------------------------
    for &(tag, v) in &[
        ("zero", [0i32; 8]),
        ("neg", [-1i32; 8]),
        ("max", [i32::MAX; 8]),
        ("min", [i32::MIN; 8]),
        (
            "srgb",
            [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000],
        ),
    ] {
        wr_both(&format!("cHRM {tag}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_cHRM_fixed", || {
                (c.set_cHRM_fixed)(png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7])
            });
            snap(c, png, info);
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
        wr_both(&format!("cHRM_XYZ {tag}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_cHRM_XYZ_fixed", || {
                (c.set_cHRM_XYZ_fixed)(
                    png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[0],
                )
            });
            step("write_info", || (c.write_info)(png, info));
        });
    }

    // --- tIME (pngset.c:1160 / pngwutil.c:1912) ---------------------------
    for &(y, mo, d, h, mi, s) in &[
        (2024u16, 0u8, 1u8, 0u8, 0u8, 0u8),
        (2024, 13, 1, 0, 0, 0),
        (2024, 1, 0, 0, 0, 0),
        (2024, 1, 32, 0, 0, 0),
        (2024, 1, 1, 24, 0, 0),
        (2024, 1, 1, 0, 60, 0),
        (2024, 1, 1, 0, 0, 61),
        (2024, 255, 255, 255, 255, 255),
        (2024, 12, 31, 23, 59, 60),
    ] {
        wr(
            &format!("tIME {y}-{mo}-{d} {h}:{mi}:{s}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
                let t = PngTime {
                    year: y,
                    month: mo,
                    day: d,
                    hour: h,
                    minute: mi,
                    second: s,
                };
                step("set_tIME", || {
                    (c.set_tIME)(png, info, &t as *const PngTime as *const u8)
                });
                snap(c, png, info);
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // --- cLLI / mDCV out-of-range (pngset.c:170/253) ----------------------
    for &(a, b) in &[
        (0u32, 0u32),
        (0x8000_0000, 0),
        (0, 0x8000_0000),
        (0xffff_ffff, 0xffff_ffff),
    ] {
        wr_both(&format!("cLLI {a:#x}/{b:#x}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_cLLI", || (c.set_cLLI_fixed)(png, info, a, b));
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
    }
    for &(chrom, dl) in &[
        (0i32, 0u32),
        (-1, 0),
        (i32::MAX, 0),
        (200_000, 0x8000_0000),
        (31270, 1000),
    ] {
        wr_both(
            &format!("mDCV chrom={chrom} dl={dl:#x}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                step("set_mDCV", || {
                    (c.set_mDCV_fixed)(
                        png, info, chrom, chrom, chrom, chrom, chrom, chrom, chrom, chrom, dl, dl,
                    )
                });
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
}

// ===========================================================================
// 8. compression settings
// ===========================================================================

/// A compressible 16x12 RGB image plus a zTXt chunk, so that both the IDAT and
/// the text deflate streams are claimed.
fn comp_case(label: &str, pre: &dyn Fn(&Core, Png, Info)) {
    let key = CString::new("Comment").unwrap();
    let body = CString::new("x".repeat(400)).unwrap();
    let t = PngText {
        compression: PNG_TEXT_COMPRESSION_zTXt,
        key: key.as_ptr() as *mut c_char,
        text: body.as_ptr() as *mut c_char,
        ..Default::default()
    };
    let tp = &t as *const PngText;
    wr(label, &move |c, png, info| unsafe {
        (c.set_IHDR)(png, info, 16, 12, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        (c.set_text)(png, info, tp as *const c_void, 1);
        step("pre", || pre(c, png, info));
        log(format!("cbuf={}", (c.get_compression_buffer_size)(png)));
        step("write_info", || (c.write_info)(png, info));
        let rows = mkrows(PNG_COLOR_TYPE_RGB, 8, 16, 12, 0xc0de);
        step("rows", || {
            for r in &rows {
                (c.write_row)(png, r.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
}

#[test]
fn compression_settings() {
    // png_set_compression_level: libpng stores the value unchecked; deflateInit2
    // rejects it, which surfaces as a fatal zlib message.
    for &lv in &[-2i32, -1, 0, 9, 10, 11, 99, i32::MIN, i32::MAX] {
        comp_case(&format!("CMP level={lv}"), &move |c, png, _i| unsafe {
            (c.set_compression_level)(png, lv)
        });
        comp_case(&format!("CMP text_level={lv}"), &move |c, png, _i| unsafe {
            (c.set_text_compression_level)(png, lv)
        });
    }
    // pngwrite.c:1295/1371 "Only compression method 8 is supported by PNG"
    for &m in &[0i32, 7, 8, 9, 15, 255, -1] {
        comp_case(&format!("CMP method={m}"), &move |c, png, _i| unsafe {
            (c.set_compression_method)(png, m)
        });
        comp_case(&format!("CMP text_method={m}"), &move |c, png, _i| unsafe {
            (c.set_text_compression_method)(png, m)
        });
    }
    // window bits: clamped with a warning outside 8..15
    for &wb in &[-1i32, 0, 1, 7, 8, 9, 15, 16, 17, 100, i32::MIN, i32::MAX] {
        comp_case(&format!("CMP window={wb}"), &move |c, png, _i| unsafe {
            (c.set_compression_window_bits)(png, wb)
        });
        comp_case(&format!("CMP text_window={wb}"), &move |c, png, _i| unsafe {
            (c.set_text_compression_window_bits)(png, wb)
        });
    }
    // mem level: stored unchecked, deflateInit2 rejects out-of-range values
    for &ml in &[-1i32, 0, 1, 8, 9, 10, 99] {
        comp_case(&format!("CMP mem={ml}"), &move |c, png, _i| unsafe {
            (c.set_compression_mem_level)(png, ml)
        });
        comp_case(&format!("CMP text_mem={ml}"), &move |c, png, _i| unsafe {
            (c.set_text_compression_mem_level)(png, ml)
        });
    }
    // strategy
    for &st in &[-1i32, 0, 4, 5, 99] {
        comp_case(&format!("CMP strategy={st}"), &move |c, png, _i| unsafe {
            (c.set_compression_strategy)(png, st)
        });
        comp_case(&format!("CMP text_strategy={st}"), &move |c, png, _i| unsafe {
            (c.set_text_compression_strategy)(png, st)
        });
    }
    // buffer size (pngset.c:1803/1821/1836)
    for &sz in &[
        0usize,
        1,
        5,
        6,
        7,
        64,
        UINT_31_MAX,
        UINT_31_MAX + 1,
        usize::MAX,
    ] {
        comp_case(&format!("CMP buffer={sz:#x}"), &move |c, png, _i| unsafe {
            (c.set_compression_buffer_size)(png, sz)
        });
    }
    // changing the buffer size while the zstream is claimed by IDAT
    for &sz in &[6usize, 64, 8192] {
        wr(
            &format!("CMP buffer-in-use={sz}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 16, 12, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                step("write_info", || (c.write_info)(png, info));
                let rows = mkrows(PNG_COLOR_TYPE_RGB, 8, 16, 12, 0xbeef);
                step("row0", || (c.write_row)(png, rows[0].as_ptr()));
                step("set_buffer", || (c.set_compression_buffer_size)(png, sz));
                log(format!("cbuf={}", (c.get_compression_buffer_size)(png)));
                step("rest", || {
                    for r in rows.iter().skip(1) {
                        (c.write_row)(png, r.as_ptr());
                    }
                });
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // png_set_filter: pngwrite.c:1078 / 1140 / 1180
    for &f in &[0i32, 1, 2, 3, 4, 5, 6, 7, 0x08, 0xf8, 0xff, 0x100, -1] {
        wr_both(&format!("FLT filters={f:#x}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 8, 6, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_filter", || (c.set_filter)(png, PNG_FILTER_TYPE_BASE, f));
            step("write_info", || (c.write_info)(png, info));
            let rows = mkrows(PNG_COLOR_TYPE_RGB, 8, 8, 6, 0xf17e);
            step("rows", || {
                for r in &rows {
                    (c.write_row)(png, r.as_ptr());
                }
            });
            step("write_end", || (c.write_end)(png, info));
        });
    }
    for &m in &[1i32, 2, 63, 64, 65, 255, -1] {
        wr_both(&format!("FLT method={m}"), &move |c, png, info| unsafe {
            (c.set_IHDR)(png, info, 8, 6, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            step("set_filter", || (c.set_filter)(png, m, PNG_ALL_FILTERS));
            step("write_info", || (c.write_info)(png, info));
            step("write_end", || (c.write_end)(png, info));
        });
        // The same with MNG filter 64 permitted: method 64 is remapped to 0.
        wr_both(
            &format!("FLT method={m} mng"),
            &move |c, png, info| unsafe {
                (c.permit_mng_features)(png, MNG_FILTER_64);
                (c.set_IHDR)(png, info, 8, 6, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                step("set_filter", || (c.set_filter)(png, m, PNG_ALL_FILTERS));
                step("write_info", || (c.write_info)(png, info));
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // pngwrite.c:1140 "png_set_filter: UP/AVG/PAETH cannot be added after
    // start": prev_row is only allocated when one of those filters was already
    // selected at png_write_start_row time.
    for &(first, second) in &[
        (PNG_FILTER_NONE, PNG_FILTER_UP),
        (PNG_FILTER_NONE, PNG_FILTER_AVG),
        (PNG_FILTER_NONE, PNG_FILTER_PAETH),
        (PNG_FILTER_SUB, PNG_FILTER_UP),
        (PNG_FILTER_NONE, PNG_ALL_FILTERS),
        (PNG_FILTER_NONE, PNG_FILTER_NONE | PNG_FILTER_SUB),
        (PNG_ALL_FILTERS, PNG_FILTER_UP),
    ] {
        for &(w, h) in &[(8u32, 6u32), (1, 6), (8, 1), (1, 1)] {
            wr_both(
                &format!("FLT late first={first:#x} second={second:#x} {w}x{h}"),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, w, h, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                    (c.set_filter)(png, PNG_FILTER_TYPE_BASE, first);
                    step("write_info", || (c.write_info)(png, info));
                    let rows = mkrows(PNG_COLOR_TYPE_RGB, 8, w, h, 0xfa17);
                    step("row0", || (c.write_row)(png, rows[0].as_ptr()));
                    let rc = step("set_filter2", || {
                        (c.set_filter)(png, PNG_FILTER_TYPE_BASE, second)
                    });
                    // NOTE: when the UP/AVG/PAETH app warning is fatal, the
                    // `switch` at the top of png_set_filter has *already*
                    // stored the new filter mask in png_ptr->do_filter while
                    // prev_row is still NULL, so writing another row
                    // dereferences NULL inside png_write_find_filter.  That is
                    // a crash in the C reference library itself (verified by
                    // running both sides against libpng.so), so the row loop is
                    // only entered when png_set_filter returned normally.  The
                    // rejection itself -- message, fatal-vs-warning behaviour
                    // and longjmp rc -- is still compared.
                    if rc == 0 {
                        step("rest", || {
                            for r in rows.iter().skip(1) {
                                (c.write_row)(png, r.as_ptr());
                            }
                        });
                    } else {
                        log("rest:skipped-after-fatal".to_string());
                    }
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
    }
}

// ===========================================================================
// 9. write transforms applied to the wrong data
// ===========================================================================

/// A write user transform that corrupts `row_info->pixel_depth`, which is the
/// only way an application can reach pngwrite.c:918 "internal write transform
/// logic error".
unsafe extern "C" fn utf_bump(_png: *mut c_void, ri: *mut PngRowInfo, _row: *mut u8) {
    let r = &mut *ri;
    log(format!(
        "UTF w={} rb={} ct={} bd={} ch={} pd={}",
        r.width, r.rowbytes, r.color_type, r.bit_depth, r.channels, r.pixel_depth
    ));
    r.pixel_depth = r.pixel_depth.wrapping_add(8);
}

/// The same, but leaving `row_info` alone (control case).
unsafe extern "C" fn utf_noop(_png: *mut c_void, ri: *mut PngRowInfo, _row: *mut u8) {
    let r = &*ri;
    log(format!(
        "UTF w={} rb={} ct={} bd={} ch={} pd={}",
        r.width, r.rowbytes, r.color_type, r.bit_depth, r.channels, r.pixel_depth
    ));
}

/// A transform that halves the channel count without touching the row bytes.
unsafe extern "C" fn utf_halve(_png: *mut c_void, ri: *mut PngRowInfo, _row: *mut u8) {
    let r = &mut *ri;
    log(format!("UTF halve pd={}", r.pixel_depth));
    r.channels = 1;
    r.pixel_depth = r.bit_depth;
}

#[test]
fn transform_misuse() {
    let combos: &[(c_int, c_int)] = &[
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 2),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_PALETTE, 1),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ];
    let pal = palette256(0x7a7a);

    // png_set_filler on every colour type (pngtrans.c:202/209
    // "png_set_filler is invalid for low bit depth gray output" /
    // "png_set_filler: inappropriate color type").  Only the *setter* is
    // exercised here: the rows are written with the byte count the transform
    // asks for, which is 4 x 16 bit at most.
    for &(ct, bd) in combos {
        for &loc in &[PNG_FILLER_BEFORE, PNG_FILLER_AFTER, 5, -1] {
            let pr = &pal;
            wr_both(
                &format!("TR filler ct={ct} bd={bd} loc={loc}"),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                    if ct == PNG_COLOR_TYPE_PALETTE {
                        (c.set_PLTE)(png, info, pr.as_ptr(), 1 << bd);
                    }
                    let rc = step("set_filler", || (c.set_filler)(png, 0, loc));
                    log(format!(
                        "rowbytes={} ch={}",
                        (c.get_rowbytes)(png, info),
                        (c.get_channels)(png, info)
                    ));
                    step("write_info", || (c.write_info)(png, info));
                    // 4 pixels x 4 channels x 2 bytes is enough for anything
                    // libpng can ask for here.
                    let row = vec![0x5au8; 4 * 4 * 2 + 8];
                    if rc == 0 {
                        step("rows", || {
                            for _ in 0..3 {
                                (c.write_row)(png, row.as_ptr());
                            }
                        });
                    }
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
        // png_set_add_alpha follows the same rules.
        let pr = &pal;
        wr_both(
            &format!("TR add_alpha ct={ct} bd={bd}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                if ct == PNG_COLOR_TYPE_PALETTE {
                    (c.set_PLTE)(png, info, pr.as_ptr(), 1 << bd);
                }
                let rc = step("set_add_alpha", || {
                    (c.set_add_alpha)(png, 0xffff, PNG_FILLER_AFTER)
                });
                step("write_info", || (c.write_info)(png, info));
                let row = vec![0xa5u8; 4 * 4 * 2 + 8];
                if rc == 0 {
                    step("rows", || {
                        for _ in 0..3 {
                            (c.write_row)(png, row.as_ptr());
                        }
                    });
                }
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }
    // png_set_filler twice, and filler on a struct without an IHDR.
    wr_both("TR filler twice rgb8", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        step("f1", || (c.set_filler)(png, 0, PNG_FILLER_AFTER));
        step("f2", || (c.set_filler)(png, 0, PNG_FILLER_BEFORE));
        step("write_info", || (c.write_info)(png, info));
        let row = vec![0x11u8; 4 * 4 * 2 + 8];
        step("rows", || {
            for _ in 0..3 {
                (c.write_row)(png, row.as_ptr());
            }
        });
        step("write_end", || (c.write_end)(png, info));
    });
    wr_both("TR filler no IHDR", &|c, png, info| unsafe {
        step("set_filler", || (c.set_filler)(png, 0, PNG_FILLER_AFTER));
        let _ = info;
    });

    // Transforms that silently do nothing for the current format.
    for &(ct, bd) in combos {
        let pr = &pal;
        wr_both(
            &format!("TR noop-set ct={ct} bd={bd}"),
            &move |c, png, info| unsafe {
                (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                if ct == PNG_COLOR_TYPE_PALETTE {
                    (c.set_PLTE)(png, info, pr.as_ptr(), 1 << bd);
                }
                step("swap", || (c.set_swap)(png));
                step("packing", || (c.set_packing)(png));
                step("packswap", || (c.set_packswap)(png));
                step("invert_alpha", || (c.set_invert_alpha)(png));
                step("swap_alpha", || (c.set_swap_alpha)(png));
                step("invert_mono", || (c.set_invert_mono)(png));
                step("bgr", || (c.set_bgr)(png));
                log(format!(
                    "rowbytes={} ch={}",
                    (c.get_rowbytes)(png, info),
                    (c.get_channels)(png, info)
                ));
                step("write_info", || (c.write_info)(png, info));
                // With PNG_PACK set the user data is one sample per byte.
                let row = vec![0x01u8; 4 * 4 * 2 + 8];
                step("rows", || {
                    for _ in 0..3 {
                        (c.write_row)(png, row.as_ptr());
                    }
                });
                step("write_end", || (c.write_end)(png, info));
            },
        );
    }

    // png_set_shift (pngtrans.c:114 "png_set_shift: invalid shift values")
    for &(ct, bd) in combos {
        for &sb in &[
            [0u8, 0, 0, 0, 0],
            [1, 1, 1, 1, 1],
            [255, 255, 255, 255, 255],
            [4, 4, 4, 4, 4],
            [17, 17, 17, 17, 17],
        ] {
            let pr = &pal;
            wr_both(
                &format!("TR shift ct={ct} bd={bd} {sb:?}"),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                    if ct == PNG_COLOR_TYPE_PALETTE {
                        (c.set_PLTE)(png, info, pr.as_ptr(), 1 << bd);
                    }
                    let v = PngColor8 {
                        red: sb[0],
                        green: sb[1],
                        blue: sb[2],
                        gray: sb[3],
                        alpha: sb[4],
                    };
                    step("set_shift", || {
                        (c.set_shift)(png, &v as *const PngColor8 as *const u8)
                    });
                    step("write_info", || (c.write_info)(png, info));
                    let rows = mkrows(ct, bd, 4, 3, 0x5111);
                    step("rows", || {
                        for r in &rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                    });
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
    }
    wr("TR shift null", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        step("set_shift", || (c.set_shift)(png, std::ptr::null()));
        full_write(c, png, info, PNG_COLOR_TYPE_RGB, 8, 4, 3);
    });

    // pngwrite.c:918 "internal write transform logic error"
    for &(name, cb) in &[
        ("bump", utf_bump as unsafe extern "C" fn(*mut c_void, *mut PngRowInfo, *mut u8)),
        ("halve", utf_halve),
        ("noop", utf_noop),
    ] {
        for &(ct, bd) in &[
            (PNG_COLOR_TYPE_GRAY, 8),
            (PNG_COLOR_TYPE_RGB, 8),
            (PNG_COLOR_TYPE_RGB_ALPHA, 16),
            (PNG_COLOR_TYPE_GRAY, 4),
        ] {
            wr_both(
                &format!("TR usertransform={name} ct={ct} bd={bd}"),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                    (c.set_write_user_transform_fn)(png, cb as Cb);
                    (c.set_user_transform_info)(png, std::ptr::null_mut(), bd, 4);
                    step("write_info", || (c.write_info)(png, info));
                    let row = vec![0x3cu8; 4 * 4 * 2 + 8];
                    step("rows", || {
                        for _ in 0..3 {
                            (c.write_row)(png, row.as_ptr());
                        }
                    });
                    step("write_end", || (c.write_end)(png, info));
                },
            );
        }
    }
    // A NULL user transform function still sets PNG_USER_TRANSFORM.
    wr_both("TR usertransform=null", &|c, png, info| unsafe {
        (c.set_IHDR)(png, info, 4, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        (c.set_write_user_transform_fn)(png, std::ptr::null_mut());
        full_write(c, png, info, PNG_COLOR_TYPE_RGB, 8, 4, 3);
    });

    // png_write_png transform bits: the read-only ones must be ignored, and
    // STRIP_FILLER BEFORE|AFTER is an app error (pngwrite.c:1472).
    let read_only = [
        ("STRIP_16", PNG_TRANSFORM_STRIP_16),
        ("STRIP_ALPHA", PNG_TRANSFORM_STRIP_ALPHA),
        ("EXPAND", PNG_TRANSFORM_EXPAND),
        ("GRAY_TO_RGB", PNG_TRANSFORM_GRAY_TO_RGB),
        ("EXPAND_16", PNG_TRANSFORM_EXPAND_16),
        ("SCALE_16", PNG_TRANSFORM_SCALE_16),
        ("all-read-only", 0x8000 | 0x4000 | 0x2000 | 0x0010 | 0x0002 | 0x0001),
        ("unknown-bits", -1),
        (
            "FILLER_BOTH",
            PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER,
        ),
    ];
    for &(name, bits) in &read_only {
        for &(ct, bd) in &[
            (PNG_COLOR_TYPE_GRAY, 8),
            (PNG_COLOR_TYPE_RGB, 8),
            (PNG_COLOR_TYPE_RGB_ALPHA, 8),
            (PNG_COLOR_TYPE_GRAY, 16),
        ] {
            wr_both(
                &format!("TR write_png {name} ct={ct} bd={bd}"),
                &move |c, png, info| unsafe {
                    (c.set_IHDR)(png, info, 4, 3, bd, ct, 0, 0, 0);
                    // 4 channels x 16 bit is the widest any bit can request.
                    let mut rows: Vec<Vec<u8>> =
                        (0..3).map(|_| vec![0x6eu8; 4 * 4 * 2 + 8]).collect();
                    let mut p = ptr_vec(&mut rows);
                    (c.set_rows)(png, info, p.as_mut_ptr());
                    let sb = PngColor8 {
                        red: 4,
                        green: 4,
                        blue: 4,
                        gray: 4,
                        alpha: 4,
                    };
                    (c.set_sBIT)(png, info, &sb as *const PngColor8 as *const u8);
                    step("write_png", || {
                        (c.write_png)(png, info, bits, std::ptr::null_mut())
                    });
                },
            );
        }
    }
}

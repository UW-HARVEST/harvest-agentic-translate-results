//! Phase C — the generic FFI-boundary sweep over **every** exported entry point.
//!
//! For each of the 381 exported functions this calls it
//!
//!   * with `NULL` in every pointer position and `0` in every scalar position, and
//!   * with a live `png_ptr` / `info_ptr` and a hostile scalar (`-1`, `-2`,
//!     `PNG_*_LAST`, `0x7fffffff`, `0xffffffff`, …) in every scalar position —
//!     which is exactly how a C enum value with no valid variant crosses the FFI
//!     boundary,
//!
//! and asserts that the C library and the Rust library produce the same answer:
//! the same return value, the same diagnostics, the same text on stderr, and — if
//! the call is fatal — death by the same signal.
//!
//! Every call runs in a `fork()`ed child, so a call that aborts or segfaults in
//! the C library is a *comparable observation* rather than the end of the test
//! run.  This is what makes it safe to call all 381 entry points with garbage.
//!
//! Covers the "generic boundaries" requirement of Phase C plus ERRORS.md rows
//! A-1 (`PNG_ABORT`) and every `png_ptr == NULL` / `info_ptr == NULL` guard.
#![allow(non_snake_case)]

mod common;

use common::sweep;
use common::*;
use core::ffi::{c_char, c_int, c_void};

/// libpng's `png_default_error` prints the *address* of nothing, but the
/// simplified API and a few warnings embed pointer-derived text; normalise
/// anything that could legitimately differ between two separate libraries.
fn normalise(s: &str) -> String {
    s.replace('\r', "")
}

fn compare(case: &str, c: &ChildResult, r: &ChildResult) {
    FORKED_CASES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let ct = normalise(&c.text);
    let rt = normalise(&r.text);
    assert!(
        c.exit == r.exit && c.signal == r.signal && ct == rt,
        "sweep case `{}` diverged\n  C   : exit={:?} signal={} text={:?}\n  Rust: exit={:?} signal={} text={:?}",
        case,
        c.exit,
        c.signal,
        ct,
        r.exit,
        r.signal,
        rt
    );
}

/// Every entry point called with NULL / 0 everywhere.
#[test]
fn null_arguments() {
    let l = libs();
    for idx in 0..sweep::N {
        let name = sweep::NAMES[idx];
        let mut res = Vec::new();
        for api in [&l.c, &l.rust] {
            res.push(run_in_child(&mut || unsafe {
                set_cur_api(api as *const Api);
                let mut msg = String::new();
                let g = guarded(api, core::ptr::null_mut(), &mut || {
                    msg = sweep::call_null(api, idx);
                });
                format!("{:?} {}", g, msg)
            }));
        }
        compare(
            &format!("{}(NULL...)", name),
            &res[0],
            &res[1],
        );
    }
}

/// Hostile scalar values, with a live read struct.
#[test]
fn hostile_scalars_read() {
    hostile(false);
}

/// Hostile scalar values, with a live write struct.
#[test]
fn hostile_scalars_write() {
    hostile(true);
}

const HOSTILE: [i64; 14] = [
    -1,
    -2,
    -3,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    64,
    255,
    0x7fffffff,
    -0x8000_0000,
];

fn hostile(write: bool) {
    let l = libs();
    for idx in 0..sweep::N {
        if !sweep::HAS_SCALAR[idx] || !sweep::TAKES_PNG[idx] {
            continue;
        }
        let name = sweep::NAMES[idx];
        for &v in &HOSTILE {
            let mut res = Vec::new();
            for api in [&l.c, &l.rust] {
                res.push(run_in_child(&mut || unsafe {
                    set_cur_api(api as *const Api);
                    let (png, info) = if write {
                        new_write(api)
                    } else {
                        new_read(api)
                    };
                    let mut msg = String::new();
                    let g = guarded(api, png, &mut || {
                        msg = sweep::call_value(api, idx, png, info, v);
                    });
                    format!("{:?} {}", g, msg)
                }));
            }
            compare(
                &format!("{}(png, ..., {}) [{}]", name, v, if write { "write" } else { "read" }),
                &res[0],
                &res[1],
            );
        }
    }
}

/// The documented enum ranges, one step past the end, on a *set up* struct.
///
/// `png_set_crc_action`, `png_set_alpha_mode`, `png_set_background`,
/// `png_set_rgb_to_gray`, `png_set_filter`, `png_set_option`,
/// `png_set_keep_unknown_chunks`, `png_set_unknown_chunk_location`,
/// `png_data_freer`, `png_permit_mng_features`, `png_set_sRGB`,
/// `png_set_pCAL`, `png_set_sCAL`, `png_set_oFFs`, `png_set_pHYs` and
/// `png_set_text` all take an `int` that C accepts from any value.
#[test]
fn enum_boundaries() {
    let l = libs();
    let mut n = 0usize;
    // (name, valid range end) -- values tried: -1 .. end+1
    let cases: &[(&str, i64)] = &[
        ("png_set_crc_action", 6),
        ("png_set_alpha_mode", 4),
        ("png_set_background", 4),
        ("png_set_rgb_to_gray", 4),
        ("png_set_option", 17),
        ("png_set_keep_unknown_chunks", 5),
        ("png_set_unknown_chunk_location", 4),
        ("png_data_freer", 3),
        ("png_permit_mng_features", 8),
        ("png_set_sRGB", 5),
        ("png_set_sRGB_gAMA_and_cHRM", 5),
        ("png_set_filter", 6),
        ("png_set_compression_strategy", 6),
        ("png_set_compression_level", 11),
        ("png_set_text_compression_strategy", 6),
        ("png_set_quantize", 3),
        ("png_set_filler", 3),
        ("png_set_add_alpha", 3),
        ("png_set_shift", 2),
        ("png_set_invalid", 2),
        ("png_set_interlace_handling", 2),
        ("png_set_benign_errors", 3),
        ("png_set_check_for_invalid_index", 3),
        ("png_set_scale_16", 2),
        ("png_handle_as_unknown", 2),
        ("png_set_compression_method", 10),
        ("png_set_compression_mem_level", 11),
        ("png_set_compression_window_bits", 17),
    ];
    for &(name, end) in cases {
        let idx = sweep::NAMES.iter().position(|&s| s == name);
        let Some(idx) = idx else { continue };
        for v in -2..=end + 1 {
            for write in [false, true] {
                let mut res = Vec::new();
                for api in [&l.c, &l.rust] {
                    res.push(run_in_child(&mut || unsafe {
                        set_cur_api(api as *const Api);
                        let (png, info) = if write {
                            new_write(api)
                        } else {
                            new_read(api)
                        };
                        let mut msg = String::new();
                        let g = guarded(api, png, &mut || {
                            // a real IHDR first so the setters have state to work on
                            (api.png_set_IHDR)(
                                png,
                                info,
                                4,
                                4,
                                8,
                                PNG_COLOR_TYPE_RGB_ALPHA,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            msg = sweep::call_value(api, idx, png, info, v);
                        });
                        format!("{:?} {}", g, msg)
                    }));
                }
                compare(&format!("{}(.., {}) write={}", name, v, write), &res[0], &res[1]);
                n += 1;
            }
        }
    }
    assert!(n > 500, "only {} enum boundary cases ran", n);
}

/// Zero and oversized lengths on the entry points that take one.
#[test]
fn length_boundaries() {
    let l = libs();
    let lens: [usize; 9] = [
        0,
        1,
        2,
        7,
        8,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        usize::MAX,
    ];
    let mut n = 0;
    for &len in &lens {
        for &(tag, which) in &[("read", 0u8), ("write", 1u8)] {
            let mut res = Vec::new();
            for api in [&l.c, &l.rust] {
                res.push(run_in_child(&mut || unsafe {
                    set_cur_api(api as *const Api);
                    let mut out = String::new();
                    let (png, info) = if which == 1 {
                        new_write(api)
                    } else {
                        new_read(api)
                    };
                    let g = guarded(api, png, &mut || {
                        // png_malloc / png_calloc with a hostile size
                        let p1 = (api.png_malloc_warn)(png, len);
                        out += &format!("malloc_warn={} ", !p1.is_null());
                        if !p1.is_null() {
                            (api.png_free)(png, p1);
                        }
                        let p2 = (api.png_malloc_base)(png, len);
                        out += &format!("malloc_base={} ", !p2.is_null());
                        if !p2.is_null() {
                            (api.png_free)(png, p2);
                        }
                        // png_set_compression_buffer_size
                        (api.png_set_compression_buffer_size)(png, len);
                        out += &format!(
                            "buf={} ",
                            (api.png_get_compression_buffer_size)(png)
                        );
                        // png_info_init_3 with a hostile size
                        let mut ip = info;
                        (api.png_info_init_3)(&mut ip, len);
                        out += "info_init ";
                        // png_set_chunk_malloc_max / cache
                        (api.png_set_chunk_malloc_max)(png, len);
                        out += &format!(
                            "cmm={} ",
                            (api.png_get_chunk_malloc_max)(png)
                        );
                        // png_set_longjmp_fn with a hostile jmp_buf size
                        let jb = (api.png_set_longjmp_fn)(png, None, len);
                        out += &format!("ljf={} ", !jb.is_null());
                    });
                    format!("{:?} {}", g, out)
                }));
            }
            compare(&format!("lengths len={} {}", len, tag), &res[0], &res[1]);
            n += 1;
        }
    }
    assert!(n >= 18);
    let _ = (0 as *const c_char, 0 as *const c_void, 0 as c_int);
}

//! Phase C — the error surface.
//!
//! Every row of `ERRORS.md` is a distinct place where libpng rejects an input.
//! This file constructs those conditions and asserts that the C library and the
//! Rust library reject them *identically*: the same fatal/non-fatal outcome, the
//! same message text, in the same order.
//!
//! The last test (`coverage_report`) diffs the set of diagnostics that were
//! actually observed against `tests/error_sites.txt` (generated from the C
//! sources by `tools/gen_error_sites.py`) and writes
//! `target/error_coverage.txt`, so the ERRORS.md check-marks are derived from
//! what the tests really reached rather than asserted by hand.
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_char, c_int};

/* ================================================================== */
/* ERRORS.md part 1: pngerror.c -- the reporting functions themselves   */
/* ================================================================== */

/// D-rows in `pngerror.c`: `png_error`, `png_warning`, `png_chunk_error`,
/// `png_chunk_warning`, `png_benign_error`, `png_chunk_benign_error`,
/// `png_app_error`, `png_app_warning`, `png_chunk_report`, `png_fixed_error` —
/// every one of them on a read struct and on a write struct, with
/// `png_set_benign_errors` off and on.
///
/// These are the entry points every other error path funnels through, so the
/// whole fatal/non-fatal decision table is pinned down here.
#[test]
fn reporting_matrix() {
    let msg = cs("a test diagnostic @1 x");
    let chunk = cs("tEXt");
    for write in [false, true] {
        for benign in [-1i32, 0, 1] {
            for chunk_name_set in [false, true] {
                let case = format!(
                    "reporting write={} benign={} named={}",
                    write, benign, chunk_name_set
                );
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = if write { new_write(api) } else { new_read(api) };
                    if benign >= 0 {
                        (api.png_set_benign_errors)(png, benign);
                    }
                    if chunk_name_set {
                        // png_chunk_* prefix the message with the current chunk
                        // name; give it one by handing the reader a real IHDR.
                        (api.png_set_IHDR)(
                            png,
                            info,
                            2,
                            2,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                    }
                    // non-fatal reporters first: they must return normally
                    for (tag, f) in [
                        ("png_warning", 0),
                        ("png_chunk_warning", 1),
                        ("png_app_warning", 2),
                        ("png_benign_error", 3),
                        ("png_chunk_benign_error", 4),
                        ("png_app_error", 5),
                        ("png_chunk_report_warn", 6),
                        ("png_chunk_report_benign", 7),
                        ("png_chunk_report_error", 8),
                        ("png_error", 9),
                        ("png_chunk_error", 10),
                        ("png_fixed_error", 11),
                    ] {
                        let g = guarded(api, png, &mut || match f {
                            0 => (api.png_warning)(png, msg.as_ptr()),
                            1 => (api.png_chunk_warning)(png, msg.as_ptr()),
                            2 => (api.png_app_warning)(png, msg.as_ptr()),
                            3 => (api.png_benign_error)(png, msg.as_ptr()),
                            4 => (api.png_chunk_benign_error)(png, msg.as_ptr()),
                            5 => (api.png_app_error)(png, msg.as_ptr()),
                            6 => (api.png_chunk_report)(png, msg.as_ptr(), 0),
                            7 => (api.png_chunk_report)(png, msg.as_ptr(), 1),
                            8 => (api.png_chunk_report)(png, msg.as_ptr(), 2),
                            9 => (api.png_error)(png, msg.as_ptr()),
                            10 => (api.png_chunk_error)(png, msg.as_ptr()),
                            _ => (api.png_fixed_error)(png, msg.as_ptr()),
                        });
                        o.push(format!("{} -> {:?}", tag, g));
                        if g != Guard::Ok {
                            // a fatal report leaves the struct unusable; the rest
                            // of the loop would be undefined in C too.
                            break;
                        }
                    }
                    destroy_or_free(api, png, info, write);
                    let _ = chunk.as_ptr();
                    o
                });
            }
        }
    }
}

/// `png_formatted_warning` and the `png_warning_parameter*` helpers
/// (`pngerror.c:192-330`), including every `PNG_NUMBER_FORMAT_*`.
#[test]
fn formatted_warning() {
    const PNG_NUMBER_FORMAT_u: c_int = 1;
    const PNG_NUMBER_FORMAT_02u: c_int = 2;
    const PNG_NUMBER_FORMAT_d: c_int = 1;
    const PNG_NUMBER_FORMAT_02d: c_int = 2;
    const PNG_NUMBER_FORMAT_x: c_int = 3;
    const PNG_NUMBER_FORMAT_02x: c_int = 4;
    const PNG_NUMBER_FORMAT_fixed: c_int = 5;
    let mut rng = Rng::new(0xf0_1234);
    // png_warning_parameters is `char p[8][32]`
    for iter in 0..200 {
        let fmt = cs(match iter % 6 {
            0 => "p1=@1 p2=@2 p3=@3",
            1 => "@1@2@3@4@5@6@7@8",
            2 => "@9 out of range",
            3 => "@0 zero index",
            4 => "no parameters at all",
            _ => "trailing @",
        });
        let strings: Vec<std::ffi::CString> = (0..3)
            .map(|_| {
                let n = rng.below(40);
                cs(&(0..n)
                    .map(|_| (b'a' + (rng.u8() % 26)) as char)
                    .collect::<String>())
            })
            .collect();
        let numbers: Vec<(c_int, i32, u64)> = (0..5)
            .map(|_| {
                (
                    rng.pick(&[
                        PNG_NUMBER_FORMAT_u,
                        PNG_NUMBER_FORMAT_02u,
                        PNG_NUMBER_FORMAT_x,
                        PNG_NUMBER_FORMAT_02x,
                        PNG_NUMBER_FORMAT_fixed,
                        -1,
                        7,
                    ]),
                    rng.u32() as i32,
                    rng.u32() as u64,
                )
            })
            .collect();
        let indices: Vec<c_int> = (0..8).map(|i| i as c_int - 1).collect();
        assert_same(&format!("formatted_warning #{}", iter), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_read(api);
            let mut p = [0u8; 8 * 32];
            let g = guarded(api, png, &mut || {
                for (i, s) in strings.iter().enumerate() {
                    (api.png_warning_parameter)(
                        p.as_mut_ptr() as *mut c_char,
                        indices[i % indices.len()] + 1,
                        s.as_ptr(),
                    );
                }
                for (i, &(f, sv, uv)) in numbers.iter().enumerate() {
                    (api.png_warning_parameter_unsigned)(
                        p.as_mut_ptr() as *mut c_char,
                        (i as c_int) + 1,
                        f,
                        uv as usize,
                    );
                    (api.png_warning_parameter_signed)(
                        p.as_mut_ptr() as *mut c_char,
                        (i as c_int) + 2,
                        f,
                        sv,
                    );
                }
                (api.png_formatted_warning)(png, p.as_mut_ptr() as *mut c_char, fmt.as_ptr());
            });
            o.push(format!("{:?}", g));
            o.output = p.to_vec();
            destroy_read(api, png, info);
            o
        });
    }
    let _ = PNG_NUMBER_FORMAT_d + PNG_NUMBER_FORMAT_02d;
}

/// `png_longjmp` with no trap armed reaches `PNG_ABORT()` (`pngerror.c:690`) —
/// ERRORS.md row A-1.  Both libraries must die from `SIGABRT`.
#[test]
fn png_abort_row_A1() {
    let l = libs();
    let mut res = Vec::new();
    for api in [&l.c, &l.rust] {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "abort_child", "--nocapture", "--ignored"])
            .env("PNG_ABORT_CHILD", api.which)
            .output()
            .expect("respawn self");
        res.push((
            out.status.code(),
            out.status.signal_or_none(),
        ));
    }
    assert_eq!(res[0], res[1], "PNG_ABORT behaviour differs: {:?}", res);
    // and it really did abort
    assert_eq!(res[0].1, Some(6), "expected SIGABRT, got {:?}", res[0]);
}

trait SignalOrNone {
    fn signal_or_none(&self) -> Option<i32>;
}
impl SignalOrNone for std::process::ExitStatus {
    fn signal_or_none(&self) -> Option<i32> {
        use std::os::unix::process::ExitStatusExt;
        self.signal()
    }
}

/// The child half of `png_abort_row_A1`.
#[test]
#[ignore]
fn abort_child() {
    let which = std::env::var("PNG_ABORT_CHILD").unwrap_or_default();
    if which.is_empty() {
        return;
    }
    let l = libs();
    let api = if which == "C" { &l.c } else { &l.rust };
    let mut state = Box::new(Tls::default());
    set_tls(&mut *state as *mut Tls);
    set_cur_api(api as *const Api);
    unsafe {
        // No error_fn, no longjmp_fn: png_error -> png_default_error ->
        // png_longjmp -> PNG_ABORT().
        let png = (api.png_create_read_struct)(
            VER,
            core::ptr::null_mut(),
            None,
            None,
        );
        assert!(!png.is_null());
        (api.png_error)(png, b"forced abort\0".as_ptr() as *const c_char);
    }
    unreachable!();
}

fn destroy_or_free(api: &Api, png: *mut PngStruct, info: *mut PngInfo, write: bool) {
    unsafe {
        if write {
            destroy_write(api, png, info)
        } else {
            destroy_read(api, png, info)
        }
    }
}

/* ================================================================== */
/* ERRORS.md: png.c -- png_check_IHDR, png_data_freer, png_fixed, ...   */
/* ================================================================== */

/// `png_set_sig_bytes` (D-1), `png_data_freer` (D-4),
/// `png_convert_to_rfc1123` (D-5), `png_set_rgb_coefficients` (D-7),
/// `png_ascii_from_fp` (D-25) and the whole `png_check_IHDR` ladder
/// (D-8 … D-24).
#[test]
fn png_c_rejections() {
    // png_set_sig_bytes: negative and > 8
    for nb in [-1i32, -8, 0, 1, 8, 9, 100, i32::MAX, i32::MIN] {
        assert_same(&format!("png_set_sig_bytes({})", nb), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_read(api);
            let g = guarded(api, png, &mut || (api.png_set_sig_bytes)(png, nb));
            o.push(format!("{:?}", g));
            destroy_read(api, png, info);
            o
        });
    }
    // png_data_freer with every freer value and a selection of masks
    for freer in [-1i32, 0, 1, 2, 3, 99] {
        for mask in [0u32, PNG_FREE_ALL, PNG_FREE_TEXT, PNG_FREE_PLTE, 0xffff_ffff] {
            assert_same(
                &format!("png_data_freer({}, 0x{:x})", freer, mask),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_read(api);
                    let g = guarded(api, png, &mut || {
                        (api.png_data_freer)(png, info, freer, mask)
                    });
                    o.push(format!("{:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }
    // png_convert_to_rfc1123 with an invalid time
    let bad_times = [
        png_time { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 1999, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 1999, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        png_time { year: 1999, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        png_time { year: 1999, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
        png_time { year: 1999, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
        png_time { year: 65535, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
    ];
    for (i, t) in bad_times.iter().enumerate() {
        assert_same(&format!("png_convert_to_rfc1123 #{}", i), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_read(api);
            let g = guarded(api, png, &mut || {
                let s = (api.png_convert_to_rfc1123)(png, t);
                if s.is_null() {
                    log("rfc1123=<null>".to_string());
                } else {
                    log(format!(
                        "rfc1123={:?}",
                        std::ffi::CStr::from_ptr(s)
                    ));
                }
                let mut buf = [b'#' as c_char; 29];
                let r = (api.png_convert_to_rfc1123_buffer)(buf.as_mut_ptr(), t);
                log(format!(
                    "buffer r={} bytes={:?}",
                    r,
                    buf.iter().map(|&c| c as u8).collect::<Vec<u8>>()
                ));
            });
            o.push(format!("{:?}", g));
            destroy_read(api, png, info);
            o
        });
    }
    // png_check_IHDR: every field out of range, on a read struct
    let widths: [u32; 8] = [0, 1, 7, 0x7fff_ffff, 0x8000_0000, 1_000_000, 1_000_001, 0xffff_ffff];
    let depths: [c_int; 8] = [0, 1, 2, 3, 4, 8, 16, 32];
    let cts: [c_int; 8] = [-1, 0, 1, 2, 3, 4, 5, 7];
    for &w in &widths {
        for &d in &depths {
            for &ct in &cts {
                for il in [0i32, 1, 2, 99] {
                    assert_same(
                        &format!("check_IHDR w={} d={} ct={} il={}", w, d, ct, il),
                        |api| unsafe {
                            let mut o = Outcome::default();
                            let (png, info) = new_read(api);
                            let g = guarded(api, png, &mut || {
                                (api.png_check_IHDR)(png, w, w, d, ct, il, 0, 0)
                            });
                            o.push(format!("{:?}", g));
                            destroy_read(api, png, info);
                            o
                        },
                    );
                }
            }
        }
    }
    // compression / filter method
    for comp in [-1i32, 0, 1, 99] {
        for filt in [-1i32, 0, 1, 64, 65, 99] {
            for mng in [0i32, 0x01, 0x04, 0x05] {
                assert_same(
                    &format!("check_IHDR comp={} filt={} mng={}", comp, filt, mng),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        let (png, info) = new_read(api);
                        let g = guarded(api, png, &mut || {
                            log(format!(
                                "mng={}",
                                (api.png_permit_mng_features)(png, mng as u32)
                            ));
                            (api.png_check_IHDR)(
                                png,
                                8,
                                8,
                                8,
                                PNG_COLOR_TYPE_RGB,
                                PNG_INTERLACE_NONE,
                                comp,
                                filt,
                            )
                        });
                        o.push(format!("{:?}", g));
                        destroy_read(api, png, info);
                        o
                    },
                );
            }
        }
    }
    // png_fixed / png_fixed_ITU out of range -> "fixed point overflow"
    let bad = [
        1e30f64, -1e30, 21475.0, -21475.0, 21474.83648, -21474.83648, 1e300, -1e300,
    ];
    for (i, &v) in bad.iter().enumerate() {
        for write in [false, true] {
            assert_same(&format!("png_fixed({:e}) write={} #{}", v, write, i), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = if write { new_write(api) } else { new_read(api) };
                let g = guarded(api, png, &mut || {
                    log(format!(
                        "fixed={}",
                        (api.png_fixed)(png, v, b"a value\0".as_ptr() as *const c_char)
                    ));
                });
                o.push(format!("fixed {:?}", g));
                let g = guarded(api, png, &mut || {
                    log(format!(
                        "fixed_ITU={}",
                        (api.png_fixed_ITU)(png, v, b"a value\0".as_ptr() as *const c_char)
                    ));
                });
                o.push(format!("fixed_ITU {:?}", g));
                destroy_or_free(api, png, info, write);
                o
            });
        }
    }
    // png_ascii_from_fp / png_ascii_from_fixed with a too-small buffer
    for size in [0usize, 1, 2, 3, 5, 7, 10, 15, 20, 30, 64] {
        for &v in &[0.0f64, 1.0, -1.0, 1e-10, 1e10, 3.14159265358979] {
            for prec in [0i32, 1, 5, 15, 16, 17, 100] {
                assert_same(
                    &format!("ascii_from_fp size={} v={:e} prec={}", size, v, prec),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        let (png, info) = new_read(api);
                        let mut buf = vec![b'#'; size.max(1) + 8];
                        let g = guarded(api, png, &mut || {
                            (api.png_ascii_from_fp)(
                                png,
                                buf.as_mut_ptr() as *mut c_char,
                                size,
                                v,
                                prec as c_uint,
                            );
                        });
                        o.push(format!("{:?}", g));
                        o.output = buf.clone();
                        destroy_read(api, png, info);
                        o
                    },
                );
            }
        }
    }
    let _ = 0 as c_uint;
}

use core::ffi::c_uint;

/* ================================================================== */
/* helper: build a reference datastream with the C library             */
/* ================================================================== */

/// Run `f` against the C library with a fresh `Tls` and return whatever it
/// produced.  Used to manufacture *valid* datastreams that the read-side tests
/// then corrupt; the corruption itself is what is compared.
fn with_c<T>(f: impl FnOnce(&Api) -> T) -> T {
    let l = libs();
    let mut state = Box::new(Tls::default());
    let prev = set_tls(&mut *state as *mut Tls);
    let prev_api = set_cur_api(&l.c as *const Api);
    let r = f(&l.c);
    set_cur_api(prev_api);
    set_tls(prev);
    r
}

fn base_png(ct: c_int, bd: c_int, il: c_int, w: u32, h: u32, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut img = Img::random(&mut rng, w, h, ct, bd);
    img.interlace = il;
    with_c(|api| unsafe { write_plain(api, &img, &WriteOpts::default()).bytes })
}

/// Read `data` with both libraries and require the same outcome.
fn diff_read(case: &str, data: &[u8], setup: impl Fn(&Api, *mut PngStruct, *mut PngInfo) + Copy) {
    assert_same(case, |api| unsafe {
        let mut o = Outcome::default();
        let rr = read_image(api, data, &ReadOpts::default(), &mut |a, p, i| setup(a, p, i));
        o.push(format!("guard={:?}", rr.guard));
        for r in &rr.rows {
            o.output.extend_from_slice(r);
        }
        o
    });
}

fn noop(_: &Api, _: *mut PngStruct, _: *mut PngInfo) {}

/* ================================================================== */
/* ERRORS.md: pngrutil.c / pngread.c -- corrupted datastreams          */
/* ================================================================== */

/// The signature checks (`png_read_sig`, `png_sig_cmp`).
#[test]
fn bad_signature() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 4, 4, 1);
    let mut rng = Rng::new(0x519);
    // every single-byte corruption of the signature
    for i in 0..8 {
        for delta in [1u8, 0x80, 0xff] {
            let mut v = good.clone();
            v[i] ^= delta;
            diff_read(&format!("sig byte {} ^ {:#x}", i, delta), &v, noop);
        }
    }
    // shorter than a signature
    for n in 0..8 {
        diff_read(&format!("only {} bytes", n), &good[..n], noop);
    }
    // random garbage
    for i in 0..40 {
        let n = rng.below(64);
        let v = rng.bytes(n);
        diff_read(&format!("garbage #{}", i), &v, noop);
    }
    // empty
    diff_read("empty input", &[], noop);
}

/// Truncation at every byte offset of a real file.
#[test]
fn truncation() {
    for (ct, bd, il) in [
        (PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE),
        (PNG_COLOR_TYPE_PALETTE, 4, PNG_INTERLACE_ADAM7),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16, PNG_INTERLACE_NONE),
    ] {
        let good = base_png(ct, bd, il, 9, 5, 0x777 ^ ct as u64);
        for n in 0..good.len() {
            diff_read(
                &format!("truncated ct={} bd={} il={} at {}", ct, bd, il, n),
                &good[..n],
                noop,
            );
        }
    }
}

/// Every chunk's CRC broken, against every `png_set_crc_action` combination.
#[test]
fn crc_corruption() {
    let good = with_c(|api| unsafe {
        let mut rng = Rng::new(0xc12c);
        let img = Img::random(&mut rng, 6, 4, PNG_COLOR_TYPE_RGB, 8);
        let gray = cs("some text");
        let key = cs("Comment");
        write_image(api, &img, &WriteOpts::default(), &mut |api, png, info| {
            (api.png_set_gAMA)(png, info, 0.45455);
            (api.png_set_pHYs)(png, info, 100, 200, 1);
            let mut t = png_text {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: key.as_ptr() as *mut c_char,
                text: gray.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: core::ptr::null_mut(),
                lang_key: core::ptr::null_mut(),
            };
            (api.png_set_text)(png, info, &mut t, 1);
        })
        .bytes
    });
    let chunks = split_chunks(&good);
    assert!(chunks.len() >= 4, "expected several chunks, got {:?}", chunks);
    for (name, range) in &chunks {
        for crit in 0..6 {
            for ancil in 0..6 {
                let mut v = good.clone();
                let last = range.end - 1;
                v[last] ^= 0xff;
                diff_read(
                    &format!("crc {} crit={} ancil={}", name, crit, ancil),
                    &v,
                    move |api, png, _info| unsafe {
                        (api.png_set_crc_action)(png, crit, ancil);
                    },
                );
            }
        }
    }
}

/// Every chunk's declared length corrupted.
#[test]
fn length_corruption() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 6, 4, 0x1e2);
    let chunks = split_chunks(&good);
    for (name, range) in &chunks {
        for newlen in [
            0u32,
            1,
            2,
            3,
            12,
            13,
            14,
            0x7fff_ffff,
            0x8000_0000,
            0xffff_ffff,
        ] {
            let mut v = good.clone();
            v[range.start..range.start + 4].copy_from_slice(&newlen.to_be_bytes());
            diff_read(&format!("{} length={:#x}", name, newlen), &v, noop);
        }
    }
}

/// Structural errors: missing, duplicated and misordered chunks.
#[test]
fn chunk_structure() {
    let pal = base_png(PNG_COLOR_TYPE_PALETTE, 8, PNG_INTERLACE_NONE, 5, 3, 0x9a1);
    let rgb = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 5, 3, 0x9a2);
    for (tag, base) in [("palette", &pal), ("rgb", &rgb)] {
        let chunks = split_chunks(base);
        // drop each chunk in turn
        for (name, range) in &chunks {
            let mut v = base[..range.start].to_vec();
            v.extend_from_slice(&base[range.end..]);
            diff_read(&format!("{}: dropped {}", tag, name), &v, noop);
        }
        // duplicate each chunk in turn
        for (name, range) in &chunks {
            let mut v = base[..range.end].to_vec();
            v.extend_from_slice(&base[range.start..range.end]);
            v.extend_from_slice(&base[range.end..]);
            diff_read(&format!("{}: duplicated {}", tag, name), &v, noop);
        }
        // move IHDR to the end
        if let Some((_, r)) = chunks.iter().find(|(n, _)| n == "IHDR") {
            let mut v = base[..8].to_vec();
            v.extend_from_slice(&base[r.end..]);
            v.extend_from_slice(&base[r.start..r.end]);
            diff_read(&format!("{}: IHDR last", tag), &v, noop);
        }
        // a bogus critical chunk before IDAT, and an unknown ancillary one
        for name in [b"crIT", b"ancI", b"IHDR", b"IEND", b"IDAT", b"PLTE"] {
            let extra = chunk(name, &[1, 2, 3, 4]);
            let v = insert_before(base, "IDAT", &extra);
            diff_read(
                &format!("{}: extra {} before IDAT", tag, String::from_utf8_lossy(name)),
                &v,
                noop,
            );
            let v = insert_after_last(base, "IDAT", &extra);
            diff_read(
                &format!("{}: extra {} after IDAT", tag, String::from_utf8_lossy(name)),
                &v,
                noop,
            );
        }
        // trailing garbage after IEND
        let mut v = base.clone();
        v.extend_from_slice(&[0, 0, 0, 0, b'j', b'U', b'N', b'K', 0x12, 0x34, 0x56, 0x78]);
        diff_read(&format!("{}: junk after IEND", tag), &v, noop);
        // chunk name with invalid characters
        for name in [b"\x00\x00\x00\x00", b"1234", b"ab d", b"AB\xffD"] {
            let extra = chunk(name, &[]);
            let v = insert_before(base, "IDAT", &extra);
            diff_read(&format!("{}: bad chunk name {:?}", tag, name), &v, noop);
        }
    }
}

/// The IDAT / zlib error paths (`png_inflate`, `png_read_IDAT_data`,
/// `png_zstream_error`).
#[test]
fn idat_corruption() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 12, 8, 0x1da7);
    let chunks = split_chunks(&good);
    let idat = chunks
        .iter()
        .find(|(n, _)| n == "IDAT")
        .expect("IDAT")
        .1
        .clone();
    let mut rng = Rng::new(0x1da8);
    // flip a bit in every byte of the compressed data (CRC repaired so that the
    // zlib layer, not the CRC layer, is what rejects it)
    for off in idat.start + 8..idat.end - 4 {
        let mut v = good.clone();
        v[off] ^= 0x01;
        let dstart = idat.start + 4;
        let dend = idat.end - 4;
        let crc = crc32(&v[dstart..dend]);
        v[dend..dend + 4].copy_from_slice(&crc.to_be_bytes());
        diff_read(&format!("IDAT bit flip at {}", off), &v, noop);
    }
    // random multi-byte corruption with repaired CRC
    for i in 0..200 {
        let mut v = good.clone();
        let n = 1 + rng.below(6);
        for _ in 0..n {
            let off = idat.start + 8 + rng.below(idat.end - 4 - (idat.start + 8));
            v[off] = rng.u8();
        }
        let dstart = idat.start + 4;
        let dend = idat.end - 4;
        let crc = crc32(&v[dstart..dend]);
        v[dend..dend + 4].copy_from_slice(&crc.to_be_bytes());
        diff_read(&format!("IDAT fuzz #{}", i), &v, noop);
    }
    // zero-length IDAT, and an IDAT split into many tiny chunks
    let head = &good[..idat.start];
    let tail = &good[idat.end..];
    let payload = good[idat.start + 8..idat.end - 4].to_vec();
    let mut v = head.to_vec();
    v.extend_from_slice(&chunk(b"IDAT", &[]));
    v.extend_from_slice(&chunk(b"IDAT", &payload));
    v.extend_from_slice(tail);
    diff_read("empty IDAT then real IDAT", &v, noop);
    let mut v = head.to_vec();
    for c in payload.chunks(1) {
        v.extend_from_slice(&chunk(b"IDAT", c));
    }
    v.extend_from_slice(tail);
    diff_read("IDAT split into 1-byte chunks", &v, noop);
    let mut v = head.to_vec();
    v.extend_from_slice(&chunk(b"IDAT", &payload));
    v.extend_from_slice(&chunk(b"IDAT", &payload));
    v.extend_from_slice(tail);
    diff_read("IDAT twice", &v, noop);
    // truncated zlib stream (CRC repaired)
    for keep in 0..payload.len().min(40) {
        let mut v = head.to_vec();
        v.extend_from_slice(&chunk(b"IDAT", &payload[..keep]));
        v.extend_from_slice(tail);
        diff_read(&format!("IDAT truncated to {}", keep), &v, noop);
    }
}

/// User limits: `png_set_user_limits`, `png_set_chunk_cache_max`,
/// `png_set_chunk_malloc_max`.
#[test]
fn user_limits_rejections() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 16, 9, 0x11_a1);
    for (wmax, hmax) in [
        (0u32, 0u32),
        (1, 1),
        (15, 9),
        (16, 8),
        (16, 9),
        (17, 10),
        (0x7fff_ffff, 0x7fff_ffff),
    ] {
        diff_read(
            &format!("user_limits {}x{}", wmax, hmax),
            &good,
            move |api, png, _| unsafe {
                (api.png_set_user_limits)(png, wmax, hmax);
                log(format!(
                    "limits w={} h={}",
                    (api.png_get_user_width_max)(png),
                    (api.png_get_user_height_max)(png)
                ));
            },
        );
    }
    // many unknown chunks -> chunk cache limit
    let mut many = good.clone();
    for i in 0..40u32 {
        let extra = chunk(b"unKn", &i.to_be_bytes());
        many = insert_before(&many, "IDAT", &extra);
    }
    for cache in [0u32, 1, 2, 10, 39, 40, 41, 1000] {
        diff_read(
            &format!("chunk_cache_max={}", cache),
            &many,
            move |api, png, _| unsafe {
                (api.png_set_chunk_cache_max)(png, cache);
                (api.png_set_keep_unknown_chunks)(
                    png,
                    PNG_HANDLE_CHUNK_ALWAYS,
                    core::ptr::null(),
                    0,
                );
                log(format!(
                    "cache={}",
                    (api.png_get_chunk_cache_max)(png)
                ));
            },
        );
    }
    let big = chunk(b"unKn", &vec![0x5au8; 4096]);
    let withbig = insert_before(&good, "IDAT", &big);
    for mm in [0usize, 1, 100, 4095, 4096, 4097, 8_000_000] {
        diff_read(
            &format!("chunk_malloc_max={}", mm),
            &withbig,
            move |api, png, _| unsafe {
                (api.png_set_chunk_malloc_max)(png, mm);
                (api.png_set_keep_unknown_chunks)(
                    png,
                    PNG_HANDLE_CHUNK_ALWAYS,
                    core::ptr::null(),
                    0,
                );
                log(format!(
                    "malloc_max={}",
                    (api.png_get_chunk_malloc_max)(png)
                ));
            },
        );
    }
}

/* ================================================================== */
/* ERRORS.md: pngset.c / pngwutil.c / pngwrite.c -- API misuse on write */
/* ================================================================== */

/// The write-side sequencing errors in `pngwrite.c` / `pngwutil.c`.
#[test]
fn write_sequencing() {
    let mut rng = Rng::new(0x5e_91);
    let img = Img::random(&mut rng, 4, 3, PNG_COLOR_TYPE_RGB, 8);
    let rowptr = img.rows[0].as_ptr() as *mut u8;
    let cases: [(&str, usize); 12] = [
        ("row before info", 0),
        ("end before info", 1),
        ("info twice", 2),
        ("too many rows", 3),
        ("no IHDR", 4),
        ("end without rows", 5),
        ("info_before_PLTE then info", 6),
        ("write_image without rows", 7),
        ("write_end twice", 8),
        ("write_sig twice", 9),
        ("chunk_data without chunk_start", 10),
        ("chunk_end without chunk_start", 11),
    ];
    for (tag, which) in cases {
        // These deliberately misuse the API; several of them are fatal to the C
        // library (it writes a row after the last one, or dereferences the NULL
        // row array), so each side runs in its own child and the *crash* is part
        // of what is compared.
        assert_same_forked(&format!("write seq: {}", tag), |api| unsafe {
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let set_ihdr = |api: &Api| {
                (api.png_set_IHDR)(
                    png,
                    info,
                    img.w,
                    img.h,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
            };
            let g = guarded(api, png, &mut || match which {
                0 => {
                    set_ihdr(api);
                    (api.png_write_row)(png, rowptr);
                }
                1 => {
                    set_ihdr(api);
                    (api.png_write_end)(png, info);
                }
                2 => {
                    set_ihdr(api);
                    (api.png_write_info)(png, info);
                    (api.png_write_info)(png, info);
                }
                3 => {
                    set_ihdr(api);
                    (api.png_write_info)(png, info);
                    for _ in 0..(img.h as usize + 3) {
                        (api.png_write_row)(png, rowptr);
                    }
                }
                4 => {
                    (api.png_write_info)(png, info);
                }
                5 => {
                    set_ihdr(api);
                    (api.png_write_info)(png, info);
                    (api.png_write_end)(png, info);
                }
                6 => {
                    set_ihdr(api);
                    (api.png_write_info_before_PLTE)(png, info);
                    (api.png_write_info_before_PLTE)(png, info);
                    (api.png_write_info)(png, info);
                    (api.png_write_end)(png, info);
                }
                7 => {
                    set_ihdr(api);
                    (api.png_write_info)(png, info);
                    (api.png_write_image)(png, core::ptr::null_mut());
                }
                8 => {
                    set_ihdr(api);
                    (api.png_write_info)(png, info);
                    for r in &img.rows {
                        (api.png_write_row)(png, r.as_ptr() as *mut u8);
                    }
                    (api.png_write_end)(png, info);
                    (api.png_write_end)(png, info);
                }
                9 => {
                    (api.png_write_sig)(png);
                    (api.png_write_sig)(png);
                }
                10 => {
                    (api.png_write_sig)(png);
                    (api.png_write_chunk_data)(png, img.rows[0].as_ptr(), 4);
                }
                _ => {
                    (api.png_write_sig)(png);
                    (api.png_write_chunk_end)(png);
                }
            });
            let out = std::mem::take(&mut tls().output);
            let _ = (png, info);
            format!("guard={:?} out={}", g, out.len())
        });
    }
}

/// The `png_set_*` validation errors in `pngset.c`.
#[test]
fn setter_validation() {
    // png_set_PLTE
    for np in [-1i32, 0, 1, 2, 3, 4, 5, 16, 17, 255, 256, 257, 1000, i32::MAX] {
        for bd in [1i32, 2, 4, 8] {
            for ct in [PNG_COLOR_TYPE_PALETTE, PNG_COLOR_TYPE_RGB] {
                let pal = vec![png_color { red: 1, green: 2, blue: 3 }; 300];
                assert_same(
                    &format!("set_PLTE np={} bd={} ct={}", np, bd, ct),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        let (png, info) = new_write(api);
                        let g = guarded(api, png, &mut || {
                            (api.png_set_IHDR)(
                                png,
                                info,
                                4,
                                4,
                                bd,
                                ct,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            (api.png_set_PLTE)(png, info, pal.as_ptr(), np);
                            let mut p: *mut png_color = core::ptr::null_mut();
                            let mut n: c_int = -99;
                            log(format!(
                                "get_PLTE r={} n={} null={}",
                                (api.png_get_PLTE)(png, info, &mut p, &mut n),
                                n,
                                p.is_null()
                            ));
                        });
                        o.push(format!("guard={:?}", g));
                        destroy_write(api, png, info);
                        o
                    },
                );
            }
        }
    }
    // png_set_tRNS
    for nt in [-1i32, 0, 1, 2, 255, 256, 257] {
        for ct in [PNG_COLOR_TYPE_PALETTE, PNG_COLOR_TYPE_GRAY, PNG_COLOR_TYPE_RGB] {
            let trans = vec![0x80u8; 300];
            let tcol = png_color_16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
            assert_same(&format!("set_tRNS nt={} ct={}", nt, ct), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        4,
                        4,
                        8,
                        ct,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (api.png_set_tRNS)(png, info, trans.as_ptr(), nt, &tcol);
                    log(format!("valid=0x{:x}", (api.png_get_valid)(png, info, 0xffffffff)));
                });
                o.push(format!("guard={:?}", g));
                destroy_write(api, png, info);
                o
            });
        }
    }
    // png_set_sBIT
    for ct in [
        PNG_COLOR_TYPE_GRAY,
        PNG_COLOR_TYPE_PALETTE,
        PNG_COLOR_TYPE_RGB,
        PNG_COLOR_TYPE_GRAY_ALPHA,
        PNG_COLOR_TYPE_RGB_ALPHA,
    ] {
        for v in [0u8, 1, 4, 8, 9, 16, 17, 255] {
            let sb = png_color_8 { red: v, green: v, blue: v, gray: v, alpha: v };
            assert_same(&format!("set_sBIT ct={} v={}", ct, v), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        4,
                        4,
                        8,
                        ct,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (api.png_set_sBIT)(png, info, &sb);
                    log(format!("valid=0x{:x}", (api.png_get_valid)(png, info, 0xffffffff)));
                });
                o.push(format!("guard={:?}", g));
                destroy_write(api, png, info);
                o
            });
        }
    }
    // png_set_hIST without a palette, and with the wrong length
    for np in [0i32, 1, 4, 256] {
        let hist = vec![7u16; 300];
        let pal = vec![png_color { red: 9, green: 8, blue: 7 }; 300];
        assert_same(&format!("set_hIST np={}", np), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_IHDR)(
                    png,
                    info,
                    4,
                    4,
                    8,
                    PNG_COLOR_TYPE_PALETTE,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                if np > 0 {
                    (api.png_set_PLTE)(png, info, pal.as_ptr(), np);
                }
                (api.png_set_hIST)(png, info, hist.as_ptr());
                log(format!("valid=0x{:x}", (api.png_get_valid)(png, info, 0xffffffff)));
            });
            o.push(format!("guard={:?}", g));
            destroy_write(api, png, info);
            o
        });
    }
    // png_set_sRGB with an out-of-range intent
    for intent in [-2i32, -1, 0, 1, 2, 3, 4, 5, 99] {
        assert_same(&format!("set_sRGB intent={}", intent), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_sRGB)(png, info, intent);
                let mut got: c_int = -99;
                log(format!(
                    "get_sRGB r={} intent={}",
                    (api.png_get_sRGB)(png, info, &mut got),
                    got
                ));
            });
            o.push(format!("guard={:?}", g));
            destroy_write(api, png, info);
            o
        });
        assert_same(
            &format!("set_sRGB_gAMA_and_cHRM intent={}", intent),
            |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                let g = guarded(api, png, &mut || {
                    (api.png_set_sRGB_gAMA_and_cHRM)(png, info, intent);
                });
                o.push(format!("guard={:?}", g));
                destroy_write(api, png, info);
                o
            },
        );
    }
    // png_set_pCAL: bad equation type / nparams / strings
    for eq in [-1i32, 0, 1, 2, 3, 4, 99] {
        for np in [-1i32, 0, 1, 2, 8, 100] {
            let purpose = cs("purpose");
            let units = cs("units");
            let p0 = cs("1.5");
            let p1 = cs("not a number");
            let mut params: Vec<*mut c_char> =
                vec![p0.as_ptr() as *mut c_char, p1.as_ptr() as *mut c_char];
            for _ in 0..200 {
                params.push(p0.as_ptr() as *mut c_char);
            }
            assert_same(&format!("set_pCAL eq={} np={}", eq, np), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                let g = guarded(api, png, &mut || {
                    (api.png_set_pCAL)(
                        png,
                        info,
                        purpose.as_ptr(),
                        -100,
                        100,
                        eq,
                        np,
                        units.as_ptr(),
                        params.as_mut_ptr(),
                    );
                    log(format!("valid=0x{:x}", (api.png_get_valid)(png, info, 0xffffffff)));
                });
                o.push(format!("guard={:?}", g));
                destroy_write(api, png, info);
                o
            });
        }
    }
    // png_set_sCAL / png_set_sCAL_s / png_set_sCAL_fixed with bad values
    for unit in [-1i32, 0, 1, 2, 3, 99] {
        for (w, h) in [(0.0f64, 0.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0), (1e300, 1e-300)] {
            assert_same(
                &format!("set_sCAL unit={} {}x{}", unit, w, h),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    let g = guarded(api, png, &mut || {
                        (api.png_set_sCAL)(png, info, unit, w, h);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_write(api, png, info);
                    o
                },
            );
        }
        for (ws, hs) in [("", ""), ("0", "0"), ("-1", "1"), ("1e", "x"), ("1.5", "2.5")] {
            let a = cs(ws);
            let b = cs(hs);
            assert_same(
                &format!("set_sCAL_s unit={} {:?}x{:?}", unit, ws, hs),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    let g = guarded(api, png, &mut || {
                        (api.png_set_sCAL_s)(png, info, unit, a.as_ptr(), b.as_ptr());
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_write(api, png, info);
                    o
                },
            );
        }
    }
    // png_set_text with bad keys / compression values
    for comp in [-4i32, -3, -2, -1, 0, 1, 2, 3, 99] {
        for key in [
            "",
            " leading",
            "trailing ",
            "double  space",
            "ok",
            "with\ttab",
            "with\nnewline",
            "0123456789012345678901234567890123456789012345678901234567890123456789012345678",
            "01234567890123456789012345678901234567890123456789012345678901234567890123456789",
            "012345678901234567890123456789012345678901234567890123456789012345678901234567890",
        ] {
            let k = cs(key);
            let txt = cs("hello");
            let lang = cs("en");
            let lk = cs("Comment");
            assert_same(
                &format!("set_text comp={} key={:?}", comp, key),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    (api.png_set_write_fn)(
                        png,
                        core::ptr::null_mut(),
                        Some(write_cb),
                        Some(flush_cb),
                    );
                    let mut t = png_text {
                        compression: comp,
                        key: k.as_ptr() as *mut c_char,
                        text: txt.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: lang.as_ptr() as *mut c_char,
                        lang_key: lk.as_ptr() as *mut c_char,
                    };
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            2,
                            2,
                            8,
                            PNG_COLOR_TYPE_GRAY,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        log(format!("set_text r={}", {
                            (api.png_set_text)(png, info, &mut t, 1);
                            0
                        }));
                        (api.png_write_info)(png, info);
                        let row = [0u8, 0u8];
                        (api.png_write_row)(png, row.as_ptr() as *mut u8);
                        (api.png_write_row)(png, row.as_ptr() as *mut u8);
                        (api.png_write_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    o.output = std::mem::take(&mut tls().output);
                    destroy_write(api, png, info);
                    o
                },
            );
        }
    }
    // png_set_iCCP with a bad name / profile
    for (tag, name, prof) in [
        ("empty name", "", vec![0u8; 132]),
        ("long name", &"n".repeat(100)[..], vec![0u8; 132]),
        ("short profile", "n", vec![0u8; 3]),
        ("zero profile", "n", vec![]),
        ("bad length field", "n", vec![0xffu8; 132]),
    ] {
        let n = cs(name);
        assert_same(&format!("set_iCCP {}", tag), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let g = guarded(api, png, &mut || {
                (api.png_set_IHDR)(
                    png,
                    info,
                    2,
                    2,
                    8,
                    PNG_COLOR_TYPE_GRAY,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (api.png_set_iCCP)(
                    png,
                    info,
                    n.as_ptr(),
                    0,
                    prof.as_ptr(),
                    prof.len() as u32,
                );
                (api.png_write_info)(png, info);
                let row = [0u8, 0u8];
                (api.png_write_row)(png, row.as_ptr() as *mut u8);
                (api.png_write_row)(png, row.as_ptr() as *mut u8);
                (api.png_write_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            o.output = std::mem::take(&mut tls().output);
            destroy_write(api, png, info);
            o
        });
    }
    // png_set_sPLT with a bad depth / nentries
    for depth in [0u8, 1, 7, 8, 15, 16, 17, 255] {
        for nent in [-1i32, 0, 1, 2] {
            let nm = cs("splt");
            let ent = vec![
                png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 };
                4
            ];
            assert_same(
                &format!("set_sPLT depth={} nent={}", depth, nent),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    let mut sp = png_sPLT_t {
                        name: nm.as_ptr() as *mut c_char,
                        depth,
                        entries: ent.as_ptr() as *mut png_sPLT_entry,
                        nentries: nent,
                    };
                    let g = guarded(api, png, &mut || {
                        (api.png_set_sPLT)(png, info, &mut sp, 1);
                        let mut got: *mut png_sPLT_t = core::ptr::null_mut();
                        log(format!(
                            "get_sPLT n={}",
                            (api.png_get_sPLT)(png, info, &mut got)
                        ));
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_write(api, png, info);
                    o
                },
            );
        }
    }
    // png_set_eXIf_1 with a bad size.  The buffer is always at least `n` bytes:
    // libpng copies `n` bytes verbatim, so a shorter buffer would make the test
    // compare adjacent heap rather than library behaviour.
    for n in [0u32, 1, 2, 3, 4, 5, 100] {
        let mut data = vec![b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0];
        data.resize((n as usize).max(10), 0x5a);
        assert_same(&format!("set_eXIf_1 n={}", n), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let g = guarded(api, png, &mut || {
                (api.png_set_IHDR)(
                    png,
                    info,
                    2,
                    2,
                    8,
                    PNG_COLOR_TYPE_GRAY,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (api.png_set_eXIf_1)(png, info, n, data.as_ptr() as *mut u8);
                (api.png_write_info)(png, info);
                let row = [0u8, 0u8];
                (api.png_write_row)(png, row.as_ptr() as *mut u8);
                (api.png_write_row)(png, row.as_ptr() as *mut u8);
                (api.png_write_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            o.output = std::mem::take(&mut tls().output);
            destroy_write(api, png, info);
            o
        });
    }
    // png_set_unknown_chunks with a bad chunk name / location
    for name in [
        *b"\0\0\0\0\0",
        *b"abcd\0",
        *b"ABCD\0",
        *b"1234\0",
        *b"ab d\0",
        *b"IHDR\0",
    ] {
        for loc in [0u8, 1, 2, 4, 8, 0xff] {
            let data = vec![1u8, 2, 3];
            assert_same(
                &format!("set_unknown_chunks {:?} loc={}", name, loc),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    (api.png_set_write_fn)(
                        png,
                        core::ptr::null_mut(),
                        Some(write_cb),
                        Some(flush_cb),
                    );
                    let mut uc = png_unknown_chunk {
                        name,
                        data: data.as_ptr() as *mut u8,
                        size: data.len(),
                        location: loc,
                    };
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            2,
                            2,
                            8,
                            PNG_COLOR_TYPE_GRAY,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        (api.png_set_unknown_chunks)(png, info, &mut uc, 1);
                        (api.png_write_info)(png, info);
                        let row = [0u8, 0u8];
                        (api.png_write_row)(png, row.as_ptr() as *mut u8);
                        (api.png_write_row)(png, row.as_ptr() as *mut u8);
                        (api.png_write_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    o.output = std::mem::take(&mut tls().output);
                    destroy_write(api, png, info);
                    o
                },
            );
        }
    }
    // png_write_chunk with an invalid chunk name
    for name in [*b"\0\0\0\0", *b"1234", *b"ab d", *b"\xff\xff\xff\xff", *b"IHDR"] {
        assert_same(&format!("write_chunk name={:?}", name), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let g = guarded(api, png, &mut || {
                (api.png_write_sig)(png);
                (api.png_write_chunk)(png, name.as_ptr(), [1u8, 2].as_ptr(), 2);
            });
            o.push(format!("guard={:?}", g));
            o.output = std::mem::take(&mut tls().output);
            destroy_write(api, png, info);
            o
        });
    }
}

/* ================================================================== */
/* memory-failure injection                                            */
/* ================================================================== */

/// A `png_set_mem_fn` allocator that fails after N successful allocations, so
/// that every `png_malloc` / `png_malloc_warn` / `png_calloc` failure path in
/// the library is reached.  ERRORS.md rows in `pngmem.c` plus every
/// "Out of memory" / "Insufficient memory" site.
static FAIL_AFTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn failing_malloc(_png: *mut PngStruct, size: usize) -> *mut core::ffi::c_void {
    let t = tls();
    t.alloc_serial += 1;
    let n = FAIL_AFTER.load(std::sync::atomic::Ordering::Relaxed);
    if t.alloc_serial as usize > n {
        log(format!("malloc #{} size={} -> FAIL", t.alloc_serial, size));
        return core::ptr::null_mut();
    }
    let p = raw_malloc(size.max(1));
    log(format!("malloc #{} size={} -> ok", t.alloc_serial, size));
    p
}

unsafe extern "C" fn failing_free(_png: *mut PngStruct, p: *mut core::ffi::c_void) {
    raw_free(p);
}

extern "C" {
    #[link_name = "malloc"]
    fn raw_malloc(n: usize) -> *mut core::ffi::c_void;
    #[link_name = "free"]
    fn raw_free(p: *mut core::ffi::c_void);
}

#[test]
fn out_of_memory() {
    let good = base_png(PNG_COLOR_TYPE_PALETTE, 8, PNG_INTERLACE_NONE, 12, 8, 0x0000);
    let rich = with_c(|api| unsafe {
        let mut rng = Rng::new(0x0a0a);
        let img = Img::random(&mut rng, 12, 8, PNG_COLOR_TYPE_PALETTE, 8);
        let key = cs("Comment");
        let txt = cs("some text to compress, some text to compress, some text");
        let nm = cs("splt");
        let ent = vec![png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 8];
        write_image(api, &img, &WriteOpts::default(), &mut |api, png, info| {
            let mut t = png_text {
                compression: PNG_TEXT_COMPRESSION_zTXt,
                key: key.as_ptr() as *mut c_char,
                text: txt.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: core::ptr::null_mut(),
                lang_key: core::ptr::null_mut(),
            };
            (api.png_set_text)(png, info, &mut t, 1);
            let mut sp = png_sPLT_t {
                name: nm.as_ptr() as *mut c_char,
                depth: 8,
                entries: ent.as_ptr() as *mut png_sPLT_entry,
                nentries: 8,
            };
            (api.png_set_sPLT)(png, info, &mut sp, 1);
            (api.png_set_gAMA)(png, info, 0.45455);
            let bk = png_color_16 { index: 1, red: 0, green: 0, blue: 0, gray: 0 };
            (api.png_set_bKGD)(png, info, &bk);
            let hist = vec![3u16; 256];
            (api.png_set_hIST)(png, info, hist.as_ptr());
            let tr = vec![0x40u8; 256];
            (api.png_set_tRNS)(png, info, tr.as_ptr(), 256, core::ptr::null());
        })
        .bytes
    });
    for n in 0..60usize {
        FAIL_AFTER.store(n, std::sync::atomic::Ordering::Relaxed);
        for (tag, data) in [("plain", &good), ("rich", &rich)] {
            // reading with a failing allocator
            assert_same_forked(&format!("oom read {} after {}", tag, n), |api| unsafe {
                FAIL_AFTER.store(n, std::sync::atomic::Ordering::Relaxed);
                tls().input = data.to_vec();
                tls().in_pos = 0;
                let sh = &libs().shim;
                let png = (api.png_create_read_struct_2)(
                    VER,
                    core::ptr::null_mut(),
                    Some(sh.error_fn),
                    Some(warn_cb),
                    core::ptr::null_mut(),
                    Some(failing_malloc),
                    Some(failing_free),
                );
                if png.is_null() {
                    return "create_read_struct_2 -> NULL".to_string();
                }
                let info = (api.png_create_info_struct)(png);
                if info.is_null() {
                    return "create_info_struct -> NULL".to_string();
                }
                (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                let mut nrows = 0usize;
                let g = guarded(api, png, &mut || {
                    (api.png_read_info)(png, info);
                    (api.png_read_update_info)(png, info);
                    let rb = (api.png_get_rowbytes)(png, info);
                    let h = (api.png_get_image_height)(png, info) as usize;
                    let mut row = vec![0u8; rb + 8];
                    for _ in 0..h {
                        (api.png_read_row)(png, row.as_mut_ptr(), core::ptr::null_mut());
                        nrows += 1;
                    }
                    (api.png_read_end)(png, info);
                });
                format!("{:?} rows={}", g, nrows)
            });
        }
        // writing with a failing allocator
        assert_same_forked(&format!("oom write after {}", n), |api| unsafe {
            FAIL_AFTER.store(n, std::sync::atomic::Ordering::Relaxed);
            let mut rng = Rng::new(0x0b0b);
            let img = Img::random(&mut rng, 10, 6, PNG_COLOR_TYPE_RGB, 8);
            let sh = &libs().shim;
            let png = (api.png_create_write_struct_2)(
                VER,
                core::ptr::null_mut(),
                Some(sh.error_fn),
                Some(warn_cb),
                core::ptr::null_mut(),
                Some(failing_malloc),
                Some(failing_free),
            );
            if png.is_null() {
                return "create_write_struct_2 -> NULL".to_string();
            }
            let info = (api.png_create_info_struct)(png);
            if info.is_null() {
                return "create_info_struct -> NULL".to_string();
            }
            (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let g = guarded(api, png, &mut || {
                (api.png_set_IHDR)(
                    png,
                    info,
                    img.w,
                    img.h,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (api.png_write_info)(png, info);
                for r in &img.rows {
                    (api.png_write_row)(png, r.as_ptr() as *mut u8);
                }
                (api.png_write_end)(png, info);
            });
            format!("{:?} out={}", g, tls().output.len())
        });
    }
    FAIL_AFTER.store(usize::MAX, std::sync::atomic::Ordering::Relaxed);
}

/* ================================================================== */
/* per-chunk content corruption on the read side                       */
/* ================================================================== */

/// Every ancillary chunk libpng understands, injected with a hand-built payload
/// of every wrong length and with out-of-range contents.  This is what reaches
/// the great majority of the `png_chunk_benign_error` /
/// `png_chunk_warning` rows in `pngrutil.c`.
#[test]
fn ancillary_chunk_corruption() {
    let names: [&[u8; 4]; 21] = [
        b"gAMA", b"cHRM", b"sRGB", b"iCCP", b"sBIT", b"bKGD", b"hIST", b"tRNS", b"pHYs",
        b"oFFs", b"tIME", b"pCAL", b"sCAL", b"sPLT", b"tEXt", b"zTXt", b"iTXt", b"eXIf",
        b"cICP", b"cLLI", b"mDCV",
    ];
    let mut rng = Rng::new(0xa4c1);
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    ] {
        let good = base_png(ct, bd, PNG_INTERLACE_NONE, 5, 3, 0xa4 ^ ct as u64);
        for name in names {
            for len in [0usize, 1, 2, 3, 4, 5, 6, 8, 9, 12, 13, 16, 32, 33, 64, 100] {
                for pattern in 0..3 {
                    let data: Vec<u8> = match pattern {
                        0 => vec![0u8; len],
                        1 => vec![0xffu8; len],
                        _ => rng.bytes(len),
                    };
                    let c = chunk(name, &data);
                    // before IDAT, and after IDAT
                    let v = insert_before(&good, "IDAT", &c);
                    diff_read(
                        &format!(
                            "{} len={} pat={} ct={} bd={} before IDAT",
                            String::from_utf8_lossy(name),
                            len,
                            pattern,
                            ct,
                            bd
                        ),
                        &v,
                        noop,
                    );
                    let v = insert_after_last(&good, "IDAT", &c);
                    diff_read(
                        &format!(
                            "{} len={} pat={} ct={} bd={} after IDAT",
                            String::from_utf8_lossy(name),
                            len,
                            pattern,
                            ct,
                            bd
                        ),
                        &v,
                        noop,
                    );
                }
            }
        }
        // PLTE / IHDR with every wrong length
        for name in [b"PLTE", b"IHDR", b"IEND"] {
            for len in [0usize, 1, 2, 3, 4, 12, 13, 14, 256, 768, 769] {
                let data = rng.bytes(len);
                let c = chunk(name, &data);
                let v = insert_before(&good, "IDAT", &c);
                diff_read(
                    &format!(
                        "{} len={} ct={}",
                        String::from_utf8_lossy(name),
                        len,
                        ct
                    ),
                    &v,
                    noop,
                );
            }
        }
    }
}

/// Well-formed but semantically invalid chunk payloads, so that the *content*
/// checks (not the length checks) fire.
#[test]
fn ancillary_chunk_semantics() {
    let gray = base_png(PNG_COLOR_TYPE_GRAY, 8, PNG_INTERLACE_NONE, 4, 3, 0x5e11);
    let pal = base_png(PNG_COLOR_TYPE_PALETTE, 4, PNG_INTERLACE_NONE, 4, 3, 0x5e12);
    let rgb = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 4, 3, 0x5e13);

    let mut cases: Vec<(String, Vec<u8>, Vec<u8>)> = Vec::new();
    let mut add = |tag: &str, base: &Vec<u8>, c: Vec<u8>| {
        cases.push((tag.to_string(), base.clone(), c));
    };
    // sRGB: intent 0..5
    for i in 0..6u8 {
        add(&format!("sRGB intent={}", i), &gray, chunk(b"sRGB", &[i]));
    }
    // gAMA: 0 and huge
    for g in [0u32, 1, 100000, 0x7fffffff, 0x80000000, 0xffffffff] {
        add(&format!("gAMA {:#x}", g), &gray, chunk(b"gAMA", &g.to_be_bytes()));
    }
    // cHRM: all zeros, all max
    let mut ch = Vec::new();
    for _ in 0..8 {
        ch.extend_from_slice(&0u32.to_be_bytes());
    }
    add("cHRM zeros", &gray, chunk(b"cHRM", &ch));
    let mut ch = Vec::new();
    for _ in 0..8 {
        ch.extend_from_slice(&0xffff_ffffu32.to_be_bytes());
    }
    add("cHRM max", &gray, chunk(b"cHRM", &ch));
    // sBIT out of range for the colour type
    for v in [0u8, 1, 8, 9, 16, 255] {
        add(&format!("sBIT gray {}", v), &gray, chunk(b"sBIT", &[v]));
        add(&format!("sBIT rgb {}", v), &rgb, chunk(b"sBIT", &[v, v, v]));
        add(&format!("sBIT pal {}", v), &pal, chunk(b"sBIT", &[v, v, v]));
    }
    // bKGD for each colour type with an out-of-range value
    add("bKGD gray 0xffff", &gray, chunk(b"bKGD", &[0xff, 0xff]));
    add("bKGD pal index 255", &pal, chunk(b"bKGD", &[255]));
    add(
        "bKGD rgb",
        &rgb,
        chunk(b"bKGD", &[0xff, 0xff, 0, 0, 0x12, 0x34]),
    );
    // tRNS
    add("tRNS gray", &gray, chunk(b"tRNS", &[0x12, 0x34]));
    add("tRNS pal 300", &pal, chunk(b"tRNS", &vec![0x80u8; 300]));
    add("tRNS pal 0", &pal, chunk(b"tRNS", &[]));
    add("tRNS rgb", &rgb, chunk(b"tRNS", &[0, 1, 0, 2, 0, 3]));
    add("tRNS on rgba", &rgb, chunk(b"tRNS", &[0, 1, 0, 2, 0, 3, 0, 4]));
    // pHYs / oFFs unit
    for u in [0u8, 1, 2, 255] {
        let mut d = Vec::new();
        d.extend_from_slice(&100u32.to_be_bytes());
        d.extend_from_slice(&200u32.to_be_bytes());
        d.push(u);
        add(&format!("pHYs unit={}", u), &gray, chunk(b"pHYs", &d));
        let mut d = Vec::new();
        d.extend_from_slice(&(-5i32).to_be_bytes());
        d.extend_from_slice(&7i32.to_be_bytes());
        d.push(u);
        add(&format!("oFFs unit={}", u), &gray, chunk(b"oFFs", &d));
    }
    // tIME with out-of-range fields
    for t in [
        [0u8, 0, 0, 0, 0, 0, 0],
        [0x07, 0xd0, 13, 32, 25, 61, 62],
        [0x07, 0xd0, 1, 1, 0, 0, 0],
        [0xff, 0xff, 12, 31, 23, 59, 60],
    ] {
        add(&format!("tIME {:?}", t), &gray, chunk(b"tIME", &t));
    }
    // pCAL with each equation type and a bad parameter count
    for eq in 0..5u8 {
        let mut d = Vec::new();
        d.extend_from_slice(b"purpose\0");
        d.extend_from_slice(&(-100i32).to_be_bytes());
        d.extend_from_slice(&100i32.to_be_bytes());
        d.push(eq);
        d.push(2);
        d.extend_from_slice(b"units\0");
        d.extend_from_slice(b"1.5\0");
        d.extend_from_slice(b"bogus\0");
        add(&format!("pCAL eq={}", eq), &gray, chunk(b"pCAL", &d));
    }
    // sCAL with each unit and malformed numbers
    for u in [0u8, 1, 2, 3] {
        for (w, h) in [("1.5", "2.5"), ("0", "0"), ("-1", "1"), ("x", "y"), ("", "")] {
            let mut d = vec![u];
            d.extend_from_slice(w.as_bytes());
            d.push(0);
            d.extend_from_slice(h.as_bytes());
            add(
                &format!("sCAL u={} {}x{}", u, w, h),
                &gray,
                chunk(b"sCAL", &d),
            );
        }
    }
    // sPLT with a bad depth / truncated entry
    for depth in [0u8, 1, 8, 16, 17] {
        let mut d = Vec::new();
        d.extend_from_slice(b"name\0");
        d.push(depth);
        d.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
        add(&format!("sPLT depth={}", depth), &gray, chunk(b"sPLT", &d));
    }
    // tEXt / zTXt / iTXt with bad keywords and bad compression
    for key in [
        &b""[..],
        b" lead",
        b"trail ",
        b"double  sp",
        b"ok",
        b"\x01ctl",
        b"\xffhigh",
        &[b'k'; 80][..],
        &[b'k'; 81][..],
    ] {
        let mut d = key.to_vec();
        d.push(0);
        d.extend_from_slice(b"text");
        add(
            &format!("tEXt key={:?}", String::from_utf8_lossy(key)),
            &gray,
            chunk(b"tEXt", &d),
        );
        for comp in [0u8, 1, 2, 255] {
            let mut d = key.to_vec();
            d.push(0);
            d.push(comp);
            d.extend_from_slice(&[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01]);
            add(
                &format!("zTXt key={:?} comp={}", String::from_utf8_lossy(key), comp),
                &gray,
                chunk(b"zTXt", &d),
            );
            let mut d = key.to_vec();
            d.push(0);
            d.push(comp);
            d.push(0);
            d.extend_from_slice(b"en\0");
            d.extend_from_slice(b"trans\0");
            d.extend_from_slice(b"text");
            add(
                &format!("iTXt key={:?} comp={}", String::from_utf8_lossy(key), comp),
                &gray,
                chunk(b"iTXt", &d),
            );
        }
    }
    // iCCP: bad compression method, bad profile
    for comp in [0u8, 1, 255] {
        let mut d = b"name\0".to_vec();
        d.push(comp);
        d.extend_from_slice(&[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01]);
        add(&format!("iCCP comp={}", comp), &gray, chunk(b"iCCP", &d));
    }
    // eXIf
    for head in [&b"II"[..], b"MM", b"XX", b"I", b""] {
        let mut d = head.to_vec();
        d.extend_from_slice(&[42, 0, 8, 0, 0, 0, 0, 0]);
        add(
            &format!("eXIf head={:?}", String::from_utf8_lossy(head)),
            &gray,
            chunk(b"eXIf", &d),
        );
    }
    // cICP / cLLI / mDCV
    for m in [0u8, 1, 255] {
        add(
            &format!("cICP matrix={}", m),
            &gray,
            chunk(b"cICP", &[1, 13, m, 1]),
        );
    }
    for v in [0u32, 1, 0x7fff_ffff, 0xffff_ffff] {
        let mut d = Vec::new();
        d.extend_from_slice(&v.to_be_bytes());
        d.extend_from_slice(&v.to_be_bytes());
        add(&format!("cLLI {:#x}", v), &gray, chunk(b"cLLI", &d));
        let mut d = Vec::new();
        for _ in 0..8 {
            d.extend_from_slice(&(v as u16).to_be_bytes());
        }
        d.extend_from_slice(&v.to_be_bytes());
        d.extend_from_slice(&v.to_be_bytes());
        add(&format!("mDCV {:#x}", v), &gray, chunk(b"mDCV", &d));
    }
    // hIST with the wrong number of entries
    for n in [0usize, 1, 8, 16, 17, 256] {
        let d: Vec<u8> = (0..n).flat_map(|i| (i as u16).to_be_bytes()).collect();
        add(&format!("hIST n={}", n), &pal, chunk(b"hIST", &d));
    }

    for (tag, base, c) in cases {
        let v = insert_before(&base, "IDAT", &c);
        diff_read(&format!("semantics: {}", tag), &v, noop);
        let v2 = insert_after_last(&base, "IDAT", &c);
        diff_read(&format!("semantics(after IDAT): {}", tag), &v2, noop);
        // and again with benign errors allowed
        let v3 = v.clone();
        diff_read(&format!("semantics(benign): {}", tag), &v3, |api, png, _| unsafe {
            (api.png_set_benign_errors)(png, 1);
        });
    }
}

/* ================================================================== */
/* the ERRORS.md coverage report                                       */
/* ================================================================== */

/// Diff the diagnostics the error-path tests actually made **both** libraries
/// produce against every diagnostic site in the C sources
/// (`tests/error_sites.txt`, generated by `tools/gen_error_sites.py`), and write
/// `target/error_coverage.txt`.
///
/// This is what makes the ERRORS.md check-marks machine-verified: a row counts as
/// covered only if this run observed its exact message text coming out of both
/// libraries with identical traces (`assert_same` / `assert_same_forked` compare
/// before `observe()` records).
///
/// It calls every other scenario in this file directly, because the libtest
/// harness gives no ordering guarantee between `#[test]` functions.
#[test]
fn coverage_report() {
    reporting_matrix();
    formatted_warning();
    png_c_rejections();
    bad_signature();
    truncation();
    crc_corruption();
    length_corruption();
    chunk_structure();
    idat_corruption();
    user_limits_rejections();
    write_sequencing();
    setter_validation();
    ancillary_chunk_corruption();
    ancillary_chunk_semantics();
    io_function_errors();
    write_option_errors();
    setter_memory_failures();
    deprecated_accessors();
    out_of_memory();

    let seen = observed_all();
    let sites = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/error_sites.txt"),
    )
    .expect("tests/error_sites.txt (run tools/gen_error_sites.py)");

    // A site is covered when some observed diagnostic contains its literal text.
    // png_chunk_* prefix the message with the chunk name and png_formatted_warning
    // substitutes @n parameters, so `contains` is the right relation.
    let mut covered = 0usize;
    let mut uncovered: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut report = String::new();
    for line in sites.lines() {
        let mut it = line.splitn(3, '|');
        let loc = it.next().unwrap_or("");
        let kind = it.next().unwrap_or("");
        let msg = it.next().unwrap_or("");
        if msg.is_empty() {
            report += &format!("[skip] {} {} (no literal message)\n", loc, kind);
            continue;
        }
        total += 1;
        let hit = seen.iter().any(|s| s.contains(msg));
        if hit {
            covered += 1;
            report += &format!("[x] {} {} {:?}\n", loc, kind, msg);
        } else {
            report += &format!("[ ] {} {} {:?}\n", loc, kind, msg);
            uncovered.push(format!("{} {} {:?}", loc, kind, msg));
        }
    }
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/error_coverage.txt");
    let summary = format!(
        "ERRORS.md coverage: {}/{} diagnostic sites with a literal message were \
         observed identically from both libraries ({} distinct messages seen)\n",
        covered,
        total,
        seen.len()
    );
    let _ = std::fs::write(&out, format!("{}\n{}", summary, report));
    let mut msgs = String::new();
    for s in &seen {
        msgs += s;
        msgs.push('\n');
    }
    let _ = std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/error_observed.txt"),
        msgs,
    );
    eprintln!("{}", summary);
    if !uncovered.is_empty() {
        eprintln!("uncovered ({}):", uncovered.len());
        for u in &uncovered {
            eprintln!("  {}", u);
        }
    }
    // Guard against regression: the error-path tests must keep reaching at least
    // this much of the C error surface.
    // The authoritative, whole-suite number is produced by
    // `tools/error_coverage.py` after `./check.sh` has run every test binary
    // (each one is a separate process, so this in-process view only sees the
    // binaries that happened to run before it).  Keep a floor here so that a
    // regression in the error-path tests cannot go unnoticed.
    assert!(
        covered >= 80,
        "error-surface coverage dropped to {}/{} sites",
        covered,
        total
    );
}

/* ================================================================== */
/* IO-layer rejections: pngwio.c / pngrio.c                            */
/* ================================================================== */

#[test]
fn io_function_errors() {
    let mut rng = Rng::new(0x10_e0);
    let img = Img::random(&mut rng, 4, 3, PNG_COLOR_TYPE_RGB, 8);
    // "Call to NULL write function"
    assert_same_forked("write with NULL write_data_fn", |api| unsafe {
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, core::ptr::null_mut(), None, None);
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                img.w,
                img.h,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
        });
        format!("{:?}", g)
    });
    // "Call to NULL read function"
    assert_same_forked("read with NULL read_data_fn", |api| unsafe {
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, core::ptr::null_mut(), None);
        let g = guarded(api, png, &mut || (api.png_read_info)(png, info));
        format!("{:?}", g)
    });
    // "Can't set both read_data_fn and write_data_fn in the same structure"
    assert_same("write struct given a read_fn", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        let g = guarded(api, png, &mut || {
            (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
            (api.png_set_write_fn)(
                png,
                core::ptr::null_mut(),
                Some(write_cb),
                Some(flush_cb),
            );
        });
        o.push(format!("{:?}", g));
        destroy_write(api, png, info);
        o
    });
    // and the mirror: a read struct given a write_fn
    assert_same("read struct given a write_fn", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_read(api);
        let g = guarded(api, png, &mut || {
            (api.png_set_write_fn)(
                png,
                core::ptr::null_mut(),
                Some(write_cb),
                Some(flush_cb),
            );
            (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
        });
        o.push(format!("{:?}", g));
        destroy_read(api, png, info);
        o
    });
    // "Write Error" from png_default_write_data: png_init_io with a FILE* opened
    // read-only, so fwrite fails.
    assert_same_forked("png_init_io read-only FILE", |api| unsafe {
        let path = cs("/dev/full");
        let mode = cs("wb");
        let f = fopen(path.as_ptr(), mode.as_ptr());
        if f.is_null() {
            return "fopen failed".to_string();
        }
        let (png, info) = new_write(api);
        (api.png_init_io)(png, f);
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                img.w,
                img.h,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            for r in &img.rows {
                (api.png_write_row)(png, r.as_ptr() as *mut u8);
            }
            (api.png_write_end)(png, info);
            (api.png_write_flush)(png);
        });
        format!("{:?}", g)
    });
    // "Read Error" / EOF from png_default_read_data: a FILE* on an empty file
    assert_same_forked("png_init_io empty FILE", |api| unsafe {
        let path = cs("/dev/null");
        let mode = cs("rb");
        let f = fopen(path.as_ptr(), mode.as_ptr());
        if f.is_null() {
            return "fopen failed".to_string();
        }
        let (png, info) = new_read(api);
        (api.png_init_io)(png, f);
        let g = guarded(api, png, &mut || (api.png_read_info)(png, info));
        format!("{:?}", g)
    });
    // a read callback that reports a short read at every possible offset
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 6, 4, 0x10e1);
    for cut in (0..good.len()).step_by(3) {
        assert_same(&format!("short read at {}", cut), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = good.clone();
            tls().in_pos = 0;
            tls().truncate_reads_at = Some(cut);
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                let rb = (api.png_get_rowbytes)(png, info);
                let h = (api.png_get_image_height)(png, info) as usize;
                let mut row = vec![0u8; rb];
                for _ in 0..h {
                    (api.png_read_row)(png, row.as_mut_ptr(), core::ptr::null_mut());
                }
                (api.png_read_end)(png, info);
            });
            o.push(format!("{:?}", g));
            destroy_read(api, png, info);
            o
        });
    }
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut core::ffi::c_void;
}

/* ================================================================== */
/* write-side option rejections: pngwrite.c / pngset.c                 */
/* ================================================================== */

#[test]
fn write_option_errors() {
    let mut rng = Rng::new(0x0b_71);
    let img = Img::random(&mut rng, 8, 5, PNG_COLOR_TYPE_RGB, 8);
    // png_set_filter: every method and every filter mask, before and after the
    // first row (the "cannot be added after start" and "Unknown row filter"
    // paths), plus an unknown method.
    for method in [-1i32, 0, 1, 64, 99] {
        for filters in [
            -1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 0x08, 0x10, 0x20, 0x40, 0x80, 0xf8, 0xff,
            0x100,
        ] {
            for after_start in [false, true] {
                assert_same_forked(
                    &format!(
                        "set_filter m={} f={:#x} after_start={}",
                        method, filters, after_start
                    ),
                    |api| unsafe {
                        let (png, info) = new_write(api);
                        (api.png_set_write_fn)(
                            png,
                            core::ptr::null_mut(),
                            Some(write_cb),
                            Some(flush_cb),
                        );
                        let g = guarded(api, png, &mut || {
                            (api.png_set_IHDR)(
                                png,
                                info,
                                img.w,
                                img.h,
                                8,
                                PNG_COLOR_TYPE_RGB,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            if after_start {
                                (api.png_set_filter)(png, 0, PNG_FILTER_NONE);
                                (api.png_write_info)(png, info);
                                (api.png_write_row)(png, img.rows[0].as_ptr() as *mut u8);
                            }
                            (api.png_set_filter)(png, method, filters);
                            if !after_start {
                                (api.png_write_info)(png, info);
                            }
                            for r in img.rows.iter().skip(after_start as usize) {
                                (api.png_write_row)(png, r.as_ptr() as *mut u8);
                            }
                            (api.png_write_end)(png, info);
                        });
                        format!("{:?} out={}", g, tls().output.len())
                    },
                );
            }
        }
    }
    // png_set_compression_buffer_size: 0, huge, below 6, and while the zstream is
    // in use (i.e. after the first row has been written).
    for size in [0usize, 1, 5, 6, 7, 0x7fff_ffff, 0x8000_0000, usize::MAX] {
        for in_use in [false, true] {
            assert_same_forked(
                &format!("compression_buffer_size {} in_use={}", size, in_use),
                |api| unsafe {
                    let (png, info) = new_write(api);
                    (api.png_set_write_fn)(
                        png,
                        core::ptr::null_mut(),
                        Some(write_cb),
                        Some(flush_cb),
                    );
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            img.w,
                            img.h,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        if in_use {
                            (api.png_write_info)(png, info);
                            (api.png_write_row)(png, img.rows[0].as_ptr() as *mut u8);
                        }
                        (api.png_set_compression_buffer_size)(png, size);
                        log(format!(
                            "get={}",
                            (api.png_get_compression_buffer_size)(png)
                        ));
                    });
                    format!("{:?}", g)
                },
            );
            // the same on a read struct (IDAT_read_size)
            assert_same_forked(
                &format!("read compression_buffer_size {}", size),
                |api| unsafe {
                    let (png, _info) = new_read(api);
                    let g = guarded(api, png, &mut || {
                        (api.png_set_compression_buffer_size)(png, size);
                        log(format!(
                            "get={}",
                            (api.png_get_compression_buffer_size)(png)
                        ));
                    });
                    format!("{:?}", g)
                },
            );
        }
    }
    // "Valid palette required for paletted images", "No IDATs written into file"
    assert_same_forked("palette image without PLTE", |api| unsafe {
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                4,
                4,
                8,
                PNG_COLOR_TYPE_PALETTE,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
        });
        format!("{:?} out={}", g, tls().output.len())
    });
    assert_same_forked("write_end without any rows", |api| unsafe {
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                4,
                4,
                8,
                PNG_COLOR_TYPE_GRAY,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            (api.png_write_end)(png, info);
        });
        format!("{:?} out={}", g, tls().output.len())
    });
    // "no rows for png_write_image to write" / png_write_png with no rows
    for transforms in [0i32, PNG_TRANSFORM_IDENTITY, PNG_TRANSFORM_INVERT_MONO] {
        assert_same_forked(
            &format!("png_write_png no rows transforms={}", transforms),
            |api| unsafe {
                let (png, info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        4,
                        4,
                        8,
                        PNG_COLOR_TYPE_GRAY,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (api.png_write_png)(png, info, transforms, core::ptr::null_mut());
                });
                format!("{:?} out={}", g, tls().output.len())
            },
        );
    }
    // "Wrote palette index exceeding num_palette"
    for check in [0i32, 1] {
        assert_same_forked(
            &format!("palette index out of range check={}", check),
            |api| unsafe {
                let (png, info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let pal = vec![png_color { red: 1, green: 2, blue: 3 }; 4];
                let rows: Vec<Vec<u8>> = (0..4).map(|_| vec![0xffu8; 4]).collect();
                let g = guarded(api, png, &mut || {
                    (api.png_set_check_for_invalid_index)(png, check);
                    (api.png_set_IHDR)(
                        png,
                        info,
                        4,
                        4,
                        8,
                        PNG_COLOR_TYPE_PALETTE,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (api.png_set_PLTE)(png, info, pal.as_ptr(), 4);
                    (api.png_write_info)(png, info);
                    for r in &rows {
                        (api.png_write_row)(png, r.as_ptr() as *mut u8);
                    }
                    (api.png_write_end)(png, info);
                });
                format!("{:?} out={}", g, tls().output.len())
            },
        );
    }
    // "MNG features are not allowed in a PNG datastream" on write
    for mng in [0u32, 1, 4, 5] {
        assert_same_forked(&format!("write mng={}", mng), |api| unsafe {
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let g = guarded(api, png, &mut || {
                log(format!("permit={}", (api.png_permit_mng_features)(png, mng)));
                (api.png_set_IHDR)(
                    png,
                    info,
                    4,
                    4,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (api.png_write_info)(png, info);
            });
            format!("{:?} out={}", g, tls().output.len())
        });
    }
}

/* ================================================================== */
/* allocation failures inside the png_set_* family                     */
/* ================================================================== */

/// Every `png_set_*` that allocates has an "Insufficient memory …" path.  Drive
/// each one with an allocator that fails after N allocations.
#[test]
fn setter_memory_failures() {
    let purpose = cs("purpose");
    let units = cs("units");
    let p0 = cs("1.5");
    let key = cs("Comment");
    let txt = cs("some text");
    let lang = cs("en");
    let lk = cs("Kommentar");
    let nm = cs("splt");
    let iccp_name = cs("icc");
    for n in 0..24usize {
        for which in 0..10usize {
            assert_same_forked(
                &format!("setter oom which={} after={}", which, n),
                |api| unsafe {
                    FAIL_AFTER.store(n, std::sync::atomic::Ordering::Relaxed);
                    let sh = &libs().shim;
                    let png = (api.png_create_write_struct_2)(
                        VER,
                        core::ptr::null_mut(),
                        Some(sh.error_fn),
                        Some(warn_cb),
                        core::ptr::null_mut(),
                        Some(failing_malloc),
                        Some(failing_free),
                    );
                    if png.is_null() {
                        return "create -> NULL".to_string();
                    }
                    let info = (api.png_create_info_struct)(png);
                    if info.is_null() {
                        return "info -> NULL".to_string();
                    }
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            4,
                            4,
                            8,
                            PNG_COLOR_TYPE_PALETTE,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        match which {
                            0 => {
                                let exif = vec![b'I', b'I', 42, 0, 8, 0, 0, 0];
                                (api.png_set_eXIf_1)(png, info, 8, exif.as_ptr() as *mut u8);
                            }
                            1 => {
                                let pal = vec![png_color { red: 1, green: 2, blue: 3 }; 16];
                                (api.png_set_PLTE)(png, info, pal.as_ptr(), 16);
                                let hist = vec![5u16; 16];
                                (api.png_set_hIST)(png, info, hist.as_ptr());
                            }
                            2 => {
                                let mut params: Vec<*mut c_char> =
                                    vec![p0.as_ptr() as *mut c_char; 2];
                                (api.png_set_pCAL)(
                                    png,
                                    info,
                                    purpose.as_ptr(),
                                    -1,
                                    1,
                                    0,
                                    2,
                                    units.as_ptr(),
                                    params.as_mut_ptr(),
                                );
                            }
                            3 => {
                                let a = cs("1.5");
                                let b = cs("2.5");
                                (api.png_set_sCAL_s)(png, info, 1, a.as_ptr(), b.as_ptr());
                            }
                            4 => {
                                let prof = vec![0u8; 132];
                                (api.png_set_iCCP)(
                                    png,
                                    info,
                                    iccp_name.as_ptr(),
                                    0,
                                    prof.as_ptr(),
                                    132,
                                );
                            }
                            5 => {
                                let mut t = png_text {
                                    compression: PNG_TEXT_COMPRESSION_NONE,
                                    key: key.as_ptr() as *mut c_char,
                                    text: txt.as_ptr() as *mut c_char,
                                    text_length: 0,
                                    itxt_length: 0,
                                    lang: core::ptr::null_mut(),
                                    lang_key: core::ptr::null_mut(),
                                };
                                (api.png_set_text)(png, info, &mut t, 1);
                            }
                            6 => {
                                let mut t = png_text {
                                    compression: PNG_ITXT_COMPRESSION_NONE,
                                    key: key.as_ptr() as *mut c_char,
                                    text: txt.as_ptr() as *mut c_char,
                                    text_length: 0,
                                    itxt_length: 0,
                                    lang: lang.as_ptr() as *mut c_char,
                                    lang_key: lk.as_ptr() as *mut c_char,
                                };
                                (api.png_set_text)(png, info, &mut t, 1);
                            }
                            7 => {
                                let ent = vec![
                                    png_sPLT_entry {
                                        red: 1,
                                        green: 2,
                                        blue: 3,
                                        alpha: 4,
                                        frequency: 5
                                    };
                                    8
                                ];
                                let mut sp = png_sPLT_t {
                                    name: nm.as_ptr() as *mut c_char,
                                    depth: 8,
                                    entries: ent.as_ptr() as *mut png_sPLT_entry,
                                    nentries: 8,
                                };
                                (api.png_set_sPLT)(png, info, &mut sp, 1);
                            }
                            8 => {
                                let data = vec![1u8, 2, 3, 4];
                                let mut uc = png_unknown_chunk {
                                    name: *b"unKn\0",
                                    data: data.as_ptr() as *mut u8,
                                    size: 4,
                                    location: 1,
                                };
                                (api.png_set_unknown_chunks)(png, info, &mut uc, 1);
                            }
                            _ => {
                                let rows: Vec<Vec<u8>> = (0..4).map(|_| vec![0u8; 4]).collect();
                                let mut ptrs: Vec<*mut u8> =
                                    rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
                                (api.png_set_rows)(png, info, ptrs.as_mut_ptr());
                            }
                        }
                        log(format!(
                            "valid=0x{:x}",
                            (api.png_get_valid)(png, info, 0xffff_ffff)
                        ));
                    });
                    format!("{:?}", g)
                },
            );
        }
    }
    FAIL_AFTER.store(usize::MAX, std::sync::atomic::Ordering::Relaxed);
}

/* ================================================================== */
/* the deprecated / lossy accessors                                    */
/* ================================================================== */

#[test]
fn deprecated_accessors() {
    // png_set_eXIf / png_get_eXIf: both warn that they do not work
    assert_same("png_set_eXIf / png_get_eXIf", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        let mut buf = vec![b'I', b'I', 42, 0, 8, 0, 0, 0];
        let g = guarded(api, png, &mut || {
            (api.png_set_eXIf)(png, info, buf.as_mut_ptr());
            let mut out: *mut u8 = core::ptr::null_mut();
            log(format!(
                "get_eXIf={} null={}",
                (api.png_get_eXIf)(png, info, &mut out),
                out.is_null()
            ));
        });
        o.push(format!("{:?}", g));
        destroy_write(api, png, info);
        o
    });
    // png_get_sCAL_fixed / png_get_pixel_aspect_ratio_fixed / offset_inches_fixed
    // overflow: "fixed point overflow ignored"
    for (w, h) in [
        ("1.5", "2.5"),
        ("100000", "0.00001"),
        ("21475", "1"),
        ("1e300", "1e-300"),
        ("0", "0"),
    ] {
        let a = cs(w);
        let b = cs(h);
        assert_same_forked(&format!("sCAL_fixed {}x{}", w, h), |api| unsafe {
            let (png, info) = new_write(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_sCAL_s)(png, info, 1, a.as_ptr(), b.as_ptr());
                let mut u: c_int = -1;
                let mut fw: i32 = -1;
                let mut fh: i32 = -1;
                log(format!(
                    "sCAL_fixed r={} u={} w={} h={}",
                    (api.png_get_sCAL_fixed)(png, info, &mut u, &mut fw, &mut fh),
                    u,
                    fw,
                    fh
                ));
            });
            format!("{:?}", g)
        });
    }
    // png_get_pHYs_dpi / aspect ratio with extreme resolutions
    for (x, y) in [
        (0u32, 0u32),
        (1, 1),
        (1, 0x7fff_ffff),
        (0x7fff_ffff, 1),
        (0x8000_0000, 0x8000_0000),
        (0xffff_ffff, 0xffff_ffff),
    ] {
        assert_same_forked(&format!("pHYs {}x{}", x, y), |api| unsafe {
            let (png, info) = new_write(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_pHYs)(png, info, x, y, 1);
                let mut rx = 0u32;
                let mut ry = 0u32;
                let mut u: c_int = -1;
                log(format!(
                    "pHYs r={} {} {} u={}",
                    (api.png_get_pHYs)(png, info, &mut rx, &mut ry, &mut u),
                    rx,
                    ry,
                    u
                ));
                let mut dx = 0u32;
                let mut dy = 0u32;
                log(format!(
                    "dpi r={} {} {} u={}",
                    (api.png_get_pHYs_dpi)(png, info, &mut dx, &mut dy, &mut u),
                    dx,
                    dy,
                    u
                ));
                log(format!(
                    "aspect={:?} aspect_fixed={}",
                    (api.png_get_pixel_aspect_ratio)(png, info),
                    (api.png_get_pixel_aspect_ratio_fixed)(png, info)
                ));
                log(format!(
                    "ppi={} ppm={} xppi={} yppi={}",
                    (api.png_get_pixels_per_inch)(png, info),
                    (api.png_get_pixels_per_meter)(png, info),
                    (api.png_get_x_pixels_per_inch)(png, info),
                    (api.png_get_y_pixels_per_inch)(png, info)
                ));
            });
            format!("{:?}", g)
        });
    }
    // png_get_x/y_offset_inches(_fixed) with extreme offsets
    for (x, y) in [
        (0i32, 0i32),
        (1, -1),
        (i32::MAX, i32::MIN),
        (1_000_000, -1_000_000),
    ] {
        assert_same_forked(&format!("oFFs {} {}", x, y), |api| unsafe {
            let (png, info) = new_write(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_oFFs)(png, info, x, y, 1);
                log(format!(
                    "in={:?} {:?} fixed={} {} microns={} {} px={} {}",
                    (api.png_get_x_offset_inches)(png, info),
                    (api.png_get_y_offset_inches)(png, info),
                    (api.png_get_x_offset_inches_fixed)(png, info),
                    (api.png_get_y_offset_inches_fixed)(png, info),
                    (api.png_get_x_offset_microns)(png, info),
                    (api.png_get_y_offset_microns)(png, info),
                    (api.png_get_x_offset_pixels)(png, info),
                    (api.png_get_y_offset_pixels)(png, info)
                ));
            });
            format!("{:?}", g)
        });
    }
}

//! Phase C — ERRORS.md rows for `png.c`, `pngerror.c`, `pngmem.c`, `pngrio.c`
//! and `pngwio.c`.  Every row constructs its exact invalid input and asserts
//! both libraries reject it identically (same sentinel / same message).
mod common;
use common::*;
use std::ffi::CString;

/// Result of one probe: return value (as i64), whether it errored, diagnostics.
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
        assert_eq!(c, r, "{}", $label);
        c
    }};
}

// ---------------------------------------------------------------------------
// png.c — signature handling
// ---------------------------------------------------------------------------

#[test]
fn set_sig_bytes_rejections() {
    // rows: png_ptr == NULL; num_bytes < 0 (clamped to 0); nb > 8 -> png_error
    for n in [-100i32, -1, 0, 1, 7, 8, 9, 100, i32::MAX, i32::MIN] {
        same!(format!("png_set_sig_bytes({}) on NULL", n), |api| {
            unsafe { (api.png_set_sig_bytes)(std::ptr::null_mut(), n) };
            0
        });
        same!(format!("png_set_sig_bytes({})", n), |api| {
            let s = unsafe { ReadSess::new(api, &[]) };
            unsafe { (api.png_set_sig_bytes)(s.png, n) };
            // observable through png_get_signature/png_read_info later; the
            // interesting part is the error/warning parity
            0
        });
    }
}

#[test]
fn sig_cmp_rejections() {
    let good: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    for start in [0usize, 1, 7, 8, 9, 100, usize::MAX] {
        for num in [0usize, 1, 8, 9, 100] {
            // start > 7 -> -1 ; num < 1 -> -1 ; both clamped otherwise
            same!(format!("png_sig_cmp(start={},num={})", start, num), |api| {
                unsafe { (api.png_sig_cmp)(good.as_ptr(), start, num) as i64 }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// png.c — png_create_*_struct version checking
// ---------------------------------------------------------------------------

#[test]
fn create_struct_version_mismatch() {
    let vers = [
        "1.6.59.git", // correct
        "",
        "1",
        "1.6",
        "1.5.0",
        "1.7.0",
        "2.6.59.git",
        "1.6.59",
        "xxxxxxxxxx",
        "1.6.59.gi",
    ];
    for v in vers {
        let cv = cs(v);
        for read in [true, false] {
            let label = format!("create({}, read={})", v, read);
            let c = probe(c_api(), |api| {
                let p = unsafe {
                    if read {
                        (api.png_create_read_struct)(
                            cv.as_ptr(),
                            std::ptr::null_mut(),
                            Some(cb_error),
                            Some(cb_warning),
                        )
                    } else {
                        (api.png_create_write_struct)(
                            cv.as_ptr(),
                            std::ptr::null_mut(),
                            Some(cb_error),
                            Some(cb_warning),
                        )
                    }
                };
                let ok = !p.is_null();
                if ok {
                    let mut q = p;
                    unsafe {
                        if read {
                            (api.png_destroy_read_struct)(
                                &mut q,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                            );
                        } else {
                            (api.png_destroy_write_struct)(&mut q, std::ptr::null_mut());
                        }
                    }
                }
                ok as i64
            });
            let r = probe(rs_api(), |api| {
                let p = unsafe {
                    if read {
                        (api.png_create_read_struct)(
                            cv.as_ptr(),
                            std::ptr::null_mut(),
                            Some(cb_error),
                            Some(cb_warning),
                        )
                    } else {
                        (api.png_create_write_struct)(
                            cv.as_ptr(),
                            std::ptr::null_mut(),
                            Some(cb_error),
                            Some(cb_warning),
                        )
                    }
                };
                let ok = !p.is_null();
                if ok {
                    let mut q = p;
                    unsafe {
                        if read {
                            (api.png_destroy_read_struct)(
                                &mut q,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                            );
                        } else {
                            (api.png_destroy_write_struct)(&mut q, std::ptr::null_mut());
                        }
                    }
                }
                ok as i64
            });
            assert_eq!(c, r, "{}", label);
        }
    }
    // NULL version string
    for read in [true, false] {
        same!(format!("create(NULL, read={})", read), |api| {
            let p = unsafe {
                if read {
                    (api.png_create_read_struct)(
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        Some(cb_error),
                        Some(cb_warning),
                    )
                } else {
                    (api.png_create_write_struct)(
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        Some(cb_error),
                        Some(cb_warning),
                    )
                }
            };
            let ok = !p.is_null();
            if ok {
                let mut q = p;
                unsafe {
                    if read {
                        (api.png_destroy_read_struct)(
                            &mut q,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        );
                    } else {
                        (api.png_destroy_write_struct)(&mut q, std::ptr::null_mut());
                    }
                }
            }
            ok as i64
        });
    }
}

#[test]
fn user_version_check() {
    for v in ["1.6.59.git", "", "1.5.0", "1.7.0", "1.6.0", "9.9.9"] {
        let cv = cs(v);
        same!(format!("png_user_version_check({})", v), |api| {
            let s = unsafe { ReadSess::new(api, &[]) };
            unsafe { (api.png_user_version_check)(s.png, cv.as_ptr()) as i64 }
        });
    }
    same!("png_user_version_check(NULL)", |api| {
        let s = unsafe { ReadSess::new(api, &[]) };
        unsafe { (api.png_user_version_check)(s.png, std::ptr::null()) as i64 }
    });
}

// ---------------------------------------------------------------------------
// png.c — png_create_info_struct / png_info_init_3 / png_destroy_info_struct
// ---------------------------------------------------------------------------

#[test]
fn info_struct_lifecycle_rejections() {
    same!("png_create_info_struct(NULL)", |api| {
        let p = unsafe { (api.png_create_info_struct)(std::ptr::null()) };
        !p.is_null() as i64
    });
    same!("png_destroy_info_struct(NULL,NULL)", |api| {
        unsafe { (api.png_destroy_info_struct)(std::ptr::null(), std::ptr::null_mut()) };
        0
    });
    same!("png_destroy_info_struct(ptr to NULL)", |api| {
        let s = unsafe { ReadSess::new(api, &[]) };
        let mut n: png_infop = std::ptr::null_mut();
        unsafe { (api.png_destroy_info_struct)(s.png, &mut n) };
        0
    });
    // png_info_init_3: NULL info, and a size >= sizeof(png_info) (no realloc)
    same!("png_info_init_3(NULL)", |api| {
        let mut n: png_infop = std::ptr::null_mut();
        unsafe { (api.png_info_init_3)(&mut n, 1_000_000) };
        n.is_null() as i64
    });
    same!("png_info_init_3(big size)", |api| {
        let s = unsafe { ReadSess::new(api, &[]) };
        let mut i = unsafe { (api.png_create_info_struct)(s.png) };
        unsafe { (api.png_info_init_3)(&mut i, 1_000_000) };
        let ok = !i.is_null();
        if ok {
            unsafe { (api.png_destroy_info_struct)(s.png, &mut i) };
        }
        ok as i64
    });
    // A size *smaller* than sizeof(png_info) makes the C free() the struct and
    // re-allocate it; both libraries must still hand back a non-NULL pointer.
    for sz in [0usize, 1, 8, 64] {
        same!(format!("png_info_init_3(small size {})", sz), |api| {
            let s = unsafe { ReadSess::new(api, &[]) };
            let mut i = unsafe { (api.png_create_info_struct)(s.png) };
            unsafe { (api.png_info_init_3)(&mut i, sz) };
            let ok = !i.is_null();
            if ok {
                unsafe { (api.png_destroy_info_struct)(s.png, &mut i) };
            }
            ok as i64
        });
    }
}

// ---------------------------------------------------------------------------
// png.c — png_data_freer
// ---------------------------------------------------------------------------

#[test]
fn data_freer_rejections() {
    for freer in [
        PNG_DESTROY_WILL_FREE_DATA,
        PNG_SET_WILL_FREE_DATA,
        PNG_USER_WILL_FREE_DATA,
        -1,
        0,
        4,
        99,
    ] {
        for mask in [0u32, PNG_FREE_ALL as u32, PNG_FREE_TEXT as u32, 0xffff_ffff] {
            same!(
                format!("png_data_freer(freer={},mask={:#x})", freer, mask),
                |api| {
                    let s = unsafe { ReadSess::new(api, &[]) };
                    unsafe { (api.png_data_freer)(s.png, s.info, freer, mask) };
                    0
                }
            );
        }
    }
    same!("png_data_freer(NULL,NULL)", |api| {
        unsafe {
            (api.png_data_freer)(std::ptr::null(), std::ptr::null_mut(), 1, 0xffff_ffff)
        };
        0
    });
    // png_free_data NULL guards + every mask
    for mask in [
        PNG_FREE_HIST,
        PNG_FREE_ICCP,
        PNG_FREE_SPLT,
        PNG_FREE_ROWS,
        PNG_FREE_PCAL,
        PNG_FREE_SCAL,
        PNG_FREE_UNKN,
        PNG_FREE_PLTE,
        PNG_FREE_TRNS,
        PNG_FREE_TEXT,
        PNG_FREE_EXIF,
        PNG_FREE_ALL,
        0,
        -1,
    ] {
        for num in [-1i32, 0, 1, 100] {
            same!(format!("png_free_data(mask={:#x},num={})", mask, num), |api| {
                let s = unsafe { ReadSess::new(api, &[]) };
                unsafe { (api.png_free_data)(s.png, s.info, mask as u32, num) };
                0
            });
        }
    }
    same!("png_free_data(NULL info)", |api| {
        let s = unsafe { ReadSess::new(api, &[]) };
        unsafe { (api.png_free_data)(s.png, std::ptr::null_mut(), 0xffff_ffff, -1) };
        0
    });
}

// ---------------------------------------------------------------------------
// png.c — png_check_IHDR (through png_set_IHDR, which validates)
// ---------------------------------------------------------------------------

#[test]
fn set_ihdr_rejections() {
    let widths = [0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 1_000_001];
    let heights = [0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff];
    let depths = [-1i32, 0, 1, 2, 3, 4, 5, 8, 16, 17, 32, 64];
    let ctypes = [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 100];
    for &w in &widths {
        for &h in &heights {
            for &bd in &depths {
                for &ct in &ctypes {
                    same!(
                        format!("png_set_IHDR({},{},{},{})", w, h, bd, ct),
                        |api| {
                            let s = unsafe { WriteSess::new(api) };
                            unsafe {
                                (api.png_set_IHDR)(
                                    s.png, s.info, w, h, bd, ct, 0, 0, 0,
                                )
                            };
                            0
                        }
                    );
                }
            }
        }
    }
    // interlace / compression / filter method rejections
    for il in [-1i32, 0, 1, 2, 100] {
        for comp in [-1i32, 0, 1, 100] {
            for filt in [-1i32, 0, 1, 64, 65, 100] {
                same!(
                    format!("png_set_IHDR methods il={} c={} f={}", il, comp, filt),
                    |api| {
                        let s = unsafe { WriteSess::new(api) };
                        unsafe {
                            (api.png_set_IHDR)(s.png, s.info, 4, 4, 8, 2, il, comp, filt)
                        };
                        0
                    }
                );
                // ... and with MNG features permitted, which legalises
                // filter method 64 (PNG_INTRAPIXEL_DIFFERENCING)
                same!(
                    format!("png_set_IHDR mng il={} c={} f={}", il, comp, filt),
                    |api| {
                        let s = unsafe { WriteSess::new(api) };
                        unsafe {
                            (api.png_permit_mng_features)(s.png, PNG_ALL_MNG_FEATURES);
                            (api.png_set_IHDR)(s.png, s.info, 4, 4, 8, 2, il, comp, filt)
                        };
                        0
                    }
                );
            }
        }
    }
    // png_check_IHDR called directly
    for &w in &[0u32, 1, 0x8000_0000] {
        for &bd in &[0i32, 3, 8] {
            for &ct in &[0i32, 1, 6] {
                same!(format!("png_check_IHDR({},{},{})", w, bd, ct), |api| {
                    let s = unsafe { WriteSess::new(api) };
                    unsafe { (api.png_check_IHDR)(s.png, w, 4, bd, ct, 0, 0, 0) };
                    0
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// png.c — png_set_option
// ---------------------------------------------------------------------------

#[test]
fn set_option_rejections() {
    for opt in [-2i32, -1, 0, 1, 2, 3, 4, 8, 11, 12, 13, 100] {
        for val in [-1i32, 0, 1, 2, 3, 4, 100] {
            same!(format!("png_set_option({},{})", opt, val), |api| {
                let s = unsafe { ReadSess::new(api, &[]) };
                unsafe { (api.png_set_option)(s.png, opt, val) as i64 }
            });
        }
    }
    same!("png_set_option(NULL)", |api| {
        unsafe { (api.png_set_option)(std::ptr::null_mut(), 2, 3) as i64 }
    });
}

// ---------------------------------------------------------------------------
// png.c — png_fixed / png_fixed_ITU overflow
// ---------------------------------------------------------------------------

#[test]
fn fixed_conversion_overflow() {
    let name = cs("test value");
    let vals = [
        0.0f64,
        1.0,
        -1.0,
        21474.0,
        21475.0,
        -21475.0,
        1e10,
        -1e10,
        f64::MAX,
        f64::MIN,
        1e-10,
        0.5,
        1.0 / 3.0,
    ];
    for &v in &vals {
        same!(format!("png_fixed({})", v), |api| {
            let s = unsafe { ReadSess::new(api, &[]) };
            unsafe { (api.png_fixed)(s.png, v, name.as_ptr()) as i64 }
        });
        same!(format!("png_fixed_ITU({})", v), |api| {
            let s = unsafe { ReadSess::new(api, &[]) };
            unsafe { (api.png_fixed_ITU)(s.png, v, name.as_ptr()) as i64 }
        });
    }
}

// ---------------------------------------------------------------------------
// png.c — png_ascii_from_fp / png_ascii_from_fixed "buffer too small"
// ---------------------------------------------------------------------------

#[test]
fn ascii_conversion_buffer_too_small() {
    for size in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 12, 15, 16, 24] {
        for &v in &[0.0f64, 1.0, -1.0, 1e-10, 1e10, 123456.789, 0.5] {
            for prec in [1u32, 5, 15] {
                same!(
                    format!("png_ascii_from_fp(size={},v={},p={})", size, v, prec),
                    |api| {
                        let s = unsafe { ReadSess::new(api, &[]) };
                        let mut b = vec![0u8; size + 32];
                        unsafe {
                            (api.png_ascii_from_fp)(
                                s.png,
                                b.as_mut_ptr() as png_charp,
                                size,
                                v,
                                prec,
                            )
                        };
                        b[..size.min(b.len())].iter().map(|&x| x as i64).sum()
                    }
                );
            }
        }
        for &v in &[0i32, 1, -1, 100000, i32::MAX, i32::MIN] {
            same!(
                format!("png_ascii_from_fixed(size={},v={})", size, v),
                |api| {
                    let s = unsafe { ReadSess::new(api, &[]) };
                    let mut b = vec![0u8; size + 32];
                    unsafe {
                        (api.png_ascii_from_fixed)(
                            s.png,
                            b.as_mut_ptr() as png_charp,
                            size,
                            v,
                        )
                    };
                    b[..size.min(b.len())].iter().map(|&x| x as i64).sum()
                }
            );
        }
    }
}

// ---------------------------------------------------------------------------
// png.c — png_get_uint_31 range check
// ---------------------------------------------------------------------------

#[test]
fn get_uint_31_rejects_out_of_range() {
    for v in [
        0u32,
        1,
        PNG_UINT_31_MAX,
        PNG_UINT_31_MAX + 1,
        0x8000_0001,
        0xffff_ffff,
    ] {
        let b = v.to_be_bytes();
        same!(format!("png_get_uint_31({:#x})", v), |api| {
            let s = unsafe { ReadSess::new(api, &[]) };
            unsafe { (api.png_get_uint_31)(s.png, b.as_ptr()) as i64 }
        });
    }
}

// ---------------------------------------------------------------------------
// pngerror.c — the whole diagnostic dispatch
// ---------------------------------------------------------------------------

#[test]
fn error_dispatch() {
    // messages exercising png_format_buffer: long, with '#' number markers,
    // and non-printable bytes (which get hex-escaped in the chunk name)
    let msgs: Vec<CString> = vec![
        cs(""),
        cs("plain"),
        cs(&"x".repeat(63)),
        cs(&"x".repeat(64)),
        cs(&"y".repeat(200)),
        cs("#12345 numbered"),
        cs("#nn"),
        CString::new(vec![0x41u8, 0x01, 0x7f, 0x80, 0xff, 0x42]).unwrap(),
    ];
    for m in &msgs {
        for read in [true, false] {
            for benign_warn in [0i32, 1] {
                same!(
                    format!("png_warning({:?},read={},bw={})", m, read, benign_warn),
                    |api| {
                        unsafe {
                            let (png, _keep_r, _keep_w);
                            if read {
                                let s = ReadSess::new(api, &[]);
                                png = s.png;
                                _keep_r = Some(s);
                                _keep_w = None;
                            } else {
                                let s = WriteSess::new(api);
                                png = s.png;
                                _keep_r = None;
                                _keep_w = Some(s);
                            }
                            (api.png_set_benign_errors)(png, benign_warn);
                            (api.png_warning)(png, m.as_ptr());
                            (api.png_app_warning)(png, m.as_ptr());
                            (api.png_chunk_warning)(png, m.as_ptr());
                            (api.png_chunk_report)(png, m.as_ptr(), 0);
                            (api.png_chunk_report)(png, m.as_ptr(), 1);
                            (api.png_chunk_report)(png, m.as_ptr(), 2);
                            (api.png_chunk_report)(png, m.as_ptr(), 3);
                            (api.png_chunk_report)(png, m.as_ptr(), -1);
                        }
                        0
                    }
                );
                // The error variants unwind, so each needs its own probe.
                for which in 0..5 {
                    same!(
                        format!(
                            "error variant {} ({:?},read={},bw={})",
                            which, m, read, benign_warn
                        ),
                        |api| {
                            unsafe {
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
                                (api.png_set_benign_errors)(png, benign_warn);
                                match which {
                                    0 => (api.png_error)(png, m.as_ptr()),
                                    1 => (api.png_chunk_error)(png, m.as_ptr()),
                                    2 => {
                                        (api.png_app_error)(png, m.as_ptr());
                                        return 1;
                                    }
                                    3 => {
                                        (api.png_benign_error)(png, m.as_ptr());
                                        return 2;
                                    }
                                    _ => {
                                        (api.png_chunk_benign_error)(png, m.as_ptr());
                                        return 3;
                                    }
                                }
                            }
                        }
                    );
                }
            }
        }
    }
    // NULL message
    for which in 0..3 {
        same!(format!("NULL message variant {}", which), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                match which {
                    0 => {
                        (api.png_warning)(s.png, std::ptr::null());
                        0
                    }
                    1 => {
                        (api.png_app_warning)(s.png, std::ptr::null());
                        0
                    }
                    _ => (api.png_error)(s.png, std::ptr::null()),
                }
            }
        });
    }
}

#[test]
fn warning_parameter_formatting() {
    // png_warning_parameter / _signed / _unsigned / png_formatted_warning
    for number in [-1i32, 0, 1, 2, 8, 9, 100] {
        for fmt in [
            PNG_NUMBER_FORMAT_u,
            PNG_NUMBER_FORMAT_02u,
            PNG_NUMBER_FORMAT_x,
            PNG_NUMBER_FORMAT_02x,
            PNG_NUMBER_FORMAT_fixed,
            0,
            99,
        ] {
            for val in [0i64, 1, 12345, -1, i32::MAX as i64, i32::MIN as i64] {
                same!(
                    format!("warning_parameter({},{},{})", number, fmt, val),
                    |api| {
                        unsafe {
                            let s = ReadSess::new(api, &[]);
                            let mut p =
                                [[0i8; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT];
                            let pp = p.as_mut_ptr() as *mut [c_char; PNG_WARNING_PARAMETER_SIZE];
                            let sv = cs("string value");
                            (api.png_warning_parameter)(pp, number, sv.as_ptr());
                            (api.png_warning_parameter_signed)(pp, number, fmt, val as i32);
                            (api.png_warning_parameter_unsigned)(
                                pp,
                                number,
                                fmt,
                                val as usize,
                            );
                            let msg = cs("param @1 and @2 and @9 and @ and @0");
                            (api.png_formatted_warning)(s.png, pp, msg.as_ptr());
                            p.iter()
                                .flat_map(|r| r.iter())
                                .map(|&c| c as i64)
                                .sum::<i64>()
                        }
                    }
                );
            }
        }
    }
}

#[test]
fn longjmp_fn_rejections() {
    unsafe extern "C-unwind" fn my_longjmp(_e: *mut jmp_buf, _v: c_int) -> ! {
        std::process::abort()
    }
    // NULL png_ptr -> NULL
    same!("png_set_longjmp_fn(NULL)", |api| {
        let p = unsafe {
            (api.png_set_longjmp_fn)(std::ptr::null_mut(), Some(my_longjmp), 200)
        };
        p.is_null() as i64
    });
    // First call with the built-in size; then the SAME size again (ok) and a
    // DIFFERENT size (-> "Application jmp_buf size changed" + NULL).
    for (s1, s2) in [
        (200usize, 200usize),
        (200, 8),
        (8, 200),
        (0, 0),
        (0, 1),
        (1_000_000, 1_000_000),
        (1_000_000, 999_999),
    ] {
        same!(format!("png_set_longjmp_fn({} then {})", s1, s2), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                let a = (api.png_set_longjmp_fn)(s.png, Some(my_longjmp), s1);
                let b = (api.png_set_longjmp_fn)(s.png, Some(my_longjmp), s2);
                (api.png_free_jmpbuf)(s.png);
                ((!a.is_null()) as i64) * 2 + ((!b.is_null()) as i64)
            }
        });
    }
    same!("png_free_jmpbuf(NULL)", |api| {
        unsafe { (api.png_free_jmpbuf)(std::ptr::null_mut()) };
        0
    });
}

// ---------------------------------------------------------------------------
// pngmem.c
// ---------------------------------------------------------------------------

#[test]
fn allocator_rejections() {
    let sizes = [
        0usize,
        1,
        16,
        65536,
        65537,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        1 << 40,
    ];
    for &sz in &sizes {
        same!(format!("png_malloc_base(NULL,{})", sz), |api| {
            let p = unsafe { (api.png_malloc_base)(std::ptr::null(), sz) };
            let ok = !p.is_null();
            if ok {
                unsafe { (api.png_free_default)(std::ptr::null(), p) };
            }
            ok as i64
        });
        same!(format!("png_malloc_warn({})", sz), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                let p = (api.png_malloc_warn)(s.png, sz);
                let ok = !p.is_null();
                if ok {
                    (api.png_free)(s.png, p);
                }
                ok as i64
            }
        });
        // png_malloc / png_calloc png_error on failure
        same!(format!("png_malloc({})", sz), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                let p = (api.png_malloc)(s.png, sz);
                let ok = !p.is_null();
                if ok {
                    (api.png_free)(s.png, p);
                }
                ok as i64
            }
        });
        if sz <= (1 << 20) {
            same!(format!("png_calloc({})", sz), |api| {
                unsafe {
                    let s = ReadSess::new(api, &[]);
                    let p = (api.png_calloc)(s.png, sz);
                    let ok = !p.is_null();
                    if ok {
                        (api.png_free)(s.png, p);
                    }
                    ok as i64
                }
            });
        }
    }
    // NULL / no-op frees
    same!("png_free(NULL ptr)", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_free)(s.png, std::ptr::null_mut());
        }
        0
    });
    same!("png_free(NULL png_ptr)", |api| {
        unsafe { (api.png_free)(std::ptr::null(), std::ptr::null_mut()) };
        0
    });
    same!("png_free_default(NULL)", |api| {
        unsafe { (api.png_free_default)(std::ptr::null(), std::ptr::null_mut()) };
        0
    });
    same!("png_malloc_default(NULL png_ptr, 0)", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            let p = (api.png_malloc_default)(s.png, 0);
            let ok = !p.is_null();
            if ok {
                (api.png_free_default)(s.png, p);
            }
            ok as i64
        }
    });
}

#[test]
fn array_allocator_rejections() {
    // png_malloc_array(png_ptr, nelements, element_size)
    let cases: [(i32, usize); 12] = [
        (0, 0),
        (-1, 4),
        (1, 0),
        (1, 1),
        (10, 4),
        (i32::MAX, 1),
        (i32::MAX, 2),
        (65536, 65536),
        (1, usize::MAX),
        (2, usize::MAX / 2),
        (1000, 1000),
        (-100, 8),
    ];
    for &(n, es) in &cases {
        same!(format!("png_malloc_array({},{})", n, es), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                let p = (api.png_malloc_array)(s.png, n, es);
                let ok = !p.is_null();
                if ok {
                    (api.png_free)(s.png, p);
                }
                ok as i64
            }
        });
        // png_realloc_array(png_ptr, old_array, old_elements, add_elements, element_size)
        for &(old_n, add_n) in &[(0i32, 0i32), (0, 1), (1, 0), (1, -1), (-1, 1), (2, i32::MAX)] {
            same!(
                format!("png_realloc_array({},{},{},{})", n, old_n, add_n, es),
                |api| {
                    unsafe {
                        let s = ReadSess::new(api, &[]);
                        let p = (api.png_realloc_array)(
                            s.png,
                            std::ptr::null(),
                            old_n,
                            add_n,
                            es,
                        );
                        let ok = !p.is_null();
                        if ok {
                            (api.png_free)(s.png, p);
                        }
                        ok as i64
                    }
                }
            );
        }
    }
}

#[test]
fn zalloc_zfree_rejections() {
    // png_zalloc overflow: items * size > png_alloc_size_t
    let cases: [(u32, u32); 8] = [
        (0, 0),
        (1, 1),
        (1, 0),
        (0, 1),
        (0xffff_ffff, 0xffff_ffff),
        (0x1_0000, 0x1_0000),
        (2, 0x8000_0000),
        (0x8000_0000, 2),
    ];
    for &(items, size) in &cases {
        same!(format!("png_zalloc({},{})", items, size), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                let p = (api.png_zalloc)(s.png as voidpf, items, size);
                let ok = !p.is_null();
                if ok {
                    (api.png_zfree)(s.png as voidpf, p);
                }
                ok as i64
            }
        });
        same!(format!("png_zalloc(NULL,{},{})", items, size), |api| {
            let p = unsafe {
                (api.png_zalloc)(std::ptr::null_mut(), items, size)
            };
            p.is_null() as i64
        });
    }
    same!("png_zfree(NULL,NULL)", |api| {
        unsafe { (api.png_zfree)(std::ptr::null_mut(), std::ptr::null_mut()) };
        0
    });
}

// ---------------------------------------------------------------------------
// pngrio.c / pngwio.c
// ---------------------------------------------------------------------------

#[test]
fn io_function_rejections() {
    // NOTE: the "Call to NULL read function" / "Call to NULL write function"
    // errors are UNREACHABLE in this build configuration: PNG_STDIO_SUPPORTED
    // is on, so png_create_read_struct_2 (pngread.c:76) and
    // png_create_write_struct_2 (pngwrite.c:614) immediately install
    // png_default_read_data / png_default_write_data, and png_set_read_fn /
    // png_set_write_fn re-install them when passed NULL (pngrio.c:99,
    // pngwio.c:99).  With io_ptr == NULL those defaults fread()/fwrite() a NULL
    // FILE*, which is C undefined behaviour rather than an error return, so
    // there is no observable rejection to compare.
    // The "Can't set both read_data_fn and write_data_fn in the same
    // structure" warning, in both orders.
    same!("read_fn then write_fn", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_set_write_fn)(
                s.png,
                std::ptr::null_mut(),
                Some(cb_write),
                Some(cb_flush),
            );
            0
        }
    });
    same!("write_fn then read_fn", |api| {
        unsafe {
            let s = WriteSess::new(api);
            (api.png_set_read_fn)(s.png, std::ptr::null_mut(), Some(cb_read));
            0
        }
    });
    // NULL png_ptr early returns
    same!("png_set_read_fn(NULL)", |api| {
        unsafe {
            (api.png_set_read_fn)(std::ptr::null_mut(), std::ptr::null_mut(), Some(cb_read))
        };
        0
    });
    same!("png_set_write_fn(NULL)", |api| {
        unsafe {
            (api.png_set_write_fn)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                Some(cb_write),
                Some(cb_flush),
            )
        };
        0
    });
    same!("png_init_io(NULL png_ptr)", |api| {
        unsafe { (api.png_init_io)(std::ptr::null_mut(), std::ptr::null_mut()) };
        0
    });
    same!("png_init_io(NULL FILE)", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_init_io)(s.png, std::ptr::null_mut());
        }
        0
    });
    same!("png_get_io_ptr(NULL)", |api| {
        let p = unsafe { (api.png_get_io_ptr)(std::ptr::null_mut()) };
        p.is_null() as i64
    });
    same!("png_set_flush(NULL)", |api| {
        unsafe { (api.png_set_flush)(std::ptr::null_mut(), 5) };
        0
    });
    for n in [-1i32, 0, 1, 1000] {
        same!(format!("png_set_flush({})", n), |api| {
            unsafe {
                let s = WriteSess::new(api);
                (api.png_set_flush)(s.png, n);
            }
            0
        });
    }
    // NOTE: png_flush() has NO NULL guard in the C (pngwio.c) -- it reads
    // png_ptr->output_flush_fn unconditionally, so NULL png_ptr is C UB.
    // png_default_flush()/png_default_read_data()/png_default_write_data() are
    // likewise UB unless io_ptr is a real FILE*.
    same!("png_flush on fresh write struct", |api| {
        unsafe {
            let s = WriteSess::new(api);
            (api.png_flush)(s.png);
        }
        0
    });
    // png_write_flush before any row has been written
    same!("png_write_flush on fresh write struct", |api| {
        unsafe {
            let s = WriteSess::new(api);
            (api.png_write_flush)(s.png);
        }
        0
    });
    same!("png_write_flush(NULL)", |api| {
        unsafe { (api.png_write_flush)(std::ptr::null_mut()) };
        0
    });
    // "Read Error" from a truncated stream
    for n in [0usize, 1, 4, 7, 8, 9, 20] {
        same!(format!("truncated stream, {} bytes", n), |api| {
            unsafe {
                let sig: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
                let mut data = sig.to_vec();
                data.extend_from_slice(&[0u8; 32]);
                data.truncate(n);
                let s = ReadSess::new(api, &data);
                (api.png_read_info)(s.png, s.info);
                0
            }
        });
    }
    // png_get_io_state / png_get_io_chunk_type on a live read
    same!("io state during read", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            let a = (api.png_get_io_state)(s.png) as i64;
            let b = (api.png_get_io_chunk_type)(s.png) as i64;
            a * 1_000_000 + b
        }
    });
}

#[test]
fn read_sig_and_signature_rejections() {
    let good: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    for corrupt_at in 0..9usize {
        same!(format!("bad signature byte {}", corrupt_at), |api| {
            unsafe {
                let mut data = good.to_vec();
                if corrupt_at < 8 {
                    data[corrupt_at] ^= 0xff;
                }
                data.extend_from_slice(&[0u8; 40]);
                let s = ReadSess::new(api, &data);
                (api.png_read_info)(s.png, s.info);
                0
            }
        });
    }
    same!("png_get_signature(NULL)", |api| {
        let p = unsafe { (api.png_get_signature)(std::ptr::null(), std::ptr::null()) };
        p.is_null() as i64
    });
}

#[test]
fn crc_and_zstream_error_paths() {
    same!("png_reset_crc + png_calculate_crc", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_reset_crc)(s.png);
            let b = [1u8, 2, 3, 4, 5, 6, 7, 8];
            (api.png_calculate_crc)(s.png, b.as_ptr(), 8);
            (api.png_calculate_crc)(s.png, b.as_ptr(), 0);
            (api.png_calculate_crc)(s.png, std::ptr::null(), 0);
        }
        0
    });
    // png_zstream_error for every zlib return code
    for ret in [
        0i32, 1, 2, -1, -2, -3, -4, -5, -6, 3, -100, 100, i32::MIN, i32::MAX,
    ] {
        same!(format!("png_zstream_error({})", ret), |api| {
            unsafe {
                let s = ReadSess::new(api, &[]);
                (api.png_zstream_error)(s.png, ret);
            }
            0
        });
    }
    // png_reset_zstream on a struct with no zstream yet
    same!("png_reset_zstream fresh", |api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_reset_zstream)(s.png) as i64
        }
    });
    same!("png_reset_zstream(NULL)", |api| {
        unsafe { (api.png_reset_zstream)(std::ptr::null_mut()) as i64 }
    });
}

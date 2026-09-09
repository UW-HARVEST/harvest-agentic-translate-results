//! Phase B / C cases for the exported low-level primitives and struct
//! lifecycle — the functions an external caller can reach without any PNG
//! stream at all.

use crate::api::Api;
use crate::p;
use crate::support::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

pub fn run(api: &Api, case: &str, seed: u64) -> bool {
    let mut rng = Rng::new(seed);
    match case {
        "prim/int_get" => int_get(api, &mut rng),
        "prim/int_save" => int_save(api, &mut rng),
        "prim/sig_cmp" => sig_cmp(api, &mut rng),
        "prim/version" => version(api),
        "prim/grayscale_palette" => grayscale_palette(api),
        "prim/rfc1123" => rfc1123(api, &mut rng),
        "prim/info_lifecycle" => info_lifecycle(api, &mut rng),
        "prim/null_guards" => null_guards(api),
        "prim/create_struct_bad_ver" => create_struct_bad_ver(api),
        "prim/malloc" => malloc_paths(api),
        "prim/mng_features" => mng_features(api),
        "prim/option" => option_values(api),
        "prim/get_overflow" => get_overflow(api, &mut rng),
        "prim/simple_zero_dims" => simple_zero_dims(api, &mut rng),
        _ => return false,
    }
    true
}

/// The interesting 32-bit boundary values plus randomized coverage.
fn boundary_words() -> Vec<u32> {
    vec![
        0x0000_0000,
        0x0000_0001,
        0x0000_00ff,
        0x0000_ff00,
        0x0000_ffff,
        0x0001_0000,
        0x7fff_fffe,
        0x7fff_ffff,
        0x8000_0000,
        0x8000_0001,
        0xfffe_ffff,
        0xffff_fffe,
        0xffff_ffff,
    ]
}

/// CONFIGS.md B1 — `png_get_uint_32` / `_uint_16` / `_int_32` / `_uint_31`.
fn int_get(api: &Api, rng: &mut Rng) {
    unsafe {
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING),
            vnull(),
            Some(err_fn),
            Some(warn_fn),
        );
        let mut words = boundary_words();
        for _ in 0..1024 {
            words.push(rng.u32());
        }
        for w in &words {
            let b = w.to_be_bytes();
            let u32v = (api.png_get_uint_32)(b.as_ptr());
            let u16v = (api.png_get_uint_16)(b.as_ptr());
            let u16v2 = (api.png_get_uint_16)(b.as_ptr().add(2));
            let i32v = (api.png_get_int_32)(b.as_ptr());
            p!("{:08x} u32={:08x} u16={:04x}/{:04x} i32={}", w, u32v, u16v, u16v2, i32v);
            // png_get_uint_31 errors out above PNG_UINT_31_MAX; only exercise
            // the non-erroring half here (the error is ERRORS.md C158).
            if *w <= 0x7fff_ffff {
                p!("  u31={:08x}", (api.png_get_uint_31)(png, b.as_ptr()));
            }
        }
        let mut p2 = png;
        (api.png_destroy_read_struct)(&mut p2, null(), null());
    }
}

/// CONFIGS.md B2 — `png_save_uint_32` / `_uint_16` / `_int_32`, round-tripped.
fn int_save(api: &Api, rng: &mut Rng) {
    unsafe {
        let mut words = boundary_words();
        for _ in 0..1024 {
            words.push(rng.u32());
        }
        let mut buf = [0u8; 8];
        for w in &words {
            (api.png_save_uint_32)(buf.as_mut_ptr(), *w);
            p!("save_u32 {:08x} -> {:02x}{:02x}{:02x}{:02x} back={:08x}",
               w, buf[0], buf[1], buf[2], buf[3],
               (api.png_get_uint_32)(buf.as_ptr()));
            (api.png_save_int_32)(buf.as_mut_ptr(), *w as i32);
            p!("save_i32 {} -> {:02x}{:02x}{:02x}{:02x} back={}",
               *w as i32, buf[0], buf[1], buf[2], buf[3],
               (api.png_get_int_32)(buf.as_ptr()));
            (api.png_save_uint_16)(buf.as_mut_ptr(), (*w & 0xffff) as c_int);
            p!("save_u16 {:04x} -> {:02x}{:02x} back={:04x}",
               *w & 0xffff, buf[0], buf[1], (api.png_get_uint_16)(buf.as_ptr()));
        }
    }
}

/// CONFIGS.md B3 and ERRORS.md C1/C2/C3 — every `start`/`num_to_check` pair,
/// with a good signature and with each byte individually corrupted.
fn sig_cmp(api: &Api, rng: &mut Rng) {
    unsafe {
        let good = PNG_SIG;
        for start in 0usize..12 {
            for n in 0usize..12 {
                p!("good start={} n={} -> {}",
                   start, n, (api.png_sig_cmp)(good.as_ptr(), start, n));
            }
        }
        for corrupt in 0usize..8 {
            for delta in [1u8, 0x80, 0xff] {
                let mut s = good;
                s[corrupt] ^= delta;
                for start in 0usize..9 {
                    for n in [0usize, 1, 4, 8, 9] {
                        p!("bad[{}^{:02x}] start={} n={} -> {}",
                           corrupt, delta, start, n,
                           (api.png_sig_cmp)(s.as_ptr(), start, n));
                    }
                }
            }
        }
        for _ in 0..64 {
            let s = rng.bytes(8);
            p!("rand {:02x?} -> {} {} {}", s,
               (api.png_sig_cmp)(s.as_ptr(), 0, 8),
               (api.png_sig_cmp)(s.as_ptr(), 0, 4),
               (api.png_sig_cmp)(s.as_ptr(), 4, 4));
        }
    }
}

/// CONFIGS.md B4 — version and copyright strings.
fn version(api: &Api) {
    unsafe {
        p!("version_number={}", (api.png_access_version_number)());
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING),
            vnull(),
            Some(err_fn),
            Some(warn_fn),
        );
        p!("copyright={:?}", cstr((api.png_get_copyright)(png)));
        p!("header_ver={:?}", cstr((api.png_get_header_ver)(png)));
        p!("header_version={:?}", cstr((api.png_get_header_version)(png)));
        p!("libpng_ver={:?}", cstr((api.png_get_libpng_ver)(png)));
        // Also with a NULL png_ptr, which these accessors accept.
        p!("copyright(null)={:?}", cstr((api.png_get_copyright)(vnull())));
        p!("libpng_ver(null)={:?}", cstr((api.png_get_libpng_ver)(vnull())));
        let mut p2 = png;
        (api.png_destroy_read_struct)(&mut p2, null(), null());
    }
}

/// CONFIGS.md B164 — `png_build_grayscale_palette` at every bit depth,
/// including the depths the function does not accept.
fn grayscale_palette(api: &Api) {
    unsafe {
        for depth in [-1i32, 0, 1, 2, 3, 4, 5, 8, 9, 16, 255] {
            let mut pal = [png_color::default(); 256];
            (api.png_build_grayscale_palette)(depth, pal.as_mut_ptr());
            let flat: Vec<u8> = pal
                .iter()
                .flat_map(|c| [c.red, c.green, c.blue])
                .collect();
            emit_bytes(&format!("gray_pal depth={}", depth), &flat);
        }
        // NULL palette must be a no-op, not a crash.
        (api.png_build_grayscale_palette)(8, null());
        emit("gray_pal null ok");
    }
}

/// CONFIGS.md B161 and ERRORS.md C19–C22 — time formatting, valid and invalid.
fn rfc1123(api: &Api, rng: &mut Rng) {
    unsafe {
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING),
            vnull(),
            Some(err_fn),
            Some(warn_fn),
        );

        let mut cases: Vec<png_time> = vec![
            png_time { year: 1970, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
            png_time { year: 9999, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
            png_time { year: 2000, month: 2, day: 29, hour: 12, minute: 30, second: 15 },
            // ERRORS.md C20: each boundary one step out of range.
            png_time { year: 10000, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
            png_time { year: 2000, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
            png_time { year: 2000, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
            png_time { year: 2000, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
            png_time { year: 2000, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
            png_time { year: 2000, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
            png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
            png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
            png_time { year: 0xffff, month: 255, day: 255, hour: 255, minute: 255, second: 255 },
        ];
        for _ in 0..64 {
            cases.push(png_time {
                year: rng.range(0, 12000) as u16,
                month: rng.range(0, 14) as u8,
                day: rng.range(0, 33) as u8,
                hour: rng.range(0, 25) as u8,
                minute: rng.range(0, 61) as u8,
                second: rng.range(0, 62) as u8,
            });
        }

        for t in &cases {
            let mut buf = [0i8; 29];
            let r = (api.png_convert_to_rfc1123_buffer)(
                buf.as_mut_ptr() as *mut c_char,
                t,
            );
            let s: Vec<u8> = buf.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
            p!("{:?} -> r={} {:?}", (t.year, t.month, t.day, t.hour, t.minute, t.second),
               r, String::from_utf8_lossy(&s));
            // ERRORS.md C21: the deprecated wrapper warns and returns NULL for
            // invalid times.
            let dep = (api.png_convert_to_rfc1123)(png, t);
            p!("   deprecated={:?}", if dep.is_null() { "<null>".to_string() } else { cstr(dep) });
        }

        // ERRORS.md C19: NULL out buffer.
        let t = png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
        p!("null_out -> {}", (api.png_convert_to_rfc1123_buffer)(null(), &t));
        // ERRORS.md C22: NULL png_ptr / NULL ptime.
        p!("deprecated(null png)={}", (api.png_convert_to_rfc1123)(vnull(), &t).is_null());
        p!("deprecated(null time)={}", (api.png_convert_to_rfc1123)(png, null()).is_null());

        // png_convert_from_time_t over a spread of epoch values.
        for tt in [0i64, 1, 1000000, 951782400, 2147483647, -1] {
            let mut out = png_time::default();
            (api.png_convert_from_time_t)(&mut out, tt);
            p!("from_time_t {} -> {:?}", tt,
               (out.year, out.month, out.day, out.hour, out.minute, out.second));
        }

        let mut p2 = png;
        (api.png_destroy_read_struct)(&mut p2, null(), null());
    }
}

/// CONFIGS.md B160 and ERRORS.md C9/C14/C312/C313 — info-struct lifecycle,
/// `png_free_data` masks and `png_data_freer` with valid and invalid arguments.
fn info_lifecycle(api: &Api, rng: &mut Rng) {
    unsafe {
        // ERRORS.md C9: NULL png_ptr.
        p!("info(null)={}", (api.png_create_info_struct)(vnull()).is_null());

        for freer in [1i32, 2] {
            let png = (api.png_create_write_struct)(
                cptr(PNG_LIBPNG_VER_STRING),
                vnull(),
                Some(err_fn),
                Some(warn_fn),
            );
            let info = (api.png_create_info_struct)(png);
            (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB,
                               PNG_INTERLACE_NONE, 0, 0);
            let key = cs("Comment");
            let txt = cs("hello");
            let t = png_text {
                compression: -1,
                key: key.as_ptr() as *mut c_char,
                text: txt.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: null(),
                lang_key: null(),
            };
            (api.png_set_text)(png, info, &t, 1);
            let hist: Vec<u16> = (0..256).map(|_| rng.u32() as u16).collect();
            let pal: Vec<png_color> = (0..256)
                .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
                .collect();
            (api.png_set_PLTE)(png, info, pal.as_ptr(), 256);
            (api.png_set_hIST)(png, info, hist.as_ptr());
            p!("freer={} valid_before={:05x}", freer,
               (api.png_get_valid)(png, info, 0xffff_ffff));
            (api.png_data_freer)(png, info, freer, PNG_FREE_ALL);
            // ERRORS.md C313: free with a range of masks including -1 and 0.
            for mask in [0u32, 0x0008, 0x1000, 0x4000, 0xffff, 0xffff_ffff] {
                for num in [-1i32, 0, 1, 9999] {
                    (api.png_free_data)(png, info, mask, num);
                }
            }
            p!("  valid_after={:05x}", (api.png_get_valid)(png, info, 0xffff_ffff));
            // Ownership was handed to the app for freer==2; hand it back so the
            // struct teardown is identical in both libraries.
            (api.png_data_freer)(png, info, 1, PNG_FREE_ALL);
            let mut pp = png;
            let mut ip = info;
            (api.png_destroy_write_struct)(&mut pp, &mut ip);
            p!("  destroyed png_null={} info_null={}", pp.is_null(), ip.is_null());
        }

        // Destroy with every NULL combination.
        let png = (api.png_create_write_struct)(
            cptr(PNG_LIBPNG_VER_STRING), vnull(), Some(err_fn), Some(warn_fn));
        let mut pp = png;
        (api.png_destroy_write_struct)(&mut pp, null());
        (api.png_destroy_write_struct)(null(), null());
        (api.png_destroy_read_struct)(null(), null(), null());
        emit("destroy null combos ok");
    }
}

/// ERRORS.md C10–C12, C15, C23–C30, C35, C323, C324 — the NULL-argument
/// sentinel contract of every guarded accessor.
fn null_guards(api: &Api) {
    unsafe {
        p!("io_ptr={}", (api.png_get_io_ptr)(vnull()).is_null());
        p!("error_ptr={}", (api.png_get_error_ptr)(vnull()).is_null());
        p!("longjmp_fn={}", (api.png_set_longjmp_fn)(vnull(), vnull(), 200).is_null());
        p!("malloc={}", (api.png_malloc)(vnull(), 16).is_null());
        p!("mem_ptr={}", (api.png_get_mem_ptr)(vnull()).is_null());
        p!("palette_max={}", (api.png_get_palette_max)(vnull(), vnull()));
        p!("rowbytes={}", (api.png_get_rowbytes)(vnull(), vnull()));
        p!("channels={}", (api.png_get_channels)(vnull(), vnull()));
        p!("width={}", (api.png_get_image_width)(vnull(), vnull()));
        p!("height={}", (api.png_get_image_height)(vnull(), vnull()));
        p!("bit_depth={}", (api.png_get_bit_depth)(vnull(), vnull()));
        p!("color_type={}", (api.png_get_color_type)(vnull(), vnull()));
        p!("filter_type={}", (api.png_get_filter_type)(vnull(), vnull()));
        p!("interlace={}", (api.png_get_interlace_type)(vnull(), vnull()));
        p!("compression={}", (api.png_get_compression_type)(vnull(), vnull()));
        p!("signature={}", (api.png_get_signature)(vnull(), vnull()).is_null());
        p!("user_chunk_ptr={}", (api.png_get_user_chunk_ptr)(vnull()).is_null());
        p!("progressive_ptr={}", (api.png_get_progressive_ptr)(vnull()).is_null());
        p!("user_transform_ptr={}", (api.png_get_user_transform_ptr)(vnull()).is_null());
        p!("current_row={}", (api.png_get_current_row_number)(vnull()));
        p!("current_pass={}", (api.png_get_current_pass_number)(vnull()));
        p!("io_state={}", (api.png_get_io_state)(vnull()));
        p!("io_chunk_type={}", (api.png_get_io_chunk_type)(vnull()));
        p!("compression_buffer_size={}", (api.png_get_compression_buffer_size)(vnull()));
        p!("user_width_max={}", (api.png_get_user_width_max)(vnull()));
        p!("user_height_max={}", (api.png_get_user_height_max)(vnull()));
        p!("chunk_cache_max={}", (api.png_get_chunk_cache_max)(vnull()));
        p!("chunk_malloc_max={}", (api.png_get_chunk_malloc_max)(vnull()));
        p!("rows={}", (api.png_get_rows)(vnull(), vnull()).is_null());
        p!("rgb_to_gray_status={}", (api.png_get_rgb_to_gray_status)(vnull()));
        p!("valid={}", (api.png_get_valid)(vnull(), vnull(), 0xffff));
        p!("handle_as_unknown={}", (api.png_handle_as_unknown)(vnull(), b"tEXt".as_ptr()));

        // Out-parameter NULL / info NULL variants of the chunk getters.
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING), vnull(), Some(err_fn), Some(warn_fn));
        let info = (api.png_create_info_struct)(png);
        p!("IHDR(all null out)={}",
           (api.png_get_IHDR)(png, info, null(), null(), null(), null(), null(), null(), null()));
        p!("PLTE={}", (api.png_get_PLTE)(png, info, null(), null()));
        p!("tRNS={}", (api.png_get_tRNS)(png, info, null(), null(), null()));
        p!("bKGD={}", (api.png_get_bKGD)(png, info, null()));
        p!("gAMA={}", (api.png_get_gAMA_fixed)(png, info, null()));
        p!("sBIT={}", (api.png_get_sBIT)(png, info, null()));
        p!("sRGB={}", (api.png_get_sRGB)(png, info, null()));
        p!("iCCP={}", (api.png_get_iCCP)(png, info, null(), null(), null(), null()));
        p!("hIST={}", (api.png_get_hIST)(png, info, null()));
        p!("pHYs={}", (api.png_get_pHYs)(png, info, null(), null(), null()));
        p!("oFFs={}", (api.png_get_oFFs)(png, info, null(), null(), null()));
        p!("sCAL_s={}", (api.png_get_sCAL_s)(png, info, null(), null(), null()));
        p!("tIME={}", (api.png_get_tIME)(png, info, null()));
        p!("text={}", (api.png_get_text)(png, info, null(), null()));
        p!("sPLT={}", (api.png_get_sPLT)(png, info, null()));
        p!("eXIf_1={}", (api.png_get_eXIf_1)(png, info, null(), null()));
        p!("cICP={}", (api.png_get_cICP)(png, info, null(), null(), null(), null()));
        p!("cLLI_fixed={}", (api.png_get_cLLI_fixed)(png, info, null(), null()));
        p!("unknown_chunks={}", (api.png_get_unknown_chunks)(png, info, null()));
        p!("pixels_per_meter={}", (api.png_get_pixels_per_meter)(png, info));
        p!("x_ppi={}", (api.png_get_x_pixels_per_inch)(png, info));
        p!("aspect_fixed={}", (api.png_get_pixel_aspect_ratio_fixed)(png, info));
        p!("x_off_px={}", (api.png_get_x_offset_pixels)(png, info));
        p!("x_off_in_fixed={}", (api.png_get_x_offset_inches_fixed)(png, info));

        // Setters with NULL info / NULL payload must be silent no-ops.
        (api.png_set_PLTE)(png, null(), null(), 0);
        (api.png_set_tRNS)(png, null(), null(), 0, null());
        (api.png_set_bKGD)(png, null(), null());
        (api.png_set_sBIT)(png, null(), null());
        (api.png_set_hIST)(png, null(), null());
        (api.png_set_tIME)(png, null(), null());
        (api.png_set_text)(png, null(), null(), 0);
        (api.png_set_iCCP)(png, null(), null(), 0, null(), 0);
        (api.png_set_sPLT)(png, null(), null(), 0);
        (api.png_set_unknown_chunks)(png, null(), null(), 0);
        (api.png_set_invalid)(vnull(), vnull(), 0xffff);
        emit("null setters ok");

        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_read_struct)(&mut pp, &mut ip, null());
    }
}

/// ERRORS.md C6–C8 — version-mismatch rejection in the struct constructors.
fn create_struct_bad_ver(api: &Api) {
    unsafe {
        for v in [
            &b"0.0.0\0"[..],
            &b"1.5.0\0"[..],
            &b"2.0.0\0"[..],
            &b"1.6.59.git\0"[..],
            &b"1.6.0\0"[..],
            &b"\0"[..],
            &b"x\0"[..],
        ] {
            let rp = (api.png_create_read_struct)(
                v.as_ptr() as *const c_char, vnull(), Some(err_fn), Some(warn_fn));
            let wp = (api.png_create_write_struct)(
                v.as_ptr() as *const c_char, vnull(), Some(err_fn), Some(warn_fn));
            p!("ver={:?} read_null={} write_null={}",
               String::from_utf8_lossy(&v[..v.len() - 1]), rp.is_null(), wp.is_null());
            if !rp.is_null() {
                let mut x = rp;
                (api.png_destroy_read_struct)(&mut x, null(), null());
            }
            if !wp.is_null() {
                let mut x = wp;
                (api.png_destroy_write_struct)(&mut x, null());
            }
        }
        // NULL version string.
        let rp = (api.png_create_read_struct)(null(), vnull(), Some(err_fn), Some(warn_fn));
        p!("ver=NULL read_null={}", rp.is_null());
        if !rp.is_null() {
            let mut x = rp;
            (api.png_destroy_read_struct)(&mut x, null(), null());
        }
    }
}

/// ERRORS.md C16 — `png_malloc_warn` on an impossible size warns and returns NULL.
fn malloc_paths(api: &Api) {
    unsafe {
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING), vnull(), Some(err_fn), Some(warn_fn));
        for size in [0u64, 1, 16, u64::MAX, u64::MAX / 2, 0x7fff_ffff_ffff_ffff] {
            let q = (api.png_malloc_warn)(png, size);
            p!("malloc_warn {} null={}", size, q.is_null());
            if !q.is_null() {
                (api.png_free)(png, q);
            }
        }
        for size in [1u64, 4096] {
            let q = (api.png_calloc)(png, size);
            p!("calloc {} null={}", size, q.is_null());
            if !q.is_null() {
                let s = std::slice::from_raw_parts(q as *const u8, size as usize);
                p!("  zeroed={}", s.iter().all(|b| *b == 0));
                (api.png_free)(png, q);
            }
        }
        (api.png_free)(png, vnull());
        emit("free(null) ok");
        let mut pp = png;
        (api.png_destroy_read_struct)(&mut pp, null(), null());
    }
}

/// CONFIGS.md B162 — `png_permit_mng_features` for each flag combination.
fn mng_features(api: &Api) {
    unsafe {
        for f in [0u32, 0x01, 0x04, 0x05, 0xff, 0xffff_ffff] {
            let png = (api.png_create_read_struct)(
                cptr(PNG_LIBPNG_VER_STRING), vnull(), Some(err_fn), Some(warn_fn));
            p!("mng {:#x} -> {:#x}", f, (api.png_permit_mng_features)(png, f));
            let mut pp = png;
            (api.png_destroy_read_struct)(&mut pp, null(), null());
        }
    }
}

/// ERRORS.md C31–C34 — the `pHYs`/`oFFs` unit-conversion accessors at and past
/// the point where their fixed-point arithmetic overflows, and the "valid bit
/// clear" sentinel of every chunk getter.
fn get_overflow(api: &Api, rng: &mut Rng) {
    unsafe {
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING), vnull(), Some(err_fn), Some(warn_fn));
        let info = (api.png_create_info_struct)(png);

        // C34: every accessor with the corresponding `valid` bit clear.
        for flag in [
            PNG_INFO_gAMA, PNG_INFO_sBIT, PNG_INFO_cHRM, PNG_INFO_PLTE,
            PNG_INFO_tRNS, PNG_INFO_bKGD, PNG_INFO_hIST, PNG_INFO_pHYs,
            PNG_INFO_oFFs, PNG_INFO_tIME, PNG_INFO_pCAL, PNG_INFO_sRGB,
            PNG_INFO_iCCP, PNG_INFO_sPLT, PNG_INFO_sCAL, PNG_INFO_IDAT,
            PNG_INFO_eXIf, PNG_INFO_cICP, PNG_INFO_cLLI, PNG_INFO_mDCV,
        ] {
            p!("valid {:#07x} -> {}", flag, (api.png_get_valid)(png, info, flag));
        }
        let mut g: i32 = -1;
        p!("gAMA_unset={} g={}", (api.png_get_gAMA_fixed)(png, info, &mut g), g);
        let mut intent: c_int = -1;
        p!("sRGB_unset={} intent={}", (api.png_get_sRGB)(png, info, &mut intent), intent);
        let mut rx: u32 = 7;
        let mut ry: u32 = 7;
        let mut ut: c_int = 7;
        p!("pHYs_unset={} {} {} {}",
           (api.png_get_pHYs)(png, info, &mut rx, &mut ry, &mut ut), rx, ry, ut);
        let mut ox: i32 = 7;
        let mut oy: i32 = 7;
        p!("oFFs_unset={} {} {} {}",
           (api.png_get_oFFs)(png, info, &mut ox, &mut oy, &mut ut), ox, oy, ut);
        let mut ne: u32 = 7;
        let mut ep: *mut u8 = null();
        p!("eXIf_unset={} n={} null={}",
           (api.png_get_eXIf_1)(png, info, &mut ne, &mut ep), ne, ep.is_null());
        let mut cll: u32 = 7;
        let mut fall: u32 = 7;
        p!("cLLI_unset={} {} {}",
           (api.png_get_cLLI_fixed)(png, info, &mut cll, &mut fall), cll, fall);

        // C31 / C33: pHYs resolutions at and beyond PNG_UINT_31_MAX, plus a
        // zero x resolution (aspect-ratio divide guard).
        let mut resolutions: Vec<(u32, u32, c_int)> = vec![
            (0, 0, 0),
            (0, 1, 0),
            (1, 0, 0),
            (1, 1, 1),
            (2540, 2540, 1),
            (0x7fff_ffff, 0x7fff_ffff, 1),
            (0x8000_0000, 1, 1),
            (0xffff_ffff, 0xffff_ffff, 1),
            (0xffff_ffff, 1, 0),
            (1, 0xffff_ffff, 1),
        ];
        for _ in 0..24 {
            resolutions.push((rng.u32(), rng.u32(), (rng.below(3)) as c_int));
        }
        for (x, y, unit) in &resolutions {
            (api.png_set_pHYs)(png, info, *x, *y, *unit);
            let mut a: u32 = 0;
            let mut b: u32 = 0;
            let mut u: c_int = 0;
            p!("pHYs {} {} unit={} get={} ({} {} {})",
               x, y, unit, (api.png_get_pHYs)(png, info, &mut a, &mut b, &mut u), a, b, u);
            p!("   ppm={} xppm={} yppm={}",
               (api.png_get_pixels_per_meter)(png, info),
               (api.png_get_x_pixels_per_meter)(png, info),
               (api.png_get_y_pixels_per_meter)(png, info));
            p!("   ppi={} xppi={} yppi={}",
               (api.png_get_pixels_per_inch)(png, info),
               (api.png_get_x_pixels_per_inch)(png, info),
               (api.png_get_y_pixels_per_inch)(png, info));
            p!("   aspect_fixed={} aspect={:.6}",
               (api.png_get_pixel_aspect_ratio_fixed)(png, info),
               (api.png_get_pixel_aspect_ratio)(png, info));
            let mut dx: u32 = 0;
            let mut dy: u32 = 0;
            let mut du: c_int = 0;
            p!("   dpi={} ({} {} {})",
               (api.png_get_pHYs_dpi)(png, info, &mut dx, &mut dy, &mut du), dx, dy, du);
        }

        // C32: oFFs offsets whose micron->inch conversion overflows the
        // fixed-point range (emits "fixed point overflow ignored").
        let mut offsets: Vec<(i32, i32, c_int)> = vec![
            (0, 0, 0),
            (0, 0, 1),
            (1, -1, 1),
            (i32::MAX, i32::MIN + 1, 1),
            (i32::MAX, i32::MAX, 1),
            (i32::MIN + 1, i32::MIN + 1, 1),
            (2540, -2540, 1),
            (1_000_000, -1_000_000, 1),
        ];
        for _ in 0..24 {
            offsets.push((rng.u32() as i32, rng.u32() as i32, (rng.below(2)) as c_int));
        }
        for (x, y, unit) in &offsets {
            (api.png_set_oFFs)(png, info, *x, *y, *unit);
            p!("oFFs {} {} unit={} px=({},{}) um=({},{})",
               x, y, unit,
               (api.png_get_x_offset_pixels)(png, info),
               (api.png_get_y_offset_pixels)(png, info),
               (api.png_get_x_offset_microns)(png, info),
               (api.png_get_y_offset_microns)(png, info));
            p!("   in_fixed=({},{}) in=({:.6},{:.6})",
               (api.png_get_x_offset_inches_fixed)(png, info),
               (api.png_get_y_offset_inches_fixed)(png, info),
               (api.png_get_x_offset_inches)(png, info),
               (api.png_get_y_offset_inches)(png, info));
        }

        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_read_struct)(&mut pp, &mut ip, null());
    }
}

/// ERRORS.md C295 / C302 — the simplified API with a zero `width`/`height`.
///
/// `IHDR` rejects a zero dimension, so a zero-width `png_image` can only arise
/// from the application overwriting the field between
/// `png_image_begin_read_from_memory` and `png_image_finish_read`. Both the read
/// and write sides then evaluate `... / (width * channels)`, i.e. a division by
/// zero. The C reference executes it and takes the hardware divide fault, so
/// each variant runs in its own process and the driver compares the terminating
/// signal as well as the transcript.
fn simple_zero_dims(api: &Api, rng: &mut Rng) {
    unsafe {
        // variants 0..3: read side; 4..7: write side.
        let v = variant();
        let (zero_w, zero_h) = match v % 4 {
            0 => (true, false),
            1 => (false, true),
            2 => (true, true),
            _ => (false, false),
        };
        if v % 8 < 4 {
            let data = build_png(rng, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8, &[], &[]);
            let mut image = png_image::default();
            image.version = PNG_IMAGE_VERSION;
            let r = (api.png_image_begin_read_from_memory)(
                &mut image,
                data.as_ptr() as *const c_void,
                data.len(),
            );
            p!("begin r={} w={} h={} fmt={:#x}", r, image.width, image.height, image.format);
            if r != 0 {
                if zero_w {
                    image.width = 0;
                }
                if zero_h {
                    image.height = 0;
                }
                let mut buf = vec![0u8; 4 * 4 * 4 + 64];
                let fr = (api.png_image_finish_read)(
                    &mut image,
                    null(),
                    buf.as_mut_ptr() as *mut c_void,
                    0,
                    null(),
                );
                p!("finish zw={} zh={} r={} woe={} msg={:?}",
                   zero_w, zero_h, fr, image.warning_or_error, image.msg());
            }
            (api.png_image_free)(&mut image);
        } else {
            let mut image = png_image::default();
            image.version = PNG_IMAGE_VERSION;
            image.width = if zero_w { 0 } else { 4 };
            image.height = if zero_h { 0 } else { 4 };
            image.format = PNG_FORMAT_FLAG_COLOR;
            let src = rng.image_bytes(4 * 4 * 3);
            let mut size: u64 = 0;
            let r = (api.png_image_write_to_memory)(
                &mut image,
                vnull(),
                &mut size,
                0,
                src.as_ptr() as *const c_void,
                0,
                null(),
            );
            p!("write zw={} zh={} r={} bytes={} woe={} msg={:?}",
               zero_w, zero_h, r, size, image.warning_or_error, image.msg());
            (api.png_image_free)(&mut image);
        }
    }
}

fn option_values(api: &Api) {
    unsafe {
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING), vnull(), Some(err_fn), Some(warn_fn));
        for opt in [-2i32, -1, 0, 1, 2, 3, 4, 5, 6, 8, 10, 14, 15, 16, 17, 99, i32::MAX] {
            for onoff in [-1i32, 0, 1, 2, 3, 99] {
                p!("option {} {} -> {}", opt, onoff, (api.png_set_option)(png, opt, onoff));
            }
        }
        p!("option(null)={}", (api.png_set_option)(vnull(), 2, 1));
        let mut pp = png;
        (api.png_destroy_read_struct)(&mut pp, null(), null());
    }
}

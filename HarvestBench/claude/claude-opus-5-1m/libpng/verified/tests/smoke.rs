//! Harness self-check: both shared objects load, expose the same symbols, and
//! agree on a trivial write/read round trip.
mod common;

use common::*;
use core::ffi::c_char;

#[test]
fn both_libraries_load_and_export_every_symbol() {
    let l = libs();
    // `Api::load` panics on the first missing symbol, so reaching here means all
    // 384 exported symbols resolved in BOTH shared objects.
    assert_eq!(API_NAMES.len(), 381);
    assert_eq!(l.c.which, "C");
    assert_eq!(l.rust.which, "Rust");
}

#[test]
fn version_numbers_match() {
    let l = libs();
    unsafe {
        assert_eq!(
            (l.c.png_access_version_number)(),
            (l.rust.png_access_version_number)()
        );
        let a = std::ffi::CStr::from_ptr((l.c.png_get_libpng_ver)(core::ptr::null_mut()));
        let b = std::ffi::CStr::from_ptr((l.rust.png_get_libpng_ver)(core::ptr::null_mut()));
        assert_eq!(a, b);
        let a = std::ffi::CStr::from_ptr((l.c.png_get_header_version)(core::ptr::null_mut()));
        let b = std::ffi::CStr::from_ptr((l.rust.png_get_header_version)(core::ptr::null_mut()));
        assert_eq!(a, b);
        let a = std::ffi::CStr::from_ptr((l.c.png_get_copyright)(core::ptr::null_mut()));
        let b = std::ffi::CStr::from_ptr((l.rust.png_get_copyright)(core::ptr::null_mut()));
        assert_eq!(a, b);
    }
}

/// Build an 8-bit RGB image of the given size with a deterministic pattern.
fn pattern(w: u32, h: u32, chans: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    (0..h)
        .map(|_| (0..(w as usize * chans)).map(|_| rng.u8()).collect())
        .collect()
}

#[test]
fn trivial_write_round_trip_is_byte_identical() {
    assert_same("write 8x8 rgb8", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
        let rows = pattern(8, 8, 3, 1);
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                8,
                8,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            for r in &rows {
                (api.png_write_row)(png, r.as_ptr() as *mut u8);
            }
            (api.png_write_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        o.output = std::mem::take(&mut tls().output);
        destroy_write(api, png, info);
        o
    });
}

#[test]
fn trivial_read_round_trip_is_byte_identical() {
    // Produce the file once with the C library, then read it with both.
    let l = libs();
    let file = unsafe {
        let mut state = Box::new(Tls::default());
        let prev = set_tls(&mut *state as *mut Tls);
        let prev_api = set_cur_api(&l.c as *const Api);
        let api = &l.c;
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
        let rows = pattern(9, 5, 4, 7);
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                9,
                5,
                8,
                PNG_COLOR_TYPE_RGB_ALPHA,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            for r in &rows {
                (api.png_write_row)(png, r.as_ptr() as *mut u8);
            }
            (api.png_write_end)(png, info);
        });
        assert_eq!(g, Guard::Ok);
        destroy_write(api, png, info);
        let out = std::mem::take(&mut state.output);
        set_cur_api(prev_api);
        set_tls(prev);
        out
    };
    assert!(file.len() > 8);

    assert_same("read 9x5 rgba8", |api| unsafe {
        let mut o = Outcome::default();
        tls().input = file.clone();
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            let w = (api.png_get_image_width)(png, info);
            let h = (api.png_get_image_height)(png, info);
            let rb = (api.png_get_rowbytes)(png, info);
            log(format!("w={} h={} rowbytes={}", w, h, rb));
            log(format!(
                "depth={} color={} interlace={} comp={} filter={}",
                (api.png_get_bit_depth)(png, info),
                (api.png_get_color_type)(png, info),
                (api.png_get_interlace_type)(png, info),
                (api.png_get_compression_type)(png, info),
                (api.png_get_filter_type)(png, info)
            ));
            let mut row = vec![0u8; rb];
            for y in 0..h {
                (api.png_read_row)(png, row.as_mut_ptr(), core::ptr::null_mut());
                log(format!("row {}: {:02x?}", y, &row));
            }
            (api.png_read_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o
    });
}

#[test]
fn fatal_error_is_reported_identically() {
    // Feed both libraries garbage: the signature check must fail the same way.
    assert_same("bad signature", |api| unsafe {
        let mut o = Outcome::default();
        tls().input = b"not a png file at all, really not".to_vec();
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o
    });
}

#[test]
fn pure_functions_agree() {
    let l = libs();
    let mut rng = Rng::new(0xfeed);
    unsafe {
        for _ in 0..2000 {
            let b = rng.bytes(4);
            assert_eq!(
                (l.c.png_get_uint_32)(b.as_ptr()),
                (l.rust.png_get_uint_32)(b.as_ptr())
            );
            assert_eq!(
                (l.c.png_get_uint_16)(b.as_ptr()),
                (l.rust.png_get_uint_16)(b.as_ptr())
            );
            assert_eq!(
                (l.c.png_get_int_32)(b.as_ptr()),
                (l.rust.png_get_int_32)(b.as_ptr())
            );
            let mut x = [0u8; 4];
            let mut y = [0u8; 4];
            let v = rng.u32();
            (l.c.png_save_uint_32)(x.as_mut_ptr(), v);
            (l.rust.png_save_uint_32)(y.as_mut_ptr(), v);
            assert_eq!(x, y);
            (l.c.png_save_int_32)(x.as_mut_ptr(), v as i32);
            (l.rust.png_save_int_32)(y.as_mut_ptr(), v as i32);
            assert_eq!(x, y);
            (l.c.png_save_uint_16)(x.as_mut_ptr(), (v & 0xffff) as u32);
            (l.rust.png_save_uint_16)(y.as_mut_ptr(), (v & 0xffff) as u32);
            assert_eq!(x, y);
        }
        // png_sig_cmp over random prefixes
        for _ in 0..2000 {
            let b = rng.bytes(8);
            let start = rng.below(8);
            let n = rng.below(9 - start);
            assert_eq!(
                (l.c.png_sig_cmp)(b.as_ptr(), start, n),
                (l.rust.png_sig_cmp)(b.as_ptr(), start, n),
                "png_sig_cmp({:02x?}, {}, {})",
                b,
                start,
                n
            );
        }
        // exported data tables
        for i in 0..256 {
            assert_eq!(
                *l.c.png_sRGB_table.add(i),
                *l.rust.png_sRGB_table.add(i),
                "png_sRGB_table[{}]",
                i
            );
        }
        for i in 0..512 {
            assert_eq!(*l.c.png_sRGB_base.add(i), *l.rust.png_sRGB_base.add(i));
            assert_eq!(*l.c.png_sRGB_delta.add(i), *l.rust.png_sRGB_delta.add(i));
        }
    }
    let _ = 0 as *const c_char;
}

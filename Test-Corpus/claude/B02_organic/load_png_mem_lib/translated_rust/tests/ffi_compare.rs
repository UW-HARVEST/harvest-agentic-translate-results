// Integration tests comparing the Rust .so to the C .so via libloading.
//
// We do not call the Rust functions directly. Both .so files are loaded
// through libloading, and outputs are byte-compared.

mod common;

use common::*;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::ffi::c_void;
use std::io::Write;
use std::os::raw::c_int;

fn deflate_zlib(data: &[u8], level: u32) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).expect("zlib write");
    e.finish().expect("zlib finish")
}

#[test]
fn test_cp_inflate_basic() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_inflate = get_cp_inflate(&c_lib);
        let r_inflate = get_cp_inflate(&r_lib);

        // Test with several payloads, each through cp_inflate which expects
        // a raw DEFLATE stream (note: PNG path strips the 2-byte zlib header
        // before calling cp_inflate).
        let payloads: Vec<Vec<u8>> = vec![
            b"Hello, world!".to_vec(),
            b"".to_vec(),
            (0..256u32).map(|i| (i & 0xff) as u8).collect(),
            (0..1024).map(|i| ((i * 31) & 0xff) as u8).collect(),
            // a highly compressible stream
            vec![0x42u8; 4096],
            // pseudo-random with mid-pattern
            (0..2048).map(|i| ((i * 137 + 7) & 0xff) as u8).collect(),
        ];

        for (idx, payload) in payloads.iter().enumerate() {
            // Encode as zlib, strip 2-byte zlib header and 4-byte adler32 trailer.
            // load_png_mem itself passes data+2 with datalen-6, which is
            // raw deflate sandwiched between the zlib wrapper.
            for level in [0u32, 1, 6, 9] {
                let z = deflate_zlib(payload, level);
                if z.len() < 6 {
                    continue;
                }
                let raw = &z[2..z.len() - 4];

                let mut c_out = vec![0u8; payload.len() + 16];
                let mut r_out = vec![0u8; payload.len() + 16];

                // Need a writeable buffer for the input (cp_inflate takes void*).
                let mut c_in = raw.to_vec();
                let mut r_in = raw.to_vec();

                let c_rc = c_inflate(
                    c_in.as_mut_ptr() as *mut c_void,
                    c_in.len() as c_int,
                    c_out.as_mut_ptr() as *mut c_void,
                    payload.len() as c_int,
                );
                let r_rc = r_inflate(
                    r_in.as_mut_ptr() as *mut c_void,
                    r_in.len() as c_int,
                    r_out.as_mut_ptr() as *mut c_void,
                    payload.len() as c_int,
                );

                assert_eq!(
                    c_rc, r_rc,
                    "rc mismatch idx={} level={}", idx, level
                );
                assert_eq!(
                    &c_out[..payload.len()],
                    &r_out[..payload.len()],
                    "output mismatch idx={} level={}",
                    idx,
                    level
                );
            }
        }
    }
}

fn run_load_png_mem_test(name: &str, png: &[u8]) {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_load = get_load_png_mem(&c_lib);
        let r_load = get_load_png_mem(&r_lib);

        let mut c_img = c_load(png.as_ptr(), png.len() as c_int);
        let mut r_img = r_load(png.as_ptr(), png.len() as c_int);

        assert_eq!(c_img.w, r_img.w, "{}: width", name);
        assert_eq!(c_img.h, r_img.h, "{}: height", name);

        if c_img.pix.is_null() && r_img.pix.is_null() {
            return;
        }
        assert!(!c_img.pix.is_null(), "{}: c null pix", name);
        assert!(!r_img.pix.is_null(), "{}: rust null pix", name);

        let n = (c_img.w as isize * c_img.h as isize) as usize;
        let cs = std::slice::from_raw_parts(c_img.pix, n);
        let rs = std::slice::from_raw_parts(r_img.pix, n);
        assert_eq!(cs, rs, "{}: pixel data differs", name);

        free_image(&mut c_img);
        free_image(&mut r_img);
    }
}

#[test]
fn test_load_png_rgba() {
    let w = 8u32;
    let h = 6u32;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            rgba.push((x * 31) as u8);
            rgba.push((y * 41) as u8);
            rgba.push(((x ^ y) * 17) as u8);
            rgba.push(((x + y) * 23) as u8);
        }
    }
    let png = encode_png_rgba(w, h, &rgba);
    run_load_png_mem_test("rgba", &png);
}

#[test]
fn test_load_png_rgb() {
    let w = 5u32;
    let h = 4u32;
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            rgb.push((x * 51) as u8);
            rgb.push((y * 61) as u8);
            rgb.push(((x + y) * 11) as u8);
        }
    }
    let png = encode_png_rgb(w, h, &rgb);
    run_load_png_mem_test("rgb", &png);
}

#[test]
fn test_load_png_grayscale() {
    let w = 7u32;
    let h = 3u32;
    let gray: Vec<u8> = (0..(w * h)).map(|i| (i * 13) as u8).collect();
    let png = encode_png_gray(w, h, &gray);
    run_load_png_mem_test("grayscale", &png);
}

#[test]
fn test_load_png_grayscale_alpha() {
    let w = 4u32;
    let h = 4u32;
    let mut ga = Vec::with_capacity((w * h * 2) as usize);
    for i in 0..(w * h) {
        ga.push((i * 17) as u8);
        ga.push((i * 23 + 5) as u8);
    }
    let png = encode_png_gray_alpha(w, h, &ga);
    run_load_png_mem_test("grayscale_alpha", &png);
}

#[test]
fn test_load_png_indexed() {
    let w = 6u32;
    let h = 6u32;
    let palette: Vec<u8> = (0..(8 * 3)).map(|i| (i * 11) as u8).collect();
    let indices: Vec<u8> = (0..(w * h)).map(|i| (i % 8) as u8).collect();
    let png = encode_png_indexed(w, h, &indices, &palette, None);
    run_load_png_mem_test("indexed", &png);
}

#[test]
fn test_load_png_indexed_with_trns() {
    let w = 6u32;
    let h = 6u32;
    let palette: Vec<u8> = (0..(8 * 3)).map(|i| (i * 11) as u8).collect();
    let indices: Vec<u8> = (0..(w * h)).map(|i| (i % 8) as u8).collect();
    let trns = vec![0u8, 64, 128, 200];
    let png = encode_png_indexed(w, h, &indices, &palette, Some(&trns));
    run_load_png_mem_test("indexed_trns", &png);
}

#[test]
fn test_load_png_invalid_signature() {
    let bad = vec![0u8; 32];
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_load = get_load_png_mem(&c_lib);
        let r_load = get_load_png_mem(&r_lib);

        let c_img = c_load(bad.as_ptr(), bad.len() as c_int);
        let r_img = r_load(bad.as_ptr(), bad.len() as c_int);
        // Both should return an image with null pix and same w,h.
        assert_eq!(c_img.w, r_img.w);
        assert_eq!(c_img.h, r_img.h);
        assert!(c_img.pix.is_null());
        assert!(r_img.pix.is_null());
    }
}

#[test]
fn test_load_png_larger() {
    // A larger image to exercise dynamic huffman blocks.
    let w = 64u32;
    let h = 48u32;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            // Mix patterns + repetition to encourage backreferences.
            rgba.push(((x * 7 + y) & 0xff) as u8);
            rgba.push(((y * 13 + x) & 0xff) as u8);
            rgba.push(((x ^ y) & 0xff) as u8);
            rgba.push(((x + y) & 0xff) as u8);
        }
    }
    let png = encode_png_rgba(w, h, &rgba);
    run_load_png_mem_test("rgba_64x48", &png);
}

#[test]
fn test_exported_global_arrays_match() {
    // Verify that globals exported by both libs have the same contents.
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let names: &[(&[u8], usize)] = &[
            (b"cp_fixed_table\0", 288 + 32),
            (b"cp_permutation_order\0", 19),
            (b"cp_len_extra_bits\0", 31),
            (b"cp_dist_extra_bits\0", 32),
        ];
        for (sym, len) in names {
            let cs: libloading::Symbol<*const u8> = c_lib.get(sym).unwrap();
            let rs: libloading::Symbol<*const u8> = r_lib.get(sym).unwrap();
            let c = std::slice::from_raw_parts(*cs, *len);
            let r = std::slice::from_raw_parts(*rs, *len);
            assert_eq!(c, r, "global {:?}", std::str::from_utf8(sym).unwrap());
        }

        let u32_names: &[(&[u8], usize)] = &[
            (b"cp_len_base\0", 31),
            (b"cp_dist_base\0", 32),
        ];
        for (sym, len) in u32_names {
            let cs: libloading::Symbol<*const u32> = c_lib.get(sym).unwrap();
            let rs: libloading::Symbol<*const u32> = r_lib.get(sym).unwrap();
            let c = std::slice::from_raw_parts(*cs, *len);
            let r = std::slice::from_raw_parts(*rs, *len);
            assert_eq!(c, r, "global {:?}", std::str::from_utf8(sym).unwrap());
        }
    }
}

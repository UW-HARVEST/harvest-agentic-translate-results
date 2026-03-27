use libloading::{Library, Symbol};
use premultiply_lib::{cp_image_t, cp_pixel_t, premultiply};
use std::os::raw::c_int;

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libpremultiply_lib.so", manifest)
}

/// Test pixels covering key alpha values: 0, 1, 127, 128, 254, 255
/// and various RGB combos including edge cases.
fn test_pixels() -> Vec<cp_pixel_t> {
    let mut pixels = Vec::new();
    let vals: &[u8] = &[0, 1, 2, 127, 128, 200, 254, 255];
    for &a in vals {
        for &r in vals {
            for &g in &[0u8, 128, 255] {
                for &b in &[0u8, 128, 255] {
                    pixels.push(cp_pixel_t { r, g, b, a });
                }
            }
        }
    }
    pixels
}

#[test]
fn test_premultiply_matches_c() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let c_premultiply: Symbol<unsafe extern "C" fn(*mut cp_image_t)> =
        unsafe { lib.get(b"premultiply").expect("Failed to find premultiply") };

    let pixels = test_pixels();
    let w = pixels.len() as c_int;
    let h = 1 as c_int;

    // Clone for C and Rust separately
    let mut c_pixels: Vec<cp_pixel_t> = pixels
        .iter()
        .map(|p| cp_pixel_t { r: p.r, g: p.g, b: p.b, a: p.a })
        .collect();
    let mut rust_pixels: Vec<cp_pixel_t> = pixels
        .iter()
        .map(|p| cp_pixel_t { r: p.r, g: p.g, b: p.b, a: p.a })
        .collect();

    let mut c_img = cp_image_t { w, h, pix: c_pixels.as_mut_ptr() };
    let mut rust_img = cp_image_t { w, h, pix: rust_pixels.as_mut_ptr() };

    unsafe {
        c_premultiply(&mut c_img);
        premultiply(&mut rust_img);
    }

    for (i, (c, r)) in c_pixels.iter().zip(rust_pixels.iter()).enumerate() {
        let orig = &pixels[i];
        assert_eq!(
            (c.r, c.g, c.b, c.a),
            (r.r, r.g, r.b, r.a),
            "Mismatch at pixel {}: input=({},{},{},{}), C=({},{},{},{}), Rust=({},{},{},{})",
            i, orig.r, orig.g, orig.b, orig.a,
            c.r, c.g, c.b, c.a,
            r.r, r.g, r.b, r.a,
        );
    }
}

/// Test with a multi-row image to verify stride handling
#[test]
fn test_premultiply_multirow() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let c_premultiply: Symbol<unsafe extern "C" fn(*mut cp_image_t)> =
        unsafe { lib.get(b"premultiply").expect("Failed to find premultiply") };

    let w = 3i32;
    let h = 2i32;
    let base: Vec<cp_pixel_t> = (0..6)
        .map(|i| cp_pixel_t {
            r: (i * 40 + 10) as u8,
            g: (i * 30 + 20) as u8,
            b: (i * 20 + 30) as u8,
            a: (i * 50) as u8,
        })
        .collect();

    let mut c_pix: Vec<cp_pixel_t> = base.iter().map(|p| cp_pixel_t { r: p.r, g: p.g, b: p.b, a: p.a }).collect();
    let mut r_pix: Vec<cp_pixel_t> = base.iter().map(|p| cp_pixel_t { r: p.r, g: p.g, b: p.b, a: p.a }).collect();

    let mut c_img = cp_image_t { w, h, pix: c_pix.as_mut_ptr() };
    let mut r_img = cp_image_t { w, h, pix: r_pix.as_mut_ptr() };

    unsafe {
        c_premultiply(&mut c_img);
        premultiply(&mut r_img);
    }

    for (i, (c, r)) in c_pix.iter().zip(r_pix.iter()).enumerate() {
        assert_eq!(
            (c.r, c.g, c.b, c.a),
            (r.r, r.g, r.b, r.a),
            "Multirow mismatch at pixel {}", i
        );
    }
}

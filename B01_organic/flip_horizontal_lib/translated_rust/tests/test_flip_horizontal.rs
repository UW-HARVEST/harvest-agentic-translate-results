use libloading::{Library, Symbol};
use std::ffi::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct cp_pixel_t {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct cp_image_t {
    w: c_int,
    h: c_int,
    pix: *mut cp_pixel_t,
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libflip_horizontal_lib.so", manifest)
}

fn make_pixels(w: usize, h: usize) -> Vec<cp_pixel_t> {
    (0..(w * h) as u8)
        .map(|i| cp_pixel_t {
            r: i,
            g: i.wrapping_mul(3),
            b: i.wrapping_mul(7),
            a: 255 - i,
        })
        .collect()
}

#[test]
fn test_flip_horizontal_matches_c() {
    let w: usize = 4;
    let h: usize = 3;

    // Prepare two identical pixel buffers
    let mut c_pixels = make_pixels(w, h);
    let mut rust_pixels = c_pixels.clone();

    // Call C version via libloading
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let func: Symbol<unsafe extern "C" fn(*mut cp_image_t)> =
            lib.get(b"flip_horizontal").expect("Failed to find flip_horizontal");

        let mut c_img = cp_image_t {
            w: w as c_int,
            h: h as c_int,
            pix: c_pixels.as_mut_ptr(),
        };
        func(&mut c_img);
    }

    // Call Rust version
    unsafe {
        let mut rust_img = flip_horizontal_lib::cp_image_t {
            w: w as c_int,
            h: h as c_int,
            pix: rust_pixels.as_mut_ptr() as *mut flip_horizontal_lib::cp_pixel_t,
        };
        flip_horizontal_lib::flip_horizontal(&mut rust_img);
    }

    // Compare byte-for-byte
    let c_bytes = unsafe {
        std::slice::from_raw_parts(c_pixels.as_ptr() as *const u8, c_pixels.len() * 4)
    };
    let rust_bytes = unsafe {
        std::slice::from_raw_parts(rust_pixels.as_ptr() as *const u8, rust_pixels.len() * 4)
    };
    assert_eq!(c_bytes, rust_bytes, "flip_horizontal output mismatch");
}

#[test]
fn test_flip_horizontal_even_height() {
    let w: usize = 3;
    let h: usize = 4;

    let mut c_pixels = make_pixels(w, h);
    let mut rust_pixels = c_pixels.clone();

    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let func: Symbol<unsafe extern "C" fn(*mut cp_image_t)> =
            lib.get(b"flip_horizontal").expect("symbol");
        let mut c_img = cp_image_t {
            w: w as c_int,
            h: h as c_int,
            pix: c_pixels.as_mut_ptr(),
        };
        func(&mut c_img);
    }

    unsafe {
        let mut rust_img = flip_horizontal_lib::cp_image_t {
            w: w as c_int,
            h: h as c_int,
            pix: rust_pixels.as_mut_ptr() as *mut flip_horizontal_lib::cp_pixel_t,
        };
        flip_horizontal_lib::flip_horizontal(&mut rust_img);
    }

    let c_bytes = unsafe {
        std::slice::from_raw_parts(c_pixels.as_ptr() as *const u8, c_pixels.len() * 4)
    };
    let rust_bytes = unsafe {
        std::slice::from_raw_parts(rust_pixels.as_ptr() as *const u8, rust_pixels.len() * 4)
    };
    assert_eq!(c_bytes, rust_bytes, "flip_horizontal even-height mismatch");
}

#[test]
fn test_flip_horizontal_single_row() {
    let w: usize = 5;
    let h: usize = 1;

    let mut c_pixels = make_pixels(w, h);
    let mut rust_pixels = c_pixels.clone();

    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let func: Symbol<unsafe extern "C" fn(*mut cp_image_t)> =
            lib.get(b"flip_horizontal").expect("symbol");
        let mut c_img = cp_image_t {
            w: w as c_int,
            h: h as c_int,
            pix: c_pixels.as_mut_ptr(),
        };
        func(&mut c_img);
    }

    unsafe {
        let mut rust_img = flip_horizontal_lib::cp_image_t {
            w: w as c_int,
            h: h as c_int,
            pix: rust_pixels.as_mut_ptr() as *mut flip_horizontal_lib::cp_pixel_t,
        };
        flip_horizontal_lib::flip_horizontal(&mut rust_img);
    }

    let c_bytes = unsafe {
        std::slice::from_raw_parts(c_pixels.as_ptr() as *const u8, c_pixels.len() * 4)
    };
    let rust_bytes = unsafe {
        std::slice::from_raw_parts(rust_pixels.as_ptr() as *const u8, rust_pixels.len() * 4)
    };
    assert_eq!(c_bytes, rust_bytes, "flip_horizontal single-row mismatch");
}

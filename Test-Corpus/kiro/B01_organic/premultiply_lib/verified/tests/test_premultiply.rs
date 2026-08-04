use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

type PremultiplyFn = unsafe extern "C" fn(*mut CpImage);

fn load_libs() -> (Library, Library) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_path = format!("{}/c_src/build/libtranslated_rust.so", manifest);
    let rust_path = format!("{}/target/debug/libpremultiply_lib.so", manifest);
    unsafe {
        (
            Library::new(&c_path).expect("load C .so"),
            Library::new(&rust_path).expect("load Rust .so"),
        )
    }
}

fn run_both(pixels: &[CpPixel], w: c_int, h: c_int) -> (Vec<CpPixel>, Vec<CpPixel>) {
    let (c_lib, rust_lib) = load_libs();

    let mut c_pix = pixels.to_vec();
    let mut r_pix = pixels.to_vec();

    let mut c_img = CpImage { w, h, pix: c_pix.as_mut_ptr() };
    let mut r_img = CpImage { w, h, pix: r_pix.as_mut_ptr() };

    unsafe {
        let c_fn: Symbol<PremultiplyFn> = c_lib.get(b"premultiply").unwrap();
        let r_fn: Symbol<PremultiplyFn> = rust_lib.get(b"premultiply").unwrap();
        c_fn(&mut c_img);
        r_fn(&mut r_img);
    }

    (c_pix, r_pix)
}

#[test]
fn test_single_pixel_opaque() {
    let pixels = vec![CpPixel { r: 200, g: 100, b: 50, a: 255 }];
    let (c, r) = run_both(&pixels, 1, 1);
    assert_eq!(c, r, "opaque pixel mismatch");
}

#[test]
fn test_single_pixel_transparent() {
    let pixels = vec![CpPixel { r: 200, g: 100, b: 50, a: 0 }];
    let (c, r) = run_both(&pixels, 1, 1);
    assert_eq!(c, r, "transparent pixel mismatch");
}

#[test]
fn test_single_pixel_half_alpha() {
    let pixels = vec![CpPixel { r: 200, g: 100, b: 50, a: 128 }];
    let (c, r) = run_both(&pixels, 1, 1);
    assert_eq!(c, r, "half-alpha pixel mismatch");
}

#[test]
fn test_multiple_pixels() {
    let pixels = vec![
        CpPixel { r: 255, g: 255, b: 255, a: 128 },
        CpPixel { r: 0, g: 0, b: 0, a: 255 },
        CpPixel { r: 100, g: 200, b: 50, a: 64 },
        CpPixel { r: 1, g: 1, b: 1, a: 1 },
    ];
    let (c, r) = run_both(&pixels, 2, 2);
    assert_eq!(c, r, "multi-pixel mismatch");
}

#[test]
fn test_wide_image() {
    let pixels = vec![CpPixel { r: 123, g: 45, b: 67, a: 89 }; 100];
    let (c, r) = run_both(&pixels, 100, 1);
    assert_eq!(c, r, "wide image mismatch");
}

#[test]
fn test_exhaustive_alpha_channel() {
    // Test all 256 alpha values with a fixed RGB
    let pixels: Vec<CpPixel> = (0..=255u8)
        .map(|a| CpPixel { r: 200, g: 100, b: 50, a })
        .collect();
    let (c, r) = run_both(&pixels, 256, 1);
    for i in 0..256 {
        assert_eq!(c[i], r[i], "mismatch at alpha={}", i);
    }
}

#[test]
fn test_exhaustive_r_channel() {
    let pixels: Vec<CpPixel> = (0..=255u8)
        .map(|v| CpPixel { r: v, g: 100, b: 50, a: 170 })
        .collect();
    let (c, r) = run_both(&pixels, 256, 1);
    for i in 0..256 {
        assert_eq!(c[i], r[i], "mismatch at r={}", i);
    }
}

#[test]
fn test_empty_image() {
    let pixels: Vec<CpPixel> = vec![];
    let (c, r) = run_both(&pixels, 0, 0);
    assert_eq!(c, r);
}

// Integration tests that load both C .so and Rust .so via libloading
// and compare their byte-identical outputs through the FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

fn c_lib_path() -> PathBuf {
    // c_src/build/libtranslated_rust.so relative to crate root
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // target/<profile>/libpremultiply_lib.so
    // CARGO_TARGET_TMPDIR points inside target; use OUT_DIR-style discovery.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try debug, then release.
    let debug = p.join("debug").join("libpremultiply_lib.so");
    let release = p.join("release").join("libpremultiply_lib.so");
    if debug.exists() {
        debug
    } else if release.exists() {
        release
    } else {
        panic!(
            "Could not find Rust .so at {} or {}",
            debug.display(),
            release.display()
        );
    }
}

unsafe fn run_premultiply_via(lib_path: &std::path::Path, pixels: &mut [CpPixel], w: c_int, h: c_int) {
    let lib = Library::new(lib_path).expect("failed to load library");
    let func: Symbol<PremultiplyFn> =
        lib.get(b"premultiply").expect("symbol premultiply not found");
    let mut img = CpImage {
        w,
        h,
        pix: pixels.as_mut_ptr(),
    };
    func(&mut img as *mut CpImage);
}

fn run_both(input: Vec<CpPixel>, w: c_int, h: c_int) -> (Vec<CpPixel>, Vec<CpPixel>) {
    let mut c_buf = input.clone();
    let mut rust_buf = input;
    unsafe {
        run_premultiply_via(&c_lib_path(), &mut c_buf, w, h);
        run_premultiply_via(&rust_lib_path(), &mut rust_buf, w, h);
    }
    (c_buf, rust_buf)
}

#[test]
fn test_premultiply_zero_alpha() {
    let pixels = vec![
        CpPixel { r: 255, g: 128, b: 64, a: 0 },
        CpPixel { r: 100, g: 200, b: 50,  a: 0 },
    ];
    let (c_out, rust_out) = run_both(pixels, 2, 1);
    assert_eq!(c_out, rust_out);
}

#[test]
fn test_premultiply_full_alpha() {
    let pixels = vec![
        CpPixel { r: 255, g: 128, b: 64, a: 255 },
        CpPixel { r: 100, g: 200, b: 50,  a: 255 },
    ];
    let (c_out, rust_out) = run_both(pixels, 2, 1);
    assert_eq!(c_out, rust_out);
}

#[test]
fn test_premultiply_half_alpha() {
    let pixels = vec![
        CpPixel { r: 255, g: 128, b: 64, a: 128 },
        CpPixel { r: 100, g: 200, b: 50,  a: 128 },
        CpPixel { r: 0,   g: 0,   b: 0,   a: 128 },
        CpPixel { r: 255, g: 255, b: 255, a: 128 },
    ];
    let (c_out, rust_out) = run_both(pixels, 2, 2);
    assert_eq!(c_out, rust_out);
}

#[test]
fn test_premultiply_all_alpha_values() {
    // For each alpha, several r/g/b channel combos.
    let mut pixels = Vec::new();
    for a in 0u16..=255u16 {
        for c in [0u8, 1, 64, 127, 128, 200, 254, 255] {
            pixels.push(CpPixel {
                r: c,
                g: c.wrapping_add(1),
                b: c.wrapping_add(2),
                a: a as u8,
            });
        }
    }
    let h = 1;
    let w = pixels.len() as c_int;
    let (c_out, rust_out) = run_both(pixels, w, h);
    assert_eq!(c_out, rust_out, "C and Rust results differ");
}

#[test]
fn test_premultiply_exhaustive_random() {
    // Exhaustive over all (r,a) pairs, and some g,b values.
    let mut pixels = Vec::new();
    for r in 0u16..=255u16 {
        for a in 0u16..=255u16 {
            pixels.push(CpPixel {
                r: r as u8,
                g: ((r + a) & 0xff) as u8,
                b: ((r ^ a) & 0xff) as u8,
                a: a as u8,
            });
        }
    }
    let total = pixels.len() as c_int;
    let (c_out, rust_out) = run_both(pixels, total, 1);
    assert_eq!(c_out, rust_out);
}

#[test]
fn test_premultiply_empty() {
    let pixels: Vec<CpPixel> = vec![];
    let (c_out, rust_out) = run_both(pixels, 0, 0);
    assert_eq!(c_out, rust_out);
}

#[test]
fn test_premultiply_h_only() {
    // w=0, h=5 should produce no work.
    let pixels: Vec<CpPixel> = vec![
        CpPixel { r: 1, g: 2, b: 3, a: 4 };
        0
    ];
    let (c_out, rust_out) = run_both(pixels, 0, 5);
    assert_eq!(c_out, rust_out);
}

#[test]
fn test_premultiply_single_pixel_2d() {
    let pixels = vec![
        CpPixel { r: 200, g: 100, b: 50, a: 64 },
    ];
    let (c_out, rust_out) = run_both(pixels, 1, 1);
    assert_eq!(c_out, rust_out);
}

#[test]
fn test_premultiply_alpha_preservation() {
    // Alpha must remain unchanged after premultiplication.
    let pixels = vec![
        CpPixel { r: 100, g: 100, b: 100, a: 77 },
    ];
    let (c_out, rust_out) = run_both(pixels.clone(), 1, 1);
    assert_eq!(c_out, rust_out);
    assert_eq!(c_out[0].a, 77);
    assert_eq!(rust_out[0].a, 77);
}

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug");
    dir.join("libflip_horizontal_lib.so")
}

type FlipFn = unsafe extern "C" fn(*mut CpImage);

fn load_flip(lib: &Library) -> Symbol<FlipFn> {
    unsafe { lib.get(b"flip_horizontal").expect("symbol not found") }
}

fn run_case(w: c_int, h: c_int, pixels: &[CpPixel]) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };
    let c_fn = load_flip(&c_lib);
    let r_fn = load_flip(&r_lib);

    let mut c_pix = pixels.to_vec();
    let mut r_pix = pixels.to_vec();

    let mut c_img = CpImage { w, h, pix: c_pix.as_mut_ptr() };
    let mut r_img = CpImage { w, h, pix: r_pix.as_mut_ptr() };

    unsafe {
        c_fn(&mut c_img);
        r_fn(&mut r_img);
    }

    assert_eq!(c_pix, r_pix, "mismatch for {}x{}", w, h);
}

fn px(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

#[test]
fn test_1x1() {
    run_case(1, 1, &[px(1, 2, 3, 4)]);
}

#[test]
fn test_2x2() {
    run_case(2, 2, &[px(1,0,0,255), px(2,0,0,255), px(3,0,0,255), px(4,0,0,255)]);
}

#[test]
fn test_3x3() {
    let pixels: Vec<CpPixel> = (0..9).map(|i| px(i as u8, 0, 0, 255)).collect();
    run_case(3, 3, &pixels);
}

#[test]
fn test_4x2() {
    let pixels: Vec<CpPixel> = (0..8).map(|i| px(i as u8, i as u8, 0, 255)).collect();
    run_case(4, 2, &pixels);
}

#[test]
fn test_1x5() {
    let pixels: Vec<CpPixel> = (0..5).map(|i| px(i as u8 * 50, 0, 0, 255)).collect();
    run_case(1, 5, &pixels);
}

#[test]
fn test_empty_0x0() {
    run_case(0, 0, &[]);
}

#[test]
fn test_wide_10x1() {
    let pixels: Vec<CpPixel> = (0..10).map(|i| px(i, i, i, 255)).collect();
    run_case(10, 1, &pixels);
}

#[test]
fn test_large_10x10() {
    let pixels: Vec<CpPixel> = (0..100).map(|i| px(i as u8, (i*2) as u8, (i*3) as u8, 255)).collect();
    run_case(10, 10, &pixels);
}

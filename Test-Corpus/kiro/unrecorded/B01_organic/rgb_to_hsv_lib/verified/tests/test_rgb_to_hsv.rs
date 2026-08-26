use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.join("target/debug/librgb_to_hsv_lib.so")
}

type RgbToHsvFn = unsafe extern "C" fn(*mut f32, *const f32);

fn call_both(src: &[f32; 3]) -> ([f32; 3], [f32; 3]) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let c_fn: Symbol<RgbToHsvFn> = c_lib.get(b"rgb_to_hsv").unwrap();
        let r_fn: Symbol<RgbToHsvFn> = r_lib.get(b"rgb_to_hsv").unwrap();

        let mut c_dest = [0f32; 3];
        let mut r_dest = [0f32; 3];
        c_fn(c_dest.as_mut_ptr(), src.as_ptr());
        r_fn(r_dest.as_mut_ptr(), src.as_ptr());
        (c_dest, r_dest)
    }
}

fn assert_match(src: &[f32; 3]) {
    let (c, r) = call_both(src);
    assert_eq!(
        c.map(f32::to_bits),
        r.map(f32::to_bits),
        "Mismatch for src={src:?}: C={c:?} Rust={r:?}"
    );
}

#[test]
fn test_pure_red() {
    assert_match(&[1.0, 0.0, 0.0]);
}

#[test]
fn test_pure_green() {
    assert_match(&[0.0, 1.0, 0.0]);
}

#[test]
fn test_pure_blue() {
    assert_match(&[0.0, 0.0, 1.0]);
}

#[test]
fn test_white() {
    assert_match(&[1.0, 1.0, 1.0]);
}

#[test]
fn test_black() {
    assert_match(&[0.0, 0.0, 0.0]);
}

#[test]
fn test_gray() {
    assert_match(&[0.5, 0.5, 0.5]);
}

#[test]
fn test_yellow() {
    assert_match(&[1.0, 1.0, 0.0]);
}

#[test]
fn test_cyan() {
    assert_match(&[0.0, 1.0, 1.0]);
}

#[test]
fn test_magenta() {
    assert_match(&[1.0, 0.0, 1.0]);
}

#[test]
fn test_arbitrary_values() {
    assert_match(&[0.2, 0.4, 0.6]);
    assert_match(&[0.8, 0.3, 0.1]);
    assert_match(&[0.1, 0.9, 0.5]);
    assert_match(&[0.0, 0.0, 0.001]);
    assert_match(&[0.999, 0.999, 0.998]);
}

#[test]
fn test_negative_hue_wrap() {
    // r == max, g < b => h negative before wrap
    assert_match(&[1.0, 0.0, 0.5]);
    assert_match(&[0.8, 0.1, 0.7]);
}

#[test]
fn test_delta_zero() {
    // all equal => delta == 0
    assert_match(&[0.3, 0.3, 0.3]);
}

#[test]
fn test_max_zero() {
    // all zero => max == 0
    assert_match(&[0.0, 0.0, 0.0]);
}

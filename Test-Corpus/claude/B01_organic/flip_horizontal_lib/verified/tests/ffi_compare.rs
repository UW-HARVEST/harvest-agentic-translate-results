use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: i32,
    h: i32,
    pix: *mut CpPixel,
}

type FlipHorizontalFn = unsafe extern "C" fn(*mut CpImage);

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // Walk up from CARGO_MANIFEST_DIR to find the target/<profile>/<lib>.so
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try debug first then release
    let candidates = [
        ("debug", "libflip_horizontal_lib.so"),
        ("release", "libflip_horizontal_lib.so"),
    ];
    for (profile, file) in &candidates {
        let mut q = p.clone();
        q.push(profile);
        q.push(file);
        if q.exists() {
            return q;
        }
    }
    // Default to debug path
    p.push("debug");
    p.push("libflip_horizontal_lib.so");
    p
}

fn run_flip(lib_path: &std::path::Path, w: i32, h: i32, pix: &mut [CpPixel]) {
    unsafe {
        let lib = Library::new(lib_path).expect("failed to load library");
        let sym: Symbol<FlipHorizontalFn> = lib
            .get(b"flip_horizontal")
            .expect("flip_horizontal symbol not found");
        let mut img = CpImage {
            w,
            h,
            pix: pix.as_mut_ptr(),
        };
        sym(&mut img as *mut CpImage);
    }
}

fn make_image(w: i32, h: i32, seed: u64) -> Vec<CpPixel> {
    let n = (w as usize).saturating_mul(h as usize);
    let mut v = Vec::with_capacity(n);
    let mut s = seed.wrapping_add(0x9E3779B97F4A7C15);
    for i in 0..n {
        // Some pseudorandom-but-deterministic content
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
            .wrapping_add(i as u64);
        v.push(CpPixel {
            r: (s >> 24) as u8,
            g: (s >> 32) as u8,
            b: (s >> 40) as u8,
            a: (s >> 48) as u8,
        });
    }
    v
}

fn compare_for(w: i32, h: i32, seed: u64) {
    let original = make_image(w, h, seed);

    let mut c_pix = original.clone();
    run_flip(&c_lib_path(), w, h, &mut c_pix);

    let mut r_pix = original.clone();
    run_flip(&rust_lib_path(), w, h, &mut r_pix);

    assert_eq!(
        c_pix, r_pix,
        "Mismatch for w={} h={} seed={}",
        w, h, seed
    );
}

#[test]
fn flip_basic_2x2() {
    compare_for(2, 2, 1);
}

#[test]
fn flip_basic_3x3() {
    compare_for(3, 3, 2);
}

#[test]
fn flip_4x4() {
    compare_for(4, 4, 3);
}

#[test]
fn flip_1x1() {
    compare_for(1, 1, 4);
}

#[test]
fn flip_1x10() {
    compare_for(1, 10, 5);
}

#[test]
fn flip_10x1() {
    compare_for(10, 1, 6);
}

#[test]
fn flip_5x7() {
    compare_for(5, 7, 7);
}

#[test]
fn flip_8x8() {
    compare_for(8, 8, 8);
}

#[test]
fn flip_zero_height() {
    // h=0: no flips
    compare_for(4, 0, 9);
}

#[test]
fn flip_zero_width() {
    // w=0: inner loop is no-op
    compare_for(0, 4, 10);
}

#[test]
fn flip_large() {
    compare_for(64, 48, 11);
}

#[test]
fn flip_odd_height() {
    // odd h: middle row not touched
    compare_for(7, 5, 12);
}

#[test]
fn flip_idempotent_check() {
    // flipping twice should restore original
    let w = 9;
    let h = 11;
    let original = make_image(w, h, 42);
    let mut pix = original.clone();
    run_flip(&rust_lib_path(), w, h, &mut pix);
    run_flip(&rust_lib_path(), w, h, &mut pix);
    assert_eq!(pix, original);
}

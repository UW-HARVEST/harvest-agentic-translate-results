use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::slice;

const C_LIB: &str = "/tmp/harvest-work-3RkR0c/translated_rust/c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "/tmp/harvest-work-3RkR0c/translated_rust/target/debug/libload_png_mem_lib.so";

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
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

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(C_LIB).expect("Failed to load C .so");
        let r = Library::new(RUST_LIB).expect("Failed to load Rust .so");
        (c, r)
    }
}

// --- Data table tests ---

#[test]
fn compare_cp_fixed_table() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_sym: Symbol<*const [u8; 320]> = c_lib.get(b"cp_fixed_table").unwrap();
        let r_sym: Symbol<*const [u8; 320]> = r_lib.get(b"cp_fixed_table").unwrap();
        let c_data = slice::from_raw_parts(*c_sym as *const u8, 320);
        let r_data = slice::from_raw_parts(*r_sym as *const u8, 320);
        assert_eq!(c_data, r_data, "cp_fixed_table mismatch");
    }
}

#[test]
fn compare_cp_permutation_order() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_sym: Symbol<*const [u8; 19]> = c_lib.get(b"cp_permutation_order").unwrap();
        let r_sym: Symbol<*const [u8; 19]> = r_lib.get(b"cp_permutation_order").unwrap();
        let c_data = slice::from_raw_parts(*c_sym as *const u8, 19);
        let r_data = slice::from_raw_parts(*r_sym as *const u8, 19);
        assert_eq!(c_data, r_data, "cp_permutation_order mismatch");
    }
}

#[test]
fn compare_cp_len_extra_bits() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_sym: Symbol<*const [u8; 31]> = c_lib.get(b"cp_len_extra_bits").unwrap();
        let r_sym: Symbol<*const [u8; 31]> = r_lib.get(b"cp_len_extra_bits").unwrap();
        let c_data = slice::from_raw_parts(*c_sym as *const u8, 31);
        let r_data = slice::from_raw_parts(*r_sym as *const u8, 31);
        assert_eq!(c_data, r_data, "cp_len_extra_bits mismatch");
    }
}

#[test]
fn compare_cp_len_base() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_sym: Symbol<*const [u32; 31]> = c_lib.get(b"cp_len_base").unwrap();
        let r_sym: Symbol<*const [u32; 31]> = r_lib.get(b"cp_len_base").unwrap();
        let c_data = slice::from_raw_parts(*c_sym as *const u8, 31 * 4);
        let r_data = slice::from_raw_parts(*r_sym as *const u8, 31 * 4);
        assert_eq!(c_data, r_data, "cp_len_base mismatch");
    }
}

#[test]
fn compare_cp_dist_extra_bits() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_sym: Symbol<*const [u8; 32]> = c_lib.get(b"cp_dist_extra_bits").unwrap();
        let r_sym: Symbol<*const [u8; 32]> = r_lib.get(b"cp_dist_extra_bits").unwrap();
        let c_data = slice::from_raw_parts(*c_sym as *const u8, 32);
        let r_data = slice::from_raw_parts(*r_sym as *const u8, 32);
        assert_eq!(c_data, r_data, "cp_dist_extra_bits mismatch");
    }
}

#[test]
fn compare_cp_dist_base() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_sym: Symbol<*const [u32; 32]> = c_lib.get(b"cp_dist_base").unwrap();
        let r_sym: Symbol<*const [u32; 32]> = r_lib.get(b"cp_dist_base").unwrap();
        let c_data = slice::from_raw_parts(*c_sym as *const u8, 32 * 4);
        let r_data = slice::from_raw_parts(*r_sym as *const u8, 32 * 4);
        assert_eq!(c_data, r_data, "cp_dist_base mismatch");
    }
}

// --- cp_inflate test ---

#[test]
fn compare_cp_inflate() {
    let (c_lib, r_lib) = load_libs();
    // Raw deflate of 52 'A' bytes
    let compressed: &[u8] = &[115, 116, 36, 29, 0, 0];
    let expected = [b'A'; 52];

    unsafe {
        type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
        let c_inflate: Symbol<InflateFn> = c_lib.get(b"cp_inflate").unwrap();
        let r_inflate: Symbol<InflateFn> = r_lib.get(b"cp_inflate").unwrap();

        let mut c_out = vec![0u8; 52];
        let mut r_out = vec![0u8; 52];

        let c_ret = c_inflate(
            compressed.as_ptr() as *mut c_void,
            compressed.len() as c_int,
            c_out.as_mut_ptr() as *mut c_void,
            c_out.len() as c_int,
        );
        let r_ret = r_inflate(
            compressed.as_ptr() as *mut c_void,
            compressed.len() as c_int,
            r_out.as_mut_ptr() as *mut c_void,
            r_out.len() as c_int,
        );

        assert_eq!(c_ret, 1, "C cp_inflate failed");
        assert_eq!(r_ret, 1, "Rust cp_inflate failed");
        assert_eq!(c_out, expected.to_vec(), "C inflate output wrong");
        assert_eq!(r_out, expected.to_vec(), "Rust inflate output wrong");
        assert_eq!(c_out, r_out, "cp_inflate output mismatch");
    }
}

// --- load_png_mem tests ---

fn compare_load_png(png_data: &[u8], label: &str) {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        type LoadPngFn = unsafe extern "C" fn(*const u8, c_int) -> CpImage;
        let c_load: Symbol<LoadPngFn> = c_lib.get(b"load_png_mem").unwrap();
        let r_load: Symbol<LoadPngFn> = r_lib.get(b"load_png_mem").unwrap();

        let c_img = c_load(png_data.as_ptr(), png_data.len() as c_int);
        let r_img = r_load(png_data.as_ptr(), png_data.len() as c_int);

        assert!(
            !c_img.pix.is_null(),
            "{label}: C load_png_mem returned null pix"
        );
        assert!(
            !r_img.pix.is_null(),
            "{label}: Rust load_png_mem returned null pix"
        );
        assert_eq!(c_img.w, r_img.w, "{label}: width mismatch");
        assert_eq!(c_img.h, r_img.h, "{label}: height mismatch");

        let npix = (c_img.w * c_img.h) as usize;
        let c_pixels = slice::from_raw_parts(c_img.pix, npix);
        let r_pixels = slice::from_raw_parts(r_img.pix, npix);
        assert_eq!(c_pixels, r_pixels, "{label}: pixel data mismatch");

        // Free malloc'd memory
        libc::free(c_img.pix as *mut c_void);
        libc::free(r_img.pix as *mut c_void);
    }
}

#[test]
fn compare_load_png_rgba_2x2() {
    let png_data = include_bytes!("test_2x2.png");
    compare_load_png(png_data, "RGBA 2x2");
}

#[test]
fn compare_load_png_grayscale_2x2() {
    let png_data = include_bytes!("test_gray.png");
    compare_load_png(png_data, "Grayscale 2x2");
}

#[test]
fn compare_load_png_rgb_2x2() {
    let png_data = include_bytes!("test_rgb.png");
    compare_load_png(png_data, "RGB 2x2");
}

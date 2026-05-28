// Shared utilities for FFI tests.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

pub type LoadPngMem = unsafe extern "C" fn(*const u8, c_int) -> CpImage;
pub type CpInflate = unsafe extern "C" fn(
    *mut std::ffi::c_void,
    c_int,
    *mut std::ffi::c_void,
    c_int,
) -> c_int;

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_lib_path() -> PathBuf {
    manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

pub fn rust_lib_path() -> PathBuf {
    // The cargo target dir for this crate.
    // Try multiple locations: target/debug under manifest, plus CARGO_TARGET_DIR.
    if let Some(p) = std::env::var_os("CARGO_TARGET_DIR") {
        let pb: PathBuf = p.into();
        let candidate = pb.join("debug").join("libload_png_mem_lib.so");
        if candidate.exists() {
            return candidate;
        }
        let candidate = pb.join("release").join("libload_png_mem_lib.so");
        if candidate.exists() {
            return candidate;
        }
    }
    let pb = manifest_dir().join("target");
    let candidate = pb.join("debug").join("libload_png_mem_lib.so");
    if candidate.exists() {
        return candidate;
    }
    let candidate = pb.join("release").join("libload_png_mem_lib.so");
    if candidate.exists() {
        return candidate;
    }
    panic!("Could not locate libload_png_mem_lib.so");
}

pub fn load_libs() -> (Library, Library) {
    // Force a build of the Rust cdylib so it exists.
    // The build script for tests doesn't automatically build the cdylib in newer
    // cargo, but `cargo test` does build the cdylib package.
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");
        (c, r)
    }
}

pub unsafe fn get_load_png_mem<'a>(lib: &'a Library) -> Symbol<'a, LoadPngMem> {
    lib.get(b"load_png_mem\0").expect("symbol load_png_mem")
}

pub unsafe fn get_cp_inflate<'a>(lib: &'a Library) -> Symbol<'a, CpInflate> {
    lib.get(b"cp_inflate\0").expect("symbol cp_inflate")
}

/// Encode a PNG image.
pub fn encode_png_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(rgba).expect("png write");
    }
    out
}

pub fn encode_png_rgb(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(rgb).expect("png write");
    }
    out
}

pub fn encode_png_gray(width: u32, height: u32, gray: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(gray).expect("png write");
    }
    out
}

pub fn encode_png_gray_alpha(width: u32, height: u32, ga: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::GrayscaleAlpha);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(ga).expect("png write");
    }
    out
}

pub fn encode_png_indexed(
    width: u32,
    height: u32,
    indices: &[u8],
    palette: &[u8],
    trns: Option<&[u8]>,
) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(palette);
        if let Some(t) = trns {
            encoder.set_trns(t);
        }
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(indices).expect("png write");
    }
    out
}

/// Free image pixels using the libc `free` from each lib.
/// We can't easily access libc directly in a portable way through libloading,
/// but the C and Rust .so use the system free, so we can free using libc.
pub unsafe fn free_image(img: &mut CpImage) {
    if !img.pix.is_null() {
        libc::free(img.pix as *mut std::ffi::c_void);
        img.pix = std::ptr::null_mut();
    }
}

pub unsafe fn pixels_slice(img: &CpImage) -> &[CpPixel] {
    if img.pix.is_null() {
        return &[];
    }
    let n = (img.w as isize * img.h as isize) as usize;
    std::slice::from_raw_parts(img.pix, n)
}
